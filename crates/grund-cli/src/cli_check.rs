/// Parse and validate the complete `grund check` input, including repeatable
/// exact-code selectors (§FS-check.1), then select only after the full scan and
/// before rendering and the exit decision (§FS-check.2.1).
fn command_check(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut format_override = None;
    let mut require_grounding = false;
    let mut include_suggestions = false;
    let mut full = false;
    let mut selection = CheckFindingSelection::default();
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
            other if other.starts_with("--only=") => {
                let value = other
                    .strip_prefix("--only=")
                    .expect("guarded by starts_with");
                if let Err(err) = selection.add_only(value) {
                    eprintln!("error: {err}");
                    return ExitCode::from(2);
                }
            }
            "--only" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --only requires a finding code");
                    return ExitCode::from(2);
                }
                if let Err(err) = selection.add_only(&args[idx]) {
                    eprintln!("error: {err}");
                    return ExitCode::from(2);
                }
            }
            other if other.starts_with("--ignore=") => {
                let value = other
                    .strip_prefix("--ignore=")
                    .expect("guarded by starts_with");
                if let Err(err) = selection.add_ignore(value) {
                    eprintln!("error: {err}");
                    return ExitCode::from(2);
                }
            }
            "--ignore" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --ignore requires a finding code");
                    return ExitCode::from(2);
                }
                if let Err(err) = selection.add_ignore(&args[idx]) {
                    eprintln!("error: {err}");
                    return ExitCode::from(2);
                }
            }
            // §FS-check.1.3: widen the walk past `[scan] include` for this run.
            "--full" => full = true,
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
    let mut output = match check_with_opts(CheckOpts {
        path,
        path_provided,
        require_grounding,
        include_suggestions,
        full,
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
    // §FS-check.2.1: the complete API report exists before the CLI applies its
    // presentation query; retained diagnostics then use ordinary rendering.
    output
        .report
        .errors
        .retain(|finding| selection.retains(finding.code));
    output
        .report
        .warnings
        .retain(|finding| selection.retains(finding.code));
    output
        .report
        .suggestions
        .retain(|finding| selection.retains(finding.code));
    if format == "json" {
        render_check_json(&output.report);
    } else {
        render_check_text(&output.report, output.unread_opted_out_blocks);
    }
    if output.had_scan_errors {
        ExitCode::from(2)
    } else if output.report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Compare one format's retained findings by the fixed bytewise key
/// (§FS-errors.4); text applies it per channel and JSON applies it globally.
fn finding_cmp(a: &Finding, b: &Finding) -> std::cmp::Ordering {
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
}

fn sorted_text_findings(report: &Report) -> Vec<(&'static str, &Finding)> {
    // §FS-check.2.3: `report.suggestions` is populated only when the caller
    // asked for them, so chaining it unconditionally is a no-op otherwise.
    // §FS-errors.4: text has fixed error, warning, suggestion groups, with
    // the bytewise location/message comparator applied inside each group.
    let mut errors = report
        .errors
        .iter()
        .map(|finding| ("error", finding))
        .collect::<Vec<_>>();
    let mut warnings = report
        .warnings
        .iter()
        .map(|finding| ("warning", finding))
        .collect::<Vec<_>>();
    let mut suggestions = report
        .suggestions
        .iter()
        .map(|finding| ("suggestion", finding))
        .collect::<Vec<_>>();
    errors.sort_by(|(_, a), (_, b)| finding_cmp(a, b));
    warnings.sort_by(|(_, a), (_, b)| finding_cmp(a, b));
    suggestions.sort_by(|(_, a), (_, b)| finding_cmp(a, b));
    errors.extend(warnings);
    errors.extend(suggestions);
    errors
}

/// Keep JSON in its pre-existing global bytewise location/message order
/// (§FS-check.2.1, §FS-errors.4), independent of text's severity groups.
fn sorted_json_findings(report: &Report) -> Vec<(&'static str, &Finding)> {
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
    findings.sort_by(|(_, a), (_, b)| finding_cmp(a, b));
    findings
}

/// `unread_opted_out_blocks` is the §FS-check.4.10 lines the run already printed on
/// stderr, before this report existed. They are not findings, so nothing in
/// `report` records them — and a run that says its citations are unchecked must not
/// also say `success` (§FS-check.2.1).
fn render_check_text(report: &Report, unread_opted_out_blocks: usize) {
    // §FS-check.2.3: suggestions never suppress `success`, but when present
    // (caller passed --suggestions) they are printed, so the marker only stands
    // in for a run with nothing at all to show.
    if unread_opted_out_blocks == 0
        && report.errors.is_empty()
        && report.warnings.is_empty()
        && report.suggestions.is_empty()
    {
        println!("success");
        return;
    }
    for (severity, finding) in sorted_text_findings(report) {
        let line = match (finding.path.as_deref(), finding.line) {
            // §FS-errors.2.1: retain the jump-friendly location prefix and
            // place `check`'s structural channel before unchanged message bytes.
            (Some(path), Some(line)) => {
                format!("{path}:{line}: {severity}: {}", finding.message)
            }
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
    for (severity, finding) in sorted_json_findings(report) {
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
