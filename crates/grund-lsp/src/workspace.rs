/// Resolve an editor folder to the scan root it names. A discovered config owns
/// the scan, including sibling `[scan] include` roots; only a zero-config folder
/// remains its own boundary (§FS-lsp.2.2).
fn project_root(folder: &Path) -> Result<PathBuf> {
    let config = effective_config(folder)?;
    if config.config_file.is_some() {
        Ok(canonical_snapshot_path(&config.root))
    } else {
        Ok(canonical_snapshot_path(folder))
    }
}

struct ProjectSnapshot {
    root: PathBuf,
    snapshot: LspSnapshot,
    /// The directories `snapshot.scanned_files` lie in, so a file created since
    /// the last scan is still recognized as this project's — a new file under a
    /// symlinked or parent-relative `[scan] include` has no root prefix and is
    /// in no scan yet, and would otherwise look like nobody's (§FS-lsp.2.2).
    scanned_dirs: BTreeSet<PathBuf>,
}

impl ProjectSnapshot {
    fn new(root: PathBuf, snapshot: LspSnapshot) -> Self {
        let scanned_dirs = snapshot
            .scanned_files
            .iter()
            .filter_map(|file| file.parent().map(Path::to_path_buf))
            .collect();
        Self {
            root,
            snapshot,
            scanned_dirs,
        }
    }

    /// Whether this project's scan may need rebuilding for `path` (already
    /// canonicalized). The directory fallback deliberately over-approximates
    /// newly created files; ownership below never uses that approximation
    /// (§FS-lsp.2.2).
    fn might_cover(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
            || self.snapshot.scanned_files.contains(path)
            || path
                .parent()
                .is_some_and(|parent| self.scanned_dirs.contains(parent))
    }

    /// How specific this project's claim on `path` is, for picking one owner
    /// among the projects that cover it (§FS-lsp.2.2). A containing root beats
    /// a bare `[scan] include` reach, and the deepest root wins among nested
    /// project trees.
    fn ownership_rank(&self, path: &Path) -> Option<(u8, usize)> {
        if path.starts_with(&self.root) {
            Some((2, self.root.components().count()))
        } else if self.snapshot.scanned_files.contains(path) {
            Some((1, self.root.components().count()))
        } else {
            None
        }
    }
}
