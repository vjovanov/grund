/// Test module: scanner comment handling, stubs, and anchors (§AR-scanner)
#[cfg(test)]
mod tests_scanner {
    use super::*;
    use super::tests_support::*;

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
        let shown = show_declaration(&config, &config, &findings, &id, None, ShowRenderMode::Default, false)
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
        let shown = show_declaration(&config, &config, &findings, &id, None, ShowRenderMode::Default, false)
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

    /// §DF-github-anchor-fidelity: a renderer resolves inline code spans before
    /// it looks for markup, so `<alias>/<ID>` inside backticks is literal text
    /// and survives into the anchor. Verified against the rendered heading on
    /// github.com, which carries `id="user-content-81-grund-aliasid"`.
    #[test]
    fn section_anchor_keeps_angle_brackets_inside_code_spans() {
        let heading = "### 8.1 `grund <alias>/<ID>`";
        let text = section_anchor_text(heading, "8.1");

        assert_eq!(text, "81 `grund <alias>/<ID>`");
        assert_eq!(anchor_slug_github(&text), "81-grund-aliasid");

        // Outside a code span the same shape *is* a tag, and a renderer drops
        // it — leaving the space that preceded it, which slugs to a trailing
        // `-`. `## RM-refs: grund refs <ID>` really does carry
        // `id="user-content-rm-refs-grund-refs-"` on github.com, so the two
        // cases must not be conflated.
        let raw = reduce_heading_text("RM-refs: grund refs <ID>");
        assert_eq!(raw, "RM-refs: grund refs ");
        assert_eq!(anchor_slug_github(&raw), "rm-refs-grund-refs-");

        // A backtick run that never closes is ordinary text, not an opener that
        // swallows the rest of the heading.
        let unclosed = section_anchor_text("### 3.1 a ` b <ID>", "3.1");
        assert_eq!(anchor_slug_github(&unclosed), "31-a--b");

        // A link inside a code span is literal too — no label extraction.
        let literal_link = section_anchor_text("### 4.2 `[a](b)`", "4.2");
        assert_eq!(anchor_slug_github(&literal_link), "42-ab");
    }
}
