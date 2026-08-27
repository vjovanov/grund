/// Test module: the opt-in inline note layout check — its two config keys, the
/// channel each level speaks through, and the sentence the managed entrypoint
/// block teaches (§FS-inline-citation-style.4.4, §FS-inline-citation-style.5)
#[cfg(test)]
mod tests_inline_note_layout_check {
    use super::tests_support::*;
    use super::*;

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
                "// Walks the credential store.\n",
                "// §FS-001-login: one error per expired credential.\n",
                "// §FS-001-login — and one more, laid out wrong.\n",
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
    // no existing managed block drifts. The doc-comment sentence closes the copy
    // at every `inline_style`, after whatever the other keys produced.
    #[test]
    fn agents_sentence_teaches_the_configured_layout() {
        let root = test_root("agents_sentence_teaches_the_configured_layout");
        let any = layout_config(root.clone(), "any");
        assert_eq!(
            inline_citation_style_sentence(&any),
            "Inline notes: ≤ 1 line preferred, hard cap 3 lines; ≤ 100 columns. Doc-comments (`///`, `//!`, `/** */`, a docstring, a comment right above a definition) are documentation, not notes: they are never measured, so cite in-sentence there."
        );

        let mut colon = layout_config(root.clone(), "citation-first-colon");
        colon.inline_note_layout_check = "error".into();
        assert_eq!(
            inline_citation_style_sentence(&colon),
            "Inline notes: ≤ 1 line preferred, hard cap 3 lines; ≤ 100 columns. Lay each note out citation-first: `// §<ID>: <note>` (several citations: `// §<ID>, §<ID>: <note>`). Doc-comments (`///`, `//!`, `/** */`, a docstring, a comment right above a definition) are documentation, not notes: they are never measured, so cite in-sentence there."
        );

        // The enforcement level is not an instruction: `off` renders the same
        // sentence `error` does.
        let mut off = layout_config(root.clone(), "citation-first-colon");
        off.marker = "@".into();
        assert!(inline_citation_style_sentence(&off).contains("`// @<ID>: <note>`"));

        // A style that permits no note at all has no layout to teach — but the
        // doc-comment sentence closes this style too (§FS-inline-citation-style.5).
        let mut citation_only = layout_config(root, "citation-first-colon");
        citation_only.inline_style = "citation-only".into();
        assert_eq!(
            inline_citation_style_sentence(&citation_only),
            "Inline citations carry no prose — put rationale in the spec. Doc-comments (`///`, `//!`, `/** */`, a docstring, a comment right above a definition) are documentation, not notes: they are never measured, so cite in-sentence there."
        );
        assert!(
            inline_citation_style_sentence(&citation_only)
                .contains("are documentation, not notes"),
            "the doc-comment sentence must close `citation-only` too"
        );
    }
}
