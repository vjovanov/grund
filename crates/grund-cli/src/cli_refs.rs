/// `grund refs <ID> [--summary] [--format text|json]`: every citation of one
/// ID, rendered as `path:line` (§FS-refs).
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
