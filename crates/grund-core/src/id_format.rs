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
}

/// The number-only shorthand's element list, or `None` when the format has no
/// shorthand: without `{number}` nothing is left to name the declaration, and
/// without `{slug}` there is nothing to omit (§FS-check.1.2, §FS-id.4.1).
fn shorthand_elements(elements: &[IdElement]) -> Option<Vec<IdElement>> {
    let has_both =
        elements.contains(&IdElement::Number) && elements.contains(&IdElement::Slug);
    has_both.then(|| elements_without(elements, &IdElement::Slug))
}
