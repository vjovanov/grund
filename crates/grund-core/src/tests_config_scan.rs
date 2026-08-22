/// Test module: config parsing, kind homes, and the parallel scan path (§FS-config, §FS-check)
#[cfg(test)]
mod tests_config_scan {
    use super::*;
    use super::tests_support::*;

    #[test]
    fn report_path_rendering_uses_forward_slashes() {
        assert_eq!(
            format_path(Path::new(r"docs\functional-spec\FS-001-alpha.md")),
            "docs/functional-spec/FS-001-alpha.md"
        );
    }

    #[test]
    fn default_config_requires_marker_prefixed_citations() {
        let config = Config::default_for(test_root("default_config_requires_marker_prefixed_citations"));

        assert!(config.strict, "§FS-config.3.1: strict reference mode is the default");
    }

    #[test]
    fn parallel_file_scan_matches_sequential_scan() {
        let root = test_root("parallel_file_scan_matches_sequential_scan");
        write(
            &root.join("docs/functional-spec/FS-001-root.md"),
            "# FS-001-root: Root\n\n## 1. Contract\nReferenced by source files.\n",
        );
        for idx in 0..300 {
            write(
                &root.join(format!("src/module-{idx:03}.rs")),
                &format!("// §FS-001-root.1\npub fn module_{idx:03}() {{}}\n"),
            );
        }

        let config = legacy_fs_folder_config(root.clone());
        let (sequential, sequential_errors) =
            scan_tree_with_workspace_threshold(
                &config,
                Some(&root),
                true,
                &[],
                usize::MAX,
                &TextOverlays::new(),
            )
                .expect("sequential scan");
        let (parallel, parallel_errors) =
            scan_tree_with_workspace_threshold(
                &config,
                Some(&root),
                true,
                &[],
                1,
                &TextOverlays::new(),
            )
                .expect("parallel scan");

        assert_eq!(
            findings_signature(&config, &parallel),
            findings_signature(&config, &sequential),
            "parallel file scanning must merge to the same Findings as the sequential path"
        );
        assert_eq!(
            scan_errors_signature(parallel_errors),
            scan_errors_signature(sequential_errors),
            "parallel file scanning must preserve scan-error ordering"
        );
    }

    #[test]
    fn parallel_workspace_scan_matches_sequential_project_scans() {
        let root = test_root("parallel_workspace_scan_matches_sequential_project_scans");
        write(
            &root.join(".agents/grund.toml"),
            r#"[workspace]
members = ["packages/*"]
"#,
        );
        write(
            &root.join("docs/functional-spec/FS-001-root.md"),
            "# FS-001-root: Root\n\nMentions §alpha/FS-001-alpha and §beta/FS-001-beta.\n",
        );
        write(
            &root.join("packages/alpha/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nMentions §root/FS-001-root.\n",
        );
        write(
            &root.join("packages/beta/docs/functional-spec/FS-001-beta.md"),
            "# FS-001-beta: Beta\n\nMentions §alpha/FS-001-alpha.\n",
        );

        let mut root_config = resolve_workspace_config(&root).expect("workspace config");
        let member_roots = expand_workspace_members(&root_config).expect("members");
        root_config.workspace_boundary_roots = member_roots.clone();
        let mut entries = vec![("root".to_string(), root_config.clone())];
        for member_root in member_roots {
            let member_config = load_config_at_with_report_base(
                &member_root,
                &root_config.cli_base,
                Some(&root_config.root),
            )
            .expect("member config");
            let alias = member_root
                .file_name()
                .and_then(|name| name.to_str())
                .expect("member alias")
                .to_string();
            entries.push((alias, member_config));
        }
        let targets = entries
            .iter()
            .map(|(alias, config)| WorkspaceCitationTarget {
                alias: alias.clone(),
                config: config.clone(),
            })
            .collect::<Vec<_>>();

        let sequential = entries
            .iter()
            .map(|(alias, config)| {
                let (findings, errors) = scan_tree_with_workspace_threshold(
                    config,
                    Some(&config.root),
                    true,
                    &targets,
                    usize::MAX,
                    &TextOverlays::new(),
                )
                .expect("sequential workspace project scan");
                (
                    alias.clone(),
                    findings_signature(config, &findings),
                    scan_errors_signature(errors),
                )
            })
            .collect::<Vec<_>>();
        let mut parallel = entries
            .into_par_iter()
            .map(|(alias, config)| {
                let (findings, errors) = scan_tree_with_workspace_threshold(
                    &config,
                    Some(&config.root),
                    true,
                    &targets,
                    1,
                    &TextOverlays::new(),
                )
                .expect("parallel workspace project scan");
                (
                    alias,
                    findings_signature(&config, &findings),
                    scan_errors_signature(errors),
                )
            })
            .collect::<Vec<_>>();
        parallel.sort_by(|a, b| a.0.cmp(&b.0));
        let mut sequential_sorted = sequential;
        sequential_sorted.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(
            parallel, sequential_sorted,
            "parallel workspace project scanning must preserve each project's scanner output"
        );
    }

    #[test]
    fn explicit_file_scope_ignores_unrelated_findings() {
        let root = test_root("explicit_file_scope_ignores_unrelated_findings");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        write(
            &root.join("docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\nMentions FS-999-missing.\n",
        );

        let config = legacy_fs_folder_config(root.clone());
        let (findings, _) = scan_tree(
            &config,
            Some(&root.join("docs/functional-spec/FS-001-alpha.md")),
            true,
        )
        .expect("scan scoped file");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.is_empty(),
            "unrelated dangling citation should not be reported"
        );
    }

    #[test]
    fn check_rejects_declaration_in_wrong_unique_kind_home() {
        let root = test_root("check_rejects_declaration_in_wrong_unique_kind_home");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# AR-001-router: Router\n\nSupports §FS-002-login.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-002-login.md"),
            "# FS-002-login: Login\n\nUses §AR-001-router.\n",
        );

        let config = legacy_fs_folder_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let misplaced = report
            .errors
            .iter()
            .filter(|error| error.code == "misplaced-declaration")
            .collect::<Vec<_>>();

        assert_eq!(misplaced.len(), 1);
        assert_eq!(
            misplaced[0].message,
            "AR-001-router declares kind AR inside FS home docs/functional-spec"
        );
    }

    #[cfg(unix)]
    #[test]
    fn check_uses_scanned_symlink_path_for_kind_home() {
        let root = test_root("check_uses_scanned_symlink_path_for_kind_home");
        write(
            &root.join("outside/spec.md"),
            "# AR-001-router: Router\n\nSupports §FS-002-login.\n",
        );
        std::fs::create_dir_all(root.join("docs/functional-spec")).expect("create docs");
        std::os::unix::fs::symlink(
            "../../outside/spec.md",
            root.join("docs/functional-spec/link.md"),
        )
        .expect("create symlink");

        let config = legacy_fs_folder_config(root.clone());
        let id = Id {
            kind: "AR".to_string(),
            num: Some(1),
            slug: Some("router".to_string()),
        };
        let mut findings = Findings::default();
        findings.declarations.insert(
            id.clone(),
            vec![Declaration {
                id,
                file: root.join("docs/functional-spec/link.md"),
                line: 1,
                heading_level: 1,
                sections: BTreeMap::new(),
                duplicate_sections: Vec::new(),
                is_stub: false,
                defined_in: None,
                e2e_case: None,
                title: Some("Router".to_string()),
                body_start: 1,
                body_end: 1,
            }],
        );
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|error| {
                error.code == "misplaced-declaration"
                    && error.message
                        == "AR-001-router declares kind AR inside FS home docs/functional-spec"
            }),
            "home-kind placement must use the scanned symlink path: {:?}",
            report
                .errors
                .iter()
                .map(|error| (&error.code, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_skips_home_kind_rule_for_overlapping_kind_homes() {
        let root = test_root("check_skips_home_kind_rule_for_overlapping_kind_homes");
        write(
            &root.join("docs/architecture/AR-001-router.md"),
            "# FS-001-login: Login\n\nImplemented here.\n",
        );

        let mut config = Config::default_for(root.clone());
        for kind in &mut config.kinds {
            match kind.prefix.as_str() {
                "FS" => {
                    kind.folder = Some("docs".to_string());
                    kind.file = None;
                }
                "AR" => kind.folder = Some("docs/architecture".to_string()),
                _ => {}
            }
        }
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report
                .errors
                .iter()
                .any(|error| error.code == "misplaced-declaration"),
            "overlapping homes do not provide a unique expected kind"
        );
    }

    #[test]
    fn scanner_ignores_bare_source_citations_inside_strings() {
        let root = test_root("scanner_ignores_bare_source_citations_inside_strings");
        write(
            &root.join("src/app.rs"),
            "fn main() {\n    let value = \"FS-999-missing\";\n}\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) =
            scan_tree(&config, Some(&root.join("src/app.rs")), true).expect("scan source file");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.is_empty(),
            "string literal must not be a citation"
        );
    }

    /// §AR-scanner.2.3: a marked *qualified* citation is suppressed inside an
    /// inline-code span or a string literal **only** in source files; in Markdown
    /// it is always a citation. This pins the whole (file-type × context) matrix
    /// end-to-end so no single detection pass can drift from the shared
    /// `qualified_suppressed_in_source` rule again — the exact divergence PR #44
    /// fixed (one pass gated differently from the others).
    #[test]
    fn qualified_citation_suppression_is_uniform_across_passes() {
        let root = test_root("qualified_citation_suppression_is_uniform_across_passes");
        // Markdown: plain prose and backticked inline code both resolve.
        write(&root.join("docs/prose.md"), "See §api/FS-001-login here.\n");
        write(&root.join("docs/code.md"), "See `§api/FS-001-login` here.\n");
        // Source: a plain comment resolves; inline code and a string suppress.
        write(&root.join("src/plain.rs"), "// see §api/FS-001-login\n");
        write(&root.join("src/inline.rs"), "/// see `§api/FS-001-login`\n");
        write(
            &root.join("src/string.rs"),
            "fn f() { let _ = \"§api/FS-001-login\"; }\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        let mut detected: Vec<String> = findings
            .citations
            .iter()
            .filter(|c| c.namespace.as_deref() == Some("api"))
            .filter_map(|c| c.file.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        detected.sort();
        detected.dedup();
        assert_eq!(
            detected,
            vec![
                "code.md".to_string(),
                "plain.rs".to_string(),
                "prose.md".to_string(),
            ],
            "markdown (prose + inline code) and a plain source comment detect; \
             source inline-code and string-literal contexts stay suppressed"
        );
    }

    /// §FS-check.2.3.1: a `<§>`-escaped illustration is inert to every check, but
    /// one whose ID resolves is surfaced as a suggestion (not an error). Also
    /// guards the per-file findings merge (§AR-scanner) — both escapes must
    /// survive `merge_findings`, which once dropped `escaped_citations`.
    #[test]
    fn escaped_citation_that_resolves_is_suggested_not_errored() {
        let root = test_root("escaped_citation_that_resolves_is_suggested_not_errored");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        // A live citation keeps FS-001-login from being "unused"; the file also
        // holds a resolving escape and a dangling escape.
        write(
            &root.join("docs/guide.md"),
            "Live §FS-001-login. Escaped `<§>FS-001-login`. Ghost `<§>FS-999-ghost`.\n",
        );

        let config = legacy_fs_folder_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        // The escape is inert: exactly one *live* citation, not three.
        assert_eq!(
            findings.citations.len(),
            1,
            "a `<§>`-escaped token is never counted as a live citation"
        );
        // Both escapes are recorded and survive the per-file merge.
        assert_eq!(
            findings.escaped_citations.len(),
            2,
            "both escapes recorded across the findings merge"
        );

        let report = check_findings(&findings, &config);
        assert!(
            !report
                .errors
                .iter()
                .any(|d| matches!(d.code, "dangling" | "unknown-project")),
            "an escaped citation never raises a dangling or unknown-project error"
        );
        let escaped: Vec<_> = report
            .suggestions
            .iter()
            .filter(|d| d.code == "escaped-citation-resolves")
            .collect();
        assert_eq!(
            escaped.len(),
            1,
            "only the escape whose ID resolves is suggested; the ghost stays quiet"
        );
        assert!(escaped[0].message.contains("FS-001-login"));
    }

    /// §FS-check.3.1: the near-ID nudge and the `<§>`-escape hint are independent
    /// — the near ID appears whenever one is close, the escape only inside inline
    /// code, and both together when a dangling citation in backticks also has a
    /// near match. This is the matrix behind the "context-aware, one line" rule.
    #[test]
    fn dangling_hint_combines_near_id_and_inline_code_escape() {
        let root = test_root("dangling_hint_combines_near_id_and_inline_code_escape");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        let config = legacy_fs_folder_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        let near = Id {
            kind: "FS".to_string(),
            num: Some(1),
            slug: Some("logon".to_string()),
        };
        let far = Id {
            kind: "FS".to_string(),
            num: Some(999),
            slug: Some("zzz".to_string()),
        };
        let msg = |id, inline| dangling_message(&config, None, &findings, id, inline);

        // near ID, plain prose: only the "did you mean?" nudge.
        assert_eq!(
            msg(&near, false),
            "unknown reference FS-001-logon; did you mean FS-001-login?"
        );
        // near ID, inline code: both hints.
        assert_eq!(
            msg(&near, true),
            "unknown reference FS-001-logon; did you mean FS-001-login? (or write <§>FS-001-logon if this is an illustration)"
        );
        // no near ID, plain prose: neither hint.
        assert_eq!(msg(&far, false), "unknown reference FS-999-zzz");
        // no near ID, inline code: only the escape hint.
        assert_eq!(
            msg(&far, true),
            "unknown reference FS-999-zzz; write <§>FS-999-zzz if this is an illustration"
        );
    }
}
