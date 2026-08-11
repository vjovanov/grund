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
/// v3 (§DF-conversation-link-target): the `link` text addresses the declaration
/// through `conversation_target`, gated per agent.
const AGENT_GUIDANCE_BLOCK_VERSION: u32 = 3;

/// How much of the linked conversation form one agent's renderer is *verified*
/// to honor (§DF-conversation-link-target.2.4). Anything unverified resolves to
/// `ConversationTarget::Path`, the form that surface already had — the gate can
/// hold a target where it is, never make one worse.
#[derive(Clone, Copy, Eq, PartialEq)]
enum LinkSupport {
    /// Every target, local schemes included (matrix rows 12–14).
    Every,
    /// `file:` and web URLs dispatch, editor schemes do not; labels survive
    /// either way (matrix row 16, Pi).
    FileAndWeb,
    /// Web URLs only; a local destination is rendered in place of the link
    /// label, erasing the citation (matrix rows 3 and 15, Codex).
    WebOnly,
    /// No click-test either way, so the citation keeps its plain location.
    Unverified,
}

impl LinkSupport {
    fn resolve(self, target: ConversationTarget) -> ConversationTarget {
        match (self, target) {
            (Self::Every, _) => target,
            (Self::FileAndWeb, ConversationTarget::File | ConversationTarget::Web) => target,
            (Self::WebOnly, ConversationTarget::Web) => target,
            _ => ConversationTarget::Path,
        }
    }
}

/// One agent's file-backed global instruction surface (§FS-integrations.4.3).
/// `home` is the directory whose presence says the user actually runs that
/// agent: `--write` installs a *rendering layer*, and provisioning the config
/// tree of five agents the machine does not have is not part of that.
struct GlobalAgentTarget {
    /// The agent's name — the key `[reference.agents.<agent>]` is written under
    /// and the value `--agent` accepts (§FS-integrations.4.4).
    agent: &'static str,
    /// `~`-rooted instruction file, printed verbatim so reports stay stable.
    file: &'static str,
    /// `~`-rooted directory that shows the agent is in use.
    home: &'static str,
    /// What this agent's renderer is verified to do with a linked citation.
    link_support: LinkSupport,
}

/// The file-backed global instruction surfaces for every agent grund supports
/// end-to-end (§FS-integrations.4.3). Keep this superset aligned with the
/// repository entrypoints in §FS-init.2.1.
const GLOBAL_AGENT_INSTRUCTION_TARGETS: [GlobalAgentTarget; 6] = [
    GlobalAgentTarget {
        agent: "codex",
        file: "~/.codex/AGENTS.md",
        home: "~/.codex",
        link_support: LinkSupport::WebOnly,
    },
    GlobalAgentTarget {
        agent: "claude",
        file: "~/.claude/CLAUDE.md",
        home: "~/.claude",
        link_support: LinkSupport::Every,
    },
    GlobalAgentTarget {
        agent: "gemini",
        file: "~/.gemini/GEMINI.md",
        home: "~/.gemini",
        link_support: LinkSupport::Unverified,
    },
    GlobalAgentTarget {
        agent: "copilot",
        file: "~/.copilot/copilot-instructions.md",
        home: "~/.copilot",
        link_support: LinkSupport::Unverified,
    },
    GlobalAgentTarget {
        agent: "zed",
        file: "~/.config/zed/AGENTS.md",
        home: "~/.config/zed",
        link_support: LinkSupport::Unverified,
    },
    GlobalAgentTarget {
        agent: "pi",
        file: "~/.pi/agent/AGENTS.md",
        home: "~/.pi",
        link_support: LinkSupport::FileAndWeb,
    },
];

/// The user-level Grund configuration `--write` records the preference in
/// (§FS-integrations.4.3). `~/.config` resolves through `XDG_CONFIG_HOME`.
const USER_CONFIG_TARGET: &str = "~/.config/grund/config.toml";

/// The user-level setting spelled exactly as the repository key for the same
/// concept (§FS-config.3.1): one name, two scopes.
const CONVERSATION_KEY_PATH: &str = "reference.conversation";

/// How a linked citation addresses its declaration (§FS-integrations.4.3). No
/// repository spelling — the scheme is machine state
/// (§DF-conversation-link-target.2.3).
const CONVERSATION_TARGET_KEY_PATH: &str = "reference.conversation_target";

/// The keys grund consumes from the user configuration, for the unused-key
/// warning that names them.
const USER_CONFIG_KEY_PATHS: &str = "`reference.conversation`, `reference.conversation_target`, and `reference.agents.<agent>.conversation_target`";

/// The table one agent's overrides live under (§FS-integrations.4.4). The
/// partial is merged over the machine-wide keys, so only the names above are
/// accepted inside it.
fn agent_override_table(agent: &str) -> String {
    format!("reference.agents.{agent}")
}

/// `codex | claude | …` for the errors that list the accepted set.
fn known_agents_list() -> String {
    GLOBAL_AGENT_INSTRUCTION_TARGETS
        .iter()
        .map(|target| target.agent)
        .collect::<Vec<_>>()
        .join(", ")
}

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
    //
    // These texts name no marker, for the same reason the matchers and the
    // resolver do not hardcode one (§FS-integrations.3.1): they are user-global
    // and written once, before grund knows which repositories will be opened,
    // and `[reference] marker` is per-repo. There is nothing to interpolate at
    // install time, so these carry the policy and the repository entrypoint —
    // which does render its own marker — carries the syntax.
    fn instruction(self, target: ConversationTarget) -> String {
        match self {
            Self::Plain => "In repositories with a `.agents/grund.toml`: write citations bare in local conversations — the marker and ID alone, nothing appended; `grund integrations` makes them clickable. Follow this even when repository instructions ask for linked citations — that repository sentence defers to this block, and the installed rendering layer already resolves bare citations. Elsewhere, ignore this.".to_string(),
            // The target is the one value interpolated here, and legitimately
            // so: unlike the marker it *is* machine state, which is what a
            // user-global file is for (§FS-integrations.4.3).
            Self::Link => match target.uri_phrase() {
                None => "In repositories with a `.agents/grund.toml`: follow each citation with its declaration location as plain `path:line` text in local conversations; fall back to the bare citation when unsure. Elsewhere, ignore this.".to_string(),
                Some(phrase) => format!(
                    "In repositories with a `.agents/grund.toml`: in local conversations render each citation as a Markdown link whose visible text is the citation itself and whose target is {phrase}; fall back to the bare citation when unsure. Elsewhere, ignore this."
                ),
            },
        }
    }
}

/// How a linked citation addresses its declaration (§FS-config.3.1,
/// §DF-conversation-link-target.2.2). A closed enum: each value names one fixed
/// template an agent fills from the declaration's absolute path and line.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ConversationTarget {
    /// `file://<abs>#L<line>` — the default, and the only local form that
    /// presumes nothing about the machine beyond a handler for `file:`.
    #[default]
    File,
    /// No URI: the location travels as plain `path:line` text. The pre-2026-08-11
    /// form, kept as the opt-out and used as the gate's fallback.
    Path,
    /// The forge blob URL for the current ref, per the repository-web rule.
    Web,
    Vscode,
    Vscodium,
    Cursor,
}

impl ConversationTarget {
    /// Accepted values, in the order the error message lists them.
    const ALL: [ConversationTarget; 6] = [
        ConversationTarget::File,
        ConversationTarget::Path,
        ConversationTarget::Web,
        ConversationTarget::Vscode,
        ConversationTarget::Vscodium,
        ConversationTarget::Cursor,
    ];

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|target| target.name() == name)
    }

    fn name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Path => "path",
            Self::Web => "web",
            Self::Vscode => "vscode",
            Self::Vscodium => "vscodium",
            Self::Cursor => "cursor",
        }
    }

    /// `file | path | web | …` for the error that lists the accepted set.
    fn accepted_list() -> String {
        Self::ALL
            .iter()
            .map(|target| target.name())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// The template clause the instruction block names, or `None` for `Path`,
    /// which carries no URI and takes the plain-location sentence instead.
    fn uri_phrase(self) -> Option<&'static str> {
        match self {
            Self::Path => None,
            Self::File => Some("`file://<absolute path>#L<line>` for the declaration"),
            Self::Web => Some("the declaration's forge URL at the current commit"),
            Self::Vscode => Some("`vscode://file<absolute path>:<line>` for the declaration"),
            Self::Vscodium => Some("`vscodium://file<absolute path>:<line>` for the declaration"),
            Self::Cursor => Some("`cursor://file<absolute path>:<line>` for the declaration"),
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
    conversation_target: Option<ConversationTarget>,
    /// `--agent <name>`: scope `conversation_target` to one agent instead of
    /// the machine (§FS-integrations.4.4).
    agent: Option<&'static str>,
}

/// Parse args, or return an error `ExitCode` after printing a CLI-level message.
fn parse_integrations_args(args: &[String]) -> Result<IntegrationsInvocation, ExitCode> {
    let mut client = None;
    let mut write = false;
    let mut format: Option<String> = None;
    let mut conversation: Option<String> = None;
    let mut conversation_target: Option<String> = None;
    let mut agent: Option<String> = None;
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
            "--agent" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --agent requires a value");
                    return Err(ExitCode::from(2));
                }
                agent = Some(args[idx].clone());
            }
            other if other.starts_with("--agent=") => {
                agent = Some(other.trim_start_matches("--agent=").to_string());
            }
            "--conversation-target" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --conversation-target requires a value");
                    return Err(ExitCode::from(2));
                }
                conversation_target = Some(args[idx].clone());
            }
            other if other.starts_with("--conversation-target=") => {
                conversation_target =
                    Some(other.trim_start_matches("--conversation-target=").to_string());
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
    let conversation_target = match conversation_target.as_deref() {
        None => None,
        Some(value) => match ConversationTarget::from_name(value) {
            Some(value) => Some(value),
            None => {
                eprintln!(
                    "error: --conversation-target must be one of {}",
                    ConversationTarget::accepted_list()
                );
                return Err(ExitCode::from(2));
            }
        },
    };
    // §FS-integrations.4.4: `--agent` scopes `--conversation-target` and
    // nothing else, so an agent with no target is an error rather than a
    // silent no-op.
    let agent = match agent.as_deref() {
        None => None,
        Some(value) => match known_agent(value) {
            Some(known) => Some(known),
            None => {
                eprintln!(
                    "error: unknown agent `{value}`; known agents: {}",
                    known_agents_list()
                );
                return Err(ExitCode::from(2));
            }
        },
    };
    if agent.is_some() && !write {
        eprintln!("error: --agent requires --write");
        return Err(ExitCode::from(2));
    }
    if agent.is_some() && conversation_target.is_none() {
        eprintln!("error: --agent requires --conversation-target");
        return Err(ExitCode::from(2));
    }
    if conversation.is_some() && !write {
        eprintln!("error: --conversation requires --write");
        return Err(ExitCode::from(2));
    }
    if conversation_target.is_some() && !write {
        eprintln!("error: --conversation-target requires --write");
        return Err(ExitCode::from(2));
    }
    // Either conversation flag is enough to make the clientless form
    // unambiguous: it updates the preference and the instruction blocks and
    // installs no arbitrary client (§FS-integrations.1).
    if write && client.is_none() && conversation.is_none() && conversation_target.is_none() {
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
        conversation_target,
        agent,
    })
}

/// The `grund integrations` entry point, called from both CLI frontends
/// (§FS-integrations). Prints by default; writes only under `--write`.
pub fn run_integrations(args: &[String]) -> ExitCode {
    let invocation = match parse_integrations_args(args) {
        Ok(invocation) => invocation,
        Err(code) => return code,
    };
    // §FS-integrations.4.3: `--write` reads the user configuration exactly once,
    // before any artifact is installed — so its warnings are reported once, and
    // a file grund cannot parse fails the command outright rather than after a
    // client's config and the resolver are already on disk.
    if invocation.write {
        let user_config = match load_user_config() {
            Ok(config) => config,
            Err((path, message)) => {
                eprintln!("error: {}: {message}", path.display());
                return ExitCode::from(2);
            }
        };
        return match invocation.client {
            Some(client) => write_integration(
                client,
                invocation.conversation,
                invocation.conversation_target,
                invocation.agent,
                user_config,
            ),
            None => write_user_citation_guidance_command(
                invocation.conversation,
                invocation.conversation_target,
                invocation.agent,
                user_config,
            ),
        };
    }
    match invocation.client {
        None => print_detection(invocation.json),
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
    conversation_target: Option<ConversationTarget>,
    agent: Option<&'static str>,
    user_config: UserConfig,
) -> ExitCode {
    let integration_status = match client.install_kind() {
        InstallKind::Block => write_terminal_integration(client, client.snippet().unwrap_or("")),
        InstallKind::Vscode => write_vscode_integration(client),
        InstallKind::Manual => write_manual_integration(client),
    };
    if integration_status != ExitCode::SUCCESS {
        return integration_status;
    }
    write_user_citation_guidance_command(conversation, conversation_target, agent, user_config)
}

fn write_user_citation_guidance_command(
    conversation: Option<ConversationRendering>,
    conversation_target: Option<ConversationTarget>,
    agent: Option<&'static str>,
    user_config: UserConfig,
) -> ExitCode {
    match write_user_citation_guidance(conversation, conversation_target, agent, user_config) {
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

/// What `--write` will do to one user-guidance file, decided before any of them
/// is touched (§FS-integrations.4.3).
enum GuidancePlan {
    /// Write these bytes, reporting the block outcome's verb.
    Write(String, BlockOutcome),
    /// The agent is not in use on this machine — report why and write nothing.
    /// Named so the report says which directory would make it apply.
    Skip(&'static str),
    /// An instruction file, reported with the form it received
    /// (§FS-integrations.4.4).
    WriteAgent(String, BlockOutcome, EffectiveForm),
}

/// What one agent's block ended up teaching, and why it is not what was asked
/// for when it is not (§FS-integrations.4.4). Reported per target, because
/// unreported an override, a gate downgrade, and an unread key are
/// indistinguishable from the outside.
struct EffectiveForm {
    rendering: ConversationRendering,
    target: ConversationTarget,
    requested: ConversationTarget,
    overridden: bool,
}

impl EffectiveForm {
    fn describe(&self) -> String {
        if self.rendering == ConversationRendering::Plain {
            return ConversationRendering::Plain.name().to_string();
        }
        let mut note = format!("{} \u{2192} {}", self.rendering.name(), self.target.name());
        if self.target != self.requested {
            note.push_str(&format!("; {} unverified here", self.requested.name()));
        } else if self.overridden {
            note.push_str("; agent override");
        }
        note
    }
}

/// The user configuration `--write` reads, loaded once per invocation so its
/// warnings are reported exactly once (§FS-integrations.4.3).
struct UserConfig {
    path: PathBuf,
    text: String,
    preference: Option<ConversationRendering>,
    target: Option<ConversationTarget>,
    agent_targets: Vec<(String, ConversationTarget)>,
}

/// Read and report on the user configuration without writing anything. Every
/// line grund did not act on is warned about here, at the one point the file is
/// read: nothing else in this file has any effect, and a setting that silently
/// does nothing is indistinguishable from one that works. Only failing to reach
/// the file is an error; its contents never are (§FS-integrations.4.3).
fn load_user_config() -> Result<UserConfig, (PathBuf, String)> {
    let path = user_grund_config_path().ok_or_else(|| {
        (
            PathBuf::from(USER_CONFIG_TARGET),
            "cannot resolve user configuration directory".to_string(),
        )
    })?;
    let text = read_optional_text(&path)?;
    let scan = scan_user_config(&text);
    for (line, message) in &scan.problems {
        eprintln!("warning: {}:{line}: {message}", path.display());
    }
    Ok(UserConfig {
        path,
        text,
        preference: scan.preference,
        target: scan.target,
        agent_targets: scan.agent_targets,
    })
}

/// Persist the machine-local conversation preference and synchronize it into
/// global agent instructions (§FS-integrations.4.3). All files are planned
/// before the first write so malformed managed blocks fail without touching any
/// of these user-guidance targets.
fn write_user_citation_guidance(
    requested: Option<ConversationRendering>,
    requested_target: Option<ConversationTarget>,
    scoped_agent: Option<&'static str>,
    user_config: UserConfig,
) -> Result<(), (PathBuf, String)> {
    let UserConfig {
        path: config_path,
        text: config_existing,
        preference: stored,
        target: stored_target,
        mut agent_targets,
    } = user_config;
    let effective = requested.or(stored).unwrap_or(ConversationRendering::Plain);
    // A scoped write changes one agent's partial and leaves the base exactly as
    // it was — that is the whole point of the flag (§FS-integrations.4.4).
    let machine_target = if scoped_agent.is_some() {
        stored_target.unwrap_or_default()
    } else {
        requested_target.or(stored_target).unwrap_or_default()
    };
    // Both keys are recorded, and both are recorded even when inert: a machine
    // that set a target under `plain` keeps it when it later switches to `link`
    // (§FS-integrations.1).
    let (config_updated, conversation_outcome) = install_reference_key(
        &config_existing,
        "reference",
        "conversation",
        effective.name(),
        stored == Some(effective),
    );
    let (config_updated, target_outcome) = install_reference_key(
        &config_updated,
        "reference",
        "conversation_target",
        machine_target.name(),
        stored_target == Some(machine_target),
    );
    let mut config_outcome = merge_outcomes(conversation_outcome, target_outcome);
    let mut config_updated = config_updated;
    if let (Some(agent), Some(target)) = (scoped_agent, requested_target) {
        let stored_for_agent = agent_targets
            .iter()
            .find(|(name, _)| name == agent)
            .map(|(_, value)| *value);
        let (next, outcome) = install_reference_key(
            &config_updated,
            &agent_override_table(agent),
            "conversation_target",
            target.name(),
            stored_for_agent == Some(target),
        );
        config_updated = next;
        config_outcome = merge_outcomes(config_outcome, outcome);
        match agent_targets.iter_mut().find(|(name, _)| name == agent) {
            Some(entry) => entry.1 = target,
            None => agent_targets.push((agent.to_string(), target)),
        }
    }

    let mut plans = vec![(
        config_path,
        GuidancePlan::Write(config_updated, config_outcome),
    )];
    for target in GLOBAL_AGENT_INSTRUCTION_TARGETS {
        let path = expand_target(target.file).ok_or_else(|| {
            (
                PathBuf::from(target.file),
                "cannot resolve home directory".to_string(),
            )
        })?;
        // In use when the agent's own directory exists, or when the instruction
        // file already does — the latter keeps a target grund wrote earlier (or
        // the user maintains by hand) synchronized even if the directory check
        // would no longer select it.
        let in_use = expand_target(target.home).is_some_and(|home| home.is_dir()) || path.is_file();
        if !in_use {
            plans.push((path, GuidancePlan::Skip(target.home)));
            continue;
        }
        // §FS-integrations.4.4: the agent's own partial replaces the base, then
        // §DF-conversation-link-target.2.4 gates the result — the override moves
        // the request, never the verdict, so no key can instruct a form recorded
        // as erasing the citation on that surface.
        let overridden = agent_targets
            .iter()
            .find(|(name, _)| name == target.agent)
            .map(|(_, value)| *value);
        let requested_for_agent = overridden.unwrap_or(machine_target);
        let gated = target.link_support.resolve(requested_for_agent);
        let form = EffectiveForm {
            rendering: effective,
            target: gated,
            requested: requested_for_agent,
            overridden: overridden.is_some(),
        };
        let existing = read_optional_text(&path)?;
        let (updated, outcome) = install_agent_guidance_block(&existing, effective, gated)
            .map_err(|message| (path.clone(), message))?;
        plans.push((path, GuidancePlan::WriteAgent(updated, outcome, form)));
    }

    for (path, plan) in plans {
        match plan {
            GuidancePlan::Write(updated, outcome) => {
                if outcome != BlockOutcome::Unchanged {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|err| (parent.to_path_buf(), err.to_string()))?;
                    }
                    fs::write(&path, updated).map_err(|err| (path.clone(), err.to_string()))?;
                }
                eprintln!("{} {}", block_outcome_verb(outcome), path.display());
            }
            GuidancePlan::WriteAgent(updated, outcome, form) => {
                if outcome != BlockOutcome::Unchanged {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|err| (parent.to_path_buf(), err.to_string()))?;
                    }
                    fs::write(&path, updated).map_err(|err| (path.clone(), err.to_string()))?;
                }
                eprintln!(
                    "{} {} ({})",
                    block_outcome_verb(outcome),
                    path.display(),
                    form.describe()
                );
            }
            GuidancePlan::Skip(home) => {
                eprintln!("skipped {} (no {home})", path.display());
            }
        }
    }
    Ok(())
}

/// One file, two managed keys: `exists` only when neither line moved, and an
/// append anywhere makes the whole write an append (§FS-integrations.6).
fn merge_outcomes(first: BlockOutcome, second: BlockOutcome) -> BlockOutcome {
    match (first, second) {
        (BlockOutcome::Unchanged, other) | (other, BlockOutcome::Unchanged) => other,
        (BlockOutcome::Appended, _) | (_, BlockOutcome::Appended) => BlockOutcome::Appended,
        _ => BlockOutcome::Updated,
    }
}

fn read_optional_text(path: &Path) -> Result<String, (PathBuf, String)> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err((path.to_path_buf(), err.to_string())),
    }
}

fn user_grund_config_path() -> Option<PathBuf> {
    expand_target(USER_CONFIG_TARGET)
}

/// The dotted name of a `[section]` header line, whitespace-normalized so
/// `[ reference ]` and `[reference]` — the same table in TOML — are the same
/// section here. `[[array]]` headers keep their inner bracket and therefore
/// never compare equal to a real section name.
fn section_header_name(line: &str) -> Option<String> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split('.')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// The contents of a TOML basic or literal string, or `None` when the value is
/// not quoted. Both quote styles mean the same thing for a value from a closed
/// enum, and rejecting `'link'` — which TOML accepts — would reject a config the
/// user wrote correctly.
fn unquote_toml_string(value: &str) -> Option<&str> {
    let value = value.trim();
    ['"', '\''].into_iter().find_map(|quote| {
        value
            .strip_prefix(quote)
            .and_then(|inner| inner.strip_suffix(quote))
    })
}

/// The fully-qualified key path of a `key = value` line inside `section`, so the
/// equivalent TOML spellings — `[reference]` + `conversation`, `[reference]` +
/// nothing with a dotted root `reference.conversation`, `[reference.sub]` — all
/// reduce to one name.
fn qualified_key(section: &str, key: &str) -> String {
    if section.is_empty() {
        key.to_string()
    } else {
        format!("{section}.{key}")
    }
}

/// What one scan of the user configuration found (§FS-integrations.4.3).
struct UserConfigScan {
    preference: Option<ConversationRendering>,
    /// The recorded addressing target, independent of `preference`: an
    /// unreadable target never costs the `plain`/`link` value recorded beside
    /// it (§FS-integrations.4.3).
    target: Option<ConversationTarget>,
    /// `[reference.agents.<agent>]` partials, in file order — the override
    /// layer merged over `target` per agent (§FS-integrations.4.4). Only known
    /// agents land here; an unknown one is a warning naming the closed set.
    agent_targets: Vec<(String, ConversationTarget)>,
    /// `(line, message)` for everything in the file grund did not act on, in
    /// file order. Every message names what is being ignored, so the report says
    /// what the run will do rather than only what is wrong.
    problems: Vec<(usize, String)>,
}

/// Scan the user configuration for the single preference grund reads there, and
/// collect everything else it did not act on (§FS-integrations.4.3).
///
/// This is a targeted scan, not a TOML parser, so it must accept every spelling
/// TOML calls equivalent — whitespace inside the section header, a dotted key
/// path, either quote style. A spelling grund merely failed to *see* would be
/// silently reversed to the default and then written back alongside the
/// original, leaving the opposite of what the user asked for.
///
/// Recognizing the right spelling is only half of that, though: nothing tells a
/// reader of this file which keys grund actually consumes, so a typo, a retired
/// spelling, or a repository key set here in the belief that it applies globally
/// all read as "configured" and do nothing.
///
/// **Nothing here fails.** One rule covers the whole file: report it, ignore it,
/// continue on the documented default. This file is machine-local, read by one
/// command, and decides nothing about whether a tree checks clean — the opposite
/// of the closed allow-list the repository config enforces (§FS-config.4.3),
/// where an unknown key means two installs could disagree about a checked tree.
/// A stale line in a personal config is not a reason to refuse to install a
/// terminal integration, and an unparseable value is not more of a reason than
/// an unread key: both mean grund has no preference from this file, which is
/// exactly the state of a machine that never wrote one.
fn scan_user_config(text: &str) -> UserConfigScan {
    let mut section = String::new();
    let mut preference = None;
    let mut target = None;
    let mut agent_targets: Vec<(String, ConversationTarget)> = Vec::new();
    let mut problems = Vec::new();
    for (idx, raw_line) in text.lines().enumerate() {
        let line = strip_comment(raw_line).trim();
        if let Some(name) = section_header_name(line) {
            section = name;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let qualified = qualified_key(&section, key.trim());
        let value = value.trim();
        // First occurrence wins, per key, and it is the one a write rewrites, so
        // a read and a write can never disagree about which line is the setting.
        match qualified.as_str() {
            CONVERSATION_KEY_PATH => {
                if preference.is_some() {
                    problems.push((
                        idx + 1,
                        format!(
                            "ignoring duplicate `{CONVERSATION_KEY_PATH}`; the first one is used"
                        ),
                    ));
                    continue;
                }
                let Some(unquoted) = unquote_toml_string(value) else {
                    problems.push((
                        idx + 1,
                        format!(
                            "ignoring `{CONVERSATION_KEY_PATH} = {value}`: must be a quoted plain | link value"
                        ),
                    ));
                    continue;
                };
                match ConversationRendering::from_name(unquoted) {
                    Some(parsed) => preference = Some(parsed),
                    None => problems.push((
                        idx + 1,
                        format!(
                            "ignoring `{CONVERSATION_KEY_PATH} = {value}`: must be one of plain | link"
                        ),
                    )),
                }
            }
            CONVERSATION_TARGET_KEY_PATH => {
                if target.is_some() {
                    problems.push((
                        idx + 1,
                        format!(
                            "ignoring duplicate `{CONVERSATION_TARGET_KEY_PATH}`; the first one is used"
                        ),
                    ));
                    continue;
                }
                let accepted = ConversationTarget::accepted_list();
                let Some(unquoted) = unquote_toml_string(value) else {
                    problems.push((
                        idx + 1,
                        format!(
                            "ignoring `{CONVERSATION_TARGET_KEY_PATH} = {value}`: must be a quoted {accepted} value"
                        ),
                    ));
                    continue;
                };
                match ConversationTarget::from_name(unquoted) {
                    Some(parsed) => target = Some(parsed),
                    None => problems.push((
                        idx + 1,
                        format!(
                            "ignoring `{CONVERSATION_TARGET_KEY_PATH} = {value}`: must be one of {accepted}"
                        ),
                    )),
                }
            }
            // §FS-integrations.4.4: `[reference.agents.<agent>]` is a partial of
            // the keys above, so it accepts the same names and nothing else.
            _ if section.starts_with("reference.agents.") => {
                let agent = section.trim_start_matches("reference.agents.");
                let Some(known) = known_agent(agent) else {
                    problems.push((
                        idx + 1,
                        format!(
                            "unknown agent `{agent}` in reference.agents; known agents: {}",
                            known_agents_list()
                        ),
                    ));
                    continue;
                };
                if key.trim() != "conversation_target" {
                    problems.push((
                        idx + 1,
                        format!(
                            "unused key `{qualified}`; grund reads only {USER_CONFIG_KEY_PATHS} from this file"
                        ),
                    ));
                    continue;
                }
                if agent_targets.iter().any(|(name, _)| name == known) {
                    problems.push((
                        idx + 1,
                        format!("ignoring duplicate `{qualified}`; the first one is used"),
                    ));
                    continue;
                }
                let accepted = ConversationTarget::accepted_list();
                match unquote_toml_string(value).and_then(ConversationTarget::from_name) {
                    Some(parsed) => agent_targets.push((known.to_string(), parsed)),
                    None => problems.push((
                        idx + 1,
                        format!("ignoring `{qualified} = {value}`: must be one of {accepted}"),
                    )),
                }
            }
            _ => problems.push((
                idx + 1,
                format!(
                    "unused key `{qualified}`; grund reads only {USER_CONFIG_KEY_PATHS} from this file"
                ),
            )),
        }
    }
    UserConfigScan {
        preference,
        target,
        agent_targets,
        problems,
    }
}

/// The canonical spelling of a known agent name, or `None` (§FS-integrations.4.4).
fn known_agent(name: &str) -> Option<&'static str> {
    GLOBAL_AGENT_INSTRUCTION_TARGETS
        .iter()
        .map(|target| target.agent)
        .find(|known| *known == name)
}

/// The `plain`/`link` preference alone, for tests that assert on one key.
#[cfg(test)]
fn conversation_preference(text: &str) -> Option<ConversationRendering> {
    scan_user_config(text).preference
}

/// The recorded addressing target alone, for tests that assert on one key.
#[cfg(test)]
fn conversation_target_preference(text: &str) -> Option<ConversationTarget> {
    scan_user_config(text).target
}

/// `install_reference_key` bound to the `conversation` key, which is what most
/// of the preference-file tests exercise.
#[cfg(test)]
fn install_conversation_preference(
    existing: &str,
    preference: ConversationRendering,
) -> (String, BlockOutcome) {
    install_reference_key(
        existing,
        "reference",
        "conversation",
        preference.name(),
        conversation_preference(existing) == Some(preference),
    )
}

/// Install or replace one `[reference]` line while preserving unrelated bytes.
/// Infallible: every defect in this file is a warning reported at load
/// (§FS-integrations.4.3), so there is nothing left here to refuse.
///
/// `table` is the table the key belongs to — `reference`, or one agent's
/// `reference.agents.<agent>` partial (§FS-integrations.4.4) — and `bare_key`
/// the name to write when the key is absent; the two are joined to match an
/// existing line, since the file may spell the key dotted at root.
/// `already_recorded` says the scan already read this exact value.
fn install_reference_key(
    existing: &str,
    table: &str,
    bare_key: &str,
    value: &str,
    already_recorded: bool,
) -> (String, BlockOutcome) {
    let key_path = format!("{table}.{bare_key}");
    // Already recorded: leave the bytes alone. Rewriting an identical value would
    // report `updated` for a no-op and drop whatever comment the user wrote
    // beside it — a second `--write` is a no-op reporting `exists`
    // (§FS-integrations.6).
    if already_recorded {
        return (existing.to_string(), BlockOutcome::Unchanged);
    }
    let mut offset = 0;
    let mut section = String::new();
    let mut section_stop = None;
    for raw_line in existing.split_inclusive('\n') {
        let stop = offset + raw_line.len();
        let raw = raw_line.trim_end_matches(['\n', '\r']);
        let line = strip_comment(raw).trim();
        if let Some(name) = section_header_name(line) {
            section = name;
            if section == table {
                section_stop = Some(stop);
            }
            offset = stop;
            continue;
        }
        if let Some((key, _)) = line.split_once('=')
            && qualified_key(&section, key.trim()) == key_path
        {
            // Rewrite only the value, keeping the key exactly as written: a
            // dotted `reference.conversation` at root rewritten as a bare
            // `conversation` would land in whatever table precedes it and stop
            // being this setting at all.
            let head = &raw[..raw.find('=').unwrap_or(raw.len()) + 1];
            let replacement = format!("{head} \"{value}\"\n");
            let mut updated = String::with_capacity(existing.len() + replacement.len());
            updated.push_str(&existing[..offset]);
            updated.push_str(&replacement);
            updated.push_str(&existing[stop..]);
            let outcome = if updated == existing {
                BlockOutcome::Unchanged
            } else {
                BlockOutcome::Updated
            };
            return (updated, outcome);
        }
        offset = stop;
    }

    let replacement = format!("{bare_key} = \"{value}\"\n");
    if let Some(insert_at) = section_stop {
        let mut updated = String::with_capacity(existing.len() + replacement.len());
        updated.push_str(&existing[..insert_at]);
        if !existing[..insert_at].ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&replacement);
        updated.push_str(&existing[insert_at..]);
        return (updated, BlockOutcome::Updated);
    }

    let mut updated = String::with_capacity(existing.len() + replacement.len() + 18);
    updated.push_str(existing);
    if !existing.is_empty() && !existing.ends_with('\n') {
        updated.push('\n');
    }
    if !existing.is_empty() {
        updated.push('\n');
    }
    updated.push_str(&format!("[{table}]\n"));
    updated.push_str(&replacement);
    (updated, BlockOutcome::Appended)
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
    target: ConversationTarget,
) -> Result<(String, BlockOutcome), String> {
    let (begin, end) = agent_guidance_markers(AGENT_GUIDANCE_BLOCK_VERSION);
    let block = format!(
        "{begin}\n## Grund citation rendering\n\n{}\n{end}\n",
        preference.instruction(target)
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

/// Expand a leading `~` in an install-target hint against `$HOME`, resolving
/// `~/.config` through `XDG_CONFIG_HOME` when that is set (§FS-integrations.4).
///
/// The hints themselves are printed verbatim and stay `~`-rooted so reports are
/// byte-stable across machines (§FS-integrations.6); only resolution is
/// environment-dependent. Every client whose configuration lives under
/// `~/.config` — kitty, WezTerm, Zed, and grund's own user config — reads it
/// from `$XDG_CONFIG_HOME` when set, so writing to a hardcoded `~/.config`
/// there lands where the tool never looks, and the failure is silent in exactly
/// the way §FS-integrations.3.2 refuses to accept for VSCodium.
fn expand_target(target: &str) -> Option<PathBuf> {
    if let Some(rest) = target.strip_prefix("~/.config/") {
        return Some(user_config_base()?.join(rest));
    }
    if let Some(rest) = target.strip_prefix("~/") {
        let home = std::env::var_os("HOME")?;
        return Some(PathBuf::from(home).join(rest));
    }
    Some(PathBuf::from(target))
}

/// `$XDG_CONFIG_HOME`, or `$HOME/.config` when it is unset or empty.
fn user_config_base() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(base));
    }
    Some(PathBuf::from(std::env::var_os("HOME")?).join(".config"))
}
