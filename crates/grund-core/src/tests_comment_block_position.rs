/// Test module: the **position** languages of the doc-comment recognizer — Go,
/// Ruby, shell and SQL, which spell a doc comment like any other comment, so the
/// line under it and the top of the file decide
/// (§FS-inline-citation-style.1.1, §AR-scanner.4). The marker languages are in
/// `tests_comment_block.rs`.
#[cfg(test)]
mod tests_comment_block_position {
    use super::tests_support::*;

    #[test]
    fn go_measures_a_comment_that_is_not_above_a_definition() {
        let findings = inline_style_findings(
            "go_measures_a_comment_that_is_not_above_a_definition",
            "src/auth.go",
            concat!(
                "package auth\n",
                "\n",
                "// Documents §FS-001-login in a sentence\n",
                "// that wraps to a second line,\n",
                "// and a third,\n",
                "// and a fourth.\n",
                "func Login() {}\n",
                "\n",
                "func Relogin() {\n",
                "\t// Notes §FS-001-login in a comment\n",
                "\t// that wraps to a second line,\n",
                "\t// and a third,\n",
                "\t// and a fourth.\n",
                "\trelog()\n",
                "}\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(10, 4, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: the definition has to be the *next* line.
    /// A blank line between makes the block a detached note, which is what a
    /// blank line means to a reader too.
    #[test]
    fn a_blank_line_before_the_definition_makes_the_block_a_note() {
        let adjacent = concat!(
            "package auth\n",
            "\n",
            "// Documents §FS-001-login in a sentence\n",
            "// that wraps to a second line,\n",
            "// and a third,\n",
            "// and a fourth.\n",
            "func Login() {}\n",
        );
        assert_eq!(
            inline_style_findings(
                "a_blank_line_before_the_definition_makes_the_block_a_note_adjacent",
                "src/auth.go",
                adjacent,
            ),
            Vec::new()
        );

        let detached = concat!(
            "package auth\n",
            "\n",
            "// Notes §FS-001-login in a comment\n",
            "// that wraps to a second line,\n",
            "// and a third,\n",
            "// and a fourth.\n",
            "\n",
            "func Login() {}\n",
        );
        assert_eq!(
            inline_style_findings(
                "a_blank_line_before_the_definition_makes_the_block_a_note_detached",
                "src/auth.go",
                detached,
            ),
            over_the_line_cap(3, 4, "§FS-001-login")
        );
    }

    /// §FS-inline-citation-style.1.1: leading whitespace is stripped before the
    /// starter test, so a `def` indented inside a `class` still documents.
    #[test]
    fn ruby_reads_an_indented_def_as_a_definition() {
        let findings = inline_style_findings(
            "ruby_reads_an_indented_def_as_a_definition",
            "src/auth.rb",
            concat!(
                "class Auth\n",
                "  # Documents §FS-001-login in a sentence\n",
                "  # that wraps to a second line,\n",
                "  # and a third,\n",
                "  # and a fourth.\n",
                "  def login\n",
                "    # Notes §FS-001-login in a comment\n",
                "    # that wraps to a second line,\n",
                "    # and a third,\n",
                "    # and a fourth.\n",
                "    relog\n",
                "  end\n",
                "end\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(7, 4, "§FS-001-login"));
    }

    #[test]
    fn shell_reads_a_name_paren_definition() {
        let findings = inline_style_findings_with(
            "shell_reads_a_name_paren_definition",
            "src/tool.sh",
            concat!(
                "set -euo pipefail\n",
                "\n",
                "# Documents §FS-001-login in a sentence\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "login() {\n",
                "  relog\n",
                "}\n",
                "\n",
                "# Notes §FS-001-login in a comment\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "relog\n",
            ),
            // `sh` is outside the default `[scan] extensions`.
            |config| config.extensions.push("sh".to_string()),
        );
        assert_eq!(findings, over_the_line_cap(11, 4, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: shell's other spelling, and the identifier
    /// boundary that keeps `functional_helper` from opening one.
    #[test]
    fn shell_reads_the_function_keyword_but_not_a_word_starting_with_it() {
        let findings = inline_style_findings_with(
            "shell_reads_the_function_keyword_but_not_a_word_starting_with_it",
            "src/tool.sh",
            concat!(
                "set -euo pipefail\n",
                "\n",
                "# Documents §FS-001-login in a sentence\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "function login {\n",
                "  relog\n",
                "}\n",
                "\n",
                "# Notes §FS-001-login in a comment\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "functional_helper\n",
            ),
            |config| config.extensions.push("sh".to_string()),
        );
        assert_eq!(findings, over_the_line_cap(11, 4, "§FS-001-login"));
    }

    #[test]
    fn sql_reads_create_in_either_case() {
        let findings = inline_style_findings(
            "sql_reads_create_in_either_case",
            "src/schema.sql",
            concat!(
                "SET search_path = public;\n",
                "\n",
                "-- Documents §FS-001-login in a sentence\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "CREATE TABLE login (id int);\n",
                "\n",
                "-- Documents §FS-001-login once more, in a sentence\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "create or replace function login_count() returns int as 'select 1';\n",
                "\n",
                "-- Notes §FS-001-login in a comment\n",
                "-- that wraps to a second line,\n",
                "-- and a third,\n",
                "-- and a fourth.\n",
                "INSERT INTO login VALUES (1);\n",
            ),
        );
        assert_eq!(findings, over_the_line_cap(15, 4, "§FS-001-login"));
    }

    /// §FS-inline-citation-style.1.1: a position language's file header is its
    /// module doc — with a `#!` shebang above it, or with nothing but blank
    /// lines.
    #[test]
    fn the_leading_comment_of_a_position_language_documents_the_file() {
        let with_shebang = concat!(
            "#!/usr/bin/env bash\n",
            "\n",
            "# Documents §FS-001-login in a sentence\n",
            "# that wraps to a second line,\n",
            "# and a third,\n",
            "# and a fourth.\n",
            "echo hi\n",
        );
        assert_eq!(
            inline_style_findings_with(
                "the_leading_comment_of_a_position_language_documents_the_file_shebang",
                "src/tool.sh",
                with_shebang,
                |config| config.extensions.push("sh".to_string()),
            ),
            Vec::new()
        );

        let without_shebang = concat!(
            "\n",
            "# Documents §FS-001-login in a sentence\n",
            "# that wraps to a second line,\n",
            "# and a third,\n",
            "# and a fourth.\n",
            "echo hi\n",
        );
        assert_eq!(
            inline_style_findings_with(
                "the_leading_comment_of_a_position_language_documents_the_file_blank",
                "src/tool.sh",
                without_shebang,
                |config| config.extensions.push("sh".to_string()),
            ),
            Vec::new()
        );
    }

    /// §FS-inline-citation-style.1.1: once a line of code has gone by, a block
    /// is no longer the file's leading comment.
    #[test]
    fn a_block_below_the_first_line_of_code_is_not_the_leading_comment() {
        let findings = inline_style_findings_with(
            "a_block_below_the_first_line_of_code_is_not_the_leading_comment",
            "src/tool.sh",
            concat!(
                "echo start\n",
                "\n",
                "# Notes §FS-001-login in a comment\n",
                "# that wraps to a second line,\n",
                "# and a third,\n",
                "# and a fourth.\n",
                "echo hi\n",
            ),
            |config| config.extensions.push("sh".to_string()),
        );
        assert_eq!(findings, over_the_line_cap(3, 4, "§FS-001-login"));
    }
}
