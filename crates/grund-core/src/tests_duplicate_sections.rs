/// Test module: two headings claiming one dotted section path (§FS-check.3.16,
/// §FS-show.2.2.2, §DF-duplicate-section-path). The scanner half is
/// first-wins recording, the checker half is the error, the `show` half is the
/// refusal that replaced a body assembled out of both headings.
#[cfg(test)]
mod tests_duplicate_sections {
    use super::*;
    use super::tests_support::*;

    fn alpha() -> Id {
        Id {
            kind: "FS".to_string(),
            num: Some(1),
            slug: Some("alpha".to_string()),
        }
    }

    /// A repo whose `FS-001-alpha` writes `## 1.` twice, plus whatever extra
    /// body the caller needs. Cited so the unused-declaration warning stays out
    /// of the way of what each case is asserting.
    fn duplicated_repo(name: &str, extra: &str) -> (PathBuf, Config) {
        let root = test_root(name);
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            &format!(
                "# FS-001-alpha: Alpha\n\nLead.\n\n## 1. First\n\nFirst body.\n\n## 1. Second\n\nSecond body.\n{extra}"
            ),
        );
        write(&root.join("src/alpha.rs"), "// Implements \u{a7}FS-001-alpha.1\n");
        let config = legacy_fs_folder_config(root.clone());
        (root, config)
    }

    /// §AR-scanner.2.2: the path is recorded once, by the first heading, and the
    /// later claimant goes to `duplicate_sections` rather than overwriting it.
    #[test]
    fn scanner_records_the_first_heading_and_keeps_the_rest_beside_it() {
        let (root, config) = duplicated_repo("duplicate_sections_scanner_records_first", "");
        let findings = scan_findings(&config, &root);

        let decl = &findings.declarations[&alpha()][0];
        assert_eq!(
            decl.sections["1"].line, 5,
            "§AR-scanner.2.2: the map holds the first heading, not the last"
        );
        assert_eq!(decl.sections["1"].title, "1 First");
        assert_eq!(
            decl.duplicate_sections
                .iter()
                .map(|(path, info)| (path.as_str(), info.line))
                .collect::<Vec<_>>(),
            vec![("1", 9)],
            "the later claimant is kept so §FS-check.3.16 can name its line"
        );
    }

    /// §FS-check.3.16: one error, anchored at the first heading, naming the rest.
    #[test]
    fn check_reports_the_collision_anchored_at_the_first_heading() {
        let (root, config) = duplicated_repo("duplicate_sections_check_reports", "");
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        let errors: Vec<&Diagnostic> = report
            .errors
            .iter()
            .filter(|error| error.code == "duplicate-section")
            .collect();
        assert_eq!(errors.len(), 1, "one finding per collided path: {:?}", error_codes(&report));
        assert_eq!(
            located_diagnostics(&config, errors.clone()),
            vec![
                "docs/functional-spec/FS-001-alpha.md:5: duplicate section FS-001-alpha.1 \
                 (also declared at docs/functional-spec/FS-001-alpha.md:9)"
            ]
        );
        assert_eq!(
            errors[0]
                .sites
                .iter()
                .map(|site| site.line)
                .collect::<Vec<_>>(),
            vec![5, 9],
            "§FS-errors.5: a multi-site finding carries every site"
        );
        // The rest of the report is the assertion too: a rule that fires once
        // correctly and once spuriously is still a rule that turns a tree red for
        // the wrong reason, and filtering by code hides exactly that.
        assert_eq!(
            error_codes(&report),
            vec!["duplicate-section@5"],
            "the collision is the only error this tree earns"
        );
        assert!(
            report.warnings.iter().all(|warning| warning.code != "unused"),
            "fixture keeps the declaration cited so the report is only the rule"
        );
    }

    /// A third claimant joins the message rather than opening a second finding.
    #[test]
    fn a_third_heading_joins_the_same_finding() {
        let (root, config) = duplicated_repo(
            "duplicate_sections_third_heading",
            "\n## 1. Third\n\nThird body.\n",
        );
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            located_diagnostics(
                &config,
                report.errors.iter().filter(|e| e.code == "duplicate-section")
            ),
            vec![
                "docs/functional-spec/FS-001-alpha.md:5: duplicate section FS-001-alpha.1 \
                 (also declared at docs/functional-spec/FS-001-alpha.md:9, \
                 docs/functional-spec/FS-001-alpha.md:13)"
            ]
        );
    }

    /// §DF-duplicate-section-path.2.2: a section path is addressed as
    /// `<ID>.<path>`, so the same number under two declarations never collided.
    #[test]
    fn the_same_path_under_two_declarations_is_not_a_collision() {
        let root = test_root("duplicate_sections_scoped_to_one_declaration");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\n## 1. Inputs\n\nBody.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\n## 1. Inputs\n\nBody.\n",
        );
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().all(|error| error.code != "duplicate-section"),
            "two declarations each owning a `1.` are two coordinates: {:?}",
            error_codes(&report)
        );
    }

    /// §DF-duplicate-section-path.2.2: the collision is a collision in every
    /// `[id] section_heading_levels` mode — `"loose"`, where `## 1.` and `### 1.`
    /// both claim path `1`, most of all.
    #[test]
    fn loose_heading_levels_do_not_excuse_the_collision() {
        let root = test_root("duplicate_sections_loose_mode");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\n## 1. First\n\nBody.\n\n### 1. Second\n\nBody.\n",
        );
        let mut config = legacy_fs_folder_config(root.clone());
        config.section_heading_levels = "loose".to_string();
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert_eq!(
            located_diagnostics(
                &config,
                report.errors.iter().filter(|e| e.code == "duplicate-section")
            ),
            vec![
                "docs/functional-spec/FS-001-alpha.md:3: duplicate section FS-001-alpha.1 \
                 (also declared at docs/functional-spec/FS-001-alpha.md:7)"
            ]
        );
    }

    /// §DF-duplicate-section-path.2.4: §FS-check.3.9 judges the heading the path
    /// resolves to — the first — and does not additionally measure the duplicate,
    /// which nothing resolves to and the run has already said should not exist.
    #[test]
    fn the_heading_level_rule_judges_only_the_resolving_heading() {
        let root = test_root("duplicate_sections_heading_level_rule");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\n## 1. First\n\nBody.\n\n#### 1. Second\n\nBody.\n",
        );
        let config = legacy_fs_folder_config(root.clone());
        let findings = scan_findings(&config, &root);
        let report = check_findings(&findings, &config);

        assert!(
            report
                .errors
                .iter()
                .all(|error| error.code != "section-heading-level"),
            "the depth of a heading the tool is reporting away is not a second finding: {:?}",
            error_codes(&report)
        );
        assert!(
            report.errors.iter().any(|error| error.code == "duplicate-section"),
            "the pair is still reported, at both lines: {:?}",
            error_codes(&report)
        );
    }

    /// §FS-show.2.2.2: the query refuses rather than picking — and rather than
    /// returning the merged body of §DF-duplicate-section-path.1.
    #[test]
    fn show_refuses_the_ambiguous_section_in_every_slice() {
        let (root, config) = duplicated_repo("duplicate_sections_show_refuses", "");
        let findings = scan_findings(&config, &root);

        // §FS-show.2.2.2: `--toc` is exempt only over the *whole* declaration.
        // Selected onto the ambiguous coordinate it is one more slice of it.
        for mode in [
            ShowRenderMode::Default,
            ShowRenderMode::Brief,
            ShowRenderMode::Outline,
            ShowRenderMode::Toc,
            ShowRenderMode::Full,
        ] {
            let Err(err) = show_declaration(&config, &config, &findings, &alpha(), Some("1"), mode, false)
            else {
                panic!("an ambiguous section is refused, never merged into one slice");
            };
            assert_eq!(
                format!("{err:#}"),
                "ambiguous section: FS-001-alpha.1 (declared at \
                 docs/functional-spec/FS-001-alpha.md:5, docs/functional-spec/FS-001-alpha.md:9)",
                "every slice of an ambiguous coordinate refuses, in §FS-show.2.2.1's shape"
            );
        }
    }

    /// §FS-show.2.2.2: only the *requested* path can be ambiguous, and `--toc`
    /// over the whole declaration still maps what is written.
    #[test]
    fn an_untouched_section_still_answers_and_toc_still_lists_both() {
        let (root, config) = duplicated_repo(
            "duplicate_sections_untouched_section",
            "\n## 2. Third\n\nThird body.\n",
        );
        let findings = scan_findings(&config, &root);

        let shown = show_declaration(&config, &config, &findings, &alpha(), Some("2"), ShowRenderMode::Default, false)
            .expect("a path no second heading claims answers normally");
        assert_eq!(shown.body, "## 2. Third\n\nThird body.\n");

        let toc = show_declaration(&config, &config, &findings, &alpha(), None, ShowRenderMode::Outline, false)
            .expect("--toc maps the file as written");
        assert_eq!(
            toc.body, "## 1. First\n## 1. Second\n## 2. Third\n",
            "§FS-show.2.2.2: the map shows the collision rather than refusing"
        );
    }

    /// §FS-workspace.8.1 / §FS-config.3.6: in a workspace the report is rendered
    /// from the workspace root, so the path *inside* the message is spelled from
    /// there too. Member-relative text beside a workspace-relative anchor is a line
    /// no editor can follow to the heading it names.
    #[test]
    fn a_workspace_names_the_other_heading_from_the_workspace_root() {
        let root = test_root("duplicate_sections_workspace_paths");
        write(
            &root.join("apps/api/docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\n## 1. First\n\nBody.\n\n## 1. Second\n\nBody.\n",
        );
        let mut root_config = legacy_fs_folder_config(root.clone());
        root_config.workspace_boundary_roots = vec![canonical_test_path(&root.join("apps/api"))];
        let api_config = legacy_fs_folder_config(root.join("apps/api"));
        let (api_findings, _) =
            scan_tree(&api_config, Some(&api_config.root), true).expect("scan member");
        let workspace = BTreeMap::from([(
            "api".to_string(),
            WorkspaceCheckTarget {
                findings: &api_findings,
                config: &api_config,
            },
        )]);

        let report =
            check_with_workspace(&api_findings, &api_config, &root_config, Some("api"), &workspace);
        assert_eq!(
            located_diagnostics(
                &root_config,
                report.errors.iter().filter(|e| e.code == "duplicate-section")
            ),
            vec![
                "apps/api/docs/functional-spec/FS-001-alpha.md:3: duplicate section \
                 FS-001-alpha.1 (also declared at \
                 apps/api/docs/functional-spec/FS-001-alpha.md:7)"
            ]
        );

        // The `show` twin of the same rule: `path_config` renders the sites.
        let Err(err) = show_declaration(
            &api_config,
            &root_config,
            &api_findings,
            &alpha(),
            Some("1"),
            ShowRenderMode::Default,
            false,
        ) else {
            panic!("the coordinate is ambiguous whichever root it is spelled from");
        };
        assert_eq!(
            format!("{err:#}"),
            "ambiguous section: FS-001-alpha.1 (declared at \
             apps/api/docs/functional-spec/FS-001-alpha.md:3, \
             apps/api/docs/functional-spec/FS-001-alpha.md:7)"
        );
    }
}
