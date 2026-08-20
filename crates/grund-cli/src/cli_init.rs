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
            "--pi" => agent_selection.pi = true,
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
            render_init_output(&err.output);
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    render_init_output(&output);
    ExitCode::SUCCESS
}

fn render_init_output(output: &InitOutput) {
    for event in &output.events {
        eprintln!("{} {}", event.verb, event.path);
    }
    // §FS-init.2.3.4.17: reported, never fatal — a note names something the run
    // could not do that the caller would otherwise have to notice for itself.
    for note in &output.notes {
        eprintln!("note: {note}");
    }
    if let Some(next) = &output.next {
        render_init_next(next);
    }
}

fn render_init_next(next: &InitNext) {
    eprintln!();
    eprintln!("next:");
    if next.docs {
        eprintln!("  1. run `grund check` — a freshly scaffolded tree is clean");
        match &next.fs_home {
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
        let fs_home_path = match &next.fs_home {
            InitFsHome::File { path, .. } | InitFsHome::Folder { path } => path,
        };
        eprintln!("  1. re-run with --docs to scaffold the FS home ({fs_home_path}), docs/, and e2e/ (or create them yourself) — until then `grund check` has nothing to scan");
        eprintln!("  2. run `grund check` — a scaffolded tree is clean");
        match &next.fs_home {
            InitFsHome::File { path, .. } => {
                eprintln!("  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it to {path}");
            }
            InitFsHome::Folder { path } => {
                eprintln!("  3. allocate an ID:  ID=$(grund id FS \"…\")  then add it under {path}");
            }
        }
    }
    eprintln!("see {} for the full workflow.", next.entrypoint);
}
