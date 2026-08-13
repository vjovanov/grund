/// Print the text report in the fixed output shapes (§FS-errors.1,
/// §FS-errors.2.1, §FS-errors.2.4): `path:line: message` for located findings,
/// run-level diagnostics on stderr, and `success` for a clean text check
/// (§FS-check.2.1). Diagnostic lines stay in the fixed order (§FS-errors.4).
fn print_report(config: &Config, report: &CheckReport, include_suggestions: bool) {
    // §FS-check.2.3: the `success` marker keys off errors and warnings only — a
    // suggestion is not a finding about well-formedness, so it never suppresses
    // `success`, and without `--suggestions` it is not printed at all.
    if report.errors.is_empty()
        && report.warnings.is_empty()
        && (!include_suggestions || report.suggestions.is_empty())
    {
        println!("success");
        return;
    }
    let mut diagnostics = report
        .warnings
        .iter()
        .map(|diagnostic| ("warning", diagnostic))
        .chain(report.errors.iter().map(|diagnostic| ("error", diagnostic)))
        .collect::<Vec<_>>();
    if include_suggestions {
        diagnostics.extend(
            report
                .suggestions
                .iter()
                .map(|diagnostic| ("suggestion", diagnostic)),
        );
    }
    diagnostics.sort_by(|(_, a), (_, b)| diagnostic_cmp(a, b));
    for (severity, diagnostic) in diagnostics {
        let line = render_diagnostic_text(config, severity, diagnostic);
        // §FS-errors.1 / §FS-check.2.1: a located finding (`<path>:<line>: …`) is
        // `check`'s output → stdout. A `line`-less diagnostic — a mid-walk read
        // failure (§FS-check.2) or the empty-scan caution (§FS-check.2.2) — is a
        // CLI-level message about the run, not a finding → stderr.
        if diagnostic.line.is_some() {
            println!("{line}");
        } else {
            eprintln!("{line}");
        }
    }
}

fn render_diagnostic_text(config: &Config, severity: &str, diagnostic: &Diagnostic) -> String {
    match (&diagnostic.path, diagnostic.line) {
        (Some(path), Some(line)) => {
            format!(
                "{}:{}: {}",
                display_path(config, path),
                line,
                diagnostic.message
            )
        }
        // A file-level finding with no line to point at (e.g. an unreadable file
        // discovered mid-walk) uses the CLI-level shape — §FS-check.2, §FS-errors.2.2.
        (Some(path), None) => format!(
            "{severity}: {}: {}",
            display_path(config, path),
            diagnostic.message
        ),
        _ => format!("{severity}: {}", diagnostic.message),
    }
}

fn sorted_json_diagnostics(
    report: &CheckReport,
    include_suggestions: bool,
) -> Vec<(&'static str, &Diagnostic)> {
    let mut diagnostics = report
        .warnings
        .iter()
        .map(|diagnostic| ("warning", diagnostic))
        .chain(report.errors.iter().map(|diagnostic| ("error", diagnostic)))
        .collect::<Vec<_>>();
    if include_suggestions {
        diagnostics.extend(
            report
                .suggestions
                .iter()
                .map(|diagnostic| ("suggestion", diagnostic)),
        );
    }
    diagnostics.sort_by(|(_, a), (_, b)| diagnostic_cmp(a, b));
    diagnostics
}

/// Print the report as newline-delimited JSON objects — the `--format json` /
/// `[output] format = "json"` shape (§FS-errors.5): one object per finding with
/// `severity`, `path`, `line`, `code`, `message`, `sites`. Located findings go to
/// stdout (`check`'s output, §FS-errors.1); a `line`-less diagnostic (mid-walk read
/// failure, empty-scan caution) goes to stderr, mirroring the text form.
fn print_json_report(config: &Config, report: &CheckReport, include_suggestions: bool) {
    for (channel, diagnostic) in sorted_json_diagnostics(report, include_suggestions) {
        let object = render_diagnostic_json(config, channel, diagnostic);
        if diagnostic.line.is_some() {
            println!("{object}");
        } else {
            eprintln!("{object}");
        }
    }
}

/// Render one diagnostic as a JSON object (§FS-errors.5). An `error` / `warning`
/// carries a `"severity"`; a citation-direction `suggestion` carries
/// `"channel":"suggestion"` instead, keeping the frozen `{error, warning}`
/// severity set intact (§FS-config.6, §FS-check.2.3).
fn render_diagnostic_json(config: &Config, channel: &str, diagnostic: &Diagnostic) -> String {
    let path = diagnostic
        .path
        .as_ref()
        .map(|path| format!("\"{}\"", json_escape(&display_path(config, path))))
        .unwrap_or_else(|| "null".to_string());
    let line = diagnostic
        .line
        .map(|line| line.to_string())
        .unwrap_or_else(|| "null".to_string());
    let sites = if diagnostic.sites.is_empty() {
        "null".to_string()
    } else {
        let values = diagnostic
            .sites
            .iter()
            .map(|site| {
                format!(
                    "{{\"path\":\"{}\",\"line\":{}}}",
                    json_escape(&display_path(config, &site.path)),
                    site.line
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", values)
    };
    let tag = if channel == "suggestion" {
        "\"channel\":\"suggestion\"".to_string()
    } else {
        format!("\"severity\":\"{channel}\"")
    };
    format!(
        "{{{},\"path\":{},\"line\":{},\"code\":\"{}\",\"message\":\"{}\",\"sites\":{}}}",
        tag,
        path,
        line,
        diagnostic.code,
        json_escape(&diagnostic.message),
        sites
    )
}

fn print_bare_query_json(config: &Config, code: &'static str, message: &str) {
    let diagnostic = Diagnostic {
        code,
        path: None,
        line: None,
        column: None,
        message: message.to_string(),
        sites: Vec::new(),
    };
    eprintln!("{}", render_diagnostic_json(config, "error", &diagnostic));
}

fn show_query_error_code(message: &str) -> &'static str {
    if message.starts_with("ID not found:") {
        "not-found"
    } else if message.starts_with("section not found:") {
        "missing-section"
    } else if message.starts_with("invalid ID") {
        "invalid-id"
    } else if message.starts_with("ambiguous ID:") {
        "ambiguous"
    } else if message.starts_with("broken stub:") {
        "broken-stub"
    } else {
        "query-failed"
    }
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

/// Render a path the way reports show it: relative to the repo root by default,
/// or relative to the CLI base directory when `[output] relative_paths = false`
/// (§FS-config.3.6, §FS-errors.4 — never an absolute path outside the root).
fn display_path(config: &Config, path: &Path) -> String {
    let base = if config.relative_paths {
        &config.root
    } else {
        &config.cli_base
    };
    format_path(path.strip_prefix(base).unwrap_or(path))
}

fn format_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sort_path_key(path: &Path) -> String {
    format_path(path)
}

/// The CLI-level warning `check` reports when the tree walk matched no files
/// (§FS-check.2.2): a scan that read nothing is almost always a misconfigured
/// scope, so we say so instead of printing nothing and exiting `0`. This is a
/// warning — it never changes the exit code.
fn empty_scan_warning(config: &Config, path: &Path, path_provided: bool) -> Diagnostic {
    // `grund`, `grund check .`, and `grund check <repo-root>` all walk `[scan] include`
    // relative to the config root — so the "looked under include" message is the
    // accurate one whenever the requested path *is* that root, not just when the
    // path was omitted.
    let scoped_to_root = !path_provided
        || path == Path::new(".")
        || fs::canonicalize(path)
            .map(|p| p == config.root)
            .unwrap_or(false);
    let message = match (&config.include, scoped_to_root) {
        (Some(dirs), true) => format!(
            "nothing to scan — grund looked under [scan] include = [{}] and found no files. Run \
             `grund init --docs` to scaffold the canonical requirements.md, docs/, and e2e/ \
             trees, point `[scan] include` in `grund.toml` at your sources, or pass a \
             path explicitly (`grund check <dir>`).",
            dirs.iter()
                .map(|dir| format!("\"{dir}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!(
            "nothing to scan — no files under `{}` matched grund's extensions ({}).",
            format_path(path),
            config.extensions.join(", ")
        ),
    };
    Diagnostic {
        code: "empty-scan",
        path: None,
        line: None,
        column: None,
        message,
        sites: Vec::new(),
    }
}

/// Print [`config_warnings`] in the CLI-level shape (§FS-errors.2.2): one
/// `warning: ` line each, on stderr, exit code untouched. Rendering, so it
/// lives with the other report printers rather than in `api.rs`, which is the
/// embedding surface *without* stdout/stderr rendering
/// (§AR-core-module-layout.1) — and in one place rather than in each `config`
/// frontend, so the published CLI and the deprecated `grund_core` adapter
/// cannot drift on the prefix or the stream (§FS-config.4.1, §FS-config.4.2).
pub fn print_config_warnings(config: &Config) {
    for warning in config_warnings(config) {
        eprintln!("warning: {warning}");
    }
}

/// §FS-check.4.3: the warning for a config root holding both discovery names —
/// the bare `grund.toml` won, and the `.agents/grund.toml` beside it is read by
/// nothing (§FS-config.1.1). `line`-less, so it prints as a CLI-level `warning:`
/// on stderr: it says which file the run read, not what is wrong at a site.
fn redundant_config_warning(config: &Config) -> Option<Diagnostic> {
    let ignored = config.redundant_config_file.as_ref()?;
    let winner = config.config_file.as_ref()?;
    Some(Diagnostic {
        code: "redundant-config",
        path: None,
        line: None,
        column: None,
        message: format!(
            "{} is ignored — {} takes precedence; delete one",
            format_path(ignored),
            format_path(winner)
        ),
        sites: Vec::new(),
    })
}
