// The tree walk: which roots a scan starts from, which files it yields, and what
// it does with a path it cannot read (§AR-scanner.1). It sits beside `scanner.rs`
// rather than inside it because the two are different machines — that one is a
// line-by-line pass over a file's text, this one is a directory traversal — and
// they meet only at the file list one hands the other.

/// The tree walk (§AR-scanner.1): from each scan root, descend skipping hidden and
/// `[scan] exclude` directories, honouring `.gitignore` and friends unless
/// `respect_gitignore = false` (§AR-scanner.1.1, §FS-config.3.5), keeping only
/// scannable files, in a sorted order so findings are deterministic (§FS-errors.4).
fn walk_scannable_files(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Vec<PathBuf>> {
    let roots = scan_roots(config, scope, explicit_scope)?;
    // §FS-check.1.3: only an aliased root can hand the same file to two walks
    // under two spellings, so the ordinary walk never pays for the identity pass.
    let roots_alias = config.scan_full && roots_can_alias(&roots);
    let mut files = Vec::new();
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
        for entry in walker {
            let entry = entry?;
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                || !is_scannable(entry.path(), config)
            {
                continue;
            }
            files.push(entry.path().to_path_buf());
        }
    }
    // One file, one read (§FS-check.1.3). Two spellings first, while the list is
    // still in walk order and first-seen wins; then the byte-identical ones,
    // which the sort has just brought together.
    if roots_alias {
        dedup_by_file_identity(&mut files);
    }
    files.sort_by_key(|path| sort_path_key(path));
    // The roots may overlap — `include = ["docs", "docs/api"]` names one subtree
    // twice, and under `--full` every `include` root is walked beside the config
    // root that already contains it (§FS-check.1.3). Scanning a file twice would
    // report its declaration as a duplicate of itself.
    files.dedup();
    Ok(files)
}

/// Whether two of these walk roots can reach one file under two *different*
/// spellings (§FS-check.1.3). Only a root whose canonical form differs from the
/// path the walk starts at can: a symlink to a directory inside the config root,
/// or a case alias of one on a case-insensitive filesystem. Every other pair of
/// overlapping roots yields byte-identical descendant paths, which the exact-path
/// `dedup()` after the sort already collapses — so this predicate is what keeps
/// the ordinary walk from canonicalizing a file at a time.
fn roots_can_alias(roots: &[PathBuf]) -> bool {
    roots
        .iter()
        .any(|root| fs::canonicalize(root).is_ok_and(|canonical| canonical != *root))
}

/// Collapse the files two aliased roots reached under two spellings, keeping the
/// **first** (§FS-check.1.3). `root_scope_roots` walks the `include` roots before
/// the config root `--full` adds, so the surviving spelling is the one the plain
/// run reports, and `--full` stays purely additive: it appends out-of-scope lines
/// and never restates an in-scope one under a second name.
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
