const SEC_GROUP: &str = r"(?P<sec>\d+(?:\.\d+)*)";
const DEFAULT_INCLUDE: &[&str] = &["requirements.md", "docs", "e2e", "src"];
const DEFAULT_SCAN_EXTENSIONS: &[&str] = &[
    "md", "rs", "go", "java", "kt", "ts", "tsx", "js", "py", "c", "cpp", "swift", "scala",
    "rb", "php", "cs", "lisp", "scm", "clj", "sql", "hs", "lhs", "lua", "ada", "adb", "ads",
];
const DEFAULT_COMMENT_PREFIXES: &[&str] = &["//", "#", ";", "--", "*", "/*"];
static STUB_LINK_HEADING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*:\s*\[[^\]]*\]\(\s*(?P<path>[^)\s]+)\s*\)\s*$").unwrap());
/// An inline Markdown link `[text](url)` — used to reduce a heading to the text a
/// renderer would slugify (the destination URL is not part of that text), so an
/// anchor stays correct even when a citation in a section heading has been wrapped
/// by `grund fmt --cross-refs` (§DF-github-anchor-fidelity, §FS-fmt.6.2).
static MD_INLINE_LINK: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap());
/// An HTML-tag-shaped span `<…>` — a renderer drops it from a heading's text
/// (`## RM-read: grund <ID>` slugs as `rm-read-grund`), so it must be removed
/// before slugging the heading (§DF-github-anchor-fidelity, §FS-fmt.6.2).
static HTML_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]*>").unwrap());

/// Reduce a heading's source text to the text content a Markdown renderer would
/// slugify: an inline link `[text](url)` shows as `text`, an HTML-tag span `<…>`
/// is dropped. Used by both the section-anchor and declaration-anchor paths
/// (§DF-github-anchor-fidelity, §DF-declaration-anchor).
///
/// Neither reduction applies **inside an inline code span**, because a renderer
/// resolves code spans first and everything within one is literal text. GitHub
/// slugs `### 8.1 \`grund <alias>/<ID>\`` as `81-grund-aliasid`, not
/// `81-grund-`: the angle brackets are content there, not a tag. Stripping them
/// anyway produced an anchor that resolves nowhere, and every heading that
/// documents a placeholder-carrying command has that shape.
fn reduce_heading_text(text: &str) -> String {
    let mut out = String::new();
    for (segment, is_code) in inline_code_segments(text) {
        if is_code {
            out.push_str(segment);
        } else {
            out.push_str(&HTML_TAG.replace_all(&MD_INLINE_LINK.replace_all(segment, "$1"), ""));
        }
    }
    out
}

/// Split heading text into its non-code and inline-code segments, flagged
/// `true` for code. A span opens on a run of *n* backticks and closes on the
/// next run of exactly *n* (CommonMark); a run that never closes is ordinary
/// text, which is why the scan continues past it rather than swallowing the
/// rest of the heading.
fn inline_code_segments(text: &str) -> Vec<(&str, bool)> {
    let bytes = text.as_bytes();
    let mut segments = Vec::new();
    let mut plain_start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let open = i;
        while i < bytes.len() && bytes[i] == b'`' {
            i += 1;
        }
        let fence = i - open;
        let mut scan = i;
        let close = loop {
            while scan < bytes.len() && bytes[scan] != b'`' {
                scan += 1;
            }
            if scan >= bytes.len() {
                break None;
            }
            let run = scan;
            while scan < bytes.len() && bytes[scan] == b'`' {
                scan += 1;
            }
            if scan - run == fence {
                break Some(scan);
            }
        };
        if let Some(stop) = close {
            if plain_start < open {
                segments.push((&text[plain_start..open], false));
            }
            segments.push((&text[open..stop], true));
            plain_start = stop;
            i = stop;
        }
    }
    if plain_start < text.len() {
        segments.push((&text[plain_start..], false));
    }
    segments
}
/// The explicit managed-block delimiters (§FS-init.2.3,
/// §DF-managed-block-delimiters): standard `BEGIN`/`END` HTML-comment lines
/// bound the managed region from block v4 on. Legacy v3-and-earlier blocks have
/// no delimiters and are found by `AGENTS_BLOCK_H2` alone.
static AGENTS_BLOCK_BEGIN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^<!-- BEGIN GRUND MANAGED BLOCK -->[ \t]*\r?$").unwrap()
});
static AGENTS_BLOCK_END: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^<!-- END GRUND MANAGED BLOCK -->[ \t]*\r?$").unwrap()
});
/// The managed block's version marker: an H2 heading carrying the block
/// version. Inside a delimited block it names the schema version; for a legacy
/// block it is also the begin marker, and the block runs until the next H1/H2
/// or EOF (§FS-init.2.3.1).
static AGENTS_BLOCK_H2: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^##[ \t]+Grounding with grund[ \t]+\(v(?P<version>\d+)\)[ \t]*\r?$")
        .unwrap()
});
/// The next H1 or H2 heading after a position — the implicit end of a legacy
/// managed section. A legacy block ends at this line's start, or at EOF if no
/// such line follows.
static AGENTS_SECTION_BOUNDARY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^#{1,2}[ \t]+\S").unwrap());
/// ID grammar compiled from [id].format + [[kinds]] — the single place that knows the
/// shape of a declaration heading or a citation. Built once per config load.
/// Realizes §FS-config.3.1, §FS-config.3.2, §FS-config.3.3 and the regex-not-a-parser
/// stance of §AR-scanner.5.
/// The pattern one alias segment must match (§FS-workspace.1, §AR-workspace.2).
/// One canonical place — also referenced by the config-load alias validator
/// (`is_valid_project_alias` in `config.rs`).
const PROJECT_ALIAS_PATTERN: &str = "[a-z][a-z0-9-]*";
/// The namespace a qualified citation carries: one alias segment per workspace
/// level, so a project nested inside a member workspace is named by its whole
/// chain (§FS-workspace.1, §FS-workspace.6.1). Greedy by construction and the
/// ID that follows never contains `/`, so the last `/` in the token is always
/// the boundary between the project path and the ID.
static PROJECT_PATH_PATTERN: Lazy<String> =
    Lazy::new(|| format!("{PROJECT_ALIAS_PATTERN}(?:/{PROJECT_ALIAS_PATTERN})*"));
static QUALIFIED_CITATION_PREFIX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(r"^(?P<namespace>{})/", *PROJECT_PATH_PATTERN)).unwrap()
});

#[derive(Clone)]
pub struct Grammar {
    decl_re: Regex,
    docstring_decl_re: Regex,
    section_re: Regex,
    /// One citation regex, capturing an optional `<namespace>/` prefix
    /// (§FS-workspace.1, §AR-workspace.3.1). The scanner decides whether to
    /// emit a qualified citation based on whether the marker `§` precedes the
    /// match; this regex never has two modes.
    citation_re: Regex,
    id_input_re: Regex,
    /// The near-miss patterns (§FS-check.4.7): a heading that opens with a
    /// configured kind and the literal an ID puts after it, without parsing as
    /// an ID. `None` where `[id] format` puts no literal between `{kind}` and
    /// what follows — there "looks like a declaration" cannot be told from prose
    /// beginning with a kind name, and the rule declines rather than guess.
    near_miss: Option<NearMissGrammar>,
    /// The number-only shorthand patterns (§FS-check.1.2, §AR-scanner.2.6),
    /// present only when `[id] format` carries both `{number}` and `{slug}`.
    /// `None` is the whole opt-out: every shorthand pass downstream is gated on
    /// this being `Some`, so a `{kind}-{slug}` repo like `grund` itself compiles
    /// nothing extra and pays nothing (§FS-id.4.1).
    shorthand: Option<ShorthandGrammar>,
    /// The parsed `[id] format`. Kept so `render_id` reduces a partial `Id` by
    /// the same rule the shorthand pattern was derived from, rather than a
    /// second interpretation of the template (§AR-scanner.2.6).
    elements: Vec<IdElement>,
}

impl Grammar {
    /// Compile the four regexes from the effective config. The validation rejections
    /// here (`{kind}` required, at least one of `{number}`/`{slug}`, separator must be
    /// lexically distinct) are §FS-config.3.2; the optional `§`-marker prefix on a
    /// citation is §FS-config.3.1 / §DF-reference-marker; the comment-prefix wrapper
    /// on declaration/section regexes is §AR-scanner.4 (declarations live in code
    /// doc-comments too).
    fn build(
        format: &str,
        kinds: &[String],
        number_pattern: &str,
        slug_pattern: &str,
        section_separator: &str,
        comment_prefixes: &[String],
    ) -> Result<Self> {
        let kind_alt = if kinds.is_empty() {
            return Err(anyhow!("[id] grammar needs at least one [[kinds]] entry"));
        } else {
            kinds
                .iter()
                .map(|k| regex::escape(k))
                .collect::<Vec<_>>()
                .join("|")
        };
        // §FS-config.3.2: the "an ID never contains `/`" invariant, enforced over
        // every component an ID is built from. `config.rs` rejects each key at its
        // own line first; this is the backstop that keeps the invariant true for a
        // `Config` assembled in code, since the whole namespace grammar rests on it.
        if let Some(message) = id_grammar_literal_slash_error("[id].format", format) {
            return Err(anyhow!("{message}"));
        }
        for (label, pattern) in [
            ("[id].number_pattern", number_pattern),
            ("[id].slug_pattern", slug_pattern),
        ] {
            if let Some(message) = id_grammar_pattern_slash_error(label, pattern) {
                return Err(anyhow!("{message}"));
            }
        }
        for kind in kinds {
            if let Some(message) =
                id_grammar_literal_slash_error(&format!("[[kinds]] prefix `{kind}`"), kind)
            {
                return Err(anyhow!("{message}"));
            }
        }

        let kind_group = format!("(?P<kind>{})", kind_alt);
        let num_group = format!("(?P<num>{})", number_pattern);
        let slug_group = format!("(?P<slug>{})", slug_pattern);

        let elements = parse_id_format(format)?;
        let id_pat = id_pattern(&elements, &kind_group, &num_group, &slug_group);
        let literals: Vec<&String> = elements
            .iter()
            .filter_map(|element| match element {
                IdElement::Literal(text) => Some(text),
                _ => None,
            })
            .collect();
        let has_number = elements.contains(&IdElement::Number);

        // §FS-config.3.2: the section separator must be lexically distinguishable
        // from the ID grammar — otherwise a citation like `FS-foo<sep>bar` could
        // not be split into ID and section unambiguously.
        if section_separator.is_empty() {
            return Err(anyhow!("[id].section_separator must not be empty"));
        }
        if let Some(message) = section_separator_slash_error(section_separator) {
            return Err(anyhow!("{message}"));
        }
        if literals.iter().any(|lit| lit.contains(section_separator)) {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` collides with a literal in [id].format"
            ));
        }
        // §FS-config.3.2: each component pattern must be a valid regex *on its
        // own*, not merely valid once spliced into `id_pat`. Two patterns whose
        // parentheses balance only against each other — `number_pattern = "("`
        // with `slug_pattern = "a)"` — compile fine as one ID pattern and then
        // fall apart the moment a pass builds a pattern from a subset of the
        // elements, which is exactly what the number-only shorthand does
        // (§FS-check.1.2, `shorthand_elements`). Rejecting them here is what
        // lets every derived pattern be built by string surgery and still be
        // guaranteed to compile.
        let slug_re = Regex::new(slug_pattern)
            .map_err(|err| anyhow!("[id].slug_pattern is not a valid regex: {err}"))?;
        if slug_re.is_match(section_separator) {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` is matched by [id].slug_pattern"
            ));
        }
        if has_number {
            let number_re = Regex::new(number_pattern)
                .map_err(|err| anyhow!("[id].number_pattern is not a valid regex: {err}"))?;
            if number_re.is_match(section_separator) {
                return Err(anyhow!(
                    "[id].section_separator `{section_separator}` is matched by [id].number_pattern"
                ));
            }
        }

        let sep_quoted = regex::escape(section_separator);
        let sec_suffix = format!(r"(?:{}{})?", sep_quoted, SEC_GROUP);

        let comment_prefix = comment_prefix_regex(comment_prefixes);
        // Declaration grammar (§AR-scanner.2.1):
        //   1. Markdown-form: `#+`, then ID. The `#` is mandatory in `.md`.
        //   2. Code-form (§DF-code-declarations-drop-hash): comment prefix required,
        //      then ID directly. So `/// AR-foo: title` matches, but a bare prose
        //      line `AR-foo: title` in markdown does not.
        let decl_re = Regex::new(&format!(
            r"^\s*(?:{prefix}\s+|(?P<mdhashes>#+)\s+){id}\b",
            prefix = comment_prefix,
            id = id_pat
        ))?;
        let docstring_decl_re = Regex::new(&format!(r"^\s*{id}\b", id = id_pat))?;
        let section_re = Regex::new(&format!(
            r"^\s*(?:{})?\s*(?P<hashes>#+)\s+{}\.?\s+\S",
            comment_prefix, SEC_GROUP
        ))?;
        // §FS-workspace.1: the optional `<alias>/` namespace prefix is part of
        // the citation grammar, not a separate parser pass. The scanner gates
        // it on the marker (§AR-workspace.3.1) — without `§`, a `slug/ID`
        // token is treated as text, not a citation. One segment per workspace
        // level (§FS-workspace.6.1), so nesting needs no second grammar.
        let namespace_prefix = format!(r"(?:(?P<namespace>{})/)?", *PROJECT_PATH_PATTERN);
        let citation_re =
            Regex::new(&format!(r"\b{}{}{}", namespace_prefix, id_pat, sec_suffix))?;
        let id_input_re = Regex::new(&format!(r"^{}{}$", id_pat, sec_suffix))?;

        // §FS-check.1.2: the same two shapes over the slug-less element list.
        // Compiled only where the format has a shorthand at all, so `has_shorthand`
        // is the single gate the scanner, checker, `fmt`, and the LSP all read.
        let shorthand = shorthand_elements(&elements).map(|short| {
            let short_pat = id_pattern(&short, &kind_group, &num_group, &slug_group);
            ShorthandGrammar {
                full_prefix_pattern: format!(r"\A{}{}{}", namespace_prefix, id_pat, sec_suffix),
                prefix_pattern: format!(r"\A{}{}{}", namespace_prefix, short_pat, sec_suffix),
                // §FS-fmt.2.4.1 clause 2: the same shorthand shape with no
                // `<alias>/` in front of it. A namespace precedes a citation and
                // is never the second number of a run, so reusing `prefix_pattern`
                // here would count every path ending in an ID-shaped segment.
                unqualified_prefix_pattern: format!(r"\A{}{}", short_pat, sec_suffix),
                // Non-capturing: this one is only ever asked `is_match`, and a
                // second `(?P<num>…)` beside the one in `short_pat` would be a
                // duplicate group name if the two ever met in one pattern.
                number_prefix_pattern: format!(r"\A(?:{})", number_pattern),
                full_prefix_re: once_cell::sync::OnceCell::new(),
                prefix_re: once_cell::sync::OnceCell::new(),
                unqualified_prefix_re: once_cell::sync::OnceCell::new(),
                number_prefix_re: once_cell::sync::OnceCell::new(),
            }
        });

        Ok(Self {
            decl_re,
            docstring_decl_re,
            section_re,
            citation_re,
            id_input_re,
            near_miss: literal_after_kind_placeholder(format)
                .filter(|literal| !literal.is_empty())
                .map(|literal| NearMissGrammar::build(&kind_alt, &comment_prefix, literal)),
            shorthand,
            elements,
        })
    }
}

/// Build the alternation a declaration/section heading may be prefixed by — one
/// entry per `[scan] comment_prefixes` value (§FS-config.3.5), with `//` widened to
/// also catch Rust/JS doc-comment forms `///` and `//!` so inline declarations in
/// code are seen (§AR-scanner.4). Longest-first so `//` does not shadow `///`.
fn comment_prefix_regex(comment_prefixes: &[String]) -> String {
    let mut prefixes = comment_prefixes
        .iter()
        .filter(|prefix| !prefix.is_empty())
        .map(|prefix| {
            if prefix == "//" {
                r"//[/!]?".to_string()
            } else {
                regex::escape(prefix)
            }
        })
        .collect::<Vec<_>>();
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    if prefixes.is_empty() {
        "(?!)".to_string()
    } else {
        format!("(?:{})", prefixes.join("|"))
    }
}

#[derive(Default)]
struct PythonDocstringScanState {
    quote: Option<&'static str>,
}

struct SourceScanLine<'a> {
    text: &'a str,
    in_py_docstring: bool,
    column_offset: usize,
    closed_py_docstring: bool,
}

/// Normalize one source line for scanner-style declaration/section/citation
/// detection while preserving the original-file column offset for emitted
/// ranges (§AR-scanner.4).
fn source_scan_line<'a>(
    line: &'a str,
    is_py: bool,
    docstring_python: bool,
    py_docstring: &mut PythonDocstringScanState,
) -> SourceScanLine<'a> {
    if !docstring_python || !is_py {
        return SourceScanLine {
            text: line,
            in_py_docstring: false,
            column_offset: 0,
            closed_py_docstring: false,
        };
    }

    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if let Some(quote) = py_docstring.quote {
        if let Some(close) = trimmed.find(quote) {
            py_docstring.quote = None;
            return SourceScanLine {
                text: &trimmed[..close],
                in_py_docstring: true,
                column_offset: indent,
                closed_py_docstring: true,
            };
        }
        return SourceScanLine {
            text: trimmed,
            in_py_docstring: true,
            column_offset: indent,
            closed_py_docstring: false,
        };
    }

    let Some(quote) = python_docstring_quote(line) else {
        return SourceScanLine {
            text: line,
            in_py_docstring: false,
            column_offset: 0,
            closed_py_docstring: false,
        };
    };
    let after_open = &trimmed[quote.len()..];
    if let Some(close) = after_open.find(quote) {
        return SourceScanLine {
            text: &after_open[..close],
            in_py_docstring: true,
            column_offset: indent + quote.len(),
            closed_py_docstring: true,
        };
    }
    py_docstring.quote = Some(quote);
    SourceScanLine {
        text: after_open,
        in_py_docstring: true,
        column_offset: indent + quote.len(),
        closed_py_docstring: false,
    }
}

fn python_docstring_quote(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("\"\"\"") {
        Some("\"\"\"")
    } else if trimmed.starts_with("'''") {
        Some("'''")
    } else {
        None
    }
}

/// The near-miss half of the compiled [`Grammar`] (§FS-check.4.7): the
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
    fn build(kind_alt: &str, comment_prefix: &str, after_kind: &str) -> Self {
        // The trailing `:` is the discriminator, and it is doing real work: a
        // line that opens with an ID-shaped token and *no* colon is prose far
        // more often than it is a declaration attempt — a wrapped comment whose
        // continuation happens to begin with one is the case that found this.
        // §FS-check.4.7 therefore reads what §RM-declaration-near-miss describes,
        // `<KIND>-…: <title>`, and says nothing about the rest.
        //
        // The token stops at whitespace, at the colon, and at a backtick, so an
        // inline-code mention (`` `FS-login`: ``) is not one either and the
        // quoted token is the token as written.
        let near = format!(
            r"(?P<near>(?:{kind_alt}){after}[^\s:`]*):",
            after = regex::escape(after_kind)
        );
        Self {
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

/// The heading token §FS-check.4.7 reports, or `None` when this line is not one.
/// Asked only where [`declaration_captures`] already declined, so a hit is by
/// construction a heading that came close and missed.
fn near_miss_heading<'a>(
    grammar: &Grammar,
    line: &'a str,
    in_py_docstring: bool,
    is_md: bool,
) -> Option<&'a str> {
    grammar
        .near_miss
        .as_ref()?
        .heading_text(line, in_py_docstring, is_md)
}

fn declaration_captures<'a>(
    grammar: &Grammar,
    line: &'a str,
    in_py_docstring: bool,
    is_md: bool,
) -> Option<regex::Captures<'a>> {
    if in_py_docstring {
        grammar.docstring_decl_re.captures(line)
    } else {
        grammar
            .decl_re
            .captures(line)
            .filter(|caps| is_md || caps.name("mdhashes").is_none())
    }
}
