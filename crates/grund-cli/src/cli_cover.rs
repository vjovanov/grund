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
    // §FS-cover.1: a bad `--format` value is a usage error the caller can fix
    // without touching the repository, so it is answered before the scan — the
    // scan can now fail first (a workspace whose members will not expand,
    // §FS-cover.4), and which of two errors a caller sees must not depend on
    // the tree they happened to point at.
    if let Some(format) = format_override.as_deref() {
        if !matches!(format, "text" | "json") {
            eprintln!("error: unsupported cover format `{format}`");
            return ExitCode::from(2);
        }
    }
    let opts = CoverOpts {
        path,
        path_provided,
    };
    // One load, whichever view is rendered: `cover` and `cover_text` build the
    // same index (§FS-workspace.8.6), and calling both walked every project in
    // the workspace twice.
    let output = match cover(opts) {
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
        render_cover_json(&output.entries);
    } else {
        render_cover_text(&output.entries);
    }
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
            "{{{}\"path\":\"{}\",\"citations\":[{}]}}",
            cover_project_field(entry.project.as_deref()),
            json_escape(&entry.path),
            citation_json
        );
    }
}

/// §FS-cover.3.2: the leading `"project":"<alias>"` a workspace run adds, and
/// nothing at all outside one — a single-project repo's JSON keeps the bytes it
/// had (§DF-cover-workspace-scope.2.3).
fn cover_project_field(alias: Option<&str>) -> String {
    alias
        .map(|alias| format!("\"project\":\"{}\",", json_escape(alias)))
        .unwrap_or_default()
}

/// §FS-cover.3.1: the human view prints the file, then each citation's
/// `line:column` and verbatim token — the alias is already in the token and the
/// path already renders from the workspace root, so no field of the JSON view is
/// missing here.
fn render_cover_text(entries: &[grund_core::CoverEntry]) {
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
        "{{{}\"path\":\"{}\",\"line\":{},\"column\":{},\"id\":\"{}\",\"section\":{},\"marker\":{},\"text\":\"{}\"}}",
        cover_project_field(citation.project.as_deref()),
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
