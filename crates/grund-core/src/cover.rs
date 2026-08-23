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
    // §RM-core-cli-split: the deprecated compatibility surface renders the
    // same `cover` the CLI does, so the index — including its workspace scope
    // (§FS-workspace.8.6) — is built once, in the API.
    let opts = CoverOpts {
        path,
        path_provided,
    };
    let output = match cover(opts) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = format_override.unwrap_or_else(|| output.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported cover format `{format}`");
        return ExitCode::from(2);
    }

    if format == "json" {
        for entry in &output.entries {
            let citation_json = entry
                .citations
                .iter()
                .map(compat_cover_citation_json)
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "{{{}\"path\":\"{}\",\"citations\":[{}]}}",
                compat_cover_project_field(entry.project.as_deref()),
                json_escape(&entry.path),
                citation_json
            );
        }
    } else {
        for entry in &output.entries {
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

    if output.scan_errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-cover.4 / §FS-check.2): the emitted records
        // are real but incomplete, so callers must treat the result as untrusted.
        // In a workspace that includes a member's unreadable file, since the
        // index the run just printed is incomplete for the tree it claimed
        // (§FS-workspace.8.7).
        for error in &output.scan_errors {
            eprintln!("error: {}: {}", error.path, error.message);
        }
        ExitCode::from(2)
    }
}

/// §FS-cover.3.2: see `cover_project_field` in the CLI — the two renderers emit
/// the same bytes and are held to it by `e2e/cases/workspace-cover-json`.
fn compat_cover_project_field(alias: Option<&str>) -> String {
    alias
        .map(|alias| format!("\"project\":\"{}\",", json_escape(alias)))
        .unwrap_or_default()
}

fn compat_cover_citation_json(citation: &CoverCitation) -> String {
    format!(
        "{{{}\"path\":\"{}\",\"line\":{},\"column\":{},\"id\":\"{}\",\"section\":{},\"marker\":{},\"text\":\"{}\"}}",
        compat_cover_project_field(citation.project.as_deref()),
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
