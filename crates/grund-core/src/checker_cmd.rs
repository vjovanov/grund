/// `grund check [path] [--format text|json]`: scan the tree, run the checker
/// (§FS-check), print the report, and exit `0` clean
/// / `1` on a finding / `2` on a CLI or I/O error (§FS-check.2.1, §FS-cli.5).
fn command_check(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut format_override = None;
    let mut require_grounding = false;
    let mut include_suggestions = false;
    let mut full = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            other if other.starts_with("--format=") => {
                format_override = Some(other.trim_start_matches("--format=").to_string());
            }
            "--format" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --format requires a value");
                    return ExitCode::from(2);
                }
                format_override = Some(args[idx].clone());
            }
            "--require-grounding" => require_grounding = true,
            "--full" => full = true,
            "--suggestions" => include_suggestions = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                // §FS-cli.3: a path-taking subcommand accepts at most one path;
                // a second positional is a CLI error, never a silent drop.
                if path_provided {
                    eprintln!("error: check takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    if let Some(format) = &format_override
        && !matches!(format.as_str(), "text" | "json")
    {
        eprintln!("error: unsupported check format `{format}`");
        return ExitCode::from(2);
    }
    let run = match run_check(&path, path_provided, require_grounding, full) {
        Ok(run) => run,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = format_override.unwrap_or_else(|| run.config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported check format `{format}`");
        return ExitCode::from(2);
    }
    if format == "json" {
        print_json_report(&run.config, &run.report, include_suggestions);
    } else {
        print_report(&run.config, &run.report, include_suggestions);
    }
    if run.had_scan_errors {
        ExitCode::from(2)
    } else if run.report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// §FS-check.4.5: whether a walk that read files matched nothing in them. It asks
/// *recognized*, not *declared* — a project that only cites another project's
/// specs (§FS-workspace.1) declares nothing and is working as intended, so one
/// citation anywhere answers the question and the caution stays quiet.
fn nothing_recognized(findings: &Findings) -> bool {
    !findings.scanned_files.is_empty()
        && findings.declarations.is_empty()
        && findings.citations.is_empty()
}

/// The one caution a walk earns for what it did *not* find: §FS-check.2.2 when it
/// read no files, §FS-check.4.5 when it read files and matched nothing in them.
/// At most one — the empty scan is asked first, because a walk that read nothing
/// had nothing to recognize.
///
/// One decision for every surface that runs the engine: `grund check`, the
/// workspace loop beside it, and the LSP snapshot (§FS-lsp.4, `check_workspace_context`).
/// Spelled once per surface it was already wrong once — the LSP kept the empty
/// scan and never grew the second caution, so an editor and a terminal disagreed
/// about one tree.
///
/// `report_is_silent` is the caller's own answer to "did this project report
/// anything about its configured scope?" — findings and unreadable files both.
/// The out-of-scope tier (§FS-check.3.14) is deliberately not part of it: those
/// are findings about the tree *outside* the scope, and a run that finds the
/// citations out there is exactly the one where saying the configured scope is
/// empty helps most.
fn scan_scope_caution(
    config: &Config,
    findings: &Findings,
    path: &Path,
    path_provided: bool,
    report_is_silent: bool,
) -> Option<Diagnostic> {
    if !report_is_silent {
        return None;
    }
    if findings.scanned_files.is_empty() {
        return Some(empty_scan_warning(config, path, path_provided));
    }
    // §FS-check.4.5: only a run over the whole project makes the claim. A narrowed
    // `grund check <dir>` is a slice the caller chose, and a slice with no
    // declarations and no citations is an answer, not a misconfiguration.
    (nothing_recognized(findings) && scope_is_config_root(config, path, path_provided))
        .then(|| nothing_recognized_warning(config, findings.scanned_files.len()))
}

struct CheckRun {
    config: Config,
    report: CheckReport,
    had_scan_errors: bool,
}

/// One `grund check` run over `path`: resolve the config, scan, check, and fold in
/// the cautions and warnings the run earns for its scope and its config files.
///
/// Why the scope caution is only a warning: it never changes the exit code. The
/// agent-entrypoint check (§FS-check.3.5) runs even when no source file is scanned,
/// so a missing or stale `AGENTS.md` block still reports normally and suppresses
/// both cautions.
fn run_check(
    path: &Path,
    path_provided: bool,
    force_require_grounding: bool,
    full: bool,
) -> Result<CheckRun> {
    let mut config = resolve_workspace_config(path)?;
    // §FS-check.1.3: `--full` cancels `[scan] include` for the walk. It is a
    // per-run flag, never a config key (§DF-check-full-scope.2.5).
    config.scan_full = full;
    // §FS-check.1: the flag and `[reference] require_grounding` are one knob, so
    // it sets the same global default — it never turns the key off, and a
    // `[[kinds]]` row that says `false` stays exempt under it (§FS-config.3.4.8).
    if force_require_grounding {
        config.require_grounding = true;
    }
    if config.workspace_declared && scope_is_config_root(&config, path, path_provided) {
        return run_workspace_check(config, force_require_grounding, full);
    }

    let (mut findings, scan_errors) = scan_tree(&config, Some(path), path_provided)?;
    // §FS-check.1.3 / §FS-check.3.14: read the out-of-scope tier off the whole
    // `--full` walk first, then narrow the findings back to the configured scope
    // so every other rule reports exactly what a run without the flag reports.
    let scope = configured_scope(&config, path, path_provided, full)?;
    let out_of_scope =
        out_of_scope_references(&findings, &config, &BTreeMap::new(), scope.as_ref());
    retain_findings_in_scope(&mut findings, scope.as_ref());
    let mut report = check_findings(&findings, &config);
    let had_scan_errors = append_scan_errors(&mut report, scan_errors);
    // §FS-check.2.2 / §FS-check.4.5: a walk that read no files, or read them and
    // recognized nothing in them, is almost always a misconfigured scope rather
    // than a clean repo — say so on stderr instead of exiting 0 in silence.
    let report_is_silent = report.errors.is_empty() && report.warnings.is_empty();
    report.warnings.extend(scan_scope_caution(
        &config,
        &findings,
        path,
        path_provided,
        report_is_silent,
    ));
    // §FS-check.4.3, after the scope caution above and deliberately outside
    // `report_is_silent`: the two are independent, and a repository mid-migration
    // must not lose the scope diagnostic just because it also has a config pair.
    report.warnings.extend(redundant_config_warning(&config));
    // §FS-config.4.1: the deprecated `[[kinds]] prefix` spelling, named once per
    // config and on the same channel — it is a fact about the file this run
    // read, and the run is where a repository notices it.
    report
        .warnings
        .extend(deprecated_kind_prefix_warning(&config));
    // §FS-check.1.3, also after the scope caution: `--full` cancels `[scan] include`,
    // and an explicit path other than the config root already bypasses that key — so
    // the flag changed nothing, and the caller who typed it wanted a wider search.
    report
        .warnings
        .extend(full_scope_ignored_warning(&config, path, path_provided, full));
    // §FS-check.4.9: the blocks this walk met that no enclosing one lists. A report
    // warning, not a line printed past it: that is what stands it in place of
    // `success` (§FS-check.2.1) and makes §DF-unlisted-workspace-block.2.1's ramp work.
    report.warnings.extend(unlisted_workspace_block_warnings(
        &config,
        &config,
        None,
        &findings.walked_dirs,
    ));
    // §FS-check.3.14, after the scope caution above (§FS-check.2.2, §FS-check.4.5):
    // a `--full` run whose *configured* scope read or recognized nothing still earns
    // that caution — the tier says where the citations are, the config was not told.
    report.errors.extend(out_of_scope);
    sort_diagnostics(&mut report.errors);

    Ok(CheckRun {
        config,
        report,
        had_scan_errors,
    })
}

/// The workspace arm of `grund check`: every member checked in turn, its findings
/// anchored on the workspace root's config, with the per-project cautions and the
/// per-config warnings folded into one report.
///
/// Why the root is skipped in the per-project warning loop at the end: with
/// `include_root = true` the root is itself a `projects` entry, so warning once per
/// project would name the root's directory twice.
fn run_workspace_check(
    mut root_config: Config,
    force_require_grounding: bool,
    full: bool,
) -> Result<CheckRun> {
    let mut projects = load_workspace_projects(&mut root_config)?;
    // §FS-check.3.5: `--require-grounding` propagates to every member's
    // config. The flag only affects checking, not scanning, so applying it
    // after the load is equivalent to setting it before.
    if force_require_grounding {
        for project in &mut projects {
            project.config.require_grounding = true;
        }
    }
    // §FS-check.1.3: `include` is a per-project statement, so each project's
    // walk is widened past its own and tiered against its own configured scope.
    let scopes = projects
        .iter()
        .map(|project| configured_scope(&project.config, &project.config.root, true, full))
        .collect::<Result<Vec<_>>>()?;
    let out_of_scope = workspace_out_of_scope_references(&projects, &scopes);
    for (project, scope) in projects.iter_mut().zip(&scopes) {
        retain_findings_in_scope(&mut project.findings, scope.as_ref());
    }

    let workspace = projects
        .iter()
        .map(|project| {
            (
                project.alias.clone(),
                WorkspaceCheckTarget {
                    findings: &project.findings,
                    config: &project.config,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut report = CheckReport::default();
    let mut had_scan_errors = false;
    for project in &projects {
        let mut project_report = check_with_workspace(
            &project.findings,
            &project.config,
            // §FS-workspace.8.1: the report is rendered from the workspace root,
            // so a path a member's message names is spelled from there too.
            &root_config,
            Some(&project.alias),
            &workspace,
        );
        let project_has_findings =
            !project_report.errors.is_empty() || !project_report.warnings.is_empty();
        report.errors.append(&mut project_report.errors);
        report.warnings.append(&mut project_report.warnings);
        report.suggestions.append(&mut project_report.suggestions);
        had_scan_errors |= append_scan_errors(&mut report, project.scan_errors.iter().cloned());
        // §FS-check.2.2 / §FS-check.4.5 / §FS-workspace.5: the same two cautions as
        // the single-project path, asked per project — one member's empty scope or
        // grammar mismatch says nothing about another's, and each names its own.
        report.warnings.extend(scan_scope_caution(
            &project.config,
            &project.findings,
            &project.config.root,
            true,
            project.scan_errors.is_empty() && !project_has_findings,
        ));
    }
    // §FS-workspace.2.2: a block whose only members may be absent contributes no
    // project, which is not an error — but a run left with nothing to read still
    // says so, beside the announcement below (§FS-check.2.2).
    if projects.is_empty() {
        report.warnings.extend(scan_scope_caution(
            &root_config,
            &Findings::default(),
            &root_config.root,
            true,
            true,
        ));
    }
    // §FS-check.4.10: the namespaces this run did not read, one located warning
    // each on stdout at the `optional_members` entry that made the skip legal. It
    // is what buys the green exit — the exit code says nothing about coverage here,
    // so the report must — and it withholds `success` like any other warning
    // (§FS-check.2.1).
    report
        .warnings
        .extend(absent_optional_member_warnings(&root_config));
    // §FS-check.4.3: the root's pair plus every member's, each named at the path
    // that project's config was loaded under. The root is skipped in the loop
    // below, because the warning is about one directory, not one scope.
    report.warnings.extend(redundant_config_warning(&root_config));
    report
        .warnings
        .extend(deprecated_kind_prefix_warning(&root_config));
    for project in &projects {
        if project.config.root != root_config.root {
            report.warnings.extend(redundant_config_warning(&project.config));
            report
                .warnings
                .extend(deprecated_kind_prefix_warning(&project.config));
        }
    }
    // §FS-check.4.9: per project — the candidates are what *that* walk reached, and
    // the absorbing namespace is its own. Rendered against the workspace root like
    // every other message here (§FS-workspace.8.1).
    for project in &projects {
        report.warnings.extend(unlisted_workspace_block_warnings(
            &project.config,
            &root_config,
            Some(&project.alias),
            &project.findings.walked_dirs,
        ));
    }
    report.errors.extend(out_of_scope);
    sort_diagnostics(&mut report.errors);
    sort_diagnostics(&mut report.warnings);
    sort_diagnostics(&mut report.suggestions);

    Ok(CheckRun {
        config: root_config,
        report,
        had_scan_errors,
    })
}

fn append_scan_errors(
    report: &mut CheckReport,
    scan_errors: impl IntoIterator<Item = (PathBuf, String)>,
) -> bool {
    let mut had_scan_errors = false;
    for (file, message) in scan_errors {
        had_scan_errors = true;
        // A file that could not be read mid-walk is reported as a CLI-shaped
        // `error: <path>: <reason>` finding (§FS-check.2): the walk continued,
        // the findings below are real, but the view of the tree was incomplete.
        report.errors.push(Diagnostic {
            code: "io",
            path: Some(file),
            line: None,
            column: None,
            message,
            sites: Vec::new(),
        });
    }
    had_scan_errors
}
