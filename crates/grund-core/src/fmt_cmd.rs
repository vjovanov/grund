/// §FS-fmt.6.6: whether this invocation turns the cross-reference pass on by
/// itself — `[fmt.cross_refs] enabled`, a `--write` scope, and at least one
/// Markdown file in it. It answers a question about the *command*, so it lives
/// beside the command surface rather than in the rewrite walk.
///
/// The compatibility CLI's `fmt` command: flags, the per-project walk, the report
/// on stdout, and the exit code (§FS-fmt.1, §FS-fmt.3). It sits beside `fmt.rs`
/// for the reason `config_cmd.rs` sits beside `config.rs` — that file is the
/// normalizer, this one is the command surface wrapped around it, and only this
/// one knows about argv, stdout, and `ExitCode`.
fn auto_cross_refs_for_scope(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    write: bool,
) -> Result<bool> {
    if !write || !config.fmt_cross_refs_enabled {
        return Ok(false);
    }
    scope_contains_markdown(config, scope, explicit_scope)
}

fn scope_contains_markdown(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<bool> {
    Ok(walk_scannable_files(config, scope, explicit_scope)?
        .iter()
        .any(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md")))
}

/// A project's already-computed findings from `load_workspace_context`,
/// reusable as `fmt_tree`'s `precomputed_findings` only when the scan that
/// produced them met no error (§FS-fmt.3). Resolving a cross-reference or a
/// shorthand against a partial declaration set can name the wrong
/// declaration — the hazard `fmt_findings_or_abort` already guards a fresh
/// scan against — so a caller that reuses a scan instead of running one must
/// check the same field a fresh scan would have failed on, not just borrow
/// the `Findings` beside it.
fn usable_findings(project: &WorkspaceProject) -> Option<&Findings> {
    project.scan_errors.is_empty().then_some(&project.findings)
}

/// Run the `fmt` command: parse the flags, walk each project in scope, print the
/// report, and map the exit code.
///
/// Why a workspace-root run walks every project: it wraps qualified citations in
/// *every* project's files, so a heading rename in `api` triggers a `fmt` diff in
/// any sibling project that wrapped a citation of the renamed declaration. Each
/// project's `[scan] include`, `[scan] exclude`, and anchor profile still applies
/// to its own files.
///
/// Why each project's findings are passed through: `fmt --cross-refs` would
/// otherwise re-scan every project. Where a project's set is withheld — its scan
/// met an error — `fmt_tree` falls back to a fresh scan and hits the same
/// `fmt_findings_or_abort` refusal an explicit path already gets on this tree.
///
/// Why a scope-narrowed run does not reuse the context's findings: a scope-narrow
/// scan is too thin for cross-file wrap targets, so `fmt_tree` scans the project
/// itself. Requiring the whole-project scan to be error-free keeps `--write` and
/// `--write .` refusing alike on the same tree, instead of one silently resolving
/// against a set the other just reported as incomplete.
fn command_fmt(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut write = false;
    let mut check_flag = false;
    let mut marker = false;
    let mut cross_refs = false;
    for arg in args {
        match arg.as_str() {
            "--check" => check_flag = true,
            "--write" => write = true,
            "--marker" => marker = true,
            "--cross-refs" => cross_refs = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path_provided {
                    eprintln!("error: fmt takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
    }
    if write && check_flag {
        eprintln!("error: --check and --write cannot be used together");
        return ExitCode::from(2);
    }
    // §FS-fmt.3 / §AR-bindings.2: the compatibility adapter uses the same
    // workspace-wide strict preflight as the public API. Keeping a second
    // project loop here could let its mutation and aggregation ordering drift.
    let output = match format_references(FmtOpts {
        path,
        path_provided,
        write,
        add_marker: marker,
        cross_refs,
    }) {
        Ok(output) => output,
        Err(err) => {
            print_fmt_error(&err);
            return ExitCode::from(2);
        }
    };
    for path in &output.refused_writes {
        eprintln!("warning: {path}: not rewritten: the symlink target is outside the config root");
    }
    // §FS-fmt.3 / §FS-errors.1: the report is `fmt`'s output — on stdout, the
    // same stream `grund check`'s findings use, not the stderr transcript shape
    // `grund init` uses (§FS-errors.6). Only CLI-level `error:` lines go to stderr.
    if write {
        let mut files = output
            .changes
            .iter()
            .map(|change| change.path.clone())
            .collect::<Vec<_>>();
        files.sort();
        files.dedup();
        println!(
            "rewrote {} reference{}{}",
            output.changes.len(),
            if output.changes.len() == 1 { "" } else { "s" },
            if files.is_empty() { "" } else { ":" }
        );
        for path in &files {
            let count = output
                .changes
                .iter()
                .filter(|change| &change.path == path)
                .count();
            println!("  {path} ({count})");
        }
    } else {
        for change in &output.changes {
            println!("{}:{}: {}", change.path, change.line, change.label);
        }
    }
    if !output.scan_errors.is_empty() {
        // Partial-scan semantics (§FS-fmt.3 / §FS-check.2): what was rewritten is
        // real — and, under `--write`, already on disk — but the tree the rewrite
        // ran over was not the whole tree.
        for error in &output.scan_errors {
            eprintln!("error: {}: {}", error.path, error.message);
        }
        return ExitCode::from(2);
    }
    if write || output.changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Render the compatibility CLI's fatal formatter error. Strict scan aborts
/// stay structured through the API boundary, so each path gets its own CLI
/// prefix and the same unmistakable refusal as the published CLI (§FS-fmt.3).
fn print_fmt_error(err: &anyhow::Error) {
    if let Some(abort) = err.downcast_ref::<FmtScanAbort>() {
        for error in &abort.scan_errors {
            eprintln!(
                "error: nothing was rewritten: {}: {}",
                error.path, error.message
            );
        }
    } else {
        eprintln!("error: {err:#}");
    }
}
