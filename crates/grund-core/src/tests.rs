#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        );
        let dir = std::env::temp_dir().join("grund-lib-tests").join(unique);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test root");
        dir
    }

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, text).expect("write fixture");
    }

    fn legacy_fs_folder_config(root: PathBuf) -> Config {
        let mut config = Config::default_for(root);
        for kind in &mut config.kinds {
            if kind.prefix == "FS" {
                kind.folder = Some("docs/functional-spec".to_string());
                kind.file = None;
            }
        }
        config
    }

    fn canonical_test_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn findings_signature(config: &Config, findings: &Findings) -> Vec<String> {
        let mut rows = Vec::new();
        for (id, declarations) in &findings.declarations {
            for declaration in declarations {
                rows.push(format!(
                    "decl|{}|{}|{}|{}|{}|{}|{}",
                    render_id(config, id),
                    sort_path_key(&declaration.file),
                    declaration.line,
                    declaration.heading_level,
                    declaration.is_stub,
                    declaration.title.as_deref().unwrap_or(""),
                    declaration
                        .defined_in
                        .as_ref()
                        .map(|path| format_path(path))
                        .unwrap_or_default()
                ));
                for (section, info) in &declaration.sections {
                    rows.push(format!(
                        "section|{}|{}|{}|{}|{}",
                        render_id(config, id),
                        section,
                        info.title,
                        info.line,
                        info.heading_level
                    ));
                }
                if let Some(case) = &declaration.e2e_case {
                    rows.push(format!(
                        "e2e|{}|{}|{}|{}|{}",
                        render_id(config, id),
                        sort_path_key(&case.dir),
                        case.expected_exit,
                        case.args.join(" "),
                        case.fixtures
                            .iter()
                            .map(|path| format_path(path))
                            .collect::<Vec<_>>()
                            .join(",")
                    ));
                }
            }
        }
        for citation in &findings.citations {
            rows.push(format!(
                "cite|{}|{}|{}|{}|{}|{}|{}|{}",
                citation.namespace.as_deref().unwrap_or(""),
                render_id(config, &citation.id),
                citation.section.as_deref().unwrap_or(""),
                sort_path_key(&citation.file),
                citation.line,
                citation.column,
                citation.has_marker,
                citation.text
            ));
        }
        for file in &findings.scanned_files {
            rows.push(format!("file|{}", sort_path_key(file)));
        }
        rows
    }

    fn scan_errors_signature(errors: Vec<ScanError>) -> Vec<String> {
        errors
            .into_iter()
            .map(|(path, message)| format!("{}|{}", sort_path_key(&path), message))
            .collect()
    }

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

    fn current_block() -> String {
        render_agents_append_block(
            "demo",
            &Config::default_for(PathBuf::from(".")),
            Path::new("."),
            true,
        )
    }

    fn current_marker() -> &'static str {
        "## Grounding with grund (v2)"
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
            scan_tree_with_workspace_threshold(&config, Some(&root), true, &[], usize::MAX)
                .expect("sequential scan");
        let (parallel, parallel_errors) =
            scan_tree_with_workspace_threshold(&config, Some(&root), true, &[], 1)
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

    #[test]
    fn require_grounding_off_by_default() {
        let root = test_root("require_grounding_off_by_default");
        write(&root.join("src/util.rs"), "pub fn helper() {}\n");

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report.errors.iter().any(|e| e.code == "ungrounded"),
            "grounding is opt-in: an uncited source file is not an error by default"
        );
    }

    #[test]
    fn require_grounding_flags_uncited_source_file() {
        let root = test_root("require_grounding_flags_uncited_source_file");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.rs"),
            "// §FS-001-login\npub fn login() {}\n",
        );
        write(&root.join("src/util.rs"), "pub fn helper() {}\n");

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        let ungrounded: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.code == "ungrounded")
            .map(|e| canonical_test_path(e.path.as_deref().unwrap()))
            .collect();
        assert_eq!(
            ungrounded,
            vec![canonical_test_path(&root.join("src/util.rs"))],
            "only the uncited source file is flagged; the one citing §FS-001-login is grounded"
        );
    }

    #[test]
    fn inline_citation_only_rejects_prose_once_per_site() {
        let root = test_root("inline_citation_only_rejects_prose_once_per_site");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.rs"),
            "// §FS-001-login login branch rationale\n// §FS-001-login\npub fn login() {}\n",
        );

        let mut config = Config::default_for(root.clone());
        config.inline_style = "citation-only".into();
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let style_errors = report
            .errors
            .iter()
            .filter(|error| error.code == "inline-citation-style")
            .collect::<Vec<_>>();

        assert_eq!(style_errors.len(), 1, "one finding per offending site");
        assert_eq!(style_errors[0].line, Some(1));
        assert_eq!(
            style_errors[0].message,
            "inline citation must carry no prose"
        );
    }

    #[test]
    fn inline_note_hard_caps_can_report_multiple_errors() {
        let root = test_root("inline_note_hard_caps_can_report_multiple_errors");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.rs"),
            "/// §FS-001-login rationale with a long line that exceeds the cap\n/// second line\npub fn login() {}\n",
        );

        let mut config = Config::default_for(root.clone());
        config.inline_note_max_lines = 1;
        config.inline_note_max_columns = 40;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let messages = report
            .errors
            .iter()
            .filter(|error| error.code == "inline-citation-style")
            .map(|error| error.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages.contains(&"inline note exceeds 1-line maximum"),
            "line cap should be reported: {messages:?}"
        );
        assert!(
            messages.contains(&"inline note exceeds 40-column maximum"),
            "column cap should be reported: {messages:?}"
        );
    }

    #[test]
    fn inline_note_soft_cap_is_warning_only_when_enabled() {
        let root = test_root("inline_note_soft_cap_is_warning_only_when_enabled");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.rs"),
            "// §FS-001-login rationale\n// continuation\npub fn login() {}\n",
        );

        let mut config = Config::default_for(root.clone());
        config.warn_on_suggested = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.code == "inline-citation-style"
                    && warning.message == "inline note exceeds 1-line preferred limit"),
            "soft-cap overrun should be a warning when enabled"
        );
        assert!(
            !report
                .errors
                .iter()
                .any(|error| error.code == "inline-citation-style"),
            "soft-cap overrun within the hard cap must not be an error"
        );
    }

    #[test]
    fn inline_declaration_blocks_are_not_citation_style_sites() {
        let root = test_root("inline_declaration_blocks_are_not_citation_style_sites");
        write(
            &root.join("docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n",
        );
        write(
            &root.join("src/auth.rs"),
            "/// FS-001-login: Login\n/// §FS-002-beta body citation with prose\npub fn login() {}\n",
        );

        let mut config = Config::default_for(root.clone());
        config.inline_style = "citation-only".into();
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report
                .errors
                .iter()
                .any(|error| error.code == "inline-citation-style"),
            "inline spec declaration bodies are not style-checked"
        );
    }

    #[test]
    fn inline_style_respects_disabled_python_docstring_scanning() {
        let root = test_root("inline_style_respects_disabled_python_docstring_scanning");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.py"),
            "\"\"\"\n§FS-001-login\nsecond line\nthird line\n\"\"\"\n",
        );

        let mut config = Config::default_for(root.clone());
        config.docstring_python = false;
        config.inline_note_max_lines = 1;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report
                .errors
                .iter()
                .any(|error| error.code == "inline-citation-style"),
            "triple-quoted strings are not inline citation sites when docstring scanning is disabled"
        );
    }

    #[test]
    fn inline_style_strips_configured_comment_prefixes_for_note_detection() {
        let root = test_root("inline_style_strips_configured_comment_prefixes_for_note_detection");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(&root.join("src/auth.rs"), "% §FS-001-login\n");

        let mut config = Config::default_for(root.clone());
        config.inline_style = "citation-only".into();
        config.comment_prefixes = vec!["%".into()];
        config.rebuild_grammar().expect("rebuild grammar");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report
                .errors
                .iter()
                .any(|error| error.code == "inline-citation-style"),
            "a pure citation with a configured custom comment prefix must not count as prose"
        );
    }

    #[test]
    fn inline_style_strips_block_comment_continuation_prefix() {
        let root = test_root("inline_style_strips_block_comment_continuation_prefix");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.rs"),
            "/**\n * §FS-001-login\n */\npub fn login() {}\n",
        );

        let mut config = Config::default_for(root.clone());
        config.inline_style = "citation-only".into();
        config.comment_prefixes = vec!["/*".into()];
        config.rebuild_grammar().expect("rebuild grammar");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report
                .errors
                .iter()
                .any(|error| error.code == "inline-citation-style"),
            "block-comment `*` continuation markers must be stripped when `/*` is configured"
        );
    }

    #[test]
    fn inline_note_config_rejects_soft_cap_above_hard_cap() {
        let root = test_root("inline_note_config_rejects_soft_cap_above_hard_cap");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n\n[reference]\ninline_note_suggested_lines = 4\ninline_note_max_lines = 3\n",
        );

        let err = match load_config(&root) {
            Ok(_) => panic!("invalid inline-note caps should fail"),
            Err(err) => err,
        };
        assert!(
            err.to_string()
                .contains("reference.inline_note_suggested_lines must be <= inline_note_max_lines"),
            "unexpected error: {err:#}"
        );
    }

    /// §FS-config.3: `project_description` parses as optional one-line
    /// top-level metadata next to `project_name`.
    #[test]
    fn config_parses_project_description() {
        let root = test_root("config_parses_project_description");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"api\"\nproject_description = \"Payment API service\"\n",
        );

        let config = load_config(&root).expect("load config");
        assert_eq!(config.project_description.as_deref(), Some("Payment API service"));
    }

    /// §FS-config.3: a `project_description` with an embedded line break is a
    /// config error at the offending line — the key feeds single-line
    /// workspace member bullets.
    #[test]
    fn config_rejects_multiline_project_description() {
        let root = test_root("config_rejects_multiline_project_description");
        write(
            &root.join(".agents/grund.toml"),
            "project_description = \"first\\nsecond\"\n",
        );

        let err = match load_config(&root) {
            Ok(_) => panic!("multi-line project_description should fail"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("project_description must be a single line"),
            "unexpected error: {err:#}"
        );
    }

    /// §FS-workspace.1, §AR-workspace.3.1: a marker-prefixed qualified
    /// citation (`<§>alias/<ID>`) is recognised; an unmarked `alias/<ID>` in
    /// prose is text. There is one scan mode, not two.
    #[test]
    fn marked_qualified_citation_is_recognised_unmarked_one_is_text() {
        let root = test_root("marked_qualified_citation_is_recognised_unmarked_one_is_text");
        let body = format!(
            "# FS-login: Login\n\nMarked qualified: {marker}api/FS-login.\nBare path-shaped token: api/FS-login is just prose.\n",
            marker = "§"
        );
        write(&root.join("docs/functional-spec/FS-login.md"), &body);

        let mut config = Config::default_for(root.clone());
        config.id_format = "{kind}-{slug}".into();
        config.slug_pattern = "[a-z][a-z0-9-]*".into();
        config.rebuild_grammar().expect("rebuild grammar");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        assert_eq!(findings.citations.len(), 1, "exactly one citation expected");
        let cite = &findings.citations[0];
        assert_eq!(cite.namespace.as_deref(), Some("api"));
        assert_eq!(cite.line, 3);
    }

    /// §AR-workspace.3.1: in non-strict mode, an unmarked `path/<ID>` must
    /// not be silently promoted to a qualified citation. Was a regression on
    /// the first workspace slice; this test pins the marker-anchored rule.
    #[test]
    fn non_strict_bare_token_with_slash_prefix_is_not_a_citation() {
        let root = test_root("non_strict_bare_token_with_slash_prefix_is_not_a_citation");
        write(
            &root.join("docs/functional-spec/FS-login.md"),
            "# FS-login: Login\n\nA bare path-looking token api/FS-other in prose.\n",
        );

        let mut config = Config::default_for(root.clone());
        config.id_format = "{kind}-{slug}".into();
        config.slug_pattern = "[a-z][a-z0-9-]*".into();
        config.strict = false;
        config.rebuild_grammar().expect("rebuild grammar");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        assert!(
            findings.citations.is_empty(),
            "non-strict mode must not turn `path/FS-x` in prose into a citation"
        );
    }

    #[test]
    fn workspace_root_scope_requires_canonical_root_for_explicit_path() {
        let root =
            canonical_test_path(&test_root("workspace_root_scope_requires_canonical_root_for_explicit_path"));
        let subdir = root.join("apps/api");
        std::fs::create_dir_all(&subdir).expect("create subdir");
        let config = Config::default_for(root.clone());

        assert!(is_workspace_root_scope(&config, Path::new("."), false));
        assert!(is_workspace_root_scope(&config, &root, true));
        assert!(
            !is_workspace_root_scope(&config, &subdir, true),
            "an explicit subdirectory scope must not be promoted to workspace root"
        );
    }

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

    #[test]
    fn workspace_boundary_root_is_not_scanned_as_parent_content() {
        let root = test_root("workspace_boundary_root_is_not_scanned_as_parent_content");
        write(
            &root.join("apps/api/docs/functional-spec/FS-child.md"),
            "# FS-child: Child\n",
        );

        let mut config = Config::default_for(root.clone());
        config.id_format = "{kind}-{slug}".into();
        config.slug_pattern = "[a-z][a-z0-9-]*".into();
        config.include = Some(vec!["apps/api".into()]);
        config.workspace_boundary_roots = vec![canonical_test_path(&root.join("apps/api"))];
        config.rebuild_grammar().expect("rebuild grammar");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");

        assert!(
            findings.declarations.is_empty(),
            "a scan root that is exactly a workspace member boundary must be skipped"
        );
    }

    /// §AR-workspace.6: the root namespace must not absorb member
    /// declarations even when `[scan] include` points below a member root.
    #[test]
    fn workspace_boundary_nested_scan_root_is_not_scanned_as_parent_content() {
        let root = test_root("workspace_boundary_nested_scan_root_is_not_scanned_as_parent_content");
        let root_doc = format!(
            "# FS-root: Root\n\nThe root has a local citation to {marker}FS-child.\n",
            marker = "§"
        );
        write(
            &root.join("docs/functional-spec/FS-root.md"),
            &root_doc,
        );
        write(
            &root.join("apps/api/docs/functional-spec/FS-child.md"),
            "# FS-child: Child\n",
        );

        let mut config = Config::default_for(root.clone());
        config.id_format = "{kind}-{slug}".into();
        config.slug_pattern = "[a-z][a-z0-9-]*".into();
        config.include = Some(vec!["docs".into(), "apps/api/docs".into()]);
        config.workspace_boundary_roots = vec![canonical_test_path(&root.join("apps/api"))];
        config.rebuild_grammar().expect("rebuild grammar");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|error| error.code == "dangling"),
            "a root include below a workspace member boundary must not import member declarations"
        );
    }

    #[test]
    fn dangling_reference_suggests_nearest_declared_id() {
        let root = test_root("dangling_reference_suggests_nearest_declared_id");
        write(
            &root.join("docs/functional-spec/FS-check.md"),
            "# FS-check: Check\n",
        );

        let mut config = Config::default_for(root.clone());
        config.id_format = "{kind}-{slug}".into();
        config.slug_pattern = "[a-z][a-z0-9-]*".into();
        config.rebuild_grammar().expect("rebuild grammar");
        write(
            &root.join("src/lib.rs"),
            &format!("//! {}FS-chek\n", config.marker),
        );
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|error| error.code == "dangling"
                && error.message == "unknown reference FS-chek; did you mean FS-check?"),
            "dangling diagnostic should suggest the nearest declared ID: {:?}",
            report
                .errors
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>()
        );
    }

    /// §FS-workspace.1: a qualified citation's ID tail is parsed with the
    /// target project's grammar, not the citing project's grammar.
    #[test]
    fn workspace_qualified_citation_uses_target_id_grammar() {
        let root = test_root("workspace_qualified_citation_uses_target_id_grammar");
        write(
            &root.join("docs/functional-spec/FS-root.md"),
            "# FS-root: Root\n\nThe root cites the member: §api/FS-001-session.\n",
        );
        write(
            &root.join("apps/api/docs/functional-spec/FS-001-session.md"),
            "# FS-001-session: Session\n",
        );

        let mut root_config = Config::default_for(root.clone());
        root_config.id_format = "{kind}-{slug}".into();
        root_config.slug_pattern = "[a-z][a-z-]*".into();
        root_config.workspace_boundary_roots = vec![canonical_test_path(&root.join("apps/api"))];
        root_config.rebuild_grammar().expect("root grammar");
        let api_config = Config::default_for(root.join("apps/api"));

        let targets = vec![
            WorkspaceCitationTarget {
                alias: "root".to_string(),
                config: root_config.clone(),
            },
            WorkspaceCitationTarget {
                alias: "api".to_string(),
                config: api_config.clone(),
            },
        ];
        let (root_findings, _) =
            scan_tree_with_workspace(&root_config, Some(&root), true, &targets)
                .expect("scan root");
        let (api_findings, _) =
            scan_tree_with_workspace(&api_config, Some(&api_config.root), true, &targets)
                .expect("scan api");

        let cite = root_findings
            .citations
            .iter()
            .find(|cite| cite.namespace.as_deref() == Some("api"))
            .expect("root citation should be recognised");
        assert_eq!(cite.id.num, Some(1));
        assert_eq!(cite.id.slug.as_deref(), Some("session"));

        let workspace = BTreeMap::from([
            (
                "root".to_string(),
                WorkspaceCheckTarget {
                    findings: &root_findings,
                    config: &root_config,
                },
            ),
            (
                "api".to_string(),
                WorkspaceCheckTarget {
                    findings: &api_findings,
                    config: &api_config,
                },
            ),
        ]);
        let root_report =
            check_with_workspace(&root_findings, &root_config, Some("root"), &workspace);
        assert!(
            !root_report.errors.iter().any(|error| error.code == "dangling"),
            "target-shaped cross-project citation must resolve: {:?}",
            root_report
                .errors
                .iter()
                .map(|error| (&error.code, &error.message))
                .collect::<Vec<_>>()
        );
        let api_report = check_with_workspace(&api_findings, &api_config, Some("api"), &workspace);
        assert!(
            !api_report
                .warnings
                .iter()
                .any(|warning| warning.code == "unused"),
            "the member declaration should be counted as cited by the root"
        );
    }

    /// §FS-workspace.5: member-local checks must report qualified citations even
    /// when the cited token only matches another project's ID grammar.
    #[test]
    fn member_local_qualified_citation_with_foreign_grammar_reports_unknown_alias() {
        let root = test_root(
            "member_local_qualified_citation_with_foreign_grammar_reports_unknown_alias",
        );
        let member = root.join("apps/api");
        write(
            &member.join("docs/functional-spec/FS-001-api.md"),
            "# FS-001-api: API\n\nThe member cites the root: §root/FS-root.\n",
        );

        let config = Config::default_for(member);
        let (findings, _) = scan_tree(&config, Some(&config.root), true).expect("scan member");
        assert!(
            findings
                .citations
                .iter()
                .any(|cite| cite.namespace.as_deref() == Some("root")
                    && cite.text == "§root/FS-root"),
            "foreign-shaped qualified citation should be recognised"
        );

        let report = check_findings(&findings, &config);
        assert!(
            report.errors.iter().any(|error| {
                error.code == "unknown-project" && error.message == "unknown project alias root"
            }),
            "member-local qualified citation should report unknown alias: {:?}",
            report
                .errors
                .iter()
                .map(|error| (&error.code, &error.message))
                .collect::<Vec<_>>()
        );
    }

    /// §FS-workspace.4: a qualified dangling diagnostic names the target ID
    /// using the target project's grammar, not the citing project's grammar.
    #[test]
    fn workspace_qualified_dangling_diagnostic_uses_target_id_grammar() {
        let root = test_root("workspace_qualified_dangling_diagnostic_uses_target_id_grammar");
        write(
            &root.join("docs/functional-spec/FS-root.md"),
            "# FS-root: Root\n\nThe root cites a missing member ID: §api/FS-001-missing.\n",
        );
        std::fs::create_dir_all(root.join("apps/api/docs/functional-spec"))
            .expect("create api docs");

        let mut root_config = Config::default_for(root.clone());
        root_config.id_format = "{kind}-{slug}".into();
        root_config.slug_pattern = "[a-z][a-z-]*".into();
        root_config.workspace_boundary_roots = vec![canonical_test_path(&root.join("apps/api"))];
        root_config.rebuild_grammar().expect("root grammar");
        let api_config = Config::default_for(root.join("apps/api"));

        let targets = vec![
            WorkspaceCitationTarget {
                alias: "root".to_string(),
                config: root_config.clone(),
            },
            WorkspaceCitationTarget {
                alias: "api".to_string(),
                config: api_config.clone(),
            },
        ];
        let (root_findings, _) =
            scan_tree_with_workspace(&root_config, Some(&root), true, &targets)
                .expect("scan root");
        let (api_findings, _) =
            scan_tree_with_workspace(&api_config, Some(&api_config.root), true, &targets)
                .expect("scan api");

        let workspace = BTreeMap::from([
            (
                "root".to_string(),
                WorkspaceCheckTarget {
                    findings: &root_findings,
                    config: &root_config,
                },
            ),
            (
                "api".to_string(),
                WorkspaceCheckTarget {
                    findings: &api_findings,
                    config: &api_config,
                },
            ),
        ]);
        let report = check_with_workspace(&root_findings, &root_config, Some("root"), &workspace);
        assert!(
            report.errors.iter().any(|error| {
                error.code == "dangling"
                    && error.message == "unknown reference api/FS-001-missing"
            }),
            "dangling diagnostic should render the api ID grammar: {:?}",
            report
                .errors
                .iter()
                .map(|error| (&error.code, &error.message))
                .collect::<Vec<_>>()
        );
    }

    /// §FS-workspace.8.1 / §FS-workspace.8.2: qualified query arguments route
    /// to the alias first, then parse the ID under that project's config.
    #[test]
    fn workspace_qualified_query_uses_target_id_grammar() {
        let root = test_root("workspace_qualified_query_uses_target_id_grammar");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[id]
format = "{kind}-{slug}"
slug_pattern = "[a-z][a-z-]*"

[workspace]
members = ["apps/api"]
"#,
        );
        write(
            &root.join("docs/functional-spec/FS-root.md"),
            "# FS-root: Root\n",
        );
        write(
            &root.join("apps/api/docs/functional-spec/FS-001-session.md"),
            "# FS-001-session: Session\n\nMember body.\n",
        );

        let context = load_workspace_context(&root, true).expect("load workspace context");
        let (alias, raw_id) =
            split_qualified_id_arg("api/FS-001-session").expect("split qualified ID");
        let project = context
            .project_by_alias(alias.as_deref().unwrap())
            .expect("api project");
        let (id, section) =
            parse_id_arg(raw_id, &project.config.grammar).expect("parse with api grammar");
        assert_eq!(section, None);
        assert_eq!(id.num, Some(1));
        let shown = show_declaration(
            &project.config,
            &project.findings,
            &id,
            None,
            ShowRenderMode::Default,
            false,
        )
        .expect("show member declaration");
        assert!(shown.body.contains("Member body."));

        let root_project = context.current_project().expect("root project");
        let wrapped = wrap_markdown_links(
            "See §api/FS-001-session.",
            &root.join("docs/functional-spec/FS-root.md"),
            &root_project.config,
            &root_project.findings,
            Some(&context),
        );
        assert_eq!(
            wrapped,
            "See [§api/FS-001-session](../../apps/api/docs/functional-spec/FS-001-session.md#fs-001-session-session)."
        );
    }

    /// §FS-workspace.2 / §FS-check.2.1: an explicitly empty workspace is a
    /// configuration error for `check`, not a successful scan of nothing.
    #[test]
    fn check_rejects_workspace_with_no_projects_in_scope() {
        let root = test_root("check_rejects_workspace_with_no_projects_in_scope");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[workspace]
include_root = false
members = []
"#,
        );

        let code = command_check(&[root.to_string_lossy().into_owned()]);
        assert_eq!(code, ExitCode::from(2));
    }

    /// §FS-errors.2.1 / §AR-workspace.5.1: member config parse errors loaded
    /// from a workspace command render relative to the workspace root.
    #[test]
    fn member_config_errors_render_workspace_relative_path() {
        let root = test_root("member_config_errors_render_workspace_relative_path");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[workspace]
members = ["apps/api"]
"#,
        );
        write(
            &root.join("apps/api/.agents/grund.toml"),
            r#"grund_config_version = 1

[unknown]
"#,
        );

        let err = match load_workspace_context(&root, true) {
            Ok(_) => panic!("bad member config should fail"),
            Err(err) => err.to_string(),
        };
        assert!(
            err.contains("apps/api/.agents/grund.toml:3: unknown config section `unknown`"),
            "error should point at the member path relative to the workspace root: {err}"
        );
    }

    #[test]
    fn require_grounding_accepts_inline_declaration() {
        let root = test_root("require_grounding_accepts_inline_declaration");
        write(
            &root.join("src/router.rs"),
            "// AR-001-router: Router\n//\n// ## 1. Shape\npub struct Router;\n",
        );

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report.errors.iter().any(|e| e.code == "ungrounded"),
            "a file that declares a spec inline is grounded in the spec it is"
        );
    }

    #[test]
    fn require_grounding_ignores_markdown() {
        let root = test_root("require_grounding_ignores_markdown");
        write(
            &root.join("docs/notes.md"),
            "# Notes\n\nNothing cited here.\n",
        );

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report.errors.iter().any(|e| e.code == "ungrounded"),
            "the grounding rule applies to source files, not Markdown"
        );
    }

    #[test]
    fn require_grounding_treats_dangling_only_file_as_ungrounded() {
        let root = test_root("require_grounding_treats_dangling_only_file_as_ungrounded");
        write(
            &root.join("src/app.rs"),
            "// §FS-001-missing\npub fn run() {}\n",
        );

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|e| e.code == "dangling"),
            "the dangling citation is still its own error"
        );
        let app = canonical_test_path(&root.join("src/app.rs"));
        assert!(
            report.errors.iter().any(|e| e.code == "ungrounded"
                && e.path.as_deref().map(canonical_test_path).as_deref() == Some(app.as_path())),
            "a file whose only citation resolves to nothing is not grounded"
        );
    }

    #[test]
    fn scanner_uses_configured_comment_prefixes() {
        let root = test_root("scanner_uses_configured_comment_prefixes");
        let mut config = Config::default_for(root.clone());
        config.comment_prefixes = vec!["//".to_string()];
        config.rebuild_grammar().expect("rebuild grammar");
        write(
            &root.join("src/router.rs"),
            "// AR-001-router: Router\n//\n// ## 1. Shape\n",
        );

        let (findings, _) =
            scan_tree(&config, Some(&root.join("src/router.rs")), true).expect("scan source file");

        assert!(
            findings.declarations.contains_key(&Id {
                kind: "AR".to_string(),
                num: Some(1),
                slug: Some("router".to_string())
            }),
            "configured // prefix should allow inline declarations"
        );
    }

    #[test]
    fn scanner_rejects_markdown_heading_inside_source_comment() {
        let root = test_root("scanner_rejects_markdown_heading_inside_source_comment");
        write(
            &root.join("src/router.rs"),
            "// # AR-001-router: Router\n//\n// ## 1. Shape\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) =
            scan_tree(&config, Some(&root.join("src/router.rs")), true).expect("scan source file");

        assert!(
            !findings.declarations.contains_key(&Id {
                kind: "AR".to_string(),
                num: Some(1),
                slug: Some("router".to_string())
            }),
            "source declarations must put the ID directly after the comment marker"
        );
    }

    #[test]
    fn scanner_rejects_bare_markdown_heading_in_source_file() {
        let root = test_root("scanner_rejects_bare_markdown_heading_in_source_file");
        write(
            &root.join("src/router.rb"),
            "## AR-001-router: Router\n# ## 1. Shape\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) =
            scan_tree(&config, Some(&root.join("src/router.rb")), true).expect("scan source file");

        assert!(
            !findings.declarations.contains_key(&Id {
                kind: "AR".to_string(),
                num: Some(1),
                slug: Some("router".to_string())
            }),
            "Markdown headings are declarations only in Markdown files"
        );
    }

    #[test]
    fn stub_resolution_prefers_markdown_relative_target() {
        let root = test_root("stub_resolution_prefers_markdown_relative_target");
        write(
            &root.join("docs/architecture/AR-001-router.md"),
            "# AR-001-router: [router](../../crates/grund-core/src/router.rs)\n",
        );
        write(
            &root.join("crates/grund-core/src/router.rs"),
            "/// AR-001-router: Router\n///\n/// ## 1. Shape\npub struct Router;\n",
        );
        write(
            &root.join("src/router.rs"),
            "pub struct Router;\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report
                .errors
                .iter()
                .any(|error| matches!(error.code, "broken-stub" | "duplicate")),
            "markdown-relative inline-spec stub should not be broken or duplicate: {:?}",
            report
                .errors
                .iter()
                .map(|error| (&error.code, &error.message))
                .collect::<Vec<_>>()
        );

        let id = Id {
            kind: "AR".to_string(),
            num: Some(1),
            slug: Some("router".to_string()),
        };
        let shown = show_declaration(&config, &findings, &id, None, ShowRenderMode::Default, false)
            .expect("show inline declaration");

        assert_eq!(
            canonical_test_path(&shown.path),
            canonical_test_path(&root.join("crates/grund-core/src/router.rs")),
            "show should follow the Markdown-relative stub target, not the repo-root fallback"
        );
    }

    #[test]
    fn stub_resolution_keeps_repo_root_fallback_for_old_stubs() {
        let root = test_root("stub_resolution_keeps_repo_root_fallback_for_old_stubs");
        write(
            &root.join("docs/architecture/AR-001-router.md"),
            "# AR-001-router: [router](src/router.rs)\n",
        );
        write(
            &root.join("src/router.rs"),
            "/// AR-001-router: Router\n///\n/// ## 1. Shape\npub struct Router;\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            !report
                .errors
                .iter()
                .any(|error| error.code == "broken-stub"),
            "repo-root fallback should keep older stubs valid: {:?}",
            report
                .errors
                .iter()
                .map(|error| (&error.code, &error.message))
                .collect::<Vec<_>>()
        );

        let id = Id {
            kind: "AR".to_string(),
            num: Some(1),
            slug: Some("router".to_string()),
        };
        let shown = show_declaration(&config, &findings, &id, None, ShowRenderMode::Default, false)
            .expect("show inline declaration through fallback");

        assert_eq!(
            canonical_test_path(&shown.path),
            canonical_test_path(&root.join("src/router.rs")),
            "show should keep following repo-root-relative legacy stubs"
        );
    }

    #[test]
    fn diagnostics_render_custom_id_format() {
        let root = test_root("diagnostics_render_custom_id_format");
        write(
            &root.join(".agents/grund.toml"),
            r#"grund_config_version = 1

[id]
format = "{kind}_{number}_{slug}"
section_separator = "."
number_pattern = "\\d+"
slug_pattern = "[a-z0-9][a-z0-9-]*"
"#,
        );
        write(
            &root.join("docs/functional-spec/FS_001_alpha.md"),
            "# FS_001_alpha: Alpha\n\nMentions §FS_999_missing.\n",
        );
        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report
                .errors
                .iter()
                .any(|error| error.message == "unknown reference FS_999_missing"),
            "diagnostic should use configured ID rendering: {:?}",
            report.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn section_anchor_uses_visible_markdown_link_text() {
        let heading = "### 2.2 Dangling citations ([§FS-check.3.1](../functional-spec/FS-check.md#31-dangling-citation))";
        let text = section_anchor_text(heading, "2.2");

        assert_eq!(text, "22 Dangling citations (§FS-check.3.1)");
        assert_eq!(
            anchor_slug_github(&text),
            "22-dangling-citations-fs-check31"
        );
    }

    #[test]
    fn embedded_templates_are_lf_canonical() {
        assert_eq!(
            canonical_template_text("alpha\r\nbeta\rgamma\n"),
            "alpha\nbeta\ngamma\n"
        );

        let config = Config::default_for(PathBuf::from("."));
        assert!(!render_agents_md("demo", &config, Path::new("."), true).contains('\r'));
        assert!(!render_grund_toml("demo", None).contains('\r'));
        assert!(!canonical_template_text(AGENT_SETUP_INSTRUCTIONS).contains('\r'));
        let fs_home = init_fs_home(&config);
        for (_, contents) in docs_scaffold(&fs_home) {
            assert!(!contents.contains('\r'));
        }
    }

    #[test]
    fn agents_guidance_uses_configured_section_separator() {
        let mut config = Config::default_for(PathBuf::from("."));
        config.section_separator = "#".to_string();

        let rendered = render_agents_md("demo", &config, Path::new("."), true);

        assert!(
            rendered.contains("§<ID>#1` / `§<ID>#1.1"),
            "section examples should use the configured outer separator: {rendered}"
        );
        assert!(
            !rendered.contains("§<ID>.1` / `§<ID>.1.1"),
            "section examples must not hard-code dot as the outer separator"
        );
    }

    #[test]
    fn agents_update_appends_managed_block_when_missing() {
        let (updated, result) =
            update_agents_text("# Existing agents\n", &current_block(), "AGENTS.md")
                .expect("append block");

        assert_eq!(result, AgentsUpdateResult::Appended);
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_does_not_append_current_block_twice() {
        // §FS-init.2.2: a file already holding the current rendered block is left
        // untouched (`Unchanged` → `exists `), not rewritten and reported `updated `.
        let existing = current_block();
        let (updated, result) =
            update_agents_text(&existing, &current_block(), "AGENTS.md").expect("current block");

        assert_eq!(result, AgentsUpdateResult::Unchanged);
        assert_eq!(updated, existing);
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_rewrites_current_block_from_rendered_template() {
        // A block that differs from the current render (here: an extra hand-added
        // line) is replaced and reported `Updated`.
        let mut stale = current_block();
        stale.insert_str(stale.len() - 1, "\nhand-edited line\n");
        let existing = format!("# Local notes\n\n{stale}");

        let (updated, result) = update_agents_text(&existing, &current_block(), "AGENTS.md")
            .expect("rewrite current block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(updated.starts_with("# Local notes\n\n"));
        assert!(!updated.contains("hand-edited line"));
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_keeps_current_block_in_middle_position() {
        // §FS-init.2.3.1 / §FS-init.2.2: a block already current and already
        // sitting between user-authored sections is left byte-for-byte untouched
        // (`Unchanged` → `exists `) — nothing around it moves, nothing is rewritten.
        let existing = format!(
            "# Existing agents\n\n{}\n# Local notes\n",
            current_block()
        );
        let (updated, result) = update_agents_text(&existing, &current_block(), "AGENTS.md")
            .expect("non-EOF current block");

        assert_eq!(result, AgentsUpdateResult::Unchanged);
        assert_eq!(
            updated, existing,
            "an already-current block preserves every byte, inside and out"
        );
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert!(updated.ends_with("\n# Local notes\n"));
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_handles_crlf_line_endings() {
        // §FS-init.2.3.2: a CRLF-encoded AGENTS.md whose managed block is stale
        // (same version, different body) must still be detected and rewritten,
        // with the surrounding CRLF preserved verbatim.
        let existing = format!(
            "# Existing agents\r\n\r\n{}\r\n\r\nstale body line\r\n\r\n# Local notes\r\n",
            current_marker()
        );
        let (updated, result) = update_agents_text(&existing, &current_block(), "AGENTS.md")
            .expect("update CRLF stale block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(
            updated.starts_with("# Existing agents\r\n\r\n"),
            "CRLF prefix must be preserved verbatim"
        );
        assert!(
            updated.ends_with("\n# Local notes\r\n"),
            "CRLF suffix must be preserved verbatim"
        );
        assert_eq!(updated.matches(current_marker()).count(), 1);
        assert!(!updated.contains("stale body line"));
    }

    #[test]
    fn discovers_known_companion_agent_entrypoints() {
        let root = test_root("discovers_known_companion_agent_entrypoints");
        write(&root.join("AGENTS.override.md"), "# Codex override notes\n");
        write(&root.join("CLAUDE.md"), "# Claude notes\n");
        write(&root.join(".claude/CLAUDE.md"), "# Claude project notes\n");
        write(&root.join("GEMINI.md"), "# Gemini notes\n");
        write(
            &root.join(".github/copilot-instructions.md"),
            "# Copilot notes\n",
        );

        let companions = companion_agent_entrypoints(&root).expect("discover companions");
        let rels = companions
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            rels,
            vec![
                "AGENTS.override.md",
                "CLAUDE.md",
                ".claude/CLAUDE.md",
                "GEMINI.md",
                ".github/copilot-instructions.md"
            ]
        );
    }

    #[test]
    fn init_discovers_missing_aliases_for_existing_agent_workspaces() {
        let root = test_root("init_discovers_missing_aliases_for_existing_agent_workspaces");
        fs::create_dir_all(root.join(".claude")).expect("create .claude");
        fs::create_dir_all(root.join(".gemini")).expect("create .gemini");
        fs::create_dir_all(root.join(".github/workflows")).expect("create github metadata");

        let companions = workspace_init_companion_agent_entrypoints(&root);
        let rels = companions
            .iter()
            .map(|entrypoint| match entrypoint {
                InitCompanionAgentEntrypoint::Existing(path)
                | InitCompanionAgentEntrypoint::MissingAlias(path) => path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            })
            .collect::<Vec<_>>();

        assert_eq!(
            rels,
            vec![
                "CLAUDE.md",
                ".claude/CLAUDE.md",
                "GEMINI.md"
            ]
        );
    }

    #[test]
    fn check_ignores_companion_agent_entrypoints_without_canonical_agents_md() {
        let root =
            test_root("check_ignores_companion_agent_entrypoints_without_canonical_agents_md");
        write(&root.join("CLAUDE.md"), "# Project agent notes\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report
                .errors
                .iter()
                .all(|error| error.code != "agents-init"),
            "project-owned AGENTS.md should not require a managed block without canonical AGENTS.md"
        );
    }

    #[test]
    fn check_validates_managed_companion_without_canonical_agents_md() {
        let root =
            test_root("check_validates_managed_companion_without_canonical_agents_md");
        write(
            &root.join("CLAUDE.md"),
            "## Grounding with grund (v99)\n\nold block\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let expected_path = root.join("CLAUDE.md");

        assert!(
            report.errors.iter().any(|error| error.code == "agents-init"
                && error.path.as_deref() == Some(expected_path.as_path())
                && error.message.contains("unsupported grund init block v99")),
            "managed companion entrypoint should be version-checked without AGENTS.md: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_validates_managed_zed_rules_without_canonical_agents_md() {
        // §FS-check.3.5 / §FS-init.2.1: `.rules` is not discovered by filename
        // alone, but a managed block proves it is a grund-owned Zed companion
        // and must still get init-block drift detection.
        let root = test_root("check_validates_managed_zed_rules_without_canonical_agents_md");
        write(
            &root.join(".rules"),
            "## Grounding with grund (v99)\n\nold block\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let expected_path = root.join(".rules");

        assert!(
            report.errors.iter().any(|error| error.code == "agents-init"
                && error.path.as_deref() == Some(expected_path.as_path())
                && error.message.contains("unsupported grund init block v99")),
            "managed .rules should be version-checked without AGENTS.md: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_validates_zed_workspace_rules_when_canonical_exists() {
        // §FS-check.3.5 / §FS-init.2.1: in a Zed workspace, `.rules` is owned
        // by the Zed companion path and must be validated when AGENTS.md exists.
        let root = test_root("check_validates_zed_workspace_rules_when_canonical_exists");
        write(&root.join("AGENTS.md"), &current_block());
        write(&root.join(".zed/settings.json"), "{}\n");
        write(&root.join(".rules"), "# Zed notes without a managed block\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let expected_path = root.join(".rules");

        assert!(
            report.errors.iter().any(|error| error.code == "agents-init"
                && error.path.as_deref() == Some(expected_path.as_path())
                && error.message.contains("missing grund init block v2")),
            "Zed workspace .rules should be required to carry the managed block: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_ignores_unmanaged_generic_rules_without_zed_workspace() {
        // §FS-init.2.1: `.rules` is too generic to attribute to Zed by file
        // existence alone, so a generic unmanaged file outside a `.zed/`
        // workspace must not become a companion check target.
        let root = test_root("check_ignores_unmanaged_generic_rules_without_zed_workspace");
        write(&root.join("AGENTS.md"), &current_block());
        write(&root.join(".rules"), "# Build rules, not Zed\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let generic_rules = root.join(".rules");

        assert!(
            report.errors.iter().all(|error| {
                error.code != "agents-init"
                    || error.path.as_deref() != Some(generic_rules.as_path())
            }),
            "generic .rules must not be validated as a Zed companion: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }

    /// §FS-init.2.3.4.15: `render_workspace_members_section` returns the empty
    /// string for a target that is not inside a workspace. The Project Map
    /// section is unchanged from the no-workspace fixture (§FS-init-fixtures.6.3).
    #[test]
    fn workspace_members_empty_when_no_workspace_declared() {
        let root = test_root("workspace_members_empty_when_no_workspace_declared");
        // No `.agents/grund.toml` at all — fall through to defaults.
        assert_eq!(render_workspace_members_section(&root, None, None, "§", true), "");
        // And the rendered AGENTS.md contains neither the section heading nor
        // the discoverability line.
        let config = Config::default_for(root.clone());
        let rendered = render_agents_md("demo", &config, &root, true);
        assert!(!rendered.contains("### Workspace members"));
        assert!(!rendered.contains("Cross-project citations"));
    }

    /// §FS-init.2.3.4.15: invoked at the workspace root, the section lists
    /// every member sorted by alias, marks uninitialized members with
    /// `*(not yet initialized)*`, and includes the root row when
    /// `include_root = true` (the default). Mirrors §FS-init-fixtures.6.1.
    #[test]
    fn workspace_members_root_init_lists_aliases_and_initialization_state() {
        let root = test_root("workspace_members_root_init_lists_aliases_and_initialization_state");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"packages/*\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("packages/core")).expect("create core");
        std::fs::create_dir_all(root.join("packages/ui")).expect("create ui");
        write(&root.join("apps/api/AGENTS.md"), "## existing block\n");

        let section = render_workspace_members_section(&root, None, None, "§", true);

        assert!(section.contains("### Workspace members"));
        assert!(section.contains("Cross-project citations use §alias/<ID>."));
        assert!(section.contains("- [`api`](apps/api/AGENTS.md)"));
        assert!(
            section.contains("- [`core`](packages/core/) *(not yet initialized)*")
        );
        assert!(section.contains("- [`ui`](packages/ui/) *(not yet initialized)*"));
        // `include_root = true` (default), and the root row is rendered with
        // the uniform `alias → AGENTS.md` shape — self counts as initialized
        // even though `root/AGENTS.md` does not yet exist on disk.
        assert!(section.contains("- [`root`](AGENTS.md)"));
        // Alias-sorted: api < core < root < ui.
        let api = section.find("`api`").unwrap();
        let core = section.find("`core`").unwrap();
        let root_pos = section.find("`root`").unwrap();
        let ui = section.find("`ui`").unwrap();
        assert!(api < core && core < root_pos && root_pos < ui);
    }

    /// §FS-init.2.3.4.15: invoked inside a member, the section has the same
    /// alias list and ordering as the root run, the member-being-initialized
    /// is marked as `self` (initialized even before the write completes), and
    /// link paths are recomputed relative to the member's AGENTS.md. Mirrors
    /// §FS-init-fixtures.6.2.
    #[test]
    fn workspace_members_member_init_uses_self_exception_and_relative_paths() {
        let root = test_root("workspace_members_member_init_uses_self_exception_and_relative_paths");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"packages/*\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("packages/core")).expect("create core");
        std::fs::create_dir_all(root.join("packages/ui")).expect("create ui");
        // None of the members are initialized — root/AGENTS.md absent too.
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, None, None, "§", true);

        // Self counts as initialized — `api` row is the uniform-shape link.
        assert!(section.contains("- [`api`](AGENTS.md)"));
        // Sibling members and the workspace root all carry the marker.
        assert!(section
            .contains("- [`core`](../../packages/core/) *(not yet initialized)*"));
        assert!(section
            .contains("- [`ui`](../../packages/ui/) *(not yet initialized)*"));
        // Root row points at the workspace root *directory* because its
        // AGENTS.md does not exist.
        assert!(section.contains("- [`root`](../../) *(not yet initialized)*"));
        // Alias list and ordering are independent of which project is self.
        let api = section.find("`api`").unwrap();
        let core = section.find("`core`").unwrap();
        let root_pos = section.find("`root`").unwrap();
        let ui = section.find("`ui`").unwrap();
        assert!(api < core && core < root_pos && root_pos < ui);
    }

    /// §FS-init.2.3.4.15: companion-only init does not create the canonical
    /// AGENTS.md, so the self row must still point at the project directory when
    /// AGENTS.md is absent.
    #[test]
    fn workspace_members_companion_only_init_marks_missing_self_agents_md() {
        let root = test_root("workspace_members_companion_only_init_marks_missing_self_agents_md");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, None, None, "§", false);

        assert!(section.contains("- [`api`](./) *(not yet initialized)*"));
        assert!(!section.contains("- [`api`](AGENTS.md)"));
    }

    /// §FS-init.2.3.4.15: the discoverability line uses the target project's
    /// configured marker, not a hard-coded `§`.
    #[test]
    fn workspace_members_discoverability_line_uses_configured_marker() {
        let root = test_root("workspace_members_discoverability_line_uses_configured_marker");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");

        let section = render_workspace_members_section(&root, None, None, "@", true);

        assert!(section.contains("Cross-project citations use @alias/<ID>."));
        assert!(!section.contains("Cross-project citations use §alias/<ID>."));
    }

    /// §FS-init.2.3.4.15: when a member has no local config yet, its self row
    /// uses the `project_name` that `init` is about to write instead of the
    /// directory basename, so the generated block matches later workspace
    /// resolution.
    #[test]
    fn workspace_members_member_init_uses_pending_name_for_self_alias() {
        let root = test_root("workspace_members_member_init_uses_pending_name_for_self_alias");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, Some("service"), None, "§", true);

        assert!(section.contains("- [`service`](AGENTS.md)"));
        assert!(
            !section.contains("`api`"),
            "the basename fallback must not leak into the generated block"
        );
    }

    /// §FS-init.2.3.4.15: `include_root = false` drops the root row entirely;
    /// the section still emits when there is at least one member to list.
    #[test]
    fn workspace_members_omits_root_when_include_root_false() {
        let root = test_root("workspace_members_omits_root_when_include_root_false");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\ninclude_root = false\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");

        let section = render_workspace_members_section(&root, None, None, "§", true);

        assert!(section.contains("### Workspace members"));
        assert!(section.contains("`api`"));
        assert!(
            !section.contains("`root`"),
            "include_root = false should suppress the root row entirely"
        );
    }

    /// §FS-init.2.3.4.15 + §FS-workspace.6: a configured-but-misconfigured
    /// workspace (e.g. a member directory that does not exist) silently
    /// suppresses the section so `init` does not fail. `grund check` will
    /// surface the configuration error separately.
    #[test]
    fn workspace_members_silently_skipped_on_workspace_config_error() {
        let root = test_root("workspace_members_silently_skipped_on_workspace_config_error");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        // `apps/api` directory missing — `expand_workspace_members` errors,
        // but `render_workspace_members_section` must degrade gracefully.

        assert_eq!(render_workspace_members_section(&root, None, None, "§", true), "");
    }

    /// §FS-init.2.3.4.15: duplicate aliases are a workspace configuration
    /// error, so `init` must suppress the section instead of rendering
    /// ambiguous bullets with the same alias.
    #[test]
    fn workspace_members_suppresses_duplicate_aliases() {
        let root = test_root("workspace_members_suppresses_duplicate_aliases");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"services/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create apps/api");
        std::fs::create_dir_all(root.join("services/api")).expect("create services/api");

        assert_eq!(render_workspace_members_section(&root, None, None, "§", true), "");
    }

    /// §FS-init.2.3.4.15 + §DF-workspace-member-descriptions: a project's
    /// `project_description` renders after its link (before any trailing
    /// marker), and a project without one keeps the link-only bullet. Mirrors
    /// §FS-init-fixtures.6.4.
    #[test]
    fn workspace_members_renders_configured_descriptions() {
        let root = test_root("workspace_members_renders_configured_descriptions");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\nproject_description = \"Workspace root: shared specs\"\n\n[workspace]\nmembers = [\"apps/api\", \"packages/*\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("packages/core")).expect("create core");
        std::fs::create_dir_all(root.join("packages/ui")).expect("create ui");
        write(
            &root.join("apps/api/.agents/grund.toml"),
            "project_name = \"api\"\nproject_description = \"Payment API service\"\n",
        );
        write(&root.join("apps/api/AGENTS.md"), "## existing block\n");
        write(
            &root.join("packages/core/.agents/grund.toml"),
            "project_name = \"core\"\nproject_description = \"Core domain library\"\n",
        );

        let section = render_workspace_members_section(&root, None, None, "§", true);

        // Initialized member: description after the link.
        assert!(section
            .contains("- [`api`](apps/api/AGENTS.md): Payment API service"));
        // Uninitialized member: description before the trailing marker.
        assert!(section.contains(
            "- [`core`](packages/core/): Core domain library *(not yet initialized)*"
        ));
        // Root row: description from the root config.
        assert!(section.contains("- [`root`](AGENTS.md): Workspace root: shared specs"));
        // No config ⇒ no description ⇒ bullet unchanged.
        assert!(section.contains("- [`ui`](packages/ui/) *(not yet initialized)*"));
    }

    /// §FS-init.2.3.4.15: when a member has no local config yet, its self row
    /// uses the pending `project_description` from `--description`, mirroring
    /// the pending `project_name` behavior.
    #[test]
    fn workspace_members_member_init_uses_pending_description_for_self_row() {
        let root = test_root("workspace_members_member_init_uses_pending_description_for_self_row");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(
            &api_target,
            Some("service"),
            Some("Billing service"),
            "§",
            true,
        );

        assert!(section.contains("- [`service`](AGENTS.md): Billing service"));
    }

    /// §FS-init.2.4 + §DF-workspace-member-descriptions: the generated config
    /// teaches `project_description` with a commented line by default, and
    /// `--description` turns it into the real key.
    #[test]
    fn grund_toml_description_teaching_line_and_substitution() {
        let teaching_line =
            "# project_description = \"<one line shown next to this project in workspace member lists>\"";
        let without = render_grund_toml("demo", None);
        assert!(without.contains(teaching_line));
        assert!(!without.contains("\nproject_description = "));

        let with = render_grund_toml("demo", Some("Demo \"quoted\" service"));
        assert!(!with.contains(teaching_line));
        assert!(with.contains("project_description = \"Demo \\\"quoted\\\" service\""));
    }

    // §AR-scanner.2.4: a Markdown declaration's body runs until the next
    // same-or-higher heading; an enclosed citation is classified by the
    // declaration's kind.
    #[test]
    fn scanner_markdown_body_and_source_kind() {
        let root = test_root("scanner_markdown_body_and_source_kind");
        write(
            &root.join("docs/goals.md"),
            "# Goals\n\n## GOAL-001-first: First\n\nGrounds in §GRUND-001-why.\n\n### 1. Detail\n\nMore.\n\n## GOAL-002-second: Second\n\nNothing cited.\n",
        );
        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let first = Id {
            kind: "GOAL".into(),
            num: Some(1),
            slug: Some("first".into()),
        };
        let decls = findings.declarations.get(&first).expect("GOAL-first");
        let decl = &decls[0];
        // Body runs from the `## GOAL-first` line up to (not including) the next
        // H2 `## GOAL-second`.
        assert_eq!(decl.body_start, 3);
        assert_eq!(decl.body_end, 10);

        let cite = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("why"))
            .expect("GRUND-why citation");
        assert_eq!(cite.source_kind, "GOAL");
        assert_eq!(cite.enclosing_declaration.as_ref(), Some(&first));
    }

    // §AR-scanner.2.4: a citation in a source file outside any inline
    // declaration falls through to the reserved `code` pseudo-kind; one inside
    // an inline declaration's comment block takes that declaration's kind.
    #[test]
    fn scanner_code_source_kind_and_inline_block() {
        let root = test_root("scanner_code_source_kind_and_inline_block");
        write(
            &root.join("src/app.rs"),
            "/// AR-001-router: Router\n/// Implements §FS-001-cli.\n\nfn main() {\n    // see §FS-002-check\n}\n",
        );
        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let inline = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("cli"))
            .expect("FS-cli citation");
        assert_eq!(inline.source_kind, "AR");
        assert_eq!(inline.enclosing_declaration.as_ref().map(|id| id.kind.as_str()), Some("AR"));

        let loose = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("check"))
            .expect("FS-check citation");
        assert_eq!(loose.source_kind, "code");
        assert!(loose.enclosing_declaration.is_none());
    }

    // §AR-scanner.2.4 step 2: a citation in a Markdown file under a kind home
    // but outside any declaration body takes the file's home kind.
    #[test]
    fn scanner_file_home_source_kind() {
        let root = test_root("scanner_file_home_source_kind");
        write(
            &root.join("docs/architecture/README.md"),
            "Overview prose citing §FS-001-cli before any declaration.\n",
        );
        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let cite = findings
            .citations
            .iter()
            .find(|c| c.id.slug.as_deref() == Some("cli"))
            .expect("FS-cli citation");
        assert_eq!(cite.source_kind, "AR");
        assert!(cite.enclosing_declaration.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn claude_symlink_to_agents_is_not_a_companion_entrypoint() {
        let root = test_root("claude_symlink_to_agents_is_not_a_companion_entrypoint");
        write(&root.join("AGENTS.md"), &current_block());
        std::os::unix::fs::symlink("AGENTS.md", root.join("CLAUDE.md"))
            .expect("create CLAUDE.md symlink");

        let companions = companion_agent_entrypoints(&root).expect("discover companions");

        assert!(
            companions.is_empty(),
            "CLAUDE.md symlinked to AGENTS.md should be covered by AGENTS.md"
        );
    }
}
