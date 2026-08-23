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
        return single_project_context(config, path, path_provided, overlays);
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

/// The one-project context every non-workspace run collapses to: the discovered
/// config, scanned under the caller's scope, with `workspace_loaded == false` so
/// qualified `<alias>/<ID>` lookups fail loud (§FS-workspace.8 intro).
///
/// Shared by the two ways a run gets here — no `[workspace]` block at all, and a
/// scope that narrows inside one (`load_narrowable_workspace_context`) — so the
/// two cannot drift on what "single project" means.
fn single_project_context(
    config: Config,
    path: &Path,
    path_provided: bool,
    overlays: &TextOverlays,
) -> Result<WorkspaceContext> {
    let (findings, scan_errors) =
        scan_tree_with_workspace_overlays(&config, Some(path), path_provided, &[], overlays)?;
    let render_root = config.root.clone();
    let render_config = config.clone();
    Ok(WorkspaceContext {
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
    })
}

/// The loader for a command whose `<path>` argument narrows the **walk** rather
/// than only choosing which config answers — today that is `grund cover`
/// ([§FS-cover.1](FS-cover.md)).
///
/// `grund check` draws this line already: it takes the workspace-aggregate path
/// only when the scope *is* the config root, and otherwise runs one narrowed scan
/// (`scope_is_config_root`, §FS-check.1.3). A scope narrower than the root has to
/// stay narrow — an explicit path bypasses `[scan] include` (§AR-scanner.1), so
/// widening it back to every project would both answer a question the caller did
/// not ask and lose the files the narrowing was for.
///
/// `list`, `refs`, `show`, completions, and `fmt` keep [`load_workspace_context`]:
/// their `<path>` selects a project, it does not bound a walk (§FS-workspace.8).
fn load_narrowable_workspace_context(path: &Path, path_provided: bool) -> Result<WorkspaceContext> {
    let mut config = resolve_workspace_config(path)?;
    if !config.workspace_declared || scope_is_config_root(&config, path, path_provided) {
        return load_workspace_context(path, path_provided);
    }
    // §AR-scanner.2.4: the caller is a read-only query — skip the classification
    // post-pass, exactly as `load_workspace_context` does (§AR-benchmarks).
    config.classify_citation_sources = false;
    single_project_context(config, path, path_provided, &TextOverlays::new())
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
    // Stage 1: build the (alias, config) list, recursing into any member that
    // is itself a workspace root. Failing fast on alias errors, empty
    // workspaces, duplicates, member cycles, and missing members before any
    // scan keeps misconfiguration cheap to diagnose.
    let mut entries = expand_workspace_tree(root_config)?;

    // §AR-scanner.2.4: members inherit the root's classification intent, so a
    // read-only command (root off) skips the post-pass for every member and
    // `grund check` (root on) classifies the whole workspace.
    // §FS-check.1.3: `grund check --full` is a property of the run, not of one
    // project's config, so every member walks past its own `[scan] include` too.
    for entry in &mut entries {
        entry.config.classify_citation_sources = root_config.classify_citation_sources;
        entry.config.scan_full = root_config.scan_full;
    }

    // Stage 2: build the target list up-front so each project's scan can
    // parse `§<alias>/<ID>` citations with the target's grammar inline —
    // no second disk pass (§FS-workspace.1, §AR-workspace.2).
    let targets = entries
        .iter()
        .map(|entry| WorkspaceCitationTarget {
            alias: entry.alias.clone(),
            config: entry.config.clone(),
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
            .map(|(index, entry)| {
                (index, load_workspace_project(entry.alias, entry.config, &targets, overlays))
            })
            .collect::<Vec<_>>()
    } else {
        entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                (index, load_workspace_project(entry.alias, entry.config, &targets, overlays))
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
/// (§FS-workspace.1). An ID never contains `/`, so the **last** separator is
/// the boundary — that is what lets a nested project be addressed by its whole
/// alias path (§FS-workspace.6.1). Every segment is validated against the slug
/// grammar here, before resolution; the ID tail is deliberately left raw so the
/// caller can parse it with the target project's grammar.
fn split_qualified_id_arg(raw: &str) -> Result<(Option<String>, &str)> {
    if let Some((alias, rest)) = raw.rsplit_once('/') {
        if let Some(message) = invalid_alias_path_message(alias) {
            return Err(anyhow!("{message}"));
        }
        return Ok((Some(alias.to_string()), rest));
    }
    Ok((None, raw))
}

/// §FS-workspace.8: the diagnostic for an alias path that is not one slug per
/// level, naming the **segment** that failed. Naming the whole path against a
/// pattern that forbids `/` would read as "a namespace may not contain `/`",
/// which is the opposite of the rule (§FS-workspace.1) — and in a nested tree the
/// path is usually mostly right. A single-segment path is its own segment, so it
/// is named plainly; an empty segment has nothing to quote and says so.
fn invalid_alias_path_message(alias: &str) -> Option<String> {
    const EXPECTED: &str = "expected [a-z][a-z0-9-]*, one segment per workspace level";
    let segments: Vec<&str> = alias.split('/').collect();
    let bad = segments
        .iter()
        .find(|segment| !is_valid_project_alias(segment))?;
    Some(if alias.is_empty() {
        format!("invalid project alias: the path before the ID is empty ({EXPECTED})")
    } else if bad.is_empty() {
        format!("invalid project alias `{alias}`: a segment is empty ({EXPECTED})")
    } else if segments.len() == 1 {
        format!("invalid project alias `{alias}` ({EXPECTED})")
    } else {
        format!("invalid project alias segment `{bad}` in `{alias}` ({EXPECTED})")
    })
}
