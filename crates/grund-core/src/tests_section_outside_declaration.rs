/// Test module: section coordinates stay inside the declaration body that owns
/// them, and headings left behind by stale scanner context become the hard
/// finding specified by §FS-check.3.23 and the shared-map contract in
/// §FS-show.2.1.2.
#[cfg(test)]
mod tests_section_outside_declaration {
    use super::tests_support::*;
    use super::*;

    fn id(kind: &str, number: u32, slug: &str) -> Id {
        Id {
            kind: kind.to_string(),
            num: Some(number),
            slug: Some(slug.to_string()),
        }
    }

    fn section_paths(findings: &Findings, id: &Id, file_suffix: &str) -> Vec<String> {
        findings.declarations[id]
            .iter()
            .find(|declaration| declaration.file.ends_with(file_suffix))
            .unwrap_or_else(|| panic!("missing {id:?} declaration in {file_suffix}"))
            .sections
            .keys()
            .cloned()
            .collect()
    }

    /// The ticket shape, plus its two controls: deeper headings before the
    /// body-closing plain chapter remain coordinates, while a fenced
    /// pseudo-heading remains content. Numeric and enabled named headings after
    /// that chapter are findings, not coordinates.
    #[test]
    fn markdown_section_map_stops_at_the_declaration_body() {
        let root = test_root("section_outside_markdown");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "## FS-001-alpha: Alpha\n\nIntro.\n\n\
             ### 2. Inside\n\nInside body.\n\n\
             ```markdown\n### 9. Fenced example\n```\n\n\
             ### goals: Goals\n\nAlso inside.\n\n\
             ## Plain chapter\n\nOutside prose.\n\n\
             ### 1. Outside body\n\nWrong owner.\n\n\
             ### later: Outside named body\n\nWrong owner too.\n",
        );
        let mut config = legacy_fs_folder_config(root.clone());
        config.named_sections = true;
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);
        let alpha = id("FS", 1, "alpha");

        assert_eq!(
            section_paths(
                &findings,
                &alpha,
                "docs/functional-spec/FS-001-alpha.md"
            ),
            ["2", "goals"],
            "the shared coordinate map contains only real in-body headings"
        );
        assert_eq!(
            located_diagnostics(
                &config,
                report
                    .errors
                    .iter()
                    .filter(|finding| finding.code == "section-outside-declaration")
            ),
            [
                "docs/functional-spec/FS-001-alpha.md:21: numbered section outside any declaration",
                "docs/functional-spec/FS-001-alpha.md:25: named section outside any declaration",
            ]
        );

        assert!(
            show_declaration(
                &config,
                &config,
                &findings,
                &alpha,
                Some("2"),
                ShowRenderMode::Full,
                false,
            )
            .is_ok(),
            "the in-body control remains resolvable"
        );
        let error = match show_declaration(
            &config,
            &config,
            &findings,
            &alpha,
            Some("1"),
            ShowRenderMode::Full,
            false,
        ) {
            Ok(_) => panic!("an outside heading is not a queryable coordinate"),
            Err(error) => error,
        };
        assert_eq!(format!("{error:#}"), "section not found: FS-001-alpha.1");
    }

    /// Source block ends, docstring ends, stub bodies, and a next declaration in
    /// one shared block all use the already-defined body spans. The first three
    /// leave a heading outside; the shared-block control transfers ownership to
    /// the second declaration instead of creating an orphan.
    #[test]
    fn source_docstring_stub_and_shared_block_boundaries_own_sections_once() {
        let root = test_root("section_outside_source_boundaries");
        write(
            &root.join("src/core.rs"),
            "/// AR-001-core: Core\n///\n/// ## 1. Inside\n///\n/// Body.\n\
             pub struct Core;\n\n\
             /// ## 2. Outside\n///\n/// Later item.\npub struct Later;\n",
        );
        write(
            &root.join("src/router.py"),
            "\"\"\"\nAR-002-router: Router\n\n## 1. Inside\n\nBody.\n\"\"\"\n\n\
             \"\"\"\n## 2. Outside\n\nLater docstring.\n\"\"\"\n",
        );
        write(
            &root.join("docs/architecture/AR-003-stubbed.md"),
            "# AR-003-stubbed: [src/stubbed.rs](../../src/stubbed.rs)\n\n\
             ## 9. Outside stub body\n",
        );
        write(
            &root.join("src/stubbed.rs"),
            "/// AR-003-stubbed: Stubbed\n///\n/// ## 1. Inside\n///\n/// Body.\n\
             pub struct Stubbed;\n",
        );
        write(
            &root.join("src/shared.rs"),
            "/// AR-004-first: First\n///\n/// ## 1. First body\n///\n\
             /// F\
             S-004-second: Second\n///\n/// ## 2. Second body\n\
             pub struct Shared;\n",
        );
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            section_paths(&findings, &id("AR", 1, "core"), "src/core.rs"),
            ["1"]
        );
        assert_eq!(
            section_paths(&findings, &id("AR", 2, "router"), "src/router.py"),
            ["1"]
        );
        assert_eq!(
            section_paths(&findings, &id("AR", 3, "stubbed"), "src/stubbed.rs"),
            ["1"]
        );
        assert_eq!(
            section_paths(&findings, &id("AR", 4, "first"), "src/shared.rs"),
            ["1"]
        );
        assert_eq!(
            section_paths(&findings, &id("FS", 4, "second"), "src/shared.rs"),
            ["2"]
        );
        assert_eq!(
            located_diagnostics(
                &config,
                report
                    .errors
                    .iter()
                    .filter(|finding| finding.code == "section-outside-declaration")
            ),
            [
                "docs/architecture/AR-003-stubbed.md:3: numbered section outside any declaration",
                "src/core.rs:8: numbered section outside any declaration",
                "src/router.py:10: numbered section outside any declaration",
            ],
            "each unowned source-shaped heading is reported once"
        );
    }
}
