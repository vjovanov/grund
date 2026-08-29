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
    // §FS-workspace.8.5: a workspace-root run loads every member so qualified
    // `§<alias>/<ID>` citations can be wrapped; a member-local run, or a
    // single-project repo, collapses to one project with its wraps preserved.
    let context = match load_workspace_context(&path, path_provided) {
        Ok(context) => context,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let config = context.render_config().clone();
    let explicit_cross_refs = cross_refs;
    // §FS-workspace.8.5: walk each project's tree under its own config, but
    // share one workspace handle so a qualified `§<alias>/<ID>` resolves
    // through `WorkspaceContext`.
    let workspace_for_wrap = if context.workspace_loaded {
        Some(&context)
    } else {
        None
    };
    let mut changes: Vec<(PathBuf, usize, String)> = Vec::new();
    // §FS-fmt.3: a path the walk could not read, already rendered against the
    // project whose walk met it. Reported after the report, exit `2`.
    let mut scan_errors: Vec<ApiScanError> = Vec::new();
    // §FS-fmt.2.3.2: the files the rewrite would not write through, named on
    // stderr here rather than from the engine — the same place every other path
    // this command prints is turned into a line.
    let mut refused_writes: Vec<String> = Vec::new();
    // §FS-workspace.8.5: with no explicit path (or `path == workspace root`),
    // walk every project so cross-project wraps are emitted across the whole
    // repo; an explicit path narrows the walk to the project holding it.
    let walk_all_projects = context.workspace_loaded
        && (!path_provided
            || fs::canonicalize(&path)
                .map(|canonical| canonical == config.root)
                .unwrap_or(false));
    if walk_all_projects {
        // §AR-workspace.8: `load_workspace_context` already produced each
        // project's findings, so pass them through — but only where that scan
        // met no error, which `usable_findings` decides (§FS-fmt.3).
        for project in &context.projects {
            let auto_cross_refs =
                match auto_cross_refs_for_scope(&project.config, Some(&project.config.root), true, write) {
                    Ok(enabled) => enabled,
                    Err(err) => {
                        eprintln!("error: {err:#}");
                        return ExitCode::from(2);
                    }
                };
            let cross_refs = explicit_cross_refs || auto_cross_refs;
            let opts = FmtRunOpts {
                add_marker: marker,
                cross_refs,
                write,
                render: &config,
                workspace: workspace_for_wrap,
                precomputed_findings: usable_findings(project),
                // §FS-fmt.6.1: the index is linkified whatever the toggle says.
                index_cross_refs: write || explicit_cross_refs,
            };
            match fmt_tree(
                &project.config,
                Some(&project.config.root),
                true,
                &opts,
            ) {
                Ok(mut walked) => {
                    changes.append(&mut walked.changes);
                    scan_errors.append(&mut walked.scan_errors);
                    refused_writes.append(&mut walked.refused_writes);
                }
                Err(err) => {
                    print_fmt_error(&err);
                    return ExitCode::from(2);
                }
            }
        }
    } else {
        // Single-project / member-local / scope-narrowed: reuse the context's
        // findings only where they cover the whole project (the implicit "."
        // scope) and that scan met no error, per `usable_findings` (§FS-fmt.3).
        let reusable_findings = (!path_provided)
            .then(|| context.current_project())
            .flatten()
            .and_then(usable_findings);
        let auto_cross_refs =
            match auto_cross_refs_for_scope(&config, Some(&path), path_provided, write) {
                Ok(enabled) => enabled,
                Err(err) => {
                    eprintln!("error: {err:#}");
                    return ExitCode::from(2);
                }
            };
        let cross_refs = explicit_cross_refs || auto_cross_refs;
        let opts = FmtRunOpts {
            add_marker: marker,
            cross_refs,
            write,
            render: &config,
            workspace: workspace_for_wrap,
            precomputed_findings: reusable_findings,
            index_cross_refs: write || explicit_cross_refs,
        };
        match fmt_tree(&config, Some(&path), path_provided, &opts) {
            Ok(walked) => {
                changes = walked.changes;
                scan_errors = walked.scan_errors;
                refused_writes = walked.refused_writes;
            }
            Err(err) => {
                print_fmt_error(&err);
                return ExitCode::from(2);
            }
        }
    }
    for path in &refused_writes {
        eprintln!("warning: {path}: not rewritten: the symlink target is outside the config root");
    }
    // §FS-fmt.3 / §FS-errors.1: the report is `fmt`'s output — on stdout, the
    // same stream `grund check`'s findings use, not the stderr transcript shape
    // `grund init` uses (§FS-errors.6). Only CLI-level `error:` lines go to stderr.
    if write {
        let mut files: Vec<PathBuf> = changes.iter().map(|(path, _, _)| path.clone()).collect();
        files.sort_by_key(|path| sort_path_key(path));
        files.dedup();
        println!(
            "rewrote {} reference{}{}",
            changes.len(),
            if changes.len() == 1 { "" } else { "s" },
            if files.is_empty() { "" } else { ":" }
        );
        for path in &files {
            let count = changes.iter().filter(|(p, _, _)| p == path).count();
            println!("  {} ({})", display_path(&config, path), count);
        }
    } else {
        for (path, line, label) in &changes {
            println!("{}:{}: {}", display_path(&config, path), line, label);
        }
    }
    if !scan_errors.is_empty() {
        // Partial-scan semantics (§FS-fmt.3 / §FS-check.2): what was rewritten is
        // real — and, under `--write`, already on disk — but the tree the rewrite
        // ran over was not the whole tree.
        for error in &scan_errors {
            eprintln!("error: {}: {}", error.path, error.message);
        }
        return ExitCode::from(2);
    }
    if write || changes.is_empty() {
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
