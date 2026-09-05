// The scaffold templates `grund init` writes are embedded in the binary; the
// reference copies live under `templates/` in the source tree (§FS-init.2.1).
const AGENTS_TEMPLATE: &str = include_str!("../assets/templates/AGENTS.md");
/// The scaffold config. Its `[citations]` block comment explains the five levels
/// and hands the reader `CITATION_DIRECTIONS_URL` below, where it used to cite
/// §FS-config.3.9 by ID: the file lands verbatim in the adopting repository, so
/// an ID of this one names a document that reader does not have — or an
/// unrelated one of their own (§REQ-shipped-surfaces.1).
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
const CITATION_DIRECTIONS_URL: &str =
    "https://github.com/vjovanov/grund/blob/main/docs/user-facing/citation-directions.md";
/// The setup skill, printed byte-for-byte by `grund agent-setup-instructions`
/// into whatever repository the agent is standing in (§FS-init.5). That is why
/// `skills/` is an unwalked home (§FS-config.3.4.7) and why nothing in the file
/// cites an ID of this repository: it would name a document the reader does not
/// have (§REQ-shipped-surfaces.1), so its links are the public URLs of the pages
/// they used to cite. What it teaches is grounded here instead — the config
/// walkthrough ends on the full-tree scope that reports what `[scan] include`
/// left out (§FS-check.1.3), its `[citations]` section is the canonical
/// citation-directions page verbatim (§FS-config.3.9, kept in sync by the
/// asset-sync check), and its setup step covers the clickable integrations
/// (§FS-integrations), the global agent instruction files `--write`
/// synchronizes (§FS-integrations.4.3), and the committable repository opinion
/// with its per-agent gate (§DF-repo-conversation-opinion,
/// §DF-conversation-link-target.2.4).
pub const AGENT_SETUP_INSTRUCTIONS: &str = include_str!("../assets/skills/grund-init/SKILL.md");
/// v5 (§FS-init.2.3.6, §DF-integrations-command, §DF-repo-conversation-opinion):
/// the block gains the `### Clickable citations` section — the fixed
/// repository-web convention, plus a config-derived local-conversation sentence
/// when `[reference] conversation = "link"` is set. v4 (§FS-init.2.3,
/// §DF-managed-block-delimiters): explicit `<!-- BEGIN/END GRUND MANAGED BLOCK -->`
/// delimiters replace the implicit H2-to-next-heading region, and the worked
/// citation example is `<§>`-escaped so generated output passes `grund check`
/// unmodified. v3 (§FS-init.2.3.5, §DF-citation-directions) replaced the
/// hand-written climbing-rule bullet with a generated `### Citation directions`
/// section derived from `[citations]`.
/// v6 (§FS-init.2.3.6, §DF-conversation-link-target): the local-conversation
/// sentence became the gated link form — a Markdown link over the `file` target
/// on the Claude entrypoints, the plain location everywhere else.
/// v7 (§FS-config.1, §DF-config-file-location.2.3): the namespace rule tells an
/// agent to give a new subproject a bare `grund.toml` rather than
/// `.agents/grund.toml`. That is the taught workflow changing — an agent
/// following a v6 block creates a config in the form `init` no longer
/// generates — so it carries a version bump rather than a silent rewrite
/// (§FS-init.2.3).
/// v8 (§FS-init.2.3.5, §DF-directions-render): the generated
/// `### Citation directions` section is re-rendered exactly — a unit per bullet,
/// a grouped conjunction of alternatives, `*/K` said in words, a closed per-kind
/// default folded into its permission, a legend for what gates, and the
/// grounding sentence `[reference] require_grounding` was never rendering. The
/// rules an agent reads changed, so it carries a bump rather than a silent
/// rewrite (§FS-init.2.3).
const AGENTS_BLOCK_VERSION: u32 = 8;

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
/// `workspace_members` arrives already rendered (§FS-init.2.3.4.15): it is the
/// one substitution that reads the tree rather than the config, it does not vary
/// by surface, and its walk-up must run once per `init` invocation rather than
/// once per entrypoint file — see [`agents_workspace_members_section`]. Which is
/// also why nothing here needs the target directory any more.
///
/// Why the worked citation example is escaped: a live marker would make the
/// generated block fail the host repo's own `grund check` as a dangling
/// reference.
fn agents_template_substitutions(
    name: &str,
    config: &Config,
    workspace_members: &str,
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
    // §FS-init.2.3: the worked example illustrates a non-existent ID, so it is
    // rendered in the `<marker>`-escaped form (§FS-workspace.1).
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
        ("{WORKSPACE_MEMBERS}", workspace_members.to_string()),
    ]
}

/// The `### Workspace members` section one `init` run renders (§FS-init.2.3.4.15),
/// built here rather than inside [`agents_template_substitutions`] so it is built
/// **once**. It does not vary by [`ConversationSurface`], and a run that writes
/// both `AGENTS.md` and a `CLAUDE.md` companion renders two blocks from one
/// invocation — so a per-block build would repeat the walk-up's I/O and, worse,
/// ask every block in the workspace twice whether its members swallowed its scan,
/// against §FS-check.4.8's once per block per run.
///
/// Canonical target identity omits self regardless of whether this run selected
/// the canonical `AGENTS.md` or only a companion.
fn agents_workspace_members_section(
    name: &str,
    config: &Config,
    target: &Path,
    canonical_agent_entrypoint_selected: bool,
) -> String {
    render_workspace_members_section(
        target,
        Some(name),
        // Collect effective pending metadata. The renderer omits self (and its
        // description); the pending name still participates in alias validation
        // (§FS-init.2.3.4.15).
        config.project_description.as_deref(),
        config.marker.as_str(),
        canonical_agent_entrypoint_selected,
    )
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

/// §FS-inline-citation-style.5: the sentence that closes the rendered copy at
/// every `inline_style`, after whatever the other keys produced, so the author
/// and the linter agree on where the shape rules stop
/// (§FS-inline-citation-style.1.1). It moves no managed-block version: it only
/// widens what an author may write, so a block that predates it teaches a
/// narrower rule than the gate enforces — an over-careful comment, never a
/// finding.
const DOC_COMMENT_SENTENCE: &str = " Doc-comments (`///`, `//!`, `/** */`, a docstring, a comment right above a definition) are documentation, not notes: they are never measured, so cite in-sentence there.";

/// §FS-inline-citation-style.5: the sentence that follows the budgets and
/// precedes the layout sentence, under `citation-with-note` only — restating
/// §1's block rule at the point an agent needs it to act on a cap finding. It
/// moves no managed-block version, for the same reason the layout and
/// doc-comment sentences do not (§2.2): a block that predates it teaches the
/// same rule less precisely, an over-careful comment, never a finding.
const BLOCK_SENTENCE: &str =
    " A note is one comment block: a blank line splits it, an empty comment line does not.";

fn inline_citation_style_sentence(config: &Config) -> String {
    if config.inline_style == "citation-only" {
        return format!(
            "Inline citations carry no prose — put rationale in the spec.{DOC_COMMENT_SENTENCE}"
        );
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
    // §FS-inline-citation-style.5: the layout sentence appends to the budgets
    // and the block sentence, empty under `any`, so a project with no layout
    // renders the byte-identical block it rendered before that key existed.
    format!(
        "{budgets}{BLOCK_SENTENCE}{}{DOC_COMMENT_SENTENCE}",
        inline_note_layout_sentence(config)
    )
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

/// Render the `### Clickable citations` section (§FS-init.2.3.6): the fixed
/// repository-web convention always, plus the config-derived local-conversation
/// sentence when the repo commits the `link` opinion (§FS-init.2.3.4.17,
/// §DF-repo-conversation-opinion). Without the opinion, local conversation
/// rendering belongs to user-level instructions installed by
/// `grund integrations --write` (§FS-integrations.4.3).
///
/// Why the marker is interpolated rather than left as a `{MARKER}` placeholder:
/// this section is spliced into the template *after* that placeholder is
/// expanded, so a placeholder in this string would survive into the written
/// block.
///
/// Why the committed local-conversation form is always the `file` target: it is
/// composed at write time from the repository root the agent already holds, and
/// embeds nothing about any machine, so two installs render byte-identical
/// files. The per-agent gate is what picks between the two forms — instructing
/// the link form to a renderer that shows the destination in place of the label
/// would erase the citation itself.
///
/// Why the deference clause is there: it is the §DF-repo-conversation-opinion.2.3
/// precedence. This committed opinion is the no-knowledge fallback, and a
/// machine whose user-level block states a rendering knows something about its
/// own surface that the repository cannot — its choice wins.
pub(crate) fn clickable_citations_section(config: &Config, surface: ConversationSurface) -> String {
    // The wording is fixed; the marker is the repository's own
    // (§FS-init.2.3.6), interpolated here rather than left as a `{MARKER}`
    // placeholder.
    let marker = config.marker.as_str();
    let mut section = format!(
        "### Clickable citations\n\nOn repository web surfaces, link `{marker}<ID>` to the PR branch in PR bodies, the reviewed commit in reviews, an exact commit for permalinks, and the default branch otherwise; fall back to plain when unsure."
    );
    if config.conversation.as_deref() == Some("link") {
        // §DF-conversation-link-target: the committed form is always the `file`
        // target (§FS-non-goals.13); which of the two forms is rendered is the
        // per-agent gate (§DF-conversation-link-target.2.4).
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

/// The Project map rows (§FS-init.2.3.4.4): one per configured kind, linking
/// its home. A **citable** kind is named by its kind name — the prefix an agent
/// will type in a citation. A **non-citable** kind is named by its *place*: it
/// has no ID namespace, so its name is a config handle and the only useful thing
/// to show an agent is the directory to go and read.
///
/// Either way, every kind with a home links to it, and an unwalked kind is one
/// of them: its row is why it is configured.
fn declaration_map(config: &Config) -> String {
    // §FS-init.2.3.4.4: the homeless kind gets no row. Every row is a link to a
    // place, and it is the one kind that is not a place — the complement of all
    // of them. Its citation directions still render (§FS-init.2.3.5).
    let homeless = config.homeless_kind();
    let rows = config.kinds.iter().filter(|kind| kind.kind != homeless).map(|kind| {
        let title = kind.title.as_deref().unwrap_or("Declaration");
        // A non-citable kind is labelled by its home, which `place_label`
        // already renders. An unwalked kind (§FS-config.3.4.7) is one of them;
        // its missing directions bullet is §2.3.5's.
        match (kind.file.as_deref().or(kind.folder.as_deref()), kind.citable) {
            (Some(home), true) => row(&kind.kind, home, title),
            (Some(home), false) => row(&kind.place_label().unwrap_or_default(), home, title),
            (None, _) => format!(
                "- `{}`: {title} (inline / configured by convention)",
                kind.kind.replace('`', "\\`")
            ),
        }
    });
    rows.collect::<Vec<_>>().join("\n")
}

/// One Project map row: a Markdown link from `label` to `home`, then the title.
fn row(label: &str, home: &str, title: &str) -> String {
    format!(
        "- [{}]({}): {title}",
        markdown_link_label(label),
        markdown_link_destination(home)
    )
}

/// The managed block — just the H2 section that `init` appends to, or replaces
/// inside, an existing `AGENTS.md` (§FS-init.2.3). The template *is* the block;
/// the H2 line carrying the version is its own begin marker (§FS-init.2.3.1).
/// `workspace_members` is the §FS-init.2.3.4.15 section, rendered once per `init`
/// invocation by [`agents_workspace_members_section`] against the directory being
/// initialized and handed to every surface that block is written to.
fn render_agents_append_block(
    name: &str,
    config: &Config,
    workspace_members: &str,
    surface: ConversationSurface,
) -> String {
    let mut rendered = canonical_template_text(AGENTS_TEMPLATE);
    for (placeholder, value) in
        agents_template_substitutions(name, config, workspace_members, surface)
    {
        rendered = rendered.replace(placeholder, &value);
    }
    rendered
}

/// [`render_agents_append_block`] with the §FS-init.2.3.4.15 section rendered for
/// it — the shape `command_init` had before the section was hoisted out of the
/// substitutions, kept for the tests that render one block from a target and have
/// no second surface for the walk-up to be repeated by.
#[cfg(test)]
fn render_agents_append_block_at(
    name: &str,
    config: &Config,
    target: &Path,
    canonical_agent_entrypoint_selected: bool,
    surface: ConversationSurface,
) -> String {
    let workspace_members =
        agents_workspace_members_section(name, config, target, canonical_agent_entrypoint_selected);
    render_agents_append_block(name, config, &workspace_members, surface)
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
    let block = render_agents_append_block_at(
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
