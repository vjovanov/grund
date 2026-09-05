/// One resolved workspace project — the alias, canonical root, and optional
/// one-line description — collected by [`find_init_workspace_context`] so the
/// workspace-members renderer never has to talk to the config layer directly
/// (§FS-init.2.3.4.15, §DF-workspace-member-descriptions).
///
/// What this file holds: the workspace half of the `init` renderer
/// (§FS-init.2.3.4.15), in a file of its own beside `init_entrypoints.rs`
/// (§AR-core-module-layout.1) — finding the workspace a target sits in, and
/// rendering the member list the managed block carries. `init_templates.rs`
/// keeps the block itself and the generated config.
struct InitWorkspaceProject {
    alias: String,
    project_root: PathBuf,
    description: Option<String>,
}

/// Walk up from `target` to the outermost ancestor whose `grund.toml` — either
/// discovery form (§FS-config.1) — declares `[workspace]`, then expand the
/// whole tree and derive each alias the same way `grund check` does
/// (§FS-workspace.2 / §FS-workspace.3 / §FS-workspace.6.1). Returns the
/// alias-sorted project list (every block's root, subject to its own
/// `include_root`, plus every member at every depth) when `target` sits inside
/// a workspace; `None` otherwise. Returns `None` rather than an error on any
/// workspace configuration problem (missing member, duplicate alias, member
/// cycle, …) — init must not fail because a sibling member is misconfigured;
/// the next `grund check` will surface the issue (§FS-init.2.3.4.15).
///
/// Why a `target` that will not canonicalize suppresses the section: canonical
/// identity is what lets the renderer omit the local project instead of
/// presenting it as a foreign namespace.
///
/// Pending metadata is applied while collecting the effective project tree.
/// The renderer later omits that canonical self project; only its pending name
/// can still affect the section, by colliding with a foreign alias during the
/// ambiguity check below.
fn find_init_workspace_context(
    target: &Path,
    pending_project_name: Option<&str>,
    pending_project_description: Option<&str>,
) -> Option<Vec<InitWorkspaceProject>> {
    let (mut root_config, run_root) = find_init_workspace_root(target)?;
    // `expand_workspace_tree` returns canonical project roots, so canonicalize
    // `target` before §FS-init.2.3.4.15's identity-based self omission.
    let target_canonical = fs::canonicalize(target).ok()?;
    let mut projects = Vec::new();
    // §FS-check.4.8: the expansion below is the only route that walks *down* from
    // a root above the run, so it is the only one that has to be told where the
    // run is — every other command re-roots onto it first (§AR-workspace.5.1).
    for entry in expand_workspace_tree_with_report_base(&mut root_config, &run_root).ok()? {
        let mut alias = entry.alias;
        let mut description = entry.config.project_description.clone();
        if entry.config.root == target_canonical && config_file_in(&entry.config.root).is_none() {
            // Apply pending config before validating the tree. The renderer
            // omits this project; its pending name matters only if it makes
            // the foreign aliases ambiguous (§FS-init.2.3.4.15).
            if let Some(name) = pending_project_name {
                if !is_valid_project_alias(name) {
                    return None;
                }
                // `--name` renames the project, not the workspace levels above
                // it: only the last segment of the alias path changes
                // (§FS-workspace.6.1).
                alias = match alias.rsplit_once('/') {
                    Some((prefix, _)) => format!("{prefix}/{name}"),
                    None => name.to_string(),
                };
            }
            if let Some(pending) = pending_project_description {
                description = Some(pending.to_string());
            }
        }
        projects.push(InitWorkspaceProject {
            alias,
            project_root: entry.config.root,
            description,
        });
    }
    // The pending `--name` above is applied after expansion, so it can collide
    // with a project `expand_workspace_tree` already accepted; the check below
    // is what catches that case, not a second opinion on the tree itself.
    let mut seen = BTreeMap::new();
    for project in &projects {
        if seen
            .insert(project.alias.clone(), project.project_root.clone())
            .is_some()
        {
            // §FS-init.2.3.4.15: duplicate aliases make the guidance
            // ambiguous, so suppress the section and leave the diagnostic to
            // `grund check`, just as other workspace config errors do.
            return None;
        }
    }
    projects.sort_by(|a, b| a.alias.cmp(&b.alias));
    Some(projects)
}

/// The outermost workspace whose tree actually contains `target`: start at the
/// config that governs it (§FS-config.1) and climb the *claimed chain* — the
/// same walk `enclosing_alias_prefix` uses, so `init` teaches exactly the alias
/// set a command run here resolves (§FS-init.2.3.4.15, §FS-workspace.6.1).
///
/// Unlike [`load_config`] the walk does not stop at the first config it finds — a
/// member with its own config must still see the workspace root above it. It does
/// stop where the claims stop: an ancestor `[workspace]` that does not list the
/// directory below it describes a different workspace, whose aliases resolve
/// nowhere here and whose members lie outside this repository.
///
/// Returns the climbed config **and the root this run was launched at** — the
/// block governing `target`, before the climb moved off it. That is the base every
/// diagnostic of this run is rendered against (§FS-errors.4), and the climb has
/// already used it for the blocks above; the expansion downward needs the same one
/// (§FS-check.4.8).
fn find_init_workspace_root(target: &Path) -> Option<(Config, PathBuf)> {
    // Without a canonical anchor we cannot reliably compare against the
    // canonicalized project roots `expand_workspace_tree` returns; bail
    // out so the section is suppressed (§FS-init.2.3.4.15).
    let canonical_target = fs::canonicalize(target).ok()?;
    let mut cursor: Option<&Path> = Some(&canonical_target);
    let mut config = loop {
        let dir = cursor?;
        if config_file_in(dir).is_some() {
            break load_config_at(dir, &canonical_target).ok()?;
        }
        cursor = dir.parent();
    };
    let run_root = config.root.clone();
    let mut ancestors = AncestorWorkspaces::for_run_at(&config.root);
    loop {
        match enclosing_workspace_of(&config.root, &canonical_target, &mut ancestors) {
            Ok(Some(parent)) => config = parent,
            Ok(None) => break,
            // A broken block above us is `grund check`'s to report; `init` must
            // not describe a tree it cannot see whole (§FS-init.2.3.4.15).
            Err(_) => return None,
        }
    }
    if !config.workspace_declared {
        return None;
    }
    // §FS-check.4.8: the one route to `expand_workspace_tree` that does not come
    // through `resolve_workspace_config`, and the expansion asks every block but
    // this one — so without this, `init` says nothing about the block it is on.
    apply_workspace_boundary(&mut config).ok()?;
    Some((config, run_root))
}

/// Render the §FS-init.2.3.4.15 Workspace Members section, or the empty string
/// when `target` is not inside a workspace. The leading `\n\n` is the
/// separator from the preceding namespace guidance block, so an empty value
/// leaves the surrounding spacing unchanged.
fn render_workspace_members_section(
    target: &Path,
    pending_project_name: Option<&str>,
    pending_project_description: Option<&str>,
    citation_marker: &str,
    _canonical_agent_entrypoint_selected: bool,
) -> String {
    let Some(projects) = find_init_workspace_context(
        target,
        pending_project_name,
        pending_project_description,
    ) else {
        return String::new();
    };
    // `find_init_workspace_context` already required `target` to canonicalize
    // before it returned `Some`, so this call cannot fail in practice; bail
    // out instead of falling back to a non-canonical path that would break
    // the `is_self` comparison below.
    let Ok(target_canonical) = fs::canonicalize(target) else {
        return String::new();
    };
    let mut bullets = Vec::with_capacity(projects.len());
    for project in &projects {
        // §FS-init.2.3.4.15: the canonical init target is the local namespace,
        // so this cross-project list contains only foreign projects.
        if project.project_root == target_canonical {
            continue;
        }
        let agents_md_path = project.project_root.join("AGENTS.md");
        let initialized = agents_md_path.exists();
        let link = if initialized {
            relative_link_path(&target_canonical, &agents_md_path)
        } else {
            let dir_rel = relative_link_path(&target_canonical, &project.project_root);
            if dir_rel == "." {
                "./".to_string()
            } else {
                format!("{dir_rel}/")
            }
        };
        let suffix = if initialized { "" } else { " *(not yet initialized)*" };
        // §FS-init.2.3.4.15: the alias is the link label so the path appears
        // once, mirroring the Project Map's `- [x](y): …` shape; the one-line
        // description follows `: `, before the trailing marker.
        let description = project
            .description
            .as_deref()
            .map(|description| format!(": {description}"))
            .unwrap_or_default();
        bullets.push(format!(
            "- [`{alias}`]({dest}){description}{suffix}",
            alias = project.alias,
            dest = markdown_link_destination(&link),
        ));
    }
    let mut out = format!(
        "\n\n### Workspace members\n\nCross-project citations use {citation_marker}alias/<ID>.\n\n",
    );
    out.push_str(&bullets.join("\n"));
    out
}

/// Compute a relative POSIX-style path from `from_dir` to `to`. Both inputs
/// must be absolute (canonicalized) paths. Used to render workspace member
/// links from inside the AGENTS.md being written (§FS-init.2.3.4.15); Markdown
/// links are always forward-slash regardless of platform.
fn relative_link_path(from_dir: &Path, to: &Path) -> String {
    let from = normalize_path_lexically(from_dir);
    let to = normalize_path_lexically(to);
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<String> = Vec::new();
    for _ in &from_components[common..] {
        parts.push("..".to_string());
    }
    for component in &to_components[common..] {
        parts.push(component.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
