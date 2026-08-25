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
const DF_README_TEMPLATE: &str =
    include_str!("../assets/templates/decisions-functional-README.md");
const DA_README_TEMPLATE: &str =
    include_str!("../assets/templates/decisions-architectural-README.md");
const GITKEEP_TEMPLATE: &str = include_str!("../assets/templates/gitkeep.md");
pub const AGENT_SETUP_INSTRUCTIONS: &str = include_str!("../assets/skills/grund-init/SKILL.md");
// v5 (§FS-init.2.3.6, §DF-integrations-command, §DF-repo-conversation-opinion):
// the block gains the `### Clickable citations` section — the fixed
// repository-web convention, plus a config-derived local-conversation sentence
// when `[reference] conversation = "link"` is set. v4 (§FS-init.2.3,
// §DF-managed-block-delimiters): explicit `<!-- BEGIN/END GRUND MANAGED BLOCK -->`
// delimiters replace the implicit H2-to-next-heading region, and the worked
// citation example is `<§>`-escaped so generated output passes `grund check`
// unmodified. v3 (§FS-init.2.3.5, §DF-citation-directions) replaced the
// hand-written climbing-rule bullet with a generated `### Citation directions`
// section derived from `[citations]`.
// v6 (§FS-init.2.3.6, §DF-conversation-link-target): the local-conversation
// sentence became the gated link form — a Markdown link over the `file` target
// on the Claude entrypoints, the plain location everywhere else.
// v7 (§FS-config.1, §DF-config-file-location.2.3): the namespace rule tells an
// agent to give a new subproject a bare `grund.toml` rather than
// `.agents/grund.toml`. That is the taught workflow changing — an agent
// following a v6 block creates a config in the form `init` no longer
// generates — so it carries a version bump rather than a silent rewrite
// (§FS-init.2.3).
const AGENTS_BLOCK_VERSION: u32 = 7;

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
    surface: ConversationSurface,
) -> Vec<(&'static str, String)> {
    let sep = config.section_separator.as_str();
    let marker = config.marker.as_str();
    let id_shape = id_shape(&config.id_format);
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
            "Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `grund.toml`, so only `{marker}`-prefixed citations are checked."
        )
    } else {
        format!(
            "Bare ID-shaped tokens are also recognized as citations because `[reference] strict = false` is set in `grund.toml`; remove that compatibility override or set strict back to `true` to require the `{marker}` marker (run `grund fmt --marker` first to upgrade existing bare citations)."
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
        (
            "{CLICKABLE_CITATIONS}",
            clickable_citations_section(config, surface),
        ),
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
    let budgets = if config.inline_note_suggested_lines == config.inline_note_max_lines {
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
    };
    // §FS-inline-citation-style.5: the layout sentence appends to the budgets and
    // is empty under `any`, so a project that configures no layout renders the
    // byte-identical block it rendered before this key existed.
    format!("{budgets}{}", inline_note_layout_sentence(config))
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

/// Render the `### Clickable citations` section (§FS-init.2.3.6): the fixed
/// repository-web convention always, plus the config-derived local-conversation
/// sentence when the repo commits the `link` opinion (§FS-init.2.3.4.17,
/// §DF-repo-conversation-opinion). Without the opinion, local conversation
/// rendering belongs to user-level instructions installed by
/// `grund integrations --write` (§FS-integrations.4.3).
pub(crate) fn clickable_citations_section(config: &Config, surface: ConversationSurface) -> String {
    // The wording is fixed; the marker is the repository's own (§FS-init.2.3.6).
    // Interpolated here rather than left as a `{MARKER}` placeholder because this
    // section is spliced into the template *after* that placeholder is expanded,
    // so a placeholder in this string would survive into the written block.
    let marker = config.marker.as_str();
    let mut section = format!(
        "### Clickable citations\n\nOn repository web surfaces, link `{marker}<ID>` to the PR branch in PR bodies, the reviewed commit in reviews, an exact commit for permalinks, and the default branch otherwise; fall back to plain when unsure."
    );
    if config.conversation.as_deref() == Some("link") {
        // §DF-conversation-link-target: the committed form is always the `file`
        // target, composed at write time from the repository root the agent
        // already holds — it embeds nothing about any machine, so two installs
        // render byte-identical files (§FS-non-goals.13). Which of the two forms
        // is rendered is the per-agent gate (§DF-conversation-link-target.2.4):
        // instructing the link form to a renderer that shows the destination in
        // place of the label would erase the citation itself.
        //
        // The deference clause is the §DF-repo-conversation-opinion.2.3
        // precedence: this committed opinion is the no-knowledge fallback, and a
        // machine whose user-level block states a rendering knows something
        // about its own surface that the repository cannot — its choice wins.
        let local = match surface {
            ConversationSurface::Linked => format!(
                " In local conversations, render `{marker}<ID>` as a Markdown link whose visible text is the citation itself and whose target is `file://<absolute path>#L<line>` for its declaration; fall back to the bare citation when unsure."
            ),
            ConversationSurface::Plain => format!(
                " In local conversations, follow `{marker}<ID>` with its declaration location as plain `path:line` text; fall back to the bare citation when unsure."
            ),
        };
        section.push_str(&local);
        section.push_str(
            " If a user-level grund block states a local-conversation rendering, follow that instead: that machine knows what its surface can open.",
        );
    }
    section
}

/// Which local-conversation form one entrypoint file teaches
/// (§FS-init.2.3.4.17). A pure function of the target path, so the generated
/// block stays reproducible (§FS-non-goals.13).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConversationSurface {
    /// The Claude entrypoints, whose renderer is verified to honor an
    /// absolute-URI Markdown link (§DF-neural-link-generation, rows 12–14).
    Linked,
    /// Every other entrypoint: the location travels as plain `path:line` text
    /// until a click-test says more.
    Plain,
}

impl ConversationSurface {
    /// `CLAUDE.md` at the repository root or under `.claude/` — the two paths
    /// the Claude entrypoint family occupies (§FS-init.2.3).
    pub(crate) fn for_entrypoint(path: &Path) -> Self {
        match path.file_name().and_then(|name| name.to_str()) {
            Some("CLAUDE.md") => Self::Linked,
            _ => Self::Plain,
        }
    }
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
    surface: ConversationSurface,
) -> String {
    let mut rendered = canonical_template_text(AGENTS_TEMPLATE);
    for (placeholder, value) in agents_template_substitutions(
        name,
        config,
        target,
        canonical_agent_entrypoint_selected,
        surface,
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
    let block = render_agents_append_block(
        name,
        config,
        target,
        canonical_agent_entrypoint_selected,
        ConversationSurface::Plain,
    );
    render_agents_md_from_block(name, &block)
}

/// Same shape as [`render_agents_md`] but takes a pre-rendered managed block,
/// so `command_init` can render the block once and reuse it as both the full
/// `AGENTS.md` body *and* the append/update payload — the workspace-members
/// walk-up (§FS-init.2.3.4.15) only runs once per `init` invocation.
fn render_agents_md_from_block(name: &str, block: &str) -> String {
    format!("# {name} — agent instructions\n\n{block}")
}

/// The config that `grund init` will leave governing `target`, which the generated
/// `AGENTS.md` must describe (§FS-init.2.3): `target`'s existing config in either
/// discovery form if there is one (§FS-config.1), otherwise the defaults plus the
/// *pending* `project_name` and `project_description` that `init` is about to
/// write into `target/grund.toml` (§FS-init.2.4). The `pending` in the name flags
/// that the returned `Config` may carry values that are not yet on disk —
/// callers must not treat it as reflecting persisted state. We do **not** walk
/// up to an ancestor's config here — `init` always writes a config *in*
/// `target` when one is absent.
///
/// A config that fails to load is an error, not a fallback to defaults
/// (§FS-init.2.3): the block is rendered *from* this config, so silently
/// substituting defaults writes agent instructions that describe a repository
/// the user does not have — an invalid `[reference] conversation`, marker, or
/// kind set would drop the guidance it selects while `init` still reported
/// success. `grund check` rejects the same file with exit `2`.
fn init_pending_effective_config(
    target: &Path,
    name: &str,
    description: Option<&str>,
) -> Result<Config> {
    if config_file_in(target).is_some() {
        load_config(target)
    } else {
        let mut config = Config::default_for(target.to_path_buf());
        config.project_name = Some(name.to_string());
        config.project_description = description.map(str::to_string);
        Ok(config)
    }
}

/// The generated `grund.toml` — every default written out explicitly as a
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

/// Walk up from `target` to the outermost ancestor whose `grund.toml` — either
/// discovery form (§FS-config.1) — declares `[workspace]`, then expand the
/// whole tree and derive each alias the same way `grund check` does
/// (§FS-workspace.2 / §FS-workspace.3 / §FS-workspace.6.1). Returns the
/// alias-sorted project list (every block's root, subject to its own
/// `include_root`, plus every member at every depth) when `target` sits inside
/// a workspace; `None` otherwise. Returns `None` rather than an error on any
/// workspace configuration problem (missing member, duplicate alias, member
/// cycle, …) — init must not fail because a sibling member is misconfigured;
/// the next `grund check` will surface the issue (§FS-init.2.3.4.15).
fn find_init_workspace_context(
    target: &Path,
    pending_project_name: Option<&str>,
    pending_project_description: Option<&str>,
) -> Option<Vec<InitWorkspaceProject>> {
    let mut root_config = find_init_workspace_root(target)?;
    // `expand_workspace_tree` returns canonical project roots, so a
    // non-canonical `target` would never match the self project on path
    // equality and the self-exception in §FS-init.2.3.4.15 would silently
    // misfire. Suppress the section rather than render a wrong self row.
    let target_canonical = fs::canonicalize(target).ok()?;
    let mut projects = Vec::new();
    for entry in expand_workspace_tree(&mut root_config).ok()? {
        let mut alias = entry.alias;
        let mut description = entry.config.project_description.clone();
        if entry.config.root == target_canonical && config_file_in(&entry.config.root).is_none() {
            // §FS-init.2.3.4.15: self is rendered against the config `init`
            // is about to write, so `grund init member --name service
            // --description "…"` teaches the future `service/...` workspace
            // alias and its description immediately.
            if let Some(name) = pending_project_name {
                if !is_valid_project_alias(name) {
                    return None;
                }
                // `--name` renames the project, not the workspace levels above
                // it: only the last segment of the alias path changes
                // (§FS-workspace.6.1).
                alias = match alias.rsplit_once('/') {
                    Some((prefix, _)) => format!("{prefix}/{name}"),
                    None => name.to_string(),
                };
            }
            if let Some(pending) = pending_project_description {
                description = Some(pending.to_string());
            }
        }
        projects.push(InitWorkspaceProject {
            alias,
            project_root: entry.config.root,
            description,
        });
    }
    // The pending `--name` above is applied after expansion, so it can collide
    // with a project `expand_workspace_tree` already accepted; the check below
    // is what catches that case, not a second opinion on the tree itself.
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

/// The outermost workspace whose tree actually contains `target`: start at the
/// config that governs it (§FS-config.1) and climb the *claimed chain* — the
/// same walk `enclosing_alias_prefix` uses, so `init` teaches exactly the alias
/// set a command run here resolves (§FS-init.2.3.4.15, §FS-workspace.6.1).
///
/// Unlike [`load_config`] the walk does not stop at the first config it finds — a
/// member with its own config must still see the workspace root above it. It does
/// stop where the claims stop: an ancestor `[workspace]` that does not list the
/// directory below it describes a different workspace, whose aliases resolve
/// nowhere here and whose members lie outside this repository.
fn find_init_workspace_root(target: &Path) -> Option<Config> {
    // Without a canonical anchor we cannot reliably compare against the
    // canonicalized project roots `expand_workspace_tree` returns; bail
    // out so the section is suppressed (§FS-init.2.3.4.15).
    let canonical_target = fs::canonicalize(target).ok()?;
    let mut cursor: Option<&Path> = Some(&canonical_target);
    let mut config = loop {
        let dir = cursor?;
        if config_file_in(dir).is_some() {
            break load_config_at(dir, &canonical_target).ok()?;
        }
        cursor = dir.parent();
    };
    let mut ancestors = AncestorWorkspaces::for_run_at(&config.root);
    loop {
        match enclosing_workspace_of(&config.root, &canonical_target, &mut ancestors) {
            Ok(Some(parent)) => config = parent,
            Ok(None) => break,
            // A broken block above us is `grund check`'s to report; `init` must
            // not describe a tree it cannot see whole (§FS-init.2.3.4.15).
            Err(_) => return None,
        }
    }
    config.workspace_declared.then_some(config)
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
