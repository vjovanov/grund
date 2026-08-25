/// Test module: the index a folder kind keeps (§FS-check.4.6, §FS-check.3.17,
/// §DF-index-entry-form). Every case here is about the two conditions an entry
/// has to meet — present, and a full link — plus the two carve-outs that make
/// the rule cheap: `fmt` always linkifies an index (§FS-fmt.6.1), and an entry
/// is never an inbound citation (§FS-check.4.1).
#[cfg(test)]
mod tests_kind_index {
    use super::*;
    use super::tests_support::*;

    /// A repo whose `FS` kind is a folder with the default `README.md` index.
    const FOLDER_CONFIG: &str = "grund_config_version = 1\n\n\
        [[kinds]]\nprefix = \"FS\"\nfolder = \"docs/specs\"\n\n\
        [scan]\ninclude = [\"docs\"]\n";

    fn setup(name: &str) -> PathBuf {
        let root = test_root(name);
        write(&root.join("grund.toml"), FOLDER_CONFIG);
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        root
    }

    fn codes(run: &CheckRun) -> Vec<String> {
        run.report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    }

    fn only<'a>(run: &'a CheckRun, code: &str) -> &'a Diagnostic {
        run.report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("expected a {code} finding, got {:?}", codes(run)))
    }

    /// §FS-check.4.6: the folder has an index and the index does not name the
    /// declaration. The finding is anchored at the declaration's own heading and
    /// names the index file plus the release the warning becomes an error in.
    #[test]
    fn a_declaration_the_index_does_not_name_is_a_warning_at_the_declaration() {
        let root = setup("a_declaration_the_index_does_not_name_is_a_warning_at_the_declaration");
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
            finding.message.contains("becomes an error in grund 0.12.0"),
            "§REQ-backwards-compatibility.2: the warning names the release: {}",
            finding.message
        );
        assert!(
            run.report.errors.is_empty(),
            "§DF-index-compatibility-ramp.2.1: no command writes the entry, so it warns"
        );
    }

    /// §FS-check.4.6: a folder with no index *file* is the same finding class,
    /// once per declaration — which is why it is anchored at the declaration.
    #[test]
    fn a_missing_index_file_reports_once_per_declaration() {
        let root = setup("a_missing_index_file_reports_once_per_declaration");
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
        let root = setup("a_bare_entry_is_an_error_at_its_line_in_the_index");
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
            !codes(&run).contains(&"missing-index-entry".to_string()),
            "one cause, one finding: an ID with an entry never also reports as missing"
        );
    }

    /// §FS-check.4.6 / §FS-check.3.17: the wrapped form is what satisfies the
    /// rule, and `check` never looks at where the link points (§DF-index-entry-form.2.2).
    #[test]
    fn a_linked_entry_satisfies_the_rule_whatever_the_target_says() {
        let root = setup("a_linked_entry_satisfies_the_rule_whatever_the_target_says");
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
             [[kinds]]\nprefix = \"AR\"\nfolder = \"docs/architecture\"\n\n\
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

    /// §DF-index-entry-form.2.3: a citation inside an inline-code span is neither
    /// an entry nor a finding — `fmt` never wraps one, so demanding it would leave
    /// the repository permanently red.
    #[test]
    fn an_inline_code_mention_is_not_an_entry_and_is_not_an_error() {
        let root = setup("an_inline_code_mention_is_not_an_entry_and_is_not_an_error");
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\nThe login flow is `§FS-001-login`, explained below.\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlinked-index-entry".to_string()),
            "`fmt` declines to wrap it, so `check` must not demand it: {:?}",
            findings(&run)
        );
        assert!(
            codes(&run).contains(&"missing-index-entry".to_string()),
            "the ID still has no entry, and that is the finding with a fix"
        );
    }

    /// §DF-index-entry-form.2.3: one link per ID. A prose mention beside a real
    /// linked entry is untouched — the AR index in this repository is the case.
    #[test]
    fn one_link_satisfies_an_id_that_is_also_mentioned_bare() {
        let root = setup("one_link_satisfies_an_id_that_is_also_mentioned_bare");
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
        let root = setup("the_walk_reaches_a_declaration_in_a_subdirectory");
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

    /// §FS-config.3.4: `index = false` opts the kind out, and the rule then has
    /// nothing to say about the folder at all.
    #[test]
    fn index_false_opts_the_kind_out() {
        let root = test_root("index_false_opts_the_kind_out");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nprefix = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"missing-index-entry".to_string()),
            "a kind whose declarations are exercised, not navigated, says so: {:?}",
            findings(&run)
        );
    }

    /// §FS-config.3.4: `index` names a file *inside* `folder`, so a kind with no
    /// folder has nothing to index and the key is a config error.
    #[test]
    fn index_without_a_folder_is_a_config_error() {
        let root = test_root("index_without_a_folder_is_a_config_error");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[[kinds]]\nprefix = \"FS\"\nfile = \"requirements.md\"\nindex = \"README.md\"\n",
        );

        let message = config_error(&root);
        assert!(
            message.contains("sets `index` without `folder`"),
            "the error names the key and why it cannot apply: {message}"
        );
    }

    /// §FS-config.3.4: `index = true` names no file. The default is spelled by
    /// leaving the key out, so `true` is rejected rather than read as one.
    #[test]
    fn index_true_is_a_config_error() {
        let root = test_root("index_true_is_a_config_error");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[[kinds]]\nprefix = \"FS\"\nfolder = \"docs/specs\"\nindex = true\n",
        );

        let message = config_error(&root);
        assert!(
            message.contains("takes a file name or `false`"),
            "the error says what to write instead: {message}"
        );
    }

    /// §FS-config.3.4: `index = "<name>"` names another file, resolved relative
    /// to `folder`.
    #[test]
    fn a_named_index_is_resolved_under_the_folder() {
        let root = test_root("a_named_index_is_resolved_under_the_folder");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nprefix = \"FS\"\nfolder = \"docs/specs\"\nindex = \"INDEX.md\"\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(&root.join("docs/specs/README.md"), "# Specs\n\nProse, not the index.\n");

        let run = check_run(&root, false);
        assert!(
            only(&run, "missing-index-entry")
                .message
                .contains("docs/specs/INDEX.md"),
            "the named file is the index, and the README beside it is just a file"
        );
    }

    /// §FS-check.4.1 / §DF-index-not-an-inbound-citation: the hazard. An index
    /// names every declaration in its folder by construction, so its entries must
    /// not make a declaration look used.
    #[test]
    fn an_index_entry_does_not_suppress_the_unused_warning() {
        let root = setup("an_index_entry_does_not_suppress_the_unused_warning");
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
             [[kinds]]\nprefix = \"FS\"\nfolder = \"docs/specs\"\n\n\
             [[kinds]]\nprefix = \"AR\"\nfolder = \"docs/architecture\"\nindex = false\n\n\
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

    /// The message a config this repo cannot load fails with. `Config` carries no
    /// `Debug`, so the error is unwrapped by matching rather than by `expect_err`.
    fn config_error(root: &Path) -> String {
        match load_config(root) {
            Ok(_) => panic!("expected the config to be rejected"),
            Err(error) => format!("{error:#}"),
        }
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
