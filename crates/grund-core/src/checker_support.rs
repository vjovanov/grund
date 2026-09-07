/// §FS-check.3.1: the dangling message. A near same-kind ID is a likely typo; a
/// Markdown inline-code context is a likely illustration. Offer whichever
/// applies — and both when a dangling citation in backticks also has a near
/// match. Outside inline code the escape hint is withheld so a prose typo is
/// nudged toward the near ID, not toward escaping.
fn dangling_message(
    config: &Config,
    namespace: Option<&str>,
    findings: &Findings,
    missing: &Id,
    in_inline_code: bool,
) -> String {
    let unknown = render_qualified_id(config, namespace, missing);
    let near = nearest_declared_id(config, namespace, findings, missing);
    let escape = in_inline_code.then(|| format!("<{}>{unknown}", config.marker));
    match (near, escape) {
        (Some(near), Some(escape)) => format!(
            "unknown reference {unknown}; did you mean {near}? (or write {escape} if this is an illustration)"
        ),
        (Some(near), None) => format!("unknown reference {unknown}; did you mean {near}?"),
        (None, Some(escape)) => {
            format!("unknown reference {unknown}; write {escape} if this is an illustration")
        }
        (None, None) => format!("unknown reference {unknown}"),
    }
}

/// §FS-check.3.1 / §FS-check.4.12: a missing declaration in a fetch-enabled
/// home. Existing typo/illustration hints replace the fetch action while the
/// snapshot-specific base and fixed finding class remain.
fn missing_snapshot_message(
    config: &Config,
    namespace: Option<&str>,
    findings: &Findings,
    missing: &Id,
    in_inline_code: bool,
    home: &str,
    must: bool,
) -> String {
    let rendered = render_qualified_id(config, namespace, missing);
    let base = if must {
        format!("unknown reference {rendered}; no snapshot in {home}")
    } else {
        format!("no snapshot for {rendered} in {home}")
    };
    let near = nearest_declared_id(config, namespace, findings, missing);
    let escape = in_inline_code.then(|| format!("<{}>{rendered}", config.marker));
    match (near, escape) {
        (Some(near), Some(escape)) => format!(
            "{base}; did you mean {near}? (or write {escape} if this is an illustration)"
        ),
        (Some(near), None) => format!("{base}; did you mean {near}?"),
        (None, Some(escape)) => format!("{base}; write {escape} if this is an illustration"),
        (None, None) => format!("{base} — run grund fetch {rendered}"),
    }
}

/// §FS-check.3.1: whether a citation site sits inside a Markdown inline-code
/// span, the signal that a dangling `§`-citation may be an illustration. Only
/// the rare dangling path asks, so this re-reads the one line rather than
/// widening every `Citation`; source files (columns shifted by stripped comment
/// prefixes) never qualify. Any read/bounds failure yields `false` — no hint.
fn citation_in_markdown_inline_code(cite: &Citation) -> bool {
    if cite.file.extension().and_then(|e| e.to_str()) != Some("md") {
        return false;
    }
    let Ok(text) = fs::read_to_string(&cite.file) else {
        return false;
    };
    let Some(line) = text.lines().nth(cite.line.saturating_sub(1)) else {
        return false;
    };
    let pos = cite.column.saturating_sub(1);
    pos <= line.len() && is_inside_inline_code(line, pos)
}

fn nearest_declared_id(
    config: &Config,
    namespace: Option<&str>,
    findings: &Findings,
    missing: &Id,
) -> Option<String> {
    let missing_text = render_id(config, missing);
    let mut best: Option<(usize, String)> = None;
    for candidate in findings.declarations.keys() {
        if candidate.kind != missing.kind {
            continue;
        }
        let candidate_text = render_id(config, candidate);
        let distance = edit_distance(&missing_text, &candidate_text);
        if !close_enough_for_hint(
            distance,
            missing_text.chars().count(),
            candidate_text.chars().count(),
        ) {
            continue;
        }
        let rendered = render_qualified_id(config, namespace, candidate);
        match &best {
            Some((best_distance, best_rendered))
                if distance > *best_distance
                    || (distance == *best_distance && rendered >= *best_rendered) => {}
            _ => best = Some((distance, rendered)),
        }
    }
    best.map(|(_, rendered)| rendered)
}

fn close_enough_for_hint(distance: usize, left_len: usize, right_len: usize) -> bool {
    distance > 0 && distance <= 3 && distance * 3 <= left_len.max(right_len)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];

    for (i, left_char) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[j] + usize::from(left_char != *right_char);
            let insertion = current[j] + 1;
            let deletion = previous[j + 1] + 1;
            current[j + 1] = substitution.min(insertion).min(deletion);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
}

fn section_depth(section_path: &str) -> usize {
    section_path.split('.').count()
}

fn heading_marks(level: usize) -> String {
    "#".repeat(level)
}

fn target_for_citation<'a>(
    cite: &Citation,
    local: &'a Findings,
    local_config: &'a Config,
    workspace: &'a BTreeMap<String, WorkspaceCheckTarget<'a>>,
) -> Option<WorkspaceCheckTarget<'a>> {
    match cite.namespace.as_deref() {
        Some(namespace) => workspace.get(namespace).map(|target| WorkspaceCheckTarget {
            findings: target.findings,
            config: target.config,
        }),
        None => Some(WorkspaceCheckTarget {
            findings: local,
            config: local_config,
        }),
    }
}

fn citation_resolves(
    cite: &Citation,
    local: &Findings,
    local_config: &Config,
    workspace: &BTreeMap<String, WorkspaceCheckTarget<'_>>,
) -> bool {
    target_for_citation(cite, local, local_config, workspace)
        .map(|target| target.findings.declarations.contains_key(&cite.id))
        .unwrap_or(false)
}

/// Put diagnostics in the one fixed order `grund` ever prints them in — by path, then
/// line, then message text — so two runs over the same tree agree byte-for-byte
/// (§FS-errors.4) and ordering is not a knob (§FS-non-goals.9).
fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(diagnostic_cmp);
}

fn diagnostic_cmp(a: &Diagnostic, b: &Diagnostic) -> std::cmp::Ordering {
    (
        a.path.as_ref().map(|p| sort_path_key(p)),
        a.line.unwrap_or(0),
        a.message.as_str(),
    )
        .cmp(&(
            b.path.as_ref().map(|p| sort_path_key(p)),
            b.line.unwrap_or(0),
            b.message.as_str(),
        ))
}
