fn show_declaration(
    config: &Config,
    path_config: &Config,
    findings: &Findings,
    id: &Id,
    section: Option<&str>,
    mode: ShowRenderMode,
    include_heading: bool,
) -> Result<ShowOutput> {
    show_declaration_with_overlays(
        config,
        path_config,
        findings,
        id,
        section,
        mode,
        include_heading,
        &TextOverlays::new(),
    )
}

/// `path_config` renders every path this function reports (§FS-config.3.6) — the
/// sites a refusal names (§FS-errors.3) as well as the E2E manifest's, whose
/// JSON is baked here rather than in `render_show_output_json`. It must be the
/// same config the caller hands that renderer, so a member's declaration reports
/// against the root the rest of the run spells its paths from
/// (§FS-workspace.8.1). `config` stays the *project's*: it owns the ID grammar
/// `render_id` reads and the tree the body is read out of.
fn show_declaration_with_overlays(
    config: &Config,
    path_config: &Config,
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
        // §FS-errors.3: every path this refusal names is spelled from the report
        // root, like the `path` of any diagnostic printed beside it.
        let mut sites: Vec<String> = homes
            .iter()
            .map(|d| format!("{}:{}", display_path(path_config, &d.file), d.line))
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
        return show_e2e_case(config, path_config, id, case, section, mode);
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
                display_path(path_config, &decl.file),
                decl.line,
                format_path(decl.defined_in.as_ref().unwrap())
            ));
        }
        if !file_declares_inline_home(&file, id, config).unwrap_or(false) {
            return Err(anyhow!(
                "broken stub: {} (stub at {}:{} points at {}, which contains no inline declaration of {})",
                render_id(config, id),
                display_path(path_config, &decl.file),
                decl.line,
                format_path(decl.defined_in.as_ref().unwrap()),
                render_id(config, id)
            ));
        }
    }
    if let Some(section) = section
        && let Some(refusal) = ambiguous_section_refusal(config, path_config, decls, decl, &file, id, section)
    {
        return Err(refusal);
    }
    extract_declaration_body(&file, id, section, mode, include_heading, config, overlays)
}

/// §FS-show.2.2.2: refuse a section coordinate two headings claim, before the
/// body is read.
///
/// The claimants are the ones the *scan* recorded (§AR-scanner.2.2) — the same
/// record §FS-check.3.16 reports from — so `show` refuses exactly the
/// coordinates `check` names and no others. Re-detecting them while extracting
/// the body would be a second, weaker reader: it would have to redo the fence
/// tracking, the heading-level gate, and the body bounds, and any of the three
/// getting a different answer is a coordinate one command calls clean and the
/// other will not resolve.
///
/// For a stub the sections belong to the **inline home**, which is the file the
/// body comes out of; a stub's own prose declares none (§FS-check.3.16). Paths
/// in the message use `path_config`, the report-path config named on
/// `show_declaration_with_overlays`.
fn ambiguous_section_refusal(
    config: &Config,
    path_config: &Config,
    decls: &[Declaration],
    decl: &Declaration,
    file: &Path,
    id: &Id,
    section: &str,
) -> Option<anyhow::Error> {
    let body_decl = if decl.is_stub {
        decls
            .iter()
            .find(|other| paths_same_location(&other.file, file))
            .unwrap_or(decl)
    } else {
        decl
    };
    let mut lines: Vec<usize> = body_decl
        .duplicate_sections
        .iter()
        .filter(|(path, _)| path == section)
        .map(|(_, info)| info.line)
        .collect();
    if lines.is_empty() {
        return None;
    }
    // The map holds the first claimant (§AR-scanner.2.2); the list holds the
    // rest. Sorted so the sites read in file order, as §FS-show.2.2.1 requires.
    lines.extend(body_decl.sections.get(section).map(|first| first.line));
    lines.sort_unstable();
    let rendered = display_path(path_config, file);
    let sites = lines
        .iter()
        .map(|line| format!("{rendered}:{line}"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(anyhow!(
        "ambiguous section: {}{}{} (declared at {sites})",
        render_id(config, id),
        config.section_separator,
        section
    ))
}

/// Render an e2e case as an ID-query body: the invocation, expected exit, and
/// fixture list (or just the invocation with `--brief`), plus the JSON shape — the
/// case manifest of §FS-show.2.4. E2E declarations have no sections, so any
/// `.<section>` is "section not found".
fn show_e2e_case(
    config: &Config,
    path_config: &Config,
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
        // path_config, not config: an `<alias>/E2E-x` shown from a workspace
        // root must report the same root-relative path as every other kind
        // (§FS-workspace.8.1) — this baked JSON bypasses render_show_output_json.
        json_escape(&display_path(path_config, &case.dir)),
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
