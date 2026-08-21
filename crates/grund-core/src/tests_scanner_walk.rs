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
}
