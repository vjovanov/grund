/// Compute the link URL for a citation: a repo-relative path to the declaration's
/// home file — following an inline-spec stub to its real source file — plus a
/// heading anchor whenever the home is Markdown: the cited section's heading for a
/// `.<section>` citation, the declaration's own heading for a bare-ID citation
/// (§FS-fmt.6.2, §DF-md-link-anchor-strategy, §DF-declaration-anchor). A source-file
/// home (a stub's target) and the `none` profile both get a bare file link.
/// `None` if the ID does not resolve (§FS-fmt.6.3).
fn markdown_link_target(
    from_file: &Path,
    id: &Id,
    section: Option<&str>,
    config: &Config,
    findings: &Findings,
) -> Option<String> {
    markdown_link_target_with_root(from_file, id, section, config, findings, None)
}

/// §FS-workspace.8.5: same as `markdown_link_target`, but with an explicit
/// `path_root` override for relative-path computation. The target's `config`
/// still drives anchor profile (§FS-fmt.6.7) and stub resolution, but the
/// link path is anchored at `path_root` (the workspace root) when the
/// citing file and the target's home live in different projects.
fn markdown_link_target_with_root(
    from_file: &Path,
    id: &Id,
    section: Option<&str>,
    config: &Config,
    findings: &Findings,
    path_root: Option<&Path>,
) -> Option<String> {
    let decls = findings.declarations.get(id)?;
    let stub = decls.iter().find(|decl| decl.is_stub);
    let home_decl = decls
        .iter()
        .find(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
        .or_else(|| decls.first())?;
    let home = if let Some(stub) = stub {
        let target = stub.defined_in.as_ref()?;
        resolve_stub_target(&config.root, &stub.file, target)
    } else {
        home_decl.file.clone()
    };
    let rel = match path_root {
        Some(root) => relative_url_under(from_file, &home, root),
        None => relative_url(from_file, &home, config),
    };
    let is_md = home.extension().and_then(|e| e.to_str()) == Some("md");
    if !is_md || config.cross_ref_anchor_format == "none" {
        return Some(rel);
    }
    let heading = match section {
        Some(sec) => home_decl
            .sections
            .get(sec)
            .map(|section| section.title.clone())
            .or_else(|| section_heading_text(&home, id, sec, config).ok().flatten())?,
        // §DF-declaration-anchor: a bare-ID citation to a Markdown home links to
        // that declaration's own heading anchor, not just the file.
        None => declaration_heading_text(home_decl, config),
    };
    let anchor = anchor_slug(&heading, &config.cross_ref_anchor_format);
    Some(format!("{}#{}", rel, anchor))
}

/// The text content of a Markdown declaration's `# <ID>: <title>` heading — the
/// `<ID>` rendered per `[id] format`, then `: <title>` if the heading carries one — i.e.
/// what a renderer slugifies for the declaration's own anchor. The title is reduced
/// to its rendered form (`reduce_heading_text`), matching `section_anchor_text`
/// (§DF-declaration-anchor, §DF-github-anchor-fidelity).
fn declaration_heading_text(decl: &Declaration, config: &Config) -> String {
    let id = render_id(config, &decl.id);
    match &decl.title {
        Some(title) => format!("{id}: {}", reduce_heading_text(title)),
        None => id,
    }
}

/// `../`-style relative path from one repo file to another — the link form
/// `grund fmt --cross-refs` writes (§FS-fmt.6.2).
fn relative_url(from_file: &Path, to_file: &Path, config: &Config) -> String {
    relative_url_under(from_file, to_file, &config.root)
}

/// Same as `relative_url`, but uses an explicit project root for stripping —
/// the workspace-root variant (§FS-workspace.8.5) so a citing file in one
/// project can link to a target file in another project under the same
/// workspace root with a single common-prefix walk.
fn relative_url_under(from_file: &Path, to_file: &Path, root: &Path) -> String {
    let (from_rel, to_rel) = match (from_file.strip_prefix(root), to_file.strip_prefix(root)) {
        (Ok(from_rel), Ok(to_rel)) => (
            std::borrow::Cow::Borrowed(from_rel),
            std::borrow::Cow::Borrowed(to_rel),
        ),
        _ => {
            let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
            let from_file =
                std::fs::canonicalize(from_file).unwrap_or_else(|_| from_file.to_path_buf());
            let to_file =
                std::fs::canonicalize(to_file).unwrap_or_else(|_| to_file.to_path_buf());
            let from_rel = from_file
                .strip_prefix(&root)
                .map(Path::to_path_buf)
                .unwrap_or(from_file);
            let to_rel = to_file
                .strip_prefix(&root)
                .map(Path::to_path_buf)
                .unwrap_or(to_file);
            (
                std::borrow::Cow::Owned(from_rel),
                std::borrow::Cow::Owned(to_rel),
            )
        }
    };
    let from_dir = from_rel.parent().unwrap_or(Path::new(""));
    let from_components = path_components(from_dir);
    let to_components = path_components(&to_rel);
    let mut common = 0;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }
    let mut parts = Vec::new();
    for _ in common..from_components.len() {
        parts.push("..".to_string());
    }
    parts.extend(to_components[common..].iter().cloned());
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

/// The heading text a section anchor is built from — `<number> <title>` taken
/// straight off the heading line, since anchors are derived from heading text, not
/// stored (§DF-md-link-anchor-strategy). The title is reduced to its rendered form
/// (`reduce_heading_text`: `[§FS-<x>.1](path)` → `§FS-<x>.1`, `<ID>` dropped) so
/// the anchor is stable whether or not a citation in this heading has been wrapped
/// by `grund fmt --cross-refs` (§DF-github-anchor-fidelity).
fn section_anchor_text(line: &str, section: &str) -> String {
    let trimmed = line.trim_start();
    // §FS-fmt.6.2: named anchors derive from the complete rendered heading, so
    // its explicit colon reaches the renderer (`goals: Scope`). Numeric paths
    // retain their historical normalized stored text byte for byte.
    if section
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_lowercase)
    {
        return reduce_heading_text(trimmed.trim_start_matches('#').trim_start());
    }
    let heading = trimmed
        .trim_start_matches('#')
        .trim_start()
        .trim_start_matches(section)
        .trim_start_matches('.')
        .trim_start();
    format!(
        "{} {}",
        section.replace('.', ""),
        reduce_heading_text(heading)
    )
    .trim()
    .to_string()
}

/// Re-read a home file to find the heading text of a cited section — the fallback
/// when the section isn't already in the declaration's section map, so a link
/// anchor is always re-derived from the current heading (§FS-fmt.6.3,
/// §DF-md-link-anchor-strategy).
fn section_heading_text(
    path: &Path,
    id: &Id,
    section: &str,
    config: &Config,
) -> Result<Option<String>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let mut in_decl = false;
    let mut py_docstring = PythonDocstringScanState::default();
    for line in text.lines() {
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        let scan_line = scan.text;
        if let Some((found, _)) =
            declaration_id_on_line(&config.grammar, scan_line, scan.in_py_docstring, is_md)
        {
            if in_decl && &found != id {
                break;
            }
            if &found == id {
                in_decl = true;
                continue;
            }
        }
        if !in_decl {
            continue;
        }
        if let Some(caps) = config.grammar.section_re.captures(scan_line)
            && section_path(&caps).is_some_and(|found| found == section)
        {
            return Ok(Some(section_anchor_text(scan_line, section)));
        }
    }
    Ok(None)
}

/// Slugify a heading into a fragment anchor, dispatching on the configured
/// `[fmt.cross_refs] anchor_format` profile (github / gitlab / mkdocs / pandoc) —
/// §FS-fmt.6.7, §DF-md-link-anchor-strategy.
fn anchor_slug(text: &str, profile: &str) -> String {
    match profile {
        "pandoc" => anchor_slug_pandoc(text),
        "mkdocs" => anchor_slug_mkdocs(text),
        "gitlab" => anchor_slug_gitlab(text),
        _ => anchor_slug_github(text),
    }
}

/// Reproduce GitHub's `github-slugger` byte-for-byte: lowercase the text, delete
/// every character that is not a letter, digit, `_`, or `-` (each deletion in
/// place, so the neighbours close up), then turn each remaining space into one
/// `-`. It does **not** collapse runs of `-` and does **not** trim trailing ones —
/// `## A — B` → `#a--b`, `` ## 6. Watch mode (`--watch`) `` → `#6-watch-mode---watch`.
/// Matching that exactly is the whole point of the `github` profile: the emitted
/// `#fragment` navigates only if it is the slug GitHub itself renders
/// (§DF-github-anchor-fidelity, correcting the "collapse consecutive `-`" wording
/// in §DF-md-link-anchor-strategy.2.3).
fn anchor_slug_github(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else if ch == ' ' {
            out.push('-');
        }
        // anything else (`.`, brackets, backticks, em dash, tabs, …) is dropped
    }
    out
}

fn anchor_slug_gitlab(text: &str) -> String {
    // "Similar to GitHub with minor Unicode-handling differences"
    // (§DF-md-link-anchor-strategy.2.3); identical for the ASCII headings grund's own
    // specs use, so it rides the github slugger (§DF-github-anchor-fidelity).
    anchor_slug_github(text)
}

// Python-Markdown's TOC slugger: lowercase, drop everything that isn't a word
// char, whitespace, or `-`, then collapse each run of whitespace-and-`-` to one
// `-` (`re.sub(r'[-\s]+', sep, value)`). The keep-set includes `-`, unlike a naive
// "alnum + `_`" filter — `# FS-1-x: Y` slugs to `#fs-1-x-y`, not `#fs1x-y`.
fn anchor_slug_mkdocs(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() || lower == '_' {
            out.push(lower);
            last_dash = false;
        } else if (lower.is_ascii_whitespace() || lower == '-') && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn anchor_slug_pandoc(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() || lower == '_' || lower == '-' || lower == '.' {
            out.push(lower);
            last_dash = lower == '-';
        } else if lower.is_ascii_whitespace() && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
