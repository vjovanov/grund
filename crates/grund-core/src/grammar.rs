const SEC_GROUP: &str = r"(?P<sec>\d+(?:\.\d+)*)";
const DEFAULT_INCLUDE: &[&str] = &["requirements.md", "docs", "e2e", "src"];
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
fn reduce_heading_text(text: &str) -> String {
    HTML_TAG
        .replace_all(&MD_INLINE_LINK.replace_all(text, "$1"), "")
        .into_owned()
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
/// The pattern an alias must match before the `/` of a qualified citation
/// (§FS-workspace.1, §AR-workspace.2). One canonical place — also referenced by
/// the config-load alias validator (`is_valid_project_alias` in `config.rs`).
const PROJECT_ALIAS_PATTERN: &str = "[a-z][a-z0-9-]*";
static QUALIFIED_CITATION_PREFIX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(r"^(?P<namespace>{})/", PROJECT_ALIAS_PATTERN)).unwrap()
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
        let kind_group = format!("(?P<kind>{})", kind_alt);
        let num_group = format!("(?P<num>{})", number_pattern);
        let slug_group = format!("(?P<slug>{})", slug_pattern);

        let mut id_pat = String::new();
        let mut literals: Vec<String> = Vec::new();
        let mut has_kind = false;
        let mut has_number = false;
        let mut has_slug = false;
        let mut cursor = 0;
        let bytes = format.as_bytes();
        while cursor < bytes.len() {
            if let Some(end) = format[cursor..].find('}') {
                let abs_end = cursor + end;
                if let Some(start_rel) = format[cursor..].find('{') {
                    let abs_start = cursor + start_rel;
                    if abs_start < abs_end {
                        // Append literal between cursor and abs_start (escaped).
                        if abs_start > cursor {
                            literals.push(format[cursor..abs_start].to_string());
                        }
                        id_pat.push_str(&regex::escape(&format[cursor..abs_start]));
                        let placeholder = &format[abs_start + 1..abs_end];
                        match placeholder {
                            "kind" => {
                                if has_kind {
                                    return Err(anyhow!("[id].format: {{kind}} appears twice"));
                                }
                                has_kind = true;
                                id_pat.push_str(&kind_group);
                            }
                            "number" => {
                                if has_number {
                                    return Err(anyhow!("[id].format: {{number}} appears twice"));
                                }
                                has_number = true;
                                id_pat.push_str(&num_group);
                            }
                            "slug" => {
                                if has_slug {
                                    return Err(anyhow!("[id].format: {{slug}} appears twice"));
                                }
                                has_slug = true;
                                id_pat.push_str(&slug_group);
                            }
                            other => {
                                return Err(anyhow!(
                                    "[id].format: unknown placeholder `{{{other}}}`"
                                ));
                            }
                        }
                        cursor = abs_end + 1;
                        continue;
                    }
                }
                return Err(anyhow!("[id].format: stray `}}` in template"));
            }
            // No more placeholders — append the rest as literal.
            if cursor < format.len() {
                literals.push(format[cursor..].to_string());
            }
            id_pat.push_str(&regex::escape(&format[cursor..]));
            break;
        }

        if !has_kind {
            return Err(anyhow!("[id].format must contain {{kind}}"));
        }
        if !has_number && !has_slug {
            return Err(anyhow!(
                "[id].format must contain at least one of {{number}} or {{slug}}"
            ));
        }

        // §FS-config.3.2: the section separator must be lexically distinguishable
        // from the ID grammar — otherwise a citation like `FS-foo<sep>bar` could
        // not be split into ID and section unambiguously.
        if section_separator.is_empty() {
            return Err(anyhow!("[id].section_separator must not be empty"));
        }
        if literals.iter().any(|lit| lit.contains(section_separator)) {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` collides with a literal in [id].format"
            ));
        }
        if Regex::new(slug_pattern)
            .map(|re| re.is_match(section_separator))
            .unwrap_or(false)
        {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` is matched by [id].slug_pattern"
            ));
        }
        if has_number
            && Regex::new(number_pattern)
                .map(|re| re.is_match(section_separator))
                .unwrap_or(false)
        {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` is matched by [id].number_pattern"
            ));
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
        // token is treated as text, not a citation.
        let namespace_prefix = format!(r"(?:(?P<namespace>{})/)?", PROJECT_ALIAS_PATTERN);
        let citation_re =
            Regex::new(&format!(r"\b{}{}{}", namespace_prefix, id_pat, sec_suffix))?;
        let id_input_re = Regex::new(&format!(r"^{}{}$", id_pat, sec_suffix))?;

        Ok(Self {
            decl_re,
            docstring_decl_re,
            section_re,
            citation_re,
            id_input_re,
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
