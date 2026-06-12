/// Discover and load the effective config: walk upward from `start` for the
/// nearest `.agents/grund.toml` (§FS-config.1), parse it over the defaults
/// (§FS-config.2), or fall back to the pure defaults if none is found
/// (§GOAL-zero-config).
fn load_config(start: &Path) -> Result<Config> {
    let start_dir = if start.is_file() {
        start.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        start.to_path_buf()
    };
    // Resolve to an absolute path before walking up, mirroring how `cargo` finds
    // `Cargo.toml` (§FS-config.1): a relative `.` or `subdir/` must still discover
    // a `.agents/grund.toml` in an ancestor directory.
    let walk_start = fs::canonicalize(&start_dir).unwrap_or(start_dir);
    let mut cursor = Some(walk_start.as_path());
    while let Some(dir) = cursor {
        let candidate = dir.join(".agents").join("grund.toml");
        if candidate.exists() {
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
    let mut config = Config::default_for(root.clone());
    config.cli_base = cli_base.to_path_buf();
    let candidate = root.join(".agents").join("grund.toml");
    if candidate.exists() {
        // Report config errors against a stable relative path, never the
        // absolute discovered path (§FS-errors.4: deterministic, no absolute
        // paths outside the configured root).
        let base = report_base.unwrap_or(&root);
        let report_path = candidate
            .strip_prefix(base)
            .map(Path::to_path_buf)
            .or_else(|_| candidate.strip_prefix(&root).map(Path::to_path_buf))
            .unwrap_or_else(|_| candidate.clone());
        parse_config_file(&candidate, &report_path, &mut config)?;
    }
    Ok(config)
}

/// Parse one `.agents/grund.toml` over `config` — the schema of §FS-config.3 and its
/// subsections (`[reference]` 3.1, `[id]` 3.2/3.3, `[[kinds]]` 3.4, `[scan]` 3.5,
/// `[output]` 3.6, `[fmt.cross_refs]` 3.7). Any unknown section/key or malformed
/// value is a hard error reported as `path:line:` (§FS-config.4.3, §FS-errors.2.1).
fn parse_config_file(read_path: &Path, report_path: &Path, config: &mut Config) -> Result<()> {
    let text = fs::read_to_string(read_path)
        .with_context(|| format!("read {}", format_path(report_path)))?;
    // Everything below reports problems against the stable relative path.
    let path = report_path;
    let mut section = String::new();
    let mut grammar_dirty = false;
    let mut parsed_kinds: Vec<KindConfig> = Vec::new();
    let mut current_kind: Option<KindConfig> = None;
    let mut kinds_block_seen = false;
    let mut inline_note_suggested_lines_source = None;
    let mut inline_note_max_lines_source = None;
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let is_array_table = line.starts_with("[[") && line.ends_with("]]");
            section = line.trim_matches(['[', ']']).to_string();
            match section.as_str() {
                "reference" | "scan" | "output" | "id" | "fmt.cross_refs" | "workspace" => {
                    if section == "workspace" && is_array_table {
                        bail_config(
                            path,
                            line_no,
                            "expected `[workspace]` (table)".to_string(),
                        )?;
                    }
                    if section == "workspace" {
                        config.workspace_declared = true;
                    }
                }
                "kinds" => {
                    if !is_array_table {
                        bail_config(
                            path,
                            line_no,
                            "expected `[[kinds]]` (array of tables)".to_string(),
                        )?;
                    }
                    // Flush any open kind entry, then start a new one.
                    if let Some(prefix) = current_kind.take() {
                        parsed_kinds.push(prefix);
                    }
                    current_kind = Some(KindConfig {
                        prefix: String::new(),
                        folder: None,
                        file: None,
                        title: None,
                    });
                    kinds_block_seen = true;
                }
                other => bail_config(path, line_no, format!("unknown config section `{other}`"))?,
            }
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            bail_config(path, line_no, "expected `key = value`".to_string())?;
            unreachable!();
        };
        let key = key.trim();
        let value = value.trim();
        match (section.as_str(), key) {
            ("", "grund_config_version") => {
                if value != "1" {
                    bail_config(
                        path,
                        line_no,
                        format!(
                            "unsupported config version `{value}` \
                             (this grund understands grund_config_version = 1; \
                             upgrade grund if the config is newer)"
                        ),
                    )?;
                }
            }
            ("", "project_name") => {
                config.project_name = Some(parse_string(path, line_no, value)?);
                config.project_name_source = Some(ConfigLocation {
                    path: path.to_path_buf(),
                    line: line_no,
                });
            }
            ("", "project_description") => {
                let description = parse_string(path, line_no, value)?;
                // §FS-config.3: the key feeds single-line workspace member
                // bullets, so an embedded line break is a config error.
                if description.contains('\n') || description.contains('\r') {
                    bail_config(
                        path,
                        line_no,
                        "project_description must be a single line".to_string(),
                    )?;
                }
                config.project_description = Some(description);
            }
            ("reference", "marker") => config.marker = parse_string(path, line_no, value)?,
            ("reference", "trigger") => config.trigger = parse_string(path, line_no, value)?,
            ("reference", "strict") => config.strict = parse_bool(path, line_no, value)?,
            ("reference", "require_grounding") => {
                config.require_grounding = parse_bool(path, line_no, value)?
            }
            ("reference", "inline_style") => {
                let style = parse_string(path, line_no, value)?;
                if !matches!(style.as_str(), "citation-with-note" | "citation-only") {
                    bail_config(
                        path,
                        line_no,
                        "unknown [reference] inline_style".to_string(),
                    )?;
                }
                config.inline_style = style;
            }
            ("reference", "inline_note_suggested_lines") => {
                config.inline_note_suggested_lines = parse_usize(path, line_no, value)?;
                inline_note_suggested_lines_source = Some(line_no);
            }
            ("reference", "inline_note_max_lines") => {
                config.inline_note_max_lines = parse_usize(path, line_no, value)?;
                inline_note_max_lines_source = Some(line_no);
            }
            ("reference", "inline_note_max_columns") => {
                config.inline_note_max_columns = parse_usize(path, line_no, value)?
            }
            ("reference", "warn_on_suggested") => {
                config.warn_on_suggested = parse_bool(path, line_no, value)?
            }
            ("id", "format") => {
                config.id_format = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("id", "section_separator") => {
                config.section_separator = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("id", "number_pattern") => {
                config.number_pattern = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("id", "slug_pattern") => {
                config.slug_pattern = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("id", "section_heading_levels") => {
                let mode = parse_string(path, line_no, value)?;
                if !matches!(mode.as_str(), "strict" | "warn" | "loose") {
                    bail_config(
                        path,
                        line_no,
                        format!(
                            "unknown [id] section_heading_levels `{mode}` (expected strict, warn, or loose)"
                        ),
                    )?;
                }
                config.section_heading_levels = mode;
            }
            ("kinds", "prefix") => {
                let prefix = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    slot.prefix = prefix;
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`prefix` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("kinds", "folder") => {
                let folder = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    slot.folder = Some(folder);
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`folder` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("kinds", "file") => {
                let file = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    slot.file = Some(file);
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`file` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("kinds", "title") => {
                let title = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    slot.title = Some(title);
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`title` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("scan", "include") => config.include = Some(parse_string_list(path, line_no, value)?),
            ("scan", "exclude") => config.exclude = parse_string_list(path, line_no, value)?,
            ("scan", "extensions") => config.extensions = parse_string_list(path, line_no, value)?,
            ("scan", "comment_prefixes") => {
                config.comment_prefixes = parse_string_list(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("scan", "docstring_python") => {
                config.docstring_python = parse_bool(path, line_no, value)?;
            }
            ("scan", "respect_gitignore") => {
                config.respect_gitignore = parse_bool(path, line_no, value)?;
            }
            ("output", "format") => {
                let format = parse_string(path, line_no, value)?;
                if !matches!(format.as_str(), "text" | "json") {
                    bail_config(path, line_no, "unsupported output format".to_string())?;
                }
                config.output_format = format;
            }
            ("output", "color") => {
                // Reserved — colored output is not yet implemented (§FS-config.6,
                // §FS-errors.3): the value is inert today, but it is still validated
                // against the documented `auto | always | never` set so a typo here
                // fails on load like any other enum knob, rather than being silently
                // accepted and then ignored when the feature lands.
                let color = parse_string(path, line_no, value)?;
                if !matches!(color.as_str(), "auto" | "always" | "never") {
                    bail_config(
                        path,
                        line_no,
                        format!(
                            "unknown [output] color `{color}` (expected auto, always, or never)"
                        ),
                    )?;
                }
            }
            ("output", "relative_paths") => {
                config.relative_paths = parse_bool(path, line_no, value)?;
            }
            ("fmt.cross_refs", "enabled") => {
                config.fmt_cross_refs_enabled = parse_bool(path, line_no, value)?;
            }
            ("fmt.cross_refs", "anchor_format") => {
                let format = parse_string(path, line_no, value)?;
                if !matches!(
                    format.as_str(),
                    "github" | "gitlab" | "mkdocs" | "pandoc" | "none"
                ) {
                    bail_config(path, line_no, "unknown md link anchor format".to_string())?;
                }
                config.cross_ref_anchor_format = format;
            }
            ("workspace", "members") => {
                config.workspace_members = parse_string_list(path, line_no, value)?;
                config.workspace_members_source = Some(ConfigLocation {
                    path: path.to_path_buf(),
                    line: line_no,
                });
            }
            ("workspace", "include_root") => {
                config.workspace_include_root = parse_bool(path, line_no, value)?;
            }
            _ => bail_config(path, line_no, format!("unknown config key `{key}`"))?,
        }
    }
    if let Some(prefix) = current_kind.take() {
        parsed_kinds.push(prefix);
    }
    if config.strict && config.marker.is_empty() {
        return Err(anyhow!(
            "{}: reference.strict requires a non-empty marker",
            format_path(path)
        ));
    }
    if config.inline_note_suggested_lines > config.inline_note_max_lines {
        let line = inline_note_suggested_lines_source
            .or(inline_note_max_lines_source)
            .unwrap_or(1);
        bail_config(
            path,
            line,
            "reference.inline_note_suggested_lines must be <= inline_note_max_lines".to_string(),
        )?;
    }
    if kinds_block_seen {
        // [[kinds]] replaces defaults entirely, per §FS-config.3.4.
        if parsed_kinds.iter().any(|p| p.prefix.is_empty()) {
            return Err(anyhow!(
                "{}: every [[kinds]] entry must declare a `prefix`",
                format_path(path)
            ));
        }
        if parsed_kinds.is_empty() {
            return Err(anyhow!(
                "{}: at least one [[kinds]] entry must declare a `prefix`",
                format_path(path)
            ));
        }
        // Reject kinds that set both `folder` and `file` — they're mutually
        // exclusive (§FS-config.3.4). A kind is either multi-file (folder) or
        // single-file (file); the schema models the "can always be broken up"
        // transition as swapping one key for the other, not setting both.
        for k in &parsed_kinds {
            if k.folder.is_some() && k.file.is_some() {
                return Err(anyhow!(
                    "{}: kind `{}` sets both `folder` and `file` (use one)",
                    format_path(path),
                    k.prefix
                ));
            }
        }
        // Reject kinds whose prefix is itself a prefix of another kind's prefix
        // (§FS-config.3.4 — would make tokenization ambiguous).
        for (i, a) in parsed_kinds.iter().enumerate() {
            for (j, b) in parsed_kinds.iter().enumerate() {
                if i != j
                    && a.prefix.len() <= b.prefix.len()
                    && b.prefix.starts_with(a.prefix.as_str())
                {
                    return Err(anyhow!(
                        "{}: kinds `{}` and `{}` collide (one is a prefix of the other)",
                        format_path(path),
                        a.prefix,
                        b.prefix
                    ));
                }
            }
        }
        config.kinds = parsed_kinds;
    }
    if grammar_dirty || kinds_block_seen {
        config
            .rebuild_grammar()
            .with_context(|| format!("{}: invalid [id] grammar", format_path(path)))?;
    }
    // §AR-workspace.5.2: every post-parse invariant runs on every config
    // load, not gated on which section happened to appear. `project_name` is
    // free-form metadata (§FS-config.3); the slug check against the alias
    // grammar happens once, where it matters, at workspace-project loading
    // (§AR-workspace.5.3). The workspace member
    // list, by contrast, is shape-checked here — an entry like
    // `members = ["/abs/path"]` is wrong before we even look at it.
    if let Some(source) = &config.workspace_members_source {
        for member in &config.workspace_members {
            validate_workspace_member(&source.path, source.line, member)?;
        }
    }
    Ok(())
}

fn validate_workspace_member(path: &Path, line: usize, member: &str) -> Result<()> {
    let member_path = Path::new(member);
    if member.is_empty()
        || member_path.is_absolute()
        || member_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
                    | std::path::Component::CurDir
                    | std::path::Component::ParentDir
            )
        })
        || member.contains('\\')
        || member.split('/').enumerate().any(|(index, part)| {
            part.is_empty()
                || part == "."
                || part == ".."
                || (index == 0 && looks_like_windows_drive_prefix(part))
        })
        || member.matches('*').count() > 1
        || (member.contains('*') && !member.ends_with("/*"))
    {
        return Err(anyhow!(
            "{}:{line}: invalid [workspace] member `{member}` (expected relative path or trailing /* glob)",
            format_path(path),
        ));
    }
    Ok(())
}

fn looks_like_windows_drive_prefix(part: &str) -> bool {
    let bytes = part.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_valid_project_alias(alias: &str) -> bool {
    let mut chars = alias.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_lowercase())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

/// Drop a trailing `#`-comment from a `.agents/grund.toml` line (§FS-config.3).
fn strip_comment(line: &str) -> &str {
    // A `#` inside a quoted string is not a comment marker. Walk the line and stop at the
    // first unquoted `#`; otherwise return the line unchanged.
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' if !is_escaped(bytes, i) => in_string = !in_string,
            b'#' if !in_string => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

fn is_escaped(bytes: &[u8], pos: usize) -> bool {
    let mut count = 0;
    let mut j = pos;
    while j > 0 && bytes[j - 1] == b'\\' {
        count += 1;
        j -= 1;
    }
    count % 2 == 1
}

/// Fail config parsing with a `path:line: message` error — the located-finding
/// shape applied to a malformed `.agents/grund.toml` (§FS-config.4.3, §FS-errors.2.1).
fn bail_config<T>(path: &Path, line: usize, message: String) -> Result<T> {
    Err(anyhow!("{}:{}: {}", format_path(path), line, message))
}

fn parse_string(path: &Path, line: usize, value: &str) -> Result<String> {
    if !(value.starts_with('"') && value.ends_with('"') && value.len() >= 2) {
        return bail_config(path, line, "expected string".to_string());
    }
    let inner = &value[1..value.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => {
                return bail_config(
                    path,
                    line,
                    format!("invalid escape sequence `\\{other}` in string"),
                );
            }
            None => {
                return bail_config(path, line, "trailing backslash in string".to_string());
            }
        }
    }
    Ok(out)
}

fn parse_bool(path: &Path, line: usize, value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => bail_config(path, line, "expected boolean".to_string()),
    }
}

fn parse_usize(path: &Path, line: usize, value: &str) -> Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| anyhow!("{}:{}: expected non-negative integer", format_path(path), line))
}

fn parse_string_list(path: &Path, line: usize, value: &str) -> Result<Vec<String>> {
    if !value.starts_with('[') || !value.ends_with(']') {
        return bail_config(path, line, "expected string list".to_string());
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|part| parse_string(path, line, part.trim()))
        .collect()
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
        "validate" => match load_config(&path) {
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::FAILURE
            }
        },
        "show" => match load_config(&path) {
            Ok(config) => {
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
                    println!("prefix = \"{}\"", escape_toml_basic(&kind.prefix));
                    if let Some(folder) = &kind.folder {
                        println!("folder = \"{}\"", escape_toml_basic(folder));
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
