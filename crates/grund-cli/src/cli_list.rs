/// `grund list [PATH] [--kind K,...] [--unused] [--summary] [--format text|json]`:
/// the ID catalog — every declared ID with its `path:line` and title
/// (§FS-list). The complement of `grund refs`.
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
