/// `grund config validate` / `grund config show`: the deprecated compatibility
/// command adapter for §FS-config.4, reached only through
/// `grund_core::main_entry()` (§AR-bindings.2). The published `grund` CLI owns
/// its own copy of this rendering in `crates/grund-cli`; the duplication is the
/// deprecation boundary, not an oversight — `grund-cli` imports no
/// `grund_core::command_*` symbol.
fn render_citation_disjunction(disjunction: &CitationDisjunction) -> String {
    disjunction
        .targets
        .iter()
        .map(render_citation_target)
        .collect::<Vec<_>>()
        .join("|")
}

fn citation_level_str(level: CitationLevel) -> &'static str {
    match level {
        CitationLevel::Must => "must",
        CitationLevel::Should => "should",
        CitationLevel::May => "may",
        CitationLevel::ShouldNot => "should-not",
        CitationLevel::MustNot => "must-not",
    }
}

fn command_config(args: &[String]) -> ExitCode {
    let Some(action) = args.first().map(|arg| arg.as_str()) else {
        eprintln!("error: expected `config validate` or `config show`");
        return ExitCode::from(2);
    };
    if !matches!(action, "validate" | "show") {
        if action.starts_with('-') {
            eprintln!("error: unknown flag `{action}`");
        } else {
            eprintln!("error: unknown config command `{action}`");
            eprintln!("expected: config validate, config show");
        }
        return ExitCode::from(2);
    }

    let mut path: Option<PathBuf> = None;
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("error: unknown flag `{arg}`");
            return ExitCode::from(2);
        }
        if path.is_some() {
            eprintln!("error: config {action} takes at most one path argument");
            return ExitCode::from(2);
        }
        path = Some(PathBuf::from(arg));
    }
    let path = path.unwrap_or_else(|| ".".into());

    match action {
        "validate" => match validate_config(&path) {
            Ok(config) => {
                print_config_warnings(&config);
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::FAILURE
            }
        },
        "show" => match load_config(&path) {
            Ok(config) => {
                print_config_warnings(&config);
                println!("grund_config_version = 1");
                if let Some(name) = &config.project_name {
                    println!("project_name = \"{}\"", escape_toml_basic(name));
                }
                if let Some(description) = &config.project_description {
                    println!("project_description = \"{}\"", escape_toml_basic(description));
                }
                println!();
                println!("[reference]");
                println!("marker = \"{}\"", config.marker);
                println!("trigger = \"{}\"", config.trigger);
                println!("strict = {}", config.strict);
                println!("require_grounding = {}", config.require_grounding);
                // Optional opinion (§FS-config.3.1): absent means none, so only a
                // set value round-trips — there is no "none" spelling to print.
                if let Some(conversation) = &config.conversation {
                    println!("conversation = \"{conversation}\"");
                }
                println!("inline_style = \"{}\"", config.inline_style);
                println!(
                    "inline_note_suggested_lines = {}",
                    config.inline_note_suggested_lines
                );
                println!(
                    "inline_note_max_lines = {}",
                    config.inline_note_max_lines
                );
                println!(
                    "inline_note_max_columns = {}",
                    config.inline_note_max_columns
                );
                println!("inline_note_layout = \"{}\"", config.inline_note_layout);
                println!(
                    "inline_note_layout_check = \"{}\"",
                    config.inline_note_layout_check
                );
                println!("warn_on_suggested = {}", config.warn_on_suggested);
                println!();
                println!("[id]");
                println!("format = \"{}\"", config.id_format);
                println!("section_separator = \"{}\"", config.section_separator);
                println!(
                    "section_heading_levels = \"{}\"",
                    config.section_heading_levels
                );
                // `number_pattern` / `slug_pattern` each govern one `[id] format`
                // placeholder — under a format that omits the placeholder the pattern
                // is dead config, so don't print it.
                if config.id_format.contains("{number}") {
                    println!(
                        "number_pattern = \"{}\"",
                        escape_toml_basic(&config.number_pattern)
                    );
                }
                if config.id_format.contains("{slug}") {
                    println!(
                        "slug_pattern = \"{}\"",
                        escape_toml_basic(&config.slug_pattern)
                    );
                }
                println!();
                for kind in &config.kinds {
                    println!("[[kinds]]");
                    println!("kind = \"{}\"", escape_toml_basic(&kind.kind));
                    if let Some(folder) = &kind.folder {
                        println!("folder = \"{}\"", escape_toml_basic(folder));
                    }
                    if let Some(file) = &kind.file {
                        println!("file = \"{}\"", escape_toml_basic(file));
                    }
                    // §FS-config.3.4: the effective index, spelled out — a
                    // folder kind either has one or has opted out, and which it
                    // is decides a verdict (§FS-check.4.6).
                    if let Some(index) = kind.index_toml_value() {
                        println!("index = {index}");
                    }
                    // §FS-config.3.4: printed only where it is set, because absence
                    // *is* `citable = true` — the shown config has to load back as
                    // itself, and a `citable` line on every kind would be noise.
                    if !kind.citable {
                        println!("citable = false");
                    }
                    // §FS-config.3.4.7: the same rule — absence is `scan = true`.
                    if !kind.scan {
                        println!("scan = false");
                    }
                    if let Some(title) = &kind.title {
                        println!("title = \"{}\"", escape_toml_basic(title));
                    }
                    println!();
                }
                println!("[scan]");
                println!(
                    "include = {}",
                    format_toml_string_list(config.include.as_deref().unwrap_or(&[]))
                );
                println!("exclude = {}", format_toml_string_list(&config.exclude));
                println!(
                    "extensions = {}",
                    format_toml_string_list(&config.extensions)
                );
                println!(
                    "comment_prefixes = {}",
                    format_toml_string_list(&config.comment_prefixes)
                );
                println!("docstring_python = {}", config.docstring_python);
                println!("respect_gitignore = {}", config.respect_gitignore);
                println!();
                println!("[output]");
                println!("format = \"{}\"", config.output_format);
                println!("color = \"auto\"");
                println!("relative_paths = {}", config.relative_paths);
                println!();
                println!("[fmt.cross_refs]");
                println!("enabled = {}", config.fmt_cross_refs_enabled);
                println!("anchor_format = \"{}\"", config.cross_ref_anchor_format);
                if config.workspace_declared {
                    println!();
                    println!("[workspace]");
                    println!(
                        "members = {}",
                        format_toml_string_list(&config.workspace_members)
                    );
                    println!("include_root = {}", config.workspace_include_root);
                }
                if config.citations.declared {
                    print_citation_rules(&config.citations);
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::from(2)
            }
        },
        _ => unreachable!(),
    }
}

/// Print the effective `[citations]` section for `grund config show`
/// (§FS-config.4.2). Per-kind tables print in sorted order — deterministic, as
/// `config show` requires (§FS-errors.4).
fn print_citation_rules(citations: &CitationRules) {
    println!();
    println!("[citations]");
    if let Some(default) = citations.global_default {
        println!("default = \"{}\"", citation_level_str(default));
    }
    for (kind, rules) in &citations.per_kind {
        println!();
        println!("[citations.{kind}]");
        if let Some(default) = rules.default {
            println!("default = \"{}\"", citation_level_str(default));
        }
        let lists: [(&str, &[CitationDisjunction]); 5] = [
            ("must", &rules.must),
            ("should", &rules.should),
            ("may", &rules.may),
            ("should-not", &rules.should_not),
            ("must-not", &rules.must_not),
        ];
        for (key, disjunctions) in lists {
            if disjunctions.is_empty() {
                continue;
            }
            let entries: Vec<String> = disjunctions.iter().map(render_citation_disjunction).collect();
            println!("{key} = {}", format_toml_string_list(&entries));
        }
    }
}

fn format_toml_string_list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", escape_toml_basic(value)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
