/// Test module: what the tree walk does with a link it cannot read
/// (§FS-config.3.5.5, §FS-config.3.5.6, §FS-check.2) — which loops and broken
/// links are owed a report, where that report names them, and what the walk
/// does *not* read on the way. Split from `tests_scanner_walk.rs` along the
/// seam the source already draws between `scanner_walk.rs` and
/// `scanner_walk_errors.rs`. Unix-only: every case is about a symlink.
#[cfg(unix)]
#[cfg(test)]
mod tests_scanner_walk_errors {
    use super::tests_support::*;

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

    /// §FS-config.3.5.5: the walker compares a link's target against the
    /// directories it is *inside*, so a target above the walk root is not a loop
    /// to it and it descends a whole second copy of the tree before noticing. The
    /// run then reported findings out of a tree it had just called unreadable,
    /// from paths `[scan] include` does not cover.
    #[test]
    fn a_loop_above_the_walk_root_is_not_descended_into() {
        let root = linked_repo("a_loop_above_the_walk_root_is_not_descended_into");
        write(
            &root.join("other/FS-002-outside.md"),
            "# FS-002-outside: Outside\n",
        );
        symlink("..", &root.join("docs/up"));

        let run = check_run(&root, false);

        assert_eq!(
            scan_errors(&run),
            vec!["docs/up: symlink loop: the target contains the link"],
            "§FS-config.3.5.5: the link is still reported, without the descent"
        );
        assert!(
            !findings(&run)
                .iter()
                .any(|line| line.contains("FS-002-outside")),
            "§FS-config.3.5.5: nothing is read through a link the run has called unreadable"
        );
    }

    /// §FS-config.3.5.2: two links that point at each other's directories are met
    /// one spelling deep — `docs/a/link/link` — and neither of the names the user
    /// has to fix is a prefix of that path.
    #[test]
    fn a_two_hop_loop_is_reported_at_the_links_that_form_it() {
        let root = linked_repo("a_two_hop_loop_is_reported_at_the_links_that_form_it");
        write(&root.join("docs/a/keep.md"), "# FS-002-beta: Beta\n");
        write(&root.join("docs/b/keep.md"), "# FS-003-gamma: Gamma\n");
        symlink("../b", &root.join("docs/a/link"));
        symlink("../a", &root.join("docs/b/link"));

        let run = check_run(&root, false);

        assert_eq!(
            scan_errors(&run),
            vec![
                "docs/a/link: symlink loop: the target is the ancestor directory docs/b",
                "docs/b/link: symlink loop: the target is the ancestor directory docs/a",
            ],
            "§FS-config.3.5.2: the report names the links, not the spelling the descent produced"
        );
    }
}
