/// Test module: the `[[kinds]] index` key itself (§FS-config.3.4) — which file a
/// folder kind's index is, and every shape of the value the config parser
/// refuses. The rules built on the key, and the two findings they produce, are
/// in `tests_kind_index.rs`; this file is about the key alone, so a reader
/// chasing a rejected `grund.toml` is not paging through §FS-check.4.6's cases.
#[cfg(test)]
mod tests_kind_index_config {
    use super::*;
    use super::tests_support::*;

    /// §FS-config.3.4: `index = false` opts the kind out, and the rule then has
    /// nothing to say about the folder at all.
    #[test]
    fn index_false_opts_the_kind_out() {
        let root = test_root("index_false_opts_the_kind_out");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
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
            "grund_config_version = 1\n\n[[kinds]]\nkind = \"FS\"\nfile = \"requirements.md\"\nindex = \"README.md\"\n",
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
            "grund_config_version = 1\n\n[[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = true\n",
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
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = \"INDEX.md\"\n\n\
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

    /// §FS-config.3.4: an index the cross-reference pass can never run on
    /// (§FS-fmt.6.1) would carry an error class whose one documented fix declines
    /// to act, so the name is refused at config time instead.
    #[test]
    fn index_naming_a_non_markdown_file_is_a_config_error() {
        let root = test_root("index_naming_a_non_markdown_file_is_a_config_error");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = \"INDEX.py\"\n",
        );

        let message = config_error(&root);
        assert!(
            message.contains("must name a Markdown file"),
            "the error says which property the name failed: {message}"
        );
    }

    /// §FS-config.3.4: `index` is joined onto `folder`, so an absolute path or one
    /// that climbs out with `..` replaces the folder rather than naming a file in
    /// it — and `check` would then read outside the tree the config describes.
    #[test]
    fn an_index_outside_its_folder_is_a_config_error() {
        for (case, value) in [
            ("absolute", "/tmp/elsewhere.md"),
            ("parent", "../../elsewhere.md"),
            ("dot", "./README.md"),
        ] {
            let root = test_root(&format!("an_index_outside_its_folder_is_a_config_error_{case}"));
            write(
                &root.join("grund.toml"),
                &format!(
                    "grund_config_version = 1\n\n\
                     [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = \"{value}\"\n"
                ),
            );

            let message = config_error(&root);
            assert!(
                message.contains("must be a relative path inside `folder`"),
                "{case}: {message}"
            );
        }
    }

    /// §FS-config.3.4: the `index` default is keyed on the prefix, and a declared
    /// `[[kinds]]` block that omits the key gets the same answer the built-in list
    /// does. Without this every config `grund init` wrote before the key existed
    /// would take one §FS-check.4.6 warning per e2e case on upgrade.
    #[test]
    fn a_declared_e2e_kind_takes_the_index_false_default() {
        let root = test_root("a_declared_e2e_kind_takes_the_index_false_default");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\n\n\
             [[kinds]]\nkind = \"E2E\"\nfolder = \"e2e/cases\"\n\n\
             [scan]\ninclude = [\"docs\", \"e2e\"]\n",
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(
            &root.join("docs/specs/README.md"),
            "# Specs\n\n- [§FS-001-login](FS-001-login.md#fs-001-login-a-user-logs-in)\n",
        );
        write(&root.join("e2e/cases/login/spec.refs"), "FS-001-login\n");
        write(&root.join("e2e/cases/login/expected.exit"), "0\n");

        let config = load_config(&root).expect("config");
        let e2e = config
            .kinds
            .iter()
            .find(|kind| kind.kind == "E2E")
            .expect("the declared E2E kind");
        assert_eq!(
            e2e.index_toml_value().as_deref(),
            Some("false"),
            "a declared kind and a built-in one agree about the default"
        );

        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"missing-index-entry".to_string()),
            "the folder is exercised, not navigated: {:?}",
            findings(&run)
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
}
