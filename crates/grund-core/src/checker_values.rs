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
        let target = match site.binding_namespace.as_deref() {
            Some(alias) => workspace.get(alias).map(|target| WorkspaceCheckTarget {
                findings: target.findings,
                config: target.config,
            }),
            None => Some(WorkspaceCheckTarget { findings, config }),
        };
        if site.id.as_ref().is_some_and(|id| {
            !target.is_some_and(|target| {
                binding_target_has_value_authority(
                    target.findings,
                    target.config,
                    id,
                    site.binding_section.as_deref(),
                )
            })
        }) {
            continue;
        }
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
        let Some(decls) = target.findings.declarations.get(&binding.id) else {
            continue;
        };
        let homes = value_homes(decls, &target.config.root);
        if homes.len() != 1 {
            continue;
        }
        let declaration = homes[0];
        let whole_authority = declaration.value_valid.is_some();
        let embedded = (!whole_authority)
            .then(|| embedded_root_for_binding(declaration, &binding.section))
            .flatten();
        let section = match embedded {
            Some((_, EmbeddedBindingRelation::ImmediateComponent)) => {
                let Some(section) = declaration.sections.get(&binding.section) else {
                    continue;
                };
                section
            }
            Some((_, EmbeddedBindingRelation::RootOrDescendant)) => {
                report.errors.push(invalid_binding_diagnostic(binding));
                continue;
            }
            None if whole_authority && !binding.section.contains('.') =>
            {
                if declaration.value_valid != Some(true) {
                    continue;
                }
                let Some(section) = declaration.sections.get(&binding.section) else {
                    continue;
                };
                section
            }
            None if whole_authority => {
                report.errors.push(invalid_binding_diagnostic(binding));
                continue;
            }
            None => continue,
        };
        if declaration
            .duplicate_sections
            .iter()
            .any(|(duplicate, _)| duplicate == &binding.section)
        {
            continue;
        }
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

#[derive(Clone, Copy)]
enum EmbeddedBindingRelation {
    ImmediateComponent,
    RootOrDescendant,
}

/// Find the existing marked section whose path owns this binding. Invalid roots
/// still count as authority for syntax classification, but only a valid root's
/// immediate child reaches comparison (§FS-values.3.1, §FS-values.5.1).
fn embedded_root_for_binding<'a>(
    declaration: &'a Declaration,
    section: &str,
) -> Option<(&'a EmbeddedValueRoot, EmbeddedBindingRelation)> {
    declaration.sections.iter().find_map(|(root_path, info)| {
        let root = info.value_root.as_ref()?;
        if section == root_path {
            return Some((root, EmbeddedBindingRelation::RootOrDescendant));
        }
        let remainder = section.strip_prefix(&format!("{root_path}."))?;
        let relation = if !remainder.contains('.') && root.valid {
            EmbeddedBindingRelation::ImmediateComponent
        } else if !remainder.contains('.') {
            // Invalid authority suppresses comparison and does not turn a
            // grammatically immediate binding into a second finding.
            return None;
        } else {
            EmbeddedBindingRelation::RootOrDescendant
        };
        Some((root, relation))
    })
}

fn binding_target_has_value_authority(
    findings: &Findings,
    config: &Config,
    id: &Id,
    section: Option<&str>,
) -> bool {
    if kind_uses_values(config, &id.kind) {
        return true;
    }
    let Some(section) = section else { return false };
    findings
        .declarations
        .get(id)
        .into_iter()
        .flatten()
        .any(|declaration| {
            declaration.sections.iter().any(|(root_path, info)| {
                info.value_root.is_some()
                    && (section == root_path
                        || section
                            .strip_prefix(&format!("{root_path}."))
                            .is_some_and(|remainder| !remainder.is_empty()))
            })
        })
}

fn invalid_binding_diagnostic(binding: &ValueBinding) -> Diagnostic {
    Diagnostic {
        code: "invalid-value-binding",
        path: Some(binding.file.clone()),
        line: Some(binding.line),
        column: Some(binding.column),
        message: "invalid value binding: value binding must be exactly `literal` (marker-prefixed full value ID with one positive numeric field)".to_string(),
        sites: Vec::new(),
    }
}

fn value_homes<'a>(decls: &'a [Declaration], root: &Path) -> Vec<&'a Declaration> {
    decls
        .iter()
        .filter(|decl| !is_stub_for_inline_decl(root, decl, decls))
        .collect()
}
