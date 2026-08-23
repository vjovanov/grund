// The `[id] format` template, parsed once into elements (§FS-config.3.2).
//
// Split out of `shorthand.rs`: the shorthand's *shape* is derived from this
// template by dropping a placeholder, but the template itself is grammar, not
// shorthand. `Grammar::build` compiles every pattern from these elements and
// `render_id` prints every ID back through them, so the parsed form is what
// keeps the pattern and the rendering two readings of one template rather than
// two rules that can disagree (§AR-scanner.2.6).
//
// File-level prose, so `//` rather than `///` — see the note in `shorthand.rs`.

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
}

/// The longest prefix of `raw` that parses as an ID under `grammar`, with the
/// byte length consumed. Accepts the number-only shorthand (§FS-check.1.2) at
/// each candidate length, so a qualified `§api/FS-042` is recognized with the
/// *target* project's shorthand shape rather than the citing project's — the
/// longest-first walk keeps a full ID winning over a shorthand prefix of it.
fn parse_longest_id_prefix(raw: &str, grammar: &Grammar) -> Option<ParsedIdPrefix> {
    let search_end = raw
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(raw.len());
    // `char_indices` already yields strictly increasing, unique byte offsets,
    // and `search_end` is strictly greater than the last char-start it can
    // emit — so the chained list is sorted and unique without further work.
    let ends = raw[..search_end]
        .char_indices()
        .map(|(idx, _)| idx)
        .chain(std::iter::once(search_end))
        .filter(|end| *end > 0)
        .collect::<Vec<_>>();
    for end in ends.into_iter().rev() {
        if let Ok(parsed) = parse_id_arg_with_shorthand(&raw[..end], grammar) {
            // §DF-number-only-citation-shorthand.2.6: the walk is longest-first,
            // so a *full* ID has already been preferred by the time a shorthand
            // parse succeeds — which means a shorthand match here is always a
            // proper prefix of something longer. It only counts if what follows
            // cannot continue the token; otherwise `§api/FS-042-Session`, whose
            // slug the target grammar rejects, would be read as `§api/FS-042`
            // with `-Session` left dangling off the end of the citation.
            if parsed.shorthand && !grammar.id_token_ends_cleanly(raw, end) {
                continue;
            }
            return Some(ParsedIdPrefix {
                id: parsed.id,
                section: parsed.section,
                len: end,
                shorthand: parsed.shorthand,
            });
        }
    }
    None
}

/// `parse_longest_id_prefix`'s result: the parsed ID and section, how many bytes
/// of the input it consumed, and whether it came from the shorthand branch.
struct ParsedIdPrefix {
    id: Id,
    section: Option<String>,
    len: usize,
    shorthand: bool,
}

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
    /// `prefix_pattern` without the `<alias>/` namespace — the shorthand shape
    /// on its own, which is what §FS-fmt.2.4.1 clause 2 asks the token after a
    /// run's delimiters to have.
    unqualified_prefix_pattern: String,
    /// Bare `[id] number_pattern` as a prefix of the same slice — the second
    /// half of the numeric-run test (§FS-fmt.2.4.1), which asks whether the
    /// token glued after a shorthand is another number (`§SPEC-001/003`) as
    /// well as whether it is another shorthand (`§SPEC-001→SPEC-003`).
    number_prefix_pattern: String,
    full_prefix_re: once_cell::sync::OnceCell<Regex>,
    prefix_re: once_cell::sync::OnceCell<Regex>,
    unqualified_prefix_re: once_cell::sync::OnceCell<Regex>,
    number_prefix_re: once_cell::sync::OnceCell<Regex>,
}

impl ShorthandGrammar {
    /// Compiled on first use, not at config load. Most runs never meet a
    /// shorthand, and on a small tree the fixed cost of compiling a regex is a
    /// visible share of the whole command — `grund show` is ~120M instructions
    /// of which the scan is a small part. Compiling these two eagerly measured
    /// as +3–5% on *every* read command in a numbered repo, for patterns those
    /// commands never ran (§GOAL-fast-feedback, §AR-benchmarks).
    ///
    /// The `expect` cannot fire, and `Grammar::build` is what makes that true
    /// rather than the shape of these patterns: it rejects a `number_pattern` or
    /// `slug_pattern` that is not a valid regex *standalone*, so every capture
    /// group in `id_pat` is self-contained and dropping one whole group leaves a
    /// balanced pattern. Without that check two patterns could balance only
    /// against each other and the removal would produce an unclosed group.
    fn prefix_re(&self) -> &Regex {
        self.prefix_re
            .get_or_init(|| Regex::new(&self.prefix_pattern).expect("shorthand pattern compiles"))
    }

    fn full_prefix_re(&self) -> &Regex {
        self.full_prefix_re.get_or_init(|| {
            Regex::new(&self.full_prefix_pattern).expect("full-ID prefix pattern compiles")
        })
    }

    /// Compiled on first use like the others, and reached later still: only a
    /// shorthand that has already passed every gate of §FS-fmt.2.4 asks the
    /// numeric-run question, so a tree without shorthands never builds it.
    fn unqualified_prefix_re(&self) -> &Regex {
        self.unqualified_prefix_re.get_or_init(|| {
            Regex::new(&self.unqualified_prefix_pattern)
                .expect("unqualified shorthand prefix pattern compiles")
        })
    }

    /// The other half of that question, compiled on the same terms.
    fn number_prefix_re(&self) -> &Regex {
        self.number_prefix_re.get_or_init(|| {
            Regex::new(&self.number_prefix_pattern).expect("number prefix pattern compiles")
        })
    }
}

/// The number-only shorthand's element list, or `None` when the format has no
/// shorthand: without `{number}` nothing is left to name the declaration, and
/// without `{slug}` there is nothing to omit (§FS-check.1.2, §FS-id.4.1).
fn shorthand_elements(elements: &[IdElement]) -> Option<Vec<IdElement>> {
    let has_both =
        elements.contains(&IdElement::Number) && elements.contains(&IdElement::Slug);
    has_both.then(|| elements_without(elements, &IdElement::Slug))
}

// ---------------------------------------------------------------------------
// Where a token of this grammar *ends*, and what that ending means.
//
// These sit here rather than in `shorthand.rs` for the reason the header above
// gives: they are questions about the template's shape, not about the shorthand
// rule that the shape serves. Every one is a pure function of the compiled
// grammar and a string — none reads a declaration, a citation, or a finding —
// and `shorthand_sits_in_numeric_run` reaches straight into the `ShorthandGrammar`
// patterns defined a few lines above it.
//
// The `regex` crate has no lookahead, so neither question can be part of a
// pattern; both are post-match tests over the bytes that follow it.
// ---------------------------------------------------------------------------

impl Grammar {
    /// Whether this repo's `[id] format` has a number-only shorthand at all
    /// (§FS-check.1.2) — the gate every shorthand pass checks first.
    fn has_shorthand(&self) -> bool {
        self.shorthand.is_some()
    }

    /// Whether `ch` could extend an ID token that has just ended — the test that
    /// gives the shorthand pattern the trailing boundary it cannot express
    /// (§DF-number-only-citation-shorthand.2.6).
    ///
    /// The shorthand pattern is `\A`-anchored at the *start* only, so under the
    /// default format it matches the `FS-042` inside `FS-042-User-Login` — a full
    /// ID whose slug this grammar rejects — and, without this test, a pass would
    /// claim the prefix and leave `-User-Login` glued to whatever it wrote in its
    /// place. A `regex` pattern cannot say "and nothing that could continue an ID
    /// follows" (the crate has no lookahead), so the rule lives here.
    ///
    /// Two classes continue a token: an ID component character (an alphanumeric,
    /// plus `_` because it is the separator people reach for by mistake), and a
    /// literal from `[id] format` — `-` under the default, which is exactly what
    /// makes `FS-042` a prefix of `FS-042-user-login`.
    ///
    /// `/` is deliberately **not** one of them. It is the namespace separator of
    /// §FS-workspace.1, which can only *precede* a kind, never follow a number, and
    /// the full-ID pass already reads `§FS-042-user-login/x` as a citation of
    /// `FS-042-user-login`. Treating it as a continuation here would make the
    /// shorthand and the canonical form disagree about the same boundary — and the
    /// shorthand would be the one silently dropped, which is the false negative
    /// §GOAL-no-dangling-refs exists to forbid.
    fn id_token_continues_with(&self, ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_' || self.format_literal_starting_with(ch).is_some()
    }

    /// The `[id] format` literal beginning with `ch`, if any.
    fn format_literal_starting_with(&self, ch: char) -> Option<&str> {
        self.elements.iter().find_map(|element| match element {
            IdElement::Literal(text) if text.starts_with(ch) => Some(text.as_str()),
            _ => None,
        })
    }

    /// Whether an ID token that the pattern matched up to `end` really ends there
    /// (§DF-number-only-citation-shorthand.2.6). `end` is a regex match end, so it
    /// is always a char boundary.
    ///
    /// A separator only continues the token when a component actually follows it.
    /// That distinction matters wherever a format literal is also ordinary
    /// punctuation: under `{kind}.{number}.{slug}` the sentence `see §FS.042.` ends
    /// with a literal `.`, and reading that as a continuation would drop a real
    /// citation. `see §FS.042.User-Login` has a component after the separator and
    /// is correctly refused.
    fn id_token_ends_cleanly(&self, rest: &str, end: usize) -> bool {
        let tail = &rest[end..];
        let Some(next) = tail.chars().next() else {
            return true;
        };
        if next.is_alphanumeric() || next == '_' {
            return false;
        }
        let Some(literal) = self.format_literal_starting_with(next) else {
            return true;
        };
        !tail
            .strip_prefix(literal)
            .and_then(|after| after.chars().next())
            .is_some_and(|after| after.is_alphanumeric() || after == '_')
    }

    /// Whether a shorthand token ending at `end` in `rest` is glued to a second
    /// number, which makes it a numeral in a run rather than a citation
    /// (§FS-fmt.2.4.1, §DF-shorthand-numeric-run.2.2).
    ///
    /// `id_token_ends_cleanly` answers whether the token *ended*; it says nothing
    /// about whether the token is a citation. `§SPEC-001→SPEC-003` clears it —
    /// `→` cannot continue an ID — and is a renumbering table, so expanding the
    /// left number attaches today's slug to a number that named something else
    /// and produces a citation `check` can never question.
    ///
    /// The evidence is the second number, not the punctuation, which is why no
    /// delimiter list appears here: whatever a person writes between two numbers
    /// is a delimiter, and what makes the pair a run is that the pair exists.
    ///
    /// The neighbour is matched **unqualified**, never with `prefix_re`, whose
    /// optional `<alias>/` would make any path ending in an ID-shaped segment a
    /// second number — `docs/functional-spec/FS-042-user-login.md` among them.
    fn shorthand_sits_in_numeric_run(&self, marker: &str, rest: &str, end: usize) -> bool {
        let Some(shorthand) = self.shorthand.as_ref() else {
            return false;
        };
        let Some(neighbor) = numeric_run_neighbor(marker, &rest[end..]) else {
            return false;
        };
        shorthand.number_prefix_re().is_match(neighbor)
            || shorthand.unqualified_prefix_re().is_match(neighbor)
    }
}

/// The token a delimiter run separates from the shorthand that just ended, or
/// `None` when `tail` opens no run at all (§DF-shorthand-numeric-run.2.2).
///
/// A run is the delimiters and what follows them. Three things disqualify it, and
/// they are the whole safety margin of the rule:
///
/// - **Whitespace.** The gluing is the evidence — a run is written as one
///   unbroken string because its parts belong together. Without this, the default
///   `number_pattern = "\d+"` would read `§FS-042 (2024)` as a run and refuse to
///   expand an ordinary citation.
/// - **The marker.** `§FS-042, §FS-043` is two citations the author marked one at
///   a time, which is the clearest statement of intent this grammar offers.
/// - **A bracket or a quote.** Those bound a construct instead of gluing two
///   numerals, so a walk that crossed one would join the characters *closing* the
///   citation's own construct to the ones *opening* the next and read the next
///   construct's number as the second one — making a run of the Markdown link
///   `[§FS-042](FS-042-user-login.md)` that §FS-fmt.6 itself writes, and of the
///   footnote reference `§FS-042[^1]`.
fn numeric_run_neighbor<'a>(marker: &str, tail: &'a str) -> Option<&'a str> {
    let mut end = 0;
    for ch in tail.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            break;
        }
        if ch.is_whitespace() || bounds_a_construct(ch) {
            return None;
        }
        if !marker.is_empty() && tail[end..].starts_with(marker) {
            return None;
        }
        end += ch.len_utf8();
    }
    // No delimiter is not a run, and delimiters running to the end of the line
    // have nothing after them to be one with.
    (end > 0 && end < tail.len()).then(|| &tail[end..])
}

/// Whether `ch` opens or closes a paired construct (§FS-fmt.2.4.1 clause 1).
///
/// Not the delimiter list §DF-shorthand-numeric-run.3 rejects, which enumerated
/// what *makes* a run and had to be complete to be correct. Every character here
/// is here for one property — it pairs — so a member nobody thought of costs a
/// withheld rewrite, not a corrupted line. `|` is deliberately absent: it is a
/// delimiter people write between numbers, and a Markdown table cell writes
/// `| §FS-042 |` with the spaces the whitespace rule already covers.
fn bounds_a_construct(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | '`')
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
    shorthand
        .prefix_re()
        .find(rest)
        // §DF-number-only-citation-shorthand.2.6: `$$FS-042abc` is not a
        // shorthand, so the trigger before it is not rewritable either.
        .filter(|found| grammar.id_token_ends_cleanly(rest, found.end()))
        .map(|found| at + found.end())
}

/// The `[id] format` template with each placeholder replaced by the schematic
/// name of what it accepts — `{kind}-{number}-{slug}` reads `<KIND>-<NNN>-<slug>`
/// (§FS-init.2.3, §FS-check.4.5). A substitution over the template's literal
/// text, so it is a fact about the config rather than a guess at what
/// `[id] number_pattern` and `[id] slug_pattern` accept: an ID assembled from
/// those patterns would be wrong for every project that narrows them, printed by
/// the tool in the message whose job is to say what the grammar wants.
///
/// Shared by the managed entrypoint block and the nothing-recognized caution so
/// the shape a user is taught and the shape a diagnostic names are one string.
fn id_shape(id_format: &str) -> String {
    id_format
        .replace("{kind}", "<KIND>")
        .replace("{number}", "<NNN>")
        .replace("{slug}", "<slug>")
}
