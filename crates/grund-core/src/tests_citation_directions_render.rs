/// Test module: the generated `### Citation directions` section (§FS-init.2.3.5),
/// one case per row of the table §DF-directions-render pins. The rules are what
/// an agent reads instead of `grund.toml`, so these cases hold the *wording* —
/// the unit each bullet names, the grouping, the prose forms of rule grammar,
/// the legend, the grounding sentence, and the closing line.
#[cfg(test)]
mod tests_citation_directions_render {
    use super::tests_support::*;
    use super::*;

    /// The kinds most cases share: three citable homes, a single-file citable
    /// kind, and one non-citable home. A case that needs another row, another
    /// order, or no non-citable home at all writes its own table with
    /// [`render`].
    const KINDS: &str = r#"[[kinds]]
kind = "FS"
folder = "docs/functional-spec"
[[kinds]]
kind = "GOAL"
file = "docs/goals.md"
[[kinds]]
kind = "AR"
folder = "docs/architecture"
[[kinds]]
kind = "RM"
file = "docs/roadmap.md"
[[kinds]]
kind = "skill"
folder = "skills"
citable = false
"#;

    /// Render the section for a config written into a fresh test root. The homes
    /// named in a config need not exist: the render reads `[[kinds]]` and
    /// `[citations]`, never the tree.
    fn render(name: &str, config: &str) -> String {
        let root = test_root(name);
        write(&root.join(".agents/grund.toml"), config);
        citation_directions_section(&load_config(&root).expect("load config"))
    }

    /// [`render`] over the shared kinds table plus one case's own rules.
    fn render_shared(name: &str, rules: &str) -> String {
        render(name, &format!("{KINDS}{rules}"))
    }

    /// §FS-init.2.3.5: three units sit under one verb — a declaration, a file in
    /// a home, and a source file that already cites something — so every bullet
    /// says which one it is.
    #[test]
    fn every_bullet_names_the_unit_its_rules_are_checked_per() {
        let section = render_shared(
            "every_bullet_names_the_unit_its_rules_are_checked_per",
            r#"[[kinds]]
kind = "runbook"
file = "docs/runbook.md"
citable = false
[citations]
[citations.FS]
should = ["FS"]
[citations.skill]
must = ["FS"]
[citations.runbook]
must = ["FS"]
[citations.code]
should = ["FS"]
"#,
        );
        assert!(section.contains("- Each **FS** declaration should cite FS."), "{section}");
        assert!(section.contains("- Each file in **skills/** must cite FS."), "{section}");
        // A single-file home is one file, so "each file in" would promise a
        // directory that is not there.
        assert!(section.contains("- The file **docs/runbook.md** must cite FS."), "{section}");
        assert!(
            section.contains(
                "- Each source file outside the Project map (**code**) that cites anything should cite FS."
            ),
            "{section}"
        );
    }

    /// §FS-config.3.9.2: the homeless kind's `title` says what it covers, and its
    /// row closes the list wherever the table declared it.
    #[test]
    fn the_homeless_bullet_carries_its_title_and_renders_last() {
        let section = render(
            "the_homeless_bullet_carries_its_title_and_renders_last",
            r#"[[kinds]]
kind = "code"
citable = false
title = "Gradle build and workflows"
[[kinds]]
kind = "FS"
folder = "docs/functional-spec"
[citations]
[citations.code]
should = ["FS"]
[citations.FS]
should = ["FS"]
"#,
        );
        assert!(
            section.contains(
                "- Each source file outside the Project map (**code**: Gradle build and workflows) that cites anything should cite FS."
            ),
            "{section}"
        );
        let code = section.find("(**code**").expect("the code row");
        let fs = section.find("Each **FS** declaration").expect("the FS row");
        assert!(fs < code, "the homeless kind closes the list: {section}");
    }

    /// §FS-init.2.3.5: `must = ["FS|GOAL", "AR"]` is *(FS or GOAL) and AR*, and
    /// the ungrouped prose said the opposite. A conjunction of singletons needs
    /// no parentheses, and one entry alone is never parenthesised.
    #[test]
    fn a_conjunction_of_alternatives_is_grouped() {
        let section = render_shared(
            "a_conjunction_of_alternatives_is_grouped",
            r#"[[kinds]]
kind = "DA"
folder = "docs/decisions/architectural"
[citations]
[citations.DA]
must = ["FS|GOAL", "AR"]
[citations.AR]
must = ["FS", "GOAL"]
[citations.FS]
should = ["FS|GOAL"]
"#,
        );
        assert!(section.contains("must cite (FS or GOAL) and AR."), "{section}");
        assert!(section.contains("must cite FS and GOAL."), "{section}");
        assert!(section.contains("should cite FS or GOAL."), "{section}");
    }

    /// §FS-init.2.3.5: three alternatives take the Oxford comma, so a three-way
    /// rule cannot be read as a two-way one.
    #[test]
    fn three_alternatives_take_the_oxford_comma() {
        let section = render_shared(
            "three_alternatives_take_the_oxford_comma",
            "[citations]\n[citations.skill]\nmust = [\"FS|AR|RM\"]\n",
        );
        assert!(section.contains("must cite FS, AR, or RM."), "{section}");
    }

    /// §FS-init.2.3.5: a pinned alias is spelled the way a citation is spelled;
    /// `*/K` is rule grammar that is never a citation (§FS-config.3.9.3), so it
    /// is said in words instead of leaked into the entrypoint.
    #[test]
    fn a_pinned_alias_stays_as_spelled_and_any_namespace_becomes_prose() {
        let section = render_shared(
            "a_pinned_alias_stays_as_spelled_and_any_namespace_becomes_prose",
            r#"[citations]
[citations.FS]
must = ["FS"]
must-not = ["api/AR"]
[citations.skill]
must = ["FS"]
should-not = ["*/AR"]
"#,
        );
        assert!(section.contains("never cite api/AR"), "{section}");
        assert!(section.contains("avoid citing AR in any project."), "{section}");
        assert!(!section.contains("*/AR"), "rule grammar must not reach the prose: {section}");
    }

    /// §FS-init.2.3.5: a closed per-kind default plus a `may` list is one rule,
    /// "only these", and takes one clause rather than a permission followed by a
    /// prohibition of everything else.
    #[test]
    fn a_closed_per_kind_default_folds_into_its_permission() {
        let section = render_shared(
            "a_closed_per_kind_default_folds_into_its_permission",
            r#"[citations]
[citations.AR]
default = "must-not"
may = ["FS|GOAL"]
"#,
        );
        assert!(section.contains("- Each **AR** declaration may cite only FS or GOAL."), "{section}");
        assert!(!section.contains("unlisted citations"), "one clause, not two: {section}");
    }

    /// §DF-directions-render.2.5: with a `must` beside it the permitted set is
    /// wider than the `may` list, so "only" would name the wrong set and the
    /// bullet ends with the explicit closing clause instead.
    #[test]
    fn an_obligation_beside_a_closed_default_blocks_the_fold() {
        let section = render_shared(
            "an_obligation_beside_a_closed_default_blocks_the_fold",
            r#"[citations]
[citations.AR]
default = "must-not"
must = ["FS"]
may = ["GOAL"]
"#,
        );
        assert!(
            section.contains("- Each **AR** declaration must cite FS; may cite GOAL; never cite anything else."),
            "{section}"
        );
    }

    /// §FS-init.2.3.5: a `should-not` per-kind default discourages the rest; a
    /// per-kind default with no list at all has nothing to be "else" to.
    #[test]
    fn a_discouraging_default_and_a_default_with_no_lists() {
        let section = render_shared(
            "a_discouraging_default_and_a_default_with_no_lists",
            r#"[citations]
[citations.FS]
default = "should-not"
must = ["AR"]
[citations.AR]
default = "must-not"
"#,
        );
        assert!(
            section.contains("- Each **FS** declaration must cite AR; avoid citing anything else."),
            "{section}"
        );
        assert!(
            section.contains("- Each **AR** declaration must not cite anything."),
            "{section}"
        );
    }

    /// §DF-directions-render.2.4: a per-kind default that leaves its kind open
    /// says so only where the global default is closed and the kind is therefore
    /// a hole in it.
    #[test]
    fn an_open_per_kind_default_speaks_only_under_a_closed_global_one() {
        let config = |global: &str| {
            format!("[citations]\ndefault = \"{global}\"\n[citations.FS]\ndefault = \"may\"\nmust = [\"AR\"]\n")
        };
        let closed = render_shared(
            "an_open_per_kind_default_speaks_only_under_a_closed_global_one_closed",
            &config("must-not"),
        );
        assert!(
            closed.contains("- Each **FS** declaration must cite AR; may cite anything else."),
            "{closed}"
        );
        let open = render_shared(
            "an_open_per_kind_default_speaks_only_under_a_closed_global_one_open",
            &config("may"),
        );
        assert!(open.contains("- Each **FS** declaration must cite AR."), "{open}");
    }

    /// §DF-directions-render.2.6: the subject is a noun phrase, so a leading
    /// prohibition takes the modal §FS-check.3.12 uses and a following one keeps
    /// the short form the legend names.
    #[test]
    fn a_leading_prohibition_takes_the_modal() {
        let section = render_shared(
            "a_leading_prohibition_takes_the_modal",
            r#"[citations]
[citations.FS]
must-not = ["AR"]
should-not = ["GOAL"]
[citations.AR]
should = ["FS"]
must-not = ["GOAL"]
"#,
        );
        assert!(
            section.contains("- Each **FS** declaration must not cite AR; avoid citing GOAL."),
            "{section}"
        );
        assert!(
            section.contains("- Each **AR** declaration should cite FS; never cite GOAL."),
            "{section}"
        );
    }

    /// §DF-directions-render.2.4: the closing line reports the *global* default
    /// alone, and only `must-not` / `should-not` close anything — a `must` or
    /// `should` default invents no obligation and forbids nothing
    /// (§FS-config.3.9.4).
    #[test]
    fn the_closing_line_reports_the_global_default() {
        let config = |global: &str| {
            format!("[citations]\ndefault = \"{global}\"\n[citations.FS]\nshould = [\"FS\"]\n")
        };
        for global in ["may", "must", "should"] {
            let section = render_shared(
                &format!("the_closing_line_reports_the_global_default_{global}"),
                &config(global),
            );
            assert!(
                section.trim_end().ends_with("Anything not listed above is allowed."),
                "`{global}` closes as open: {section}"
            );
            assert!(!section.contains("By default,"), "no top default sentence: {section}");
        }
        let forbidden = render_shared(
            "the_closing_line_reports_the_global_default_must_not",
            &config("must-not"),
        );
        assert!(
            forbidden.trim_end().ends_with("Any citation not listed above is forbidden."),
            "{forbidden}"
        );
        let discouraged = render_shared(
            "the_closing_line_reports_the_global_default_should_not",
            &config("should-not"),
        );
        assert!(
            discouraged.trim_end().ends_with("Any citation not listed above is discouraged."),
            "{discouraged}"
        );
    }

    /// §FS-init.2.3.5: the opening paragraph is the legend plus the grounding
    /// sentence. The sentence distinguishes citing from declaring (§FS-check.3.6)
    /// and names the non-citable homes, whose files can only cite
    /// (§FS-check.3.7); an unwalked home is left out because nothing in it is
    /// scanned.
    #[test]
    fn the_legend_and_the_grounding_sentence_open_the_section() {
        let section = render_shared(
            "the_legend_and_the_grounding_sentence_open_the_section",
            r#"[[kinds]]
kind = "e2e"
folder = "tests/e2e"
citable = false
[[kinds]]
kind = "template"
folder = "templates"
citable = false
scan = false
[reference]
require_grounding = true
[citations]
[citations.FS]
should = ["FS"]
"#,
        );
        assert!(
            section.contains(
                "`must`/`never` are `grund check` errors; `should`/`avoid` are suggestions (`grund check --suggestions`). Every source file must cite a declared ID or declare one inline; every file under skills/ and tests/e2e/ must cite one."
            ),
            "{section}"
        );
        assert!(!section.contains("templates/"), "an unwalked home is never scanned: {section}");
    }

    /// §FS-init.2.3.5: with no walked non-citable home there is no second clause,
    /// and with the key off there is no grounding sentence at all.
    #[test]
    fn grounding_shrinks_to_the_source_file_rule_and_disappears_when_off() {
        let config = |reference: &str| {
            format!("[[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\n{reference}[citations]\n[citations.FS]\nshould = [\"FS\"]\n")
        };
        let homeless_only = render(
            "grounding_shrinks_to_the_source_file_rule",
            &config("[reference]\nrequire_grounding = true\n"),
        );
        assert!(
            homeless_only
                .contains("Every source file must cite a declared ID or declare one inline.\n"),
            "{homeless_only}"
        );
        let off = render("grounding_disappears_when_off", &config(""));
        assert!(!off.contains("must cite a declared ID"), "{off}");
        assert!(off.contains(CITATION_LEVEL_LEGEND), "the legend stands alone: {off}");
    }

    /// §FS-init.2.3.4.10: whether a file must ground at all is not a direction
    /// rule, so the sentence does not wait on `[citations]` being declared —
    /// which is the config this repository's own defect was reported against.
    #[test]
    fn grounding_renders_without_a_citations_section() {
        let section = render(
            "grounding_renders_without_a_citations_section",
            "[reference]\nrequire_grounding = true\n[[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\n",
        );
        assert!(section.contains("Specs cite goals, architecture cites specs"), "{section}");
        assert!(
            section.ends_with(
                "Every source file must cite a declared ID or declare one inline."
            ),
            "{section}"
        );
    }

    /// §FS-init.2.3.5: a kind with no rule has no bullet, and the section never
    /// ends with a newline — the template's placeholder supplies the one that
    /// keeps `grund init` idempotent.
    #[test]
    fn a_kind_without_rules_has_no_bullet() {
        let section = render_shared(
            "a_kind_without_rules_has_no_bullet",
            "[citations]\n[citations.FS]\nshould = [\"RM\"]\n",
        );
        assert!(!section.contains("**RM**"), "cited, never citing: {section}");
        assert!(!section.ends_with('\n'), "{section}");
    }

    /// §FS-init.2.3.5: the documented config/render pair must stay an exact
    /// example of the production Citation directions renderer. The extractor
    /// follows the shared fence state machine so another fence style or a
    /// shorter closing run cannot make the test compare the wrong block.
    #[test]
    fn documented_citation_directions_example_matches_production_render() {
        let page_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/user-facing/citation-directions.md");
        let page = std::fs::read_to_string(&page_path).expect("read canonical page");
        let documented_config = fenced_example(&page, "toml");
        let documented_render = fenced_example(&page, "markdown");

        let root = test_root("documented_citation_directions_example_matches_production_render");
        // The page intentionally shows only the citation tables. Add the
        // three homes needed to give those tables their configured meanings;
        // the citation fragment itself is read verbatim from the page above.
        write(
            &root.join(".agents/grund.toml"),
            &format!(
                "[[kinds]]\nkind = \"GOAL\"\nfile = \"docs/goals.md\"\n\n[[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\n\n[[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\n\n[[kinds]]\nkind = \"DA\"\nfolder = \"docs/decisions/architectural\"\n\n[[kinds]]\nkind = \"skill\"\nfolder = \"skills\"\ncitable = false\n\n{documented_config}"
            ),
        );
        let config = load_config(&root).expect("load documented config");
        let rendered = citation_directions_section(&config);
        let expected = documented_render
            .strip_suffix('\n')
            .expect("render example ends with a newline");
        assert_eq!(rendered, expected);
    }

    fn fenced_example(page: &str, language: &str) -> String {
        let mut open = None;
        let mut target = false;
        let mut body = String::new();
        for raw_line in page.split_inclusive('\n') {
            let line = raw_line
                .strip_suffix('\n')
                .unwrap_or(raw_line)
                .strip_suffix('\r')
                .unwrap_or_else(|| raw_line.strip_suffix('\n').unwrap_or(raw_line));
            let was_open = open.is_some();
            let delimiter = markdown_fence_delimiter(&mut open, line);
            if !was_open && delimiter {
                target = fence_language(line) == Some(language);
                continue;
            }
            if was_open && delimiter {
                if target {
                    return body;
                }
                target = false;
                continue;
            }
            if target {
                body.push_str(raw_line);
            }
        }
        panic!("missing fenced {language} example");
    }

    fn fence_language(line: &str) -> Option<&str> {
        let bytes = line.as_bytes();
        let indent = bytes.iter().take_while(|byte| **byte == b' ').count();
        if indent > 3 || indent == bytes.len() {
            return None;
        }
        let delimiter = bytes[indent];
        if delimiter != b'`' && delimiter != b'~' {
            return None;
        }
        let run = bytes[indent..]
            .iter()
            .take_while(|byte| **byte == delimiter)
            .count();
        (run >= 3).then(|| line[indent + run..].trim())
    }
}
