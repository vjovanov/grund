/// Whether the requested scope *is* the config root — the scope `[scan] include`
/// governs, and therefore the only one `grund check --full` can widen
/// (§FS-check.1.3). It is also what decides a workspace-wide run
/// (§FS-workspace.5): both questions are "did the caller ask for the whole
/// project, however they spelled it?".
///
/// This file answers which project a path belongs to, and how that project is named
/// in a message. The file-level prose rides on this first item rather than a `//!`
/// module doc because the crate is assembled by `include!` (§AR-core-module-layout.2).
///
/// Split out of `checker_cmd.rs` the way `workspace_members.rs` was: that file is
/// the `check` command's argument adapter, and resolving a scope to a config —
/// upward discovery, the member-scope rewrite, the boundary-root population, and
/// the alias a diagnostic spells the result with — is config work every walking
/// command shares, not something `check` owns (§AR-workspace.5.1,
/// §AR-workspace.6, §AR-workspace.8).
fn scope_is_config_root(config: &Config, path: &Path, path_provided: bool) -> bool {
    !path_provided
        || fs::canonicalize(path)
            .map(|path| path == config.root)
            .unwrap_or(false)
}

/// §AR-workspace.5.1, §AR-workspace.6, §AR-workspace.8: every CLI entry point
/// that walks the tree funnels through this helper so workspace handling is
/// identical across `check`, `fmt`, `refs`, `list`, `cover`, `show`, `id`, and
/// completions. The three steps are upward discovery, the member-scope
/// rewrite, and boundary-root population — the last is what stops a root-scope
/// scan from absorbing member declarations into the parent namespace.
fn resolve_workspace_config(path: &Path) -> Result<Config> {
    let mut config = load_config(path)?;
    config = config_for_member_scope(config, path)?;
    apply_workspace_boundary(&mut config)?;
    Ok(config)
}

/// §AR-workspace.6: a workspace-declared scan must never descend into member
/// roots. The boundary is the same list that `run_workspace_check`
/// computes; setting it on the Config makes the scanner skip those subtrees.
///
/// §FS-check.4.8: it is also where the block a run is rooted at is asked whether
/// that boundary leaves it anything to read. Every command that walks resolves
/// its config through here, so asking at this one point is what puts the warning
/// on `list`, `refs`, `cover` and `fmt` rather than on `check` alone — and asking
/// it *here* rather than on the expansion below is what keeps it to once per
/// block per run, since a workspace-wide run expands the same block a second
/// time. A run narrowed inside a member never populates the parent block's
/// boundary (`config_for_member_scope` rewrites first), so it stays silent about
/// a block it is not reading through.
fn apply_workspace_boundary(config: &mut Config) -> Result<()> {
    if !config.workspace_declared {
        return Ok(());
    }
    let members = expand_workspace_member_list(config)?;
    warn_if_members_absorb_scan(config, &members);
    config.workspace_boundary_roots = members.into_iter().map(|member| member.root).collect();
    Ok(())
}

/// §FS-workspace.2 / §FS-workspace.5: when the requested scope lies inside a
/// configured workspace member, rewrite the resolved config so the run is
/// rooted at the member rather than the workspace root. This applies whether
/// the member has its own `grund.toml` or not — either way a
/// member-scoped command runs as an independent project, with member defaults
/// when no member config exists.
fn config_for_member_scope(mut config: Config, path: &Path) -> Result<Config> {
    if !config.workspace_declared {
        return Ok(config);
    }
    let scope = config_scope_start(path);
    if scope == config.root {
        return Ok(config);
    }
    if let Some(member_root) = configured_member_root_for_scope(&config, &scope) {
        config = load_config_at(&member_root, &config.cli_base)?;
    }
    Ok(config)
}

fn config_scope_start(path: &Path) -> PathBuf {
    let start = if path.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };
    fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf())
}

fn configured_member_root_for_scope(config: &Config, scope: &Path) -> Option<PathBuf> {
    config
        .workspace_members
        .iter()
        .filter_map(|member| configured_member_root_candidate(config, member, scope))
        .max_by_key(|root| root.components().count())
}

fn configured_member_root_candidate(config: &Config, member: &str, scope: &Path) -> Option<PathBuf> {
    if let Some(parent) = member.strip_suffix("/*") {
        let parent = canonical_workspace_path(&config.root.join(parent));
        let relative = scope.strip_prefix(&parent).ok()?;
        let Component::Normal(child) = relative.components().next()? else {
            return None;
        };
        let root = parent.join(child);
        return Some(canonical_workspace_path(&root));
    }
    let root = canonical_workspace_path(&config.root.join(member));
    if scope == root || scope.starts_with(&root) {
        Some(root)
    } else {
        None
    }
}

fn workspace_members_error(config: &Config, message: String) -> anyhow::Error {
    config_location_error(config.workspace_members_source.as_ref(), message)
}

fn project_name_error(config: &Config, message: String) -> anyhow::Error {
    config_location_error(config.project_name_source.as_ref(), message)
}

fn config_location_error(source: Option<&ConfigLocation>, message: String) -> anyhow::Error {
    anyhow!("{}", config_location_message(source, message))
}

/// The breadcrumb every diagnostic about a config key wears — `<config>:<line>:`
/// ahead of the sentence (§FS-config.4.3) — built apart from the error above
/// because a *warning* about such a key needs the same one and is not an error
/// (§FS-check.4.8).
fn config_location_message(source: Option<&ConfigLocation>, message: String) -> String {
    match source {
        Some(source) => format!("{}:{}: {message}", format_path(&source.path), source.line),
        None => message,
    }
}

enum RootMode {
    Root,
    Member,
}

/// Phrase a workspace project's identity for a launch-time error message: the
/// root collapses to `workspace root`, members render as `workspace member
/// `<rel-path>``. §GOAL-friendliness-first.1 — duplicate-alias and
/// invalid-alias errors are CLI-level (no `path:line:`), so they have to name
/// the source project in the message itself.
fn project_label(root_config: &Config, project_root: &Path) -> String {
    if project_root == root_config.root {
        "workspace root".to_string()
    } else {
        format!(
            "workspace member `{}`",
            display_path(root_config, project_root)
        )
    }
}

/// Render the two colliding project roots for `duplicate workspace project
/// alias`. When both are members we fold the shared `workspace member` prefix
/// (`workspace members `a` and `b``); when one side is the root we keep the
/// asymmetric pairing (`workspace root and workspace member `b``).
fn duplicate_alias_sites(root_config: &Config, first: &Path, second: &Path) -> String {
    let first_is_root = first == root_config.root;
    let second_is_root = second == root_config.root;
    if !first_is_root && !second_is_root {
        return format!(
            "workspace members `{}` and `{}`",
            display_path(root_config, first),
            display_path(root_config, second)
        );
    }
    format!(
        "{} and {}",
        project_label(root_config, first),
        project_label(root_config, second)
    )
}

/// §AR-workspace.5.3: the single canonical place that derives a project's
/// workspace alias. `project_name` wins; otherwise the member directory's
/// basename, or the literal `root` for an unnamed workspace root. Whichever
/// source fires, the result is validated against the alias slug grammar so a
/// bad name fails fast at workspace expansion, not later inside a citation.
fn derive_alias(
    config: &Config,
    member_root: Option<&Path>,
    mode: RootMode,
) -> std::result::Result<String, String> {
    let alias = match (&config.project_name, &mode) {
        (Some(name), _) => name.clone(),
        (None, RootMode::Root) => "root".to_string(),
        (None, RootMode::Member) => {
            // Members always have a canonical absolute path with a final
            // component; the basename fallback is the alias source defined in
            // §AR-workspace.5.3.
            let path = member_root.expect("member alias derivation needs a member root");
            path.file_name()
                .and_then(|name| name.to_str())
                .expect("workspace member root has a final UTF-8 path component")
                .to_string()
        }
    };
    if is_valid_project_alias(&alias) {
        Ok(alias)
    } else {
        Err(format!(
            "invalid workspace project alias `{alias}` (expected [a-z][a-z0-9-]*)"
        ))
    }
}
