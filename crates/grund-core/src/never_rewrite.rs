/// The never-rewrite predicates shared by the scanner (§AR-scanner.2.3), `fmt`
/// (§FS-fmt.2.3), and the LSP on-type path (§FS-lsp.1.4): where a citation-shaped
/// token sits in a context none of the three may touch — a string literal, an
/// inline-code span, or a Markdown link destination.
///
/// Whether a citation at `marker_start` sits in one of the contexts `grund fmt`
/// must never rewrite (§FS-fmt.2.3): an illustration in inline code, a Markdown
/// link destination, or a runtime string literal.
///
/// One predicate, read by both the rewrite and the scanner, because the two have
/// to agree: `fmt` is forbidden to canonicalize a shorthand here, so
/// §FS-check.3.13 must not demand it here either — an error whose named fix the
/// tool refuses to perform is an error a repository can never clear.
fn never_rewrite_context(line: &str, is_md: bool, marker_start: usize) -> bool {
    if is_md {
        is_inside_inline_code(line, marker_start)
            || is_inside_markdown_link_destination(line, marker_start)
    } else {
        is_inside_string_literal(line, marker_start)
    }
}

/// Where a Python docstring's **content** sits on one raw source line: the text
/// between the `"""` / `'''` delimiters, and the byte offset it starts at
/// (§AR-scanner.4). Empty on every other line — a code line, any line of a file
/// that is not `.py`, any line under `docstring_python = false` — where the raw
/// line is its own content and nothing below changes.
///
/// §FS-fmt.2.3.1: a docstring's delimiters are doc-comment syntax, not quotes, so
/// the never-rewrite walk runs over this slice rather than the raw line and a
/// docstring line is judged exactly like a `#` comment line. This is the one view
/// the scanner, `fmt`, and the LSP on-type path all hold, which is what keeps
/// §AR-scanner.2.6's "one predicate serving both" true now that the predicate has
/// two texts to choose between.
#[derive(Clone, Copy, Default)]
struct DocstringContent<'a> {
    span: Option<(usize, &'a str)>,
}

impl<'a> DocstringContent<'a> {
    /// The content span of a line the scanner has already normalized
    /// (§AR-scanner.4). `SourceScanLine::text` is a slice of the raw line starting
    /// at `column_offset`, which is exactly the pair this view is.
    fn of(scan: &SourceScanLine<'a>) -> Self {
        Self {
            span: scan
                .in_py_docstring
                .then_some((scan.column_offset, scan.text)),
        }
    }

    /// The text a whole-line reader sees here: the docstring's content on a
    /// docstring line, the raw line everywhere else (§AR-scanner.4).
    fn text_of(self, raw_line: &'a str) -> &'a str {
        self.span.map_or(raw_line, |(_, text)| text)
    }

    /// Whether this line is docstring content at all.
    fn is_docstring(self) -> bool {
        self.span.is_some()
    }

    /// The text a never-rewrite question at raw-line offset `pos` is asked of, with
    /// `pos` translated into it. A position outside the content — the delimiter
    /// itself, or the code after a one-line docstring closes — is judged on the raw
    /// line, the only text that describes it.
    fn view(self, raw_line: &'a str, pos: usize) -> (&'a str, usize) {
        match self.span {
            Some((offset, text)) if pos >= offset && pos < offset + text.len() => {
                (text, pos - offset)
            }
            _ => (raw_line, pos),
        }
    }
}

/// `never_rewrite_context` for a source line whose docstring content `docstring`
/// describes, at the raw-line offset `marker_start` (§FS-fmt.2.3.1). Every surface
/// that asks "may `fmt` rewrite here?" goes through this, so all three reach one
/// verdict per site.
fn never_rewrite_context_in(
    docstring: DocstringContent<'_>,
    line: &str,
    is_md: bool,
    marker_start: usize,
) -> bool {
    let (text, pos) = docstring.view(line, marker_start);
    never_rewrite_context(text, is_md, pos)
}

/// Whether `fmt` may rewrite the citation whose marker starts at `marker_start` —
/// a **`scan_line`** offset, which is what both scanner passes hold — on the line
/// they are scanning (§FS-fmt.2.3, §FS-check.3.13). One place asks it, so the
/// qualified pass and the unqualified one can never reach different verdicts about
/// one site; the recorded column stays a raw-file column either way.
fn scanned_citation_rewritable(line: &CitationLine<'_>, marker_start: usize) -> bool {
    !never_rewrite_context_in(
        line.docstring,
        line.raw_line,
        line.is_md,
        line.column_offset + marker_start,
    )
}

/// `is_inside_string_literal` asked of the same view (§FS-fmt.2.3.1) — what the
/// two `fmt` passes with no Markdown branch of their own, `replace_trigger` and
/// `add_markers`, use.
fn string_literal_in(docstring: DocstringContent<'_>, line: &str, pos: usize) -> bool {
    let (text, pos) = docstring.view(line, pos);
    is_inside_string_literal(text, pos)
}

/// The Python-docstring state a line **begins** in, so a caller that rewrites the
/// line can ask where its docstring content sits *in the line it is about to
/// change* (§FS-fmt.2.3.1).
///
/// `fmt` re-derives the span per rewrite stage instead of carrying it, because a
/// stage before it may have changed the line's length: `--marker` splices a `§`
/// into the docstring's content, and every offset after the splice moves. The
/// content's *start* never moves — only indentation and the delimiter precede it —
/// so replaying `source_scan_line` from the entry state is exact.
#[derive(Clone, Copy, Default)]
struct DocstringCursor {
    /// `true` only for a `.py` file in a project that scans docstrings; otherwise
    /// every line yields an empty view and no work is done.
    scanning: bool,
    quote: Option<&'static str>,
}

impl DocstringCursor {
    fn new(is_py: bool, docstring_python: bool) -> Self {
        Self {
            scanning: is_py && docstring_python,
            quote: None,
        }
    }

    /// Advance over one raw line, returning its content view.
    fn advance<'a>(&mut self, line: &'a str) -> DocstringContent<'a> {
        if !self.scanning {
            return DocstringContent::default();
        }
        let mut state = PythonDocstringScanState { quote: self.quote };
        let scan = source_scan_line(line, true, true, &mut state);
        self.quote = state.quote;
        DocstringContent::of(&scan)
    }

    /// The content view of `line` read from this cursor's *current* state, leaving
    /// the cursor where it is — the per-stage replay `fmt` needs.
    fn peek<'a>(self, line: &'a str) -> DocstringContent<'a> {
        let mut probe = self;
        probe.advance(line)
    }
}

/// §FS-check.1.1: a **bare** (unmarked) citation is suppressed in the same
/// never-rewrite zones — a source-file string literal, or, in a Markdown file, a
/// link destination — because a marker-prefixed citation is the only kind
/// `grund fmt` ever promotes to canonical form there (§FS-fmt.2.3). Recognizing a
/// bare token as a citation in a zone `fmt` will never touch would leave `check`
/// demanding an edit the formatter refuses to make. Markdown inline-code spans
/// are prose formatting rather than a rewrite hazard, so a bare token there stays
/// a citation off strict mode — this predicate is narrower than
/// `never_rewrite_context`, which also withholds the marked case from code.
fn bare_token_in_never_rewrite_zone(line: &str, is_md: bool, pos: usize) -> bool {
    if is_md {
        is_inside_markdown_link_destination(line, pos)
    } else {
        is_inside_string_literal(line, pos)
    }
}

/// Whether byte offset `pos` falls inside a `'…'`, `"…"`, or `` `…` `` literal on
/// this line — the source-code exclusion that keeps an ID printed in a string
/// from being treated as a citation by the scanner or rewritten by `fmt`
/// (§FS-fmt.2.3.1).
fn is_inside_string_literal(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut single = false;
    let mut double = false;
    let mut backtick = false;
    let mut i = 0;
    while i < pos && i < bytes.len() {
        match bytes[i] {
            b'\'' if !double && !backtick && !is_escaped(bytes, i) => single = !single,
            b'"' if !single && !backtick && !is_escaped(bytes, i) => double = !double,
            b'`' if !single && !double && !is_escaped(bytes, i) => backtick = !backtick,
            _ => {}
        }
        i += 1;
    }
    single || double || backtick
}

/// Whether byte offset `pos` falls inside a `` `…` `` inline-code span in Markdown
/// — citations there are illustrative, not real, so `fmt` leaves them alone
/// (§FS-fmt.2.3, §FS-fmt.6.4).
fn is_inside_inline_code(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut in_code = false;
    let mut i = 0;
    while i < pos && i < bytes.len() {
        if bytes[i] == b'`' && !is_escaped(bytes, i) {
            in_code = !in_code;
        }
        i += 1;
    }
    in_code
}

/// Whether byte offset `pos` falls inside the destination part of an inline
/// Markdown link (`[text](destination)`). URLs are presentation syntax, not
/// citations, so `fmt --marker` must not rewrite ID-shaped file names there
/// (§FS-fmt.2.3).
fn is_inside_markdown_link_destination(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b'(' && !is_escaped(bytes, i) {
            let start = i + 2;
            let mut depth = 1usize;
            let mut j = start;
            while j < bytes.len() {
                match bytes[j] {
                    b'(' if !is_escaped(bytes, j) => depth += 1,
                    b')' if !is_escaped(bytes, j) => {
                        depth -= 1;
                        if depth == 0 {
                            if pos >= start && pos < j {
                                return true;
                            }
                            i = j;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if j >= bytes.len() {
                return pos >= start;
            }
        }
        i += 1;
    }
    false
}
