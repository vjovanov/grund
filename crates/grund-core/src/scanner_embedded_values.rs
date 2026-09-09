/// Embedded section value recognition and strict subtree validation. A root is
/// metadata on the scanner's existing section record, never another resolver
/// or declaration (§FS-values.2.4, §FS-values.5.1, §FS-values.6).

const EMBEDDED_VALUE_MARKER: &str = "<!-- grund:value -->";

/// Return the marker's byte offset only for the one authored suffix that grants
/// authority: one ASCII separator space, exact lowercase marker bytes, then
/// optional trailing whitespace (§FS-values.2.4). Lookalikes stay prose.
fn exact_embedded_value_marker(line: &str) -> Option<usize> {
    let mut trimmed = line.trim_end_matches([' ', '\t']);
    if let Some(before_close) = trimmed.strip_suffix("*/") {
        trimmed = before_close.trim_end_matches([' ', '\t']);
    }
    let marker_start = trimmed.len().checked_sub(EMBEDDED_VALUE_MARKER.len())?;
    if &trimmed[marker_start..] != EMBEDDED_VALUE_MARKER {
        return None;
    }
    let before = &trimmed[..marker_start];
    let title = before.strip_suffix(' ')?;
    if title.ends_with(char::is_whitespace) || title.is_empty() {
        return None;
    }
    Some(marker_start)
}

fn push_invalid_embedded_marker(
    findings: &mut Findings,
    id: Option<Id>,
    path: &Path,
    line: usize,
    column_offset: usize,
    scan_line: &str,
    message: &str,
) {
    let marker = exact_embedded_value_marker(scan_line).unwrap_or(0);
    findings.invalid_value_declarations.push(InvalidValueSite {
        id,
        file: path.to_path_buf(),
        line,
        column: Some(column_offset + marker + 1),
        message: message.to_string(),
        source: DeclarationSource::Text,
        binding_namespace: None,
        binding_section: None,
    });
}

fn component_without_block_close(component: &str) -> &str {
    component
        .strip_suffix("*/")
        .map(str::trim_end)
        .unwrap_or(component)
}

/// Heading depth after the same configured wrapper accepted by declaration and
/// section scanning has been removed (§FS-values.2.4). This deliberately does
/// not decide whether the heading is citable; it also identifies plain/named
/// headings that are forbidden inside a strict embedded root.
fn authored_heading_level(line: &str, markdown: bool, config: &Config) -> Option<usize> {
    let mut content = line.trim_start();
    if !markdown {
        if content.starts_with('#') {
            let level = content.bytes().take_while(|byte| *byte == b'#').count();
            return content[level..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
                .then_some(level);
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
        let prefix = prefixes
            .into_iter()
            .find(|prefix| content.starts_with(prefix))?;
        content = content[prefix.len()..].trim_start();
    }
    let level = content.bytes().take_while(|byte| *byte == b'#').count();
    (level > 0
        && content[level..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace))
    .then_some(level)
}

fn semantic_comment_content<'a>(
    line: &'a str,
    markdown: bool,
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
    content.strip_suffix("*/").unwrap_or(content).trim()
}

/// Validate every marked section against its physical one-level subtree and
/// place decoded components on the existing descendant section records
/// (§FS-values.2.4, §FS-values.5.2). No alternate section map is built.
fn validate_embedded_value_roots(
    path: &Path,
    text: &str,
    is_md: bool,
    is_py: bool,
    config: &Config,
    findings: &mut Findings,
) {
    let raw_lines = text.lines().collect::<Vec<_>>();
    let mut normalized = Vec::with_capacity(raw_lines.len());
    let mut py_docstring = PythonDocstringScanState::default();
    for line in &raw_lines {
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        normalized.push((
            scan.text.to_string(),
            scan.column_offset,
            scan.in_py_docstring,
        ));
    }

    let mut invalid = Vec::new();
    for decl in findings
        .declarations
        .values_mut()
        .flatten()
        .filter(|decl| paths_same_location(&decl.file, path))
    {
        let mut roots = decl
            .sections
            .iter()
            .filter_map(|(path, info)| {
                info.value_root
                    .as_ref()
                    .map(|_| (path.clone(), info.line))
            })
            .collect::<Vec<_>>();
        roots.sort_by_key(|(_, line)| *line);

        // A whole-declaration authority owns its complete section tree; an
        // embedded mark inside it is invalid and never competes for ownership.
        let whole_value = value_declaration_is_in_home(config, path, &decl.id);
        for index in 0..roots.len() {
            let (root_path, root_line) = roots[index].clone();
            let mut reasons: Vec<(usize, Option<usize>, String)> = Vec::new();
            let mut overlapping_lines = BTreeSet::new();
            if whole_value {
                reasons.push((
                    root_line,
                    decl.sections
                        .get(&root_path)
                        .and_then(|info| info.value_root.as_ref())
                        .map(|root| root.marker_column),
                    "embedded value root may not occur inside a whole-declaration value".to_string(),
                ));
            }

            for (other_path, other_line) in roots.iter().skip(index + 1) {
                if other_path.starts_with(&format!("{root_path}.")) {
                    overlapping_lines.insert(*other_line);
                    reasons.push((
                        *other_line,
                        decl.sections
                            .get(other_path)
                            .and_then(|info| info.value_root.as_ref())
                            .map(|root| root.marker_column),
                        "embedded value roots may not be nested or overlap".to_string(),
                    ));
                    if let Some(other) = decl.sections.get_mut(other_path)
                        && let Some(root) = &mut other.value_root
                    {
                        root.valid = false;
                    }
                }
            }

            let Some(root_info) = decl.sections.get(&root_path) else {
                continue;
            };
            let root_level = root_info.heading_level;
            let root_marker_column = root_info
                .value_root
                .as_ref()
                .map(|root| root.marker_column);
            let root_title_valid = normalized
                .get(root_line.saturating_sub(1))
                .and_then(|(line, _, _)| markdown_component(line, config))
                .and_then(|(title, _)| {
                    component_without_block_close(title)
                        .strip_suffix(&format!(" {EMBEDDED_VALUE_MARKER}"))
                })
                .is_some_and(|title| !title.is_empty());
            if !(decl.body_start..=decl.body_end).contains(&root_line) {
                reasons.push((
                    root_line,
                    root_marker_column,
                    "embedded value marker must be inside its declaration body".to_string(),
                ));
            }
            if !root_title_valid {
                reasons.push((
                    root_line,
                    root_marker_column,
                    "embedded value marker must follow a nonempty numeric section title".to_string(),
                ));
            }

            let subtree_end = ((root_line + 1)..=decl.body_end)
                .find(|line_no| {
                    normalized
                        .get(line_no.saturating_sub(1))
                        .and_then(|(line, _, docstring)| {
                            authored_heading_level(line, is_md || *docstring, config)
                        })
                        .is_some_and(|level| level <= root_level)
                })
                .map(|line| line - 1)
                .unwrap_or(decl.body_end);

            let mut expected = 1usize;
            let mut updates = Vec::new();
            let duplicate_lines = decl
                .duplicate_sections
                .iter()
                .filter(|(path, _)| path.starts_with(&format!("{root_path}.")))
                .map(|(_, info)| info.line)
                .collect::<BTreeSet<_>>();
            for line_no in root_line.saturating_add(1)..=subtree_end {
                if duplicate_lines.contains(&line_no) || overlapping_lines.contains(&line_no) {
                    continue;
                }
                let Some((line, column_offset, docstring)) = normalized.get(line_no - 1) else {
                    continue;
                };
                let markdown = is_md || *docstring;
                let content = semantic_comment_content(line, markdown, config);
                if content.is_empty() || matches!(content, "/*" | "/**" | "/*!" | "*" | "*/") {
                    continue;
                }
                let captures = config.grammar.section_re.captures(line);
                let child = captures.as_ref().and_then(section_path);
                let level = captures
                    .as_ref()
                    .map(|caps| heading_level_for_line(line, markdown, caps));
                let expected_path = format!("{root_path}.{expected}");
                if child != Some(expected_path.as_str()) || level != Some(root_level + 1) {
                    reasons.push((
                        line_no,
                        None,
                        format!(
                            "embedded value components must be immediate contiguous headings `.1` through `.N` (expected `{expected_path}`)"
                        ),
                    ));
                    continue;
                }
                let Some((component, column)) = markdown_component(line, config) else {
                    reasons.push((
                        line_no,
                        None,
                        "embedded value component must be a valid one-line heading title".to_string(),
                    ));
                    continue;
                };
                let component = component_without_block_close(component);
                if !component_text_is_valid(component) || component.contains(EMBEDDED_VALUE_MARKER) {
                    reasons.push((
                        line_no,
                        Some(column_offset + column),
                        "embedded value component must be nonempty, single-line, edge-unspaced, and contain no backtick or control character".to_string(),
                    ));
                    continue;
                }
                updates.push((
                    expected_path,
                    authored_component(component, column_offset + column),
                ));
                expected += 1;
            }
            if expected == 1 {
                reasons.push((
                    root_line,
                    root_marker_column,
                    "embedded value root must define contiguous numbered components starting at `.1`"
                        .to_string(),
                ));
            }
            for (path, component) in updates {
                if let Some(info) = decl.sections.get_mut(&path) {
                    info.value = Some(component);
                }
            }
            for (duplicate_path, duplicate) in &decl.duplicate_sections {
                if duplicate_path.starts_with(&format!("{root_path}.")) {
                    reasons.push((
                        duplicate.line,
                        None,
                        "embedded value component coordinate is duplicated".to_string(),
                    ));
                }
            }
            let valid = reasons.is_empty();
            if let Some(info) = decl.sections.get_mut(&root_path)
                && let Some(root) = &mut info.value_root
            {
                root.valid &= valid;
            }
            invalid.extend(
                reasons
                    .into_iter()
                    .map(|(line, column, message)| InvalidValueSite {
                        id: Some(decl.id.clone()),
                        file: decl.file.clone(),
                        line,
                        column,
                        message,
                        source: decl.source.clone(),
                        binding_namespace: None,
                        binding_section: None,
                    }),
            );
        }
    }
    findings.invalid_value_declarations.extend(invalid);
}
