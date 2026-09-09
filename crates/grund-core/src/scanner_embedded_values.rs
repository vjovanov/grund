/// Strict subtree validation for embedded section values. A root is metadata
/// on the scanner's existing section record, never another resolver or
/// declaration (§FS-values.2.4, §FS-values.5.1, §FS-values.6).

/// Validate every marked section against its physical one-level subtree and
/// place decoded components on the existing descendant section records
/// (§FS-values.2.4, §FS-values.5.2). No alternate section map is built.
fn validate_embedded_value_roots(
    path: &Path,
    text: &str,
    is_md: bool,
    is_py: bool,
    config: &Config,
    source_contexts: Option<&[Option<SourceValueLineContext>]>,
    findings: &mut Findings,
) {
    let raw_lines = text.lines().collect::<Vec<_>>();
    let mut normalized = Vec::with_capacity(raw_lines.len());
    let mut py_docstring = PythonDocstringScanState::default();
    for (index, line) in raw_lines.iter().enumerate() {
        let scan = source_scan_line(line, is_py, config.docstring_python, &mut py_docstring);
        normalized.push((
            scan.text.to_string(),
            scan.column_offset,
            scan.in_py_docstring,
            source_contexts
                .and_then(|contexts| contexts.get(index).copied().flatten())
                .is_some_and(|context| context.block_comment),
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

        // Root claims include resolver entries and later duplicate headings.
        // Resolve every pair by coordinate, independent of source order
        // (§FS-values.2.4).
        let mut claims = decl
            .sections
            .iter()
            .filter_map(|(path, info)| {
                info.value_root
                    .as_ref()
                    .map(|root| (path.clone(), info.line, root.marker_column))
            })
            .chain(decl.duplicate_sections.iter().filter_map(|(path, info)| {
                info.value_root
                    .as_ref()
                    .map(|root| (path.clone(), info.line, root.marker_column))
            }))
            .collect::<Vec<_>>();
        claims.sort_by_key(|(_, line, _)| *line);
        let mut invalid_root_paths = BTreeSet::new();
        let mut roots_with_descendants = BTreeSet::new();
        let mut overlap_sites = BTreeSet::new();
        for left in 0..claims.len() {
            for right in left + 1..claims.len() {
                let (left_path, left_line, left_column) = &claims[left];
                let (right_path, right_line, right_column) = &claims[right];
                if left_path == right_path {
                    invalid_root_paths.insert(left_path.clone());
                    continue;
                }
                let (ancestor, descendant_line, descendant_column) =
                    if right_path.starts_with(&format!("{left_path}.")) {
                        (left_path, *right_line, *right_column)
                    } else if left_path.starts_with(&format!("{right_path}.")) {
                        (right_path, *left_line, *left_column)
                    } else {
                        continue;
                    };
                invalid_root_paths.insert(left_path.clone());
                invalid_root_paths.insert(right_path.clone());
                roots_with_descendants.insert(ancestor.clone());
                overlap_sites.insert((descendant_line, Some(descendant_column)));
            }
        }
        for (root_path, info) in &mut decl.sections {
            if invalid_root_paths.contains(root_path)
                && let Some(root) = &mut info.value_root
            {
                root.valid = false;
            }
        }
        for (root_path, info) in &mut decl.duplicate_sections {
            if invalid_root_paths.contains(root_path)
                && let Some(root) = &mut info.value_root
            {
                root.valid = false;
            }
        }
        invalid.extend(overlap_sites.into_iter().map(|(line, column)| InvalidValueSite {
            id: Some(decl.id.clone()),
            file: decl.file.clone(),
            line,
            column,
            message: "embedded value roots may not be nested or overlap".to_string(),
            source: decl.source.clone(),
            binding_namespace: None,
            binding_section: None,
        }));

        // A whole-declaration authority owns its complete section tree; an
        // embedded mark inside it is invalid and never competes for ownership.
        let whole_value = value_declaration_is_in_home(config, path, &decl.id);
        for (root_path, root_line) in roots {
            let mut reasons: Vec<(usize, Option<usize>, String)> = Vec::new();
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
            // The nested marker is the complete overlap finding; do not also
            // reinterpret its subtree as malformed outer-root children
            // (§FS-values.2.4).
            if roots_with_descendants.contains(&root_path) {
                invalid.extend(reasons.into_iter().map(|(line, column, message)| {
                    InvalidValueSite {
                        id: Some(decl.id.clone()),
                        file: decl.file.clone(),
                        line,
                        column,
                        message,
                        source: decl.source.clone(),
                        binding_namespace: None,
                        binding_section: None,
                    }
                }));
                continue;
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
                .and_then(|(line, _, _, block_comment)| {
                    markdown_component(line, config)
                        .map(|(title, column)| (title, column, *block_comment))
                })
                .and_then(|(title, _, block_comment)| {
                    component_without_block_close(title, block_comment)
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
                        .and_then(|(line, _, docstring, block_comment)| {
                            authored_heading_level(
                                line,
                                is_md || *docstring,
                                *block_comment,
                                config,
                            )
                        })
                        .is_some_and(|level| level <= root_level)
                })
                .map(|line| line - 1)
                .unwrap_or(decl.body_end);

            let mut expected = 1usize;
            let mut component_candidates = 0usize;
            let mut updates = Vec::new();
            let duplicate_lines = decl
                .duplicate_sections
                .iter()
                .filter(|(path, _)| path.starts_with(&format!("{root_path}.")))
                .map(|(_, info)| info.line)
                .collect::<BTreeSet<_>>();
            for line_no in root_line.saturating_add(1)..=subtree_end {
                let Some((line, column_offset, docstring, block_comment)) =
                    normalized.get(line_no - 1)
                else {
                    continue;
                };
                let markdown = is_md || *docstring;
                let content = semantic_comment_content(line, markdown, *block_comment, config);
                if content.is_empty() || matches!(content, "/*" | "/**" | "/*!" | "*" | "*/") {
                    continue;
                }
                let child = authored_numeric_heading_path(
                    line,
                    markdown,
                    *block_comment,
                    config,
                );
                let level = authored_heading_level(line, markdown, *block_comment, config);
                let expected_path = format!("{root_path}.{expected}");
                let immediate_numeric = child.as_deref().is_some_and(|path| {
                    path.strip_prefix(&format!("{root_path}."))
                        .is_some_and(|tail| {
                            !tail.is_empty()
                                && !tail.contains('.')
                                && tail.bytes().all(|byte| byte.is_ascii_digit())
                        })
                });
                if !immediate_numeric {
                    reasons.push((
                        line_no,
                        None,
                        format!(
                            "embedded value components must be immediate numeric headings `.1` through `.N` (expected `{expected_path}`)"
                        ),
                    ));
                    continue;
                }
                component_candidates += 1;
                // Physical position—not content validity—advances the cursor;
                // bad text cannot cascade, and `.2` then `.1` reports both sites
                // (§FS-values.2.4).
                expected += 1;
                let coordinate_valid = child.as_deref() == Some(expected_path.as_str());
                let depth_valid = level == Some(root_level + 1);
                if !coordinate_valid {
                    reasons.push((
                        line_no,
                        None,
                        format!(
                            "embedded value component coordinate must follow physical order (expected `{expected_path}`)"
                        ),
                    ));
                }
                if !depth_valid {
                    reasons.push((
                        line_no,
                        None,
                        "embedded value component heading must be exactly one authored level below its root"
                            .to_string(),
                    ));
                }
                let Some((component, column)) = markdown_component(line, config) else {
                    reasons.push((
                        line_no,
                        None,
                        "embedded value component must be a valid one-line heading title".to_string(),
                    ));
                    continue;
                };
                let component = component_without_block_close(component, *block_comment);
                if !component_text_is_valid(component) || component.contains(EMBEDDED_VALUE_MARKER) {
                    reasons.push((
                        line_no,
                        Some(column_offset + column),
                        "embedded value component must be nonempty, single-line, edge-unspaced, and contain no backtick or control character".to_string(),
                    ));
                    continue;
                }
                if coordinate_valid && depth_valid && !duplicate_lines.contains(&line_no) {
                    updates.push((
                        expected_path,
                        authored_component(component, column_offset + column),
                    ));
                }
            }
            if component_candidates == 0 {
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
