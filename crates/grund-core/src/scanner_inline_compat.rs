/// Reconcile promoted persisted citations with inline-note classification
/// (§FS-inline-citation-style.3.1, §FS-config.3.2). Kept beside scanner
/// compatibility because this is the post-catalog half of that pass, not a new
/// inline-style rule.

/// Refresh note-presence and layout from the final citation set after a legacy
/// candidate becomes catalog-backed. The file pass retained this one affected
/// block, so overlays and LSP scans use the same bytes and promotion never
/// re-reads the filesystem.
fn reconcile_promoted_inline_site(
    config: &Config,
    citations: &mut [Citation],
    file: &Path,
    mut site: InlineCitationSite,
    block_lines: &[String],
) {
    let mut ranges = vec![Vec::new(); block_lines.len()];
    for citation in citations.iter() {
        if citation.file != file
            || citation.line < site.first_line
            || citation.line > site.last_line
            || citation
                .inline_site
                .as_ref()
                .is_none_or(|candidate_site| candidate_site.first_line != site.first_line)
        {
            continue;
        }
        let start = citation.column.saturating_sub(1);
        ranges[citation.line - site.first_line].push((start, start + citation.text.len()));
    }
    for line_ranges in &mut ranges {
        line_ranges.sort_unstable();
        line_ranges.dedup();
    }
    let lines = block_lines.iter().map(String::as_str).collect::<Vec<_>>();
    let (has_note, layout_violations) =
        inline_note_verdicts_from_ranges(&lines, site.first_line, config, ranges);
    site.has_note = has_note;
    site.layout_violations = layout_violations;
    for citation in citations.iter_mut() {
        if citation.file == file
            && citation
                .inline_site
                .as_ref()
                .is_some_and(|candidate_site| candidate_site.first_line == site.first_line)
        {
            citation.inline_site = Some(site.clone());
        }
    }
}

fn inline_note_verdicts_from_ranges(
    lines: &[&str],
    first_line: usize,
    config: &Config,
    ranges: Vec<Vec<(usize, usize)>>,
) -> (bool, Vec<usize>) {
    let prefixes = comment_strip_prefixes(config);
    if !layout_pass_enabled(config) {
        let has_note = lines
            .iter()
            .zip(&ranges)
            .any(|(line, ranges)| line_says_something(line, ranges, &prefixes));
        return (has_note, Vec::new());
    }
    let mut block = BlockCitations {
        lines,
        config,
        workspace_targets: &[],
        ranges: ranges.into_iter().map(Some).collect(),
    };
    let has_note = block_has_inline_note_memoized(&mut block, &prefixes);
    let layout_violations = inline_layout_violations(&mut block, &prefixes, first_line, has_note);
    (has_note, layout_violations)
}
