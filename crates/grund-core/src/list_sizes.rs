/// Options for the additive per-point size catalog (§FS-list.1, §FS-list.3.4).
#[derive(Clone)]
pub struct ListSizeOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub kind_filter: BTreeSet<String>,
    pub project_filter: BTreeSet<String>,
    pub unused_only: bool,
    pub units: Vec<PointSizeUnit>,
    pub top: Option<usize>,
}

impl Default for ListSizeOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            kind_filter: BTreeSet::new(),
            project_filter: BTreeSet::new(),
            unused_only: false,
            units: vec![
                PointSizeUnit::Lines,
                PointSizeUnit::Words,
                PointSizeUnit::Bytes,
            ],
            top: None,
        }
    }
}

/// One selected unit's lead/full pair, kept in caller order (§FS-list.3.4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListSizeMeasurement {
    pub unit: PointSizeUnit,
    pub lead: Option<usize>,
    pub full: Option<usize>,
}

/// One declaration or section site in the size catalog (§FS-list.2,
/// §FS-list.3.4). Unlike [`ListEntry`], it deliberately carries no title/ref
/// fields and never collapses ambiguous sites.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListSizeEntry {
    pub project: Option<String>,
    pub id: String,
    pub section: Option<String>,
    /// The owning project's configured separator, used to render the text
    /// coordinate without adding a wire field (§FS-list.3.4).
    pub section_separator: String,
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub stub: bool,
    pub defines: Option<String>,
    pub duplicate: bool,
    pub measurements: Vec<ListSizeMeasurement>,
}

/// Structured result for one deterministic size-catalog scan (§FS-list.3.4).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListSizeOutput {
    pub output_format: String,
    pub workspace: bool,
    pub entries: Vec<ListSizeEntry>,
    pub scan_errors: Vec<ApiScanError>,
}

/// Programmatic point-size catalog. It selects the same declaration set as
/// [`list`], adds scanner-recorded section sites, and measures show-identical
/// lead/full bodies through one per-file cache (§FS-list.2, §FS-list.3.4,
/// §FS-workspace.8.3).
pub fn list_sizes(opts: ListSizeOpts) -> Result<ListSizeOutput> {
    if opts.units.is_empty() {
        return Err(anyhow!("at least one size unit is required"));
    }
    if matches!(opts.top, Some(0)) {
        return Err(anyhow!("top must be positive"));
    }
    let context = load_workspace_context(&opts.path, opts.path_provided)?;
    validate_list_scope_filters(&context, &opts.project_filter, &opts.kind_filter)?;

    struct Pending<'a> {
        project_alias: &'a str,
        project_config: &'a Config,
        id: &'a Id,
        declaration: &'a Declaration,
        section: Option<(&'a str, &'a SectionInfo)>,
        duplicate: bool,
    }

    let counts = ListCitationCounts::new(&context);
    let mut pending = Vec::new();
    let mut scan_errors = Vec::new();
    for project in &context.projects {
        if !opts.project_filter.is_empty() && !opts.project_filter.contains(&project.alias) {
            continue;
        }
        scan_errors.extend(
            project
                .scan_errors
                .iter()
                .map(|(file, message)| api_scan_error(context.render_config(), file, message)),
        );
        let used_counts = counts.used_for(project.alias.as_str());
        for (id, declarations) in &project.findings.declarations {
            if !opts.kind_filter.is_empty() && !opts.kind_filter.contains(&id.kind) {
                continue;
            }
            if opts.unused_only && used_counts.get(id).copied().unwrap_or(0) > 0 {
                continue;
            }
            if opts.unused_only && id.kind == "E2E" && !opts.kind_filter.contains("E2E") {
                continue;
            }
            let mut homes: Vec<&Declaration> = declarations
                .iter()
                .filter(|decl| {
                    !is_stub_for_inline_decl(&project.config.root, decl, declarations)
                })
                .collect();
            homes.sort_by(|a, b| {
                (sort_path_key(&a.file), a.line).cmp(&(sort_path_key(&b.file), b.line))
            });
            let duplicate_declaration = homes.len() > 1;
            for declaration in homes {
                pending.push(Pending {
                    project_alias: &project.alias,
                    project_config: &project.config,
                    id,
                    declaration,
                    section: None,
                    duplicate: duplicate_declaration,
                });

                let mut section_sites: BTreeMap<&str, Vec<&SectionInfo>> = BTreeMap::new();
                for (section, info) in &declaration.sections {
                    section_sites.entry(section).or_default().push(info);
                }
                for (section, info) in &declaration.duplicate_sections {
                    section_sites.entry(section).or_default().push(info);
                }
                for (section, mut sites) in section_sites {
                    sites.sort_by_key(|info| info.line);
                    let duplicate_section = duplicate_declaration || sites.len() > 1;
                    for info in sites {
                        pending.push(Pending {
                            project_alias: &project.alias,
                            project_config: &project.config,
                            id,
                            declaration,
                            section: Some((section, info)),
                            duplicate: duplicate_section,
                        });
                    }
                }
            }
        }
    }

    // The normal order is also the complete tie-break for `--top`.
    pending.sort_by(|a, b| {
        (
            a.project_alias,
            a.id,
            a.section.map(|(section, _)| section),
            sort_path_key(&a.declaration.file),
            a.section.map(|(_, info)| info.line).unwrap_or(a.declaration.line),
        )
            .cmp(&(
                b.project_alias,
                b.id,
                b.section.map(|(section, _)| section),
                sort_path_key(&b.declaration.file),
                b.section.map(|(_, info)| info.line).unwrap_or(b.declaration.line),
            ))
    });

    let overlays = TextOverlays::new();
    let mut cache = PointBodyCache::new(&overlays);
    let render_config = context.render_config();
    let mut entries = Vec::with_capacity(pending.len());
    for row in pending {
        let bodies = point_body_pair(
            &mut cache,
            row.project_config,
            row.id,
            row.declaration,
            row.section,
        )?;
        let measurements = opts
            .units
            .iter()
            .map(|unit| ListSizeMeasurement {
                unit: *unit,
                lead: bodies
                    .as_ref()
                    .map(|(lead, _)| measure_point_text(lead, *unit)),
                full: bodies
                    .as_ref()
                    .map(|(_, full)| measure_point_text(full, *unit)),
            })
            .collect();
        let rendered = render_id(row.project_config, row.id);
        entries.push(ListSizeEntry {
            project: context
                .workspace_loaded
                .then(|| row.project_alias.to_string()),
            id: if context.workspace_loaded {
                format!("{}/{rendered}", row.project_alias)
            } else {
                rendered
            },
            section: row.section.map(|(section, _)| section.to_string()),
            section_separator: row.project_config.section_separator.clone(),
            kind: row.id.kind.clone(),
            path: display_path(render_config, &row.declaration.file),
            line: row
                .section
                .map(|(_, info)| info.line)
                .unwrap_or(row.declaration.line),
            stub: row.declaration.is_stub,
            defines: row
                .declaration
                .defined_in
                .as_ref()
                .map(|target| format_path(target)),
            duplicate: row.duplicate,
            measurements,
        });
    }

    if let Some(top) = opts.top {
        entries.retain(|entry| entry.measurements[0].lead.is_some());
        entries.sort_by(|a, b| {
            b.measurements[0]
                .lead
                .cmp(&a.measurements[0].lead)
                // `entries` entered this stable sort in normal order.
                .then(std::cmp::Ordering::Equal)
        });
        entries.truncate(top);
    }

    Ok(ListSizeOutput {
        output_format: render_config.output_format.clone(),
        workspace: context.workspace_loaded,
        entries,
        scan_errors,
    })
}

/// Shared validation for size mode's pre-measurement catalog selectors
/// (§FS-list.1, §FS-workspace.8.3).
fn validate_list_scope_filters(
    context: &WorkspaceContext,
    project_filter: &BTreeSet<String>,
    kind_filter: &BTreeSet<String>,
) -> Result<()> {
    if !project_filter.is_empty() && !context.workspace_loaded {
        return Err(anyhow!(
            "--project requires workspace mode (no [workspace] block discovered)"
        ));
    }
    for alias in project_filter {
        if context.project_by_alias(alias).is_none() {
            let known = context.aliases().join(", ");
            return if known.is_empty() {
                Err(anyhow!("unknown project alias `{alias}`"))
            } else {
                Err(anyhow!("unknown project alias `{alias}`\nknown aliases: {known}"))
            };
        }
    }
    for kind in kind_filter {
        let matched = context
            .projects
            .iter()
            .filter(|project| project_filter.is_empty() || project_filter.contains(&project.alias))
            .find_map(|project| {
                project
                    .config
                    .kinds
                    .iter()
                    .find(|candidate| &candidate.kind == kind)
            });
        if !matches!(matched, Some(candidate) if candidate.citable) {
            let headline = match matched {
                Some(candidate) => non_citable_kind_error(candidate),
                None => format!("unknown kind `{kind}`"),
            };
            let mut known = Vec::new();
            let mut seen = BTreeSet::new();
            for project in &context.projects {
                for configured in &project.config.kinds {
                    if configured.citable && seen.insert(configured.kind.clone()) {
                        known.push(configured.kind.clone());
                    }
                }
            }
            return Err(anyhow!("{headline}\nknown kinds: {}", known.join(", ")));
        }
    }
    Ok(())
}
