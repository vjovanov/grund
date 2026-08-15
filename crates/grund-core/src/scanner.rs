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
    /// Python docstring's interior with the quotes stripped (§AR-scanner.4) — and
    /// the two answer differently when asked whether a position sits inside a
    /// string literal. Anything deciding what `grund fmt` would do must ask the
    /// raw line, because that is the text `fmt` sees (§FS-fmt.2.3).
    raw_line: &'a str,
    column_offset: usize,
    lineno: usize,
    path: &'a Path,
    config: &'a Config,
    is_md: bool,
    inline_sites: &'a BTreeMap<usize, InlineCitationSite>,
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
/// (§AR-scanner.2.3, §FS-check.1.1) — skipping fenced code blocks and, outside
/// Markdown, bare ID-shaped tokens inside string literals (§FS-fmt.2.3.1) and any
/// bare token at all under `[reference] strict` (§FS-config.3.1).
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

fn scan_file_text(
    path: &Path,
    text: &str,
    config: &Config,
    findings: &mut Findings,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Result<()> {
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let inline_sites = inline_citation_sites(&text, is_md, is_py, config, workspace_targets);
    let in_docs = path.components().any(|c| c.as_os_str() == "docs");
    let mut in_fence = false;
    let mut py_docstring = PythonDocstringScanState::default();
    let mut current: Option<Declaration> = None;
    // §AR-scanner.2.4: citing-side classification (declaration body ranges and
    // each citation's source kind) is consumed only by the citation-direction
    // checks, so it is computed only when the project declares `[citations]` and
    // the caller asked for it. `grund check` asks; the read-only commands turn it
    // off, and a project without direction rules pays nothing (§AR-benchmarks).
    let classify = config.classify_citation_sources && config.citations.declared;
    // §AR-scanner.2.4: every Markdown heading (line, level) outside a fence — a
    // declaration body runs until the next heading at the same or higher level.
    let mut md_headings: Vec<(usize, usize)> = Vec::new();
    let mut total_lines = 0usize;

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        total_lines = lineno;
        let trimmed = line.trim_start();
        if is_md && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if classify && is_md && let Some(level) = markdown_heading_level(line) {
            md_headings.push((lineno, level));
        }
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        let scan_line = scan.text;

        if let Some(caps) = declaration_captures(&config.grammar, scan_line, scan.in_py_docstring, is_md)
            && let Some(id) = parse_id(&caps)
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
                is_stub,
                defined_in,
                e2e_case: None,
                title,
                // §AR-scanner.2.4: real body span is assigned in the post-pass
                // below once every declaration and (for Markdown) every heading
                // on the file is known. Default to the single declaration line.
                body_start: lineno,
                body_end: lineno,
            });
            continue;
        }

        if let Some(caps) = config.grammar.section_re.captures(scan_line)
            && let Some(decl) = current.as_mut()
            && let Some(sec) = caps.name("sec")
        {
            let heading_level = heading_level_for_line(scan_line, is_md || scan.in_py_docstring, &caps);
            if heading_level > decl.heading_level {
                decl.sections.insert(
                    sec.as_str().to_string(),
                    SectionInfo {
                        title: section_anchor_text(scan_line, sec.as_str()),
                        line: lineno,
                        heading_level,
                    },
                );
            }
        }

        let workspace_mode = !workspace_targets.is_empty();
        let mut qualified_marker_starts = BTreeSet::new();
        // §AR-scanner.2.6: every marker the full-ID pattern matched at, whether
        // or not this pass went on to emit a citation there. The shorthand pass
        // (§DF-number-only-citation-shorthand.2.6) skips these, so the full ID
        // always wins and the shorthand pattern is never run on a line whose
        // markers are all accounted for.
        let mut claimed_markers: Vec<usize> = Vec::new();
        for caps in config.grammar.citation_re.captures_iter(scan_line) {
            let Some(full) = caps.get(0) else { continue };
            let namespace = caps.name("namespace").map(|m| m.as_str().to_string());
            let has_marker = scan_line[..full.start()].ends_with(&config.marker);
            if has_marker {
                claimed_markers.push(full.start() - config.marker.len());
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
            if !is_md && !has_marker && is_inside_string_literal(scan_line, full.start()) {
                continue;
            }
            let Some(id) = parse_id(&caps) else { continue };
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
            column_offset: scan.column_offset,
            lineno,
            path,
            config,
            is_md,
            inline_sites: &inline_sites,
        };
        if workspace_mode {
            scan_workspace_qualified_pass(
                &citation_line,
                workspace_targets,
                findings,
            );
        } else {
            scan_fallback_qualified_citations(
                &citation_line,
                &qualified_marker_starts,
                findings,
            );
        }
        scan_shorthand_citations(&citation_line, workspace_mode, &claimed_markers, findings);
        scan_escaped_citations(&citation_line, findings);
    }

    if let Some(decl) = current.take() {
        findings
            .declarations
            .entry(decl.id.clone())
            .or_default()
            .push(decl);
    }

    // §AR-scanner.2.4: now that every declaration and (for Markdown) every
    // heading on the file is known, fix each declaration's body span and
    // classify each citation's citing side. `scan_one_file` gives this call a
    // fresh `Findings`, so `findings` holds exactly this file's records. Skipped
    // unless the project declares `[citations]` — nothing else reads the result.
    if classify {
        assign_declaration_bodies(findings, is_md, is_py, config, &text, &md_headings, total_lines);
        classify_citation_sources(findings, config, path);
    }
    Ok(())
}

/// The level of a Markdown ATX heading line (`#` count), or `None` when the line
/// is not a heading (§AR-scanner.2.4). A heading is one or more leading `#`
/// followed by whitespace or end of line.
fn markdown_heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 {
        return None;
    }
    match trimmed[hashes..].chars().next() {
        None => Some(hashes),
        Some(ch) if ch.is_whitespace() => Some(hashes),
        _ => None,
    }
}

/// Assign every declaration in `findings` its body line span (§AR-scanner.2.4).
/// In Markdown the body runs until the next heading at the same or higher level;
/// in a source file it is bounded by the comment/docstring block the declaration
/// opens, capped before the next declaration sharing that block.
fn assign_declaration_bodies(
    findings: &mut Findings,
    is_md: bool,
    is_py: bool,
    config: &Config,
    text: &str,
    md_headings: &[(usize, usize)],
    total_lines: usize,
) {
    // The lines (sorted) at which this file declares — used to cap a source
    // declaration's body before the next declaration in the same comment block.
    let mut decl_lines: Vec<usize> = findings
        .declarations
        .values()
        .flatten()
        .map(|decl| decl.line)
        .collect();
    decl_lines.sort_unstable();

    let code_blocks = (!is_md).then(|| comment_block_ranges(text, is_py, config));

    for decl in findings.declarations.values_mut().flatten() {
        decl.body_start = decl.line;
        if decl.is_stub {
            decl.body_end = decl.line;
            continue;
        }
        if is_md {
            let next_break = md_headings
                .iter()
                .filter(|(line, level)| *line > decl.line && *level <= decl.heading_level)
                .map(|(line, _)| *line)
                .min();
            decl.body_end = next_break
                .map(|line| line - 1)
                .unwrap_or(total_lines)
                .max(decl.line);
        } else {
            let block_end = code_blocks
                .as_ref()
                .and_then(|blocks| {
                    blocks
                        .iter()
                        .find(|(start, end)| *start <= decl.line && decl.line <= *end)
                        .map(|(_, end)| *end)
                })
                .unwrap_or(decl.line);
            let next_decl_cap = decl_lines
                .iter()
                .copied()
                .find(|line| *line > decl.line)
                .filter(|line| *line <= block_end)
                .map(|line| line - 1);
            decl.body_end = next_decl_cap.unwrap_or(block_end).max(decl.line);
        }
    }
}

/// The 1-indexed inclusive line spans of every comment / docstring block in a
/// source file (§AR-scanner.2.4) — the same block boundaries
/// `inline_citation_sites` walks, without the declares-an-ID filtering, so a
/// declaration's body can be bounded by the block that hosts it.
fn comment_block_ranges(text: &str, is_py: bool, config: &Config) -> Vec<(usize, usize)> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(kind) = comment_block_kind(lines[index], is_py, config) else {
            index += 1;
            continue;
        };
        let start = index;
        let end = match &kind {
            CommentBlockKind::Line(marker) => {
                let mut end = index;
                while end + 1 < lines.len()
                    && matches!(
                        comment_block_kind(lines[end + 1], is_py, config),
                        Some(CommentBlockKind::Line(next)) if next == *marker
                    )
                {
                    end += 1;
                }
                end
            }
            CommentBlockKind::Block => {
                let mut end = index;
                while end + 1 < lines.len() && !lines[end].contains("*/") {
                    end += 1;
                }
                end
            }
            CommentBlockKind::PythonDocstring => {
                let quote = python_docstring_quote(lines[index]).unwrap_or("\"\"\"");
                let mut end = index;
                while end + 1 < lines.len()
                    && !python_docstring_closes(lines[end], quote, end == start)
                {
                    end += 1;
                }
                end
            }
        };
        ranges.push((start + 1, end + 1));
        index = end + 1;
    }
    ranges
}

/// Classify each citation's citing side by the three-step fallback of
/// §AR-scanner.2.4: the enclosing declaration's kind (nearest preceding
/// declaration whose body contains the site), else the file's unique kind home,
/// else the reserved `code` pseudo-kind.
fn classify_citation_sources(findings: &mut Findings, config: &Config, path: &Path) {
    // (body_start, body_end, id) for this file's declarations, so the enclosing
    // lookup is a scan of a small local list.
    let bodies: Vec<(usize, usize, Id)> = findings
        .declarations
        .values()
        .flatten()
        .map(|decl| (decl.body_start, decl.body_end, decl.id.clone()))
        .collect();
    let file_home = file_home_kind(path, config);
    for cite in &mut findings.citations {
        let enclosing = bodies
            .iter()
            .filter(|(start, end, _)| *start <= cite.line && cite.line <= *end)
            // Nearest preceding declaration: the one whose body starts latest.
            .max_by_key(|(start, _, _)| *start);
        match enclosing {
            Some((_, _, id)) => {
                cite.source_kind = id.kind.clone();
                cite.enclosing_declaration = Some(id.clone());
            }
            None => {
                cite.source_kind = file_home
                    .clone()
                    .unwrap_or_else(|| CODE_SOURCE_KIND.to_string());
            }
        }
    }
}

/// The kind whose configured home (`[[kinds]] folder` / `file`, §FS-config.3.4)
/// uniquely contains `path` — step 2 of §AR-scanner.2.4. `None` when no home or
/// more than one home matches, so the citation falls through to `code`.
fn file_home_kind(path: &Path, config: &Config) -> Option<String> {
    // Walked file paths are canonicalized against the scan root (`scan_roots`
    // resolves an explicit scope), while `config.root` is the configured,
    // possibly-symlinked root — so on macOS a temp dir resolves through
    // `/private` and on Windows through a `\\?\` verbatim prefix, and a plain
    // `strip_prefix(config.root)` misses. Reuse the checker's reverse-home
    // helper, which strips against the physical root first, then the configured
    // one, so the lookup is identical to the declaration-home lookup.
    let physical_root = fs::canonicalize(&config.root).unwrap_or_else(|_| config.root.clone());
    let relative = scanned_decl_relative_path(path, &config.root, &physical_root)?;
    let relative = relative.as_ref();
    let mut matched: Option<&str> = None;
    for kind in &config.kinds {
        let hit = match (kind.file.as_deref(), kind.folder.as_deref()) {
            (Some(file), _) => relative == scanned_path_key(Path::new(file)).as_path(),
            (_, Some(folder)) => relative.starts_with(scanned_path_key(Path::new(folder))),
            _ => false,
        };
        if hit {
            if matched.is_some_and(|prev| prev != kind.prefix.as_str()) {
                return None;
            }
            matched = Some(kind.prefix.as_str());
        }
    }
    matched.map(str::to_string)
}

#[derive(Clone)]
enum CommentBlockKind {
    Line(String),
    Block,
    PythonDocstring,
}

/// Locate the source-comment blocks that can host inline citation sites
/// (§FS-inline-citation-style.1). Markdown prose is deliberately out of scope.
fn inline_citation_sites(
    text: &str,
    is_md: bool,
    is_py: bool,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> BTreeMap<usize, InlineCitationSite> {
    let mut sites = BTreeMap::new();
    if is_md {
        return sites;
    }
    let lines = text.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let Some(kind) = comment_block_kind(lines[index], is_py, config) else {
            index += 1;
            continue;
        };
        let start = index;
        let end = match &kind {
            CommentBlockKind::Line(marker) => {
                let mut end = index;
                while end + 1 < lines.len()
                    && matches!(
                        comment_block_kind(lines[end + 1], is_py, config),
                        Some(CommentBlockKind::Line(next)) if next == *marker
                    )
                {
                    end += 1;
                }
                end
            }
            CommentBlockKind::Block => {
                let mut end = index;
                while end + 1 < lines.len() && !lines[end].contains("*/") {
                    end += 1;
                }
                end
            }
            CommentBlockKind::PythonDocstring => {
                let quote = python_docstring_quote(lines[index]).unwrap_or("\"\"\"");
                let mut end = index;
                while end + 1 < lines.len()
                    && !python_docstring_closes(lines[end], quote, end == start)
                {
                    end += 1;
                }
                end
            }
        };
        if !block_declares_id(&lines[start..=end], matches!(kind, CommentBlockKind::PythonDocstring), config) {
            let site = InlineCitationSite {
                first_line: start + 1,
                last_line: end + 1,
                max_columns: lines[start..=end]
                    .iter()
                    .map(|line| line.len())
                    .max()
                    .unwrap_or(0),
                has_note: block_has_inline_note(&lines[start..=end], config, workspace_targets),
            };
            for line in (start + 1)..=(end + 1) {
                sites.insert(line, site.clone());
            }
        }
        index = end + 1;
    }
    sites
}

fn comment_block_kind(line: &str, is_py: bool, config: &Config) -> Option<CommentBlockKind> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    if config.docstring_python && is_py && python_docstring_quote(line).is_some() {
        return Some(CommentBlockKind::PythonDocstring);
    }
    if config.comment_prefixes.iter().any(|prefix| prefix == "/*") && trimmed.starts_with("/*") {
        return Some(CommentBlockKind::Block);
    }
    line_comment_marker(trimmed, config).map(CommentBlockKind::Line)
}

fn line_comment_marker(trimmed: &str, config: &Config) -> Option<String> {
    for marker in ["///", "//!", "//"] {
        if config.comment_prefixes.iter().any(|prefix| prefix == "//")
            && trimmed.starts_with(marker)
        {
            return Some(marker.to_string());
        }
    }
    let mut prefixes = config
        .comment_prefixes
        .iter()
        .filter(|prefix| !matches!(prefix.as_str(), "" | "//" | "*" | "/*"))
        .collect::<Vec<_>>();
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    prefixes
        .into_iter()
        .find(|prefix| trimmed.starts_with(prefix.as_str()))
        .map(|prefix| prefix.to_string())
}

fn python_docstring_closes(line: &str, quote: &str, is_opening_line: bool) -> bool {
    let trimmed = line.trim_start();
    let search = if is_opening_line {
        trimmed.strip_prefix(quote).unwrap_or(trimmed)
    } else {
        trimmed
    };
    search.contains(quote)
}

fn block_declares_id(lines: &[&str], in_py_docstring: bool, config: &Config) -> bool {
    let mut py_docstring = PythonDocstringScanState::default();
    lines.iter().any(|line| {
        let scan = if in_py_docstring {
            source_scan_line(line, true, config.docstring_python, &mut py_docstring)
        } else {
            SourceScanLine {
                text: line,
                in_py_docstring: false,
                column_offset: 0,
                closed_py_docstring: false,
            }
        };
        declaration_captures(&config.grammar, scan.text, scan.in_py_docstring, false)
            .and_then(|caps| parse_id(&caps))
            .is_some()
    })
}

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
    for (start, end) in ranges {
        if start < cursor {
            continue;
        }
        out.push_str(&line[cursor..start]);
        cursor = end;
    }
    out.push_str(&line[cursor..]);
    out
}

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
        if namespace.is_some() && !has_marker {
            continue;
        }
        if config.strict && !has_marker {
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

fn strip_comment_tokens(line: &str, config: &Config) -> String {
    let marker_start = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(line.len());
    let (_, body) = line.split_at(marker_start);
    let mut rest = body;
    for prefix in comment_strip_prefixes(config) {
        if let Some(stripped) = rest.strip_prefix(prefix) {
            rest = stripped.strip_prefix(' ').unwrap_or(stripped);
            break;
        }
    }
    let trimmed_end = rest.trim_end();
    let rest = trimmed_end
        .strip_suffix("*/")
        .or_else(|| trimmed_end.strip_suffix("\"\"\""))
        .or_else(|| trimmed_end.strip_suffix("'''"))
        .unwrap_or(trimmed_end);
    rest.to_string()
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
fn scan_fallback_qualified_citations(
    line: &CitationLine<'_>,
    already_seen: &BTreeSet<usize>,
    findings: &mut Findings,
) {
    if line.config.marker.is_empty() {
        return;
    }
    for (marker_start, _) in line.scan_line.match_indices(&line.config.marker) {
        if already_seen.contains(&marker_start) {
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
        let parsed = match targets.iter().find(|target| target.alias == alias) {
            Some(target) => parse_longest_id_prefix(id_rest, &target.config.grammar),
            None => targets
                .iter()
                .find_map(|target| parse_longest_id_prefix(id_rest, &target.config.grammar)),
        };
        let Some(parsed) = parsed else {
            continue;
        };
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
            // wherever an unqualified one is — the workspace pass reaches the
            // aliased project's declarations, so `fmt` can name the canonical
            // form here too.
            shorthand_rewritable: !never_rewrite_context(
                line.raw_line,
                line.is_md,
                line.column_offset + marker_start,
            ),
            shorthand: parsed.shorthand,
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
            // An escape is check-inert; nothing rewrites it (§AR-scanner.2.5).
            shorthand_rewritable: false,
            text: line.scan_line[escape_start..token_end].to_string(),
            inline_site: None,
            source_kind: String::new(),
            enclosing_declaration: None,
        });
    }
}

/// Discover `e2e/cases/<name>/` directories and register each as an `E2E-<name>`
/// declaration whose body is the case manifest (§AR-scanner.6, §FS-show.2.4) — so
/// `grund check` sees `§E2E-…` citations resolve and `grund refs` finds e2e tests.
fn scan_e2e_cases(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    findings: &mut Findings,
) -> Result<()> {
    let Some(kind) = config.kinds.iter().find(|kind| kind.prefix == "E2E") else {
        return Ok(());
    };
    let Some(folder) = kind.folder.as_deref() else {
        return Ok(());
    };
    let cases_root = config.root.join(folder);
    if !cases_root.exists() || !cases_root.is_dir() {
        return Ok(());
    }
    let cases_root = fs::canonicalize(&cases_root).unwrap_or(cases_root);
    let mut scan_root = cases_root.clone();

    if explicit_scope {
        let scope = scope.unwrap_or(Path::new("."));
        if scope.is_file() {
            return Ok(());
        }
        let scope = fs::canonicalize(scope).unwrap_or_else(|_| scope.to_path_buf());
        if scope.starts_with(&cases_root) {
            scan_root = scope;
        } else if !cases_root.starts_with(&scope) {
            return Ok(());
        }
    } else if !config.scan_full
        && let Some(include) = &config.include
    {
        // §FS-check.1.3: `--full` cancels `include`, so the e2e cases are in the
        // walk whether or not `include` happens to name their folder.
        let covered = include.iter().any(|path| {
            let root = config.root.join(path);
            cases_root.starts_with(&root) || root.starts_with(&cases_root)
        });
        if !covered {
            return Ok(());
        }
    }

    let mut case_dirs = Vec::new();
    if scan_root.join("expected.exit").is_file() {
        case_dirs.push(scan_root);
    } else {
        for entry in fs::read_dir(&scan_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("expected.exit").is_file() {
                case_dirs.push(path);
            }
        }
    }
    case_dirs.sort_by_key(|path| sort_path_key(path));

    for dir in case_dirs {
        let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(id) = e2e_id_from_case_dir_name(config, name) else {
            continue;
        };
        let case = read_e2e_case(config, &dir)?;
        findings
            .declarations
            .entry(id.clone())
            .or_default()
            .push(Declaration {
                id,
                file: dir.clone(),
                line: 1,
                heading_level: 1,
                sections: BTreeMap::new(),
                is_stub: false,
                defined_in: None,
                e2e_case: Some(case),
                title: Some(format!("e2e case `{name}`")),
                // §AR-scanner.2.4: an E2E case spans its manifest line only; its
                // obligations evaluate over the case's scanned files, not a body.
                body_start: 1,
                body_end: 1,
            });
    }
    Ok(())
}

/// Map an `e2e/cases/<name>/` directory name to its `E2E-<name>` `Id` under the
/// repo's `[id] format` (§AR-scanner.6, §FS-config.3.4).
fn e2e_id_from_case_dir_name(config: &Config, name: &str) -> Option<Id> {
    let after_kind_literal = literal_after_kind_placeholder(&config.id_format)?;
    let raw = format!("E2E{after_kind_literal}{name}");
    let (id, section) = parse_id_arg(&raw, &config.grammar).ok()?;
    if section.is_none() && id.kind == "E2E" {
        Some(id)
    } else {
        None
    }
}

/// The literal text between `{kind}` and the next placeholder in `[id] format`
/// (e.g. `-` in `{kind}-{slug}`) — the glue an `E2E-<dirname>` ID is reassembled
/// with (§AR-scanner.6).
fn literal_after_kind_placeholder(format: &str) -> Option<&str> {
    let marker = "{kind}";
    let start = format.find(marker)? + marker.len();
    let rest = &format[start..];
    let end = rest.find('{').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Inverse of `e2e_id_from_case_dir_name`: strip the `E2E` prefix off a rendered ID
/// to get the `e2e/cases/<name>/` directory `grund id` tells the author to create
/// (§FS-id.2, §AR-scanner.6).
fn e2e_case_dir_name(config: &Config, rendered: &str) -> String {
    let prefix = format!(
        "E2E{}",
        literal_after_kind_placeholder(&config.id_format).unwrap_or("-")
    );
    rendered
        .strip_prefix(&prefix)
        .unwrap_or(rendered)
        .to_string()
}

/// Read one e2e case directory into an `E2eCase` — `command.args` (defaulting to
/// `check`), `expected.exit`, `spec.refs`, and the recursive fixture file list —
/// the data `grund E2E-<name>` renders or checks (§FS-show.2.4, §FS-config.3.9).
fn read_e2e_case(config: &Config, dir: &Path) -> Result<E2eCase> {
    let command_args = dir.join("command.args");
    let args = if command_args.is_file() {
        fs::read_to_string(&command_args)?
            .split_whitespace()
            .map(str::to_string)
            .collect()
    } else {
        vec!["check".to_string()]
    };
    let expected_exit = fs::read_to_string(dir.join("expected.exit"))?
        .trim()
        .parse::<i32>()
        .with_context(|| format!("parse {}/expected.exit", format_path(dir)))?;
    let mut fixtures = Vec::new();
    collect_relative_fixture_files(dir, dir, &mut fixtures)?;
    fixtures.sort_by_key(|path| sort_path_key(path));
    let spec_refs = read_e2e_spec_refs(config, dir)?;
    Ok(E2eCase {
        dir: dir.to_path_buf(),
        args,
        expected_exit,
        fixtures,
        spec_refs,
    })
}

fn read_e2e_spec_refs(config: &Config, dir: &Path) -> Result<Vec<E2eSpecRef>> {
    let path = dir.join("spec.refs");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    Ok(text
        .lines()
        .filter_map(|line| e2e_spec_ref_from_line(config, line.trim()))
        .collect())
}

fn e2e_spec_ref_from_line(config: &Config, line: &str) -> Option<E2eSpecRef> {
    let token = line.split_whitespace().next()?;
    if token.is_empty() {
        return None;
    }
    let token = token.strip_prefix(&config.marker).unwrap_or(token);
    let (namespace, id_text) = match token.split_once('/') {
        Some((namespace, id_text)) if !namespace.is_empty() => {
            (Some(namespace.to_string()), id_text)
        }
        _ => (None, token),
    };
    if let Ok((id, _section)) = parse_id_arg(id_text, &config.grammar) {
        return Some(E2eSpecRef {
            namespace,
            kind: id.kind,
        });
    }
    let kind = id_text.split_once('-')?.0;
    if kind.is_empty() {
        return None;
    }
    Some(E2eSpecRef {
        namespace,
        kind: kind.to_string(),
    })
}

fn collect_relative_fixture_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_fixture_files(root, &path, files)?;
        } else {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

/// Depth of a heading line — count of leading `#` — used to decide whether a
/// section heading nests under the current declaration (§AR-scanner.2.2).
fn heading_level_for_line(line: &str, markdown_heading: bool, caps: &regex::Captures) -> usize {
    if markdown_heading {
        return line
            .trim_start()
            .chars()
            .take_while(|ch| *ch == '#')
            .count()
            .max(1);
    }
    // Code-form declarations (§DF-code-declarations-drop-hash) match the branch
    // that has no `#+`, so no heading group is set; default to depth 1.
    caps.name("hashes")
        .or_else(|| caps.name("mdhashes"))
        .map(|m| m.as_str().len())
        .unwrap_or(1)
}

/// The tree walk (§AR-scanner.1): from each scan root, descend skipping hidden and
/// `[scan] exclude` directories, honouring `.gitignore` and friends unless
/// `respect_gitignore = false` (§AR-scanner.1.1, §FS-config.3.5), keeping only
/// scannable files, in a sorted order so findings are deterministic (§FS-errors.4).
fn walk_scannable_files(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Vec<PathBuf>> {
    let roots = scan_roots(config, scope, explicit_scope)?;
    let mut files = Vec::new();
    for scan_root in roots {
        if !scan_root.exists() {
            continue;
        }
        let canonical_scan_root =
            fs::canonicalize(&scan_root).unwrap_or_else(|_| scan_root.to_path_buf());
        // §AR-workspace.6: a root scan starts outside member namespaces; an
        // included path at or below a member boundary belongs to the member scan.
        if config
            .workspace_boundary_roots
            .iter()
            .any(|root| canonical_scan_root.starts_with(root))
        {
            continue;
        }
        if scan_root.is_file() {
            if is_scannable(&scan_root, config) {
                files.push(scan_root);
            }
            continue;
        }
        let mut builder = WalkBuilder::new(&scan_root);
        builder.hidden(false);
        if !config.respect_gitignore {
            builder
                .ignore(false)
                .git_ignore(false)
                .git_global(false)
                .git_exclude(false)
                .parents(false);
        }
        let excluded = config.exclude.clone();
        let e2e_cases_root = config
            .kinds
            .iter()
            .find(|kind| kind.prefix == "E2E")
            .and_then(|kind| kind.folder.as_deref())
            .map(|folder| config.root.join(folder));
        let e2e_config = config.clone();
        // §AR-workspace.6: precompute the boundary path components once,
        // expressed relative to the canonical scan root. The walker filter is
        // then a single component-suffix compare — no per-entry `canonicalize`
        // syscall, no allocation in the hot path. `strip_prefix` only removes
        // the root, so the descendant suffix is invariant under symlink
        // resolution — comparing against `scan_root_for_filter` works even if
        // `scan_root` itself is a symlink.
        let boundary_suffixes: Vec<PathBuf> = config
            .workspace_boundary_roots
            .iter()
            .filter_map(|root| root.strip_prefix(&canonical_scan_root).ok())
            .map(Path::to_path_buf)
            .collect();
        let scan_root_for_filter = scan_root.clone();
        builder.filter_entry(move |e| {
            if e.depth() == 0 {
                return true;
            }
            if e.file_type().is_some_and(|file_type| file_type.is_dir())
                && let Ok(relative) = e.path().strip_prefix(&scan_root_for_filter)
                && boundary_suffixes
                    .iter()
                    .any(|suffix| relative == suffix.as_path())
            {
                return false;
            }
            if e.file_type().is_some_and(|file_type| file_type.is_dir()) {
                if is_hidden(e.path()) {
                    return false;
                }
                if is_direct_e2e_case_dir(e.path(), e2e_cases_root.as_deref(), &e2e_config) {
                    return false;
                }
                let Some(name) = e.path().file_name().and_then(|name| name.to_str()) else {
                    return true;
                };
                return !excluded.iter().any(|item| item == name);
            }
            true
        });
        let walker = builder.build();
        for entry in walker {
            let entry = entry?;
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                || !is_scannable(entry.path(), config)
            {
                continue;
            }
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort_by_key(|path| sort_path_key(path));
    // One file, one read: the roots may overlap — `include = ["docs", "docs/api"]`
    // names one subtree twice, and under `--full` every `include` root is walked
    // beside the config root that already contains it (§FS-check.1.3). Scanning a
    // file twice would report its declaration as a duplicate of itself.
    files.dedup();
    Ok(files)
}

/// Direct `e2e/cases/<name>/` directories are E2E manifest declarations
/// (§AR-scanner.6), so the ordinary file walk must not scan their fixture repos.
fn is_direct_e2e_case_dir(path: &Path, cases_root: Option<&Path>, config: &Config) -> bool {
    let Some(cases_root) = cases_root else {
        return false;
    };
    if path.parent() != Some(cases_root) || !path.join("expected.exit").is_file() {
        return false;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| e2e_id_from_case_dir_name(config, name))
        .is_some()
}

/// The directories (or single file) the walk starts from: a `[path]` argument when
/// given (narrowing the default scope), otherwise `[scan] include` resolved against
/// the repo root, otherwise the whole root (§FS-config.3.5, §AR-scanner.1).
fn scan_roots(config: &Config, scope: Option<&Path>, explicit_scope: bool) -> Result<Vec<PathBuf>> {
    scan_roots_for(config, scope, explicit_scope, config.scan_full)
}

/// §FS-check.1.3: `full` cancels `[scan] include` for this walk and nothing else
/// — an explicit path argument still narrows, and `exclude`, the ignore files,
/// and `extensions` are untouched. `check --full` asks both ways: once with
/// `true` to walk the whole root, and once with `false` to learn which of what it
/// read was inside the configured scope (§FS-check.3.14).
fn scan_roots_for(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    full: bool,
) -> Result<Vec<PathBuf>> {
    if explicit_scope {
        let scope = scope.unwrap_or(Path::new("."));
        if !scope.exists() {
            return Err(anyhow!("path does not exist: {}", scope.display()));
        }
        let scope = fs::canonicalize(scope).unwrap_or_else(|_| scope.to_path_buf());
        if scope.is_file() {
            return Ok(vec![scope]);
        }
        if scope == config.root {
            return Ok(root_scope_roots(config, full));
        }
        return Ok(vec![scope]);
    }
    Ok(root_scope_roots(config, full))
}

/// The roots a walk of the whole config root starts from (§FS-config.3.5).
///
/// Without `--full` that is `[scan] include`, or the root itself when the key is
/// unset. With `--full` it is the root **and** every `include` root, not the root
/// alone: a walk root is never pruned by `.gitignore`, `[scan] exclude`, or the
/// hidden-directory rule, while the same directory reached as a *descendant* of
/// the config root is. Starting only at the root would therefore read *fewer*
/// files than the plain walk whenever an `include` entry is gitignored, excluded,
/// or hidden — and `--full` would turn a red run green, which is exactly what
/// §FS-check.1.3 promises it can never do. `walk_scannable_files` deduplicates
/// the file list, so an `include` root the root walk already covers is read once.
/// Collecting each root through `components()` folds away the `./` and trailing
/// separator an entry may be written with, so two roots naming one directory
/// yield byte-identical descendant paths and the dedup can recognize the reread.
fn root_scope_roots(config: &Config, full: bool) -> Vec<PathBuf> {
    let include = config
        .include
        .iter()
        .flatten()
        .map(|entry| config.root.join(entry).components().collect::<PathBuf>());
    match (full, config.include.is_some()) {
        (true, _) => std::iter::once(config.root.clone()).chain(include).collect(),
        (false, true) => include.collect(),
        (false, false) => vec![config.root.clone()],
    }
}

/// A file that could not be read or decoded during the walk. The walk continues
/// past it (§FS-check.2); callers that are point queries treat any entry here as
/// fatal, `check` and `refs` report it and exit 2 with a still-printed report.
type ScanError = (PathBuf, String);

type FileScanResult = (PathBuf, std::result::Result<Findings, String>);

fn scan_one_file(
    file: &Path,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
    overlays: &TextOverlays,
) -> FileScanResult {
    let mut findings = Findings::default();
    let result = if let Some(text) = overlay_text(overlays, file) {
        scan_file_text(file, text, config, &mut findings, workspace_targets)
    } else {
        scan_file(file, config, &mut findings, workspace_targets)
    };
    match result {
        Ok(()) => {
            findings.scanned_files.push(file.to_path_buf());
            (file.to_path_buf(), Ok(findings))
        }
        Err(err) => (file.to_path_buf(), Err(format!("{err:#}"))),
    }
}

fn merge_findings(target: &mut Findings, mut source: Findings) {
    for (id, mut declarations) in source.declarations {
        target
            .declarations
            .entry(id)
            .or_default()
            .append(&mut declarations);
    }
    target.citations.append(&mut source.citations);
    target.escaped_citations.append(&mut source.escaped_citations);
    target.scanned_files.append(&mut source.scanned_files);
}

fn scan_file_results(
    files: &[PathBuf],
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
    overlays: &TextOverlays,
) -> Vec<FileScanResult> {
    files
        .par_iter()
        .map(|file| scan_one_file(file, config, workspace_targets, overlays))
        .collect::<Vec<_>>()
}

/// One full tree walk: scan every file (§AR-scanner.2) plus the e2e case
/// directories (§AR-scanner.6), collecting unreadable files rather than aborting
/// so `check` can report them and keep going (§FS-check.2). The wrapper around
/// the workspace-aware variant with no targets — single-project scans and
/// member-local scans share this path.
fn scan_tree(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<(Findings, Vec<ScanError>)> {
    scan_tree_with_workspace(config, scope, explicit_scope, &[])
}

/// Workspace-aware tree walk: `§<alias>/<ID>` citations parse with each
/// target's grammar inline, so the workspace layer (§FS-workspace.1,
/// §AR-workspace.2) never needs to re-read the files the initial scan
/// already read.
fn scan_tree_with_workspace(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Result<(Findings, Vec<ScanError>)> {
    scan_tree_with_workspace_threshold(
        config,
        scope,
        explicit_scope,
        workspace_targets,
        PARALLEL_SCAN_MIN_FILES,
        &TextOverlays::new(),
    )
}

fn scan_tree_with_workspace_threshold(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    workspace_targets: &[WorkspaceCitationTarget],
    parallel_min_files: usize,
    overlays: &TextOverlays,
) -> Result<(Findings, Vec<ScanError>)> {
    let mut findings = Findings::default();
    let mut errors = Vec::new();
    let mut files = walk_scannable_files(config, scope, explicit_scope)?;
    add_overlay_scan_files(config, scope, explicit_scope, overlays, &mut files)?;
    if files.len() >= parallel_min_files {
        for (file, result) in scan_file_results(&files, config, workspace_targets, overlays) {
            match result {
                Ok(file_findings) => merge_findings(&mut findings, file_findings),
                Err(message) => errors.push((file, message)),
            }
        }
    } else {
        for file in files {
            match scan_one_file(&file, config, workspace_targets, overlays) {
                (_, Ok(file_findings)) => merge_findings(&mut findings, file_findings),
                (_, Err(message)) => errors.push((file, message)),
            }
        }
    }
    if let Err(err) = scan_e2e_cases(config, scope, explicit_scope, &mut findings) {
        errors.push((config.root.join("e2e/cases"), format!("{err:#}")));
    }
    // §FS-workspace.1: when the citing-grammar pass and the target-grammar
    // pass both fire on the same line they emit in source order *per pass*;
    // sort once at the end so a workspace scan's per-line citation order
    // matches the single-project scan's left-to-right invariant.
    if !workspace_targets.is_empty() {
        findings.citations.sort_by(|a, b| {
            (sort_path_key(&a.file), a.line, a.column).cmp(&(
                sort_path_key(&b.file),
                b.line,
                b.column,
            ))
        });
    }
    // §AR-scanner.2.6: shorthand citations name a declaration that may live in
    // any file, so they can only be resolved once the whole walk (including the
    // E2E cases above) has produced the declaration set.
    resolve_shorthand_citations(&mut findings);
    Ok((findings, errors))
}

fn scan_tree_with_workspace_overlays(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    workspace_targets: &[WorkspaceCitationTarget],
    overlays: &TextOverlays,
) -> Result<(Findings, Vec<ScanError>)> {
    scan_tree_with_workspace_threshold(
        config,
        scope,
        explicit_scope,
        workspace_targets,
        PARALLEL_SCAN_MIN_FILES,
        overlays,
    )
}

fn add_overlay_scan_files(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    overlays: &TextOverlays,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    if overlays.is_empty() {
        return Ok(());
    }
    let roots = scan_roots(config, scope, explicit_scope)?;
    for path in overlays.keys() {
        if !is_scannable(path, config) {
            continue;
        }
        let path = normalize_path_lexically(path);
        if !roots.iter().any(|root| path_starts_with(&path, root)) {
            continue;
        }
        if files.iter().any(|file| paths_same_location(file, &path)) {
            continue;
        }
        if path.exists() || !new_overlay_file_passes_walk_filters(config, &roots, &path) {
            continue;
        }
        files.push(path);
    }
    files.sort_by_key(|path| sort_path_key(path));
    Ok(())
}

fn new_overlay_file_passes_walk_filters(config: &Config, roots: &[PathBuf], path: &Path) -> bool {
    roots.iter().any(|root| {
        let root = canonicalize_existing_prefix(root);
        let path = canonicalize_existing_prefix(path);
        if path == root || !path.starts_with(&root) {
            return false;
        }
        if config
            .workspace_boundary_roots
            .iter()
            .any(|boundary| path_starts_with(&path, boundary))
        {
            return false;
        }
        let Ok(relative) = path.strip_prefix(&root) else {
            return false;
        };
        let components: Vec<_> = relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect();
        if components
            .iter()
            .any(|component| component.starts_with('.'))
        {
            return false;
        }
        if components
            .iter()
            .take(components.len().saturating_sub(1))
            .any(|component| config.exclude.iter().any(|excluded| excluded == component))
        {
            return false;
        }
        let e2e_cases_root = config
            .kinds
            .iter()
            .find(|kind| kind.prefix == "E2E")
            .and_then(|kind| kind.folder.as_deref())
            .map(|folder| config.root.join(folder));
        let mut ancestor = path.parent();
        while let Some(dir) = ancestor {
            if dir == root {
                break;
            }
            if is_direct_e2e_case_dir(dir, e2e_cases_root.as_deref(), config) {
                return false;
            }
            ancestor = dir.parent();
        }
        !path_ignored_by_gitignore(config, &root, &path)
    })
}

fn path_ignored_by_gitignore(config: &Config, root: &Path, path: &Path) -> bool {
    if !config.respect_gitignore {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let mut dirs = Vec::new();
    let mut cursor = Some(parent);
    while let Some(dir) = cursor {
        dirs.push(dir);
        if dir == root {
            break;
        }
        cursor = dir.parent();
    }
    dirs.reverse();
    let mut ignored = false;
    for dir in dirs {
        let gitignore = dir.join(".gitignore");
        if !gitignore.is_file() {
            continue;
        }
        let mut builder = ignore::gitignore::GitignoreBuilder::new(dir);
        if builder.add(&gitignore).is_some() {
            continue;
        }
        let Ok(matcher) = builder.build() else {
            continue;
        };
        let matched = matcher.matched_path_or_any_parents(path, false);
        if matched.is_ignore() {
            ignored = true;
        } else if matched.is_whitelist() {
            ignored = false;
        }
    }
    ignored
}

fn path_starts_with(path: &Path, root: &Path) -> bool {
    let path = canonicalize_existing_prefix(path);
    let root = canonicalize_existing_prefix(root);
    path == root || path.starts_with(root)
}

fn canonicalize_existing_prefix(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        normalize_path_lexically(path)
    } else {
        std::env::current_dir()
            .map(|cwd| normalize_path_lexically(&cwd.join(path)))
            .unwrap_or_else(|_| normalize_path_lexically(path))
    };
    if let Ok(canonical) = fs::canonicalize(&path) {
        return canonical;
    }
    let mut suffix = PathBuf::new();
    let mut cursor = path.as_path();
    while !cursor.exists() {
        let Some(name) = cursor.file_name() else {
            break;
        };
        suffix = Path::new(name).join(suffix);
        let Some(parent) = cursor.parent() else {
            break;
        };
        cursor = parent;
    }
    fs::canonicalize(cursor)
        .unwrap_or_else(|_| normalize_path_lexically(cursor))
        .join(suffix)
}

fn overlay_text<'a>(overlays: &'a TextOverlays, path: &Path) -> Option<&'a str> {
    overlays
        .get(&normalize_path_lexically(path))
        .or_else(|| overlays.get(path))
        .map(String::as_str)
}

/// Scan helper for point-query subcommands (`show`, `id`): any unreadable file
/// is fatal — a partial view of the tree could miss the declaration entirely or
/// allocate a colliding number (§FS-show.3, §FS-id.4).
fn scan_tree_strict(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Findings> {
    let (findings, errors) = scan_tree(config, scope, explicit_scope)?;
    if let Some((path, message)) = errors.into_iter().next() {
        return Err(anyhow!("{}: {}", display_path(config, &path), message));
    }
    Ok(findings)
}
