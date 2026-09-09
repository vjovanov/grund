/// Enrollment helpers for JSON value members discovered by the value scanner
/// (§FS-values.2.2, §AR-scanner.2.1, §AR-scanner.3).

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
