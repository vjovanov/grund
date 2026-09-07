/// One `[[kinds]]` entry: the kind name plus the folder its declarations live in
/// and the human title `grund id` prints (§FS-config.3.4). When `file` is set,
/// every declaration of this kind must live in that exact file — a *single-file
/// kind*, used by `GRUND`/`GOAL`/`RM` whose IDs all live in one document
/// (`docs/grund.md`, `docs/goals.md`, `docs/roadmap.md`).
#[derive(Clone)]
pub struct KindConfig {
    /// The `kind` key (§FS-config.3.4) — the name `[citations.<kind>]` keys on,
    /// and, for a citable kind, the literal prefix of every ID in it. Spelled
    /// `prefix` before the rename; that spelling stopped loading in 0.13.0
    /// (§FS-config.3.4.6).
    pub kind: String,
    pub folder: Option<String>,
    pub file: Option<String>,
    pub title: Option<String>,
    /// The `index` key (§FS-config.3.4): which file under `folder` must list
    /// every declaration in it (§FS-check.3.18). Absent means the `README.md`
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
    /// The absent-by-default first-class-value opt-in (§FS-config.3.4.9,
    /// §FS-values.1).
    pub values: bool,
    /// A grammar override for this kind; absent inherits `[id].format`
    /// (§FS-config.3.4.10). Public because [`KindConfig`] is part of the
    /// embedding API, so consumers can inspect the effective snapshot shape.
    pub format: Option<String>,
    /// The fixed target-side resolution obligation (§FS-config.3.4.10).
    /// `None` means an ordinary kind; a fetch-enabled kind resolves it to
    /// [`KindResolution::Must`] while loading configuration.
    pub resolve: Option<KindResolution>,
    /// The repository-relative executable used only by explicit `grund fetch`
    /// (§FS-fetch.2).
    pub fetch: Option<String>,
}

/// A fetch-enabled kind's target-side resolution obligation
/// (§FS-config.3.4.10). This selects a finding class; it is not configurable
/// severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KindResolution {
    Must,
    Should,
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
    /// This row's ID template, falling back to the repository default
    /// (§FS-config.3.2, §FS-config.3.4.10).
    pub fn effective_format<'a>(&'a self, config: &'a Config) -> &'a str {
        self.format.as_deref().unwrap_or(&config.id_format)
    }

    /// The effective snapshot resolution obligation, if this is a
    /// fetch-enabled kind (§FS-config.3.4.10).
    pub fn resolution(&self) -> Option<KindResolution> {
        self.resolve
    }

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
