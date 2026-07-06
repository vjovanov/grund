fn command_agent_setup_instructions(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("error: agent-setup-instructions takes no arguments");
        return ExitCode::from(2);
    }
    print!("{}", canonical_template_text(AGENT_SETUP_INSTRUCTIONS));
    ExitCode::SUCCESS
}

fn command_check(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut format_override = None;
    let mut require_grounding = false;
    let mut include_suggestions = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
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
            "--require-grounding" => require_grounding = true,
            "--suggestions" => include_suggestions = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path_provided {
                    eprintln!("error: check takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    if let Some(format) = &format_override
        && !matches!(format.as_str(), "text" | "json")
    {
        eprintln!("error: unsupported check format `{format}`");
        return ExitCode::from(2);
    }
    let output = match check_with_opts(CheckOpts {
        path,
        path_provided,
        require_grounding,
        include_suggestions,
    }) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = format_override.unwrap_or(output.output_format);
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported check format `{format}`");
        return ExitCode::from(2);
    }
    if format == "json" {
        render_check_json(&output.report);
    } else {
        render_check_text(&output.report);
    }
    if output.had_scan_errors {
        ExitCode::from(2)
    } else if output.report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn sorted_findings(report: &Report) -> Vec<(&'static str, &Finding)> {
    // §FS-check.2.3: `report.suggestions` is populated only when the caller
    // asked for them, so chaining it unconditionally is a no-op otherwise.
    let mut findings = report
        .warnings
        .iter()
        .map(|finding| ("warning", finding))
        .chain(report.errors.iter().map(|finding| ("error", finding)))
        .chain(
            report
                .suggestions
                .iter()
                .map(|finding| ("suggestion", finding)),
        )
        .collect::<Vec<_>>();
    findings.sort_by(|(_, a), (_, b)| {
        (
            a.path.as_deref(),
            a.line.unwrap_or(0),
            a.message.as_str(),
        )
            .cmp(&(
                b.path.as_deref(),
                b.line.unwrap_or(0),
                b.message.as_str(),
            ))
    });
    findings
}

fn render_check_text(report: &Report) {
    // §FS-check.2.3: suggestions never suppress `success`, but when present
    // (caller passed --suggestions) they are printed, so the marker only stands
    // in for a run with nothing at all to show.
    if report.errors.is_empty() && report.warnings.is_empty() && report.suggestions.is_empty() {
        println!("success");
        return;
    }
    for (severity, finding) in sorted_findings(report) {
        let line = match (finding.path.as_deref(), finding.line) {
            (Some(path), Some(line)) => format!("{path}:{line}: {}", finding.message),
            (Some(path), None) => format!("{severity}: {path}: {}", finding.message),
            _ => format!("{severity}: {}", finding.message),
        };
        if finding.line.is_some() {
            println!("{line}");
        } else {
            eprintln!("{line}");
        }
    }
}

fn render_check_json(report: &Report) {
    for (severity, finding) in sorted_findings(report) {
        let object = render_finding_json(severity, finding);
        if finding.line.is_some() {
            println!("{object}");
        } else {
            eprintln!("{object}");
        }
    }
}

fn render_finding_json(severity: &str, finding: &Finding) -> String {
    let path = finding
        .path
        .as_deref()
        .map(|path| format!("\"{}\"", json_escape(path)))
        .unwrap_or_else(|| "null".to_string());
    let line = finding
        .line
        .map(|line| line.to_string())
        .unwrap_or_else(|| "null".to_string());
    let sites = if finding.sites.is_empty() {
        "null".to_string()
    } else {
        let values = finding
            .sites
            .iter()
            .map(|site| {
                format!(
                    "{{\"path\":\"{}\",\"line\":{}}}",
                    json_escape(&site.path),
                    site.line
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", values)
    };
    // §FS-errors.5: a suggestion carries `"channel":"suggestion"` rather than a
    // `"severity"`, so the frozen `{error, warning}` set stays intact.
    let tag = if severity == "suggestion" {
        "\"channel\":\"suggestion\"".to_string()
    } else {
        format!("\"severity\":\"{severity}\"")
    };
    format!(
        "{{{},\"path\":{},\"line\":{},\"code\":\"{}\",\"message\":\"{}\",\"sites\":{}}}",
        tag,
        path,
        line,
        finding.code,
        json_escape(&finding.message),
        sites
    )
}

fn command_show(args: &[String]) -> ExitCode {
    command_show_impl(args, false)
}

fn command_show_default(args: &[String]) -> ExitCode {
    command_show_impl(args, true)
}

fn looks_like_subcommand_typo(arg: &str) -> bool {
    !arg.is_empty() && !arg.contains('-') && !arg.contains('/') && !arg.contains('.')
}

fn command_show_impl(args: &[String], default_invocation: bool) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut mode = ShowMode::Lead;
    let mut mode_flag: Option<&'static str> = None;
    let mut section_override = None;
    let mut format = "text".to_string();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--brief" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --brief cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--brief");
                mode = ShowMode::Brief;
            }
            "--toc" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --toc cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--toc");
                mode = ShowMode::Toc;
            }
            "--full" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --full cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--full");
                mode = ShowMode::Full;
            }
            other if other.starts_with("--format=") => {
                format = other.trim_start_matches("--format=").to_string();
            }
            "--section" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --section requires a value");
                    return ExitCode::from(2);
                }
                section_override = Some(args[idx].clone());
            }
            "--path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --path requires a value");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(&args[idx]);
                path_provided = true;
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
            other if id_arg.is_none() => id_arg = Some(other.to_string()),
            other => {
                if path_provided {
                    eprintln!("error: show takes an ID and at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let Some(id_arg) = id_arg else {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    };
    let Some(show_format) = parse_show_format(&format) else {
        eprintln!("error: unsupported show format `{format}`");
        return ExitCode::from(2);
    };
    match show_with_scope(
        &id_arg,
        ShowOpts {
            path: path.clone(),
            section: section_override,
            mode,
            format: show_format,
        },
        path_provided,
    ) {
        Ok(output) => {
            if format == "json" {
                println!("{}", output.json.unwrap_or_default());
            } else {
                print!("{}", output.body);
            }
            ExitCode::SUCCESS
        }
        Err(err) => render_show_error(&id_arg, &path, default_invocation, &format, err),
    }
}

fn render_show_error(
    id_arg: &str,
    path: &Path,
    default_invocation: bool,
    format: &str,
    err: impl std::fmt::Display,
) -> ExitCode {
    let message = format!("{err:#}");
    if default_invocation && message.starts_with("unknown project alias") && Path::new(id_arg).exists() {
        eprintln!("invalid ID `{id_arg}`");
        print_id_format_hint(path);
        eprintln!("hint: run `grund check {id_arg}` to validate a path");
        return ExitCode::FAILURE;
    }
    let query_error_code = show_query_error_code(&message);
    if format == "json"
        && let Some(code) = query_error_code
    {
        print_bare_query_json(code, &message);
        return ExitCode::FAILURE;
    }
    if query_error_code.is_none() {
        eprintln!("error: {message}");
        if default_invocation {
            eprintln!("hint: run `grund check {id_arg}` to validate a path");
        }
        return ExitCode::from(2);
    }
    eprintln!("{message}");
    if message.starts_with("invalid ID") {
        print_id_format_hint(path);
        if default_invocation {
            eprintln!("hint: run `grund check {id_arg}` to validate a path");
            if looks_like_subcommand_typo(id_arg) {
                eprintln!("hint: run `grund --help` for the list of subcommands");
            }
        }
    } else if message.starts_with("ID not found:") {
        eprintln!(
            "hint: run `grund list` to see every declared ID, or `grund id <KIND> \"<title>\"` to propose a new one"
        );
    } else if message.starts_with("section not found:") {
        let base_id = effective_config(path)
            .ok()
            .and_then(|config| {
                id_arg
                    .rsplit_once(&config.section_separator)
                    .map(|(base, _)| base.to_string())
            })
            .unwrap_or_else(|| id_arg.to_string());
        eprintln!("hint: run `grund {base_id} --toc` to print the lead with the section map");
    }
    ExitCode::FAILURE
}

fn print_bare_query_json(code: &'static str, message: &str) {
    eprintln!(
        "{{\"severity\":\"error\",\"path\":null,\"line\":null,\"code\":\"{}\",\"message\":\"{}\",\"sites\":null}}",
        code,
        json_escape(message)
    );
}

fn print_id_format_hint(path: &Path) {
    if let Ok(config) = effective_config(path) {
        eprintln!(
            "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
            config.id_format
        );
    }
}

fn show_query_error_code(message: &str) -> Option<&'static str> {
    if message.starts_with("ID not found:") {
        Some("not-found")
    } else if message.starts_with("section not found:") {
        Some("missing-section")
    } else if message.starts_with("invalid ID") {
        Some("invalid-id")
    } else if message.starts_with("ambiguous ID:") {
        Some("ambiguous")
    } else if message.starts_with("broken stub:") {
        Some("broken-stub")
    } else {
        None
    }
}

fn parse_show_format(format: &str) -> Option<ShowFormat> {
    match format {
        "text" => Some(ShowFormat::Text),
        "md" => Some(ShowFormat::Markdown),
        "json" => Some(ShowFormat::Json),
        _ => None,
    }
}

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
    let output = match list(ListOpts {
        path,
        path_provided,
        kind_filter,
        project_filter,
        unused_only,
    }) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = match command_output_format("list", &output.output_format, format_override) {
        Ok(format) => format,
        Err(code) => return code,
    };

    if summary {
        render_list_summary(&output.summaries, output.workspace, &format);
    } else if format == "json" {
        for entry in &output.entries {
            println!("{}", render_list_entry_json(entry));
        }
    } else {
        render_list_text(&output.entries);
    }

    exit_after_scan_errors(&output.scan_errors)
}

fn render_list_summary(summaries: &[grund_core::ListSummary], workspace: bool, format: &str) {
    for summary in summaries {
        if workspace {
            let project = summary.project.as_deref().unwrap_or("");
            if format == "json" {
                println!(
                    "{{\"project\":\"{}\",\"kind\":\"{}\",\"title\":\"{}\",\"home\":\"{}\",\"count\":{}}}",
                    json_escape(project),
                    json_escape(&summary.kind),
                    json_escape(&summary.title),
                    json_escape(&summary.home),
                    summary.count
                );
            } else {
                println!(
                    "{:<10}  {:<4}  {:>3}  {}",
                    project, summary.kind, summary.count, summary.home
                );
            }
        } else if format == "json" {
            println!(
                "{{\"kind\":\"{}\",\"title\":\"{}\",\"home\":\"{}\",\"count\":{}}}",
                json_escape(&summary.kind),
                json_escape(&summary.title),
                json_escape(&summary.home),
                summary.count
            );
        } else {
            println!(
                "{:<4}  {:>3}  {}",
                summary.kind, summary.count, summary.home
            );
        }
    }
}

fn render_list_entry_json(entry: &ListEntry) -> String {
    let project_field = entry
        .project
        .as_deref()
        .map(|project| format!("\"project\":\"{}\",", json_escape(project)))
        .unwrap_or_default();
    format!(
        "{{{}\"id\":\"{}\",\"kind\":\"{}\",\"path\":\"{}\",\"line\":{},\"title\":{},\"stub\":{},\"defines\":{},\"refs\":{},\"duplicate\":{}}}",
        project_field,
        json_escape(&entry.id),
        json_escape(&entry.kind),
        json_escape(&entry.path),
        entry.line,
        entry
            .title
            .as_deref()
            .map(|title| format!("\"{}\"", json_escape(title)))
            .unwrap_or_else(|| "null".to_string()),
        entry.stub,
        entry
            .defines
            .as_deref()
            .map(|target| format!("\"{}\"", json_escape(target)))
            .unwrap_or_else(|| "null".to_string()),
        entry.refs,
        entry.duplicate,
    )
}

fn render_list_text(entries: &[ListEntry]) {
    let id_width = entries
        .iter()
        .map(|entry| entry.id.chars().count())
        .max()
        .unwrap_or(0)
        .min(40);
    for entry in entries {
        let location = format!("{}:{}", entry.path, entry.line);
        let mut note = if entry.stub {
            entry
                .defines
                .as_ref()
                .map(|target| format!("→ {target}"))
                .unwrap_or_default()
        } else {
            entry.title.clone().unwrap_or_default()
        };
        if entry.duplicate {
            if note.is_empty() {
                note = "(duplicate declaration — grund check)".to_string();
            } else {
                note.push_str("  (duplicate declaration — grund check)");
            }
        }
        if note.is_empty() {
            println!("{:<id_width$}  {location}", entry.id);
        } else {
            println!("{:<id_width$}  {location}  {note}", entry.id);
        }
    }
}

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
    let output = match refs(RefsOpts {
        path,
        path_provided,
        id: id_arg,
        section: section_override,
    }) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = match command_output_format("refs", &output.output_format, format_override) {
        Ok(format) => format,
        Err(code) => return code,
    };
    if let Some(note) = &output.note {
        eprintln!("note: {note}");
    }
    if summary {
        render_refs_summary(&output.hits, output.workspace, &format);
    } else if format == "json" {
        for hit in &output.hits {
            println!("{}", render_ref_hit_json(hit, output.workspace));
        }
    } else {
        for hit in &output.hits {
            println!("{}:{}: {}", hit.path, hit.line, hit.text);
        }
    }

    exit_after_scan_errors(&output.scan_errors)
}

fn render_refs_summary(hits: &[RefHit], workspace: bool, format: &str) {
    let mut by_file: BTreeMap<String, (Option<String>, usize, BTreeSet<usize>)> = BTreeMap::new();
    for hit in hits {
        let entry = by_file
            .entry(hit.path.clone())
            .or_insert_with(|| (hit.project.clone(), 0, BTreeSet::new()));
        entry.1 += 1;
        entry.2.insert(hit.line);
    }
    for (path, (project, count, lines)) in by_file {
        if format == "json" {
            let lines_json = lines
                .iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join(",");
            if workspace {
                println!(
                    "{{\"project\":\"{}\",\"path\":\"{}\",\"count\":{},\"lines\":[{}]}}",
                    json_escape(project.as_deref().unwrap_or("")),
                    json_escape(&path),
                    count,
                    lines_json
                );
            } else {
                println!(
                    "{{\"path\":\"{}\",\"count\":{},\"lines\":[{}]}}",
                    json_escape(&path),
                    count,
                    lines_json
                );
            }
        } else {
            let label = if lines.len() == 1 { "line" } else { "lines" };
            let lines_text = lines
                .iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("{path}: {count} ({label} {lines_text})");
        }
    }
}

fn render_ref_hit_json(hit: &RefHit, workspace: bool) -> String {
    let project_field = if workspace {
        format!(
            "\"project\":\"{}\",",
            json_escape(hit.project.as_deref().unwrap_or(""))
        )
    } else {
        String::new()
    };
    format!(
        "{{{}\"path\":\"{}\",\"line\":{},\"column\":{},\"id\":\"{}\",\"section\":{},\"marker\":{},\"text\":\"{}\"}}",
        project_field,
        json_escape(&hit.path),
        hit.line,
        hit.column,
        json_escape(&hit.id),
        hit.section
            .as_deref()
            .map(|section| format!("\"{}\"", json_escape(section)))
            .unwrap_or_else(|| "null".to_string()),
        hit.marker,
        json_escape(&hit.text)
    )
}

fn add_kind_filters(filters: &mut BTreeSet<String>, raw: &str) {
    for value in raw.split(',') {
        filters.insert(value.to_string());
    }
}

fn add_project_filters(filters: &mut BTreeSet<String>, raw: &str) {
    for value in raw.split(',') {
        if !value.is_empty() {
            filters.insert(value.to_string());
        }
    }
}

fn command_init(args: &[String]) -> ExitCode {
    let mut path: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut docs = false;
    let mut force = false;
    let mut dry_run = false;
    let mut agent_selection = InitAgentEntrypointSelection::default();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--docs" => docs = true,
            "--force" => force = true,
            "--dry-run" => dry_run = true,
            "--agents-md" => agent_selection.canonical = true,
            "--claude" => agent_selection.claude = true,
            "--gemini" => agent_selection.gemini = true,
            "--copilot" => agent_selection.copilot = true,
            "--cursor" => agent_selection.cursor = true,
            "--windsurf" => agent_selection.windsurf = true,
            "--zed" => agent_selection.zed = true,
            "--name" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --name requires a value");
                    return ExitCode::from(2);
                }
                name = Some(args[idx].clone());
            }
            other if other.starts_with("--name=") => {
                name = Some(other.trim_start_matches("--name=").to_string());
            }
            "--description" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --description requires a value");
                    return ExitCode::from(2);
                }
                description = Some(args[idx].clone());
            }
            other if other.starts_with("--description=") => {
                description = Some(other.trim_start_matches("--description=").to_string());
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("error: init takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(other));
            }
        }
        idx += 1;
    }
    let output = match init(InitOpts {
        target: path.unwrap_or_else(|| PathBuf::from(".")),
        name,
        description,
        docs,
        force,
        dry_run,
        agent_selection,
    }) {
        Ok(output) => output,
        Err(err) => {
            render_init_output(&err.output);
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    render_init_output(&output);
    ExitCode::SUCCESS
}

fn render_init_output(output: &InitOutput) {
    for event in &output.events {
        eprintln!("{} {}", event.verb, event.path);
    }
    if let Some(next) = &output.next {
        render_init_next(next);
    }
}

fn render_init_next(next: &InitNext) {
    eprintln!();
    eprintln!("next:");
    if next.docs {
        eprintln!("  1. run `grund check` — a freshly scaffolded tree is clean");
        match &next.fs_home {
            InitFsHome::File { path, heading_name, heading_marker } => {
                eprintln!("  2. allocate an ID:  ID=$(grund id FS \"…\")  then add it to {path}");
                eprintln!(
                    "     ({heading_name}: `{heading_marker} <ID>: <one-line statement of the behavior>`)"
                );
            }
            InitFsHome::Folder { path } => {
                eprintln!("  2. allocate an ID:  ID=$(grund id FS \"…\")  then add it under {path}");
                eprintln!("     (H1: `# <ID>: <one-line statement of the behavior>`)");
            }
        }
        eprintln!(
            "  3. cite it as §<ID> from the docs and e2e tests that depend on it, then `grund check` again"
        );
    } else {
        let fs_home_path = match &next.fs_home {
            InitFsHome::File { path, .. } | InitFsHome::Folder { path } => path,
        };
        eprintln!("  1. re-run with --docs to scaffold the FS home ({fs_home_path}), docs/, and e2e/ (or create them yourself) — until then `grund check` has nothing to scan");
        eprintln!("  2. run `grund check` — a scaffolded tree is clean");
        match &next.fs_home {
            InitFsHome::File { path, .. } => {
                eprintln!("  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it to {path}");
            }
            InitFsHome::Folder { path } => {
                eprintln!("  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it under {path}");
            }
        }
    }
    eprintln!("see {} for the full workflow.", next.entrypoint);
}

fn command_cover(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
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
                    eprintln!("error: cover takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let opts = CoverOpts {
        path,
        path_provided,
    };
    if format_override.as_deref() == Some("json") {
        let output = match cover(opts.clone()) {
            Ok(output) => output,
            Err(err) => {
                eprintln!("error: {err:#}");
                return ExitCode::from(2);
            }
        };
        let format = match command_output_format("cover", &output.output_format, format_override) {
            Ok(format) => format,
            Err(code) => return code,
        };
        debug_assert_eq!(format, "json");
        render_cover_json(&output.entries);
        return exit_after_scan_errors(&output.scan_errors);
    }

    let output = match cover_text(opts.clone()) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = match command_output_format("cover", &output.output_format, format_override) {
        Ok(format) => format,
        Err(code) => return code,
    };
    if format == "json" {
        let output = match cover(opts) {
            Ok(output) => output,
            Err(err) => {
                eprintln!("error: {err:#}");
                return ExitCode::from(2);
            }
        };
        render_cover_json(&output.entries);
        return exit_after_scan_errors(&output.scan_errors);
    }

    render_cover_text(&output.entries);
    exit_after_scan_errors(&output.scan_errors)
}

fn render_cover_json(entries: &[grund_core::CoverEntry]) {
    for entry in entries {
        let citation_json = entry
            .citations
            .iter()
            .map(render_cover_citation_json)
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{{\"path\":\"{}\",\"citations\":[{}]}}",
            json_escape(&entry.path),
            citation_json
        );
    }
}

fn render_cover_text(entries: &[CoverTextEntry]) {
    for entry in entries {
        println!("{}:", entry.path);
        if entry.citations.is_empty() {
            println!("  (no citations)");
        } else {
            for citation in &entry.citations {
                println!("  {}:{} {}", citation.line, citation.column, citation.text);
            }
        }
    }
}

fn render_cover_citation_json(citation: &CoverCitation) -> String {
    format!(
        "{{\"path\":\"{}\",\"line\":{},\"column\":{},\"id\":\"{}\",\"section\":{},\"marker\":{},\"text\":\"{}\"}}",
        json_escape(&citation.path),
        citation.line,
        citation.column,
        json_escape(&citation.id),
        citation
            .section
            .as_deref()
            .map(|section| format!("\"{}\"", json_escape(section)))
            .unwrap_or_else(|| "null".to_string()),
        citation.marker,
        json_escape(&citation.text)
    )
}

fn command_fmt(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut write = false;
    let mut check_flag = false;
    let mut marker = false;
    let mut cross_refs = false;
    for arg in args {
        match arg.as_str() {
            "--check" => check_flag = true,
            "--write" => write = true,
            "--marker" => marker = true,
            "--cross-refs" => cross_refs = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path_provided {
                    eprintln!("error: fmt takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
    }
    if write && check_flag {
        eprintln!("error: --check and --write cannot be used together");
        return ExitCode::from(2);
    }
    let output = match format_references(FmtOpts {
        path,
        path_provided,
        write,
        add_marker: marker,
        cross_refs,
    }) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    if write {
        let mut files = output
            .changes
            .iter()
            .map(|change| change.path.clone())
            .collect::<Vec<_>>();
        files.sort();
        files.dedup();
        println!(
            "rewrote {} reference{}{}",
            output.changes.len(),
            if output.changes.len() == 1 { "" } else { "s" },
            if files.is_empty() { "" } else { ":" }
        );
        for path in &files {
            let count = output
                .changes
                .iter()
                .filter(|change| &change.path == path)
                .count();
            println!("  {path} ({count})");
        }
    } else {
        for change in &output.changes {
            println!("{}:{}: {}", change.path, change.line, change.label);
        }
    }
    if write || output.changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

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
    let outcome = match propose_id(
        kind,
        title,
        IdOpts {
            path,
            path_provided,
            width,
        },
    ) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    match outcome {
        IdProposalOutcome::UnknownKind { kind, known } => {
            eprintln!("error: unknown kind `{kind}`");
            eprintln!("known kinds: {}", known.join(", "));
            ExitCode::from(2)
        }
        IdProposalOutcome::Rejected { message } => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
        IdProposalOutcome::Proposed(proposal) => {
            print_id_proposal(&proposal, &format, explain);
            ExitCode::SUCCESS
        }
    }
}

fn print_id_proposal(proposal: &IdProposal, format: &str, explain: bool) {
    if format == "json" {
        println!(
            "{{\"id\":\"{}\",\"kind\":\"{}\",\"number\":{},\"slug\":\"{}\",\"folder\":\"{}\",\"file\":\"{}\"}}",
            json_escape(&proposal.id),
            json_escape(&proposal.kind),
            proposal
                .number
                .map(|number| number.to_string())
                .unwrap_or_else(|| "null".to_string()),
            json_escape(&proposal.slug),
            json_escape(proposal.folder.as_deref().unwrap_or("")),
            json_escape(proposal.file.as_deref().unwrap_or(""))
        );
        return;
    }

    println!("{}", proposal.id);
    if !explain {
        return;
    }
    match proposal.folder.as_deref() {
        Some(folder) if proposal.kind == "E2E" => {
            let case_dir = proposal
                .e2e_case_dir
                .as_deref()
                .unwrap_or(proposal.id.as_str());
            eprintln!(
                "next: create the case directory at {folder}/{case_dir}/ with expected.exit and fixtures, then cite it as §{}",
                proposal.id
            );
        }
        Some(folder) => eprintln!(
            "next: write the declaration at {folder}/{}.md  (H1: `# {}: <one-line statement>`), then cite it as §{}",
            proposal.id, proposal.id, proposal.id
        ),
        None if proposal.file.is_some() => {
            let file = proposal.file.as_deref().unwrap();
            let (heading_name, heading_marker) = if proposal.kind == "GRUND" {
                ("H1", "#")
            } else {
                ("H2", "##")
            };
            eprintln!(
                "next: add the declaration to {file}  ({heading_name}: `{heading_marker} {}: <one-line statement>`), then cite it as §{}",
                proposal.id, proposal.id
            );
        }
        None => eprintln!(
            "next: write the declaration with H1 `# {}: <one-line statement>`, then cite it as §{}",
            proposal.id, proposal.id
        ),
    }
}

fn command_config(args: &[String]) -> ExitCode {
    let Some(action) = args.first().map(|arg| arg.as_str()) else {
        eprintln!("error: expected `config validate` or `config show`");
        return ExitCode::from(2);
    };
    if !matches!(action, "validate" | "show") {
        if action.starts_with('-') {
            eprintln!("error: unknown flag `{action}`");
        } else {
            eprintln!("error: unknown config command `{action}`");
            eprintln!("expected: config validate, config show");
        }
        return ExitCode::from(2);
    }

    let mut path: Option<PathBuf> = None;
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("error: unknown flag `{arg}`");
            return ExitCode::from(2);
        }
        if path.is_some() {
            eprintln!("error: config {action} takes at most one path argument");
            return ExitCode::from(2);
        }
        path = Some(PathBuf::from(arg));
    }
    let path = path.unwrap_or_else(|| ".".into());

    match action {
        "validate" => match validate_config(&path) {
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::FAILURE
            }
        },
        "show" => match effective_config(&path) {
            Ok(config) => {
                print_effective_config(&config);
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::from(2)
            }
        },
        _ => unreachable!(),
    }
}

fn print_effective_config(config: &Config) {
    // §FS-config.4.2
    println!("grund_config_version = 1");
    if let Some(name) = &config.project_name {
        println!("project_name = \"{}\"", escape_toml_basic(name));
    }
    if let Some(description) = &config.project_description {
        println!(
            "project_description = \"{}\"",
            escape_toml_basic(description)
        );
    }
    println!();
    println!("[reference]");
    println!("marker = \"{}\"", config.marker);
    println!("trigger = \"{}\"", config.trigger);
    println!("strict = {}", config.strict);
    println!("require_grounding = {}", config.require_grounding);
    println!("inline_style = \"{}\"", config.inline_style);
    println!(
        "inline_note_suggested_lines = {}",
        config.inline_note_suggested_lines
    );
    println!("inline_note_max_lines = {}", config.inline_note_max_lines);
    println!("inline_note_max_columns = {}", config.inline_note_max_columns);
    println!("warn_on_suggested = {}", config.warn_on_suggested);
    println!();
    println!("[id]");
    println!("format = \"{}\"", config.id_format);
    println!("section_separator = \"{}\"", config.section_separator);
    println!(
        "section_heading_levels = \"{}\"",
        config.section_heading_levels
    );
    if config.id_format.contains("{number}") {
        println!(
            "number_pattern = \"{}\"",
            escape_toml_basic(&config.number_pattern)
        );
    }
    if config.id_format.contains("{slug}") {
        println!(
            "slug_pattern = \"{}\"",
            escape_toml_basic(&config.slug_pattern)
        );
    }
    println!();
    for kind in &config.kinds {
        println!("[[kinds]]");
        println!("prefix = \"{}\"", escape_toml_basic(&kind.prefix));
        if let Some(folder) = &kind.folder {
            println!("folder = \"{}\"", escape_toml_basic(folder));
        }
        if let Some(file) = &kind.file {
            println!("file = \"{}\"", escape_toml_basic(file));
        }
        if let Some(title) = &kind.title {
            println!("title = \"{}\"", escape_toml_basic(title));
        }
        println!();
    }
    println!("[scan]");
    println!(
        "include = {}",
        format_toml_string_list(config.include.as_deref().unwrap_or(&[]))
    );
    println!("exclude = {}", format_toml_string_list(&config.exclude));
    println!(
        "extensions = {}",
        format_toml_string_list(&config.extensions)
    );
    println!(
        "comment_prefixes = {}",
        format_toml_string_list(&config.comment_prefixes)
    );
    println!("docstring_python = {}", config.docstring_python);
    println!("respect_gitignore = {}", config.respect_gitignore);
    println!();
    println!("[output]");
    println!("format = \"{}\"", config.output_format);
    println!("color = \"auto\"");
    println!("relative_paths = {}", config.relative_paths);
    println!();
    println!("[fmt.cross_refs]");
    println!("enabled = {}", config.fmt_cross_refs_enabled);
    println!("anchor_format = \"{}\"", config.cross_ref_anchor_format);
    println!();
    println!("[render.links]");
    println!("web_base = \"{}\"", config.render_links_web_base);
    println!("web_ref = \"{}\"", config.render_links_web_ref);
    println!("hover_title = {}", config.render_links_hover_title);
    println!("local = \"{}\"", config.render_links_local);
    if config.workspace_declared {
        println!();
        println!("[workspace]");
        println!(
            "members = {}",
            format_toml_string_list(&config.workspace_members)
        );
        println!("include_root = {}", config.workspace_include_root);
    }
    if config.citations.declared {
        print_citation_rules(&config.citations);
    }
}

/// Print the effective `[citations]` section for `grund config show`
/// (§FS-config.4.2). Per-kind tables print in sorted order for deterministic
/// output (§FS-errors.4).
fn print_citation_rules(citations: &CitationRules) {
    println!();
    println!("[citations]");
    if let Some(default) = citations.global_default {
        println!("default = \"{}\"", citation_level_str(default));
    }
    for (kind, rules) in &citations.per_kind {
        println!();
        println!("[citations.{kind}]");
        if let Some(default) = rules.default {
            println!("default = \"{}\"", citation_level_str(default));
        }
        let lists: [(&str, &[CitationDisjunction]); 5] = [
            ("must", &rules.must),
            ("should", &rules.should),
            ("may", &rules.may),
            ("should-not", &rules.should_not),
            ("must-not", &rules.must_not),
        ];
        for (key, disjunctions) in lists {
            if disjunctions.is_empty() {
                continue;
            }
            let entries: Vec<String> = disjunctions.iter().map(render_citation_disjunction).collect();
            println!("{key} = {}", format_toml_string_list(&entries));
        }
    }
}

fn render_citation_disjunction(disjunction: &CitationDisjunction) -> String {
    disjunction
        .targets
        .iter()
        .map(render_citation_target)
        .collect::<Vec<_>>()
        .join("|")
}

fn render_citation_target(target: &CitationTarget) -> String {
    format!("{}{}", citation_namespace_label(&target.namespace), target.kind)
}

fn citation_namespace_label(namespace: &NamespaceMatch) -> String {
    match namespace {
        NamespaceMatch::Local => String::new(),
        NamespaceMatch::Any => "*/".to_string(),
        NamespaceMatch::Alias(alias) => format!("{alias}/"),
    }
}

fn citation_level_str(level: CitationLevel) -> &'static str {
    match level {
        CitationLevel::Must => "must",
        CitationLevel::Should => "should",
        CitationLevel::May => "may",
        CitationLevel::ShouldNot => "should-not",
        CitationLevel::MustNot => "must-not",
    }
}

fn format_toml_string_list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", escape_toml_basic(value)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn escape_toml_basic(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

fn command_complete(args: &[String]) -> ExitCode {
    match args.first().map(|arg| arg.as_str()) {
        Some("ids") => command_complete_ids(&args[1..]),
        _ => {
            eprintln!("error: expected `complete ids`");
            ExitCode::from(2)
        }
    }
}

fn command_complete_ids(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut prefix = String::new();
    let mut force_sections = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--prefix" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --prefix requires a value");
                    return ExitCode::from(2);
                }
                prefix = args[idx].clone();
            }
            other if other.starts_with("--prefix=") => {
                prefix = other.trim_start_matches("--prefix=").to_string();
            }
            "--sections" => force_sections = true,
            "--path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --path requires a value");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(&args[idx]);
                path_provided = true;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let candidates = match complete_ids(CompleteIdsOpts {
        path,
        path_provided,
        prefix,
        sections: force_sections,
    }) {
        Ok(candidates) => candidates,
        Err(_) => return ExitCode::SUCCESS,
    };
    for candidate in candidates {
        println!("{candidate}");
    }
    ExitCode::SUCCESS
}

fn command_completions(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: completions requires <bash|zsh|fish>");
        return ExitCode::from(2);
    }
    if args.len() > 1 {
        eprintln!("error: completions takes exactly one shell argument");
        return ExitCode::from(2);
    }
    match args[0].as_str() {
        "bash" => {
            print_bash_completion();
            ExitCode::SUCCESS
        }
        "zsh" => {
            print_zsh_completion();
            ExitCode::SUCCESS
        }
        "fish" => {
            print_fish_completion();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("error: unsupported shell `{other}`");
            eprintln!("known shells: bash, zsh, fish");
            ExitCode::from(2)
        }
    }
}

fn print_bash_completion() {
    print!(
        r#"# bash completion for grund
_grund_complete_ids() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    mapfile -t COMPREPLY < <(grund complete ids --prefix "$cur" 2>/dev/null)
    for candidate in "${{COMPREPLY[@]}}"; do
        if [[ "$candidate" == */ ]]; then
            compopt -o nospace 2>/dev/null
            break
        fi
    done
}}

_grund() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local sub="${{COMP_WORDS[1]}}"
    COMPREPLY=()

    if [[ $COMP_CWORD -eq 1 ]]; then
        local commands=($(compgen -W "check show list refs cover fmt id init config agent-setup-instructions completions" -- "$cur"))
        # IDs start with an uppercase kind (FS-…, GOAL-…), but workspace aliases
        # are lowercase (`api/FS-login`). Once the user has typed a non-flag
        # prefix, ask the helper for matching IDs and aliases.
        if [[ -n "$cur" && "$cur" != -* ]]; then
            _grund_complete_ids
        fi
        COMPREPLY=("${{commands[@]}}" "${{COMPREPLY[@]}}")
        return 0
    fi

    case "$sub" in
        show|refs)
            _grund_complete_ids
            return 0
            ;;
    esac
}}

complete -F _grund grund
"#
    );
}

fn print_zsh_completion() {
    println!(
        r#"#compdef grund

_grund_ids() {{
  local -a raw bare aliases
  raw=("${{(@f)$(grund complete ids --prefix "$words[CURRENT]" 2>/dev/null)}}")
  for candidate in $raw; do
    if [[ -z "$candidate" ]]; then
      continue
    fi
    if [[ "$candidate" == */ ]]; then
      aliases+=("$candidate")
    else
      bare+=("$candidate")
    fi
  done
  if (( ${{#aliases}} > 0 )); then
    compadd -S '' -a aliases
  fi
  if (( ${{#bare}} > 0 )); then
    _describe 'grund ids' bare
  fi
}}

_grund() {{
  local -a commands
  commands=(
    'check:validate every reference in a repo'
    'show:print one declaration body by ID'
    'list:list declared IDs'
    'refs:list citations of an ID'
    'cover:group citations by file'
    'fmt:normalize citation syntax'
    'id:emit the next conflict-free ID'
    'init:scaffold AGENTS.md and config'
    'config:inspect the effective config'
    'agent-setup-instructions:print the guided setup instructions for AI agents'
    'completions:print shell completion script'
  )

  if (( CURRENT == 2 )); then
    _describe 'grund command' commands
    # IDs start with an uppercase kind, but workspace aliases are lowercase.
    # Once a non-flag prefix exists, ask the helper for matching IDs/aliases.
    if [[ -n "$words[CURRENT]" && "$words[CURRENT]" != -* ]]; then
      _grund_ids
    fi
    return
  fi

  case "$words[2]" in
    show|refs) _grund_ids ;;
    *) _files ;;
  esac
}}

_grund "$@"
"#
    );
}

fn print_fish_completion() {
    println!(
        r#"# fish completion for grund
function __grund_complete_ids
    set -l token (commandline -ct)
    grund complete ids --prefix "$token" 2>/dev/null
end

complete -c grund -f -n "__fish_use_subcommand" -a "check show list refs cover fmt id init config agent-setup-instructions completions"
# IDs start with an uppercase kind, but workspace aliases are lowercase. Once a
# non-flag prefix exists, ask the helper for matching IDs/aliases.
complete -c grund -f -k -n "__fish_use_subcommand; and test -n (commandline -ct); and not string match -qr '^-' -- (commandline -ct)" -a "(__grund_complete_ids)"
complete -c grund -f -k -n "__fish_seen_subcommand_from show refs" -a "(__grund_complete_ids)"
"#
    );
}

fn command_output_format(
    command: &str,
    configured: &str,
    override_format: Option<String>,
) -> Result<String, ExitCode> {
    let format = override_format.unwrap_or_else(|| configured.to_string());
    if matches!(format.as_str(), "text" | "json") {
        Ok(format)
    } else {
        eprintln!("error: unsupported {command} format `{format}`");
        Err(ExitCode::from(2))
    }
}

fn exit_after_scan_errors(scan_errors: &[ApiScanError]) -> ExitCode {
    if scan_errors.is_empty() {
        return ExitCode::SUCCESS;
    }
    for err in scan_errors {
        eprintln!("error: {}: {}", err.path, err.message);
    }
    ExitCode::from(2)
}

fn json_escape(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => escaped.push_str(&format!("\\u{:04x}", other as u32)),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Restore the default `SIGPIPE` disposition (Unix only).
///
/// Rust ignores `SIGPIPE` at startup, which turns a closed downstream pipe
/// (`grund list | head`) into an `EPIPE` on the next write — and `println!`
/// panics on a write error. A CLI in a pipeline should instead die quietly,
/// the way `ls | head` does. This is a no-op off Unix.
#[cfg(unix)]
fn restore_default_sigpipe() {
    // SIGPIPE == 13 and SIG_DFL == (void(*)(int))0 on Linux, macOS, and the BSDs.
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// The CLI entry point: parse `argv`, dispatch to the matching `command_*`, and
/// return its `ExitCode` (§FS-cli). `grund <ID>` is the default ID query
/// (§FS-cli.1); `grund` with no arguments keeps the historical `check .`
/// behavior with a deprecation warning; `--version`/`--help` short-circuits to
/// stdout, exit 0 (§FS-cli.2); help on an unknown command exits 2 and lists the
/// known ones (§FS-cli.4). The exit-code mapping (0/1/2) is fixed (§FS-cli.5).
pub fn main_entry() -> ExitCode {
    restore_default_sigpipe();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("grund {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let first = args.first().map(|arg| arg.as_str());
    // `grund help [<subcommand>]` — the top-level page with no argument, that
    // subcommand's page with one, an error for an unknown name (§FS-cli.2).
    if first == Some("help") {
        return match args.get(1).map(String::as_str) {
            None => {
                print_help();
                ExitCode::SUCCESS
            }
            Some(cmd) if SUBCOMMANDS.contains(&cmd) => {
                print_subcommand_help(cmd);
                ExitCode::SUCCESS
            }
            Some(other) => {
                eprintln!("error: unknown command: {other}");
                eprintln!("known commands: {}", SUBCOMMANDS.join(", "));
                ExitCode::from(2)
            }
        };
    }
    // `--help` / `-h` short-circuits before any work; with a known subcommand
    // first it prints that subcommand's page, with no command it prints the
    // top-level one, and with an unknown first word it remains an unknown-command
    // error rather than hiding a typo behind generic help (§FS-cli.2, §FS-cli.4).
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        match first {
            Some(cmd) if SUBCOMMANDS.contains(&cmd) => print_subcommand_help(cmd),
            None | Some("--help" | "-h") => print_help(),
            Some(other) if other.starts_with('-') => print_help(),
            Some(other) => {
                eprintln!("error: unknown command: {other}");
                eprintln!("known commands: {}", SUBCOMMANDS.join(", "));
                return ExitCode::from(2);
            }
        }
        return ExitCode::SUCCESS;
    }
    match first {
        None => {
            eprintln!(
                "warning: bare `grund` still runs `grund check .`; use `grund check` explicitly."
            );
            command_check(&[])
        }
        Some("check") => command_check(&args[1..]),
        Some("show") => command_show(&args[1..]),
        Some("list") => command_list(&args[1..]),
        Some("refs") => command_refs(&args[1..]),
        Some("cover") => command_cover(&args[1..]),
        Some("fmt") => command_fmt(&args[1..]),
        Some("id") => command_id(&args[1..]),
        Some("init") => command_init(&args[1..]),
        Some("config") => command_config(&args[1..]),
        Some("agent-setup-instructions") => command_agent_setup_instructions(&args[1..]),
        Some("completions") => command_completions(&args[1..]),
        Some("integrations") => run_integrations(&args[1..]),
        Some("complete") => command_complete(&args[1..]),
        // Any first argument that is not a known subcommand is an ID query
        // (§FS-cli.1). Check is explicit as `grund check [path]`.
        Some(_) => command_show_default(&args),
    }
}
