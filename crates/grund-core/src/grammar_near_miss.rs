/// The near-miss half of the compiled [`Grammar`] (§FS-check.4.6): the
/// declaration patterns with the ID grammar replaced by "a configured kind, the
/// separator an ID puts after it, and whatever follows". Two of them for the
/// same reason the declaration pair has two — a Python docstring line carries no
/// comment prefix (§AR-scanner.4).
///
/// Derived with the rest of the grammar so the rule reads the *project's* kinds
/// and comment prefixes rather than a second opinion about them, but **compiled
/// on first use**, the way the shorthand patterns are: a tree whose headings all
/// parse never matches with these, and on a small tree the fixed cost of
/// compiling a regex is a visible share of the whole command (§GOAL-fast-feedback,
/// §AR-benchmarks). The `expect` cannot fire: both patterns are built from an
/// alternation of `regex::escape`d kinds and the comment-prefix group
/// [`Grammar::build`] has already compiled on its own.
#[derive(Clone)]
struct NearMissGrammar {
    format: String,
    decl_pattern: String,
    docstring_pattern: String,
    decl_re: once_cell::sync::OnceCell<Regex>,
    docstring_re: once_cell::sync::OnceCell<Regex>,
    /// The bytes a line in declaration position can start with — `#`, and the
    /// first byte of every configured comment prefix. The gate below rejects on
    /// this and on the absence of the declaration colon before the regex is
    /// asked anything, because the regex is asked of *every* line the scan did
    /// not take as a declaration, which is very nearly every line in the tree.
    /// Measured on the 10k-file fixture, base against branch in one worktree:
    /// without the gate this rule cost +0.65% of `check` on the mean and +1.69%
    /// at worst; with it, +0.07% and +0.66% (§GOAL-fast-feedback,
    /// §AR-benchmarks.5, whose rule the two numbers were taken under).
    first_bytes: Vec<u8>,
}

impl NearMissGrammar {
    /// Why the trailing `:` is the discriminator: a line that opens with an
    /// ID-shaped token and *no* colon is prose far more often than a declaration
    /// attempt — a wrapped comment whose continuation begins with one is the case
    /// that found this, and the rule says nothing about the rest. The token
    /// stopping at a backtick likewise keeps an inline-code mention
    /// (`` `FS-login`: ``) from being one, and keeps the quoted token as written.
    fn build(kind_alt: &str, comment_prefix: &str, after_kind: &str, format: &str) -> Self {
        // §FS-check.4.6 reads only the shape it names, `<KIND>-…: <title>`: the
        // trailing `:` is the discriminator, and the token stops at whitespace, at
        // the colon, and at a backtick.
        let near = format!(
            r"(?P<near>(?:{kind_alt}){after}[^\s:`]*):",
            after = regex::escape(after_kind)
        );
        Self {
            format: format.to_string(),
            decl_pattern: format!(r"^\s*(?:{comment_prefix}\s+|(?P<mdhashes>#+)\s+){near}"),
            docstring_pattern: format!(r"^\s*{near}"),
            decl_re: once_cell::sync::OnceCell::new(),
            docstring_re: once_cell::sync::OnceCell::new(),
            first_bytes: first_declaration_bytes(comment_prefix),
        }
    }

    /// Whether this line is worth asking the regex about — a cheap conservative
    /// over-approximation of the pattern, never narrower than it. Both tests are
    /// implied by the pattern itself: it requires the declaration colon, and it
    /// anchors at `#` or a comment prefix unless the line is inside a Python
    /// docstring, where a declaration carries no prefix at all (§AR-scanner.4).
    fn could_match(&self, line: &str, in_py_docstring: bool) -> bool {
        if !line.as_bytes().contains(&b':') {
            return false;
        }
        if in_py_docstring {
            return true;
        }
        line.trim_start()
            .as_bytes()
            .first()
            .is_some_and(|byte| self.first_bytes.contains(byte))
    }

    /// The ID-shaped token of a heading, or `None` where no heading is. Same
    /// position rules as [`declaration_captures`] — including the one that keeps
    /// a Markdown-style heading in a source file from counting
    /// (§DF-code-declarations-drop-hash) — so a near miss is only ever read
    /// where a declaration would have been.
    fn heading_text<'a>(
        &self,
        line: &'a str,
        in_py_docstring: bool,
        is_md: bool,
    ) -> Option<&'a str> {
        if !self.could_match(line, in_py_docstring) {
            return None;
        }
        let caps = if in_py_docstring {
            self.docstring_re
                .get_or_init(|| {
                    Regex::new(&self.docstring_pattern).expect("near-miss pattern compiles")
                })
                .captures(line)
        } else {
            self.decl_re
                .get_or_init(|| {
                    Regex::new(&self.decl_pattern).expect("near-miss pattern compiles")
                })
                .captures(line)
                .filter(|caps| is_md || caps.name("mdhashes").is_none())
        }?;
        Some(caps.name("near")?.as_str())
    }
}

/// The bytes a declaration-position line can begin with: `#` for the Markdown
/// form, plus the first byte of every alternative in the comment-prefix group.
/// Read off the compiled alternation rather than the raw `[scan] comment_prefixes`
/// so it cannot drift from what the pattern actually accepts — `//` is widened to
/// `//[/!]?` there, and both still begin with `/`.
fn first_declaration_bytes(comment_prefix: &str) -> Vec<u8> {
    let mut bytes = vec![b'#'];
    for alternative in comment_prefix.trim_matches(['(', ')']).split('|') {
        // Every alternative is `regex::escape`d, so a leading `\` is the escape
        // of the byte that follows it.
        let literal = alternative.strip_prefix('\\').unwrap_or(alternative);
        if let Some(&byte) = literal.as_bytes().first() {
            bytes.push(byte);
        }
    }
    bytes.sort_unstable();
    bytes.dedup();
    bytes
}

/// The heading token §FS-check.4.6 reports, or `None` when this line is not one.
/// Asked only where [`declaration_captures`] already declined, so a hit is by
/// construction a heading that came close and missed.
fn near_miss_heading<'line, 'grammar>(
    grammar: &'grammar Grammar,
    line: &'line str,
    in_py_docstring: bool,
    is_md: bool,
) -> Option<(&'line str, &'grammar str, &'grammar str)> {
    if let Some(found) = grammar
        .near_misses
        .iter()
        .find_map(|near_miss| {
            near_miss
                .heading_text(line, in_py_docstring, is_md)
                .and_then(|text| {
                    grammar
                        .legacy_kind_and_format(text)
                        .map(|(kind, _)| (text, near_miss.format.as_str(), kind))
                })
        })
    {
        return Some(found);
    }

    // §FS-check.4.6: retain an unambiguous rejected declaration token. Its kind's
    // effective grammar is authoritative; the repository default cannot suppress a
    // persisted spelling rejected by an override (§FS-config.3.2).
    let caps = legacy_declaration_captures(grammar, line, in_py_docstring, is_md)?;
    let text = caps.name("near")?.as_str();
    grammar
        .legacy_kind_and_format(text)
        .map(|(kind, format)| (text, format, kind))
}

impl Grammar {
    fn legacy_kind_and_format<'a>(&'a self, token: &str) -> Option<(&'a str, &'a str)> {
        let mut matches = self.legacy.kinds.iter().filter(|(kind, _)| {
            token.strip_prefix(kind).is_some_and(|rest| {
                !rest.is_empty()
                    && rest
                        .chars()
                        .next()
                        .is_some_and(|ch| !ch.is_ascii_alphanumeric())
            })
        });
        let first = matches.next()?;
        matches.next().is_none().then_some((first.0.as_str(), first.1.as_str()))
    }
}

fn declaration_captures<'a>(
    grammar: &Grammar,
    line: &'a str,
    in_py_docstring: bool,
    is_md: bool,
) -> Option<regex::Captures<'a>> {
    let captures = if in_py_docstring {
        grammar.docstring_decl_re.captures(line)
    } else {
        grammar
            .decl_re
            .captures(line)
            .filter(|caps| is_md || caps.name("mdhashes").is_none())
    }?;
    // §FS-config.3.2: retain the exact token written before `:`. A narrowed
    // component pattern may match a shorter prefix (`FS-legacy` in
    // `FS-legacy-2:`); that prefix must not claim the declaration first.
    if let Some(complete) = legacy_declaration_captures(grammar, line, in_py_docstring, is_md)
        .and_then(|caps| caps.name("near"))
        && captures
            .name("id")
            .is_none_or(|id| id.as_str() != complete.as_str())
    {
        return None;
    }
    Some(captures)
}

fn legacy_declaration_captures<'a>(
    grammar: &Grammar,
    line: &'a str,
    in_py_docstring: bool,
    is_md: bool,
) -> Option<regex::Captures<'a>> {
    if in_py_docstring {
        grammar.legacy.docstring_decl_re.captures(line)
    } else {
        grammar
            .legacy
            .decl_re
            .captures(line)
            .filter(|caps| is_md || caps.name("mdhashes").is_none())
    }
}

/// Parse either a conforming declaration or a catalog-compatible persisted one
/// from a declaration-position line, returning the end of its written ID token
/// (§FS-config.3.2). Re-read consumers use this instead of inventing their own
/// compatibility fallback.
fn declaration_id_on_line(
    grammar: &Grammar,
    line: &str,
    in_py_docstring: bool,
    is_md: bool,
) -> Option<(Id, usize)> {
    if let Some(caps) = declaration_captures(grammar, line, in_py_docstring, is_md)
        && let Some(id) = parse_id(&caps, grammar)
    {
        return Some((id, caps.get(0)?.end()));
    }
    let (text, _, kind) = near_miss_heading(grammar, line, in_py_docstring, is_md)?;
    let start = line.find(text)?;
    Some((Id::legacy(kind.to_string(), text), start + text.len()))
}
