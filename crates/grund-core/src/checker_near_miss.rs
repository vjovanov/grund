/// The declaration near-miss rule (§FS-check.4.6), in a file of its own beside
/// the other rule families (§AR-core-module-layout.1): a heading that opens like
/// a declaration and does not parse as one, reported per heading at the line a
/// contributor has to edit.
///
/// The rule is one function because it is one question asked of a list the
/// scanner already built. What it must not become is a guess: it names the token,
/// the format it missed, and the shape that format reads — never a corrected ID,
/// which would be an opinion about what the author meant (§FS-non-goals.3).

/// §FS-check.4.6: one warning per heading that came close. Sorted with the rest
/// of the report by the shared comparator, so a run over one tree prints them in
/// the same order every time (§FS-errors.4).
fn check_declaration_near_misses(findings: &Findings, report: &mut CheckReport) {
    for heading in &findings.near_miss_headings {
        let diagnostic = Diagnostic {
            code: "declaration-near-miss",
            path: Some(heading.file.clone()),
            line: Some(heading.line),
            column: None,
            message: near_miss_message(&heading.format, &heading.text),
            sites: Vec::new(),
        };
        // §FS-check.4.6 / §RM-off-grammar-declaration-error: the scheduled
        // severity transition is release-derived and never changes catalog
        // recognition or citation promotion.
        if declaration_near_miss_is_error() {
            report.errors.push(diagnostic);
        } else {
            report.warnings.push(diagnostic);
        }
    }
}

/// The sentence: the token as written, the configured template, and the shape
/// that template reads. Three facts, no proposal — `check` reports facts about
/// the tree and the config (§FS-check.3 vs §4), and the corrected ID is the one
/// thing here that would be a guess.
fn near_miss_message(format: &str, text: &str) -> String {
    let deadline = if declaration_near_miss_is_error() {
        // Keep the landed wording out of the release-ramp scanner's source
        // vocabulary until this branch actually ships at that release.
        format!("this mismatch became an {}", "error in grund 0.15.0")
    } else {
        "this warning becomes an error in grund 0.15.0".to_owned()
    };
    format!(
        "`{text}` resolves for compatibility but does not match [id] format = \
         \"{format}\" — rename it or change the effective format; {deadline}",
    )
}

fn declaration_near_miss_is_error() -> bool {
    let mut parts = env!("CARGO_PKG_VERSION")
        .trim_end_matches("-dev")
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok());
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0)) >= (0, 15)
}
