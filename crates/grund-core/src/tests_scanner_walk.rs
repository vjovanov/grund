/// Test module: the tree walk's symlink policy (§FS-config.3.5, §AR-scanner.1,
/// §DF-symlink-scan) — what the walk follows, which spelling of an aliased file
/// it reports, and the boundaries a link may not carry it across. What it does
/// with a link it *cannot* read is `tests_scanner_walk_errors.rs`. Unix-only:
/// the cases are about symlinks, and creating one on Windows needs developer
/// mode.
#[cfg(unix)]
#[cfg(test)]
mod tests_scanner_walk {
    use super::*;
    use super::tests_support::*;

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

    /// §DF-symlink-scan §3: the identity pass no longer waits for `--full`, so a
    /// plain run stops reporting the duplicate two spellings of one root produced.
    #[test]
    fn a_plain_run_collapses_an_aliased_root() {
        let root = test_root("a_plain_run_collapses_an_aliased_root");
        write(
            &root.join("grund.toml"),
            // §FS-config.3.4: `index = false` — this case is about one physical
            // file read once under two root spellings, not about the index a
            // folder kind keeps.
            "grund_config_version = 1\n\n\
             [[kinds]]\nprefix = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\", \"docs-link\"]\n",
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

    /// §FS-workspace.6: a leaf member has no members of its own, so the downward
    /// boundary list is empty and a link inside it walked straight into a
    /// sibling's tree — re-declaring that project's IDs in this one's namespace.
    #[test]
    fn a_link_into_a_sibling_project_stays_out_of_this_scan() {
        let root = test_root("a_link_into_a_sibling_project_stays_out_of_this_scan");
        write(
            &root.join("packages/other/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        write(
            &root.join("packages/sub/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        symlink("../../sub", &root.join("packages/other/docs/blink"));

        let (config, findings) = member_scan(&root, "packages/other");

        assert_eq!(
            scanned(&config, &findings),
            vec!["docs/functional-spec/FS-001-alpha.md"],
            "§FS-workspace.6: the sibling owns its files, so this walk stops at its root"
        );
    }

    /// The same boundary one entry lower down: a link onto another project's
    /// *file* crosses exactly as a link onto its directory does.
    #[test]
    fn a_link_onto_another_projects_file_is_not_read() {
        let root = test_root("a_link_onto_another_projects_file_is_not_read");
        write(
            &root.join("packages/other/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        write(
            &root.join("packages/sub/docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n",
        );
        symlink(
            "../../../sub/docs/functional-spec/FS-002-beta.md",
            &root.join("packages/other/docs/functional-spec/FS-002-beta.md"),
        );

        let (config, findings) = member_scan(&root, "packages/other");

        assert_eq!(
            scanned(&config, &findings),
            vec!["docs/functional-spec/FS-001-alpha.md"],
            "§FS-workspace.6: a file another project owns is not this project's to declare"
        );
    }

    /// A directory no project owns is not a boundary — the walk was handed it by
    /// a link the repository wrote, and that is §FS-config.3.5.1's whole point.
    #[test]
    fn a_link_to_content_no_project_owns_is_still_followed() {
        let root = test_root("a_link_to_content_no_project_owns_is_still_followed");
        write(
            &root.join("packages/other/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        write(&root.join("packages/sub/docs/functional-spec/FS-002-beta.md"), "# FS-002-beta: Beta\n");
        write(&root.join("shared/notes.md"), "Cites §FS-001-alpha.\n");
        symlink("../../../shared", &root.join("packages/other/docs/shared"));

        let (config, findings) = member_scan(&root, "packages/other");

        assert_eq!(
            scanned(&config, &findings),
            vec![
                "docs/functional-spec/FS-001-alpha.md",
                "docs/shared/notes.md",
            ],
            "§FS-config.3.5.1: outside content is read at the path the link gives it"
        );
    }

    /// A member's scan, with the run's other project roots on the config — what
    /// workspace expansion stamps onto every project it loaded (§AR-workspace.6).
    fn member_scan(root: &Path, member: &str) -> (Config, Findings) {
        let mut config = Config::default_for(root.join(member));
        config.include = Some(vec!["docs".into()]);
        config.workspace_project_roots = vec![
            canonical_test_path(&root.join("packages/other")),
            canonical_test_path(&root.join("packages/sub")),
        ];
        let scope = root.join(member);
        let (findings, _) = scan_tree(&config, Some(&scope), true).expect("scan member");
        (config, findings)
    }
}
