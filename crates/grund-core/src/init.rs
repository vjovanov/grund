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
            output: InitOutput { events, next: None },
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
/// entrypoint(s) and `.agents/grund.toml` (and, with `--docs`, the `docs/`+`e2e/`
/// tree, §FS-init.2.1), preserve an existing repo's agent-entrypoint choice by
/// default (§FS-init.2.1), refuse to clobber edited scaffold files without
/// `--force` — and never overwrite an existing `.agents/grund.toml` even with
/// `--force`, since that file is the user's config (§FS-init.3) — print a `next:`
/// block (suppressed when every reported path is `exists `, §FS-init.2.2), and
/// exit `2` on a missing target / CLI error / unsupported block version
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
    let mut agent_selection = InitAgentEntrypointSelection::default();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--docs" => docs = true,
            "--force" => force = true,
            "--dry-run" => dry_run = true,
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
    if !target.exists() {
        return Err(InitError::new(format!(
            "target directory does not exist: {}",
            target.display()
        )));
    }
    if !target.is_dir() {
        return Err(InitError::new(format!(
            "target is not a directory: {}",
            target.display()
        )));
    }

    let resolved_name = match name {
        Some(value) => value,
        None => derive_default_name(&target).map_err(|err| InitError::new(err.to_string()))?,
    };

    let agent_entrypoints = match selected_init_agent_entrypoints(&target, &agent_selection) {
        Ok(entrypoints) => entrypoints,
        Err((path, message)) => {
            return Err(InitError::new(format!(
                "inspect {}: {message}",
                path.display()
            )));
        }
    };

    // §FS-init.2.3: render agent instructions against the config `init` leaves in
    // place, so the ID-shape / kind / marker prose matches `.agents/grund.toml`.
    let init_config =
        init_pending_effective_config(&target, &resolved_name, description.as_deref());

    // Render the managed block once and reuse it for both surfaces — the
    // workspace-members walk-up (§FS-init.2.3.4.15) is non-trivial I/O for a
    // large workspace and produces byte-identical output each time. The selected
    // entrypoint plan determines whether a missing self `AGENTS.md` should be
    // treated as about-to-exist; companion-only init must not link to a missing
    // canonical entrypoint.
    let agents_block = render_agents_append_block(
        &resolved_name,
        &init_config,
        &target,
        agent_entrypoints.canonical,
    );
    let agents_contents = render_agents_md_from_block(&resolved_name, &agents_block);
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
        let path_ref = match &entrypoint {
            InitCompanionAgentEntrypoint::Existing(path)
            | InitCompanionAgentEntrypoint::MissingAlias(path) => path.as_path(),
        };
        let rel = path_ref.strip_prefix(&target).unwrap_or(path_ref).to_path_buf();
        let rel = format_path(&rel);
        if workflow_entrypoint.is_none() {
            workflow_entrypoint = Some(rel.clone());
        }
        match entrypoint {
            InitCompanionAgentEntrypoint::Existing(path) => {
                match update_agents_block(&path, &agents_block, &rel, dry_run) {
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
                            format!("update {}: {err}", path.display()),
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
                    && let Err(err) = fs::write(&path, &agents_block)
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

    // `.agents/grund.toml` is the project's configuration — the surface a repo
    // customizes (kinds, marker, scan scope, …, §GOAL-configurable). `init` writes
    // the canonical template only when it is **absent**; an existing config is never
    // overwritten, not even with `--force`. `--force` targets the things `init`
    // owns end to end — the managed agent-instructions block and the `--docs` scaffold
    // stubs — not the user's settings (§FS-init.3).
    let config_rel = ".agents/grund.toml";
    let config_dest = target.join(config_rel);
    if config_dest.exists() {
        events.push(InitEvent { verb: "exists", path: config_rel.to_string() });
    } else {
        if !dry_run
            && let Some(parent) = config_dest.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            return Err(InitError::with_events(
                events,
                format!("create {}: {err}", parent.display()),
            ));
        }
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
    Ok(InitOutput { events, next })
}

fn print_init_output(output: &InitOutput) {
    for event in &output.events {
        eprintln!("{} {}", event.verb, event.path);
    }
    if let Some(next) = &output.next {
        print_next_block(next.docs, Some(&next.entrypoint));
    }
}

fn event_is_change(event: &InitEvent) -> bool {
    event.verb != "exists"
}

/// The trailing `next:` guidance block (§FS-init.2.2). Suppressed by the caller
/// when every reported path was `exists ` — when the repo is already current
/// there is no next step to teach. `entrypoint` is the first agent entrypoint
/// `init` touched, used in the final `see <entrypoint> …` pointer; `None`
/// falls back to the canonical `AGENTS.md`.
fn print_next_block(docs: bool, entrypoint: Option<&str>) {
    let config = Config::default_for(PathBuf::from("."));
    let fs_home = init_fs_home(&config);
    print_next_block_for_home(docs, entrypoint, &fs_home);
}

fn print_next_block_for_home(docs: bool, entrypoint: Option<&str>, fs_home: &InitFsHome) {
    eprintln!();
    eprintln!("next:");
    if docs {
        eprintln!("  1. run `grund check` — a freshly scaffolded tree is clean");
        match fs_home {
            InitFsHome::File { path, heading_name, heading_marker } => {
                eprintln!("  2. allocate an ID:  ID=$(grund id FS \"…\")  then add it to {path}");
                eprintln!(
                    "     ({heading_name}: `{heading_marker} <ID>: <one-line statement of the behavior>`)"
                );
            }
            InitFsHome::Folder { path } => {
                eprintln!("  2. allocate an ID:  ID=$(grund id FS \"…\")  then add it under {path}");
                eprintln!("     (H1: `# <ID>: <one-line statement of the behavior>`)");
            }
        }
        eprintln!(
            "  3. cite it as §<ID> from the docs and e2e tests that depend on it, then `grund check` again"
        );
    } else {
        let fs_home_path = match fs_home {
            InitFsHome::File { path, .. } | InitFsHome::Folder { path } => path,
        };
        eprintln!("  1. re-run with --docs to scaffold the FS home ({fs_home_path}), docs/, and e2e/ (or create them yourself) — until then `grund check` has nothing to scan");
        eprintln!("  2. run `grund check` — a scaffolded tree is clean");
        match fs_home {
            InitFsHome::File { path, .. } => {
                eprintln!("  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it to {path}");
            }
            InitFsHome::Folder { path } => {
                eprintln!("  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it under {path}");
            }
        }
    }
    eprintln!(
        "see {} for the full workflow.",
        entrypoint.unwrap_or(CANONICAL_AGENT_ENTRYPOINT)
    );
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

fn selected_init_agent_entrypoints(
    target: &Path,
    selection: &InitAgentEntrypointSelection,
) -> Result<SelectedInitAgentEntrypoints, (PathBuf, String)> {
    if selection.any() {
        let (canonical_from_symlink, companions) =
            requested_init_companion_agent_entrypoints(target, selection)?;
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

    let workspace_companions = workspace_init_companion_agent_entrypoints(target);
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

/// What `init` did to an existing `AGENTS.md`'s managed block — `appended ` (no
/// block before), `updated ` (a supported block whose bytes changed: an older
/// block upgraded, or a same-version block re-rendered against a changed
/// template or config), or `unchanged` (a supported block already byte-identical
/// to the current render — `init` rewrites nothing, §FS-init.2.2/§FS-init.2.3,
/// and reports it with the `exists ` prefix like any other untouched file).
#[derive(Debug, Eq, PartialEq)]
enum AgentsUpdateResult {
    Appended,
    Updated,
    Unchanged,
}

/// Returns the file event for the canonical entrypoint, or an already formatted
/// I/O message for the caller to surface.
fn write_or_update_canonical_agent_entrypoint(
    target: &Path,
    rel: &str,
    contents: &str,
    block: &str,
    force: bool,
    dry_run: bool,
) -> Result<InitEvent, String> {
    let dest = target.join(rel);
    if !force && dest.exists() {
        match update_agents_block(&dest, block, rel, dry_run) {
            Ok(AgentsUpdateResult::Appended) => Ok(InitEvent {
                verb: verb_appended(dry_run),
                path: rel.to_string(),
            }),
            Ok(AgentsUpdateResult::Updated) => Ok(InitEvent {
                verb: verb_updated(dry_run),
                path: rel.to_string(),
            }),
            Ok(AgentsUpdateResult::Unchanged) => Ok(InitEvent {
                verb: "exists",
                path: rel.to_string(),
            }),
            Err(err) => Err(format!("update {}: {err}", dest.display())),
        }
    } else {
        if !dry_run
            && let Some(parent) = dest.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            return Err(format!("create {}: {err}", parent.display()));
        }
        if !dry_run
            && let Err(err) = fs::write(&dest, contents)
        {
            return Err(format!("write {}: {err}", dest.display()));
        }
        Ok(InitEvent {
            verb: verb_wrote(dry_run),
            path: rel.to_string(),
        })
    }
}

/// Append or update the managed block in an existing agent entrypoint on disk
/// (§FS-init.2.3). A supported block is re-rendered from the current
/// template/config even when the schema version already matches — but when that
/// re-render is byte-identical to what is on disk the file is left untouched
/// (`Unchanged`, reported as `exists `), so re-running `grund init` on an
/// up-to-date repo writes nothing (§FS-init.2.2). Under `--dry-run`, the
/// computed result is returned without writing.
fn update_agents_block(
    dest: &Path,
    block: &str,
    label: &str,
    dry_run: bool,
) -> Result<AgentsUpdateResult> {
    let existing = fs::read_to_string(dest)?;
    let (updated, result) = update_agents_text(&existing, block, label)?;
    if !dry_run && result != AgentsUpdateResult::Unchanged {
        fs::write(dest, updated)?;
    }
    Ok(result)
}

/// The pure string transform behind `update_agents_block`: splice the current
/// managed block into `existing`, preserving everything outside it byte-for-byte
/// — including the block's position and any CRLF endings (§FS-init.2.3.1,
/// §FS-init.2.3.2). Returns `Unchanged` when the splice would reproduce
/// `existing` exactly. A newer-than-supported block is an error.
fn update_agents_text(
    existing: &str,
    block: &str,
    label: &str,
) -> Result<(String, AgentsUpdateResult)> {
    if let Some(existing_block) = find_agents_block(existing) {
        if existing_block.version > AGENTS_BLOCK_VERSION {
            return Err(anyhow!(
                "{label} contains newer grund init block v{}; this binary supports v{}",
                existing_block.version,
                AGENTS_BLOCK_VERSION
            ));
        }
        let mut updated = String::with_capacity(existing.len() + block.len());
        updated.push_str(&existing[..existing_block.start]);
        updated.push_str(block);
        updated.push_str(&existing[existing_block.end..]);
        let result = if updated == existing {
            AgentsUpdateResult::Unchanged
        } else {
            AgentsUpdateResult::Updated
        };
        return Ok((updated, result));
    }

    let separator = if existing.is_empty() || existing.ends_with("\n\n") {
        ""
    } else if existing.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let mut updated = String::with_capacity(existing.len() + separator.len() + block.len());
    updated.push_str(existing);
    updated.push_str(separator);
    updated.push_str(block);
    Ok((updated, AgentsUpdateResult::Appended))
}

/// The byte span and `vN` version of the managed block inside an `AGENTS.md`
/// (§FS-init.2.3) — what both `grund init`'s update and `grund check`'s validation
/// (§FS-check.3.5) key off.
struct AgentsBlock {
    start: usize,
    end: usize,
    version: u32,
}

/// Locate the managed block in `AGENTS.md`. The current marker is an H2 line
/// (`## Grounding with grund (vN)`); the block runs until the next H1 or H2 (or
/// EOF) (§FS-init.2.3).
fn find_agents_block(text: &str) -> Option<AgentsBlock> {
    if let Some(caps) = AGENTS_BLOCK_H2.captures(text) {
        let begin_match = caps.get(0)?;
        let version = caps.name("version")?.as_str().parse::<u32>().ok()?;
        let after = begin_match.end();
        let section_end = AGENTS_SECTION_BOUNDARY
            .find_at(text, after)
            .map(|m| m.start())
            .unwrap_or(text.len());
        // Trailing blank lines before the next section are inter-section spacing,
        // not part of the managed body. Trim them back so a re-render of the same
        // content is a no-op (`exists `, §FS-init.2.3.1).
        let mut end = section_end;
        while end > after && text[..end].ends_with("\n\n") {
            end -= 1;
        }
        return Some(AgentsBlock {
            start: begin_match.start(),
            end,
            version,
        });
    }
    None
}

/// The default project name when `--name` is omitted: the basename of `<path>`
/// resolved to an absolute path (§FS-init.1).
fn derive_default_name(target: &Path) -> Result<String> {
    let absolute =
        fs::canonicalize(target).with_context(|| format!("resolve {}", target.display()))?;
    absolute
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| anyhow!("cannot derive project name from {}", absolute.display()))
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
            canonical_template_text(E2E_README_TEMPLATE),
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
