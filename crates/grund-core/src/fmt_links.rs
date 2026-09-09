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
#[cfg(test)]
fn wrap_markdown_links(
    line: &str,
    path: &Path,
    config: &Config,
    findings: &Findings,
    workspace: Option<&WorkspaceContext>,
    only_ids: Option<&BTreeSet<Id>>,
) -> String {
    let targets = ShorthandTargets::new(config, Some(findings), workspace);
    wrap_markdown_links_with_targets(
        line, path, config, findings, workspace, only_ids, &targets,
    )
}

/// The production §FS-fmt.6 wrapper pass, sharing §FS-fmt.2.4's already-built
/// target indexes so accepted shorthand adds no per-citation catalog scan.
fn wrap_markdown_links_with_targets(
    line: &str,
    path: &Path,
    config: &Config,
    findings: &Findings,
    workspace: Option<&WorkspaceContext>,
    only_ids: Option<&BTreeSet<Id>>,
    shorthand_targets: &ShorthandTargets<'_>,
) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    for citation in
        markdown_link_citations(line, config, findings, workspace, shorthand_targets)
    {
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
        // §FS-values.8: wrapping the citation would destroy the sole accepted
        // authored binding form, so a recognized value binding keeps its bytes.
        if markdown_citation_is_value_binding(line, &citation, config, workspace) {
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

fn markdown_citation_is_value_binding(
    line: &str,
    citation: &MarkdownLineCitation,
    config: &Config,
    workspace: Option<&WorkspaceContext>,
) -> bool {
    let target_config = citation
        .namespace
        .as_deref()
        .and_then(|alias| workspace.and_then(|workspace| workspace.project_by_alias(alias)))
        .map(|project| &project.config)
        .unwrap_or(config);
    if !kind_uses_values(target_config, &citation.id.kind) {
        return false;
    }
    let Some(section) = citation.section.as_deref() else {
        return false;
    };
    if section.starts_with('0') || !section.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Some(before_marker) = line[..citation.marker_start].strip_suffix(" (") else {
        return false;
    };
    let Some(close_tick) = before_marker.len().checked_sub(1) else {
        return false;
    };
    if before_marker.as_bytes().get(close_tick) != Some(&b'`')
        || !line[citation.token_end..].starts_with(')')
    {
        return false;
    }
    let Some(open_tick) = before_marker[..close_tick].rfind('`') else {
        return false;
    };
    component_text_is_valid(&before_marker[open_tick + 1..close_tick])
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
    findings: &Findings,
    workspace: Option<&WorkspaceContext>,
    shorthand_targets: &ShorthandTargets<'_>,
) -> Vec<MarkdownLineCitation> {
    let mut citations = Vec::new();
    if let Some(workspace) = workspace {
        collect_workspace_markdown_link_citations(
            line,
            config,
            workspace,
            shorthand_targets,
            &mut citations,
        );
    }
    for caps in config.grammar.citation_re.captures_iter(line) {
        let Some(full) = caps.get(0) else { continue };
        if config.grammar.has_reserved_named_tail(line, full.end()) {
            continue;
        }
        let marker_start = full.start().saturating_sub(config.marker.len());
        if !line[..full.start()].ends_with(&config.marker) {
            continue;
        }
        if is_inside_inline_code(line, marker_start) {
            continue;
        }
        let Some(id) = parse_id(&caps, &config.grammar) else { continue };
        citations.push(MarkdownLineCitation {
            marker_start,
            token_end: full.end(),
            namespace: caps.name("namespace").map(|m| m.as_str().to_string()),
            id,
            section: caps.name("sec").map(|m| m.as_str().to_string()),
        });
    }
    collect_local_accepted_shorthand_links(
        line,
        config,
        shorthand_targets.local.as_ref(),
        &mut citations,
    );
    collect_local_legacy_markdown_citations(line, config, findings, &mut citations);
    citations.sort_by(|a, b| {
        (a.marker_start, std::cmp::Reverse(a.token_end)).cmp(&(
            b.marker_start,
            std::cmp::Reverse(b.token_end),
        ))
    });
    citations.dedup_by(|a, b| a.marker_start == b.marker_start && a.token_end == b.token_end);
    citations
}

fn collect_workspace_markdown_link_citations(
    line: &str,
    config: &Config,
    workspace: &WorkspaceContext,
    shorthand_targets: &ShorthandTargets<'_>,
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
        let target_index = shorthand_targets.by_alias.get(alias);
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = line.get(id_start..) else {
            continue;
        };
        let parsed = parse_longest_id_prefix(id_rest, &target_project.config.grammar)
            .filter(|parsed| {
                !target_project
                    .config
                    .grammar
                    .has_reserved_named_tail(id_rest, parsed.len)
            })
            .map(|parsed| (parsed.id, parsed.section, parsed.len))
            .or_else(|| {
                let catalog = legacy_catalog_ids(&target_project.findings.declarations);
                match_legacy_tail(id_rest, &target_project.config, &catalog)
            })
            .or_else(|| {
                accepted_shorthand_link(
                    id_rest,
                    config,
                    &target_project.config,
                    target_index.map(|target| &target.index),
                )
            });
        let Some((id, section, len)) = parsed else {
            continue;
        };
        out.push(MarkdownLineCitation {
            marker_start,
            token_end: id_start + len,
            namespace: Some(alias.to_string()),
            id,
            section,
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
    if marker.is_empty() {
        return line.to_string();
    }
    let mut output = String::new();
    let mut cursor = 0usize;
    let wrapper_start = format!("[{marker}");
    for (bracket_pos, _) in line.match_indices(&wrapper_start) {
        let marker_start = bracket_pos + 1;
        let label_start = marker_start + marker.len();
        let Some(label_close_rel) = line[label_start..].find("](") else {
            continue;
        };
        let cite_end = label_start + label_close_rel;
        let token = &line[label_start..cite_end];
        // §FS-show.3.2: require a citation-shaped label the formatter can emit,
        // including persisted and qualified legacy spellings but excluding an
        // ordinary link whose label merely starts with the marker.
        if token.is_empty()
            || token
                .chars()
                .any(|ch| ch.is_whitespace() || matches!(ch, '[' | ']' | '(' | ')' | '`'))
            || !formatter_wrapper_label_is_citation(token, config)
        {
            continue;
        }
        // A citation shown inside `` `…` `` is an illustration, not a citation —
        // leave it exactly as written, the same call `grund fmt --cross-refs` makes.
        if is_inside_inline_code(line, bracket_pos) {
            continue;
        }
        let rest = &line[cite_end + 2..];
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
