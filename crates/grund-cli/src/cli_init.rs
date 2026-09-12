fn command_init(args: &[String]) -> ExitCode {
    let mut path: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut docs = false;
    let mut force = false;
    let mut dry_run = false;
    let mut check = false;
    let mut no_vcs = false;
    let mut agent_selection = InitAgentEntrypointSelection::default();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--docs" => docs = true,
            "--force" => force = true,
            "--dry-run" => dry_run = true,
            // §FS-init.1: the same preview, taken as a verdict.
            "--check" => check = true,
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
        check,
        no_vcs,
        agent_selection,
    }) {
        Ok(output) => output,
        // §FS-init.4: `2` wins over `1` — a run that could not be performed
        // produced no report to gate on.
        Err(err) => {
            render_init_output(&err.output);
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    render_init_output(&output);
    // §FS-init.4: `--check` draws its verdict from the report it just printed —
    // `1` when any reported path was a `would-…`, nothing else. `--dry-run`
    // alone keeps `0` (§REQ-backwards-compatibility.1).
    if check && output.has_pending_changes() {
        return ExitCode::from(1);
    }
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
    // §FS-init.2.2: both command adapters print the core-rendered decision, so
    // the shipped CLI cannot drift from the deprecated compatibility path.
    eprint!("{}", next.render());
}
