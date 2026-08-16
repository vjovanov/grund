/// Test module: the inline note layout classifier and its opt-in check
/// (§FS-inline-citation-style.3.3, §FS-inline-citation-style.4.4)
#[cfg(test)]
mod tests_inline_note_layout {
    use super::tests_support::*;
    use super::*;

    fn layout_config(root: PathBuf, layout: &str) -> Config {
        let mut config = legacy_fs_folder_config(root);
        config.inline_note_layout = layout.to_string();
        config
    }

    /// A layout the check actually reads: the classifier records nothing while
    /// `inline_note_layout_check` is `off` (§FS-inline-citation-style.4.4), so a
    /// test that asks it a question has to turn the gate on.
    fn checked_layout_config(root: PathBuf, layout: &str) -> Config {
        let mut config = layout_config(root, layout);
        config.inline_note_layout_check = "error".to_string();
        config
    }

    fn conforms(config: &Config, line: &str) -> bool {
        let ranges = line_citation_ranges(line, config, &[]);
        line_conforms(InlineNoteLayout::from_config(config), line, &ranges, config)
    }

    fn has_note(config: &Config, block: &[&str]) -> bool {
        block_has_inline_note(&mut BlockCitations::new(block, config, &[]))
    }

    fn violations(config: &Config, block: &[&str], has_note: bool) -> Vec<usize> {
        inline_layout_violations(&mut BlockCitations::new(block, config, &[]), 1, has_note)
    }

    // §FS-inline-citation-style.2.1: the default imposes nothing, so every
    // arrangement a `citation-with-note` tree already wrote still passes.
    #[test]
    fn any_layout_accepts_every_arrangement() {
        let config = layout_config(test_root("any_layout_accepts_every_arrangement"), "any");
        for line in [
            "// §FS-001-login: reject an expired credential",
            "// §FS-001-login reject an expired credential",
            "// reject an expired credential (§FS-001-login)",
            "// §FS-001-login §FS-002-logout: two of them",
        ] {
            assert!(conforms(&config, line), "`any` must accept `{line}`");
        }
    }

    // §FS-inline-citation-style.3.3: the canonical form, its multi-citation
    // spelling, a citation later in the note, a colon that ends the line, and a
    // line with no citation at all.
    #[test]
    fn citation_first_colon_accepts_the_canonical_forms() {
        let config = layout_config(
            test_root("citation_first_colon_accepts_the_canonical_forms"),
            "citation-first-colon",
        );
        for line in [
            "// §FS-001-login: reject an expired credential",
            "    // §FS-001-login: indented like the code it annotates",
            "// §FS-001-login, §FS-002-logout: both branches",
            "// §FS-001-login, §FS-002-logout, §FS-003-reset: three of them",
            "// §FS-001-login: the rule (see also §FS-002-logout)",
            "// §FS-001-login:",
            "// §FS-001-login:  two spaces still open a note",
            "///   §FS-001-login: indented past the prefix",
            "//\t§FS-001-login: a tab past the prefix",
            " *   §FS-001-login: aligned under the block opener",
            "#\t§FS-001-login: a tab after a hash",
            "/// Walks every recognized citation and resolves it.",
            "//",
        ] {
            assert!(conforms(&config, line), "must accept `{line}`");
        }
    }

    // §FS-inline-citation-style.3.3, rule 4: the form is exact, so each near miss
    // is a deviation rather than a tolerated spelling.
    #[test]
    fn citation_first_colon_rejects_near_misses() {
        let config = layout_config(
            test_root("citation_first_colon_rejects_near_misses"),
            "citation-first-colon",
        );
        for line in [
            "// §FS-001-login reject an expired credential",
            "// reject an expired credential (§FS-001-login)",
            "// §FS-001-login — reject an expired credential",
            "// §FS-001-login §FS-002-logout: both branches",
            "// §FS-001-login,§FS-002-logout: both branches",
            "// §FS-001-login , §FS-002-logout: both branches",
            "// §FS-001-login :reject an expired credential",
            "// §FS-001-login::doubled",
            "// see §FS-001-login: reject an expired credential",
            "// §FS-001-login, reject an expired credential",
            "// §FS-001-login, x §FS-002-logout: a word inside the run",
            "// §FS-001-login, §FS-002-logout, prose the run does not close",
        ] {
            assert!(!conforms(&config, line), "must reject `{line}`");
        }
    }

    // §FS-inline-citation-style.3.3: the line is read after the comment prefix and
    // any block closer are stripped, so every recognized comment shape is judged on
    // the same content the author sees.
    #[test]
    fn citation_first_colon_reads_every_comment_prefix() {
        let config = layout_config(
            test_root("citation_first_colon_reads_every_comment_prefix"),
            "citation-first-colon",
        );
        for line in [
            "/** §FS-001-login: reject an expired credential",
            " * §FS-001-login: reject an expired credential",
            "/* §FS-001-login: reject an expired credential */",
            "\"\"\"§FS-001-login: reject an expired credential\"\"\"",
            "# §FS-001-login: reject an expired credential",
            "//! §FS-001-login: reject an expired credential",
            "-- §FS-001-login: reject an expired credential",
        ] {
            assert!(conforms(&config, line), "must accept `{line}`");
        }
        for line in [
            " * §FS-001-login reject an expired credential",
            "# reject an expired credential (§FS-001-login)",
        ] {
            assert!(!conforms(&config, line), "must reject `{line}`");
        }
    }

    // §FS-inline-citation-style.3.3: the block closer and the space in front of it
    // are stripped together, so a colon that ends a `/* … */` line closes the
    // grammar's empty tail rather than opening a note made of one space.
    #[test]
    fn a_block_closer_leaves_no_trailing_space() {
        let config = layout_config(
            test_root("a_block_closer_leaves_no_trailing_space"),
            "citation-first-colon",
        );
        for (line, content) in [
            ("/* §FS-001-login: */", "§FS-001-login:"),
            ("/* §FS-001-login: reject it */", "§FS-001-login: reject it"),
            ("\"\"\"§FS-001-login: \"\"\"", "§FS-001-login:"),
        ] {
            let (start, end) = comment_content_range(line, &config);
            assert_eq!(&line[start..end], content, "content of `{line}`");
            assert!(conforms(&config, line), "must accept `{line}`");
        }

        // A citation alone inside a block comment still carries no note.
        let block = ["/* §FS-001-login */"];
        assert!(!has_note(&config, &block));
    }

    // §FS-inline-citation-style.3.3, rule 5: a workspace-qualified token is one
    // citation token, so the run reads it as the line's opening citation.
    #[test]
    fn citation_first_colon_reads_a_qualified_token() {
        let config = layout_config(
            test_root("citation_first_colon_reads_a_qualified_token"),
            "citation-first-colon",
        );
        assert!(conforms(&config, "// §api/FS-001-login: the member's rule"));
        assert!(!conforms(&config, "// §api/FS-001-login the member's rule"));
    }

    // §FS-inline-citation-style.3.3, rule 2: a site with no note has no layout, so
    // nothing in it is classified — not even a line that would otherwise deviate.
    #[test]
    fn a_site_without_a_note_is_exempt() {
        let config = checked_layout_config(
            test_root("a_site_without_a_note_is_exempt"),
            "citation-first-colon",
        );
        let block = ["// §FS-001-login  §FS-002-logout"];
        assert!(violations(&config, &block, false).is_empty());
        // The same block, told it carries a note, is judged and fails.
        assert_eq!(violations(&config, &block, true), vec![1]);
    }

    // §FS-inline-citation-style.1: what joins two citations of one run says
    // nothing, so a chain stays a pure citation comment however it is spelled —
    // including with the `, ` the layout itself mandates in front of a colon.
    #[test]
    fn a_citation_chain_carries_no_note() {
        let config = checked_layout_config(
            test_root("a_citation_chain_carries_no_note"),
            "citation-first-colon",
        );
        for line in [
            "// §FS-001-login  §FS-002-logout",
            "// §FS-001-login, §FS-002-logout",
            "// §FS-001-login,§FS-002-logout",
            "// §FS-001-login  ,  §FS-002-logout",
            "/** §FS-001-login, §FS-002-logout */",
            "// §FS-001-login, §FS-002-logout, §FS-003-reset",
        ] {
            let block = [line];
            let carries_note = has_note(&config, &block);
            assert!(!carries_note, "`{line}` is a pure citation comment");
            assert!(
                violations(&config, &block, carries_note).is_empty(),
                "`{line}` has no note, so it has no layout to deviate from"
            );
        }
        for line in [
            "// §FS-001-login + §FS-002-logout",
            "// §FS-001-login and §FS-002-logout",
            "// §FS-001-login,, §FS-002-logout",
            "// §FS-001-login, §FS-002-logout: both branches",
        ] {
            let block = [line];
            assert!(
                has_note(&config, &block),
                "`{line}` says something between or after its citations"
            );
        }
    }

    // §FS-inline-citation-style.3.1: the same reading governs `citation-only`, so
    // a chain the layout would mandate is never rejected as prose.
    #[test]
    fn citation_only_accepts_a_comma_joined_chain() {
        let root = test_root("citation_only_accepts_a_comma_joined_chain");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("docs/functional-spec/FS-002-logout.md"),
            "# FS-002-logout: Logout\n",
        );
        write(
            &root.join("src/auth.rs"),
            concat!(
                "// §FS-001-login, §FS-002-logout\n",
                "pub fn login() {}\n",
                "\n",
                "// §FS-001-login + §FS-002-logout\n",
                "pub fn logout() {}\n",
            ),
        );
        let mut config = legacy_fs_folder_config(root.clone());
        config.inline_style = "citation-only".into();

        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let lines = report
            .errors
            .iter()
            .filter(|finding| finding.code == "inline-citation-style")
            .filter_map(|finding| finding.line)
            .collect::<Vec<_>>();
        assert_eq!(lines, vec![4], "only the line that says something is prose");
    }

    // §FS-inline-citation-style.3.3, §FS-inline-citation-style.4.4: no layout, no
    // note style, or no channel for the verdict to reach — no classification. Each
    // short-circuit stands on its own, so neither the default nor a
    // documented-only layout ever asks the classifier a question
    // (§GOAL-fast-feedback).
    #[test]
    fn no_layout_records_no_violations() {
        let root = test_root("no_layout_records_no_violations");
        let block = ["// §FS-001-login reject an expired credential"];

        let any = checked_layout_config(root.clone(), "any");
        assert!(violations(&any, &block, true).is_empty());

        let mut citation_only = checked_layout_config(root.clone(), "citation-first-colon");
        citation_only.inline_style = "citation-only".into();
        assert!(violations(&citation_only, &block, true).is_empty());

        // A layout the project documents but does not gate: at `off` the verdicts
        // have no consumer, so the deviating line is never classified.
        let documented_only = layout_config(root, "citation-first-colon");
        assert_eq!(documented_only.inline_note_layout_check, "off");
        assert!(violations(&documented_only, &block, true).is_empty());
    }

    /// How many of a site's lines have been tokenized so far: the memo slots the
    /// two passes have filled (§AR-scanner.3).
    fn filled_slots(block: &BlockCitations<'_>) -> usize {
        block.ranges.iter().filter(|slot| slot.is_some()).count()
    }

    // §GOAL-fast-feedback: the sharing between the two note verdicts is a memo
    // filled as a pass reaches a line, never up front — the property the rest of
    // this module cannot see, since an eager tokenization answers every question
    // above identically and only costs more. Pinned here because it has already
    // been regressed once and caught by hand.
    #[test]
    fn note_walk_tokenizes_only_the_lines_it_reads() {
        let root = test_root("note_walk_tokenizes_only_the_lines_it_reads");
        // The prose is on the block's first line, so the note walk has its answer
        // there and the two citation lines below it are never read.
        let block = [
            "/// Walks the credential store.",
            "/// §FS-001-login: one error per expired credential.",
            "/// §FS-001-login — and one more, laid out wrong.",
        ];

        // Documented-only (§FS-inline-citation-style.4.4): the classifier returns
        // on the `off` guard, before it looks at a line, so the memo does not grow.
        let documented_only = layout_config(root.clone(), "citation-first-colon");
        assert_eq!(documented_only.inline_note_layout_check, "off");
        let mut site = BlockCitations::new(&block, &documented_only, &[]);
        let has_note = block_has_inline_note(&mut site);
        assert!(has_note);
        assert_eq!(
            filled_slots(&site),
            1,
            "the note walk stops at the first line that says something"
        );
        assert!(inline_layout_violations(&mut site, 1, has_note).is_empty());
        assert_eq!(
            filled_slots(&site),
            1,
            "at `off` no line is classified, so no further line is tokenized"
        );

        // Gated: the layout pass judges every line, so every line is tokenized —
        // and the one the note walk already read is not tokenized twice.
        let gated_config = checked_layout_config(root, "citation-first-colon");
        let mut gated = BlockCitations::new(&block, &gated_config, &[]);
        let has_note = block_has_inline_note(&mut gated);
        assert_eq!(filled_slots(&gated), 1);
        assert_eq!(inline_layout_violations(&mut gated, 1, has_note), vec![3]);
        assert_eq!(
            filled_slots(&gated),
            block.len(),
            "the layout pass reads every line of the site"
        );
    }

    fn layout_fixture(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.rs"),
            concat!(
                "// §FS-001-login: reject an expired credential\n",
                "pub fn login() {}\n",
                "\n",
                "// §FS-001-login reject an expired credential\n",
                "pub fn relogin() {}\n",
                "\n",
                "/// Walks the credential store.\n",
                "/// §FS-001-login: one error per expired credential.\n",
                "/// §FS-001-login — and one more, laid out wrong.\n",
                "pub fn sweep() {}\n",
            ),
        );
        root
    }

    fn style_findings(config: &Config, root: &Path) -> (Vec<usize>, Vec<usize>) {
        let (findings, _) = scan_tree(config, Some(root), true).expect("scan root");
        let report = check_findings(&findings, config);
        let lines = |diagnostics: &[Diagnostic]| {
            diagnostics
                .iter()
                .filter(|finding| {
                    finding.code == "inline-citation-style"
                        && finding.message.starts_with("inline note must open")
                })
                .filter_map(|finding| finding.line)
                .collect::<Vec<_>>()
        };
        (lines(&report.errors), lines(&report.warnings))
    }

    // §FS-inline-citation-style.4.4: one error per nonconforming line, anchored at
    // the line — never at the site's opener, and never on a conforming sibling.
    #[test]
    fn layout_error_reports_one_finding_per_offending_line() {
        let root = layout_fixture("layout_error_reports_one_finding_per_offending_line");
        let mut config = layout_config(root.clone(), "citation-first-colon");
        config.inline_note_layout_check = "error".into();

        let (errors, warnings) = style_findings(&config, &root);
        assert_eq!(errors, vec![4, 9], "the two deviating lines, in file order");
        assert!(warnings.is_empty(), "`error` speaks through one channel only");
    }

    // §FS-inline-citation-style.4.4: `warn` reports the same lines with the same
    // message on the warning channel, which never moves the exit code.
    #[test]
    fn layout_warn_reports_the_same_lines_as_warnings() {
        let root = layout_fixture("layout_warn_reports_the_same_lines_as_warnings");
        let mut config = layout_config(root.clone(), "citation-first-colon");
        config.inline_note_layout_check = "warn".into();

        let (errors, warnings) = style_findings(&config, &root);
        assert!(errors.is_empty(), "a warning never becomes an error");
        assert_eq!(warnings, vec![4, 9]);
    }

    // §FS-inline-citation-style.4.4: the message is identical at both levels, so a
    // project moving from `warn` to `error` changes the exit code and nothing else.
    #[test]
    fn layout_message_names_the_form_with_the_configured_marker() {
        let root = layout_fixture("layout_message_names_the_form_with_the_configured_marker");
        let mut config = layout_config(root.clone(), "citation-first-colon");
        config.inline_note_layout_check = "error".into();

        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let messages = report
            .errors
            .iter()
            .filter(|finding| finding.code == "inline-citation-style")
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            messages,
            vec![
                "inline note must open with its citations and a colon (§<ID>: note)".to_string();
                2
            ]
        );
    }

    // §FS-inline-citation-style.4.4: both keys default to the inert value, and the
    // check key is inert on its own under `any` — an upgrade turns nothing red.
    #[test]
    fn layout_is_silent_when_either_key_is_inert() {
        let root = layout_fixture("layout_is_silent_when_either_key_is_inert");

        let off = layout_config(root.clone(), "citation-first-colon");
        assert_eq!(style_findings(&off, &root), (Vec::new(), Vec::new()));

        let mut any = layout_config(root.clone(), "any");
        any.inline_note_layout_check = "error".into();
        assert_eq!(style_findings(&any, &root), (Vec::new(), Vec::new()));

        let defaults = legacy_fs_folder_config(root.clone());
        assert_eq!(defaults.inline_note_layout, "any");
        assert_eq!(defaults.inline_note_layout_check, "off");
        assert_eq!(style_findings(&defaults, &root), (Vec::new(), Vec::new()));
    }

    // §FS-inline-citation-style.2.2: both keys are closed enums read from the
    // project's own config, and an unrecognized value fails on load.
    #[test]
    fn both_layout_keys_load_and_reject_unknown_values() {
        let root = test_root("both_layout_keys_load_and_reject_unknown_values");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n[reference]\ninline_note_layout = \"citation-first-colon\"\ninline_note_layout_check = \"warn\"\n",
        );
        let config = load_config(&root).expect("load config");
        assert_eq!(config.inline_note_layout, "citation-first-colon");
        assert_eq!(config.inline_note_layout_check, "warn");

        for (key, value, expected) in [
            (
                "inline_note_layout",
                "citation-first",
                "unknown [reference] inline_note_layout `citation-first` (expected any or citation-first-colon)",
            ),
            (
                "inline_note_layout_check",
                "errors",
                "unknown [reference] inline_note_layout_check `errors` (expected off, warn, or error)",
            ),
        ] {
            write(
                &root.join("grund.toml"),
                &format!("grund_config_version = 1\n[reference]\n{key} = \"{value}\"\n"),
            );
            let error = match load_config(&root) {
                Ok(_) => panic!("{key} = \"{value}\" must be rejected"),
                Err(error) => error,
            };
            assert!(
                error.to_string().contains(expected),
                "expected `{expected}`, got `{error}`"
            );
        }
    }

    // §FS-inline-citation-style.5: the layout sentence appends to the budget
    // sentence, is written with the project's marker, and is absent under `any` so
    // no existing managed block drifts.
    #[test]
    fn agents_sentence_teaches_the_configured_layout() {
        let root = test_root("agents_sentence_teaches_the_configured_layout");
        let any = layout_config(root.clone(), "any");
        assert_eq!(
            inline_citation_style_sentence(&any),
            "Inline notes: ≤ 1 line preferred, hard cap 3 lines; ≤ 100 columns."
        );

        let mut colon = layout_config(root.clone(), "citation-first-colon");
        colon.inline_note_layout_check = "error".into();
        assert_eq!(
            inline_citation_style_sentence(&colon),
            "Inline notes: ≤ 1 line preferred, hard cap 3 lines; ≤ 100 columns. Lay each note out citation-first: `// §<ID>: <note>` (several citations: `// §<ID>, §<ID>: <note>`)."
        );

        // The enforcement level is not an instruction: `off` renders the same
        // sentence `error` does.
        let mut off = layout_config(root.clone(), "citation-first-colon");
        off.marker = "@".into();
        assert!(inline_citation_style_sentence(&off).contains("`// @<ID>: <note>`"));

        // A style that permits no note at all has no layout to teach.
        let mut citation_only = layout_config(root, "citation-first-colon");
        citation_only.inline_style = "citation-only".into();
        assert_eq!(
            inline_citation_style_sentence(&citation_only),
            "Inline citations carry no prose — put rationale in the spec."
        );
    }
}
