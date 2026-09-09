/// §FS-fmt.2.4 / §FS-fmt.6.2: accepted local shorthand survives the expansion
/// pass but remains a resolved citation eligible for cross-reference wrapping.
/// Its visible short span is retained while its canonical `Id` selects the link.
fn collect_local_accepted_shorthand_links(
    line: &str,
    config: &Config,
    index: Option<&ShorthandIndex<'_>>,
    out: &mut Vec<MarkdownLineCitation>,
) {
    if config.shorthand != ShorthandPolicy::Accepted || config.marker.is_empty() {
        return;
    }
    for (marker_start, _) in line.match_indices(&config.marker) {
        if is_inside_inline_code(line, marker_start)
            || is_inside_markdown_link_destination(line, marker_start)
        {
            continue;
        }
        let token_start = marker_start + config.marker.len();
        let Some(rest) = line.get(token_start..) else {
            continue;
        };
        if QUALIFIED_CITATION_PREFIX.is_match(rest) {
            continue;
        }
        let Some((id, section, consumed)) = accepted_shorthand_link(rest, config, config, index)
        else {
            continue;
        };
        out.push(MarkdownLineCitation {
            marker_start,
            token_end: token_start + consumed,
            namespace: None,
            id,
            section,
        });
    }
}

/// Resolve one accepted shorthand through the same grammar/index pair used by
/// §FS-fmt.2.4. Unknown, ambiguous, numeric-run, and non-whole-token candidates
/// stay absent, so link wrapping never becomes a second resolver or guesses.
fn accepted_shorthand_link(
    rest: &str,
    citing_config: &Config,
    target_config: &Config,
    index: Option<&ShorthandIndex<'_>>,
) -> Option<(Id, Option<String>, usize)> {
    if target_config.shorthand != ShorthandPolicy::Accepted {
        return None;
    }
    let shorthand = target_config.grammar.shorthand_for(rest)?;
    if shorthand.full_prefix_re().is_match(rest) {
        return None;
    }
    let caps = shorthand.prefix_re().captures(rest)?;
    let consumed = caps.get(0)?.end();
    if target_config
        .grammar
        .has_reserved_named_tail(rest, consumed)
        || !target_config.grammar.id_token_ends_cleanly(rest, consumed)
        || target_config.grammar.shorthand_sits_in_numeric_run(
            &citing_config.marker,
            rest,
            consumed,
        )
    {
        return None;
    }
    let parsed = parse_id(&caps, &target_config.grammar)?;
    let unique = index?.unique(&parsed)?;
    if unique.legacy_spelling().is_some() {
        return None;
    }
    Some((
        unique.clone(),
        caps.name("sec").map(|section| section.as_str().to_string()),
        consumed,
    ))
}
