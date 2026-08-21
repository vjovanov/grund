// The tree walk: which roots a scan starts from, which files it yields, and what
// it does with a path it cannot read (§AR-scanner.1). It sits beside `scanner.rs`
// rather than inside it because the two are different machines — that one is a
// line-by-line pass over a file's text, this one is a directory traversal — and
// they meet only at the file list one hands the other.

/// The tree walk for the callers that ask a yes/no question about the tree and
/// nothing else — today the `--cross-refs` auto-enable probe, which wants to know
/// whether the scope holds any Markdown (§FS-fmt.6.6). Every caller that *reports*
/// takes `walk_scannable_files_reporting`, so an unresolvable link reaches the
/// report rather than being dropped here (§FS-config.3.5, §FS-check.2): this one
/// is walking a tree that the reporting walk is about to walk again and account
/// for, so repeating its errors would print each of them twice.
fn walk_scannable_files(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Vec<PathBuf>> {
    Ok(walk_scannable_files_reporting(config, scope, explicit_scope)?.files)
}

/// What one walk of the tree produced (§AR-scanner.1).
struct WalkedTree {
    /// The scannable files, sorted, one entry per physical file (§FS-errors.4).
    files: Vec<PathBuf>,
    /// The paths the walk could not read, for the caller to report (§FS-check.2).
    errors: Vec<ScanError>,
    /// The files that are in this tree only by a link: their physical path is
    /// outside the config root. Read like any other file (§FS-config.3.5) —
    /// `fmt --write` is the caller that treats them differently, because
    /// rewriting one edits a file the project does not own (§FS-fmt.2.3.2).
    outside_root: BTreeSet<PathBuf>,
}

/// The tree walk (§AR-scanner.1): from each scan root, descend skipping hidden and
/// `[scan] exclude` directories, honouring `.gitignore` and friends unless
/// `respect_gitignore = false` (§AR-scanner.1.1, §FS-config.3.5), following
/// symlinks, keeping only scannable files, in a sorted order so findings are
/// deterministic (§FS-errors.4). Returns the paths it could not read beside the
/// files, for the caller to report (§FS-check.2).
fn walk_scannable_files_reporting(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<WalkedTree> {
    let roots = scan_roots(config, scope, explicit_scope)?;
    // §FS-config.3.5: an aliased root, or a symlink met on the way down, is what
    // hands the same file to the walk under two spellings — nothing else does, so
    // a tree with neither never pays for the identity pass (§GOAL-fast-feedback).
    // This is the *list* of those files rather than a flag saying one exists: the
    // pass resolves what is in it and compares everything else by path, so one
    // link in a repository costs one `realpath` and not one per file
    // (§AR-scanner.1).
    let mut aliasable = BTreeSet::new();
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for scan_root in roots {
        if !scan_root.exists() {
            continue;
        }
        let canonical_scan_root =
            fs::canonicalize(&scan_root).unwrap_or_else(|_| scan_root.to_path_buf());
        // §AR-workspace.6: a root scan starts outside member namespaces; an
        // included path at or below a member boundary belongs to the member scan,
        // and one inside any *other* project of the run belongs to that project
        // (§FS-workspace.6).
        if config
            .workspace_boundary_roots
            .iter()
            .any(|root| canonical_scan_root.starts_with(root))
            || owned_by_another_project(config, &canonical_scan_root)
        {
            continue;
        }
        if scan_root.is_file() {
            if is_scannable(&scan_root, config) {
                files.push(scan_root);
            }
            continue;
        }
        let mut builder = WalkBuilder::new(&scan_root);
        builder.hidden(false);
        // §FS-config.3.5: a symlink is part of the tree at the path it occupies, so
        // the walk reads through it — a linked file as the file, a linked directory
        // by descending. The entry keeps its in-tree path either way, which is what
        // the directory filter below and every finding are then expressed in.
        builder.follow_links(true);
        if !config.respect_gitignore {
            builder
                .ignore(false)
                .git_ignore(false)
                .git_global(false)
                .git_exclude(false)
                .parents(false);
        }
        // §AR-workspace.6: precompute the boundary path components once,
        // expressed relative to the canonical scan root. The walker filter is
        // then a single component-suffix compare — no per-entry `canonicalize`
        // syscall, no allocation in the hot path. `strip_prefix` only removes
        // the root, so the descendant suffix is invariant under symlink
        // resolution — comparing against the filter's `scan_root` works even if
        // `scan_root` itself is a symlink.
        let boundary_suffixes: Vec<PathBuf> = config
            .workspace_boundary_roots
            .iter()
            .filter_map(|root| root.strip_prefix(&canonical_scan_root).ok())
            .map(Path::to_path_buf)
            .collect();
        // The directory links the filter met, shared with the loop below: a file
        // under one of them is reached under a spelling that is not its own, the
        // same as a file that is a link itself (§AR-scanner.1).
        let link_roots = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let filter = WalkDirFilter {
            scan_root: scan_root.clone(),
            boundary_suffixes,
            boundary_roots: config.workspace_boundary_roots.clone(),
            excluded: config.exclude.clone(),
            e2e_cases_root: config
                .kinds
                .iter()
                .find(|kind| kind.prefix == "E2E")
                .and_then(|kind| kind.folder.as_deref())
                .map(|folder| config.root.join(folder)),
            config: config.clone(),
            link_roots: std::sync::Arc::clone(&link_roots),
        };
        builder.filter_entry(move |entry| filter.keep(entry));
        // A root that resolves elsewhere reaches every one of its files under a
        // spelling that is not the file's own, so all of them can alias (§FS-check.1.3).
        let root_is_aliased = canonical_scan_root != scan_root;
        let walker = builder.build();
        let mut root_files = Vec::new();
        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                // §FS-config.3.5: a link the walk cannot resolve is a file the scan
                // cannot read — reported at its own path, the walk continuing past
                // it (§FS-check.2). Failing the whole scan here would let one broken
                // link take the entire report with it.
                Err(err) => {
                    errors.extend(walk_error_report(&err, config, &scan_root));
                    continue;
                }
            };
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                || !is_scannable(entry.path(), config)
            {
                continue;
            }
            if root_is_aliased || entry.path_is_symlink() || under_link(&link_roots, entry.path()) {
                aliasable.insert(entry.path().to_path_buf());
            }
            root_files.push(entry.path().to_path_buf());
        }
        // §FS-errors.4: within one root the order is the filesystem's, and the
        // first-seen rule below turns that order into a choice of *spelling*. Sort
        // each root's own list before it joins the others, so the choice is ours:
        // the earlier root wins, and within a root the lexicographically first path.
        root_files.sort_by_key(|path| sort_path_key(path));
        files.append(&mut root_files);
    }
    // One file, one read (§FS-check.1.3, §FS-config.3.5). Two spellings first,
    // while the list is still in walk order and first-seen wins; then the
    // byte-identical ones, which the sort has just brought together.
    let resolved = resolve_aliasable(&aliasable);
    // §FS-workspace.6: the directory filter stops a *directory* link at another
    // project's root, and a link straight onto one of its files is the same
    // crossing one entry lower down. The resolution is already paid for, so the
    // boundary costs a prefix test over the files that can wear a second name and
    // nothing at all for a run that loaded no workspace.
    if !config.workspace_project_roots.is_empty() {
        files.retain(|file| {
            !resolved
                .get(file.as_path())
                .is_some_and(|physical| owned_by_another_project(config, physical))
        });
    }
    if !resolved.is_empty() {
        dedup_by_file_identity(&mut files, &resolved);
    }
    files.sort_by_key(|path| sort_path_key(path));
    // The roots may overlap — `include = ["docs", "docs/api"]` names one subtree
    // twice, and under `--full` every `include` root is walked beside the config
    // root that already contains it (§FS-check.1.3). Scanning a file twice would
    // report its declaration as a duplicate of itself.
    files.dedup();
    // §FS-errors.4: the walk meets its unreadable paths in readdir order, which is
    // the filesystem's; the text report sorts before printing, the API surface
    // hands the list over as it stands, so it is sorted once here for both. Then
    // deduplicated for the same reason the file list is: overlapping roots meet
    // the same broken link once per root, and under `--full` every `include` root
    // is walked beside the config root that contains it, so every scan error
    // would be printed twice — which the additivity rule of §FS-check.1.3 forbids
    // as much for an `error:` line as for a finding.
    errors.sort_by_key(|(path, message)| (sort_path_key(path), message.clone()));
    errors.dedup();
    // §FS-fmt.2.3.2: a file whose physical path is not under the config root is in
    // this tree only by the link that reaches it. The resolution is already paid
    // for above, so this is a prefix test over the links and nothing more.
    let outside_root = files
        .iter()
        .filter(|file| {
            resolved
                .get(file.as_path())
                .is_some_and(|physical| !physical.starts_with(&config.root))
        })
        .cloned()
        .collect();
    Ok(WalkedTree {
        files,
        errors,
        outside_root,
    })
}

/// The directory filter the walk runs on every entry it meets: the workspace
/// boundary (§AR-workspace.6), hidden names and `[scan] exclude`
/// (§FS-config.3.5), and the E2E case directories the manifest pass owns
/// (§AR-scanner.6).
///
/// The **name** tests read the in-tree path, so a followed link is pruned under
/// the name it wears in the tree — `docs/node_modules -> ../../node_modules` is
/// excluded exactly as a real directory of that name would be. The two
/// **boundary** tests read the canonical path as well, because a member root and
/// a case directory are properties of the directory rather than of the name it
/// is reached under (§AR-scanner.1): reached through a link they match neither
/// the precomputed suffix nor the parent compare, and the walk would descend
/// into a namespace that is not its to read.
///
/// `link_roots` is how a link-reached directory is recognized without a syscall
/// per entry. The sequential walker filters a directory before its children, so
/// every directory link the walk meets is recorded here before anything below it
/// is asked about, and a prefix test then answers for the whole subtree; a link
/// already covered by one is not recorded again, so the list holds the disjoint
/// link subtrees and nothing more.
struct WalkDirFilter {
    scan_root: PathBuf,
    boundary_suffixes: Vec<PathBuf>,
    boundary_roots: Vec<PathBuf>,
    excluded: Vec<String>,
    e2e_cases_root: Option<PathBuf>,
    config: Config,
    link_roots: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl WalkDirFilter {
    /// Whether the walk keeps this entry — and, for a directory, descends into
    /// it. Files are never filtered here; the extension test is the caller's.
    fn keep(&self, entry: &ignore::DirEntry) -> bool {
        if entry.depth() == 0 {
            return true;
        }
        if !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_dir())
        {
            return true;
        }
        let path = entry.path();
        let resolved = self.resolved_link_dir(entry);
        let resolved = resolved.as_deref();
        if self.crosses_a_project_boundary(path, resolved) || is_hidden(path) {
            return false;
        }
        if self.is_e2e_case_dir(path) || resolved.is_some_and(|path| self.is_e2e_case_dir(path)) {
            return false;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };
        !self.excluded.iter().any(|item| item == name)
    }

    /// The canonical path of a directory the walk reached **through** a symlink
    /// — the link itself, or anything below one — and `None` for a directory
    /// reached under its own name, which is the ordinary case and pays no
    /// syscall (§AR-scanner.1, §GOAL-fast-feedback).
    fn resolved_link_dir(&self, entry: &ignore::DirEntry) -> Option<PathBuf> {
        let path = entry.path();
        let mut link_roots = self
            .link_roots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let is_link = entry.path_is_symlink();
        let covered = link_roots.iter().any(|root| path.starts_with(root));
        if is_link && !covered {
            link_roots.push(path.to_path_buf());
        }
        drop(link_roots);
        (is_link || covered)
            .then(|| fs::canonicalize(path).ok())
            .flatten()
    }

    /// §AR-workspace.6: a member root is out of bounds for the root scan under
    /// every name it wears — the precomputed suffix for an ordinary descent, the
    /// canonical member root for a directory reached through a link — and so is
    /// any directory another project of the run owns, which is the same boundary
    /// read in the directions the member list cannot see (§FS-workspace.6).
    fn crosses_a_project_boundary(&self, path: &Path, resolved: Option<&Path>) -> bool {
        if let Ok(relative) = path.strip_prefix(&self.scan_root)
            && self
                .boundary_suffixes
                .iter()
                .any(|suffix| relative == suffix.as_path())
        {
            return true;
        }
        resolved.is_some_and(|resolved| {
            self.boundary_roots
                .iter()
                .any(|root| resolved.starts_with(root))
                || owned_by_another_project(&self.config, resolved)
        })
    }

    fn is_e2e_case_dir(&self, path: &Path) -> bool {
        is_direct_e2e_case_dir(path, self.e2e_cases_root.as_deref(), &self.config)
    }
}

/// Whether a canonical path belongs to a project of this run that is **not** the
/// one doing the walking (§FS-workspace.6, §AR-workspace.6). The owner is the
/// innermost project root containing it, since a nested member's root sits inside
/// the block that listed it; a path no project owns is not a boundary at all, but
/// outside content the tree linked in deliberately (§FS-config.3.5). Empty list —
/// every run that loaded no workspace — answers `false` without a comparison.
fn owned_by_another_project(config: &Config, canonical: &Path) -> bool {
    config
        .workspace_project_roots
        .iter()
        .filter(|root| canonical.starts_with(root))
        .max_by_key(|root| root.components().count())
        .is_some_and(|owner| owner != &config.root)
}

/// Whether the walk reached this path *through* one of the directory links it
/// recorded — which makes the file's spelling not its own, exactly as being a
/// link itself would (§AR-scanner.1).
fn under_link(link_roots: &std::sync::Mutex<Vec<PathBuf>>, path: &Path) -> bool {
    link_roots
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .any(|root| path.starts_with(root))
}

/// Where each file that can wear a second name physically is: the in-tree path it
/// was walked under, mapped to the path `canonicalize` resolves it to.
type AliasTargets = std::collections::HashMap<PathBuf, PathBuf>;

/// Resolve the files the walk saw arrive under a spelling that is not their own —
/// a link, a file below a directory link, or any file of an aliased root. This is
/// the only `canonicalize` the walk spends: everything else is answered by
/// comparing a path against what these resolved to (§AR-scanner.1,
/// §GOAL-fast-feedback).
fn resolve_aliasable(aliasable: &BTreeSet<PathBuf>) -> AliasTargets {
    aliasable
        .iter()
        .map(|file| (file.clone(), physical_path_key(file)))
        .collect()
}

/// Collapse the files reached under two spellings, keeping the **first**
/// (§FS-check.1.3, §FS-config.3.5). `root_scope_roots` walks the `include` roots
/// before the config root `--full` adds, so the surviving spelling is the one the
/// plain run reports, and `--full` stays purely additive: it appends out-of-scope
/// lines and never restates an in-scope one under a second name. Within a single
/// root the caller has already sorted, so "first" there is the lexicographically
/// first path rather than whatever readdir happened to say (§FS-errors.4).
///
/// `resolved` covers the files that can wear a second name, and the paths they
/// resolve to are the only ones another file can turn out to be — so every other
/// file is answered by a lookup in that small target set and is never resolved at
/// all. A repository with one symlink pays one `realpath` and not one per file,
/// which is what a flag saying "this tree has a link in it" could not do
/// (§GOAL-fast-feedback, §AR-scanner.1).
fn dedup_by_file_identity(files: &mut Vec<PathBuf>, resolved: &AliasTargets) {
    let targets: std::collections::HashSet<&Path> =
        resolved.values().map(PathBuf::as_path).collect();
    let mut seen = BTreeSet::new();
    files.retain(|file| {
        let key = resolved
            .get(file.as_path())
            .map_or(file.as_path(), PathBuf::as_path);
        !targets.contains(key) || seen.insert(key.to_path_buf())
    });
}

/// Direct `e2e/cases/<name>/` directories are E2E manifest declarations
/// (§AR-scanner.6), so the ordinary file walk must not scan their fixture repos.
fn is_direct_e2e_case_dir(path: &Path, cases_root: Option<&Path>, config: &Config) -> bool {
    let Some(cases_root) = cases_root else {
        return false;
    };
    if path.parent() != Some(cases_root) || !path.join("expected.exit").is_file() {
        return false;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| e2e_id_from_case_dir_name(config, name))
        .is_some()
}

/// The directories (or single file) the walk starts from: a `[path]` argument when
/// given (narrowing the default scope), otherwise `[scan] include` resolved against
/// the repo root, otherwise the whole root (§FS-config.3.5, §AR-scanner.1).
fn scan_roots(config: &Config, scope: Option<&Path>, explicit_scope: bool) -> Result<Vec<PathBuf>> {
    scan_roots_for(config, scope, explicit_scope, config.scan_full)
}

/// §FS-check.1.3: `full` cancels `[scan] include` for this walk and nothing else
/// — an explicit path argument still narrows, and `exclude`, the ignore files,
/// and `extensions` are untouched. `check --full` asks both ways: once with
/// `true` to walk the whole root, and once with `false` to learn which of what it
/// read was inside the configured scope (§FS-check.3.14).
fn scan_roots_for(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    full: bool,
) -> Result<Vec<PathBuf>> {
    if explicit_scope {
        let scope = scope.unwrap_or(Path::new("."));
        if !scope.exists() {
            return Err(anyhow!("path does not exist: {}", scope.display()));
        }
        let scope = fs::canonicalize(scope).unwrap_or_else(|_| scope.to_path_buf());
        if scope.is_file() {
            return Ok(vec![scope]);
        }
        if scope == config.root {
            return Ok(root_scope_roots(config, full));
        }
        return Ok(vec![scope]);
    }
    Ok(root_scope_roots(config, full))
}

/// The roots a walk of the whole config root starts from (§FS-config.3.5).
///
/// Without `--full` that is `[scan] include`, or the root itself when the key is
/// unset. With `--full` it is the root **and** every `include` root, not the root
/// alone: a walk root is never pruned by `.gitignore`, `[scan] exclude`, or the
/// hidden-directory rule, while the same directory reached as a *descendant* of
/// the config root is. Starting only at the root would therefore read *fewer*
/// files than the plain walk whenever an `include` entry is gitignored, excluded,
/// or hidden — and `--full` would turn a red run green, which is exactly what
/// §FS-check.1.3 promises it can never do. `walk_scannable_files` deduplicates
/// the file list, so an `include` root the root walk already covers is read once.
/// Collecting each root through `components()` folds away the `./` and trailing
/// separator an entry may be written with, so two roots naming one directory
/// yield byte-identical descendant paths and the dedup can recognize the reread.
///
/// The `include` roots come **first** under `--full`, ahead of the config root.
/// An `include` root that is a symlink to a directory inside the root, or a case
/// alias of one, reaches its files under a spelling the root walk does not
/// reproduce, so the dedup has to choose between two names for one file; walking
/// `include` first makes the first-seen winner the spelling `grund check` prints
/// without the flag, which is what keeps `--full` purely additive.
fn root_scope_roots(config: &Config, full: bool) -> Vec<PathBuf> {
    let include = config
        .include
        .iter()
        .flatten()
        .map(|entry| config.root.join(entry).components().collect::<PathBuf>());
    match (full, config.include.is_some()) {
        (true, _) => include.chain(std::iter::once(config.root.clone())).collect(),
        (false, true) => include.collect(),
        (false, false) => vec![config.root.clone()],
    }
}
