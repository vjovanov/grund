/// Public programmatic list types and catalog implementation (§FS-list.2,
/// §FS-list.3, §AR-bindings.2).
/// This category is included beside the API facade so the public list contract
/// can evolve without making the facade file exceed its reviewed size budget.

#[derive(Clone)]
pub struct ListOpts {
    pub path: PathBuf,
    pub path_provided: bool,
    pub kind_filter: BTreeSet<String>,
    pub project_filter: BTreeSet<String>,
    pub unused_only: bool,
}

impl Default for ListOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            path_provided: false,
            kind_filter: BTreeSet::new(),
            project_filter: BTreeSet::new(),
            unused_only: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListEntry {
    pub project: Option<String>,
    pub id: String,
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub title: Option<String>,
    pub stub: bool,
    pub defines: Option<String>,
    pub refs: usize,
    pub duplicate: bool,
    /// Embedded value authorities owned by this declaration's existing
    /// sections, omitted by serializers when empty (§FS-values.6,
    /// §FS-list.3.1).
    pub value_roots: Vec<ListValueRoot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListValueRoot {
    pub id: String,
    pub valid: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListSummary {
    pub project: Option<String>,
    pub kind: String,
    pub title: String,
    pub home: String,
    pub count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListOutput {
    pub output_format: String,
    pub workspace: bool,
    pub entries: Vec<ListEntry>,
    pub summaries: Vec<ListSummary>,
    pub scan_errors: Vec<ApiScanError>,
}

fn list_summary_home(kind: &KindConfig) -> String {
    kind.file
        .as_deref()
        .or(kind.folder.as_deref())
        .unwrap_or_default()
        .to_string()
}

/// Programmatic `list`: return the catalog and per-kind summary rows without
/// selecting text/JSON rendering or an exit code (§AR-bindings.2).
pub fn list(opts: ListOpts) -> Result<ListOutput> {
    let context = load_workspace_context(&opts.path, opts.path_provided)?;
    if !opts.project_filter.is_empty() && !context.workspace_loaded {
        return Err(anyhow!(
            "--project requires workspace mode (no [workspace] block discovered)"
        ));
    }
    for alias in &opts.project_filter {
        if context.project_by_alias(alias).is_none() {
            let known = context.aliases().join(", ");
            return if known.is_empty() {
                Err(anyhow!("unknown project alias `{alias}`"))
            } else {
                Err(anyhow!("unknown project alias `{alias}`\nknown aliases: {known}"))
            };
        }
    }
    for kind in &opts.kind_filter {
        // §FS-list.1, as in the CLI frontend: a configured but non-citable kind
        // is refused with its reason rather than selected into an empty list.
        let matched = context
            .projects
            .iter()
            .filter(|project| {
                opts.project_filter.is_empty() || opts.project_filter.contains(&project.alias)
            })
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
            let mut known: Vec<String> = Vec::new();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for project in &context.projects {
                for k in &project.config.kinds {
                    if k.citable && seen.insert(k.kind.clone()) {
                        known.push(k.kind.clone());
                    }
                }
            }
            return Err(anyhow!("{headline}\nknown kinds: {}", known.join(", ")));
        }
    }

    struct Entry<'a> {
        project_alias: &'a str,
        project_config: &'a Config,
        id: &'a Id,
        home: &'a Declaration,
        duplicate: bool,
        refs: usize,
    }

    let counts = ListCitationCounts::new(&context);
    let mut entries: Vec<Entry<'_>> = Vec::new();
    let mut scan_errors = Vec::new();
    for project in &context.projects {
        if !opts.project_filter.is_empty() && !opts.project_filter.contains(&project.alias) {
            continue;
        }
        // §FS-workspace.8.7: rendered against the run's config, like the entries
        // below, not the scanning project's — the same spelling `check` uses.
        scan_errors.extend(
            project
                .scan_errors
                .iter()
                .map(|(file, message)| api_scan_error(context.render_config(), file, message)),
        );
        let ref_counts = counts.refs_for(project.alias.as_str());
        let used_counts = counts.used_for(project.alias.as_str());
        for (id, decls) in &project.findings.declarations {
            if !opts.kind_filter.is_empty() && !opts.kind_filter.contains(&id.kind) {
                continue;
            }
            let refs = ref_counts.get(id).copied().unwrap_or(0);
            if opts.unused_only && used_counts.get(id).copied().unwrap_or(0) > 0 {
                continue;
            }
            if opts.unused_only && id.kind == "E2E" && !opts.kind_filter.contains("E2E") {
                continue;
            }
            let mut homes: Vec<&Declaration> = decls
                .iter()
                .filter(|decl| !is_stub_for_inline_decl(&project.config.root, decl, decls))
                .collect();
            homes.sort_by(|a, b| {
                (sort_path_key(&a.file), a.line).cmp(&(sort_path_key(&b.file), b.line))
            });
            let duplicate = homes.len() > 1;
            for home in homes {
                entries.push(Entry {
                    project_alias: project.alias.as_str(),
                    project_config: &project.config,
                    id,
                    home,
                    duplicate,
                    refs,
                });
            }
        }
    }
    if context.workspace_loaded {
        entries.sort_by(|a, b| {
            (a.project_alias, a.id, sort_path_key(&a.home.file), a.home.line).cmp(&(
                b.project_alias,
                b.id,
                sort_path_key(&b.home.file),
                b.home.line,
            ))
        });
    }

    let render_qualified = |entry: &Entry<'_>| -> String {
        if context.workspace_loaded {
            format!(
                "{}/{}",
                entry.project_alias,
                render_id(entry.project_config, entry.id)
            )
        } else {
            render_id(entry.project_config, entry.id)
        }
    };
    let render_config = context.render_config();
    let public_entries = entries
        .iter()
        .map(|entry| ListEntry {
            project: context
                .workspace_loaded
                .then(|| entry.project_alias.to_string()),
            id: render_qualified(entry),
            kind: entry.id.kind.clone(),
            path: display_path(render_config, &entry.home.file),
            line: entry.home.line,
            title: entry.home.title.clone(),
            stub: entry.home.is_stub,
            defines: entry.home.defined_in.as_ref().map(|target| format_path(target)),
            refs: entry.refs,
            duplicate: entry.duplicate,
            value_roots: entry
                .home
                .sections
                .iter()
                .filter_map(|(section, info)| {
                    info.value_root.as_ref().map(|root| ListValueRoot {
                        id: format!(
                            "{}{}{}",
                            render_qualified(entry),
                            entry.project_config.section_separator,
                            section
                        ),
                        valid: root.valid,
                    })
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    let mut summaries = Vec::new();
    if context.workspace_loaded {
        let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        for entry in &entries {
            *counts
                .entry((entry.project_alias.to_string(), entry.id.kind.clone()))
                .or_insert(0) += 1;
        }
        // §FS-workspace.8.3: rows sorted by alias — the same byte-wise `str`
        // order the catalog above sorts `entries` by — then by that
        // project's configured kind order.
        let mut projects: Vec<&WorkspaceProject> = context.projects.iter().collect();
        projects.sort_by(|a, b| a.alias.cmp(&b.alias));
        for project in projects {
            if !opts.project_filter.is_empty() && !opts.project_filter.contains(&project.alias) {
                continue;
            }
            for kind in &project.config.kinds {
                let count = counts
                    .get(&(project.alias.clone(), kind.kind.clone()))
                    .copied()
                    .unwrap_or(0);
                if count == 0 {
                    continue;
                }
                summaries.push(ListSummary {
                    project: Some(project.alias.clone()),
                    kind: kind.kind.clone(),
                    title: kind.title.clone().unwrap_or_else(|| "Declaration".to_string()),
                    home: list_summary_home(kind),
                    count,
                });
            }
        }
    } else {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in &entries {
            *counts.entry(&entry.id.kind).or_insert(0) += 1;
        }
        for kind in &render_config.kinds {
            let count = counts.get(kind.kind.as_str()).copied().unwrap_or(0);
            if count == 0 {
                continue;
            }
            summaries.push(ListSummary {
                project: None,
                kind: kind.kind.clone(),
                title: kind.title.clone().unwrap_or_else(|| "Declaration".to_string()),
                home: list_summary_home(kind),
                count,
            });
        }
    }

    Ok(ListOutput {
        output_format: render_config.output_format.clone(),
        workspace: context.workspace_loaded,
        entries: public_entries,
        summaries,
        scan_errors,
    })
}
