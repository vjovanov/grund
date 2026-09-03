/// The deprecated `main_entry()` adapter for `grund init` (§AR-bindings.2): the
/// argument parsing, the verdict, and the stderr rendering that only this path
/// uses. The shipped CLI carries its own copy in `grund-cli`; this is the half
/// of `init` that is a *command* rather than the library function `init()`
/// beside it, which is why it sits in its own file the way `checker_cmd.rs`,
/// `config_cmd.rs` and `fmt_cmd.rs` do.
///
/// `grund init [path] [--name N] [--docs] [--force] [--dry-run] [--check] [agent flags]` —
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
/// is written to disk; `--check` prints that same report and exits `1` when any
/// line of it is a `would-` (§FS-init.4).
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
            "--check" => check = true,
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
        check,
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
    // §FS-init.4: the verdict is drawn from the report that was just printed,
    // and only `--check` asks for one — `--dry-run` alone keeps its `0`.
    if check && output.has_pending_changes() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
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

fn print_next_block_for_home(docs: bool, entrypoint: Option<&str>, fs_home: &InitFsHome) {
    eprint!("{}", render_next_block_for_home(docs, entrypoint, fs_home));
}
