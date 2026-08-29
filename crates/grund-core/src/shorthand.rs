/// One parsed ID token: the `Id`, its optional section path, and whether it was
/// written in the number-only shorthand (§FS-check.1.2). A shorthand `Id` carries
/// `slug: None` until the resolution pass fills it in (§AR-scanner.2.6).
///
/// The number-only citation shorthand (§FS-check.1.2, §FS-fmt.2.4,
/// §DF-number-only-citation-shorthand), gathered here rather than spread across
/// the seven passes it plugs into.
///
/// The categories in §AR-core-module-layout.1 cut by *stage* — scanner, checker,
/// fmt, api. This rule is one contract that has to hold identically at every one
/// of them: the shape recognized in a file, the shape accepted as a CLI
/// argument, the shape reported, and the shape rewritten are the same shape, and
/// a divergence between any two of them is the defect the rule exists to fix.
/// Split by stage, the four halves of one invariant would sit in four files with
/// nothing naming the invariant — so it lives as a feature module and each stage
/// keeps a one-line call into it.
///
/// What is *not* the rule sits in `id_format.rs`: the `[id] format` template and
/// the post-match tests for where a token of that grammar ends — questions about
/// the shape, which the rule here then serves.
///
/// The crate is assembled by a flat `include!` in `lib.rs`
/// (§AR-core-module-layout.2), which makes an inner `//!` illegal here, so this
/// file-level prose hangs off the first item. Everything here is crate-private and
/// reached that way; the public embedding surface stays in `api.rs`.
struct ParsedId {
    id: Id,
    section: Option<String>,
    shorthand: bool,
}

/// `parse_id_arg` widened to also accept the number-only shorthand
/// (§FS-check.1.2). The full grammar is tried first and its result is returned
/// unconditionally when it matches, which is what makes "the full ID always wins"
/// (§DF-number-only-citation-shorthand.2.6) true by construction rather than by a
/// separate check.
fn parse_id_arg_with_shorthand(raw: &str, grammar: &Grammar) -> Result<ParsedId> {
    let full = match parse_id_arg(raw, grammar) {
        Ok((id, section)) => {
            return Ok(ParsedId {
                id,
                section,
                shorthand: false,
            });
        }
        Err(err) => err,
    };
    let Some(shorthand) = &grammar.shorthand else {
        return Err(full);
    };
    // The whole argument must be the shorthand — a trailing tail is not a
    // shorthand with junk after it, it is a token this grammar does not accept.
    let caps = shorthand
        .prefix_re()
        .captures(raw)
        .filter(|caps| caps.get(0).is_some_and(|found| found.end() == raw.len()))
        .ok_or(full)?;
    let id = parse_id(&caps).ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    Ok(ParsedId {
        id,
        section: caps.name("sec").map(|m| m.as_str().to_string()),
        shorthand: true,
    })
}

/// Resolve a CLI `<ID>[.<section>]` argument that may be written in the
/// number-only shorthand (§FS-check.1.2). A query persists nothing, so the
/// shorthand is simply expanded here rather than reported the way a shorthand in
/// a file is (§DF-number-only-citation-shorthand.2.2) — this is what lets a
/// clicked `§FS-042` open in a terminal (§FS-integrations.3.1).
///
/// A shorthand matching several declarations is the same class of query failure
/// as an ambiguous full ID (§FS-show.2.2.1); one matching none keeps its
/// shorthand `Id`, so the caller's own "not found" path reports it as written
/// instead of a second message saying the same thing.
fn resolve_id_arg(
    raw: &str,
    config: &Config,
    findings: &Findings,
) -> std::result::Result<(Id, Option<String>), IdArgError> {
    let parsed =
        parse_id_arg_with_shorthand(raw, &config.grammar).map_err(IdArgError::Unparsable)?;
    if !parsed.shorthand {
        return Ok((parsed.id, parsed.section));
    }
    match shorthand_candidates(&parsed.id, &findings.declarations).as_slice() {
        [unique] => Ok(((*unique).clone(), parsed.section)),
        [] => Ok((parsed.id, parsed.section)),
        many => Err(IdArgError::Ambiguous(anyhow!(
            "ambiguous ID: {} (matches {})",
            render_id(config, &parsed.id),
            many.iter()
                .map(|id| render_id(config, id))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Why an `<ID>` argument could not be turned into one declaration
/// (§FS-show.2.2.1, §FS-refs.4).
///
/// The two cases want different help. `Unparsable` is the classic stumble — an
/// argument shaped for a different repo's `[id] format` — and the format hint is
/// exactly what resolves it. `Ambiguous` means the argument *did* parse and named
/// several declarations; repeating the format there would be advice for a problem
/// the caller does not have, so the candidate list stands alone.
#[derive(Debug)]
enum IdArgError {
    Unparsable(anyhow::Error),
    Ambiguous(anyhow::Error),
}

impl IdArgError {
    fn error(&self) -> &anyhow::Error {
        match self {
            Self::Unparsable(err) | Self::Ambiguous(err) => err,
        }
    }

    /// Whether the caller should follow this with its `[id] format` hint.
    fn wants_format_hint(&self) -> bool {
        matches!(self, Self::Unparsable(_))
    }
}

impl std::fmt::Display for IdArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self.error())
    }
}

/// Every declaration whose kind and number match a shorthand `Id`, in the
/// deterministic `BTreeMap` key order the report needs (§FS-check.3.13). One
/// match is the resolution; zero or several is what the checker reports.
///
/// One-shot lookup, for the sites that answer a single question — a CLI
/// argument, one report line. A pass that asks per citation must build a
/// `ShorthandIndex` instead: this walks every declaration in the project.
fn shorthand_candidates<'a>(
    id: &Id,
    declarations: &'a BTreeMap<Id, Vec<Declaration>>,
) -> Vec<&'a Id> {
    declarations.keys().filter(|declared| shorthand_names(declared, id)).collect()
}

/// Whether `declared` is a declaration the shorthand `id` could name: same kind,
/// same number, and a slug to stand in for (§FS-check.1.2). The slug test is what
/// keeps a partially-parsed `Id` out of its own candidate set.
fn shorthand_names(declared: &Id, id: &Id) -> bool {
    declared.kind == id.kind && declared.num == id.num && declared.slug.is_some()
}

/// Declarations grouped by the `(kind, number)` pair a shorthand names
/// (§FS-check.1.2), so a pass that resolves many sites pays one walk of the
/// declaration set instead of one per site.
///
/// Without it `check` and `fmt --write` are O(sites × declarations), which is
/// quadratic in exactly the case this rule exists to serve: a repository full of
/// shorthands being migrated to canonical form. Measured on a synthetic tree at
/// 8k declarations and 8k shorthand citations, the linear-scan form spent 3.6s in
/// `check` where a canonical tree spent 0.11s (§GOAL-fast-feedback).
struct ShorthandIndex<'a> {
    by_number: BTreeMap<(&'a str, Option<u32>), Vec<&'a Id>>,
}

impl<'a> ShorthandIndex<'a> {
    fn build(declarations: impl IntoIterator<Item = &'a Id>) -> Self {
        let mut by_number: BTreeMap<(&'a str, Option<u32>), Vec<&'a Id>> = BTreeMap::new();
        for declared in declarations {
            if declared.slug.is_none() {
                continue;
            }
            by_number
                .entry((declared.kind.as_str(), declared.num))
                .or_default()
                .push(declared);
        }
        Self { by_number }
    }

    /// The declarations `id` could name, in `BTreeMap` key order — the same order
    /// `shorthand_candidates` produces, so the report and the resolution agree.
    fn candidates<'b>(&'b self, id: &'b Id) -> &'b [&'a Id] {
        self.by_number
            .get(&(id.kind.as_str(), id.num))
            .map_or(&[][..], |found| found.as_slice())
    }

    /// The single declaration `id` names, or `None` when zero or several match —
    /// the only outcome that resolves (§DF-number-only-citation-shorthand.2.7).
    fn unique(&self, id: &Id) -> Option<&'a Id> {
        match self.candidates(id) {
            [unique] => Some(unique),
            _ => None,
        }
    }
}

/// Everything one `grund fmt` walk needs to expand a shorthand: this project's
/// declaration index, plus one per workspace alias for the qualified form
/// (§FS-fmt.2.4, §FS-workspace.8.5).
///
/// Built once per walk rather than per line. `fmt` visits every marker of every
/// scanned file, so resolving each against a linear scan of the declaration set
/// is quadratic on exactly the tree this rewrite exists to clean up
/// (§GOAL-fast-feedback).
struct ShorthandTargets<'a> {
    /// `None` until the walk has a declaration set — §FS-fmt.2.4 defers that scan
    /// until a shorthand is actually met, so a repo without one never pays for it.
    local: Option<ShorthandIndex<'a>>,
    by_alias: BTreeMap<&'a str, ShorthandAliasTarget<'a>>,
}

/// One aliased project's half of `ShorthandTargets`: its declarations, and the
/// config the canonical ID renders under (a workspace may mix `[id] format`s).
struct ShorthandAliasTarget<'a> {
    config: &'a Config,
    index: ShorthandIndex<'a>,
}

impl<'a> ShorthandTargets<'a> {
    fn new(findings: Option<&'a Findings>, workspace: Option<&'a WorkspaceContext>) -> Self {
        Self {
            local: findings.map(|found| ShorthandIndex::build(found.declarations.keys())),
            by_alias: workspace
                .map(|workspace| {
                    workspace
                        .projects
                        .iter()
                        .map(|project| {
                            (
                                project.alias.as_str(),
                                ShorthandAliasTarget {
                                    config: &project.config,
                                    index: ShorthandIndex::build(
                                        project.findings.declarations.keys(),
                                    ),
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// §AR-scanner.2.6: collect number-only shorthand citations — `§FS-042` for
/// `§FS-042-user-login` — under a `[id] format` that carries both `{number}` and
/// `{slug}`. Three gates in order, each cheap enough to run per line: the repo
/// must have a shorthand at all, the line must contain the marker, and the token
/// must not already be claimed by the full-ID pass (§DF-number-only-citation-shorthand.2.6).
///
/// The marker is required unconditionally — there is no bare branch even under
/// `strict = false`, because `KIND-NNN` carries no slug to make an accidental
/// match unlikely and occurs constantly as issue keys and part numbers
/// (§DF-number-only-citation-shorthand.2.4).
///
/// Testing `claimed_markers` before the regex is also what keeps the pass cheap on
/// a well-formed tree, where every marker is claimed and no shorthand pattern is
/// ever run.
///
/// A qualified marker belongs to the pass that claimed it — the workspace one,
/// which claims every `§<alias>/...` on the line, or the loose fallback outside it,
/// which records each token it parsed. Without the record the shorthand pattern
/// matched the same token a second time and it became two identical citations: a
/// duplicated row in `cover` and a diagnostic `check` printed twice. Skipping
/// unconditionally instead would delete the citation wherever the loose parser
/// declines a shape this project's `[id] format` accepts. The qualified form also
/// collides with a path, so a marked qualified token inside inline code or a string
/// literal is not a citation at all — the same carve-out the other passes apply.
///
/// Qualified `§<alias>/FS-042` is left to the workspace pass, which parses the ID
/// tail with the *target* project's grammar — the citing project's shorthand
/// shape would be the wrong one to apply across a namespace boundary. Outside
/// workspace mode there is no such pass to defer to unconditionally, so the
/// deferral is by record: `qualified_claimed` holds the markers a qualified pass
/// actually emitted at, and only those are skipped.
fn scan_shorthand_citations(
    line: &CitationLine<'_>,
    workspace_mode: bool,
    claimed_markers: &[usize],
    qualified_claimed: &BTreeSet<usize>,
    findings: &mut Findings,
) {
    let Some(shorthand) = &line.config.grammar.shorthand else {
        return;
    };
    if line.config.marker.is_empty() {
        return;
    }
    for (marker_start, _) in line.scan_line.match_indices(&line.config.marker) {
        // §DF-number-only-citation-shorthand.2.6: the full-ID pass owns every token
        // it can claim, and `claimed_markers` is the record of what it claimed on
        // this line — tested before the regex (§GOAL-fast-feedback).
        if claimed_markers.contains(&marker_start) {
            continue;
        }
        let token_start = marker_start + line.config.marker.len();
        let Some(rest) = line.scan_line.get(token_start..) else {
            continue;
        };
        let Some(caps) = shorthand.prefix_re().captures(rest) else {
            continue;
        };
        let match_end = caps.get(0).map_or(0, |found| found.end());
        // §DF-number-only-citation-shorthand.2.6: the pattern is anchored only at
        // the start, so without this the `FS-042` inside the rejected full ID
        // `§FS-042-User-Login` would be reported as a token the file does not hold.
        if !line.config.grammar.id_token_ends_cleanly(rest, match_end) {
            continue;
        }
        // §FS-fmt.2.4.1: the token ended, which does not make it a citation.
        let numeric_run = line.config.grammar.shorthand_sits_in_numeric_run(
            &line.config.marker,
            rest,
            match_end,
        );
        // §AR-scanner.2.6: a qualified marker a qualified pass already claimed —
        // the workspace one, or the loose fallback (§FS-workspace.5) — belongs to
        // that pass alone (§REQ-no-missed-citation.1, §AR-scanner.2.3).
        let namespace = caps.name("namespace").map(|m| m.as_str().to_string());
        if namespace.is_some()
            && (workspace_mode
                || qualified_claimed.contains(&marker_start)
                || qualified_suppressed_in_source(line.scan_line, line.is_md, marker_start))
        {
            continue;
        }
        let Some(id) = parse_id(&caps) else { continue };
        let token_end = token_start + match_end;
        findings.citations.push(Citation {
            namespace,
            id,
            section: caps.name("sec").map(|m| m.as_str().to_string()),
            file: line.path.to_path_buf(),
            line: line.lineno,
            column: line.column_offset + marker_start + 1,
            has_marker: true,
            shorthand: true,
            // §FS-check.3.13: still a citation here — it resolves, it counts, it
            // grounds its file — but `fmt` may not rewrite it (§FS-fmt.2.3), so the
            // checker withholds the "write the canonical form" error.
            shorthand_rewritable: scanned_citation_rewritable(line, marker_start),
            numeric_run,
            text: line.scan_line[marker_start..token_end].to_string(),
            inline_site: line.inline_sites.get(&line.lineno).cloned(),
            // §AR-scanner.2.4: classified in the post-pass in `scan_file`.
            source_kind: String::new(),
            enclosing_declaration: None,
        });
    }
}

/// §AR-scanner.2.6: rewrite each shorthand citation's `Id` to the declaration it
/// names, once the whole project's declarations are known. This is the step that
/// makes a resolved shorthand invisible to everything downstream — the checker,
/// `refs`, `cover`, the unused warning, and the LSP snapshot all read a canonical
/// `Id` and never learn the shorthand existed.
///
/// Zero or several matches leave `slug: None`, which is exactly the state
/// §FS-check.3.13 reports as unknown or ambiguous.
///
/// The escaped citations are resolved too. Without that, `<§>FS-042` escaping a
/// real declaration is silently exempt from a check that catches
/// `<§>FS-042-user-login`.
fn resolve_shorthand_citations(findings: &mut Findings) {
    let pending = |citations: &[Citation]| {
        citations
            .iter()
            .any(|cite| cite.shorthand && cite.namespace.is_none())
    };
    if !pending(&findings.citations) && !pending(&findings.escaped_citations) {
        return;
    }
    // Snapshot the declaration keys first: the loop below mutates `citations`
    // while it reads `declarations`, and only local (unqualified) shorthands can
    // resolve here — a qualified one is resolved against its own namespace by
    // the workspace checker.
    let declared: Vec<Id> = findings.declarations.keys().cloned().collect();
    let index = ShorthandIndex::build(declared.iter());
    // §FS-check.2.3.1: an escape earns its "this resolves — did you mean it to be
    // live?" suggestion only by carrying an `Id` that is actually declared, and a
    // shorthand's `Id` never is until it is rewritten here.
    for cite in findings
        .citations
        .iter_mut()
        .chain(findings.escaped_citations.iter_mut())
    {
        if !cite.shorthand || cite.namespace.is_some() || cite.id.slug.is_some() {
            continue;
        }
        if let Some(unique) = index.unique(&cite.id) {
            cite.id = unique.clone();
        }
    }
}

/// §FS-check.3.13 / §AR-checker.2.12: the one finding a number-only shorthand
/// site earns. The candidate set is re-derived here rather than read off the
/// citation, so the message is right whether or not the scanner's resolution
/// pass has run — a synthetic `Findings` fed straight to the checker gets the
/// same three shapes.
///
/// The marker comes from the *citing* project (it is what the author types) while
/// the ID renders under the *target* project's `[id] format`, so the replacement
/// in a mixed-format workspace is pasteable as printed.
///
/// `None` for a resolving shorthand at a site `fmt` may not rewrite
/// (§FS-fmt.2.3): the citation is real and counts everywhere, but the only fix
/// this error knows how to name is one the formatter declines to apply, so
/// reporting it would leave a repository permanently red with nothing to run.
/// A shorthand that resolves to zero or several declarations is still reported
/// there — that is a dangling reference, not a formatting nit. The out-of-scope
/// tier of `check --full` withholds it for the same reason: `fmt` scopes by
/// `[scan] include` too (§FS-check.3.14).
fn shorthand_diagnostic(
    config: &Config,
    cite: &Citation,
    target_config: &Config,
    tier: ReferenceTier,
    candidates: &[&Id],
) -> Option<Diagnostic> {
    let written = cite.text.trim();
    let message = match candidates {
        [] => format!("shorthand citation {written} matches no declaration"),
        [unique] => {
            // §FS-check.3.14: outside the configured scope `fmt` will not rewrite
            // the site either, so the same withholding applies for the same reason.
            if !cite.shorthand_rewritable || tier == ReferenceTier::OutOfScope {
                return None;
            }
            let section = cite
                .section
                .as_ref()
                .map(|section| format!("{}{}", target_config.section_separator, section))
                .unwrap_or_default();
            let canonical = format!(
                "{}{}{}",
                config.marker,
                render_qualified_id(target_config, cite.namespace.as_deref(), unique),
                section
            );
            // §FS-check.3.15: the same site, a different verdict — `fmt` will not
            // rewrite a numeral in a run, so naming only the canonical form would
            // advise the edit that corrupts the line. Both exits; the author picks.
            if cite.numeric_run {
                return Some(Diagnostic {
                    code: "shorthand-numeric-run",
                    path: Some(cite.file.clone()),
                    line: Some(cite.line),
                    column: Some(cite.column),
                    message: format!(
                        "shorthand {written} sits in a numeric run and was not rewritten; \
                         write {canonical}, or <{}>{} if these are old numbers",
                        config.marker,
                        written
                            .strip_prefix(config.marker.as_str())
                            .unwrap_or(written),
                    ),
                    sites: Vec::new(),
                });
            }
            format!("shorthand citation {written}; write {canonical}")
        }
        many => format!(
            "shorthand citation {written} is ambiguous: {}",
            many.iter()
                .map(|id| render_id(target_config, id))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    Some(Diagnostic {
        code: "shorthand-citation",
        path: Some(cite.file.clone()),
        line: Some(cite.line),
        column: Some(cite.column),
        message,
        sites: Vec::new(),
    })
}

/// Declaration indexes for the checker's shorthand pass, keyed by the citation's
/// target namespace and populated on first use — so a tree without shorthands
/// builds none (§FS-check.3.13).
type ShorthandIndexes<'a> = BTreeMap<Option<String>, ShorthandIndex<'a>>;

/// §FS-check.3.13 / §AR-checker.2.12: report one citation's shorthand finding, if
/// it earns one. Returns `true` when the citation resolved to nothing and the
/// caller should skip its remaining rules — §3.1 in particular, which would
/// otherwise name a token that is not a full ID.
///
/// "Resolved" is read off the candidate set this check can see, not off the
/// scanner's rewrite: under `--full` the scan resolved every shorthand against
/// the whole walk and the findings were then narrowed back to `[scan] include`
/// (§FS-check.1.3), so a site can arrive here holding a canonical ID whose
/// declaration is no longer in the target's set. Judging it by the rewrite alone
/// would earn that site a shorthand error *and* a dangling error for one cause,
/// which §FS-check.3.13 says never happens. The qualified cross-member form is
/// resolved in a workspace pass of its own, so this is the only place both forms
/// meet.
///
/// The index is built per target namespace rather than per site: re-deriving the
/// candidate set by walking every declaration each time is quadratic on a tree
/// mid-migration, which is precisely the tree this rule asks people to run
/// (§GOAL-fast-feedback).
fn report_shorthand_citation<'a>(
    cite: &Citation,
    config: &Config,
    target: &WorkspaceCheckTarget<'a>,
    tier: ReferenceTier,
    indexes: &mut ShorthandIndexes<'a>,
    report: &mut CheckReport,
) -> bool {
    let index = indexes
        .entry(cite.namespace.clone())
        .or_insert_with(|| ShorthandIndex::build(target.findings.declarations.keys()));
    let candidates = index.candidates(&cite.id);
    let resolved = cite.id.slug.is_some() && candidates.len() == 1;
    if let Some(diagnostic) = shorthand_diagnostic(config, cite, target.config, tier, candidates) {
        report.errors.push(diagnostic);
    }
    !resolved
}

/// §FS-fmt.2.4: expand every number-only shorthand citation on the line that
/// resolves to exactly one declaration. An ambiguous or unknown shorthand is left
/// exactly as written — `fmt` normalizes, it does not guess, and §FS-check.3.13
/// is where those two outcomes are reported.
///
/// Qualified `§<alias>/FS-042` resolves against the aliased project's
/// declarations, which is why `workspace` is threaded in: §FS-fmt.2.4 promises the
/// rewrite preserves the `<alias>/` namespace, and a rule that reported the site
/// but never fixed it would leave the §FS-check.3.13 error with no bulk remedy.
/// A member-local run carries no workspace context and leaves the citation alone
/// — there the alias resolves nowhere and `check` says so instead
/// (§FS-workspace.8.5).
///
/// Whose grammar parses a token: the scanner routes qualified citations to the
/// target project's grammar (`scan_workspace_qualified_pass`) and this pass has to
/// agree with it byte for byte — matching the tail with the citing project's
/// shorthand instead would rewrite tokens `check` never saw and skip the ones it
/// reported, in a workspace that mixes `[id] format`s. A project alias is
/// lower-case-initial and a kind is not, which is why one byte can skip the
/// qualified pattern for essentially every citation in a real tree; `fmt` runs this
/// per marker of every scanned line.
///
/// Why the gates are in this order: testing a claimable token with the *anchored*
/// full-ID pattern rather than an unanchored search is what keeps `fmt --check`
/// cheap — a canonical citation, nearly every marker in a real tree, never reaches
/// the shorthand pattern at all. That pattern is anchored only at its start, so
/// `§FS-042-User-Login`, a full ID whose slug this grammar rejects, matches on its
/// `FS-042` prefix; rewriting it would splice the canonical slug into the middle of
/// the author's token, leave the tail glued on, and silently corrupt the file. And
/// declarations are reached for last because `fmt --check` has no other reason to
/// scan: the walk starts without them, this pass reports the first candidate it
/// actually meets, the caller scans then and re-runs the single file, and a repo
/// that never writes a shorthand pays nothing at all. The expansion report is what
/// makes a rewrite reviewable before it is written.
fn expand_shorthand_citations(
    line: &str,
    docstring: DocstringContent<'_>,
    config: &Config,
    is_md: bool,
    targets: &ShorthandTargets<'_>,
    saw_candidate: &mut bool,
    expansions: &mut Vec<(String, String)>,
) -> Option<String> {
    // The local grammar is only one of the grammars in play: a qualified citation
    // is parsed with the *target's*, so a citing project with no shorthand of its
    // own can still hold one that needs expanding.
    if config.marker.is_empty()
        || !line.contains(&config.marker)
        || (!config.grammar.has_shorthand() && targets.by_alias.is_empty())
    {
        return None;
    }
    let mut output = String::new();
    let mut cursor = 0;
    // Driven from marker positions, like the scan pass and for the same reason: the
    // shorthand prefixes every full ID under the default format, so a line sweep
    // costs a candidate and a rejection per citation (§AR-scanner.2.6, §GOAL-fast-feedback).
    for (marker_start, _) in line.match_indices(&config.marker) {
        if marker_start < cursor {
            continue;
        }
        let token_start = marker_start + config.marker.len();
        let Some(rest) = line.get(token_start..) else {
            continue;
        };
        // §FS-workspace.1: an `<alias>/` prefix decides *whose* grammar parses the
        // rest of the token, and a lower-case initial is the one byte that skips the
        // qualified pattern for nearly every citation (§GOAL-fast-feedback).
        let alias = rest
            .starts_with(|ch: char| ch.is_ascii_lowercase())
            .then(|| {
                QUALIFIED_CITATION_PREFIX
                    .captures(rest)
                    .and_then(|caps| Some((caps.name("namespace")?.as_str(), caps.get(0)?.end())))
            })
            .flatten();
        let target = match alias {
            Some((alias, _)) => match targets.by_alias.get(alias) {
                Some(target) => Some(target),
                None => continue,
            },
            None => None,
        };
        let target_config = target.map_or(config, |target| target.config);
        let alias_len = alias.map_or(0, |(_, len)| len);
        let tail = &rest[alias_len..];
        let Some(shorthand) = target_config.grammar.shorthand.as_ref() else {
            continue;
        };
        // §DF-number-only-citation-shorthand.2.6: a token the full-ID pattern can
        // claim is already canonical and must not be touched; anchored, the test
        // costs one match bounded by the token (§GOAL-fast-feedback).
        if shorthand.full_prefix_re().is_match(tail) {
            continue;
        }
        let Some(caps) = shorthand.prefix_re().captures(tail) else {
            continue;
        };
        let match_end = caps.get(0).map_or(0, |found| found.end());
        // §DF-number-only-citation-shorthand.2.6: the pattern is anchored only at
        // the start, so rewriting `§FS-042-User-Login` on its `FS-042` prefix would
        // corrupt the file — see the gate order above.
        if !target_config.grammar.id_token_ends_cleanly(tail, match_end) {
            continue;
        }
        // §FS-fmt.2.4.1: `§SPEC-001→SPEC-003` is a renumbering table, not a citation.
        // The marker is the *citing* project's — what the author typed — while the
        // number shape is the target's, the same split the rewrite below uses.
        if target_config
            .grammar
            .shorthand_sits_in_numeric_run(&config.marker, tail, match_end)
        {
            continue;
        }
        // §FS-fmt.2.3: the same exclusions the other rewrites honour — inline code,
        // a link destination, a runtime string — asked of the docstring's content on
        // a docstring line, exactly as the scanner asks it (§FS-fmt.2.3.1).
        if never_rewrite_context_in(docstring, line, is_md, marker_start) {
            continue;
        }
        let Some(id) = parse_id(&caps) else { continue };
        // §FS-fmt.2.4: only *now* are declarations needed — every gate above rejects
        // on the line text alone, and reaching for them earlier was a measured 79%
        // regression on the benchmark fixture (§GOAL-fast-feedback, §AR-ci.5).
        let index = match target {
            Some(target) => &target.index,
            None => match targets.local.as_ref() {
                Some(index) => index,
                None => {
                    *saw_candidate = true;
                    continue;
                }
            },
        };
        let Some(unique) = index.unique(&id) else {
            continue;
        };
        let namespace = alias.map(|(alias, _)| alias);
        let match_end = alias_len + match_end;
        output.push_str(&line[cursor..token_start]);
        // §FS-fmt.3: the written and canonical forms are recorded as the line is
        // built, because this is the only point that holds both — and expanding is
        // the one rewrite here whose mistakes no later pass can see.
        let written_start = output.len();
        if let Some(alias) = namespace {
            output.push_str(alias);
            output.push('/');
        }
        // The ID renders under the *target* project's `[id] format`, which is what
        // makes the rewrite correct in a mixed-format workspace.
        output.push_str(&render_id(target_config, unique));
        if let Some(section) = caps.name("sec") {
            // The target's separator, matching the form §FS-check.3.13 names —
            // the section belongs to the target's ID, not the citing project's.
            output.push_str(&target_config.section_separator);
            output.push_str(section.as_str());
        }
        expansions.push((
            line[marker_start..token_start + match_end].to_string(),
            format!("{}{}", config.marker, &output[written_start..]),
        ));
        cursor = token_start + match_end;
    }
    // `cursor` moves only when something was rewritten, so this is the
    // "unchanged" signal — and returning `None` lets the caller move the
    // original line through instead of allocating a copy of it.
    if cursor == 0 {
        return None;
    }
    output.push_str(&line[cursor..]);
    Some(output)
}

/// §AR-scanner.2.6: resolve `§<alias>/FS-042` against the aliased project's
/// declarations. A per-project scan cannot do this — it sees only its own
/// declaration set — so the cross-namespace half of the shorthand rule lands
/// here, once every project has been scanned. Unqualified shorthands were
/// already resolved inside each project's own walk.
fn resolve_qualified_shorthand_citations(projects: &mut [WorkspaceProject]) {
    let pending = |project: &WorkspaceProject| {
        project.findings.citations.iter().any(|cite| {
            cite.shorthand && cite.namespace.is_some() && cite.id.slug.is_none()
        })
    };
    if !projects.iter().any(pending) {
        return;
    }
    let declared: BTreeMap<String, Vec<Id>> = projects
        .iter()
        .map(|project| {
            (
                project.alias.clone(),
                project.findings.declarations.keys().cloned().collect(),
            )
        })
        .collect();
    let indexes: BTreeMap<&str, ShorthandIndex<'_>> = declared
        .iter()
        .map(|(alias, ids)| (alias.as_str(), ShorthandIndex::build(ids.iter())))
        .collect();
    for project in projects.iter_mut() {
        for cite in &mut project.findings.citations {
            if !cite.shorthand || cite.id.slug.is_some() {
                continue;
            }
            let Some(index) = cite
                .namespace
                .as_deref()
                .and_then(|alias| indexes.get(alias))
            else {
                continue;
            };
            if let Some(unique) = index.unique(&cite.id) {
                cite.id = unique.clone();
            }
        }
    }
}

/// The canonical ID a typed shorthand token expands to, or `None` when the token
/// is not a shorthand or names other than exactly one declared ID
/// (§FS-lsp.1.4). Candidates are matched by re-parsing each declared ID under
/// the same grammar, so the editor and `grund fmt` agree on what resolves
/// (§FS-fmt.2.4).
fn shorthand_token_expansion(
    config: &Config,
    token: &str,
    declared_ids: &[&str],
) -> Option<String> {
    if !config.grammar.has_shorthand() {
        return None;
    }
    let parsed = parse_id_arg_with_shorthand(token, &config.grammar).ok()?;
    if !parsed.shorthand {
        return None;
    }
    let mut matches = declared_ids.iter().filter(|declared| {
        parse_id_arg(declared, &config.grammar).is_ok_and(|(id, section)| {
            section.is_none() && shorthand_names(&id, &parsed.id)
        })
    });
    let unique = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let section = parsed
        .section
        .map(|section| format!("{}{}", config.section_separator, section))
        .unwrap_or_default();
    Some(format!("{unique}{section}"))
}
