/// One project in scope for a query command — an alias, the loaded config,
/// and the scanner's findings + scan errors for that project's tree.
/// Mirrors `ProjectScan` in `checker_cmd.rs`; kept here as the shared shape
/// every query command consumes (§AR-workspace.8).
struct WorkspaceProject {
    alias: String,
    config: Config,
    findings: Findings,
    scan_errors: Vec<ScanError>,
}

/// Everything a workspace-aware query command needs (§FS-workspace.8 intro,
/// §AR-workspace.8): every loaded project (the current one plus, when running
/// at the workspace root, every member configured under `[workspace]`), an
/// optional index naming the project unqualified IDs resolve against, and the
/// canonical render-root used for `[output] relative_paths`.
///
/// Member-local and standalone runs collapse to one project at index `0` with
/// `workspace_loaded == false` — every command can route through one struct.
struct WorkspaceContext {
    projects: Vec<WorkspaceProject>,
    /// Index into `projects` for the "current project" — what `<ID>` (no
    /// alias) resolves against (§FS-workspace.8 intro). `None` only for a
    /// workspace-root run with `include_root = false`, where there is no root
    /// project for unqualified lookups (§FS-workspace.8 intro).
    current: Option<usize>,
    /// `true` only when a `[workspace]` block was discovered AND the
    /// invocation actually loads the workspace (i.e. not pinned member-local
    /// by an explicit path inside a member). When `false`, `projects` is a
    /// single entry and qualified `alias/<ID>` lookups must fail with
    /// `unknown project alias <name>`.
    workspace_loaded: bool,
    /// The repository root used for path rendering in workspace mode (the
    /// `[output] relative_paths` base). For workspace mode this is the
    /// workspace root; for single-project mode it equals
    /// `projects[current].config.root`. Used by `fmt --cross-refs` to
    /// compute a relative URL that spans projects (§FS-workspace.8.5).
    render_root: PathBuf,
    /// The config that owns the render root. In workspace mode this is the
    /// root workspace config even when `include_root = false`; commands use it
    /// for output format and path rendering without pretending it is a loaded
    /// project.
    render_config: Config,
}

impl WorkspaceContext {
    fn current_project(&self) -> Option<&WorkspaceProject> {
        self.current.map(|current| &self.projects[current])
    }

    fn render_config(&self) -> &Config {
        &self.render_config
    }

    fn project_by_alias(&self, alias: &str) -> Option<&WorkspaceProject> {
        self.projects.iter().find(|project| project.alias == alias)
    }

    /// Every known alias in the workspace, in `projects` order. An empty list
    /// when `workspace_loaded == false`. Used by completions and by the
    /// "neither declared nor cited" hint in `refs` to suggest the right
    /// `--project` slug.
    fn aliases(&self) -> Vec<&str> {
        if !self.workspace_loaded {
            return Vec::new();
        }
        self.projects
            .iter()
            .map(|project| project.alias.as_str())
            .collect()
    }
}

/// Load every project a query command should see, given the same `(path,
/// path_provided)` pair every entry point already accepts. The three cases:
///
/// - **Standalone** (no `[workspace]` discovered) → one project, the
///   discovered config.
/// - **Member-local** (path resolves inside a workspace member, or the
///   discovered config is a member's own) → one project, with the member
///   config. `workspace_loaded == false`; qualified citations cannot resolve.
/// - **Workspace** (path is at the workspace root or a non-member subdir of
///   it) → root (when `include_root = true`) plus every configured member.
///   `current` is the root when it is included, otherwise `None` so
///   unqualified lookups cannot silently resolve against a member.
///
/// Discovery itself is delegated to the existing `resolve_workspace_config`
/// — this helper is strictly the "load every project that's in scope" layer
/// on top of it (§AR-workspace.5.1).
fn load_workspace_context(path: &Path, path_provided: bool) -> Result<WorkspaceContext> {
    load_workspace_context_with_overlays(path, path_provided, &TextOverlays::new(), false)
}

fn load_workspace_context_with_overlays(
    path: &Path,
    path_provided: bool,
    overlays: &TextOverlays,
    classify_citation_sources: bool,
) -> Result<WorkspaceContext> {
    let mut config = resolve_workspace_config(path)?;
    // §AR-scanner.2.4 / §AR-benchmarks: the read-only commands (`list`, `show`,
    // `refs`, `fmt`) never read citing-side classification, so they pass
    // `classify_citation_sources = false` to skip the scan post-pass. The LSP
    // snapshot passes `true` so `grund check`'s citation-direction errors
    // (`missing-citation` / `forbidden-citation`, §FS-lsp.1.1) surface in the
    // editor; `grund check` itself uses `load_workspace_projects` directly and
    // keeps the default (on). Workspace members inherit this below.
    config.classify_citation_sources = classify_citation_sources;
    // §FS-workspace.5 / §AR-workspace.6: workspace mode applies whenever
    // the discovered config carries `[workspace]` after member-scope
    // rewriting. A path that resolves member-local has already been
    // rewritten by `config_for_member_scope` to drop `workspace_declared`,
    // so this flag is the single canonical "is this a workspace run?"
    // — independent of where in the workspace the user invoked the
    // command, so `grund alias/FS-x docs/`, `grund refs FS-y .`, and
    // `grund fmt --cross-refs subdir/` all see the same workspace.
    if !config.workspace_declared {
        let (findings, scan_errors) =
            scan_tree_with_workspace_overlays(&config, Some(path), path_provided, &[], overlays)?;
        let render_root = config.root.clone();
        let render_config = config.clone();
        return Ok(WorkspaceContext {
            projects: vec![WorkspaceProject {
                alias: String::new(),
                config,
                findings,
                scan_errors,
            }],
            current: Some(0),
            workspace_loaded: false,
            render_root,
            render_config,
        });
    }

    let mut root_config = config;
    let render_root = root_config.root.clone();
    let render_config = root_config.clone();
    // §FS-workspace.8 intro: the current project is the root iff
    // `include_root = true` (the helper always emits the root first).
    let current = root_config.workspace_include_root.then_some(0);
    let projects = load_workspace_projects_with_overlays(&mut root_config, overlays)?;
    Ok(WorkspaceContext {
        projects,
        current,
        workspace_loaded: true,
        render_root,
        render_config,
    })
}

/// Load every workspace project a workspace-mode command operates on:
/// expand the configured members, derive each alias, scan each project, and
/// reparse qualified citations against the full target list (§AR-workspace.5.1).
///
/// Returns one [`WorkspaceProject`] per project in the canonical order:
/// the root first when `include_root = true`, then members in member-glob
/// order. Mutates `root_config.workspace_boundary_roots` so any subsequent
/// root scan respects the member boundary (§AR-workspace.6).
fn load_workspace_projects(root_config: &mut Config) -> Result<Vec<WorkspaceProject>> {
    load_workspace_projects_with_overlays(root_config, &TextOverlays::new())
}

fn load_workspace_projects_with_overlays(
    root_config: &mut Config,
    overlays: &TextOverlays,
) -> Result<Vec<WorkspaceProject>> {
    let member_roots = expand_workspace_members(root_config)?;
    root_config.workspace_boundary_roots = member_roots.clone();

    // Stage 1: build the (alias, config) list. Failing fast on alias errors,
    // empty workspaces, duplicates, nested workspaces, and missing members
    // before any scan keeps misconfiguration cheap to diagnose.
    let mut entries: Vec<(String, Config)> = Vec::new();
    if root_config.workspace_include_root {
        let alias = derive_alias(root_config, None, RootMode::Root).map_err(|err| {
            let message = format!("{err} for {}", project_label(root_config, &root_config.root));
            project_name_error(root_config, message)
        })?;
        entries.push((alias, root_config.clone()));
    }
    for member_root in member_roots {
        let member_config = load_config_at_with_report_base(
            &member_root,
            &root_config.cli_base,
            Some(&root_config.root),
        )?;
        // §AR-workspace.6.1: nested workspaces are rejected at load — not
        // silently flattened, so the resolver invariants stay pinned.
        if member_config.workspace_declared {
            return Err(anyhow!(
                "workspace member `{}` declares its own `[workspace]` block (nested workspaces are not supported)",
                display_path(root_config, &member_config.root)
            ));
        }
        let alias = derive_alias(&member_config, Some(&member_root), RootMode::Member)
            .map_err(|err| {
                let message = format!("{err} for {}", project_label(root_config, &member_root));
                if member_config.project_name_source.is_some() {
                    project_name_error(&member_config, message)
                } else {
                    workspace_members_error(root_config, message)
                }
            })?;
        entries.push((alias, member_config));
    }

    if entries.is_empty() {
        return Err(workspace_members_error(
            root_config,
            "workspace has no projects in scope (include_root = false and no members)"
                .to_string(),
        ));
    }

    // §FS-workspace.3 / §AR-workspace.5.3: duplicate aliases are caught at
    // load — qualified citations have a single resolver target.
    let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
    for (alias, config) in &entries {
        if let Some(first) = seen.get(alias) {
            return Err(workspace_members_error(
                root_config,
                format!(
                    "duplicate workspace project alias `{alias}` ({})",
                    duplicate_alias_sites(root_config, first, &config.root)
                ),
            ));
        }
        seen.insert(alias.clone(), config.root.clone());
    }

    // §AR-scanner.2.4: members inherit the root's classification intent, so a
    // read-only command (root off) skips the post-pass for every member and
    // `grund check` (root on) classifies the whole workspace.
    // §FS-check.1.3: `grund check --full` is a property of the run, not of one
    // project's config, so every member walks past its own `[scan] include` too.
    for (_, config) in &mut entries {
        config.classify_citation_sources = root_config.classify_citation_sources;
        config.scan_full = root_config.scan_full;
    }

    // Stage 2: build the target list up-front so each project's scan can
    // parse `§<alias>/<ID>` citations with the target's grammar inline —
    // no second disk pass (§FS-workspace.1, §AR-workspace.2).
    let targets = entries
        .iter()
        .map(|(alias, config)| WorkspaceCitationTarget {
            alias: alias.clone(),
            config: config.clone(),
        })
        .collect::<Vec<_>>();

    // Stage 3: scan every project under its own config, with the workspace
    // targets in scope. Project scans are independent once aliases and target
    // grammars are validated; sort by the original entry index before returning
    // so root/member ordering stays byte-deterministic.
    let mut indexed = if entries.len() >= 2 {
        entries
            .into_par_iter()
            .enumerate()
            .map(|(index, (alias, config))| {
                (index, load_workspace_project(alias, config, &targets, overlays))
            })
            .collect::<Vec<_>>()
    } else {
        entries
            .into_iter()
            .enumerate()
            .map(|(index, (alias, config))| {
                (index, load_workspace_project(alias, config, &targets, overlays))
            })
            .collect::<Vec<_>>()
    };
    indexed.sort_by_key(|(index, _)| *index);
    let mut projects = indexed
        .into_iter()
        .map(|(_, project)| project)
        .collect::<Result<Vec<_>>>()?;
    resolve_qualified_shorthand_citations(&mut projects);
    Ok(projects)
}

fn load_workspace_project(
    alias: String,
    config: Config,
    targets: &[WorkspaceCitationTarget],
    overlays: &TextOverlays,
) -> Result<WorkspaceProject> {
    let (findings, scan_errors) =
        scan_tree_with_workspace_overlays(&config, Some(&config.root), true, targets, overlays)?;
    Ok(WorkspaceProject {
        alias,
        config,
        findings,
        scan_errors,
    })
}

/// Split a CLI ID argument that may carry a qualifying `<alias>/` prefix
/// (§FS-workspace.1). The alias is validated against the slug grammar here,
/// before resolution; the ID tail is deliberately left raw so the caller can
/// parse it with the target project's grammar.
fn split_qualified_id_arg(raw: &str) -> Result<(Option<String>, &str)> {
    if let Some(slash) = raw.find('/') {
        let (alias, rest) = raw.split_at(slash);
        let rest = &rest[1..];
        if !is_valid_project_alias(alias) {
            return Err(anyhow!(
                "invalid project alias `{alias}` (expected [a-z][a-z0-9-]*)"
            ));
        }
        return Ok((Some(alias.to_string()), rest));
    }
    Ok((None, raw))
}
