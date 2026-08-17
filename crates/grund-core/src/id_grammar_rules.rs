// The rules an `[id]` table must satisfy before a grammar is built from it
// (§FS-config.3.2). Today that is one rule — no ID the grammar can build may
// contain a `/`, the character the citation namespace has already spent
// (§FS-workspace.1) — asked once per key, because a `/` reaches an ID
// differently through each: `format` and a `[[kinds]]` prefix contribute literal
// text, `number_pattern` and `slug_pattern` contribute whatever they match.
//
// Split out of `grammar.rs`, which compiles the regexes: these functions answer
// a question about a *config key* and are the same answer whether it is asked at
// the line that wrote the key (`config.rs`, located) or of a `Config` assembled
// in code (`Grammar::build`, the backstop). One rule, one place, two callers.
//
// File-level prose, so `//` rather than `///` — see the note in `shorthand.rs`.

/// §FS-config.3.2: no ID the grammar can build may contain a `/`. The character
/// belongs to the citation namespace (§FS-workspace.1) — a qualified citation and
/// every `<alias>/<ID>` CLI argument split on the **last** one — so an ID that
/// contained a `/` would declare and resolve and then be unqueryable, the alias
/// boundary landing inside it. Both functions return the message body for a
/// located config error, or `None` when the key is clean. `label` names the key as
/// the config wrote it.
///
/// The literal half: `[id] format` and a `[[kinds]]` prefix contribute
/// `regex::escape`d text, so a `/` in the key is a `/` in every ID built from it
/// and a substring test is exact.
fn id_grammar_literal_slash_error(label: &str, value: &str) -> Option<String> {
    value.contains('/').then(|| id_grammar_slash_message(label, "contain"))
}

/// The pattern half: `number_pattern` and `slug_pattern` are *regexes*, where the
/// character's presence in the text answers neither direction. `[^/.]+` contains a
/// `/` and can never produce one; `[^.[:space:]]+` and `.*` contain none and match
/// one freely — that second case is the defect this rule exists to close, and a
/// substring test left it wide open while rejecting configs that had always
/// loaded. So ask what the pattern can *match*.
fn id_grammar_pattern_slash_error(label: &str, pattern: &str) -> Option<String> {
    pattern_admits_slash(pattern).then(|| id_grammar_slash_message(label, "match"))
}

/// §FS-config.3.2: and `section_separator` may not carry a `/` either — the same
/// invariant from the other side. A citation is `[<alias path>/]<ID>[<sep><section>]`
/// and its alias-path boundary is the **last** `/`, so a `/` separator makes the two
/// boundaries the same character: `<§>root/fs-x/1` — section 1 of `fs-x` in project
/// `root` — reads as alias path `root/fs-x` and ID `1`. That citation resolved before
/// alias *paths* existed, so a `[citations]` obligation resting on it turns red with
/// no config change (§FS-workspace.1).
fn section_separator_slash_error(separator: &str) -> Option<String> {
    separator.contains('/').then(|| {
        "[id].section_separator must not contain `/` (a citation's alias path ends at the last `/`, so a `/` here would put the ID/section boundary inside it)".to_string()
    })
}

fn id_grammar_slash_message(label: &str, verb: &str) -> String {
    // The parenthetical carries what the key must satisfy, like the `(expected …)`
    // clause the neighbouring `[id]` validators use (§FS-errors.3). For a pattern
    // that is a property of what it matches, not of its text, and saying so is what
    // keeps `must not match` from being read as `must not contain`.
    let expected = match verb {
        "match" => "expected a pattern that cannot produce one",
        _ => "an ID never contains `/`",
    };
    format!("{label} must not {verb} `/` ({expected} — it separates the alias path from the ID)")
}

/// Whether any string this pattern matches can contain a `/`: walk the parsed
/// syntax and ask whether any literal or character class in it admits the
/// character. Matching against sample strings cannot answer this — `[a-z0-9-]*`
/// matches the empty string at position 0, so `is_match("/")` says yes to a
/// pattern that can never produce one, and no finite set of samples covers a
/// pattern like `[a-z]{3}/[a-z]{3}` either.
///
/// A pattern that does not parse admits nothing here: the regex error is reported
/// by the "valid regex on its own" validator (§FS-config.3.2), and naming it a `/`
/// rejection would name the wrong defect.
fn pattern_admits_slash(pattern: &str) -> bool {
    regex_syntax::parse(pattern).is_ok_and(|hir| hir_admits_slash(&hir))
}

fn hir_admits_slash(hir: &regex_syntax::hir::Hir) -> bool {
    use regex_syntax::hir::{Class, HirKind};
    match hir.kind() {
        HirKind::Empty | HirKind::Look(_) => false,
        HirKind::Literal(literal) => literal.0.contains(&b'/'),
        HirKind::Class(Class::Unicode(class)) => class
            .ranges()
            .iter()
            .any(|range| range.start() <= '/' && '/' <= range.end()),
        HirKind::Class(Class::Bytes(class)) => class
            .ranges()
            .iter()
            .any(|range| range.start() <= b'/' && b'/' <= range.end()),
        // A sub-expression repeated zero times contributes nothing to any match.
        HirKind::Repetition(repetition) => {
            repetition.max != Some(0) && hir_admits_slash(&repetition.sub)
        }
        HirKind::Capture(capture) => hir_admits_slash(&capture.sub),
        HirKind::Concat(parts) | HirKind::Alternation(parts) => {
            parts.iter().any(hir_admits_slash)
        }
    }
}

/// §FS-config.3.2: the `/` rule for one `[id]` key, by key name — the one entry
/// point `config.rs` needs, so the caller that reads a TOML line does not also
/// have to know which shape of rule that line's key takes. An unknown key has no
/// rule.
fn id_grammar_key_slash_error(key: &str, value: &str) -> Option<String> {
    match key {
        "format" => id_grammar_literal_slash_error("[id].format", value),
        "section_separator" => section_separator_slash_error(value),
        "number_pattern" | "slug_pattern" => {
            id_grammar_pattern_slash_error(&format!("[id].{key}"), value)
        }
        _ => None,
    }
}
