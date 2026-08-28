/// `grund refs <ID>[.<section>] [--summary] [--format text|json]` — the reverse of an ID query:
/// list every place that cites the ID (§FS-refs.1, §FS-refs.2), scheme-aware where
/// a grep cannot be. Shares the scanner with `check` so the two never disagree on
/// what counts as a citation (§FS-refs.5). Empty results, including undeclared IDs
/// with no citations, exit `0` (§FS-refs.4).
///
/// Why a bad ID argument still gets the `[id] format` hint: a format that differs
/// from the `{kind}-{slug}` `grund` itself uses is the common surprise, so the
/// stumble is worth naming. The hint is withheld for an ambiguous shorthand, which
/// matched the format fine and already printed every candidate: repeating the
/// format there would send the reader to `grund config show` for a config that is
/// not wrong.
///
/// Why the target project is matched by alias string: workspace aliases are unique
/// and that is enforced at load, so string equality is the canonical "is this the
/// target project?" check — preferred over pointer identity so a future refactor
/// that clones a project does not silently drop local hits.
///
/// Why an empty result still leaves a note: an ID that is neither cited nor
/// declared was most likely fat-fingered. In workspace mode that hint points at
/// `--project <alias>` so the reader sees the namespace.
///
/// Where the output goes: the citation list is text and JSON alike on stdout, like
/// `grund list` / `grund cover` / ID queries, even though a line shares the
/// `path:line: <text>` shape `check` uses for diagnostics on stderr. In workspace
/// mode paths render relative to the workspace root so a `--summary` line points at
/// the same file regardless of which member it lives in, and each citation's
/// project alias is attached to the JSON object as `"project"` — the *citing*
/// project, not the target, which is the query argument.
fn command_refs(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: refs requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut section_override: Option<String> = None;
    let mut format_override: Option<String> = None;
    let mut summary = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--summary" => summary = true,
            "--section" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --section requires a value");
                    return ExitCode::from(2);
                }
                section_override = Some(args[idx].clone());
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
            other if id_arg.is_none() => id_arg = Some(other.to_string()),
            other => {
                if path_provided {
                    eprintln!("error: refs takes an ID and at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let Some(id_arg) = id_arg else {
        eprintln!("error: refs requires an ID");
        return ExitCode::from(2);
    };
    let context = match load_workspace_context(&path, path_provided) {
        Ok(context) => context,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let current_config = context
        .current_project()
        .map(|project| &project.config)
        .unwrap_or_else(|| context.render_config());
    let (alias, raw_id) = match split_qualified_id_arg(&id_arg) {
        Ok(parsed) => parsed,
        Err(err) => {
            // §FS-refs.1: an ID arg that does not match `[id] format` is a CLI-level
            // error (exit 2 — `refs` has no exit-`1` query-failure class, §FS-refs.4),
            // with the same hint the ID query gives for the same stumble (§FS-show.3).
            eprintln!("error: {err:#}");
            eprintln!(
                "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                current_config.id_format
            );
            return ExitCode::from(2);
        }
    };
    // §FS-workspace.8.2: pick the *target* project — the alias arg's project
    // in workspace mode, or the current project for an unqualified lookup.
    let target_project = match alias.as_deref() {
        Some(name) => match context.project_by_alias(name) {
            Some(project) => project,
            None => {
                eprintln!("error: unknown project alias `{name}`");
                if !context.workspace_loaded {
                    eprintln!(
                        "note: workspace aliases are defined in the root grund.toml under [workspace]"
                    );
                } else {
                    let known = context.aliases().join(", ");
                    eprintln!("known aliases: {known}");
                }
                return ExitCode::from(2);
            }
        },
        None => match context.current_project() {
            Some(project) => project,
            None => {
                eprintln!(
                    "error: unqualified ID requires a project alias when include_root = false"
                );
                let known = context.aliases().join(", ");
                if !known.is_empty() {
                    eprintln!("known aliases: {known}");
                }
                return ExitCode::from(2);
            }
        },
    };
    let target_alias = target_project.alias.as_str();
    let render_config = &target_project.config;
    // §FS-workspace.1: a qualified query's alias decides which grammar parses
    // the ID tail. This keeps `refs api/FS-001-session` aligned with the same
    // target namespace that `check` uses for `§api/FS-001-session`.
    let (id, inline_section) = match resolve_id_arg(raw_id, render_config, &target_project.findings)
    {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            // §FS-refs.4: the format hint belongs to an argument that does not
            // match `[id] format`; an ambiguous shorthand matched it fine and
            // already printed every candidate.
            if err.wants_format_hint() {
                eprintln!(
                    "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                    render_config.id_format
                );
            }
            return ExitCode::from(2);
        }
    };
    if section_override.is_some() && inline_section.is_some() {
        eprintln!("error: --section cannot be combined with an inline section");
        return ExitCode::from(2);
    }
    let section = section_override.or(inline_section);
    let format = format_override.unwrap_or_else(|| render_config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported refs format `{format}`");
        return ExitCode::from(2);
    }
    // §FS-workspace.8.2: a `§<alias>/<ID>` from sibling projects AND a
    // local `§<ID>` from inside `<alias>` both cite the same declaration.
    // Walk every project's citations and pick both forms.
    struct Hit<'a> {
        project: &'a WorkspaceProject,
        citation: &'a Citation,
    }
    let mut hits: Vec<Hit> = Vec::new();
    let mut had_scan_errors = false;
    for project in &context.projects {
        had_scan_errors |= !project.scan_errors.is_empty();
        // Workspace aliases are unique (§FS-workspace.3, enforced at load), so
        // equality on the alias string is the canonical "is this the target
        // project?" check.
        let is_target = project.alias == target_alias;
        for citation in &project.findings.citations {
            // Same-project local citation: counts when this project IS the
            // target. A `§<ID>` inside a different project resolves against
            // that project, not the target.
            let local_match = citation.namespace.is_none() && is_target;
            // Cross-project citation: an explicit `§<target_alias>/<ID>`
            // from anywhere in the workspace.
            let qualified_match = citation
                .namespace
                .as_deref()
                .map(|ns| ns == target_alias)
                .unwrap_or(false);
            if !(local_match || qualified_match) {
                continue;
            }
            if citation.id != id {
                continue;
            }
            if let Some(expected) = section.as_deref()
                && citation.section.as_deref() != Some(expected)
            {
                continue;
            }
            hits.push(Hit { project, citation });
        }
    }
    hits.sort_by(|a, b| {
        (sort_path_key(&a.citation.file), a.citation.line, a.citation.column).cmp(&(
            sort_path_key(&b.citation.file),
            b.citation.line,
            b.citation.column,
        ))
    });
    // §FS-refs.2: zero citations is a normal answer, not an error — but if the ID
    // is *also* undeclared, leave a breadcrumb on stderr without changing the
    // exit code.
    if hits.is_empty() && !target_project.findings.declarations.contains_key(&id) {
        if context.workspace_loaded && alias.is_some() {
            eprintln!(
                "note: {}/{} is neither declared nor cited — run `grund list --project {}` to see {}'s declared IDs",
                target_alias,
                render_id(render_config, &id),
                target_alias,
                target_alias
            );
        } else {
            eprintln!(
                "note: {} is neither declared nor cited — run `grund list` to see every declared ID",
                render_id(render_config, &id)
            );
        }
    }
    // §FS-refs.3: the citation list is the query's *result*, so it goes to stdout;
    // only the `note:` breadcrumb above stays on stderr. §FS-workspace.8.2: in
    // workspace mode paths render relative to the workspace root.
    let render_path = |project: &WorkspaceProject, path: &Path| -> String {
        if context.workspace_loaded {
            display_path(context.render_config(), path)
        } else {
            display_path(&project.config, path)
        }
    };
    if summary {
        let mut by_file: BTreeMap<PathBuf, (usize, BTreeSet<usize>)> = BTreeMap::new();
        for hit in &hits {
            let entry = by_file
                .entry(hit.citation.file.clone())
                .or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.insert(hit.citation.line);
        }
        // Each entry's citing project (the file is unique, so the project is
        // unique too — a file lives in exactly one project's tree, by the
        // boundary rule §FS-workspace.6).
        let project_for_file: BTreeMap<&PathBuf, &WorkspaceProject> = hits
            .iter()
            .map(|hit| (&hit.citation.file, hit.project))
            .collect();
        let mut entries = by_file.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(file, _)| {
            render_path(project_for_file[file], file)
        });
        if format == "json" {
            for (file, (count, lines)) in entries {
                let lines_json = lines
                    .iter()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let project = project_for_file[file];
                if context.workspace_loaded {
                    println!(
                        "{{\"project\":\"{}\",\"path\":\"{}\",\"count\":{},\"lines\":[{}]}}",
                        json_escape(&project.alias),
                        json_escape(&render_path(project, file)),
                        count,
                        lines_json
                    );
                } else {
                    println!(
                        "{{\"path\":\"{}\",\"count\":{},\"lines\":[{}]}}",
                        json_escape(&render_path(project, file)),
                        count,
                        lines_json
                    );
                }
            }
        } else {
            for (file, (count, lines)) in entries {
                let label = if lines.len() == 1 { "line" } else { "lines" };
                let lines_text = lines
                    .iter()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let project = project_for_file[file];
                println!(
                    "{}: {} ({} {})",
                    render_path(project, file),
                    count,
                    label,
                    lines_text
                );
            }
        }
    } else if format == "json" {
        for hit in &hits {
            let project_field = if context.workspace_loaded {
                format!("\"project\":\"{}\",", json_escape(&hit.project.alias))
            } else {
                String::new()
            };
            println!(
                "{{{}{}}}",
                project_field,
                citation_json_body(
                    &target_project.config,
                    hit.citation,
                    &render_path(hit.project, &hit.citation.file)
                )
            );
        }
    } else {
        for hit in &hits {
            println!(
                "{}:{}: {}",
                render_path(hit.project, &hit.citation.file),
                hit.citation.line,
                hit.citation.text
            );
        }
    }
    if !had_scan_errors {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-refs.4 / §FS-check.2): citations are real but
        // the view was incomplete. §FS-workspace.8.7: rendered against the run's
        // config, like the hit rows above via `render_path`, not the scanning project's.
        for project in &context.projects {
            for (file, message) in &project.scan_errors {
                eprintln!(
                    "error: {}: {}",
                    display_path(context.render_config(), file),
                    message
                );
            }
        }
        ExitCode::from(2)
    }
}

/// The shared body of a `refs --format json` per-citation object — everything
/// after the optional `"project":"…",` prefix. Pulled out so both the
/// workspace-aware and single-project code paths emit identical fields.
fn citation_json_body(config: &Config, citation: &Citation, rendered_path: &str) -> String {
    format!(
        "\"path\":\"{}\",\"line\":{},\"column\":{},\"id\":\"{}\",\"section\":{},\"marker\":{},\"text\":\"{}\"",
        json_escape(rendered_path),
        citation.line,
        citation.column,
        json_escape(&render_id(config, &citation.id)),
        citation
            .section
            .as_deref()
            .map(|section| format!("\"{}\"", json_escape(section)))
            .unwrap_or_else(|| "null".to_string()),
        citation.has_marker,
        json_escape(&citation.text)
    )
}
