/// Declaration-body extraction: where a declaration's body begins and ends in
/// the file that holds it, across Markdown and every supported comment dialect
/// (§FS-show.2.1, §FS-show.2.2, §FS-show.2.3).
///
/// This is a question about *source structure*, not about rendering — the same
/// question §AR-scanner.2.4 answers for the citing side — so it sits beside the
/// renderer rather than inside it (§AR-core-module-layout.1). `show_render.rs`
/// keeps what is genuinely rendering: the entry points, the E2E case manifest,
/// and the `--toc` / `--brief` shaping of what this file returns.

/// Pull the body text of a declaration out of its file: the lines under the
/// `# <ID>: …` heading down to the next same-or-shallower heading (§FS-show.2.1),
/// optionally just one citable subsection (§FS-show.2.2) or just the lead
/// paragraph (§FS-show.2.1.1). For an inline declaration in a code/`"""` doc-comment
/// this walks the comment block (§FS-show.2.3.1) and strips comment markers
/// (§FS-show.2.3.2) before returning the text.
///
/// Why `--brief` always keeps a heading: the slice has to be self-labeled whatever
/// the output format, `text` or `md`, so it carries the H1 for a whole declaration
/// and the section heading for a selected section. When a section is selected the
/// H1 is suppressed — only the most specific heading is kept.
///
/// Why a fence suspends structure: inside a Markdown fence nothing is structure —
/// not a section heading, not a declaration heading. Fences are tracked exactly as
/// §AR-scanner.2.2's scan tracks them, Markdown only, so the section map `check`
/// reads and the body this returns are bounded by the same lines.
///
/// Why a repeated section path terminates the section: a *second* heading claiming
/// the requested path does not continue the section, it ends it the way a sibling
/// heading does, so two bodies are never merged into a slice no heading spans. Such
/// a query does not reach here at all — it is refused from the scanner's record
/// before the body is read (`ambiguous_section_refusal`). Before the target section
/// is found, unrelated headings are scanned past rather than ending anything.
fn extract_declaration_body(
    path: &Path,
    id: &Id,
    declaration: &Declaration,
    section: Option<&str>,
    mode: ShowRenderMode,
    include_heading: bool,
    config: &Config,
    overlays: &TextOverlays,
) -> Result<ShowOutput> {
    let mut cache = PointBodyCache::new(overlays);
    extract_declaration_body_cached(
        &mut cache,
        path,
        id,
        section,
        mode,
        include_heading,
        config,
        Some(PointBodySite {
            declaration_line: declaration.line,
            declaration_body_end: (section.is_some()
                || declaration.body_end > declaration.line)
                .then_some(declaration.body_end),
            section_line: section.and_then(|path| declaration.sections.get(path).map(|info| info.line)),
        }),
    )
}

/// The exact scanner-recorded site whose body is being sliced. `show` supplies
/// it after establishing uniqueness so an assigned scanner body span bounds the
/// source slice too; size rows additionally use it to keep duplicate homes and
/// duplicate section coordinates site-local (§FS-show.2.1.2, §FS-list.3.4).
#[derive(Clone, Copy)]
struct PointBodySite {
    declaration_line: usize,
    /// Present when the scanner computed a meaningful body span. A read-only
    /// scan without sections retains its lazy single-line placeholder and lets
    /// the slicer derive that declaration's boundary itself (§AR-benchmarks).
    declaration_body_end: Option<usize>,
    section_line: Option<usize>,
}

/// Per-operation source cache shared by list and check point measurements. It
/// owns no parsing rules: the cached bytes still flow through the exact show
/// slicer below (§FS-list.3.4, §FS-check.4.13).
struct PointBodyCache<'a> {
    overlays: &'a TextOverlays,
    text: BTreeMap<PathBuf, String>,
}

impl<'a> PointBodyCache<'a> {
    fn new(overlays: &'a TextOverlays) -> Self {
        Self {
            overlays,
            text: BTreeMap::new(),
        }
    }

    fn read(&mut self, path: &Path) -> Result<&str> {
        if !self.text.contains_key(path) {
            let text = read_text_with_overlays(path, self.overlays)?;
            self.text.insert(path.to_path_buf(), text);
        }
        Ok(self.text.get(path).expect("cached point source"))
    }
}

/// Return the lead/full text pair for one catalog site. JSON values and E2E
/// cases already carry their canonical show bodies in scanner records; text
/// declarations use the cached show slicer. A retained stub is broken (healthy
/// stub rows collapse onto their inline home) and therefore unmeasurable
/// (§FS-list.2, §FS-list.3.4).
fn point_body_pair(
    cache: &mut PointBodyCache<'_>,
    config: &Config,
    id: &Id,
    declaration: &Declaration,
    section: Option<(&str, &SectionInfo)>,
) -> Result<Option<(String, String)>> {
    if declaration.is_stub {
        return Ok(None);
    }
    if matches!(declaration.source, DeclarationSource::Json { .. }) {
        let body = match section {
            Some((_, info)) => info
                .value
                .as_ref()
                .map(|value| value.source_slice.clone())
                .unwrap_or_default(),
            None => match &declaration.source {
                DeclarationSource::Json { member_slice, .. } => member_slice.clone(),
                DeclarationSource::Text => unreachable!("guarded JSON source"),
            },
        };
        return Ok(Some((body.clone(), body)));
    }
    if let Some(case) = &declaration.e2e_case {
        let lead = show_e2e_case(config, config, id, case, None, ShowRenderMode::Default)?.body;
        let full = show_e2e_case(config, config, id, case, None, ShowRenderMode::Full)?.body;
        return Ok(Some((lead, full)));
    }

    let section_path = section.map(|(path, _)| path);
    let site = PointBodySite {
        declaration_line: declaration.line,
        declaration_body_end: (section.is_some() || declaration.body_end > declaration.line)
            .then_some(declaration.body_end),
        section_line: section.map(|(_, info)| info.line),
    };
    let mut lead = extract_declaration_body_cached(
        cache,
        &declaration.file,
        id,
        section_path,
        ShowRenderMode::Default,
        false,
        config,
        Some(site),
    )?
    .body;
    let mut full = extract_declaration_body_cached(
        cache,
        &declaration.file,
        id,
        section_path,
        ShowRenderMode::Full,
        false,
        config,
        Some(site),
    )?
    .body;
    // Text and JSON `show` flatten generated cross-reference wrappers before
    // exposing their bodies; point measurements promise those same bytes
    // (§FS-show.3.2, §FS-list.3.4).
    lead = flatten_cross_ref_links(&lead, config);
    full = flatten_cross_ref_links(&full, config);
    Ok(Some((lead, full)))
}

/// Byte-defined size counting with no locale or Unicode-table input
/// (§FS-list.3.4).
fn measure_point_text(text: &str, unit: PointSizeUnit) -> usize {
    let ascii_space = |byte: &u8| matches!(*byte, b'\t'..=b'\r' | b' ');
    match unit {
        PointSizeUnit::Lines => text
            .as_bytes()
            .split(|byte| *byte == b'\n')
            .filter(|line| line.iter().any(|byte| !ascii_space(byte)))
            .count(),
        PointSizeUnit::Words => text
            .as_bytes()
            .split(ascii_space)
            .filter(|word| !word.is_empty())
            .count(),
        PointSizeUnit::Bytes => text.len(),
    }
}

/// Shared show slicer, optionally pinned to one scanner-recorded site so the
/// size catalog can expose duplicates without making them resolvable
/// (§FS-show.2.1, §FS-list.3.4).
fn extract_declaration_body_cached(
    cache: &mut PointBodyCache<'_>,
    path: &Path,
    id: &Id,
    section: Option<&str>,
    mode: ShowRenderMode,
    include_heading: bool,
    config: &Config,
    site: Option<PointBodySite>,
) -> Result<ShowOutput> {
    // `--toc` = the default lead, then a blank line, then the nested section
    // headings (§FS-show.2.1.2). Internally: compose the Default body with an
    // Outline-only scan, sharing the same `(path, id, section)` resolution.
    if mode == ShowRenderMode::Toc {
        let mut default_output = extract_declaration_body_cached(
            cache,
            path,
            id,
            section,
            ShowRenderMode::Default,
            include_heading,
            config,
            site,
        )?;
        let outline_output = extract_declaration_body_cached(
            cache,
            path,
            id,
            section,
            ShowRenderMode::Outline,
            false,
            config,
            site,
        )?;
        default_output.body = join_with_blank(&default_output.body, &outline_output.body);
        default_output.sections = outline_output.sections;
        return Ok(default_output);
    }

    // `--brief` = heading + first paragraph (§FS-show.2.1.1); the H1 is
    // suppressed when a section is selected.
    if mode == ShowRenderMode::Brief {
        let want_h1_for_default = section.is_none();
        let mut output = extract_declaration_body_cached(
            cache,
            path,
            id,
            section,
            ShowRenderMode::Default,
            want_h1_for_default,
            config,
            site,
        )?;
        output.body = truncate_to_first_paragraph(&output.body);
        return Ok(output);
    }

    let text = cache.read(path)?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let mut in_decl = false;
    let mut line_style_comment = false;
    let mut py_docstring = PythonDocstringScanState::default();
    let mut found_section = section.is_none();
    let mut target_depth = usize::MAX;
    let mut lines = Vec::new();
    let mut sections = Vec::new();
    let mut output_line = 1;
    let mut markdown_fence = None;

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        if in_decl
            && site
                .and_then(|site| site.declaration_body_end)
                .is_some_and(|body_end| lineno > body_end)
        {
            break;
        }
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        let scan_line = scan.text;
        if in_decl && scan.in_py_docstring && scan.closed_py_docstring && scan_line.trim().is_empty() {
            break;
        }
        // §FS-show.2.5: inside a Markdown fence nothing is structure. The
        // delimiters and their contents are body text and still reach `lines`
        // below.
        let was_fenced = markdown_fence.is_some();
        let fence_delimiter = is_md && markdown_fence_delimiter(&mut markdown_fence, line);
        let fenced = was_fenced || fence_delimiter;
        if !fenced
            && let Some((found, _)) =
                declaration_id_on_line(&config.grammar, scan_line, scan.in_py_docstring, is_md)
        {
            if in_decl && (site.is_some() || &found != id) {
                break;
            }
            let selected_declaration = site
                .map(|site| site.declaration_line == lineno)
                .unwrap_or(true);
            if &found == id && selected_declaration {
                in_decl = true;
                line_style_comment = is_line_style_comment_line(scan_line);
                output_line = lineno;
                // `md` format keeps the heading verbatim — including for `--brief`,
                // which then prints heading + first paragraph (§FS-show.3.1).
                if include_heading {
                    lines.push(clean_body_line(scan_line, is_md || scan.in_py_docstring));
                }
                if scan.closed_py_docstring {
                    break;
                }
                continue;
            }
        }
        if !in_decl {
            continue;
        }
        if !is_md {
            let blank = line.trim().is_empty();
            if scan.in_py_docstring {
                // Python docstring content is plain Markdown; delimiter-only
                // triple-quote lines are skipped by `source_scan_line`
                // (§AR-scanner.4, §FS-show.2.3.2).
            } else if blank {
                // A blank line ends a line-style comment block (`//`, `#`, …);
                // inside a `/* … */` block or a docstring it is part of the body
                // (§FS-show.2.3.1).
                if line_style_comment {
                    break;
                }
            } else if !is_comment_body_line(scan_line) {
                break;
            }
        }
        if !fenced
            && let Some(caps) = config.grammar.section_re.captures(scan_line)
        {
            let sec = section_path(&caps).unwrap_or("");
            let depth = sec.split('.').count();
            match section {
                // Whole-declaration lead: stop at the first citable subsection.
                None => {
                    if mode == ShowRenderMode::Default {
                        break;
                    }
                    if mode == ShowRenderMode::Outline {
                        push_outline_section(
                            &mut lines,
                            &mut sections,
                            scan_line,
                            sec,
                            depth,
                            is_md || scan.in_py_docstring,
                        );
                        continue;
                    }
                }
                Some(target) => {
                    // `!found_section`: a *second* heading claiming the requested
                    // path terminates the section rather than continuing it
                    // (§DF-duplicate-section-path.1, §FS-show.2.2.2).
                    let selected_section = site
                        .and_then(|site| site.section_line)
                        .map(|line| line == lineno)
                        .unwrap_or(true);
                    if sec == target && !found_section && selected_section {
                        found_section = true;
                        target_depth = depth;
                        output_line = lineno;
                        if mode != ShowRenderMode::Outline {
                            lines.push(clean_body_line(scan_line, is_md || scan.in_py_docstring));
                        }
                        continue;
                    }
                    // Inside the target section: a sibling-or-shallower heading ends
                    // it, and in default mode any further numbered heading — including
                    // a child — ends the section's lead prose (§FS-show.2.2).
                    if found_section && (mode == ShowRenderMode::Default || depth <= target_depth) {
                        break;
                    }
                    if found_section && mode == ShowRenderMode::Outline {
                        push_outline_section(
                            &mut lines,
                            &mut sections,
                            scan_line,
                            sec,
                            depth - target_depth,
                            is_md || scan.in_py_docstring,
                        );
                        continue;
                    }
                }
            }
        }
        if found_section && mode != ShowRenderMode::Outline {
            lines.push(clean_body_line(scan_line, is_md || scan.in_py_docstring));
        }
        if in_decl && scan.closed_py_docstring {
            break;
        }
    }

    if !in_decl {
        return Err(anyhow!("ID not found: {}", render_id(config, id)));
    }
    if !found_section {
        return Err(anyhow!(
            "section not found: {}{}{}",
            render_id(config, id),
            config.section_separator,
            section.unwrap_or("")
        ));
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let body = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };
    Ok(ShowOutput {
        body,
        path: path.to_path_buf(),
        line: output_line,
        json: None,
        sections,
    })
}

fn push_outline_section(
    lines: &mut Vec<String>,
    sections: &mut Vec<ShowSection>,
    line: &str,
    section: &str,
    depth: usize,
    markdown_heading: bool,
) {
    lines.push(clean_body_line(line, markdown_heading));
    sections.push(ShowSection {
        path: section.to_string(),
        title: section_title(line, section, markdown_heading),
        depth,
    });
}

fn section_title(line: &str, section: &str, markdown_heading: bool) -> String {
    let clean = clean_body_line(line, markdown_heading);
    clean
        .trim_start()
        .trim_start_matches('#')
        .trim_start()
        .trim_start_matches(section)
        .trim_start_matches(['.', ':'])
        .trim_start()
        .to_string()
}

/// Strip the comment marker (`///`, `//!`, `//`, `#`, `*`, `/*`, `*/`) off a body
/// line when the declaration lives in a code/`"""` doc-comment — Markdown bodies
/// pass through unchanged (§FS-show.2.3.2).
fn clean_body_line(line: &str, is_md: bool) -> String {
    if is_md {
        return line.to_string();
    }

    let marker_start = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(line.len());
    let (leading, body) = line.split_at(marker_start);
    for prefix in ["///", "//!", "//", "*/", "#", "*", "/*"] {
        if let Some(rest) = body.strip_prefix(prefix) {
            if prefix == "*/" && rest.trim().is_empty() {
                return String::new();
            }
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            return format!("{leading}{rest}");
        }
    }
    line.to_string()
}

/// Whether a line still looks like part of the comment block — used to decide
/// where an inline declaration's body ends (§FS-show.2.3.1).
fn is_comment_body_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    ["///", "//!", "//", "#", "*", "/*", "*/"]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

/// Whether a declaration heading line sits inside a *line-style* comment
/// (`//`-family, `#`, `;`, `--`) as opposed to a `/* … */` block (which opens
/// `*` continuation lines). Line-style blocks end at a blank line; block-style
/// ones end at `*/` (§FS-show.2.3.1).
fn is_line_style_comment_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with(';')
        || trimmed.starts_with("--")
}
