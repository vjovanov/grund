// The kind-index invariant, in a file of its own beside the section and
// reference families (§AR-checker.2.16, §AR-core-module-layout.1): a kind with a
// `folder` and an `index` (§FS-config.3.4) promises that the index names every
// declaration in that folder, as a full Markdown link. §FS-check.4.6 is the
// coverage half and §FS-check.3.17 the link half; this module owns both, plus
// the set of citations they make navigational rather than referential
// (§FS-check.4.1, §DF-index-not-an-inbound-citation).

/// One kind's index obligation, resolved against the config root
/// (§FS-config.3.4). `folder_key` and `index_key` are config-root-relative and
/// lexically normalized, so they compare against the paths the scanner recorded;
/// `index_file` is the readable path.
struct KindIndexTarget<'a> {
    kind: &'a str,
    folder_key: PathBuf,
    index_key: PathBuf,
    index_file: PathBuf,
}

/// Every `[[kinds]]` entry that carries both a `folder` and an enabled `index`
/// (§FS-config.3.4). A kind with no folder, or with `index = false`, is absent
/// from this list and is therefore invisible to every rule below.
fn kind_index_targets(config: &Config) -> Vec<KindIndexTarget<'_>> {
    config
        .kinds
        .iter()
        .filter_map(|kind| {
            let folder = kind.folder.as_deref()?;
            let index = kind.index_path()?;
            // §FS-config.3.4 rejects an `index` that does not name a `.md` file,
            // so this filter never fires on a config that loaded. It is here
            // because every rule below is stated in terms of what
            // `grund fmt --cross-refs` would write, and that pass runs on `.md`
            // files only (§FS-fmt.6.1): a non-Markdown index is a file the
            // formatter can never repair, and the honest report about one is no
            // report at all rather than a finding with no fix.
            if index.extension().and_then(|ext| ext.to_str()) != Some("md") {
                return None;
            }
            Some(KindIndexTarget {
                kind: kind.prefix.as_str(),
                folder_key: configured_home_path_key(folder),
                index_key: scanned_path_key(&index),
                index_file: config.root.join(&index),
            })
        })
        .collect()
}

/// How one citation of an indexed ID sits in the index file — the entry's form
/// (§FS-check.4.6, §FS-check.3.17, §DF-index-entry-form.2.1).
#[derive(Clone, Copy, Eq, PartialEq)]
enum IndexCitationForm {
    /// Wrapped as `[§<ID>…](<target>)` — the form `grund fmt --cross-refs`
    /// writes (§FS-fmt.6.2), which is what the entry has to be.
    Link,
    /// A recognized citation of the ID, unwrapped, **and one the
    /// cross-reference pass would wrap on its next `--write`**. §FS-check.3.17's
    /// finding, and the only form that earns it.
    Bare,
    /// Everything else: a citation `fmt` declines to wrap, so it is neither an
    /// entry nor a finding, and the ID falls to §FS-check.4.6's warning
    /// (§DF-index-entry-form.2.3).
    Ignored,
}

/// Classify every occurrence of `text` on `line` and keep the strongest form.
/// Reading the line rather than trusting a recorded column is what makes this
/// agree with `fmt` on a line that carries the same citation twice.
///
/// The two verdicts are deliberately asymmetric, because they ask different
/// questions. `Link` asks whether the entry *is* the link a reader can follow,
/// and any wrap satisfies that — including a hand-written one around an
/// unmarked token, which `fmt` would leave alone but which is a correct entry
/// as it stands. `Bare` asks whether `grund fmt --write` would turn this
/// occurrence into that link, and only an occurrence the cross-reference pass
/// actually reaches may answer yes: marker-prefixed (§FS-fmt.6.5) and outside
/// §FS-fmt.2.3's never-rewrite zones. Anything else is `Ignored` — §FS-check.3.17
/// names `grund fmt --write` as its fix, and an error whose named fix the tool
/// declines to perform is an error a repository can never clear
/// (§DF-index-entry-form.2.3), the same trap `shorthand_rewritable` keeps
/// §FS-check.3.13 out of.
///
/// `line` is Markdown: §FS-config.3.4 requires `index` to name a `.md` file and
/// `kind_index_targets` drops anything else, which is what lets the never-rewrite
/// test below be asked in its Markdown form.
fn index_citation_form(line: &str, text: &str, marker: &str) -> IndexCitationForm {
    let mut form = IndexCitationForm::Ignored;
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find(text) {
        let start = cursor + relative;
        let end = start + text.len();
        // Past the whole match, never `start + 1`: a citation begins with the
        // marker, which is multi-byte under the default `§`, and slicing one byte
        // into it would panic (§REQ-never-crashes).
        cursor = end;
        // §FS-fmt.6.3's wrap detection, from the other side: a `[` immediately
        // before the citation and `](…)` immediately after it is the shape the
        // formatter writes and re-derives, and the only shape it recognizes.
        let wrapped = start > 0
            && line.as_bytes()[start - 1] == b'['
            && line[end..]
                .strip_prefix("](")
                .and_then(|rest| rest.find(')'))
                .is_some_and(|close| close > 0);
        if wrapped {
            if is_inside_inline_code(line, start - 1) {
                continue;
            }
            return IndexCitationForm::Link;
        }
        // §FS-fmt.6.5: `--cross-refs` wraps marker-prefixed citations only, and
        // without `--marker` a bare token stays bare — so a bare token is not an
        // entry `fmt` can repair. The marker may be part of the recorded text or
        // sit just before this occurrence of it, which is the same token either
        // way; an empty marker (permitted outside strict mode) matches both
        // tests, exactly as it makes the formatter's own test vacuous.
        let marked = text.starts_with(marker) || line[..start].ends_with(marker);
        // §FS-fmt.2.3, through the one predicate the scanner and the rewrite
        // already share: an inline-code illustration and a Markdown link
        // destination are zones `fmt` never writes in.
        if !marked || never_rewrite_context(line, true, start) {
            continue;
        }
        form = IndexCitationForm::Bare;
    }
    form
}

/// What an index says about one ID: whether any citation of it is a full link,
/// and where the first bare one sits (§FS-check.3.17 anchors there).
#[derive(Default)]
struct IndexEntryState {
    linked: bool,
    first_bare: Option<(usize, usize)>,
}

impl IndexEntryState {
    fn record(&mut self, form: IndexCitationForm, line: usize, column: usize) {
        match form {
            IndexCitationForm::Link => self.linked = true,
            IndexCitationForm::Bare => {
                let here = (line, column);
                if self.first_bare.is_none_or(|seen| here < seen) {
                    self.first_bare = Some(here);
                }
            }
            IndexCitationForm::Ignored => {}
        }
    }
}

/// The declaration a finding about `id` points at — the same home `grund list`
/// and the unused warning pick, so a collapsed stub-and-inline pair is named at
/// the body rather than twice (§FS-list.2, §DF-index-entry-form.2.5).
fn index_home_declaration<'a>(
    config: &Config,
    decls: &'a [Declaration],
) -> Option<&'a Declaration> {
    decls
        .iter()
        .find(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
        .or_else(|| decls.first())
}

/// Whether any of `decls` sits under `folder_key` — the recursive membership
/// test of §FS-check.4.6: a stub in the folder is what puts an inline-homed ID
/// in it, and a folder's whole subtree counts, not its top level.
fn declarations_under_folder(
    decls: &[Declaration],
    folder_key: &Path,
    configured_root: &Path,
    physical_root: &Path,
) -> bool {
    decls.iter().any(|decl| {
        scanned_decl_relative_path(&decl.file, configured_root, physical_root)
            .is_some_and(|relative| relative.starts_with(folder_key))
    })
}

/// The IDs each kind index owes an entry for, keyed by the index's
/// config-root-relative path (§FS-check.4.6). Built once and read by both the
/// rule below and the unused-declaration carve-out.
struct KindIndexEntries {
    configured_root: PathBuf,
    physical_root: PathBuf,
    owed: BTreeMap<PathBuf, BTreeSet<Id>>,
}

impl KindIndexEntries {
    fn new(findings: &Findings, config: &Config) -> Self {
        let configured_root = scanned_path_key(&config.root);
        let physical_root = physical_path_key(&config.root);
        let mut owed: BTreeMap<PathBuf, BTreeSet<Id>> = BTreeMap::new();
        for target in kind_index_targets(config) {
            for (id, decls) in &findings.declarations {
                if id.kind != target.kind {
                    continue;
                }
                if declarations_under_folder(
                    decls,
                    &target.folder_key,
                    &configured_root,
                    &physical_root,
                ) {
                    owed
                        .entry(target.index_key.clone())
                        .or_default()
                        .insert(id.clone());
                }
            }
        }
        Self {
            configured_root,
            physical_root,
            owed,
        }
    }

    /// The IDs this index owes an entry for, or `None` when `path` is not a
    /// configured index (§FS-fmt.6.1). What the always-linkify carve-out wraps:
    /// the entries the rule is about, and not the rest of the page
    /// (§DF-index-always-linkified.2.2).
    fn entries_in(&self, path: &Path) -> Option<&BTreeSet<Id>> {
        if self.owed.is_empty() {
            return None;
        }
        let relative =
            scanned_decl_relative_path(path, &self.configured_root, &self.physical_root)?;
        self.owed.get(relative.as_ref())
    }

    /// §FS-check.4.1 / §DF-index-not-an-inbound-citation.2.2: this citation is a
    /// kind's own index entry — navigation, not use. Narrow on purpose: a
    /// citation in an index file of an ID whose home lies outside that folder is
    /// an ordinary reference and is not matched here.
    fn is_index_entry(&self, citation: &Citation) -> bool {
        if citation.namespace.is_some() || self.owed.is_empty() {
            return false;
        }
        let Some(relative) = scanned_decl_relative_path(
            &citation.file,
            &self.configured_root,
            &self.physical_root,
        ) else {
            return false;
        };
        self.owed
            .get(relative.as_ref())
            .is_some_and(|ids| ids.contains(&citation.id))
    }
}

// The three releases the kind-index ramp is stated in
// (§DF-index-compatibility-ramp.2.3). All three are named in message text, so
// all three are part of the release process: bumping the workspace version is
// also the moment to ask whether these still say what they mean
// (§FS-distribution.4). `index_entry_ramp_releases_are_ordered` below is the
// guard — it fails the build once the tree reaches `INDEX_ENTRY_ERROR_RELEASE`,
// which is the release §RM-index-entry-error has to land in, so the ramp cannot
// expire quietly.

/// The last release before `check` knew anything about a kind's index — the
/// "from" half of the pair §REQ-backwards-compatibility.3 requires a
/// verdict-flipping finding to name.
const INDEX_RULE_PRIOR_RELEASE: &str = "0.11.0";

/// The release the kind-index rules arrive in, and in which §FS-check.3.17 is an
/// error on arrival — the "to" half of that pair.
const INDEX_RULE_RELEASE: &str = "0.12.0";

/// The release in which §FS-check.4.6's warning becomes an error
/// (§REQ-backwards-compatibility.2, §DF-index-compatibility-ramp.2.3). Named in
/// the message text, because a warning that does not say when it bites tells a
/// maintainer they have a problem and not that they have a deadline.
const INDEX_ENTRY_ERROR_RELEASE: &str = "0.13.0";

/// §AR-checker.2.16 — the kind-index rule (§FS-check.4.6, §FS-check.3.17). One
/// pass per configured index: read the file once, classify the citations the
/// scanner already recorded in it, then judge each declaration the folder holds.
/// The index file is the only thing re-read here, the way §AR-checker.2.5
/// re-reads a stub's target.
fn check_kind_indexes(
    findings: &Findings,
    config: &Config,
    path_config: &Config,
    report: &mut CheckReport,
) {
    let targets = kind_index_targets(config);
    if targets.is_empty() {
        return;
    }
    let configured_root = scanned_path_key(&config.root);
    let physical_root = physical_path_key(&config.root);

    // One pass over the citations, bucketed by index file, so the per-kind loop
    // below is a lookup rather than another walk of the whole citation list
    // (§GOAL-fast-feedback).
    let index_keys: BTreeSet<&Path> = targets
        .iter()
        .map(|target| target.index_key.as_path())
        .collect();
    let mut cited_in_index: BTreeMap<&Path, Vec<&Citation>> = BTreeMap::new();
    for citation in &findings.citations {
        if citation.namespace.is_some() {
            continue;
        }
        let Some(relative) =
            scanned_decl_relative_path(&citation.file, &configured_root, &physical_root)
        else {
            continue;
        };
        if let Some(key) = index_keys.get(relative.as_ref()) {
            cited_in_index.entry(key).or_default().push(citation);
        }
    }
    // §FS-check.4.6: which index files *this run* read. The entries come from the
    // scan and the form from disk, so an index the run never scanned — a narrowed
    // `grund check <one-file>`, or an index the `[scan]` set excludes — would
    // otherwise look empty and report every declaration in the folder as
    // unlisted. A run that cannot see the index does not get to judge it; an
    // index file that does not exist is a different fact and still reported.
    let index_scanned: BTreeSet<&Path> = findings
        .scanned_files
        .iter()
        .filter_map(|file| scanned_decl_relative_path(file, &configured_root, &physical_root))
        .filter_map(|relative| index_keys.get(relative.as_ref()).copied())
        .collect();

    for target in &targets {
        // `is_file`, not `exists`: a path that is not a readable file is not an
        // index this run failed to read, it is an index that is not there — a
        // missing one, or a directory wearing the name — and that is §FS-check.4.6's
        // finding, not a reason to stay quiet.
        if !index_scanned.contains(target.index_key.as_path()) && target.index_file.is_file() {
            continue;
        }
        let covered: Vec<(&Id, &Declaration)> = findings
            .declarations
            .iter()
            .filter(|(id, _)| id.kind == target.kind)
            .filter(|(_, decls)| {
                declarations_under_folder(
                    decls,
                    &target.folder_key,
                    &configured_root,
                    &physical_root,
                )
            })
            .filter_map(|(id, decls)| Some((id, index_home_declaration(config, decls)?)))
            .collect();
        if covered.is_empty() {
            continue;
        }
        let index_display = display_path(path_config, &target.index_file);
        // §FS-check.4.6: a folder whose index file does not exist is the same
        // finding, once per declaration — the strongest form of the same fact,
        // not a different one. The three ways it can fail to read are named
        // apart, because "does not exist" said about a directory that plainly
        // does is a diagnosis a reader has to argue with.
        let text = fs::read_to_string(&target.index_file).ok();
        let absent = match &text {
            Some(_) => "",
            None if target.index_file.is_dir() => " (the index file is a directory)",
            None if target.index_file.exists() => " (the index file could not be read)",
            None => " (the index file does not exist)",
        };
        let lines: Vec<&str> = text.as_deref().map(|text| text.lines().collect()).unwrap_or_default();
        let mut entries: BTreeMap<&Id, IndexEntryState> = BTreeMap::new();
        for citation in cited_in_index
            .get(target.index_key.as_path())
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let Some(line) = lines.get(citation.line.saturating_sub(1)) else {
                continue;
            };
            // §FS-fmt.6.4: `fmt` leaves a declaration heading alone, so a
            // citation riding on one is no more repairable than one in an
            // inline-code span. Fenced blocks need no test here — the scanner
            // records no citation inside one.
            if declaration_captures(&config.grammar, line, false, true).is_some() {
                continue;
            }
            // An `Ignored` form creates no entry: a citation `fmt` will not wrap
            // neither satisfies the rule nor triggers §FS-check.3.17
            // (§DF-index-entry-form.2.3), so the ID is reported as unlisted.
            let form = index_citation_form(line, &citation.text, &config.marker);
            if form == IndexCitationForm::Ignored {
                continue;
            }
            entries
                .entry(&citation.id)
                .or_default()
                .record(form, citation.line, citation.column);
        }

        for (id, decl) in covered {
            match entries.get(id) {
                // §FS-check.3.17: an entry that exists and is not a link, at the
                // line `grund fmt --write` rewrites.
                Some(state) if !state.linked => {
                    let (line, column) = state.first_bare.unwrap_or((1, 1));
                    report.errors.push(Diagnostic {
                        code: "unlinked-index-entry",
                        path: Some(target.index_file.clone()),
                        line: Some(line),
                        column: Some(column),
                        // §REQ-backwards-compatibility.3 wants all three: the
                        // versions the verdict moved between, one command the
                        // tool ships, and a release note. The first two are
                        // here — and the command is only ever named on a site
                        // `fmt --write` will in fact rewrite, which is what
                        // `IndexCitationForm::Bare` is narrowed to mean.
                        message: format!(
                            "index entry {}{} is not a link; unchecked in grund {INDEX_RULE_PRIOR_RELEASE}, an error in {INDEX_RULE_RELEASE} — run `grund fmt --write`",
                            config.marker,
                            render_id(config, id)
                        ),
                        sites: Vec::new(),
                    });
                }
                Some(_) => {}
                // §FS-check.4.6: no entry at all, anchored at the declaration's
                // own heading — the one line that exists whether or not the
                // index file does (§DF-index-entry-form.2.6).
                None => {
                    report.warnings.push(Diagnostic {
                        code: "missing-index-entry",
                        path: Some(decl.file.clone()),
                        line: Some(decl.line),
                        column: None,
                        message: format!(
                            "{} is not listed in {index_display}{absent} — an index entry becomes an error in grund {INDEX_ENTRY_ERROR_RELEASE}",
                            render_id(config, id)
                        ),
                        sites: Vec::new(),
                    });
                }
            }
        }
    }
}

/// The configured index files, as a path-membership test (§FS-config.3.4).
/// `grund fmt` reads it to keep the cross-reference pass running over an index
/// whatever `[fmt.cross_refs] enabled` says (§FS-fmt.6.1,
/// §DF-index-always-linkified) — the one region the formatter always writes,
/// mirroring §FS-fmt.2.3's regions it never writes.
struct KindIndexFiles {
    configured_root: PathBuf,
    physical_root: PathBuf,
    keys: BTreeSet<PathBuf>,
}

impl KindIndexFiles {
    fn new(config: &Config) -> Self {
        Self {
            configured_root: scanned_path_key(&config.root),
            physical_root: physical_path_key(&config.root),
            keys: kind_index_targets(config)
                .into_iter()
                .map(|target| target.index_key)
                .collect(),
        }
    }

    /// The set a run that already linkifies everything needs — no config read,
    /// no allocation, and `contains` short-circuits on it.
    fn empty() -> Self {
        Self {
            configured_root: PathBuf::new(),
            physical_root: PathBuf::new(),
            keys: BTreeSet::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    fn contains(&self, path: &Path) -> bool {
        if self.keys.is_empty() {
            return false;
        }
        scanned_decl_relative_path(path, &self.configured_root, &self.physical_root)
            .is_some_and(|relative| self.keys.contains(relative.as_ref()))
    }
}
