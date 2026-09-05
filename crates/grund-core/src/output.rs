/// Print the text report in the fixed output shapes (§FS-errors.1,
/// §FS-errors.2.1, §FS-errors.2.4): `path:line: message` for located findings,
/// run-level diagnostics on stderr, and `success` for a clean text check
/// (§FS-check.2.1). Diagnostic lines stay in the fixed order (§FS-errors.4).
fn print_report(config: &Config, report: &CheckReport, include_suggestions: bool) {
    // §FS-check.2.3: the `success` marker keys off errors and warnings only — a
    // suggestion is not a finding about well-formedness, so it never suppresses
    // `success`, and without `--suggestions` it is not printed at all.

    // §FS-check.4.11: a warning printed before this report existed, so the marker
    // asks the config too — else stderr says unchecked and stdout says `success`.
    if config.unread_opted_out_blocks == 0
        && report.errors.is_empty()
        && report.warnings.is_empty()
        && (!include_suggestions || report.suggestions.is_empty())
    {
        println!("success");
        return;
    }
    let mut diagnostics = report
        .warnings
        .iter()
        .map(|diagnostic| ("warning", diagnostic))
        .chain(report.errors.iter().map(|diagnostic| ("error", diagnostic)))
        .collect::<Vec<_>>();
    if include_suggestions {
        diagnostics.extend(
            report
                .suggestions
                .iter()
                .map(|diagnostic| ("suggestion", diagnostic)),
        );
    }
    diagnostics.sort_by(|(_, a), (_, b)| diagnostic_cmp(a, b));
    for (severity, diagnostic) in diagnostics {
        let line = render_diagnostic_text(config, severity, diagnostic);
        // §FS-errors.1 / §FS-check.2.1: a located finding is `check`'s output →
        // stdout. A `line`-less diagnostic — a mid-walk read failure (§FS-check.2)
        // or the empty-scan caution (§FS-check.2.2) — is about the run → stderr.
        if diagnostic.line.is_some() {
            println!("{line}");
        } else {
            eprintln!("{line}");
        }
    }
}

fn render_diagnostic_text(config: &Config, severity: &str, diagnostic: &Diagnostic) -> String {
    match (&diagnostic.path, diagnostic.line) {
        (Some(path), Some(line)) => {
            format!(
                "{}:{}: {}",
                display_path(config, path),
                line,
                diagnostic.message
            )
        }
        // A file-level finding with no line to point at (e.g. an unreadable file
        // discovered mid-walk) uses the CLI-level shape — §FS-check.2, §FS-errors.2.2.
        (Some(path), None) => format!(
            "{severity}: {}: {}",
            display_path(config, path),
            diagnostic.message
        ),
        _ => format!("{severity}: {}", diagnostic.message),
    }
}

fn sorted_json_diagnostics(
    report: &CheckReport,
    include_suggestions: bool,
) -> Vec<(&'static str, &Diagnostic)> {
    let mut diagnostics = report
        .warnings
        .iter()
        .map(|diagnostic| ("warning", diagnostic))
        .chain(report.errors.iter().map(|diagnostic| ("error", diagnostic)))
        .collect::<Vec<_>>();
    if include_suggestions {
        diagnostics.extend(
            report
                .suggestions
                .iter()
                .map(|diagnostic| ("suggestion", diagnostic)),
        );
    }
    diagnostics.sort_by(|(_, a), (_, b)| diagnostic_cmp(a, b));
    diagnostics
}

/// Print the report as newline-delimited JSON objects — the `--format json` /
/// `[output] format = "json"` shape (§FS-errors.5): one object per finding with
/// `severity`, `path`, `line`, `code`, `message`, `sites`. Located findings go to
/// stdout (`check`'s output, §FS-errors.1); a `line`-less diagnostic (mid-walk read
/// failure, empty-scan caution) goes to stderr, mirroring the text form.
fn print_json_report(config: &Config, report: &CheckReport, include_suggestions: bool) {
    for (channel, diagnostic) in sorted_json_diagnostics(report, include_suggestions) {
        let object = render_diagnostic_json(config, channel, diagnostic);
        if diagnostic.line.is_some() {
            println!("{object}");
        } else {
            eprintln!("{object}");
        }
    }
}

/// Render one diagnostic as a JSON object (§FS-errors.5). An `error` / `warning`
/// carries a `"severity"`; a citation-direction `suggestion` carries
/// `"channel":"suggestion"` instead, keeping the frozen `{error, warning}`
/// severity set intact (§FS-config.6, §FS-check.2.3).
fn render_diagnostic_json(config: &Config, channel: &str, diagnostic: &Diagnostic) -> String {
    let path = diagnostic
        .path
        .as_ref()
        .map(|path| format!("\"{}\"", json_escape(&display_path(config, path))))
        .unwrap_or_else(|| "null".to_string());
    let line = diagnostic
        .line
        .map(|line| line.to_string())
        .unwrap_or_else(|| "null".to_string());
    let sites = if diagnostic.sites.is_empty() {
        "null".to_string()
    } else {
        let values = diagnostic
            .sites
            .iter()
            .map(|site| {
                format!(
                    "{{\"path\":\"{}\",\"line\":{}}}",
                    json_escape(&display_path(config, &site.path)),
                    site.line
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", values)
    };
    let tag = if channel == "suggestion" {
        "\"channel\":\"suggestion\"".to_string()
    } else {
        format!("\"severity\":\"{channel}\"")
    };
    format!(
        "{{{},\"path\":{},\"line\":{},\"code\":\"{}\",\"message\":\"{}\",\"sites\":{}}}",
        tag,
        path,
        line,
        diagnostic.code,
        json_escape(&diagnostic.message),
        sites
    )
}

/// §FS-errors.5: `sites` here are already display strings from the raise site
/// in `show_render.rs` (rendered against `path_config`, which may be a
/// workspace root this printer's own `Config` is not), so they are rendered
/// through the shared [`render_finding_sites_json`] rather than re-derived
/// from a `Config` the way `render_diagnostic_json` renders `check`'s sites.
fn print_bare_query_json(code: &'static str, message: &str, sites: &[FindingSite]) {
    eprintln!(
        "{{\"severity\":\"error\",\"path\":null,\"line\":null,\"code\":\"{}\",\"message\":\"{}\",\"sites\":{}}}",
        code,
        json_escape(message),
        render_finding_sites_json(sites)
    );
}

fn show_query_error_code(message: &str) -> &'static str {
    if message.starts_with("ID not found:") {
        "not-found"
    } else if message.starts_with("section not found:") {
        "missing-section"
    } else if message.starts_with("invalid ID") {
        "invalid-id"
    } else if message.starts_with("ambiguous ID:") {
        "ambiguous"
    // §FS-show.2.2.2: the section-level twin of the ambiguous-ID refusal, under
    // its own code — the two need different edits, and a JSON consumer should not
    // have to read the prose to tell them apart (§DF-duplicate-section-path.2.5).
    } else if message.starts_with("ambiguous section:") {
        "ambiguous-section"
    } else if message.starts_with("broken stub:") {
        "broken-stub"
    } else {
        "query-failed"
    }
}

fn json_escape(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => escaped.push_str(&format!("\\u{:04x}", other as u32)),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Render a path the way reports show it: relative to the repo root by default,
/// or relative to the CLI base directory when `[output] relative_paths = false`
/// (§FS-config.3.6 — an in-root target outside that base uses bounded `..`).
fn display_path(config: &Config, path: &Path) -> String {
    let base = if config.relative_paths {
        &config.root
    } else {
        &config.cli_base
    };
    let relative = path.strip_prefix(base).map(Path::to_path_buf).unwrap_or_else(|_| {
        if !config.relative_paths && path.starts_with(&config.root) {
            relative_from_base(base, path)
        } else {
            path.to_path_buf()
        }
    });
    format_path(&relative)
}

/// Render an explicit lexical scope without resolving its final symlink.
/// Canonical ancestors only identify the report base when the OS respells it.
fn display_lexical_scope(config: &Config, path: &Path) -> String {
    let base = if config.relative_paths {
        &config.root
    } else {
        &config.cli_base
    };
    for ancestor in path.ancestors() {
        if fs::canonicalize(ancestor)
            .map(|resolved| resolved == *base)
            .unwrap_or(false)
        {
            return format_path(path.strip_prefix(ancestor).unwrap_or(path));
        }
    }
    display_path(config, path)
}

fn format_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// `path` expressed relative to `base`, walking up with `..` for a target that
/// lies *outside* it (§FS-errors.4: a report never carries an absolute path, and a
/// reader resolves what it prints against the directory they are standing in). The
/// case that needs it is a config file **above** the run's root: an enclosing
/// workspace's `members` line, reported at a run narrowed into one of its members
/// (§FS-workspace.6.1). Both paths are canonical there. A pair with no shared
/// component at all — different Windows prefixes — has no relative form, so the
/// target is returned unchanged.
fn relative_from_base(base: &Path, path: &Path) -> PathBuf {
    if let Ok(inside) = path.strip_prefix(base) {
        return inside.to_path_buf();
    }
    let base_parts: Vec<_> = base.components().collect();
    let path_parts: Vec<_> = path.components().collect();
    let shared = base_parts
        .iter()
        .zip(&path_parts)
        .take_while(|(a, b)| a == b)
        .count();
    if shared == 0 {
        return path.to_path_buf();
    }
    let mut relative = PathBuf::new();
    for _ in shared..base_parts.len() {
        relative.push("..");
    }
    relative.extend(&path_parts[shared..]);
    relative
}

fn sort_path_key(path: &Path) -> String {
    format_path(path)
}

/// Whether the path `check` was handed is a **file** that the hidden-name rule
/// alone kept out of the walk (§FS-check.2.2): the caller really handed a path,
/// its own name begins with `.`, and its extension is one `[scan] extensions`
/// lists — so the extension list is the one rule that did *not* decide, and the
/// message must not send the reader there.
///
/// Every term is load-bearing. Without `path_provided` a library caller pairing a
/// hidden `path` with `path_provided: false` would be told a file the run never
/// looked at was skipped, since that run walks `[scan] include` and ignores `path`
/// entirely. A hidden file whose extension is *also* unlisted has two reasons and
/// keeps the extension message, because naming one of two causes is its own
/// misdirection. And the `is_file` test keeps this to files: a hidden **directory**
/// handed explicitly is walked (§FS-config.3.5), so an empty one really did match
/// no extensions — and a directory whose only content is hidden keeps the extension
/// message too, a boundary §FS-check.2.2 states rather than leaves to be found.
fn handed_a_hidden_file(config: &Config, path: &Path, path_provided: bool) -> bool {
    path_provided
        && path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'))
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| config.extensions.iter().any(|allowed| allowed == ext))
}

/// The CLI-level warning `check` reports when the tree walk matched no files
/// (§FS-check.2.2): a scan that read nothing is almost always a misconfigured
/// scope, so we say so instead of printing nothing and exiting `0`. This is a
/// warning — it never changes the exit code. Which of the three messages it
/// carries is a question about *why* nothing was read, so the hidden file is
/// asked first: it was skipped before either of the arms below could decide.
fn empty_scan_warning(config: &Config, path: &Path, path_provided: bool) -> Diagnostic {
    // `grund`, `grund check .`, and `grund check <repo-root>` all walk `[scan] include`
    // relative to the config root — so the "looked under include" message is the
    // accurate one whenever the requested path *is* that root, not just when the
    // path was omitted.
    let scoped_to_root = !path_provided
        || path == Path::new(".")
        || fs::canonicalize(path)
            .map(|p| p == config.root)
            .unwrap_or(false);
    let message = match (&config.include, scoped_to_root) {
        // §FS-check.2.2: name the hidden-name rule that actually skipped the file,
        // rather than the `include` list or the extensions that never got to answer.
        _ if handed_a_hidden_file(config, path, path_provided) => format!(
            "nothing to scan — `{}` is a hidden file. grund reads no file whose own name \
             begins with `.`, whatever `[scan] extensions` says. Rename it, or move what \
             needs checking into a file that is not hidden.",
            format_path(path)
        ),
        (Some(dirs), true) => format!(
            "nothing to scan — grund looked under [scan] include = [{}] and found no files. Run \
             `grund init --docs` to scaffold the canonical requirements.md, docs/, and e2e/ \
             trees, point `[scan] include` in `grund.toml` at your sources, or pass a \
             path explicitly (`grund check <dir>`).",
            dirs.iter()
                .map(|dir| format!("\"{dir}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!(
            "nothing to scan — no files under `{}` matched grund's extensions ({}).",
            format_path(path),
            config.extensions.join(", ")
        ),
    };
    Diagnostic {
        code: "empty-scan",
        path: None,
        line: None,
        column: None,
        message,
        sites: Vec::new(),
    }
}

/// The CLI-level warning `check` reports when the walk read files and recognized
/// nothing in them — no declaration and no citation (§FS-check.4.5). The empty
/// scan above says the scope found no files; this says the scope was right and
/// the grammar matched none of their content, which is what a docs tree written
/// for a different `[id] format` looks like from the inside. A warning, so the
/// exit code is untouched — what it takes away is the `success` marker
/// (§DF-nothing-recognized.2.2).
///
/// The shapes come from the configured template (`id_shape`) and the marker, and
/// the kinds are named in config order; nothing here is derived from the tree, so
/// two runs over one config print one string (§FS-errors.4).
///
/// The closing sentence offers both readings because the run cannot tell them
/// apart without judging a line, which is §FS-check.4.7's job: a tree
/// written to another format and a `grund init` scaffold nobody has declared in
/// yet produce the identical fact, and naming only the first would send a fresh
/// adopter to look for a bug in a config that is fine.
fn nothing_recognized_warning(config: &Config, scanned_files: usize) -> Diagnostic {
    let shape = id_shape(&config.id_format);
    let files = if scanned_files == 1 { "file" } else { "files" };
    Diagnostic {
        code: "nothing-recognized",
        path: None,
        line: None,
        column: None,
        message: format!(
            "nothing recognized — grund read {scanned_files} {files} and found no declaration \
             and no citation in them. A declaration heading reads `# {shape}: <title>` and a \
             citation `{marker}{shape}`, under [id] format = \"{format}\" with <KIND> one of \
             {{{kinds}}}. Either nothing is declared yet, or the headings are written to a \
             different shape than that.",
            marker = config.marker,
            format = config.id_format,
            kinds = kind_prefixes(&config.kinds).join(", "),
        ),
        sites: Vec::new(),
    }
}

/// §FS-check.1.3: the caution a `--full` run earns when the caller also typed a
/// path that is not the config root. `--full` cancels `[scan] include`, and an
/// explicit path already bypasses that key, so the flag has nothing left to
/// cancel and the run is the ordinary one. A warning rather than a rejection:
/// the invocation is valid, and a script that passes `--full` uniformly must not
/// fail on the one call where it is redundant. Like every warning it leaves the
/// exit code alone and, per §FS-check.2.1, stands in place of the `success`
/// marker on an otherwise clean run.
fn full_scope_ignored_warning(
    config: &Config,
    path: &Path,
    path_provided: bool,
    full: bool,
) -> Option<Diagnostic> {
    if !full || scope_is_config_root(config, path, path_provided) {
        return None;
    }
    // §FS-config.3.5.2 / §FS-config.3.6: the warning reports the explicit
    // spelling the scanner keeps, not the physical target an in-tree symlink
    // resolves to. Identity checks above may canonicalize; report text may not.
    let lexical_scope = if path.is_absolute() {
        normalize_path_lexically(path)
    } else {
        std::env::current_dir()
            .map(|cwd| normalize_path_lexically(&cwd.join(path)))
            .unwrap_or_else(|_| normalize_path_lexically(path))
    };
    Some(Diagnostic {
        code: "full-scope-ignored",
        path: None,
        line: None,
        column: None,
        message: format!(
            "--full has no effect with an explicit PATH — it cancels [scan] include, and {} \
             already bypasses it",
            display_lexical_scope(config, &lexical_scope)
        ),
        sites: Vec::new(),
    })
}

/// Print [`config_warnings`] in the CLI-level shape (§FS-errors.2.2): one
/// `warning: ` line each, on stderr, exit code untouched. Rendering, so it
/// lives with the other report printers rather than in `api.rs`, which is the
/// embedding surface *without* stdout/stderr rendering
/// (§AR-core-module-layout.1) — and in one place rather than in each `config`
/// frontend, so the published CLI and the deprecated `grund_core` adapter
/// cannot drift on the prefix or the stream (§FS-config.4.1, §FS-config.4.2).
pub fn print_config_warnings(config: &Config) {
    for warning in config_warnings(config) {
        eprintln!("warning: {warning}");
    }
}

/// §FS-check.4.9, §DF-unlisted-workspace-block.2.4: the unlisted-`[workspace]`
/// finding on the five surfaces that have no report to carry it — `list`, `refs`,
/// `cover`, `fmt` and the ID read. One CLI-level `warning:` per block on stderr
/// (§FS-errors.2.2), the identical text `check` puts in `report.warnings`, so what
/// a consumer greps for does not depend on which command produced it.
///
/// Here rather than beside the rule for the reason `print_config_warnings` above is
/// here: rendering belongs to the output category, and `workspace_unlisted.rs`
/// builds the message and prints nothing (§AR-core-module-layout.1, §AR-bindings.2).
fn print_unlisted_workspace_block_warnings(
    config: &Config,
    render: &Config,
    alias: Option<&str>,
    walked_dirs: &[PathBuf],
) {
    for warning in unlisted_workspace_block_warnings(config, render, alias, walked_dirs) {
        eprintln!("warning: {}", warning.message);
    }
}

/// §FS-check.4.3: the warning for a config root holding both discovery names —
/// the bare `grund.toml` won, and the `.agents/grund.toml` beside it is read by
/// nothing (§FS-config.1.1). `line`-less, so it prints as a CLI-level `warning:`
/// on stderr: it says which file the run read, not what is wrong at a site.
fn redundant_config_warning(config: &Config) -> Option<Diagnostic> {
    let ignored = config.redundant_config_file.as_ref()?;
    let winner = config.config_file.as_ref()?;
    Some(Diagnostic {
        code: "redundant-config",
        path: None,
        line: None,
        column: None,
        message: format!(
            "{} is ignored — {} takes precedence; delete one",
            format_path(ignored),
            format_path(winner)
        ),
        sites: Vec::new(),
    })
}

/// §FS-check.4.12: the warning for a config the run read from the deprecated
/// `.agents/` location — the file still governs the project, so the message
/// names the move a reader can type rather than a fault (§FS-config.1.2).
/// `line`-less for §4.3's reason, which this finding shares whole: the subject
/// is which file the run read, not a site inside it.
///
/// Keyed off the file actually read, which is what excludes the redundant pair
/// by construction: there the bare name won the tie, so `config_file` already
/// names the home path and §4.3 is the finding the directory earns
/// (§FS-config.1.1).
fn deprecated_config_location_warning(config: &Config) -> Option<Diagnostic> {
    let found = config.config_file.as_ref()?;
    let home = home_form_of(found)?;
    Some(Diagnostic {
        code: "deprecated-config-location",
        path: None,
        line: None,
        column: None,
        message: format!(
            "{} is a deprecated config location — move it to {}",
            format_path(found),
            format_path(&home)
        ),
        sites: Vec::new(),
    })
}

/// The findings a loaded config carries on its own — the redundant discovery
/// pair (§FS-check.4.3) and the deprecated `.agents/` location (§FS-check.4.12).
/// Both are known from the file the run read rather than from the walk, which is
/// why they arrive together.
///
/// One list rather than a call each at each site: `grund check` names them per
/// project and `grund config validate` and `grund config show` name them for
/// one, and a finding that reached only some of those surfaces would be a
/// finding a repository can hide from by choosing a command.
fn config_diagnostics(config: &Config) -> impl Iterator<Item = Diagnostic> {
    redundant_config_warning(config)
        .into_iter()
        .chain(deprecated_config_location_warning(config))
}

/// `a`, `b`, `c`, or `d` — a list spelled the way the message reads, joined by
/// the `conjunction` that message wants before the last item. Lives with the
/// other shared renderers rather than beside any one message: the refusals in
/// `init_target.rs` and the duplicate-entrypoint note in `init_notes.rs`
/// both spell a list, and neither owns the spelling (§AR-core-module-layout.1).
fn format_list(items: &[&str], conjunction: &str) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_string(),
        [first, second] => format!("{first} {conjunction} {second}"),
        [rest @ .., last] => format!("{}, {conjunction} {last}", rest.join(", ")),
    }
}
