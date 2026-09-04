/// The unlisted-`[workspace]` rule (§FS-check.4.9), in a file of its own beside
/// the rest of the workspace machinery (§AR-core-module-layout.1, §AR-workspace.6.1):
/// a directory this run's walk reached that declares `[workspace]` and that no
/// enclosing block lists among its `members` is claimed by nobody, so its subtree
/// is absorbed into the enclosing namespace instead of named under its own alias
/// path (§FS-workspace.6.1).
///
/// The rule sits **above** the walk, never inside it: the scanner carries out the
/// directories it reached and asks nothing of them (§AR-workspace.1), and every
/// question about claims, configs and messages is answered here.
///
/// Three filters in cost order, so a tree with no nested config pays the probe and
/// nothing else (§GOAL-fast-feedback): one `config_file_in` probe per walked
/// directory, a text-only `[workspace]`-header read for the directories that carry
/// a config, and the ancestor claim climb only for the ones that declare the table.
/// `load_config_at` is deliberately not used for a candidate — a config that will
/// not parse must not fail the run, and a full load rebuilds the grammar regex set
/// per candidate.

/// The release §FS-check.4.9's warning becomes an error in
/// (§REQ-backwards-compatibility.2, §DF-unlisted-workspace-block.2.1,
/// §RM-unlisted-workspace-error). Named in the
/// message text, because a warning that does not say when it bites tells a
/// maintainer they have a problem and not that they have a deadline; held ahead of
/// the running version by a unit test, so the deadline fails the build rather than
/// passing unnoticed.
const UNLISTED_WORKSPACE_BLOCK_ERROR_RELEASE: &str = "0.14.0";

/// §FS-check.4.9: one warning per outermost `[workspace]` block this run's walk
/// met that no enclosing block lists.
///
/// `config` is the project whose walk produced `walked_dirs` — the namespace the
/// block is absorbed into, and the root both remedies are written from. `render`
/// is the config every path renders against (§FS-errors.3): the workspace root in
/// a workspace run, `config` itself otherwise. `alias` is the alias path this run
/// spells that project with, so the message can be matched against what
/// §FS-list printed.
///
/// Outermost-only falls out of the claim test rather than needing a pass of its
/// own: a block below an unlisted one *is* claimed — by the unlisted block — so
/// `enclosing_workspace_of` answers it, and one edit fixes the chain.
fn unlisted_workspace_block_warnings(
    config: &Config,
    render: &Config,
    alias: Option<&str>,
    walked_dirs: &[PathBuf],
) -> Vec<Diagnostic> {
    // §GOAL-fast-feedback: one cache for the whole rule, the way
    // `enclosing_alias_prefix` shares one per climb — every candidate walks the same
    // ancestors, and without it each ancestor's config is re-read per candidate.

    // Quiet (§FS-check.4.9): this climb spells no alias path, so an ancestor it
    // cannot read is this rule's silence rather than the reader's warning.
    let mut ancestors = AncestorWorkspaces::quiet_for_run_at(&config.root);
    let mut reported = BTreeSet::new();
    let mut warnings = Vec::new();
    for dir in walked_dirs {
        // The probe first: two `is_file` calls, and all a tree with no nested config
        // pays (§FS-config.1). It is what finds the `.agents/` form, which the walk
        // never meets as a file entry — it prunes hidden directories (§FS-check.4.9).
        let Some(config_path) = config_file_in(dir) else {
            continue;
        };
        let Some(line) = workspace_table_line(&config_path) else {
            continue;
        };
        // §FS-check.4.9: a project root of this run is never a candidate — without it
        // `--full`, which makes the config root a walk root (§FS-check.1.3), reports
        // every workspace repository against itself. Canonicalizing is the dear half.
        let canonical = canonical_workspace_path(dir);
        if is_project_root_of_run(config, &canonical) {
            continue;
        }
        // §FS-check.4.9 "one finding for one edit": one block the walk reached under
        // two spellings answers the claim test identically, so the first spelling met
        // is the one reported and a symlinked second is not another finding.
        if !reported.insert(canonical) {
            continue;
        }
        match enclosing_workspace_of(dir, &config.cli_base, &mut ancestors) {
            // Claimed, at any depth: inside the chain, so nothing is absorbed.
            Ok(Some(_)) => continue,
            // §FS-workspace.6.1: a claim an ancestor names but cannot answer is
            // undecidable in both directions, so the block is left unreported and
            // unexplained (§FS-check.4.9) rather than called unlisted.
            Err(_) => continue,
            Ok(None) => {}
        }
        warnings.push(Diagnostic {
            code: "unlisted-workspace-block",
            // §FS-errors.2.2, §DF-unlisted-workspace-block.2.4: the CLI-level shape —
            // `line`-less, so it prints as one `warning:` on stderr with the location
            // in the text. A fact about the run's configuration, not about a site.
            path: None,
            line: None,
            column: None,
            message: unlisted_workspace_block_message(
                config,
                render,
                alias,
                dir,
                &config_path,
                line,
            ),
            sites: Vec::new(),
        });
    }
    warnings
}

/// §FS-check.4.9: the sentence, built apart from the reporting so both channels
/// print one text — `check`'s report warning and the direct stderr line every
/// other walking surface prints (§DF-unlisted-workspace-block.2.4).
///
/// Four facts and one deadline: the block's `[workspace]` line, what the
/// absorption costs, the two config edits that clear it, and the release the
/// finding becomes an error in. The second remedy is stated as an outcome rather
/// than as a key on purpose — `[scan] exclude` prunes descendants and never the
/// directory a walk starts at (§FS-check.1.3), so a block that is itself an
/// `include` root takes `include` and one below it takes `exclude`.
fn unlisted_workspace_block_message(
    config: &Config,
    render: &Config,
    alias: Option<&str>,
    dir: &Path,
    config_path: &Path,
    line: usize,
) -> String {
    // Both remedies are written from the enclosing project's root, so the entry is
    // spelled from there — while the two file paths render against the run's report
    // base like every other path in a diagnostic (§FS-errors.3).
    let entry = format_path(&relative_from_base(&config.root, dir));
    let enclosing_config = config
        .config_file
        .clone()
        .unwrap_or_else(|| config.root.join("grund.toml"));
    format!(
        "{block}:{line}: this [workspace] is listed by no enclosing workspace \
         — the projects under it are absorbed into `{absorbing}` instead of named under \
         their own alias path; add \"{entry}\" to [workspace] members in {enclosing}, \
         or keep it out of that project's [scan] \
         — an unlisted [workspace] becomes an error in grund \
         {UNLISTED_WORKSPACE_BLOCK_ERROR_RELEASE}",
        block = display_path(render, config_path),
        absorbing = absorbing_project_name(config, alias),
        enclosing = display_path(render, &enclosing_config),
    )
}

/// The alias path this run spells the absorbing project with (§FS-workspace.3), so
/// the message can be read beside what §FS-list printed. A run that loaded no
/// workspace has no alias path to offer, and falls back to the name the project
/// would carry as a workspace root — its `project_name`, or `root`
/// (§AR-workspace.5.3) — which is what the reader would see the moment the block
/// is listed and the namespace becomes one.
fn absorbing_project_name(config: &Config, alias: Option<&str>) -> String {
    match alias {
        Some(alias) if !alias.is_empty() => alias.to_string(),
        _ => derive_alias(config, None, RootMode::Root)
            .unwrap_or_else(|_| "the enclosing project".to_string()),
    }
}

/// §FS-check.4.9: whether this canonical directory is one of the project roots the
/// run names everything else from — its own root, and each member root the walk
/// stops at (§FS-workspace.6).
fn is_project_root_of_run(config: &Config, canonical: &Path) -> bool {
    canonical == canonical_workspace_path(&config.root)
        || config
            .workspace_project_roots
            .iter()
            .chain(&config.workspace_boundary_roots)
            .any(|root| root == canonical)
}

/// The line a config's `[workspace]` table opens on, or `None` when it declares
/// none (§FS-check.4.9).
///
/// A text-only read, for the same reason `ancestor_member_entries` is one
/// (§FS-workspace.6.1): the question is asked of a config this run does not
/// otherwise load, and a candidate that will not parse must not fail the run. The
/// section header is read exactly as `parse_config_file` reads one, so the forms it
/// *rejects* still count as a declared block — a `[[workspace]]` here is a block
/// somebody meant, and calling it "no block" would silence the finding on the very
/// config that is most confused. An unreadable file declares nothing this run can
/// see and says nothing.
///
/// Reading the header exactly as the parser reads one is the deliberate half, and
/// the identity with `ancestor_member_entries` is worth more than being cleverer
/// here: a textual read also sees a header-shaped line nobody meant as a header —
/// one inside a multi-line value — in a config `parse_config_file` already
/// rejects, and two readers of one file that disagreed about what a section header
/// is would be the worse bug (§FS-workspace.6.1).
fn workspace_table_line(config_path: &Path) -> Option<usize> {
    let text = fs::read_to_string(config_path).ok()?;
    text.lines().enumerate().find_map(|(idx, raw_line)| {
        let line = strip_comment(raw_line).trim();
        (line.starts_with('[')
            && line.ends_with(']')
            && line.trim_matches(['[', ']']) == "workspace")
            .then_some(idx + 1)
    })
}
