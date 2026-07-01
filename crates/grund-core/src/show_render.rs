fn show_declaration(
    config: &Config,
    findings: &Findings,
    id: &Id,
    section: Option<&str>,
    mode: ShowRenderMode,
    include_heading: bool,
) -> Result<ShowOutput> {
    show_declaration_with_overlays(
        config,
        findings,
        id,
        section,
        mode,
        include_heading,
        &TextOverlays::new(),
    )
}

fn show_declaration_with_overlays(
    config: &Config,
    findings: &Findings,
    id: &Id,
    section: Option<&str>,
    mode: ShowRenderMode,
    include_heading: bool,
    overlays: &TextOverlays,
) -> Result<ShowOutput> {
    let root = &config.root;
    let decls = findings
        .declarations
        .get(id)
        .ok_or_else(|| anyhow!("ID not found: {}", render_id(config, id)))?;
    let homes: Vec<&Declaration> = decls
        .iter()
        .filter(|decl| !is_stub_for_inline_decl(root, decl, decls))
        .collect();
    if homes.len() > 1 {
        let mut sites: Vec<String> = homes
            .iter()
            .map(|d| format!("{}:{}", display_path(config, &d.file), d.line))
            .collect();
        sites.sort();
        return Err(anyhow!(
            "ambiguous ID: {} (declared at {})",
            render_id(config, id),
            sites.join(", ")
        ));
    }
    let decl = decls.iter().find(|decl| decl.is_stub).unwrap_or(&decls[0]);
    if let Some(case) = &decl.e2e_case {
        return show_e2e_case(config, id, case, section, mode);
    }
    let file = if let Some(target) = &decl.defined_in {
        resolve_stub_target(root, &decl.file, target)
    } else {
        decl.file.clone()
    };
    if decl.is_stub {
        if !file.exists() {
            return Err(anyhow!(
                "broken stub: {} (stub at {}:{} points at {}, which does not exist)",
                render_id(config, id),
                display_path(config, &decl.file),
                decl.line,
                format_path(decl.defined_in.as_ref().unwrap())
            ));
        }
        if !file_declares_inline_home(&file, id, config).unwrap_or(false) {
            return Err(anyhow!(
                "broken stub: {} (stub at {}:{} points at {}, which contains no inline declaration of {})",
                render_id(config, id),
                display_path(config, &decl.file),
                decl.line,
                format_path(decl.defined_in.as_ref().unwrap()),
                render_id(config, id)
            ));
        }
    }
    extract_declaration_body(&file, id, section, mode, include_heading, config, overlays)
}

/// Render an e2e case as an ID-query body: the invocation, expected exit, and
/// fixture list (or just the invocation with `--brief`), plus the JSON shape — the
/// case manifest of §FS-show.2.4. E2E declarations have no sections, so any
/// `.<section>` is "section not found".
fn show_e2e_case(
    config: &Config,
    id: &Id,
    case: &E2eCase,
    section: Option<&str>,
    mode: ShowRenderMode,
) -> Result<ShowOutput> {
    if let Some(section) = section {
        return Err(anyhow!(
            "section not found: {}{}{}",
            render_id(config, id),
            config.section_separator,
            section
        ));
    }
    let invocation = format!("grund {}", case.args.join(" "));
    let brief_body = format!("{invocation}\n");
    let manifest = {
        let mut lines = vec![
            invocation.clone(),
            format!("expected exit: {}", case.expected_exit),
            "fixtures:".to_string(),
        ];
        lines.extend(
            case.fixtures
                .iter()
                .map(|path| format!("- {}", format_path(path))),
        );
        format!("{}\n", lines.join("\n"))
    };
    let body = match mode {
        ShowRenderMode::Brief => brief_body,
        ShowRenderMode::Outline => String::new(),
        ShowRenderMode::Default | ShowRenderMode::Toc | ShowRenderMode::Full => manifest,
    };
    let args_json = case
        .args
        .iter()
        .map(|arg| format!("\"{}\"", json_escape(arg)))
        .collect::<Vec<_>>()
        .join(",");
    let fixtures_json = case
        .fixtures
        .iter()
        .map(|path| format!("\"{}\"", json_escape(&format_path(path))))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        "{{\"id\":\"{}\",\"kind\":\"E2E\",\"path\":\"{}\",\"args\":[{}],\"expected_exit\":{},\"fixtures\":[{}]}}",
        json_escape(&render_id(config, id)),
        json_escape(&display_path(config, &case.dir)),
        args_json,
        case.expected_exit,
        fixtures_json
    );
    Ok(ShowOutput {
        body,
        path: case.dir.clone(),
        line: 1,
        json: Some(json),
        sections: Vec::new(),
    })
}

/// Pull the body text of a declaration out of its file: the lines under the
/// `# <ID>: …` heading down to the next same-or-shallower heading (§FS-show.2.1),
/// optionally just one numbered subsection (§FS-show.2.2) or just the lead
/// paragraph (§FS-show.2.1.1). For an inline declaration in a code/`"""` doc-comment
/// this walks the comment block (§FS-show.2.3.1) and strips comment markers
/// (§FS-show.2.3.2) before returning the text.
fn extract_declaration_body(
    path: &Path,
    id: &Id,
    section: Option<&str>,
    mode: ShowRenderMode,
    include_heading: bool,
    config: &Config,
    overlays: &TextOverlays,
) -> Result<ShowOutput> {
    // `--toc` = the default lead, then a blank line, then the nested section
    // headings (§FS-show.2.1.2). Internally: compose the Default body with an
    // Outline-only scan, sharing the same `(path, id, section)` resolution.
    if mode == ShowRenderMode::Toc {
        let mut default_output = extract_declaration_body(
            path,
            id,
            section,
            ShowRenderMode::Default,
            include_heading,
            config,
            overlays,
        )?;
        let outline_output = extract_declaration_body(
            path,
            id,
            section,
            ShowRenderMode::Outline,
            false,
            config,
            overlays,
        )?;
        default_output.body = join_with_blank(&default_output.body, &outline_output.body);
        default_output.sections = outline_output.sections;
        return Ok(default_output);
    }

    // `--brief` = heading + first paragraph (§FS-show.2.1.1). The heading is
    // always included (H1 for a whole declaration, section heading for a
    // selected section) so the slice is self-labeled regardless of `text` vs
    // `md`. When a section is selected we suppress the H1 — only the most
    // specific heading is kept.
    if mode == ShowRenderMode::Brief {
        let want_h1_for_default = section.is_none();
        let mut output = extract_declaration_body(
            path,
            id,
            section,
            ShowRenderMode::Default,
            want_h1_for_default,
            config,
            overlays,
        )?;
        output.body = truncate_to_first_paragraph(&output.body);
        return Ok(output);
    }

    let text = read_text_with_overlays(path, overlays)?;
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

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        let scan_line = scan.text;
        if in_decl && scan.in_py_docstring && scan.closed_py_docstring && scan_line.trim().is_empty() {
            break;
        }
        if let Some(caps) =
            declaration_captures(&config.grammar, scan_line, scan.in_py_docstring, is_md)
        {
            let found = parse_id(&caps);
            if in_decl && found.as_ref() != Some(id) {
                break;
            }
            if found.as_ref() == Some(id) {
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
        if let Some(caps) = config.grammar.section_re.captures(scan_line) {
            let sec = caps.name("sec").map(|m| m.as_str()).unwrap_or("");
            let depth = sec.split('.').count();
            match section {
                // Whole-declaration lead: stop at the first numbered subsection.
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
                    if sec == target {
                        found_section = true;
                        target_depth = depth;
                        output_line = lineno;
                        if mode != ShowRenderMode::Outline {
                            lines.push(clean_body_line(scan_line, is_md || scan.in_py_docstring));
                        }
                        continue;
                    }
                    // Inside the target section: a sibling-or-shallower heading
                    // ends it (§FS-show.2.2); in default mode any further numbered
                    // heading — including a child — ends the section's lead prose
                    // (§FS-show.2.2). Before the target section is found, keep
                    // scanning past unrelated headings.
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

fn read_text_with_overlays(path: &Path, overlays: &TextOverlays) -> Result<String> {
    if let Some(text) = overlay_text(overlays, path) {
        Ok(text.to_string())
    } else {
        fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
    }
}

/// `--toc` joins the default body with the section-map body, separated by one
/// blank line. Empty halves are dropped; if both are empty the result is empty.
/// Each body already ends with `\n`, so `{a}\n{b}` produces `<a>\n\n<b>\n`
/// (§FS-show.2.1.2).
fn join_with_blank(default_body: &str, outline_body: &str) -> String {
    match (default_body.is_empty(), outline_body.is_empty()) {
        (true, true) => String::new(),
        (true, false) => outline_body.to_string(),
        (false, true) => default_body.to_string(),
        (false, false) => format!("{default_body}\n{outline_body}"),
    }
}

/// `--brief` truncates the (default-mode, heading-included) body to its first
/// blank-line-separated paragraph (§FS-show.2.1.1). Keeps the heading line and
/// at most one blank-line separator before the first paragraph; stops at the
/// next blank line (or end of body).
fn truncate_to_first_paragraph(body: &str) -> String {
    let mut lines: Vec<&str> = body.split('\n').collect();
    // `body` ends with `\n`, so the split produces a trailing empty element.
    if lines.last() == Some(&"") {
        lines.pop();
    }
    if lines.is_empty() {
        return String::new();
    }
    let mut out: Vec<&str> = vec![lines[0]];
    let mut i = 1;
    let mut kept_separator = false;
    while i < lines.len() && lines[i].trim().is_empty() {
        if !kept_separator {
            out.push(lines[i]);
            kept_separator = true;
        }
        i += 1;
    }
    while i < lines.len() && !lines[i].trim().is_empty() {
        out.push(lines[i]);
        i += 1;
    }
    while out.last().is_some_and(|line| line.trim().is_empty()) {
        out.pop();
    }
    if out.is_empty() {
        String::new()
    } else {
        format!("{}\n", out.join("\n"))
    }
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
        .trim_start_matches('.')
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
