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

/// The `[workspace]` blocks an ancestor climb has already looked at, keyed by
/// directory (§AR-workspace.6.1). The climb asks the same ancestors about every
/// level below them — level *n* re-walks all of level *n+1*'s ancestors — so
/// without this each ancestor's config is re-read, and its grammar regex set
/// rebuilt, once per level: quadratic in the depth of the chain, and measurably
/// so, a run narrowed deep into a 40-level tree taking longer than checking the
/// whole tree from its root. One cache belongs to one climb, which is also why it
/// needs no invalidation.
#[derive(Default)]
struct AncestorWorkspaces {
    blocks: BTreeMap<PathBuf, Option<(Config, Vec<PathBuf>)>>,
}

impl AncestorWorkspaces {
    /// The `[workspace]` block `dir` declares together with the member roots it
    /// claims, or `None` when `dir` holds no config, its config does not load, or
    /// it declares no `[workspace]`. A block that declares one and cannot expand
    /// its members is that block's own error (§FS-workspace.6.1) — it ends the
    /// run, so it is never cached and never asked twice.
    fn block_at(
        &mut self,
        dir: &Path,
        cli_base: &Path,
    ) -> Result<Option<&(Config, Vec<PathBuf>)>> {
        if !self.blocks.contains_key(dir) {
            let block = load_ancestor_workspace_block(dir, cli_base)?;
            self.blocks.insert(dir.to_path_buf(), block);
        }
        Ok(self.blocks.get(dir).and_then(Option::as_ref))
    }
}

fn load_ancestor_workspace_block(
    dir: &Path,
    cli_base: &Path,
) -> Result<Option<(Config, Vec<PathBuf>)>> {
    if config_file_in(dir).is_none() {
        return Ok(None);
    }
    // A config that does not even load is no claim at all: this walk climbs to
    // the filesystem root, and a stray unparseable `grund.toml` above the
    // repository must not break every run beneath it (§AR-workspace.6.1).
    let Ok(config) = load_config_at(dir, cli_base) else {
        return Ok(None);
    };
    if !config.workspace_declared {
        return Ok(None);
    }
    let members = expand_workspace_members(&config)?;
    Ok(Some((config, members)))
}
