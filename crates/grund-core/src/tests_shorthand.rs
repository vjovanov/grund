/// Test module: the number-only citation shorthand — recognition, resolution,
/// the check error, bulk normalization, and the query surfaces
/// (§FS-check.1.2, §FS-check.3.13, §FS-fmt.2.4, §DF-number-only-citation-shorthand).
#[cfg(test)]
mod tests_shorthand {
    use super::tests_support::*;
    use super::*;

    /// The default `grund init` config: `{kind}-{number}-{slug}`, which is the
    /// only shape that has a shorthand at all (§FS-check.1.2).
    fn numbered_config(root: PathBuf) -> Config {
        let config = legacy_fs_folder_config(root);
        assert_eq!(config.id_format, "{kind}-{number}-{slug}");
        config
    }

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

    // §AR-scanner.2.6: `render_id` reduces a partial `Id` by the same rule the
    // shorthand pattern is derived from, so an unresolved shorthand prints as
    // `FS-042` rather than leaking the raw `{slug}` placeholder into a report.
    #[test]
    fn render_id_prints_a_slugless_id_as_the_shorthand() {
        let config = numbered_config(test_root("render_id_prints_a_slugless_id_as_the_shorthand"));
        let shorthand = Id {
            kind: "FS".into(),
            num: Some(42),
            slug: None,
        };
        assert_eq!(render_id(&config, &shorthand), "FS-042");
        assert_eq!(
            render_id(
                &config,
                &Id {
                    kind: "FS".into(),
                    num: Some(42),
                    slug: Some("user-login".into()),
                }
            ),
            "FS-042-user-login"
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

    // §FS-check.1.2 / §FS-show.1: a query persists nothing, so the shorthand is
    // simply expanded at the CLI boundary. This is also what makes a clicked
    // `§FS-042` open (§FS-integrations.3.1).
    #[test]
    fn shorthand_resolves_as_a_query_argument() {
        let root = test_root("shorthand_resolves_as_a_query_argument");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let (id, section) = resolve_id_arg("FS-042", &config, &findings).expect("resolve");
        assert_eq!(render_id(&config, &id), "FS-042-user-login");
        assert_eq!(section, None);

        let (id, section) = resolve_id_arg("FS-042.1", &config, &findings).expect("resolve");
        assert_eq!(render_id(&config, &id), "FS-042-user-login");
        assert_eq!(section.as_deref(), Some("1"));

        // A full ID is unaffected, and an unknown shorthand keeps its written
        // form so the caller's own "not found" path names what was asked for.
        let (id, _) = resolve_id_arg("FS-999", &config, &findings).expect("resolve");
        assert_eq!(render_id(&config, &id), "FS-999");
    }

    // §FS-show.2.2.1: an ambiguous shorthand argument is a query failure that
    // lists every candidate rather than picking one.
    #[test]
    fn ambiguous_shorthand_argument_lists_every_candidate() {
        let root = test_root("ambiguous_shorthand_argument_lists_every_candidate");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-logout.md"),
            "# FS-042-user-logout: User logout\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let err = resolve_id_arg("FS-042", &config, &findings).expect_err("ambiguous");
        assert_eq!(
            format!("{err:#}"),
            "ambiguous ID: FS-042 (matches FS-042-user-login, FS-042-user-logout)"
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

    // §FS-lsp.1.4: a shorthand already in the document navigates like any other
    // citation. The snapshot carries the canonical target while the range stays
    // the written token, which is what makes hover, go-to-definition,
    // references, document links, and highlight all work without any of them
    // knowing the shorthand exists.
    #[test]
    fn lsp_snapshot_navigates_a_shorthand_citation() {
        let root = test_root("lsp_snapshot_navigates_a_shorthand_citation");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{number}-{slug}\"\n",
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n\n## 1. Inputs\n\nStuff.\n",
        );
        write(&root.join("src/lib.rs"), "//! §FS-042.1\n");
        let snapshot = lsp_snapshot(LspSnapshotOpts {
            path: root.clone(),
            path_provided: true,
            open_documents: BTreeMap::new(),
        })
        .expect("lsp snapshot");

        let citation = snapshot
            .citations
            .iter()
            .find(|citation| citation.display_path == "src/lib.rs")
            .expect("shorthand citation");
        assert_eq!(citation.text, "\u{a7}FS-042.1", "range covers what was typed");
        assert_eq!(citation.query_id, "FS-042-user-login.1");
        assert_eq!(citation.declaration_query_id, "FS-042-user-login");
        assert_eq!(
            citation.target_path.as_deref().map(canonical_test_path),
            Some(canonical_test_path(
                &root.join("docs/functional-spec/FS-042-user-login.md")
            ))
        );
        assert_eq!(citation.target_line, Some(5), "jumps to the cited section");

        // The §FS-check.3.13 finding reaches the editor as a diagnostic.
        assert!(
            snapshot
                .report
                .errors
                .iter()
                .any(|finding| finding.code == "shorthand-citation"),
            "{:?}",
            snapshot.report.errors
        );
    }

    // §FS-integrations.3.1: the clients need no shorthand matcher of their own —
    // the shared citation shape already accepts one, in every form. This pins
    // that, because the spec claims it and six hand-written regexes would
    // otherwise be free to drift from the engine.
    #[test]
    fn client_matchers_already_accept_the_shorthand() {
        let citation_shape = Regex::new(
            r"[^\w\s]{1,3}(?:[a-z][a-z0-9-]*/)?[A-Z][A-Z0-9]*-[a-z0-9][a-z0-9-]*(?:\.[0-9]+)*",
        )
        .expect("client citation shape");
        for (text, expected) in [
            ("see \u{a7}FS-042 here", "\u{a7}FS-042"),
            ("see \u{a7}FS-042.1 here", "\u{a7}FS-042.1"),
            ("see \u{a7}api/FS-042 here", "\u{a7}api/FS-042"),
            ("see \u{a7}FS-042-user-login here", "\u{a7}FS-042-user-login"),
        ] {
            assert_eq!(
                citation_shape.find(text).map(|found| found.as_str()),
                Some(expected),
                "client matcher must claim {text:?}"
            );
        }
        // The same shape is what every client artifact embeds.
        for artifact in [
            WEZTERM_SNIPPET,
            KITTY_SNIPPET,
            ITERM2_SNIPPET,
            VSCODE_EXTENSION_JS,
        ] {
            assert!(
                artifact.contains("[A-Z][A-Z0-9]*-[a-z0-9][a-z0-9-]*"),
                "client artifact lost the shared citation shape"
            );
        }
    }
}
