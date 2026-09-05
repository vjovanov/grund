/// Test module: the index a folder kind keeps (§FS-check.3.18, §FS-check.3.17,
/// §DF-index-entry-form). Every case here is about the two conditions an entry
/// has to meet — present, and a full link — plus the two carve-outs that make
/// the rule cheap: `fmt` always linkifies an index (§FS-fmt.6.1), and an entry
/// is never an inbound citation (§FS-check.4.1).
#[cfg(test)]
mod tests_kind_index {
    use super::*;
    use super::tests_support::*;

    /// §FS-check.3.18: the folder has an index and the index does not name the
    /// declaration. The finding is an **error**, anchored at the declaration's own
    /// heading and naming the index file — and naming nothing else, because the
    /// ramp §REQ-backwards-compatibility.2 opened ended in this release and a
    /// deadline that has arrived is not news the message can still carry.
    #[test]
    fn a_declaration_the_index_does_not_name_is_an_error_at_the_declaration() {
        let root = kind_index_repo("a_declaration_the_index_does_not_name_is_an_error_at_the_declaration");
        write(&root.join("docs/specs/README.md"), "# Specs\n\nNothing here yet.\n");

        let run = check_run(&root, false);
        let finding = only(&run, "missing-index-entry");
        assert_eq!(
            finding.path.as_deref().map(|path| display_path(&run.config, path)),
            Some("docs/specs/FS-001-login.md".to_string()),
            "§DF-index-entry-form.2.6: the declaration's heading, not the index"
        );
        assert_eq!(finding.line, Some(1));
        assert!(
            finding.message.contains("docs/specs/README.md"),
            "the message names the index that owes the entry: {}",
            finding.message
        );
        assert!(
            !finding.message.contains("becomes an error in grund"),
            "§FS-check.3.18: the ramp ended, so the message names no release: {}",
            finding.message
        );
        assert!(
            run.report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == "missing-index-entry"),
            "§FS-check.3.18: an error, so it reaches the exit code: {:?}",
            findings(&run)
        );
        assert!(
            run.report
                .warnings
                .iter()
                .all(|diagnostic| diagnostic.code != "missing-index-entry"),
            "§FS-check.3.18: and it is not also on the warning path: {:?}",
            findings(&run)
        );
    }

    /// §FS-check.3.18: a folder with no index *file* is the same finding class,
    /// once per declaration — which is why it is anchored at the declaration.
    #[test]
    fn a_missing_index_file_reports_once_per_declaration() {
        let root = kind_index_repo("a_missing_index_file_reports_once_per_declaration");
        write(
            &root.join("docs/specs/FS-002-logout.md"),
            "# FS-002-logout: A user logs out\n\nBody.\n",
        );

        let run = check_run(&root, false);
        assert_eq!(
            codes(&run)
                .iter()
                .filter(|code| *code == "missing-index-entry")
                .count(),
            2,
            "one per declaration in the folder: {:?}",
            findings(&run)
        );
        assert!(
            only(&run, "missing-index-entry")
                .message
                .contains("the index file does not exist"),
            "the run says which of the two states the folder is in"
        );
    }

    /// §FS-check.3.17: the entry is there and is not a link. Reported at the
    /// citation's own line in the index, naming the command that fixes it.
    #[test]
    fn a_bare_entry_is_an_error_at_its_line_in_the_index() {
        let root = kind_index_repo("a_bare_entry_is_an_error_at_its_line_in_the_index");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- §FS-001-login\n",
        );

        let run = check_run(&root, false);
        let finding = only(&run, "unlinked-index-entry");
        assert_eq!(
            finding.path.as_deref().map(|path| display_path(&run.config, path)),
            Some("docs/specs/README.md".to_string())
        );
        assert_eq!(finding.line, Some(3), "the line `grund fmt --write` rewrites");
        assert!(
            finding.message.contains("grund fmt --write"),
            "§REQ-backwards-compatibility.3: an error on arrival names its one command: {}",
            finding.message
        );
        assert!(
            finding.message.contains(&format!("unchecked in grund {INDEX_RULE_PRIOR_RELEASE}"))
                && finding.message.contains(&format!("an error in {INDEX_RULE_RELEASE}")),
            "§REQ-backwards-compatibility.3 asks for the versions the verdict moved between: {}",
            finding.message
        );
        assert!(
            !codes(&run).contains(&"missing-index-entry".to_string()),
            "one cause, one finding: an ID with an entry never also reports as missing"
        );
    }

    /// §FS-check.3.18 / §FS-check.3.17: the wrapped form is what satisfies the
    /// rule, and `check` never looks at where the link points (§DF-index-entry-form.2.2).
    #[test]
    fn a_linked_entry_satisfies_the_rule_whatever_the_target_says() {
        let root = kind_index_repo("a_linked_entry_satisfies_the_rule_whatever_the_target_says");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [§FS-001-login](FS-001-login.md#a-stale-anchor)\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string())
                && !codes(&run).contains(&"missing-index-entry".to_string()),
            "the shape is the requirement; the target is `fmt`'s to re-derive: {:?}",
            findings(&run)
        );
    }

    /// §DF-index-entry-form.2.2: a declaration whose home is a source file links
    /// to the bare path with no anchor — "full link" is the link `fmt` writes
    /// here, not "carries an anchor".
    #[test]
    fn an_anchorless_link_to_a_source_home_is_a_full_link() {
        let root = test_root("an_anchorless_link_to_a_source_home_is_a_full_link");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\n\n\
             [scan]\ninclude = [\"docs\", \"src\"]\n",
        );
        write(
            &root.join("src/bus.rs"),
            "/// AR-001-bus: The in-process event bus\n///\n/// Body.\nfn bus() {}\n",
        );
        write(
            &root.join("docs/architecture/AR-001-bus.md"),
            "# AR-001-bus: [src/bus.rs](../../src/bus.rs)\n",
        );
        write(
            &root.join("docs/architecture/README.md"),
            "# Architecture\n\n- [§AR-001-bus](../../src/bus.rs)\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string())
                && !codes(&run).contains(&"missing-index-entry".to_string()),
            "§FS-list.2: the stub-and-inline pair collapses to one entry: {:?}",
            findings(&run)
        );
    }

    /// §DF-index-entry-form.2.3: one link per ID. A prose mention beside a real
    /// linked entry is untouched — the AR index in this repository is the case.
    #[test]
    fn one_link_satisfies_an_id_that_is_also_mentioned_bare() {
        let root = kind_index_repo("one_link_satisfies_an_id_that_is_also_mentioned_bare");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [§FS-001-login](FS-001-login.md#fs-login-a-user-logs-in)\n\nSee also `§FS-001-login`.\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string())
                && !codes(&run).contains(&"missing-index-entry".to_string()),
            "every other occurrence of the ID is untouched: {:?}",
            findings(&run)
        );
    }

    /// §DF-index-entry-form.2.5: the walk is the folder's whole subtree, because
    /// a kind's folder routinely holds a directory per topic.
    #[test]
    fn the_walk_reaches_a_declaration_in_a_subdirectory() {
        let root = kind_index_repo("the_walk_reaches_a_declaration_in_a_subdirectory");
        write(
            &root.join("docs/specs/proposals/FS-003-draft.md"),
            "# FS-003-draft: A drafted spec\n\nBody.\n",
        );
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [§FS-001-login](FS-001-login.md#fs-login-a-user-logs-in)\n",
        );

        let run = check_run(&root, false);
        let finding = only(&run, "missing-index-entry");
        assert!(
            finding.message.contains("FS-003-draft"),
            "a nested declaration is still the folder's: {}",
            finding.message
        );
    }

    /// §FS-check.4.1 / §DF-index-not-an-inbound-citation: the hazard. An index
    /// names every declaration in its folder by construction, so its entries must
    /// not make a declaration look used.
    #[test]
    fn an_index_entry_does_not_suppress_the_unused_warning() {
        let root = kind_index_repo("an_index_entry_does_not_suppress_the_unused_warning");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [§FS-001-login](FS-001-login.md#fs-login-a-user-logs-in)\n",
        );

        let run = check_run(&root, false);
        assert!(
            codes(&run).contains(&"unused".to_string()),
            "the index entry is navigation, not use: {:?}",
            findings(&run)
        );
    }

    /// §DF-index-not-an-inbound-citation.2.2: the exclusion is the *entry*. A
    /// citation in an index file of an ID whose home is outside that folder is an
    /// ordinary reference and counts like any other.
    #[test]
    fn an_index_citation_of_a_foreign_id_is_an_ordinary_citation() {
        let root = test_root("an_index_citation_of_a_foreign_id_is_an_ordinary_citation");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\n\n\
             [[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(
            &root.join("docs/architecture/AR-001-bus.md"),
            "# AR-001-bus: The bus\n\nBody.\n",
        );
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [§FS-001-login](FS-001-login.md#fs-login-a-user-logs-in) — built on [§AR-001-bus](../architecture/AR-001-bus.md#ar-bus-the-bus)\n",
        );

        let unused: Vec<String> = run_unused_ids(&root);
        assert!(
            unused.contains(&"FS-001-login".to_string()),
            "its only citation is its own index entry: {unused:?}"
        );
        assert!(
            !unused.contains(&"AR-001-bus".to_string()),
            "the AR citation is a reference the author wrote, not an index entry: {unused:?}"
        );
    }

    /// Ordering only, so the `-dev` suffix is dropped rather than modelled: a
    /// test that needs to tell `0.12.4-dev` from `0.12.4` reads the suffix
    /// itself, because both land here as the same triple.
    fn version(text: &str) -> (u64, u64, u64) {
        let mut parts = text.split('.').map(|part| {
            part.split(|ch: char| !ch.is_ascii_digit())
                .next()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("not a version: {text}"))
        });
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    }

    /// §DF-index-compatibility-ramp.2.3: the releases §FS-check.3.17's message
    /// names are literals in message text, and the version they are measured
    /// against is bumped at release time rather than when the work lands
    /// (§FS-distribution.4). This is the guard on the pair that is left. Both
    /// halves of it are claims about releases that happened — the verdict a
    /// §REQ-backwards-compatibility.3 migration *moved between* — so a run that
    /// printed either as a date still ahead would be promising rather than
    /// reporting. §FS-check.3.18's own deadline literal is gone with its ramp.
    #[test]
    fn index_rule_releases_are_ordered_and_behind_us() {
        let current = version(env!("CARGO_PKG_VERSION"));
        let prior = version(INDEX_RULE_PRIOR_RELEASE);
        let arrival = version(INDEX_RULE_RELEASE);
        assert!(prior < arrival, "{INDEX_RULE_PRIOR_RELEASE} < {INDEX_RULE_RELEASE}");
        assert!(
            prior <= current,
            "§FS-check.3.17 says the rule was unchecked in {INDEX_RULE_PRIOR_RELEASE}, which has to be a release that happened (this tree is {})",
            env!("CARGO_PKG_VERSION")
        );
        assert!(
            arrival <= current,
            "§FS-check.3.17 says the rule is an error as of {INDEX_RULE_RELEASE}, which has to be a release that happened rather than one still ahead (this tree is {})",
            env!("CARGO_PKG_VERSION")
        );
    }

    /// §REQ-backwards-compatibility.2, §FS-check.3.18: the deprecation path lets
    /// the old verdict die "no earlier than `N+1`", and every run of `0.12.x`
    /// said which release that was. Arriving early breaks that promise exactly as
    /// letting the date slip breaks it — a `0.12.x` patch carrying this error
    /// would fail a repository one minor before the tool told it to expect it.
    /// The patch helper derives its version from the last tag rather than from
    /// anything a human chose (§FS-distribution.4), so a test on the bumped tree
    /// is where the refusal has to live.
    ///
    /// A `-dev` tree is exempt because it is never published (§FS-distribution.4.1),
    /// and the exemption reads the suffix rather than the numbers: `0.12.4-dev`
    /// and the `0.12.4` this must refuse are the same triple.
    #[test]
    fn the_index_entry_error_is_not_published_before_the_release_it_named() {
        let current = env!("CARGO_PKG_VERSION");
        if current.ends_with("-dev") {
            return;
        }
        assert!(
            version(current) >= version(INDEX_ENTRY_ERROR_RELEASE),
            "this tree is {current}, a release before the {INDEX_ENTRY_ERROR_RELEASE} \
             §FS-check.3.18's warning named as the one it becomes an error in. Publishing \
             the error here breaks §REQ-backwards-compatibility.2 from the early side. Cut \
             {INDEX_ENTRY_ERROR_RELEASE} rather than a patch off the release that announced it."
        );
    }

    /// §FS-check.3.18: the three ways an index file fails to read are named apart.
    /// "does not exist", said about a directory that plainly does, is a diagnosis
    /// the reader has to argue with before acting on it.
    #[test]
    fn an_index_path_that_is_a_directory_says_so() {
        let root = kind_index_repo("an_index_path_that_is_a_directory_says_so");
        std::fs::create_dir_all(root.join("docs/specs/README.md")).expect("create dir");

        let run = check_run(&root, false);
        assert!(
            only(&run, "missing-index-entry")
                .message
                .contains("the index file is a directory"),
            "{:?}",
            findings(&run)
        );
    }

    /// §FS-check.3.18: the entries come from the scan and the form from disk, so a
    /// run that never scanned the index cannot say what it lists. Reporting every
    /// declaration as unlisted there would be a finding about the scope.
    #[test]
    fn a_run_that_did_not_scan_the_index_does_not_judge_it() {
        let root = kind_index_repo("a_run_that_did_not_scan_the_index_does_not_judge_it");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [§FS-001-login](FS-001-login.md#fs-001-login-a-user-logs-in)\n",
        );

        let run = run_check(&root.join("docs/specs/FS-001-login.md"), true, false, false)
            .expect("narrowed check run");
        let codes: Vec<&str> = run
            .report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert!(
            !codes.contains(&"missing-index-entry"),
            "the index is out of this run's scope, and it does list the ID: {codes:?}"
        );
    }

    /// §FS-fmt.6.1 / §DF-index-always-linkified.2.2: under `enabled = false` the
    /// carve-out reaches the index for the sake of its entries and writes nothing
    /// else — the smallest write that clears §FS-check.3.17.
    #[test]
    fn the_always_linkify_carve_out_wraps_entries_and_not_prose() {
        let root = test_root("the_always_linkify_carve_out_wraps_entries_and_not_prose");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\n\n\
             [[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n\n\
             [fmt.cross_refs]\nenabled = false\n",
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(
            &root.join("docs/architecture/AR-001-bus.md"),
            "# AR-001-bus: The bus\n\nBody.\n",
        );
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- §FS-001-login\n\nBuilt on §AR-001-bus.\n",
        );

        format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            ..FmtOpts::default()
        })
        .expect("fmt");

        let index = std::fs::read_to_string(root.join("docs/specs/README.md")).expect("read");
        assert!(
            index.contains("- [§FS-001-login](FS-001-login.md#fs-001-login-a-user-logs-in)"),
            "the entry is wrapped whatever the toggle says: {index}"
        );
        assert!(
            index.contains("Built on §AR-001-bus."),
            "a citation of a foreign ID is prose in an ordinary file, and stays bare: {index}"
        );
        assert!(
            check_run(&root, false)
                .report
                .errors
                .iter()
                .all(|diagnostic| diagnostic.code != "unlinked-index-entry"),
            "and the minimum write is enough to clear the error"
        );
    }

    fn run_unused_ids(root: &Path) -> Vec<String> {
        let run = check_run(root, false);
        run.report
            .warnings
            .iter()
            .filter(|diagnostic| diagnostic.code == "unused")
            .map(|diagnostic| {
                diagnostic
                    .message
                    .rsplit_once(": ")
                    .map(|(_, id)| id.to_string())
                    .unwrap_or_default()
            })
            .collect()
    }
}
