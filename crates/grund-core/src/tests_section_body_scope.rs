/// Test module: which numbered headings are a declaration's **own** sections
/// (§FS-check.3.16's body scope, §FS-show.2.5, §FS-show.2.3.1), and the agreement
/// that rests on the answer — `grund <ID>.<path>` refuses exactly the coordinates
/// `grund check` reports as `duplicate-section` (§FS-show.2.2.2). The cases here
/// are the shapes that made the two disagree: a heading inside a fenced example, a
/// heading in the next item's doc-comment, and a stub whose prose repeats one.
///
/// Split from `tests_duplicate_sections.rs`, which keeps the rule itself — what
/// the finding says, where it anchors, which modes refuse. These fail together
/// for a different reason: not the rule being wrong, but it being pointed at
/// headings the declaration does not own.
#[cfg(test)]
mod tests_section_body_scope {
    use super::*;
    use super::tests_support::*;

    fn alpha() -> Id {
        Id {
            kind: "FS".to_string(),
            num: Some(1),
            slug: Some("alpha".to_string()),
        }
    }

    /// §FS-show.2.5 / §FS-check.3.16: a numbered heading inside a fenced block is
    /// an example, not a claimant — in a repository whose documents *are* Markdown
    /// examples, the difference is every spec in the tree. `check` stays silent and
    /// `show` returns the section whole, fence included.
    #[test]
    fn a_fenced_example_is_not_a_second_claimant() {
        let root = test_root("duplicate_sections_fenced_example");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\n## 1. First\n\nTwo headings claiming one path:\n\n\
             ```markdown\n## 1. First\n\n## 1. Second\n```\n\nStill section one.\n\n\
             ## 2. Second\n\nBody.\n",
        );
        write(&root.join("src/alpha.rs"), "// Implements \u{a7}FS-001-alpha.1\n");
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            error_codes(&report),
            Vec::<String>::new(),
            "the scan skips fenced lines, so nothing claimed `1` twice"
        );
        let decl = &findings.declarations[&alpha()][0];
        assert!(decl.duplicate_sections.is_empty());
        assert_eq!(decl.sections.keys().collect::<Vec<_>>(), vec!["1", "2"]);

        let shown = show_declaration(
            &config,
            &config,
            &findings,
            &alpha(),
            Some("1"),
            ShowRenderMode::Full,
            false,
        )
        .expect("§FS-show.2.5: a fence is body text, never a section boundary");
        assert_eq!(
            shown.body,
            "## 1. First\n\nTwo headings claiming one path:\n\n\
             ```markdown\n## 1. First\n\n## 1. Second\n```\n\nStill section one.\n",
            "the whole section, fence and all — not a slice cut at a fake heading"
        );
        let toc = show_declaration(
            &config,
            &config,
            &findings,
            &alpha(),
            None,
            ShowRenderMode::Outline,
            false,
        )
        .expect("--toc maps the declaration");
        assert_eq!(
            toc.body, "## 1. First\n## 2. Second\n",
            "an example heading is not offered as a coordinate to cite"
        );
    }

    fn core() -> Id {
        Id {
            kind: "AR".to_string(),
            num: Some(1),
            slug: Some("core".to_string()),
        }
    }

    /// §FS-check.3.16, the body scope: the scan's "current declaration" runs to the
    /// next declaration line, so a `## 1.` in the *next* item's doc-comment lands on
    /// the one above it. It is not one of that declaration's sections — `show` stops
    /// at the blank line ending the comment block (§FS-show.2.3.1) and never reads it
    /// — so a collision reported against it asks for a renumbering that changes what
    /// nothing points at.
    #[test]
    fn a_heading_in_a_later_doc_comment_is_not_this_declarations_section() {
        let root = test_root("duplicate_sections_later_doc_comment");
        write(
            &root.join("src/core.rs"),
            "/// AR-001-core: Core module\n///\n/// ## 1. First\n///\n/// First body.\n\
             pub struct A;\n\n/// ## 1. Second\n///\n/// Second body.\npub struct B;\n",
        );
        write(&root.join("docs/goals.md"), "# GOAL-x: X\n\nCites §AR-001-core.1.\n");
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            error_codes(&report),
            Vec::<String>::new(),
            "line 8 is outside the declaration's body, so it collides with nothing"
        );
        assert!(
            findings.declarations[&core()][0]
                .duplicate_sections
                .is_empty(),
            "an out-of-body heading is dropped before any rule reads the list"
        );
        let shown = show_declaration(
            &config,
            &config,
            &findings,
            &core(),
            Some("1"),
            ShowRenderMode::Full,
            false,
        )
        .expect("the section resolves");
        assert_eq!(
            shown.body, "## 1. First\n\nFirst body.\n",
            "which is the proof: the second heading is not part of this body"
        );
    }

    /// The same collision written *inside* one doc-comment is the real thing, in
    /// Rust `///` and in a Python docstring alike (§FS-show.2.3) — the rule is about
    /// the declaration's body, not about Markdown files.
    #[test]
    fn a_collision_inside_one_doc_comment_is_reported() {
        let root = test_root("duplicate_sections_inside_one_doc_comment");
        write(
            &root.join("src/core.rs"),
            "/// AR-001-core: Core module\n///\n/// ## 1. First\n///\n/// First body.\n\
             ///\n/// ## 1. Second\n///\n/// Second body.\npub struct A;\n",
        );
        write(
            &root.join("src/router.py"),
            "\"\"\"\nAR-002-router: Router\n\n## 1. First\n\nFirst body.\n\n\
             ## 1. Second\n\nSecond body.\n\"\"\"\n",
        );
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            located_diagnostics(
                &config,
                report.errors.iter().filter(|e| e.code == "duplicate-section")
            ),
            vec![
                "src/core.rs:3: duplicate section AR-001-core.1 (also declared at src/core.rs:7)",
                "src/router.py:4: duplicate section AR-002-router.1 \
                 (also declared at src/router.py:8)",
            ]
        );
    }

    /// §FS-check.3.16, the stub clause: a stub's heading tail is a path, its body is
    /// one line, and the prose under it belongs to no declaration's sections. The
    /// headings that count are the inline home's — the file `show` actually reads.
    #[test]
    fn a_stubs_own_prose_declares_no_sections() {
        let root = test_root("duplicate_sections_stub_prose");
        write(
            &root.join("docs/architecture/AR-001-core.md"),
            "# AR-001-core: [src/core.rs](../../src/core.rs)\n\n## 1. Note\n\nSome prose.\n\n\
             ## 1. Note again\n\nMore prose.\n",
        );
        write(
            &root.join("src/core.rs"),
            "/// AR-001-core: Core module\n///\n/// ## 1. First\n///\n/// First body.\n\
             pub struct A;\n",
        );
        write(&root.join("docs/goals.md"), "# GOAL-x: X\n\nCites §AR-001-core.1.\n");
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            error_codes(&report),
            Vec::<String>::new(),
            "a stub declares no sections, so it collides with nothing"
        );
        let shown = show_declaration(
            &config,
            &config,
            &findings,
            &core(),
            Some("1"),
            ShowRenderMode::Full,
            false,
        )
        .expect("the query resolves through the stub to the inline home");
        assert_eq!(shown.body, "## 1. First\n\nFirst body.\n");
    }

    /// And the other half of the stub case: a collision in the *inline home* is
    /// refused for the stubbed ID too — `show` resolves the sections through the
    /// stub rather than reading the stub's own record (§FS-show.2.2.2).
    #[test]
    fn a_collision_in_the_inline_home_refuses_through_the_stub() {
        let root = test_root("duplicate_sections_stub_to_colliding_home");
        write(
            &root.join("docs/architecture/AR-001-core.md"),
            "# AR-001-core: [src/core.rs](../../src/core.rs)\n",
        );
        write(
            &root.join("src/core.rs"),
            "/// AR-001-core: Core module\n///\n/// ## 1. First\n///\n/// First body.\n\
             ///\n/// ## 1. Second\n///\n/// Second body.\npub struct A;\n",
        );
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            located_diagnostics(
                &config,
                report.errors.iter().filter(|e| e.code == "duplicate-section")
            ),
            vec!["src/core.rs:3: duplicate section AR-001-core.1 (also declared at src/core.rs:7)"]
        );
        let Err(err) = show_declaration(
            &config,
            &config,
            &findings,
            &core(),
            Some("1"),
            ShowRenderMode::Default,
            false,
        ) else {
            panic!("the inline home's collision is the stubbed ID's collision");
        };
        assert_eq!(
            format!("{err:#}"),
            "ambiguous section: AR-001-core.1 (declared at src/core.rs:3, src/core.rs:7)",
            "the sites are the home's lines, not the stub's"
        );
    }

    /// The premise of the whole rule, as one assertion over a tree holding every
    /// shape that has broken it: `grund <ID>.<path>` refuses **if and only if**
    /// `check` reports `duplicate-section` for that exact coordinate. Two readers
    /// each deciding for themselves is how §DF-duplicate-section-path.1 happened,
    /// and a coordinate `check` calls clean that `show` will not resolve is
    /// §REQ-no-wrong-citation failing in the quiet direction.
    #[test]
    fn show_refuses_exactly_the_coordinates_check_reports() {
        let root = test_root("duplicate_sections_check_and_show_agree");
        // A real collision, plus a fenced example of one.
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\n## 1. First\n\n\
             ```markdown\n## 2. Fenced\n\n## 2. Fenced again\n```\n\n\
             ## 1. Second\n\nBody.\n\n## 2. Two\n\nBody.\n",
        );
        // A clean neighbour: nothing here may refuse.
        write(
            &root.join("docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\n## 1. One\n\nBody.\n\n### 1.1 Nested\n\nBody.\n",
        );
        // An inline home whose next item repeats a heading, behind a stub.
        write(
            &root.join("docs/architecture/AR-001-core.md"),
            "# AR-001-core: [src/core.rs](../../src/core.rs)\n\n## 1. Stub prose\n\n\
             Prose.\n\n## 1. Stub prose again\n\nProse.\n",
        );
        write(
            &root.join("src/core.rs"),
            "/// AR-001-core: Core module\n///\n/// ## 1. First\n///\n/// First body.\n\
             pub struct A;\n\n/// ## 1. Second\n///\n/// Second body.\npub struct B;\n",
        );
        // A docstring that really does collide.
        write(
            &root.join("src/router.py"),
            "\"\"\"\nAR-002-router: Router\n\n## 1. First\n\nFirst body.\n\n\
             ## 1. Second\n\nSecond body.\n\"\"\"\n",
        );
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        let mut coordinates = 0usize;
        let mut refusals = 0usize;
        for (id, decls) in &findings.declarations {
            let mut paths: BTreeSet<&str> = BTreeSet::new();
            for decl in decls {
                paths.extend(decl.sections.keys().map(String::as_str));
                paths.extend(decl.duplicate_sections.iter().map(|(path, _)| path.as_str()));
            }
            for path in paths {
                let prefix = format!(
                    "duplicate section {}{}{} (",
                    render_id(&config, id),
                    config.section_separator,
                    path
                );
                let reported = report.errors.iter().any(|error| {
                    error.code == "duplicate-section" && error.message.starts_with(&prefix)
                });
                let refused = show_declaration(
                    &config,
                    &config,
                    &findings,
                    id,
                    Some(path),
                    ShowRenderMode::Full,
                    false,
                )
                .err()
                .is_some_and(|err| format!("{err:#}").starts_with("ambiguous section:"));
                assert_eq!(
                    reported,
                    refused,
                    "check and show disagree about {}{}{}",
                    render_id(&config, id),
                    config.section_separator,
                    path
                );
                coordinates += 1;
                refusals += usize::from(refused);
            }
        }
        assert!(
            coordinates >= 6,
            "the fixture must offer coordinates of every shape, got {coordinates}"
        );
        assert_eq!(
            refusals, 2,
            "exactly the Markdown collision and the docstring one refuse"
        );
    }
}
