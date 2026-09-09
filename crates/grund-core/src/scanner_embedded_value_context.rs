/// Host-context recognition and heading normalization for embedded values
/// (§FS-values.2.4, §FS-values.9).

const EMBEDDED_VALUE_MARKER: &str = "<!-- grund:value -->";

/// Return the marker's byte offset only for the one authored suffix that grants
/// authority: one ASCII separator space, exact lowercase marker bytes, then
/// optional trailing whitespace (§FS-values.2.4). Lookalikes stay prose.
fn exact_embedded_value_marker(line: &str) -> Option<usize> {
    let trimmed = line.trim_end_matches([' ', '\t']);
    // `strip_suffix` proves the byte boundary as well as the suffix. Computing
    // an arbitrary byte offset and slicing there can land inside any preceding
    // non-ASCII character (§REQ-never-crashes, §FS-values.9).
    let before = trimmed.strip_suffix(EMBEDDED_VALUE_MARKER)?;
    let marker_start = before.len();
    let title = before.strip_suffix(' ')?;
    if title.ends_with(char::is_whitespace) || title.is_empty() {
        return None;
    }
    Some(marker_start)
}

fn without_source_block_close(line: &str, block_comment: bool) -> &str {
    if !block_comment {
        return line;
    }
    let trimmed = line.trim_end_matches([' ', '\t']);
    trimmed
        .strip_suffix("*/")
        .map(|before| before.trim_end_matches([' ', '\t']))
        .unwrap_or(line)
}

/// Enroll a marker only after the host line has been classified. Markdown and
/// enabled Python docstrings already provide semantic content; every other
/// source form must put the marker bytes inside the recognized comment span.
fn embedded_value_marker_for_line(
    line: &str,
    markdown: bool,
    in_py_docstring: bool,
    source_context: Option<SourceValueLineContext>,
) -> Option<usize> {
    if markdown || in_py_docstring {
        return exact_embedded_value_marker(line);
    }
    let context = source_context?;
    let semantic_line = without_source_block_close(line, context.block_comment);
    let marker = exact_embedded_value_marker(semantic_line)?;
    context
        .contains(marker, marker + EMBEDDED_VALUE_MARKER.len())
        .then_some(marker)
}

fn push_invalid_embedded_marker(
    findings: &mut Findings,
    id: Option<Id>,
    path: &Path,
    line: usize,
    column_offset: usize,
    marker_start: usize,
    message: &str,
) {
    findings.invalid_value_declarations.push(InvalidValueSite {
        id,
        file: path.to_path_buf(),
        line,
        column: Some(column_offset + marker_start + 1),
        message: message.to_string(),
        source: DeclarationSource::Text,
        binding_namespace: None,
        binding_section: None,
    });
}

fn component_without_block_close(component: &str, block_comment: bool) -> &str {
    without_source_block_close(component, block_comment)
}

/// Heading depth after the same configured wrapper accepted by declaration and
/// section scanning has been removed (§FS-values.2.4). This deliberately does
/// not decide whether the heading is citable; it also identifies plain/named
/// headings that are forbidden inside a strict embedded root.
fn authored_heading_level(
    line: &str,
    markdown: bool,
    block_comment: bool,
    config: &Config,
) -> Option<usize> {
    // Strip a source wrapper—including `#`—before authored heading hashes;
    // Python docstrings arrive as Markdown after quote normalization
    // (§FS-values.2.4).
    let content = if markdown {
        line.trim_start()
    } else {
        semantic_comment_content(line, false, block_comment, config).trim_start()
    };
    let level = content.bytes().take_while(|byte| *byte == b'#').count();
    (level > 0
        && content[level..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace))
    .then_some(level)
}

/// A numeric heading coordinate before title validation. This recognizes the
/// physical child slot even when the title is absent, so the component's own
/// error does not manufacture a second zero-component error at its root.
fn authored_numeric_heading_path(
    line: &str,
    markdown: bool,
    block_comment: bool,
    config: &Config,
) -> Option<String> {
    let content = if markdown {
        line.trim_start()
    } else {
        semantic_comment_content(line, false, block_comment, config).trim_start()
    };
    let level = content.bytes().take_while(|byte| *byte == b'#').count();
    let rest = content.get(level..)?;
    if level == 0 || !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let token = rest.trim_start().split_whitespace().next()?;
    let coordinate = token.strip_suffix('.').unwrap_or(token);
    coordinate
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| coordinate.to_string())
}

fn semantic_comment_content<'a>(
    line: &'a str,
    markdown: bool,
    block_comment: bool,
    config: &Config,
) -> &'a str {
    let mut content = line.trim();
    if markdown {
        return content;
    }
    if matches!(content, "/*" | "/**" | "/*!" | "*" | "*/") {
        return "";
    }
    let mut prefixes = config
        .comment_prefixes
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if config.comment_prefixes.iter().any(|prefix| prefix == "//") {
        prefixes.extend(["///", "//!", "//"]);
    }
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    if let Some(prefix) = prefixes
        .into_iter()
        .find(|prefix| content.starts_with(prefix))
    {
        content = content[prefix.len()..].trim();
    }
    without_source_block_close(content, block_comment).trim()
}
