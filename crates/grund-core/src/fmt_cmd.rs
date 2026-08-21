// The compatibility CLI's `fmt` command: flags, the per-project walk, the report
// on stdout, and the exit code (§FS-fmt.1, §FS-fmt.3). It sits beside `fmt.rs`
// for the reason `config_cmd.rs` sits beside `config.rs` — that file is the
// normalizer, this one is the command surface wrapped around it, and only this
// one knows about argv, stdout, and `ExitCode`.

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
    // §FS-workspace.8.5: a workspace-root run loads every member so that
    // qualified `§<alias>/<ID>` citations can be wrapped; a member-local
    // run (or a single-project repo) collapses to one project and the
    // wrapper preserves any existing qualified wraps unchanged.
    let context = match load_workspace_context(&path, path_provided) {
        Ok(context) => context,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let config = context.render_config().clone();
    let explicit_cross_refs = cross_refs;
    // §FS-workspace.8.5: a workspace-root run wraps qualified citations in
    // *every* project's files — a heading rename in `api` must trigger a
    // `fmt` diff in any sibling project that wrapped a citation of the
    // renamed declaration. Walk each project's tree under its own config
    // (each project's `[scan] include`, `[scan] exclude`, and anchor
    // profile applies to its own files) but share one workspace handle so
    // a qualified `§<alias>/<ID>` resolves through `WorkspaceContext`.
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
    // §FS-workspace.8.5: in workspace mode with no explicit path (or with
    // `path == workspace root`), walk every project so cross-project wraps
    // are emitted across the whole repo. An explicit path inside one
    // project's tree narrows the walk to that project; the workspace
    // context still lets `<§>alias/<ID>` resolve.
    let walk_all_projects = context.workspace_loaded
        && (!path_provided
            || fs::canonicalize(&path)
                .map(|canonical| canonical == config.root)
                .unwrap_or(false));
    if walk_all_projects {
        // Each project's findings were already produced by
        // `load_workspace_context` at project.root (§AR-workspace.8) — pass
        // them through so `fmt --cross-refs` does not re-scan every project.
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
                precomputed_findings: Some(&project.findings),
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
                    eprintln!("error: {err:#}");
                    return ExitCode::from(2);
                }
            }
        }
    } else {
        // Single-project / member-local / scope-narrowed: only reuse the
        // context's findings when they cover the whole project (the
        // implicit "." scope). A scope-narrow context scan is too thin for
        // cross-file wrap targets, so let `fmt_tree` scan the project
        // itself in that case.
        let reusable_findings = (!path_provided)
            .then(|| context.current_project().map(|project| &project.findings))
            .flatten();
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
        };
        match fmt_tree(&config, Some(&path), path_provided, &opts) {
            Ok(walked) => {
                changes = walked.changes;
                scan_errors = walked.scan_errors;
                refused_writes = walked.refused_writes;
            }
            Err(err) => {
                eprintln!("error: {err:#}");
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
