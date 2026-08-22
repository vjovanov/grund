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

struct CheckRun {
    config: Config,
    report: CheckReport,
    had_scan_errors: bool,
}

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
    // `--require-grounding` only ever turns the check on for this run; it never
    // turns off a `[reference] require_grounding = true` set in the config.
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
    // §FS-check.2.2: a walk that read no files and turned up nothing to report is
    // almost always a misconfigured scope, not a clean repo — say so on stderr
    // instead of printing nothing and exiting 0. This is a warning: it never
    // changes the exit code. (The agent-entrypoint check, §FS-check.3.5, runs even
    // when no source file is scanned, so a missing/stale `AGENTS.md` block still
    // reports normally and suppresses this notice.)
    if findings.scanned_files.is_empty() && report.errors.is_empty() && report.warnings.is_empty() {
        report
            .warnings
            .push(empty_scan_warning(&config, path, path_provided));
    }
    // §FS-check.4.3, after the empty-scan test above: the two cautions are
    // independent, and a repository mid-migration must not lose the scope
    // diagnostic just because it also has a config pair.
    report.warnings.extend(redundant_config_warning(&config));
    // §FS-check.1.3, also after the empty-scan test: `--full` cancels
    // `[scan] include`, and an explicit path other than the config root already
    // bypasses that key — so the flag changed nothing and the caller who typed it
    // wanted a wider search. Say so instead of accepting it silently.
    report
        .warnings
        .extend(full_scope_ignored_warning(&config, path, path_provided, full));
    // §FS-check.3.14, after the empty-scan test above: a `--full` run whose
    // *configured* scope read nothing still earns that caution — the tier says
    // where the citations are, the caution says the config has not been told.
    report.errors.extend(out_of_scope);
    sort_diagnostics(&mut report.errors);

    Ok(CheckRun {
        config,
        report,
        had_scan_errors,
    })
}

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
        // §FS-check.2.2: same empty-scan warning as the single-project path —
        // a member that scanned zero files and reported nothing is almost
        // always a misconfigured scope, not a clean repo.
        if project.findings.scanned_files.is_empty()
            && project.scan_errors.is_empty()
            && !project_has_findings
        {
            report
                .warnings
                .push(empty_scan_warning(&project.config, &project.config.root, true));
        }
    }
    // §FS-check.4.3: the root's pair plus every member's, each named at the path
    // that project's config was loaded under. The root project is skipped in the
    // loop — with `include_root = true` it is also a `projects` entry, and the
    // warning is about one directory, not one scope.
    report.warnings.extend(redundant_config_warning(&root_config));
    for project in &projects {
        if project.config.root != root_config.root {
            report.warnings.extend(redundant_config_warning(&project.config));
        }
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

/// Whether the requested scope *is* the config root — the scope `[scan] include`
/// governs, and therefore the only one `grund check --full` can widen
/// (§FS-check.1.3). It is also what decides a workspace-wide run
/// (§FS-workspace.5): both questions are "did the caller ask for the whole
/// project, however they spelled it?".
fn scope_is_config_root(config: &Config, path: &Path, path_provided: bool) -> bool {
    !path_provided
        || fs::canonicalize(path)
            .map(|path| path == config.root)
            .unwrap_or(false)
}

/// §AR-workspace.5.1, §AR-workspace.6, §AR-workspace.8: every CLI entry point
/// that walks the tree funnels through this helper so workspace handling is
/// identical across `check`, `fmt`, `refs`, `list`, `cover`, `show`, `id`, and
/// completions. The three steps are upward discovery, the member-scope
/// rewrite, and boundary-root population — the last is what stops a root-scope
/// scan from absorbing member declarations into the parent namespace.
fn resolve_workspace_config(path: &Path) -> Result<Config> {
    let mut config = load_config(path)?;
    config = config_for_member_scope(config, path)?;
    apply_workspace_boundary(&mut config)?;
    Ok(config)
}

/// §AR-workspace.6: a workspace-declared scan must never descend into member
/// roots. The boundary is the same list that `run_workspace_check`
/// computes; setting it on the Config makes the scanner skip those subtrees.
fn apply_workspace_boundary(config: &mut Config) -> Result<()> {
    if !config.workspace_declared {
        return Ok(());
    }
    config.workspace_boundary_roots = expand_workspace_members(config)?;
    Ok(())
}

/// §FS-workspace.2 / §FS-workspace.5: when the requested scope lies inside a
/// configured workspace member, rewrite the resolved config so the run is
/// rooted at the member rather than the workspace root. This applies whether
/// the member has its own `grund.toml` or not — either way a
/// member-scoped command runs as an independent project, with member defaults
/// when no member config exists.
fn config_for_member_scope(mut config: Config, path: &Path) -> Result<Config> {
    if !config.workspace_declared {
        return Ok(config);
    }
    let scope = config_scope_start(path);
    if scope == config.root {
        return Ok(config);
    }
    if let Some(member_root) = configured_member_root_for_scope(&config, &scope) {
        config = load_config_at(&member_root, &config.cli_base)?;
    }
    Ok(config)
}

fn config_scope_start(path: &Path) -> PathBuf {
    let start = if path.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };
    fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf())
}

fn configured_member_root_for_scope(config: &Config, scope: &Path) -> Option<PathBuf> {
    config
        .workspace_members
        .iter()
        .filter_map(|member| configured_member_root_candidate(config, member, scope))
        .max_by_key(|root| root.components().count())
}

fn configured_member_root_candidate(config: &Config, member: &str, scope: &Path) -> Option<PathBuf> {
    if let Some(parent) = member.strip_suffix("/*") {
        let parent = canonical_workspace_path(&config.root.join(parent));
        let relative = scope.strip_prefix(&parent).ok()?;
        let Component::Normal(child) = relative.components().next()? else {
            return None;
        };
        let root = parent.join(child);
        return Some(canonical_workspace_path(&root));
    }
    let root = canonical_workspace_path(&config.root.join(member));
    if scope == root || scope.starts_with(&root) {
        Some(root)
    } else {
        None
    }
}

fn workspace_members_error(config: &Config, message: String) -> anyhow::Error {
    config_location_error(config.workspace_members_source.as_ref(), message)
}

fn project_name_error(config: &Config, message: String) -> anyhow::Error {
    config_location_error(config.project_name_source.as_ref(), message)
}

fn config_location_error(source: Option<&ConfigLocation>, message: String) -> anyhow::Error {
    if let Some(source) = source {
        anyhow!("{}:{}: {message}", format_path(&source.path), source.line)
    } else {
        anyhow!("{message}")
    }
}

enum RootMode {
    Root,
    Member,
}

/// Phrase a workspace project's identity for a launch-time error message: the
/// root collapses to `workspace root`, members render as `workspace member
/// `<rel-path>``. §GOAL-friendliness-first.1 — duplicate-alias and
/// invalid-alias errors are CLI-level (no `path:line:`), so they have to name
/// the source project in the message itself.
fn project_label(root_config: &Config, project_root: &Path) -> String {
    if project_root == root_config.root {
        "workspace root".to_string()
    } else {
        format!(
            "workspace member `{}`",
            display_path(root_config, project_root)
        )
    }
}

/// Render the two colliding project roots for `duplicate workspace project
/// alias`. When both are members we fold the shared `workspace member` prefix
/// (`workspace members `a` and `b``); when one side is the root we keep the
/// asymmetric pairing (`workspace root and workspace member `b``).
fn duplicate_alias_sites(root_config: &Config, first: &Path, second: &Path) -> String {
    let first_is_root = first == root_config.root;
    let second_is_root = second == root_config.root;
    if !first_is_root && !second_is_root {
        return format!(
            "workspace members `{}` and `{}`",
            display_path(root_config, first),
            display_path(root_config, second)
        );
    }
    format!(
        "{} and {}",
        project_label(root_config, first),
        project_label(root_config, second)
    )
}

/// §AR-workspace.5.3: the single canonical place that derives a project's
/// workspace alias. `project_name` wins; otherwise the member directory's
/// basename, or the literal `root` for an unnamed workspace root. Whichever
/// source fires, the result is validated against the alias slug grammar so a
/// bad name fails fast at workspace expansion, not later inside a citation.
fn derive_alias(
    config: &Config,
    member_root: Option<&Path>,
    mode: RootMode,
) -> std::result::Result<String, String> {
    let alias = match (&config.project_name, &mode) {
        (Some(name), _) => name.clone(),
        (None, RootMode::Root) => "root".to_string(),
        (None, RootMode::Member) => {
            // Members always have a canonical absolute path with a final
            // component; the basename fallback is the alias source defined in
            // §AR-workspace.5.3.
            let path = member_root.expect("member alias derivation needs a member root");
            path.file_name()
                .and_then(|name| name.to_str())
                .expect("workspace member root has a final UTF-8 path component")
                .to_string()
        }
    };
    if is_valid_project_alias(&alias) {
        Ok(alias)
    } else {
        Err(format!(
            "invalid workspace project alias `{alias}` (expected [a-z][a-z0-9-]*)"
        ))
    }
}
