/// A parsed ID: its kind plus whichever of `{number}` / `{slug}` the configured
/// `[id] format` carries (§FS-config.3.2).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Id {
    pub kind: String,
    pub num: Option<u32>,
    pub slug: Option<String>,
}

// `Id` is rendered for output via `render_id` / `format_id`, which honour the
// repo's `[id] format` and `--width` (§FS-config.3.2). There is deliberately no
// `Display` impl — a bare `{}` would have to guess the format and would be wrong
// on any repo that configured a non-default one.

/// One declaration site discovered by the scanner: a `# <ID>: …` heading in a
/// Markdown file or an inline declaration in a code doc-comment
/// (§AR-scanner.2.1, §AR-scanner.4), with its section body map
/// (§AR-scanner.2.2) and, for stub headings, the inline-home path it points at
/// (§FS-show.2.3, §FS-check.3.4).
#[derive(Debug)]
pub struct Declaration {
    pub id: Id,
    pub file: PathBuf,
    pub line: usize,
    pub heading_level: usize,
    pub sections: BTreeMap<String, SectionInfo>,
    pub is_stub: bool,
    pub defined_in: Option<PathBuf>,
    pub e2e_case: Option<E2eCase>,
    /// Heading text after `<ID>:` — the one-line title an author wrote
    /// (§AR-scanner.2.1). `None` when the heading carries no `: <text>` tail, or
    /// when the heading is a stub link (`# <ID>: [<text>](<path>)`), whose tail
    /// is a path, not a title.
    pub title: Option<String>,
    /// The declaration's body line span (1-indexed, inclusive), §AR-scanner.2.4:
    /// in Markdown it runs from the declaration heading to the line before the
    /// next same-or-higher heading (or end of file); in a source file it is
    /// bounded by the comment/docstring block, capped before the next
    /// declaration in a multi-ID block. A stub heading and an `E2E` case span
    /// their single declaration line only. Used to classify a citation's citing
    /// side (its `enclosing_declaration`) and shared with `grund cover` /
    /// §RM-gap-report.
    pub body_start: usize,
    pub body_end: usize,
}

/// One numbered subsection heading recorded inside a declaration
/// (§AR-scanner.2.2): the heading text used for anchors, plus the source line and
/// Markdown heading level used by the strict section-depth checker
/// (§FS-check.3.9).
#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub title: String,
    pub line: usize,
    pub heading_level: usize,
}

/// An `e2e/cases/<name>/` directory treated as an `E2E-<name>` declaration
/// (§AR-scanner.6) — its `command.args`, `expected.exit`, and fixture file list
/// are what `grund E2E-<name>` renders (§FS-show.2.4).
#[derive(Debug)]
pub struct E2eCase {
    pub dir: PathBuf,
    pub args: Vec<String>,
    pub expected_exit: i32,
    pub fixtures: Vec<PathBuf>,
    pub spec_refs: Vec<E2eSpecRef>,
}

/// A non-empty `spec.refs` manifest line from an E2E case (§AR-scanner.6).
/// It is evidence for E2E citation-direction obligations (§FS-config.3.9), not a
/// normal citation site, so it does not produce dangling-ref findings: an E2E
/// case grounds in the *layer* a `spec.refs` entry names, and entries are
/// deliberately allowed to use idealized, not-locally-resolvable IDs, so only
/// `kind` (plus `namespace`) is retained.
#[derive(Debug)]
pub struct E2eSpecRef {
    pub namespace: Option<String>,
    pub kind: String,
}

/// One citation site: an `<ID>[.<section>]` token, optionally `§`-prefixed
/// (§AR-scanner.2.3, §FS-check.1.1). `has_marker` drives strict-mode filtering
/// (§FS-config.3.1) and is what `grund fmt` upgrades a bare token from (§FS-fmt.2.2).
#[derive(Debug)]
pub struct Citation {
    pub namespace: Option<String>,
    pub id: Id,
    pub section: Option<String>,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub has_marker: bool,
    /// Written in the number-only shorthand (§FS-check.1.2). The scanner's
    /// resolution pass has already rewritten `id` to the canonical declaration
    /// when exactly one matched (§AR-scanner.2.6), so this flag is what
    /// distinguishes a resolved shorthand from a full citation — and the only
    /// thing that has to: every graph consumer deliberately ignores it and reads
    /// `id`. When `id.slug` is still `None` the shorthand resolved to zero or
    /// several declarations.
    pub shorthand: bool,
    /// Whether `grund fmt` is allowed to canonicalize this shorthand in place —
    /// `false` inside inline code, a Markdown link destination, or a runtime
    /// string literal, the contexts §FS-fmt.2.3 forbids every rewrite from
    /// touching. The site is still a citation in every other sense; the flag only
    /// withholds the §FS-check.3.13 error that names `grund fmt --write` as its
    /// fix, so `check` never demands an edit the formatter refuses to make.
    /// Always `true` when `shorthand` is `false`.
    pub shorthand_rewritable: bool,
    pub text: String,
    pub inline_site: Option<InlineCitationSite>,
    /// The resolved *citing* kind for this site (§AR-scanner.2.4): the kind of
    /// the enclosing declaration, else the file's unique kind home, else the
    /// reserved lowercase `code` pseudo-kind. Drives the citation-direction
    /// checks (§FS-config.3.9, §AR-checker.2.9, §AR-checker.2.10).
    pub source_kind: String,
    /// The nearest preceding declaration whose body range contains this site
    /// (§AR-scanner.2.4), or `None` when the site sits in no declaration body.
    /// Lets the obligation pass ask "does this declaration cite the target?" as
    /// a lookup rather than a re-scan.
    pub enclosing_declaration: Option<Id>,
}

/// The enclosing source-comment citation site for one citation
/// (§FS-inline-citation-style.1, §FS-inline-citation-style.2.3). Markdown
/// citations and citations outside recognized comment blocks carry `None`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct InlineCitationSite {
    pub first_line: usize,
    pub last_line: usize,
    pub max_columns: usize,
    pub has_note: bool,
    /// The site's citation-carrying lines that deviate from
    /// `[reference] inline_note_layout` (§FS-inline-citation-style.3.3), 1-based
    /// and ascending. Always empty — and never computed — under the default
    /// `inline_note_layout = "any"`, under `inline_style = "citation-only"`,
    /// where no note is permitted and so none has a layout, and at
    /// `inline_note_layout_check = "off"`, where the verdicts would reach no
    /// channel (§FS-inline-citation-style.4.4). So the field costs a project only
    /// what it asked for: until it configures a layout and gates it, no line is
    /// tokenized or classified on its account (§AR-scanner.3).
    pub layout_violations: Vec<usize>,
}

#[derive(Clone)]
struct WorkspaceCitationTarget {
    alias: String,
    config: Config,
}

type TextOverlays = BTreeMap<PathBuf, String>;

/// Everything the scanner found in one tree walk — declarations grouped by ID
/// (so duplicates surface, §FS-check.3.3) and citations in encounter order. This
/// is the scanner's whole output; the checker (§AR-checker) consumes it without
/// re-reading files.
#[derive(Default)]
pub struct Findings {
    pub declarations: BTreeMap<Id, Vec<Declaration>>,
    pub citations: Vec<Citation>,
    /// Every file the walk read successfully (§AR-scanner.1) — the universe the
    /// `[reference] require_grounding` check iterates over (§FS-check.3.6,
    /// §DF-require-grounding). Files that failed to read are not here; they are in
    /// the walk's `ScanError` list instead.
    pub scanned_files: Vec<PathBuf>,
    /// `<§>`-escaped citation illustrations (§AR-scanner.2.5): the schematic
    /// `<§>[alias/]ID[.section]` shape the detection passes deliberately skip
    /// because the literal `<§>` does not end with the marker. Inert to every
    /// existing check; recorded only so the checker can flag one whose ID
    /// resolves to a real declaration — a likely bracketed live citation rather
    /// than an intended illustration (§FS-check.2.3.1, §AR-checker.2.11).
    pub escaped_citations: Vec<Citation>,
}

/// ID-query slice mode (§FS-show.1): each rung adds to the previous one —
/// `--brief` is heading + first paragraph; `Default` adds the rest of the lead
/// (cut at the first child section); `Toc` adds the nested section map; `Full`
/// adds every subsection body. `Outline` is an internal-only mode used by `Toc`
/// to collect the section map; the CLI does not expose it.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ShowRenderMode {
    Brief,
    Default,
    Toc,
    Full,
    Outline,
}

pub struct ShowSection {
    pub path: String,
    pub title: String,
    pub depth: usize,
}

/// One `[[kinds]]` entry: prefix plus the folder its declarations live in and the
/// human title `grund id` prints (§FS-config.3.4). When `file` is set, every
/// declaration of this kind must live in that exact file — a *single-file kind*,
/// used by `GRUND`/`GOAL`/`RM` whose IDs all live in one document
/// (`docs/grund.md`, `docs/goals.md`, `docs/roadmap.md`).
#[derive(Clone)]
pub struct KindConfig {
    pub prefix: String,
    pub folder: Option<String>,
    pub file: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone)]
pub struct ConfigLocation {
    pub path: PathBuf,
    pub line: usize,
}

/// One RFC-2119 level a `[citations]` rule entry can carry (§FS-config.3.9.1,
/// §DF-citation-directions.2.1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CitationLevel {
    Must,
    Should,
    May,
    ShouldNot,
    MustNot,
}

/// How a rule entry's namespace qualifier matches a citation's namespace
/// (§FS-config.3.9.3): bare `KIND` is local-only, `alias/KIND` pins one member,
/// `*/KIND` matches any namespace — rule grammar only, never a citation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NamespaceMatch {
    Local,
    Alias(String),
    Any,
}

/// One cited target in a rule entry: a namespace qualifier plus a kind prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationTarget {
    pub namespace: NamespaceMatch,
    pub kind: String,
}

/// One `[citations]` array entry — a disjunction of targets joined by `|`
/// (§FS-config.3.9.1). Satisfied by a citation matching any one target.
#[derive(Clone, Debug)]
pub struct CitationDisjunction {
    pub targets: Vec<CitationTarget>,
}

/// The direction rules for one citing kind (§FS-config.3.9). `must` / `should`
/// are obligations checked per declaration; `should_not` / `must_not` are
/// prohibitions checked per citation site; `may` is an explicit permission that
/// punches a hole in a stricter `default`.
#[derive(Clone, Debug, Default)]
pub struct KindCitationRules {
    pub default: Option<CitationLevel>,
    pub must: Vec<CitationDisjunction>,
    pub should: Vec<CitationDisjunction>,
    pub may: Vec<CitationDisjunction>,
    pub should_not: Vec<CitationDisjunction>,
    pub must_not: Vec<CitationDisjunction>,
}

/// The parsed `[citations]` section (§FS-config.3.9): the global default level
/// and the per-citing-kind rule tables. `declared` records whether the section
/// was present at all — absent means no direction checks run.
#[derive(Clone, Debug, Default)]
pub struct CitationRules {
    pub declared: bool,
    pub global_default: Option<CitationLevel>,
    pub per_kind: BTreeMap<String, KindCitationRules>,
}

/// The effective configuration: every `grund.toml` key (§FS-config.3) merged
/// over the built-in defaults (§FS-config.2), plus the compiled `Grammar` and the
/// `root` / `cli_base` paths the walk and the report use.
#[derive(Clone)]
pub struct Config {
    pub root: PathBuf,
    /// The resolved path argument (or cwd) — the base for reports when
    /// `[output] relative_paths = false`, i.e. the base `grund` would use if no
    /// config were discovered (§FS-config.3.6).
    pub cli_base: PathBuf,
    /// The config file that was actually read — either `.agents/grund.toml` or
    /// the bare `grund.toml` (§FS-config.1). `None` in a zero-config tree, where
    /// the defaults come from no file at all.
    ///
    /// A **report path**, not a filesystem handle: relative to `root` for a
    /// standalone project, but to the *workspace* root for a member loaded as
    /// part of one (§FS-errors.4 — a workspace report names members from the
    /// root the run was launched at, so `packages/beta/grund.toml` reads the
    /// same in the diagnostic as it does in `[workspace] members`). Join it
    /// against the base the report uses, never unconditionally against `root`.
    pub config_file: Option<PathBuf>,
    /// The config file at `root` that the discovered one outranks (§FS-config.1.1).
    /// `Some` only for the redundant pair `check` and `config` warn about
    /// (§FS-check.4.3); read by nothing else. Same report-path base as
    /// [`Config::config_file`].
    pub redundant_config_file: Option<PathBuf>,
    pub project_name: Option<String>,
    pub project_name_source: Option<ConfigLocation>,
    /// Optional one-line description rendered beside the project's alias in
    /// generated workspace member lists (§FS-config.3, §FS-workspace.3,
    /// §DF-workspace-member-descriptions). Presentation metadata only.
    pub project_description: Option<String>,
    pub marker: String,
    pub trigger: String,
    pub strict: bool,
    /// `[reference] require_grounding` (§FS-config.3.1, §FS-check.3.6,
    /// §DF-require-grounding) — when true, `check` also reports every scanned
    /// source file that carries no resolving citation (and declares no ID inline).
    /// `--require-grounding` on `grund check` forces it on for one run.
    pub require_grounding: bool,
    /// `[reference] conversation` (§FS-config.3.1, §DF-repo-conversation-opinion) —
    /// the repository's committed conversation-rendering opinion. `None` means no
    /// opinion; the only accepted value is `"link"` (closed enum, widenable later).
    /// Read solely by the agent-entrypoint renderer (§FS-init.2.3.4.17).
    pub conversation: Option<String>,
    pub inline_style: String,
    pub inline_note_suggested_lines: usize,
    pub inline_note_max_lines: usize,
    pub inline_note_max_columns: usize,
    /// `[reference] inline_note_layout` (§FS-config.3.1,
    /// §FS-inline-citation-style.3.3, §DF-inline-note-layout) — the project's
    /// house style for where citations sit inside an inline note. Closed enum:
    /// `any` (default, no constraint) or `citation-first-colon`.
    pub inline_note_layout: String,
    /// `[reference] inline_note_layout_check` (§FS-inline-citation-style.4.4) —
    /// whether `check` reports a layout deviation, and through which channel.
    /// Closed enum: `off` (default), `warn`, or `error`. Inert under
    /// `inline_note_layout = "any"`, which is why it is a second key
    /// (§DF-inline-note-layout.2.1).
    pub inline_note_layout_check: String,
    pub warn_on_suggested: bool,
    pub include: Option<Vec<String>>,
    /// §FS-check.1.3: `grund check --full` for this run — walk the whole config
    /// root and ignore `include`. Not a `grund.toml` key and never read from one
    /// (§DF-check-full-scope.2.5): a project that wants its whole tree governed
    /// widens `include`, and a second knob describing one scope is how two
    /// installs come to disagree (§FS-non-goals.13).
    pub scan_full: bool,
    pub exclude: Vec<String>,
    pub extensions: Vec<String>,
    pub comment_prefixes: Vec<String>,
    pub docstring_python: bool,
    pub respect_gitignore: bool,
    pub output_format: String,
    pub relative_paths: bool,
    pub id_format: String,
    pub section_separator: String,
    pub number_pattern: String,
    pub slug_pattern: String,
    pub section_heading_levels: String,
    pub kinds: Vec<KindConfig>,
    pub fmt_cross_refs_enabled: bool,
    pub cross_ref_anchor_format: String,
    pub workspace_declared: bool,
    pub workspace_members: Vec<String>,
    pub workspace_members_source: Option<ConfigLocation>,
    pub workspace_include_root: bool,
    pub workspace_boundary_roots: Vec<PathBuf>,
    /// Parsed `[citations]` direction rules (§FS-config.3.9). Empty/absent unless
    /// the config declares the section.
    pub citations: CitationRules,
    /// Whether the scan computes citing-side classification — declaration body
    /// ranges and each citation's `source_kind` (§AR-scanner.2.4). Only the
    /// citation-direction checks read it, so `grund check` leaves it on while the
    /// read-only commands (`list`, `show`, `refs`, `cover`, `fmt`) turn it off to
    /// skip the post-pass entirely (§AR-benchmarks). Combined with
    /// `citations.declared`, a project without direction rules pays nothing.
    pub classify_citation_sources: bool,
    pub grammar: Grammar,
}

const DEFAULT_KINDS: &[&str] = &["GRUND", "GOAL", "FS", "AR", "DF", "DA", "E2E", "RM"];
/// The reserved lowercase citing kind for a citation site outside every
/// configured kind home (§AR-scanner.2.4, §FS-config.3.9.2). Lowercase so it can
/// never collide with a real `[[kinds]]` prefix, which must be uppercase-shaped.
const CODE_SOURCE_KIND: &str = "code";
const DEFAULT_ID_FORMAT: &str = "{kind}-{number}-{slug}";
const DEFAULT_SECTION_SEPARATOR: &str = ".";
const DEFAULT_NUMBER_PATTERN: &str = r"\d+";
const DEFAULT_SLUG_PATTERN: &str = r"[a-z0-9][a-z0-9-]*";

impl Config {
    /// The built-in defaults — the canonical grammar a conformant tree gets with
    /// no config at all (§FS-config.2, §GOAL-zero-config). `grund init`
    /// writes these same values out verbatim as a teaching surface (§FS-init.2.4).
    fn default_for(root: PathBuf) -> Self {
        let kinds: Vec<KindConfig> = DEFAULT_KINDS
            .iter()
            .map(|prefix| KindConfig {
                prefix: prefix.to_string(),
                folder: default_kind_folder(prefix).map(str::to_string),
                file: default_kind_file(prefix).map(str::to_string),
                title: default_kind_title(prefix).map(str::to_string),
            })
            .collect();
        let kind_prefixes = kind_prefixes(&kinds);
        let grammar = Grammar::build(
            DEFAULT_ID_FORMAT,
            &kind_prefixes,
            DEFAULT_NUMBER_PATTERN,
            DEFAULT_SLUG_PATTERN,
            DEFAULT_SECTION_SEPARATOR,
            &DEFAULT_COMMENT_PREFIXES
                .iter()
                .map(|prefix| prefix.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("default grammar must compile");
        Self {
            cli_base: root.clone(),
            root,
            config_file: None,
            redundant_config_file: None,
            project_name: None,
            project_name_source: None,
            project_description: None,
            marker: "§".to_string(),
            trigger: "$$".to_string(),
            strict: true,
            require_grounding: false,
            conversation: None,
            inline_style: "citation-with-note".into(),
            inline_note_suggested_lines: 1,
            inline_note_max_lines: 3,
            inline_note_max_columns: 100,
            inline_note_layout: "any".into(),
            inline_note_layout_check: "off".into(),
            warn_on_suggested: false,
            include: Some(
                DEFAULT_INCLUDE
                    .iter()
                    .map(|path| path.to_string())
                    .collect(),
            ),
            scan_full: false,
            exclude: vec![
                "target".into(),
                "node_modules".into(),
                ".git".into(),
                "dist".into(),
                "build".into(),
                ".venv".into(),
            ],
            extensions: vec![
                "md".into(),
                "rs".into(),
                "go".into(),
                "java".into(),
                "kt".into(),
                "ts".into(),
                "tsx".into(),
                "js".into(),
                "py".into(),
                "c".into(),
                "cpp".into(),
                "swift".into(),
                "scala".into(),
                "rb".into(),
                "php".into(),
                "cs".into(),
            ],
            comment_prefixes: DEFAULT_COMMENT_PREFIXES
                .iter()
                .map(|prefix| prefix.to_string())
                .collect(),
            docstring_python: true,
            respect_gitignore: true,
            output_format: "text".into(),
            relative_paths: true,
            id_format: DEFAULT_ID_FORMAT.into(),
            section_separator: DEFAULT_SECTION_SEPARATOR.into(),
            number_pattern: DEFAULT_NUMBER_PATTERN.into(),
            slug_pattern: DEFAULT_SLUG_PATTERN.into(),
            section_heading_levels: "strict".into(),
            kinds,
            fmt_cross_refs_enabled: true,
            cross_ref_anchor_format: "github".into(),
            workspace_declared: false,
            workspace_members: Vec::new(),
            workspace_members_source: None,
            workspace_include_root: true,
            workspace_boundary_roots: Vec::new(),
            citations: CitationRules::default(),
            // On by default so `grund check` (and tests) classify; the read-only
            // commands turn it off (§AR-scanner.2.4, §AR-benchmarks).
            classify_citation_sources: true,
            grammar,
        }
    }

    /// Compatibility defaults for an already-authored `grund.toml` that
    /// predates the `requirements.md` generated default and omits `[[kinds]]`.
    /// New zero-config projects and freshly generated configs use
    /// [`Config::default_for`]; existing configs without explicit kind homes keep
    /// the old implicit FS folder until they opt into `file = "requirements.md"`.
    fn default_for_existing_config(root: PathBuf) -> Self {
        let mut config = Self::default_for(root);
        if let Some(fs_kind) = config.kinds.iter_mut().find(|kind| kind.prefix == "FS") {
            fs_kind.folder = Some("docs/functional-spec".to_string());
            fs_kind.file = None;
        }
        config
    }

    /// Recompile the `Grammar` after `[id]` / `[[kinds]]` / `[scan].comment_prefixes`
    /// keys are read from a config file (§FS-config.3) — keeps the regexes and the
    /// scalar config in lockstep.
    fn rebuild_grammar(&mut self) -> Result<()> {
        let prefixes = kind_prefixes(&self.kinds);
        self.grammar = Grammar::build(
            &self.id_format,
            &prefixes,
            &self.number_pattern,
            &self.slug_pattern,
            &self.section_separator,
            &self.comment_prefixes,
        )?;
        Ok(())
    }
}

fn kind_prefixes(kinds: &[KindConfig]) -> Vec<String> {
    kinds.iter().map(|kind| kind.prefix.clone()).collect()
}

/// Default home folder for each built-in kind — the directory `grund id` proposes
/// a path under and `grund check` expects the declaration to live in (§FS-config.3.4).
fn default_kind_folder(prefix: &str) -> Option<&'static str> {
    match prefix {
        "AR" => Some("docs/architecture"),
        "DA" => Some("docs/decisions/architectural"),
        "DF" => Some("docs/decisions/functional"),
        "E2E" => Some("e2e/cases"),
        // GRUND, GOAL, RM are single-file kinds — see `default_kind_file`. A
        // kind can always be broken up later by swapping `file = "…"` for
        // `folder = "…"` and moving the document into the folder.
        _ => None,
    }
}

/// Default single-file home for kinds whose declarations all live in one
/// document — `GRUND` in `docs/grund.md`, `GOAL` in `docs/goals.md`, `FS` in
/// `requirements.md`, and `RM` in `docs/roadmap.md` (§FS-config.3.4). Other
/// built-in kinds have no `file` (each declaration is its own file).
fn default_kind_file(prefix: &str) -> Option<&'static str> {
    match prefix {
        "GRUND" => Some("docs/grund.md"),
        "GOAL" => Some("docs/goals.md"),
        "FS" => Some("requirements.md"),
        "RM" => Some("docs/roadmap.md"),
        _ => None,
    }
}

/// Default human title for each built-in kind, printed by `grund id` (§FS-config.3.4,
/// §FS-id.2).
fn default_kind_title(prefix: &str) -> Option<&'static str> {
    match prefix {
        "GRUND" => Some("Why: project motivation"),
        "GOAL" => Some("Where: project direction and outcomes"),
        "FS" => Some("What: behavior, requirements, and constraints"),
        "AR" => Some("How: high-level implementation, structure, and design"),
        "DA" => Some("Architecture decisions and tradeoffs"),
        "DF" => Some("Product behavior decisions and tradeoffs"),
        "E2E" => Some("Executable user scenarios"),
        "RM" => Some("Planned milestones and sequencing"),
        _ => None,
    }
}

/// A secondary location attached to a diagnostic — e.g. the other declaration in a
/// duplicate pair, or the citation that pointed at a missing section (§FS-errors.2.1).
#[derive(Clone)]
struct Site {
    path: PathBuf,
    line: usize,
}

/// One finding in the located-finding shape of §FS-errors.2.1: a fixed `code`, the
/// `path:line` it occurred at, the message text, and any cross-reference `sites`.
/// `column` is the 1-based start column of the offending token when the finding
/// concerns a specific citation, so a consumer can anchor on that token rather
/// than the first one on the line (§FS-lsp.1.1); it is `None` for line-anchored
/// findings.
struct Diagnostic {
    code: &'static str,
    path: Option<PathBuf>,
    line: Option<usize>,
    column: Option<usize>,
    message: String,
    sites: Vec<Site>,
}

/// The outcome of `check`: errors and warnings, kept apart so the exit code keys
/// off errors only (§FS-check.2, §FS-check.4) and the printed order is fixed
/// (§FS-errors.4, §FS-non-goals.9). `suggestions` is the third, non-severity
/// advisory channel (§FS-check.2.3, §DF-citation-directions.2.3): the
/// `should` / `should-not` citation-direction findings, withheld from the
/// default run and surfaced only under `--suggestions`. It never affects the
/// exit code.
#[derive(Default)]
struct CheckReport {
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
    suggestions: Vec<Diagnostic>,
}

/// What an ID query resolved to: the body text to print, the `path:line` it
/// came from, the section map (`--toc` only), and the pre-rendered JSON when
/// `--format json` was asked for (§FS-show.3, §FS-errors.5).
pub struct ShowOutput {
    pub body: String,
    pub path: PathBuf,
    pub line: usize,
    pub json: Option<String>,
    pub sections: Vec<ShowSection>,
}

fn resolve_stub_target(root: &Path, stub_file: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        return target.to_path_buf();
    }
    let stub_file = if stub_file.is_absolute() {
        stub_file.to_path_buf()
    } else {
        root.join(stub_file)
    };
    let markdown_relative =
        normalize_path_lexically(&stub_file.parent().unwrap_or(root).join(target));
    if markdown_relative.exists() {
        markdown_relative
    } else {
        normalize_path_lexically(&root.join(target))
    }
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

/// Pull an `Id` out of a `Grammar` regex match — the `kind` / `num` / `slug`
/// capture groups the `[id] format` defined (§FS-config.3.2, §AR-scanner.2.1).
fn parse_id(caps: &regex::Captures) -> Option<Id> {
    let kind = caps.name("kind")?.as_str().to_string();
    let num = match caps.name("num") {
        Some(m) => Some(m.as_str().parse().ok()?),
        None => None,
    };
    let slug = caps.name("slug").map(|m| m.as_str().to_string());
    Some(Id { kind, num, slug })
}

/// Parse a CLI `<ID>[.<section>]` argument (the form ID queries and `grund refs` take,
/// §FS-show.1, §FS-refs.1) into an `Id` and an optional section path (§FS-config.3.3).
fn parse_id_arg(raw: &str, grammar: &Grammar) -> Result<(Id, Option<String>)> {
    let caps = grammar
        .id_input_re
        .captures(raw)
        .ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    let id = parse_id(&caps).ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    Ok((id, caps.name("sec").map(|m| m.as_str().to_string())))
}

fn render_qualified_id(config: &Config, namespace: Option<&str>, id: &Id) -> String {
    match namespace {
        Some(namespace) => format!("{}/{}", namespace, render_id(config, id)),
        None => render_id(config, id),
    }
}
