/// The section-shape rule family: the two things `grund check` says about a
/// declaration's own numbered headings, rather than about a citation of one.
///
/// - **Heading level** (§FS-check.3.9) — the Markdown depth a heading writes must
///   mirror the dotted path it claims, as strictly as `[id] section_heading_levels`
///   asks (§FS-config.3.3).
/// - **Duplicate path** (§FS-check.3.16) — two headings claiming one path give a
///   section citation two destinations, which is §FS-check.3.3's ambiguity one
///   level down, and is reported rather than ranked (§DF-duplicate-section-path).
///
/// They sit beside `checker.rs` as one family because they read the same two
/// things and nothing else does: the recorded section map, and the
/// `duplicate_sections` list the scanner keeps beside it (§AR-scanner.2.2,
/// §AR-core-module-layout.1).

/// Both section-shape rules, as two independent passes over the declarations
/// (§AR-checker.2.15). Order does not matter — the report is sorted before it is
/// printed (§FS-errors.4).
///
/// `config` is the project being checked (it owns the ID grammar and the
/// separator); `path_config` is the one the printed report renders paths
/// against, so in a workspace a path named *inside* a message points where the
/// finding's own anchor points (§FS-config.3.6, §FS-workspace.8.1).
fn check_section_headings(
    findings: &Findings,
    config: &Config,
    path_config: &Config,
    report: &mut CheckReport,
) {
    // §FS-check.3.9 / §FS-config.3.3: in strict mode, the Markdown heading level
    // must mirror the dotted section depth so `## 1`, `### 1.1`, ...
    // communicate the same tree that `§ID.1.1` addresses.
    if matches!(config.section_heading_levels.as_str(), "strict" | "warn") {
        let target = if config.section_heading_levels == "strict" {
            &mut report.errors
        } else {
            &mut report.warnings
        };
        for (id, decls) in &findings.declarations {
            for decl in decls {
                for (section_path, section) in &decl.sections {
                    let expected_level = decl.heading_level + section_depth(section_path);
                    if section.heading_level != expected_level {
                        target.push(Diagnostic {
                            code: "section-heading-level",
                            path: Some(decl.file.clone()),
                            line: Some(section.line),
                            column: None,
                            message: format!(
                                "section {}{}{} heading level mismatch: expected {} (level {}), found {} (level {})",
                                render_id(config, id),
                                config.section_separator,
                                section_path,
                                heading_marks(expected_level),
                                expected_level,
                                heading_marks(section.heading_level),
                                section.heading_level
                            ),
                            sites: Vec::new(),
                        });
                    }
                }
            }
        }
    }
    // §FS-check.3.16: two headings inside one declaration claiming one dotted
    // section path give `§<ID>.<path>` two destinations — §3.3's ambiguity one
    // level down, reported in §3.3's shape rather than ranked
    // (§DF-duplicate-section-path.2.1). What the list holds is already scoped to
    // the declaration's own body, which is what keeps a later item's doc-comment
    // and a stub's prose out of the rule (§AR-scanner.2.2).
    for (id, decls) in &findings.declarations {
        for decl in decls {
            let mut colliding: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
            for (path, info) in &decl.duplicate_sections {
                colliding.entry(path.as_str()).or_default().push(info.line);
            }
            for (path, mut lines) in colliding {
                // The map holds the first heading (§AR-scanner.2.2), which is
                // where the finding anchors; the rest are named in the message.
                // The same insert that starts the list fills the map, so the
                // lookup always hits — but the finding does not hang on that
                // being true (§REQ-no-missed-citation): with no map entry the
                // earliest recorded claimant anchors it instead. `lines` is
                // non-empty either way, since a path is in `colliding` only
                // because a heading claimed it twice.
                lines.extend(decl.sections.get(path).map(|first| first.line));
                lines.sort_unstable();
                let sites: Vec<Site> = lines
                    .iter()
                    .map(|line| Site {
                        path: decl.file.clone(),
                        line: *line,
                    })
                    .collect();
                let others = lines[1..]
                    .iter()
                    .map(|line| format!("{}:{}", display_path(path_config, &decl.file), line))
                    .collect::<Vec<_>>()
                    .join(", ");
                report.errors.push(Diagnostic {
                    code: "duplicate-section",
                    path: Some(decl.file.clone()),
                    line: Some(lines[0]),
                    column: None,
                    message: format!(
                        "duplicate section {}{}{} (also declared at {others})",
                        render_id(config, id),
                        config.section_separator,
                        path
                    ),
                    sites,
                });
            }
        }
    }
}
