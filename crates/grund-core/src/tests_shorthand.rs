/// Test module: the number-only citation shorthand — recognition, resolution,
/// the check error, bulk normalization, and the query surfaces
/// (§FS-check.1.2, §FS-check.3.13, §FS-fmt.2.4, §DF-number-only-citation-shorthand).
#[cfg(test)]
mod tests_shorthand {
    use super::tests_support::*;
    use super::*;

    fn messages(report: &CheckReport) -> Vec<String> {
        report
            .errors
            .iter()
            .chain(report.warnings.iter())
            .map(|finding| finding.message.clone())
            .collect()
    }

    fn check_tree(config: &Config, root: &Path) -> (Findings, CheckReport) {
        let (findings, errors) = scan_tree(config, Some(root), true).expect("scan");
        assert!(errors.is_empty(), "unexpected scan errors: {errors:?}");
        let report = check_findings(&findings, config);
        (findings, report)
    }

    // §FS-check.3.13: a shorthand that names exactly one declaration is reported
    // once, with the canonical form to write — and, per §FS-check.1.2, it still
    // counts as a citation everywhere else. The uncited warning firing here was
    // the original defect: `check` said "declared but never cited" about a
    // declaration this file cites twice.
    #[test]
    fn resolvable_shorthand_reports_once_and_counts_as_a_citation() {
        let root = test_root("resolvable_shorthand_reports_once_and_counts_as_a_citation");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n\n## 1. Inputs\n\nStuff.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Bare ID: §FS-042\n\nWith a section: §FS-042.1\n",
        );
        let config = numbered_config(root.clone());
        let (findings, report) = check_tree(&config, &root);

        assert_eq!(
            messages(&report),
            vec![
                "shorthand citation §FS-042; write §FS-042-user-login",
                "shorthand citation §FS-042.1; write §FS-042-user-login.1",
            ]
        );
        // Exactly one finding per site: no `unknown reference`, no `unused`.
        assert!(report.warnings.is_empty(), "{:?}", messages(&report));

        // Both sites resolved to the canonical ID, so every graph consumer sees
        // a normal citation (§DF-number-only-citation-shorthand.2.8).
        let declared = Id {
            kind: "FS".into(),
            num: Some(42),
            slug: Some("user-login".into()),
        };
        assert_eq!(findings.citations.len(), 2);
        for cite in &findings.citations {
            assert!(cite.shorthand, "{:?}", cite.text);
            assert_eq!(cite.id, declared);
        }
        // The written token is preserved, which is what keeps editor ranges and
        // report columns pointing at what the author actually typed.
        assert_eq!(findings.citations[0].text, "§FS-042");
        assert_eq!(findings.citations[0].column, 10);
        assert_eq!(findings.citations[1].text, "§FS-042.1");
        assert_eq!(findings.citations[1].section.as_deref(), Some("1"));
    }

    // §FS-check.3.13: the unknown and ambiguous outcomes. Neither resolves, and
    // neither is guessed at (§DF-number-only-citation-shorthand.2.7).
    #[test]
    fn unknown_and_ambiguous_shorthands_are_named_not_guessed() {
        let root = test_root("unknown_and_ambiguous_shorthands_are_named_not_guessed");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-logout.md"),
            "# FS-042-user-logout: User logout\n\nLead.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Ambiguous: §FS-042\n\nUnknown: §FS-999\n",
        );
        let config = numbered_config(root.clone());
        let (_, report) = check_tree(&config, &root);

        let errors: Vec<_> = report
            .errors
            .iter()
            .map(|finding| finding.message.as_str())
            .collect();
        assert_eq!(
            errors,
            vec![
                "shorthand citation §FS-042 is ambiguous: FS-042-user-login, FS-042-user-logout",
                "shorthand citation §FS-999 matches no declaration",
            ]
        );
        // §FS-check.3.1 is suppressed at a shorthand site, so `unknown reference
        // FS-999` — naming a token that is not a full ID — is never emitted.
        assert!(
            !errors.iter().any(|message| message.contains("unknown reference")),
            "{errors:?}"
        );
        assert!(report.errors.iter().all(|finding| finding.code == "shorthand-citation"));
    }

    // §DF-number-only-citation-shorthand.2.4: the marker is what supplies the
    // intent, so a bare `FS-042` stays text even in the compatibility mode where
    // a bare *full* ID is a citation.
    #[test]
    fn bare_shorthand_is_text_even_when_strict_is_off() {
        let root = test_root("bare_shorthand_is_text_even_when_strict_is_off");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Ticket FS-042 is unrelated prose, and FS-042-user-login is a bare citation.\n",
        );
        let mut config = numbered_config(root.clone());
        config.strict = false;
        let (findings, report) = check_tree(&config, &root);

        assert!(
            findings.citations.iter().all(|cite| !cite.shorthand),
            "a bare shorthand must not be recognized"
        );
        assert_eq!(findings.citations.len(), 1, "only the bare full ID counts");
        assert!(messages(&report).is_empty(), "{:?}", messages(&report));
    }

    // §DF-number-only-citation-shorthand.2.6: the full-ID pass claims its tokens
    // first, so a full citation is never also read as the shorthand prefix
    // inside it.
    #[test]
    fn full_id_wins_over_the_shorthand_prefix_inside_it() {
        let root = test_root("full_id_wins_over_the_shorthand_prefix_inside_it");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n\n## 1. Inputs\n\nStuff.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Canonical: §FS-042-user-login and §FS-042-user-login.1\n",
        );
        let config = numbered_config(root.clone());
        let (findings, report) = check_tree(&config, &root);

        assert_eq!(findings.citations.len(), 2);
        assert!(findings.citations.iter().all(|cite| !cite.shorthand));
        assert!(messages(&report).is_empty(), "{:?}", messages(&report));
    }

    // §FS-id.4.1: a format missing `{number}` or `{slug}` has no shorthand, so
    // neither the grammar nor any pass downstream does anything. `grund` itself
    // is on `{kind}-{slug}`, which is why its own tree gains no findings.
    #[test]
    fn number_less_and_slug_less_formats_have_no_shorthand() {
        let root = test_root("number_less_and_slug_less_formats_have_no_shorthand");
        write(
            &root.join("docs/functional-spec/FS-login.md"),
            "# FS-login: Login\n\nLead.\n",
        );
        // The marker is escaped in this fixture string only: `FS-login` matches
        // *this* repo's own `{kind}-{slug}` grammar, so a literal `<§>FS-login`
        // here would be a live, dangling citation in grund's self-check. Every
        // other fixture string in this module uses a digit-leading slug, which
        // grund's `slug_pattern` already makes inert.
        write(
            &root.join("docs/notes.md"),
            "Cited: \u{a7}FS-login and \u{a7}FS-042\n",
        );
        let mut config = legacy_fs_folder_config(root.clone());
        config.id_format = "{kind}-{slug}".into();
        config.rebuild_grammar().expect("rebuild grammar");
        assert!(!config.grammar.has_shorthand());
        let (findings, report) = check_tree(&config, &root);

        // `FS-042` parses as a full `{kind}-{slug}` ID under the permissive
        // default slug pattern, so it is an ordinary dangling citation here —
        // never a shorthand finding.
        assert!(findings.citations.iter().all(|cite| !cite.shorthand));
        assert!(
            !messages(&report).iter().any(|message| message.contains("shorthand")),
            "{:?}",
            messages(&report)
        );

        let mut slug_less = legacy_fs_folder_config(root.clone());
        slug_less.id_format = "{kind}-{number}".into();
        slug_less.rebuild_grammar().expect("rebuild grammar");
        assert!(!slug_less.grammar.has_shorthand());
    }

    // §FS-config.3.2: the shorthand pattern is the ID pattern with one capture
    // group cut out, which is only sound when each component pattern is a valid
    // regex on its own. Two that balance only against each other compile as one ID
    // pattern and then fail the moment a group is removed — a config `grund` had
    // accepted turning into a panic on the first citation scanned.
    #[test]
    fn component_patterns_must_be_valid_regexes_on_their_own() {
        let root = test_root("component_patterns_must_be_valid_regexes_on_their_own");
        let mut config = numbered_config(root);
        config.number_pattern = "(".into();
        config.slug_pattern = "a)".into();
        let err = config.rebuild_grammar().expect_err("rejected at build");
        assert!(
            format!("{err:#}").contains("[id].slug_pattern is not a valid regex"),
            "{err:#}"
        );
    }

    // §FS-check.3.13 / §FS-fmt.2.3: a shorthand `fmt` is forbidden to rewrite —
    // inline code, a link destination, a runtime string — is still a citation that
    // resolves and counts, but earns no "write the canonical form" error. An error
    // whose only named fix is one the formatter refuses to apply is an error the
    // repository can never clear.
    #[test]
    fn a_shorthand_fmt_cannot_rewrite_is_counted_but_not_reported() {
        let root = test_root("a_shorthand_fmt_cannot_rewrite_is_counted_but_not_reported");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Illustration: `§FS-042`\n\nLink: [t](§FS-042)\n\nUnknown here: `§FS-999`\n",
        );
        write(&root.join("src/login.rs"), "fn f() { let s = \"§FS-042\"; }\n");
        let config = numbered_config(root.clone());
        let (findings, report) = check_tree(&config, &root);

        // Three resolving shorthands, none of them reported…
        assert_eq!(
            messages(&report),
            vec!["shorthand citation §FS-999 matches no declaration"],
            "only the genuinely dangling one is reported"
        );
        // …but all of them are real citations, so the declaration is not unused.
        assert_eq!(
            findings
                .citations
                .iter()
                .filter(|cite| cite.shorthand && cite.id.slug.is_some())
                .count(),
            3
        );
        assert!(
            findings
                .citations
                .iter()
                .filter(|cite| cite.id.slug.is_some())
                .all(|cite| !cite.shorthand_rewritable)
        );
    }

    // §FS-check.3.13 / §FS-fmt.2.3: the "may `fmt` rewrite this?" question has to
    // be asked of the text `fmt` will see. A Python docstring's opening line is
    // where the two texts differ — the scanner works on the interior with the
    // quotes stripped, `fmt` on the raw line where `"""` opens a string literal —
    // so asking the scanner's view reports an error `fmt --write` never clears.
    #[test]
    fn rewritability_is_judged_on_the_raw_line_not_the_scanned_one() {
        let root = test_root("rewritability_is_judged_on_the_raw_line_not_the_scanned_one");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("src/opening.py"),
            "def f():\n    \"\"\"§FS-042 on the opening line.\"\"\"\n    return 1\n",
        );
        write(
            &root.join("src/interior.py"),
            "def g():\n    \"\"\"\n    §FS-042 on an interior line.\n    \"\"\"\n    return 2\n",
        );
        let mut config = numbered_config(root.clone());
        config.docstring_python = true;
        let (findings, report) = check_tree(&config, &root);

        // Both are citations; only the interior one is a site `fmt` can rewrite.
        let mut sites: Vec<(String, bool)> = findings
            .citations
            .iter()
            .filter(|cite| cite.shorthand)
            .map(|cite| {
                (
                    cite.file.file_name().unwrap().to_string_lossy().into_owned(),
                    cite.shorthand_rewritable,
                )
            })
            .collect();
        sites.sort();
        assert_eq!(
            sites,
            vec![
                ("interior.py".to_string(), true),
                ("opening.py".to_string(), false),
            ]
        );
        // …so exactly the rewritable one is reported.
        assert_eq!(
            messages(&report),
            vec!["shorthand citation §FS-042; write §FS-042-user-login"]
        );
    }

    // §FS-check.2.3.1: an escape only earns its "this resolves" suggestion by
    // carrying an `Id` that is declared, so the shorthand form has to be resolved
    // like any other — otherwise `<§>FS-042` is silently exempt from a check that
    // catches `<§>FS-042-user-login`.
    #[test]
    fn an_escaped_shorthand_that_resolves_is_suggested() {
        let root = test_root("an_escaped_shorthand_that_resolves_is_suggested");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(&root.join("docs/notes.md"), "Escaped: <§>FS-042\n");
        let config = numbered_config(root.clone());
        let (findings, _) = check_tree(&config, &root);

        let escaped = findings
            .escaped_citations
            .iter()
            .find(|cite| cite.shorthand)
            .expect("escaped shorthand recorded");
        assert_eq!(escaped.text, "<§>FS-042");
        assert_eq!(
            render_id(&config, &escaped.id),
            "FS-042-user-login",
            "resolved, so the escape check can see that it would be live"
        );
    }

    // §AR-scanner.2.6: the reduction drops the placeholder together with one
    // adjacent separator, whichever side carries it — so a format that puts the
    // slug in the middle still yields `{kind}-{number}`.
    #[test]
    fn shorthand_shape_is_derived_from_either_separator_side() {
        let root = test_root("shorthand_shape_is_derived_from_either_separator_side");
        let mut config = legacy_fs_folder_config(root);
        config.id_format = "{kind}-{slug}-{number}".into();
        config.rebuild_grammar().expect("rebuild grammar");
        assert!(config.grammar.has_shorthand());
        assert_eq!(
            render_id(
                &config,
                &Id {
                    kind: "FS".into(),
                    num: Some(42),
                    slug: None,
                }
            ),
            "FS-042"
        );
    }

    // §FS-check.3.13: the same boundary in the scanner — a site the rewrite will
    // not touch must not be reported either, and reporting it would name a token
    // (`§FS-042`) that does not appear in the file.
    #[test]
    fn a_shorthand_prefix_of_a_longer_token_is_never_reported() {
        let root = test_root("a_shorthand_prefix_of_a_longer_token_is_never_reported");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Typo: §FS-042-User-Login\n\nGlued: §FS-042abc\n\nReal: §FS-042\n",
        );
        let config = numbered_config(root.clone());
        let (findings, report) = check_tree(&config, &root);

        assert_eq!(
            report
                .errors
                .iter()
                .filter(|finding| finding.code == "shorthand-citation")
                .count(),
            1,
            "{:?}",
            messages(&report)
        );
        assert!(
            messages(&report)
                .contains(&"shorthand citation §FS-042; write §FS-042-user-login".to_string()),
            "{:?}",
            messages(&report)
        );
        let shorthands: Vec<&str> = findings
            .citations
            .iter()
            .filter(|cite| cite.shorthand)
            .map(|cite| cite.text.as_str())
            .collect();
        assert_eq!(shorthands, vec!["§FS-042"]);
    }

    // §FS-check.1.2: a resolved shorthand grounds its file under
    // `require_grounding`, exactly as the full citation it stands for would.
    #[test]
    fn a_resolved_shorthand_grounds_its_source_file() {
        let root = test_root("a_resolved_shorthand_grounds_its_source_file");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(&root.join("src/login.rs"), "// §FS-042: enforce login\nfn main() {}\n");
        let mut config = numbered_config(root.clone());
        config.require_grounding = true;
        let (_, report) = check_tree(&config, &root);

        assert!(
            !messages(&report).iter().any(|message| message.contains("ungrounded")),
            "{:?}",
            messages(&report)
        );
    }

    /// §AR-scanner.2.6: one marker is one citation. A qualified marker belongs
    /// to the qualified pass, and outside workspace mode that pass is the loose
    /// fallback (§FS-workspace.5) — which records nothing into
    /// `claimed_markers`, so the shorthand pattern used to match the same
    /// `\u{a7}<alias>/<ID>` a second time. The token then became two identical
    /// citations: a row `grund cover` printed twice (§FS-cover.2) and a
    /// diagnostic `grund check` printed twice, and §RM-cochange-gate, which
    /// consumes that index, would have double-counted it.
    #[test]
    fn a_qualified_shorthand_is_one_citation_not_two() {
        let root = test_root("a_qualified_shorthand_is_one_citation_not_two");
        let config = numbered_config(root.clone());
        write(
            &root.join("docs/functional-spec/FS-001-local.md"),
            "# FS-001-local: Local\n\nOne token: \u{a7}api/FS-042\n",
        );
        let findings = scan_findings(&config, &root);
        let qualified: Vec<&Citation> = findings
            .citations
            .iter()
            .filter(|cite| cite.namespace.as_deref() == Some("api"))
            .collect();
        assert_eq!(
            qualified.len(),
            1,
            "one marker, one citation: {:?}",
            qualified
                .iter()
                .map(|cite| (cite.line, cite.column, cite.text.clone()))
                .collect::<Vec<_>>()
        );
        assert_eq!(qualified[0].text, "\u{a7}api/FS-042");
        // And the local shorthand beside it is untouched — the skip is about the
        // namespace, not about shorthands.
        write(
            &root.join("docs/functional-spec/FS-002-other.md"),
            "# FS-002-other: Other\n\nLocal shorthand: \u{a7}FS-001\n",
        );
        let findings = scan_findings(&config, &root);
        assert_eq!(
            findings
                .citations
                .iter()
                .filter(|cite| cite.namespace.is_none() && cite.shorthand)
                .count(),
            1
        );
    }

    /// §REQ-no-missed-citation.1: the shorthand pass defers a qualified marker to
    /// the qualified pass only where that pass actually claimed it. Outside
    /// workspace mode the claimant is the loose fallback (§FS-workspace.5), which
    /// parses the tail as `KIND[-NUM]-SLUG` with an uppercase kind — so under an
    /// `[id] format` it cannot read, the shorthand pass is the only producer and
    /// a blanket skip would delete the citation and turn a red tree green.
    #[test]
    fn a_qualified_shorthand_the_loose_parser_cannot_read_is_still_a_citation() {
        // Two shapes the loose parser rejects: a kind not separated from its
        // number, and a kind that is not uppercase.
        for (name, id_format, kind_prefix, token) in [
            (
                "unseparated",
                "{kind}{number}-{slug}",
                "FS",
                "\u{a7}api/FS042",
            ),
            (
                "lowercase-kind",
                "{kind}-{number}-{slug}",
                "fs",
                "\u{a7}api/fs-042",
            ),
        ] {
            let root = test_root(&format!(
                "a_qualified_shorthand_the_loose_parser_cannot_read_{name}"
            ));
            let mut config = legacy_fs_folder_config(root.clone());
            config.id_format = id_format.into();
            for kind in &mut config.kinds {
                if kind.kind == "FS" {
                    kind.kind = kind_prefix.into();
                }
            }
            config.rebuild_grammar().expect("rebuild grammar");
            assert!(config.grammar.has_shorthand(), "{name}");
            write(&root.join("docs/notes.md"), &format!("Cited: {token}\n"));

            let findings = scan_findings(&config, &root);
            let qualified: Vec<&Citation> = findings
                .citations
                .iter()
                .filter(|cite| cite.namespace.as_deref() == Some("api"))
                .collect();
            assert_eq!(qualified.len(), 1, "{name}: exactly one citation");
            assert_eq!(qualified[0].text, token, "{name}");

            // And `check` still reports it: this is the verdict the blanket skip
            // silently flipped to success (§REQ-backwards-compatibility.1).
            let report = check_findings(&findings, &config);
            assert!(
                report
                    .errors
                    .iter()
                    .any(|error| error.message.contains("unknown project alias")),
                "{name}: {:?}",
                messages(&report)
            );
        }
    }

    /// §AR-scanner.2.6: the claim record is scoped to **one line**. A marker
    /// offset means nothing across lines, so a record that outlived its line
    /// would let a qualified citation on line 1 suppress the shorthand pass at
    /// the same byte offset on every line below it — deleting citations exactly
    /// the way an unconditional skip did (§REQ-no-missed-citation.1).
    ///
    /// The two tokens are chosen so only the scoping can tell them apart: under
    /// `{kind}{number}-{slug}` the loose fallback parses the first (it has the
    /// `-` and the uppercase kind it needs) and declines the second, and both
    /// sit at the same column.
    #[test]
    fn a_claimed_marker_does_not_reach_the_next_line() {
        let root = test_root("a_claimed_marker_does_not_reach_the_next_line");
        let mut config = numbered_config(root.clone());
        config.id_format = "{kind}{number}-{slug}".to_string();
        config.rebuild_grammar().expect("rebuild grammar");
        write(
            &root.join("docs/functional-spec/FS001-local.md"),
            // Same byte offset on both lines: the fallback claims line 3's
            // marker, and line 5's must not inherit that claim.
            "# FS001-local: Local\n\n\u{a7}api/FS042-user-login here\n\n\u{a7}api/FS042 here\n",
        );
        let findings = scan_findings(&config, &root);
        let qualified: Vec<(usize, usize, &str)> = findings
            .citations
            .iter()
            .filter(|cite| cite.namespace.as_deref() == Some("api"))
            .map(|cite| (cite.line, cite.column, cite.text.as_str()))
            .collect();
        assert_eq!(
            qualified,
            vec![
                (3, 1, "\u{a7}api/FS042-user-login"),
                (5, 1, "\u{a7}api/FS042"),
            ],
            "both lines keep their citation, once each"
        );
    }
}
