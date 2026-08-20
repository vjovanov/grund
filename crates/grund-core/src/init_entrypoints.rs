// Which agent entrypoint files a repository *has* (§FS-init.2.1): the table of
// paths each supported agent reads, and the rules that decide whether a file
// standing at one of them is that agent's entrypoint or somebody else's. This is
// the half that only ever asks the tree — `init_plan.rs` turns these answers
// into the plan one run acts on, `init_templates.rs` answers what bytes go in
// them, and `init.rs` performs the writes (§AR-core-module-layout.1).
//
// The table below is the one place the supported agent set is spelled out, and
// the agent behind each row is what makes one entrypoint per agent decidable
// (§FS-init.2.1.1). `grund check`'s companion scan resolves through the same
// rules, so the checker and the writer cannot disagree about what an entrypoint
// is (§FS-check.3.5).

const CANONICAL_AGENT_ENTRYPOINT: &str = "AGENTS.md";
const COMPANION_AGENT_ENTRYPOINTS: &[CompanionAgentEntrypoint] = &[
    CompanionAgentEntrypoint {
        rel: "AGENTS.override.md",
        workspace: None,
        agent: None,
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: "CLAUDE.md",
        workspace: Some(".claude"),
        agent: Some(AgentEntrypoint::Claude),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: ".claude/CLAUDE.md",
        workspace: Some(".claude"),
        agent: Some(AgentEntrypoint::Claude),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: "GEMINI.md",
        workspace: Some(".gemini"),
        agent: Some(AgentEntrypoint::Gemini),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: ".pi/AGENTS.md",
        workspace: Some(".pi"),
        agent: Some(AgentEntrypoint::Pi),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: ".github/copilot-instructions.md",
        workspace: None,
        agent: Some(AgentEntrypoint::Copilot),
        discovery: true,
        create_on_request: true,
    },
    // §FS-init.2.1 / §FS-init.2.3: Cursor uses `.cursor/rules/*.mdc` files (the
    // modern form) and a legacy `.cursorrules` single-file form. We create a
    // grund-specific `.cursor/rules/grund.mdc` (won't collide with any other
    // rule file) when `.cursor/` already exists or `--cursor` is passed; the
    // legacy `.cursorrules` is only updated if it already exists, never
    // created — the modern path is preferred for new adopters.
    CompanionAgentEntrypoint {
        rel: ".cursor/rules/grund.mdc",
        workspace: Some(".cursor"),
        agent: Some(AgentEntrypoint::Cursor),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: ".cursorrules",
        workspace: None,
        agent: Some(AgentEntrypoint::Cursor),
        discovery: true,
        create_on_request: false,
    },
    CompanionAgentEntrypoint {
        rel: ".windsurfrules",
        workspace: None,
        agent: Some(AgentEntrypoint::Windsurf),
        discovery: true,
        create_on_request: true,
    },
    // §FS-init.2.3: `.rules` is too generic to attribute to Zed by filename
    // alone, so we only touch it when the `.zed/` workspace already exists or
    // `--zed` is explicit — discovery-by-file-existence is disabled.
    CompanionAgentEntrypoint {
        rel: ".rules",
        workspace: Some(".zed"),
        agent: Some(AgentEntrypoint::Zed),
        discovery: false,
        create_on_request: true,
    },
];

#[derive(Clone, Copy, Eq, PartialEq)]
enum AgentEntrypoint {
    Claude,
    Gemini,
    Pi,
    Copilot,
    Cursor,
    Windsurf,
    Zed,
}

impl AgentEntrypoint {
    /// The agent's name as a report spells it (§FS-init.2.1.1).
    fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Gemini => "Gemini",
            Self::Pi => "Pi",
            Self::Copilot => "Copilot",
            Self::Cursor => "Cursor",
            Self::Windsurf => "Windsurf",
            Self::Zed => "Zed",
        }
    }
}

/// How far the canonical entrypoint's own render reaches (§FS-init.2.1.1) — the
/// one question a companion *symlinked* to `AGENTS.md` turns on: does the agent
/// reading through that link get the block it should, or a form it is not meant
/// to read? The answer is a property of the effective config, not of any one
/// path, so it is decided once per run and carried; every site that asks about a
/// symlinked companion asks it through [`Self::leaves_uncovered`], so the rule
/// has one spelling and a second gated surface changes only this file.
#[derive(Clone, Copy, Eq, PartialEq)]
enum CanonicalSurfaceReach {
    /// The canonical render is what every entrypoint would carry, so a symlink
    /// to it is simply its agent's copy of the block.
    EveryEntrypoint,
    /// `[reference] conversation = "link"` gates the Claude form away from the
    /// canonical file, which every other agent reads too (§FS-init.2.3.4.17), so
    /// the canonical render can only speak for entrypoints on the plain surface.
    PlainEntrypointsOnly,
}

impl CanonicalSurfaceReach {
    fn for_config(config: &Config) -> Self {
        if config.conversation.as_deref() == Some("link") {
            Self::PlainEntrypointsOnly
        } else {
            Self::EveryEntrypoint
        }
    }

    /// Whether a companion symlinked to the canonical entrypoint leaves its
    /// agent without the block it should read (§FS-init.2.1.1). Only when the
    /// canonical render is gated *and* this path's surface is one it cannot
    /// carry: the surfaces are what differ (§FS-init.2.3.4.17), so a symlink
    /// whose surface is the canonical file's own carries exactly the bytes that
    /// file carries.
    fn leaves_uncovered(self, path: &Path) -> bool {
        self == Self::PlainEntrypointsOnly
            && ConversationSurface::for_entrypoint(path) != ConversationSurface::Plain
    }
}

struct CompanionAgentEntrypoint {
    rel: &'static str,
    workspace: Option<&'static str>,
    agent: Option<AgentEntrypoint>,
    /// Whether automatic mode should detect this entrypoint by file existence
    /// alone. `false` for entrypoints whose filename is too generic to
    /// attribute to a single tool (e.g. `.rules`) — those rely on the
    /// workspace directory or an explicit agent flag instead.
    discovery: bool,
    /// Whether an explicit agent flag creates this entrypoint when it is absent.
    /// Legacy Cursor `.cursorrules` is updated when present but never created;
    /// new Cursor installs use `.cursor/rules/grund.mdc` instead (§FS-init.2.1).
    create_on_request: bool,
}

enum InitCompanionAgentEntrypoint {
    Existing(PathBuf),
    MissingAlias(PathBuf),
}

impl InitCompanionAgentEntrypoint {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Existing(path) | Self::MissingAlias(path) => path.as_path(),
        }
    }
}

/// Existing companion agent entrypoints that should carry the same managed grund
/// block as `AGENTS.md` (§FS-init.2.1) — what `grund check` validates. A symlink
/// to `AGENTS.md` is already covered by the canonical file and is intentionally
/// skipped. Generic non-discovery entrypoints (currently `.rules`) are included
/// only when their workspace proves ownership or the file already carries a
/// managed grund block.
///
/// This is `init`'s existing-entrypoint set with the plan stripped off, and it
/// is derived from it rather than restated: the two answer the same question —
/// which companion files does this repository have — and a second copy of that
/// walk is a place for `check` and `init` to disagree about what an entrypoint
/// is (§FS-check.3.5).
fn companion_agent_entrypoints(root: &Path) -> Result<Vec<PathBuf>, (PathBuf, String)> {
    let (_, companions) = existing_init_companion_agent_entrypoints(root)?;
    Ok(companions
        .iter()
        .map(|companion| companion.path().to_path_buf())
        .collect())
}

/// Companion entrypoints `grund init` should update or create (§FS-init.2.1).
/// Existing companions are updated in place. Generic non-discovery entrypoints
/// are not selected by filename alone, but they are selected when the owning
/// workspace exists or when a previous `grund init` left a managed block there.
///
/// The first half of the pair is the companion paths that are *symlinks* to the
/// canonical entrypoint. They are never in the plan — the canonical file is the
/// one write that reaches them — but they are entrypoints the run puts the block
/// in front of, which is what the duplicate-entrypoint `note:` has to count
/// (§FS-init.2.1.1). Returning them rather than a bare "some companion was a
/// symlink" flag is what lets that note be built from the plan instead of a
/// second walk of the same table.
fn existing_init_companion_agent_entrypoints(
    root: &Path,
) -> Result<(Vec<PathBuf>, Vec<InitCompanionAgentEntrypoint>), (PathBuf, String)> {
    let mut paths = Vec::new();
    let mut canonical_symlinks = Vec::new();
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if is_file_or_symlink(&path) {
            match is_symlink_to(&path, &canonical) {
                Ok(true) => {
                    canonical_symlinks.push(path);
                    continue;
                }
                Ok(false) => {
                    if companion_selected_by_evidence(root, entrypoint, &path) {
                        paths.push(InitCompanionAgentEntrypoint::Existing(path));
                    }
                }
                Err(err) => return Err((path, format!("{err:#}"))),
            }
        }
    }
    Ok((canonical_symlinks, paths))
}

/// The agents that already carry the managed block in a file of their own
/// (§FS-init.2.1.1) — the ones `init` has something to update and so must not
/// create a second file for.
///
/// A companion that is a *symlink* to the canonical entrypoint counts, because
/// the agent reads that block through it (§FS-init.2.1): creating a second file
/// beside it would hand the agent the same bytes twice, which is the thing this
/// rule exists to stop. The one exception is the one that makes the two files
/// differ — a repository committing `[reference] conversation = "link"`, whose
/// canonical file cannot carry the Claude form (§FS-init.2.3.4.17). There the
/// symlink resolves to the plain form, the agent has no file carrying its own,
/// and an explicit request writes it the first path the symlink has not taken.
///
/// A file at one of these paths is the agent's entrypoint only on the evidence
/// the rest of this module uses (`companion_selected_by_evidence`): a `.rules`
/// that no `.zed/` and no managed block claims for Zed is somebody else's file,
/// and answering *does this agent have an entrypoint* with a looser definition
/// than `check` and the update set use is how the two come to disagree about
/// what an entrypoint is (§FS-check.3.5).
fn agents_with_own_entrypoint(
    root: &Path,
    reach: CanonicalSurfaceReach,
) -> Result<Vec<AgentEntrypoint>, (PathBuf, String)> {
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    let mut agents = Vec::new();
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let Some(agent) = entrypoint.agent.filter(|agent| !agents.contains(agent)) else {
            continue;
        };
        let path = root.join(entrypoint.rel);
        if !is_file_or_symlink(&path) {
            continue;
        }
        match is_symlink_to(&path, &canonical) {
            Ok(true) if reach.leaves_uncovered(&path) => continue,
            // A symlink to the canonical file is grund-owned by construction, so
            // only a file standing on its own has to prove it.
            Ok(true) => agents.push(agent),
            Ok(false) if companion_selected_by_evidence(root, entrypoint, &path) => {
                agents.push(agent)
            }
            Ok(false) => continue,
            Err(err) => return Err((path, format!("{err:#}"))),
        }
    }
    Ok(agents)
}

fn companion_workspace_exists(root: &Path, entrypoint: &CompanionAgentEntrypoint) -> bool {
    entrypoint
        .workspace
        .is_some_and(|workspace| root.join(workspace).is_dir())
}

/// Whether the on-disk `path` is grund-owned despite belonging to an entrypoint
/// whose filename is too generic to attribute by existence alone (currently
/// `.rules`, §FS-init.2.1). True when the entry is discovery-safe by filename,
/// when its owning workspace directory proves ownership, or when the file
/// already carries a managed block from a prior `grund init` — same evidence
/// for both `grund check`'s companion scan and `grund init`'s update set, so
/// both call sites resolve through one helper. Ordering matters: `discovery`
/// is the cheap-path short-circuit so most companions never touch the disk.
fn companion_selected_by_evidence(
    root: &Path,
    entrypoint: &CompanionAgentEntrypoint,
    path: &Path,
) -> bool {
    entrypoint.discovery
        || companion_workspace_exists(root, entrypoint)
        || companion_has_managed_block(path)
}

fn companion_has_managed_block(path: &Path) -> bool {
    // Malformed delimiters still prove grund ownership — selecting the file
    // lets `check` surface the defect instead of silently skipping it.
    fs::read_to_string(path).is_ok_and(|text| {
        !matches!(find_agents_block(&text), AgentsBlockLookup::Absent)
    })
}

fn is_file_or_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type())
        .is_ok_and(|t| t.is_file() || t.is_symlink())
}

fn path_missing_without_following_symlinks(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(_) => false,
        Err(err) => err.kind() == std::io::ErrorKind::NotFound,
    }
}

fn is_symlink_to(path: &Path, target: &Path) -> Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let link = fs::read_link(path)?;
    let resolved = if link.is_absolute() {
        link
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(link)
    };
    Ok(normalize_path_lexically(&resolved) == normalize_path_lexically(target))
}
