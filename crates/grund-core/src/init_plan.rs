// The plan one `grund init` run makes: which entrypoint files *this* invocation
// writes, appends to, or updates (§FS-init.2.1, §FS-init.2.1.1). Its input is
// what the repository has — `init_entrypoints.rs` answers that — plus the flags
// the user passed; its output is the one value the writing half in `init.rs` and
// the `note:` builders in `init_notes.rs` both read, so neither has to ask the
// tree a question the selection already answered (§AR-core-module-layout.1).

#[derive(Clone, Default)]
pub struct InitAgentEntrypointSelection {
    pub canonical: bool,
    pub claude: bool,
    pub gemini: bool,
    pub pi: bool,
    pub copilot: bool,
    pub cursor: bool,
    pub windsurf: bool,
    pub zed: bool,
}

impl InitAgentEntrypointSelection {
    fn any(&self) -> bool {
        self.canonical
            || self.claude
            || self.gemini
            || self.pi
            || self.copilot
            || self.cursor
            || self.windsurf
            || self.zed
    }

    fn includes(&self, agent: AgentEntrypoint) -> bool {
        match agent {
            AgentEntrypoint::Claude => self.claude,
            AgentEntrypoint::Gemini => self.gemini,
            AgentEntrypoint::Pi => self.pi,
            AgentEntrypoint::Copilot => self.copilot,
            AgentEntrypoint::Cursor => self.cursor,
            AgentEntrypoint::Windsurf => self.windsurf,
            AgentEntrypoint::Zed => self.zed,
        }
    }
}

struct SelectedInitAgentEntrypoints {
    canonical: bool,
    companions: Vec<InitCompanionAgentEntrypoint>,
    /// The companion paths that are symlinks to the canonical entrypoint. Never
    /// in `companions` — writing `AGENTS.md` is what reaches them — but they are
    /// copies of the block for their agent all the same, which is what the
    /// duplicate-entrypoint note counts (§FS-init.2.1.1). Carried on the plan so
    /// that note is built from what the selection already learned rather than
    /// from a second walk of the same table.
    canonical_symlinks: Vec<PathBuf>,
}

impl SelectedInitAgentEntrypoints {
    /// The `<path>`-relative entrypoints in this plan that Claude reads
    /// (§FS-init.2.3.4.17) — what tells the shadowed-entrypoint note which file
    /// this run makes current for Claude, when it makes one current.
    fn companions_of_claude(&self, target: &Path) -> Vec<String> {
        self.companions
            .iter()
            .map(|companion| companion.path())
            .filter(|path| ConversationSurface::for_entrypoint(path) == ConversationSurface::Linked)
            .map(|path| format_path(path.strip_prefix(target).unwrap_or(path)))
            .collect()
    }

    /// Every entrypoint path this plan would write, append to, or update — what
    /// §FS-init.1.2's user-global rule has to be asked about. The canonical
    /// entrypoint is as `<path>`-relative as any companion, but it is carried
    /// here as a flag rather than a path, so a caller that reasons about paths
    /// cannot see it at all unless it is rebuilt: `<path>/AGENTS.md` with
    /// `<path>` at `~/.codex` is the machine-global Codex instruction file.
    fn planned_paths(&self, target: &Path) -> Vec<PathBuf> {
        self.canonical
            .then(|| target.join(CANONICAL_AGENT_ENTRYPOINT))
            .into_iter()
            .chain(
                self.companions
                    .iter()
                    .map(|companion| companion.path().to_path_buf()),
            )
            .collect()
    }
}

fn selected_init_agent_entrypoints(
    target: &Path,
    selection: &InitAgentEntrypointSelection,
    reach: CanonicalSurfaceReach,
) -> Result<SelectedInitAgentEntrypoints, (PathBuf, String)> {
    if selection.any() {
        let (canonical_symlinks, companions) =
            requested_init_companion_agent_entrypoints(target, selection, reach)?;
        return Ok(SelectedInitAgentEntrypoints {
            canonical: selection.canonical || !canonical_symlinks.is_empty(),
            companions,
            canonical_symlinks,
        });
    }

    let canonical = target.join(CANONICAL_AGENT_ENTRYPOINT);
    let canonical_exists = is_file_or_symlink(&canonical);
    let (canonical_symlinks, existing_companions) =
        existing_init_companion_agent_entrypoints(target)?;
    if canonical_exists || !existing_companions.is_empty() {
        return Ok(SelectedInitAgentEntrypoints {
            canonical: canonical_exists || !canonical_symlinks.is_empty(),
            companions: existing_companions,
            canonical_symlinks,
        });
    }

    let workspace_companions = workspace_init_companion_agent_entrypoints(target, reach)?;
    if !canonical_symlinks.is_empty() || !workspace_companions.is_empty() {
        return Ok(SelectedInitAgentEntrypoints {
            canonical: !canonical_symlinks.is_empty(),
            companions: workspace_companions,
            canonical_symlinks,
        });
    }

    Ok(SelectedInitAgentEntrypoints {
        canonical: true,
        companions: Vec::new(),
        canonical_symlinks: Vec::new(),
    })
}

/// Missing neutral aliases are created only when their owning agent-specific
/// workspace directory already exists; generic project metadata directories
/// remain existing-file-only. At most one alias per agent (§FS-init.2.1.1) —
/// `.claude/` proves Claude is in use, which is one fact, not two files.
fn workspace_init_companion_agent_entrypoints(
    root: &Path,
    reach: CanonicalSurfaceReach,
) -> Result<Vec<InitCompanionAgentEntrypoint>, (PathBuf, String)> {
    let mut paths = Vec::new();
    let mut covered = agents_with_own_entrypoint(root, reach)?;
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if !companion_workspace_exists(root, entrypoint)
            || !path_missing_without_following_symlinks(&path)
        {
            continue;
        }
        if let Some(agent) = entrypoint.agent {
            if covered.contains(&agent) {
                continue;
            }
            covered.push(agent);
        }
        paths.push(InitCompanionAgentEntrypoint::MissingAlias(path));
    }
    Ok(paths)
}

/// Explicit agent flags create their requested companion entrypoints even when
/// the normal automatic detection would not choose them — one per agent
/// (§FS-init.2.1.1): every entrypoint the repository already has is updated,
/// and a missing one is created only for an agent that has none.
fn requested_init_companion_agent_entrypoints(
    root: &Path,
    selection: &InitAgentEntrypointSelection,
    reach: CanonicalSurfaceReach,
) -> Result<(Vec<PathBuf>, Vec<InitCompanionAgentEntrypoint>), (PathBuf, String)> {
    let mut paths = Vec::new();
    let mut canonical_symlinks = Vec::new();
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    let mut covered = agents_with_own_entrypoint(root, reach)?;
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let Some(agent) = entrypoint.agent.filter(|agent| selection.includes(*agent)) else {
            continue;
        };
        let path = root.join(entrypoint.rel);
        if is_file_or_symlink(&path) {
            match is_symlink_to(&path, &canonical) {
                Ok(true) => canonical_symlinks.push(path),
                Ok(false) => paths.push(InitCompanionAgentEntrypoint::Existing(path)),
                Err(err) => return Err((path, format!("{err:#}"))),
            }
        } else if entrypoint.create_on_request && !covered.contains(&agent) {
            covered.push(agent);
            paths.push(InitCompanionAgentEntrypoint::MissingAlias(path));
        }
    }
    Ok((canonical_symlinks, paths))
}
