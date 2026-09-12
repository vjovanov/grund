/// Test module: init's effective-scope decision and shared guidance renderer
/// (§FS-init.2.2, §FS-config.3.5).
#[cfg(test)]
mod tests_init_scan_guidance {
    use super::tests_support::*;
    use super::*;

    #[test]
    fn effective_scope_recognizes_include_and_kind_home_files() {
        let include_root = test_root("effective_scope_recognizes_an_include_file");
        write(&include_root.join("notes/readable.md"), "Readable.\n");
        let mut include_config = Config::default_for(include_root);
        include_config.include = Some(vec!["notes".to_string()]);

        assert!(
            effective_scope_reads_any_file(&include_config),
            "§FS-config.3.5: a configured include file is readable"
        );

        let home_root = test_root("effective_scope_recognizes_a_kind_home_file");
        write(&home_root.join("requirements.md"), "# Requirements\n");
        let mut home_config = Config::default_for(home_root);
        home_config.include = Some(Vec::new());

        assert!(
            effective_scope_reads_any_file(&home_config),
            "§FS-config.3.5: a walked kind file home is readable even outside includes"
        );
    }

    #[test]
    fn effective_scope_rejects_files_the_scanner_skips() {
        let root = test_root("effective_scope_rejects_files_the_scanner_skips");
        write(&root.join("content/unsupported.txt"), "Not a configured extension.\n");
        write(&root.join("content/.hidden.md"), "Hidden file.\n");
        write(&root.join("content/.private/note.md"), "Hidden directory.\n");
        write(&root.join("content/excluded/note.md"), "Excluded directory.\n");
        write(&root.join("content/.gitignore"), "ignored/\n");
        write(&root.join("content/ignored/note.md"), "Ignored directory.\n");
        write(&root.join("content/templates/template.md"), "Unwalked kind home.\n");

        let mut config = Config::default_for(root);
        config.include = Some(vec!["content".to_string()]);
        config.exclude = vec!["excluded".to_string()];
        let fs = config
            .kinds
            .iter_mut()
            .find(|kind| kind.kind == "FS")
            .expect("default FS kind");
        fs.file = Some("content/templates/template.md".to_string());
        fs.scan = false;

        assert!(
            !effective_scope_reads_any_file(&config),
            "§FS-init.2.2: existence is insufficient when scanner policy skips every file"
        );
    }

    #[test]
    fn effective_scope_stops_after_the_first_readable_root() {
        let root = test_root("effective_scope_stops_after_the_first_readable_root");
        let mut config = Config::default_for(root.clone());
        config.include = Some(vec!["first".to_string(), "second".to_string()]);
        let mut visited = Vec::new();

        let reads_any = effective_scope_reads_any_file_with(&config, |candidate| {
            visited.push(candidate.to_path_buf());
            candidate == root.join("first")
        });

        assert!(reads_any);
        assert_eq!(
            visited,
            vec![root.join("first")],
            "§GOAL-fast-feedback: later scan roots are not visited after the first hit"
        );
    }

    #[test]
    fn shared_renderer_changes_only_the_empty_scan_suffix() {
        let populated = next_guidance(true).render();
        let empty = next_guidance(false).render();
        let suffix = " — until then `grund check` has nothing to scan";

        assert!(!populated.contains(suffix));
        assert!(empty.contains(suffix));
        assert_eq!(
            empty.replacen(suffix, "", 1),
            populated,
            "§FS-init.2.2: both command adapters share a renderer whose only scan-state difference is the suffix"
        );
    }

    /// The two scan states rendered by §FS-init.2.2.
    fn next_guidance(scan_reads_file: bool) -> InitNext {
        InitNext {
            docs: false,
            entrypoint: "AGENTS.md".to_string(),
            fs_home: InitFsHome::File {
                path: "requirements.md".to_string(),
                heading_name: "H2",
                heading_marker: "##",
            },
            scan_reads_file,
        }
    }
}
