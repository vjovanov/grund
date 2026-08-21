// What the walk does with a path it cannot read (§AR-scanner.1, §FS-check.2):
// which walker errors become per-file scan failures, which stay silent because
// the ordinary walk was never going to read the path anyway, and which link a
// loop is reported at. It sits beside `scanner_walk.rs` because it is the other
// half of the scanner category §AR-core-module-layout.1 names — the traversal
// there, its scan error handling here — and the two meet only at the error list
// one hands the other.

/// The per-file scan failure a walker error becomes (§FS-check.2), or `None` when
/// the walk was never going to read through the path it names (§FS-config.3.5).
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
        // §FS-config.3.5: the finding names the link, and the path the walker
        // hands back is not always it. A link that leaves the walk root —
        // `docs/up -> ..` under `include = ["docs"]` — is not an ancestor when it
        // is met, so the walker descends a second copy of the tree and only
        // notices at `docs/up/docs/up`. The link the user has to fix is the first
        // one on that path.
        let link = shortest_link_spelling(child, scan_root).unwrap_or(child);
        let name = link.file_name().and_then(|name| name.to_str())?;
        if is_hidden(link)
            || config.exclude.iter().any(|item| item == name)
            || !walk_would_have_read(link, config)
        {
            return None;
        }
        // Naming the ancestor is the useful half of the message — `docs/self -> .`
        // reads "the target is the ancestor directory docs" — but it says nothing
        // when the ancestor *is* the link: the target then reaches back over the
        // walk root, which no in-tree name below it describes.
        let reason = if ancestor.starts_with(link) {
            "symlink loop: the target contains the link".to_string()
        } else {
            format!(
                "symlink loop: the target is the ancestor directory {}",
                display_path(config, ancestor)
            )
        };
        return Some((link.to_path_buf(), reason));
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

/// The shortest path below `scan_root` that names the same **link** as `path`
/// (§FS-config.3.5). A walk that descends through a link can reach one link under
/// two spellings — `docs/up -> ..` is met again as `docs/up/docs/up` — and the
/// one to report is the one the user has to fix.
///
/// A link two spellings deep is a different link, not the same one under a longer
/// name: with `docs/shared -> ../shared-docs` and a `self -> .` inside it, the
/// loop is at `docs/shared/self` and `docs/shared` is innocent. So the test is
/// identity, not "is any prefix a symlink": the canonical *directory* holding the
/// link, plus its name. Only the error path pays for it.
fn shortest_link_spelling<'a>(path: &'a Path, scan_root: &Path) -> Option<&'a Path> {
    let identity = link_identity(path)?;
    let relative = path.strip_prefix(scan_root).ok()?;
    let depth = relative.components().count();
    let mut prefix = scan_root.to_path_buf();
    for (index, component) in relative.components().enumerate() {
        prefix.push(component);
        if link_identity(&prefix).is_some_and(|candidate| candidate == identity) {
            // Re-derived as a slice of the caller's path, so the report is spelled
            // the way the walk spelled it.
            return path.ancestors().nth(depth - index - 1);
        }
    }
    None
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
/// (§FS-config.3.5, §AR-scanner.1.1).
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
