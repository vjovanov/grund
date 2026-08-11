/// Test module: the public embedding API and init guidance (§AR-bindings.2)
#[cfg(test)]
mod tests_api {
    use super::*;
    use super::tests_support::*;

    #[test]
    fn public_embedding_api_checks_and_shows_without_cli_dispatch() {
        let root = test_root("public_embedding_api_checks_and_shows_without_cli_dispatch");
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nLead.\n",
        );
        write(&root.join("src/lib.rs"), "//! §FS-001-alpha\n");

        let report = check(&root).expect("public check api");
        assert!(report.errors.is_empty(), "expected no errors");

        let catalog = list(ListOpts {
            path: root.clone(),
            path_provided: true,
            ..ListOpts::default()
        })
        .expect("public list api");
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].id, "FS-001-alpha");
        assert_eq!(catalog.entries[0].refs, 1);

        let refs_output = refs(RefsOpts {
            path: root.clone(),
            path_provided: true,
            id: "FS-001-alpha".to_string(),
            ..RefsOpts::default()
        })
        .expect("public refs api");
        assert_eq!(refs_output.hits.len(), 1);
        assert_eq!(refs_output.hits[0].path, "src/lib.rs");

        let cover_output = cover(CoverOpts {
            path: root.clone(),
            path_provided: true,
        })
        .expect("public cover api");
        assert!(
            cover_output
                .entries
                .iter()
                .any(|entry| entry.path == "src/lib.rs" && entry.citations.len() == 1),
            "cover should expose source citation entries"
        );

        write(&root.join("docs/note.md"), "See $$FS-001-alpha.\n");
        let fmt_output = format_references(FmtOpts {
            path: root.join("docs/note.md"),
            path_provided: true,
            ..FmtOpts::default()
        })
        .expect("public fmt api");
        assert_eq!(fmt_output.changes.len(), 1);
        assert_eq!(fmt_output.changes[0].label, "trigger → marker");

        let proposed = propose_id(
            "FS",
            "Beta",
            IdOpts {
                path: root.clone(),
                path_provided: true,
                ..IdOpts::default()
            },
        )
        .expect("public id api");
        assert_eq!(
            proposed,
            IdProposalOutcome::Proposed(IdProposal {
                id: "FS-002-beta".to_string(),
                kind: "FS".to_string(),
                number: Some(2),
                slug: "beta".to_string(),
                folder: Some("docs/functional-spec".to_string()),
                file: None,
                e2e_case_dir: None,
            })
        );

        validate_config(&root).expect("public config validate api");
        let config = effective_config(&root).expect("public config api");
        assert_eq!(config.id_format, "{kind}-{number}-{slug}");

        let shown = show(
            "FS-001-alpha",
            ShowOpts {
                path: root.clone(),
                ..ShowOpts::default()
            },
        )
        .expect("public show api");
        assert_eq!(shown.body, "Lead.\n");
        assert_eq!(shown.line, 1);

        let shown_literal = show(
            "FS-001-alpha",
            ShowOpts {
                path: root.clone(),
                section: None,
                mode: ShowMode::Lead,
                format: ShowFormat::Text,
            },
        )
        .expect("public show api keeps literal construction source-compatible");
        assert_eq!(shown_literal.body, "Lead.\n");

        let shown_json = show(
            "FS-001-alpha",
            ShowOpts {
                path: root,
                format: ShowFormat::Json,
                ..ShowOpts::default()
            },
        )
        .expect("public show json api");
        let expected_json = format!(
            "{{\"id\":\"FS-001-alpha\",\"section\":null,\"body\":\"Lead.\\n\",\"path\":\"{}\",\"line\":1}}",
            "docs/functional-spec/FS-001-alpha.md"
        );
        assert_eq!(
            shown_json.json.as_deref(),
            Some(expected_json.as_str())
        );

        let lsp_root = test_root("public_embedding_api_lsp_snapshot");
        write(
            &lsp_root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[scan]\nexclude = [\"ignored\"]\n",
        );
        write(&lsp_root.join(".gitignore"), "gitignored/\n");
        write(
            &lsp_root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nLead.\n\n## 1. Detail\nMore.\n",
        );
        write(&lsp_root.join("src/lib.rs"), "//! §FS-001-alpha.1\n");
        write(&lsp_root.join("ignored/open.rs"), "//! §FS-001-alpha\n");
        write(&lsp_root.join(".hidden/open.rs"), "//! §FS-001-alpha\n");
        write(&lsp_root.join("gitignored/open.rs"), "//! §FS-001-alpha\n");
        let snapshot = lsp_snapshot(LspSnapshotOpts {
            path: lsp_root.clone(),
            path_provided: true,
            open_documents: BTreeMap::new(),
        })
        .expect("public lsp snapshot api");
        let citation = snapshot
            .citations
            .iter()
            .find(|citation| citation.display_path == "src/lib.rs")
            .expect("source citation");
        assert_eq!(citation.query_id, "FS-001-alpha.1");
        let expected_target =
            canonical_test_path(&lsp_root.join("docs/functional-spec/FS-001-alpha.md"));
        assert_eq!(
            citation.target_path.as_deref().map(canonical_test_path),
            Some(expected_target)
        );
        assert_eq!(citation.target_line, Some(5));

        let mut open_documents = BTreeMap::new();
        open_documents.insert(
            lsp_root.join("src/lib.rs"),
            "//! §FS-999-missing\n".to_string(),
        );
        let snapshot = lsp_snapshot(LspSnapshotOpts {
            path: lsp_root.clone(),
            path_provided: true,
            open_documents,
        })
        .expect("lsp snapshot uses open buffer overlay");
        assert!(
            snapshot
                .report
                .errors
                .iter()
                .any(|error| error.code == "dangling"
                    && error.message == "unknown reference FS-999-missing"),
            "unsaved overlay citation should drive diagnostics"
        );

        let mut open_documents = BTreeMap::new();
        open_documents.insert(
            lsp_root.join("ignored/open.rs"),
            "//! §FS-999-excluded\n".to_string(),
        );
        open_documents.insert(
            lsp_root.join(".hidden/open.rs"),
            "//! §FS-999-hidden\n".to_string(),
        );
        open_documents.insert(
            lsp_root.join("gitignored/open.rs"),
            "//! §FS-999-gitignored\n".to_string(),
        );
        open_documents.insert(
            lsp_root.join("gitignored/new.rs"),
            "//! §FS-999-unsaved-gitignored\n".to_string(),
        );
        let snapshot = lsp_snapshot(LspSnapshotOpts {
            path: lsp_root.clone(),
            path_provided: true,
            open_documents,
        })
        .expect("lsp snapshot respects scanner filters for overlays");
        assert!(
            snapshot
                .report
                .errors
                .iter()
                .all(|error| !matches!(error.code, "dangling")),
            "ignored overlay files must not create diagnostics"
        );

        let mut open_documents = BTreeMap::new();
        open_documents.insert(
            lsp_root.join("docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\nUnsaved beta.\n".to_string(),
        );
        let snapshot = lsp_snapshot(LspSnapshotOpts {
            path: lsp_root.clone(),
            path_provided: true,
            open_documents,
        })
        .expect("lsp snapshot reads declaration columns from overlays");
        let declaration = snapshot
            .declarations
            .iter()
            .find(|decl| decl.query_id == "FS-002-beta")
            .expect("unsaved declaration");
        assert_eq!(declaration.column, 3);

        let mut open_documents = BTreeMap::new();
        open_documents.insert(
            lsp_root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nUnsaved lead.\n\n## 1. Detail\nUnsaved detail.\n"
                .to_string(),
        );
        let shown_overlay = show_with_overlays(
            "FS-001-alpha",
            ShowOpts {
                path: lsp_root.clone(),
                mode: ShowMode::Toc,
                ..ShowOpts::default()
            },
            open_documents,
        )
        .expect("show uses open buffer overlay");
        assert!(shown_overlay.body.contains("Unsaved lead."));

        write(&lsp_root.join("docs/note.md"), "Inline `$$FS-001-alpha`.\n");
        assert!(
            !can_replace_trigger_at(
                &lsp_root.join("docs/note.md"),
                "Inline `$$FS-001-alpha`.",
                8,
                "FS-001-alpha"
            )
            .expect("markdown inline code exclusion")
        );
        assert!(
            !can_replace_trigger_at(
                &lsp_root.join("src/lib.rs"),
                "let s = \"$$FS-001-alpha\";",
                9,
                "FS-001-alpha"
            )
            .expect("source string exclusion")
        );
        assert!(
            can_replace_trigger_at(
                &lsp_root.join("src/lib.rs"),
                "//! $$FS-001-alpha",
                4,
                "FS-001-alpha"
            )
            .expect("source comment trigger")
        );

        let init_root = test_root("public_embedding_api_init_dry_run");
        let init_output = init(InitOpts {
            target: init_root,
            dry_run: true,
            ..InitOpts::default()
        })
        .expect("public init api");
        assert!(
            init_output
                .events
                .iter()
                .any(|event| event.verb == "would-write" && event.path == "AGENTS.md")
        );
        assert!(init_output.next.is_some());
    }

    #[test]
    fn list_summary_reports_single_file_kind_home() {
        let root = test_root("list_summary_reports_single_file_kind_home");
        write(
            &root.join("requirements.md"),
            "# Requirements\n\n## FS-001-alpha: Alpha\n\nLead.\n",
        );

        let catalog = list(ListOpts {
            path: root,
            path_provided: true,
            ..ListOpts::default()
        })
        .expect("public list api");

        let fs_summary = catalog
            .summaries
            .iter()
            .find(|summary| summary.kind == "FS")
            .expect("FS summary");
        assert_eq!(fs_summary.home, "requirements.md");
        assert_eq!(fs_summary.count, 1);
    }

    #[test]
    fn init_next_guidance_uses_effective_legacy_fs_home() {
        let root = test_root("init_next_guidance_uses_effective_legacy_fs_home");
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");

        let init_output = init(InitOpts {
            target: root,
            docs: true,
            dry_run: true,
            ..InitOpts::default()
        })
        .expect("init dry run");
        let next = init_output.next.expect("next guidance");
        assert_eq!(
            next.fs_home,
            InitFsHome::Folder {
                path: "docs/functional-spec".to_string()
            }
        );

        let rendered = render_next_block_for_home(next.docs, Some(&next.entrypoint), &next.fs_home);
        assert!(
            rendered.contains("then add it under docs/functional-spec"),
            "next guidance should point at the effective legacy FS home: {rendered}"
        );
        assert!(
            !rendered.contains("requirements.md"),
            "next guidance must not rebuild the new default FS home for compatibility configs: {rendered}"
        );
    }

    #[test]
    fn init_next_guidance_uses_effective_custom_fs_file() {
        let root = test_root("init_next_guidance_uses_effective_custom_fs_file");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[[kinds]]
prefix = "FS"
title = "Requirements"
file = "specs/requirements.md"
"#,
        );

        let init_output = init(InitOpts {
            target: root,
            docs: true,
            dry_run: true,
            ..InitOpts::default()
        })
        .expect("init dry run");
        let next = init_output.next.expect("next guidance");
        assert_eq!(
            next.fs_home,
            InitFsHome::File {
                path: "specs/requirements.md".to_string(),
                heading_name: "H2",
                heading_marker: "##",
            }
        );

        let rendered = render_next_block_for_home(next.docs, Some(&next.entrypoint), &next.fs_home);
        assert!(
            rendered.contains("then add it to specs/requirements.md"),
            "next guidance should point at the configured FS file: {rendered}"
        );
        assert!(
            !rendered.contains("then add it to requirements.md"),
            "next guidance must not rebuild the generated default FS file for custom configs: {rendered}"
        );
    }

    #[test]
    fn e2e_readme_scaffold_uses_effective_fs_home() {
        let files = docs_scaffold(&InitFsHome::Folder {
            path: "docs/functional-spec".to_string(),
        });
        let e2e_readme = files
            .iter()
            .find(|(path, _)| path == "e2e/README.md")
            .map(|(_, contents)| contents)
            .expect("e2e README scaffold");

        assert!(
            e2e_readme.contains("`docs/functional-spec`"),
            "e2e README should name the effective FS home: {e2e_readme}"
        );
        assert!(
            !e2e_readme.contains("`requirements.md`"),
            "e2e README must not hard-code the generated default for legacy/custom homes: {e2e_readme}"
        );

        let files = docs_scaffold(&InitFsHome::File {
            path: "specs/requirements.md".to_string(),
            heading_name: "H2",
            heading_marker: "##",
        });
        let e2e_readme = files
            .iter()
            .find(|(path, _)| path == "e2e/README.md")
            .map(|(_, contents)| contents)
            .expect("e2e README scaffold");
        assert!(
            e2e_readme.contains("`specs/requirements.md`"),
            "e2e README should name the configured FS file: {e2e_readme}"
        );
    }

    #[test]
    fn public_check_api_returns_relative_slash_normalized_paths() {
        let root = test_root("public_check_api_returns_relative_slash_normalized_paths");
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha-copy.md"),
            "# FS-001-alpha: Alpha copy\n\nLead.\n",
        );

        let report = check(&root).expect("public check api");
        let duplicate = report
            .errors
            .iter()
            .find(|finding| finding.code == "duplicate")
            .expect("duplicate diagnostic");
        let path = duplicate.path.as_deref().expect("diagnostic path");
        assert!(!Path::new(path).is_absolute(), "path must be relative");
        assert!(!path.contains('\\'), "path must use slash separators");
        assert!(path.starts_with("docs/functional-spec/"));
        for site in &duplicate.sites {
            assert!(
                !Path::new(&site.path).is_absolute(),
                "site path must be relative"
            );
            assert!(
                !site.path.contains('\\'),
                "site path must use slash separators"
            );
            assert!(site.path.starts_with("docs/functional-spec/"));
        }
    }

    #[test]
    fn deprecated_main_entry_symbol_remains_available_for_0_4_consumers() {
        #[allow(deprecated)]
        let entry: fn() -> ExitCode = main_entry;
        let _ = entry;
    }
}
