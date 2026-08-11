fn command_agent_setup_instructions(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("error: agent-setup-instructions takes no arguments");
        return ExitCode::from(2);
    }
    print!("{}", canonical_template_text(AGENT_SETUP_INSTRUCTIONS));
    ExitCode::SUCCESS
}

fn command_output_format(
    command: &str,
    configured: &str,
    override_format: Option<String>,
) -> Result<String, ExitCode> {
    let format = override_format.unwrap_or_else(|| configured.to_string());
    if matches!(format.as_str(), "text" | "json") {
        Ok(format)
    } else {
        eprintln!("error: unsupported {command} format `{format}`");
        Err(ExitCode::from(2))
    }
}

fn exit_after_scan_errors(scan_errors: &[ApiScanError]) -> ExitCode {
    if scan_errors.is_empty() {
        return ExitCode::SUCCESS;
    }
    for err in scan_errors {
        eprintln!("error: {}: {}", err.path, err.message);
    }
    ExitCode::from(2)
}

fn json_escape(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => escaped.push_str(&format!("\\u{:04x}", other as u32)),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Restore the default `SIGPIPE` disposition (Unix only).
///
/// Rust ignores `SIGPIPE` at startup, which turns a closed downstream pipe
/// (`grund list | head`) into an `EPIPE` on the next write — and `println!`
/// panics on a write error. A CLI in a pipeline should instead die quietly,
/// the way `ls | head` does. This is a no-op off Unix.
#[cfg(unix)]
fn restore_default_sigpipe() {
    // SIGPIPE == 13 and SIG_DFL == (void(*)(int))0 on Linux, macOS, and the BSDs.
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
fn restore_default_sigpipe() {}

/// The CLI entry point: parse `argv`, dispatch to the matching `command_*`, and
/// return its `ExitCode` (§FS-cli). `grund <ID>` is the default ID query
/// (§FS-cli.1); `grund` with no arguments keeps the historical `check .`
/// behavior with a deprecation warning; `--version`/`--help` short-circuits to
/// stdout, exit 0 (§FS-cli.2); help on an unknown command exits 2 and lists the
/// known ones (§FS-cli.4). The exit-code mapping (0/1/2) is fixed (§FS-cli.5).
pub fn main_entry() -> ExitCode {
    restore_default_sigpipe();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("grund {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let first = args.first().map(|arg| arg.as_str());
    // `grund help [<subcommand>]` — the top-level page with no argument, that
    // subcommand's page with one, an error for an unknown name (§FS-cli.2).
    if first == Some("help") {
        return match args.get(1).map(String::as_str) {
            None => {
                print_help();
                ExitCode::SUCCESS
            }
            Some(cmd) if SUBCOMMANDS.contains(&cmd) => {
                print_subcommand_help(cmd);
                ExitCode::SUCCESS
            }
            Some(other) => {
                eprintln!("error: unknown command: {other}");
                eprintln!("known commands: {}", SUBCOMMANDS.join(", "));
                ExitCode::from(2)
            }
        };
    }
    // `--help` / `-h` short-circuits before any work; with a known subcommand
    // first it prints that subcommand's page, with no command it prints the
    // top-level one, and with an unknown first word it remains an unknown-command
    // error rather than hiding a typo behind generic help (§FS-cli.2, §FS-cli.4).
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        match first {
            Some(cmd) if SUBCOMMANDS.contains(&cmd) => print_subcommand_help(cmd),
            None | Some("--help" | "-h") => print_help(),
            Some(other) if other.starts_with('-') => print_help(),
            Some(other) => {
                eprintln!("error: unknown command: {other}");
                eprintln!("known commands: {}", SUBCOMMANDS.join(", "));
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
        Some("agent-setup-instructions") => command_agent_setup_instructions(&args[1..]),
        Some("completions") => command_completions(&args[1..]),
        Some("integrations") => run_integrations(&args[1..]),
        Some("complete") => command_complete(&args[1..]),
        // Any first argument that is not a known subcommand is an ID query
        // (§FS-cli.1). Check is explicit as `grund check [path]`.
        Some(_) => command_show_default(&args),
    }
}
