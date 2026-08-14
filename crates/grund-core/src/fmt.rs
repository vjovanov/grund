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
    let mut changes: Vec<(PathBuf, usize, &'static str)> = Vec::new();
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
                workspace: workspace_for_wrap,
                precomputed_findings: Some(&project.findings),
            };
            match fmt_tree(
                &project.config,
                Some(&project.config.root),
                true,
                &opts,
            ) {
                Ok(mut project_changes) => changes.append(&mut project_changes),
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
            workspace: workspace_for_wrap,
            precomputed_findings: reusable_findings,
        };
        match fmt_tree(&config, Some(&path), path_provided, &opts) {
            Ok(project_changes) => changes = project_changes,
            Err(err) => {
                eprintln!("error: {err:#}");
                return ExitCode::from(2);
            }
        }
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
    if write || changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

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

/// Walk the tree and rewrite each scannable file line by line — never touching a
/// declaration heading or anything inside a fenced code block (§FS-fmt.2.3) — and
/// either write the changes back (`--write`) or just collect `(path, line, label)`
/// for `--check`/dry-run (§FS-fmt.3). `--cross-refs` needs the full `Findings` first
/// so a link is only emitted when its target resolves (§FS-fmt.6.3).
/// What `fmt_tree` rewrites and against what context — grouped so the walk
/// inputs (config + scope) and the rewrite knobs travel separately.
struct FmtRunOpts<'a> {
    add_marker: bool,
    cross_refs: bool,
    write: bool,
    workspace: Option<&'a WorkspaceContext>,
    /// Whole-project findings, when the caller has already produced them
    /// (workspace-root `fmt` reuses each project's `WorkspaceContext` scan).
    /// `None` falls back to `scan_tree_strict` inside `fmt_tree`.
    precomputed_findings: Option<&'a Findings>,
}

fn fmt_tree(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    opts: &FmtRunOpts<'_>,
) -> Result<Vec<(PathBuf, usize, &'static str)>> {
    let mut changes = Vec::new();
    let add_marker = opts.add_marker;
    let cross_refs = opts.cross_refs;
    let write = opts.write;
    let workspace = opts.workspace;
    let precomputed_findings = opts.precomputed_findings;
    // §FS-fmt.6.3: a wrap's URL is computed from the declaration's home file,
    // which may live anywhere in the project tree — not necessarily inside the
    // scope being rewritten. Reuse the caller's whole-project findings when
    // they have them (workspace mode shares the `WorkspaceContext` scan
    // across `fmt`'s rewrite walk — no second disk pass per project), and
    // fall back to a strict project scan otherwise. The fallback preserves
    // §FS-fmt.6.2 for `grund fmt --cross-refs path/to/file.md`, where the
    // caller's findings are scope-narrow and a cross-file home would
    // otherwise be invisible.
    //
    // §FS-fmt.6.3 needs the whole-project findings up front, because a wrap's
    // URL depends on a declaration that may live outside the rewrite scope.
    // §FS-fmt.2.4 needs the same declaration set, but only if the tree actually
    // contains a shorthand — so it is *not* scheduled here. `fmt --check` has no
    // other reason to scan, and paying for one on every run of every numbered
    // repo to serve a rewrite most of them never need is the wrong trade
    // (§GOAL-fast-feedback). Instead the walk below starts without findings and
    // scans on the first candidate it meets.
    let owned_findings = if cross_refs && precomputed_findings.is_none() {
        Some(scan_tree_strict(config, None, false)?)
    } else {
        None
    };
    let mut findings: Option<&Findings> = if cross_refs {
        precomputed_findings.or(owned_findings.as_ref())
    } else {
        precomputed_findings
    };
    // Holds the scan the shorthand pass triggers, so the borrow in `findings`
    // outlives the file that asked for it.
    #[allow(unused_assignments)]
    let mut shorthand_findings: Option<Findings> = None;
    for path in walk_scannable_files(config, scope, explicit_scope)? {
        let original =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
        let file_changes_start = changes.len();
        let mut rewritten = rewrite_file(&original, &path, config, is_md, &FmtLineOpts {
            add_marker,
            cross_refs,
            findings,
            workspace,
        }, &mut changes);
        // §FS-fmt.2.4: this file wants a shorthand expanded and we have no
        // declarations yet. Scan once, then redo *this* file — every file
        // already walked is final, because having no candidate is exactly why
        // the scan had not happened by then.
        if rewritten.saw_shorthand_candidate && findings.is_none() {
            shorthand_findings = Some(scan_tree_strict(config, None, false)?);
            findings = shorthand_findings.as_ref();
            changes.truncate(file_changes_start);
            rewritten = rewrite_file(&original, &path, config, is_md, &FmtLineOpts {
                add_marker,
                cross_refs,
                findings,
                workspace,
            }, &mut changes);
        }
        if write && rewritten.changed {
            let mut output = rewritten.lines.join("\n");
            if original.ends_with('\n') {
                output.push('\n');
            }
            fs::write(&path, output).with_context(|| format!("write {}", path.display()))?;
        }
    }
    Ok(changes)
}


/// One file's rewritten lines plus what the walk needs to decide afterwards:
/// whether anything changed, and whether a shorthand expansion was wanted but
/// could not be performed for lack of the declaration set (§FS-fmt.2.4).
struct RewrittenFile {
    lines: Vec<String>,
    changed: bool,
    saw_shorthand_candidate: bool,
}

/// Apply `fmt_line` to every rewritable line of one file, appending each changed
/// line to `changes`. Fenced blocks and declaration headings are passed through
/// untouched (§FS-fmt.2.3).
fn rewrite_file(
    original: &str,
    path: &Path,
    config: &Config,
    is_md: bool,
    opts: &FmtLineOpts<'_>,
    changes: &mut Vec<(PathBuf, usize, &'static str)>,
) -> RewrittenFile {
    let mut in_fence = false;
    let mut lines = Vec::new();
    let mut changed = false;
    let mut saw_shorthand_candidate = false;
    for (idx, line) in original.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_md && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            in_fence = !in_fence;
            lines.push(line.to_string());
            continue;
        }
        if in_fence || declaration_captures(&config.grammar, line, false, is_md).is_some() {
            lines.push(line.to_string());
            continue;
        }
        let (new_line, label) = fmt_line(line, path, config, is_md, opts, &mut saw_shorthand_candidate);
        if new_line != line {
            changes.push((path.to_path_buf(), idx + 1, label));
            changed = true;
        }
        lines.push(new_line);
    }
    RewrittenFile {
        lines,
        changed,
        saw_shorthand_candidate,
    }
}

/// The rewrites `fmt_line` runs and their inputs — grouped so `fmt_line` has
/// one logical "what to rewrite" parameter instead of three flags plus two
/// optional findings handles.
struct FmtLineOpts<'a> {
    add_marker: bool,
    cross_refs: bool,
    findings: Option<&'a Findings>,
    workspace: Option<&'a WorkspaceContext>,
}

/// Apply the `fmt` rewrites to one line, in order: trigger→marker (§FS-fmt.2.1),
/// then optionally bare→marker (§FS-fmt.2.2), then shorthand→canonical
/// (§FS-fmt.2.4), then optionally Markdown-link wrapping (§FS-fmt.6) —
/// returning the new line plus a label naming the most significant rewrite that
/// fired.
///
/// The shorthand pass runs after the trigger pass and reads its output, so a
/// typed `$$FS-042` is marked and then expanded within the one call — which is
/// how §FS-fmt.2.4's "in one step" holds without the trigger pass needing to
/// know about declarations.
fn fmt_line(
    line: &str,
    path: &Path,
    config: &Config,
    is_md: bool,
    opts: &FmtLineOpts<'_>,
    saw_shorthand_candidate: &mut bool,
) -> (String, &'static str) {
    let triggered = replace_trigger(line, config, is_md);
    let trigger_changed = triggered != line;
    let marked = if opts.add_marker {
        add_markers(&triggered, config, is_md)
    } else {
        triggered.clone()
    };
    let marker_changed = marked != triggered;
    // Each stage below takes ownership of the previous stage's line rather than
    // cloning it: `fmt` touches every line of every scanned file, so one avoidable
    // allocation per line is a measurable share of the command (§GOAL-fast-feedback).
    // `expand_shorthand_citations` returns `None` for "unchanged" exactly so the
    // common line can be moved through untouched.
    let expansion = expand_shorthand_citations(
        &marked,
        config,
        is_md,
        opts.findings,
        saw_shorthand_candidate,
    );
    let shorthand_changed = expansion.is_some();
    let mut final_line = expansion.unwrap_or(marked);
    let mut link_changed = false;
    if opts.cross_refs
        && is_md
        && let Some(findings) = opts.findings
    {
        let wrapped = wrap_markdown_links(&final_line, path, config, findings, opts.workspace);
        link_changed = wrapped != final_line;
        final_line = wrapped;
    }
    let label = if trigger_changed {
        "trigger \u{2192} marker"
    } else if marker_changed {
        "bare \u{2192} marker"
    } else if shorthand_changed {
        "shorthand \u{2192} canonical"
    } else if link_changed {
        "markdown link"
    } else {
        ""
    };
    (final_line, label)
}

/// Rewrite each `$$<ID>` trigger to `§<ID>` — but only where `$$` is immediately
/// followed by a real ID-shaped token, and never inside a string literal in source
/// code or Markdown link destinations (§FS-fmt.2.1, §FS-fmt.2.3.1,
/// §DF-reference-marker).
fn replace_trigger(line: &str, config: &Config, is_md: bool) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find(&config.trigger) {
        let start = cursor + relative;
        let after = start + config.trigger.len();
        if id_token_end_at(line, after, &config.grammar).is_some()
            && (is_md || !is_inside_string_literal(line, start))
            && (!is_md || !is_inside_inline_code(line, start))
            && (!is_md || !is_inside_markdown_link_destination(line, start))
        {
            output.push_str(&line[cursor..start]);
            output.push_str(&config.marker);
            cursor = after;
            continue;
        }
        output.push_str(&line[cursor..after]);
        cursor = after;
    }
    output.push_str(&line[cursor..]);
    output
}

/// Prefix `§` onto bare ID-shaped tokens that lack it — the `--marker` upgrade
/// (§FS-fmt.2.2) — skipping tokens already marked, Markdown inline-code examples,
/// Markdown link destinations, and source-code string literals (§FS-fmt.2.3).
fn add_markers(line: &str, config: &Config, is_md: bool) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    for caps in config.grammar.citation_re.captures_iter(line) {
        let Some(found) = caps.get(0) else { continue };
        // §FS-workspace.1: a `path/ID` token without a marker is text, not a
        // citation — `fmt --marker` must not auto-promote it to `§path/ID`.
        if caps.name("namespace").is_some() {
            continue;
        }
        if line[..found.start()].ends_with(&config.marker) {
            continue;
        }
        if is_md && is_inside_inline_code(line, found.start()) {
            continue;
        }
        if is_md && is_inside_markdown_link_destination(line, found.start()) {
            continue;
        }
        if !is_md && is_inside_string_literal(line, found.start()) {
            continue;
        }
        output.push_str(&line[cursor..found.start()]);
        output.push_str(&config.marker);
        output.push_str(found.as_str());
        cursor = found.end();
    }
    output.push_str(&line[cursor..]);
    output
}

/// Whether byte offset `pos` falls inside a `'…'`, `"…"`, or `` `…` `` literal on
/// this line — the source-code exclusion that keeps an ID printed in a string from
/// being treated as a citation by the scanner or rewritten by `fmt` (§FS-fmt.2.3.1).
fn is_inside_string_literal(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut single = false;
    let mut double = false;
    let mut backtick = false;
    let mut i = 0;
    while i < pos && i < bytes.len() {
        match bytes[i] {
            b'\'' if !double && !backtick && !is_escaped(bytes, i) => single = !single,
            b'"' if !single && !backtick && !is_escaped(bytes, i) => double = !double,
            b'`' if !single && !double && !is_escaped(bytes, i) => backtick = !backtick,
            _ => {}
        }
        i += 1;
    }
    single || double || backtick
}

/// Whether byte offset `pos` falls inside a `` `…` `` inline-code span in Markdown
/// — citations there are illustrative, not real, so `fmt` leaves them alone
/// (§FS-fmt.2.3, §FS-fmt.6.4).
fn is_inside_inline_code(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut in_code = false;
    let mut i = 0;
    while i < pos && i < bytes.len() {
        if bytes[i] == b'`' && !is_escaped(bytes, i) {
            in_code = !in_code;
        }
        i += 1;
    }
    in_code
}

/// Whether byte offset `pos` falls inside the destination part of an inline
/// Markdown link (`[text](destination)`). URLs are presentation syntax, not
/// citations, so `fmt --marker` must not rewrite ID-shaped file names there
/// (§FS-fmt.2.3).
fn is_inside_markdown_link_destination(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b'(' && !is_escaped(bytes, i) {
            let start = i + 2;
            let mut depth = 1usize;
            let mut j = start;
            while j < bytes.len() {
                match bytes[j] {
                    b'(' if !is_escaped(bytes, j) => depth += 1,
                    b')' if !is_escaped(bytes, j) => {
                        depth -= 1;
                        if depth == 0 {
                            if pos >= start && pos < j {
                                return true;
                            }
                            i = j;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if j >= bytes.len() {
                return pos >= start;
            }
        }
        i += 1;
    }
    false
}
