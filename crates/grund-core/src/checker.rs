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
/// ### 2.2 Misplaced declarations (§FS-check.3.7)
///
/// For each declaration, validate placement from the scanner-recorded `file` and
/// `id.kind`. A single-file kind (`[[kinds]].file`) must live in that exact file.
/// Separately, when the declaration's file is contained by exactly one configured
/// kind home, the declaration kind must match that home kind. The checker builds
/// one kind-home index per check run: `file` homes are exact matches and `folder`
/// homes are path-prefix matches under the config root; if zero or multiple homes
/// match, there is no unique expected home kind and the checker emits no
/// home-kind diagnostic.
///
/// This rule uses only `Declaration` records; it does not rescan files. Stubs are
/// checked by their stub file path for home-kind placement, while the existing
/// broken-stub rule still verifies that the linked source file contains the inline
/// declaration it claims.
///
/// ### 2.3 Dangling citations (§FS-check.3.1)
///
/// For each citation whose ID has no declaration, emit one error at the citation
/// site.
///
/// ### 2.4 Missing sections (§FS-check.3.2)
///
/// For each citation with a section path, look up the section in the matching
/// declaration's recorded sections. Missing → one error at the citation site.
///
/// ### 2.5 Broken inline-spec stubs (§FS-check.3.4)
///
/// For each declaration whose H1 has the stub shape `# <ID>: [<text>](<path>)`
/// (description after the colon is a single bare markdown link), extract the link
/// target, resolve it against the repo root, verify the path exists, then re-scan
/// that file for an inline declaration of the same ID. Either failure → one error
/// at the stub site. This is the only rule that re-reads a file; everything else
/// comes from `findings`.
///
/// ### 2.6 Unused declarations (§FS-check.4.1)
///
/// For each declared ID never cited, emit one warning. Warnings do not cause a
/// non-zero exit. `E2E` declarations are exempt — a case is exercised by being
/// run, not by being cited (§FS-check.4.1).
///
/// ### 2.7 Invalid agent-entrypoint init block (§FS-check.3.5)
///
/// When `<root>/AGENTS.md` exists, verify its versioned `grund init` block (and the
/// matching block in any non-symlink companion entrypoint that is present): a
/// missing block, an older version, or a newer unsupported version is one error at
/// the entrypoint's line. When only companion entrypoints exist, validate the ones
/// that already contain a managed block and leave project-owned unmanaged files
/// alone.
///
/// ### 2.8 Ungrounded source files — opt-in (§FS-check.3.6, §DF-require-grounding)
///
/// When `[reference] require_grounding = true` (or `grund check --require-grounding`),
/// every scanned non-`.md` file must carry at least one recognised citation that
/// resolves, or itself declare an ID inline; a source file that does neither is
/// one error anchored at line 1. Off by default.
///
/// ### 2.9 Citation-direction obligations (§FS-check.3.11, §FS-config.3.9, §DF-citation-directions)
///
/// When `[citations]` sets `must` / `should` obligations for a citing kind, every
/// top-level declaration of that kind must carry, in its body, at least one citation
/// satisfying each obligation entry (entries are conjunctive, `|` inside an entry is
/// a disjunction). The body extent and the per-citation `enclosing_declaration` come
/// from the scanner (§AR-scanner.2.4), so this pass is a lookup, not a re-scan. The
/// `code` pseudo-kind obligation is per source file (§FS-config.3.9.2) rather than
/// per declaration. A `must` miss is a `missing-citation` error; a `should` miss is a
/// `suggested-citation` suggestion, emitted only under `--suggestions` (§FS-check.2.3).
///
/// ### 2.10 Citation-direction prohibitions (§FS-check.3.12, §FS-config.3.9, §DF-citation-directions)
///
/// When `[citations]` sets `must-not` / `should-not` prohibitions for a citing kind,
/// every citation site of that kind (its resolved `source_kind`) to a prohibited
/// target — matched on cited kind and namespace per the rule grammar
/// (§FS-config.3.9.3) — is reported at the site. A `must-not` hit is a
/// `forbidden-citation` error; a `should-not` hit is a `discouraged-citation`
/// suggestion, emitted only under `--suggestions` (§FS-check.2.3).
///
/// ### 2.11 Escaped citations that resolve (§FS-check.2.3.1)
///
/// The scanner records every `<§>`-escaped illustration (§AR-scanner.2.5) into
/// `findings.escaped_citations`, a list inert to every rule above. This pass is
/// its only reader: for each escape it runs the same resolver as the dangling
/// check (§2.3) and, when the ID resolves to a real declaration, emits an
/// `escaped-citation-resolves` suggestion — the mirror of dangling, which fires
/// when a *live* citation does not resolve. It is a suggestion, never a warning
/// or error, so it is withheld unless `--suggestions` is passed and never moves
/// the exit code; illustrating a real ID is legitimate.
///
/// ### 2.12 Number-only shorthand citations (§FS-check.3.13, §DF-number-only-citation-shorthand)
///
/// The scanner flags every citation written in the number-only shorthand and, in
/// the same walk, rewrites the uniquely-resolving ones to their canonical `Id`
/// (§AR-scanner.2.6). So by the time the checker runs, a resolved shorthand is
/// indistinguishable from a full citation to every rule above — which is the
/// point: `refs`, `cover`, the unused warning (§2.6), and the direction passes
/// (§2.9, §2.10) all count it without knowing it exists.
///
/// This pass adds the one thing that does differ: a finding naming the canonical
/// form to write. It looks the candidate set up in a per-namespace `(kind,
/// number)` index — built on first use, because deriving it per site is quadratic
/// on the tree this rule asks people to migrate — so the three outcomes (unique,
/// ambiguous, unknown) pick the message. The dangling check (§2.3) skips shorthand
/// sites, so one *cause* never yields two findings; rules judging a different fact
/// about the same site, such as a missing section (§2.4) or a forbidden direction
/// (§2.10), are untouched and report alongside it.
///
/// A resolving shorthand at a site `grund fmt` may not rewrite (§FS-fmt.2.3 — inline
/// code, a link destination, a runtime string) is not reported at all. The citation
/// still resolves and still counts everywhere above; withholding the finding is what
/// keeps `check` from naming `grund fmt --write` as the fix for a site the formatter
/// declines to touch, which would leave the repository permanently red.
///
/// ### 2.13 Scope tiering — `--full` (§FS-check.1.3, §FS-check.3.14, §DF-check-full-scope)
///
/// `[scan] include` decides which roots the walk starts from, so a citation
/// outside it is invisible rather than merely unchecked. `grund check --full`
/// widens the walk to the whole config root and the run then has two scopes.
/// `checker_references.rs` owns both halves of that: the tier is read off the
/// *whole* walk first — resolution failures only, so a directory nobody
/// configured is never judged against conventions it never adopted — and the
/// findings are then narrowed in place to the configured scope, so every rule
/// above sees exactly the tree a run without the flag sees. That ordering is
/// what makes `--full` purely additive: it can only add findings, never withdraw
/// one the ordinary run would have made.
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
    let kind_homes = KindHomeIndex::new(config);
    // §FS-check.3.5: managed agent-entrypoint blocks that are out of date (or
    // newer than this binary), or whose generated citation-directions section
    // has drifted from `[citations]`, are check errors.
    check_agents_block_version(config, &mut report);

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
                column: None,
                message: format!("duplicate declaration of {}{suffix}", render_id(config, id)),
                sites,
            });
        }
    }

    // §FS-check.3.7: declarations must respect configured kind homes. A
    // single-file kind must live in its exact `file`; any declaration inside a
    // unique configured home must match that home's kind.
    for (id, decls) in &findings.declarations {
        for decl in decls {
            if let Some(expected) = kind_homes.single_file_for_kind(&id.kind)
                && !decl.is_stub
                && !paths_same_location_key(&decl.file, &expected.physical_path)
            {
                report.errors.push(Diagnostic {
                    code: "misplaced-declaration",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    column: None,
                    message: format!(
                        "{} must be declared in {} (single-file kind)",
                        render_id(config, id),
                        expected.path
                    ),
                    sites: Vec::new(),
                });
                continue;
            }

            let Some(home) = kind_homes.unique_decl_home_for_file(&decl.file) else {
                continue;
            };
            if home.kind != id.kind {
                report.errors.push(Diagnostic {
                    code: "misplaced-declaration",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    column: None,
                    message: format!(
                        "{} declares kind {} inside {} home {}",
                        render_id(config, id),
                        id.kind,
                        home.kind,
                        home.path
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    // §FS-check.3.1 / §FS-check.3.2 / §FS-check.3.8 / §FS-check.3.13: the
    // reference-resolution family, in `checker_references.rs` because
    // `check --full` runs it a second time over the tree outside `[scan]
    // include` (§AR-checker.2.13, §FS-check.3.14).
    check_citation_resolution(
        findings,
        config,
        workspace,
        ReferenceTier::Configured,
        None,
        &mut report,
    );

    // §FS-check.2.3.1 / §AR-checker.2.11: a `<§>`-escaped illustration whose ID
    // resolves to a real declaration is likely a live citation someone bracketed
    // by mistake — the escape silently makes it inert. Surface it as a suggestion
    // (never an error: illustrating a real ID is legitimate), so it is withheld
    // unless the caller passes `--suggestions`.
    for esc in &findings.escaped_citations {
        if citation_resolves(esc, findings, config, workspace) {
            report.suggestions.push(Diagnostic {
                code: "escaped-citation-resolves",
                path: Some(esc.file.clone()),
                line: Some(esc.line),
                column: Some(esc.column),
                message: format!(
                    "escaped citation {} resolves to a declaration; write {}{} for a live citation, or leave it escaped if it is only an illustration",
                    esc.text.trim(),
                    config.marker,
                    render_qualified_id(config, esc.namespace.as_deref(), &esc.id)
                ),
                sites: Vec::new(),
            });
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
                            column: None,
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
                    column: None,
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
                    column: None,
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
                column: None,
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
                    column: None,
                    message: format!(
                        "ungrounded source file: no {} citation to a declared ID",
                        config.marker
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    // §FS-config.3.9 / §FS-check.3.11 / §FS-check.3.12: citation-direction
    // obligations and prohibitions, when the project declares `[citations]`.
    if config.citations.declared {
        check_citation_obligations(findings, config, &mut report);
        check_citation_prohibitions(findings, config, &mut report);
    }

    sort_diagnostics(&mut report.errors);
    sort_diagnostics(&mut report.warnings);
    sort_diagnostics(&mut report.suggestions);
    report
}

/// §AR-checker.2.9 / §FS-check.3.11: every top-level declaration of a citing
/// kind with a `must` / `should` obligation must carry, in its body, a citation
/// satisfying each obligation entry. `must` misses are `missing-citation`
/// errors; `should` misses are `suggested-citation` suggestions.
fn check_citation_obligations(findings: &Findings, config: &Config, report: &mut CheckReport) {
    // Index every citation once, up front, so each citing kind's obligation pass
    // is a map lookup rather than a fresh O(citations) scan per declaration —
    // the per-declaration / per-case rescans were O(kinds × declarations ×
    // citations) and dominated `grund check` on a large tree (§AR-benchmarks).
    let mut by_decl: BTreeMap<&Id, Vec<&Citation>> = BTreeMap::new();
    let mut code_by_file: BTreeMap<&Path, Vec<&Citation>> = BTreeMap::new();
    for cite in &findings.citations {
        if let Some(id) = &cite.enclosing_declaration {
            by_decl.entry(id).or_default().push(cite);
        }
        if cite.source_kind == CODE_SOURCE_KIND
            && cite.file.extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            code_by_file.entry(cite.file.as_path()).or_default().push(cite);
        }
    }
    // Bucket the (usually zero — fixture trees are carved out of `[scan]`)
    // citations that live under an E2E case directory to their nearest case, so
    // the E2E obligation never `starts_with`-scans every citation per case.
    let mut e2e_by_case: BTreeMap<&Path, Vec<&Citation>> = BTreeMap::new();
    for decls in findings.declarations.values() {
        for decl in decls {
            if decl.e2e_case.is_some() {
                e2e_by_case.entry(decl.file.as_path()).or_default();
            }
        }
    }
    if !e2e_by_case.is_empty() {
        for cite in &findings.citations {
            for ancestor in cite.file.ancestors() {
                if let Some(bucket) = e2e_by_case.get_mut(ancestor) {
                    bucket.push(cite);
                    break;
                }
            }
        }
    }

    for (citing_kind, rules) in &config.citations.per_kind {
        if rules.must.is_empty() && rules.should.is_empty() {
            continue;
        }
        for unit in obligation_units(citing_kind, findings, &by_decl, &code_by_file, &e2e_by_case) {
            for entry in &rules.must {
                if !entry.targets.iter().any(|t| unit.satisfies(t)) {
                    report.errors.push(obligation_diagnostic(
                        "missing-citation",
                        config,
                        &unit,
                        entry,
                        "must",
                    ));
                }
            }
            for entry in &rules.should {
                if !entry.targets.iter().any(|t| unit.satisfies(t)) {
                    report.suggestions.push(obligation_diagnostic(
                        "suggested-citation",
                        config,
                        &unit,
                        entry,
                        "should",
                    ));
                }
            }
        }
    }
}

/// One thing an obligation is evaluated against (§AR-checker.2.9): a declaration
/// body, a `code` source file, or an `E2E` case — together with the citations
/// that count toward it, the `path:line` a finding anchors at, and the subject
/// `id` (a declaration) or `None` (a `code` source file).
struct ObligationUnit<'a> {
    id: Option<&'a Id>,
    path: PathBuf,
    line: usize,
    citations: Vec<&'a Citation>,
    e2e_spec_refs: Vec<&'a E2eSpecRef>,
}

impl ObligationUnit<'_> {
    fn satisfies(&self, target: &CitationTarget) -> bool {
        self.citations
            .iter()
            .any(|cite| citation_matches_target(cite, target))
            || self
                .e2e_spec_refs
                .iter()
                .any(|spec_ref| e2e_spec_ref_matches_target(spec_ref, target))
    }

    fn subject(&self, config: &Config) -> String {
        match self.id {
            Some(id) => render_id(config, id),
            None => "source file".to_string(),
        }
    }
}

/// The evaluation units for one citing kind's obligations (§FS-config.3.9):
/// per source file for `code`, per case (over the case's scanned-file citations)
/// for `E2E`, per non-stub declaration otherwise. Reads the citation indexes
/// built once in [`check_citation_obligations`] rather than rescanning.
fn obligation_units<'a>(
    citing_kind: &str,
    findings: &'a Findings,
    by_decl: &BTreeMap<&'a Id, Vec<&'a Citation>>,
    code_by_file: &BTreeMap<&'a Path, Vec<&'a Citation>>,
    e2e_by_case: &BTreeMap<&'a Path, Vec<&'a Citation>>,
) -> Vec<ObligationUnit<'a>> {
    if citing_kind == CODE_SOURCE_KIND {
        return code_by_file
            .iter()
            .map(|(file, citations)| ObligationUnit {
                id: None,
                path: file.to_path_buf(),
                line: 1,
                citations: citations.clone(),
                e2e_spec_refs: Vec::new(),
            })
            .collect();
    }

    let mut units = Vec::new();
    for (id, decls) in &findings.declarations {
        if id.kind != citing_kind {
            continue;
        }
        for decl in decls {
            if decl.is_stub {
                continue;
            }
            if let Some(case) = &decl.e2e_case {
                // §FS-config.3.9: an E2E obligation evaluates over the case's
                // manifest refs and scanned files when explicit scope includes
                // them. Normal root scans skip fixture trees, but a case with no
                // matching evidence is still an obligation unit, so `must`
                // remains a hard gate.
                let citations = e2e_by_case
                    .get(decl.file.as_path())
                    .cloned()
                    .unwrap_or_default();
                let e2e_spec_refs = case.spec_refs.iter().collect();
                units.push(ObligationUnit {
                    id: Some(id),
                    path: decl.file.clone(),
                    line: decl.line,
                    citations,
                    e2e_spec_refs,
                });
            } else {
                let citations = by_decl.get(id).cloned().unwrap_or_default();
                units.push(ObligationUnit {
                    id: Some(id),
                    path: decl.file.clone(),
                    line: decl.line,
                    citations,
                    e2e_spec_refs: Vec::new(),
                });
            }
        }
    }
    units
}

fn obligation_diagnostic(
    code: &'static str,
    config: &Config,
    unit: &ObligationUnit<'_>,
    entry: &CitationDisjunction,
    verb_level: &str,
) -> Diagnostic {
    Diagnostic {
        code,
        path: Some(unit.path.clone()),
        line: Some(unit.line),
        column: None,
        message: format!(
            "{} {verb_level} cite {} (citation direction)",
            unit.subject(config),
            render_target_phrase(entry)
        ),
        sites: Vec::new(),
    }
}

/// §AR-checker.2.10 / §FS-check.3.12: a citation site whose citing kind prohibits
/// its target is a `forbidden-citation` error (`must-not`) or a
/// `discouraged-citation` suggestion (`should-not`).
fn check_citation_prohibitions(findings: &Findings, config: &Config, report: &mut CheckReport) {
    for cite in &findings.citations {
        match citation_site_level(config, cite) {
            Some(CitationLevel::MustNot) => report.errors.push(prohibition_diagnostic(
                "forbidden-citation",
                cite,
                "must not",
            )),
            Some(CitationLevel::ShouldNot) => report.suggestions.push(prohibition_diagnostic(
                "discouraged-citation",
                cite,
                "should not",
            )),
            _ => {}
        }
    }
}

fn prohibition_diagnostic(code: &'static str, cite: &Citation, verb: &str) -> Diagnostic {
    let target = CitationTarget {
        namespace: match &cite.namespace {
            None => NamespaceMatch::Local,
            Some(alias) => NamespaceMatch::Alias(alias.clone()),
        },
        kind: cite.id.kind.clone(),
    };
    Diagnostic {
        code,
        path: Some(cite.file.clone()),
        line: Some(cite.line),
        column: Some(cite.column),
        message: format!(
            "{} {verb} cite {} (citation direction)",
            cite.source_kind,
            render_citation_target(&target)
        ),
        sites: Vec::new(),
    }
}

/// The direction level a citation site resolves to (§FS-config.3.9.4): the
/// explicit list it matches under its citing kind's rules, else the per-kind
/// `default`, else the global `default`, else `may`.
fn citation_site_level(config: &Config, cite: &Citation) -> Option<CitationLevel> {
    let rules = config.citations.per_kind.get(&cite.source_kind);
    if let Some(rules) = rules {
        let lists = [
            (CitationLevel::Must, &rules.must),
            (CitationLevel::Should, &rules.should),
            (CitationLevel::May, &rules.may),
            (CitationLevel::ShouldNot, &rules.should_not),
            (CitationLevel::MustNot, &rules.must_not),
        ];
        for (level, disjunctions) in lists {
            for disjunction in disjunctions {
                if disjunction
                    .targets
                    .iter()
                    .any(|target| citation_matches_target(cite, target))
                {
                    return Some(level);
                }
            }
        }
        if let Some(default) = rules.default {
            return Some(default);
        }
    }
    config.citations.global_default
}

/// Whether a citation matches a rule target: same cited kind, and a namespace
/// qualifier that covers the citation's namespace (§FS-config.3.9.3).
fn citation_matches_target(cite: &Citation, target: &CitationTarget) -> bool {
    if cite.id.kind != target.kind {
        return false;
    }
    match &target.namespace {
        NamespaceMatch::Any => true,
        NamespaceMatch::Local => cite.namespace.is_none(),
        NamespaceMatch::Alias(alias) => cite.namespace.as_deref() == Some(alias.as_str()),
    }
}

fn e2e_spec_ref_matches_target(spec_ref: &E2eSpecRef, target: &CitationTarget) -> bool {
    if spec_ref.kind != target.kind {
        return false;
    }
    match &target.namespace {
        NamespaceMatch::Any => true,
        NamespaceMatch::Local => spec_ref.namespace.is_none(),
        NamespaceMatch::Alias(alias) => spec_ref.namespace.as_deref() == Some(alias.as_str()),
    }
}

/// Render a disjunction as a human phrase for a finding message: kinds joined by
/// " or " (§FS-init.2.3.5 uses the same phrasing in the agent entrypoint).
fn render_target_phrase(entry: &CitationDisjunction) -> String {
    entry
        .targets
        .iter()
        .map(render_citation_target)
        .collect::<Vec<_>>()
        .join(" or ")
}

/// §FS-check.3.1: the dangling message. A near same-kind ID is a likely typo; a
/// Markdown inline-code context is a likely illustration. Offer whichever
/// applies — and both when a dangling citation in backticks also has a near
/// match. Outside inline code the escape hint is withheld so a prose typo is
/// nudged toward the near ID, not toward escaping.
fn dangling_message(
    config: &Config,
    namespace: Option<&str>,
    findings: &Findings,
    missing: &Id,
    in_inline_code: bool,
) -> String {
    let unknown = render_qualified_id(config, namespace, missing);
    let near = nearest_declared_id(config, namespace, findings, missing);
    let escape = in_inline_code.then(|| format!("<{}>{unknown}", config.marker));
    match (near, escape) {
        (Some(near), Some(escape)) => format!(
            "unknown reference {unknown}; did you mean {near}? (or write {escape} if this is an illustration)"
        ),
        (Some(near), None) => format!("unknown reference {unknown}; did you mean {near}?"),
        (None, Some(escape)) => {
            format!("unknown reference {unknown}; write {escape} if this is an illustration")
        }
        (None, None) => format!("unknown reference {unknown}"),
    }
}

/// §FS-check.3.1: whether a citation site sits inside a Markdown inline-code
/// span, the signal that a dangling `§`-citation may be an illustration. Only
/// the rare dangling path asks, so this re-reads the one line rather than
/// widening every `Citation`; source files (columns shifted by stripped comment
/// prefixes) never qualify. Any read/bounds failure yields `false` — no hint.
fn citation_in_markdown_inline_code(cite: &Citation) -> bool {
    if cite.file.extension().and_then(|e| e.to_str()) != Some("md") {
        return false;
    }
    let Ok(text) = fs::read_to_string(&cite.file) else {
        return false;
    };
    let Some(line) = text.lines().nth(cite.line.saturating_sub(1)) else {
        return false;
    };
    let pos = cite.column.saturating_sub(1);
    pos <= line.len() && is_inside_inline_code(line, pos)
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
                        column: None,
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
                        column: None,
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
                        column: None,
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
                        column: None,
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
fn check_agents_block_version(config: &Config, report: &mut CheckReport) {
    let root = &config.root;
    let canonical = root.join("AGENTS.md");
    let canonical_exists = canonical.exists();
    if canonical_exists {
        check_agent_block_path(config, &canonical, report, true);
    }
    match companion_agent_entrypoints(root) {
        Ok(companions) => {
            for companion in companions {
                check_agent_block_path(config, &companion, report, canonical_exists);
            }
        }
        Err((path, message)) => {
            report.errors.push(Diagnostic {
                code: "io",
                path: Some(path),
                line: Some(1),
                column: None,
                message,
                sites: Vec::new(),
            });
        }
    }
}

fn check_agent_block_path(
    config: &Config,
    path: &Path,
    report: &mut CheckReport,
    require_block: bool,
) {
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
            column: None,
            message: format!("cannot read {file_name}"),
            sites: Vec::new(),
        });
        return;
    };
    let block = match find_agents_block(&text) {
        AgentsBlockLookup::Malformed { message, at } => {
            // §FS-check.3.5 / §FS-init.2.3: broken delimiters are diagnosed at
            // the offending line and never rewritten — `grund init` refuses
            // them too.
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line_for_byte_index(&text, at)),
                column: None,
                message: format!("malformed grund managed block: {message}"),
                sites: Vec::new(),
            });
            return;
        }
        AgentsBlockLookup::Found(block) => Some(block),
        AgentsBlockLookup::Absent => None,
    };
    if let Some(block) = block {
        let line = line_for_byte_index(&text, block.start);
        if block.version < AGENTS_BLOCK_VERSION {
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line),
                column: None,
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
                column: None,
                message: format!(
                    "unsupported grund init block v{} (this grund supports v{})",
                    block.version, AGENTS_BLOCK_VERSION
                ),
                sites: Vec::new(),
            });
        } else {
            // §FS-check.3.5 / §FS-init.2.3.5: citation directions are generated
            // from `[citations]`, so the version marker alone cannot catch a
            // config edit that left the block stale. Re-render the section and
            // byte-compare — rendering is deterministic, so the render is the hash.
            //
            // Strip `\r` so a CRLF checkout (the managed `AGENTS.md` is not
            // pinned to LF in `.gitattributes`, so Windows checks it out with
            // CRLF) compares equal to the LF-rendered section rather than
            // reading as drift.
            let block_text = text[block.start..block.end].replace('\r', "");
            let generated_sections = [
                (
                    "### Citation directions",
                    citation_directions_section(config),
                    "citation directions",
                ),
                // §FS-init.2.3.6: the local-conversation sentence derives from
                // `[reference] conversation`, so flipping the key without
                // re-running `grund init` must surface as drift.
                // The local-conversation sentence also varies by entrypoint
                // (§FS-init.2.3.4.17), so the drift comparison re-renders for
                // *this* file's surface — deriving it from the path, the same
                // way `init` chose it.
                (
                    "### Clickable citations",
                    clickable_citations_section(config, ConversationSurface::for_entrypoint(path)),
                    "clickable citations",
                ),
            ];
            for (heading, expected, noun) in generated_sections {
                if section_in_block(&block_text, heading) != Some(expected.trim_end()) {
                    report.errors.push(Diagnostic {
                        code: "agents-init",
                        path: Some(path.to_path_buf()),
                        line: Some(line),
                        column: None,
                        message: format!(
                            "stale grund init block: {noun} differ from grund.toml (run `grund init` to refresh)"
                        ),
                        sites: Vec::new(),
                    });
                }
            }
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
        column: None,
        message: format!("missing grund init block v{}", AGENTS_BLOCK_VERSION),
        sites: Vec::new(),
    });
}

/// The text of a `heading`-led section inside the managed block, from the heading
/// line to the next heading of any level (or block end), trailing blank lines
/// trimmed. Used to byte-compare config-derived sections against a fresh render
/// (§FS-check.3.5). The boundary is *any* following heading, not just H1/H2, so
/// two adjacent config-derived `###` sections (Citation directions, Clickable
/// citations — §FS-init.2.3.5/2.3.6) do not bleed into each other; neither
/// rendered section contains a `#`-led line, so this cannot cut one short.
fn section_in_block<'a>(block_text: &'a str, heading: &str) -> Option<&'a str> {
    let start = block_text.match_indices(heading).find_map(|(index, _)| {
        let at_line_start = index == 0 || block_text.as_bytes().get(index - 1) == Some(&b'\n');
        let after = index + heading.len();
        let line_ends =
            after == block_text.len() || block_text.as_bytes().get(after) == Some(&b'\n');
        (at_line_start && line_ends).then_some(index)
    })?;
    let section_body_start = start + heading.len();
    // A block-final section is bounded by the `<!-- END GRUND MANAGED BLOCK -->`
    // line, not the block end: the delimiter is the block's frame, never part
    // of a rendered section's body.
    let end = [
        next_heading_offset(block_text, section_body_start),
        AGENTS_BLOCK_END
            .find_at(block_text, section_body_start)
            .map(|m| m.start()),
    ]
    .into_iter()
    .flatten()
    .min()
    .unwrap_or(block_text.len());
    Some(block_text[start..end].trim_end())
}

/// Offset of the next heading line (any `#`-led line) at or after `from`, scanning
/// line by line so a `#` mid-line never counts.
fn next_heading_offset(text: &str, from: usize) -> Option<usize> {
    let mut offset = from;
    for line in text[from..].split_inclusive('\n') {
        if line.trim_start().starts_with('#') {
            return Some(offset);
        }
        offset += line.len();
    }
    None
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

struct DeclarationHome<'a> {
    kind: &'a str,
    path: &'a str,
}

struct SingleFileHome<'a> {
    kind: &'a str,
    path: &'a str,
    physical_path: PathBuf,
}

struct ConfiguredHome<'a> {
    kind: &'a str,
    path: &'a str,
    key: PathBuf,
    exact: bool,
}

struct KindHomeIndex<'a> {
    configured_root: PathBuf,
    physical_root: PathBuf,
    single_files: Vec<SingleFileHome<'a>>,
    homes: Vec<ConfiguredHome<'a>>,
    overlapping_homes: bool,
}

impl<'a> KindHomeIndex<'a> {
    fn new(config: &'a Config) -> Self {
        let configured_root = scanned_path_key(&config.root);
        let physical_root = physical_path_key(&config.root);
        let mut single_files = Vec::new();
        let mut homes = Vec::new();

        for kind in &config.kinds {
            if let Some(file) = kind.file.as_deref() {
                single_files.push(SingleFileHome {
                    kind: kind.prefix.as_str(),
                    path: file,
                    physical_path: physical_path_key(&config.root.join(file)),
                });
                homes.push(ConfiguredHome {
                    kind: kind.prefix.as_str(),
                    path: file,
                    key: configured_home_path_key(file),
                    exact: true,
                });
            }

            if let Some(folder) = kind.folder.as_deref() {
                homes.push(ConfiguredHome {
                    kind: kind.prefix.as_str(),
                    path: folder,
                    key: configured_home_path_key(folder),
                    exact: false,
                });
            }
        }

        let overlapping_homes = homes_have_overlap(&homes);
        if !overlapping_homes {
            homes.sort_by(|left, right| left.exact.cmp(&right.exact));
        }

        Self {
            configured_root,
            physical_root,
            overlapping_homes,
            single_files,
            homes,
        }
    }

    /// The `[[kinds]].file` setting for `kind`, if any — the single document every
    /// declaration of that kind must live in (§FS-config.3.4). Returns `None` for
    /// multi-file kinds (those configured with `folder` instead).
    fn single_file_for_kind(&self, kind: &str) -> Option<&SingleFileHome<'a>> {
        self.single_files.iter().find(|home| home.kind == kind)
    }

    /// The configured kind home that contains `path`, when exactly one
    /// `[[kinds]]` home matches it. `file` homes are exact; `folder` homes are
    /// path-prefix matches against the scanner-recorded path, not the symlink
    /// target (§FS-config.3.4, §FS-check.3.7).
    fn unique_decl_home_for_file(&self, path: &Path) -> Option<DeclarationHome<'a>> {
        let path = scanned_decl_relative_path(path, &self.configured_root, &self.physical_root)?;
        if !self.overlapping_homes {
            return self
                .homes
                .iter()
                .find(|home| home_contains_path(home, path.as_ref()))
                .map(|home| DeclarationHome {
                    kind: home.kind,
                    path: home.path,
                });
        }

        let mut matches = self.homes.iter().filter_map(|home| {
            home_contains_path(home, path.as_ref()).then_some(DeclarationHome {
                kind: home.kind,
                path: home.path,
            })
        });

        let first = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(first)
    }
}

fn home_contains_path(home: &ConfiguredHome<'_>, path: &Path) -> bool {
    if home.exact {
        path == home.key
    } else {
        path.starts_with(&home.key)
    }
}

fn homes_have_overlap(homes: &[ConfiguredHome<'_>]) -> bool {
    homes.iter().enumerate().any(|(index, left)| {
        homes
            .iter()
            .skip(index + 1)
            .any(|right| homes_overlap(left, right))
    })
}

fn homes_overlap(left: &ConfiguredHome<'_>, right: &ConfiguredHome<'_>) -> bool {
    match (left.exact, right.exact) {
        (true, true) => left.key == right.key,
        (true, false) => left.key.starts_with(&right.key),
        (false, true) => right.key.starts_with(&left.key),
        (false, false) => left.key.starts_with(&right.key) || right.key.starts_with(&left.key),
    }
}

fn paths_same_location(left: &Path, right: &Path) -> bool {
    physical_path_key(left) == physical_path_key(right)
}

fn paths_same_location_key(left: &Path, right: &Path) -> bool {
    physical_path_key(left) == right
}

fn physical_path_key(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| normalize_path_lexically(path))
}

fn scanned_path_key(path: &Path) -> PathBuf {
    normalize_path_lexically(path)
}

fn configured_home_path_key(home: &str) -> PathBuf {
    scanned_path_key(Path::new(home))
}

fn scanned_decl_relative_path<'a>(
    path: &'a Path,
    configured_root: &Path,
    physical_root: &Path,
) -> Option<std::borrow::Cow<'a, Path>> {
    if let Ok(relative) = path.strip_prefix(physical_root) {
        return Some(std::borrow::Cow::Borrowed(relative));
    }
    if let Ok(relative) = path.strip_prefix(configured_root) {
        return Some(std::borrow::Cow::Owned(scanned_path_key(relative)));
    }

    let path = scanned_path_key(path);
    if let Ok(relative) = path.strip_prefix(physical_root) {
        return Some(std::borrow::Cow::Owned(scanned_path_key(relative)));
    }
    if let Ok(relative) = path.strip_prefix(configured_root) {
        return Some(std::borrow::Cow::Owned(scanned_path_key(relative)));
    }
    None
}

/// Whether `path` contains a real (non-stub) inline declaration of `id` —
/// the check that a stub's link target actually carries the inline home it claims
/// (§FS-check.3.4, §AR-checker.2.4, §AR-scanner.4).
fn file_declares_inline_home(path: &Path, id: &Id, config: &Config) -> Result<bool> {
    let text = fs::read_to_string(path)?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let mut py_docstring = PythonDocstringScanState::default();
    for line in text.lines() {
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        let scan_line = scan.text;
        if let Some(caps) =
            declaration_captures(&config.grammar, scan_line, scan.in_py_docstring, is_md)
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
