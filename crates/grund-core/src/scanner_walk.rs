// The tree walk: which roots a scan starts from, which files it yields, and what
// it does with a path it cannot read (§AR-scanner.1). It sits beside `scanner.rs`
// rather than inside it because the two are different machines — that one is a
// line-by-line pass over a file's text, this one is a directory traversal — and
// they meet only at the file list one hands the other.

/// The tree walk for callers with nowhere to put a path the walk could not read
/// — `fmt`, whose job is rewriting the files it *can* read. `check` and `refs`
/// use `walk_scannable_files_reporting` so an unresolvable link reaches the
/// report instead (§FS-config.3.5, §FS-check.2).
fn walk_scannable_files(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Vec<PathBuf>> {
    Ok(walk_scannable_files_reporting(config, scope, explicit_scope)?.0)
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
) -> Result<(Vec<PathBuf>, Vec<ScanError>)> {
    let roots = scan_roots(config, scope, explicit_scope)?;
    // §FS-config.3.5: an aliased root, or a symlink met on the way down, is what
    // hands the same file to the walk under two spellings — nothing else does, so
    // a tree with neither never pays for the identity pass (§GOAL-fast-feedback).
    let mut can_alias = roots_can_alias(&roots);
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for scan_root in roots {
        if !scan_root.exists() {
            continue;
        }
        let canonical_scan_root =
            fs::canonicalize(&scan_root).unwrap_or_else(|_| scan_root.to_path_buf());
        // §AR-workspace.6: a root scan starts outside member namespaces; an
        // included path at or below a member boundary belongs to the member scan.
        if config
            .workspace_boundary_roots
            .iter()
            .any(|root| canonical_scan_root.starts_with(root))
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
        let excluded = config.exclude.clone();
        let e2e_cases_root = config
            .kinds
            .iter()
            .find(|kind| kind.prefix == "E2E")
            .and_then(|kind| kind.folder.as_deref())
            .map(|folder| config.root.join(folder));
        let e2e_config = config.clone();
        // §AR-workspace.6: precompute the boundary path components once,
        // expressed relative to the canonical scan root. The walker filter is
        // then a single component-suffix compare — no per-entry `canonicalize`
        // syscall, no allocation in the hot path. `strip_prefix` only removes
        // the root, so the descendant suffix is invariant under symlink
        // resolution — comparing against `scan_root_for_filter` works even if
        // `scan_root` itself is a symlink.
        let boundary_suffixes: Vec<PathBuf> = config
            .workspace_boundary_roots
            .iter()
            .filter_map(|root| root.strip_prefix(&canonical_scan_root).ok())
            .map(Path::to_path_buf)
            .collect();
        let scan_root_for_filter = scan_root.clone();
        builder.filter_entry(move |e| {
            if e.depth() == 0 {
                return true;
            }
            if e.file_type().is_some_and(|file_type| file_type.is_dir())
                && let Ok(relative) = e.path().strip_prefix(&scan_root_for_filter)
                && boundary_suffixes
                    .iter()
                    .any(|suffix| relative == suffix.as_path())
            {
                return false;
            }
            if e.file_type().is_some_and(|file_type| file_type.is_dir()) {
                if is_hidden(e.path()) {
                    return false;
                }
                if is_direct_e2e_case_dir(e.path(), e2e_cases_root.as_deref(), &e2e_config) {
                    return false;
                }
                let Some(name) = e.path().file_name().and_then(|name| name.to_str()) else {
                    return true;
                };
                return !excluded.iter().any(|item| item == name);
            }
            true
        });
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
                    errors.extend(walk_error_report(&err, config));
                    continue;
                }
            };
            can_alias |= entry.path_is_symlink();
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                || !is_scannable(entry.path(), config)
            {
                continue;
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
    if can_alias {
        dedup_by_file_identity(&mut files);
    }
    files.sort_by_key(|path| sort_path_key(path));
    // The roots may overlap — `include = ["docs", "docs/api"]` names one subtree
    // twice, and under `--full` every `include` root is walked beside the config
    // root that already contains it (§FS-check.1.3). Scanning a file twice would
    // report its declaration as a duplicate of itself.
    files.dedup();
    // §FS-errors.4: the walk meets its unreadable paths in readdir order, which is
    // the filesystem's; the text report sorts before printing, the API surface
    // hands the list over as it stands, so it is sorted once here for both.
    errors.sort_by_key(|(path, message)| (sort_path_key(path), message.clone()));
    Ok((files, errors))
}

/// The per-file scan failure a walker error becomes (§FS-check.2), or `None` when
/// the walk was never going to read through the path it names (§FS-config.3.5).
///
/// `follow_links` hands back an error in place of the entry for a link it cannot
/// resolve — a broken target, or a loop, which the `ignore` crate detects and
/// reports rather than recursing into. The walker's own directory filter never
/// sees an error, so the hidden-name and `[scan] exclude` tests a looping
/// directory would have faced are applied here instead; a broken link is judged
/// by `[scan] extensions` like any other file, which is what keeps a dangling
/// `docs/logo.png` silent. Anything else — an unreadable directory, a malformed
/// ignore file — is reported as it stands: it is a hole in the walk either way,
/// and a silent one is what §REQ-no-missed-citation.1 rules out.
fn walk_error_report(err: &ignore::Error, config: &Config) -> Option<ScanError> {
    if let Some((link, ancestor)) = walk_error_loop(err) {
        let name = link.file_name().and_then(|name| name.to_str())?;
        if is_hidden(link) || config.exclude.iter().any(|item| item == name) {
            return None;
        }
        let reason = format!(
            "symlink loop: the target is the ancestor directory {}",
            display_path(config, ancestor)
        );
        return Some((link.to_path_buf(), reason));
    }
    let path = walk_error_path(err)?;
    if !fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Some((path.to_path_buf(), walk_error_reason(err)));
    }
    if !is_scannable(path, config) {
        return None;
    }
    let reason = match err.io_error().map(std::io::Error::kind) {
        Some(std::io::ErrorKind::NotFound) => {
            "broken symlink: the target does not exist".to_string()
        }
        _ => format!("unreadable symlink: {}", walk_error_reason(err)),
    };
    Some((path.to_path_buf(), reason))
}

/// The `(link, ancestor)` of a symlink loop, at any nesting depth of the error.
fn walk_error_loop(err: &ignore::Error) -> Option<(&Path, &Path)> {
    match err {
        ignore::Error::Loop { ancestor, child } => Some((child, ancestor)),
        ignore::Error::WithPath { err, .. } | ignore::Error::WithDepth { err, .. } => {
            walk_error_loop(err)
        }
        _ => None,
    }
}

/// The in-tree path a walker error is about, at any nesting depth.
fn walk_error_path(err: &ignore::Error) -> Option<&Path> {
    match err {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::WithDepth { err, .. } => walk_error_path(err),
        _ => None,
    }
}

/// A walker error's reason, without the path `ignore` writes into its own text:
/// the diagnostic already carries the path (§FS-errors.2.2), and the one `ignore`
/// writes is absolute, which a report may not contain (§FS-errors.4).
fn walk_error_reason(err: &ignore::Error) -> String {
    match err.io_error() {
        Some(io) => std::io::Error::from(io.kind()).to_string(),
        None => match err {
            ignore::Error::WithPath { err, .. } => err.to_string(),
            other => other.to_string(),
        },
    }
}

/// Whether two of these walk *roots* can reach one file under two *different*
/// spellings (§FS-check.1.3). Only a root whose canonical form differs from the
/// path the walk starts at can: a symlink to a directory inside the config root,
/// or a case alias of one on a case-insensitive filesystem. Every other pair of
/// overlapping roots yields byte-identical descendant paths, which the exact-path
/// `dedup()` after the sort already collapses.
///
/// This is one of the two ways a file gets two names; the other is a symlink met
/// *below* a root, which the walk notices per entry (§FS-config.3.5). Together
/// they are what keeps a tree with neither from canonicalizing a file at a time
/// — a `realpath` syscall per file is not a price the ordinary walk should pay
/// (§GOAL-fast-feedback).
fn roots_can_alias(roots: &[PathBuf]) -> bool {
    roots
        .iter()
        .any(|root| fs::canonicalize(root).is_ok_and(|canonical| canonical != *root))
}

/// Collapse the files reached under two spellings, keeping the **first**
/// (§FS-check.1.3, §FS-config.3.5). `root_scope_roots` walks the `include` roots
/// before the config root `--full` adds, so the surviving spelling is the one the
/// plain run reports, and `--full` stays purely additive: it appends out-of-scope
/// lines and never restates an in-scope one under a second name. Within a single
/// root the caller has already sorted, so "first" there is the lexicographically
/// first path rather than whatever readdir happened to say (§FS-errors.4).
fn dedup_by_file_identity(files: &mut Vec<PathBuf>) {
    let mut seen = BTreeSet::new();
    files.retain(|file| seen.insert(physical_path_key(file)));
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
