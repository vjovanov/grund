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
/// Everything here is crate-private and reached through the flat `include!` in
/// `lib.rs`; the public embedding surface stays in `api.rs`
/// (§AR-core-module-layout.2).

/// The compiled number-only shorthand: the shape as a *prefix* of the text
/// following a marker, and the same shape as a whole CLI argument
/// (§FS-check.1.2).
///
/// `prefix_re` is `\A`-anchored on purpose, and every caller slices the line at
/// the marker before applying it. The unanchored form is what a citation regex
/// normally looks like, and it is the wrong shape here: under the default format
/// the shorthand `FS-042` is a prefix of *every* full ID `FS-042-user-login`, so
/// an unanchored sweep produces one candidate per citation in the file and each
/// has to be rejected by a second full-ID search. On a 10k-file tree that
/// measured as a 46% instruction-count regression — a direct hit on
/// §GOAL-fast-feedback, which is why the pass is driven from marker positions
/// (§AR-scanner.2.6) rather than from its own scan of the line.
#[derive(Clone)]
struct ShorthandGrammar {
    /// The *full* ID as a prefix of the same slice. Tried first, so a canonical
    /// citation — the overwhelmingly common token after a marker — costs exactly
    /// one anchored match and never reaches the shorthand pattern.
    full_prefix_pattern: String,
    prefix_pattern: String,
    full_prefix_re: once_cell::sync::OnceCell<Regex>,
    prefix_re: once_cell::sync::OnceCell<Regex>,
}

impl ShorthandGrammar {
    /// Compiled on first use, not at config load. Most runs never meet a
    /// shorthand, and on a small tree the fixed cost of compiling a regex is a
    /// visible share of the whole command — `grund show` is ~120M instructions
    /// of which the scan is a small part. Compiling these two eagerly measured
    /// as +3–5% on *every* read command in a numbered repo, for patterns those
    /// commands never ran (§GOAL-fast-feedback, §AR-benchmarks).
    ///
    /// The `expect` cannot fire: both patterns are the already-compiled ID
    /// pattern with one capture group removed and an anchor added, so a repo
    /// whose `citation_re` built successfully has these build successfully too.
    fn prefix_re(&self) -> &Regex {
        self.prefix_re
            .get_or_init(|| Regex::new(&self.prefix_pattern).expect("shorthand pattern compiles"))
    }

    fn full_prefix_re(&self) -> &Regex {
        self.full_prefix_re.get_or_init(|| {
            Regex::new(&self.full_prefix_pattern).expect("full-ID prefix pattern compiles")
        })
    }
}

/// One component of a parsed `[id] format` template (§FS-config.3.2). Parsing the
/// template into elements once gives the regexes and `render_id` a single shared
/// reading of it, which is what lets the shorthand pattern and the shorthand
/// *rendering* be derived by the same reduction instead of two rules that could
/// disagree (§AR-scanner.2.6).
#[derive(Clone, Debug, Eq, PartialEq)]
enum IdElement {
    Literal(String),
    Kind,
    Number,
    Slug,
}

/// Parse an `[id] format` template into its element sequence, rejecting the
/// malformed shapes of §FS-config.3.2.
fn parse_id_format(format: &str) -> Result<Vec<IdElement>> {
    let mut elements = Vec::new();
    let mut cursor = 0;
    while cursor < format.len() {
        let Some(close) = format[cursor..].find('}').map(|offset| cursor + offset) else {
            elements.push(IdElement::Literal(format[cursor..].to_string()));
            break;
        };
        let open = match format[cursor..].find('{').map(|offset| cursor + offset) {
            Some(open) if open < close => open,
            _ => return Err(anyhow!("[id].format: stray `}}` in template")),
        };
        if open > cursor {
            elements.push(IdElement::Literal(format[cursor..open].to_string()));
        }
        elements.push(match &format[open + 1..close] {
            "kind" => IdElement::Kind,
            "number" => IdElement::Number,
            "slug" => IdElement::Slug,
            other => return Err(anyhow!("[id].format: unknown placeholder `{{{other}}}`")),
        });
        cursor = close + 1;
    }

    for (element, name) in [
        (IdElement::Kind, "kind"),
        (IdElement::Number, "number"),
        (IdElement::Slug, "slug"),
    ] {
        if elements.iter().filter(|found| **found == element).count() > 1 {
            return Err(anyhow!("[id].format: {{{name}}} appears twice"));
        }
    }
    if !elements.contains(&IdElement::Kind) {
        return Err(anyhow!("[id].format must contain {{kind}}"));
    }
    if !elements.contains(&IdElement::Number) && !elements.contains(&IdElement::Slug) {
        return Err(anyhow!(
            "[id].format must contain at least one of {{number}} or {{slug}}"
        ));
    }
    Ok(elements)
}

/// The element list with one placeholder and its redundant separator removed:
/// the preceding literal when there is one (`{kind}-{number}-{slug}` minus the
/// slug is `{kind}-{number}`), else the following one (`{kind}-{slug}-{number}`
/// reduces to the same shape). A template that does not carry the placeholder is
/// returned unchanged, so callers can apply this unconditionally.
fn elements_without(elements: &[IdElement], dropped: &IdElement) -> Vec<IdElement> {
    let Some(index) = elements.iter().position(|element| element == dropped) else {
        return elements.to_vec();
    };
    let separator = match index.checked_sub(1) {
        Some(before) if matches!(elements[before], IdElement::Literal(_)) => Some(before),
        _ => (index + 1 < elements.len() && matches!(elements[index + 1], IdElement::Literal(_)))
            .then_some(index + 1),
    };
    elements
        .iter()
        .enumerate()
        .filter(|(position, _)| *position != index && Some(*position) != separator)
        .map(|(_, element)| element.clone())
        .collect()
}

/// The number-only shorthand's element list, or `None` when the format has no
/// shorthand: without `{number}` nothing is left to name the declaration, and
/// without `{slug}` there is nothing to omit (§FS-check.1.2, §FS-id.4.1).
fn shorthand_elements(elements: &[IdElement]) -> Option<Vec<IdElement>> {
    let has_both =
        elements.contains(&IdElement::Number) && elements.contains(&IdElement::Slug);
    has_both.then(|| elements_without(elements, &IdElement::Slug))
}

/// Compile an element list into its regex body, substituting the capture groups
/// the `Id` parser reads back.
fn id_pattern(
    elements: &[IdElement],
    kind_group: &str,
    num_group: &str,
    slug_group: &str,
) -> String {
    let mut pattern = String::new();
    for element in elements {
        match element {
            IdElement::Literal(text) => pattern.push_str(&regex::escape(text)),
            IdElement::Kind => pattern.push_str(kind_group),
            IdElement::Number => pattern.push_str(num_group),
            IdElement::Slug => pattern.push_str(slug_group),
        }
    }
    pattern
}

impl Grammar {
    /// Render an `Id` under the parsed `[id] format`, zero-padding the number to
    /// `width` (§FS-config.3.2, §FS-id.2). A component that is `None` while its
    /// placeholder is in the format — the shorthand `Id` a shorthand citation
    /// carries before it resolves — is dropped with its separator by
    /// `elements_without`, so a partial ID prints as `FS-042` rather than
    /// leaking the raw `{slug}` placeholder into a report (§AR-scanner.2.6).
    fn render(&self, id: &Id, width: usize) -> String {
        // Reduce only when a placeholder the format *carries* has no value —
        // which is the shorthand `Id` and nothing else. A `{kind}-{slug}` repo's
        // `num: None` is not a missing component, it is a component the format
        // never had, so the common path borrows the parsed element list and
        // allocates nothing extra (§GOAL-fast-feedback — `render_id` is on the
        // report and `list` paths).
        let missing = |value_absent: bool, element| {
            value_absent && self.elements.contains(element)
        };
        let reduced = (missing(id.num.is_none(), &IdElement::Number)
            || missing(id.slug.is_none(), &IdElement::Slug))
        .then(|| {
            let mut elements = self.elements.clone();
            if id.num.is_none() {
                elements = elements_without(&elements, &IdElement::Number);
            }
            if id.slug.is_none() {
                elements = elements_without(&elements, &IdElement::Slug);
            }
            elements
        });
        let elements = reduced.as_deref().unwrap_or(&self.elements);
        let mut rendered = String::new();
        for element in elements {
            match element {
                IdElement::Literal(text) => rendered.push_str(text),
                IdElement::Kind => rendered.push_str(&id.kind),
                IdElement::Number => {
                    if let Some(number) = id.num {
                        rendered.push_str(&format!("{number:0width$}"));
                    }
                }
                IdElement::Slug => {
                    if let Some(slug) = &id.slug {
                        rendered.push_str(slug);
                    }
                }
            }
        }
        rendered
    }

    /// Whether this repo's `[id] format` has a number-only shorthand at all
    /// (§FS-check.1.2) — the gate every shorthand pass checks first.
    fn has_shorthand(&self) -> bool {
        self.shorthand.is_some()
    }
}

/// The end offset of the ID-shaped token starting at exactly `at`, or `None` when
/// none does — a full ID, or the number-only shorthand where the repo has one
/// (§FS-check.1.2). One definition of "a real ID follows here", shared by `fmt`'s
/// trigger pass (§FS-fmt.2.1) and the LSP's live transform (§FS-lsp.1.4), so the
/// two cannot disagree about which `$$` to consume.
fn id_token_end_at(line: &str, at: usize, grammar: &Grammar) -> Option<usize> {
    if let Some(found) = grammar
        .citation_re
        .find_at(line, at)
        .filter(|found| found.start() == at)
    {
        return Some(found.end());
    }
    let shorthand = grammar.shorthand.as_ref()?;
    let rest = line.get(at..)?;
    shorthand.prefix_re().find(rest).map(|found| at + found.end())
}

/// One parsed ID token: the `Id`, its optional section path, and whether it was
/// written in the number-only shorthand (§FS-check.1.2). A shorthand `Id` carries
/// `slug: None` until the resolution pass fills it in (§AR-scanner.2.6).
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
) -> Result<(Id, Option<String>)> {
    let parsed = parse_id_arg_with_shorthand(raw, &config.grammar)?;
    if !parsed.shorthand {
        return Ok((parsed.id, parsed.section));
    }
    match shorthand_candidates(&parsed.id, &findings.declarations).as_slice() {
        [unique] => Ok(((*unique).clone(), parsed.section)),
        [] => Ok((parsed.id, parsed.section)),
        many => Err(anyhow!(
            "ambiguous ID: {} (matches {})",
            render_id(config, &parsed.id),
            many.iter()
                .map(|id| render_id(config, id))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Every declaration whose kind and number match a shorthand `Id`, in the
/// deterministic `BTreeMap` key order the report needs (§FS-check.3.13). One
/// match is the resolution; zero or several is what the checker reports.
fn shorthand_candidates<'a>(
    id: &Id,
    declarations: &'a BTreeMap<Id, Vec<Declaration>>,
) -> Vec<&'a Id> {
    declarations
        .keys()
        .filter(|declared| {
            declared.kind == id.kind && declared.num == id.num && declared.slug.is_some()
        })
        .collect()
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
/// Qualified `§<alias>/FS-042` is left to the workspace pass, which parses the ID
/// tail with the *target* project's grammar — the citing project's shorthand
/// shape would be the wrong one to apply across a namespace boundary.
fn scan_shorthand_citations(
    line: &CitationLine<'_>,
    workspace_mode: bool,
    claimed_markers: &[usize],
    findings: &mut Findings,
) {
    let Some(shorthand) = &line.config.grammar.shorthand else {
        return;
    };
    if line.config.marker.is_empty() {
        return;
    }
    for (marker_start, _) in line.scan_line.match_indices(&line.config.marker) {
        // §DF-number-only-citation-shorthand.2.6: the full-ID pass owns every
        // token it can claim, and `claimed_markers` is the record of what it
        // claimed on this line. Checking the set before the regex is also what
        // keeps the pass cheap on a well-formed tree, where every marker is
        // claimed and no shorthand pattern is ever run (§GOAL-fast-feedback).
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
        let namespace = caps.name("namespace").map(|m| m.as_str().to_string());
        if workspace_mode && namespace.is_some() {
            continue;
        }
        let Some(id) = parse_id(&caps) else { continue };
        if namespace.is_some()
            && qualified_suppressed_in_source(line.scan_line, line.is_md, marker_start)
        {
            continue;
        }
        let token_end = token_start + caps.get(0).map(|found| found.end()).unwrap_or(0);
        findings.citations.push(Citation {
            namespace,
            id,
            section: caps.name("sec").map(|m| m.as_str().to_string()),
            file: line.path.to_path_buf(),
            line: line.lineno,
            column: line.column_offset + marker_start + 1,
            has_marker: true,
            shorthand: true,
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
fn resolve_shorthand_citations(findings: &mut Findings) {
    if !findings
        .citations
        .iter()
        .any(|cite| cite.shorthand && cite.namespace.is_none())
    {
        return;
    }
    // Snapshot the declaration keys first: the loop below mutates `citations`
    // while it reads `declarations`, and only local (unqualified) shorthands can
    // resolve here — a qualified one is resolved against its own namespace by
    // the workspace checker.
    let declared: Vec<Id> = findings.declarations.keys().cloned().collect();
    for cite in &mut findings.citations {
        if !cite.shorthand || cite.namespace.is_some() || cite.id.slug.is_some() {
            continue;
        }
        let mut matches = declared
            .iter()
            .filter(|declared| declared.kind == cite.id.kind && declared.num == cite.id.num);
        if let Some(unique) = matches.next()
            && matches.next().is_none()
        {
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
fn shorthand_diagnostic(
    config: &Config,
    cite: &Citation,
    target: &WorkspaceCheckTarget<'_>,
) -> Diagnostic {
    let written = cite.text.trim();
    let candidates = shorthand_candidates(&cite.id, &target.findings.declarations);
    let message = match candidates.as_slice() {
        [] => format!("shorthand citation {written} matches no declaration"),
        [unique] => {
            let section = cite
                .section
                .as_ref()
                .map(|section| format!("{}{}", target.config.section_separator, section))
                .unwrap_or_default();
            format!(
                "shorthand citation {written}; write {}{}{}",
                config.marker,
                render_qualified_id(target.config, cite.namespace.as_deref(), unique),
                section
            )
        }
        many => format!(
            "shorthand citation {written} is ambiguous: {}",
            many.iter()
                .map(|id| render_id(target.config, id))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    Diagnostic {
        code: "shorthand-citation",
        path: Some(cite.file.clone()),
        line: Some(cite.line),
        column: Some(cite.column),
        message,
        sites: Vec::new(),
    }
}

/// §FS-fmt.2.4: expand every number-only shorthand citation on the line that
/// resolves to exactly one declaration. An ambiguous or unknown shorthand is left
/// exactly as written — `fmt` normalizes, it does not guess, and §FS-check.3.13
/// is where those two outcomes are reported.
///
/// Qualified `§<alias>/FS-042` is skipped: resolving it needs the target
/// project's declarations, and `fmt` holds only this project's.
fn expand_shorthand_citations(
    line: &str,
    config: &Config,
    is_md: bool,
    findings: Option<&Findings>,
    saw_candidate: &mut bool,
) -> Option<String> {
    let shorthand = config.grammar.shorthand.as_ref()?;
    if config.marker.is_empty() || !line.contains(&config.marker) {
        return None;
    }
    let mut output = String::new();
    let mut cursor = 0;
    // Driven from marker positions, like the scan pass, and for the same reason:
    // the shorthand is a prefix of every full ID under the default format, so a
    // sweep of the line would produce a candidate per citation and reject each
    // with a second search (§AR-scanner.2.6, §GOAL-fast-feedback).
    for (marker_start, _) in line.match_indices(&config.marker) {
        if marker_start < cursor {
            continue;
        }
        let token_start = marker_start + config.marker.len();
        let Some(rest) = line.get(token_start..) else {
            continue;
        };
        // §DF-number-only-citation-shorthand.2.6: a token the full-ID pattern can
        // claim is already canonical and must not be touched. Testing that with
        // the *anchored* full-ID pattern rather than an unanchored search is what
        // keeps `fmt --check` cheap: a canonical citation — nearly every marker in
        // a real tree — costs one match bounded by the token, and never reaches
        // the shorthand pattern at all (§GOAL-fast-feedback).
        if shorthand.full_prefix_re().is_match(rest) {
            continue;
        }
        let Some(caps) = shorthand.prefix_re().captures(rest) else {
            continue;
        };
        if caps.name("namespace").is_some() {
            continue;
        }
        // §FS-fmt.2.3: the same exclusions the other rewrites honour — an
        // illustration in inline code, a link destination, a runtime string.
        let excluded = if is_md {
            is_inside_inline_code(line, marker_start)
                || is_inside_markdown_link_destination(line, marker_start)
        } else {
            is_inside_string_literal(line, marker_start)
        };
        if excluded {
            continue;
        }
        let Some(id) = parse_id(&caps) else { continue };
        // §FS-fmt.2.4: expanding needs the declaration set, and `fmt --check`
        // has no other reason to scan the tree. Rather than pay for a scan on
        // every run of a numbered repo, the caller starts without one and this
        // pass reports the first candidate it sees; the caller scans then and
        // re-runs the single file (§GOAL-fast-feedback). A repo that never
        // writes a shorthand therefore pays nothing at all.
        let Some(findings) = findings else {
            *saw_candidate = true;
            continue;
        };
        let candidates = shorthand_candidates(&id, &findings.declarations);
        let [unique] = candidates.as_slice() else {
            continue;
        };
        output.push_str(&line[cursor..token_start]);
        output.push_str(&render_id(config, unique));
        if let Some(section) = caps.name("sec") {
            output.push_str(&config.section_separator);
            output.push_str(section.as_str());
        }
        cursor = token_start + caps.get(0).map(|found| found.end()).unwrap_or(0);
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
    for project in projects.iter_mut() {
        for cite in &mut project.findings.citations {
            if !cite.shorthand || cite.id.slug.is_some() {
                continue;
            }
            let Some(ids) = cite
                .namespace
                .as_deref()
                .and_then(|alias| declared.get(alias))
            else {
                continue;
            };
            let mut matches = ids
                .iter()
                .filter(|declared| declared.kind == cite.id.kind && declared.num == cite.id.num);
            if let Some(unique) = matches.next()
                && matches.next().is_none()
            {
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
    declared_ids: &[String],
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
            section.is_none() && id.kind == parsed.id.kind && id.num == parsed.id.num
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
