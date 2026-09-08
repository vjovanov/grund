/// Scan-root regressions for directory links that cross the canonical project
/// boundary (§FS-config.3.5.1, §AR-scanner.1). Descent cases remain in
/// `tests_scanner_walk.rs`; these start the walk at or below the link.
#[cfg(unix)]
#[cfg(test)]
mod tests_scanner_walk_roots {
    use super::*;
    use super::tests_support::*;

    #[test]
    fn a_scan_root_below_an_outward_directory_link_is_pruned() {
        let base = test_root("a_scan_root_below_an_outward_directory_link_is_pruned");
        let root = base.join("project");
        write(
            &base.join("external/nested/FS-001-foreign.md"),
            "# FS-001-foreign: Foreign\n",
        );
        symlink("../external", &root.join("linked"));

        let mut configured = legacy_fs_folder_config(root.clone());
        configured.include = Some(vec!["linked/nested".into()]);
        let (configured_findings, _) =
            scan_tree(&configured, None, false).expect("configured root");
        let explicit = legacy_fs_folder_config(root);
        let (explicit_findings, _) =
            scan_tree(&explicit, Some(&explicit.root.join("linked/nested")), true)
                .expect("explicit root");

        assert!(
            scanned(&configured, &configured_findings).is_empty(),
            "§FS-config.3.5.1: a configured descendant cannot bypass its outward directory link"
        );
        assert!(
            scanned(&explicit, &explicit_findings).is_empty(),
            "§FS-config.3.5.1: an explicit descendant cannot bypass its outward directory link"
        );
    }

    #[test]
    fn an_external_explicit_directory_link_root_is_pruned_before_resolution() {
        let base = test_root("an_external_explicit_directory_link_root_is_pruned_before_resolution");
        let root = base.join("project");
        write(
            &base.join("external/FS-001-foreign.md"),
            "# FS-001-foreign: Foreign\n",
        );
        symlink("external", &base.join("external-link"));
        std::fs::create_dir_all(&root).expect("create project root");
        let config = legacy_fs_folder_config(root);

        let (findings, _) = scan_tree(&config, Some(&base.join("external-link")), true)
            .expect("explicit linked root");

        assert!(
            scanned(&config, &findings).is_empty(),
            "§FS-config.3.5.1: root selection keeps enough lexical identity to prune an explicit outward directory link"
        );
    }

    #[test]
    fn the_root_fence_preserves_intentional_external_and_in_root_scope() {
        let base = test_root("the_root_fence_preserves_intentional_external_and_in_root_scope");
        let root = base.join("project");
        write(
            &base.join("external/FS-001-external.md"),
            "# FS-001-external: External\n",
        );
        write(
            &root.join("real/FS-002-local.md"),
            "# FS-002-local: Local\n",
        );
        write(
            &base.join("FS-003-file.md"),
            "# FS-003-file: External file\n",
        );
        symlink("real", &root.join("local-link"));
        symlink("../FS-003-file.md", &root.join("linked-file.md"));
        let config = legacy_fs_folder_config(root.clone());

        let (external, _) =
            scan_tree(&config, Some(&base.join("external")), true).expect("plain external root");
        let (in_root_link, _) = scan_tree(&config, Some(&root.join("local-link")), true)
            .expect("in-root linked root");
        let (file_link, _) = scan_tree(&config, Some(&root.join("linked-file.md")), true)
            .expect("outward file link");

        assert_eq!(
            scanned(&config, &external),
            vec![canonical_test_path(&base.join("external/FS-001-external.md"))
                .display()
                .to_string()],
            "§FS-config.3.5.1: a plain non-link external root remains intentional scope"
        );
        assert_eq!(
            scanned(&config, &in_root_link),
            vec!["local-link/FS-002-local.md"],
            "§FS-config.3.5.1: a directory link whose target stays in the project remains readable"
        );
        assert_eq!(
            scanned(&config, &file_link),
            vec!["linked-file.md"],
            "§FS-config.3.5.1: the directory boundary leaves external file links readable"
        );
    }
}
