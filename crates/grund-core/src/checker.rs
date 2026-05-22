/// AR-checker: how grund validates the scanner's findings
///
/// The checker takes the `Findings` produced by §AR-scanner and produces a
/// `CheckReport`. It implements the rules in §FS-check.
///
/// ## 1. Inputs and outputs
///
/// - Input: `Findings` from the scanner, plus the repo root and config (needed
///   to resolve stub-link paths, to read managed agent-entrypoint init blocks,
///   and to know whether `[reference] require_grounding` is on).
/// - Output: a `CheckReport` containing two ordered lists: `errors` and `warnings`.
///   Order is deterministic — sorted into the fixed report order of §FS-errors.4
///   and §FS-non-goals.9 — for §GOAL-friendliness-first.
///
/// ## 2. Rules
///
/// Each rule is a single pass over part of the findings. Rules are independent —
/// adding a rule does not force re-scanning.
///
/// ### 2.1 Duplicate declarations (§FS-check.3.3)
///
/// For each ID with more than one declaration, emit one error anchored at the
/// lexicographically-first site (sort by `path`, then `line`); list every other
/// site parenthetically in the message. This keeps the report's `path:line:`
/// prefix invariant (§3, §FS-check.2.1) while still naming all sites. A stub and
/// the inline declaration it points at count as one home, not two.
///
/// ### 2.2 Dangling citations (§FS-check.3.1)
///
/// For each citation whose ID has no declaration, emit one error at the citation
/// site.
///
/// ### 2.3 Missing sections (§FS-check.3.2)
///
/// For each citation with a section path, look up the section in the matching
/// declaration's recorded sections. Missing → one error at the citation site.
///
/// ### 2.4 Broken inline-spec stubs (§FS-check.3.4)
///
/// For each declaration whose H1 has the stub shape `# <ID>: [<text>](<path>)`
/// (description after the colon is a single bare markdown link), extract the link
/// target, resolve it against the repo root, verify the path exists, then re-scan
/// that file for an inline declaration of the same ID. Either failure → one error
/// at the stub site. This is the only rule that re-reads a file; everything else
/// comes from `findings`.
///
/// ### 2.5 Unused declarations (§FS-check.4.1)
///
/// For each declared ID never cited, emit one warning. Warnings do not cause a
/// non-zero exit. `E2E` declarations are exempt — a case is exercised by being
/// run, not by being cited (§FS-check.4.1).
///
/// ### 2.6 Invalid agent-entrypoint init block (§FS-check.3.5)
///
/// When `<root>/AGENTS.md` exists, verify its versioned `grund init` block (and the
/// matching block in any non-symlink companion entrypoint that is present): a
/// missing block, an older version, or a newer unsupported version is one error at
/// the entrypoint's line. When only companion entrypoints exist, validate the ones
/// that already contain a managed block and leave project-owned unmanaged files
/// alone.
///
/// ### 2.7 Ungrounded source files — opt-in (§FS-check.3.6, §DF-require-grounding)
///
/// When `[reference] require_grounding = true` (or `grund check --require-grounding`),
/// every scanned non-`.md` file must carry at least one recognised citation that
/// resolves, or itself declare an ID inline; a source file that does neither is
/// one error anchored at line 1. Off by default.
///
/// ## 3. Error format
///
/// Every error and warning follows `<path>:<line>: <message>` so that editors and
/// agents can jump to the source. There is no severity prefix, and there is no
/// aggregate summary footer — the exit code is the machine-readable verdict. This
/// is mandated by §GOAL-friendliness-first and §FS-check.2.1.
///
/// Findings without a single source location (CLI launch errors, malformed
/// configuration that prevents a scan from starting, a per-file read failure
/// mid-walk) are emitted on stderr as `error: <message>` per §FS-check.2.1.1,
/// distinguishable from per-finding lines by the leading `error:`.
///
/// ## 4. Why a separate stage from the scanner
///
/// The scanner produces a complete view of the world; the checker enforces rules
/// on that view. Keeping them separate means:
///
/// - New rules can be added without touching the scanner.
/// - The optional LSP server (§AR-lsp) can run a subset of checks (e.g., only
///   dangling references on the active file's citations) against a cached scan.
/// - Tests can feed synthetic `Findings` directly to the checker without disk I/O.
fn check_findings(findings: &Findings, config: &Config) -> CheckReport {
    check_with_workspace(findings, config, None, &BTreeMap::new())
}

struct WorkspaceCheckTarget<'a> {
    findings: &'a Findings,
    config: &'a Config,
}

fn check_with_workspace(
    findings: &Findings,
    config: &Config,
    current_alias: Option<&str>,
    workspace: &BTreeMap<String, WorkspaceCheckTarget<'_>>,
) -> CheckReport {
    let mut report = CheckReport::default();
    // §FS-check.3.5: managed agent-entrypoint blocks that are out of date (or
    // newer than this binary) are check errors.
    check_agents_block_version(&config.root, &mut report);

    // §FS-check.3.3: an ID with more than one non-stub home is a duplicate.
    for (id, decls) in &findings.declarations {
        let duplicate_homes: Vec<&Declaration> = decls
            .iter()
            .filter(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
            .collect();
        if duplicate_homes.len() > 1 {
            let mut sites: Vec<Site> = duplicate_homes
                .iter()
                .map(|d| Site {
                    path: d.file.clone(),
                    line: d.line,
                })
                .collect();
            sites.sort_by(|a, b| {
                (sort_path_key(&a.path), a.line).cmp(&(sort_path_key(&b.path), b.line))
            });
            let primary = sites[0].clone();
            let others = sites[1..]
                .iter()
                .map(|site| format!("{}:{}", display_path(config, &site.path), site.line))
                .collect::<Vec<_>>();
            let suffix = if others.is_empty() {
                String::new()
            } else {
                format!(" (also declared at {})", others.join(", "))
            };
            report.errors.push(Diagnostic {
                code: "duplicate",
                path: Some(primary.path),
                line: Some(primary.line),
                message: format!("duplicate declaration of {}{suffix}", render_id(config, id)),
                sites,
            });
        }
    }

    // §FS-check.3.7: a kind with `file = "<path>"` set in `[[kinds]]` is a
    // single-file kind — every declaration of that kind must live in that
    // exact file. A declaration anywhere else is a misplaced-declaration error.
    for (id, decls) in &findings.declarations {
        let Some(expected) = single_file_home_for_kind(config, &id.kind) else {
            continue;
        };
        let expected_path = config.root.join(&expected);
        for decl in decls {
            if decl.is_stub {
                continue;
            }
            if !paths_same_location(&decl.file, &expected_path) {
                report.errors.push(Diagnostic {
                    code: "misplaced-declaration",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    message: format!(
                        "{} must be declared in {} (single-file kind)",
                        render_id(config, id),
                        expected
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    for cite in &findings.citations {
        let Some(target) = target_for_citation(cite, findings, config, workspace) else {
            // `target_for_citation` only returns `None` when the
            // namespace is present and unknown — so the namespace is always
            // Some here (§AR-workspace.4).
            let namespace = cite
                .namespace
                .as_deref()
                .expect("resolver only returns None for qualified citations");
            report.errors.push(Diagnostic {
                code: "unknown-project",
                path: Some(cite.file.clone()),
                line: Some(cite.line),
                message: format!("unknown project alias {namespace}"),
                sites: Vec::new(),
            });
            continue;
        };
        // §FS-check.3.1 / §FS-workspace.4: a citation whose ID is declared
        // nowhere in its target namespace is dangling.
        let Some(decls) = target.findings.declarations.get(&cite.id) else {
            let unknown = render_qualified_id(target.config, cite.namespace.as_deref(), &cite.id);
            let message =
                dangling_message(target.config, cite.namespace.as_deref(), target.findings, &cite.id);
            report.errors.push(Diagnostic {
                code: "dangling",
                path: Some(cite.file.clone()),
                line: Some(cite.line),
                message: message.unwrap_or_else(|| format!("unknown reference {unknown}")),
                sites: Vec::new(),
            });
            continue;
        };
        // §FS-check.3.2: the ID resolves but no declaration has a heading at the
        // cited section path.
        if let Some(sec) = &cite.section {
            let any_match = decls.iter().any(|d| d.sections.contains_key(sec));
            if !any_match {
                report.errors.push(Diagnostic {
                    code: "missing-section",
                    path: Some(cite.file.clone()),
                    line: Some(cite.line),
                    message: format!(
                        "missing section {}{}{}",
                        render_qualified_id(target.config, cite.namespace.as_deref(), &cite.id),
                        target.config.section_separator,
                        sec
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    // §FS-check.3.9 / §FS-config.3.3: in strict mode, the Markdown heading level
    // must mirror the dotted section depth so `## 1`, `### 1.1`, ...
    // communicate the same tree that `§ID.1.1` addresses.
    if matches!(config.section_heading_levels.as_str(), "strict" | "warn") {
        let target = if config.section_heading_levels == "strict" {
            &mut report.errors
        } else {
            &mut report.warnings
        };
        for (id, decls) in &findings.declarations {
            for decl in decls {
                for (section_path, section) in &decl.sections {
                    let expected_level = decl.heading_level + section_depth(section_path);
                    if section.heading_level != expected_level {
                        target.push(Diagnostic {
                            code: "section-heading-level",
                            path: Some(decl.file.clone()),
                            line: Some(section.line),
                            message: format!(
                                "section {}{}{} heading level mismatch: expected {} (level {}), found {} (level {})",
                                render_id(config, id),
                                config.section_separator,
                                section_path,
                                heading_marks(expected_level),
                                expected_level,
                                heading_marks(section.heading_level),
                                section.heading_level
                            ),
                            sites: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    // §FS-inline-citation-style.4: inline source-comment citation sites are
    // checked from scanner-provided site metadata; Markdown citations and
    // declaration bodies carry no site and are ignored here.
    check_inline_citation_style(findings, config, &mut report);

    // §FS-check.3.4: a `# <ID>: [text](path)` stub is broken if `path` does not
    // exist, or exists but does not itself declare `<ID>` inline (§AR-checker.2.4).
    for (id, decls) in &findings.declarations {
        for decl in decls {
            if !decl.is_stub {
                continue;
            }
            let Some(target) = &decl.defined_in else {
                continue;
            };
            let resolved = resolve_stub_target(&config.root, &decl.file, target);
            if !resolved.exists() {
                report.errors.push(Diagnostic {
                    code: "broken-stub",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    message: format!("stub link target missing: {}", format_path(target)),
                    sites: Vec::new(),
                });
                continue;
            }
            let inline_ok = if resolved.is_file() && is_scannable(&resolved, config) {
                file_declares_inline_home(&resolved, id, config).unwrap_or(false)
            } else {
                false
            };
            if !inline_ok {
                report.errors.push(Diagnostic {
                    code: "broken-stub",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    message: format!(
                        "stub link target lacks {}: {}",
                        render_id(config, id),
                        format_path(target)
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    // §FS-check.4.1: a declaration nothing cites is a warning, not an error —
    // except E2E cases, which are proof artifacts, not citation targets.
    let mut cited: BTreeSet<&Id> = findings
        .citations
        .iter()
        .filter(|cite| cite.namespace.is_none())
        .map(|c| &c.id)
        .collect();
    if let Some(alias) = current_alias {
        for target in workspace.values() {
            cited.extend(
                target
                    .findings
                    .citations
                    .iter()
                    .filter(|cite| cite.namespace.as_deref() == Some(alias))
                    .map(|cite| &cite.id),
            );
        }
    }
    for (id, decls) in &findings.declarations {
        if id.kind == "E2E" {
            continue;
        }
        if !cited.contains(id)
            && let Some(decl) = decls
                .iter()
                .find(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
                .or_else(|| decls.first())
        {
            report.warnings.push(Diagnostic {
                code: "unused",
                path: Some(decl.file.clone()),
                line: Some(decl.line),
                message: format!("declared but never cited: {}", render_id(config, id)),
                sites: Vec::new(),
            });
        }
    }

    // §FS-check.3.6 / §DF-require-grounding: under `[reference] require_grounding`,
    // every scanned source (non-Markdown) file must carry at least one citation to
    // a declared ID — or itself declare one inline (a spec home is grounded in the
    // spec it *is*). Pure function of (tree, config): no git, no AST.
    if config.require_grounding {
        // Collect the files that ground themselves in two linear passes — one over
        // citations, one over declarations — so the per-file test below is a set
        // lookup, not a re-scan of every citation and declaration for each file
        // (§GOAL-fast-feedback: speed is the ordering principle).
        let mut grounded_files: BTreeSet<&Path> = findings
            .citations
            .iter()
            .filter(|cite| citation_resolves(cite, findings, config, workspace))
            .map(|cite| cite.file.as_path())
            .collect();
        grounded_files.extend(
            findings
                .declarations
                .values()
                .flatten()
                .filter(|decl| !decl.is_stub && decl.e2e_case.is_none())
                .map(|decl| decl.file.as_path()),
        );
        for file in &findings.scanned_files {
            if file.extension().and_then(|ext| ext.to_str()) == Some("md") {
                continue;
            }
            if !grounded_files.contains(file.as_path()) {
                report.errors.push(Diagnostic {
                    code: "ungrounded",
                    path: Some(file.clone()),
                    line: Some(1),
                    message: format!(
                        "ungrounded source file: no {} citation to a declared ID",
                        config.marker
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    sort_diagnostics(&mut report.errors);
    sort_diagnostics(&mut report.warnings);
    report
}

fn dangling_message(
    config: &Config,
    namespace: Option<&str>,
    findings: &Findings,
    missing: &Id,
) -> Option<String> {
    let unknown = render_qualified_id(config, namespace, missing);
    let suggestion = nearest_declared_id(config, namespace, findings, missing)?;
    Some(format!("unknown reference {unknown}; did you mean {suggestion}?"))
}

fn nearest_declared_id(
    config: &Config,
    namespace: Option<&str>,
    findings: &Findings,
    missing: &Id,
) -> Option<String> {
    let missing_text = render_id(config, missing);
    let mut best: Option<(usize, String)> = None;
    for candidate in findings.declarations.keys() {
        if candidate.kind != missing.kind {
            continue;
        }
        let candidate_text = render_id(config, candidate);
        let distance = edit_distance(&missing_text, &candidate_text);
        if !close_enough_for_hint(
            distance,
            missing_text.chars().count(),
            candidate_text.chars().count(),
        ) {
            continue;
        }
        let rendered = render_qualified_id(config, namespace, candidate);
        match &best {
            Some((best_distance, best_rendered))
                if distance > *best_distance
                    || (distance == *best_distance && rendered >= *best_rendered) => {}
            _ => best = Some((distance, rendered)),
        }
    }
    best.map(|(_, rendered)| rendered)
}

fn close_enough_for_hint(distance: usize, left_len: usize, right_len: usize) -> bool {
    distance > 0 && distance <= 3 && distance * 3 <= left_len.max(right_len)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];

    for (i, left_char) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[j] + usize::from(left_char != *right_char);
            let insertion = current[j] + 1;
            let deletion = previous[j + 1] + 1;
            current[j + 1] = substitution.min(insertion).min(deletion);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
}

fn section_depth(section_path: &str) -> usize {
    section_path.split('.').count()
}

fn heading_marks(level: usize) -> String {
    "#".repeat(level)
}

fn check_inline_citation_style(findings: &Findings, config: &Config, report: &mut CheckReport) {
    let mut seen = BTreeSet::new();
    for cite in &findings.citations {
        let Some(site) = &cite.inline_site else {
            continue;
        };
        if !seen.insert((cite.file.clone(), site.clone())) {
            continue;
        }
        match config.inline_style.as_str() {
            "citation-only" => {
                if site.has_note {
                    report.errors.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        message: "inline citation must carry no prose".to_string(),
                        sites: Vec::new(),
                    });
                }
            }
            _ => {
                let lines = site.last_line - site.first_line + 1;
                if lines > config.inline_note_max_lines {
                    report.errors.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        message: format!(
                            "inline note exceeds {}-line maximum",
                            config.inline_note_max_lines
                        ),
                        sites: Vec::new(),
                    });
                }
                if site.max_columns > config.inline_note_max_columns {
                    report.errors.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        message: format!(
                            "inline note exceeds {}-column maximum",
                            config.inline_note_max_columns
                        ),
                        sites: Vec::new(),
                    });
                }
                if config.warn_on_suggested
                    && lines > config.inline_note_suggested_lines
                    && lines <= config.inline_note_max_lines
                {
                    report.warnings.push(Diagnostic {
                        code: "inline-citation-style",
                        path: Some(cite.file.clone()),
                        line: Some(site.first_line),
                        message: format!(
                            "inline note exceeds {}-line preferred limit",
                            config.inline_note_suggested_lines
                        ),
                        sites: Vec::new(),
                    });
                }
            }
        }
    }
}

fn target_for_citation<'a>(
    cite: &Citation,
    local: &'a Findings,
    local_config: &'a Config,
    workspace: &'a BTreeMap<String, WorkspaceCheckTarget<'a>>,
) -> Option<WorkspaceCheckTarget<'a>> {
    match cite.namespace.as_deref() {
        Some(namespace) => workspace.get(namespace).map(|target| WorkspaceCheckTarget {
            findings: target.findings,
            config: target.config,
        }),
        None => Some(WorkspaceCheckTarget {
            findings: local,
            config: local_config,
        }),
    }
}

fn citation_resolves(
    cite: &Citation,
    local: &Findings,
    local_config: &Config,
    workspace: &BTreeMap<String, WorkspaceCheckTarget<'_>>,
) -> bool {
    target_for_citation(cite, local, local_config, workspace)
        .map(|target| target.findings.declarations.contains_key(&cite.id))
        .unwrap_or(false)
}

/// Put diagnostics in the one fixed order `grund` ever prints them in — by path, then
/// line, then message text — so two runs over the same tree agree byte-for-byte
/// (§FS-errors.4) and ordering is not a knob (§FS-non-goals.9).
fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(diagnostic_cmp);
}

fn diagnostic_cmp(a: &Diagnostic, b: &Diagnostic) -> std::cmp::Ordering {
    (
        a.path.as_ref().map(|p| sort_path_key(p)),
        a.line.unwrap_or(0),
        a.message.as_str(),
    )
        .cmp(&(
            b.path.as_ref().map(|p| sort_path_key(p)),
            b.line.unwrap_or(0),
            b.message.as_str(),
        ))
}

/// Validate the managed agent-entrypoint blocks (§FS-check.3.5): the begin/end
/// marker pair must be present and intact, and the `vN` version must match this
/// binary — an older `vN` is "run `grund init`" (§FS-init.2.3), a newer one is
/// fatal. `AGENTS.md` is canonical; known companion entrypoints are checked when
/// present and not symlinked to `AGENTS.md`.
fn check_agents_block_version(root: &Path, report: &mut CheckReport) {
    let canonical = root.join("AGENTS.md");
    let canonical_exists = canonical.exists();
    if canonical_exists {
        check_agent_block_path(&canonical, report, true);
    }
    match companion_agent_entrypoints(root) {
        Ok(companions) => {
            for companion in companions {
                check_agent_block_path(&companion, report, canonical_exists);
            }
        }
        Err((path, message)) => {
            report.errors.push(Diagnostic {
                code: "io",
                path: Some(path),
                line: Some(1),
                message,
                sites: Vec::new(),
            });
        }
    }
}

fn check_agent_block_path(path: &Path, report: &mut CheckReport, require_block: bool) {
    if !path.exists() {
        return;
    }
    let Ok(text) = fs::read_to_string(path) else {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agent entrypoint");
        report.errors.push(Diagnostic {
            code: "io",
            path: Some(path.to_path_buf()),
            line: Some(1),
            message: format!("cannot read {file_name}"),
            sites: Vec::new(),
        });
        return;
    };
    if let Some(block) = find_agents_block(&text) {
        let line = line_for_byte_index(&text, block.start);
        if block.version < AGENTS_BLOCK_VERSION {
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line),
                message: format!(
                    "outdated grund init block v{} (run `grund init` to update to v{})",
                    block.version, AGENTS_BLOCK_VERSION
                ),
                sites: Vec::new(),
            });
        } else if block.version > AGENTS_BLOCK_VERSION {
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line),
                message: format!(
                    "unsupported grund init block v{} (this grund supports v{})",
                    block.version, AGENTS_BLOCK_VERSION
                ),
                sites: Vec::new(),
            });
        }
        return;
    }
    if !require_block {
        return;
    }
    report.errors.push(Diagnostic {
        code: "agents-init",
        path: Some(path.to_path_buf()),
        line: Some(1),
        message: format!("missing grund init block v{}", AGENTS_BLOCK_VERSION),
        sites: Vec::new(),
    });
}

fn line_for_byte_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

/// Whether this stub heading is the one-line pointer to an inline declaration in
/// code (`# <ID>: [text](src/foo.rs)` whose target also declares `<ID>`) — such a
/// stub does not count as a second home, so it is not a duplicate (§AR-scanner.4,
/// §FS-show.2.3).
fn is_stub_for_inline_decl(root: &Path, decl: &Declaration, decls: &[Declaration]) -> bool {
    if !decl.is_stub {
        return false;
    }
    let Some(target) = &decl.defined_in else {
        return false;
    };
    let resolved = resolve_stub_target(root, &decl.file, target);
    decls
        .iter()
        .any(|other| paths_same_location(&other.file, &resolved) && other.file != decl.file)
}

/// The `[[kinds]].file` setting for `kind`, if any — the single document every
/// declaration of that kind must live in (§FS-config.3.4). Returns `None` for
/// multi-file kinds (those configured with `folder` instead).
fn single_file_home_for_kind(config: &Config, kind: &str) -> Option<String> {
    config
        .kinds
        .iter()
        .find(|k| k.prefix == kind)
        .and_then(|k| k.file.clone())
}

fn paths_same_location(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| normalize_path_lexically(left));
    let right = fs::canonicalize(right).unwrap_or_else(|_| normalize_path_lexically(right));
    left == right
}

/// Whether `path` contains a real (non-stub) inline declaration of `id` —
/// the check that a stub's link target actually carries the inline home it claims
/// (§FS-check.3.4, §AR-checker.2.4, §AR-scanner.4).
fn file_declares_inline_home(path: &Path, id: &Id, config: &Config) -> Result<bool> {
    let text = fs::read_to_string(path)?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let mut in_py_docstring = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if config.docstring_python
            && is_py
            && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''"))
        {
            in_py_docstring = !in_py_docstring;
            continue;
        }
        let scan_line = if in_py_docstring { trimmed } else { line };
        if let Some(caps) =
            declaration_captures(&config.grammar, scan_line, in_py_docstring, is_md)
            && let Some(found) = parse_id(&caps)
            && &found == id
        {
            let tail = &scan_line[caps.get(0).unwrap().end()..];
            if STUB_LINK_HEADING.is_match(tail) {
                continue;
            }
            return Ok(true);
        }
    }
    Ok(false)
}
