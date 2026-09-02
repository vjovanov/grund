/// The `### Citation directions` renderer of the managed block (§FS-init.2.3.5),
/// split out of `init_templates.rs` so the payload constants and the renderer
/// that composes them stop sharing a file (§AR-core-module-layout.1).
///
/// The section is what an agent reads *instead of* `grund.toml`, so every line
/// here is held to the rule it states: a bullet names the unit the rule is
/// checked per, a conjunction of alternatives is grouped so it has one reading,
/// rule grammar never reaches the prose, and the levels that gate are told apart
/// from the levels that suggest. The wording is pinned by
/// §DF-directions-render against the canonical config that exercises every
/// branch.
///
/// Rendering stays deterministic — `[[kinds]]` order, the homeless kind last,
/// fixed phrases — because `grund check` re-renders it and byte-compares for
/// drift (§FS-check.3.5): the render *is* the hash.

/// The legend (§FS-init.2.3.5): which of the five levels are `grund check`
/// errors and which are suggestions. Fixed text, rendered whenever `[citations]`
/// is declared, because a bullet's verb is unreadable without it — an agent that
/// cannot tell `should` from `must` treats every rule as one or the other.
const CITATION_LEVEL_LEGEND: &str = "`must`/`never` are `grund check` errors; `should`/`avoid` are suggestions (`grund check --suggestions`).";

/// Render the `### Citation directions` managed-block section (§FS-init.2.3.5)
/// from the effective config. When no `[citations]` section is declared, the
/// static climbing-rule sentence stands in, so a config that predates the
/// feature keeps a stable block (§FS-init.2.3.4.10); the grounding sentence
/// renders under either, because it is generated from `[reference]
/// require_grounding` and not from the direction rules (§FS-check.3.6).
fn citation_directions_section(config: &Config) -> String {
    // Built as lines joined with `\n` and returned without a trailing newline:
    // the `{CITATION_DIRECTIONS}` placeholder in the template supplies the single
    // block-final newline, so `grund init` stays idempotent on re-run.
    let mut lines = vec!["### Citation directions".to_string(), String::new()];
    let grounding = citation_grounding_sentence(config);
    if !config.citations.declared {
        lines.push(join_sentences(
            &format!(
                "Specs cite goals, architecture cites specs, code and executable tests cite the specs they realize. In a citation rule array, entries are all required; `|` inside one entry means any one alternative. See {CITATION_DIRECTIONS_URL} for the levels and examples."
            ),
            grounding.as_deref(),
        ));
        return lines.join("\n");
    }
    lines.push(join_sentences(CITATION_LEVEL_LEGEND, grounding.as_deref()));
    lines.push(String::new());
    // `[[kinds]]` order, then the homeless kind last (§FS-init.2.3.5) — wherever
    // in the table a project happened to declare it, because it is the
    // complement of every row above it and reads as the closing case.
    let homeless = config.homeless_kind();
    let mut kinds: Vec<&str> = config
        .kinds
        .iter()
        .map(|kind| kind.kind.as_str())
        .filter(|kind| *kind != homeless)
        .collect();
    if config.citations.per_kind.contains_key(homeless) {
        kinds.push(homeless);
    }
    for kind in kinds {
        let Some(rules) = config.citations.per_kind.get(kind) else {
            continue;
        };
        let Some(clauses) = citation_direction_clauses(config, rules) else {
            continue;
        };
        lines.push(format!(
            "- {} {clauses}.",
            citation_direction_subject(config, kind, homeless)
        ));
    }
    // Load-bearing (§FS-init.2.3.5): silence is open only when the global default
    // leaves it open, and a per-kind default never reaches it — that one is
    // folded into its own bullet, which is *listed above*.
    lines.push(citation_closing_line(config.citations.global_default));
    lines.join("\n")
}

/// Two sentences of one paragraph, space-joined, with the second optional.
fn join_sentences(first: &str, second: Option<&str>) -> String {
    match second {
        Some(second) => format!("{first} {second}"),
        None => first.to_string(),
    }
}

/// The grounding sentence (§FS-init.2.3.5), generated from `[reference]
/// require_grounding` and the configured non-citable homes. It claims only what
/// §FS-check.3.6 enforces, and it distinguishes *cite* from *declare*: a source
/// file grounds by citing a declared ID **or** by declaring one inline, while a
/// file in a non-citable home can only cite, because a declaration there is
/// misplaced (§FS-check.3.7). An unwalked home (§FS-config.3.4.7) is left out —
/// nothing in it is scanned, so the rule never reaches it. Per-row grounding
/// levels (§FS-config.3.4.8) are not this sentence's.
fn citation_grounding_sentence(config: &Config) -> Option<String> {
    if !config.require_grounding {
        return None;
    }
    let base = "Every source file must cite a declared ID or declare one inline";
    let homeless = config.homeless_kind();
    let homes = config
        .kinds
        .iter()
        .filter(|kind| !kind.citable && kind.scan && kind.kind != homeless)
        .filter_map(KindConfig::place_label)
        .collect::<Vec<_>>();
    if homes.is_empty() {
        return Some(format!("{base}."));
    }
    Some(format!(
        "{base}; every file under {} must cite one.",
        join_english_list(&homes, "and")
    ))
}

/// What one bullet's rules are checked *per* (§FS-init.2.3.5) — the unit the
/// ticket's first defect was that no bullet stated.
///
/// A citable kind's unit is the top-level declaration (§FS-check.3.11). A
/// non-citable kind's is every scanned file in its home, so the bullet names the
/// place, as the Project map does (§2.3.4.4) — naming the kind would name
/// something an agent can never write. The homeless kind has no place, so it
/// keeps its name and says what it covers: its `title` where the project wrote
/// one. Its unit is narrower again — a *source* file (not `.md`) that already
/// cites something (§FS-config.3.9.2) — because the obligation constrains what a
/// file cites and never whether it cites at all; a util that cites nothing is
/// not a unit. In a non-citable home the opposite holds — a skill without a spec
/// is the defect — so nothing narrows that subject, and `require_grounding`
/// closes the hole.
fn citation_direction_subject(config: &Config, kind: &str, homeless: &str) -> String {
    if kind == homeless {
        let scope = config
            .kinds
            .iter()
            .find(|configured| configured.kind == kind)
            .and_then(|configured| configured.title.as_deref())
            .map_or_else(String::new, |title| format!(": {title}"));
        return format!("Each source file outside the Project map (**{kind}**{scope}) that cites anything");
    }
    let configured = config
        .kinds
        .iter()
        .find(|configured| configured.kind == kind && !configured.citable);
    match configured {
        Some(place) if place.folder.is_some() => {
            format!("Each file in **{}**", place.place_label().unwrap_or_default())
        }
        // A single-file non-citable home is one file, so "each file in" would
        // promise a directory that is not there.
        Some(place) => format!("The file **{}**", place.place_label().unwrap_or_default()),
        None => format!("Each **{kind}** declaration"),
    }
}

/// The verb-phrase clauses for one citing kind's rules, joined by "; " in the
/// order obligations, permissions, prohibitions, then the default
/// (§FS-init.2.3.5). `None` when the kind has no renderable rule.
///
/// A prohibition leads with the modal (`must not cite`, the wording
/// §FS-check.3.12 already uses in its findings) and follows with the short form
/// (`never cite`) the legend names: the subject is a noun phrase, so a bullet
/// opening on a bare `never cite` is not a sentence.
fn citation_direction_clauses(config: &Config, rules: &KindCitationRules) -> Option<String> {
    let folded = citation_permission_is_closed(rules);
    let mut clauses = Vec::new();
    if !rules.must.is_empty() {
        clauses.push(format!("must cite {}", citation_rule_targets(&rules.must)));
    }
    if !rules.should.is_empty() {
        clauses.push(format!("should cite {}", citation_rule_targets(&rules.should)));
    }
    if !rules.may.is_empty() {
        // §FS-init.2.3.5: a closed per-kind default plus a `may` list is one
        // rule — "only these" — and takes one clause, not a permission followed
        // by a prohibition of everything else.
        let only = if folded { "only " } else { "" };
        clauses.push(format!("may cite {only}{}", citation_rule_targets(&rules.may)));
    }
    if !rules.must_not.is_empty() {
        let verb = if clauses.is_empty() { "must not cite" } else { "never cite" };
        clauses.push(format!("{verb} {}", citation_rule_targets(&rules.must_not)));
    }
    if !rules.should_not.is_empty() {
        let verb = if clauses.is_empty() { "should not cite" } else { "avoid citing" };
        clauses.push(format!("{verb} {}", citation_rule_targets(&rules.should_not)));
    }
    if !folded && let Some(clause) = citation_default_clause(config, rules, clauses.is_empty()) {
        clauses.push(clause);
    }
    if clauses.is_empty() {
        return None;
    }
    Some(clauses.join("; "))
}

/// Whether this kind's `may` list is the whole of what it permits, so a closed
/// per-kind default folds into it as `may cite only …` (§FS-init.2.3.5). With a
/// `must` or a `should` beside it the permitted set is wider than the `may`
/// list, and "only" would name the wrong set — those bullets keep the explicit
/// closing clause instead.
fn citation_permission_is_closed(rules: &KindCitationRules) -> bool {
    rules.default == Some(CitationLevel::MustNot)
        && !rules.may.is_empty()
        && rules.must.is_empty()
        && rules.should.is_empty()
}

/// The clause a per-kind `default` adds to its bullet (§FS-init.2.3.5), or
/// `None` when it changes nothing a reader could act on.
///
/// Only `must-not` and `should-not` defaults are load-bearing: §FS-config.3.9.4
/// makes a default of `must` / `should` / `may` invent no obligation and forbid
/// nothing, so those say something only where they punch a hole in a **closed
/// global** default — and then what they say is that this kind is open.
fn citation_default_clause(
    config: &Config,
    rules: &KindCitationRules,
    first: bool,
) -> Option<String> {
    let anything = if first { "anything" } else { "anything else" };
    match rules.default? {
        CitationLevel::MustNot if first => Some(format!("must not cite {anything}")),
        CitationLevel::MustNot => Some(format!("never cite {anything}")),
        CitationLevel::ShouldNot if first => Some(format!("should not cite {anything}")),
        CitationLevel::ShouldNot => Some(format!("avoid citing {anything}")),
        CitationLevel::May | CitationLevel::Must | CitationLevel::Should => {
            citation_default_is_closed(config.citations.global_default)
                .then(|| format!("may cite {anything}"))
        }
    }
}

/// The section's closing line (§FS-init.2.3.5) — load-bearing either way, so an
/// agent neither over-infers prohibitions from silence nor misses a closed
/// world. A global default of `must` or `should` closes as open: it obliges
/// nothing and forbids nothing (§FS-config.3.9.4).
fn citation_closing_line(global_default: Option<CitationLevel>) -> String {
    match global_default {
        Some(CitationLevel::MustNot) => "Any citation not listed above is forbidden.".to_string(),
        Some(CitationLevel::ShouldNot) => {
            "Any citation not listed above is discouraged.".to_string()
        }
        _ => "Anything not listed above is allowed.".to_string(),
    }
}

/// Whether a default level closes the world — the only two that do anything to
/// an unlisted pair (§FS-config.3.9.4).
fn citation_default_is_closed(level: Option<CitationLevel>) -> bool {
    matches!(
        level,
        Some(CitationLevel::MustNot | CitationLevel::ShouldNot)
    )
}

/// Render a rule array as a target phrase (§FS-init.2.3.5): alternatives inside
/// one entry are joined with "or", conjunctive entries with "and". When there is
/// more than one entry, an entry that has alternatives of its own is
/// parenthesised — `must = ["FS|GOAL", "AR"]` is *(FS or GOAL) and AR*, and the
/// ungrouped prose said the opposite.
fn citation_rule_targets(disjunctions: &[CitationDisjunction]) -> String {
    let grouped = disjunctions.len() > 1;
    let entries = disjunctions
        .iter()
        .map(|disjunction| {
            let targets = disjunction
                .targets
                .iter()
                .map(citation_target_phrase)
                .collect::<Vec<_>>();
            let phrase = join_english_list(&targets, "or");
            if grouped && targets.len() > 1 {
                format!("({phrase})")
            } else {
                phrase
            }
        })
        .collect::<Vec<_>>();
    join_english_list(&entries, "and")
}

/// One rule target as prose (§FS-init.2.3.5). A pinned alias stays exactly as
/// spelled, because that is how the citation itself is written; `*/K` is rule
/// grammar that is never a citation (§FS-config.3.9.3), so it is said in words
/// instead of leaked into the entrypoint.
fn citation_target_phrase(target: &CitationTarget) -> String {
    match &target.namespace {
        NamespaceMatch::Local => target.kind.clone(),
        NamespaceMatch::Any => format!("{} in any project", target.kind),
        NamespaceMatch::Alias(alias) => format!("{alias}/{}", target.kind),
    }
}

/// An English list: `A`, `A or B`, `A, B, or C` — the Oxford comma from three
/// items on, so a three-way rule cannot be read as a two-way one (§FS-init.2.3.5).
fn join_english_list(items: &[String], conjunction: &str) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} {conjunction} {second}"),
        [head @ .., last] => format!("{}, {conjunction} {last}", head.join(", ")),
    }
}
