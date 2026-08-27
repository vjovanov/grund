/// Parse one `grund.toml` over `config` — the schema of §FS-config.3 and its
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
    let mut parsed_kinds: Vec<ParsedKind> = Vec::new();
    let mut current_kind: Option<ParsedKind> = None;
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
                        // §FS-workspace.6.1: the block's own anchor, for the
                        // errors that are about the block and not about a key.
                        config.workspace_section_source = Some(ConfigLocation {
                            path: path.to_path_buf(),
                            line: line_no,
                        });
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
                    if let Some(kind) = current_kind.take() {
                        parsed_kinds.push(kind);
                    }
                    current_kind = Some(ParsedKind::new(line_no));
                    kinds_block_seen = true;
                }
                // §FS-config.3.9: `[citations]` and per-kind `[citations.<KIND>]`
                // tables. Both are plain tables, never arrays of tables.
                other if other == "citations" || other.starts_with("citations.") => {
                    if is_array_table {
                        bail_config(
                            path,
                            line_no,
                            "expected `[citations]` / `[citations.<KIND>]` (table)".to_string(),
                        )?;
                    }
                    config.citations.declared = true;
                    if let Some(kind) = other.strip_prefix("citations.") {
                        config
                            .citations
                            .per_kind
                            .entry(kind.to_string())
                            .or_default();
                    }
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
            ("reference", "conversation") => {
                // §FS-config.3.1, §DF-repo-conversation-opinion.2.2: closed enum with the
                // single member "link" — `plain` encodes machine state and stays user-scoped.
                let opinion = parse_string(path, line_no, value)?;
                if opinion != "link" {
                    bail_config(
                        path,
                        line_no,
                        format!("unknown [reference] conversation `{opinion}` (expected link)"),
                    )?;
                }
                config.conversation = Some(opinion);
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
            // §FS-inline-citation-style.2.2: two closed enums, rejected on load like
            // `inline_style` above — a typo must not read as "no house style".
            ("reference", "inline_note_layout") => {
                let layout = parse_string(path, line_no, value)?;
                if !matches!(layout.as_str(), "any" | "citation-first-colon") {
                    bail_config(
                        path,
                        line_no,
                        format!(
                            "unknown [reference] inline_note_layout `{layout}` (expected any or citation-first-colon)"
                        ),
                    )?;
                }
                config.inline_note_layout = layout;
            }
            ("reference", "inline_note_layout_check") => {
                let level = parse_string(path, line_no, value)?;
                if !matches!(level.as_str(), "off" | "warn" | "error") {
                    bail_config(
                        path,
                        line_no,
                        format!(
                            "unknown [reference] inline_note_layout_check `{level}` (expected off, warn, or error)"
                        ),
                    )?;
                }
                config.inline_note_layout_check = level;
            }
            ("reference", "warn_on_suggested") => {
                config.warn_on_suggested = parse_bool(path, line_no, value)?
            }
            // §FS-config.3.2: the keys an ID is built from share one rule — no `/` —
            // so they share one arm and `id_grammar_rules.rs` answers per key; checked
            // here in the config-error style (§FS-errors.2.1), `Grammar::build` backstops.
            ("id", key @ ("format" | "section_separator" | "number_pattern" | "slug_pattern")) => {
                let parsed = parse_string(path, line_no, value)?;
                if let Some(message) = id_grammar_key_slash_error(key, &parsed) {
                    bail_config(path, line_no, message)?;
                }
                match key {
                    "format" => config.id_format = parsed,
                    "section_separator" => config.section_separator = parsed,
                    "number_pattern" => config.number_pattern = parsed,
                    _ => config.slug_pattern = parsed,
                }
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
            // §FS-config.3.4: the `[[kinds]]` keys, in `config_kinds.rs`
            // (§AR-core-module-layout.1).
            ("kinds", key) => {
                if !parse_kinds_key(path, line_no, key, value, &mut current_kind)? {
                    bail_config(path, line_no, format!("unknown config key `{key}`"))?;
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
                // §FS-errors.3): the value is inert today but still validated against
                // the documented set, so a typo fails on load instead of being ignored.
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
            // §FS-config.3.9: `[citations]` `default`, and the level keys of each
            // `[citations.<KIND>]` table.
            (s, k) if s == "citations" || s.starts_with("citations.") => {
                parse_citation_entry(path, line_no, s, k, value, &mut config.citations)?;
            }
            _ => bail_config(path, line_no, format!("unknown config key `{key}`"))?,
        }
    }
    if let Some(kind) = current_kind.take() {
        parsed_kinds.push(kind);
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
        apply_parsed_kinds(path, parsed_kinds, config)?;
    }
    if grammar_dirty || kinds_block_seen {
        config
            .rebuild_grammar()
            .with_context(|| format!("{}: invalid [id] grammar", format_path(path)))?;
    }
    // §AR-workspace.5.2: post-parse invariants run on every load, not gated on which
    // section appeared. Free-form `project_name` (§FS-config.3) is slug-checked against
    // the alias grammar later (§AR-workspace.5.3); members are shape-checked here.
    if let Some(source) = &config.workspace_members_source {
        for member in &config.workspace_members {
            validate_workspace_member(&source.path, source.line, member)?;
        }
    }
    // §FS-config.3.9.5: validate `[citations]` after the kind set is final.
    if config.citations.declared {
        validate_citation_rules(path, config)?;
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

/// Drop a trailing `#`-comment from a `grund.toml` line (§FS-config.3).
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
/// shape applied to a malformed `grund.toml` (§FS-config.4.3, §FS-errors.2.1).
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

/// Parse one `[citations]` / `[citations.<KIND>]` key (§FS-config.3.9). The
/// top-level table takes only `default`; a per-kind table takes `default` plus
/// the five level lists.
fn parse_citation_entry(
    path: &Path,
    line_no: usize,
    section: &str,
    key: &str,
    value: &str,
    citations: &mut CitationRules,
) -> Result<()> {
    if section == "citations" {
        return match key {
            "default" => {
                citations.global_default = Some(parse_citation_level(path, line_no, value)?);
                Ok(())
            }
            other => bail_config(
                path,
                line_no,
                format!(
                    "unknown key `{other}` in [citations] (expected `default`, or a [citations.<KIND>] table)"
                ),
            ),
        };
    }
    let kind = section
        .strip_prefix("citations.")
        .expect("caller guarantees a citations. section");
    let rules = citations.per_kind.entry(kind.to_string()).or_default();
    match key {
        "default" => rules.default = Some(parse_citation_level(path, line_no, value)?),
        "must" => rules.must = parse_citation_disjunctions(path, line_no, value)?,
        "should" => rules.should = parse_citation_disjunctions(path, line_no, value)?,
        "may" => rules.may = parse_citation_disjunctions(path, line_no, value)?,
        "should-not" => rules.should_not = parse_citation_disjunctions(path, line_no, value)?,
        "must-not" => rules.must_not = parse_citation_disjunctions(path, line_no, value)?,
        other => bail_config(
            path,
            line_no,
            format!(
                "unknown key `{other}` in [citations.{kind}] (expected must, should, may, should-not, must-not, or default)"
            ),
        )?,
    }
    Ok(())
}

fn parse_citation_level(path: &Path, line_no: usize, value: &str) -> Result<CitationLevel> {
    let level = parse_string(path, line_no, value)?;
    match level.as_str() {
        "must" => Ok(CitationLevel::Must),
        "should" => Ok(CitationLevel::Should),
        "may" => Ok(CitationLevel::May),
        "should-not" => Ok(CitationLevel::ShouldNot),
        "must-not" => Ok(CitationLevel::MustNot),
        other => bail_config(
            path,
            line_no,
            format!(
                "unknown citation level `{other}` (expected must, should, may, should-not, or must-not)"
            ),
        ),
    }
}

fn parse_citation_disjunctions(
    path: &Path,
    line_no: usize,
    value: &str,
) -> Result<Vec<CitationDisjunction>> {
    parse_string_list(path, line_no, value)?
        .iter()
        .map(|entry| parse_citation_disjunction(path, line_no, entry))
        .collect()
}

fn parse_citation_disjunction(
    path: &Path,
    line_no: usize,
    entry: &str,
) -> Result<CitationDisjunction> {
    let mut targets = Vec::new();
    for token in entry.split('|') {
        let token = token.trim();
        if token.is_empty() {
            bail_config(path, line_no, "empty citation target".to_string())?;
        }
        targets.push(parse_citation_target(path, line_no, token)?);
    }
    Ok(CitationDisjunction { targets })
}

fn parse_citation_target(path: &Path, line_no: usize, token: &str) -> Result<CitationTarget> {
    // §FS-config.3.9: the kind is the last segment, so a nested member is pinned
    // by its whole alias path (`group/api/AR`) exactly as it is cited
    // (§FS-workspace.6.1).
    let (namespace, kind) = match token.rsplit_once('/') {
        Some((qualifier, kind)) => {
            let namespace = if qualifier == "*" {
                NamespaceMatch::Any
            } else {
                // §FS-config.3.9.3: a qualifier is the alias path a citation writes, so
                // a bad one names the failing *segment*, not the whole path (mostly right
                // in a nested tree), like the CLI (§FS-workspace.8, §GOAL-friendliness-first).
                if let Some(message) = invalid_alias_path_message(qualifier) {
                    bail_config(
                        path,
                        line_no,
                        format!("citation target `{token}`: {message} — `*` matches any project"),
                    )?;
                }
                NamespaceMatch::Alias(qualifier.to_string())
            };
            (namespace, kind)
        }
        None => (NamespaceMatch::Local, token),
    };
    if kind.is_empty() {
        bail_config(
            path,
            line_no,
            format!("citation target `{token}` names no kind"),
        )?;
    }
    Ok(CitationTarget {
        namespace,
        kind: kind.to_string(),
    })
}

/// Validate the parsed `[citations]` rules against the finalized kind set
/// (§FS-config.3.9.5): every citing kind is a configured kind or `code`, every
/// target names a *citable* configured kind, and no two targets of the same
/// cited kind whose namespace matchers overlap sit at different levels.
fn validate_citation_rules(path: &Path, config: &Config) -> Result<()> {
    // The citing side is any name in the table plus `code` — a non-citable kind
    // cites like any other place (§FS-config.3.9). The cited side is narrower:
    // only a citable kind has IDs to be the target of a citation.
    let citing_known: BTreeSet<&str> = citing_kind_names(&config.kinds).into_iter().collect();
    let known: BTreeSet<&str> = config
        .kinds
        .iter()
        .filter(|k| k.citable)
        .map(|k| k.kind.as_str())
        .collect();
    for (citing, rules) in &config.citations.per_kind {
        if !citing_known.contains(citing.as_str()) {
            return Err(anyhow!(
                "{}: [citations.{citing}] names an unknown kind `{citing}`",
                format_path(path)
            ));
        }
        // §FS-config.3.4.7: a rule on a kind whose home is not walked could
        // never fire — the vacuous pass §DF-non-citable-kinds.2.5 refused, one
        // level up — so the config is refused where it makes the promise.
        if config.kinds.iter().any(|k| k.kind == *citing && !k.scan) {
            return Err(anyhow!(
                "{}: [citations.{citing}] names an unwalked kind `{citing}` (its home is `scan = false`, so no file in it is checked and the rule could never fire)",
                format_path(path)
            ));
        }
        // Flatten every target with the level it was declared at, rejecting any
        // that names an unconfigured kind on the way.
        let mut targets: Vec<(&'static str, &CitationTarget)> = Vec::new();
        let levels: [(&'static str, &[CitationDisjunction]); 5] = [
            ("must", &rules.must),
            ("should", &rules.should),
            ("may", &rules.may),
            ("should-not", &rules.should_not),
            ("must-not", &rules.must_not),
        ];
        for (level_name, disjunctions) in levels {
            for disjunction in disjunctions {
                for target in &disjunction.targets {
                    if !known.contains(target.kind.as_str()) {
                        // A non-citable kind is a name the table knows and a
                        // citation can never carry, so say which of the two it
                        // is rather than calling a configured kind unknown.
                        let why = if citing_known.contains(target.kind.as_str()) {
                            "a non-citable target kind"
                        } else {
                            "an unknown target kind"
                        };
                        return Err(anyhow!(
                            "{}: [citations.{citing}] {level_name} names {why} `{}`",
                            format_path(path),
                            target.kind
                        ));
                    }
                    targets.push((level_name, target));
                }
            }
        }
        // Two targets of one kind whose matchers can match the same citation (e.g.
        // bare `AR` and `*/AR`) must not sit at different levels — such a citation
        // would have no single level (§FS-config.3.9.5). Identical entries are fine.
        for (index, (level_a, a)) in targets.iter().enumerate() {
            for (level_b, b) in targets.iter().skip(index + 1) {
                if level_a != level_b
                    && a.kind == b.kind
                    && namespaces_overlap(&a.namespace, &b.namespace)
                {
                    return Err(anyhow!(
                        "{}: [citations.{citing}] `{}` ({level_a}) and `{}` ({level_b}) overlap (a citation matching both has no single level)",
                        format_path(path),
                        render_citation_target(a),
                        render_citation_target(b)
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Whether two rule-target namespace matchers can match the same citation
/// (§FS-config.3.9.3): `*/` (any namespace) overlaps every qualifier; otherwise
/// two matchers overlap only when identical — both local, or the same pinned
/// alias. A local matcher and a pinned-alias matcher are disjoint, so permitting
/// a local kind while forbidding one member's same kind is allowed.
fn namespaces_overlap(a: &NamespaceMatch, b: &NamespaceMatch) -> bool {
    match (a, b) {
        (NamespaceMatch::Any, _) | (_, NamespaceMatch::Any) => true,
        (NamespaceMatch::Local, NamespaceMatch::Local) => true,
        (NamespaceMatch::Alias(left), NamespaceMatch::Alias(right)) => left == right,
        _ => false,
    }
}

/// The namespace qualifier as written in config — for round-tripping in
/// `grund config show` and for the duplicate-target message (§FS-config.3.9).
fn citation_namespace_label(namespace: &NamespaceMatch) -> String {
    match namespace {
        NamespaceMatch::Local => String::new(),
        NamespaceMatch::Any => "*/".to_string(),
        NamespaceMatch::Alias(alias) => format!("{alias}/"),
    }
}

fn render_citation_target(target: &CitationTarget) -> String {
    format!("{}{}", citation_namespace_label(&target.namespace), target.kind)
}
