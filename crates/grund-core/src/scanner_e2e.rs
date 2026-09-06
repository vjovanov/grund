/// Discover `e2e/cases/<name>/` directories and register each as an `E2E-<name>`
/// declaration whose body is the case manifest (§AR-scanner.6, §FS-show.2.4) — so
/// `grund check` sees `§E2E-…` citations resolve and `grund refs` finds e2e tests.
fn scan_e2e_cases(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    findings: &mut Findings,
) -> Result<()> {
    let Some(kind) = config.kinds.iter().find(|kind| kind.kind == "E2E" && kind.citable) else {
        return Ok(());
    };
    let Some(folder) = kind.folder.as_deref() else {
        return Ok(());
    };
    let cases_root = config.root.join(folder);
    if !cases_root.exists() || !cases_root.is_dir() {
        return Ok(());
    }
    let cases_root = fs::canonicalize(&cases_root).unwrap_or(cases_root);
    let mut scan_root = cases_root.clone();

    if explicit_scope {
        let scope = scope.unwrap_or(Path::new("."));
        if scope.is_file() {
            return Ok(());
        }
        let scope = fs::canonicalize(scope).unwrap_or_else(|_| scope.to_path_buf());
        if scope.starts_with(&cases_root) {
            scan_root = scope;
        } else if !cases_root.starts_with(&scope) {
            return Ok(());
        }
    } else if !config.scan_full
        && let Some(include) = &config.include
    {
        // §FS-check.1.3: `--full` cancels `include`, so the e2e cases are in the
        // walk whether or not `include` happens to name their folder.
        let covered = include.iter().any(|path| {
            let root = config.root.join(path);
            cases_root.starts_with(&root) || root.starts_with(&cases_root)
        });
        if !covered {
            return Ok(());
        }
    }

    let mut case_dirs = Vec::new();
    if scan_root.join("expected.exit").is_file() {
        case_dirs.push(scan_root);
    } else {
        for entry in fs::read_dir(&scan_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("expected.exit").is_file() {
                case_dirs.push(path);
            }
        }
    }
    case_dirs.sort_by_key(|path| sort_path_key(path));

    for dir in case_dirs {
        let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(id) = e2e_id_from_case_dir_name(config, name) else {
            continue;
        };
        let case = read_e2e_case(config, &dir)?;
        findings
            .declarations
            .entry(id.clone())
            .or_default()
            .push(Declaration {
                id,
                file: dir.clone(),
                line: 1,
                heading_level: 1,
                sections: BTreeMap::new(),
                duplicate_sections: Vec::new(),
                is_stub: false,
                defined_in: None,
                e2e_case: Some(case),
                title: Some(format!("e2e case `{name}`")),
                // §AR-scanner.2.4: an E2E case spans its manifest line only; its
                // obligations evaluate over the case's scanned files, not a body.
                body_start: 1,
                body_end: 1,
            });
    }
    Ok(())
}

/// Map an `e2e/cases/<name>/` directory name to its `E2E-<name>` `Id` under the
/// repo's `[id] format` (§AR-scanner.6, §FS-config.3.4).
fn e2e_id_from_case_dir_name(config: &Config, name: &str) -> Option<Id> {
    let after_kind_literal = literal_after_kind_placeholder(&config.id_format)?;
    let raw = format!("E2E{after_kind_literal}{name}");
    let (id, section) = parse_id_arg(&raw, &config.grammar).ok()?;
    if section.is_none() && id.kind == "E2E" {
        Some(id)
    } else {
        None
    }
}

/// The literal text between `{kind}` and the next placeholder in `[id] format`
/// (e.g. `-` in `{kind}-{slug}`) — the glue an `E2E-<dirname>` ID is reassembled
/// with (§AR-scanner.6).
fn literal_after_kind_placeholder(format: &str) -> Option<&str> {
    let marker = "{kind}";
    let start = format.find(marker)? + marker.len();
    let rest = &format[start..];
    let end = rest.find('{').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Inverse of `e2e_id_from_case_dir_name`: strip the `E2E` prefix off a rendered ID
/// to get the `e2e/cases/<name>/` directory `grund id` tells the author to create
/// (§FS-id.2, §AR-scanner.6).
fn e2e_case_dir_name(config: &Config, rendered: &str) -> String {
    let prefix = format!(
        "E2E{}",
        literal_after_kind_placeholder(&config.id_format).unwrap_or("-")
    );
    rendered
        .strip_prefix(&prefix)
        .unwrap_or(rendered)
        .to_string()
}

/// Read one e2e case directory into an `E2eCase` — `command.args` (defaulting to
/// `check`), `expected.exit`, `spec.refs`, and the recursive fixture file list —
/// the data `grund E2E-<name>` renders or checks (§FS-show.2.4, §FS-config.3.9).
fn read_e2e_case(config: &Config, dir: &Path) -> Result<E2eCase> {
    let command_args = dir.join("command.args");
    let args = if command_args.is_file() {
        fs::read_to_string(&command_args)?
            .split_whitespace()
            .map(str::to_string)
            .collect()
    } else {
        vec!["check".to_string()]
    };
    let expected_exit = fs::read_to_string(dir.join("expected.exit"))?
        .trim()
        .parse::<i32>()
        .with_context(|| format!("parse {}/expected.exit", format_path(dir)))?;
    let mut fixtures = Vec::new();
    collect_relative_fixture_files(dir, dir, &mut fixtures)?;
    fixtures.sort_by_key(|path| sort_path_key(path));
    let spec_refs = read_e2e_spec_refs(config, dir)?;
    Ok(E2eCase {
        dir: dir.to_path_buf(),
        args,
        expected_exit,
        fixtures,
        spec_refs,
    })
}

fn read_e2e_spec_refs(config: &Config, dir: &Path) -> Result<Vec<E2eSpecRef>> {
    let path = dir.join("spec.refs");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    Ok(text
        .lines()
        .filter_map(|line| e2e_spec_ref_from_line(config, line.trim()))
        .collect())
}

fn e2e_spec_ref_from_line(config: &Config, line: &str) -> Option<E2eSpecRef> {
    let token = line.split_whitespace().next()?;
    if token.is_empty() {
        return None;
    }
    let token = token.strip_prefix(&config.marker).unwrap_or(token);
    // §FS-workspace.6.1: an alias path may carry slashes, and an ID never does,
    // so the last separator is the boundary — the same split every other
    // consumer of a qualified token makes.
    let (namespace, id_text) = match token.rsplit_once('/') {
        Some((namespace, id_text)) if !namespace.is_empty() => {
            (Some(namespace.to_string()), id_text)
        }
        _ => (None, token),
    };
    if let Ok((id, _section)) = parse_id_arg(id_text, &config.grammar) {
        return Some(E2eSpecRef {
            namespace,
            kind: id.kind,
        });
    }
    let kind = id_text.split_once('-')?.0;
    if kind.is_empty() {
        return None;
    }
    Some(E2eSpecRef {
        namespace,
        kind: kind.to_string(),
    })
}

fn collect_relative_fixture_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_fixture_files(root, &path, files)?;
        } else {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

