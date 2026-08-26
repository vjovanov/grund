const COMPAT_SUBCOMMANDS: &[&str] = &[
    "check",
    "show",
    "list",
    "refs",
    "cover",
    "fmt",
    "id",
    "init",
    "config",
    "agent-setup-instructions",
    "completions",
    "integrations",
];

fn compat_agent_setup_instructions(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("error: agent-setup-instructions takes no arguments");
        return ExitCode::from(2);
    }
    print!("{}", canonical_template_text(AGENT_SETUP_INSTRUCTIONS));
    ExitCode::SUCCESS
}

#[cfg(unix)]
fn compat_restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn compat_restore_default_sigpipe() {}

fn compat_print_help() {
    println!("grund — ground your agents in the spec.");
    println!("Usage:  grund [COMMAND] [ARGS]");
    println!();
    println!("Commands:");
    println!("  show     Print one declaration body for agent context (default).");
    println!("  check    Validate every reference in a repo.");
    println!("  list     Print the ID catalog.");
    println!("  refs     Print every citation of an ID.");
    println!("  cover    Group the citation graph by scanned file.");
    println!("  fmt      Normalize citation markers and cross-reference links.");
    println!("  id       Propose the next ID for a declaration.");
    println!("  init     Scaffold grund config, docs, and agent instructions.");
    println!("  config   Inspect or validate config.");
    println!("  completions  Print shell completions.");
}

fn compat_print_subcommand_help(cmd: &str) {
    println!("grund {cmd}");
    println!();
    println!(
        "This compatibility help page is emitted by deprecated grund_core::main_entry(); install and run the `grund` CLI package for the full help text."
    );
}

/// Backward-compatible CLI entry point retained for `grund-core = "0.4"`
/// consumers that built their own thin binary around the old core symbol.
///
/// New code should depend on the `grund` CLI package for process entry points,
/// or call the structured `grund-core` APIs (`check`, `show`, `scan`) when
/// embedding the engine (§AR-bindings.2, §FS-distribution.3.1).
#[deprecated(
    since = "0.4.1",
    note = "use the `grund` CLI package for process entry points, or `grund_core::{check, show, scan}` for embedding"
)]
pub fn main_entry() -> ExitCode {
    compat_restore_default_sigpipe();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("grund {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let first = args.first().map(|arg| arg.as_str());
    if first == Some("help") {
        return match args.get(1).map(String::as_str) {
            None => {
                compat_print_help();
                ExitCode::SUCCESS
            }
            Some(cmd) if COMPAT_SUBCOMMANDS.contains(&cmd) => {
                compat_print_subcommand_help(cmd);
                ExitCode::SUCCESS
            }
            Some(other) => {
                eprintln!("error: unknown command: {other}");
                eprintln!("known commands: {}", COMPAT_SUBCOMMANDS.join(", "));
                ExitCode::from(2)
            }
        };
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        match first {
            Some(cmd) if COMPAT_SUBCOMMANDS.contains(&cmd) => compat_print_subcommand_help(cmd),
            None | Some("--help" | "-h") => compat_print_help(),
            Some(other) if other.starts_with('-') => compat_print_help(),
            Some(other) => {
                eprintln!("error: unknown command: {other}");
                eprintln!("known commands: {}", COMPAT_SUBCOMMANDS.join(", "));
                return ExitCode::from(2);
            }
        }
        return ExitCode::SUCCESS;
    }
    match first {
        None => {
            eprintln!(
                "warning: bare `grund` still runs `grund check .`; use `grund check` explicitly."
            );
            command_check(&[])
        }
        Some("check") => command_check(&args[1..]),
        Some("show") => command_show(&args[1..]),
        Some("list") => command_list(&args[1..]),
        Some("refs") => command_refs(&args[1..]),
        Some("cover") => command_cover(&args[1..]),
        Some("fmt") => command_fmt(&args[1..]),
        Some("id") => command_id(&args[1..]),
        Some("init") => command_init(&args[1..]),
        Some("config") => command_config(&args[1..]),
        Some("agent-setup-instructions") => compat_agent_setup_instructions(&args[1..]),
        Some("completions") => command_completions(&args[1..]),
        Some("integrations") => run_integrations(&args[1..]),
        Some("complete") => command_complete(&args[1..]),
        Some(_) => command_show_default(&args),
    }
}
