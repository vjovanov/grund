/// `grund show <ID>[.<section>] [--brief|--toc|--full] [--format text|md|json]`:
/// print one declaration body so an agent pulls a single fact into context
/// instead of the whole document (§FS-show). Also the default command — a bare
/// `grund <ID>` lands here (§FS-cli.1).
fn command_show(args: &[String]) -> ExitCode {
    command_show_impl(args, false)
}

fn command_show_default(args: &[String]) -> ExitCode {
    command_show_impl(args, true)
}

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
    let mut mode = ShowMode::Lead;
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
                mode = ShowMode::Brief;
            }
            "--toc" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --toc cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--toc");
                mode = ShowMode::Toc;
            }
            "--full" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --full cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--full");
                mode = ShowMode::Full;
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
    let Some(show_format) = parse_show_format(&format) else {
        eprintln!("error: unsupported show format `{format}`");
        return ExitCode::from(2);
    };
    match show_with_scope(
        &id_arg,
        ShowOpts {
            path: path.clone(),
            section: section_override,
            mode,
            format: show_format,
        },
        path_provided,
    ) {
        Ok(output) => {
            if format == "json" {
                println!("{}", output.json.unwrap_or_default());
            } else {
                print!("{}", output.body);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let message = format!("{err:#}");
            // §FS-errors.5: the ambiguous refusals carry their sites in a typed
            // carrier the query itself raised; every other query failure downcasts
            // to `None` and keeps `sites: null`.
            let sites = err
                .downcast_ref::<ShowQueryError>()
                .map(|carrier| carrier.sites.as_slice())
                .unwrap_or(&[]);
            render_show_error(&id_arg, &path, default_invocation, &format, &message, sites)
        }
    }
}

fn render_show_error(
    id_arg: &str,
    path: &Path,
    default_invocation: bool,
    format: &str,
    message: &str,
    sites: &[FindingSite],
) -> ExitCode {
    if default_invocation && message.starts_with("unknown project alias") && Path::new(id_arg).exists() {
        eprintln!("invalid ID `{id_arg}`");
        print_id_format_hint(path);
        eprintln!("hint: run `grund check {id_arg}` to validate a path");
        return ExitCode::FAILURE;
    }
    let query_error_code = show_query_error_code(message);
    if format == "json"
        && let Some(code) = query_error_code
    {
        print_bare_query_json(code, message, sites);
        return ExitCode::FAILURE;
    }
    if query_error_code.is_none() {
        eprintln!("error: {message}");
        if default_invocation {
            eprintln!("hint: run `grund check {id_arg}` to validate a path");
        }
        return ExitCode::from(2);
    }
    eprintln!("{message}");
    if message.starts_with("invalid ID") {
        print_id_format_hint(path);
        if default_invocation {
            eprintln!("hint: run `grund check {id_arg}` to validate a path");
            if looks_like_subcommand_typo(id_arg) {
                eprintln!("hint: run `grund --help` for the list of subcommands");
            }
        }
    } else if message.starts_with("ID not found:") {
        eprintln!(
            "hint: run `grund list` to see every declared ID, or `grund id <KIND> \"<title>\"` to propose a new one"
        );
    } else if message.starts_with("section not found:") {
        let base_id = effective_config(path)
            .ok()
            .and_then(|config| {
                id_arg
                    .rsplit_once(&config.section_separator)
                    .map(|(base, _)| base.to_string())
            })
            .unwrap_or_else(|| id_arg.to_string());
        eprintln!("hint: run `grund {base_id} --toc` to print the lead with the section map");
    }
    ExitCode::FAILURE
}

fn print_bare_query_json(code: &'static str, message: &str, sites: &[FindingSite]) {
    eprintln!(
        "{{\"severity\":\"error\",\"path\":null,\"line\":null,\"code\":\"{}\",\"message\":\"{}\",\"sites\":{}}}",
        code,
        json_escape(message),
        render_finding_sites_json(sites)
    );
}

fn print_id_format_hint(path: &Path) {
    if let Ok(config) = effective_config(path) {
        eprintln!(
            "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
            config.id_format
        );
    }
}

fn show_query_error_code(message: &str) -> Option<&'static str> {
    if message.starts_with("ID not found:") {
        Some("not-found")
    } else if message.starts_with("section not found:") {
        Some("missing-section")
    } else if message.starts_with("invalid ID") {
        Some("invalid-id")
    } else if message.starts_with("ambiguous ID:") {
        Some("ambiguous")
    // §FS-show.2.2.2: the section-level twin of the ambiguous-ID refusal, under
    // its own code — the two need different edits, and a JSON consumer should not
    // have to read the prose to tell them apart (§DF-duplicate-section-path.2.5).
    } else if message.starts_with("ambiguous section:") {
        Some("ambiguous-section")
    } else if message.starts_with("broken stub:") {
        Some("broken-stub")
    } else {
        None
    }
}

fn parse_show_format(format: &str) -> Option<ShowFormat> {
    match format {
        "text" => Some(ShowFormat::Text),
        "md" => Some(ShowFormat::Markdown),
        "json" => Some(ShowFormat::Json),
        _ => None,
    }
}
