/// Every recognized citation token on one line, as byte ranges into it
/// (§FS-check.1.1): the configured marker, `[reference] strict`, the
/// string-literal exclusion, and workspace-qualified `§<alias>/<ID>` tokens
/// (§FS-workspace.1). Ranges may repeat and may arrive in either pass's order —
/// `line_citation_ranges` is what makes them a set.
///
/// One comment line, reduced: where its citation tokens sit, and what it still
/// says once those tokens and its comment punctuation are taken out
/// (§FS-inline-citation-style.1, §FS-inline-citation-style.2.3).
///
/// These are the pure per-line functions the scanner's walk used to carry
/// (§AR-scanner.2.3): they hold no state, read no block, and are coupled to their
/// callers only by call. They live here because two rules now share them — note
/// presence and inline note layout (`inline_note_layout.rs`) both need the same
/// answer about the same line, and a second copy of "what does this line say?"
/// is how two rules come to disagree about one comment.
fn citation_token_ranges(
    line: &str,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for caps in config.grammar.citation_re.captures_iter(line) {
        let Some(full) = caps.get(0) else { continue };
        let namespace = caps.name("namespace");
        let has_marker = line[..full.start()].ends_with(&config.marker);
        if config.grammar.has_reserved_named_tail(line, full.end()) {
            continue;
        }
        if namespace.is_some() && !has_marker {
            continue;
        }
        if config.strict && !has_marker {
            continue;
        }
        if !has_marker
            && config
                .grammar
                .is_named_section(caps.name("sec").map(|sec| sec.as_str()))
        {
            continue;
        }
        if !has_marker && is_inside_string_literal(line, full.start()) {
            continue;
        }
        let start = if has_marker {
            full.start().saturating_sub(config.marker.len())
        } else {
            full.start()
        };
        if namespace.is_some()
            && has_marker
            && (is_inside_inline_code(line, start) || is_inside_string_literal(line, start))
        {
            continue;
        }
        ranges.push((start, full.end()));
    }

    if config.marker.is_empty() {
        return ranges;
    }
    for (marker_start, _) in line.match_indices(&config.marker) {
        if ranges.iter().any(|(start, _)| *start == marker_start)
            || is_inside_string_literal(line, marker_start)
        {
            continue;
        }
        let token_start = marker_start + config.marker.len();
        let Some(rest) = line.get(token_start..) else {
            continue;
        };
        let Some(prefix) = QUALIFIED_CITATION_PREFIX.captures(rest) else {
            continue;
        };
        let Some(alias) = prefix.name("namespace").map(|m| m.as_str()) else {
            continue;
        };
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = line.get(id_start..) else {
            continue;
        };
        let parsed = if workspace_targets.is_empty() {
            parse_loose_qualified_id_prefix(id_rest).map(|(_, _, len)| len)
        } else {
            match workspace_targets.iter().find(|target| target.alias == alias) {
                Some(target) => parse_longest_id_prefix(id_rest, &target.config.grammar),
                None => workspace_targets
                    .iter()
                    .find_map(|target| parse_longest_id_prefix(id_rest, &target.config.grammar)),
            }
            .map(|parsed| parsed.len)
        };
        let Some(id_len) = parsed else {
            continue;
        };
        ranges.push((marker_start, id_start + id_len));
    }
    ranges
}

/// One line's citation tokens, sorted and deduplicated, so every reader of that
/// line sees the same tokenization.
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

/// The line's citation tokens that fall inside a stripped content window, rebased
/// onto that window. A token straddling the window's edge — one the closer cut in
/// half — is dropped rather than reported at a shifted offset. The ranges arrive
/// already sorted and deduplicated, so this is a translation, never a second
/// reading of the line.
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

fn strip_comment_tokens<'a>(line: &'a str, prefixes: &[&str]) -> &'a str {
    let (start, end) = comment_content_range(line, prefixes);
    &line[start..end]
}

/// The byte range of a comment line's content: what is left once the leading
/// whitespace, the comment prefix and one space after it, and any block closer are
/// removed. Returned as a range into the original line so a caller holding byte
/// offsets into that line — the citation-token ranges above — can translate them
/// instead of re-tokenizing the stripped copy and risking a different answer.
///
/// `prefixes` is `comment_strip_prefixes(config)`, built once per block and passed
/// down: it is a pure function of the configured comment prefixes, so rebuilding
/// and re-sorting it per line — twice per line, once the layout pass runs — bought
/// nothing (§GOAL-fast-feedback).
fn comment_content_range(line: &str, prefixes: &[&str]) -> (usize, usize) {
    let body_start = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(line.len());
    let mut offset = body_start;
    let mut rest = &line[body_start..];
    for prefix in prefixes {
        if let Some(stripped) = rest.strip_prefix(*prefix) {
            // §FS-inline-citation-style.3.3: whatever indents the content past the
            // prefix goes with it: a wrapped Rustdoc continuation, an aligned ` * `
            // filler, a tab after `#`. Content starts at the first byte that says something.
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
