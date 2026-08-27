/// The per-file scan failure a walker error becomes (§FS-check.2), or `None` when
/// the walk was never going to read through the path it names (§FS-config.3.5.6).
///
/// What the walk does with a path it cannot read (§AR-scanner.1, §FS-check.2):
/// which walker errors become per-file scan failures, which stay silent because
/// the ordinary walk was never going to read the path anyway, and which link a
/// loop is reported at. It sits beside `scanner_walk.rs` because it is the other
/// half of the scanner category §AR-core-module-layout.1 names — the traversal
/// there, its scan error handling here — and the two meet only at the error list
/// one hands the other.
///
/// `follow_links` hands back an error in place of the entry for a link it cannot
/// resolve — a broken target, or a loop, which the `ignore` crate detects and
/// reports rather than recursing into. The walker applies neither its directory
/// filter nor its ignore files to an error entry, so the tests the link would
/// have faced are applied here by hand: the hidden-name and `[scan] exclude`
/// tests for a looping directory, the ignore files for either kind, and
/// `[scan] extensions` as well for a broken link, which names a file where a loop
/// names a directory. That is what keeps a dangling `docs/logo.png`, and a link
/// of either kind that `.gitignore` covers, silent — the walk was never going to
/// read them. Anything else — an unreadable directory, a malformed ignore file —
/// is reported as it stands, at the walk root when the error names no path of its
/// own: it is a hole in the walk either way, and a silent one is what
/// §REQ-no-missed-citation.1 rules out.
fn walk_error_report(
    err: &ignore::Error,
    config: &Config,
    scan_root: &Path,
) -> Option<ScanError> {
    if let Some((child, ancestor)) = walk_error_loop(err) {
        // §FS-config.3.5.2: the finding names the link, and the path the walker
        // hands back is not always it — a loop met one link deep is reported at
        // the doubled spelling the descent produced.
        let link = shortest_link_spelling(child, scan_root).unwrap_or_else(|| child.to_path_buf());
        // Naming the ancestor is the useful half of the message, but it says
        // nothing when the ancestor *is* the link: the target then reaches back
        // over the walk root, which no in-tree name below it describes.
        let ancestor = (!ancestor.starts_with(&link)).then_some(ancestor);
        return symlink_loop_report(&link, ancestor, config);
    }
    let path = walk_error_path(err).unwrap_or(scan_root);
    if !fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Some((path.to_path_buf(), walk_error_reason(err)));
    }
    if !is_scannable(path, config) || !walk_would_have_read(path, config) {
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

/// The scan error a looping directory link earns (§FS-config.3.5.5), or `None`
/// where the walk was never going to read through it: the hidden-name and
/// `[scan] exclude` tests the walker's own directory filter never applies to an
/// error entry, and the ignore files, which reach a link the walker could not
/// resolve no other way.
///
/// `ancestor` is the directory the link reaches back into, and `None` where the
/// target reaches over the walk root, which no in-tree name below it describes. Shared by the two ways a loop is found: the walker's
/// own detection, and the filter pruning a link whose target is at or above the
/// walk root — which the walker cannot see without descending a second copy of
/// the tree first (§AR-scanner.1).
///
/// Why the ancestor name can be missing: the config root renders as nothing at
/// all, and a target at or above the walk root has no in-tree name below it
/// either. Both read the same way — the target is not somewhere inside the tree,
/// it is the tree.
fn symlink_loop_report(link: &Path, ancestor: Option<&Path>, config: &Config) -> Option<ScanError> {
    let name = link.file_name().and_then(|name| name.to_str())?;
    if is_hidden(link)
        || config.exclude.iter().any(|item| item == name)
        || !walk_would_have_read(link, config)
    {
        return None;
    }
    // Naming the directory is the useful half of the message, and it is available
    // only where that directory has a name *in the report* — the config root
    // renders as nothing at all (§FS-config.3.6).
    let ancestor = ancestor
        .map(|ancestor| display_path(config, ancestor))
        .filter(|name| !name.is_empty());
    let reason = match ancestor {
        Some(name) => format!("symlink loop: the target is the ancestor directory {name}"),
        None => "symlink loop: the target contains the link".to_string(),
    };
    Some((link.to_path_buf(), reason))
}

/// The link `path` names, spelled as the walk root reaches it (§FS-config.3.5.2).
/// A walk that descends through a link meets one link under two spellings —
/// `docs/up -> ..` is met again as `docs/up/docs/up`, and `docs/a/link -> ../b`
/// with `docs/b/link -> ../a` is met as `docs/a/link/link` — and the one to
/// report is the name the user can go and fix.
///
/// The link is identified by the canonical *directory* holding it plus its own
/// name, so a link two spellings deep is recognized as the different link it is:
/// with `docs/shared -> ../shared-docs` and a `self -> .` inside it, the loop is
/// at `docs/shared/self` and `docs/shared` is innocent. That identity is then
/// re-expressed under the walk root, which is what finds the *sibling* spelling
/// a prefix search cannot: `docs/a/link/link` is `docs/b/link`, and no prefix of
/// the reported path is. A link whose canonical directory lies outside the walk
/// root has no in-tree name of its own, so the reported path stands. Only the
/// error path pays for any of it.
fn shortest_link_spelling(path: &Path, scan_root: &Path) -> Option<PathBuf> {
    let identity = link_identity(path)?;
    let canonical_root = fs::canonicalize(scan_root).ok()?;
    let relative = identity.strip_prefix(&canonical_root).ok()?;
    let candidate = scan_root.join(relative);
    // The canonical directory may be reached under several in-tree names; this
    // one is the walk root's own, and it counts only if it really is the link.
    (link_identity(&candidate)? == identity).then_some(candidate)
}

/// Which directory entry a path names, independent of how the walk spelled the
/// directories above it: the canonical parent plus the final component. Unlike
/// `canonicalize` on the whole path, this does not resolve the entry itself, so a
/// link is identified as the link rather than as its target.
fn link_identity(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?;
    Some(fs::canonicalize(path.parent()?).ok()?.join(name))
}

/// Whether the walk would have read the file at `path` had the link resolved —
/// the ignore-file half of that question, which `is_scannable` cannot answer
/// (§FS-config.3.5.6, §AR-scanner.1.1).
///
/// `.gitignore` and friends are applied to the entries the walker *yields*, and a
/// link it cannot resolve arrives as an error instead, so an ignored generated
/// file would otherwise earn a scan error about a path the ordinary walk was
/// never going to look at. Re-walking the link's own directory one level deep,
/// without following links, answers it: there the link is an ordinary entry and
/// the ignore rules do apply to it. Only a positive "the walker filtered this
/// out" suppresses the report — an unreadable parent or no parent at all reports,
/// because a silent skip is the failure §REQ-no-missed-citation.1 rules out. The
/// cost is one directory read, and only for a link that is already broken.
fn walk_would_have_read(path: &Path, config: &Config) -> bool {
    if !config.respect_gitignore {
        return true;
    }
    let Some(parent) = path.parent() else {
        return true;
    };
    let mut builder = WalkBuilder::new(parent);
    builder.hidden(false).max_depth(Some(1)).follow_links(false);
    builder
        .build()
        .filter_map(std::result::Result::ok)
        .any(|entry| entry.path() == path)
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
