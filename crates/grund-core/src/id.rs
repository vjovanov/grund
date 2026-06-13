fn command_id(args: &[String]) -> ExitCode {
    let mut positional = Vec::new();
    let mut width = 3usize;
    let mut format = "text".to_string();
    let mut explain = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--explain" => explain = true,
            "--width" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --width requires a value");
                    return ExitCode::from(2);
                }
                width = match args[idx].parse::<usize>() {
                    Ok(value) => value,
                    Err(_) => {
                        eprintln!("error: --width requires a positive integer");
                        return ExitCode::from(2);
                    }
                };
            }
            other if other.starts_with("--format=") => {
                format = other.trim_start_matches("--format=").to_string();
            }
            "--format" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --format requires a value");
                    return ExitCode::from(2);
                }
                format = args[idx].clone();
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => positional.push(other.to_string()),
        }
        idx += 1;
    }
    if positional.len() < 2 {
        eprintln!("error: id requires <KIND> and <title>");
        return ExitCode::from(2);
    }
    if positional.len() > 3 {
        eprintln!("error: id takes <KIND>, <title>, and at most one path argument");
        return ExitCode::from(2);
    }
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported id format `{format}`");
        return ExitCode::from(2);
    }
    let kind = &positional[0];
    let title = &positional[1];
    let path_provided = positional.get(2).is_some();
    let path = positional
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let config = match resolve_workspace_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let kind_config = match config
        .kinds
        .iter()
        .find(|candidate| &candidate.prefix == kind)
    {
        Some(kind_config) => kind_config,
        None => {
            eprintln!("error: unknown kind `{kind}`");
            eprintln!("known kinds: {}", kind_prefixes(&config.kinds).join(", "));
            return ExitCode::from(2);
        }
    };
    let slug = slugify_title(title, &config.slug_pattern);
    if slug.is_empty() {
        eprintln!("title produces empty slug after normalization: \"{title}\"");
        return ExitCode::FAILURE;
    }
    let findings = match scan_tree_strict(&config, Some(&path), path_provided) {
        Ok(findings) => findings,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let uses_number = config.id_format.contains("{number}");
    let number = if uses_number {
        let max = findings
            .declarations
            .keys()
            .filter(|id| &id.kind == kind)
            .filter_map(|id| id.num)
            .max()
            .unwrap_or(0);
        Some(max + 1)
    } else {
        None
    };
    let id = Id {
        kind: kind.clone(),
        num: number,
        slug: if config.id_format.contains("{slug}") {
            Some(slug.clone())
        } else {
            None
        },
    };
    if let Some(decls) = findings.declarations.get(&id)
        && let Some(decl) = decls.first()
    {
        eprintln!(
            "proposed ID `{}` already declared at {}:{}",
            format_id(&id, &config, width),
            display_path(&config, &decl.file),
            decl.line
        );
        return ExitCode::FAILURE;
    }
    let rendered = format_id(&id, &config, width);
    if format == "json" {
        let folder = kind_config.folder.as_deref().unwrap_or("");
        let file = kind_config.file.as_deref().unwrap_or("");
        println!(
            "{{\"id\":\"{}\",\"kind\":\"{}\",\"number\":{},\"slug\":\"{}\",\"folder\":\"{}\",\"file\":\"{}\"}}",
            json_escape(&rendered),
            json_escape(kind),
            number
                .map(|number| number.to_string())
                .unwrap_or_else(|| "null".to_string()),
            json_escape(&slug),
            json_escape(folder),
            json_escape(file)
        );
    } else {
        println!("{rendered}");
        if explain {
            match kind_config.folder.as_deref() {
                Some(folder) if kind == "E2E" => {
                    let case_dir = e2e_case_dir_name(&config, &rendered);
                    eprintln!(
                        "next: create the case directory at {folder}/{case_dir}/ with expected.exit and fixtures, then cite it as §{rendered}"
                    );
                }
                Some(folder) => eprintln!(
                    "next: write the declaration at {folder}/{rendered}.md  (H1: `# {rendered}: <one-line statement>`), then cite it as §{rendered}"
                ),
                None if kind_config.file.is_some() => {
                    let file = kind_config.file.as_deref().unwrap();
                    let (heading_name, heading_marker) = if kind == "GRUND" {
                        ("H1", "#")
                    } else {
                        ("H2", "##")
                    };
                    eprintln!(
                        "next: add the declaration to {file}  ({heading_name}: `{heading_marker} {rendered}: <one-line statement>`), then cite it as §{rendered}"
                    );
                }
                None => eprintln!(
                    "next: write the declaration with H1 `# {rendered}: <one-line statement>`, then cite it as §{rendered}"
                ),
            }
        }
    }
    ExitCode::SUCCESS
}

/// The repeating character class of a slug pattern — the last `[...]` bracket
/// expression in `slug_pattern` (e.g. `[a-z0-9-]` from `[a-z0-9][a-z0-9-]*`) —
/// used when slugifying a `grund id` title so the result fits the configured
/// `[id] slug_pattern` (§FS-id.3, §FS-config.3.2). Falls back to the canonical
/// default if the pattern has no bracket expression.
fn slug_char_class(slug_pattern: &str) -> String {
    if let Some(end) = slug_pattern.rfind(']')
        && let Some(start) = slug_pattern[..end].rfind('[')
    {
        return slug_pattern[start..=end].to_string();
    }
    "[a-z0-9-]".to_string()
}

/// Derive a slug from a `grund id` title (§FS-id.3).
fn slugify_title(title: &str, slug_pattern: &str) -> String {
    // §FS-id.3: NFKD-normalize, drop combining marks, lower-case to ASCII, then
    // replace every run of characters outside the configured slug character class
    // with a single `-`; trim, collapse, truncate to 60 at a `-` boundary.
    let class = slug_char_class(slug_pattern);
    let valid = Regex::new(&format!("^(?:{class})$"))
        .unwrap_or_else(|_| Regex::new("^(?:[a-z0-9-])$").unwrap());
    let mut buf = [0u8; 4];
    let mut out = String::new();
    let mut last_dash = false;
    for ch in title.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii() && valid.is_match(lower.encode_utf8(&mut buf)) {
            out.push(lower);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    if out.len() > 60 {
        let mut truncated = out[..60].to_string();
        if let Some(cut) = truncated.rfind('-') {
            truncated.truncate(cut);
        }
        out = truncated;
    }
    out
}

/// Render an `Id` back to text under the repo's `[id] format`, zero-padding the
/// number to `width` (§FS-config.3.2, §FS-id.2 — the form `grund id` prints and
/// every report uses).
fn format_id(id: &Id, config: &Config, width: usize) -> String {
    let mut rendered = config.id_format.clone();
    rendered = rendered.replace("{kind}", &id.kind);
    if let Some(number) = id.num {
        rendered = rendered.replace("{number}", &format!("{number:0width$}"));
    }
    if let Some(slug) = &id.slug {
        rendered = rendered.replace("{slug}", slug);
    }
    rendered
}

/// Render an `Id` at the default 3-digit number width — the form used everywhere
/// `grund` prints an ID in a report, listing, or message (§FS-config.3.2).
fn render_id(config: &Config, id: &Id) -> String {
    format_id(id, config, 3)
}
