/// What one `fmt` walk produced (§FS-fmt.3). Every path here is already rendered
/// against the run's config, because that is where the config naming it is at
/// hand; the command surface prints them and maps the exit code.
///
/// Citation normalization: one pass over a document that rewrites triggers to
/// markers, marks bare citations, expands shorthands, and keeps cross-reference
/// links current (§FS-fmt.2). The modes share the traversal — each line is asked
/// every question once — so what lives here is the walk and the per-line
/// decisions inside it. The command surface around it is `fmt_cmd.rs`, and the
/// link construction §FS-fmt.6 needs is `fmt_links.rs`.
struct FmtTreeOutcome {
    /// The lines it rewrote — or, in a dry run, would have.
    changes: Vec<(PathBuf, usize, String)>,
    /// The paths it could not read at all (§FS-check.2).
    scan_errors: Vec<ApiScanError>,
    /// The files it read and would not rewrite, because a link reaches them from
    /// outside the config root (§FS-fmt.2.3.2). Named in both modes: `--write`
    /// did not write them, and the dry run is saying `--write` will not.
    refused_writes: Vec<String>,
}

/// What `fmt_tree` rewrites and against what context — grouped so the walk
/// inputs (config + scope) and the rewrite knobs travel separately.
struct FmtRunOpts<'a> {
    add_marker: bool,
    cross_refs: bool,
    write: bool,
    /// The config every path in the *report* is rendered against — the workspace
    /// root's, where `check` renders too (§FS-fmt.3). Each project is walked and
    /// rewritten under its own config, and rendering against that one instead
    /// spelled a member's file from the member root: `docs/FS-003.md` for
    /// `packages/sub/docs/FS-003.md`, which is a different real file in the same
    /// run's output.
    render: &'a Config,
    workspace: Option<&'a WorkspaceContext>,
    /// Whole-project findings, when the caller has already produced them
    /// (workspace-root `fmt` reuses each project's `WorkspaceContext` scan).
    /// `None` falls back to a complete `scan_tree` inside `fmt_tree`, whose
    /// errors become one structured strict abort rather than a partial result.
    precomputed_findings: Option<&'a Findings>,
    /// §FS-fmt.6.1 / §DF-index-always-linkified: run the cross-reference pass on
    /// a kind's index file even where `[fmt.cross_refs] enabled = false` turned
    /// `cross_refs` off. It decides *which files* the pass touches when the pass
    /// runs at all; dry-run and write mode both enable this carve-out so the
    /// former previews the exact index-entry wraps the latter applies.
    index_cross_refs: bool,
}

/// Walk the tree and rewrite each scannable file line by line — never touching a
/// declaration heading or anything inside a fenced code block (§FS-fmt.2.3) — and
/// either write the changes back (`--write`) or just collect `(path, line, label)`
/// for `--check`/dry-run (§FS-fmt.3). `--cross-refs` needs the full `Findings` first
/// so a link is only emitted when its target resolves (§FS-fmt.6.3).
///
/// Why the link pass takes the whole project's declarations and the shorthand
/// pass does not: a wrap's URL is computed from the declaration's home file,
/// which may live anywhere in the project tree, so workspace mode reuses the
/// caller's whole-project findings — the `WorkspaceContext` scan, no second disk
/// pass per project — and falls back to a strict project scan otherwise. That
/// fallback preserves §FS-fmt.6.2 for `grund fmt --cross-refs path/to/file.md`,
/// where the caller's findings are scope-narrow and a cross-file home would
/// otherwise be invisible. A shorthand needs the same declaration set, but only
/// where the tree actually contains one, and paying for a scan on every run of
/// every numbered repo to serve a rewrite most of them never need is the wrong
/// trade (§GOAL-fast-feedback): the walk starts without findings and scans on the
/// first candidate it meets.
///
/// Why the unreadable paths are collected here: they are rendered while the
/// config that names them is at hand, and `fmt` walks the tree `check` walks —
/// the alternative is the one command that edits files in place also being the
/// one that will not say which files it never saw.
///
/// Why a file reached from outside the config root is refused: editing it would
/// put this project's bytes into a file the project does not own. The refusal is
/// named once, on stderr, with the exit code untouched — it is intended behavior
/// and not a failure of the run. The dry run refuses it too and reports no
/// rewrite for it: a dry run predicts what `--write` does, and a pending rewrite
/// `--write` will never perform is one no edit can clear, so `fmt --check` would
/// exit `1` on this tree forever and a gate built on it could never pass.
fn fmt_tree(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    opts: &FmtRunOpts<'_>,
) -> Result<FmtTreeOutcome> {
    let mut changes = Vec::new();
    let mut refused_writes = Vec::new();
    let add_marker = opts.add_marker;
    let cross_refs = opts.cross_refs;
    let write = opts.write;
    let workspace = opts.workspace;
    let precomputed_findings = opts.precomputed_findings;
    // §FS-fmt.6.3: the link pass needs the whole project's declarations, because
    // a wrap's URL comes from a home file that may sit outside the rewrite scope.
    // §FS-fmt.2.4's scan is deferred instead, to the first shorthand candidate.
    let walked = walk_scannable_files_reporting(config, scope, explicit_scope)?;
    // §FS-fmt.2.5.1: the files this config takes out of every rewrite. The walk
    // above is untouched — only what happens to each file's bytes changes.
    let excluded = FmtExcluded::new(config)?;
    // §FS-fmt.6.1: where the always-linkify carve-out may fire. Built for every
    // run that could need it, since §FS-fmt.2.5.3 makes it outrank a suppressed
    // scope too — a handful of configured paths, so nothing is deferred here.
    let index_files = if opts.index_cross_refs {
        KindIndexFiles::new(config)
    } else {
        KindIndexFiles::empty()
    };
    let index_in_scope =
        !index_files.is_empty() && walked.files.iter().any(|path| index_files.contains(path));
    let link_pass = cross_refs || index_in_scope;
    let owned_findings = if link_pass && precomputed_findings.is_none() {
        Some(fmt_findings_or_abort(config, opts.render)?)
    } else {
        None
    };
    let mut findings: Option<&Findings> = if link_pass {
        precomputed_findings.or(owned_findings.as_ref())
    } else {
        precomputed_findings
    };
    // §FS-fmt.6.1: which IDs each index owes an entry for. A pass over the
    // declarations, so it waits for the first file that asks (§GOAL-fast-feedback)
    // — an index in the walk forced the link pass, so they are there by then.
    let mut index_entries: Option<KindIndexEntries> = None;
    // Holds the scan the shorthand pass triggers, so the borrow in `findings`
    // outlives the file that asked for it.
    #[allow(unused_assignments)]
    let mut shorthand_findings: Option<Findings> = None;
    // §FS-fmt.2.4: built once for the whole walk, not once per line — see
    // `ShorthandTargets`. Rebuilt at most once, when the deferred scan lands.
    let mut shorthand_targets = ShorthandTargets::new(findings, workspace);
    // §FS-fmt.3: the paths this walk could not read, rendered here while the
    // config that names them is at hand — the same account `check` owes of the
    // tree it walks (§FS-check.2).
    let scan_errors: Vec<ApiScanError> = walked
        .errors
        .iter()
        .map(|(file, message)| api_scan_error(opts.render, file, message))
        .collect();
    for path in walked.files {
        // §FS-fmt.2.3.2: this file was reached through a link that leaves the
        // config root, so the rewrite stops here (§REQ-no-data-loss.2). The dry
        // run refuses it too, and reports no rewrite for it (§FS-fmt.3).
        if walked.outside_root.contains(&path) {
            refused_writes.push(display_path(opts.render, &path));
            continue;
        }
        let original =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
        // §FS-fmt.2.5.1: an excluded file keeps its bytes, so the ordinary pass
        // is off here — every rewrite with it (§FS-fmt.2.5).
        let file_excluded = excluded.contains(&path);
        // §FS-fmt.6.1, §FS-fmt.2.5.3: an index's entries are linkified whatever
        // turned the ordinary pass off here — `enabled = false`, the exclude
        // list, or a region the text carries. The entries, not the page.
        let carve_out = index_files.contains(&path)
            && (!cross_refs || file_excluded || original.contains(FMT_DIRECTIVE));
        if carve_out
            && index_entries.is_none()
            && let Some(findings) = findings
        {
            index_entries = Some(KindIndexEntries::new(findings, config));
        }
        let index_entry_ids = carve_out
            .then(|| index_entries.as_ref().and_then(|it| it.entries_in(&path)))
            .flatten();
        let cross_refs = cross_refs && !file_excluded;
        let file_changes_start = changes.len();
        let mut rewritten = rewrite_file(&original, &path, config, is_md, &FmtLineOpts {
            add_marker,
            cross_refs,
            excluded: file_excluded,
            index_entry_ids,
            findings,
            workspace,
            shorthand_targets: &shorthand_targets,
        }, &mut changes);
        // §FS-fmt.2.4: a shorthand to expand and no declarations yet. Scan once,
        // then redo *this* file — every file already walked is final, because
        // having no candidate is exactly why the scan had not happened by then.
        if rewritten.saw_shorthand_candidate && findings.is_none() {
            shorthand_findings = Some(fmt_findings_or_abort(config, opts.render)?);
            findings = shorthand_findings.as_ref();
            shorthand_targets = ShorthandTargets::new(findings, workspace);
            changes.truncate(file_changes_start);
            rewritten = rewrite_file(&original, &path, config, is_md, &FmtLineOpts {
                add_marker,
                cross_refs,
                excluded: file_excluded,
                index_entry_ids,
                findings,
                workspace,
                shorthand_targets: &shorthand_targets,
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
    Ok(FmtTreeOutcome {
        changes,
        scan_errors,
        refused_writes,
    })
}


/// The whole project's declarations for a run that cannot rewrite anything
/// without them — `--cross-refs`, or a shorthand to expand (§FS-fmt.2.4) — with
/// every unreadable path fatal up front and preserved for reporting (§FS-fmt.3).
///
/// Every refusal line names itself. Both `fmt` failures exit `2`, but the
/// partial-scan one means every readable file was rewritten and this one means
/// nothing was, and `--write` reaches this path in the ordinary case rather
/// than the exceptional one — it turns `--cross-refs` on by itself wherever
/// the scope holds Markdown (§FS-fmt.6.6). Paths are rendered against the run's
/// config, like every other path `fmt` prints.
fn fmt_findings_or_abort(config: &Config, render: &Config) -> Result<Findings> {
    let (findings, errors) = scan_tree(config, None, false)?;
    if !errors.is_empty() {
        return Err(FmtScanAbort {
            scan_errors: errors
                .into_iter()
                .map(|(path, message)| api_scan_error(render, &path, &message))
                .collect(),
        }
        .into());
    }
    Ok(findings)
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
/// untouched (§FS-fmt.2.3), and so is every line of a suppressed scope
/// (§FS-fmt.2.5) — the `grund:fmt` region state is carried here beside the fence
/// state, because both are facts about where in the file the reader has got to.
fn rewrite_file(
    original: &str,
    path: &Path,
    config: &Config,
    is_md: bool,
    opts: &FmtLineOpts<'_>,
    changes: &mut Vec<(PathBuf, usize, String)>,
) -> RewrittenFile {
    let mut markdown_fence = None;
    let mut lines = Vec::new();
    let mut changed = false;
    let mut saw_shorthand_candidate = false;
    // §FS-fmt.2.5.2: every file starts with the rewrite on — a region never
    // carries across files.
    let mut directives = FmtDirectives::new(config, is_md);
    // §FS-fmt.2.3.1: `fmt` judges a docstring line on its content, so it carries
    // the scanner's docstring state — advanced on *every* line, including the ones
    // passed through untouched, or one unvisited `"""` desynchronizes the file.
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let mut docstrings = DocstringCursor::new(is_py, config.docstring_python);
    for (idx, line) in original.lines().enumerate() {
        if is_md && markdown_fence_delimiter(&mut markdown_fence, line) {
            lines.push(line.to_string());
            continue;
        }
        // The state this line *starts* in, kept so each rewrite stage can re-derive
        // the span from the line it is about to change (see `DocstringCursor`).
        let entry = docstrings;
        let docstring = docstrings.advance(line);
        // §FS-fmt.2.5.2: a directive inside a fenced block is an illustration, so
        // the fence is asked first and toggles nothing; the directive line itself
        // is passed through whatever state it leaves behind.
        if markdown_fence.is_some() {
            lines.push(line.to_string());
            continue;
        }
        if directives.consume(line, docstring) {
            lines.push(line.to_string());
            continue;
        }
        if declaration_captures(
            &config.grammar,
            docstring.text_of(line),
            docstring.is_docstring(),
            is_md,
        )
        .is_some()
        {
            lines.push(line.to_string());
            continue;
        }
        // §FS-fmt.2.5: the file's own `[fmt] exclude` verdict, or the region the
        // directives above have opened. Either one leaves only the index carve-out.
        let suppressed = opts.excluded || !directives.rewriting();
        let (new_line, label) =
            fmt_line(line, entry, path, config, is_md, opts, suppressed, &mut saw_shorthand_candidate);
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
    /// §FS-fmt.2.5.1: this file is named by `[fmt] exclude`, so every line of it
    /// is suppressed. The per-region directives (§FS-fmt.2.5.2) are the other half
    /// of the same verdict and are read line by line in `rewrite_file`.
    excluded: bool,
    /// §FS-fmt.6.1: when this file is a kind's index the always-linkify carve-out
    /// may have to reach, the IDs that index owes an entry for — the only citations
    /// the pass wraps where the ordinary one is off. `None` for every other file,
    /// and read on a line only where that line's ordinary pass is off: under
    /// `[fmt.cross_refs] enabled = false`, or inside a suppressed scope
    /// (§FS-fmt.2.5.3). Elsewhere `cross_refs` already means "wrap what this file
    /// has" and the carve-out has nothing to add.
    index_entry_ids: Option<&'a BTreeSet<Id>>,
    findings: Option<&'a Findings>,
    workspace: Option<&'a WorkspaceContext>,
    /// The declaration indexes the shorthand rewrite resolves against, built once
    /// per walk (§FS-fmt.2.4). Separate from `findings` because a qualified
    /// shorthand reads another project's declarations entirely.
    shorthand_targets: &'a ShorthandTargets<'a>,
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
///
/// Why each stage takes ownership of the previous stage's line:
/// `expand_shorthand_citations` returns `None` for "unchanged" exactly so the
/// common line can be moved through untouched.
///
/// Why a shorthand expansion names the text it wrote in the label: the other
/// three rewrites move markup around an unchanged ID token, so `grund check` can
/// still see a mistake in them; this one writes the slug *into* the token, and a
/// wrong one is a well-formed citation of the wrong declaration.
#[allow(clippy::too_many_arguments)]
fn fmt_line(
    line: &str,
    docstrings: DocstringCursor,
    path: &Path,
    config: &Config,
    is_md: bool,
    opts: &FmtLineOpts<'_>,
    suppressed: bool,
    saw_shorthand_candidate: &mut bool,
) -> (String, String) {
    if suppressed {
        return suppressed_line(line, path, config, is_md, opts);
    }
    let triggered = replace_trigger(line, docstrings.peek(line), config, is_md);
    let trigger_changed = triggered != line;
    let marked = if opts.add_marker {
        add_markers(&triggered, docstrings.peek(&triggered), config, is_md)
    } else {
        triggered.clone()
    };
    let marker_changed = marked != triggered;
    // Each stage below takes ownership of the previous stage's line rather than
    // cloning it: `fmt` touches every line of every scanned file, so one avoidable
    // allocation per line is a measurable share of the command (§GOAL-fast-feedback).
    let mut expansions = Vec::new();
    let expansion = expand_shorthand_citations(
        &marked,
        docstrings.peek(&marked),
        config,
        is_md,
        opts.shorthand_targets,
        saw_shorthand_candidate,
        &mut expansions,
    );
    let shorthand_changed = expansion.is_some();
    let mut final_line = expansion.unwrap_or(marked);
    let mut link_changed = false;
    // §FS-fmt.6.1: the carve-out narrows the pass to the index's own entries only
    // where the ordinary pass is off; where it runs, it already wraps the page.
    let entry_ids = if opts.cross_refs { None } else { opts.index_entry_ids };
    if (opts.cross_refs || entry_ids.is_some())
        && is_md
        && let Some(findings) = opts.findings
    {
        let wrapped = wrap_markdown_links(&final_line, path, config, findings, opts.workspace,
            entry_ids);
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
    // §FS-fmt.3: whichever label won, a line that expanded a shorthand also names
    // the text it will write — the one rewrite no later pass can question
    // (§DF-shorthand-numeric-run.2.7).
    let label = if expansions.is_empty() {
        label.to_string()
    } else {
        format!(
            "{label}: {}",
            expansions
                .iter()
                .map(|(written, canonical)| format!("{written} \u{2192} {canonical}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    (final_line, label)
}

/// One line of a suppressed scope (§FS-fmt.2.5): an excluded file, or a
/// `grund:fmt off` region. Every rewrite is off here — the trigger, the marker
/// upgrade, the shorthand expansion, and the ordinary wrap alike — so the line
/// keeps its bytes and the report says nothing about it.
///
/// The one survivor is the always-linkify carve-out (§FS-fmt.2.5.3): a kind's
/// index entries are wrapped in a suppressed scope exactly as they are under
/// `[fmt.cross_refs] enabled = false`, because a suppression that could leave an
/// index in a state §FS-check.3.17 reports and `fmt` refuses to repair would
/// reopen the hole §DF-index-always-linkified closed. `index_entry_ids` is
/// `None` for every file that is not a configured index, which is the ordinary
/// case and returns the line untouched.
fn suppressed_line(
    line: &str,
    path: &Path,
    config: &Config,
    is_md: bool,
    opts: &FmtLineOpts<'_>,
) -> (String, String) {
    let unchanged = || (line.to_string(), String::new());
    if !is_md {
        return unchanged();
    }
    let (Some(entry_ids), Some(findings)) = (opts.index_entry_ids, opts.findings) else {
        return unchanged();
    };
    let wrapped = wrap_markdown_links(line, path, config, findings, opts.workspace, Some(entry_ids));
    if wrapped == line {
        return unchanged();
    }
    (wrapped, "markdown link".to_string())
}

/// Rewrite each `$$<ID>` trigger to `§<ID>` — but only where `$$` is immediately
/// followed by a real ID-shaped token, and never inside a string literal in source
/// code or Markdown link destinations (§FS-fmt.2.1, §FS-fmt.2.3.1,
/// §DF-reference-marker).
fn replace_trigger(
    line: &str,
    docstring: DocstringContent<'_>,
    config: &Config,
    is_md: bool,
) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find(&config.trigger) {
        let start = cursor + relative;
        let after = start + config.trigger.len();
        if id_token_end_at(line, after, &config.grammar).is_some()
            && (is_md || !string_literal_in(docstring, line, start))
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
fn add_markers(
    line: &str,
    docstring: DocstringContent<'_>,
    config: &Config,
    is_md: bool,
) -> String {
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
        if !is_md && string_literal_in(docstring, line, found.start()) {
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
