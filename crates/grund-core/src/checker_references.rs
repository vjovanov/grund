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
    /// The `scan = false` homes under those roots (§FS-config.3.4.7): listed by
    /// the config, and read by this run only because `--full` widened the walk.
    unwalked: Vec<PathBuf>,
}

impl ScanScope {
    fn contains(&self, path: &Path) -> bool {
        // §FS-config.3.4.7: a file in a home the config lists without walking is
        // outside the configured scope even when a root above it is inside — the
        // scope is a set of roots, less the homes a run without `--full` never reads.
        if self.unwalked.iter().any(|home| path.starts_with(home)) {
            return false;
        }
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
    // Both spellings again, for the same reason the roots carry both: a finding
    // is recorded under the path the walk reached it by (§FS-config.3.5.2).
    let mut unwalked = unwalked_home_roots(config);
    let canonical = unwalked
        .iter()
        .filter_map(|home| fs::canonicalize(home).ok())
        .collect::<Vec<_>>();
    unwalked.extend(canonical);
    unwalked.sort_by_key(|home| sort_path_key(home));
    unwalked.dedup();
    Ok(Some(ScanScope { roots, unwalked }))
}

/// §FS-check.1.3: drop everything the wider `--full` walk read from outside the
/// configured scope, so every rule but the out-of-scope tier sees exactly the
/// tree a run without the flag sees and reports exactly what it reports. A no-op
/// without `--full`. Nothing is cloned — the walk's findings are narrowed in
/// place, after the tier has been read off the whole of them.
///
/// Why there is no undo pass for the shorthand resolutions §AR-scanner.2.6 made
/// against the whole walk: the narrowed declarations yield the candidate set
/// `report_shorthand_citation` judges a site against — one predicate covering the
/// unqualified and the cross-member qualified form alike, where an undo pass here
/// could only reach the unqualified one.
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
    // §FS-check.4.6 asks a question about the configured scope, so a `--full`
    // walk's extra files are dropped with the rest: `--full` widens the
    // *reference* tier (§FS-check.3.14) and nothing else.
    findings
        .near_miss_headings
        .retain(|heading| scope.contains(&heading.file));
    findings.scanned_files.retain(|file| scope.contains(file));
    // The shorthand resolutions §AR-scanner.2.6 performed against the whole walk
    // are deliberately left standing: a site whose declaration the retains above
    // just dropped is re-judged in `report_shorthand_citation` (§FS-check.3.13).
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
///
/// §FS-workspace.4: an alias path into an absent optional member is neither
/// resolved nor unknown — it is *unverified*, and the run says so once at the entry
/// that made the skip legal rather than at every site (§FS-check.4.9). Every other
/// unknown alias still errors here.
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
            // §FS-workspace.4: unverified, not unknown — see this function's docs.
            if namespace_is_unverified(config, namespace) {
                continue;
            }
            report.errors.push(Diagnostic {
                code: "unknown-project",
                path: Some(cite.file.clone()),
                line: Some(cite.line),
                column: Some(cite.column),
                message: unknown_project_message(
                    namespace,
                    workspace.keys().map(String::as_str),
                    &config.workspace_scope_path,
                ),
                sites: Vec::new(),
            });
            continue;
        };
        // §FS-check.3.13 / §AR-checker.2.12: the shorthand pass, and the one rule that
        // can end this citation early — an unresolved shorthand skips the dangling check
        // below rather than adding `unknown reference FS-042` for a token that is not an ID.
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
                let coordinate = format!(
                    "{}{}{}",
                    render_qualified_id(target.config, cite.namespace.as_deref(), &cite.id),
                    target.config.section_separator,
                    sec
                );
                let message = if target.config.named_sections
                    && target.config.grammar.is_named_section(Some(sec))
                    && cite.has_marker
                {
                    format!(
                        "section not found: {coordinate}; write <{}> before it to show the shape without citing it",
                        target.config.marker
                    )
                } else {
                    format!("missing section {coordinate}")
                };
                report.errors.push(Diagnostic {
                    code: "missing-section",
                    path: Some(cite.file.clone()),
                    line: Some(cite.line),
                    column: Some(cite.column),
                    message,
                    sites: Vec::new(),
                });
            }
        }
    }
}

/// §FS-check.3.8: the unknown-alias message. A qualified citation names its
/// target by the whole alias path (§FS-workspace.6.1), and the mistake that
/// invites is writing a project's short name where its path is required. So
/// before giving up, look for a project the citation could have meant: one
/// whose path *ends* with what was written (`sprayer` for `group/sprayer`), one
/// whose last segment matches under a different parent (a wrong prefix), or one
/// a typo away. §GOAL-friendliness-first — the reader who has the tree in front
/// of them is the one who does not need this; the agent editing one file is.
///
/// Every tier is off in a **narrowed** run (non-empty `scope_path`), which sees
/// only its own subtree: it cannot tell a path that dropped a prefix from one
/// that correctly names a project above or beside the subtree, so every
/// candidate it could offer is a rewrite of a citation the run CI performs
/// accepts — green before and green after, with nothing left to catch it. Such a
/// run names the scope it covers and offers nothing (§FS-check.3.8,
/// §FS-workspace.6.1). It names it as a *subtree*, because the scope path is one
/// project among several in scope — `alpha` with `alpha/beta` below it — and
/// "only alpha is in scope" reads as "alpha is the only project".
fn unknown_project_message<'a>(
    namespace: &str,
    known: impl Iterator<Item = &'a str>,
    scope_path: &str,
) -> String {
    if !scope_path.is_empty() {
        return format!(
            "unknown project alias {namespace}; only the {scope_path} subtree is in scope here — check from the workspace root for a path outside it"
        );
    }
    let candidates = nearest_project_aliases(namespace, known);
    if candidates.is_empty() {
        return format!("unknown project alias {namespace}");
    }
    format!(
        "unknown project alias {namespace}; did you mean {}?",
        join_alternatives(&candidates)
    )
}

/// The projects a written alias path plausibly meant, best tier first. Tiers do
/// not mix: a suffix match is a near-certain "you dropped the prefix", and
/// diluting it with edit-distance noise would make the good hint harder to act
/// on. Only the outermost root asks — a narrowed run offers nothing at all
/// (§FS-check.3.8), so no tier here is conditional on scope.
fn nearest_project_aliases<'a>(namespace: &str, known: impl Iterator<Item = &'a str>) -> Vec<String> {
    let written: Vec<&str> = namespace.split('/').collect();
    let (mut suffix, mut same_leaf, mut near) = (Vec::new(), Vec::new(), Vec::new());
    for candidate in known {
        let segments: Vec<&str> = candidate.split('/').collect();
        if segments.len() > written.len() && segments.ends_with(&written) {
            suffix.push(candidate.to_string());
        } else if segments.last() == written.last() {
            same_leaf.push(candidate.to_string());
        } else if close_enough_for_hint(
            edit_distance(namespace, candidate),
            namespace.chars().count(),
            candidate.chars().count(),
        ) {
            near.push(candidate.to_string());
        }
    }
    let mut best = if !suffix.is_empty() {
        suffix
    } else if !same_leaf.is_empty() {
        same_leaf
    } else {
        near
    };
    best.sort();
    // Three is enough to disambiguate the common `api` collision without
    // turning one finding into a catalogue; `grund list` is the catalogue.
    best.truncate(3);
    best
}

fn join_alternatives(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}
