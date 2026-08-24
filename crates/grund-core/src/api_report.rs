// Private conversion from checker diagnostics to the published report shape.
// Kept out of `api.rs` so that file remains the public embedding contract
// (§AR-core-module-layout.2).

fn public_report(config: &Config, report: CheckReport, include_suggestions: bool) -> Report {
    public_report_with_path_mode(config, report, include_suggestions, false)
}

fn public_lsp_report(config: &Config, report: CheckReport) -> Report {
    public_report_with_path_mode(config, report, false, true)
}

fn public_report_with_path_mode(
    config: &Config,
    report: CheckReport,
    include_suggestions: bool,
    absolute_paths: bool,
) -> Report {
    Report {
        errors: report
            .errors
            .into_iter()
            .map(|diagnostic| public_finding(config, "error", diagnostic, absolute_paths))
            .collect(),
        warnings: report
            .warnings
            .into_iter()
            .map(|diagnostic| public_finding(config, "warning", diagnostic, absolute_paths))
            .collect(),
        // §FS-check.2.3: suggestions are surfaced only on demand. The public
        // severity tag stays `"suggestion"` so a consumer can tell them apart.
        suggestions: if include_suggestions {
            report
                .suggestions
                .into_iter()
                .map(|diagnostic| public_finding(config, "suggestion", diagnostic, absolute_paths))
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn public_finding(
    config: &Config,
    severity: &'static str,
    diagnostic: Diagnostic,
    absolute_paths: bool,
) -> Finding {
    let render_path = |path: &Path| {
        if absolute_paths {
            // This is private LSP transport data, not user-facing report text.
            // Keep the platform-native spelling so parsing it back into a
            // `Path` preserves Windows verbatim-prefix absolute paths
            // (§FS-lsp.1.1).
            absolutize_path(path).to_string_lossy().into_owned()
        } else {
            public_path(config, path)
        }
    };
    Finding {
        severity,
        code: diagnostic.code,
        path: diagnostic.path.map(|path| render_path(&path)),
        line: diagnostic.line,
        column: diagnostic.column,
        message: diagnostic.message,
        sites: diagnostic
            .sites
            .into_iter()
            .map(|site| FindingSite {
                path: render_path(&site.path),
                line: site.line,
            })
            .collect(),
    }
}

fn public_path(config: &Config, path: &Path) -> String {
    display_path(config, path)
}
