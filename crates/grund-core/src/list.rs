/// `grund list [path] [--kind K[,K]...] [--unused] [--summary] [--format text|json]` — print every
/// declared ID with its home `path:line` and one-line title (§FS-list.1,
/// §FS-list.3), optionally filtered to one kind or to declarations nothing cites
/// (the same set as the §FS-check.4.1 warning). The discovery side of the loop:
/// how an agent finds the right `<ID>` before citing it (§FS-list.5).
fn command_list(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut kind_filter: BTreeSet<String> = BTreeSet::new();
    let mut project_filter: BTreeSet<String> = BTreeSet::new();
    let mut unused_only = false;
    let mut summary = false;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--unused" => unused_only = true,
            "--summary" => summary = true,
            "--kind" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --kind requires a value");
                    return ExitCode::from(2);
                }
                add_kind_filters(&mut kind_filter, &args[idx]);
            }
            other if other.starts_with("--kind=") => {
                add_kind_filters(&mut kind_filter, other.trim_start_matches("--kind="));
            }
            "--project" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --project requires a value");
                    return ExitCode::from(2);
                }
                add_project_filters(&mut project_filter, &args[idx]);
            }
            other if other.starts_with("--project=") => {
                add_project_filters(&mut project_filter, other.trim_start_matches("--project="));
            }
            other if other.starts_with("--format=") => {
                format_override = Some(other.trim_start_matches("--format=").to_string());
            }
            "--format" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --format requires a value");
                    return ExitCode::from(2);
                }
                format_override = Some(args[idx].clone());
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path_provided {
                    eprintln!("error: list takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let context = match load_workspace_context(&path, path_provided) {
        Ok(context) => context,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    // §FS-workspace.8.3: `--project` is only meaningful in workspace mode —
    // a member-local or standalone invocation has no alias namespace.
    if !project_filter.is_empty() && !context.workspace_loaded {
        eprintln!(
            "error: --project requires workspace mode (no [workspace] block discovered)"
        );
        return ExitCode::from(2);
    }
    for alias in &project_filter {
        if context.project_by_alias(alias).is_none() {
            eprintln!("error: unknown project alias `{alias}`");
            let known = context.aliases().join(", ");
            if !known.is_empty() {
                eprintln!("known aliases: {known}");
            }
            return ExitCode::from(2);
        }
    }
    // Pick the render context for the catalog's path/format columns: the
    // workspace root in workspace mode (so paths span members), otherwise
    // the only project.
    let config = context.render_config().clone();
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported list format `{format}`");
        return ExitCode::from(2);
    }
    // §FS-list.4: an unknown `--kind` is a CLI-level error. In workspace
    // mode kinds may differ across projects (each project has its own
    // `[[kinds]]`), so the validation widens — a kind only needs to exist
    // in at least one project in scope.
    for kind in &kind_filter {
        let exists = context
            .projects
            .iter()
            .filter(|project| project_filter.is_empty() || project_filter.contains(&project.alias))
            .any(|project| {
                project
                    .config
                    .kinds
                    .iter()
                    .any(|candidate| &candidate.prefix == kind)
            });
        if !exists {
            eprintln!("error: unknown kind `{kind}`");
            // Preserve each project's configured `[[kinds]]` order
            // (§FS-list.4 takes its hint from `[[kinds]]`, not the
            // alphabet); only deduplicate across projects.
            let mut known: Vec<String> = Vec::new();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for project in &context.projects {
                for k in &project.config.kinds {
                    if seen.insert(k.prefix.clone()) {
                        known.push(k.prefix.clone());
                    }
                }
            }
            eprintln!("known kinds: {}", known.join(", "));
            return ExitCode::from(2);
        }
    }

    struct Entry<'a> {
        project_alias: &'a str,
        project_config: &'a Config,
        id: &'a Id,
        home: &'a Declaration,
        duplicate: bool,
        refs: usize,
    }
    // §FS-workspace.8.3: each citation belongs to exactly one target
    // project — its `namespace` when qualified, the citing project when
    // not — so a single pass over every project's citations builds the
    // per-target lookup. Doing this once up front (rather than re-walking
    // every project's citations per target) keeps the count linear in the
    // total citation set, not quadratic in the project count.
    let mut ref_counts_by_alias: BTreeMap<&str, BTreeMap<&Id, usize>> = BTreeMap::new();
    // §FS-list.1 / §DF-index-not-an-inbound-citation: `--unused` counts the same
    // citations minus each kind's own index entries. `refs` below stays the total
    // `grund refs` lists (§FS-list.3.2) — two counts of "citations of this ID"
    // that differed by which command printed them would be worse than one that
    // needs a sentence of explanation.
    let mut used_counts_by_alias: BTreeMap<&str, BTreeMap<&Id, usize>> = BTreeMap::new();
    for source in &context.projects {
        let index_entries = KindIndexEntries::new(&source.findings, &source.config);
        for citation in &source.findings.citations {
            let target_alias: &str = match &citation.namespace {
                Some(ns) => ns.as_str(),
                None => source.alias.as_str(),
            };
            *ref_counts_by_alias
                .entry(target_alias)
                .or_default()
                .entry(&citation.id)
                .or_insert(0) += 1;
            if !index_entries.is_index_entry(citation) {
                *used_counts_by_alias
                    .entry(target_alias)
                    .or_default()
                    .entry(&citation.id)
                    .or_insert(0) += 1;
            }
        }
    }
    let empty_ref_counts: BTreeMap<&Id, usize> = BTreeMap::new();
    let mut entries: Vec<Entry> = Vec::new();
    let mut had_scan_errors = false;
    for project in &context.projects {
        if !project_filter.is_empty() && !project_filter.contains(&project.alias) {
            continue;
        }
        had_scan_errors |= !project.scan_errors.is_empty();
        // Per-project reference counts. A `<§><alias>/<ID>` citation
        // targets `<alias>`'s declaration, so it must be attributed to the
        // *target* project's ref count, not the citing project's — that
        // lets a workspace-root `grund list --unused` see members'
        // declarations that only sibling projects cite.
        let ref_counts: &BTreeMap<&Id, usize> = ref_counts_by_alias
            .get(project.alias.as_str())
            .unwrap_or(&empty_ref_counts);
        let used_counts: &BTreeMap<&Id, usize> = used_counts_by_alias
            .get(project.alias.as_str())
            .unwrap_or(&empty_ref_counts);
        for (id, decls) in &project.findings.declarations {
            if !kind_filter.is_empty() && !kind_filter.contains(&id.kind) {
                continue;
            }
            let refs = ref_counts.get(id).copied().unwrap_or(0);
            if unused_only && used_counts.get(id).copied().unwrap_or(0) > 0 {
                continue;
            }
            // §FS-list.1 / §FS-check.4.1: `--unused` skips E2E cases by
            // default; opting back in requires explicit `--kind E2E`.
            if unused_only && id.kind == "E2E" && !kind_filter.contains("E2E") {
                continue;
            }
            let mut homes: Vec<&Declaration> = decls
                .iter()
                .filter(|decl| !is_stub_for_inline_decl(&project.config.root, decl, decls))
                .collect();
            homes.sort_by(|a, b| {
                (sort_path_key(&a.file), a.line).cmp(&(sort_path_key(&b.file), b.line))
            });
            let duplicate = homes.len() > 1;
            for home in homes {
                entries.push(Entry {
                    project_alias: project.alias.as_str(),
                    project_config: &project.config,
                    id,
                    home,
                    duplicate,
                    refs,
                });
            }
        }
    }
    // Sort entries: in workspace mode, by alias then ID; otherwise by ID.
    // `findings.declarations` is already a BTreeMap so within a project
    // entries come out sorted; the project-prefix sort here is what makes
    // the workspace catalog deterministic across runs.
    if context.workspace_loaded {
        entries.sort_by(|a, b| {
            (a.project_alias, a.id, sort_path_key(&a.home.file), a.home.line).cmp(&(
                b.project_alias,
                b.id,
                sort_path_key(&b.home.file),
                b.home.line,
            ))
        });
    }

    // §FS-workspace.8.3: in workspace mode the ID column is always
    // qualified — even under `--project`. The helper renders `<ID>` outside
    // workspace mode and `<alias>/<ID>` inside it.
    let render_qualified = |entry: &Entry| -> String {
        if context.workspace_loaded {
            format!(
                "{}/{}",
                entry.project_alias,
                render_id(entry.project_config, entry.id)
            )
        } else {
            render_id(entry.project_config, entry.id)
        }
    };

    if summary {
        if context.workspace_loaded {
            // §FS-workspace.8.3: per-(project, kind) summary in workspace
            // mode. Project order follows `context.projects` (root first,
            // then members in member-glob order); kinds inside each project
            // follow the project's configured `[[kinds]]` order.
            let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
            for entry in &entries {
                *counts
                    .entry((entry.project_alias.to_string(), entry.id.kind.clone()))
                    .or_insert(0) += 1;
            }
            for project in &context.projects {
                if !project_filter.is_empty() && !project_filter.contains(&project.alias) {
                    continue;
                }
                for kind in &project.config.kinds {
                    let count = counts
                        .get(&(project.alias.clone(), kind.prefix.clone()))
                        .copied()
                        .unwrap_or(0);
                    if count == 0 {
                        continue;
                    }
                    if format == "json" {
                        println!(
                            "{{\"project\":\"{}\",\"kind\":\"{}\",\"title\":\"{}\",\"home\":\"{}\",\"count\":{}}}",
                            json_escape(&project.alias),
                            json_escape(&kind.prefix),
                            json_escape(kind.title.as_deref().unwrap_or("Declaration")),
                            json_escape(kind.folder.as_deref().unwrap_or("")),
                            count
                        );
                    } else {
                        println!(
                            "{:<10}  {:<4}  {:>3}  {}",
                            project.alias,
                            kind.prefix,
                            count,
                            kind.folder.as_deref().unwrap_or("")
                        );
                    }
                }
            }
        } else {
            let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
            for entry in &entries {
                *counts.entry(&entry.id.kind).or_insert(0) += 1;
            }
            if format == "json" {
                for kind in &config.kinds {
                    let count = counts.get(kind.prefix.as_str()).copied().unwrap_or(0);
                    if count == 0 {
                        continue;
                    }
                    println!(
                        "{{\"kind\":\"{}\",\"title\":\"{}\",\"home\":\"{}\",\"count\":{}}}",
                        json_escape(&kind.prefix),
                        json_escape(kind.title.as_deref().unwrap_or("Declaration")),
                        json_escape(kind.folder.as_deref().unwrap_or("")),
                        count
                    );
                }
            } else {
                for kind in &config.kinds {
                    let count = counts.get(kind.prefix.as_str()).copied().unwrap_or(0);
                    if count == 0 {
                        continue;
                    }
                    println!(
                        "{:<4}  {:>3}  {}",
                        kind.prefix,
                        count,
                        kind.folder.as_deref().unwrap_or("")
                    );
                }
            }
        }
    } else if format == "json" {
        // One compact object per line, no interior spaces. The shipped
        // `grund-open` resolver and the VS Code extension parse this by
        // substring/regex (`"id":"…"`, `"path":"…"`, `"line":N`), so keep the
        // field shape stable (§FS-integrations.3).
        for entry in &entries {
            let project_field = if context.workspace_loaded {
                format!("\"project\":\"{}\",", json_escape(entry.project_alias))
            } else {
                String::new()
            };
            println!(
                "{{{}\"id\":\"{}\",\"kind\":\"{}\",\"path\":\"{}\",\"line\":{},\"title\":{},\"stub\":{},\"defines\":{},\"refs\":{},\"duplicate\":{}}}",
                project_field,
                json_escape(&render_qualified(entry)),
                json_escape(&entry.id.kind),
                json_escape(&display_path(&config, &entry.home.file)),
                entry.home.line,
                entry
                    .home
                    .title
                    .as_deref()
                    .map(|title| format!("\"{}\"", json_escape(title)))
                    .unwrap_or_else(|| "null".to_string()),
                entry.home.is_stub,
                entry
                    .home
                    .defined_in
                    .as_ref()
                    .map(|target| format!("\"{}\"", json_escape(&format_path(target))))
                    .unwrap_or_else(|| "null".to_string()),
                entry.refs,
                entry.duplicate,
            );
        }
    } else {
        let id_width = entries
            .iter()
            .map(|entry| render_qualified(entry).chars().count())
            .max()
            .unwrap_or(0)
            .min(40);
        for entry in &entries {
            let id_text = render_qualified(entry);
            let location = format!(
                "{}:{}",
                display_path(&config, &entry.home.file),
                entry.home.line
            );
            let mut note = if entry.home.is_stub {
                entry
                    .home
                    .defined_in
                    .as_ref()
                    .map(|target| format!("→ {}", format_path(target)))
                    .unwrap_or_default()
            } else {
                entry.home.title.clone().unwrap_or_default()
            };
            if entry.duplicate {
                if note.is_empty() {
                    note = "(duplicate declaration — grund check)".to_string();
                } else {
                    note.push_str("  (duplicate declaration — grund check)");
                }
            }
            if note.is_empty() {
                println!("{id_text:<id_width$}  {location}");
            } else {
                println!("{id_text:<id_width$}  {location}  {note}");
            }
        }
    }

    if !had_scan_errors {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-check.2): the listed declarations are real
        // but the view of the tree was incomplete, so the catalog may be short.
        for project in &context.projects {
            for (file, message) in &project.scan_errors {
                eprintln!("error: {}: {}", display_path(&project.config, file), message);
            }
        }
        ExitCode::from(2)
    }
}

fn add_project_filters(project_filter: &mut BTreeSet<String>, raw: &str) {
    for alias in raw.split(',') {
        if !alias.is_empty() {
            project_filter.insert(alias.to_string());
        }
    }
}

fn add_kind_filters(kind_filter: &mut BTreeSet<String>, raw: &str) {
    for kind in raw.split(',') {
        kind_filter.insert(kind.to_string());
    }
}
