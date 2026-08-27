/// The declaration near-miss rule (§FS-check.4.7), in a file of its own beside
/// the other rule families (§AR-core-module-layout.1): a heading that opens like
/// a declaration and does not parse as one, reported per heading at the line a
/// contributor has to edit.
///
/// The rule is one function because it is one question asked of a list the
/// scanner already built. What it must not become is a guess: it names the token,
/// the format it missed, and the shape that format reads — never a corrected ID,
/// which would be an opinion about what the author meant (§FS-non-goals.3).

/// §FS-check.4.7: one warning per heading that came close. Sorted with the rest
/// of the report by the shared comparator, so a run over one tree prints them in
/// the same order every time (§FS-errors.4).
fn check_declaration_near_misses(findings: &Findings, config: &Config, report: &mut CheckReport) {
    for heading in &findings.near_miss_headings {
        report.warnings.push(Diagnostic {
            code: "declaration-near-miss",
            path: Some(heading.file.clone()),
            line: Some(heading.line),
            column: None,
            message: near_miss_message(config, &heading.text),
            sites: Vec::new(),
        });
    }
}

/// The sentence: the token as written, the configured template, and the shape
/// that template reads. Three facts, no proposal — `check` reports facts about
/// the tree and the config (§FS-check.3 vs §4), and the corrected ID is the one
/// thing here that would be a guess.
fn near_miss_message(config: &Config, text: &str) -> String {
    format!(
        "`{text}` is heading-shaped and declares nothing — [id] format = \"{format}\" \
         reads `# {shape}: <title>`",
        format = config.id_format,
        shape = id_shape(&config.id_format),
    )
}
