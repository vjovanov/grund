/// The layouts `[reference] inline_note_layout` selects, as the two dimensions a
/// value picks: where the citation run sits on the line, and what separates it
/// from the note (§FS-inline-citation-style.3.3). Only `citation-first-colon`
/// ships; a further value — a dash delimiter, a note-first arrangement — is one
/// more variant and one more predicate, never a second pass over the tree
/// (§DF-inline-note-layout.2.4).
///
/// Inline note shape: what one comment line says, and whether it says it in the
/// project's configured layout (§FS-inline-citation-style.2.3,
/// §FS-inline-citation-style.3.3).
///
/// One file for one invariant, the way `shorthand.rs` holds the number-only
/// shorthand's scanner pass and checker rule together: the scanner annotates each
/// inline citation site with the lines that deviate (§AR-scanner.3), the checker
/// turns that list into findings at the configured level (§AR-checker.2.14), and
/// both read the same classifier. Split across the two stages, the two halves of
/// "what is a well-laid-out note?" could drift.
///
/// What is *not* that invariant sits in `comment_line.rs`: reducing one line to
/// its content and its citation tokens is the same question for note presence and
/// for layout, and for the scanner walk that asked it first.
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

/// The report channel a layout deviation reaches (§FS-inline-citation-style.4.4).
/// One answer read by both stages — the scanner to decide whether classifying a
/// line buys anything, the checker to decide where the verdict goes — so a value
/// outside the enum cannot leave one of them working while the other discards
/// the result. The load-time check rejects such a value
/// (§FS-inline-citation-style.2.2); this is what keeps the two halves agreeing
/// anyway, for a `Config` built in memory.
#[derive(Clone, Copy, Eq, PartialEq)]
enum LayoutChannel {
    Warn,
    Error,
}

fn layout_channel(config: &Config) -> Option<LayoutChannel> {
    match config.inline_note_layout_check.as_str() {
        "warn" => Some(LayoutChannel::Warn),
        "error" => Some(LayoutChannel::Error),
        _ => None,
    }
}

/// Whether any line in this run will be classified at all: a layout to deviate
/// from, notes permitted, and a channel for the verdict to reach. Config-only,
/// so it is the same answer for every block and is read once per block before a
/// line is looked at — the default `any`, a layout left at `off`, and
/// `citation-only` each cost this comparison and nothing else
/// (§GOAL-fast-feedback).
fn layout_pass_enabled(config: &Config) -> bool {
    InlineNoteLayout::from_config(config) != InlineNoteLayout::Any
        && config.inline_style != "citation-only"
        && layout_channel(config).is_some()
}

/// The separator between two citations of one run: comma, exactly one space
/// (§FS-inline-citation-style.3.3, rule 4).
const CITATION_RUN_SEPARATOR: &str = ", ";

/// The 1-based lines of one comment block that are judged by the configured
/// layout and deviate from it, ascending (§FS-inline-citation-style.3.3).
///
/// Called only where `layout_pass_enabled` already said a verdict has somewhere
/// to go, so the remaining exemption is the block's own: a site with no note is
/// a pure pointer, and a layout is a relation between a citation and a note
/// (§FS-inline-citation-style.3.3, rule 2).
///
/// Which lines are judged is rule 1: the first line whose content carries a
/// citation opens the note and is always judged; a later one is judged only when
/// it *opens* with a citation, since a line that opens with prose and names an ID
/// further along is the continuation of a note that already opened correctly.
fn inline_layout_violations(
    block: &mut BlockCitations<'_>,
    prefixes: &[&str],
    first_line: usize,
    has_note: bool,
) -> Vec<usize> {
    if !has_note {
        return Vec::new();
    }
    let layout = InlineNoteLayout::from_config(block.config);
    let mut violations = Vec::new();
    let mut note_opened = false;
    for index in 0..block.len() {
        let (line, ranges) = block.line(index);
        let Some((content, tokens)) = line_layout_view(line, ranges, prefixes) else {
            continue;
        };
        let opens_with_citation = tokens.first().is_some_and(|(start, _)| *start == 0);
        let judged = !note_opened || opens_with_citation;
        note_opened = true;
        if judged && !content_conforms(layout, content, &tokens) {
            violations.push(first_line + index);
        }
    }
    violations
}

/// One line reduced to what this rule reads: its content window with any leading
/// list marker skipped, and the citation tokens inside that window rebased onto
/// it. The window is the one §2.3 uses — comment prefix and block closer stripped
/// — and the tokens are the ones the scanner itself recognizes on the line
/// (§FS-inline-citation-style.3.3, rule 5), translated rather than re-tokenized
/// in the stripped copy. `None` when no citation falls inside the window, which
/// is rule 1's unconstrained line.
fn line_layout_view<'a>(
    line: &'a str,
    ranges: &[(usize, usize)],
    prefixes: &[&str],
) -> Option<(&'a str, Vec<(usize, usize)>)> {
    let (start, end) = comment_content_range(line, prefixes);
    let content_start = start + list_marker_len(&line[start..end]);
    let tokens = content_citation_tokens(ranges, content_start, end);
    (!tokens.is_empty()).then(|| (&line[content_start..end], tokens))
}

/// The form itself: `L <delimiter> ( W T | ε )` over one line's content
/// (§FS-inline-citation-style.3.3).
fn content_conforms(layout: InlineNoteLayout, content: &str, tokens: &[(usize, usize)]) -> bool {
    let InlineNoteLayout::CitationFirst { delimiter } = layout else {
        return true;
    };
    let Some(run_end) = citation_run_end(content, tokens) else {
        return false;
    };
    let Some(rest) = content[run_end..].strip_prefix(delimiter) else {
        return false;
    };
    // The delimiter may end the line, and anything that follows it must be
    // separated by a space (§FS-inline-citation-style.3.3, rule 4).
    rest.is_empty() || rest.starts_with(' ')
}

/// The byte length of a leading Markdown list marker and the spaces behind it,
/// or `0` when the content does not open with one
/// (§FS-inline-citation-style.3.3). A bullet is item structure rather than note
/// text, so an enumerated block of grounded points can open each item with its
/// citation run. One marker is skipped, never a chain, and only where a space
/// follows — `-§<ID>` opens with a `-`.
fn list_marker_len(content: &str) -> usize {
    let after_marker = match content.strip_prefix(['-', '*', '+']) {
        Some(rest) => rest,
        None => {
            let Some(digits) = content.find(|ch: char| !ch.is_ascii_digit()) else {
                return 0;
            };
            match content[digits..].strip_prefix(['.', ')']) {
                Some(rest) if digits > 0 => rest,
                _ => return 0,
            }
        }
    };
    let spaces = after_marker.len() - after_marker.trim_start_matches(' ').len();
    if spaces == 0 {
        return 0;
    }
    content.len() - after_marker.len() + spaces
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
        index += 1;
        // A token the run has already passed is one the tokenizer produced twice
        // over the same bytes, not a break in the run — the same reading
        // `remove_inline_citation_tokens` takes of an overlap.
        if start < cursor {
            continue;
        }
        if start != cursor {
            return None;
        }
        cursor = end;
        if content[cursor..].starts_with(CITATION_RUN_SEPARATOR) {
            cursor += CITATION_RUN_SEPARATOR.len();
        } else {
            return Some(cursor);
        }
    }
}

/// The two verdicts one comment block carries about its note: whether it says
/// anything at all (§FS-inline-citation-style.2.3) and which of its citation
/// lines say it in the wrong shape (§FS-inline-citation-style.3.3).
///
/// Whether a second reader exists is a config fact, so it is settled here, once
/// per block, before a line is touched. Where one does, both passes read one
/// tokenization of each line through a memo the walks fill as they reach a line —
/// two readings that could only agree, at the price of one `Vec` per block. Where
/// none does, which is every default-configured tree, the note walk reads each
/// line once and keeps nothing, so the memo is never allocated
/// (§GOAL-fast-feedback).
fn inline_note_verdicts(
    lines: &[&str],
    first_line: usize,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> (bool, Vec<usize>) {
    let prefixes = comment_strip_prefixes(config);
    if !layout_pass_enabled(config) {
        return (
            block_has_inline_note(lines, config, workspace_targets, &prefixes),
            Vec::new(),
        );
    }
    let mut block = BlockCitations::new(lines, config, workspace_targets);
    let has_note = block_has_inline_note_memoized(&mut block, &prefixes);
    let layout_violations = inline_layout_violations(&mut block, &prefixes, first_line, has_note);
    (has_note, layout_violations)
}

/// One comment block's lines together with their citation-token byte ranges, each
/// line tokenized on the first ask and remembered for the second.
///
/// Built only where a second ask exists — a configured layout with a channel to
/// report through (`layout_pass_enabled`) — so the memo is a cost of the rule
/// rather than of scanning. A line and its ranges are handed out by one accessor,
/// so the two cannot be misaligned by a caller that pairs them itself; and no line
/// is tokenized until a pass actually looks at it, so a block the note-presence
/// walk leaves early tokenizes only the lines that were read and leaves the rest
/// of the memo empty (§GOAL-fast-feedback).
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

/// Whether any line of a comment block carries note text — non-whitespace that is
/// neither a comment token nor part of a citation (§FS-inline-citation-style.2.3).
/// The walk stops at the first line that says something, so the block is read only
/// as far as the answer needs it.
fn block_has_inline_note(
    lines: &[&str],
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
    prefixes: &[&str],
) -> bool {
    lines.iter().any(|line| {
        let ranges = line_citation_ranges(line, config, workspace_targets);
        line_says_something(line, &ranges, prefixes)
    })
}

/// The same walk where a layout pass will read these lines again, so each
/// tokenization is kept rather than dropped.
fn block_has_inline_note_memoized(block: &mut BlockCitations<'_>, prefixes: &[&str]) -> bool {
    (0..block.len()).any(|index| {
        let (line, ranges) = block.line(index);
        line_says_something(line, ranges, prefixes)
    })
}

/// Whether one line says anything once its comment tokens and its citations are
/// taken out of it (§FS-inline-citation-style.2.3).
fn line_says_something(line: &str, ranges: &[(usize, usize)], prefixes: &[&str]) -> bool {
    let tokenless = remove_inline_citation_tokens(line, ranges);
    !strip_comment_tokens(&tokenless, prefixes).trim().is_empty()
}

/// §AR-checker.2.14: the inline citation style rule, a pure pass over
/// `findings.citations` deduplicated by site. The budgets and the note-presence
/// verdict are read off the site the scanner recorded, and so are the per-line
/// layout deviations, so nothing here re-reads a file
/// (§FS-inline-citation-style.4).
fn check_inline_citation_style(findings: &Findings, config: &Config, report: &mut CheckReport) {
    // A site is identified by the file and the line it opens on — two blocks in
    // one file cannot share an opener — so the key stays two cheap fields rather
    // than a clone of the whole recorded site.
    let mut seen = BTreeSet::new();
    let layout_message = layout_violation_message(config);
    for cite in &findings.citations {
        let Some(site) = &cite.inline_site else {
            continue;
        };
        if !seen.insert((cite.file.as_path(), site.first_line)) {
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
                report_layout_deviations(cite, site, config, &layout_message, report);
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
    message: &str,
    report: &mut CheckReport,
) {
    let channel = match layout_channel(config) {
        Some(LayoutChannel::Warn) => &mut report.warnings,
        Some(LayoutChannel::Error) => &mut report.errors,
        None => return,
    };
    for line in &site.layout_violations {
        channel.push(Diagnostic {
            code: "inline-citation-style",
            path: Some(cite.file.clone()),
            line: Some(*line),
            column: None,
            message: message.to_string(),
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
