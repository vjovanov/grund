/// Source comment-span classification for explicit value bindings
/// (§FS-values.3.2, §AR-scanner.2.3).

fn value_binding_context(line: &CitationLine<'_>) -> Option<(usize, usize)> {
    if line.is_md || line.docstring.is_docstring() {
        return Some((0, line.scan_line.len()));
    }
    line.value_comment_range
}

fn binding_span_is_inside(context: (usize, usize), start: usize, end: usize) -> bool {
    context.0 <= start && start < end && end <= context.1
}

/// The exact comment byte range on every source line, derived from the same
/// block walk used for declaration bodies and doc-comment structure. A block
/// interior remains recognized without a decorative `*`; the first `*/` ends
/// the range so host expressions or strings after it cannot bind values
/// (§FS-values.3.2).
fn recognized_source_comment_ranges(
    text: &str,
    is_py: bool,
    config: &Config,
) -> Vec<Option<(usize, usize)>> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut ranges = vec![None; lines.len()];
    for (start, end, kind) in comment_blocks(&lines, is_py, config) {
        match kind {
            CommentBlockKind::PythonDocstring => {
                // `source_scan_line` already slices each docstring line to its
                // exact content, including the closing-delimiter boundary.
            }
            CommentBlockKind::Line(_) => {
                for index in start..=end {
                    let line = lines[index];
                    let comment_start = line.len() - line.trim_start().len();
                    ranges[index] = Some((comment_start, line.len()));
                }
            }
            CommentBlockKind::Block => {
                for index in start..=end {
                    let line = lines[index];
                    let comment_start = if index == start {
                        line.len() - line.trim_start().len()
                    } else {
                        0
                    };
                    let comment_end = line[comment_start..]
                        .find("*/")
                        .map_or(line.len(), |close| comment_start + close + 2);
                    ranges[index] = Some((comment_start, comment_end));
                }
            }
        }
    }
    ranges
}
