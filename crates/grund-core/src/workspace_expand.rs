/// The `[workspace]` expansion walk: turning one block into the list of
/// projects a run operates on, and naming each of them. Split out of
/// `workspace_context.rs` along the seam that was already there — that file
/// answers "what does a query command hold?", this one answers "which projects
/// are there, and what is each one called?" (§AR-workspace.6.1,
/// §AR-core-module-layout.1). Included into the same module, so the two halves
/// still share `WorkspaceProject` and the config helpers.

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
    // The first child is the caller's own config, already loaded; each later one is
    // the claiming ancestor the previous step returned — either way the config in
    // hand is the one being read, so the climb never reloads a level (§AR-workspace.5.1).
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
    // §FS-check.4.8: no warning here. `apply_workspace_boundary` already asked
    // this block, on every route a walk takes (§AR-workspace.5.1), and this line
    // only repopulates what it set — asking again would say it twice.
    root_config.workspace_boundary_roots = members.iter().map(|m| m.root.clone()).collect();

    let mut entries: Vec<WorkspaceProjectEntry> = Vec::new();
    let mut visited: Vec<PathBuf> = vec![root_config.root.clone()];
    // §FS-workspace.3: the root project and the top-level members share one
    // level of the namespace, so they are checked for collisions together.
    let mut siblings: BTreeMap<String, PathBuf> = BTreeMap::new();
    // §FS-workspace.6.1: the alias path of *this* workspace's own root, read from
    // the outermost workspace. Empty unless the run was narrowed to a subtree — and
    // that is what keeps a narrowed run resolving a subset of the same paths.
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
    // `workspace_boundary_roots` above points only downward, so a leaf member has none
    // and a symlink could cross out of it; §FS-workspace.6 forbids that in every direction.
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
///
/// What bounds the recursion is containment: every member root resolves strictly
/// inside the block that listed it (`workspace_member_root`) and no member of one
/// block contains another (`reject_overlapping_workspace_members`), so the blocks form
/// a strict containment tree — every step consumes a canonical root strictly deeper
/// than the block that named it, and no two roots in the tree can be equal. Should a
/// duplicate reach the visited check anyway it is a config error at the line that
/// introduced it, named as the entry was written and beside the root it lands on,
/// since those are two different strings and the author wrote only one of them.
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
        // §FS-workspace.6.1: an **unreachable backstop**, kept because it is the one
        // that would name the line if the containment rule that bounds this recursion
        // (see the fn docs) ever stopped holding.
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
        // §FS-check.4.8: a block below the run's root is populated here and
        // nowhere else, so this is where it is asked — once, at its own
        // `members` line (§FS-errors.4).
        warn_if_members_absorb_scan(&member_config, &nested);
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
/// location at all, in a tree that may hold many blocks (§FS-errors.4). A
/// non-empty list can reach here only when every entry is a glob that matched no
/// directories; §FS-workspace.6.1 names the first in config order instead of
/// falsely claiming there were no members.
fn empty_workspace_error(config: &Config) -> anyhow::Error {
    let source = config
        .workspace_members_source
        .as_ref()
        .or(config.workspace_section_source.as_ref());
    let message = config.workspace_members.first().map_or_else(
        || "workspace has no projects in scope (include_root = false and no members)".to_string(),
        |glob| format!("the glob `{glob}` matched no directories"),
    );
    config_location_error(source, message)
}
