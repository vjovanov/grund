/// Depth of a heading line — count of leading `#` — used to decide whether a
/// section heading nests under the current declaration (§AR-scanner.2.2).
fn heading_level_for_line(line: &str, markdown_heading: bool, caps: &regex::Captures) -> usize {
    if markdown_heading {
        return line
            .trim_start()
            .chars()
            .take_while(|ch| *ch == '#')
            .count()
            .max(1);
    }
    // Code-form declarations (§DF-code-declarations-drop-hash) match the branch
    // that has no `#+`, so no heading group is set; default to depth 1.
    caps.name("hashes")
        .or_else(|| caps.name("mdhashes"))
        .map(|m| m.as_str().len())
        .unwrap_or(1)
}

/// A file that could not be read or decoded during the walk. The walk continues
/// past it (§FS-check.2); callers that are point queries treat any entry here as
/// fatal, `check` and `refs` report it and exit 2 with a still-printed report.
type ScanError = (PathBuf, String);

type FileScanResult = (PathBuf, std::result::Result<Findings, String>);

fn scan_one_file(
    file: &Path,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
    overlays: &TextOverlays,
) -> FileScanResult {
    let mut findings = Findings::default();
    let result = if let Some(text) = overlay_text(overlays, file) {
        scan_file_text(file, text, config, &mut findings, workspace_targets)
    } else {
        scan_file(file, config, &mut findings, workspace_targets)
    };
    match result {
        Ok(()) => {
            findings.scanned_files.push(file.to_path_buf());
            (file.to_path_buf(), Ok(findings))
        }
        Err(err) => (file.to_path_buf(), Err(format!("{err:#}"))),
    }
}

fn merge_findings(target: &mut Findings, mut source: Findings) {
    for (id, mut declarations) in source.declarations {
        target
            .declarations
            .entry(id)
            .or_default()
            .append(&mut declarations);
    }
    target.citations.append(&mut source.citations);
    target.escaped_citations.append(&mut source.escaped_citations);
    target
        .near_miss_headings
        .append(&mut source.near_miss_headings);
    target.scanned_files.append(&mut source.scanned_files);
    target.file_structure.extend(source.file_structure);
}

fn scan_file_results(
    files: &[PathBuf],
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
    overlays: &TextOverlays,
) -> Vec<FileScanResult> {
    files
        .par_iter()
        .map(|file| scan_one_file(file, config, workspace_targets, overlays))
        .collect::<Vec<_>>()
}

/// One full tree walk: scan every file (§AR-scanner.2) plus the e2e case
/// directories (§AR-scanner.6), collecting unreadable files rather than aborting
/// so `check` can report them and keep going (§FS-check.2). The wrapper around
/// the workspace-aware variant with no targets — single-project scans and
/// member-local scans share this path.
fn scan_tree(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<(Findings, Vec<ScanError>)> {
    scan_tree_with_workspace(config, scope, explicit_scope, &[])
}

/// Workspace-aware tree walk: `§<alias>/<ID>` citations parse with each
/// target's grammar inline, so the workspace layer (§FS-workspace.1,
/// §AR-workspace.2) never needs to re-read the files the initial scan
/// already read.
fn scan_tree_with_workspace(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Result<(Findings, Vec<ScanError>)> {
    scan_tree_with_workspace_threshold(
        config,
        scope,
        explicit_scope,
        workspace_targets,
        PARALLEL_SCAN_MIN_FILES,
        &TextOverlays::new(),
    )
}

fn scan_tree_with_workspace_threshold(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    workspace_targets: &[WorkspaceCitationTarget],
    parallel_min_files: usize,
    overlays: &TextOverlays,
) -> Result<(Findings, Vec<ScanError>)> {
    // §FS-config.3.5: a link the walk could not resolve is already a scan failure
    // before a single file is opened — it joins the per-file ones (§FS-check.2).
    let walked = walk_scannable_files_reporting(config, scope, explicit_scope)?;
    // §FS-check.4.8: the walk's directories travel with its files, for the rule that
    // asks which of them holds a `[workspace]` block nothing claims. Carried, not
    // judged: the scanner never asks that question itself (§AR-workspace.1).
    let mut findings = Findings { walked_dirs: walked.dirs, ..Findings::default() };
    let (mut files, mut errors) = (walked.files, walked.errors);
    add_overlay_scan_files(config, scope, explicit_scope, overlays, &mut files)?;
    if files.len() >= parallel_min_files {
        for (file, result) in scan_file_results(&files, config, workspace_targets, overlays) {
            match result {
                Ok(file_findings) => merge_findings(&mut findings, file_findings),
                Err(message) => errors.push((file, message)),
            }
        }
    } else {
        for file in files {
            match scan_one_file(&file, config, workspace_targets, overlays) {
                (_, Ok(file_findings)) => merge_findings(&mut findings, file_findings),
                (_, Err(message)) => errors.push((file, message)),
            }
        }
    }
    if let Err(err) = scan_e2e_cases(config, scope, explicit_scope, &mut findings) {
        errors.push((config.root.join("e2e/cases"), format!("{err:#}")));
    }
    // §FS-workspace.1: when the citing-grammar pass and the target-grammar pass both
    // fire on the same line they emit in source order *per pass*; one sort at the end
    // keeps a workspace scan's per-line order the single-project scan's left-to-right one.
    if !workspace_targets.is_empty() {
        findings.citations.sort_by(|a, b| {
            (sort_path_key(&a.file), a.line, a.column).cmp(&(
                sort_path_key(&b.file),
                b.line,
                b.column,
            ))
        });
    }
    // §AR-scanner.2.6: shorthand citations name a declaration that may live in
    // any file, so they can only be resolved once the whole walk (including the
    // E2E cases above) has produced the declaration set.
    resolve_shorthand_citations(&mut findings);
    Ok((findings, errors))
}

fn scan_tree_with_workspace_overlays(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    workspace_targets: &[WorkspaceCitationTarget],
    overlays: &TextOverlays,
) -> Result<(Findings, Vec<ScanError>)> {
    scan_tree_with_workspace_threshold(
        config,
        scope,
        explicit_scope,
        workspace_targets,
        PARALLEL_SCAN_MIN_FILES,
        overlays,
    )
}

fn add_overlay_scan_files(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    overlays: &TextOverlays,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    if overlays.is_empty() {
        return Ok(());
    }
    let roots = scan_roots(config, scope, explicit_scope)?;
    for path in overlays.keys() {
        if !is_scannable(path, config) {
            continue;
        }
        let path = normalize_path_lexically(path);
        if !roots.iter().any(|root| path_starts_with(&path, root)) {
            continue;
        }
        if files.iter().any(|file| paths_same_location(file, &path)) {
            continue;
        }
        if path.exists() || !new_overlay_file_passes_walk_filters(config, &roots, &path) {
            continue;
        }
        files.push(path);
    }
    files.sort_by_key(|path| sort_path_key(path));
    Ok(())
}

fn new_overlay_file_passes_walk_filters(config: &Config, roots: &[PathBuf], path: &Path) -> bool {
    roots.iter().any(|root| {
        let root = canonicalize_existing_prefix(root);
        let path = canonicalize_existing_prefix(path);
        if path == root || !path.starts_with(&root) {
            return false;
        }
        if config
            .workspace_boundary_roots
            .iter()
            .any(|boundary| path_starts_with(&path, boundary))
        {
            return false;
        }
        let Ok(relative) = path.strip_prefix(&root) else {
            return false;
        };
        let components: Vec<_> = relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect();
        if components
            .iter()
            .any(|component| component.starts_with('.'))
        {
            return false;
        }
        if components
            .iter()
            .take(components.len().saturating_sub(1))
            .any(|component| config.exclude.iter().any(|excluded| excluded == component))
        {
            return false;
        }
        let e2e_cases_root = config
            .kinds
            .iter()
            .find(|kind| kind.kind == "E2E" && kind.citable)
            .and_then(|kind| kind.folder.as_deref())
            .map(|folder| config.root.join(folder));
        let mut ancestor = path.parent();
        while let Some(dir) = ancestor {
            if dir == root {
                break;
            }
            if is_direct_e2e_case_dir(dir, e2e_cases_root.as_deref(), config) {
                return false;
            }
            ancestor = dir.parent();
        }
        !path_ignored_by_gitignore(config, &root, &path)
    })
}

fn path_ignored_by_gitignore(config: &Config, root: &Path, path: &Path) -> bool {
    if !config.respect_gitignore {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let mut dirs = Vec::new();
    let mut cursor = Some(parent);
    while let Some(dir) = cursor {
        dirs.push(dir);
        if dir == root {
            break;
        }
        cursor = dir.parent();
    }
    dirs.reverse();
    let mut ignored = false;
    for dir in dirs {
        let gitignore = dir.join(".gitignore");
        if !gitignore.is_file() {
            continue;
        }
        let mut builder = ignore::gitignore::GitignoreBuilder::new(dir);
        if builder.add(&gitignore).is_some() {
            continue;
        }
        let Ok(matcher) = builder.build() else {
            continue;
        };
        let matched = matcher.matched_path_or_any_parents(path, false);
        if matched.is_ignore() {
            ignored = true;
        } else if matched.is_whitelist() {
            ignored = false;
        }
    }
    ignored
}

fn path_starts_with(path: &Path, root: &Path) -> bool {
    let path = canonicalize_existing_prefix(path);
    let root = canonicalize_existing_prefix(root);
    path == root || path.starts_with(root)
}

fn canonicalize_existing_prefix(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        normalize_path_lexically(path)
    } else {
        std::env::current_dir()
            .map(|cwd| normalize_path_lexically(&cwd.join(path)))
            .unwrap_or_else(|_| normalize_path_lexically(path))
    };
    if let Ok(canonical) = fs::canonicalize(&path) {
        return canonical;
    }
    let mut suffix = PathBuf::new();
    let mut cursor = path.as_path();
    while !cursor.exists() {
        let Some(name) = cursor.file_name() else {
            break;
        };
        suffix = Path::new(name).join(suffix);
        let Some(parent) = cursor.parent() else {
            break;
        };
        cursor = parent;
    }
    fs::canonicalize(cursor)
        .unwrap_or_else(|_| normalize_path_lexically(cursor))
        .join(suffix)
}

fn overlay_text<'a>(overlays: &'a TextOverlays, path: &Path) -> Option<&'a str> {
    overlays
        .get(&normalize_path_lexically(path))
        .or_else(|| overlays.get(path))
        .map(String::as_str)
}

/// Scan helper for point-query subcommands (`show`, `id`): any unreadable file
/// is fatal — a partial view of the tree could miss the declaration entirely or
/// allocate a colliding number (§FS-show.3, §FS-id.4).
fn scan_tree_strict(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Findings> {
    let (findings, errors) = scan_tree(config, scope, explicit_scope)?;
    if let Some((path, message)) = errors.into_iter().next() {
        return Err(anyhow!("{}: {}", display_path(config, &path), message));
    }
    Ok(findings)
}
