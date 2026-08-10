// `grund integrations` — print and install the rendering-layer integrations that
// make a §<ID> citation clickable in a terminal or editor (§FS-integrations).
// Every artifact is embedded in the binary, printed on demand, and installed
// only under `--write` as a managed marked block — the `completions`/`init`
// ethos (§DF-integrations-command). One implementation lives here and is called
// from both the live CLI and the deprecated compat frontend.

const GRUND_OPEN_RESOLVER: &str = include_str!("../assets/integrations/grund-open");
const ITERM2_SNIPPET: &str = include_str!("../assets/integrations/iterm2.txt");
const WEZTERM_SNIPPET: &str = include_str!("../assets/integrations/wezterm.lua");
const KITTY_SNIPPET: &str = include_str!("../assets/integrations/kitty.conf");
const TMUX_SNIPPET: &str = include_str!("../assets/integrations/tmux.conf");
const VSCODE_PACKAGE_JSON: &str = include_str!("../assets/integrations/vscode/package.json");
const VSCODE_EXTENSION_JS: &str = include_str!("../assets/integrations/vscode/extension.js");

/// The version stamped into the managed dotfile block markers (§FS-integrations.4.1).
/// Bumped when an embedded snippet changes in a way a re-run should propagate.
const INTEGRATIONS_BLOCK_VERSION: u32 = 1;

/// Version for the user-level agent-instruction block (§FS-integrations.4.3).
/// v2 (§DF-repo-conversation-opinion): self-scoping texts — gated on the presence
/// of a `.agents/grund.toml`, with the repo-opinion precedence sentence in `plain`.
const AGENT_GUIDANCE_BLOCK_VERSION: u32 = 2;

/// The file-backed global instruction surfaces for every agent grund supports
/// end-to-end (§FS-integrations.4.3). Keep this superset aligned with the
/// repository entrypoints in §FS-init.2.1.
const GLOBAL_AGENT_INSTRUCTION_TARGETS: [&str; 6] = [
    "~/.codex/AGENTS.md",
    "~/.claude/CLAUDE.md",
    "~/.gemini/GEMINI.md",
    "~/.copilot/copilot-instructions.md",
    "~/.config/zed/AGENTS.md",
    "~/.pi/agent/AGENTS.md",
];

/// Where `--write` installs the `grund-open` resolver for terminal clients; a
/// single source so the descriptor plan and the writer cannot drift.
const RESOLVER_TARGET: &str = "~/.local/bin/grund-open";

/// The rendering-layer clients grund ships an integration for. The set is closed
/// and frozen (§FS-integrations.1); the ordering here is the frozen output order
/// used by detection and every listing.
#[derive(Clone, Copy, Eq, PartialEq)]
enum IntegrationClient {
    Codium,
    Iterm2,
    Kitty,
    Tmux,
    Vscode,
    Wezterm,
}

/// How `--write` applies a client's integration (§FS-integrations.4).
#[derive(Clone, Copy, Eq, PartialEq)]
enum InstallKind {
    /// A marked block spliced into the client's text config.
    Block,
    /// An unpacked extension directory.
    Vscode,
    /// Nothing writable: the client stores configuration somewhere no managed
    /// block can live, so `--write` installs the resolver and prints the steps
    /// the user has to apply by hand (§FS-integrations.3.4).
    Manual,
}

impl InstallKind {
    /// Reported in the detection plan so a caller can tell why a `manual`
    /// client never reports `installed` (§FS-integrations.5).
    fn name(self) -> &'static str {
        match self {
            InstallKind::Block => "block",
            InstallKind::Vscode => "extension",
            InstallKind::Manual => "manual",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConversationRendering {
    Plain,
    Link,
}

impl ConversationRendering {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "plain" => Some(Self::Plain),
            "link" => Some(Self::Link),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Link => "link",
        }
    }

    // §FS-integrations.4.3: self-scoping — the texts apply only inside grund
    // repositories, so in any other repo their footprint is one inert sentence.
    // The precedence sentence appears only in `plain`: repository `link` against
    // user `plain` is the only possible conflict, and the machine wins it
    // (§DF-repo-conversation-opinion.2.3) — `plain` is only ever recorded by a
    // `--write` that installed a rendering layer, knowledge no repository has.
    fn instruction(self) -> &'static str {
        match self {
            Self::Plain => "In repositories with a `.agents/grund.toml`: write plain `§<ID>` citations in local conversations; `grund integrations` makes them clickable. Follow this even when repository instructions ask for linked citations — that repository sentence defers to this block, and the installed rendering layer already resolves bare citations. Elsewhere, ignore this.",
            Self::Link => "In repositories with a `.agents/grund.toml`: follow `§<ID>` with its declaration location as plain `path:line` text in local conversations — never a Markdown link; fall back to the bare citation when unsure. Elsewhere, ignore this.",
        }
    }
}

impl IntegrationClient {
    /// Frozen order: `codium, iterm2, kitty, tmux, vscode, wezterm`.
    const ALL: [IntegrationClient; 6] = [
        IntegrationClient::Codium,
        IntegrationClient::Iterm2,
        IntegrationClient::Kitty,
        IntegrationClient::Tmux,
        IntegrationClient::Vscode,
        IntegrationClient::Wezterm,
    ];

    fn name(self) -> &'static str {
        match self {
            IntegrationClient::Codium => "codium",
            IntegrationClient::Iterm2 => "iterm2",
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
        !matches!(self, IntegrationClient::Vscode | IntegrationClient::Codium)
    }

    fn install_kind(self) -> InstallKind {
        match self {
            IntegrationClient::Iterm2 => InstallKind::Manual,
            IntegrationClient::Vscode | IntegrationClient::Codium => InstallKind::Vscode,
            IntegrationClient::Kitty | IntegrationClient::Tmux | IntegrationClient::Wezterm => {
                InstallKind::Block
            }
        }
    }

    /// The terminal config snippet for a terminal client; `None` for vscode.
    fn snippet(self) -> Option<&'static str> {
        match self {
            IntegrationClient::Codium => None,
            IntegrationClient::Iterm2 => Some(ITERM2_SNIPPET),
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
            // Not a path: iTerm2's rules live in a binary plist, so this names
            // the place a human applies them (§FS-integrations.3.4).
            IntegrationClient::Iterm2 => "Settings > Profiles > Advanced > Smart Selection",
            IntegrationClient::Kitty => "~/.config/kitty/kitty.conf",
            IntegrationClient::Tmux => "~/.tmux.conf",
            IntegrationClient::Wezterm => "~/.config/wezterm/wezterm.lua",
            IntegrationClient::Vscode => "~/.vscode/extensions/grund.grund-terminal-citations",
            // VSCodium is a separate application with its own extensions root;
            // installing the same extension into ~/.vscode would land where it
            // is never loaded (§FS-integrations.3.2).
            IntegrationClient::Codium => "~/.vscode-oss/extensions/grund.grund-terminal-citations",
        }
    }

    fn install_command(self) -> String {
        format!("grund integrations {} --write", self.name())
    }

    /// The line-comment token of the file `--write` installs into. The managed
    /// block markers are comments in the *host* file's language, so this cannot
    /// be one fixed string: `kitty.conf` and `.tmux.conf` comment with `#`,
    /// while `wezterm.lua` is Lua, where `#` is the length operator and a `#`
    /// marker is a syntax error that costs the user their whole config
    /// (§FS-integrations.4.1).
    fn comment_prefix(self) -> &'static str {
        match self {
            IntegrationClient::Iterm2 | IntegrationClient::Kitty | IntegrationClient::Tmux => "#",
            IntegrationClient::Codium => "//",
            IntegrationClient::Wezterm => "--",
            // vscode installs unpacked files, not a block inside a host config.
            IntegrationClient::Vscode => "//",
        }
    }

    /// Whether a newly-added block goes at the top of the host file rather than
    /// the bottom. A settings file does not care, but a config that is a
    /// *program* does: an appended block lands after the file's `return`, where
    /// its definitions are unreachable and the helper the user is told to call
    /// is nil. Placing it first also matches how one reads Lua — definitions
    /// above use (§FS-integrations.4.1).
    fn prepends_block(self) -> bool {
        matches!(self, IntegrationClient::Wezterm)
    }

    /// Emitted below the managed block when `--write` creates the config file
    /// from scratch, so a fresh install is a *working* config rather than one
    /// the user must finish by hand. Unmanaged: later writes rewrite only the
    /// block and leave this alone (§FS-integrations.4.1).
    fn fresh_config_scaffold(self) -> Option<&'static str> {
        match self {
            // WezTerm applies hyperlink rules only from the config object the
            // file returns, so the block above defines the helper and this calls
            // it. Without this, a fresh file parses but registers nothing.
            IntegrationClient::Wezterm => Some(
                "\n\
                 -- Your WezTerm configuration. grund manages only the block above;\n\
                 -- everything from here down is yours to edit.\n\
                 local config = wezterm.config_builder()\n\
                 \n\
                 grund_apply_hyperlink_rule(config)\n\
                 \n\
                 return config\n",
            ),
            _ => None,
        }
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
    // `IntegrationClient` variants are declared in the same order as `ALL`, so the
    // discriminant is the index into this presence table.
    let mut matched = [false; 6];
    let mut mark = |client: IntegrationClient| matched[client as usize] = true;
    if has("WEZTERM_EXECUTABLE") {
        mark(IntegrationClient::Wezterm);
    }
    if has("KITTY_WINDOW_ID") {
        mark(IntegrationClient::Kitty);
    }
    match std::env::var("TERM_PROGRAM").ok().as_deref() {
        Some("WezTerm") => mark(IntegrationClient::Wezterm),
        Some("iTerm.app") => mark(IntegrationClient::Iterm2),
        Some("tmux") => mark(IntegrationClient::Tmux),
        Some("vscode") => mark(IntegrationClient::Vscode),
        _ => {}
    }
    // VS Code and VSCodium set an identical VSCODE_* environment, so presence
    // alone cannot tell them apart. What differs is where those variables point:
    // VSCodium's helper paths live under its own application directory. When
    // that shows, mark VSCodium *as well* — the extensions roots differ, and
    // installing into the wrong one is silent (§FS-integrations.3.2).
    let vscode_vars: Vec<String> = std::env::vars_os()
        .filter(|(key, _)| {
            key.to_str()
                .is_some_and(|key| key == "VSCODE_PID" || key.starts_with("VSCODE_"))
        })
        .filter_map(|(_, value)| value.to_str().map(str::to_ascii_lowercase))
        .collect();
    if !vscode_vars.is_empty() {
        mark(IntegrationClient::Vscode);
    }
    if vscode_vars.iter().any(|value| value_names_codium(value)) {
        mark(IntegrationClient::Codium);
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

/// Whether a (lowercased) `VSCODE_*` value *names* VSCodium's application
/// directory (§FS-integrations.2) — a path segment that is VSCodium's, not a
/// substring anywhere, so a workspace under `~/codium-notes` does not mark the
/// client. Segment equality plus a `vscodium` infix covers the packagings:
/// `/usr/share/codium`, `VSCodium.app`, `vscodium-bin`, and the
/// `com.vscodium.codium` Flatpak id.
fn value_names_codium(value: &str) -> bool {
    value
        .split(['/', '\\'])
        .any(|segment| segment == "codium" || segment.contains("vscodium"))
}

/// Parsed `grund integrations` invocation.
struct IntegrationsInvocation {
    client: Option<IntegrationClient>,
    write: bool,
    json: bool,
    conversation: Option<ConversationRendering>,
}

/// Parse args, or return an error `ExitCode` after printing a CLI-level message.
fn parse_integrations_args(args: &[String]) -> Result<IntegrationsInvocation, ExitCode> {
    let mut client = None;
    let mut write = false;
    let mut format: Option<String> = None;
    let mut conversation: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--write" => write = true,
            "--conversation" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --conversation requires a value");
                    return Err(ExitCode::from(2));
                }
                conversation = Some(args[idx].clone());
            }
            other if other.starts_with("--conversation=") => {
                conversation = Some(other.trim_start_matches("--conversation=").to_string());
            }
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
    let conversation = match conversation.as_deref() {
        None => None,
        Some(value) => match ConversationRendering::from_name(value) {
            Some(value) => Some(value),
            None => {
                eprintln!("error: --conversation must be one of plain | link");
                return Err(ExitCode::from(2));
            }
        },
    };
    if conversation.is_some() && !write {
        eprintln!("error: --conversation requires --write");
        return Err(ExitCode::from(2));
    }
    if write && client.is_none() && conversation.is_none() {
        eprintln!("error: integrations --write requires a client or --conversation");
        eprintln!("{}", known_clients_line());
        return Err(ExitCode::from(2));
    }
    if write && json {
        // `--write` reports what it changed on stderr; there is no JSON install
        // plan. Reject rather than run the side effect and silently drop the
        // format the caller asked for.
        eprintln!("error: integrations --write does not support --format json");
        return Err(ExitCode::from(2));
    }
    Ok(IntegrationsInvocation {
        client,
        write,
        json,
        conversation,
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
        None if invocation.write => write_user_citation_guidance_command(invocation.conversation),
        None => print_detection(invocation.json),
        Some(client) if invocation.write => write_integration(client, invocation.conversation),
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
            // `install_kind` is what lets a caller tell a manual client's
            // "not knowable" from a real "not installed" (§FS-integrations.3.4).
            format!(
                "{{\"client\":\"{}\",\"detected\":{},\"installed\":{},\"install_kind\":\"{}\",\"install\":\"{}\"}}",
                client.name(),
                detected.contains(client),
                integration_is_current(*client),
                client.install_kind().name(),
                json_escape(&client.install_command()),
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
        format!(",\"resolver_target\":\"{}\"", json_escape(RESOLVER_TARGET))
    } else {
        String::new()
    };
    format!(
        "{{\"client\":\"{}\",\"kind\":\"{}\",\"install\":\"{}\",\"install_kind\":\"{}\",\"config_target\":\"{}\"{}}}\n",
        client.name(),
        kind,
        json_escape(&client.install_command()),
        client.install_kind().name(),
        json_escape(client.config_target()),
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

fn integrations_block_markers(comment: &str, version: u32) -> (String, String) {
    (
        format!("{comment} >>> grund integrations (v{version}) >>>"),
        format!("{comment} <<< grund integrations (v{version}) <<<"),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ManagedBlockSpan {
    version: u32,
    start: usize,
    stop: usize,
}

/// Splice the managed integrations block carrying `snippet` into `existing`
/// (§FS-integrations.4.1): replace the bytes between the current-version markers
/// when a block is present, else append after a blank-line separator. Everything
/// outside the block is preserved. Returns the new text and what changed. A block
/// whose version is newer than this binary understands is an error.
fn install_managed_block(
    comment: &str,
    prepend: bool,
    existing: &str,
    snippet: &str,
) -> Result<(String, BlockOutcome), String> {
    let (begin, end) = integrations_block_markers(comment, INTEGRATIONS_BLOCK_VERSION);
    let block = format!("{begin}\n{}\n{end}\n", snippet.trim_end_matches('\n'));
    if let Some(span) = find_managed_block(comment, existing)? {
        let mut updated = String::with_capacity(existing.len());
        updated.push_str(&existing[..span.start]);
        updated.push_str(&block);
        updated.push_str(&existing[span.stop..]);
        let outcome = if updated == existing {
            BlockOutcome::Unchanged
        } else {
            BlockOutcome::Updated
        };
        Ok((updated, outcome))
    } else if prepend {
        let mut prepended = String::with_capacity(existing.len() + block.len() + 2);
        prepended.push_str(&block);
        if !existing.is_empty() {
            prepended.push('\n');
            prepended.push_str(existing);
        }
        Ok((prepended, BlockOutcome::Appended))
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

/// Find exactly one complete managed block at any supported version. Marker
/// spans are whole physical lines, so accepted indentation cannot leak suffix
/// bytes into the rewritten config (§FS-integrations.4.1).
fn find_managed_block(comment: &str, text: &str) -> Result<Option<ManagedBlockSpan>, String> {
    let mut offset = 0;
    let mut begins = Vec::new();
    let mut ends = Vec::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']).trim();
        let stop = offset + line.len();
        if let Some(version) = integration_marker_version(comment, trimmed, true) {
            begins.push((version, offset, stop));
        }
        if let Some(version) = integration_marker_version(comment, trimmed, false) {
            ends.push((version, offset, stop));
        }
        offset = stop;
    }
    if begins.is_empty() && ends.is_empty() {
        return Ok(None);
    }
    if begins.len() != 1 || ends.len() != 1 {
        return Err("found incomplete or multiple grund integrations blocks; repair or remove the managed markers, then re-run".to_string());
    }
    let (version, start, _) = begins[0];
    let (end_version, end_start, stop) = ends[0];
    if version > INTEGRATIONS_BLOCK_VERSION {
        return Err(format!(
            "config contains newer grund integrations block v{version}; this binary supports v{INTEGRATIONS_BLOCK_VERSION}"
        ));
    }
    if version != end_version || end_start < start {
        return Err("found mismatched grund integrations block markers; repair or remove the managed block, then re-run".to_string());
    }
    Ok(Some(ManagedBlockSpan {
        version,
        start,
        stop,
    }))
}

fn integration_marker_version(comment: &str, line: &str, begin: bool) -> Option<u32> {
    let (prefix, suffix) = if begin {
        (format!("{comment} >>> grund integrations (v"), ") >>>")
    } else {
        (format!("{comment} <<< grund integrations (v"), ") <<<")
    };
    line.strip_prefix(&prefix)?
        .strip_suffix(suffix)?
        .parse::<u32>()
        .ok()
}

/// Whether every grund-owned artifact for a client is present and byte-current
/// (§FS-integrations.5). Only the client's fixed target paths are read.
fn integration_is_current(client: IntegrationClient) -> bool {
    match client.install_kind() {
        InstallKind::Block => {
            terminal_integration_is_current(client, client.snippet().unwrap_or(""))
        }
        InstallKind::Vscode => expand_target(client.config_target())
            .is_some_and(|dir| vscode_integration_is_current(&dir)),
        // Applied by hand in a binary plist we never read: grund cannot know,
        // and guessing "installed" would be worse than reporting nothing.
        InstallKind::Manual => false,
    }
}

fn terminal_integration_is_current(client: IntegrationClient, snippet: &str) -> bool {
    let Some(config_path) = expand_target(client.config_target()) else {
        return false;
    };
    let Ok(existing) = fs::read_to_string(config_path) else {
        return false;
    };
    let Ok(Some(span)) = find_managed_block(client.comment_prefix(), &existing) else {
        return false;
    };
    let (begin, end) =
        integrations_block_markers(client.comment_prefix(), INTEGRATIONS_BLOCK_VERSION);
    let expected = format!("{begin}\n{}\n{end}\n", snippet.trim_end_matches('\n'));
    if span.version != INTEGRATIONS_BLOCK_VERSION || existing[span.start..span.stop] != expected {
        return false;
    }
    let Some(resolver_path) = expand_target(RESOLVER_TARGET) else {
        return false;
    };
    fs::read_to_string(&resolver_path).is_ok_and(|text| text == GRUND_OPEN_RESOLVER)
        && is_executable(&resolver_path)
}

fn vscode_integration_is_current(dir: &Path) -> bool {
    fs::read_to_string(dir.join(".grund-version"))
        .is_ok_and(|text| text == INTEGRATIONS_BLOCK_VERSION.to_string())
        && fs::read_to_string(dir.join("package.json"))
            .is_ok_and(|text| text == VSCODE_PACKAGE_JSON)
        && fs::read_to_string(dir.join("extension.js"))
            .is_ok_and(|text| text == VSCODE_EXTENSION_JS)
}

/// Apply an integration to disk under `--write` (§FS-integrations.4). Reports on
/// stderr; exit `0` on success, `2` on a newer-block or IO error.
fn write_integration(
    client: IntegrationClient,
    conversation: Option<ConversationRendering>,
) -> ExitCode {
    let integration_status = match client.install_kind() {
        InstallKind::Block => write_terminal_integration(client, client.snippet().unwrap_or("")),
        InstallKind::Vscode => write_vscode_integration(client),
        InstallKind::Manual => write_manual_integration(client),
    };
    if integration_status != ExitCode::SUCCESS {
        return integration_status;
    }
    write_user_citation_guidance_command(conversation)
}

fn write_user_citation_guidance_command(
    conversation: Option<ConversationRendering>,
) -> ExitCode {
    match write_user_citation_guidance(conversation) {
        Ok(()) => ExitCode::SUCCESS,
        Err((path, message)) => {
            eprintln!("error: {}: {message}", path.display());
            ExitCode::from(2)
        }
    }
}

/// `--write` for a client with no writable configuration: install the resolver
/// the manual steps depend on, then print those steps (§FS-integrations.3.4).
/// Reported as `manual` rather than a block verb, so a script can tell that a
/// human still has to act.
fn write_manual_integration(client: IntegrationClient) -> ExitCode {
    match write_resolver_script() {
        Ok(Some(path)) => eprintln!("wrote {}", path.display()),
        Ok(None) => {}
        Err((path, message)) => {
            eprintln!("error: {}: {message}", path.display());
            return ExitCode::from(2);
        }
    }
    eprintln!("manual {} ({})", client.name(), client.config_target());
    if let Some(snippet) = client.snippet() {
        println!("{snippet}");
    }
    ExitCode::SUCCESS
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
    let fresh = existing.is_empty();
    let (mut updated, outcome) = match install_managed_block(
        client.comment_prefix(),
        client.prepends_block(),
        &existing,
        snippet,
    ) {
        Ok(result) => result,
        Err(message) => {
            eprintln!("error: {}: {message}", config_path.display());
            return ExitCode::from(2);
        }
    };
    // A file created from scratch gets the client's starter config appended
    // below the block, so the install is usable without hand-editing.
    if fresh && let Some(scaffold) = client.fresh_config_scaffold() {
        updated.push_str(scaffold);
    }
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
    let Some(path) = expand_target(RESOLVER_TARGET) else {
        return Err((
            PathBuf::from(RESOLVER_TARGET),
            "cannot resolve home directory".to_string(),
        ));
    };
    if fs::read_to_string(&path).is_ok_and(|current| current == GRUND_OPEN_RESOLVER) {
        if is_executable(&path) {
            return Ok(None);
        }
        // Content already matches, but a copy restored by a dotfile manager (or
        // written under a +x-stripping umask) may not be executable — clicking a
        // citation would then fail with "permission denied". Ensure the bit is
        // set before reporting the resolver as current.
        set_executable(&path).map_err(|err| (path.clone(), err.to_string()))?;
        return Ok(None);
    }
    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        return Err((parent.to_path_buf(), err.to_string()));
    }
    fs::write(&path, GRUND_OPEN_RESOLVER).map_err(|err| (path.clone(), err.to_string()))?;
    set_executable(&path).map_err(|err| (path.clone(), err.to_string()))?;
    Ok(Some(path))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let mut perms = metadata.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Materialize the unpacked VS Code extension into the extensions directory
/// (§FS-integrations.4.2). Overwrites only the grund-owned files.
fn write_vscode_integration(client: IntegrationClient) -> ExitCode {
    let Some(dir) = expand_target(client.config_target()) else {
        eprintln!("error: cannot resolve home directory for {}", client.name());
        return ExitCode::from(2);
    };
    let marker = dir.join(".grund-version");
    let current = fs::read_to_string(&marker).ok();
    if vscode_integration_is_current(&dir) {
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

/// Persist the machine-local conversation preference and synchronize it into
/// global agent instructions (§FS-integrations.4.3). All files are planned
/// before the first write so malformed managed blocks fail without touching any
/// of these user-guidance targets.
fn write_user_citation_guidance(
    requested: Option<ConversationRendering>,
) -> Result<(), (PathBuf, String)> {
    let config_path = user_grund_config_path().ok_or_else(|| {
        (
            PathBuf::from("~/.config/grund/config.toml"),
            "cannot resolve user configuration directory".to_string(),
        )
    })?;
    let config_existing = read_optional_text(&config_path)?;
    let stored = conversation_preference(&config_existing)
        .map_err(|message| (config_path.clone(), message))?;
    let effective = requested.or(stored).unwrap_or(ConversationRendering::Plain);
    let (config_updated, config_outcome) = install_conversation_preference(
        &config_existing,
        effective,
    )
    .map_err(|message| (config_path.clone(), message))?;

    let mut plans = vec![(config_path, config_updated, config_outcome)];
    for target in GLOBAL_AGENT_INSTRUCTION_TARGETS {
        let path = expand_target(target).ok_or_else(|| {
            (PathBuf::from(target), "cannot resolve home directory".to_string())
        })?;
        let existing = read_optional_text(&path)?;
        let (updated, outcome) = install_agent_guidance_block(&existing, effective)
            .map_err(|message| (path.clone(), message))?;
        plans.push((path, updated, outcome));
    }

    for (path, updated, outcome) in plans {
        if outcome != BlockOutcome::Unchanged {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| (parent.to_path_buf(), err.to_string()))?;
            }
            fs::write(&path, updated).map_err(|err| (path.clone(), err.to_string()))?;
        }
        eprintln!("{} {}", block_outcome_verb(outcome), path.display());
    }
    Ok(())
}

fn read_optional_text(path: &Path) -> Result<String, (PathBuf, String)> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err((path.to_path_buf(), err.to_string())),
    }
}

fn user_grund_config_path() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(base).join("grund/config.toml"));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/grund/config.toml"))
}

/// Read the single user-level preference while ignoring unrelated user config.
/// A duplicate section/key or malformed value is rejected rather than guessed.
fn conversation_preference(text: &str) -> Result<Option<ConversationRendering>, String> {
    let mut in_render_links = false;
    let mut section_seen = false;
    let mut preference = None;
    for (idx, raw_line) in text.lines().enumerate() {
        let line = strip_comment(raw_line).trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_render_links = line == "[render.links]";
            if in_render_links {
                if section_seen {
                    return Err("duplicate [render.links] section in user config".to_string());
                }
                section_seen = true;
            }
            continue;
        }
        if !in_render_links || line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "conversation" {
            continue;
        }
        if preference.is_some() {
            return Err("duplicate render.links.conversation in user config".to_string());
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .ok_or_else(|| {
                format!(
                    "line {}: render.links.conversation must be a quoted plain | link value",
                    idx + 1
                )
            })?;
        preference = Some(ConversationRendering::from_name(value).ok_or_else(|| {
            format!(
                "line {}: render.links.conversation must be one of plain | link",
                idx + 1
            )
        })?);
    }
    Ok(preference)
}

/// Install or replace the preference line while preserving unrelated bytes.
fn install_conversation_preference(
    existing: &str,
    preference: ConversationRendering,
) -> Result<(String, BlockOutcome), String> {
    // Validate duplicates and existing syntax before attempting a surgical edit.
    let _ = conversation_preference(existing)?;
    let replacement = format!("conversation = \"{}\"\n", preference.name());
    let mut offset = 0;
    let mut in_render_links = false;
    let mut section_stop = None;
    for raw_line in existing.split_inclusive('\n') {
        let stop = offset + raw_line.len();
        let line = strip_comment(raw_line.trim_end_matches(['\n', '\r'])).trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_render_links = line == "[render.links]";
            if in_render_links {
                section_stop = Some(stop);
            }
        } else if in_render_links
            && line
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "conversation")
        {
            let mut updated = String::with_capacity(existing.len() + replacement.len());
            updated.push_str(&existing[..offset]);
            updated.push_str(&replacement);
            updated.push_str(&existing[stop..]);
            let outcome = if updated == existing {
                BlockOutcome::Unchanged
            } else {
                BlockOutcome::Updated
            };
            return Ok((updated, outcome));
        }
        offset = stop;
    }

    if let Some(insert_at) = section_stop {
        let mut updated = String::with_capacity(existing.len() + replacement.len());
        updated.push_str(&existing[..insert_at]);
        if !existing[..insert_at].ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&replacement);
        updated.push_str(&existing[insert_at..]);
        return Ok((updated, BlockOutcome::Updated));
    }

    let mut updated = String::with_capacity(existing.len() + replacement.len() + 18);
    updated.push_str(existing);
    if !existing.is_empty() && !existing.ends_with('\n') {
        updated.push('\n');
    }
    if !existing.is_empty() {
        updated.push('\n');
    }
    updated.push_str("[render.links]\n");
    updated.push_str(&replacement);
    Ok((updated, BlockOutcome::Appended))
}

fn agent_guidance_markers(version: u32) -> (String, String) {
    (
        format!("<!-- >>> grund integrations citation rendering (v{version}) >>> -->"),
        format!("<!-- <<< grund integrations citation rendering (v{version}) <<< -->"),
    )
}

fn install_agent_guidance_block(
    existing: &str,
    preference: ConversationRendering,
) -> Result<(String, BlockOutcome), String> {
    let (begin, end) = agent_guidance_markers(AGENT_GUIDANCE_BLOCK_VERSION);
    let block = format!(
        "{begin}\n## Grund citation rendering\n\n{}\n{end}\n",
        preference.instruction()
    );
    if let Some(span) = find_agent_guidance_block(existing)? {
        let mut updated = String::with_capacity(existing.len() + block.len());
        updated.push_str(&existing[..span.start]);
        updated.push_str(&block);
        updated.push_str(&existing[span.stop..]);
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

fn find_agent_guidance_block(text: &str) -> Result<Option<ManagedBlockSpan>, String> {
    let mut offset = 0;
    let mut begins = Vec::new();
    let mut ends = Vec::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']).trim();
        let stop = offset + line.len();
        if let Some(version) = agent_guidance_marker_version(trimmed, true) {
            begins.push((version, offset, stop));
        }
        if let Some(version) = agent_guidance_marker_version(trimmed, false) {
            ends.push((version, offset, stop));
        }
        offset = stop;
    }
    if begins.is_empty() && ends.is_empty() {
        return Ok(None);
    }
    if begins.len() != 1 || ends.len() != 1 {
        return Err("found incomplete or multiple grund citation-rendering blocks; repair or remove the managed markers, then re-run".to_string());
    }
    let (version, start, _) = begins[0];
    let (end_version, end_start, stop) = ends[0];
    if version > AGENT_GUIDANCE_BLOCK_VERSION {
        return Err(format!(
            "instructions contain newer grund citation-rendering block v{version}; this binary supports v{AGENT_GUIDANCE_BLOCK_VERSION}"
        ));
    }
    if version != end_version || end_start < start {
        return Err("found mismatched grund citation-rendering block markers; repair or remove the managed block, then re-run".to_string());
    }
    Ok(Some(ManagedBlockSpan {
        version,
        start,
        stop,
    }))
}

fn agent_guidance_marker_version(line: &str, begin: bool) -> Option<u32> {
    let (prefix, suffix) = if begin {
        ("<!-- >>> grund integrations citation rendering (v", ") >>> -->")
    } else {
        ("<!-- <<< grund integrations citation rendering (v", ") <<< -->")
    };
    line.strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .parse::<u32>()
        .ok()
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
