/// First-class value inputs: Markdown component validation, exact authored
/// binding recognition, and home-derived JSON catalog enrollment
/// (§FS-values.2, §FS-values.3, §AR-scanner.2.1–§AR-scanner.3).

fn value_declaration_is_in_home(config: &Config, path: &Path, id: &Id) -> bool {
    config.kinds.iter().any(|kind| {
        kind.values
            && kind.kind == id.kind
            && match (kind.file.as_deref(), kind.folder.as_deref()) {
                (Some(file), _) => paths_same_location(path, &config.root.join(file)),
                (_, Some(folder)) => path_starts_with(path, &config.root.join(folder)),
                _ => false,
            }
    })
}

/// Validate opted-in Markdown declarations after body spans are known, so a
/// later sibling heading cannot accidentally become a value field
/// (§FS-values.2.1).
fn validate_markdown_value_declarations(
    path: &Path,
    text: &str,
    is_md: bool,
    config: &Config,
    findings: &mut Findings,
) {
    if !config.kinds.iter().any(|kind| kind.values) {
        return;
    }
    let lines = text.lines().collect::<Vec<_>>();
    let mut invalid = Vec::new();
    for decl in findings.declarations.values_mut().flatten().filter(|decl| {
        paths_same_location(&decl.file, path)
            && value_declaration_is_in_home(config, path, &decl.id)
    }) {
        let invalid_id = decl.id.clone();
        let invalid_file = decl.file.clone();
        let invalid_source = decl.source.clone();
        let invalid_for = |line, reason: &str| InvalidValueSite {
            id: Some(invalid_id.clone()),
            file: invalid_file.clone(),
            line,
            column: None,
            message: reason.to_string(),
            source: invalid_source.clone(),
            binding_namespace: None,
            binding_section: None,
        };
        let mut valid = true;
        if !is_md {
            valid = false;
            invalid.push(invalid_for(
                decl.line,
                "value declarations in text homes must be Markdown headings",
            ));
        }
        for line_no in decl.body_start.saturating_add(1)..=decl.body_end {
            if lines
                .get(line_no.saturating_sub(1))
                .is_some_and(|line| empty_citable_value_heading(line, decl.heading_level, config))
            {
                valid = false;
                invalid.push(invalid_for(
                    line_no,
                    "value component heading must contain a nonempty component",
                ));
            }
        }
        let mut fields = decl
            .sections
            .iter_mut()
            .filter(|(_, info)| (decl.body_start..=decl.body_end).contains(&info.line))
            .collect::<Vec<_>>();
        fields.sort_by_key(|(_, info)| info.line);
        if fields.is_empty() {
            valid = false;
            invalid.push(invalid_for(
                decl.line,
                "value declaration must define contiguous numbered components starting at `.1`",
            ));
        }
        for (index, (coordinate, info)) in fields.into_iter().enumerate() {
            let expected = index + 1;
            if coordinate != &expected.to_string()
                || info.heading_level != decl.heading_level + 1
            {
                valid = false;
                invalid.push(invalid_for(
                    info.line,
                    &format!(
                        "value components must be immediate contiguous headings `.1` through `.N` (expected `.{expected}`)"
                    ),
                ));
                continue;
            }
            let Some(line) = lines.get(info.line.saturating_sub(1)) else {
                valid = false;
                continue;
            };
            match markdown_component(line, config) {
                Some((component, column)) if component_text_is_valid(component) => {
                    info.value = Some(authored_component(component, column));
                }
                _ => {
                    valid = false;
                    invalid.push(invalid_for(
                        info.line,
                        "value component must be nonempty, single-line, edge-unspaced, and contain no backtick or control character",
                    ));
                }
            }
        }
        for (_, duplicate) in decl
            .duplicate_sections
            .iter()
            .filter(|(_, info)| (decl.body_start..=decl.body_end).contains(&info.line))
        {
            valid = false;
            invalid.push(invalid_for(
                duplicate.line,
                "value component coordinate is duplicated",
            ));
        }
        decl.value_valid = Some(valid);
    }
    findings.invalid_value_declarations.extend(invalid);
}

fn empty_citable_value_heading(line: &str, declaration_level: usize, config: &Config) -> bool {
    let trimmed = line.trim_start();
    let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if level <= declaration_level
        || !trimmed[level..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        return false;
    }
    let rest = trimmed[level..].trim_start();
    let token = rest.split_whitespace().next().unwrap_or("");
    let numeric = token
        .strip_suffix('.')
        .unwrap_or(token)
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    let named = config.grammar.named_sections
        && token.strip_suffix(':').is_some_and(|coordinate| {
            coordinate.split('.').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            })
        });
    (numeric || named) && rest[token.len()..].trim().is_empty()
}

fn markdown_component<'a>(line: &'a str, config: &Config) -> Option<(&'a str, usize)> {
    let captures = config.grammar.section_re.captures(line)?;
    let coordinate = captures.name("sec")?;
    // Numeric heading punctuation is optional and intentionally sits outside
    // the `sec` capture; it delimits the title but is not part of it
    // (§FS-values.2.1).
    let tail = line[coordinate.end()..].strip_prefix('.').unwrap_or(&line[coordinate.end()..]);
    let component = tail.trim_start_matches([' ', '\t']);
    let start = line.len() - component.len();
    Some((component, start + 1))
}

/// Record only exact authored bindings in Markdown or a recognized whole-line
/// source comment. The citation remains in `Findings::citations`; this record
/// adds the component and binding span without a second resolver
/// (§FS-values.3, §DA-explicit-value-bindings.2).
fn scan_value_bindings(
    line: &CitationLine<'_>,
    workspace_targets: &[WorkspaceCitationTarget],
    citation_start: usize,
    findings: &mut Findings,
) {
    let Some(context) = value_binding_context(line) else {
        return;
    };
    let mut bindings = Vec::new();
    let mut invalid = Vec::new();
    let mut classified_openings = BTreeSet::new();
    for citation in &findings.citations[citation_start..] {
        if !citation.has_marker {
            continue;
        }
        let marker_start = citation
            .column
            .saturating_sub(line.column_offset)
            .saturating_sub(1);
        let token_end = marker_start.saturating_add(citation.text.len());
        let Some(prefix) = line.scan_line.get(..marker_start) else {
            continue;
        };
        let Some(before_marker) = prefix.strip_suffix(" (") else {
            continue;
        };
        if !before_marker.ends_with('`') {
            if let Some(open_tick) = unmatched_open_tick(before_marker) {
                if !binding_span_is_inside(context, open_tick, token_end) {
                    continue;
                }
                classified_openings.insert(open_tick);
                invalid.push(invalid_value_binding_site(
                    line,
                    citation.namespace.clone(),
                    Some(citation.id.clone()),
                    citation.section.clone(),
                    open_tick,
                ));
            }
            continue;
        }
        let Some(close_tick) = before_marker.len().checked_sub(1) else {
            continue;
        };
        if before_marker.as_bytes().get(close_tick) != Some(&b'`') {
            continue;
        }
        let Some(open_tick) = before_marker[..close_tick].rfind('`') else {
            continue;
        };
        let literal = &before_marker[open_tick + 1..close_tick];
        let closes = line.scan_line.get(token_end..).is_some_and(|tail| tail.starts_with(')'));
        let binding_end = token_end.saturating_add(usize::from(closes));
        if !binding_span_is_inside(context, open_tick, binding_end) {
            continue;
        }
        let section = citation.section.as_deref();
        let valid_section = section.is_some_and(|section| {
            section.split('.').all(|part| {
                !part.is_empty()
                    && !part.starts_with('0')
                    && part.bytes().all(|byte| byte.is_ascii_digit())
            })
        });
        if citation.shorthand {
            // The ordinary noncanonical-shorthand finding owns this site and
            // suppresses value handling (§FS-values.5.1).
            classified_openings.insert(open_tick);
            continue;
        }
        if !closes
            || !valid_section
            || !component_text_is_valid(literal)
        {
            classified_openings.insert(open_tick);
            invalid.push(invalid_value_binding_site(
                line,
                citation.namespace.clone(),
                Some(citation.id.clone()),
                citation.section.clone(),
                open_tick,
            ));
            continue;
        }
        classified_openings.insert(open_tick);
        bindings.push(ValueBinding {
            namespace: citation.namespace.clone(),
            id: citation.id.clone(),
            section: section.unwrap().to_string(),
            authored: authored_component(literal, line.column_offset + open_tick + 2),
            file: citation.file.clone(),
            line: citation.line,
            column: line.column_offset + open_tick + 1,
        });
    }
    scan_noncanonical_value_binding_attempts(
        line,
        workspace_targets,
        context,
        &classified_openings,
        &mut invalid,
    );
    findings.value_bindings.extend(bindings);
    findings.invalid_value_bindings.extend(invalid);
}

/// Find a backtick-delimited literal followed immediately by a reference to an
/// opted-in value kind even when punctuation, spacing, marker, or field syntax
/// kept the ordinary citation scanner from producing the exact binding record.
/// Unbackticked adjacent prose and bare citations deliberately never enter this
/// pass (§FS-values.3.1).
fn scan_noncanonical_value_binding_attempts(
    line: &CitationLine<'_>,
    workspace_targets: &[WorkspaceCitationTarget],
    context: (usize, usize),
    classified_openings: &BTreeSet<usize>,
    invalid: &mut Vec<InvalidValueSite>,
) {
    for (close_tick, _) in line.scan_line.match_indices('`') {
        if close_tick < context.0 || close_tick >= context.1 {
            continue;
        }
        let Some(tail) = line.scan_line.get(close_tick + 1..context.1) else {
            continue;
        };
        let Some(target) = attempted_value_target(tail, line.config, workspace_targets) else {
            continue;
        };
        // Without an opener on this physical line, this is the closing half of
        // a multiline literal—the binding is still an invalid attempted
        // delimited form (§FS-values.3.1).
        let open_tick = line.scan_line[..close_tick].rfind('`').unwrap_or(close_tick);
        if !binding_span_is_inside(context, open_tick, close_tick + 1) {
            continue;
        }
        if classified_openings.contains(&open_tick) {
            continue;
        }
        invalid.push(invalid_value_binding_site(
            line,
            target.namespace,
            Some(target.id),
            target.section,
            open_tick,
        ));
    }
}

fn unmatched_open_tick(prefix: &str) -> Option<usize> {
    prefix
        .match_indices('`')
        .fold(None, |opening, (index, _)| opening.map_or(Some(index), |_| None))
}

fn invalid_value_binding_site(
    line: &CitationLine<'_>,
    namespace: Option<String>,
    id: Option<Id>,
    section: Option<String>,
    open_tick: usize,
) -> InvalidValueSite {
    InvalidValueSite {
        id,
        file: line.path.to_path_buf(),
        line: line.lineno,
        column: Some(line.column_offset + open_tick + 1),
        message: "value binding must be exactly `literal` (marker-prefixed full value ID with one positive numeric field)"
            .to_string(),
        source: DeclarationSource::Text,
        binding_namespace: namespace,
        binding_section: section,
    }
}

struct AttemptedValueTarget {
    namespace: Option<String>,
    id: Id,
    section: Option<String>,
}

fn attempted_value_target(
    tail: &str,
    local: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Option<AttemptedValueTarget> {
    let mut rest = tail;
    if rest.starts_with(|ch: char| ch.is_whitespace()) {
        rest = rest.trim_start_matches(|ch: char| ch.is_whitespace());
    }
    if let Some(trimmed) = rest.strip_prefix('(') {
        rest = trimmed;
    }
    rest = rest.trim_start_matches(|ch: char| ch.is_whitespace());
    rest = rest.strip_prefix(&local.marker).unwrap_or(rest);

    if let Some(prefix) = QUALIFIED_CITATION_PREFIX.captures(rest) {
        let alias = prefix.name("namespace")?.as_str();
        let target = workspace_targets.iter().find(|target| target.alias == alias)?;
        let id_rest = &rest[prefix.get(0)?.end()..];
        let (id, section) = attempted_value_id(id_rest, &target.config)?;
        return Some(AttemptedValueTarget {
            namespace: Some(alias.to_string()),
            id,
            section,
        });
    }

    let (id, section) = attempted_value_id(rest, local)?;
    Some(AttemptedValueTarget {
        namespace: None,
        id,
        section,
    })
}

fn attempted_value_id(raw: &str, config: &Config) -> Option<(Id, Option<String>)> {
    if let Some(parsed) = parse_longest_id_prefix(raw, &config.grammar) {
        return Some((parsed.id, parsed.section));
    }
    // A named address is deliberately outside the ordinary numeric-only
    // grammar, so recover only the complete ID before the separator to classify
    // the delimited form as an attempted value binding (§FS-values.3.1).
    raw.match_indices(&config.section_separator)
        .map(|(index, _)| index)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find_map(|index| match parse_id_arg(&raw[..index], &config.grammar) {
            Ok((id, None)) => Some((id, None)),
            _ => None,
        })
}

fn enroll_json_member(
    config: &Config,
    path: &Path,
    owners: &[&KindConfig],
    text: &str,
    member: JsonMember,
    findings: &mut Findings,
) {
    let parsed = parse_id_arg(&member.key.decoded, &config.grammar);
    let Ok((id, None)) = parsed else {
        push_json_invalid(
            findings,
            None,
            path,
            text,
            member.key.span,
            "JSON value key must be a full unqualified local ID",
        );
        return;
    };
    if !owners.iter().any(|owner| id.kind == owner.kind) {
        let expected = owners
            .iter()
            .map(|owner| format!("`{}`", owner.kind))
            .collect::<Vec<_>>()
            .join(" or ");
        push_json_invalid(
            findings,
            Some(id),
            path,
            text,
            member.key.span,
            &format!("JSON value key must belong to owning kind {expected}"),
        );
        return;
    }
    let member_line = json_line_column(text, member.key.span.start).0;
    let key_column = json_line_column(text, member.key.span.start).1;
    let member_slice = text[member.span.start..member.span.end].to_string();
    let mut declaration = Declaration {
        id: id.clone(),
        file: path.to_path_buf(),
        line: member_line,
        heading_level: 1,
        sections: BTreeMap::new(),
        duplicate_sections: Vec::new(),
        is_stub: false,
        defined_in: None,
        e2e_case: None,
        title: None,
        body_start: member_line,
        body_end: json_line_column(text, member.span.end).0,
        source: DeclarationSource::Json {
            member_slice,
            key_column,
            key_text: text[member.key.span.start..member.key.span.end].to_string(),
        },
        value_valid: Some(true),
    };
    let JsonNode::Array(elements, _) = member.value else {
        declaration.value_valid = Some(false);
        push_json_invalid(
            findings,
            Some(id.clone()),
            path,
            text,
            member.value.span(),
            "JSON value member must be a nonempty array",
        );
        findings.declarations.entry(id).or_default().push(declaration);
        return;
    };
    if elements.is_empty() {
        declaration.value_valid = Some(false);
        push_json_invalid(
            findings,
            Some(id.clone()),
            path,
            text,
            member.span,
            "JSON value array must not be empty",
        );
    }
    for (index, element) in elements.into_iter().enumerate() {
        let span = element.span();
        let (line, column) = json_line_column(text, span.start);
        let source_slice = text[span.start..span.end].to_string();
        let value = match element {
            JsonNode::Number(decoded, _) => Some(ValueComponent {
                decoded,
                kind: ValueComponentKind::Number,
                source_slice,
                column,
            }),
            JsonNode::String(string) if component_text_is_valid(&string.decoded) => {
                Some(ValueComponent {
                    decoded: string.decoded,
                    kind: ValueComponentKind::String,
                    source_slice,
                    column,
                })
            }
            _ => None,
        };
        if value.is_none() {
            declaration.value_valid = Some(false);
            push_json_invalid(
                findings,
                Some(id.clone()),
                path,
                text,
                span,
                "JSON value component must be a number or a nonempty edge-unspaced string without backticks or control characters",
            );
        }
        declaration.sections.insert(
            (index + 1).to_string(),
            SectionInfo {
                title: value.as_ref().map(|value| value.decoded.clone()).unwrap_or_default(),
                line,
                heading_level: 2,
                value,
                value_root: None,
            },
        );
    }
    findings.declarations.entry(id).or_default().push(declaration);
}

fn push_json_invalid(
    findings: &mut Findings,
    id: Option<Id>,
    path: &Path,
    text: &str,
    span: JsonSpan,
    message: &str,
) {
    let (line, column) = json_line_column(text, span.start);
    findings.invalid_value_declarations.push(InvalidValueSite {
        id,
        file: path.to_path_buf(),
        line,
        column: Some(column),
        message: message.to_string(),
        source: DeclarationSource::Json {
            member_slice: text[span.start..span.end].to_string(),
            key_column: column,
            key_text: text[span.start..span.end].to_string(),
        },
        binding_namespace: None,
        binding_section: None,
    });
}
