/// `grund list [PATH] [--kind K,...] [--unused] [--summary]
/// [--size[=lines,words,bytes]] [--top N] [--format text|json]`:
/// the ID catalog — every declared ID with its `path:line` and title
/// (§FS-list). The complement of `grund refs`.
fn command_list(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut kind_filter: BTreeSet<String> = BTreeSet::new();
    let mut project_filter: BTreeSet<String> = BTreeSet::new();
    let mut unused_only = false;
    let mut summary = false;
    let mut size_units: Option<Vec<PointSizeUnit>> = None;
    let mut top: Option<usize> = None;
    let mut top_seen = false;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--unused" => unused_only = true,
            "--summary" => summary = true,
            "--size" => {
                if size_units.is_some() {
                    eprintln!("error: --size may only appear once");
                    return ExitCode::from(2);
                }
                size_units = Some(vec![
                    PointSizeUnit::Lines,
                    PointSizeUnit::Words,
                    PointSizeUnit::Bytes,
                ]);
            }
            other if other.starts_with("--size=") => {
                if size_units.is_some() {
                    eprintln!("error: --size may only appear once");
                    return ExitCode::from(2);
                }
                let raw = other.strip_prefix("--size=").expect("guarded size flag");
                let mut units = Vec::new();
                for name in raw.split(',') {
                    if name.is_empty() {
                        eprintln!("error: --size requires one or more units");
                        return ExitCode::from(2);
                    }
                    let Some(unit) = PointSizeUnit::parse(name) else {
                        eprintln!(
                            "error: unknown size unit `{name}` (expected lines, words, or bytes)"
                        );
                        return ExitCode::from(2);
                    };
                    if !units.contains(&unit) {
                        units.push(unit);
                    }
                }
                size_units = Some(units);
            }
            other if other.starts_with("--top=") => {
                if top_seen {
                    eprintln!("error: --top may only appear once");
                    return ExitCode::from(2);
                }
                top_seen = true;
                let raw = other.strip_prefix("--top=").expect("guarded top flag");
                top = match raw.parse::<usize>() {
                    Ok(value) if value > 0 => Some(value),
                    _ => {
                        eprintln!("error: --top requires a positive integer");
                        return ExitCode::from(2);
                    }
                };
            }
            "--top" => {
                if top_seen {
                    eprintln!("error: --top may only appear once");
                    return ExitCode::from(2);
                }
                top_seen = true;
                idx += 1;
                top = match args.get(idx).and_then(|raw| raw.parse::<usize>().ok()) {
                    Some(value) if value > 0 => Some(value),
                    _ => {
                        eprintln!("error: --top requires a positive integer");
                        return ExitCode::from(2);
                    }
                };
            }
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
    // §FS-list.1: size-selector combinations are launch errors decided before
    // config discovery or scanning.
    if size_units.is_some() && summary {
        eprintln!("error: --size cannot be combined with --summary");
        return ExitCode::from(2);
    }
    if top_seen && summary {
        eprintln!("error: --top cannot be combined with --summary");
        return ExitCode::from(2);
    }
    if top_seen && size_units.is_none() {
        eprintln!("error: --top requires --size");
        return ExitCode::from(2);
    }

    if let Some(units) = size_units {
        let output = match list_sizes(ListSizeOpts {
            path,
            path_provided,
            kind_filter,
            project_filter,
            unused_only,
            units,
            top,
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
        if format == "json" {
            for entry in &output.entries {
                println!("{}", render_list_size_entry_json(entry));
            }
        } else {
            render_list_size_text(&output.entries);
        }
        return exit_after_scan_errors(&output.scan_errors);
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
    // §FS-workspace.8.3: the alias column is sized to the widest alias among
    // the rows being rendered, capped like `render_list_text`'s `id_width`.
    let alias_width = summaries
        .iter()
        .filter_map(|summary| summary.project.as_deref())
        .map(|project| project.chars().count())
        .max()
        .unwrap_or(0)
        .min(40);
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
                    "{project:<alias_width$}  {:<4}  {:>3}  {}",
                    summary.kind, summary.count, summary.home
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
    let value_roots = if entry.value_roots.is_empty() {
        String::new()
    } else {
        let roots = entry
            .value_roots
            .iter()
            .map(|root| {
                format!(
                    "{{\"id\":\"{}\",\"valid\":{}}}",
                    json_escape(&root.id),
                    root.valid
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(",\"value_roots\":[{roots}]")
    };
    format!(
        "{{{}\"id\":\"{}\",\"kind\":\"{}\",\"path\":\"{}\",\"line\":{},\"title\":{},\"stub\":{},\"defines\":{},\"refs\":{},\"duplicate\":{}{}}}",
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
        value_roots,
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
        if !entry.value_roots.is_empty() {
            let roots = entry
                .value_roots
                .iter()
                .map(|root| {
                    if root.valid {
                        root.id.clone()
                    } else {
                        format!("{} (invalid)", root.id)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            if note.is_empty() {
                note = format!("[value roots: {roots}]");
            } else {
                note.push_str(&format!(" [value roots: {roots}]"));
            }
        }
        if note.is_empty() {
            println!("{:<id_width$}  {location}", entry.id);
        } else {
            println!("{:<id_width$}  {location}  {note}", entry.id);
        }
    }
}

/// Render the owning project's exact declaration/section coordinate
/// (§FS-list.3.4, §FS-workspace.8.3).
fn list_size_coordinate(entry: &ListSizeEntry) -> String {
    match &entry.section {
        Some(section) => format!("{}{}{}", entry.id, entry.section_separator, section),
        None => entry.id.clone(),
    }
}

/// §FS-list.3.4 / §FS-output-shapes.5: size-mode NDJSON has its own fixed
/// metadata prefix and appends only selected lead/full pairs in caller order.
fn render_list_size_entry_json(entry: &ListSizeEntry) -> String {
    let project_field = entry
        .project
        .as_deref()
        .map(|project| format!("\"project\":\"{}\",", json_escape(project)))
        .unwrap_or_default();
    let section = entry
        .section
        .as_deref()
        .map(|section| format!("\"{}\"", json_escape(section)))
        .unwrap_or_else(|| "null".to_string());
    let defines = entry
        .defines
        .as_deref()
        .map(|target| format!("\"{}\"", json_escape(target)))
        .unwrap_or_else(|| "null".to_string());
    let mut json = format!(
        "{{{}\"id\":\"{}\",\"section\":{},\"kind\":\"{}\",\"path\":\"{}\",\"line\":{},\"stub\":{},\"defines\":{},\"duplicate\":{}",
        project_field,
        json_escape(&entry.id),
        section,
        json_escape(&entry.kind),
        json_escape(&entry.path),
        entry.line,
        entry.stub,
        defines,
        entry.duplicate,
    );
    for measurement in &entry.measurements {
        let unit = measurement.unit.as_str();
        let lead = measurement
            .lead
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string());
        let full = measurement
            .full
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string());
        json.push_str(&format!(
            ",\"lead_{unit}\":{lead},\"full_{unit}\":{full}"
        ));
    }
    json.push('}');
    json
}

/// Headerless size rows with ordinary list alignment and site-local suffixes
/// (§FS-list.3.4).
fn render_list_size_text(entries: &[ListSizeEntry]) {
    let coordinates = entries.iter().map(list_size_coordinate).collect::<Vec<_>>();
    let width = coordinates
        .iter()
        .map(|coordinate| coordinate.chars().count())
        .max()
        .unwrap_or(0)
        .min(40);
    for (entry, coordinate) in entries.iter().zip(coordinates) {
        let location = format!("{}:{}", entry.path, entry.line);
        let sizes = entry
            .measurements
            .iter()
            .map(|measurement| {
                let unit = measurement.unit.as_str();
                match (measurement.lead, measurement.full) {
                    (Some(lead), Some(full)) => format!("{unit}={lead}/{full}"),
                    _ => format!("{unit}=-/-"),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let suffix = if entry.stub {
            format!(
                "  (broken stub → {})",
                entry.defines.as_deref().unwrap_or("")
            )
        } else if entry.duplicate {
            "  (duplicate — site-local)".to_string()
        } else {
            String::new()
        };
        println!("{coordinate:<width$}  {location}  {sizes}{suffix}");
    }
}
