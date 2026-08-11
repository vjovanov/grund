/// `grund cover [PATH] [--format text|json]`: the citation graph grouped by
/// scanned file, for finding what is under- or un-cited (§FS-cover).
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
