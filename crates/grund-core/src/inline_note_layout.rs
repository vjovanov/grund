// Inline note shape: what one comment line says, and whether it says it in the
// project's configured layout (§FS-inline-citation-style.2.3,
// §FS-inline-citation-style.3.3).
//
// One file for one invariant, the way `shorthand.rs` holds the number-only
// shorthand's scanner pass and checker rule together: the scanner annotates each
// inline citation site with the lines that deviate (§AR-scanner.3), the checker
// turns that list into findings at the configured level (§AR-checker.2.14), and
// both read the same classifier. Split across the two stages, the two halves of
// "what is a well-laid-out note?" could drift.

/// The layouts `[reference] inline_note_layout` selects, as the two dimensions a
/// value picks: where the citation run sits on the line, and what separates it
/// from the note (§FS-inline-citation-style.3.3). Only `citation-first-colon`
/// ships; a further value — a dash delimiter, a note-first arrangement — is one
/// more variant and one more predicate, never a second pass over the tree
/// (§DF-inline-note-layout.2.4).
#[derive(Clone, Copy, Eq, PartialEq)]
enum InlineNoteLayout {
    /// `any`: no constraint, and no line is ever classified.
    Any,
    /// The line opens with its citation run, then `delimiter`, then the note.
    CitationFirst { delimiter: char },
}

impl InlineNoteLayout {
    /// An unrecognized value cannot reach here — `[reference] inline_note_layout`
    /// is a closed enum validated on load (§FS-inline-citation-style.2.2) — so an
    /// unknown spelling falls back to the inert layout rather than inventing one.
    fn from_config(config: &Config) -> Self {
        match config.inline_note_layout.as_str() {
            "citation-first-colon" => Self::CitationFirst { delimiter: ':' },
            _ => Self::Any,
        }
    }
}

/// The separator between two citations of one run: comma, exactly one space
/// (§FS-inline-citation-style.3.3, rule 4).
const CITATION_RUN_SEPARATOR: &str = ", ";

/// The 1-based lines of one comment block that carry a citation and deviate from
/// the configured layout, ascending (§FS-inline-citation-style.3.3).
///
/// Empty — with no line classified at all — when no layout is configured, when
/// `inline_style` forbids notes outright, or when the block carries no note, since
/// a layout is a relation between a citation and a note
/// (§FS-inline-citation-style.3.3, rule 2). The default `any` therefore costs one
/// comparison per site (§GOAL-fast-feedback).
fn inline_layout_violations(
    lines: &[&str],
    first_line: usize,
    has_note: bool,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Vec<usize> {
    let layout = InlineNoteLayout::from_config(config);
    if layout == InlineNoteLayout::Any || !has_note || config.inline_style == "citation-only" {
        return Vec::new();
    }
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line_conforms(layout, line, config, workspace_targets))
        .map(|(offset, _)| first_line + offset)
        .collect()
}

/// Whether one raw comment line conforms to `layout`
/// (§FS-inline-citation-style.3.3). The line is read the way §2.3 reads it — the
/// comment prefix and any block closer stripped — and its citation tokens are the
/// ones the scanner itself recognizes there (§FS-inline-citation-style.3.3, rule 5),
/// translated into that stripped window rather than re-tokenized in it.
fn line_conforms(
    layout: InlineNoteLayout,
    line: &str,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> bool {
    let InlineNoteLayout::CitationFirst { delimiter } = layout else {
        return true;
    };
    let (start, end) = comment_content_range(line, config);
    let tokens = content_citation_tokens(line, start, end, config, workspace_targets);
    // §FS-inline-citation-style.3.3 rule 1: a line carrying no citation is unconstrained.
    if tokens.is_empty() {
        return true;
    }
    let content = &line[start..end];
    let Some(run_end) = citation_run_end(content, &tokens) else {
        return false;
    };
    let Some(rest) = content[run_end..].strip_prefix(delimiter) else {
        return false;
    };
    // `L <delimiter> ( W T | ε )`: the delimiter may end the line, and anything
    // that follows it must be separated by a space (§FS-inline-citation-style.3.3).
    rest.is_empty() || rest.starts_with(' ')
}

/// The end of the opening citation run `L`: the tokens that start the content,
/// joined by exactly `, `. `None` when the content does not open with a citation
/// token at all — the run has to sit on the line's first content byte
/// (§FS-inline-citation-style.3.3, rule 4).
fn citation_run_end(content: &str, tokens: &[(usize, usize)]) -> Option<usize> {
    let mut cursor = 0;
    let mut index = 0;
    loop {
        let (start, end) = *tokens.get(index)?;
        if start != cursor {
            // A separator that is not followed by another token ends the run
            // where the separator began, so the delimiter check below fails on
            // the separator rather than on the text after it.
            return (index > 0).then(|| cursor - CITATION_RUN_SEPARATOR.len());
        }
        cursor = end;
        index += 1;
        if content[cursor..].starts_with(CITATION_RUN_SEPARATOR) {
            cursor += CITATION_RUN_SEPARATOR.len();
        } else {
            return Some(cursor);
        }
    }
}

/// The citation tokens of `line` that fall inside its stripped content window,
/// rebased onto that window. A token straddling the window's edge — one the
/// closer cut in half — is dropped rather than reported at a shifted offset.
fn content_citation_tokens(
    line: &str,
    start: usize,
    end: usize,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Vec<(usize, usize)> {
    let mut tokens = citation_token_ranges(line, config, workspace_targets);
    tokens.sort_unstable();
    tokens.dedup();
    tokens.retain(|(token_start, token_end)| *token_start >= start && *token_end <= end);
    tokens
        .into_iter()
        .map(|(token_start, token_end)| (token_start - start, token_end - start))
        .collect()
}

/// Whether any line of a comment block carries note text — non-whitespace that is
/// neither a comment token nor part of a citation (§FS-inline-citation-style.2.3).
fn block_has_inline_note(
    lines: &[&str],
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> bool {
    lines.iter().any(|line| {
        let tokenless = remove_inline_citation_tokens(line, config, workspace_targets);
        let clean = strip_comment_tokens(&tokenless, config);
        !clean.trim().is_empty()
    })
}

fn remove_inline_citation_tokens(
    line: &str,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> String {
    let mut ranges = citation_token_ranges(line, config, workspace_targets);
    ranges.sort_unstable();
    ranges.dedup();
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0;
    let mut after_token = false;
    for (start, end) in ranges {
        if start < cursor {
            continue;
        }
        let gap = &line[cursor..start];
        // §FS-inline-citation-style.1: what joins two citations of one run is not
        // note text, so it is swallowed with them rather than left behind as prose.
        if !(after_token && is_citation_run_separator(gap)) {
            out.push_str(gap);
        }
        cursor = end;
        after_token = true;
    }
    out.push_str(&line[cursor..]);
    out
}

/// Whether the bytes strictly between two consecutive citation tokens join them
/// into one run rather than saying anything: whitespace, with at most one comma
/// (§FS-inline-citation-style.1). A second comma, or any other character, is a
/// note — `// §A + §B` says something `// §A, §B` does not.
fn is_citation_run_separator(gap: &str) -> bool {
    gap.chars().filter(|ch| *ch == ',').count() <= 1
        && gap.chars().all(|ch| ch == ',' || ch.is_whitespace())
}

fn strip_comment_tokens(line: &str, config: &Config) -> String {
    let (start, end) = comment_content_range(line, config);
    line[start..end].to_string()
}

/// The byte range of a comment line's content: what is left once the leading
/// whitespace, the comment prefix and one space after it, and any block closer are
/// removed. Returned as a range into the original line so a caller holding byte
/// offsets into that line — the citation-token ranges above — can translate them
/// instead of re-tokenizing the stripped copy and risking a different answer.
fn comment_content_range(line: &str, config: &Config) -> (usize, usize) {
    let body_start = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(line.len());
    let mut offset = body_start;
    let mut rest = &line[body_start..];
    for prefix in comment_strip_prefixes(config) {
        if let Some(stripped) = rest.strip_prefix(prefix) {
            let after = stripped.strip_prefix(' ').unwrap_or(stripped);
            offset += rest.len() - after.len();
            rest = after;
            break;
        }
    }
    let trimmed_end = rest.trim_end();
    let rest = trimmed_end
        .strip_suffix("*/")
        .or_else(|| trimmed_end.strip_suffix("\"\"\""))
        .or_else(|| trimmed_end.strip_suffix("'''"))
        .unwrap_or(trimmed_end);
    (offset, offset + rest.len())
}

fn comment_strip_prefixes(config: &Config) -> Vec<&str> {
    let mut prefixes = vec!["/**", "/*", "*/", "\"\"\"", "'''"];
    for prefix in &config.comment_prefixes {
        if prefix == "//" {
            prefixes.extend(["///", "//!", "//"]);
        } else if prefix == "/*" {
            prefixes.extend(["/*", "*"]);
        } else if !prefix.is_empty() {
            prefixes.push(prefix.as_str());
        }
    }
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    prefixes.dedup();
    prefixes
}

/// §AR-checker.2.14: the inline citation style rule, a pure pass over
/// `findings.citations` deduplicated by site. The budgets and the note-presence
/// verdict are read off the site the scanner recorded, and so are the per-line
/// layout deviations, so nothing here re-reads a file
/// (§FS-inline-citation-style.4).
fn check_inline_citation_style(findings: &Findings, config: &Config, report: &mut CheckReport) {
    let mut seen = BTreeSet::new();
    for cite in &findings.citations {
        let Some(site) = &cite.inline_site else {
            continue;
        };
        if !seen.insert((cite.file.clone(), site.clone())) {
            continue;
        }
        match config.inline_style.as_str() {
            "citation-only" => {
                if site.has_note {
                    report.errors.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        column: None,
                        message: "inline citation must carry no prose".to_string(),
                        sites: Vec::new(),
                    });
                }
            }
            _ => {
                let lines = site.last_line - site.first_line + 1;
                if lines > config.inline_note_max_lines {
                    report.errors.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        column: None,
                        message: format!(
                            "inline note exceeds {}-line maximum",
                            config.inline_note_max_lines
                        ),
                        sites: Vec::new(),
                    });
                }
                if site.max_columns > config.inline_note_max_columns {
                    report.errors.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        column: None,
                        message: format!(
                            "inline note exceeds {}-column maximum",
                            config.inline_note_max_columns
                        ),
                        sites: Vec::new(),
                    });
                }
                if config.warn_on_suggested
                    && lines > config.inline_note_suggested_lines
                    && lines <= config.inline_note_max_lines
                {
                    report.warnings.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        column: None,
                        message: format!(
                            "inline note exceeds {}-line preferred limit",
                            config.inline_note_suggested_lines
                        ),
                        sites: Vec::new(),
                    });
                }
                report_layout_deviations(cite, site, config, report);
            }
        }
    }
}

/// §FS-inline-citation-style.4.4: one finding per nonconforming line, anchored at
/// that line rather than at the site's opener — the only member of this rule that
/// does, because a layout deviation is a property of the line an author edits. The
/// level picks the channel and nothing else: the message is identical under `warn`
/// and `error`, so migrating a project between them re-reads nothing.
fn report_layout_deviations(
    cite: &Citation,
    site: &InlineCitationSite,
    config: &Config,
    report: &mut CheckReport,
) {
    let channel = match config.inline_note_layout_check.as_str() {
        "warn" => &mut report.warnings,
        "error" => &mut report.errors,
        _ => return,
    };
    for line in &site.layout_violations {
        channel.push(Diagnostic {
            code: "inline-citation-style",
            path: Some(cite.file.clone()),
            line: Some(*line),
            column: None,
            message: layout_violation_message(config),
            sites: Vec::new(),
        });
    }
}

/// The one message this rule emits, built with the configured marker so the form
/// it names is the form the project writes (§FS-inline-citation-style.4.4).
fn layout_violation_message(config: &Config) -> String {
    format!(
        "inline note must open with its citations and a colon ({}<ID>: note)",
        config.marker
    )
}

/// §FS-inline-citation-style.5: the sentence the managed agent-entrypoint block
/// appends when a layout is configured, written with the project's own marker and
/// `<ID>` placeholders so the example is a shape rather than a live citation.
/// Empty under `any`, and the same at every `inline_note_layout_check` — the
/// house style is what the agent is asked to write, and the gate it is measured
/// by is not an instruction (§DF-inline-note-layout.2.1).
fn inline_note_layout_sentence(config: &Config) -> String {
    let marker = &config.marker;
    match config.inline_note_layout.as_str() {
        "citation-first-colon" => format!(
            " Lay each note out citation-first: `// {marker}<ID>: <note>` (several citations: `// {marker}<ID>, {marker}<ID>: <note>`)."
        ),
        _ => String::new(),
    }
}
