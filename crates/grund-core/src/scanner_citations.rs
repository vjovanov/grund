/// §FS-workspace.5: a member-local scan must still recognize marker-qualified
/// citations before the member's own ID grammar is applied. Without this
/// fallback, `§root/FS-root` in a default member can disappear just because the
/// root uses `{kind}-{slug}`.
///
/// The fallback parses the ID tail with the conventional `KIND[-NUM]-SLUG`
/// shape (`parse_loose_qualified_id_prefix`), not the citing or any target
/// project's configured `[id] format`. Member-local scans have no workspace
/// catalogue, so the target's grammar is unreachable here. The tradeoff:
/// non-default ID grammars (lowercase kinds, slug-only shapes that don't
/// separate on `-`/`_`, kinds with characters outside `[A-Z0-9]`) won't be
/// recognised as qualified citations at member scope and will fall through
/// to be diagnosed at the workspace-root run instead. Workspace-root and
/// workspace-aware paths use the target's actual grammar via
/// `scan_workspace_qualified_pass`.
///
/// `qualified_claimed` carries the marker offsets a qualified citation already
/// exists at — the full-ID pass's on entry, this pass's own on return. The
/// shorthand pass reads the union to decide whether a qualified marker is
/// already spoken for (§AR-scanner.2.6).
fn scan_fallback_qualified_citations(
    line: &CitationLine<'_>,
    qualified_claimed: &mut BTreeSet<usize>,
    findings: &mut Findings,
) {
    if line.config.marker.is_empty() {
        return;
    }
    for (marker_start, _) in line.scan_line.match_indices(&line.config.marker) {
        if qualified_claimed.contains(&marker_start) {
            continue;
        }
        if qualified_suppressed_in_source(line.scan_line, line.is_md, marker_start) {
            continue;
        }
        let token_start = marker_start + line.config.marker.len();
        let Some(rest) = line.scan_line.get(token_start..) else {
            continue;
        };
        let Some(prefix) = QUALIFIED_CITATION_PREFIX.captures(rest) else {
            continue;
        };
        let Some(alias) = prefix.name("namespace").map(|m| m.as_str()) else {
            continue;
        };
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = line.scan_line.get(id_start..) else {
            continue;
        };
        let Some((id, section, id_len)) = parse_loose_qualified_id_prefix(id_rest) else {
            continue;
        };
        let token_end = id_start + id_len;
        qualified_claimed.insert(marker_start);
        findings.citations.push(Citation {
            namespace: Some(alias.to_string()),
            id,
            section,
            file: line.path.to_path_buf(),
            line: line.lineno,
            column: line.column_offset + marker_start + 1,
            has_marker: true,
            // The loose parser has no target grammar to derive a shorthand from,
            // so a fallback-parsed qualified citation is never one (§AR-scanner.2.6).
            shorthand: false,
            shorthand_rewritable: true,
            numeric_run: false,
            text: line.scan_line[marker_start..token_end].to_string(),
            inline_site: line.inline_sites.get(&line.lineno).cloned(),
            // §AR-scanner.2.4: classified in the post-pass in `scan_file`.
            source_kind: String::new(),
            enclosing_declaration: None,
        });
    }
}

/// The member-local fallback ID parser (§FS-workspace.5). Recognises the
/// conventional `KIND[-NUM]-SLUG` shape — uppercase-or-digit kind, optional
/// numeric middle component, non-empty slug — because the member has no
/// access to the citing or target project's `[id] format` at this point.
/// A workspace-root run uses `parse_longest_id_prefix` with the target's
/// grammar (`scan_workspace_qualified_pass`) and is not affected by this
/// fallback's assumptions.
fn parse_loose_qualified_id_prefix(raw: &str) -> Option<(Id, Option<String>, usize)> {
    let mut end = raw
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
        .map(|(idx, _)| idx)
        .unwrap_or(raw.len());
    while end > 0
        && raw[..end]
            .chars()
            .next_back()
            .is_some_and(|ch| matches!(ch, '.' | ',' | ';' | ':' | '!' | '?'))
    {
        end -= raw[..end].chars().next_back().map(char::len_utf8).unwrap_or(1);
    }
    let token = raw.get(..end)?;
    let (id_text, section) = split_loose_section(token);
    let (kind, rest) = id_text
        .split_once(['-', '_'])
        .filter(|(kind, rest)| !kind.is_empty() && !rest.is_empty())?;
    if !kind.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()) {
        return None;
    }
    let (num, slug) = match rest.split_once(['-', '_']) {
        Some((maybe_num, slug)) if maybe_num.chars().all(|ch| ch.is_ascii_digit()) => {
            (maybe_num.parse::<u32>().ok(), slug)
        }
        _ => (None, rest),
    };
    if slug.is_empty() {
        return None;
    }
    Some((
        Id {
            kind: kind.to_string(),
            num,
            slug: Some(slug.to_string()),
        },
        section.map(str::to_string),
        end,
    ))
}

fn split_loose_section(token: &str) -> (&str, Option<&str>) {
    let suffix_start = token
        .char_indices()
        .rev()
        .find(|(_, ch)| !(ch.is_ascii_digit() || *ch == '.'))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);
    let suffix = &token[suffix_start..];
    let Some(section) = suffix.strip_prefix('.') else {
        return (token, None);
    };
    if section.is_empty()
        || !section
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return (token, None);
    }
    (&token[..suffix_start], Some(section))
}

/// One line's worth of marker-qualified workspace citations: a `§<alias>/<ID>`
/// token whose ID tail parses with the target project's grammar
/// (§FS-workspace.1, §AR-workspace.2). Runs inline during `scan_file` in
/// workspace mode so the file is read once, not twice.
fn scan_workspace_qualified_pass(
    line: &CitationLine<'_>,
    targets: &[WorkspaceCitationTarget],
    findings: &mut Findings,
) {
    if line.config.marker.is_empty() || targets.is_empty() {
        return;
    }
    for (marker_start, _) in line.scan_line.match_indices(&line.config.marker) {
        if qualified_suppressed_in_source(line.scan_line, line.is_md, marker_start) {
            continue;
        }
        let token_start = marker_start + line.config.marker.len();
        let Some(rest) = line.scan_line.get(token_start..) else {
            continue;
        };
        let Some(prefix) = QUALIFIED_CITATION_PREFIX.captures(rest) else {
            continue;
        };
        let Some(alias) = prefix.name("namespace").map(|m| m.as_str()) else {
            continue;
        };
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = line.scan_line.get(id_start..) else {
            continue;
        };
        // The winning target is kept, not just its parse: §FS-fmt.2.4.1 asks the
        // numeric-run question with the *target's* number shape, the same grammar
        // that claimed the token.
        let parsed = match targets.iter().find(|target| target.alias == alias) {
            Some(target) => parse_longest_id_prefix(id_rest, &target.config.grammar)
                .map(|parsed| (parsed, &target.config)),
            None => targets.iter().find_map(|target| {
                parse_longest_id_prefix(id_rest, &target.config.grammar)
                    .map(|parsed| (parsed, &target.config))
            }),
        };
        let Some((parsed, target_config)) = parsed else {
            continue;
        };
        if target_config
            .grammar
            .has_reserved_named_tail(id_rest, parsed.len)
        {
            continue;
        }
        let token_end = id_start + parsed.len;
        findings.citations.push(Citation {
            namespace: Some(alias.to_string()),
            id: parsed.id,
            section: parsed.section,
            file: line.path.to_path_buf(),
            line: line.lineno,
            column: line.column_offset + marker_start + 1,
            has_marker: true,
            // §FS-fmt.2.3 / §FS-check.3.13: a qualified shorthand is rewritable
            // wherever an unqualified one is — the workspace pass reaches the aliased
            // project's declarations, so `fmt` can name the canonical form here too.
            shorthand_rewritable: scanned_citation_rewritable(line, marker_start),
            shorthand: parsed.shorthand,
            // §FS-fmt.2.4.1: the marker is the citing project's, the number shape
            // the target's — the same split the rewrite itself uses.
            numeric_run: parsed.shorthand
                && target_config.grammar.shorthand_sits_in_numeric_run(
                    &line.config.marker,
                    id_rest,
                    parsed.len,
                ),
            text: line.scan_line[marker_start..token_end].to_string(),
            inline_site: line.inline_sites.get(&line.lineno).cloned(),
            // §AR-scanner.2.4: classified in the post-pass in `scan_file`.
            source_kind: String::new(),
            enclosing_declaration: None,
        });
    }
}

/// §AR-scanner.2.5: collect `<§>`-escaped citation illustrations. The literal
/// `<§>[alias/]ID[.section]` is deliberately *not* a live citation — the `<` and
/// `>` around the marker mean `§` is not immediately followed by an ID, so no
/// detection pass matches it (§AR-workspace.3.1). We record the shape anyway,
/// into a check-inert list, so the checker can flag one whose ID resolves to a
/// real declaration (§FS-check.4.2): an escape of a *real* ID is a likely
/// bracketed live citation, not an intended illustration. IDs are parsed with
/// the citing project's grammar; a cross-namespace target with an exotic grammar
/// may be missed, which only ever costs a suggestion, never a false error.
fn scan_escaped_citations(line: &CitationLine<'_>, findings: &mut Findings) {
    if line.config.marker.is_empty() {
        return;
    }
    let escape = format!("<{}>", line.config.marker);
    if !line.scan_line.contains(&escape) {
        return;
    }
    for (escape_start, _) in line.scan_line.match_indices(&escape) {
        let token_start = escape_start + escape.len();
        let Some(rest) = line.scan_line.get(token_start..) else {
            continue;
        };
        let (namespace, id_rest, alias_len) = match QUALIFIED_CITATION_PREFIX
            .captures(rest)
            .and_then(|p| p.name("namespace").map(|m| (m.as_str().to_string(), p.get(0).unwrap().end())))
        {
            Some((alias, alias_len)) => {
                let Some(id_rest) = rest.get(alias_len..) else {
                    continue;
                };
                (Some(alias), id_rest, alias_len)
            }
            None => (None, rest, 0),
        };
        let Some(parsed) = parse_longest_id_prefix(id_rest, &line.config.grammar) else {
            continue;
        };
        let token_end = token_start + alias_len + parsed.len;
        findings.escaped_citations.push(Citation {
            namespace,
            id: parsed.id,
            section: parsed.section,
            file: line.path.to_path_buf(),
            line: line.lineno,
            column: line.column_offset + escape_start + 1,
            has_marker: false,
            shorthand: parsed.shorthand,
            // An escape is check-inert; nothing rewrites it (§AR-scanner.2.5), so
            // the run question — which only ever gates a rewrite — never arises.
            shorthand_rewritable: false,
            numeric_run: false,
            text: line.scan_line[escape_start..token_end].to_string(),
            inline_site: None,
            source_kind: String::new(),
            enclosing_declaration: None,
        });
    }
}

