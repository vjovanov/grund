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
