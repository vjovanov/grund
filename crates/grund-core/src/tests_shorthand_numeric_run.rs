/// Test module: the numeric-run rule — a marked shorthand glued to a second
/// number is a numeral, not a citation (§FS-fmt.2.4.1, §FS-check.3.15,
/// §DF-shorthand-numeric-run).
///
/// One module for one rule, across every stage it has to hold at: what `fmt`
/// refuses to rewrite, what `check` says about the site instead, and what the
/// editor's live transform does with a run already on the line. That grouping is
/// the point — the defect this rule fixes was a rewrite and a report agreeing
/// with each other and both being wrong, so the cases that would catch it again
/// have to fail together and be read together.
#[cfg(test)]
mod tests_shorthand_numeric_run {
    use super::tests_support::*;
    use super::*;

    fn check_tree(config: &Config, root: &Path) -> (Findings, CheckReport) {
        let (findings, errors) = scan_tree(config, Some(root), true).expect("scan");
        assert!(errors.is_empty(), "unexpected scan errors: {errors:?}");
        let report = check_findings(&findings, config);
        (findings, report)
    }

    fn messages(report: &CheckReport) -> Vec<String> {
        report
            .errors
            .iter()
            .chain(report.warnings.iter())
            .map(|finding| finding.message.clone())
            .collect()
    }

    // §FS-fmt.2.4.1: the token ends cleanly and still is not a citation. A
    // renumbering table writes the old numbers as a glued run, and expanding the
    // left one attaches today's slug to a number that named something else —
    // producing a well-formed citation of the wrong declaration that `check` can
    // never question. The discriminator is the second number, so no delimiter is
    // enumerated and every glue character behaves alike.
    #[test]
    fn a_shorthand_glued_to_a_second_number_is_never_rewritten() {
        let root = test_root("a_shorthand_glued_to_a_second_number_is_never_rewritten");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n\n## 1. Inputs\n\nStuff.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-043-user-logout.md"),
            "# FS-043-user-logout: User logout\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let expand = |line: &str| {
            let mut saw_candidate = false;
            expand_shorthand_citations(
                line,
                &config,
                true,
                &ShorthandTargets::new(Some(&findings), None),
                &mut saw_candidate,
                &mut Vec::new(),
            )
            .unwrap_or_else(|| line.to_string())
        };
        let unchanged = |line: &str| assert_eq!(expand(line), line, "expected no rewrite");

        // The reported shapes, and the ones nobody has hit yet. A bare number, a
        // kind-qualified one, and a full ID all count as the second element.
        unchanged("Renumbered: §FS-042→FS-043");
        unchanged("Renumbered: §FS-042/043");
        unchanged("Renumbered: §FS-042..043");
        unchanged("Renumbered: §FS-042\u{2026}043");
        unchanged("Renumbered: §FS-042|043");
        unchanged("Renumbered: §FS-042,043");
        unchanged("Renumbered: §FS-042→FS-043-user-logout");
        // A section suffix is part of the token, so the run begins after it.
        unchanged("Renumbered: §FS-042.1→FS-043");
        // Three elements: the head is the marked one and is still a numeral.
        unchanged("Folded: §FS-042/043/044");

        // Whitespace breaks the glue, and the glue is the whole evidence — under
        // the default `number_pattern = "\d+"` a year is number-shaped too.
        assert_eq!(
            expand("Cited §FS-042 (2024) here"),
            "Cited §FS-042-user-login (2024) here"
        );
        assert_eq!(
            expand("Cited §FS-042, 043 here"),
            "Cited §FS-042-user-login, 043 here"
        );
        // A marker on the neighbour says the author marked two citations, one at
        // a time — the clearest statement of intent this grammar offers.
        assert_eq!(
            expand("Both §FS-042, §FS-043 here"),
            "Both §FS-042-user-login, §FS-043-user-logout here"
        );
        // The rule reads forward only: a glued *left* neighbour is what a date or
        // a path looks like, and refusing those costs real citations
        // (§DF-shorthand-numeric-run.2.3).
        assert_eq!(
            expand("Dated 2026-08-19/§FS-042 here"),
            "Dated 2026-08-19/§FS-042-user-login here"
        );
        // A delimiter with no number after it is not a run.
        assert_eq!(expand("End §FS-042."), "End §FS-042-user-login.");
        assert_eq!(expand("Wrapped (§FS-042)"), "Wrapped (§FS-042-user-login)");
        assert_eq!(expand("Trailing §FS-042/"), "Trailing §FS-042-user-login/");

        // One line, both verdicts: the run is left byte-for-byte while the
        // ordinary citation beside it still expands.
        assert_eq!(
            expand("Was §FS-042→FS-043, see §FS-042 now"),
            "Was §FS-042→FS-043, see §FS-042-user-login now"
        );
    }

    // §FS-fmt.2.4.1 clause 1: a bracket or a quote bounds a construct, so the walk
    // for delimiters stops at one. Without that the characters closing the
    // citation's own construct join the ones opening the next, and whatever number
    // the next construct carries reads as the second number of a run — which turns
    // a Markdown link and a footnote reference, two shapes this project writes
    // constantly, into sites `fmt` refuses and `check` reports an unfixable error
    // on. Clause 2 is the other half: the neighbour is matched unqualified, so a
    // path that ends in an ID-shaped segment is not a second number either.
    #[test]
    fn a_construct_boundary_does_not_open_a_run() {
        let root = test_root("a_construct_boundary_does_not_open_a_run");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-043-user-logout.md"),
            "# FS-043-user-logout: User logout\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let expand = |line: &str| {
            let mut saw_candidate = false;
            expand_shorthand_citations(
                line,
                &config,
                true,
                &ShorthandTargets::new(Some(&findings), None),
                &mut saw_candidate,
                &mut Vec::new(),
            )
            .unwrap_or_else(|| line.to_string())
        };

        // The link `fmt --cross-refs` itself writes, with either kind of
        // destination. `](` is a construct boundary, and the destination is a path
        // rather than a second number even where it ends in one.
        assert_eq!(
            expand("See [§FS-042](FS-042-user-login.md) for it"),
            "See [§FS-042-user-login](FS-042-user-login.md) for it"
        );
        assert_eq!(
            expand("See [§FS-042](docs/functional-spec/FS-042-user-login.md)"),
            "See [§FS-042-user-login](docs/functional-spec/FS-042-user-login.md)"
        );
        // A footnote reference, a parenthesised year with no space in front of it,
        // a quoted citation, and a braced count.
        assert_eq!(
            expand("Note §FS-042[^1] here"),
            "Note §FS-042-user-login[^1] here"
        );
        assert_eq!(expand("Cited §FS-042(2024)"), "Cited §FS-042-user-login(2024)");
        assert_eq!(
            expand("Quoted \"§FS-042\"/2 here"),
            "Quoted \"§FS-042-user-login\"/2 here"
        );
        assert_eq!(expand("Braced §FS-042{2}"), "Braced §FS-042-user-login{2}");
        // Clause 2 on its own, with no bracket anywhere: a `<alias>/` namespace
        // precedes a citation and never follows one, so a path glued to the token
        // carries no second number.
        assert_eq!(
            expand("Moved: §FS-042/docs/functional-spec/FS-043-user-logout.md"),
            "Moved: §FS-042-user-login/docs/functional-spec/FS-043-user-logout.md"
        );

        // And the exclusion is about the delimiters, not the surroundings: a real
        // run inside brackets is still a run.
        assert_eq!(
            expand("Renumbered (§FS-042→FS-043) on import"),
            "Renumbered (§FS-042→FS-043) on import"
        );

        // `check` says the same thing about the same sites, which is the agreement
        // this module exists to hold: every one of them is §FS-check.3.13's
        // ordinary shorthand error, naming an edit `fmt` will actually make.
        write(
            &root.join("docs/notes.md"),
            "See [§FS-042](FS-042-user-login.md) and §FS-042[^1].\n\n[^1]: §FS-043\n",
        );
        let (_, report) = check_tree(&config, &root);
        assert_eq!(
            messages(&report),
            vec![
                "shorthand citation §FS-042; write §FS-042-user-login".to_string(),
                "shorthand citation §FS-042; write §FS-042-user-login".to_string(),
                "shorthand citation §FS-043; write §FS-043-user-logout".to_string(),
            ]
        );
    }

    // §FS-check.3.15: a shorthand glued to a second number is a numeral in a run,
    // so `fmt` will not rewrite it and the report says so — naming the canonical
    // form *and* the escape, because only the author knows which was meant. This
    // is §3.13's site with a different verdict, not a second finding on top of it.
    #[test]
    fn a_shorthand_in_a_numeric_run_names_both_exits() {
        let root = test_root("a_shorthand_in_a_numeric_run_names_both_exits");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Renumbered: §FS-042→FS-043, and §FS-042/043 moved.\n\nLive: §FS-042\n",
        );
        let config = numbered_config(root.clone());
        let (findings, report) = check_tree(&config, &root);

        let run = "shorthand §FS-042 sits in a numeric run and was not rewritten; \
                   write §FS-042-user-login, or <§>FS-042 if these are old numbers";
        assert_eq!(
            messages(&report),
            vec![
                run.to_string(),
                run.to_string(),
                "shorthand citation §FS-042; write §FS-042-user-login".to_string(),
            ]
        );
        assert_eq!(report.errors[0].code, "shorthand-numeric-run");
        assert_eq!(report.errors[2].code, "shorthand-citation");

        // §DF-shorthand-numeric-run.2.6: recognition is untouched. All three sites
        // are still edges, so the declaration is not reported uncited — dropping
        // the edge would reintroduce exactly the false negative the shorthand rule
        // was added to end.
        assert!(report.warnings.is_empty(), "{:?}", messages(&report));
        assert_eq!(findings.citations.len(), 3);
        assert!(findings.citations.iter().all(|cite| cite.id.slug.is_some()));
        assert_eq!(
            findings
                .citations
                .iter()
                .map(|cite| cite.numeric_run)
                .collect::<Vec<_>>(),
            vec![true, true, false]
        );
    }

    // §FS-check.3.15: the run verdict replaces only the *mechanical* message. A
    // shorthand in a run that resolves to nothing or to several declarations is a
    // resolution failure, reported on its own terms — a run is no reason to say
    // less about it. And where §FS-fmt.2.3 already forbids every rewrite, the run
    // finding is withheld like §3.13's: an illustration in inline code wants no
    // edit at all.
    #[test]
    fn a_numeric_run_changes_only_the_message_that_names_the_rewrite() {
        let root = test_root("a_numeric_run_changes_only_the_message_that_names_the_rewrite");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-007-a.md"),
            "# FS-007-a: A\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-007-b.md"),
            "# FS-007-b: B\n\nLead.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Unknown: §FS-999→FS-043\n\nAmbiguous: §FS-007→FS-008\n\nShown as `§FS-042→FS-043`.\n",
        );
        let config = numbered_config(root.clone());
        let (_, report) = check_tree(&config, &root);

        assert_eq!(
            report
                .errors
                .iter()
                .map(|finding| finding.message.as_str())
                .collect::<Vec<_>>(),
            vec![
                "shorthand citation §FS-999 matches no declaration",
                "shorthand citation §FS-007 is ambiguous: FS-007-a, FS-007-b",
            ]
        );
        // The inline-code site earns nothing at all, and the ambiguous one
        // resolves nothing, so neither candidate is cited by anything.
        assert_eq!(
            report
                .warnings
                .iter()
                .map(|finding| finding.message.as_str())
                .collect::<Vec<_>>(),
            vec![
                "declared but never cited: FS-007-a",
                "declared but never cited: FS-007-b",
            ]
        );
    }

    // §FS-fmt.2.4.1: the run rule is the bulk pass's, so the editor honours it —
    // a rule that held in CI and not at the keystroke is the drift this whole
    // module exists to prevent.
    //
    // Editing into a line that already carries the run is the case that matters,
    // and it is the case the reported defect came from: a paste or a hand edit,
    // not a fresh sentence. Typing a run left to right is not covered and cannot
    // be — the second number does not exist yet when the keystroke that ends the
    // token fires — but that expansion happens under the author's eyes and undoes
    // with one keystroke, which is the loud failure, not the silent one
    // (§DF-shorthand-numeric-run.5).
    #[test]
    fn on_type_refuses_a_run_already_on_the_line() {
        let root = test_root("on_type_refuses_a_run_already_on_the_line");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{number}-{slug}\"\n",
        );
        let path = root.join("docs/notes.md");
        write(&path, "\n");
        let home = root.join("docs/functional-spec/FS-042-user-login.md");
        let declarations = vec![DeclaredId {
            path: home.as_path(),
            id: "FS-042-user-login",
        }];

        // The cursor sits just past the `→` the author typed; the rest of the run
        // is already to the right of it.
        let edits = |line: &str, cursor: usize| {
            on_type_line_edits(&path, line, 0, cursor, &declarations).expect("on-type edits")
        };
        let run = "Renumbered §FS-042→FS-043 today";
        let cursor = run.find("FS-043").expect("run tail");
        assert!(edits(run, cursor).is_empty(), "expected no expansion in a run");

        // The identical keystroke on a line with no run behind it still expands,
        // so the refusal above is the rule and not a dead transform.
        let plain = "Renumbered §FS-042→ today";
        let cursor = plain.find('\u{2192}').expect("arrow") + '\u{2192}'.len_utf8();
        let applied = edits(plain, cursor);
        assert_eq!(applied.len(), 1, "expected one expansion");
        assert_eq!(applied[0].text, "FS-042-user-login");
    }
}
