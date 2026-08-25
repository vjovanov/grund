/// Test module: workspace boundaries and cross-project qualified citations (§FS-workspace)
#[cfg(test)]
mod tests_workspace {
    use super::*;
    use super::tests_support::*;

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
            check_with_workspace(&root_findings, &root_config, &root_config, Some("root"), &workspace);
        assert!(
            !root_report.errors.iter().any(|error| error.code == "dangling"),
            "target-shaped cross-project citation must resolve: {:?}",
            root_report
                .errors
                .iter()
                .map(|error| (&error.code, &error.message))
                .collect::<Vec<_>>()
        );
        let api_report = check_with_workspace(&api_findings, &api_config, &root_config, Some("api"), &workspace);
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
        let report = check_with_workspace(&root_findings, &root_config, &root_config, Some("root"), &workspace);
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
            None,
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
}
