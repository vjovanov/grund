/// Formatter protection for exact value bindings. Authority comes from the
/// scanner's local/workspace records so formatting cannot turn a binding into a
/// link and silently disable comparison (§FS-values.8, §AR-scanner.2.3).

fn markdown_citation_is_value_binding(
    line: &str,
    citation: &MarkdownLineCitation,
    config: &Config,
    findings: &Findings,
    workspace: Option<&WorkspaceContext>,
) -> bool {
    let (target_config, target_findings) = match citation.namespace.as_deref() {
        Some(alias) => {
            let Some(project) = workspace.and_then(|workspace| workspace.project_by_alias(alias))
            else {
                return false;
            };
            (&project.config, &project.findings)
        }
        None => (config, findings),
    };
    let Some(section) = citation.section.as_deref() else {
        return false;
    };
    if !section.split('.').all(|part| {
        !part.is_empty()
            && !part.starts_with('0')
            && part.bytes().all(|byte| byte.is_ascii_digit())
    }) || !binding_target_has_any_value_authority(
        target_findings,
        target_config,
        &citation.id,
        section,
    ) {
        return false;
    }
    let Some(before_marker) = line[..citation.marker_start].strip_suffix(" (") else {
        return false;
    };
    let Some(close_tick) = before_marker.len().checked_sub(1) else {
        return false;
    };
    if before_marker.as_bytes().get(close_tick) != Some(&b'`')
        || !line[citation.token_end..].starts_with(')')
    {
        return false;
    }
    let Some(open_tick) = before_marker[..close_tick].rfind('`') else {
        return false;
    };
    component_text_is_valid(&before_marker[open_tick + 1..close_tick])
}
