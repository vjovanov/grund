/// Test module: the tree walk's symlink policy (§FS-config.3.5, §AR-scanner.1,
/// §DF-symlink-scan) — what the walk follows, which spelling of an aliased file
/// it reports, and what a link it cannot resolve does to the run. Unix-only: the
/// cases are about symlinks, and creating one on Windows needs developer mode.
#[cfg(unix)]
#[cfg(test)]
mod tests_scanner_walk {
    use super::*;
    use super::tests_support::*;

    fn symlink(target: &str, link: &Path) {
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::os::unix::fs::symlink(target, link).expect("create symlink");
    }

    /// A repo scoped to `docs`, with one declaration inside it. Every case here
    /// adds the link it is about.
    fn linked_repo(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        root
    }

    /// The graph findings — everything the report prints on stdout as
    /// `path:line: message` (§FS-check.2.1).
    fn findings(run: &CheckRun) -> Vec<String> {
        let mut diagnostics = run
            .report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .filter(|diagnostic| diagnostic.code != "io")
            .collect::<Vec<_>>();
        // The order the report prints in (§FS-errors.4), so a case can read as
        // the lines a user would see.
        diagnostics.sort_by(|a, b| diagnostic_cmp(a, b));
        located_diagnostics(&run.config, diagnostics)
    }

    /// The files the walk handed to the scanner, in report spelling — what a case
    /// about *where the walk went* asserts on, independent of which of them
    /// happened to declare anything.
    fn scanned(config: &Config, findings: &Findings) -> Vec<String> {
        let mut files: Vec<String> = findings
            .scanned_files
            .iter()
            .map(|file| display_path(config, file))
            .collect();
        files.sort();
        files
    }

    /// The `error: <path>: <reason>` lines a file the scan could not read earns
    /// (§FS-check.2, §FS-errors.2.2).
    fn scan_errors(run: &CheckRun) -> Vec<String> {
        run.report
            .errors
            .iter()
            .filter(|diagnostic| diagnostic.code == "io")
            .map(|diagnostic| {
                format!(
                    "{}: {}",
                    diagnostic
                        .path
                        .as_ref()
                        .map(|path| display_path(&run.config, path))
                        .unwrap_or_default(),
                    diagnostic.message
                )
            })
            .collect()
    }

    #[test]
    fn a_symlinked_file_is_read_at_the_path_the_link_occupies() {
        let root = linked_repo("a_symlinked_file_is_read_at_the_path_the_link_occupies");
        write(
            &root.join("outside/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\nCites §FS-001-alpha and §FS-999-ghost.\n",
        );
        symlink(
            "../../outside/FS-002-beta.md",
            &root.join("docs/functional-spec/FS-002-beta.md"),
        );

        let run = check_run(&root, false);

        assert_eq!(
            findings(&run),
            vec![
                "docs/functional-spec/FS-002-beta.md:1: declared but never cited: FS-002-beta",
                "docs/functional-spec/FS-002-beta.md:3: unknown reference FS-999-ghost",
            ],
            "§FS-config.3.5: the linked file's citations are checked, and every finding names the link, not the target"
        );
        assert!(
            !findings(&run)
                .iter()
                .any(|line| line.contains("never cited: FS-001-alpha")),
            "§FS-config.3.5: the citation in the linked file counts, so what it cites is not reported unused"
        );
    }

    #[test]
    fn an_aliased_file_is_read_once_under_the_first_of_its_names() {
        let root = linked_repo("an_aliased_file_is_read_once_under_the_first_of_its_names");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nCites §FS-999-ghost.\n",
        );
        symlink(
            "FS-001-alpha.md",
            &root.join("docs/functional-spec/FS-000-alias.md"),
        );

        let run = check_run(&root, false);

        assert_eq!(
            findings(&run),
            vec![
                "docs/functional-spec/FS-000-alias.md:1: declared but never cited: FS-001-alpha",
                "docs/functional-spec/FS-000-alias.md:3: unknown reference FS-999-ghost",
            ],
            "§FS-config.3.5: one physical file, one read — under the lexicographically first of its two names, never a duplicate of itself"
        );
    }

    #[test]
    fn an_alias_that_sorts_last_leaves_the_real_name_reported() {
        let root = linked_repo("an_alias_that_sorts_last_leaves_the_real_name_reported");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nCites §FS-999-ghost.\n",
        );
        symlink(
            "FS-001-alpha.md",
            &root.join("docs/functional-spec/FS-002-alias.md"),
        );

        let run = check_run(&root, false);

        assert_eq!(
            findings(&run),
            vec![
                "docs/functional-spec/FS-001-alpha.md:1: declared but never cited: FS-001-alpha",
                "docs/functional-spec/FS-001-alpha.md:3: unknown reference FS-999-ghost",
            ],
            "§FS-errors.4: the surviving spelling is the lexicographically first path, not whichever one readdir happened to yield first"
        );
    }

    #[test]
    fn full_scope_keeps_the_plain_runs_spelling_of_a_linked_file() {
        let root = linked_repo("full_scope_keeps_the_plain_runs_spelling_of_a_linked_file");
        // Outside `include`, so the plain run can only reach it through the link
        // and the `--full` walk reaches it both ways.
        write(
            &root.join("outside/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\nCites §FS-999-ghost.\n",
        );
        symlink(
            "../../outside/FS-002-beta.md",
            &root.join("docs/functional-spec/FS-002-beta.md"),
        );

        let scoped = check_run(&root, false);
        let full = check_run(&root, true);

        assert_eq!(
            located_diagnostics(
                &full.config,
                full.report
                    .errors
                    .iter()
                    .filter(|diagnostic| !diagnostic.code.starts_with("out-of-scope-")),
            ),
            located_diagnostics(&scoped.config, scoped.report.errors.iter()),
            "§FS-check.1.3: --full stays purely additive — same in-scope lines, same spelling of the file two roots reach"
        );
        assert_eq!(
            located_diagnostics(&scoped.config, scoped.report.errors.iter()),
            vec!["docs/functional-spec/FS-002-beta.md:3: unknown reference FS-999-ghost"]
        );
    }

    #[test]
    fn a_broken_link_is_reported_only_where_the_walk_would_have_read_it() {
        let root = linked_repo("a_broken_link_is_reported_only_where_the_walk_would_have_read_it");
        symlink("gone.md", &root.join("docs/functional-spec/FS-002-gone.md"));
        symlink("gone.png", &root.join("docs/logo.png"));

        let run = check_run(&root, false);

        assert_eq!(
            scan_errors(&run),
            vec![
                "docs/functional-spec/FS-002-gone.md: broken symlink: the target does not exist"
            ],
            "§FS-config.3.5: a name `[scan] extensions` covers is a hole worth reporting; `logo.png` was never going to be read"
        );
        assert!(
            run.had_scan_errors,
            "§FS-check.2: a file the scan could not read makes the run exit 2"
        );
    }

    #[test]
    fn a_broken_link_an_ignore_file_covers_is_not_reported() {
        let root = linked_repo("a_broken_link_an_ignore_file_covers_is_not_reported");
        // `.ignore` rather than `.gitignore`: the `ignore` crate honours the
        // former with no repository around the fixture (§AR-scanner.1.1).
        write(&root.join(".ignore"), "generated.md\n");
        symlink("gone.md", &root.join("docs/generated.md"));
        symlink("gone.md", &root.join("docs/functional-spec/FS-002-gone.md"));

        let run = check_run(&root, false);

        assert_eq!(
            scan_errors(&run),
            vec![
                "docs/functional-spec/FS-002-gone.md: broken symlink: the target does not exist"
            ],
            "§FS-config.3.5: the walk was never going to read an ignored path, so its broken link is not a hole it has to report"
        );
    }

    #[test]
    fn a_symlink_loop_is_reported_and_the_walk_carries_on() {
        let root = linked_repo("a_symlink_loop_is_reported_and_the_walk_carries_on");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nCites §FS-999-ghost.\n",
        );
        symlink(".", &root.join("docs/self"));

        let run = check_run(&root, false);

        assert_eq!(
            scan_errors(&run),
            vec!["docs/self: symlink loop: the target is the ancestor directory docs"],
            "§FS-config.3.5: a loop is reported at the link's own path, once"
        );
        assert!(
            findings(&run)
                .iter()
                .any(|line| line.ends_with("unknown reference FS-999-ghost")),
            "§FS-check.2: the walk continues past the loop, so the findings it had already collected are still printed"
        );
    }

    /// §AR-workspace.6: the boundary compare is a path suffix, which a member
    /// reached under a *link* name never matches — so the root scan walked into
    /// the member namespace and reported its declarations as duplicates.
    #[test]
    fn a_member_reached_through_a_link_stays_out_of_the_root_scan() {
        let root = test_root("a_member_reached_through_a_link_stays_out_of_the_root_scan");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        write(
            &root.join("packages/sub/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        symlink("../packages/sub", &root.join("docs/member-link"));

        let mut config = Config::default_for(root.clone());
        config.include = Some(vec!["docs".into()]);
        config.workspace_boundary_roots = vec![canonical_test_path(&root.join("packages/sub"))];
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        assert_eq!(
            scanned(&config, &findings),
            vec!["docs/functional-spec/FS-001-alpha.md"],
            "§AR-workspace.6: a member is out of bounds under every name it wears, the link's included"
        );
    }

    /// The same escape one level up: the link names a directory that *contains*
    /// the member, so no prefix of the entry's in-tree path is the member either.
    #[test]
    fn a_link_above_a_member_does_not_carry_the_root_scan_into_it() {
        let root = test_root("a_link_above_a_member_does_not_carry_the_root_scan_into_it");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        write(
            &root.join("packages/sub/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        symlink("../packages", &root.join("docs/packages-link"));

        let mut config = Config::default_for(root.clone());
        config.include = Some(vec!["docs".into()]);
        config.workspace_boundary_roots = vec![canonical_test_path(&root.join("packages/sub"))];
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        assert_eq!(
            scanned(&config, &findings),
            vec!["docs/functional-spec/FS-001-alpha.md"],
            "§AR-workspace.6: the boundary is the directory, so it holds however many links are above it"
        );
    }

    /// §AR-scanner.6: the case-directory test compares an entry's parent with the
    /// cases root exactly, which a link onto that root does not match — and the
    /// fixture repos the manifest pass owns were scanned as repo content.
    #[test]
    fn an_e2e_case_directory_reached_through_a_link_is_not_scanned() {
        let root = linked_repo("an_e2e_case_directory_reached_through_a_link_is_not_scanned");
        write(&root.join("e2e/cases/001-case/expected.exit"), "0\n");
        write(
            &root.join("e2e/cases/001-case/repo/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        symlink("../e2e/cases", &root.join("docs/cases-link"));

        let run = check_run(&root, false);

        assert!(
            !findings(&run)
                .iter()
                .any(|line| line.contains("duplicate declaration")),
            "§AR-scanner.6: a case directory is a manifest boundary under every name it is reached by"
        );
    }

    /// §DF-symlink-scan.2.4: the loop branch asked only the hidden-name and
    /// `[scan] exclude` tests, so an ignored looping link still turned the run red.
    #[test]
    fn a_looping_link_an_ignore_file_covers_is_not_reported() {
        let root = linked_repo("a_looping_link_an_ignore_file_covers_is_not_reported");
        write(&root.join(".ignore"), "self\n");
        symlink(".", &root.join("docs/self"));
        symlink(".", &root.join("docs/functional-spec/loop"));

        let run = check_run(&root, false);

        assert_eq!(
            scan_errors(&run),
            vec![
                "docs/functional-spec/loop: symlink loop: the target is the ancestor directory docs/functional-spec"
            ],
            "§FS-config.3.5: the walk was never going to descend an ignored path, so its loop is not a hole it has to report"
        );
    }

    /// §FS-check.1.3: `--full` walks every `include` root beside the config root
    /// that already contains it, so an error met once per root printed twice.
    #[test]
    fn a_scan_error_is_reported_once_under_full_scope() {
        let root = linked_repo("a_scan_error_is_reported_once_under_full_scope");
        symlink("gone.md", &root.join("docs/functional-spec/FS-002-gone.md"));

        let full = check_run(&root, true);

        assert_eq!(
            scan_errors(&full),
            vec![
                "docs/functional-spec/FS-002-gone.md: broken symlink: the target does not exist"
            ],
            "§FS-check.1.3: overlapping roots meet one broken link once each, and the report names it once"
        );
    }

    /// §DF-symlink-scan §3: the identity pass no longer waits for `--full`, so a
    /// plain run stops reporting the duplicate two spellings of one root produced.
    #[test]
    fn a_plain_run_collapses_an_aliased_root() {
        let root = test_root("a_plain_run_collapses_an_aliased_root");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\", \"docs-link\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        symlink("docs", &root.join("docs-link"));

        let run = check_run(&root, false);

        assert_eq!(
            findings(&run),
            vec!["docs/functional-spec/FS-001-alpha.md:1: declared but never cited: FS-001-alpha"],
            "§FS-check.1.3: one physical file read once, under the earlier root's spelling — no duplicate of itself"
        );
    }

    /// §FS-config.3.5: a link whose target is above the walk root is not an
    /// ancestor when the walker meets it, so the loop surfaces a full copy of the
    /// tree later — at `docs/up/docs/up`, which is not the link to fix.
    #[test]
    fn a_loop_that_escapes_the_walk_root_is_reported_at_the_link() {
        let root = linked_repo("a_loop_that_escapes_the_walk_root_is_reported_at_the_link");
        symlink("..", &root.join("docs/up"));

        let run = check_run(&root, false);

        assert_eq!(
            scan_errors(&run),
            vec!["docs/up: symlink loop: the target contains the link"],
            "§FS-config.3.5: the finding names the link, and says what is wrong with it rather than naming the reader's own directory"
        );
    }

    #[test]
    fn an_excluded_directory_reached_through_a_link_is_still_excluded() {
        let root = linked_repo("an_excluded_directory_reached_through_a_link_is_still_excluded");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nCites §FS-003-vendor.\n",
        );
        write(
            &root.join("vendored/FS-003-vendor.md"),
            "# FS-003-vendor: Vendored\n",
        );
        symlink("../vendored", &root.join("docs/node_modules"));

        let run = check_run(&root, false);

        assert!(
            findings(&run)
                .iter()
                .any(|line| line.ends_with("unknown reference FS-003-vendor")),
            "§FS-config.3.5: the directory rules apply to a followed directory under its link name, so `node_modules` is skipped either way"
        );
        assert!(
            scan_errors(&run).is_empty(),
            "an excluded directory is not a hole the walk has to report"
        );
    }

    /// §FS-config.3.5: a repository whose own path is reached through a link is
    /// walked and reported under the path the run was handed. Resolving the scope
    /// is how the walk recognizes that it *is* the config root; it is not a
    /// decision about what the report calls it. macOS is where CI meets this —
    /// `$TMPDIR` lives under `/var/folders`, a link to `/private/var/folders` —
    /// and the fixture builds the same shape on any platform.
    #[test]
    fn a_config_root_reached_through_a_link_is_reported_under_that_name() {
        let real = linked_repo("a_config_root_reached_through_a_link_is_reported_under_that_name-target");
        let link = test_root("a_config_root_reached_through_a_link_is_reported_under_that_name")
            .join("repo");
        symlink(real.to_str().expect("a utf-8 test root"), &link);

        let mut config = Config::default_for(link.clone());
        config.include = Some(vec!["docs".into()]);
        let (findings, _) = scan_tree(&config, Some(&link), true).expect("scan root");

        assert_eq!(
            scanned(&config, &findings),
            vec!["docs/functional-spec/FS-001-alpha.md"],
            "§FS-config.3.5: the walked spelling is what the report names, not the physical one"
        );
    }
}
