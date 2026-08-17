/// Test module: the inline note layout classifier and its opt-in check
/// (§FS-inline-citation-style.3.3, §FS-inline-citation-style.4.4)
#[cfg(test)]
mod tests_inline_note_layout {
    use super::tests_support::*;
    use super::*;

    fn conforms(config: &Config, line: &str) -> bool {
        let ranges = line_citation_ranges(line, config, &[]);
        let prefixes = comment_strip_prefixes(config);
        match line_layout_view(line, &ranges, &prefixes) {
            // §FS-inline-citation-style.3.3, rule 1: no citation in the content,
            // so nothing to lay out.
            None => true,
            Some((content, tokens)) => {
                content_conforms(InlineNoteLayout::from_config(config), content, &tokens)
            }
        }
    }

    fn has_note(config: &Config, block: &[&str]) -> bool {
        block_has_inline_note(block, config, &[], &comment_strip_prefixes(config))
    }

    fn violations(config: &Config, block: &[&str], has_note: bool) -> Vec<usize> {
        inline_layout_violations(
            &mut BlockCitations::new(block, config, &[]),
            &comment_strip_prefixes(config),
            1,
            has_note,
        )
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
            let (start, end) = comment_content_range(line, &comment_strip_prefixes(&config));
            assert_eq!(&line[start..end], content, "content of `{line}`");
            assert!(conforms(&config, line), "must accept `{line}`");
        }

        // A citation alone inside a block comment still carries no note.
        let block = ["/* §FS-001-login */"];
        assert!(!has_note(&config, &block));
    }

    // §FS-inline-citation-style.3.3: a leading list marker is skipped with the
    // indentation, so an enumerated block of grounded points can open each item
    // with its citation run. One marker, and only where a space follows it.
    #[test]
    fn a_list_marker_is_skipped_like_indentation() {
        let config = layout_config(
            test_root("a_list_marker_is_skipped_like_indentation"),
            "citation-first-colon",
        );
        for line in [
            "/// - §FS-001-login: a bulleted grounded point",
            "/// * §FS-001-login: a star bullet",
            "/// + §FS-001-login: a plus bullet",
            "/// 1. §FS-001-login: an ordered item",
            "/// 12) §FS-001-login: a two-digit ordered item",
            "///   - §FS-001-login: indented past the prefix first",
        ] {
            assert!(conforms(&config, line), "must accept `{line}`");
        }
        for line in [
            // No space after the marker: the `-` is the first thing the content says.
            "// -§FS-001-login: not a list item",
            // One marker is skipped, not a chain of them.
            "// - - §FS-001-login: two markers deep",
            // The marker changes nothing about the rest of the form.
            "// - §FS-001-login a bulleted point with no colon",
        ] {
            assert!(!conforms(&config, line), "must reject `{line}`");
        }

        // §2.3 is untouched: the marker is still note text when note presence is
        // decided, so a bulleted pointer is not silently reclassified.
        assert!(has_note(&config, &["// - §FS-001-login"]));
    }

    // §FS-inline-citation-style.3.3, rule 1: the line that opens the note is
    // judged, and so is any later line that opens with a citation — but a
    // continuation line that opens with prose is note text, so a note may wrap
    // and still name a second point on the way (rule 3, and the line budget).
    #[test]
    fn a_wrapped_note_may_name_a_point_on_its_continuation() {
        let config = checked_layout_config(
            test_root("a_wrapped_note_may_name_a_point_on_its_continuation"),
            "citation-first-colon",
        );

        let wrapped = [
            "/* §FS-001-login: a note that runs past one line and",
            "   still names §FS-002-logout on the way */",
        ];
        assert!(violations(&config, &wrapped, true).is_empty());

        // The same continuation, opening with the citation: indistinguishable from
        // a note opening, so it is judged and it fails.
        let opens_with_citation = [
            "/* §FS-001-login: a note that runs past one line and",
            "   §FS-002-logout opens the continuation */",
        ];
        assert_eq!(violations(&config, &opens_with_citation, true), vec![2]);

        // The first citation-bearing line is judged whatever it opens with: a
        // summary line above it is unconstrained, it is not.
        let prose_first = [
            "/// Walks the credential store.",
            "/// then §FS-001-login decides",
            "/// and §FS-002-logout follows",
        ];
        assert_eq!(violations(&config, &prose_first, true), vec![2]);
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
        let mut citation_only = checked_layout_config(root.clone(), "citation-first-colon");
        citation_only.inline_style = "citation-only".into();
        // A layout the project documents but does not gate: at `off` the verdicts
        // have no consumer, so the deviating line is never classified.
        let documented_only = layout_config(root, "citation-first-colon");
        assert_eq!(documented_only.inline_note_layout_check, "off");

        for config in [any, citation_only, documented_only] {
            assert!(!layout_pass_enabled(&config));
            let (_, violations) = inline_note_verdicts(&block, 1, &config, &[]);
            assert!(violations.is_empty(), "the line must not be classified");
        }
    }

    // §FS-inline-citation-style.4.4: the scanner classifies a line only where the
    // checker has somewhere to report it, and both read one predicate — so a
    // `Config` built in memory with a level the load-time enum would have rejected
    // (§FS-inline-citation-style.2.2) classifies nothing rather than paying for
    // verdicts the checker then drops.
    #[test]
    fn an_unknown_check_level_classifies_nothing() {
        let root = test_root("an_unknown_check_level_classifies_nothing");
        let block = ["// §FS-001-login reject an expired credential"];

        let mut config = checked_layout_config(root, "citation-first-colon");
        config.inline_note_layout_check = "info".into();
        assert!(layout_channel(&config).is_none());
        assert!(!layout_pass_enabled(&config));
        let (_, violations) = inline_note_verdicts(&block, 1, &config, &[]);
        assert!(violations.is_empty());
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

        // Documented-only (§FS-inline-citation-style.4.4): no channel, so no second
        // reader exists, the block is walked without a memo at all, and the only
        // verdict taken is note presence.
        let documented_only = layout_config(root.clone(), "citation-first-colon");
        assert_eq!(documented_only.inline_note_layout_check, "off");
        assert!(!layout_pass_enabled(&documented_only));
        let (has_note, violations) = inline_note_verdicts(&block, 1, &documented_only, &[]);
        assert!(has_note);
        assert!(
            violations.is_empty(),
            "at `off` no line is classified, so none can deviate"
        );

        // Gated: the layout pass judges the lines rule 1 names, so every line is
        // tokenized — and the one the note walk already read is not tokenized twice.
        let gated_config = checked_layout_config(root, "citation-first-colon");
        assert!(layout_pass_enabled(&gated_config));
        let prefixes = comment_strip_prefixes(&gated_config);
        let mut gated = BlockCitations::new(&block, &gated_config, &[]);
        let has_note = block_has_inline_note_memoized(&mut gated, &prefixes);
        assert_eq!(
            filled_slots(&gated),
            1,
            "the note walk stops at the first line that says something"
        );
        assert_eq!(
            inline_layout_violations(&mut gated, &prefixes, 1, has_note),
            vec![3]
        );
        assert_eq!(
            filled_slots(&gated),
            block.len(),
            "the layout pass reads every line of the site"
        );
    }
}
