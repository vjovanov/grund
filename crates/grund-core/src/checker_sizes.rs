/// Opt-in point-lead budget checking (§FS-check.4.13).
///
/// The scanner owns the site set and `show_body.rs` owns slicing/counting. This
/// pass only applies the configured strict threshold and constructs the fixed
/// warning, keeping CLI and LSP on the same checker path.
fn check_oversized_leads(
    findings: &Findings,
    config: &Config,
    current_alias: Option<&str>,
    overlays: &TextOverlays,
    report: &mut CheckReport,
) {
    let Some(warning) = config.lead_size_warning else {
        return;
    };
    let mut cache = PointBodyCache::new(overlays);
    for (id, declarations) in &findings.declarations {
        let homes = declarations
            .iter()
            .filter(|decl| !is_stub_for_inline_decl(&config.root, decl, declarations));
        for declaration in homes {
            check_oversized_lead_site(
                &mut cache,
                config,
                current_alias,
                id,
                declaration,
                None,
                warning,
                report,
            );

            for (section, info) in &declaration.sections {
                check_oversized_lead_site(
                    &mut cache,
                    config,
                    current_alias,
                    id,
                    declaration,
                    Some((section.as_str(), info)),
                    warning,
                    report,
                );
            }
            for (section, info) in &declaration.duplicate_sections {
                check_oversized_lead_site(
                    &mut cache,
                    config,
                    current_alias,
                    id,
                    declaration,
                    Some((section.as_str(), info)),
                    warning,
                    report,
                );
            }
        }
    }
}

/// Judge and, when needed, report one declaration or section site using the
/// exact fixed warning contract (§FS-check.4.13).
#[allow(clippy::too_many_arguments)]
fn check_oversized_lead_site(
    cache: &mut PointBodyCache<'_>,
    config: &Config,
    current_alias: Option<&str>,
    id: &Id,
    declaration: &Declaration,
    section: Option<(&str, &SectionInfo)>,
    warning: LeadSizeWarning,
    report: &mut CheckReport,
) {
    let Ok(Some((lead, _))) = point_body_pair(cache, config, id, declaration, section) else {
        // A retained stub is broken and intentionally has no measurement. Any
        // concurrent read failure was already represented by the scan result.
        return;
    };
    let actual = measure_point_text(&lead, warning.unit);
    if actual <= warning.max {
        return;
    }
    let mut coordinate = render_id(config, id);
    if let Some((section, _)) = section {
        coordinate.push_str(&config.section_separator);
        coordinate.push_str(section);
    }
    if let Some(alias) = current_alias {
        coordinate = format!("{alias}/{coordinate}");
    }
    report.warnings.push(Diagnostic {
        code: "oversized-lead",
        path: Some(declaration.file.clone()),
        line: Some(
            section
                .map(|(_, info)| info.line)
                .unwrap_or(declaration.line),
        ),
        column: None,
        message: format!(
            "{coordinate} lead is {actual} {}, over the configured maximum of {}; move detail into citable child sections, or promote a child section to its own ID after running grund refs {coordinate} --summary",
            warning.unit.as_str(),
            warning.max,
        ),
        sites: Vec::new(),
    });
}
