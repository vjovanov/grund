/// Test module: the **marker** languages of the doc-comment recognizer — the
/// ones that spell documentation with a marker of their own — their corners, and
/// the two style surfaces the exemption keeps off a doc comment
/// (§FS-inline-citation-style.1.1, §AR-scanner.4). The position languages, whose
/// doc comment is spelled like any other comment, are in
/// `tests_comment_block_position.rs`.
#[cfg(test)]
mod tests_comment_block {
    use super::tests_support::*;

    #[test]
    fn rust_measures_slash_slash_but_not_slash_slash_slash_or_bang() {
        let findings = inline_style_findings(
            "rust_measures_slash_slash_but_not_slash_slash_slash_or_bang",
            "src/lib.rs",
            concat!(
                "//! Module doc citing §FS-001-login in a sentence\n",
                "//! that wraps to a second line,\n",
                "//! and a third,\n",
                "//! and a fourth.\n",
                "\n",
                "/// Documents §FS-001-login in a sentence\n",
                "/// that wraps to a second line,\n",
                "/// and a third,\n",
                "/// and a fourth.\n",
                "pub fn login() {}\n",
                "\n",
                "// Notes §FS-001-login in a comment\n",
                "// that wraps to a second line,\n",
                "// and a third,\n",
                "// and a fourth.\n",
                "pub fn relogin() {}\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(12, 4, "§FS-001-login"));
    }

    #[test]
    fn java_measures_a_plain_block_comment_but_not_a_javadoc() {
        let findings = inline_style_findings(
            "java_measures_a_plain_block_comment_but_not_a_javadoc",
            "src/App.java",
            concat!(
                "/**\n",
                " * Documents §FS-001-login in a sentence\n",
                " * that wraps to a second line,\n",
                " * and a third.\n",
                " */\n",
                "class App {}\n",
                "\n",
                "/*\n",
                " * Notes §FS-001-login in a comment\n",
                " * that wraps to a second line,\n",
                " * and a third.\n",
                " */\n",
                "class Other {}\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(8, 5, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: exactly three slashes. A `////` run is a
    /// rule drawn across the file, and Rust does not treat it as documentation
    /// either.
    #[test]
    fn a_four_slash_run_is_a_rule_line_not_a_doc_comment() {
        let findings = inline_style_findings(
            "a_four_slash_run_is_a_rule_line_not_a_doc_comment",
            "src/lib.rs",
            concat!(
                "//// Notes §FS-001-login in a banner\n",
                "//// that wraps to a second line,\n",
                "//// and a third,\n",
                "//// and a fourth.\n",
                "pub fn login() {}\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(1, 4, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: `/**/` is the empty block comment, not a
    /// Javadoc opener. Measured on the column cap, because a one-line block can
    /// never be over the line cap.
    #[test]
    fn an_empty_block_comment_is_not_a_javadoc_opener() {
        let padding = "x".repeat(100);
        let findings = inline_style_findings(
            "an_empty_block_comment_is_not_a_javadoc_opener",
            "src/App.java",
            &format!("/**/ §FS-001-login {padding} */\nclass App {{}}\n"),
        );
        assert_eq!(findings, over_the_column_cap(1, 122, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: `/**` and `/*!` both open documentation,
    /// and neither is measured however wide it runs.
    #[test]
    fn a_javadoc_or_bang_block_opener_is_a_doc_comment() {
        let padding = "x".repeat(100);
        let findings = inline_style_findings(
            "a_javadoc_or_bang_block_opener_is_a_doc_comment",
            "src/App.java",
            &format!(
                "/** §FS-001-login {padding} */\nclass App {{}}\n\n/*! §FS-001-login {padding} */\nclass Other {{}}\n"
            ),
        );
        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn python_measures_a_hash_block_but_not_a_docstring() {
        let findings = inline_style_findings(
            "python_measures_a_hash_block_but_not_a_docstring",
            "src/auth.py",
            concat!(
                "def login():\n",
                "    \"\"\"Documents §FS-001-login in a sentence\n",
                "    that wraps to a second line,\n",
                "    and a third,\n",
                "    and a fourth.\"\"\"\n",
                "\n",
                "\n",
                "# Notes §FS-001-login in a comment\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "x = 1\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(8, 4, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: PEP 257 says only a docstring is
    /// documentation, so a `#` block keeps its budget even directly above the
    /// `def` it describes — position decides nothing in a marker language.
    #[test]
    fn a_python_hash_block_above_a_def_is_still_measured() {
        let findings = inline_style_findings(
            "a_python_hash_block_above_a_def_is_still_measured",
            "src/auth.py",
            concat!(
                "# Notes §FS-001-login in a comment\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "def login():\n",
                "    pass\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(1, 4, "§FS-001-login"));
    }

    #[test]
    fn lua_measures_two_dashes_but_not_three() {
        let findings = inline_style_findings(
            "lua_measures_two_dashes_but_not_three",
            "src/mod.lua",
            concat!(
                "--- Documents §FS-001-login in a sentence\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "function login() end\n",
                "\n",
                "-- Notes §FS-001-login in a comment\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "function relogin() end\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(7, 4, "§FS-001-login"));
    }

    #[test]
    fn haskell_measures_a_plain_dash_run_but_not_a_haddock_bar() {
        let findings = inline_style_findings(
            "haskell_measures_a_plain_dash_run_but_not_a_haddock_bar",
            "src/Auth.hs",
            concat!(
                "-- | Documents §FS-001-login in a sentence\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "login = ()\n",
                "\n",
                "-- Notes §FS-001-login in a comment\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "relogin = ()\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(7, 4, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: Haddock's other marker documents what
    /// *precedes* the comment, and is a doc comment for the same reason.
    #[test]
    fn haskell_reads_a_haddock_caret_as_a_doc_comment_too() {
        let findings = inline_style_findings(
            "haskell_reads_a_haddock_caret_as_a_doc_comment_too",
            "src/Auth.hs",
            concat!(
                "login = ()\n",
                "-- ^ Documents §FS-001-login in a sentence\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
            ),
        );
        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn r_measures_a_hash_block_but_not_a_roxygen_run() {
        let findings = inline_style_findings_with(
            "r_measures_a_hash_block_but_not_a_roxygen_run",
            "src/auth.R",
            concat!(
                "#' Documents §FS-001-login in a sentence\n",
                "#' that wraps to a second line,\n",
                "#' and a third,\n",
                "#' and a fourth.\n",
                "login <- function() NULL\n",
                "\n",
                "# Notes §FS-001-login in a comment\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "relogin <- function() NULL\n",
            ),
            // `R` is outside the default `[scan] extensions`.
            |config| config.extensions.push("R".to_string()),
        );
        assert_eq!(findings, over_the_line_cap(7, 4, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: an extension neither table names has no
    /// doc-comment notion, so every one of its blocks is measured — which is
    /// what every extension did before this rule existed.
    #[test]
    fn an_unknown_extension_measures_every_block() {
        let findings = inline_style_findings(
            "an_unknown_extension_measures_every_block",
            "src/auth.ada",
            concat!(
                "-- Documents §FS-001-login in a sentence\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "procedure Login is begin null; end Login;\n",
                "\n",
                "-- Notes §FS-001-login in a comment\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "procedure Relogin is begin null; end Relogin;\n",
            ),
        );
        assert_eq!(
            findings,
            vec![
                over_the_line_cap(1, 4, "§FS-001-login")[0].clone(),
                over_the_line_cap(7, 4, "§FS-001-login")[0].clone(),
            ]
        );
    }

    /// §FS-inline-citation-style.1.1: `citation-only` does not reach a doc
    /// comment either — a Javadoc cannot be a pure pointer, and it was never
    /// asked to be. The plain block below it shows the setting is live.
    #[test]
    fn citation_only_does_not_reach_a_javadoc() {
        let findings = inline_style_findings_with(
            "citation_only_does_not_reach_a_javadoc",
            "src/App.java",
            concat!(
                "/**\n",
                " * Documents §FS-001-login and says something about it.\n",
                " */\n",
                "class App {}\n",
                "\n",
                "/* §FS-001-login and a note beside the clause */\n",
                "class Other {}\n",
            ),
            |config| config.inline_style = "citation-only".to_string(),
        );
        assert_eq!(
            findings,
            vec![(6, "inline citation must carry no prose".to_string())]
        );
    }

    /// §FS-inline-citation-style.1.1: the note layout does not reach a doc
    /// comment either, so a Rustdoc summary line above a cited sentence is not a
    /// badly laid-out note. The `//` block below it shows the gate is live.
    #[test]
    fn the_note_layout_check_does_not_reach_a_doc_comment() {
        let findings = inline_style_findings_with(
            "the_note_layout_check_does_not_reach_a_doc_comment",
            "src/lib.rs",
            concat!(
                "/// Walks the credential store.\n",
                "/// §FS-001-login — and a sentence laid out as prose.\n",
                "pub fn login() {}\n",
                "\n",
                "// Walks the credential store.\n",
                "// §FS-001-login — and a note laid out wrong.\n",
                "pub fn relogin() {}\n",
            ),
            |config| {
                config.inline_note_layout = "citation-first-colon".to_string();
                config.inline_note_layout_check = "error".to_string();
            },
        );
        assert_eq!(
            findings,
            vec![(
                6,
                "inline note must open with its citations and a colon (§<ID>: note)".to_string()
            )]
        );
    }
}
