// Member-list expansion: turning one `[workspace] members` list into the
// canonical project roots it names, and enforcing the invariants that list has
// to satisfy (§FS-workspace.2, §FS-workspace.6.1).
//
// Split out of `checker_cmd.rs`, which is the `check` command's argument
// adapter: carrying each entry's *written* spelling so a diagnostic can name it
// (§FS-errors.4) turned expansion into a small rule set of its own, and rules
// are not what that file is for. Every `[workspace]` block — outermost root or
// nested member — expands through here, so the invariants hold at every depth
// (§AR-workspace.5.1, §AR-workspace.6.1).

fn canonical_workspace_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// One expanded `members` entry: the canonical project root, and the entry **as
/// the config wrote it**. Diagnostics name the written form — a canonical root
/// renders as the empty string when it equals the render base and as an absolute
/// path when it lies outside it, and neither is something an author can act on
/// (§FS-errors.4, §FS-workspace.6.1).
struct WorkspaceMember {
    written: String,
    root: PathBuf,
}

/// The canonical roots of an expanded member list — the boundary-root form, for
/// the callers that only need to know where a scan stops (§AR-workspace.6).
fn expand_workspace_members(config: &Config) -> Result<Vec<PathBuf>> {
    Ok(expand_workspace_member_list(config)?
        .into_iter()
        .map(|member| member.root)
        .collect())
}

fn expand_workspace_member_list(config: &Config) -> Result<Vec<WorkspaceMember>> {
    let mut roots = Vec::new();
    for member in &config.workspace_members {
        if let Some(glob_parent) = member.strip_suffix("/*") {
            let parent = config.root.join(glob_parent);
            if !parent.is_dir() {
                return Err(workspace_members_error(
                    config,
                    format!(
                        "workspace member glob parent does not exist: {}",
                        display_path(config, &parent)
                    ),
                ));
            }
            for entry in fs::read_dir(&parent)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                let path = entry.path();
                // §AR-workspace.5.3: `packages/*` skips hidden dirs (`.git`,
                // `.agents`, ...) — they are never workspace members and are
                // not valid aliases either.
                if is_hidden(&path) {
                    continue;
                }
                // A glob child is written by the glob: `packages/*` names
                // `packages/api`, which is the form a diagnostic can point at.
                let written = format!("{glob_parent}/{}", entry.file_name().to_string_lossy());
                let root = workspace_member_root(config, &written, &path)?;
                roots.push(WorkspaceMember { written, root });
            }
        } else {
            let root = workspace_member_root(config, member, &config.root.join(member))?;
            roots.push(WorkspaceMember {
                written: member.clone(),
                root,
            });
        }
    }
    roots.sort_by_key(|member| sort_path_key(&member.root));
    // §FS-workspace.6.1: two entries naming one root are one member — a glob is
    // allowed to name what an explicit entry also names — so dedup by root, and
    // keep the earlier entry's spelling for any diagnostic below.
    roots.dedup_by(|later, earlier| later.root == earlier.root);
    reject_overlapping_workspace_members(config, &roots)?;
    Ok(roots)
}

fn reject_overlapping_workspace_members(config: &Config, members: &[WorkspaceMember]) -> Result<()> {
    for (i, parent) in members.iter().enumerate() {
        for (j, child) in members.iter().enumerate() {
            if i != j && child.root.starts_with(&parent.root) {
                return Err(workspace_members_error(
                    config,
                    format!(
                        "workspace members overlap: `{}` contains `{}`",
                        parent.written, child.written
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// §FS-workspace.6.1: resolve one `members` entry to the canonical project root
/// it names, or the located error that entry earns. Both errors are reported at
/// the block's `members` line and name the entry **as written**: a canonical root
/// renders as nothing when it equals the render base and as an absolute path when
/// it lies outside it, and neither is something an author can act on
/// (§FS-errors.4).
///
/// The entry has to exist, and it has to resolve *strictly inside* the block that
/// listed it. `..` is already rejected in the entry text (§FS-workspace.2), so
/// escaping takes a symlink — and a member root outside its own block breaks what
/// everything above it assumes. No lexical ancestor lists it, so its alias path is
/// read from a different chain at every scope and no citation text passes both; a
/// root that is an *ancestor* of its own block scans nothing at all, because every
/// scan root lies under its own member boundary, so the project's declarations
/// vanish and its dangling citations pass (§GOAL-no-dangling-refs).
fn workspace_member_root(config: &Config, written: &str, lexical: &Path) -> Result<PathBuf> {
    if !lexical.is_dir() {
        return Err(workspace_members_error(
            config,
            format!("workspace member does not exist: {written}"),
        ));
    }
    let root = fs::canonicalize(lexical).unwrap_or_else(|_| lexical.to_path_buf());
    let block_root = canonical_workspace_path(&config.root);
    // Strictly inside: equal is the `self` symlink, not-a-prefix is every other
    // escape, and the two differ only in the preposition the message needs.
    if root == block_root || !root.starts_with(&block_root) {
        let landing = if root == block_root { "to" } else { "outside" };
        return Err(workspace_members_error(
            config,
            format!("workspace member `{written}` resolves {landing} the workspace root that lists it"),
        ));
    }
    Ok(root)
}
