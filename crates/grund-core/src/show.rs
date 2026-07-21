/// `grund show <ID>[.<section>] [--brief|--toc|--full] [--section S] [--format text|md|json]`
/// — print a slice of one declaration's body (§FS-show.1): by default the *lead*
/// (prose down to the first child heading, §FS-show.2.1); `--brief` for heading + first
/// paragraph (§FS-show.2.1.1); `--toc` for the lead plus the section map (§FS-show.2.1.2);
/// `--full` for everything (§FS-show.2.1.3); a section with `.<section>` or `--section`
/// (§FS-show.2.2). Ambiguous IDs and missing IDs/sections exit `1` with a hint
/// (§FS-show.2.2.1, §FS-show.3).
fn command_show(args: &[String]) -> ExitCode {
    command_show_impl(args, false)
}

/// Default `grund <ID>` dispatch (§FS-cli.1): identical to explicit `show`,
/// except invalid-ID diagnostics also remind users that path validation is now
/// explicit as `grund check <path>`.
fn command_show_default(args: &[String]) -> ExitCode {
    command_show_impl(args, true)
}

/// `true` when an invalid `grund <word>` looks more like a typo'd subcommand
/// than a botched ID — no `-` (the default `{kind}-{slug}` separator), no `/`
/// (workspace alias), no `.` (section). For those inputs the default-show
/// diagnostic adds a `grund --help` pointer so the typo lands somewhere
/// useful (§FS-cli.1).
fn looks_like_subcommand_typo(arg: &str) -> bool {
    !arg.is_empty() && !arg.contains('-') && !arg.contains('/') && !arg.contains('.')
}

fn command_show_impl(args: &[String], default_invocation: bool) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut mode = ShowRenderMode::Default;
    let mut mode_flag: Option<&'static str> = None;
    let mut section_override = None;
    let mut format = "text".to_string();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--brief" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --brief cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--brief");
                mode = ShowRenderMode::Brief;
            }
            "--toc" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --toc cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--toc");
                mode = ShowRenderMode::Toc;
            }
            "--full" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --full cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--full");
                mode = ShowRenderMode::Full;
            }
            other if other.starts_with("--format=") => {
                format = other.trim_start_matches("--format=").to_string();
            }
            "--section" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --section requires a value");
                    return ExitCode::from(2);
                }
                section_override = Some(args[idx].clone());
            }
            "--path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --path requires a value");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(&args[idx]);
                path_provided = true;
            }
            "--format" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --format requires a value");
                    return ExitCode::from(2);
                }
                format = args[idx].clone();
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other if id_arg.is_none() => id_arg = Some(other.to_string()),
            other => {
                if path_provided {
                    eprintln!("error: show takes an ID and at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let Some(id_arg) = id_arg else {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    };
    let context = match load_workspace_context(&path, path_provided) {
        Ok(context) => context,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let current_config = context
        .current_project()
        .map(|project| &project.config)
        .unwrap_or_else(|| context.render_config());
    let (alias, raw_id) = match split_qualified_id_arg(&id_arg) {
        Ok(parsed) => parsed,
        Err(err) => {
            let message = format!("{err:#}");
            if format == "json" {
                print_bare_query_json(current_config, show_query_error_code(&message), &message);
            } else {
                eprintln!("{message}");
                eprintln!(
                    "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                    current_config.id_format
                );
                if default_invocation {
                    eprintln!("hint: run `grund check {id_arg}` to validate a path");
                    if looks_like_subcommand_typo(&id_arg) {
                        eprintln!("hint: run `grund --help` for the list of subcommands");
                    }
                }
            }
            return ExitCode::FAILURE;
        }
    };
    // §FS-workspace.8.1: route to the qualified project's config + findings
    // when an `<alias>/<ID>` form is given; otherwise resolve against the
    // current project. An unknown alias is a CLI-shaped error (exit 2), not
    // a query failure — matches the `unknown project alias` shape `check`
    // emits at the citation site.
    let project = match alias.as_deref() {
        Some(name) => match context.project_by_alias(name) {
            Some(project) => project,
            None => {
                if default_invocation && Path::new(&id_arg).exists() {
                    eprintln!("invalid ID `{id_arg}`");
                    eprintln!(
                        "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                        current_config.id_format
                    );
                    eprintln!("hint: run `grund check {id_arg}` to validate a path");
                    if looks_like_subcommand_typo(&id_arg) {
                        eprintln!("hint: run `grund --help` for the list of subcommands");
                    }
                    return ExitCode::FAILURE;
                }
                eprintln!("error: unknown project alias `{name}`");
                if !context.workspace_loaded {
                    eprintln!(
                        "note: workspace aliases are defined in the root .agents/grund.toml under [workspace]"
                    );
                } else {
                    let known = context.aliases().join(", ");
                    eprintln!("known aliases: {known}");
                }
                if default_invocation {
                    eprintln!("hint: run `grund check {id_arg}` to validate a path");
                }
                return ExitCode::from(2);
            }
        },
        None => match context.current_project() {
            Some(project) => project,
            None => {
                eprintln!(
                    "error: unqualified ID requires a project alias when include_root = false"
                );
                let known = context.aliases().join(", ");
                if !known.is_empty() {
                    eprintln!("known aliases: {known}");
                }
                return ExitCode::from(2);
            }
        },
    };
    let config = &project.config;
    // §FS-workspace.1: split the alias first, then parse the ID tail with the
    // target project's grammar. Mixed-format workspaces rely on this; the root
    // may use `{kind}-{slug}` while `api/FS-001-session` belongs to a numbered
    // member namespace.
    let (id, inline_section) = match parse_id_arg(raw_id, &config.grammar) {
        Ok(parsed) => parsed,
        Err(err) => {
            let message = format!("{err:#}");
            if format == "json" {
                print_bare_query_json(config, show_query_error_code(&message), &message);
            } else {
                eprintln!("{message}");
                eprintln!(
                    "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                    config.id_format
                );
                if default_invocation {
                    eprintln!("hint: run `grund check {id_arg}` to validate a path");
                    if looks_like_subcommand_typo(&id_arg) {
                        eprintln!("hint: run `grund --help` for the list of subcommands");
                    }
                }
            }
            return ExitCode::FAILURE;
        }
    };
    if section_override.is_some() && inline_section.is_some() {
        eprintln!("error: --section cannot be combined with an inline section");
        return ExitCode::from(2);
    }
    let section = section_override.or(inline_section);
    if !matches!(format.as_str(), "text" | "md" | "json") {
        eprintln!("error: unsupported show format `{format}`");
        return ExitCode::from(2);
    }
    // §FS-show.3 / partial-scan semantics: any unreadable file inside the
    // selected project's scope is fatal — the lookup could miss the home.
    if let Some((file, message)) = project.scan_errors.first() {
        eprintln!(
            "error: {}: {}",
            display_path(&project.config, file),
            message
        );
        return ExitCode::from(2);
    }
    let findings = &project.findings;
    match show_declaration(
        config,
        findings,
        &id,
        section.as_deref(),
        mode,
        format == "md",
    ) {
        Ok(mut output) => {
            // §FS-show.3.2: `text` and `json` flatten `--cross-refs` link wrappers
            // back to bare `§…` citations; `md` keeps the renderable form verbatim.
            if format != "md" {
                output.body = flatten_cross_ref_links(&output.body, config);
            }
            if format == "json" {
                println!(
                    "{}",
                    render_show_output_json(
                        config,
                        context.render_config(),
                        &id,
                        section.as_deref(),
                        mode,
                        &output
                    )
                );
            } else {
                print!("{}", output.body);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let message = format!("{err:#}");
            if format == "json" {
                print_bare_query_json(config, show_query_error_code(&message), &message);
            } else {
                eprintln!("{message}");
                if message.starts_with("ID not found:") {
                    eprintln!(
                        "hint: run `grund list` to see every declared ID, or `grund id <KIND> \"<title>\"` to propose a new one"
                    );
                } else if message.starts_with("section not found:") {
                    eprintln!(
                        "hint: run `grund {} --toc` to print the lead with the section map",
                        render_id(config, &id)
                    );
                }
            }
            ExitCode::FAILURE
        }
    }
}

/// `config` is the *target project's* config — it owns the ID grammar and marker
/// that `render_id` needs. `path_config` is the workspace render config, i.e. the
/// same base `grund list` renders against, so an `<alias>/<ID>` resolved from a
/// workspace root reports a path relative to that root rather than to the member
/// (§FS-config.3.6: paths are relative to *the config root*, and §FS-integrations.3.1
/// joins this path against the root `grund-open` discovered).
fn render_show_output_json(
    config: &Config,
    path_config: &Config,
    id: &Id,
    section: Option<&str>,
    mode: ShowRenderMode,
    output: &ShowOutput,
) -> String {
    if let Some(json) = &output.json {
        return json.clone();
    }
    let mut extra = String::new();
    if matches!(mode, ShowRenderMode::Toc) {
        extra.push_str(",\"sections\":[");
        extra.push_str(
            &output
                .sections
                .iter()
                .map(|section| {
                    format!(
                        "{{\"path\":\"{}\",\"title\":\"{}\",\"depth\":{}}}",
                        json_escape(&section.path),
                        json_escape(&section.title),
                        section.depth
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        extra.push(']');
    }
    format!(
        "{{\"id\":\"{}\",\"section\":{},\"body\":\"{}\",\"path\":\"{}\",\"line\":{}{}}}",
        json_escape(&render_id(config, id)),
        match section {
            Some(section) => format!("\"{}\"", json_escape(section)),
            None => "null".to_string(),
        },
        json_escape(&output.body),
        json_escape(&display_path(path_config, &output.path)),
        output.line,
        extra
    )
}
