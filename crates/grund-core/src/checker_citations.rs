// The citation-direction half of the checker (§FS-config.3.9), in a file of its
// own beside `checker_references.rs` and `checker_sections.rs`
// (§AR-core-module-layout.1): the obligation pass (§FS-check.3.11), the
// prohibition pass (§FS-check.3.12), and the two questions both of them ask —
// what kind of place a citation sits in, and whether it matches a rule's
// target. `checker.rs` keeps the declaration-shape rules and the diagnostic
// helpers they share.

/// How a citing kind is named in a finding (§FS-check.3.11, §FS-check.3.12): a
/// citable kind by its name, which is the prefix of every ID in it; a
/// non-citable one by its home, which is all a reader of the message could go
/// and look at. `code` keeps its own name — it is the one non-citable kind with
/// no place, being the complement of every home there is.
fn citing_side_label(config: &Config, kind: &str) -> String {
    config
        .kinds
        .iter()
        .find(|configured| configured.kind == kind && !configured.citable)
        .and_then(KindConfig::place_label)
        .unwrap_or_else(|| kind.to_string())
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
    let mut by_file: BTreeMap<(&str, &Path), Vec<&Citation>> = BTreeMap::new();
    // Resolved once, not per citation: the per-file question below is asked of
    // every citation in the tree, and answering it by scanning `[[kinds]]` each
    // time would make the pass O(citations × kinds) for no gain (§AR-benchmarks).
    let non_citable = non_citable_kind_names(config);
    for cite in &findings.citations {
        if let Some(id) = &cite.enclosing_declaration {
            by_decl.entry(id).or_default().push(cite);
        }
        if file_is_obligation_unit(&non_citable, cite) {
            by_file
                .entry((cite.source_kind.as_str(), cite.file.as_path()))
                .or_default()
                .push(cite);
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
        for unit in obligation_units(
            citing_kind,
            config,
            findings,
            &by_decl,
            &by_file,
            &e2e_by_case,
        ) {
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
    /// The place a non-citable kind's unit is named by (§FS-check.3.11) — its
    /// home, since the unit is a file in it and the kind has no ID to print.
    place: Option<String>,
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
        match (self.id, &self.place) {
            (Some(id), _) => render_id(config, id),
            (None, Some(place)) => place.clone(),
            (None, None) => "source file".to_string(),
        }
    }
}

/// Whether this citation site counts toward a *per-file* obligation unit
/// (§FS-check.3.11). Two kinds of citing side have no declaration to attach an
/// obligation to, and both answer with the file:
///
/// * `code` — every citation outside a configured home, source files only. A
///   README or a changelog is a document, and §FS-check.3.6 exempts it for the
///   same reason.
/// * a **non-citable kind** — every scanned file in its home, `.md` included.
///   Inheriting `code`'s Markdown exemption here would make `must` inert on the
///   kinds that are usually all Markdown, which is most of them: the exemption
///   reasons about implementation-versus-document, and a home the maintainer
///   named is neither guess.
fn file_is_obligation_unit(non_citable: &BTreeSet<&str>, cite: &Citation) -> bool {
    if cite.source_kind == CODE_SOURCE_KIND {
        return cite.file.extension().and_then(|ext| ext.to_str()) != Some("md");
    }
    non_citable.contains(cite.source_kind.as_str())
}

/// The configured kinds that declare no IDs (§FS-config.3.4.1), by name.
fn non_citable_kind_names(config: &Config) -> BTreeSet<&str> {
    config
        .kinds
        .iter()
        .filter(|kind| !kind.citable)
        .map(|kind| kind.kind.as_str())
        .collect()
}

/// The evaluation units for one citing kind's obligations (§FS-config.3.9):
/// per file for `code` and for every non-citable kind, per case (over the case's
/// scanned-file citations) for `E2E`, per non-stub declaration otherwise. Reads
/// the citation indexes built once in [`check_citation_obligations`] rather than
/// rescanning.
fn obligation_units<'a>(
    citing_kind: &str,
    config: &Config,
    findings: &'a Findings,
    by_decl: &BTreeMap<&'a Id, Vec<&'a Citation>>,
    by_file: &BTreeMap<(&'a str, &'a Path), Vec<&'a Citation>>,
    e2e_by_case: &BTreeMap<&'a Path, Vec<&'a Citation>>,
) -> Vec<ObligationUnit<'a>> {
    if citing_kind == CODE_SOURCE_KIND || non_citable_kind_names(config).contains(citing_kind) {
        let place = config
            .kinds
            .iter()
            .find(|kind| kind.kind == citing_kind)
            .and_then(KindConfig::place_label);
        return by_file
            .iter()
            .filter(|((kind, _), _)| *kind == citing_kind)
            .map(|((_, file), citations)| ObligationUnit {
                id: None,
                place: place.clone(),
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
                    place: None,
                    path: decl.file.clone(),
                    line: decl.line,
                    citations,
                    e2e_spec_refs,
                });
            } else {
                let citations = by_decl.get(id).cloned().unwrap_or_default();
                units.push(ObligationUnit {
                    id: Some(id),
                    place: None,
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
                config,
                cite,
                "must not",
            )),
            Some(CitationLevel::ShouldNot) => report.suggestions.push(prohibition_diagnostic(
                "discouraged-citation",
                config,
                cite,
                "should not",
            )),
            _ => {}
        }
    }
}

fn prohibition_diagnostic(
    code: &'static str,
    config: &Config,
    cite: &Citation,
    verb: &str,
) -> Diagnostic {
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
            // §FS-check.3.12: a non-citable citing kind is named by its place —
            // the same label §FS-check.3.11 and the generated directions use,
            // because its name is a config handle and not a thing to read.
            citing_side_label(config, &cite.source_kind),
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
