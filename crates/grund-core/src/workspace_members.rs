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
                // Named as written, like every other member error: a rendered
                // path here is relative to *this block's* root, which is not the
                // base the report uses, and under `relative_paths = false` it
                // could not be made relative at all (§FS-errors.4).
                return Err(workspace_members_error(
                    config,
                    format!("workspace member glob parent does not exist: {glob_parent}"),
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

/// What one `members` entry *names*, read from the entry text alone — before
/// anyone asks whether that directory exists (§FS-workspace.6.1). This is the
/// half of a member list that cannot fail, and it is all the ancestor climb
/// needs to decide whether a block claims the directory below it.
struct MemberClaim {
    /// The entry joined onto the block's root, exactly as written.
    lexical: PathBuf,
    /// The same path resolved as far as the filesystem allows: a member reached
    /// through a symlink names its target, which is the form the expanded list
    /// holds and the form a run *inside* that directory reports as its root.
    canonical: PathBuf,
    /// A `<parent>/*` entry names the visible directories **under** `lexical`,
    /// not `lexical` itself.
    glob: bool,
}

impl MemberClaim {
    fn names(&self, child: &Path, canonical_child: &Path) -> bool {
        if self.glob {
            // §AR-workspace.5.3: a glob never names a hidden directory, so one
            // is not claimed by a block whose glob parent happens to hold it.
            return !is_hidden(canonical_child)
                && (child.parent() == Some(self.lexical.as_path())
                    || canonical_child.parent() == Some(self.canonical.as_path()));
        }
        self.lexical == child || self.canonical == canonical_child
    }
}

fn member_claims(config: &Config) -> Vec<MemberClaim> {
    config
        .workspace_members
        .iter()
        .map(|entry| {
            let (path, glob) = match entry.strip_suffix("/*") {
                Some(parent) => (parent, true),
                None => (entry.as_str(), false),
            };
            let lexical = config.root.join(path);
            MemberClaim {
                canonical: canonical_workspace_path(&lexical),
                lexical,
                glob,
            }
        })
        .collect()
}

/// One ancestor `[workspace]` block: its config, what its `members` list names,
/// and — once some directory below it turns out to be claimed — the expanded
/// member roots.
struct AncestorBlock {
    config: Config,
    claims: Vec<MemberClaim>,
    members: Option<Vec<PathBuf>>,
}

/// The `[workspace]` blocks an ancestor climb has already looked at, keyed by
/// directory (§AR-workspace.6.1). The climb asks the same ancestors about every
/// level below them — level *n* re-walks all of level *n+1*'s ancestors — so
/// without this each ancestor's config is re-read, and its grammar regex set
/// rebuilt, once per level: quadratic in the depth of the chain, and measurably
/// so, a run narrowed deep into a 40-level tree taking longer than checking the
/// whole tree from its root. One cache belongs to one climb, which is also why it
/// needs no invalidation.
struct AncestorWorkspaces {
    /// The base a diagnostic from one of these blocks renders its config path
    /// against: the root this run was launched at. An ancestor's config lies
    /// *above* that root, so it renders with `..` — without this it rendered
    /// relative to its own root, and `.agents/grund.toml:16` then named a
    /// same-shaped file in the reader's own directory (§FS-errors.4).
    report_base: PathBuf,
    blocks: BTreeMap<PathBuf, Option<AncestorBlock>>,
}

impl AncestorWorkspaces {
    fn for_run_at(root: &Path) -> Self {
        Self {
            report_base: root.to_path_buf(),
            blocks: BTreeMap::new(),
        }
    }

    /// The `[workspace]` block at `dir` **when it claims `child`**, or `None`.
    ///
    /// §FS-workspace.6.1 scopes every obligation of this walk to a *claim*: a
    /// block that claims a directory and cannot answer fails the run with its own
    /// error, and one that no enclosing block lists says nothing about this tree.
    /// So the claim is decided first, on the entry text (`MemberClaim`), and the
    /// member list is expanded — and its error propagated — only for a block that
    /// names `child`. Expanding every declaring ancestor instead made one broken
    /// `members` list anywhere above a repository the answer to every command
    /// inside it, at any depth up to `/`, for a block that claimed nothing here.
    ///
    /// The claim is confirmed against the expanded roots, not the entry text:
    /// what a glob names and where a symlinked entry lands are answers only
    /// expansion has.
    fn claiming_block(
        &mut self,
        dir: &Path,
        child: &Path,
        canonical_child: &Path,
        cli_base: &Path,
    ) -> Result<Option<&Config>> {
        if !self.blocks.contains_key(dir) {
            let block = load_ancestor_workspace_block(dir, cli_base, &self.report_base);
            self.blocks.insert(dir.to_path_buf(), block);
        }
        let Some(block) = self.blocks.get_mut(dir).and_then(Option::as_mut) else {
            return Ok(None);
        };
        if !block
            .claims
            .iter()
            .any(|claim| claim.names(child, canonical_child))
        {
            return Ok(None);
        }
        if block.members.is_none() {
            block.members = Some(expand_workspace_members(&block.config)?);
        }
        let claimed = block
            .members
            .as_ref()
            .is_some_and(|members| members.iter().any(|root| root == canonical_child));
        Ok(claimed.then_some(&block.config))
    }
}

fn load_ancestor_workspace_block(
    dir: &Path,
    cli_base: &Path,
    report_base: &Path,
) -> Option<AncestorBlock> {
    config_file_in(dir)?;
    // A config that does not even load is no claim at all: this walk climbs to
    // the filesystem root, and a stray unparseable `grund.toml` above the
    // repository must not break every run beneath it (§AR-workspace.6.1).
    let config = load_config_at_with_report_base(dir, cli_base, Some(report_base)).ok()?;
    if !config.workspace_declared {
        return None;
    }
    let claims = member_claims(&config);
    Some(AncestorBlock {
        config,
        claims,
        members: None,
    })
}
