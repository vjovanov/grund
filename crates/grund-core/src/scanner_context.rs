/// Narrow the shared coordinate catalog to each declaration's body and retain
/// every rejected heading as one check site (§FS-show.2.1.2, §FS-check.3.23).
///
/// The line scan's current declaration runs farther than the body in Markdown,
/// source comments, docstrings, and stubs. Applying the already-computed span
/// here makes every map reader agree without a surface-local guard. Rejected
/// duplicate claimants become outside-heading sites too, never stale
/// `duplicate-section` collisions.
fn retain_in_body_sections(findings: &mut Findings) {
    let mut rejected = Vec::new();
    for decl in findings.declarations.values_mut().flatten() {
        let body = decl.body_start..=decl.body_end;
        decl.sections.retain(|path, info| {
            let retained = body.contains(&info.line);
            if !retained {
                rejected.push(SectionHeadingOutsideDeclaration {
                    file: decl.file.clone(),
                    line: info.line,
                    path: path.clone(),
                });
            }
            retained
        });
        decl.duplicate_sections.retain(|(path, info)| {
            let retained = body.contains(&info.line);
            if !retained {
                rejected.push(SectionHeadingOutsideDeclaration {
                    file: decl.file.clone(),
                    line: info.line,
                    path: path.clone(),
                });
            }
            retained
        });
    }
    rejected.sort_by(|left, right| {
        (sort_path_key(&left.file), left.line).cmp(&(sort_path_key(&right.file), right.line))
    });
    rejected.dedup_by(|left, right| left.file == right.file && left.line == right.line);
    findings
        .section_headings_outside_declarations
        .extend(rejected);
}

/// Retain only Markdown headings owned by a declaration body, choose the
/// nearest nested owner, and assign deterministic unused section paths
/// (§AR-scanner.2.2, §AR-scanner.2.4, §FS-check.4.14).
fn assign_unmarked_heading_owners(
    findings: &mut Findings,
    mut candidates: Vec<UnmarkedHeadingCandidate>,
) {
    #[derive(Clone)]
    struct KnownPath {
        line: usize,
        path: String,
    }

    let bodies = findings
        .declarations
        .values()
        .flatten()
        .map(|decl| {
            (
                decl.body_start,
                decl.body_end,
                decl.heading_level,
                decl.id.clone(),
                decl.sections
                    .iter()
                    .map(|(path, info)| KnownPath {
                        line: info.line,
                        path: path.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.line);
    let mut suggested_by_owner: BTreeMap<Id, Vec<KnownPath>> = BTreeMap::new();

    for candidate in candidates {
        let Some((_, _, declaration_level, owner, existing)) = bodies
            .iter()
            .filter(|(start, end, level, _, _)| {
                *start <= candidate.line
                    && candidate.line <= *end
                    && candidate.heading_level > *level
            })
            .max_by_key(|(start, _, _, _, _)| *start)
        else {
            continue;
        };
        let target_depth = candidate.heading_level - declaration_level;
        let prior_suggestions = suggested_by_owner.entry(owner.clone()).or_default();
        let mut paths = existing.clone();
        paths.extend(prior_suggestions.iter().cloned());
        let suggested_path = suggested_section_path(&paths, candidate.line, target_depth);
        prior_suggestions.push(KnownPath {
            line: candidate.line,
            path: suggested_path.clone(),
        });
        findings.unmarked_headings.push(UnmarkedHeading {
            file: candidate.file,
            line: candidate.line,
            column: candidate.column,
            heading: candidate.heading,
            heading_level: candidate.heading_level,
            title: candidate.title,
            owner: owner.clone(),
            suggested_path,
        });
    }

    fn suggested_section_path(paths: &[KnownPath], line: usize, target_depth: usize) -> String {
        let parent = paths
            .iter()
            .filter(|known| known.line < line && path_depth(&known.path) < target_depth)
            .max_by_key(|known| known.line)
            .map(|known| known.path.clone());

        let mut parts = parent
            .as_deref()
            .map(|path| path.split('.').map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default();
        if parts.is_empty() {
            parts.push(next_numeric_child(paths, &[]).to_string());
        }
        while parts.len() + 1 < target_depth {
            parts.push("1".to_string());
        }
        if parts.len() < target_depth {
            let next = next_numeric_child(paths, &parts);
            parts.push(next.to_string());
        }
        parts.join(".")
    }

    fn path_depth(path: &str) -> usize {
        path.split('.').count()
    }

    fn next_numeric_child(paths: &[KnownPath], parent: &[String]) -> u32 {
        paths
            .iter()
            .filter_map(|known| {
                let parts = known.path.split('.').collect::<Vec<_>>();
                if parts.len() != parent.len() + 1
                    || !parts[..parent.len()]
                        .iter()
                        .zip(parent)
                        .all(|(left, right)| *left == right)
                {
                    return None;
                }
                parts.last()?.parse::<u32>().ok()
            })
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

fn section_path_is_numeric(path: &str) -> bool {
    path.split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
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
/// source file (§AR-scanner.2.4) — the shared block walk of `comment_block.rs`
/// without the declares-an-ID filtering, so a declaration's body can be bounded
/// by the block that hosts it.
fn comment_block_ranges(text: &str, is_py: bool, config: &Config) -> Vec<(usize, usize)> {
    let lines = text.lines().collect::<Vec<_>>();
    comment_blocks(&lines, is_py, config)
        .into_iter()
        .map(|(start, end, _)| (start + 1, end + 1))
        .collect()
}

/// Classify each citation's citing side by the three-step fallback of
/// §AR-scanner.2.4: the enclosing declaration's kind (nearest preceding
/// declaration whose body contains the site), else the file's unique kind home,
/// else the homeless kind — `code`, or whatever the project named it
/// (§FS-config.3.9.2).
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
    // §FS-config.3.9.2: step 3 of the fallback is the homeless kind, whose name
    // is `code` only where the project did not name it something truer.
    let homeless = config.homeless_kind();
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
                    .unwrap_or_else(|| homeless.to_string());
            }
        }
    }
    for candidate in &mut findings.legacy_citation_candidates {
        let enclosing = bodies
            .iter()
            .filter(|(start, end, _)| *start <= candidate.line && candidate.line <= *end)
            .max_by_key(|(start, _, _)| *start);
        match enclosing {
            Some((_, _, id)) => {
                candidate.source_kind = id.kind.clone();
                candidate.enclosing_declaration = Some(id.clone());
            }
            None => {
                candidate.source_kind = file_home
                    .clone()
                    .unwrap_or_else(|| homeless.to_string());
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
            if matched.is_some_and(|prev| prev != kind.kind.as_str()) {
                return None;
            }
            matched = Some(kind.kind.as_str());
        }
    }
    matched.map(str::to_string)
}

/// Locate the source-comment blocks that can host inline citation sites
/// (§FS-inline-citation-style.1). Markdown prose is deliberately out of scope,
/// and so is a *doc comment*: documentation is not a note about a clause, so
/// nothing the inline citation style says applies to one
/// (§FS-inline-citation-style.1.1, §AR-scanner.4).
fn inline_citation_sites(
    path: &Path,
    text: &str,
    is_md: bool,
    is_py: bool,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> (
    BTreeMap<usize, InlineCitationSite>,
    BTreeMap<usize, std::sync::Arc<[String]>>,
) {
    let mut sites = BTreeMap::new();
    let mut site_lines = BTreeMap::new();
    if is_md {
        return (sites, site_lines);
    }
    let lines = text.lines().collect::<Vec<_>>();
    // §FS-inline-citation-style.1.1: which blocks this file's language calls
    // documentation. Read once from the extension, then applied to each block
    // below with one comparison (§AR-scanner.4).
    let doc_rule = doc_comment_rule(path);
    // Only a position language asks where the file's leading comment is, so the
    // scan for it is taken only there and the marker recognizers ignore the flag
    // it feeds (§GOAL-fast-feedback).
    let leading_limit = match doc_rule {
        DocCommentRule::Position(_) => first_content_line(&lines),
        _ => 0,
    };
    for (start, end, kind) in comment_blocks(&lines, is_py, config) {
        let block = &lines[start..=end];
        // §FS-inline-citation-style.1.1: a doc comment hosts no site — the same
        // skip a block that *declares* an ID already earned, and for the same
        // reason: its shape is not this spec's to govern.
        let is_doc_comment = block_is_doc_comment(
            doc_rule,
            &kind,
            block,
            lines.get(end + 1).copied(),
            start <= leading_limit,
        );
        if !is_doc_comment
            && !block_declares_id(block, matches!(kind, CommentBlockKind::PythonDocstring), config)
        {
            // §FS-inline-citation-style.3.3: both verdicts are taken here, while
            // the block's lines are in hand, so the checker never re-reads one
            // (§AR-scanner.3).
            let (has_note, layout_violations) =
                inline_note_verdicts(block, start + 1, config, workspace_targets);
            let site = InlineCitationSite {
                first_line: start + 1,
                last_line: end + 1,
                // §FS-inline-citation-style.2.3: a column is one character, not one
                // byte — `é` and `§` cost one each (§DF-note-columns-are-characters).
                max_columns: block.iter().map(|line| line.chars().count()).max().unwrap_or(0),
                has_note,
                layout_violations,
            };
            // Only a marker-prefixed rejected candidate can need post-catalog
            // reconciliation. Ordinary comment blocks retain no source copy.
            let block_lines = (!config.marker.is_empty()
                && block.iter().any(|line| line.contains(&config.marker)))
            .then(|| {
                std::sync::Arc::<[String]>::from(
                    block
                        .iter()
                        .map(|line| (*line).to_string())
                        .collect::<Vec<_>>(),
                )
            });
            for line in (start + 1)..=(end + 1) {
                sites.insert(line, site.clone());
                if let Some(block_lines) = &block_lines {
                    site_lines.insert(line, std::sync::Arc::clone(block_lines));
                }
            }
        }
    }
    (sites, site_lines)
}
