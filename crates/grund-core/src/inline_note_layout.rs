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
/// `inline_style` forbids notes outright, when the block carries no note, since a
/// layout is a relation between a citation and a note
/// (§FS-inline-citation-style.3.3, rule 2), or when `inline_note_layout_check` is
/// `off` and the verdicts would reach no channel (§FS-inline-citation-style.4.4).
/// The default `any` — and a documented-only layout — therefore costs one
/// comparison per site (§GOAL-fast-feedback): the guard below is read before a
/// single line is looked at, so no line is tokenized on its account.
fn inline_layout_violations(
    block: &mut BlockCitations<'_>,
    first_line: usize,
    has_note: bool,
) -> Vec<usize> {
    let config = block.config;
    let layout = InlineNoteLayout::from_config(config);
    if layout == InlineNoteLayout::Any
        || !has_note
        || config.inline_style == "citation-only"
        // §FS-inline-citation-style.4.4: at `off` the layout is documentation, so
        // classifying a line would buy a verdict nothing reads.
        || config.inline_note_layout_check == "off"
    {
        return Vec::new();
    }
    (0..block.len())
        .filter(|index| {
            let (line, ranges) = block.line(*index);
            !line_conforms(layout, line, ranges, config)
        })
        .map(|offset| first_line + offset)
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
    ranges: &[(usize, usize)],
    config: &Config,
) -> bool {
    let InlineNoteLayout::CitationFirst { delimiter } = layout else {
        return true;
    };
    let (start, end) = comment_content_range(line, config);
    let tokens = content_citation_tokens(ranges, start, end);
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
/// joined by exactly `, `. `None` when there is no such run — either the content
/// does not open with a citation token (the run has to sit on the line's first
/// content byte) or a `, ` inside it is followed by something that is not one
/// (§FS-inline-citation-style.3.3, rule 4). Both are deviations, so neither needs
/// an offset to report.
fn citation_run_end(content: &str, tokens: &[(usize, usize)]) -> Option<usize> {
    let mut cursor = 0;
    let mut index = 0;
    loop {
        let (start, end) = *tokens.get(index)?;
        if start != cursor {
            return None;
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

/// The line's citation tokens that fall inside its stripped content window,
/// rebased onto that window. A token straddling the window's edge — one the
/// closer cut in half — is dropped rather than reported at a shifted offset. The
/// ranges arrive already sorted and deduplicated, so this is a translation, never
/// a second reading of the line.
fn content_citation_tokens(
    ranges: &[(usize, usize)],
    start: usize,
    end: usize,
) -> Vec<(usize, usize)> {
    ranges
        .iter()
        .filter(|(token_start, token_end)| *token_start >= start && *token_end <= end)
        .map(|(token_start, token_end)| (token_start - start, token_end - start))
        .collect()
}

/// The two verdicts one comment block carries about its note: whether it says
/// anything at all (§FS-inline-citation-style.2.3) and which of its citation
/// lines say it in the wrong shape (§FS-inline-citation-style.3.3). Both answers
/// are about where this block's citation tokens sit, so both passes read one
/// tokenization of each line rather than two that could only agree
/// (§GOAL-fast-feedback). Shared, not eager: the sharing is one empty memo per
/// block — its whole up-front cost, paid at every configuration — that the passes
/// fill as they reach a line. So the two reasons a line goes unread, note
/// presence stopping at the first line that says something and the layout pass
/// not running at all, each cost an untouched slot and no tokenization.
fn inline_note_verdicts(
    lines: &[&str],
    first_line: usize,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> (bool, Vec<usize>) {
    let mut block = BlockCitations::new(lines, config, workspace_targets);
    let has_note = block_has_inline_note(&mut block);
    let layout_violations = inline_layout_violations(&mut block, first_line, has_note);
    (has_note, layout_violations)
}

/// One comment block's lines together with their citation-token byte ranges, each
/// line tokenized on the first ask and remembered for the second.
///
/// A line and its ranges are handed out by one accessor, so the two cannot be
/// misaligned by a caller that pairs them itself; and no line is tokenized until
/// a pass actually looks at it, so a block the note-presence walk leaves early —
/// or that no layout pass revisits — tokenizes only the lines that were read and
/// leaves the rest of the memo empty (§GOAL-fast-feedback).
struct BlockCitations<'a> {
    lines: &'a [&'a str],
    config: &'a Config,
    workspace_targets: &'a [WorkspaceCitationTarget],
    ranges: Vec<Option<Vec<(usize, usize)>>>,
}

impl<'a> BlockCitations<'a> {
    fn new(
        lines: &'a [&'a str],
        config: &'a Config,
        workspace_targets: &'a [WorkspaceCitationTarget],
    ) -> Self {
        Self {
            lines,
            config,
            workspace_targets,
            ranges: vec![None; lines.len()],
        }
    }

    fn len(&self) -> usize {
        self.lines.len()
    }

    /// Line `index` and its citation-token byte ranges, sorted and deduplicated,
    /// so every reader of that line sees the same tokenization.
    fn line(&mut self, index: usize) -> (&'a str, &[(usize, usize)]) {
        let (line, config, workspace_targets) =
            (self.lines[index], self.config, self.workspace_targets);
        let ranges = self.ranges[index]
            .get_or_insert_with(|| line_citation_ranges(line, config, workspace_targets));
        (line, ranges)
    }
}

fn line_citation_ranges(
    line: &str,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Vec<(usize, usize)> {
    let mut ranges = citation_token_ranges(line, config, workspace_targets);
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

/// Whether any line of a comment block carries note text — non-whitespace that is
/// neither a comment token nor part of a citation (§FS-inline-citation-style.2.3).
/// The walk stops at the first line that says something, so the block is read only
/// as far as the answer needs it.
fn block_has_inline_note(block: &mut BlockCitations<'_>) -> bool {
    let config = block.config;
    (0..block.len()).any(|index| {
        let (line, ranges) = block.line(index);
        let tokenless = remove_inline_citation_tokens(line, ranges);
        let clean = strip_comment_tokens(&tokenless, config);
        !clean.trim().is_empty()
    })
}

fn remove_inline_citation_tokens(line: &str, ranges: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0;
    let mut after_token = false;
    for (start, end) in ranges.iter().copied() {
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
            // §FS-inline-citation-style.3.3: whatever indents the content past the
            // prefix goes with the prefix. A wrapped Rustdoc continuation, an
            // aligned ` * ` filler, and a tab after `#` are comment formatting;
            // the layout starts at the first byte that says something.
            let after = stripped.trim_start_matches([' ', '\t']);
            offset += rest.len() - after.len();
            rest = after;
            break;
        }
    }
    let trimmed_end = rest.trim_end();
    // Trimmed again after the closer: `/* §<ID>: */` must leave `§<ID>:`, so a
    // colon that ends the content is the grammar's empty tail rather than a space
    // the closer happened to sit behind (§FS-inline-citation-style.3.3).
    let rest = trimmed_end
        .strip_suffix("*/")
        .or_else(|| trimmed_end.strip_suffix("\"\"\""))
        .or_else(|| trimmed_end.strip_suffix("'''"))
        .unwrap_or(trimmed_end)
        .trim_end();
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
