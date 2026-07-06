// `grund integrations` — print and install the rendering-layer integrations that
// make a §<ID> citation clickable in a terminal or editor (§FS-integrations).
// Every artifact is embedded in the binary, printed on demand, and installed
// only under `--write` as a managed marked block — the `completions`/`init`
// ethos (§DF-integrations-command). One implementation lives here and is called
// from both the live CLI and the deprecated compat frontend.

const GRUND_OPEN_RESOLVER: &str = include_str!("../assets/integrations/grund-open");
const WEZTERM_SNIPPET: &str = include_str!("../assets/integrations/wezterm.lua");
const KITTY_SNIPPET: &str = include_str!("../assets/integrations/kitty.conf");
const TMUX_SNIPPET: &str = include_str!("../assets/integrations/tmux.conf");
const VSCODE_PACKAGE_JSON: &str = include_str!("../assets/integrations/vscode/package.json");
const VSCODE_EXTENSION_JS: &str = include_str!("../assets/integrations/vscode/extension.js");

/// The version stamped into the managed dotfile block markers (§FS-integrations.4.1).
/// Bumped when an embedded snippet changes in a way a re-run should propagate.
const INTEGRATIONS_BLOCK_VERSION: u32 = 1;

/// The rendering-layer clients grund ships an integration for. The set is closed
/// and frozen (§FS-integrations.1); the ordering here is the frozen output order
/// used by detection and every listing.
#[derive(Clone, Copy, Eq, PartialEq)]
enum IntegrationClient {
    Kitty,
    Tmux,
    Vscode,
    Wezterm,
}

impl IntegrationClient {
    /// Frozen order: `kitty, tmux, vscode, wezterm`.
    const ALL: [IntegrationClient; 4] = [
        IntegrationClient::Kitty,
        IntegrationClient::Tmux,
        IntegrationClient::Vscode,
        IntegrationClient::Wezterm,
    ];

    fn name(self) -> &'static str {
        match self {
            IntegrationClient::Kitty => "kitty",
            IntegrationClient::Tmux => "tmux",
            IntegrationClient::Vscode => "vscode",
            IntegrationClient::Wezterm => "wezterm",
        }
    }

    fn from_name(name: &str) -> Option<IntegrationClient> {
        IntegrationClient::ALL
            .into_iter()
            .find(|client| client.name() == name)
    }

    fn is_terminal(self) -> bool {
        !matches!(self, IntegrationClient::Vscode)
    }

    /// The terminal config snippet for a terminal client; `None` for vscode.
    fn snippet(self) -> Option<&'static str> {
        match self {
            IntegrationClient::Kitty => Some(KITTY_SNIPPET),
            IntegrationClient::Tmux => Some(TMUX_SNIPPET),
            IntegrationClient::Wezterm => Some(WEZTERM_SNIPPET),
            IntegrationClient::Vscode => None,
        }
    }

    /// A `~`-rooted hint at where `--write` installs the config, printed verbatim
    /// so it stays byte-stable across machines (§FS-integrations.6).
    fn config_target(self) -> &'static str {
        match self {
            IntegrationClient::Kitty => "~/.config/kitty/kitty.conf",
            IntegrationClient::Tmux => "~/.tmux.conf",
            IntegrationClient::Wezterm => "~/.config/wezterm/wezterm.lua",
            IntegrationClient::Vscode => "~/.vscode/extensions/grund.grund-terminal-citations",
        }
    }

    fn install_command(self) -> String {
        format!("grund integrations {} --write", self.name())
    }
}

fn known_clients_line() -> String {
    format!(
        "known clients: {}",
        IntegrationClient::ALL
            .iter()
            .map(|client| client.name())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Detect which clients apply from the ambient environment (§FS-integrations.2).
/// Reads only the named variables; results are returned in the frozen client
/// order, deduplicated, so a given environment always yields the same list.
fn detect_clients() -> Vec<IntegrationClient> {
    let has = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    let mut matched = [false; 4];
    let mut mark = |client: IntegrationClient| {
        let idx = IntegrationClient::ALL
            .iter()
            .position(|candidate| *candidate == client)
            .expect("client is in ALL");
        matched[idx] = true;
    };
    if has("WEZTERM_EXECUTABLE") {
        mark(IntegrationClient::Wezterm);
    }
    if has("KITTY_WINDOW_ID") {
        mark(IntegrationClient::Kitty);
    }
    match std::env::var("TERM_PROGRAM").ok().as_deref() {
        Some("WezTerm") => mark(IntegrationClient::Wezterm),
        Some("tmux") => mark(IntegrationClient::Tmux),
        Some("vscode") => mark(IntegrationClient::Vscode),
        _ => {}
    }
    if std::env::vars_os().any(|(key, _)| {
        key.to_str()
            .is_some_and(|key| key == "VSCODE_PID" || key.starts_with("VSCODE_"))
    }) {
        mark(IntegrationClient::Vscode);
    }
    if has("TMUX") {
        mark(IntegrationClient::Tmux);
    }
    IntegrationClient::ALL
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| matched[*idx])
        .map(|(_, client)| client)
        .collect()
}

/// Parsed `grund integrations` invocation.
struct IntegrationsInvocation {
    client: Option<IntegrationClient>,
    write: bool,
    json: bool,
}

/// Parse args, or return an error `ExitCode` after printing a CLI-level message.
fn parse_integrations_args(args: &[String]) -> Result<IntegrationsInvocation, ExitCode> {
    let mut client = None;
    let mut write = false;
    let mut format: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--write" => write = true,
            "--format" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --format requires a value");
                    return Err(ExitCode::from(2));
                }
                format = Some(args[idx].clone());
            }
            other if other.starts_with("--format=") => {
                format = Some(other.trim_start_matches("--format=").to_string());
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return Err(ExitCode::from(2));
            }
            other => {
                if client.is_some() {
                    eprintln!("error: integrations takes at most one client argument");
                    return Err(ExitCode::from(2));
                }
                match IntegrationClient::from_name(other) {
                    Some(parsed) => client = Some(parsed),
                    None => {
                        eprintln!("error: unknown integration client `{other}`");
                        eprintln!("{}", known_clients_line());
                        return Err(ExitCode::from(2));
                    }
                }
            }
        }
        idx += 1;
    }
    let json = match format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => {
            eprintln!("error: unsupported integrations format `{other}`");
            return Err(ExitCode::from(2));
        }
    };
    if write && client.is_none() {
        eprintln!("error: integrations --write requires a client");
        eprintln!("{}", known_clients_line());
        return Err(ExitCode::from(2));
    }
    Ok(IntegrationsInvocation {
        client,
        write,
        json,
    })
}

/// The `grund integrations` entry point, called from both CLI frontends
/// (§FS-integrations). Prints by default; writes only under `--write`.
pub fn run_integrations(args: &[String]) -> ExitCode {
    let invocation = match parse_integrations_args(args) {
        Ok(invocation) => invocation,
        Err(code) => return code,
    };
    match invocation.client {
        None => print_detection(invocation.json),
        Some(client) if invocation.write => write_integration(client),
        Some(client) if invocation.json => {
            print!("{}", client_descriptor_json(client));
            ExitCode::SUCCESS
        }
        Some(client) => {
            print_client_artifact(client);
            ExitCode::SUCCESS
        }
    }
}

/// No-client detection print (§FS-integrations.2). Environment-dependent, so it
/// is never goldened; exit is always `0`.
fn print_detection(json: bool) -> ExitCode {
    let detected = detect_clients();
    if json {
        print!("{}", detection_plan_json(&detected));
        return ExitCode::SUCCESS;
    }
    if detected.is_empty() {
        println!("No supported terminal or editor detected. Available integrations:");
        for client in IntegrationClient::ALL {
            println!("  {:<8} {}", client.name(), client.install_command());
        }
        println!();
        println!("Run `grund integrations <client>` to preview one before installing.");
    } else {
        println!("Detected integrations for this environment:");
        for client in detected {
            println!("  {:<8} {}", client.name(), client.install_command());
        }
        println!();
        println!("Run `grund integrations <client>` to preview one before installing.");
    }
    ExitCode::SUCCESS
}

/// The machine-shaped detection plan (§FS-integrations.5): detected clients in
/// frozen order, then every client with whether it was detected and its install.
fn detection_plan_json(detected: &[IntegrationClient]) -> String {
    let detected_names = detected
        .iter()
        .map(|client| format!("\"{}\"", client.name()))
        .collect::<Vec<_>>()
        .join(",");
    let clients = IntegrationClient::ALL
        .iter()
        .map(|client| {
            format!(
                "{{\"client\":\"{}\",\"detected\":{},\"install\":\"{}\"}}",
                client.name(),
                detected.contains(client),
                json_escape_str(&client.install_command()),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"detected\":[{detected_names}],\"clients\":[{clients}]}}\n")
}

/// One JSON object describing a client's artifact and its `--write` targets,
/// without printing the artifact bytes (§FS-integrations.5).
fn client_descriptor_json(client: IntegrationClient) -> String {
    let kind = if client.is_terminal() {
        "terminal"
    } else {
        "editor"
    };
    let resolver = if client.is_terminal() {
        format!(",\"resolver_target\":\"{}\"", "~/.local/bin/grund-open")
    } else {
        String::new()
    };
    format!(
        "{{\"client\":\"{}\",\"kind\":\"{}\",\"install\":\"{}\",\"config_target\":\"{}\"{}}}\n",
        client.name(),
        kind,
        json_escape_str(&client.install_command()),
        json_escape_str(client.config_target()),
        resolver,
    )
}

/// Print a client's artifact for a human to read before installing
/// (§FS-integrations.3). Deterministic and environment-independent.
fn print_client_artifact(client: IntegrationClient) {
    match client.snippet() {
        Some(snippet) => {
            println!("# grund {} citation integration", client.name());
            println!("# Install with: {}", client.install_command());
            println!("#");
            println!("# 1. Terminal config — add to {}:", client.config_target());
            println!();
            print!("{snippet}");
            println!();
            println!("# 2. Resolver — install grund-open to a directory on PATH (e.g. ~/.local/bin):");
            println!();
            print!("{GRUND_OPEN_RESOLVER}");
        }
        None => {
            // vscode: print the unpacked extension source.
            println!("# grund {} terminal-citations extension", client.name());
            println!("# Install with: {}", client.install_command());
            println!("#");
            println!("# package.json:");
            println!();
            print!("{VSCODE_PACKAGE_JSON}");
            println!();
            println!("# extension.js:");
            println!();
            print!("{VSCODE_EXTENSION_JS}");
        }
    }
}

/// Result of splicing a managed block into a dotfile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BlockOutcome {
    Appended,
    Updated,
    Unchanged,
}

fn integrations_block_markers(version: u32) -> (String, String) {
    (
        format!("# >>> grund integrations (v{version}) >>>"),
        format!("# <<< grund integrations (v{version}) <<<"),
    )
}

/// Splice the managed integrations block carrying `snippet` into `existing`
/// (§FS-integrations.4.1): replace the bytes between the current-version markers
/// when a block is present, else append after a blank-line separator. Everything
/// outside the block is preserved. Returns the new text and what changed. A block
/// whose version is newer than this binary understands is an error.
fn install_managed_block(existing: &str, snippet: &str) -> Result<(String, BlockOutcome), String> {
    if let Some(version) = newer_integrations_block_version(existing) {
        return Err(format!(
            "config contains newer grund integrations block v{version}; this binary supports v{INTEGRATIONS_BLOCK_VERSION}"
        ));
    }
    let (begin, end) = integrations_block_markers(INTEGRATIONS_BLOCK_VERSION);
    let block = format!("{begin}\n{}\n{end}\n", snippet.trim_end_matches('\n'));
    if let Some((start, stop)) = find_block_span(existing, &begin, &end) {
        let mut updated = String::with_capacity(existing.len());
        updated.push_str(&existing[..start]);
        updated.push_str(&block);
        updated.push_str(&existing[stop..]);
        let outcome = if updated == existing {
            BlockOutcome::Unchanged
        } else {
            BlockOutcome::Updated
        };
        Ok((updated, outcome))
    } else {
        let mut appended = String::with_capacity(existing.len() + block.len() + 2);
        appended.push_str(existing);
        if !existing.is_empty() && !existing.ends_with('\n') {
            appended.push('\n');
        }
        if !existing.is_empty() {
            appended.push('\n');
        }
        appended.push_str(&block);
        Ok((appended, BlockOutcome::Appended))
    }
}

/// Byte span `[start, stop)` of an existing managed block, marker lines included,
/// with the trailing newline after the end marker consumed when present.
fn find_block_span(text: &str, begin: &str, end: &str) -> Option<(usize, usize)> {
    let start = line_start_of(text, begin)?;
    let end_line_start = line_start_of(&text[start..], end).map(|offset| start + offset)?;
    let after_end = end_line_start + end.len();
    // Consume the newline that terminates the end-marker line, if any.
    let stop = match text[after_end..].find('\n') {
        Some(0) => after_end + 1,
        _ => after_end,
    };
    Some((start, stop))
}

/// Offset of the start of the line that equals `marker` (trimmed), or `None`.
fn line_start_of(text: &str, marker: &str) -> Option<usize> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']).trim() == marker {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// The version of a managed block newer than this binary, if any is present.
fn newer_integrations_block_version(text: &str) -> Option<u32> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# >>> grund integrations (v")
            && let Some(digits) = rest.strip_suffix(") >>>")
            && let Ok(version) = digits.parse::<u32>()
            && version > INTEGRATIONS_BLOCK_VERSION
        {
            return Some(version);
        }
    }
    None
}

/// Apply an integration to disk under `--write` (§FS-integrations.4). Reports on
/// stderr; exit `0` on success, `2` on a newer-block or IO error.
fn write_integration(client: IntegrationClient) -> ExitCode {
    match client.snippet() {
        Some(snippet) => write_terminal_integration(client, snippet),
        None => write_vscode_integration(),
    }
}

fn write_terminal_integration(client: IntegrationClient, snippet: &str) -> ExitCode {
    let Some(config_path) = expand_target(client.config_target()) else {
        eprintln!("error: cannot resolve home directory for {}", client.name());
        return ExitCode::from(2);
    };
    let existing = match fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            eprintln!("error: {}: {err}", config_path.display());
            return ExitCode::from(2);
        }
    };
    let (updated, outcome) = match install_managed_block(&existing, snippet) {
        Ok(result) => result,
        Err(message) => {
            eprintln!("error: {}: {message}", config_path.display());
            return ExitCode::from(2);
        }
    };
    if outcome != BlockOutcome::Unchanged {
        if let Some(parent) = config_path.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            eprintln!("error: {}: {err}", parent.display());
            return ExitCode::from(2);
        }
        if let Err(err) = fs::write(&config_path, &updated) {
            eprintln!("error: {}: {err}", config_path.display());
            return ExitCode::from(2);
        }
    }
    eprintln!(
        "{} {}",
        block_outcome_verb(outcome),
        config_path.display()
    );
    match write_resolver_script() {
        Ok(Some(path)) => eprintln!("wrote {}", path.display()),
        Ok(None) => {}
        Err((path, message)) => {
            eprintln!("error: {}: {message}", path.display());
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn block_outcome_verb(outcome: BlockOutcome) -> &'static str {
    match outcome {
        BlockOutcome::Appended => "appended",
        BlockOutcome::Updated => "updated",
        BlockOutcome::Unchanged => "exists",
    }
}

/// Install `grund-open` to `~/.local/bin` when absent or out of date. Returns the
/// path when written, `None` when already current.
fn write_resolver_script() -> Result<Option<PathBuf>, (PathBuf, String)> {
    let Some(path) = expand_target("~/.local/bin/grund-open") else {
        return Err((PathBuf::from("~/.local/bin/grund-open"), "cannot resolve home directory".to_string()));
    };
    if fs::read_to_string(&path).is_ok_and(|current| current == GRUND_OPEN_RESOLVER) {
        return Ok(None);
    }
    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        return Err((parent.to_path_buf(), err.to_string()));
    }
    fs::write(&path, GRUND_OPEN_RESOLVER).map_err(|err| (path.clone(), err.to_string()))?;
    set_executable(&path);
    Ok(Some(path))
}

#[cfg(unix)]
fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs::metadata(path) {
        let mut perms = metadata.permissions();
        perms.set_mode(perms.mode() | 0o755);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) {}

/// Materialize the unpacked VS Code extension into the extensions directory
/// (§FS-integrations.4.2). Overwrites only the grund-owned files.
fn write_vscode_integration() -> ExitCode {
    let Some(dir) = expand_target(IntegrationClient::Vscode.config_target()) else {
        eprintln!("error: cannot resolve home directory for vscode");
        return ExitCode::from(2);
    };
    let marker = dir.join(".grund-version");
    let current = fs::read_to_string(&marker).ok();
    if current.as_deref() == Some(&INTEGRATIONS_BLOCK_VERSION.to_string()) {
        eprintln!("exists {}", dir.display());
        return ExitCode::SUCCESS;
    }
    if let Err(err) = fs::create_dir_all(&dir) {
        eprintln!("error: {}: {err}", dir.display());
        return ExitCode::from(2);
    }
    let files = [
        ("package.json", VSCODE_PACKAGE_JSON),
        ("extension.js", VSCODE_EXTENSION_JS),
        (".grund-version", &*format!("{INTEGRATIONS_BLOCK_VERSION}")),
    ];
    for (name, contents) in files {
        let path = dir.join(name);
        if let Err(err) = fs::write(&path, contents) {
            eprintln!("error: {}: {err}", path.display());
            return ExitCode::from(2);
        }
    }
    let verb = if current.is_some() { "updated" } else { "wrote" };
    eprintln!("{verb} {}", dir.display());
    ExitCode::SUCCESS
}

/// Expand a leading `~` in an install-target hint against `$HOME`.
fn expand_target(target: &str) -> Option<PathBuf> {
    if let Some(rest) = target.strip_prefix("~/") {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join(rest))
    } else {
        Some(PathBuf::from(target))
    }
}

/// Minimal JSON string escaper for the integrations plan (no serde in core).
fn json_escape_str(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if other.is_control() => out.push_str(&format!("\\u{:04x}", other as u32)),
            other => out.push(other),
        }
    }
    out
}
