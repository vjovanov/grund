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

/// `grund init [path] [--name N] [--docs] [--force] [--dry-run] [agent flags]` —
/// scaffold a repo for `grund` (§FS-init.1): write or update the selected agent
/// entrypoint(s) and `grund.toml` (and, with `--docs`, the `docs/`+`e2e/`
/// tree, §FS-init.2.1), preserve an existing repo's agent-entrypoint choice by
/// default (§FS-init.2.1), refuse to clobber edited scaffold files without
/// `--force` — and never overwrite an existing `grund.toml`, in either discovery
/// location (§FS-config.1), even with `--force`, since that file is the user's
/// config (§FS-init.3) — print a `next:`
/// block (suppressed when every reported path is `exists `, §FS-init.2.2), and
/// exit `2` on a missing target / refused target (§FS-init.1.2) / CLI error /
/// unsupported block version
/// (§FS-init.4). Non-interactive — every choice is a flag (§FS-non-goals.10).
/// With `--dry-run`, every line is reported with a `would-` prefix and nothing
/// is written to disk.
fn command_init(args: &[String]) -> ExitCode {
    let mut path: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut docs = false;
    let mut force = false;
    let mut dry_run = false;
    let mut no_vcs = false;
    let mut agent_selection = InitAgentEntrypointSelection::default();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--docs" => docs = true,
            "--force" => force = true,
            "--dry-run" => dry_run = true,
            "--no-vcs" => no_vcs = true,
            "--agents-md" => agent_selection.canonical = true,
            "--claude" => agent_selection.claude = true,
            "--gemini" => agent_selection.gemini = true,
            "--copilot" => agent_selection.copilot = true,
            "--cursor" => agent_selection.cursor = true,
            "--windsurf" => agent_selection.windsurf = true,
            "--zed" => agent_selection.zed = true,
            "--name" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --name requires a value");
                    return ExitCode::from(2);
                }
                name = Some(args[idx].clone());
            }
            other if other.starts_with("--name=") => {
                name = Some(other.trim_start_matches("--name=").to_string());
            }
            "--description" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --description requires a value");
                    return ExitCode::from(2);
                }
                description = Some(args[idx].clone());
            }
            other if other.starts_with("--description=") => {
                description = Some(other.trim_start_matches("--description=").to_string());
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("error: init takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(other));
            }
        }
        idx += 1;
    }

    let output = match init(InitOpts {
        target: path.unwrap_or_else(|| PathBuf::from(".")),
        name,
        description,
        docs,
        force,
        dry_run,
        no_vcs,
        agent_selection,
    }) {
        Ok(output) => output,
        Err(err) => {
            print_init_output(&err.output);
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    print_init_output(&output);
    ExitCode::SUCCESS
}

pub fn init(opts: InitOpts) -> std::result::Result<InitOutput, InitError> {
    let InitOpts {
        target,
        name,
        description,
        docs,
        force,
        dry_run,
        no_vcs,
        agent_selection,
    } = opts;
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

    // §FS-init.2.3: render agent instructions against the config `init` leaves in
    // place, so the ID-shape / kind / marker prose matches `grund.toml`. Read
    // before the entrypoint plan because one selection rule depends on it: a
    // companion symlinked to `AGENTS.md` leaves its agent covered unless this
    // key makes the canonical file unable to carry that agent's form
    // (§FS-init.2.1.1, §FS-init.2.3.4.17).
    let init_config = init_pending_effective_config(&target, &resolved_name, description.as_deref())
        .map_err(|err| InitError::new(err.to_string()))?;
    let linked_conversation = init_config.conversation.as_deref() == Some("link");

    let agent_entrypoints =
        match selected_init_agent_entrypoints(&target, &agent_selection, linked_conversation) {
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

    // Render the managed block once and reuse it for both surfaces — the
    // workspace-members walk-up (§FS-init.2.3.4.15) is non-trivial I/O for a
    // large workspace and produces byte-identical output each time. The selected
    // entrypoint plan determines whether a missing self `AGENTS.md` should be
    // treated as about-to-exist; companion-only init must not link to a missing
    // canonical entrypoint.
    // Two surfaces at most: the local-conversation sentence differs between the
    // Claude entrypoints and everything else (§FS-init.2.3.4.17), and nothing
    // else in the block does. The linked variant is rendered only when a Claude
    // entrypoint is actually selected, so the common run still walks the
    // workspace once.
    let render_block = |surface| {
        render_agents_append_block(
            &resolved_name,
            &init_config,
            &target,
            agent_entrypoints.canonical,
            surface,
        )
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
    // consumes the plan — `init` creates one entrypoint per agent, but a
    // repository that already carries two keeps both, and this run is where that
    // shows; and the fix the symlink note names depends on whether this run is
    // the one giving Claude an entrypoint of its own.
    let claude_companions = agent_entrypoints.companions_of_claude(&target);
    let mut notes = duplicate_agent_entrypoint_notes(
        &target,
        &agent_entrypoints.companions,
        agent_entrypoints.canonical,
        linked_conversation,
        dry_run,
    );
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
                any_change |= event_is_change(&event);
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

    // `grund.toml` is the project's configuration — the surface a repo customizes
    // (kinds, marker, scan scope, …, §GOAL-configurable). `init` writes the
    // canonical template only when the target has **no** config under either
    // discovery name (§FS-config.1); an existing one is never overwritten, not even
    // with `--force`, and is reported under the name it was found at so a repo on
    // the `.agents/` form never grows the redundant pair (§FS-init.2.4,
    // §FS-check.4.3). `--force` targets the things `init` owns end to end — the
    // managed agent-instructions block and the `--docs` scaffold stubs — not the
    // user's settings (§FS-init.3).
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
    // §FS-init.2.3.4.17: the committed `link` opinion is rendered per entrypoint,
    // and a Claude entrypoint that is a symlink to `AGENTS.md` is the canonical
    // file — which every other agent reads too, so it must keep the plain form.
    // Silence here would read as the opinion simply not working.
    if linked_conversation
        && let Some(note) =
            shadowed_claude_entrypoint_note(&target, &claude_companions, dry_run)
    {
        notes.push(note);
    }
    Ok(InitOutput {
        events,
        notes,
        next,
    })
}

fn print_init_output(output: &InitOutput) {
    for event in &output.events {
        eprintln!("{} {}", event.verb, event.path);
    }
    for note in &output.notes {
        eprintln!("note: {note}");
    }
    if let Some(next) = &output.next {
        print_next_block_for_home(next.docs, Some(&next.entrypoint), &next.fs_home);
    }
}

fn event_is_change(event: &InitEvent) -> bool {
    event.verb != "exists"
}

fn print_next_block_for_home(docs: bool, entrypoint: Option<&str>, fs_home: &InitFsHome) {
    eprint!("{}", render_next_block_for_home(docs, entrypoint, fs_home));
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
            "  1. re-run with --docs to scaffold the FS home ({fs_home_path}), docs/, and e2e/ (or create them yourself) — until then `grund check` has nothing to scan\n"
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

struct SelectedInitAgentEntrypoints {
    canonical: bool,
    companions: Vec<InitCompanionAgentEntrypoint>,
}

impl SelectedInitAgentEntrypoints {
    /// The `<path>`-relative entrypoints in this plan that Claude reads
    /// (§FS-init.2.3.4.17) — what tells the shadowed-entrypoint note whether
    /// this run is the one giving Claude a file of its own, and which file.
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
    linked_conversation: bool,
) -> Result<SelectedInitAgentEntrypoints, (PathBuf, String)> {
    if selection.any() {
        let (canonical_from_symlink, companions) =
            requested_init_companion_agent_entrypoints(target, selection, linked_conversation)?;
        return Ok(SelectedInitAgentEntrypoints {
            canonical: selection.canonical || canonical_from_symlink,
            companions,
        });
    }

    let canonical = target.join(CANONICAL_AGENT_ENTRYPOINT);
    let canonical_exists = is_file_or_symlink(&canonical);
    let (canonical_from_companion_symlink, existing_companions) =
        existing_init_companion_agent_entrypoints(target)?;
    if canonical_exists || !existing_companions.is_empty() {
        return Ok(SelectedInitAgentEntrypoints {
            canonical: canonical_exists || canonical_from_companion_symlink,
            companions: existing_companions,
        });
    }

    let workspace_companions =
        workspace_init_companion_agent_entrypoints(target, linked_conversation)?;
    if canonical_from_companion_symlink || !workspace_companions.is_empty() {
        return Ok(SelectedInitAgentEntrypoints {
            canonical: canonical_from_companion_symlink,
            companions: workspace_companions,
        });
    }

    Ok(SelectedInitAgentEntrypoints {
        canonical: true,
        companions: Vec::new(),
    })
}


/// The `--docs` scaffold: the default requirements/spec home, canonical `docs/`
/// files (`grund.md`, `goals.md`, `roadmap.md`, `changelog.md`, architecture
/// README, decision `.gitkeep`s), plus an empty `e2e/` with a README — the file
/// list of §FS-init.2.1, each a minimal starter that leaves `grund check` clean.
fn init_fs_home(config: &Config) -> InitFsHome {
    if let Some(kind) = config.kinds.iter().find(|kind| kind.prefix == "FS") {
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
        (
            "docs/decisions/architectural/.gitkeep",
            canonical_template_text(GITKEEP_TEMPLATE),
        ),
        (
            "docs/decisions/functional/.gitkeep",
            canonical_template_text(GITKEEP_TEMPLATE),
        ),
        (
            "e2e/README.md",
            render_e2e_readme(fs_home),
        ),
        (
            "e2e/cases/.gitkeep",
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
