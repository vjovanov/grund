/// A dotfile or dot-directory — same convention used by the scanner walker
/// and by `expand_workspace_members` to skip `.git`, `.agents`, `.cache`, etc.
const PARALLEL_SCAN_MIN_FILES: usize = 256;

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Whether a file is one the scanner reads: a non-hidden name with an extension in
/// `[scan] extensions` (§FS-config.3.5, §AR-scanner.1).
fn is_scannable(path: &Path, config: &Config) -> bool {
    if is_hidden(path) {
        return false;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    config.extensions.iter().any(|allowed| allowed == ext)
}

struct CitationLine<'a> {
    scan_line: &'a str,
    /// The untransformed source line. `scan_line` may be a *slice* of it — a
    /// Python docstring's interior with the quotes stripped (§AR-scanner.4) — so a
    /// position on this line and a position on that one are not the same number.
    /// Every never-rewrite question is asked at a **raw-line** offset and routed to
    /// the right text by `docstring` below (§FS-fmt.2.3.1).
    raw_line: &'a str,
    /// Where this line's Python docstring content sits in `raw_line`
    /// (§FS-fmt.2.3.1) — the view every never-rewrite question is asked through,
    /// so a docstring line is judged on the text `fmt` reads there too.
    docstring: DocstringContent<'a>,
    column_offset: usize,
    lineno: usize,
    path: &'a Path,
    config: &'a Config,
    is_md: bool,
    /// The bytes on this physical source line that the scanner's shared block
    /// walk recognizes as comment content. Markdown and Python docstrings use
    /// their already-normalized `scan_line` instead (§FS-values.3.2).
    value_comment_range: Option<(usize, usize)>,
    inline_sites: &'a BTreeMap<usize, InlineCitationSite>,
    inline_block_lines: &'a BTreeMap<usize, std::sync::Arc<[String]>>,
}

/// §AR-scanner.2.3: the qualified `alias/ID` form collides with a path, module
/// reference, or URL, so in a **source** file a marked qualified citation whose
/// start column falls inside an inline-code span or a string literal is not a
/// citation (the same path-collision caution as §AR-workspace.3.1). Markdown has
/// no string literals and its inline code is prose formatting, so a marked
/// qualified citation there is always a citation. Shared by every detection pass
/// so the rule lives in one place; returns whether the citation at `pos` must be
/// suppressed.
fn qualified_suppressed_in_source(scan_line: &str, is_md: bool, pos: usize) -> bool {
    !is_md && (is_inside_inline_code(scan_line, pos) || is_inside_string_literal(scan_line, pos))
}

/// The per-file scan (§AR-scanner.2): line by line, find declaration headings
/// (§AR-scanner.2.1 — in Markdown or in a code/`"""` doc-comment, §AR-scanner.4),
/// nested section headings (§AR-scanner.2.2), and `<ID>[.<section>]` citations
/// (§AR-scanner.2.3, §FS-check.1.1) — skipping fenced code blocks; outside
/// Markdown, bare ID-shaped tokens inside string literals (§FS-fmt.2.3.1); in
/// Markdown, bare ID-shaped tokens inside a link destination (§FS-check.1.1,
/// §FS-fmt.2.3); and any bare token at all under `[reference] strict`
/// (§FS-config.3.1).
///
/// One citation regex is used for every scan; whether a match is qualified
/// (marker + `<alias>/<ID>`) or unqualified (marker + `<ID>`) is determined by
/// whether the `<namespace>` capture fired (§AR-workspace.3.1). The alias
/// prefix is only honoured when the marker precedes it — an unmarked
/// `<alias>/<ID>` in prose is text, never a qualified citation
/// (§FS-workspace.1, §AR-workspace.3.1).
///
/// In workspace mode the caller passes a non-empty `workspace_targets` so a
/// `§<alias>/<ID>` token parses with the target project's grammar inline —
/// one disk read for both unqualified and qualified citations
/// (§AR-workspace.5.1). An empty slice falls back to the loose qualified
/// parser used by member-local scans (§FS-workspace.5).
fn scan_file(
    path: &Path,
    config: &Config,
    findings: &mut Findings,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Result<()> {
    let text = fs::read_to_string(path)?;
    scan_file_text(path, &text, config, findings, workspace_targets)
}

/// Scan one file's already-read text into `findings`: declarations, their section
/// headings, citations, and the headings that came close to being declarations.
///
/// `classify` turns on citing-side classification — declaration body ranges and each
/// citation's source kind. `grund check` asks for it; the read-only commands turn it
/// off, and a project without direction rules pays nothing.
///
/// The near-miss test runs in this pass rather than in a second read of the tree
/// because re-deriving the line, the position rules and the fence/docstring state
/// later costs a whole extra pass over every file for a list most runs find empty.
///
/// Section paths first land in the primary or duplicate map, then the body-span
/// post-pass narrows both maps and records rejected headings for §FS-check.3.23.
///
/// `claimed_markers` is what keeps the shorthand pattern from running at all on a
/// line whose markers are already accounted for.
///
/// Text section headings also require the spans, because the shared coordinate
/// catalog is body-local (§FS-show.2.1.2). `scan_one_file` gives this call a fresh
/// `Findings`, so `findings` holds exactly this file's records.
fn scan_file_text(
    path: &Path,
    text: &str,
    config: &Config,
    findings: &mut Findings,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Result<()> {
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let (inline_sites, inline_block_lines) =
        inline_citation_sites(path, &text, is_md, is_py, config, workspace_targets);
    let in_docs = path.components().any(|c| c.as_os_str() == "docs");
    let mut markdown_fence = None;
    let mut py_docstring = PythonDocstringScanState::default();
    let mut current: Option<Declaration> = None;
    // An exact embedded marker independently enables value authority in any
    // scanned document (§FS-values.1, §FS-values.9). This is still the
    // already-read file buffer; unmarked trees gain no extra filesystem read.
    let scan_values = text.contains(EMBEDDED_VALUE_MARKER)
        || config.kinds.iter().any(|kind| kind.values)
        || workspace_targets
            .iter()
            .any(|target| target.config.kinds.iter().any(|kind| kind.values));
    let has_binding_candidate = text.contains('`') && text.contains(&config.marker);
    let value_line_contexts = ((scan_values || has_binding_candidate) && !is_md)
        .then(|| recognized_source_value_contexts(&text, is_py, config));
    // §AR-scanner.2.4: citing-side classification is consumed only by the
    // citation-direction checks, so it is computed only when the project declares
    // `[citations]` and the caller asked for it (§AR-benchmarks).
    let classify = config.classify_citation_sources && config.citations.declared;
    // §AR-scanner.2.4: every Markdown heading (line, level) outside a fence — a
    // declaration body runs until the next heading at the same or higher level.
    let mut md_headings: Vec<(usize, usize)> = Vec::new();
    // §AR-scanner.2.2 / §FS-check.4.14: declaration and section recognition
    // happen in this same fence-aware pass. Plain ATX headings wait until body
    // spans are known before becoming reportable candidates.
    let mut unmarked_heading_candidates = Vec::new();
    let mut total_lines = 0usize;

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        total_lines = lineno;
        if is_md && markdown_fence_delimiter(&mut markdown_fence, line) {
            continue;
        }
        if markdown_fence.is_some() {
            continue;
        }
        let trimmed = line.trim_start();
        // Collected for every Markdown file: the shared section-map prune below
        // needs the body spans and cannot ask for them after this pass.
        if is_md && let Some(level) = markdown_heading_level(line) {
            md_headings.push((lineno, level));
        }
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        let scan_line = scan.text;
        let source_value_context = value_line_contexts
            .as_ref()
            .and_then(|contexts| contexts.get(idx).copied().flatten());
        let embedded_marker = embedded_value_marker_for_line(
            scan_line,
            is_md,
            scan.in_py_docstring,
            source_value_context,
        );

        if let Some(caps) = declaration_captures(&config.grammar, scan_line, scan.in_py_docstring, is_md)
            && let Some(id) = parse_id(&caps, &config.grammar)
        {
            if let Some(prev) = current.take() {
                findings
                    .declarations
                    .entry(prev.id.clone())
                    .or_default()
                    .push(prev);
            }
            let tail = &scan_line[caps.get(0).unwrap().end()..];
            let mut is_stub = false;
            let mut defined_in = None;
            if is_md
                && in_docs
                && let Some(link_caps) = STUB_LINK_HEADING.captures(tail)
            {
                is_stub = true;
                defined_in = Some(PathBuf::from(link_caps.name("path").unwrap().as_str()));
            }
            let title = if is_stub {
                None
            } else {
                let trimmed = tail.trim_start();
                let trimmed = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            };
            current = Some(Declaration {
                id,
                file: path.to_path_buf(),
                line: lineno,
                heading_level: heading_level_for_line(scan_line, is_md || scan.in_py_docstring, &caps),
                sections: BTreeMap::new(),
                duplicate_sections: Vec::new(),
                is_stub,
                defined_in,
                e2e_case: None,
                title,
                // §AR-scanner.2.4: real body span is assigned in the post-pass
                // below once every declaration and (for Markdown) every heading
                // on the file is known. Default to the single declaration line.
                body_start: lineno,
                body_end: lineno,
                source: DeclarationSource::Text,
                value_valid: None,
            });
            if let Some(marker_start) = embedded_marker {
                push_invalid_embedded_marker(
                    findings,
                    current.as_ref().map(|decl| decl.id.clone()),
                    path,
                    lineno,
                    scan.column_offset,
                    marker_start,
                    "embedded value marker must be on a citable numeric section heading",
                );
            }
            continue;
        }

        // §FS-check.4.6: the line was not a declaration. Ask the near-miss pattern
        // whether it looked like one, here rather than in a second read of the tree —
        // the scan has the line, the position rules and the fence/docstring state.
        if let Some((text, format, kind)) =
            near_miss_heading(&config.grammar, scan_line, scan.in_py_docstring, is_md)
        {
            if let Some(prev) = current.take() {
                findings
                    .declarations
                    .entry(prev.id.clone())
                    .or_default()
                    .push(prev);
            }
            findings.near_miss_headings.push(NearMissHeading {
                file: path.to_path_buf(),
                line: lineno,
                text: text.to_string(),
                format: format.to_string(),
            });
            let token_end = scan_line.find(text).map(|start| start + text.len()).unwrap_or(0);
            let tail = &scan_line[token_end..];
            let mut is_stub = false;
            let mut defined_in = None;
            if is_md
                && in_docs
                && let Some(link_caps) = STUB_LINK_HEADING.captures(tail)
            {
                is_stub = true;
                defined_in = Some(PathBuf::from(link_caps.name("path").unwrap().as_str()));
            }
            let title = if is_stub {
                None
            } else {
                let trimmed = tail.trim_start();
                let trimmed = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            };
            // §FS-config.3.2 / §AR-scanner.2.1: retain a rejected declaration-position
            // token as an ordinary catalog declaration, with exact spelling and all
            // body/section state from this same file pass.
            current = Some(Declaration {
                id: Id::legacy(kind.to_string(), text),
                file: path.to_path_buf(),
                line: lineno,
                heading_level: if is_md || scan.in_py_docstring {
                    scan_line
                        .trim_start()
                        .chars()
                        .take_while(|ch| *ch == '#')
                        .count()
                        .max(1)
                } else {
                    1
                },
                sections: BTreeMap::new(),
                duplicate_sections: Vec::new(),
                is_stub,
                defined_in,
                e2e_case: None,
                title,
                body_start: lineno,
                body_end: lineno,
                source: DeclarationSource::Text,
                value_valid: None,
            });
            continue;
        }

        let section_caps = config.grammar.section_re.captures(scan_line);
        let recognized_section = section_caps
            .as_ref()
            .and_then(section_path)
            .is_some();
        let mut embedded_marker_attached = false;
        if let Some(caps) = section_caps
            && let Some(decl) = current.as_mut()
            && let Some(sec) = section_path(&caps)
        {
            let heading_level = heading_level_for_line(scan_line, is_md || scan.in_py_docstring, &caps);
            if heading_level > decl.heading_level {
                let section_path = sec.to_string();
                let numeric = sec
                    .split('.')
                    .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
                embedded_marker_attached = embedded_marker.is_some() && numeric;
                let info = SectionInfo {
                    title: section_anchor_text(scan_line, sec),
                    line: lineno,
                    heading_level,
                    value: None,
                    value_root: embedded_marker.filter(|_| numeric).map(|marker_start| {
                        EmbeddedValueRoot {
                            valid: true,
                            marker_column: scan.column_offset + marker_start + 1,
                        }
                    }),
                };
                // §AR-scanner.2.2: a path is recorded once, by the first heading
                // that claims it; later claimants go to `duplicate_sections` so
                // §FS-check.3.16 can name every colliding line.
                match decl.sections.entry(section_path.clone()) {
                    std::collections::btree_map::Entry::Vacant(slot) => {
                        slot.insert(info);
                    }
                    std::collections::btree_map::Entry::Occupied(_) => {
                        if let Some(marker_start) = embedded_marker {
                            embedded_marker_attached = true;
                            push_invalid_embedded_marker(
                                findings,
                                Some(decl.id.clone()),
                                path,
                                lineno,
                                scan.column_offset,
                                marker_start,
                                "embedded value root coordinate is duplicated",
                            );
                        }
                        decl.duplicate_sections.push((section_path, info));
                    }
                }
            }
        }
        if is_md
            && !recognized_section
            && let Some(heading_level) = markdown_heading_level(line)
        {
            let heading = line.trim_end().trim_start().to_string();
            let title = heading_text(trimmed, heading_level);
            let column = line.find('#').unwrap_or(0) + 1;
            unmarked_heading_candidates.push(UnmarkedHeadingCandidate {
                file: path.to_path_buf(),
                line: lineno,
                column,
                heading,
                heading_level,
                title,
            });
        }
        if embedded_marker.is_some()
            && !embedded_marker_attached
            && authored_heading_level(
                scan_line,
                is_md || scan.in_py_docstring,
                source_value_context.is_some_and(|context| context.block_comment),
                config,
            )
            .is_some()
        {
            push_invalid_embedded_marker(
                findings,
                current.as_ref().map(|decl| decl.id.clone()),
                path,
                lineno,
                scan.column_offset,
                embedded_marker.unwrap(),
                "embedded value marker must be on a citable numeric section heading",
            );
        }

        let workspace_mode = !workspace_targets.is_empty();
        let citation_start = findings.citations.len();
        let mut qualified_marker_starts = BTreeSet::new();
        // §AR-scanner.2.6: every marker the full-ID pattern matched at, whether or
        // not this pass emitted a citation there. The shorthand pass skips these
        // (§DF-number-only-citation-shorthand.2.6), so the full ID always wins.
        let mut claimed_markers: Vec<usize> = Vec::new();
        for caps in config.grammar.citation_re.captures_iter(scan_line) {
            let Some(full) = caps.get(0) else { continue };
            let namespace = caps.name("namespace").map(|m| m.as_str().to_string());
            let has_marker = scan_line[..full.start()].ends_with(&config.marker);
            if has_marker {
                claimed_markers.push(full.start() - config.marker.len());
            }
            // §FS-check.1.1 / §AR-scanner.2.3: a reserved `number.name`
            // candidate is consumed as one rejected token, never shortened to
            // the valid numeric prefix the regex necessarily matched.
            if config.grammar.has_reserved_named_tail(scan_line, full.end()) {
                continue;
            }
            // In workspace mode, the qualified branch is parsed below with the
            // target's grammar — let that pass own every `§<alias>/...` hit so
            // we never emit one with the citing project's grammar.
            if workspace_mode && namespace.is_some() {
                continue;
            }
            // §FS-workspace.1, §AR-workspace.3.1: an unmarked `alias/ID` is text,
            // not a citation. The slash is part of the visual token; we do not
            // fall back to recognising the trailing ID as a bare citation.
            if namespace.is_some() && !has_marker {
                continue;
            }
            if config.strict && !has_marker {
                continue;
            }
            // In an opted-in repository an unmarked name tail is prose as one
            // whole token, even in compatibility scanning mode (§FS-check.1.1).
            if !has_marker
                && config
                    .grammar
                    .is_named_section(caps.name("sec").map(|sec| sec.as_str()))
            {
                continue;
            }
            if !has_marker && bare_token_in_never_rewrite_zone(scan_line, is_md, full.start()) {
                continue;
            }
            let Some(id) = parse_id(&caps, &config.grammar) else { continue };
            let start = if has_marker {
                full.start().saturating_sub(config.marker.len())
            } else {
                full.start()
            };
            if namespace.is_some()
                && has_marker
                && qualified_suppressed_in_source(scan_line, is_md, start)
            {
                continue;
            }
            if let Some(decl) = current.as_ref()
                && decl.line == lineno
                && decl.id == id
            {
                continue;
            }
            let text = scan_line[start..full.end()].to_string();
            if namespace.is_some() && has_marker {
                qualified_marker_starts.insert(start);
            }
            findings.citations.push(Citation {
                namespace,
                id,
                section: caps.name("sec").map(|m| m.as_str().to_string()),
                file: path.to_path_buf(),
                line: lineno,
                column: scan.column_offset + start + 1,
                has_marker,
                shorthand: false,
                shorthand_rewritable: true,
                numeric_run: false,
                text,
                inline_site: inline_sites.get(&lineno).cloned(),
                // §AR-scanner.2.4: classified in the post-pass below.
                source_kind: String::new(),
                enclosing_declaration: None,
            });
        }
        let citation_line = CitationLine {
            scan_line,
            raw_line: line,
            docstring: DocstringContent::of(&scan),
            column_offset: scan.column_offset,
            lineno,
            path,
            config,
            is_md,
            value_comment_range: source_value_context.map(|context| context.range),
            inline_sites: &inline_sites,
            inline_block_lines: &inline_block_lines,
        };
        if workspace_mode {
            scan_workspace_qualified_pass(
                &citation_line,
                workspace_targets,
                findings,
            );
        } else {
            // §AR-scanner.2.6: the fallback records what it claimed into the same
            // set, so the shorthand pass below can tell a qualified marker that
            // already became a citation from one no qualified pass could parse.
            scan_fallback_qualified_citations(
                &citation_line,
                &mut qualified_marker_starts,
                findings,
            );
        }
        scan_shorthand_citations(
            &citation_line,
            workspace_mode,
            &claimed_markers,
            &qualified_marker_starts,
            findings,
        );
        scan_legacy_citation_candidates(&citation_line, findings);
        scan_escaped_citations(&citation_line, findings);
        // Exact candidates stay in this pass. The checker activates them only
        // for whole or marked authority, keeping unmarked prose inert across
        // workspaces (§FS-values.3.1, §FS-values.7, §FS-values.9).
        scan_value_bindings(&citation_line, workspace_targets, citation_start, findings);
    }

    if let Some(decl) = current.take() {
        findings
            .declarations
            .entry(decl.id.clone())
            .or_default()
            .push(decl);
    }

    // §AR-scanner.2.4: now that every declaration and (for Markdown) every heading
    // on the file is known, fix each declaration's body span and classify each
    // citation's citing side — off the hot path either way (§AR-benchmarks).
    let has_text_sections = findings
        .declarations
        .values()
        .flatten()
        .any(|decl| !decl.sections.is_empty() || !decl.duplicate_sections.is_empty());
    let has_embedded_roots = findings
        .declarations
        .values()
        .flatten()
        .flat_map(|decl| decl.sections.values())
        .any(|section| section.value_root.is_some());
    let has_value_declarations = scan_values
        && findings
            .declarations
            .values()
            .flatten()
            .any(|decl| kind_uses_values(config, &decl.id.kind));
    let has_unmarked_headings = !unmarked_heading_candidates.is_empty();
    if classify
        || has_text_sections
        || has_value_declarations
        || has_embedded_roots
    {
        assign_declaration_bodies(findings, is_md, is_py, config, &text, &md_headings, total_lines);
    }
    if has_text_sections {
        retain_in_body_sections(findings);
    }
    if has_unmarked_headings {
        assign_unmarked_heading_owners(
            findings,
            unmarked_heading_candidates,
            &md_headings,
            total_lines,
        );
    }
    if has_value_declarations {
        validate_markdown_value_declarations(path, &text, is_md, config, findings);
    }
    if has_embedded_roots {
        validate_embedded_value_roots(
            path,
            &text,
            is_md,
            is_py,
            config,
            value_line_contexts.as_deref(),
            findings,
        );
    }
    if classify {
        classify_citation_sources(findings, config, path);
    }
    // §AR-scanner.2.7: the headings and doc-comment blocks a grounding unit finer
    // than the file is cut out of — recorded only where the file's own row asks
    // for one, so a level-1 tree pays nothing (§FS-config.3.4.8).
    record_file_structure(path, text, config, findings);
    Ok(())
}
