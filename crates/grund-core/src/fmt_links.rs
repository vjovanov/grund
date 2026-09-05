/// Wrap each `§<ID>[.<section>]` citation on this Markdown line as `[§<ID>…](url)`
/// — the `--cross-refs` rewrite (§FS-fmt.6.2): re-derive an existing wrapper's URL,
/// skip citations in inline code (§FS-fmt.6.4), and emit nothing when the target
/// does not resolve (§FS-fmt.6.3).
///
/// `workspace` is `None` for single-project runs (and for member-local
/// runs in a workspace — §FS-workspace.8.5). When `Some`, a qualified
/// `§<alias>/<ID>` resolves against the named project's findings, with
/// the relative path crossing the workspace and the anchor computed
/// against the target project's config (§FS-workspace.8.5).
fn wrap_markdown_links(
    line: &str,
    path: &Path,
    config: &Config,
    findings: &Findings,
    workspace: Option<&WorkspaceContext>,
    only_ids: Option<&BTreeSet<Id>>,
) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    for citation in markdown_link_citations(line, config, workspace) {
        if citation.marker_start < cursor {
            continue;
        }
        // §FS-fmt.6.1: the always-linkify carve-out reaches this file for its
        // index entries (§FS-check.3.18) and writes nothing else — a qualified
        // citation is never an entry, and an ID the index does not owe is prose.
        if let Some(only_ids) = only_ids
            && (citation.namespace.is_some() || !only_ids.contains(&citation.id))
        {
            continue;
        }
        // §FS-workspace.8.5: a qualified `§<alias>/<ID>` resolves against the
        // named project in workspace mode; a member-local run has no workspace
        // context and leaves it untouched — no wrap created, none stripped.
        let target = match citation.namespace.as_deref() {
            Some(namespace) => {
                let Some(workspace) = workspace else { continue };
                let Some(target_project) = workspace.project_by_alias(namespace) else {
                    continue;
                };
                markdown_link_target_with_root(
                    path,
                    &citation.id,
                    citation.section.as_deref(),
                    &target_project.config,
                    &target_project.findings,
                    Some(&workspace.render_root),
                )
            }
            None => markdown_link_target(
                path,
                &citation.id,
                citation.section.as_deref(),
                config,
                findings,
            ),
        };
        let Some(target) = target else {
            continue;
        };
        let marked_end = citation.token_end;
        let marker_start = citation.marker_start;
        let already_wrapped = marker_start > 0 && line.as_bytes()[marker_start - 1] == b'[';
        if already_wrapped && line[marked_end..].starts_with("](") {
            let url_start = marked_end + 2;
            if let Some(close_rel) = line[url_start..].find(')') {
                let close = url_start + close_rel;
                output.push_str(&line[cursor..url_start]);
                output.push_str(&target);
                cursor = close;
                continue;
            }
        }
        output.push_str(&line[cursor..marker_start]);
        let citation = &line[marker_start..marked_end];
        output.push('[');
        output.push_str(citation);
        output.push_str("](");
        output.push_str(&target);
        output.push(')');
        cursor = marked_end;
    }
    output.push_str(&line[cursor..]);
    output
}

struct MarkdownLineCitation {
    marker_start: usize,
    token_end: usize,
    namespace: Option<String>,
    id: Id,
    section: Option<String>,
}

fn markdown_link_citations(
    line: &str,
    config: &Config,
    workspace: Option<&WorkspaceContext>,
) -> Vec<MarkdownLineCitation> {
    let mut citations = Vec::new();
    if let Some(workspace) = workspace {
        collect_workspace_markdown_link_citations(line, config, workspace, &mut citations);
    }
    for caps in config.grammar.citation_re.captures_iter(line) {
        let Some(full) = caps.get(0) else { continue };
        let marker_start = full.start().saturating_sub(config.marker.len());
        if !line[..full.start()].ends_with(&config.marker) {
            continue;
        }
        if is_inside_inline_code(line, marker_start) {
            continue;
        }
        let Some(id) = parse_id(&caps) else { continue };
        citations.push(MarkdownLineCitation {
            marker_start,
            token_end: full.end(),
            namespace: caps.name("namespace").map(|m| m.as_str().to_string()),
            id,
            section: caps.name("sec").map(|m| m.as_str().to_string()),
        });
    }
    citations.sort_by(|a, b| {
        (a.marker_start, std::cmp::Reverse(a.token_end)).cmp(&(
            b.marker_start,
            std::cmp::Reverse(b.token_end),
        ))
    });
    citations
}

fn collect_workspace_markdown_link_citations(
    line: &str,
    config: &Config,
    workspace: &WorkspaceContext,
    out: &mut Vec<MarkdownLineCitation>,
) {
    if config.marker.is_empty() {
        return;
    }
    for (marker_start, _) in line.match_indices(&config.marker) {
        if is_inside_inline_code(line, marker_start) {
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
        let Some(target_project) = workspace.project_by_alias(alias) else {
            continue;
        };
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = line.get(id_start..) else {
            continue;
        };
        let Some(parsed) = parse_longest_id_prefix(id_rest, &target_project.config.grammar) else {
            continue;
        };
        out.push(MarkdownLineCitation {
            marker_start,
            token_end: id_start + parsed.len,
            namespace: Some(alias.to_string()),
            id: parsed.id,
            section: parsed.section,
        });
    }
}

/// Flatten `grund fmt --cross-refs` link wrappers before an ID query
/// prints it in `text` / `json` (§FS-show.3.2, §DF-show-cross-ref-flattening):
/// `[§[alias/]<ID>.<section>](path#anchor)` → `§[alias/]<ID>.<section>`. The inverse of
/// `wrap_markdown_links` (§FS-fmt.6.2) — the wrap shape is a `[` immediately
/// before a marker-prefixed citation token and `](…)` immediately after it,
/// exactly what `grund fmt --cross-refs` emits and re-derives (§FS-fmt.6.3); that
/// is the only thing flattened. Ordinary Markdown links, an unwrapped citation,
/// a citation inside an inline-code span (illustrative, like `fmt` itself —
/// §FS-fmt.6.4), and `--format md` output (kept verbatim by the caller) are all
/// left untouched. Purely textual: the citation is never resolved, so a dangling
/// one is flattened just the same and `grund check` still reports it.
fn flatten_cross_ref_links(body: &str, config: &Config) -> String {
    if !body.contains("](") {
        return body.to_string();
    }
    let mut out = String::with_capacity(body.len());
    for line in body.split_inclusive('\n') {
        out.push_str(&flatten_cross_ref_links_line(line, config));
    }
    out
}

fn flatten_cross_ref_links_line(line: &str, config: &Config) -> String {
    let marker = config.marker.as_str();
    let mut output = String::new();
    let mut cursor = 0usize;
    for caps in config.grammar.citation_re.captures_iter(line) {
        let Some(full) = caps.get(0) else { continue };
        let (cite_start, cite_end) = (full.start(), full.end());
        if !line[..cite_start].ends_with(marker) {
            continue;
        }
        let Some(marker_start) = cite_start.checked_sub(marker.len()) else {
            continue;
        };
        // `[` immediately before the marker?
        let Some(bracket_pos) = marker_start.checked_sub(1) else {
            continue;
        };
        if line.as_bytes()[bracket_pos] != b'[' {
            continue;
        }
        // A citation shown inside `` `…` `` is an illustration, not a citation —
        // leave it exactly as written, the same call `grund fmt --cross-refs` makes.
        if is_inside_inline_code(line, bracket_pos) {
            continue;
        }
        // `](…)` immediately after the citation?
        let Some(rest) = line[cite_end..].strip_prefix("](") else {
            continue;
        };
        let Some(close_rel) = rest.find(')') else {
            continue;
        };
        let close = cite_end + 2 + close_rel; // index of the `)`
        if bracket_pos < cursor {
            continue;
        }
        output.push_str(&line[cursor..bracket_pos]);
        output.push_str(&line[marker_start..cite_end]); // §[alias/]<ID>[.<section>]
        cursor = close + 1;
    }
    output.push_str(&line[cursor..]);
    output
}

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
        if let Some(caps) =
            declaration_captures(&config.grammar, scan_line, scan.in_py_docstring, is_md)
        {
            let found = parse_id(&caps);
            if in_decl && found.as_ref() != Some(id) {
                break;
            }
            if found.as_ref() == Some(id) {
                in_decl = true;
                continue;
            }
        }
        if !in_decl {
            continue;
        }
        if let Some(caps) = config.grammar.section_re.captures(scan_line)
            && caps.name("sec").is_some_and(|sec| sec.as_str() == section)
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
