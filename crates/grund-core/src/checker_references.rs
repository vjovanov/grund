/// The reference-resolution rule family — dangling citations (§FS-check.3.1),
/// missing sections (§FS-check.3.2), unknown project aliases (§FS-check.3.8),
/// and unresolved number-only shorthands (§FS-check.3.13) — together with the
/// scope layer that decides where it is reported (§FS-check.1.3, §FS-check.3.14,
/// §AR-checker.2.13).
///
/// It sits beside `checker.rs` rather than inside it because `grund check --full`
/// runs this one family a second time, over the part of the tree `[scan] include`
/// leaves out, while every other rule stays inside the configured scope
/// (§AR-core-module-layout.1).

/// Which of a `--full` run's two scopes a citation site is being judged on
/// (§FS-check.1.3). It changes exactly one thing: outside the configured scope,
/// `grund fmt --write` will not rewrite the site either, so the mechanical
/// shorthand form is withheld there (§FS-check.3.14).
#[derive(Clone, Copy, Eq, PartialEq)]
enum ReferenceTier {
    Configured,
    OutOfScope,
}

/// The roots a run *without* `--full` walks: the explicit path argument, or
/// `[scan] include` resolved against the config root (§FS-config.3.5). Under
/// `--full` the walk is wider than this, and the difference is what separates
/// the ordinary report from the out-of-scope tier (§FS-check.1.3).
///
/// Each root is held in both its written and its canonical form: the walk yields
/// paths built from `config.root`, while `E2E` case declarations carry
/// canonicalized directories (§AR-scanner.6), and a scope test has to answer the
/// same for both.
struct ScanScope {
    roots: Vec<PathBuf>,
}

impl ScanScope {
    fn contains(&self, path: &Path) -> bool {
        self.roots
            .iter()
            .any(|root| path == root || path.starts_with(root))
    }
}

/// §FS-check.1.3: the configured scope of this run, or `None` when the walk was
/// already the configured one — without `--full` there is no second tier, and no
/// narrowing to do.
fn configured_scope(
    config: &Config,
    path: &Path,
    path_provided: bool,
    full: bool,
) -> Result<Option<ScanScope>> {
    if !full {
        return Ok(None);
    }
    let mut roots = scan_roots_for(config, Some(path), path_provided, false)?;
    let canonical = roots
        .iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .collect::<Vec<_>>();
    roots.extend(canonical);
    roots.sort_by_key(|root| sort_path_key(root));
    roots.dedup();
    Ok(Some(ScanScope { roots }))
}

/// §FS-check.1.3: drop everything the wider `--full` walk read from outside the
/// configured scope, so every rule but the out-of-scope tier sees exactly the
/// tree a run without the flag sees and reports exactly what it reports. A no-op
/// without `--full`. Nothing is cloned — the walk's findings are narrowed in
/// place, after the tier has been read off the whole of them.
fn retain_findings_in_scope(findings: &mut Findings, scope: Option<&ScanScope>) {
    let Some(scope) = scope else { return };
    findings
        .declarations
        .retain(|_, decls| {
            decls.retain(|decl| scope.contains(&decl.file));
            !decls.is_empty()
        });
    findings.citations.retain(|cite| scope.contains(&cite.file));
    findings
        .escaped_citations
        .retain(|cite| scope.contains(&cite.file));
    findings.scanned_files.retain(|file| scope.contains(file));
    // The shorthand resolutions §AR-scanner.2.6 performed against the whole walk
    // are deliberately left standing. A site whose declaration the retains above
    // just dropped is judged by the candidate set the *narrowed* declarations
    // yield, in `report_shorthand_citation` — one predicate covering the
    // unqualified and the cross-member qualified form alike, rather than an undo
    // pass here that could only reach the unqualified one (§FS-check.3.13).
}

/// §FS-check.3.14: the out-of-scope tier — the reference-resolution family run
/// over the citation sites the wider `--full` walk found outside the configured
/// scope, resolved against the *whole* walk so a citation whose declaration is
/// also out there still resolves. Empty without `--full`.
fn out_of_scope_references(
    findings: &Findings,
    config: &Config,
    workspace: &BTreeMap<String, WorkspaceCheckTarget<'_>>,
    scope: Option<&ScanScope>,
) -> Vec<Diagnostic> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let mut tier = CheckReport::default();
    check_citation_resolution(
        findings,
        config,
        workspace,
        ReferenceTier::OutOfScope,
        Some(scope),
        &mut tier,
    );
    tier.errors.into_iter().map(tag_out_of_scope).collect()
}

/// §FS-check.1.3: the out-of-scope tier for a workspace run — one pass per
/// project, tiered against that project's own `[scan] include`, and resolved
/// against every project's *whole* walk, which is why it runs before the
/// findings are narrowed. `include` is a per-project statement, so a member
/// widens past its own and no other.
fn workspace_out_of_scope_references(
    projects: &[WorkspaceProject],
    scopes: &[Option<ScanScope>],
) -> Vec<Diagnostic> {
    if scopes.iter().all(Option::is_none) {
        return Vec::new();
    }
    let workspace = projects
        .iter()
        .map(|project| {
            (
                project.alias.clone(),
                WorkspaceCheckTarget {
                    findings: &project.findings,
                    config: &project.config,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Vec::new();
    for (project, scope) in projects.iter().zip(scopes) {
        diagnostics.extend(out_of_scope_references(
            &project.findings,
            &project.config,
            &workspace,
            scope.as_ref(),
        ));
    }
    diagnostics
}

/// §FS-check.3.14: the tier's own code and message shape.
///
/// The code is the in-scope one under an `out-of-scope-` prefix, so a
/// `--format=json` consumer filters the tier by prefix and the rule by exact
/// match on the `code` field the report shape already carries (§FS-errors.5) —
/// one code for all four would leave the rule readable only in the prose.
///
/// The tier leads the message rather than trailing it: out here the fix is
/// usually to widen `[scan] include`, so a rule's own fix-it hint ("did you
/// mean …?") is the fact most likely to be wrong and least deserving of being
/// read first.
fn tag_out_of_scope(mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.code = match diagnostic.code {
        "dangling" => "out-of-scope-dangling",
        "missing-section" => "out-of-scope-missing-section",
        "unknown-project" => "out-of-scope-unknown-project",
        "shorthand-citation" => "out-of-scope-shorthand-citation",
        // `check_citation_resolution` emits exactly the four above; anything
        // else would be a new rule joining the family without a tier code.
        other => other,
    };
    diagnostic.message = format!("outside [scan] include: {}", diagnostic.message);
    diagnostic
}

/// §AR-checker.2.3, §AR-checker.2.4, §AR-checker.2.12: resolve every citation and
/// report the ones that resolve to nothing. `outside`, when set, restricts the
/// pass to sites *outside* that scope — the out-of-scope tier's one difference in
/// what it looks at (§FS-check.3.14); the ordinary run passes `None` and judges
/// every site the walk found.
fn check_citation_resolution(
    findings: &Findings,
    config: &Config,
    workspace: &BTreeMap<String, WorkspaceCheckTarget<'_>>,
    tier: ReferenceTier,
    outside: Option<&ScanScope>,
    report: &mut CheckReport,
) {
    let mut shorthand_indexes = ShorthandIndexes::default();
    for cite in &findings.citations {
        if let Some(scope) = outside
            && scope.contains(&cite.file)
        {
            continue;
        }
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
                column: Some(cite.column),
                message: format!("unknown project alias {namespace}"),
                sites: Vec::new(),
            });
            continue;
        };
        // §FS-check.3.13 / §AR-checker.2.12: the shorthand pass, and the one rule
        // that can end this citation early — an unresolved shorthand skips the
        // dangling check below rather than adding `unknown reference FS-042` for a
        // token that is not a full ID.
        if cite.shorthand
            && report_shorthand_citation(
                cite,
                config,
                &target,
                tier,
                &mut shorthand_indexes,
                report,
            )
        {
            continue;
        }
        // §FS-check.3.1 / §FS-workspace.4: a citation whose ID is declared
        // nowhere in its target namespace is dangling.
        let Some(decls) = target.findings.declarations.get(&cite.id) else {
            let message = dangling_message(
                target.config,
                cite.namespace.as_deref(),
                target.findings,
                &cite.id,
                citation_in_markdown_inline_code(cite),
            );
            report.errors.push(Diagnostic {
                code: "dangling",
                path: Some(cite.file.clone()),
                line: Some(cite.line),
                column: Some(cite.column),
                message,
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
                    column: Some(cite.column),
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
}
