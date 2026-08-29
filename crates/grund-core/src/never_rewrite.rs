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
