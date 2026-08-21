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

/// One project the workspace walk reached: the alias path qualified citations
/// name it by — one segment per workspace level, so a nested project carries
/// its whole chain (§FS-workspace.3, §FS-workspace.6.1) — and its own loaded
/// config, whose `root` is the canonical project root.
struct WorkspaceProjectEntry {
    alias: String,
    config: Config,
}

/// §FS-workspace.6.1: the alias path of `config`'s own root, read from the
/// outermost workspace above it — empty when this *is* the outermost workspace.
///
/// This is what makes an alias path mean one thing at every scope: narrowing a
/// run to a subtree drops the projects outside it but never re-spells the ones
/// inside, so a citation that passes inside `hardware/` passes in CI at the
/// repository root and vice versa (§FS-workspace.5).
///
/// The climb takes the outermost ancestor that both declares `[workspace]` *and*
/// actually lists the directory below it — a workspace config that does not
/// claim this tree says nothing about how it is named.
///
/// Fallible on purpose (§FS-workspace.6.1): every block in the claimed chain has
/// to answer. One that claims this directory but cannot expand its members, or
/// cannot name the project below it, fails the run with its own error instead of
/// dropping out of the path — a dropped segment would leave the subtree with a
/// namespace no other scope agrees with, and §FS-check.3.8 would then tell the
/// author to write the one spelling that fails in CI.
fn enclosing_alias_prefix(config: &Config) -> Result<String> {
    let mut segments: Vec<String> = Vec::new();
    // The first child is the caller's own config, already loaded; each later one
    // is the claiming ancestor the previous step returned. Either way the config
    // in hand is the one whose alias segment is being read, so the climb never
    // reloads a level (§AR-workspace.5.1).
    let mut climbed: Option<Config> = None;
    // One cache for the whole climb: every level walks the same ancestors, so
    // each ancestor's config is read once per run rather than once per level
    // (§AR-workspace.6.1).
    let mut ancestors = AncestorWorkspaces::for_run_at(&config.root);
    loop {
        let child_config = climbed.as_ref().unwrap_or(config);
        let Some(parent) =
            enclosing_workspace_of(&child_config.root, &config.cli_base, &mut ancestors)?
        else {
            break;
        };
        // A claimed directory is a member of `parent`, so its alias is derived
        // and located exactly as the outer run derives and locates it — the same
        // error text, from the same line (§AR-workspace.5.3).
        let alias = member_alias(child_config, &child_config.root, &parent, &parent)?;
        segments.push(alias);
        climbed = Some(parent);
    }
    segments.reverse();
    Ok(segments.join("/"))
}

/// The **outermost** ancestor workspace that declares `child` a member, or
/// `None`. Each step moves strictly upward, so the caller's climb terminates.
///
/// Outermost rather than nearest (§FS-workspace.6.1): a multi-segment `members`
/// entry (`grp/inner`) hops the directories between, and a hopped directory may
/// declare `[workspace]` and list the same child. Stopping at the nearer of the
/// two claims would name `inner` from a directory nothing lists and lose every
/// segment above it, while the top-down walk from the outermost root composes the
/// path through the outer claim — so the climb has to follow the same one. Where
/// exactly one block claims a directory, which is ordinary nesting, the two are
/// the same block.
///
/// A block that **claims** this directory and cannot expand its member list is
/// that block's own config error, raised here rather than read as "does not claim
/// this directory": the key that failed is the very one that would have answered
/// the question (§FS-workspace.6.1). The claim is read off the entry text first,
/// so a block that names nothing here is never expanded and its errors are not
/// this run's business. The entry text is read from the file on its own
/// (`ancestor_member_entries`) precisely so that a config which does not *load*
/// can still be asked the question: a claiming block that will not parse fails the
/// run like any other claim it cannot answer, while one that names nothing here is
/// climbed past however broken it is — this walk reaches the filesystem root, and a
/// stray `grund.toml` above the repository must not break every run beneath it.
fn enclosing_workspace_of(
    child: &Path,
    cli_base: &Path,
    ancestors: &mut AncestorWorkspaces,
) -> Result<Option<Config>> {
    // The claim is compared canonically: a `members` entry may reach this
    // directory through a symlink, and then only the resolved paths agree
    // (§FS-workspace.6.1).
    let canonical_child = canonical_workspace_path(child);
    let mut claiming = None;
    let mut cursor = child.parent();
    while let Some(dir) = cursor {
        if let Some(parent) = ancestors.claiming_block(dir, child, &canonical_child, cli_base)? {
            claiming = Some(parent.clone());
        }
        cursor = dir.parent();
    }
    Ok(claiming)
}

/// Append one alias segment to the path of the workspace that contains it.
/// The outermost root contributes no segment: its members are named from the
/// top of the workspace, which is what keeps a single-level workspace's
/// citations byte-identical to what they were (§FS-workspace.6.1).
fn qualify_alias(prefix: &str, alias: &str) -> String {
    if prefix.is_empty() {
        alias.to_string()
    } else {
        format!("{prefix}/{alias}")
    }
}

/// §FS-workspace.6.1 / §AR-workspace.6.1: expand a workspace config into every
/// project in the tree — this block's own root when `include_root = true`, then
/// each member, recursing into any member that declares its own `[workspace]`.
/// The result is one list whatever the depth, because depth lives in the *key*:
/// each entry's alias is its whole path, which the walk composes one segment
/// per level.
///
/// Also sets `workspace_boundary_roots` on `root_config` and on every nested
/// workspace config, so each block's scan stops at its own members
/// (§AR-workspace.6) rather than absorbing them into its namespace, and
/// `workspace_project_roots` on every project the walk reached, so each of them
/// stops at the others in the directions the downward list cannot see
/// (§FS-workspace.6).
///
/// Every launch-time invariant fires here, before any scan: the alias grammar,
/// the per-block "something in scope" rule, alias uniqueness among siblings,
/// and the canonical-root check that both rejects a member cycle and bounds the
/// walk.
fn expand_workspace_tree(root_config: &mut Config) -> Result<Vec<WorkspaceProjectEntry>> {
    let members = expand_workspace_member_list(root_config)?;
    root_config.workspace_boundary_roots = members.iter().map(|m| m.root.clone()).collect();

    let mut entries: Vec<WorkspaceProjectEntry> = Vec::new();
    let mut visited: Vec<PathBuf> = vec![root_config.root.clone()];
    // §FS-workspace.3: the root project and the top-level members share one
    // level of the namespace, so they are checked for collisions together.
    let mut siblings: BTreeMap<String, PathBuf> = BTreeMap::new();
    // §FS-workspace.6.1: the alias path of *this* workspace's own root, read
    // from the outermost workspace. Empty unless the run was narrowed to a
    // subtree, and that is exactly what keeps a narrowed run resolving a subset
    // of the same paths instead of a differently-spelled set of its own.
    let self_path = enclosing_alias_prefix(root_config)?;
    if root_config.workspace_include_root {
        let alias = if self_path.is_empty() {
            derive_alias(root_config, None, RootMode::Root).map_err(|err| {
                let message =
                    format!("{err} for {}", project_label(root_config, &root_config.root));
                project_name_error(root_config, message)
            })?
        } else {
            self_path.clone()
        };
        // Only at the outermost root do the root project and the members share
        // a level. In a narrowed run this project sits one level *up* from the
        // members below it, exactly where the outer run puts it.
        if self_path.is_empty() {
            siblings.insert(alias.clone(), root_config.root.clone());
        }
        entries.push(WorkspaceProjectEntry {
            alias,
            config: root_config.clone(),
        });
    }
    collect_workspace_members(
        &members,
        root_config,
        root_config,
        &self_path,
        &mut siblings,
        &mut visited,
        &mut entries,
    )?;
    if entries.is_empty() {
        return Err(empty_workspace_error(root_config));
    }
    // §AR-workspace.6: every project the run loaded learns where the others are.
    // `workspace_boundary_roots` above points only downward, so a leaf member has
    // no boundary at all and a symlink inside it carries its walk into a sibling's
    // namespace — or back up into the root project's — which §FS-workspace.6
    // forbids in every direction, not just from the root down.
    let project_roots: Vec<PathBuf> = entries
        .iter()
        .map(|entry| entry.config.root.clone())
        .collect();
    // §FS-check.3.8: every project the run loaded carries the run's own scope, so
    // the diagnostic that would re-spell a citation knows whether it is looking at
    // the whole tree or a slice of it.
    for entry in &mut entries {
        entry.config.workspace_scope_path = self_path.clone();
        entry.config.workspace_project_roots = project_roots.clone();
    }
    root_config.workspace_project_roots = project_roots;
    Ok(entries)
}

/// One level of member expansion (§FS-workspace.6.1). `parent_config` is the
/// block that listed `member_roots` — it owns the `members` line a member error
/// points at; `top_config` is the outermost workspace root — it owns path
/// rendering, so every diagnostic names a project against the same base;
/// `prefix` is the alias path of the enclosing workspace, empty at the top.
fn collect_workspace_members(
    members: &[WorkspaceMember],
    parent_config: &Config,
    top_config: &Config,
    prefix: &str,
    siblings: &mut BTreeMap<String, PathBuf>,
    visited: &mut Vec<PathBuf>,
    entries: &mut Vec<WorkspaceProjectEntry>,
) -> Result<()> {
    for member in members {
        let member_root = &member.root;
        // §FS-workspace.6.1: an **unreachable backstop**, kept because it is the
        // one that would name the line if the rule above it ever stopped holding.
        // What bounds this recursion is containment: every member root resolves
        // strictly inside the block that listed it (`workspace_member_root`) and no
        // member of one block contains another (`reject_overlapping_workspace_members`),
        // so the blocks form a strict containment tree — every step consumes a
        // canonical root strictly deeper than the block that named it, and no two
        // roots in the tree can be equal. Should a duplicate reach here anyway it is
        // a config error at the line that introduced it, named as the entry was
        // written and beside the root it lands on, since those are two different
        // strings and the author wrote only one of them.
        if visited.iter().any(|seen| seen == member_root) {
            return Err(workspace_members_error(
                parent_config,
                format!(
                    "workspace member `{}` resolves to `{}`, already a project in this workspace",
                    member.written,
                    display_path(top_config, member_root)
                ),
            ));
        }
        visited.push(member_root.clone());
        let mut member_config = load_config_at_with_report_base(
            member_root,
            &top_config.cli_base,
            Some(&top_config.root),
        )?;
        // The alias is derived whether or not the member turns out to be a
        // project: a member that is itself a workspace still contributes its
        // segment to every alias path below it (§FS-workspace.6.1).
        let alias = member_alias(&member_config, member_root, parent_config, top_config)?;
        // §FS-workspace.3: uniqueness is a property of one level. Two projects
        // under different parents may share a segment — their alias paths still
        // differ — so the check is per sibling set, not per tree.
        if let Some(first) = siblings.get(&alias) {
            return Err(workspace_members_error(
                parent_config,
                format!(
                    "duplicate workspace project alias `{}` ({})",
                    qualify_alias(prefix, &alias),
                    duplicate_alias_sites(top_config, first, member_root)
                ),
            ));
        }
        siblings.insert(alias.clone(), member_root.clone());
        let qualified = qualify_alias(prefix, &alias);
        if !member_config.workspace_declared {
            entries.push(WorkspaceProjectEntry {
                alias: qualified,
                config: member_config,
            });
            continue;
        }
        // §FS-workspace.6.1: a member that is itself a workspace root
        // contributes its whole subtree, and `include_root` on *its* block
        // decides whether the grouping directory is one of the projects.
        let nested = expand_workspace_member_list(&member_config)?;
        member_config.workspace_boundary_roots = nested.iter().map(|m| m.root.clone()).collect();
        let before = entries.len();
        if member_config.workspace_include_root {
            entries.push(WorkspaceProjectEntry {
                alias: qualified.clone(),
                config: member_config.clone(),
            });
        }
        collect_workspace_members(
            &nested,
            &member_config,
            top_config,
            &qualified,
            &mut BTreeMap::new(),
            visited,
            entries,
        )?;
        if entries.len() == before {
            return Err(empty_workspace_error(&member_config));
        }
    }
    Ok(())
}

/// §AR-workspace.5.3: a member's alias, with the error anchored where the bad
/// name was written — the member's own `project_name` line when it set one,
/// otherwise the `members` line of the block that named the directory.
fn member_alias(
    member_config: &Config,
    member_root: &Path,
    parent_config: &Config,
    top_config: &Config,
) -> Result<String> {
    derive_alias(member_config, Some(member_root), RootMode::Member).map_err(|err| {
        let message = format!("{err} for {}", project_label(top_config, member_root));
        if member_config.project_name_source.is_some() {
            project_name_error(member_config, message)
        } else {
            workspace_members_error(parent_config, message)
        }
    })
}

/// §FS-workspace.6.1: every `[workspace]` block must put at least one project
/// in scope — a block that contributes nothing would silently drop its whole
/// subtree from the check.
///
/// The `members` line is the anchor when there is one; the block that reaches this
/// error with `include_root = false` and *no* `members` key has none, so it falls
/// back to its own `[workspace]` line. Without the fallback the message carried no
/// location at all, in a tree that may hold many blocks (§FS-errors.4).
fn empty_workspace_error(config: &Config) -> anyhow::Error {
    let source = config
        .workspace_members_source
        .as_ref()
        .or(config.workspace_section_source.as_ref());
    config_location_error(
        source,
        "workspace has no projects in scope (include_root = false and no members)".to_string(),
    )
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
