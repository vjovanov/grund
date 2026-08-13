// Config *discovery*: which file governs a directory, and where the walk stops
// (§FS-config.1, §DF-config-file-location). Kept apart from the parser in
// `config.rs` because the two answer different questions — "which file" versus
// "what does this file say" — and only this half knows there are two names
// (§AR-core-module-layout.1).

/// The two names one directory may hold its config under, in probe order
/// (§FS-config.1): the bare root-visible `grund.toml` first, then
/// `.agents/grund.toml`. Fixed order, not a search — §DF-config-file-location.2.2
/// gives the tie to the form `grund init` generates, so the file a project is
/// told to write is the file that governs it.
const CONFIG_NAMES: [&[&str]; 2] = [&["grund.toml"], &[".agents", "grund.toml"]];

/// Every config name `dir` actually carries, in precedence order (§FS-config.1).
fn config_files_in(dir: &Path) -> impl Iterator<Item = PathBuf> + use<'_> {
    CONFIG_NAMES
        .iter()
        .map(|segments| segments.iter().fold(dir.to_path_buf(), |acc, s| acc.join(s)))
        .filter(|candidate| candidate.is_file())
}

/// The config file `dir` carries, or `None` — the one probe every discovery site
/// funnels through, so root and workspace member ask the same question
/// (§FS-config.1).
fn config_file_in(dir: &Path) -> Option<PathBuf> {
    config_files_in(dir).next()
}

/// The config `dir` ignores because the higher-precedence name outranks it
/// (§FS-config.1.1) — `Some` only when the directory carries both names, which is
/// the redundant pair `check` warns about (§FS-check.4.3).
fn redundant_config_file_in(dir: &Path) -> Option<PathBuf> {
    config_files_in(dir).nth(1)
}

/// Discover and load the effective config: walk upward from `start` for the
/// nearest directory carrying either config name (§FS-config.1), parse it over
/// the defaults (§FS-config.2), or fall back to the pure defaults if none is
/// found (§GOAL-zero-config).
fn load_config(start: &Path) -> Result<Config> {
    let start_dir = if start.is_file() {
        start.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        start.to_path_buf()
    };
    // Resolve to an absolute path before walking up, mirroring how `cargo` finds
    // `Cargo.toml` (§FS-config.1): a relative `.` or `subdir/` must still discover
    // a `grund.toml` in an ancestor directory.
    let walk_start = fs::canonicalize(&start_dir).unwrap_or(start_dir);
    let mut cursor = Some(walk_start.as_path());
    while let Some(dir) = cursor {
        // Both names are probed at *every* level before climbing, so a bare
        // member config shadows an ancestor's `.agents/` one exactly as a nested
        // `.agents/grund.toml` always did (§FS-config.1, §DF-config-file-location.2.1).
        if config_file_in(dir).is_some() {
            return load_config_at(dir, &walk_start);
        }
        cursor = dir.parent();
    }
    // Zero-config (§GOAL-zero-config): the "project root" is the current working
    // directory, never the path that happened to be passed on the command line —
    // so `[scan] include` resolves against the repo and `grund check src/` scopes
    // *into* it instead of looking for `src/docs`, `src/e2e`, `src/src`. Reports
    // stay relative to `cli_base` (the resolved path arg) when
    // `[output] relative_paths = false` (§FS-config.3.6).
    let root = std::env::current_dir()
        .ok()
        .and_then(|cwd| fs::canonicalize(&cwd).ok())
        .unwrap_or_else(|| walk_start.clone());
    let mut config = Config::default_for(root);
    config.cli_base = walk_start;
    Ok(config)
}

/// Load the config rooted at `root` (no upward walk), using `cli_base` for
/// path rendering. The one shared loader both upward discovery (`load_config`)
/// and direct workspace-member loading funnel through (§AR-workspace.5.1).
fn load_config_at(root: &Path, cli_base: &Path) -> Result<Config> {
    load_config_at_with_report_base(root, cli_base, None)
}

fn load_config_at_with_report_base(
    root: &Path,
    cli_base: &Path,
    report_base: Option<&Path>,
) -> Result<Config> {
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let candidate = config_file_in(&root);
    let mut config = if candidate.is_some() {
        Config::default_for_existing_config(root.clone())
    } else {
        Config::default_for(root.clone())
    };
    config.cli_base = cli_base.to_path_buf();
    // Report config errors against a stable relative path, never the
    // absolute discovered path (§FS-errors.4: deterministic, no absolute
    // paths outside the configured root).
    let report_relative = |path: &Path| {
        let base = report_base.unwrap_or(&root);
        path.strip_prefix(base)
            .map(Path::to_path_buf)
            .or_else(|_| path.strip_prefix(&root).map(Path::to_path_buf))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    // §FS-check.4.3: the loser of a two-name tie is recorded, not read, so every
    // surface that reports on the config can name the file grund ignored.
    config.redundant_config_file = redundant_config_file_in(&root).map(|path| report_relative(&path));
    if let Some(candidate) = candidate {
        let report_path = report_relative(&candidate);
        config.config_file = Some(report_path.clone());
        parse_config_file(&candidate, &report_path, &mut config)?;
    }
    Ok(config)
}
