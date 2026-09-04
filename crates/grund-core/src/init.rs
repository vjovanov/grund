#[derive(Clone)]
pub struct InitOpts {
    pub target: PathBuf,
    pub name: Option<String>,
    /// `--description` — pending one-line `project_description` for a freshly
    /// written config (§FS-init.1, §DF-workspace-member-descriptions).
    pub description: Option<String>,
    pub docs: bool,
    pub force: bool,
    pub dry_run: bool,
    /// `--check` — the `--dry-run` preview taken as a verdict (§FS-init.1):
    /// writes nothing, reports what `--dry-run` reports, and leaves the caller
    /// to exit `1` when any reported event is a change (§FS-init.4). It implies
    /// `dry_run` inside `init` rather than opening a second path through it.
    pub check: bool,
    /// `--no-vcs` — scaffold into a target no version-control marker covers
    /// (§FS-init.1.2). Lifts that rule and only that one; it is not `--force`,
    /// which decides whether files `init` owns get overwritten (§FS-init.3).
    pub no_vcs: bool,
    pub agent_selection: InitAgentEntrypointSelection,
}

impl Default for InitOpts {
    fn default() -> Self {
        Self {
            target: PathBuf::from("."),
            name: None,
            description: None,
            docs: false,
            force: false,
            dry_run: false,
            check: false,
            no_vcs: false,
            agent_selection: InitAgentEntrypointSelection::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitEvent {
    pub verb: &'static str,
    pub path: String,
}

impl InitEvent {
    /// Whether this event reports work rather than a path that was already
    /// current. Every verb but `exists` is a change — `wrote`/`appended`/
    /// `updated` and their `would-` forms alike. The one definition of the
    /// predicate: it suppresses the `next:` block (§FS-init.2.2) and it decides
    /// the `--check` exit code (§FS-init.4), which is why those two agree.
    pub fn is_change(&self) -> bool {
        self.verb != "exists"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitNext {
    pub docs: bool,
    pub entrypoint: String,
    pub fs_home: InitFsHome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InitFsHome {
    File { path: String, heading_name: &'static str, heading_marker: &'static str },
    Folder { path: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InitOutput {
    pub events: Vec<InitEvent>,
    /// Things the run could not do that the caller would otherwise have to
    /// notice for itself (§FS-init.2.3.4.17). Reported, never fatal.
    pub notes: Vec<String>,
    pub next: Option<InitNext>,
}

impl InitOutput {
    /// Whether the run reported anything left to do — the verdict `--check`
    /// draws from the report it just printed (§FS-init.4). Notes and the
    /// `next:` block are deliberately not consulted: a note is a report, not a
    /// finding.
    pub fn has_pending_changes(&self) -> bool {
        self.events.iter().any(InitEvent::is_change)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitError {
    pub output: InitOutput,
    pub message: String,
}

impl InitError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            output: InitOutput::default(),
            message: message.into(),
        }
    }

    fn with_events(events: Vec<InitEvent>, message: impl Into<String>) -> Self {
        Self {
            output: InitOutput {
                events,
                notes: Vec::new(),
                next: None,
            },
            message: message.into(),
        }
    }
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for InitError {}

/// Scaffold a grund setup into `opts.target`: the agent-instruction
/// entrypoints, `grund.toml`, and — with `--docs` — the documentation stubs.
///
/// Why the effective config is read before the entrypoint plan: one selection
/// rule depends on it. A companion symlinked to `AGENTS.md` leaves its agent
/// covered unless that key makes the canonical file unable to carry that
/// agent's form.
///
/// Why the managed block is rendered once and reused for both surfaces: the
/// workspace-members walk-up is non-trivial I/O for a large workspace and
/// produces byte-identical output each time. The selected entrypoint plan
/// determines whether a missing self `AGENTS.md` should be treated as
/// about-to-exist; companion-only init must not link to a missing canonical
/// entrypoint. Two surfaces at most: the local-conversation sentence differs
/// between the Claude entrypoints and everything else, and nothing else in the
/// block does. The linked variant is rendered only when a Claude entrypoint is
/// actually selected, so the common run still walks the workspace once.
///
/// Why the duplicate-entrypoint notes and the Claude companions are computed
/// before the companion loop: the loop consumes the plan. `init` creates one
/// entrypoint per agent, but a repository that already carries two keeps both,
/// and this run is where that shows; and the entrypoint the symlink note names
/// is the one this run makes current, when it makes one current.
///
/// What `init` will and will not overwrite: `grund.toml` is the project's
/// configuration — the surface a repo customizes (kinds, marker, scan scope,
/// …). `init` writes the canonical template only when the target has **no**
/// config under either discovery name; an existing one is never overwritten,
/// not even with `--force`, and is reported under the name it was found at so a
/// repo on the `.agents/` form never grows the redundant pair. `--force`
/// targets the things `init` owns end to end — the managed agent-instructions
/// block and the `--docs` scaffold stubs — not the user's settings.
///
/// Why the shadowed-entrypoint note is emitted: the committed `link` opinion is
/// rendered per entrypoint, and a Claude entrypoint that is a symlink to
/// `AGENTS.md` is the canonical file — which every other agent reads too, so it
/// must keep the plain form.
pub fn init(opts: InitOpts) -> std::result::Result<InitOutput, InitError> {
    let InitOpts {
        target,
        name,
        description,
        docs,
        force,
        dry_run,
        check,
        no_vcs,
        agent_selection,
    } = opts;
    // §FS-init.1: `--check` is the `--dry-run` run taken as a verdict, so it
    // suppresses writes through that same flag rather than a second code path —
    // which is what makes the two reports identical by construction.
    let dry_run = dry_run || check;
    // §FS-init.1: `--description` mirrors the config-side single-line rule
    // (§FS-config.3) — reject line breaks before any file is touched.
    if let Some(description) = &description
        && (description.contains('\n') || description.contains('\r'))
    {
        return Err(InitError::new(
            "--description must be a single line".to_string(),
        ));
    }
    // §FS-init.1, §FS-init.1.2: what `<path>` is, and whether it may be
    // scaffolded at all. The companion rule that needs the entrypoint plan runs
    // below, where that plan first exists; both are ahead of every write.
    if let Some(message) = refuse_init_target(&target, no_vcs) {
        return Err(InitError::new(message));
    }

    let resolved_name = match name {
        Some(value) => value,
        None => derive_default_name(&target).map_err(|err| InitError::new(err.to_string()))?,
    };

    // §FS-init.2.3: render agent instructions against the config `init` leaves
    // in place, so the ID-shape / kind / marker prose matches `grund.toml`; read
    // before the entrypoint plan (§FS-init.2.1.1, §FS-init.2.3.4.17).
    let init_config = init_pending_effective_config(&target, &resolved_name, description.as_deref())
        .map_err(|err| InitError::new(err.to_string()))?;
    let reach = CanonicalSurfaceReach::for_config(&init_config);

    let agent_entrypoints = match selected_init_agent_entrypoints(&target, &agent_selection, reach)
    {
        Ok(entrypoints) => entrypoints,
        Err((path, message)) => {
            return Err(InitError::new(format!(
                "inspect {}: {message}",
                path.display()
            )));
        }
    };

    // §FS-init.1.2: the planned entrypoint paths are known now, so check them
    // against the user-global instruction files `grund integrations --write`
    // owns (§FS-integrations.4.3) before any of them is written.
    if let Some(message) =
        refuse_init_global_instruction_paths(&agent_entrypoints.planned_paths(&target))
    {
        return Err(InitError::new(message));
    }

    // §FS-init.2.3.4.15, §FS-check.4.9: walked once and handed to both surfaces —
    // the section does not vary by surface, and this walk is where every block is
    // asked whether its members swallowed its scan, once per run.
    let workspace_members = agents_workspace_members_section(
        &resolved_name,
        &init_config,
        &target,
        agent_entrypoints.canonical,
    );
    // Render the managed block once and reuse it for both surfaces: the two
    // surfaces differ in one sentence only (§FS-init.2.3.4.17).
    let render_block = |surface| {
        render_agents_append_block(&resolved_name, &init_config, &workspace_members, surface)
    };
    let agents_block = render_block(ConversationSurface::Plain);
    let claude_block = agent_entrypoints
        .companions
        .iter()
        .any(|entrypoint| {
            ConversationSurface::for_entrypoint(entrypoint.path()) == ConversationSurface::Linked
        })
        .then(|| render_block(ConversationSurface::Linked));
    let agents_contents = render_agents_md_from_block(&resolved_name, &agents_block);
    // §FS-init.2.1.1, §FS-init.2.3.4.17: both computed before the companion loop
    // consumes the plan.
    let claude_companions = agent_entrypoints.companions_of_claude(&target);
    let mut notes =
        duplicate_agent_entrypoint_notes(&target, &agent_entrypoints, reach, dry_run);
    let mut workflow_entrypoint = None;
    // Track whether any path changed (or, under --dry-run, *would* change).
    // The `next:` block is suppressed when every reported path is `exists `,
    // since the user already has a complete grund setup (§FS-init.2.2).
    let mut any_change = false;
    let mut events = Vec::new();
    if agent_entrypoints.canonical {
        match write_or_update_canonical_agent_entrypoint(
            &target,
            CANONICAL_AGENT_ENTRYPOINT,
            &agents_contents,
            &agents_block,
            force,
            dry_run,
        ) {
            Ok(event) => {
                any_change |= event.is_change();
                events.push(event);
            }
            Err(message) => return Err(InitError::with_events(events, message)),
        }
        workflow_entrypoint = Some(CANONICAL_AGENT_ENTRYPOINT.to_string());
    }

    for entrypoint in agent_entrypoints.companions {
        let path_ref = entrypoint.path();
        // The Claude entrypoints teach the linked form; every other companion
        // gets the plain-location block (§FS-init.2.3.4.17).
        let entrypoint_block = match ConversationSurface::for_entrypoint(path_ref) {
            ConversationSurface::Linked => claude_block.as_deref().unwrap_or(&agents_block),
            ConversationSurface::Plain => &agents_block,
        };
        let rel = path_ref.strip_prefix(&target).unwrap_or(path_ref).to_path_buf();
        let rel = format_path(&rel);
        if workflow_entrypoint.is_none() {
            workflow_entrypoint = Some(rel.clone());
        }
        match entrypoint {
            InitCompanionAgentEntrypoint::Existing(path) => {
                match update_agents_block(&path, entrypoint_block, &rel, dry_run) {
                    Ok(AgentsUpdateResult::Appended) => {
                        events.push(InitEvent { verb: verb_appended(dry_run), path: rel });
                        any_change = true;
                    }
                    Ok(AgentsUpdateResult::Updated) => {
                        events.push(InitEvent { verb: verb_updated(dry_run), path: rel });
                        any_change = true;
                    }
                    Ok(AgentsUpdateResult::Unchanged) => events.push(InitEvent { verb: "exists", path: rel }),
                    Err(err) => {
                        return Err(InitError::with_events(
                            events,
                            // Forward slashes on every platform, like report
                            // paths (§FS-errors.2.2) — Windows must not leak
                            // backslashes into the message.
                            format!("update {}: {err}", format_path(&path)),
                        ));
                    }
                }
            }
            InitCompanionAgentEntrypoint::MissingAlias(path) => {
                if !dry_run
                    && let Some(parent) = path.parent()
                    && let Err(err) = fs::create_dir_all(parent)
                {
                    return Err(InitError::with_events(
                        events,
                        format!("create {}: {err}", parent.display()),
                    ));
                }
                if !dry_run
                    && let Err(err) = fs::write(&path, entrypoint_block)
                {
                    return Err(InitError::with_events(
                        events,
                        format!("write {}: {err}", path.display()),
                    ));
                }
                events.push(InitEvent { verb: verb_wrote(dry_run), path: rel });
                any_change = true;
            }
        }
    }

    // `grund.toml` is the project's configuration (§GOAL-configurable): written
    // only when the target has none (§FS-config.1), never overwritten, reported
    // under the name found (§FS-init.2.4, §FS-check.4.3, §FS-init.3).
    if let Some(existing) = config_file_in(&target) {
        let rel = existing.strip_prefix(&target).unwrap_or(&existing).to_path_buf();
        events.push(InitEvent { verb: "exists", path: format_path(&rel) });
    } else {
        // §DF-config-file-location.2.3: the bare, root-visible form is the one
        // `init` generates, so the default a new project meets is the one the
        // rest of the ecosystem uses.
        let config_rel = "grund.toml";
        // No `create_dir_all`: the destination's parent is `target`, already
        // verified to be an existing directory above.
        let config_dest = target.join(config_rel);
        if !dry_run
            && let Err(err) = fs::write(
                &config_dest,
                render_grund_toml(&resolved_name, description.as_deref()),
            )
        {
            return Err(InitError::with_events(
                events,
                format!("write {}: {err}", config_dest.display()),
            ));
        }
        events.push(InitEvent { verb: verb_wrote(dry_run), path: config_rel.to_string() });
        any_change = true;
    }

    let fs_home = init_fs_home(&init_config);
    let files: Vec<(String, String)> = if docs {
        docs_scaffold(&fs_home)
    } else {
        Vec::new()
    };
    for (rel, contents) in &files {
        let dest = target.join(rel);
        if !force && dest.exists() {
            events.push(InitEvent { verb: "exists", path: rel.clone() });
            continue;
        }
        if !dry_run
            && let Some(parent) = dest.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            return Err(InitError::with_events(
                events,
                format!("create {}: {err}", parent.display()),
            ));
        }
        if !dry_run
            && let Err(err) = fs::write(&dest, contents)
        {
            return Err(InitError::with_events(
                events,
                format!("write {}: {err}", dest.display()),
            ));
        }
        events.push(InitEvent { verb: verb_wrote(dry_run), path: rel.clone() });
        any_change = true;
    }

    let next = any_change.then(|| InitNext {
        docs,
        entrypoint: workflow_entrypoint.unwrap_or_else(|| CANONICAL_AGENT_ENTRYPOINT.to_string()),
        fs_home,
    });
    // §FS-init.2.3.4.17: silence here would read as the committed `link` opinion
    // simply not working.
    if reach == CanonicalSurfaceReach::PlainEntrypointsOnly {
        match shadowed_claude_entrypoint_note(&target, &claude_companions, dry_run) {
            Ok(Some(note)) => notes.push(note),
            Ok(None) => {}
            // The same hard failure the selection gives for a path it cannot
            // inspect: this note is the only place the state is visible, so
            // dropping it on an unreadable link would report a clean run.
            Err((path, message)) => {
                return Err(InitError::with_events(
                    events,
                    format!("inspect {}: {message}", path.display()),
                ));
            }
        }
    }
    Ok(InitOutput {
        events,
        notes,
        next,
    })
}

/// The trailing `next:` guidance block (§FS-init.2.2). Suppressed by the caller
/// when every reported path was `exists ` — when the repo is already current
/// there is no next step to teach. `entrypoint` is the first agent entrypoint
/// `init` touched, used in the final `see <entrypoint> …` pointer; `None`
/// falls back to the canonical `AGENTS.md`.
fn render_next_block_for_home(
    docs: bool,
    entrypoint: Option<&str>,
    fs_home: &InitFsHome,
) -> String {
    let mut output = "\nnext:\n".to_string();
    if docs {
        output.push_str("  1. run `grund check` — a freshly scaffolded tree is clean\n");
        match fs_home {
            InitFsHome::File { path, heading_name, heading_marker } => {
                output.push_str(&format!(
                    "  2. allocate an ID:  ID=$(grund id FS \"…\")  then add it to {path}\n"
                ));
                output.push_str(&format!(
                    "     ({heading_name}: `{heading_marker} <ID>: <one-line statement of the behavior>`)\n"
                ));
            }
            InitFsHome::Folder { path } => {
                output.push_str(&format!(
                    "  2. allocate an ID:  ID=$(grund id FS \"…\")  then add it under {path}\n"
                ));
                output.push_str("     (H1: `# <ID>: <one-line statement of the behavior>`)\n");
            }
        }
        output.push_str(
            "  3. cite it as §<ID> from the docs and e2e tests that depend on it, then `grund check` again\n",
        );
    } else {
        let fs_home_path = match fs_home {
            InitFsHome::File { path, .. } | InitFsHome::Folder { path } => path,
        };
        output.push_str(&format!(
            "  1. re-run with --docs to scaffold the FS home ({fs_home_path}), docs/, and tests/ (or create them yourself) — until then `grund check` has nothing to scan\n"
        ));
        output.push_str("  2. run `grund check` — a scaffolded tree is clean\n");
        match fs_home {
            InitFsHome::File { path, .. } => {
                output.push_str(&format!(
                    "  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it to {path}\n"
                ));
            }
            InitFsHome::Folder { path } => {
                output.push_str(&format!(
                    "  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it under {path}\n"
                ));
            }
        }
    }
    output.push_str(&format!(
        "see {} for the full workflow.\n",
        entrypoint.unwrap_or(CANONICAL_AGENT_ENTRYPOINT)
    ));
    output
}

/// Stderr verb for a newly written file. `--dry-run` reports `would-write `
/// instead of `wrote `; otherwise the verbs match a real run (§FS-init.2.2).
fn verb_wrote(dry_run: bool) -> &'static str {
    if dry_run { "would-write" } else { "wrote" }
}

fn verb_appended(dry_run: bool) -> &'static str {
    if dry_run { "would-append" } else { "appended" }
}

fn verb_updated(dry_run: bool) -> &'static str {
    if dry_run { "would-update" } else { "updated" }
}

/// The `--docs` scaffold: the default requirements/spec home, canonical `docs/`
/// files (`grund.md`, `goals.md`, `roadmap.md`, `changelog.md`, and an index
/// README for each folder kind that has one — architecture and the two decision
/// folders), plus the two test homes — the file list of §FS-init.2.1, each a
/// minimal starter that leaves `grund check` clean.
fn init_fs_home(config: &Config) -> InitFsHome {
    if let Some(kind) = config.kinds.iter().find(|kind| kind.kind == "FS") {
        if let Some(file) = &kind.file {
            let (heading_name, heading_marker) = if file == "docs/grund.md" {
                ("H1", "#")
            } else {
                ("H2", "##")
            };
            return InitFsHome::File {
                path: file.clone(),
                heading_name,
                heading_marker,
            };
        }
        if let Some(folder) = &kind.folder {
            return InitFsHome::Folder { path: folder.clone() };
        }
    }
    InitFsHome::File {
        path: "requirements.md".to_string(),
        heading_name: "H2",
        heading_marker: "##",
    }
}

/// The `--docs` scaffold: each stub `init` writes, paired with the path it
/// lands at.
///
/// Which folder kinds get an index README: under the generated config's
/// defaults that is `AR`, `DF` and `DA`, while `E2E` sets `index = false`.
///
/// What the two test homes get instead: `tests/e2e/README.md` is the layout
/// note, and `tests/integration` gets the placeholder that makes an empty
/// directory survive `git add`.
fn docs_scaffold(fs_home: &InitFsHome) -> Vec<(String, String)> {
    let mut files = Vec::new();
    match fs_home {
        InitFsHome::File { path, .. } => files.push((
            path.clone(),
            canonical_template_text(REQUIREMENTS_TEMPLATE),
        )),
        InitFsHome::Folder { path } => files.push((
            format!("{path}/README.md"),
            canonical_template_text(FS_README_TEMPLATE),
        )),
    }
    files.extend([
        ("docs/grund.md", canonical_template_text(GRUND_DOC_TEMPLATE)),
        (
            "docs/goals.md",
            canonical_template_text(GOALS_TEMPLATE),
        ),
        (
            "docs/roadmap.md",
            "# Roadmap\n\n<!-- placeholder - replace with real content -->\n".to_string(),
        ),
        (
            "docs/changelog.md",
            "# Changelog\n\n<!-- placeholder - replace with real content -->\n".to_string(),
        ),
        (
            "docs/architecture/README.md",
            canonical_template_text(AS_README_TEMPLATE),
        ),
        // §FS-init.2.1 / §FS-check.4.6: every folder kind the generated config
        // leaves at the default `index` gets its index README scaffolded, not a
        // bare `.gitkeep` (§FS-config.3.4).
        (
            "docs/decisions/architectural/README.md",
            canonical_template_text(DA_README_TEMPLATE),
        ),
        (
            "docs/decisions/functional/README.md",
            canonical_template_text(DF_README_TEMPLATE),
        ),
        // §FS-init.2.1: the two test homes the generated config names
        // (§FS-config.3.4). Both are non-citable kinds, so neither gets an index
        // README.
        (
            "tests/e2e/README.md",
            render_e2e_readme(fs_home),
        ),
        (
            "tests/integration/.gitkeep",
            canonical_template_text(GITKEEP_TEMPLATE),
        ),
    ]
    .into_iter()
    .map(|(path, contents)| (path.to_string(), contents)));
    files
}

fn render_e2e_readme(fs_home: &InitFsHome) -> String {
    let fs_home_path = match fs_home {
        InitFsHome::File { path, .. } | InitFsHome::Folder { path } => path,
    };
    canonical_template_text(E2E_README_TEMPLATE).replace("{fs_home}", fs_home_path)
}
