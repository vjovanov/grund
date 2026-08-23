// One fence reader shared by every Markdown surface. Keeping it separate from
// the scanner makes the scope rule reusable without making any consumer parse
// a second, subtly different notion of a fenced example.

/// The delimiter that opened a Markdown fenced code block (§FS-check.1.1).
/// Remembering both fields prevents a tilde run or a shorter backtick run from
/// ending a fence it did not open.
#[derive(Clone, Copy)]
struct MarkdownFence {
    byte: u8,
    len: usize,
}

/// Consume a Markdown fence delimiter, updating `open` and returning whether
/// this line is the opener or closer itself (§FS-check.1.1). Scanning,
/// formatting, on-type rewrites, and body rendering share this state machine so
/// every surface agrees on which lines are examples rather than live text.
fn markdown_fence_delimiter(open: &mut Option<MarkdownFence>, line: &str) -> bool {
    let bytes = line.as_bytes();
    let indent = bytes.iter().take_while(|byte| **byte == b' ').count();
    if indent > 3 || indent == bytes.len() {
        return false;
    }
    let delimiter = bytes[indent];
    if delimiter != b'`' && delimiter != b'~' {
        return false;
    }
    let run = bytes[indent..]
        .iter()
        .take_while(|byte| **byte == delimiter)
        .count();
    if run < 3 {
        return false;
    }
    let tail = &line[indent + run..];

    match *open {
        Some(fence) => {
            if delimiter == fence.byte
                && run >= fence.len
                && tail.bytes().all(|byte| byte == b' ' || byte == b'\t')
            {
                *open = None;
                true
            } else {
                false
            }
        }
        None => {
            if delimiter == b'`' && tail.as_bytes().contains(&b'`') {
                return false;
            }
            *open = Some(MarkdownFence {
                byte: delimiter,
                len: run,
            });
            true
        }
    }
}
