// One comment block, classified: what opens it, where a docstring closes it,
// whether it declares an ID, and whether the language it is written in calls it
// documentation (§AR-scanner.4, §FS-inline-citation-style.1.1).
//
// These are the block-level classifiers the scanner's single pass used to carry
// (§AR-scanner.2): pure functions over a line or over one block's lines, holding
// no state of their own and coupled to the walk only by call. They live here for
// the reason `comment_line.rs` holds the per-line reductions — the walk is a
// state machine and these are not — and because two walks now ask them the same
// questions about the same block: the one that bounds a declaration body, and
// the one that decides whether a block is an inline citation site.

#[derive(Clone)]
enum CommentBlockKind {
    Line(String),
    Block,
    PythonDocstring,
}

fn comment_block_kind(line: &str, is_py: bool, config: &Config) -> Option<CommentBlockKind> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    if config.docstring_python && is_py && python_docstring_quote(line).is_some() {
        return Some(CommentBlockKind::PythonDocstring);
    }
    if config.comment_prefixes.iter().any(|prefix| prefix == "/*") && trimmed.starts_with("/*") {
        return Some(CommentBlockKind::Block);
    }
    line_comment_marker(trimmed, config).map(CommentBlockKind::Line)
}

fn line_comment_marker(trimmed: &str, config: &Config) -> Option<String> {
    for marker in ["///", "//!", "//"] {
        if config.comment_prefixes.iter().any(|prefix| prefix == "//")
            && trimmed.starts_with(marker)
        {
            return Some(marker.to_string());
        }
    }
    let mut prefixes = config
        .comment_prefixes
        .iter()
        .filter(|prefix| !matches!(prefix.as_str(), "" | "//" | "*" | "/*"))
        .collect::<Vec<_>>();
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    prefixes
        .into_iter()
        .find(|prefix| trimmed.starts_with(prefix.as_str()))
        .map(|prefix| prefix.to_string())
}

fn python_docstring_closes(line: &str, quote: &str, is_opening_line: bool) -> bool {
    let trimmed = line.trim_start();
    let search = if is_opening_line {
        trimmed.strip_prefix(quote).unwrap_or(trimmed)
    } else {
        trimmed
    };
    search.contains(quote)
}

fn block_declares_id(lines: &[&str], in_py_docstring: bool, config: &Config) -> bool {
    let mut py_docstring = PythonDocstringScanState::default();
    lines.iter().any(|line| {
        let scan = if in_py_docstring {
            source_scan_line(line, true, config.docstring_python, &mut py_docstring)
        } else {
            SourceScanLine {
                text: line,
                in_py_docstring: false,
                column_offset: 0,
                closed_py_docstring: false,
            }
        };
        declaration_captures(&config.grammar, scan.text, scan.in_py_docstring, false)
            .and_then(|caps| parse_id(&caps))
            .is_some()
    })
}

/// How one file's language spells "this comment is documentation"
/// (§FS-inline-citation-style.1.1). A doc comment is not an inline citation
/// site, so nothing the inline citation style says applies to it — the same
/// exemption a block that *declares* an ID already carries, for the same
/// reason: documentation is not a note about a clause.
///
/// The recognizer is chosen by the file's extension and read once per file
/// (§AR-scanner.4), then applied to a block with one comparison. Two shapes of
/// recognizer, because languages come in two: a **marker** language spells a doc
/// comment with its own marker, so the marker decides; a **position** language
/// spells it like any other comment, so where it sits decides. Neither parses
/// the host language (§FS-non-goals.3).
#[derive(Clone, Copy)]
enum DocCommentRule {
    /// The C family: a `///` or `//!` line run, a `/**` or `/*!` block.
    CFamily,
    /// Python: a `"""`/`'''` docstring and nothing else — under PEP 257 a `#`
    /// block is a code comment wherever it sits.
    Docstring,
    /// Lua's LDoc: a `--` run whose first line opens `---`.
    LuaDoc,
    /// Haskell's Haddock: a `--` run whose first line's content opens `|` or `^`.
    Haddock,
    /// R's roxygen2: a `#` run whose first line opens `#'`.
    Roxygen,
    /// A position language: the block is documentation when the line under it
    /// opens a definition, or when it is the file's leading comment.
    Position(DefinitionStart),
    /// An extension with no doc-comment notion of its own. Every comment block
    /// in such a file is an inline site, which is what every file was before
    /// this rule existed.
    None,
}

/// What the line under a position language's doc comment begins with
/// (§FS-inline-citation-style.1.1). Recognition, not parsing: the test reads one
/// line, so a Go `var` inside a function body classifies as a definition and a
/// Ruby `private def` does not. A miss only means a block is measured that need
/// not be; it never changes what a citation resolves to.
#[derive(Clone, Copy)]
enum DefinitionStart {
    /// Keywords, each matched at an identifier boundary so `func(` and
    /// `func main` open a definition while `functional` does not.
    Keywords(&'static [&'static str]),
    /// Shell has no single keyword to match: `function <name>`, `<name>()` and
    /// `<name> ()` are one definition wearing three spellings, so the starter is
    /// a shape rather than a word.
    ShellFunction,
    /// SQL's `create`, matched case-insensitively so `CREATE OR REPLACE
    /// FUNCTION …` counts with `create table …`.
    SqlCreate,
}

/// Go's definition keywords — `package` included, so a file-header comment above
/// the package clause is the module doc it looks like.
const GO_DEFINITION_STARTERS: &[&str] = &["func", "type", "var", "const", "package"];
/// Ruby's definition keywords. Leading whitespace is stripped before the test,
/// so an indented `def` inside a `class` still counts.
const RUBY_DEFINITION_STARTERS: &[&str] = &["class", "module", "def"];

/// The doc-comment recognizer for one file, keyed on its extension
/// (§FS-inline-citation-style.1.1, §AR-scanner.4). Built in and not
/// configurable: what a note *is* must not differ between two installs
/// (§FS-non-goals.13). An extension this table does not name has no doc notion,
/// so every one of its comment blocks stays an inline site and nothing a tree
/// already passes changes.
fn doc_comment_rule(path: &Path) -> DocCommentRule {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return DocCommentRule::None;
    };
    match extension {
        "rs" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "m" | "mm" | "java"
        | "cs" | "kt" | "kts" | "scala" | "swift" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx"
        | "php" | "dart" => DocCommentRule::CFamily,
        "py" => DocCommentRule::Docstring,
        "lua" => DocCommentRule::LuaDoc,
        "hs" | "lhs" => DocCommentRule::Haddock,
        // Both spellings an R source file is written with; the extension is the
        // only thing about R that is case-sensitive here.
        "r" | "R" => DocCommentRule::Roxygen,
        "go" => DocCommentRule::Position(DefinitionStart::Keywords(GO_DEFINITION_STARTERS)),
        "rb" => DocCommentRule::Position(DefinitionStart::Keywords(RUBY_DEFINITION_STARTERS)),
        "sh" | "bash" | "zsh" => DocCommentRule::Position(DefinitionStart::ShellFunction),
        "sql" => DocCommentRule::Position(DefinitionStart::SqlCreate),
        _ => DocCommentRule::None,
    }
}

/// Whether one comment block is a doc comment rather than an inline note
/// (§FS-inline-citation-style.1.1). Asked only where the caller already has the
/// block in hand — the scanner, for a block that carries a citation
/// (§AR-scanner.4) — so the whole rule costs one comparison per site.
///
/// `next_line` is the line immediately after the block, `None` at end of file; a
/// blank line there is what makes a `#` block above a `def` an inline comment
/// rather than its documentation. `is_leading` says every line before the block
/// is blank, or is line 1 and a `#!` shebang, and only a position language reads
/// it — a leading `//` block in Rust is a code comment, because Rust has `//!`
/// for the other thing.
///
/// A *dangling* doc comment — a `/** … */` or `///` inside a method body, which
/// javac's `-Xlint:dangling-doc-comments` and rustc's unused-doc-comment lint
/// already warn about — is a doc comment by its marker here too, and is not
/// measured. The language's own lint is the tool for that mistake.
fn block_is_doc_comment(
    rule: DocCommentRule,
    kind: &CommentBlockKind,
    block: &[&str],
    next_line: Option<&str>,
    is_leading: bool,
) -> bool {
    let first = block.first().map_or("", |line| line.trim_start());
    match rule {
        DocCommentRule::CFamily => match kind {
            // Exactly three slashes. A `////` run is a rule drawn across the
            // file, and Rust says so itself by not treating it as documentation.
            CommentBlockKind::Line(marker) => {
                (marker == "///" && !first.starts_with("////")) || marker == "//!"
            }
            // `/**/` is the empty block comment, not a Javadoc opener.
            CommentBlockKind::Block => {
                (first.starts_with("/**") && !first.starts_with("/**/")) || first.starts_with("/*!")
            }
            CommentBlockKind::PythonDocstring => false,
        },
        DocCommentRule::Docstring => matches!(kind, CommentBlockKind::PythonDocstring),
        // Exactly three dashes: `----` is a rule line, the way `////` is.
        DocCommentRule::LuaDoc => first.starts_with("---") && !first.starts_with("----"),
        DocCommentRule::Haddock => haddock_block_opens(first),
        DocCommentRule::Roxygen => first.starts_with("#'"),
        DocCommentRule::Position(start) => {
            is_leading
                || next_line.is_some_and(|line| definition_opens(start, line.trim_start()))
        }
        DocCommentRule::None => false,
    }
}

/// Haddock marks the block, not each line: `-- |` documents what follows and
/// `-- ^` what precedes, so a `--` run opening with either is documentation and
/// a plain `--` run is a code comment. The optional spaces are Haddock's own —
/// `--|` and `-- |` are the same marker.
fn haddock_block_opens(first: &str) -> bool {
    let Some(rest) = first.strip_prefix("--") else {
        return false;
    };
    matches!(
        rest.trim_start_matches(' ').as_bytes().first(),
        Some(b'|' | b'^')
    )
}

/// Whether the line under a position language's comment block opens a definition
/// (§FS-inline-citation-style.1.1). The line arrives with its leading whitespace
/// already stripped.
fn definition_opens(start: DefinitionStart, next: &str) -> bool {
    match start {
        DefinitionStart::Keywords(keywords) => keywords
            .iter()
            .any(|keyword| opens_with_keyword(next, keyword, false)),
        DefinitionStart::ShellFunction => shell_function_opens(next),
        DefinitionStart::SqlCreate => opens_with_keyword(next, "create", true),
    }
}

/// Whether `line` begins with `keyword` followed by a non-identifier character
/// or by nothing at all — the boundary that separates `func main` and `func(`
/// from `functional` (§FS-inline-citation-style.1.1).
fn opens_with_keyword(line: &str, keyword: &str, ignore_case: bool) -> bool {
    let Some(head) = line.get(..keyword.len()) else {
        return false;
    };
    let matched = if ignore_case {
        head.eq_ignore_ascii_case(keyword)
    } else {
        head == keyword
    };
    matched && !line[keyword.len()..].starts_with(is_identifier_char)
}

/// The identifier alphabet the boundary test and the shell function-name scan
/// share: `[A-Za-z0-9_]`.
fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Shell's two spellings of a function definition: `function <name>`, and
/// `<name>()` / `<name> ()` with `<name>` an `[A-Za-z_][A-Za-z0-9_]*`
/// (§FS-inline-citation-style.1.1). A scan rather than a regex — it reads at
/// most a name's worth of bytes and stops.
fn shell_function_opens(line: &str) -> bool {
    if opens_with_keyword(line, "function", false) {
        // The keyword alone is not a definition; a name has to follow it.
        return !line["function".len()..].trim_start().is_empty();
    }
    let name = shell_identifier_len(line);
    name > 0 && line[name..].trim_start().starts_with("()")
}

/// The byte length of the `[A-Za-z_][A-Za-z0-9_]*` identifier `line` opens with,
/// or `0` when it opens with something else.
fn shell_identifier_len(line: &str) -> usize {
    let mut chars = line.char_indices();
    match chars.next() {
        Some((_, ch)) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return 0,
    }
    chars
        .find(|(_, ch)| !is_identifier_char(*ch))
        .map_or(line.len(), |(index, _)| index)
}

/// The index of the first line that is neither blank nor a shebang (a `#!` on
/// the file's very first line). A comment block opening at or before it is the
/// file's *leading comment* — a position language's spelling of a module doc,
/// the thing `//!` and a module docstring are in the marker languages
/// (§FS-inline-citation-style.1.1).
///
/// One scan of the file, taken only where a position language will ask
/// (§GOAL-fast-feedback), rather than a walk back over every earlier line once
/// per block.
fn first_content_line(lines: &[&str]) -> usize {
    lines
        .iter()
        .enumerate()
        .find(|(index, line)| !line.trim().is_empty() && !(*index == 0 && line.starts_with("#!")))
        .map_or(lines.len(), |(index, _)| index)
}
