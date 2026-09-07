/// The independent explicit-value checker pass (§AR-checker.2.18,
/// §FS-values.5). It consumes scanner records, resolves through the same
/// workspace catalog as ordinary citations, and compares only one valid,
/// unique numbered target.
fn check_values(
    findings: &Findings,
    config: &Config,
    path_config: &Config,
    workspace: &BTreeMap<String, WorkspaceCheckTarget<'_>>,
    report: &mut CheckReport,
) {
    for site in &findings.invalid_value_declarations {
        if site.id.as_ref().is_some_and(|id| {
            findings
                .declarations
                .get(id)
                .is_some_and(|decls| value_homes(decls, &config.root).len() > 1)
        }) {
            continue;
        }
        report.errors.push(Diagnostic {
            code: "invalid-value-declaration",
            path: Some(site.file.clone()),
            line: Some(site.line),
            column: site.column,
            message: match &site.id {
                Some(id) => format!(
                    "invalid value declaration for {}: {}",
                    render_id(config, id),
                    site.message
                ),
                None => format!("invalid value declaration: {}", site.message),
            },
            sites: Vec::new(),
        });
    }
    for site in &findings.invalid_value_bindings {
        report.errors.push(Diagnostic {
            code: "invalid-value-binding",
            path: Some(site.file.clone()),
            line: Some(site.line),
            column: site.column,
            message: format!("invalid value binding: {}", site.message),
            sites: Vec::new(),
        });
    }
    for binding in &findings.value_bindings {
        let target = match binding.namespace.as_deref() {
            Some(alias) => workspace.get(alias).map(|target| WorkspaceCheckTarget {
                findings: target.findings,
                config: target.config,
            }),
            None => Some(WorkspaceCheckTarget { findings, config }),
        };
        let Some(target) = target else { continue };
        if !kind_uses_values(target.config, &binding.id.kind) {
            continue;
        }
        let Some(decls) = target.findings.declarations.get(&binding.id) else {
            continue;
        };
        let homes = value_homes(decls, &target.config.root);
        if homes.len() != 1 {
            continue;
        }
        let declaration = homes[0];
        if declaration.value_valid != Some(true)
            || declaration
                .duplicate_sections
                .iter()
                .any(|(section, _)| section == &binding.section)
        {
            continue;
        }
        let Some(section) = declaration.sections.get(&binding.section) else {
            continue;
        };
        let Some(declared) = section.value.as_ref() else {
            continue;
        };
        if value_components_equal(&binding.authored, declared) {
            continue;
        }
        let coordinate = format!(
            "{}{}{}",
            render_qualified_id(target.config, binding.namespace.as_deref(), &binding.id),
            target.config.section_separator,
            binding.section
        );
        let declared_site = format!(
            "{}:{}",
            display_path(path_config, &declaration.file),
            section.line
        );
        report.errors.push(Diagnostic {
            code: "value-mismatch",
            path: Some(binding.file.clone()),
            line: Some(binding.line),
            column: Some(binding.column),
            message: format!(
                "value mismatch for {coordinate}: bound `{}`, declared `{}` at {declared_site}",
                binding.authored.decoded, declared.decoded
            ),
            sites: vec![Site {
                path: declaration.file.clone(),
                line: section.line,
            }],
        });
    }
}

fn value_homes<'a>(decls: &'a [Declaration], root: &Path) -> Vec<&'a Declaration> {
    decls
        .iter()
        .filter(|decl| !is_stub_for_inline_decl(root, decl, decls))
        .collect()
}
