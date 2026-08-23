/// §FS-cover.1: everything `cover` can answer from its argv alone, answered
/// there — including a bad `--format` value, which is a usage error the caller
/// can fix without touching the repository. The scan can fail first (a
/// workspace whose members will not expand, §FS-cover.4), and which of two
/// errors a caller sees must not depend on the tree they happened to point at.
///
/// Split from `command_cover` because that ordering is otherwise unobservable:
/// both answers exit 2, so only the message distinguishes them, and nothing in
/// the e2e corpus reaches this copy (§FS-cover.3.2). A function that holds no
/// path to a loader proves the order to a test —
/// `the_compat_surface_answers_a_bad_format_before_it_loads_anything`.
fn parse_compat_cover_args(args: &[String]) -> Result<(CoverOpts, Option<String>), String> {
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
                    return Err("--format requires a value".to_string());
                }
                format_override = Some(args[idx].clone());
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag `{other}`"));
            }
            other => {
                if path_provided {
                    return Err("cover takes at most one path argument".to_string());
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    if let Some(format) = format_override.as_deref()
        && !matches!(format, "text" | "json")
    {
        return Err(format!("unsupported cover format `{format}`"));
    }
    Ok((
        CoverOpts {
            path,
            path_provided,
        },
        format_override,
    ))
}

fn command_cover(args: &[String]) -> ExitCode {
    let (opts, format_override) = match parse_compat_cover_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };
    // §RM-core-cli-split: the deprecated compatibility surface renders the
    // same `cover` the CLI does, so the index — including its workspace scope
    // (§FS-workspace.8.6) — is built once, in the API.
    let output = match cover(opts) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    // The override was validated in the parse; only the configured fallback is
    // left to check, and that one is a property of the tree just loaded.
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
/// the same bytes, and `the_compat_renderer_emits_the_same_json_the_cli_does`
/// (`tests_cover_workspace.rs`) is what holds them there. Not an e2e case: every
/// case in the corpus drives the `grund` binary, which is `grund-cli`, so
/// nothing in it reaches this copy.
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
