/// A parsed ID: its kind plus whichever of `{number}` / `{slug}` the configured
/// `[id] format` carries (§FS-config.3.2).
///
/// `Id` is rendered for output via `render_id` / `format_id`, which honour the
/// repo's `[id] format` and `--width` (§FS-config.3.2). There is deliberately no
/// `Display` impl — a bare `{}` would have to guess the format and would be wrong
/// on any repo that configured a non-default one.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Id {
    pub kind: String,
    pub num: Option<u32>,
    pub slug: Option<String>,
}

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
    /// Every later heading that claimed a section path `sections` already holds,
    /// in file order, narrowed to the ones inside this declaration's own body
    /// span (§AR-scanner.2.2).
    ///
    /// Nothing *resolves* through it: a `§<ID>.<path>` citation, the completion
    /// candidates, and §FS-check.3.9 all read the map. It exists for the two
    /// commands that have to say the coordinate is ambiguous — §FS-check.3.16
    /// names each colliding line, and §FS-show.2.2.2 refuses a query for a path
    /// it holds. `--toc` reads neither: it re-scans the source, which is what
    /// §FS-show.2.2.2 exempts it for.
    pub duplicate_sections: Vec<(String, SectionInfo)>,
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
    /// Whether this shorthand is glued to a second number — `§SPEC-001→SPEC-003`
    /// — which makes it a numeral in a run rather than a citation, and forbids
    /// `grund fmt` from expanding it (§FS-fmt.2.4.1). Distinct from
    /// `shorthand_rewritable` because the two reach different verdicts: an
    /// illustration in inline code wants no edit and earns no finding, while a
    /// run needs one and earns §FS-check.3.15. Always `false` when `shorthand`
    /// is `false`.
    pub numeric_run: bool,
    pub text: String,
    /// The comment block this citation sits in, when that block is an inline
    /// citation site (§FS-inline-citation-style.1). `None` outside a comment
    /// block, in Markdown, in a block that declares an ID — and in a **doc
    /// comment**: `///`, `//!`, `/** … */`, a docstring, or a comment a position
    /// language puts right above a definition is documentation, not a note, so
    /// it is no site and carries no budget, style, or layout
    /// (§FS-inline-citation-style.1.1).
    pub inline_site: Option<InlineCitationSite>,
    /// The resolved *citing* kind for this site (§AR-scanner.2.4): the kind of
    /// the enclosing declaration, else the file's unique kind home, else the
    /// homeless kind (`code` by default, §FS-config.3.9.2). Drives the citation-direction
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
/// citations and citations outside recognized comment blocks carry `None`, and
/// so does a citation in a **doc comment**: what a language calls documentation
/// is not an inline note, so its block is not a site
/// (§FS-inline-citation-style.1.1). Such a citation still resolves and is still
/// checked for everything else — dangling, direction, grounding, shorthand.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct InlineCitationSite {
    pub first_line: usize,
    pub last_line: usize,
    /// Width of the site's longest line in **characters** — Unicode scalar
    /// values, one column each (§FS-inline-citation-style.2.3). Not the byte
    /// length, and not the display width: `é`, `—`, and the `§` marker itself
    /// cost one column apiece, and so does a tab
    /// (§DF-note-columns-are-characters). This is a different measure from the
    /// byte-addressed start column a `Citation` records (§AR-scanner.3); the
    /// two agree only on a line of pure ASCII.
    pub max_columns: usize,
    pub has_note: bool,
    /// The site's judged lines that deviate from
    /// `[reference] inline_note_layout` (§FS-inline-citation-style.3.3), 1-based
    /// and ascending. Judged is rule 1's set, not every line carrying a citation:
    /// the line that opens the note, and any later line that opens with a
    /// citation of its own.
    ///
    /// This is **what `check` will report**, never an independent survey of the
    /// tree. It is empty — and no line is classified — under the default
    /// `inline_note_layout = "any"`, under `inline_style = "citation-only"`,
    /// where no note is permitted and so none has a layout, and at
    /// `inline_note_layout_check = "off"`, where the verdicts would reach no
    /// channel (§FS-inline-citation-style.4.4). So the field costs a project only
    /// what it asked for: until it configures a layout and gates it, no line is
    /// tokenized or classified on its account (§AR-scanner.3). A consumer that
    /// wants the deviations of a tree whose gate is `off` is asking a different
    /// question and has to gate it, or classify the lines itself — reading an
    /// empty list here is not evidence that the tree conforms.
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
    /// Every directory the walk descended into (§AR-scanner.1), scan roots
    /// included — the candidate set the unlisted-`[workspace]` rule of
    /// §FS-check.4.9 probes. Carried rather than judged here: the walk knows what
    /// it reached, and nothing about workspaces (§AR-workspace.1).
    pub walked_dirs: Vec<PathBuf>,
    /// Per-file heading and doc-comment structure, for the files a grounding
    /// unit finer than the file is asked of (§AR-scanner.2.7, §FS-check.3.6.2).
    /// Empty — and never collected — unless `Config::grounding_units` is set.
    pub file_structure: BTreeMap<PathBuf, FileStructure>,
    /// `<§>`-escaped citation illustrations (§AR-scanner.2.5): the schematic
    /// `<§>[alias/]ID[.section]` shape the detection passes deliberately skip
    /// because the literal `<§>` does not end with the marker. Inert to every
    /// existing check; recorded only so the checker can flag one whose ID
    /// resolves to a real declaration — a likely bracketed live citation rather
    /// than an intended illustration (§FS-check.2.3.1, §AR-checker.2.11).
    pub escaped_citations: Vec<Citation>,
    /// Headings that open like a declaration and do not parse as one
    /// (§FS-check.4.7) — recorded where the scan already decided the line was
    /// not a declaration, so the rule costs one regex on heading-shaped lines
    /// rather than a second read of the tree.
    pub near_miss_headings: Vec<NearMissHeading>,
}

/// One heading that opens with a configured kind and the literal an ID puts
/// after it, without parsing as an ID (§FS-check.4.7). `text` is the token as
/// written, so the finding can quote it back beside the format it missed.
pub struct NearMissHeading {
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
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

/// One `[[kinds]]` entry: the kind name plus the folder its declarations live in
/// and the human title `grund id` prints (§FS-config.3.4). When `file` is set,
/// every declaration of this kind must live in that exact file — a *single-file
/// kind*, used by `GRUND`/`GOAL`/`RM` whose IDs all live in one document
/// (`docs/grund.md`, `docs/goals.md`, `docs/roadmap.md`).
#[derive(Clone)]
pub struct KindConfig {
    /// The `kind` key (§FS-config.3.4) — the name `[citations.<kind>]` keys on,
    /// and, for a citable kind, the literal prefix of every ID in it. Spelled
    /// `prefix` in configs written before that key was renamed.
    pub kind: String,
    pub folder: Option<String>,
    pub file: Option<String>,
    pub title: Option<String>,
    /// The `index` key (§FS-config.3.4): which file under `folder` must list
    /// every declaration in it (§FS-check.4.6). Absent means the `README.md`
    /// default; `false` opts the kind out.
    pub index: KindIndex,
    /// The `citable` key (§FS-config.3.4): whether this kind declares IDs that
    /// can be cited. `false` is a kind that is a *place* and nothing more —
    /// its home is scanned and its citations are directed, but it admits no
    /// declaration and contributes no prefix to the ID grammar.
    pub citable: bool,
    /// The `scan` key (§FS-config.3.4.7): whether this kind's home is a walk
    /// root. `false` is a place that is listed — its Project map row — and not
    /// walked: content that ships verbatim, which nothing here may check.
    pub scan: bool,
    /// The `require_grounding` key (§FS-config.3.4.8): whether the files this
    /// row governs must cite a declared ID. `None` inherits the `[reference]`
    /// default, which `grund check --require-grounding` also sets — the flag and
    /// the key are one knob, so an explicit `Some(false)` wins over both.
    pub require_grounding: Option<bool>,
    /// The `grounding_level` key (§FS-config.3.4.8): the unit inside each
    /// governed file, in Markdown heading levels. `None` inherits the
    /// `[reference]` default; `1` is the file, which is what every config had
    /// before the key existed.
    pub grounding_level: Option<usize>,
}

/// The three states of `[[kinds]] index` (§FS-config.3.4): unset (the
/// `README.md` default), `false`, or a file name relative to `folder`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KindIndex {
    Default,
    Disabled,
    Named(String),
}

/// The index file a kind carries when `index` is left unset (§FS-config.3.4).
const DEFAULT_KIND_INDEX: &str = "README.md";

impl KindConfig {
    /// The `index` value `grund config show` prints for this kind
    /// (§FS-config.4.2): the TOML literal — `false`, or a quoted file name — and
    /// `None` for a kind with no folder or no IDs, neither of which has an index
    /// to speak of.
    pub fn index_toml_value(&self) -> Option<String> {
        self.folder.as_ref()?;
        if !self.citable {
            return None;
        }
        Some(match &self.index {
            KindIndex::Disabled => "false".to_string(),
            KindIndex::Default => format!("\"{DEFAULT_KIND_INDEX}\""),
            KindIndex::Named(name) => format!("\"{}\"", escape_toml_basic(name)),
        })
    }

    /// How this kind is named where a name would be useless (§FS-init.2.3.4.4,
    /// §FS-check.3.11): by its place, with a trailing `/` on a folder so it
    /// reads as the directory it is. `None` for a kind with no home. Used for
    /// non-citable kinds, whose name exists only to key `[citations.*]` on and
    /// is nothing a reader can go and look at.
    pub fn place_label(&self) -> Option<String> {
        if let Some(folder) = &self.folder {
            return Some(format!("{folder}/"));
        }
        self.file.clone()
    }

    /// This kind's index file, relative to the config root — `None` for a kind
    /// with no `folder` or with `index = false` (§FS-config.3.4). Joined onto
    /// `folder`, because the key names a file *in* the folder it indexes, which
    /// is what `kind_index_name_error` holds the value to.
    fn index_path(&self) -> Option<PathBuf> {
        if !self.citable {
            return None;
        }
        let folder = self.folder.as_deref()?;
        let name = match &self.index {
            KindIndex::Disabled => return None,
            KindIndex::Default => DEFAULT_KIND_INDEX,
            KindIndex::Named(name) => name.as_str(),
        };
        Some(Path::new(folder).join(name))
    }
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
    /// Where the deprecated `[[kinds]] prefix` spelling was read, if it was
    /// (§FS-config.3.4, §FS-config.4.1). The first entry that uses it, so the
    /// run carries one deprecation warning per config rather than one per row.
    pub deprecated_kind_prefix: Option<ConfigLocation>,
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
    /// `[reference] grounding_level` (§FS-config.3.4.8, §FS-check.3.6.2) — the
    /// default unit inside each governed file, in Markdown heading levels, for
    /// every `[[kinds]]` row that does not set its own. `1` is the file.
    pub grounding_level: usize,
    /// Whether any row's effective `grounding_level` is finer than the file
    /// (§AR-scanner.2.7). Derived from the two keys once the config is read, so
    /// the scanner records per-file structure only where a row asks for it and a
    /// level-1 tree — which is every config written before the keys existed —
    /// pays nothing (§GOAL-fast-feedback).
    pub grounding_units: bool,
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
    /// `[fmt] exclude` (§FS-config.3.10) — the files `grund fmt` performs no
    /// rewrite in, as gitignore-style globs against the config root. Read by
    /// `fmt` alone: the walk, the scan, and every check are untouched by it
    /// (§FS-fmt.2.5.1). Empty is what every config written before the key
    /// existed means.
    pub fmt_exclude: Vec<String>,
    pub fmt_cross_refs_enabled: bool,
    pub cross_ref_anchor_format: String,
    pub workspace_declared: bool,
    pub workspace_members: Vec<String>,
    pub workspace_members_source: Option<ConfigLocation>,
    /// Where the `[workspace]` table header itself was written (§FS-config.4.3).
    /// The anchor for an error about the *block* rather than about one key — a
    /// block with no `members` key at all still has to say which of a tree's many
    /// blocks it is (§FS-workspace.6.1).
    pub workspace_section_source: Option<ConfigLocation>,
    pub workspace_include_root: bool,
    pub workspace_boundary_roots: Vec<PathBuf>,
    /// §AR-workspace.6: the canonical root of **every** project this run loaded.
    /// `workspace_boundary_roots` above says what lies *below* this project, so a
    /// leaf member has none; this says where the *others* are, which is how a
    /// member's walk tells a link into a sibling — or back up into the root
    /// project — from a link into ordinary outside content. Empty for a run that
    /// loaded no workspace, a member checked on its own included (§FS-workspace.6).
    pub workspace_project_roots: Vec<PathBuf>,
    /// §FS-workspace.6.1: the alias path of the *run's* own workspace root, read
    /// from the outermost workspace and stamped onto every project the run loaded.
    /// Empty at the outermost root and for a single-project run; non-empty exactly
    /// when the run is narrowed to a subtree. Not a `grund.toml` key and never read
    /// from one (like `workspace_boundary_roots`, it is what expansion learned
    /// about this run): §FS-check.3.8 reads it to know that a path it cannot
    /// resolve may still be correct at the workspace root.
    pub workspace_scope_path: String,
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

/// The **default** name of the homeless kind — the citing kind of every site
/// outside every configured home (§AR-scanner.2.4, §FS-config.3.9.2). It is a
/// default and not a fixed name: `code` is the right word for most
/// repositories and the wrong one for a Terraform, SQL, or prose tree, so a
/// project may declare the homeless kind itself and name it (`src`,
/// `modules`, …). See [`Config::homeless_kind`].
const CODE_SOURCE_KIND: &str = "code";
/// The default `grounding_level` (§FS-config.3.4.8): the file — the H1's own
/// subtree, so one citation anywhere under it. It is the unit every config had
/// before the key existed, which is what keeps the key additive.
const DEFAULT_GROUNDING_LEVEL: usize = 1;
/// The heading levels a `grounding_level` may name (§FS-config.3.4.8). Markdown
/// has six, and a value outside them names no heading.
const GROUNDING_LEVELS: std::ops::RangeInclusive<usize> = 1..=6;
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
            .map(|kind| KindConfig {
                kind: kind.to_string(),
                folder: default_kind_folder(kind).map(str::to_string),
                file: default_kind_file(kind).map(str::to_string),
                title: default_kind_title(kind).map(str::to_string),
                index: default_kind_index(kind),
                citable: default_kind_citable(kind),
                scan: true,
                require_grounding: None,
                grounding_level: None,
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
            deprecated_kind_prefix: None,
            project_name: None,
            project_name_source: None,
            project_description: None,
            marker: "§".to_string(),
            trigger: "$$".to_string(),
            strict: true,
            require_grounding: false,
            grounding_level: DEFAULT_GROUNDING_LEVEL,
            grounding_units: false,
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
            extensions: DEFAULT_SCAN_EXTENSIONS
                .iter()
                .map(|extension| extension.to_string())
                .collect(),
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
            fmt_exclude: Vec::new(),
            fmt_cross_refs_enabled: true,
            cross_ref_anchor_format: "github".into(),
            workspace_declared: false,
            workspace_members: Vec::new(),
            workspace_members_source: None,
            workspace_section_source: None,
            workspace_scope_path: String::new(),
            workspace_include_root: true,
            workspace_boundary_roots: Vec::new(),
            workspace_project_roots: Vec::new(),
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
        if let Some(fs_kind) = config.kinds.iter_mut().find(|kind| kind.kind == "FS") {
            fs_kind.folder = Some("docs/functional-spec".to_string());
            fs_kind.file = None;
        }
        config
    }

    /// The homeless kind for this config (§FS-config.3.9.2) — the citing kind
    /// every site outside every configured home resolves to. The declared entry
    /// when the table has one, else the reserved `code`.
    fn homeless_kind(&self) -> &str {
        declared_homeless_kind(&self.kinds).map_or(CODE_SOURCE_KIND, |kind| kind.kind.as_str())
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

/// The ID prefixes a config recognizes (§FS-config.3.4): the name of every
/// *citable* kind, in `[[kinds]]` order. A non-citable kind declares no IDs, so
/// its name never tokenizes and never enters the grammar, the `KIND ∈ {…}`
/// vocabulary, or the kind lists `grund id` and `grund list --kind` accept.
fn kind_prefixes(kinds: &[KindConfig]) -> Vec<String> {
    kinds
        .iter()
        .filter(|kind| kind.citable)
        .map(|kind| kind.kind.clone())
        .collect()
}

/// Why a configured kind cannot be selected with `--kind` or minted from
/// (§FS-list.1, §FS-id.1). A non-citable kind is not a typo — it is a real row
/// in `[[kinds]]` that will never have a declaration — so the message says that
/// rather than calling it unknown, and names the home, which is the thing the
/// caller can actually go and open.
fn non_citable_kind_error(kind: &KindConfig) -> String {
    match kind.place_label() {
        Some(place) => format!(
            "kind `{}` declares no IDs — {place} is not a citable home",
            kind.kind
        ),
        None => format!("kind `{}` declares no IDs", kind.kind),
    }
}

/// Every citing kind `[citations.<kind>]` may name (§FS-config.3.9): each
/// configured kind, citable or not, plus `code` — but only where the table did
/// not declare the homeless kind itself. A config that names its complement
/// `src` has no `code`, and `[citations.code]` in it is a rule about nothing
/// (§FS-config.3.9.2).
fn citing_kind_names(kinds: &[KindConfig]) -> Vec<&str> {
    let named = declared_homeless_kind(kinds).is_some();
    kinds
        .iter()
        .map(|kind| kind.kind.as_str())
        .chain((!named).then_some(CODE_SOURCE_KIND))
        .collect()
}

/// The `[[kinds]]` entry that *is* the homeless kind, if the table declares one
/// (§FS-config.3.9.2): non-citable, and with no `folder` or `file`, because it
/// is the complement of every home rather than one of them. At most one entry
/// can be this, which the config validator holds.
fn declared_homeless_kind(kinds: &[KindConfig]) -> Option<&KindConfig> {
    kinds
        .iter()
        .find(|kind| !kind.citable && kind.folder.is_none() && kind.file.is_none())
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
