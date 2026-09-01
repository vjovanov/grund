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
const CITATION_DIRECTIONS_URL: &str =
    "https://github.com/vjovanov/grund/blob/main/docs/user-facing/citation-directions.md";
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
///
/// Why the worked citation example is escaped: a live marker would make the
/// generated block fail the host repo's own `grund check` as a dangling
/// reference.
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
        (
            "{WORKSPACE_MEMBERS}",
            render_workspace_members_section(
                target,
                Some(name),
                // The pending effective config carries the `--description`
                // value when `init` is about to write a fresh config; with an
                // existing config the walk-up reloads it (§FS-init.2.3.4.15).
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

/// Render the `### Citation directions` managed-block section (§FS-init.2.3.5)
/// from the effective `[citations]` rules. Deterministic — `[[kinds]]` order,
/// `code` last, fixed level phrases, `|`→"or", conjunction→"and" — so
/// `grund check` can re-render and byte-compare it for drift (§FS-check.3.5).
/// When no `[citations]` section is declared, the static climbing-rule sentence
/// stands in, so a config that predates the feature keeps a stable block
/// (§FS-init.2.3.4.10).
///
/// Why a homed non-citable kind is named by its place: naming the kind would
/// name something an agent can never write, while the place reads as the
/// instruction the row is. The homeless kind has no place, so it keeps its name
/// and says what it covers — its `title` where the project wrote one, the fixed
/// phrase otherwise.
fn citation_directions_section(config: &Config) -> String {
    // Built as lines joined with `\n` and returned without a trailing newline:
    // the `{CITATION_DIRECTIONS}` placeholder in the template supplies the single
    // block-final newline, so `grund init` stays idempotent on re-run.
    let mut lines = vec!["### Citation directions".to_string(), String::new()];
    if !config.citations.declared {
        lines.push(
            format!(
                "Specs cite goals, architecture cites specs, code and executable tests cite the specs they realize. In a citation rule array, entries are all required; `|` inside one entry means any one alternative. See {CITATION_DIRECTIONS_URL} for the levels and examples."
            ),
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
    // `[[kinds]]` order, then the homeless kind last (§FS-init.2.3.5) — wherever
    // in the table a project happened to declare it, because it is the
    // complement of every row above it and reads as the closing case.
    let homeless = config.homeless_kind();
    let mut kinds: Vec<String> = config
        .kinds
        .iter()
        .map(|k| k.kind.clone())
        .filter(|kind| kind != homeless)
        .collect();
    if config.citations.per_kind.contains_key(homeless) {
        kinds.push(homeless.to_string());
    }
    for kind in &kinds {
        let Some(rules) = config.citations.per_kind.get(kind) else {
            continue;
        };
        let Some(clauses) = citation_direction_clauses(rules) else {
            continue;
        };
        // §FS-init.2.3.5: a homed non-citable kind is named by its place, so
        // the row reads as the instruction it is — "files in this directory
        // cite X". The homeless kind keeps its name.
        let label = if kind == homeless {
            let scope = config
                .kinds
                .iter()
                .find(|configured| configured.kind == *kind)
                .and_then(|configured| configured.title.as_deref())
                .unwrap_or("any file outside a kind home");
            format!("**{kind}** ({scope})")
        } else {
            format!("**{}**", citing_side_label(config, kind))
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
