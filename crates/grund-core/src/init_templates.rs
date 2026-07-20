// The scaffold templates `grund init` writes are embedded in the binary; the
// reference copies live under `templates/` in the source tree (§FS-init.2.1).
const AGENTS_TEMPLATE: &str = include_str!("../assets/templates/AGENTS.md");
const GRUND_TOML_TEMPLATE: &str = include_str!("../assets/templates/grund.toml");
const GRUND_DOC_TEMPLATE: &str = include_str!("../assets/templates/grund.md");
const GOALS_TEMPLATE: &str = include_str!("../assets/templates/goals.md");
const REQUIREMENTS_TEMPLATE: &str = include_str!("../assets/templates/requirements.md");
const FS_README_TEMPLATE: &str = include_str!("../assets/templates/functional-spec-README.md");
const E2E_README_TEMPLATE: &str = include_str!("../assets/templates/e2e-README.md");
const AS_README_TEMPLATE: &str = include_str!("../assets/templates/architecture-README.md");
const GITKEEP_TEMPLATE: &str = include_str!("../assets/templates/gitkeep.md");
pub const AGENT_SETUP_INSTRUCTIONS: &str = include_str!("../assets/skills/grund-init/SKILL.md");
// v4 (§FS-init.2.3.6, §DF-integrations-command): adds a generated `### Clickable
// citations` section derived from `[render.links]`,
// byte-compared by `grund check` for drift (§FS-check.3.5). v3 (§FS-init.2.3.5,
// §DF-citation-directions) replaced the hand-written climbing-rule bullet with a
// generated `### Citation directions` section derived from `[citations]`.
const AGENTS_BLOCK_VERSION: u32 = 4;
const CANONICAL_AGENT_ENTRYPOINT: &str = "AGENTS.md";
const COMPANION_AGENT_ENTRYPOINTS: &[CompanionAgentEntrypoint] = &[
    CompanionAgentEntrypoint {
        rel: "AGENTS.override.md",
        workspace: None,
        agent: None,
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: "CLAUDE.md",
        workspace: Some(".claude"),
        agent: Some(AgentEntrypoint::Claude),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: ".claude/CLAUDE.md",
        workspace: Some(".claude"),
        agent: Some(AgentEntrypoint::Claude),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: "GEMINI.md",
        workspace: Some(".gemini"),
        agent: Some(AgentEntrypoint::Gemini),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: ".github/copilot-instructions.md",
        workspace: None,
        agent: Some(AgentEntrypoint::Copilot),
        discovery: true,
        create_on_request: true,
    },
    // §FS-init.2.1 / §FS-init.2.3: Cursor uses `.cursor/rules/*.mdc` files (the
    // modern form) and a legacy `.cursorrules` single-file form. We create a
    // grund-specific `.cursor/rules/grund.mdc` (won't collide with any other
    // rule file) when `.cursor/` already exists or `--cursor` is passed; the
    // legacy `.cursorrules` is only updated if it already exists, never
    // created — the modern path is preferred for new adopters.
    CompanionAgentEntrypoint {
        rel: ".cursor/rules/grund.mdc",
        workspace: Some(".cursor"),
        agent: Some(AgentEntrypoint::Cursor),
        discovery: true,
        create_on_request: true,
    },
    CompanionAgentEntrypoint {
        rel: ".cursorrules",
        workspace: None,
        agent: Some(AgentEntrypoint::Cursor),
        discovery: true,
        create_on_request: false,
    },
    CompanionAgentEntrypoint {
        rel: ".windsurfrules",
        workspace: None,
        agent: Some(AgentEntrypoint::Windsurf),
        discovery: true,
        create_on_request: true,
    },
    // §FS-init.2.3: `.rules` is too generic to attribute to Zed by filename
    // alone, so we only touch it when the `.zed/` workspace already exists or
    // `--zed` is explicit — discovery-by-file-existence is disabled.
    CompanionAgentEntrypoint {
        rel: ".rules",
        workspace: Some(".zed"),
        agent: Some(AgentEntrypoint::Zed),
        discovery: false,
        create_on_request: true,
    },
];

#[derive(Clone, Copy, Eq, PartialEq)]
enum AgentEntrypoint {
    Claude,
    Gemini,
    Copilot,
    Cursor,
    Windsurf,
    Zed,
}

#[derive(Clone, Default)]
pub struct InitAgentEntrypointSelection {
    pub canonical: bool,
    pub claude: bool,
    pub gemini: bool,
    pub copilot: bool,
    pub cursor: bool,
    pub windsurf: bool,
    pub zed: bool,
}

impl InitAgentEntrypointSelection {
    fn any(&self) -> bool {
        self.canonical
            || self.claude
            || self.gemini
            || self.copilot
            || self.cursor
            || self.windsurf
            || self.zed
    }

    fn includes(&self, agent: AgentEntrypoint) -> bool {
        match agent {
            AgentEntrypoint::Claude => self.claude,
            AgentEntrypoint::Gemini => self.gemini,
            AgentEntrypoint::Copilot => self.copilot,
            AgentEntrypoint::Cursor => self.cursor,
            AgentEntrypoint::Windsurf => self.windsurf,
            AgentEntrypoint::Zed => self.zed,
        }
    }
}

struct CompanionAgentEntrypoint {
    rel: &'static str,
    workspace: Option<&'static str>,
    agent: Option<AgentEntrypoint>,
    /// Whether automatic mode should detect this entrypoint by file existence
    /// alone. `false` for entrypoints whose filename is too generic to
    /// attribute to a single tool (e.g. `.rules`) — those rely on the
    /// workspace directory or an explicit agent flag instead.
    discovery: bool,
    /// Whether an explicit agent flag creates this entrypoint when it is absent.
    /// Legacy Cursor `.cursorrules` is updated when present but never created;
    /// new Cursor installs use `.cursor/rules/grund.mdc` instead (§FS-init.2.1).
    create_on_request: bool,
}

enum InitCompanionAgentEntrypoint {
    Existing(PathBuf),
    MissingAlias(PathBuf),
}

pub fn canonical_template_text(template: &str) -> String {
    template.replace("\r\n", "\n").replace('\r', "\n")
}

/// The substitutions that turn `templates/AGENTS.md` into a concrete `AGENTS.md`
/// for a repo (§FS-init.2.3): the project name, plus the ID/marker shape taken
/// from the config `grund init` leaves in place — so a `{kind}-{slug}` repo gets a
/// `<KIND>-<slug>` description, a strict repo gets the strict-mode note, custom
/// kinds show up in the kind set, and so on. Everything *not* substituted here is
/// fixed for the block version. `{ID_SHAPE_SEC}` is listed before `{ID_SHAPE}`
/// only for readability; neither placeholder is a substring of the other.
/// `target` is the directory being initialized; it's the anchor for the
/// `{WORKSPACE_MEMBERS}` walk-up and for the relative path rendering inside that
/// section (§FS-init.2.3.4.15). `canonical_agent_entrypoint_selected` records
/// whether this run is writing/updating `target/AGENTS.md`; companion-only init
/// must not pretend that missing file exists.
fn agents_template_substitutions(
    name: &str,
    config: &Config,
    target: &Path,
    canonical_agent_entrypoint_selected: bool,
) -> Vec<(&'static str, String)> {
    let sep = config.section_separator.as_str();
    let marker = config.marker.as_str();
    let id_shape = config
        .id_format
        .replace("{kind}", "<KIND>")
        .replace("{number}", "<NNN>")
        .replace("{slug}", "<slug>");
    let id_example = config
        .id_format
        .replace("{kind}", "FS")
        .replace("{number}", "042")
        .replace("{slug}", "user-login");
    // §FS-init.2.3: the worked example is an illustration of a non-existent ID,
    // so it is rendered in the `<marker>`-escaped form (§FS-workspace.1) — a
    // live marker would make the generated block fail the host repo's own
    // `grund check` as a dangling reference.
    let cite_example = format!("<{marker}>{id_example}{sep}3{sep}1");
    let kinds_set = format!("{{{}}}", kind_prefixes(&config.kinds).join(", "));
    let bare_note = if config.strict {
        format!(
            "Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `.agents/grund.toml`, so only `{marker}`-prefixed citations are checked."
        )
    } else {
        format!(
            "Bare ID-shaped tokens are also recognized as citations because `[reference] strict = false` is set in `.agents/grund.toml`; remove that compatibility override or set strict back to `true` to require the `{marker}` marker (run `grund fmt --marker` first to upgrade existing bare citations)."
        )
    };
    let section_heading_note = section_heading_note(config, marker);
    let inline_citation_style = inline_citation_style_sentence(config);
    vec![
        ("{NAME}", name.to_string()),
        ("{ID_SHAPE_SEC}", format!("{id_shape}[{sep}<section>]")),
        ("{ID_SHAPE}", id_shape),
        ("{ID_EXAMPLE}", id_example),
        ("{CITE_EXAMPLE}", cite_example),
        ("{KINDS_SET}", kinds_set),
        ("{BARE_TOKEN_NOTE}", bare_note),
        ("{SECTION_HEADING_NOTE}", section_heading_note),
        ("{INLINE_CITATION_STYLE}", inline_citation_style),
        ("{MARKER}", marker.to_string()),
        ("{TRIGGER}", config.trigger.clone()),
        ("{DECLARATION_MAP}", declaration_map(config)),
        ("{CITATION_DIRECTIONS}", citation_directions_section(config)),
        ("{CLICKABLE_CITATIONS}", clickable_citations_section(config)),
        (
            "{WORKSPACE_MEMBERS}",
            render_workspace_members_section(
                target,
                Some(name),
                // The pending effective config carries the `--description`
                // value when `init` is about to write a fresh config; with an
                // existing config the walk-up reloads it anyway
                // (§FS-init.2.3.4.15).
                config.project_description.as_deref(),
                marker,
                canonical_agent_entrypoint_selected,
            ),
        ),
    ]
}

fn section_heading_note(config: &Config, marker: &str) -> String {
    let sep = config.section_separator.as_str();
    match config.section_heading_levels.as_str() {
        "strict" => format!(
            "Numbered headings inside a declaration are citable sections: use depth-matching headings (`## 1. …`, `### 1.1 …`, etc.) so `{marker}<ID>{sep}1` / `{marker}<ID>{sep}1.1` resolve; mismatched heading depth is a `grund check` error. Plain headings or bold labels are fine for non-citable local structure."
        ),
        "warn" => format!(
            "Numbered headings inside a declaration are citable sections: use depth-matching headings (`## 1. …`, `### 1.1 …`, etc.) so `{marker}<ID>{sep}1` / `{marker}<ID>{sep}1.1` resolve; mismatched heading depth is a `grund check` warning. Plain headings or bold labels are fine for non-citable local structure."
        ),
        _ => format!(
            "Numbered headings inside a declaration are citable sections: `{marker}<ID>{sep}1` / `{marker}<ID>{sep}1.1` resolve by dotted number, and depth-matching headings (`## 1. …`, `### 1.1 …`) are recommended for readability. Plain headings or bold labels are fine for non-citable local structure."
        ),
    }
}

fn inline_citation_style_sentence(config: &Config) -> String {
    if config.inline_style == "citation-only" {
        return "Inline citations carry no prose — put rationale in the spec.".to_string();
    }
    if config.inline_note_suggested_lines == config.inline_note_max_lines {
        format!(
            "Inline notes: ≤ {} line{}, ≤ {} columns.",
            config.inline_note_max_lines,
            plural(config.inline_note_max_lines),
            config.inline_note_max_columns
        )
    } else {
        format!(
            "Inline notes: ≤ {} line{} preferred, hard cap {} lines; ≤ {} columns.",
            config.inline_note_suggested_lines,
            plural(config.inline_note_suggested_lines),
            config.inline_note_max_lines,
            config.inline_note_max_columns
        )
    }
}

fn plural(value: usize) -> &'static str {
    if value == 1 { "" } else { "s" }
}

fn markdown_link_label(raw: &str) -> String {
    raw.replace('\\', r"\\")
        .replace('[', r"\[")
        .replace(']', r"\]")
}

fn markdown_link_destination(raw: &str) -> String {
    if raw
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '(' | ')' | '<' | '>'))
    {
        format!("<{}>", raw.replace('\\', r"\\").replace('>', r"\>"))
    } else {
        raw.to_string()
    }
}

/// Render the `### Citation directions` managed-block section (§FS-init.2.3.5)
/// from the effective `[citations]` rules. Deterministic — `[[kinds]]` order,
/// `code` last, fixed level phrases, `|`→"or", conjunction→"and" — so
/// `grund check` can re-render and byte-compare it for drift (§FS-check.3.5).
/// When no `[citations]` section is declared, the static climbing-rule sentence
/// stands in, so a config that predates the feature keeps a stable block.
fn citation_directions_section(config: &Config) -> String {
    // Built as lines joined with `\n` and returned without a trailing newline:
    // the `{CITATION_DIRECTIONS}` placeholder in the template supplies the single
    // block-final newline, so `grund init` stays idempotent on re-run.
    let mut lines = vec!["### Citation directions".to_string(), String::new()];
    if !config.citations.declared {
        lines.push(
            "Specs cite goals, architecture cites specs, code and executable tests cite the specs they realize."
                .to_string(),
        );
        return lines.join("\n");
    }
    if let Some(default) = config
        .citations
        .global_default
        .and_then(citation_global_default_sentence)
    {
        lines.push(default);
    }
    // `[[kinds]]` order, then the `code` pseudo-kind last (§FS-init.2.3.5).
    let mut kinds: Vec<String> = config.kinds.iter().map(|k| k.prefix.clone()).collect();
    if config.citations.per_kind.contains_key(CODE_SOURCE_KIND) {
        kinds.push(CODE_SOURCE_KIND.to_string());
    }
    for kind in &kinds {
        let Some(rules) = config.citations.per_kind.get(kind) else {
            continue;
        };
        let Some(clauses) = citation_direction_clauses(rules) else {
            continue;
        };
        let label = if kind == CODE_SOURCE_KIND {
            "**code** (any file outside a kind home)".to_string()
        } else {
            format!("**{kind}**")
        };
        lines.push(format!("- {label} {clauses}."));
    }
    // Load-bearing (§FS-init.2.3.5): silence is open-world only when neither the
    // global default nor a per-kind default changes it.
    if citation_defaults_are_open_world(config) {
        lines.push("Unlisted kinds and pairs are fine.".to_string());
    } else {
        lines.push("Unlisted kinds and pairs follow their configured defaults.".to_string());
    }
    lines.join("\n")
}

/// Render the `### Clickable citations` managed-block section
/// (§FS-init.2.3.6) from the effective `[render.links]` config. Deterministic —
/// each key selects one of a fixed set of phrases — so `grund check` can
/// re-render and byte-compare it for drift (§FS-check.3.5). Returned without a
/// trailing newline; the `{CLICKABLE_CITATIONS}` placeholder supplies the final
/// newline, so `grund init` stays idempotent on re-run.
fn clickable_citations_section(config: &Config) -> String {
    if config.render_links_conversation == "plain" {
        return "### Clickable citations\n\nIn conversations, write plain `§<ID>` citations."
            .to_string();
    }
    let base = if config.render_links_web_base == "auto" {
        "<web-base>".to_string()
    } else {
        config.render_links_web_base.trim_end_matches('/').to_string()
    };
    let title = if config.render_links_hover_title {
        " \"<heading>\""
    } else {
        ""
    };
    let composed = format!("[§<ID>]({base}/<ref>/<path>#<anchor>{title})");
    format!(
        "### Clickable citations\n\nIn conversations, use `{composed}`; choose the target from context and fall back to plain when unsure.",
    )
}

/// The verb-phrase clauses for one citing kind's rules, joined by "; " in the
/// order obligations, permissions, prohibitions, then the default
/// (§FS-init.2.3.5). `None` when the kind has no renderable rule.
fn citation_direction_clauses(rules: &KindCitationRules) -> Option<String> {
    let mut clauses = Vec::new();
    if !rules.must.is_empty() {
        clauses.push(format!("must cite {}", citation_rule_targets(&rules.must)));
    }
    if !rules.should.is_empty() {
        clauses.push(format!("should cite {}", citation_rule_targets(&rules.should)));
    }
    if !rules.may.is_empty() {
        clauses.push(format!("may cite {}", citation_rule_targets(&rules.may)));
    }
    if !rules.must_not.is_empty() {
        clauses.push(format!("never cite {}", citation_rule_targets(&rules.must_not)));
    }
    if !rules.should_not.is_empty() {
        clauses.push(format!("avoid citing {}", citation_rule_targets(&rules.should_not)));
    }
    if let Some(default) = rules.default {
        clauses.push(citation_default_clause(default));
    }
    if clauses.is_empty() {
        return None;
    }
    Some(clauses.join("; "))
}

fn citation_defaults_are_open_world(config: &Config) -> bool {
    let global_open = matches!(config.citations.global_default, None | Some(CitationLevel::May));
    global_open
        && config
            .citations
            .per_kind
            .values()
            .all(|rules| matches!(rules.default, None | Some(CitationLevel::May)))
}

fn citation_default_clause(level: CitationLevel) -> String {
    match level {
        CitationLevel::Must => "unlisted citations default to must".to_string(),
        CitationLevel::Should => "unlisted citations default to should".to_string(),
        CitationLevel::May => "unlisted citations are fine".to_string(),
        CitationLevel::ShouldNot => "unlisted citations are discouraged".to_string(),
        CitationLevel::MustNot => "unlisted citations are forbidden".to_string(),
    }
}

fn citation_global_default_sentence(level: CitationLevel) -> Option<String> {
    match level {
        CitationLevel::Must => {
            Some("By default, unlisted citation pairs are treated as must.".to_string())
        }
        CitationLevel::Should => {
            Some("By default, unlisted citation pairs are treated as should.".to_string())
        }
        CitationLevel::May => None,
        CitationLevel::ShouldNot => {
            Some("By default, unlisted citation pairs are discouraged.".to_string())
        }
        CitationLevel::MustNot => {
            Some("By default, unlisted citation pairs are forbidden.".to_string())
        }
    }
}

/// Render a list of disjunctions as a target phrase: alternatives within an
/// entry joined by " or ", conjunctive entries joined by " and "
/// (§FS-init.2.3.5).
fn citation_rule_targets(disjunctions: &[CitationDisjunction]) -> String {
    disjunctions
        .iter()
        .map(|disjunction| {
            disjunction
                .targets
                .iter()
                .map(render_citation_target)
                .collect::<Vec<_>>()
                .join(" or ")
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

fn declaration_map(config: &Config) -> String {
    let mut lines = Vec::new();
    for kind in &config.kinds {
        let prefix = markdown_link_label(&kind.prefix);
        let title = kind.title.as_deref().unwrap_or("Declaration");
        if let Some(home) = kind.file.as_deref().or(kind.folder.as_deref()) {
            lines.push(format!(
                "- [{prefix}]({}): {title}",
                markdown_link_destination(home)
            ));
        } else {
            lines.push(format!(
                "- `{}`: {title} (inline / configured by convention)",
                kind.prefix.replace('`', "\\`")
            ));
        }
    }
    lines.join("\n")
}

/// The managed block — just the H2 section that `init` appends to, or replaces
/// inside, an existing `AGENTS.md` (§FS-init.2.3). The template *is* the block;
/// the H2 line carrying the version is its own begin marker (§FS-init.2.3.1).
/// `target` is the directory being initialized — the anchor for the
/// workspace-members walk-up (§FS-init.2.3.4.15).
fn render_agents_append_block(
    name: &str,
    config: &Config,
    target: &Path,
    canonical_agent_entrypoint_selected: bool,
) -> String {
    let mut rendered = canonical_template_text(AGENTS_TEMPLATE);
    for (placeholder, value) in agents_template_substitutions(
        name,
        config,
        target,
        canonical_agent_entrypoint_selected,
    ) {
        rendered = rendered.replace(placeholder, &value);
    }
    rendered
}

/// The full generated `AGENTS.md` for a fresh repo — the H1 scaffolding line
/// followed by the managed block (§FS-init.2.3). The H1 is *unmanaged* — `init`
/// owns the block, not the title. Deterministic: same `grund` version, same
/// `--name`, same effective config, same workspace state ⇒ byte-identical
/// output (§FS-non-goals.13).
#[cfg(test)]
fn render_agents_md(
    name: &str,
    config: &Config,
    target: &Path,
    canonical_agent_entrypoint_selected: bool,
) -> String {
    let block = render_agents_append_block(name, config, target, canonical_agent_entrypoint_selected);
    render_agents_md_from_block(name, &block)
}

/// Same shape as [`render_agents_md`] but takes a pre-rendered managed block,
/// so `command_init` can render the block once and reuse it as both the full
/// `AGENTS.md` body *and* the append/update payload — the workspace-members
/// walk-up (§FS-init.2.3.4.15) only runs once per `init` invocation.
fn render_agents_md_from_block(name: &str, block: &str) -> String {
    format!("# {name} — agent instructions\n\n{block}")
}

/// Existing companion agent entrypoints that should carry the same managed grund
/// block as `AGENTS.md` (§FS-init.2.1). A symlink to `AGENTS.md` is already
/// covered by the canonical file and is intentionally skipped. Generic
/// non-discovery entrypoints (currently `.rules`) are included only when their
/// workspace proves ownership or the file already carries a managed grund block.
fn companion_agent_entrypoints(root: &Path) -> Result<Vec<PathBuf>, (PathBuf, String)> {
    let mut paths = Vec::new();
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if !is_file_or_symlink(&path) {
            continue;
        }
        match is_symlink_to(&path, &canonical) {
            Ok(true) => continue,
            Ok(false) => {
                if companion_selected_by_evidence(root, entrypoint, &path) {
                    paths.push(path);
                }
            }
            Err(err) => return Err((path, format!("{err:#}"))),
        }
    }
    Ok(paths)
}

/// Companion entrypoints `grund init` should update or create (§FS-init.2.1).
/// Existing companions are updated in place. Generic non-discovery entrypoints
/// are not selected by filename alone, but they are selected when the owning
/// workspace exists or when a previous `grund init` left a managed block there.
fn existing_init_companion_agent_entrypoints(
    root: &Path,
) -> Result<(bool, Vec<InitCompanionAgentEntrypoint>), (PathBuf, String)> {
    let mut paths = Vec::new();
    let mut canonical_requested_by_symlink = false;
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if is_file_or_symlink(&path) {
            match is_symlink_to(&path, &canonical) {
                Ok(true) => {
                    canonical_requested_by_symlink = true;
                    continue;
                }
                Ok(false) => {
                    if companion_selected_by_evidence(root, entrypoint, &path) {
                        paths.push(InitCompanionAgentEntrypoint::Existing(path));
                    }
                }
                Err(err) => return Err((path, format!("{err:#}"))),
            }
        }
    }
    Ok((canonical_requested_by_symlink, paths))
}

/// Missing neutral aliases are created only when their owning agent-specific
/// workspace directory already exists; generic project metadata directories
/// remain existing-file-only.
fn workspace_init_companion_agent_entrypoints(root: &Path) -> Vec<InitCompanionAgentEntrypoint> {
    let mut paths = Vec::new();
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if entrypoint
            .workspace
            .is_some_and(|workspace| root.join(workspace).is_dir())
            && path_missing_without_following_symlinks(&path)
        {
            paths.push(InitCompanionAgentEntrypoint::MissingAlias(path));
        }
    }
    paths
}

/// Explicit agent flags create their requested companion entrypoints even when
/// the normal automatic detection would not choose them.
fn requested_init_companion_agent_entrypoints(
    root: &Path,
    selection: &InitAgentEntrypointSelection,
) -> Result<(bool, Vec<InitCompanionAgentEntrypoint>), (PathBuf, String)> {
    let mut paths = Vec::new();
    let mut canonical_requested_by_symlink = false;
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        if !entrypoint.agent.is_some_and(|agent| selection.includes(agent)) {
            continue;
        }
        let path = root.join(entrypoint.rel);
        if is_file_or_symlink(&path) {
            match is_symlink_to(&path, &canonical) {
                Ok(true) => {
                    canonical_requested_by_symlink = true;
                    continue;
                }
                Ok(false) => paths.push(InitCompanionAgentEntrypoint::Existing(path)),
                Err(err) => return Err((path, format!("{err:#}"))),
            }
        } else if entrypoint.create_on_request {
            paths.push(InitCompanionAgentEntrypoint::MissingAlias(path));
        }
    }
    Ok((canonical_requested_by_symlink, paths))
}

fn companion_workspace_exists(root: &Path, entrypoint: &CompanionAgentEntrypoint) -> bool {
    entrypoint
        .workspace
        .is_some_and(|workspace| root.join(workspace).is_dir())
}

/// Whether the on-disk `path` is grund-owned despite belonging to an entrypoint
/// whose filename is too generic to attribute by existence alone (currently
/// `.rules`, §FS-init.2.1). True when the entry is discovery-safe by filename,
/// when its owning workspace directory proves ownership, or when the file
/// already carries a managed block from a prior `grund init` — same evidence
/// for both `grund check`'s companion scan and `grund init`'s update set, so
/// both call sites resolve through one helper. Ordering matters: `discovery`
/// is the cheap-path short-circuit so most companions never touch the disk.
fn companion_selected_by_evidence(
    root: &Path,
    entrypoint: &CompanionAgentEntrypoint,
    path: &Path,
) -> bool {
    entrypoint.discovery
        || companion_workspace_exists(root, entrypoint)
        || companion_has_managed_block(path)
}

fn companion_has_managed_block(path: &Path) -> bool {
    // Malformed delimiters still prove grund ownership — selecting the file
    // lets `check` surface the defect instead of silently skipping it.
    fs::read_to_string(path).is_ok_and(|text| {
        !matches!(find_agents_block(&text), AgentsBlockLookup::Absent)
    })
}

fn is_file_or_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type())
        .is_ok_and(|t| t.is_file() || t.is_symlink())
}

fn path_missing_without_following_symlinks(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(_) => false,
        Err(err) => err.kind() == std::io::ErrorKind::NotFound,
    }
}

fn is_symlink_to(path: &Path, target: &Path) -> Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let link = fs::read_link(path)?;
    let resolved = if link.is_absolute() {
        link
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(link)
    };
    Ok(normalize_path_lexically(&resolved) == normalize_path_lexically(target))
}

/// The config that `grund init` will leave governing `target`, which the generated
/// `AGENTS.md` must describe (§FS-init.2.3): an existing `target/.agents/grund.toml`
/// if there is one, otherwise the defaults plus the *pending* `project_name`
/// and `project_description` that `init` is about to write into
/// `target/.agents/grund.toml` (§FS-init.2.4). The `pending` in the name flags
/// that the returned `Config` may carry values that are not yet on disk —
/// callers must not treat it as reflecting persisted state. We do **not** walk
/// up to an ancestor's config here — `init` always writes a config *in*
/// `target` when one is absent.
fn init_pending_effective_config(target: &Path, name: &str, description: Option<&str>) -> Config {
    let local_config = target.join(".agents").join("grund.toml");
    if local_config.is_file() {
        load_config(target).unwrap_or_else(|_| Config::default_for(target.to_path_buf()))
    } else {
        let mut config = Config::default_for(target.to_path_buf());
        config.project_name = Some(name.to_string());
        config.project_description = description.map(str::to_string);
        config
    }
}

/// The generated `.agents/grund.toml` — every default written out explicitly as a
/// teaching surface, with only `project_name` substituted (§FS-init.2.4). With
/// `--description`, the commented `project_description` teaching line becomes
/// the real key (§FS-init.2.4, §DF-workspace-member-descriptions).
fn render_grund_toml(name: &str, description: Option<&str>) -> String {
    let mut rendered =
        canonical_template_text(GRUND_TOML_TEMPLATE).replace("{NAME}", &escape_toml_basic(name));
    if let Some(description) = description {
        rendered = rendered.replace(
            "# project_description = \"<one line shown next to this project in workspace member lists>\"",
            &format!(
                "project_description = \"{}\"",
                escape_toml_basic(description)
            ),
        );
    }
    rendered
}

fn escape_toml_basic(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

/// One resolved workspace project — the alias, canonical root, and optional
/// one-line description — collected by [`find_init_workspace_context`] so the
/// workspace-members renderer never has to talk to the config layer directly
/// (§FS-init.2.3.4.15, §DF-workspace-member-descriptions).
struct InitWorkspaceProject {
    alias: String,
    project_root: PathBuf,
    description: Option<String>,
}

/// Walk up from `target` to the nearest ancestor whose `.agents/grund.toml`
/// declares `[workspace]`, then expand its members and derive each alias the
/// same way `grund check` does (§FS-workspace.2 / §FS-workspace.3). Returns the
/// alias-sorted project list (root + members, subject to `include_root`) when
/// `target` sits inside a workspace; `None` otherwise. Returns `None` rather
/// than an error on any workspace configuration problem (missing member,
/// duplicate alias, nested workspace, …) — init must not fail because a
/// sibling member is misconfigured; the next `grund check` will surface the
/// issue (§FS-init.2.3.4.15).
fn find_init_workspace_context(
    target: &Path,
    pending_project_name: Option<&str>,
    pending_project_description: Option<&str>,
) -> Option<Vec<InitWorkspaceProject>> {
    let root_config = find_init_workspace_root(target)?;
    // `expand_workspace_members` returns canonical member roots, so a
    // non-canonical `target` would never match the self project on path
    // equality and the self-exception in §FS-init.2.3.4.15 would silently
    // misfire. Suppress the section rather than render a wrong self row.
    let target_canonical = fs::canonicalize(target).ok()?;
    let member_roots = expand_workspace_members(&root_config).ok()?;
    let mut projects = Vec::new();
    if root_config.workspace_include_root {
        let alias = derive_alias(&root_config, None, RootMode::Root).ok()?;
        projects.push(InitWorkspaceProject {
            alias,
            project_root: root_config.root.clone(),
            description: root_config.project_description.clone(),
        });
    }
    for member_root in &member_roots {
        let mut member_config = load_config_at_with_report_base(
            member_root,
            &root_config.cli_base,
            Some(&root_config.root),
        )
        .ok()?;
        if member_root == &target_canonical
            && !member_root.join(".agents").join("grund.toml").is_file()
        {
            // §FS-init.2.3.4.15: self is rendered against the config `init`
            // is about to write, so `grund init member --name service
            // --description "…"` teaches the future `service/...` workspace
            // alias and its description immediately.
            if let Some(name) = pending_project_name {
                member_config.project_name = Some(name.to_string());
            }
            if let Some(description) = pending_project_description {
                member_config.project_description = Some(description.to_string());
            }
        }
        if member_config.workspace_declared {
            // §FS-workspace.6: nested workspaces are rejected at load — bail
            // out of the section silently and let `grund check` report it.
            return None;
        }
        let alias = derive_alias(&member_config, Some(member_root), RootMode::Member).ok()?;
        projects.push(InitWorkspaceProject {
            alias,
            project_root: member_root.clone(),
            description: member_config.project_description.clone(),
        });
    }
    if projects.is_empty() {
        return None;
    }
    let mut seen = BTreeMap::new();
    for project in &projects {
        if seen
            .insert(project.alias.clone(), project.project_root.clone())
            .is_some()
        {
            // §FS-init.2.3.4.15: duplicate aliases make the guidance
            // ambiguous, so suppress the section and leave the diagnostic to
            // `grund check`, just as other workspace config errors do.
            return None;
        }
    }
    projects.sort_by(|a, b| a.alias.cmp(&b.alias));
    Some(projects)
}

/// Walk up from `target` for the nearest ancestor (or `target` itself) whose
/// `.agents/grund.toml` declares `[workspace]`. Unlike [`load_config`], this
/// helper does **not** stop at the first config it finds — a member with its
/// own `.agents/grund.toml` (which cannot declare `[workspace]` per
/// §FS-workspace.6) must still see the workspace root above it
/// (§FS-init.2.3.4.15).
fn find_init_workspace_root(target: &Path) -> Option<Config> {
    // Without a canonical anchor we cannot reliably compare against the
    // canonicalized member roots `expand_workspace_members` returns; bail
    // out so the section is suppressed (§FS-init.2.3.4.15).
    let canonical_target = fs::canonicalize(target).ok()?;
    let mut cursor: Option<&Path> = Some(&canonical_target);
    while let Some(dir) = cursor {
        let candidate = dir.join(".agents").join("grund.toml");
        if candidate.is_file()
            && let Ok(config) = load_config_at(dir, &canonical_target)
            && config.workspace_declared
        {
            return Some(config);
        }
        cursor = dir.parent();
    }
    None
}

/// Render the §FS-init.2.3.4.15 Workspace Members section, or the empty string
/// when `target` is not inside a workspace. The leading `\n\n` is the
/// separator from the preceding namespace guidance block, so an empty value
/// leaves the surrounding spacing unchanged.
fn render_workspace_members_section(
    target: &Path,
    pending_project_name: Option<&str>,
    pending_project_description: Option<&str>,
    citation_marker: &str,
    canonical_agent_entrypoint_selected: bool,
) -> String {
    let Some(projects) = find_init_workspace_context(
        target,
        pending_project_name,
        pending_project_description,
    ) else {
        return String::new();
    };
    // `find_init_workspace_context` already required `target` to canonicalize
    // before it returned `Some`, so this call cannot fail in practice; bail
    // out instead of falling back to a non-canonical path that would break
    // the `is_self` comparison below.
    let Ok(target_canonical) = fs::canonicalize(target) else {
        return String::new();
    };
    let mut bullets = Vec::with_capacity(projects.len());
    for project in &projects {
        let is_self = project.project_root == target_canonical;
        let agents_md_path = project.project_root.join("AGENTS.md");
        // §FS-init.2.3.4.15 self exception: the self project counts as initialized
        // before the write completes only when this init run is actually writing
        // the canonical AGENTS.md. Companion-only init must not link to a missing
        // AGENTS.md.
        let initialized =
            agents_md_path.exists() || (is_self && canonical_agent_entrypoint_selected);
        let link = if initialized {
            relative_link_path(&target_canonical, &agents_md_path)
        } else {
            let dir_rel = relative_link_path(&target_canonical, &project.project_root);
            if dir_rel == "." {
                "./".to_string()
            } else {
                format!("{dir_rel}/")
            }
        };
        let suffix = if initialized { "" } else { " *(not yet initialized)*" };
        // §FS-init.2.3.4.15: the alias is the link label so the path appears
        // once, mirroring the Project Map's `- [x](y): …` shape; the one-line
        // description follows `: `, before the trailing marker.
        let description = project
            .description
            .as_deref()
            .map(|description| format!(": {description}"))
            .unwrap_or_default();
        bullets.push(format!(
            "- [`{alias}`]({dest}){description}{suffix}",
            alias = project.alias,
            dest = markdown_link_destination(&link),
        ));
    }
    let mut out = format!(
        "\n\n### Workspace members\n\nCross-project citations use {citation_marker}alias/<ID>.\n\n",
    );
    out.push_str(&bullets.join("\n"));
    out
}

/// Compute a relative POSIX-style path from `from_dir` to `to`. Both inputs
/// must be absolute (canonicalized) paths. Used to render workspace member
/// links from inside the AGENTS.md being written (§FS-init.2.3.4.15); Markdown
/// links are always forward-slash regardless of platform.
fn relative_link_path(from_dir: &Path, to: &Path) -> String {
    let from = normalize_path_lexically(from_dir);
    let to = normalize_path_lexically(to);
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<String> = Vec::new();
    for _ in &from_components[common..] {
        parts.push("..".to_string());
    }
    for component in &to_components[common..] {
        parts.push(component.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
