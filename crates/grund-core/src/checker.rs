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
/// ### 2.8 Ungrounded units — opt-in (§FS-check.3.6, §DF-require-grounding)
///
/// Off by default, and asked per `[[kinds]]` row: each scanned file resolves to
/// the one row that governs it (§FS-check.3.6.1), and that row's effective
/// `require_grounding` / `grounding_level` decide whether the file is checked and
/// what the unit inside it is (§FS-config.3.4.8). At level 1 the unit is the file
/// and the rule is what it always was — one resolving citation anywhere, or an
/// inline declaration, anchored at line 1. Above it the units are the heading
/// subtrees the scanner recorded for a Markdown file, or its doc-comment blocks
/// for a source one (§AR-scanner.2.7), each finding anchored at its own unit. The
/// pass is `checker_grounding.rs`; `[citations]` obligations read the same cut
/// (§2.9), so *whether* and *what* are asked of one thing.
///
/// ### 2.9 Citation-direction obligations (§FS-check.3.11, §FS-config.3.9, §DF-citation-directions)
///
/// When `[citations]` sets `must` / `should` obligations for a citing kind, every
/// top-level declaration of that kind must carry, in its body, at least one citation
/// satisfying each obligation entry (entries are conjunctive, `|` inside an entry is
/// a disjunction). The body extent and the per-citation `enclosing_declaration` come
/// from the scanner (§AR-scanner.2.4), so this pass is a lookup, not a re-scan. The
/// homeless-kind obligation is per source file (§FS-config.3.9.2) rather than
/// per declaration, and both per-file units are cut by the row's
/// `grounding_level` like the grounding pass above (§FS-check.3.11). A `must` miss is a `missing-citation` error; a `should` miss is a
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
/// This pass adds the one thing that does differ: under the target project's
/// `canonical` policy, a finding naming the canonical form to write; under
/// `accepted`, a unique marker-origin shorthand adds no form finding
/// (§FS-config.3.1). Unknown and ambiguous candidates remain findings under both
/// policies. It looks the candidate set up in a per-namespace `(kind,
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
/// one the ordinary run would have made. The narrowing also undoes the shorthand
/// resolution the wider walk enabled (§AR-scanner.2.6) where the declaration it
/// resolved against has just been dropped, so such a site is the unresolved
/// shorthand a plain run reports — one cause, one finding (§FS-check.3.13).
///
/// ### 2.14 Inline citation style (§FS-check.3.10, §FS-check.4.4, §FS-inline-citation-style.4)
///
/// One pass over `findings.citations`, deduplicated by enclosing comment block,
/// judging each block against `[reference] inline_style`, the `inline_note_*`
/// budgets, and `inline_note_layout`. Everything it compares — the block's span,
/// its widest column, whether it carries a note, and which of its lines deviate
/// from the configured layout — was recorded by the scanner (§AR-scanner.3), so
/// like every rule above except §2.5 this one reads no file. A site that misses
/// several caps yields one finding per cap; a block whose layout deviates yields
/// one per offending *line*, anchored there rather than at the block's opener,
/// because that is the line an author edits (§FS-inline-citation-style.4.4). Two
/// of the three tiers are opt-in and silent by default: the soft cap under
/// `warn_on_suggested`, the layout under `inline_note_layout_check`.
///
/// The rule lives in `inline_note_layout.rs` with the classifier the scanner
/// annotates from, rather than here — one file per invariant, the arrangement
/// §2.12's shorthand rule already uses for the same reason.
///
/// ### 2.15 Duplicate section paths (§FS-check.3.16, §DF-duplicate-section-path)
///
/// One pass over the declarations. The scanner records a section path once, by
/// the first heading that claims it, and appends every later claimant *inside the
/// declaration's own body* to `duplicate_sections` (§AR-scanner.2.2); this rule
/// groups that list by path and emits one error per collided path, anchored at
/// the first heading with the rest named in the message — §2.1's shape for
/// declarations, one level down. The heading-level rule above reads only the map,
/// so a duplicate heading is not additionally judged for depth: nothing resolves
/// to it, and the run has already said it should not exist
/// (§DF-duplicate-section-path.2.4).
///
/// Nothing here re-derives *which* headings a declaration owns — the scan
/// answered that once, which is what makes `show`'s refusal (§FS-show.2.2.2) name
/// exactly the coordinates this rule reports. A heading in the next item's
/// doc-comment and a stub's prose are outside the body and never reach the list,
/// so neither is filtered here (§FS-check.3.16).
///
/// ### 2.16 Kind indexes (§FS-check.3.18, §FS-check.3.17, §DF-index-entry-form)
///
/// One pass per `[[kinds]]` entry that has a `folder` and an enabled `index`
/// (§FS-config.3.4). For each, membership is the declarations under that
/// folder's whole subtree — a stub and the inline body it points at collapsing
/// to one ID, as in §2.1 — plus an external inline declaration whose canonical
/// bare-ID source link enrolls it directly (§FS-check.3.18). The citations already
/// recorded in the index file say which members it names. The index file itself
/// is re-read, the second and last rule that touches disk after §2.5, because
/// wrapper form and an external enrollment's exact destination are facts about
/// the line, not the citation record. Ordinary in-folder entries still require
/// only the wrapper shape; only external enrollment compares the destination to
/// the one `fmt` derives (§FS-fmt.6.2, §DF-index-entry-form.2.7).
///
/// Both halves of the entry contract are errors, each anchored where its own fix
/// is: a missing entry at the declaration, a bare one at its line in the index.
/// They arrived at that verdict by different routes — the bare entry on arrival,
/// the missing one at the end of a ramp (§DF-index-compatibility-ramp.3) — and
/// the anchors are what still tells them apart. The same pass owns the carve-out
/// that keeps §2.6 honest: an index entry is not an inbound citation, so the
/// unused warning still fires for a declaration only its own index names
/// (§DF-index-not-an-inbound-citation). The finding pass lives in
/// `checker_index.rs` and its shared membership derivation in
/// `checker_index_entries.rs`, one file per invariant family and bounded helper
/// (§AR-core-module-layout.1, §AR-core-module-layout.3).
///
/// ### 2.17 Named section prefixes (§FS-check.3.19)
///
/// One pass over each declaration's scanner-recorded section set. For every
/// name-bearing path, walk its proper prefixes and emit one `orphan-section`
/// error at the descendant heading for the first prefix absent from the same
/// declaration. The pass does not parse headings or infer hierarchy from their
/// Markdown placement; it consumes the shared path set. It is independent of
/// heading-depth and duplicate checks, so those findings compose rather than
/// suppress one another. Purely numeric paths bypass the pass.
///
/// ### 2.18 Explicit values (§FS-values.5, §DA-explicit-value-bindings)
///
/// A focused `checker_values` pass consumes the scanner's declarations,
/// components, bindings, and exact spans without rereading files. It routes the
/// binding citation through the same local/workspace resolver as every citation;
/// config, declaration, and resolution failures suppress comparison. Only one
/// valid unique numbered target reaches exact decimal-or-decoded-string equality,
/// producing the fixed value errors and declaration site required by §FS-values.5.
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
    check_with_workspace(findings, config, config, None, &BTreeMap::new())
}

/// `path_config` is the config the finished report renders paths against
/// (§FS-workspace.8.1) — the workspace root's in workspace mode, `config` itself
/// otherwise. A path baked *into* a message must use it, or in a workspace it
/// would be spelled from the member's root while the finding's own anchor is
/// spelled from the workspace's, and neither the reader nor an editor could
/// follow it (§FS-config.3.6).
///
/// Why the escaped-citation finding is only a suggestion: illustrating a real ID
/// in prose is legitimate, so a resolving escape is never an error — it is
/// withheld unless the caller passes `--suggestions`.
///
/// Why an index entry is not counted as an inbound citation: an index names every
/// declaration in its folder by construction, so counting its entries would leave
/// every ID in an indexed folder permanently "cited" and empty the `unused`
/// warning of everything it exists to find.
///
/// Why grounding needs no tooling: it is a pure function of (tree, config) — no
/// git, no AST. Markdown is exempt from it because a document is not
/// implementation; a non-citable home is not, because it is a directory the
/// maintainer declared matters and is usually all Markdown, so the exemption
/// would make the rule inert exactly where it was asked for. Which place is
/// asked, and how finely, is a `[[kinds]]` row's to say (§FS-config.3.4.8).
fn check_with_workspace(
    findings: &Findings,
    config: &Config,
    path_config: &Config,
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
                // §FS-errors.3 / §FS-workspace.8.1: `path_config`, not `config`
                // — the printer anchors this finding from the report root, so
                // the sites named inside its message come from there too.
                .map(|site| format!("{}:{}", display_path(path_config, &site.path), site.line))
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
                // §FS-check.3.7: a non-citable home has no kind an author could
                // have declared instead, so the message names the place and says
                // why, rather than pointing at a kind that does not exist.
                let message = if home.citable {
                    format!(
                        "{} declares kind {} inside {} home {}",
                        render_id(config, id),
                        id.kind,
                        home.kind,
                        home.path
                    )
                } else {
                    format!(
                        "{} must not be declared in {} (not a citable home)",
                        render_id(config, id),
                        home.place()
                    )
                };
                report.errors.push(Diagnostic {
                    code: "misplaced-declaration",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    column: None,
                    message,
                    sites: Vec::new(),
                });
            }
        }
    }

    // §FS-check.3.1 / §FS-check.3.2 / §FS-check.3.8 / §FS-check.3.13: the
    // reference-resolution family, in `checker_references.rs` (§AR-checker.2.13,
    // §FS-check.3.14) because `check --full` reruns it outside `[scan] include`.
    check_citation_resolution(
        findings,
        config,
        path_config,
        workspace,
        ReferenceTier::Configured,
        None,
        &mut report,
    );
    // §AR-checker.2.18 / §FS-values.5: ordinary resolution runs first and the
    // focused pass suppresses comparison at every unresolved or ambiguous site.
    check_values(findings, config, path_config, workspace, &mut report);

    // §FS-check.2.3.1 / §AR-checker.2.11: a `<§>`-escaped illustration whose ID
    // resolves to a real declaration is likely a live citation someone bracketed
    // by mistake — the escape silently makes it inert.
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

    // §FS-check.3.9 / §FS-check.3.16: the depth a declaration's own section
    // headings write, and whether two claim one path. One file per invariant
    // family in `checker_sections.rs` (§AR-checker.2.15, §AR-core-module-layout.1).
    check_section_headings(findings, config, path_config, &mut report);

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

    // §FS-check.3.18 / §FS-check.3.17: a kind's index must list every declaration
    // in its folder, as a full link. In `checker_index.rs` — one file per
    // invariant family, the arrangement §2.15's section rules already use.
    check_kind_indexes(findings, config, path_config, &mut report);

    // §FS-check.4.1: a declaration nothing cites is a warning, not an error —
    // except E2E cases, which are proof artifacts, not citation targets. An
    // index entry is not an inbound citation (§DF-index-not-an-inbound-citation).
    let index_entries = KindIndexEntries::new(findings, config);
    let mut cited: BTreeSet<&Id> = findings
        .citations
        .iter()
        .filter(|cite| cite.namespace.is_none() && !index_entries.is_index_entry(cite))
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

    // §FS-check.3.6 / §DF-require-grounding: the grounding pass, per `[[kinds]]`
    // row and per unit, in `checker_grounding.rs` (§AR-checker.2.8).
    check_grounding(findings, config, &kind_homes, workspace, &mut report);

    // §FS-check.4.6: headings that open like a declaration and parse as none.
    check_declaration_near_misses(findings, &mut report);

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
