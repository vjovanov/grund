/// Test module: grounding per place and per level (§FS-config.3.4.8,
/// §FS-check.3.6) — the two keys on the `[[kinds]]` row against their
/// `[reference]` defaults, which row governs which file, the unit each level
/// cuts, and the obligations that follow the same unit (§FS-check.3.11).
#[cfg(test)]
mod tests_grounding_per_place {
    use super::tests_support::*;
    use super::*;

    /// A repo with three places: an `FS` home, a non-citable `skills/` home, and
    /// a `src/` tree outside every home, which is the homeless kind's complement
    /// (§FS-config.3.9.2). `reference` and `rows` are spelled by the caller,
    /// because what each case is about is where the two keys are written.
    fn repo(name: &str, reference: &str, rows: &str, skill: &str, code: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n\n\
                 [reference]\n{reference}\n\
                 [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
                 [[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\nindex = false\n\n\
                 [[kinds]]\nkind = \"skill\"\nfolder = \"skills\"\ncitable = false\n\
                 title = \"Agent skills\"\n{rows}\n\
                 [scan]\ninclude = [\"docs\", \"src\"]\n"
            ),
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(
            &root.join("docs/architecture/AR-001-bus.md"),
            "# AR-001-bus: The bus\n\nBody.\n",
        );
        write(&root.join("skills/review/SKILL.md"), skill);
        write(&root.join("src/main.rs"), code);
        root
    }

    /// Every ungrounded finding as `path:line: message`, so a case asserts on
    /// which units fired and where, not only on how many did.
    fn ungrounded(run: &CheckRun) -> Vec<String> {
        findings(run)
            .into_iter()
            .filter(|line| line.contains("ungrounded"))
            .collect()
    }

    const UNCITED_SKILL: &str = "# Review skill\n\nNothing here.\n";
    const UNCITED_CODE: &str = "fn main() {}\n";

    /// §FS-config.3.4.8: the row turns grounding on for its own home and for
    /// nothing else — the whole point of moving the key off `[reference]`.
    #[test]
    fn a_row_grounds_its_own_home_with_the_global_off() {
        let run = check_run(
            &repo(
                "a_row_grounds_its_own_home_with_the_global_off",
                "",
                "require_grounding = true\n",
                UNCITED_SKILL,
                UNCITED_CODE,
            ),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec![
                "skills/review/SKILL.md:1: ungrounded file in kind home skills/: no § citation to a declared ID"
            ],
        );
    }

    /// §FS-config.3.4.8: precedence is row > global, so an explicit `false`
    /// exempts one home while every other place stays grounded.
    #[test]
    fn a_row_false_exempts_its_home_under_a_global_true() {
        let run = check_run(
            &repo(
                "a_row_false_exempts_its_home_under_a_global_true",
                "require_grounding = true\n",
                "require_grounding = false\n",
                UNCITED_SKILL,
                UNCITED_CODE,
            ),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec!["src/main.rs:1: ungrounded source file: no § citation to a declared ID"],
        );
    }

    /// §FS-check.1: the flag and the global key are one knob, so the flag sets
    /// the same default and the row's more specific word still wins.
    #[test]
    fn the_flag_does_not_override_an_explicit_row_false() {
        let root = repo(
            "the_flag_does_not_override_an_explicit_row_false",
            "",
            "require_grounding = false\n",
            UNCITED_SKILL,
            UNCITED_CODE,
        );
        let run = run_check(&root, true, true, false).expect("check run");
        assert_eq!(
            ungrounded(&run),
            vec!["src/main.rs:1: ungrounded source file: no § citation to a declared ID"],
            "the flag reaches the complement it always did, and not the exempt row",
        );
    }

    /// §FS-check.3.6.2: at level 2 every `##` subtree is a unit, and the file
    /// stays one — satisfied here by the citation before the first heading.
    #[test]
    fn level_two_asks_each_section_of_a_markdown_home() {
        let run = check_run(
            &repo(
                "level_two_asks_each_section_of_a_markdown_home",
                "",
                "require_grounding = true\ngrounding_level = 2\n",
                "# Review skill\n\nGrounded in §FS-001-login.\n\n\
                 ## Steps\n\nNothing here.\n\n\
                 ## Notes\n\nAlso §FS-001-login.\n",
                UNCITED_CODE,
            ),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec![
                "skills/review/SKILL.md:5: ungrounded section `## Steps` in kind home skills/: no § citation to a declared ID"
            ],
        );
    }

    /// §FS-check.3.6.2: a parent is satisfied by any descendant, so a cited
    /// `###` clears the `##` it sits under.
    #[test]
    fn level_three_lets_a_leaf_satisfy_its_parent() {
        let run = check_run(
            &repo(
                "level_three_lets_a_leaf_satisfy_its_parent",
                "",
                "require_grounding = true\ngrounding_level = 3\n",
                "# Review skill\n\nGrounded in §FS-001-login.\n\n\
                 ## Steps\n\n### First\n\nSee §FS-001-login.\n",
                UNCITED_CODE,
            ),
            false,
        );
        assert!(ungrounded(&run).is_empty(), "{:?}", ungrounded(&run));
    }

    /// §FS-check.3.6.2: nothing passes vacuously for lacking structure — a file
    /// with no heading at the level is one unit, the file.
    #[test]
    fn a_file_with_no_section_at_the_level_is_one_unit() {
        let run = check_run(
            &repo(
                "a_file_with_no_section_at_the_level_is_one_unit",
                "",
                "require_grounding = true\ngrounding_level = 2\n",
                UNCITED_SKILL,
                UNCITED_CODE,
            ),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec![
                "skills/review/SKILL.md:1: ungrounded file in kind home skills/: no § citation to a declared ID"
            ],
        );
    }

    /// A source tree at a level, spelled on the homeless row (§FS-config.3.9.2).
    /// The module doc grounds the file; one top-level block and one indented
    /// block are what the two ranks of §FS-check.3.6.2 tell apart.
    fn source_repo(name: &str, level: usize) -> PathBuf {
        repo(
            name,
            "",
            &format!(
                "\n[[kinds]]\nkind = \"code\"\ncitable = false\n\
                 require_grounding = true\ngrounding_level = {level}\n"
            ),
            "# Review skill\n\nNothing here.\n",
            "//! Module doc, grounded in §FS-001-login.\n\n\
             /// Ungrounded top-level item.\n\
             fn alpha() {\n\
             \x20   /// An indented block.\n\
             \x20   fn inner() {}\n\
             }\n",
        )
    }

    /// §FS-check.3.6.2: level 2 reaches the *unindented* doc-comment blocks —
    /// the parse-free stand-in for a top-level item (§FS-non-goals.3).
    #[test]
    fn level_two_reaches_only_unindented_doc_comments() {
        let run = check_run(
            &source_repo("level_two_reaches_only_unindented_doc_comments", 2),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec!["src/main.rs:3: ungrounded doc-comment: no § citation to a declared ID"],
        );
    }

    /// §FS-check.3.6.2: any higher level reaches every doc-comment block.
    #[test]
    fn level_three_reaches_every_doc_comment() {
        let run = check_run(&source_repo("level_three_reaches_every_doc_comment", 3), false);
        assert_eq!(
            ungrounded(&run),
            vec![
                "src/main.rs:3: ungrounded doc-comment: no § citation to a declared ID",
                "src/main.rs:5: ungrounded doc-comment: no § citation to a declared ID",
            ],
        );
    }

    /// §FS-check.3.11: the obligation unit follows the row's level, so `must`
    /// fires on the section that cites something else rather than on the file
    /// that already cites an `FS` somewhere.
    #[test]
    fn an_obligation_fires_per_section_at_level_two() {
        let root = repo(
            "an_obligation_fires_per_section_at_level_two",
            "",
            "require_grounding = true\ngrounding_level = 2\n",
            "# Review skill\n\nSee §FS-001-login.\n\n## Steps\n\nSee §AR-001-bus.\n",
            UNCITED_CODE,
        );
        let config_path = root.join("grund.toml");
        let config = std::fs::read_to_string(&config_path).expect("read config");
        write(
            &config_path,
            &format!("{config}\n[citations]\n[citations.skill]\nmust = [\"FS\"]\n"),
        );
        let run = check_run(&root, false);
        assert_eq!(
            findings(&run)
                .into_iter()
                .filter(|line| line.contains("must cite"))
                .collect::<Vec<_>>(),
            vec!["skills/review/SKILL.md:5: skills/ must cite FS (citation direction)"],
        );
    }

    /// §FS-check.3.11: at level 2 an unmet obligation reports on the file unit
    /// *and* on each failing section — the shape §FS-check.3.6.3 states for
    /// grounding, for the same reason: the file cites no `FS`, and neither does
    /// the section under it.
    #[test]
    fn an_unmet_obligation_reports_on_the_file_and_the_section() {
        let root = repo(
            "an_unmet_obligation_reports_on_the_file_and_the_section",
            "",
            "require_grounding = true\ngrounding_level = 2\n",
            "# Review skill\n\n## Steps\n\nSee §AR-001-bus.\n",
            UNCITED_CODE,
        );
        let config_path = root.join("grund.toml");
        let config = std::fs::read_to_string(&config_path).expect("read config");
        write(
            &config_path,
            &format!("{config}\n[citations]\n[citations.skill]\nmust = [\"FS\"]\n"),
        );
        let run = check_run(&root, false);
        assert_eq!(
            findings(&run)
                .into_iter()
                .filter(|line| line.contains("must cite"))
                .collect::<Vec<_>>(),
            vec![
                "skills/review/SKILL.md:1: skills/ must cite FS (citation direction)",
                "skills/review/SKILL.md:3: skills/ must cite FS (citation direction)",
            ],
        );
    }

    /// The §FS-check.2.2.1 warning this run earns, with `[citations.skill] must`
    /// appended to the fixture's config — the shape that makes a home with real
    /// content produce no obligation unit.
    fn no_unit_warning(name: &str, rows: &str) -> String {
        let root = repo(name, "", rows, UNCITED_SKILL, UNCITED_CODE);
        let config_path = root.join("grund.toml");
        let config = std::fs::read_to_string(&config_path).expect("read config");
        write(
            &config_path,
            &format!("{config}\n[citations]\n[citations.skill]\nmust = [\"FS\"]\n"),
        );
        check_run(&root, false)
            .report
            .warnings
            .iter()
            .find(|warning| warning.code == "empty-citation-obligation")
            .map(|warning| warning.message.clone())
            .expect("the no-unit warning")
    }

    /// §FS-check.2.2.1: the row-key half of the message is advice, so it is given
    /// only where it is still advice — a row that already grounds has made that
    /// setting, and this run is already reporting what it caught (§FS-check.3.6).
    #[test]
    fn the_no_unit_warning_asks_for_grounding_only_where_it_is_off() {
        assert_eq!(
            no_unit_warning(
                "the_no_unit_warning_asks_for_grounding_only_where_it_is_off_on",
                "require_grounding = true\n",
            ),
            "[citations.skill] must applies to nothing — no scanned file in skills/ carries a citation",
        );
        assert_eq!(
            no_unit_warning(
                "the_no_unit_warning_asks_for_grounding_only_where_it_is_off_off",
                "",
            ),
            "[citations.skill] must applies to nothing — no scanned file in skills/ carries a citation; set require_grounding = true on the skills/ row to make that an error",
        );
    }

    /// §FS-check.3.6.2: the inline-declaration escape is a source file's and has
    /// no effect in a non-citable home, where the declaration is misplaced to
    /// begin with (§FS-check.3.7) — so the file earns both findings.
    #[test]
    fn an_inline_declaration_does_not_ground_a_non_citable_home() {
        let root = repo(
            "an_inline_declaration_does_not_ground_a_non_citable_home",
            "",
            "require_grounding = true\n",
            UNCITED_SKILL,
            UNCITED_CODE,
        );
        write(
            &root.join("skills/review/helper.rs"),
            "/// AR-002-helper: A helper\n///\n/// Body.\nfn helper() {}\n",
        );
        let run = check_run(&root, false);
        assert!(
            ungrounded(&run).iter().any(|line| line
                .starts_with("skills/review/helper.rs:1: ungrounded file in kind home skills/")),
            "{:?}",
            findings(&run)
        );
    }

    /// A repo whose only non-citable home is a *single file* (§FS-config.3.4):
    /// the shape §FS-check.3.6.1 governs like any other home, and the one the
    /// keys were rejected on before they meant anything there.
    fn runbook_repo(name: &str, row: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n\n\
                 [reference]\nrequire_grounding = true\n\n\
                 [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
                 [[kinds]]\nkind = \"runbook\"\nfile = \"RUNBOOK.md\"\ncitable = false\n{row}"
            ),
        );
        write(
            &root.join("docs/specs/FS-001-login.md"),
            "# FS-001-login: A user logs in\n\nBody.\n",
        );
        write(
            &root.join("RUNBOOK.md"),
            "# Runbook\n\nNothing here.\n\n## Restart\n\nNothing here either.\n",
        );
        root
    }

    /// §FS-check.3.6.1: a non-citable `file` home is a home, so its one document
    /// is governed — and §FS-check.3.6.2 cuts it into heading subtrees at level
    /// 2 exactly as it cuts a folder home's files.
    #[test]
    fn a_non_citable_file_home_is_governed_and_cut_by_its_level() {
        let run = check_run(
            &runbook_repo(
                "a_non_citable_file_home_is_governed_and_cut_by_its_level",
                "grounding_level = 2\n",
            ),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec![
                "RUNBOOK.md:1: ungrounded file in kind home RUNBOOK.md: no § citation to a declared ID",
                "RUNBOOK.md:5: ungrounded section `## Restart` in kind home RUNBOOK.md: no § citation to a declared ID",
            ],
        );
    }

    /// §FS-config.3.4.8: and the row's own `require_grounding` exempts it under a
    /// global `true` — the per-place switch the shape most in need of it could
    /// not write before.
    #[test]
    fn a_non_citable_file_row_can_exempt_its_own_document() {
        let run = check_run(
            &runbook_repo(
                "a_non_citable_file_row_can_exempt_its_own_document",
                "require_grounding = false\n",
            ),
            false,
        );
        assert!(ungrounded(&run).is_empty(), "{:?}", ungrounded(&run));
    }

    /// §FS-check.3.6.3: every failing unit is reported — a file that cites
    /// nothing at level 2 earns the file finding *and* one per section, which is
    /// what "each level contains the one below it" means on the reporting side.
    #[test]
    fn an_uncited_file_earns_the_file_finding_and_one_per_section() {
        let run = check_run(
            &repo(
                "an_uncited_file_earns_the_file_finding_and_one_per_section",
                "",
                "require_grounding = true\ngrounding_level = 2\n",
                "# Review skill\n\nNothing here.\n\n## Steps\n\nNothing.\n\n## Notes\n\nNothing.\n",
                UNCITED_CODE,
            ),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec![
                "skills/review/SKILL.md:1: ungrounded file in kind home skills/: no § citation to a declared ID",
                "skills/review/SKILL.md:5: ungrounded section `## Steps` in kind home skills/: no § citation to a declared ID",
                "skills/review/SKILL.md:9: ungrounded section `## Notes` in kind home skills/: no § citation to a declared ID",
            ],
        );
    }

    /// §AR-scanner.2.7: the structure pass is asked per file, not per project —
    /// a level-2 row describes its own home's files and leaves every other
    /// place's alone (§GOAL-fast-feedback).
    #[test]
    fn structure_is_recorded_only_for_the_row_that_asks_for_it() {
        let root = repo(
            "structure_is_recorded_only_for_the_row_that_asks_for_it",
            "",
            "require_grounding = true\ngrounding_level = 2\n",
            UNCITED_SKILL,
            UNCITED_CODE,
        );
        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let described: Vec<String> = findings
            .file_structure
            .keys()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(described, vec!["SKILL.md".to_string()]);
    }

    /// §FS-config.3.4.8 / §REQ-backwards-compatibility.1: a config that writes
    /// only the global key behaves exactly as it did — every row inherits it,
    /// every level is the file.
    #[test]
    fn a_global_only_config_grounds_every_place_at_the_file() {
        let run = check_run(
            &repo(
                "a_global_only_config_grounds_every_place_at_the_file",
                "require_grounding = true\n",
                "",
                UNCITED_SKILL,
                UNCITED_CODE,
            ),
            false,
        );
        assert_eq!(
            ungrounded(&run),
            vec![
                "skills/review/SKILL.md:1: ungrounded file in kind home skills/: no § citation to a declared ID",
                "src/main.rs:1: ungrounded source file: no § citation to a declared ID",
            ],
        );
    }

    /// §FS-config.4.2: each key prints on a row only where its effective value
    /// differs from the effective global, and the printed config loads back to
    /// the same effective values.
    #[test]
    fn config_show_prints_a_row_key_only_where_it_differs() {
        let root = repo(
            "config_show_prints_a_row_key_only_where_it_differs",
            "require_grounding = true\n",
            "grounding_level = 2\n",
            UNCITED_SKILL,
            UNCITED_CODE,
        );
        let config = load_config(&root).expect("load config");
        let lines = |name: &str| {
            config.kind_grounding_toml_lines(
                config.kinds.iter().find(|kind| kind.kind == name).unwrap(),
            )
        };
        assert_eq!(lines("FS"), Vec::<String>::new(), "inherits both");
        assert_eq!(lines("skill"), vec!["grounding_level = 2".to_string()]);
        assert!(config.grounding_enabled(), "the [reference] level prints");
    }

    /// §FS-config.3.4.8: a row turning grounding on is enough — the global level
    /// is then a default some place actually reads.
    #[test]
    fn a_global_level_loads_when_a_row_turns_grounding_on() {
        let root = repo(
            "a_global_level_loads_when_a_row_turns_grounding_on",
            "grounding_level = 2\n",
            "require_grounding = true\n",
            UNCITED_SKILL,
            UNCITED_CODE,
        );
        let run = check_run(&root, false);
        assert_eq!(
            ungrounded(&run),
            vec![
                "skills/review/SKILL.md:1: ungrounded file in kind home skills/: no § citation to a declared ID"
            ],
        );
    }
}
