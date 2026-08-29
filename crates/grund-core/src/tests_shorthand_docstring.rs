/// Test module: a Python docstring's `"""` / `'''` is doc-comment syntax, not a
/// quote, so every never-rewrite surface judges a docstring line on its content
/// (§FS-fmt.2.3.1, §FS-check.3.13, §FS-fmt.2.4, §FS-lsp.1.4).
///
/// Its own module rather than more cases in `tests_shorthand.rs` or
/// `tests_shorthand_rewrite.rs`, because the behaviour under test is not what the
/// shorthand reports or what it rewrites but that the *three* surfaces agree:
/// nearly every case here asserts `check`'s reported set against `fmt --check`'s
/// rewrite set on one fixture, which is the shape §AR-scanner.2.6 calls "one
/// predicate serving both" and the shape neither of those modules has.
#[cfg(test)]
mod tests_shorthand_docstring {
    use super::tests_support::*;
    use super::*;

    /// A repo scoped to `docs` and `src`, declaring `FS-042-user-login`, with one
    /// Python file. `scan` appends extra `[scan]` keys — `docstring_python =
    /// false` is the one case that needs it.
    fn docstring_repo(name: &str, source: &str, scan: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n\n\
                 [[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n\n\
                 [scan]\ninclude = [\"docs\", \"src\"]\nextensions = [\"md\", \"py\"]\n{scan}"
            ),
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(&root.join("src/render.py"), source);
        root
    }

    /// The lines `check` reports a shorthand on. Every fixture here declares the
    /// target, so the only shorthand finding available is the resolving one.
    fn reported_lines(root: &Path) -> Vec<usize> {
        let run = check_run(root, false);
        let mut lines: Vec<usize> = run
            .report
            .errors
            .iter()
            .chain(run.report.warnings.iter())
            .filter(|finding| finding.code == "shorthand-citation")
            .map(|finding| finding.line.unwrap_or(0))
            .collect();
        lines.sort();
        lines
    }

    /// The lines `fmt --check` offers a shorthand expansion on, and the label it
    /// prints for each — the other half of the pair `reported_lines` has to equal.
    fn rewrite_lines(root: &Path) -> (Vec<usize>, Vec<String>) {
        let output = format_references(FmtOpts {
            path: root.to_path_buf(),
            path_provided: true,
            ..FmtOpts::default()
        })
        .expect("fmt --check");
        let mut changes: Vec<(usize, String)> = output
            .changes
            .into_iter()
            .map(|change| (change.line, change.label))
            .collect();
        changes.sort();
        (
            changes.iter().map(|(line, _)| *line).collect(),
            changes.into_iter().map(|(_, label)| label).collect(),
        )
    }

    /// One docstring, one verdict per line — and the same verdict from both
    /// surfaces (§FS-fmt.2.3.1, §FS-check.3.13). Every shape the rule has to reach
    /// the same answer on is in one file so the two sets can be compared directly:
    /// a multi-line docstring's opening, interior and closing lines; a one-line
    /// docstring; an indented method docstring; `'''`; a docstring whose content
    /// holds a `"quoted"` word before the citation (the two quotes close, so the
    /// site is reported); and a `#` comment, which is what a docstring line is
    /// meant to behave like.
    ///
    /// Silent, and asserted so by absence: a docstring line whose content holds an
    /// apostrophe before the citation, which is exactly as silent as the same
    /// apostrophe in a `#` comment — this fix makes a docstring line behave like a
    /// comment line, and does not change what a comment line does
    /// (§FS-fmt.2.3.1's walk still runs over the content). And a `"…"` literal on
    /// a code line, which is the runtime text the exclusion is about.
    const EVERY_SHAPE: &str = "\"\"\"Opening line cites §FS-042.\n\
                               \n\
                               Interior line cites §FS-042.\n\
                               Closing line cites §FS-042.\"\"\"\n\
                               \n\
                               \n\
                               def method():\n\
                               \x20   \"\"\"One-line docstring cites §FS-042.\"\"\"\n\
                               \x20   return 1\n\
                               \n\
                               \n\
                               def single():\n\
                               \x20   '''Single-quoted opening cites §FS-042.\n\
                               \n\
                               \x20   Interior of the single-quoted one cites §FS-042.\n\
                               \x20   '''\n\
                               \x20   return 2\n\
                               \n\
                               \n\
                               def quoted():\n\
                               \x20   \"\"\"A \"quoted\" word before §FS-042 here.\"\"\"\n\
                               \x20   return 3\n\
                               \n\
                               \n\
                               def apostrophe():\n\
                               \x20   \"\"\"It's per §FS-042 here.\"\"\"\n\
                               \x20   return 4\n\
                               \n\
                               \n\
                               # Comment citing §FS-042.\n\
                               # It's per §FS-042 in a comment.\n\
                               \n\
                               BODY = \"Copy citing §FS-042.\"\n";

    /// The lines of `EVERY_SHAPE` a shorthand finding and a rewrite both land on.
    const EVERY_SHAPE_SITES: [usize; 8] = [1, 3, 4, 8, 13, 15, 21, 30];

    /// §FS-check.3.13 / §FS-fmt.2.4: for every docstring shape, the set `check`
    /// reports equals the set `fmt --check` would rewrite. A site reported but not
    /// rewritten leaves a repository permanently red with nothing to run; a site
    /// rewritten but not reported edits a file `check` never complained about.
    #[test]
    fn check_and_fmt_agree_on_every_docstring_shape() {
        let root = docstring_repo(
            "check_and_fmt_agree_on_every_docstring_shape",
            EVERY_SHAPE,
            "",
        );
        let (rewritten, labels) = rewrite_lines(&root);
        assert_eq!(reported_lines(&root), EVERY_SHAPE_SITES.to_vec());
        assert_eq!(rewritten, EVERY_SHAPE_SITES.to_vec());
        assert!(
            labels.iter().all(|label| label
                == "shorthand \u{2192} canonical: \u{a7}FS-042 \u{2192} \u{a7}FS-042-user-login"),
            "every change names the expansion it will write: {labels:?}"
        );
    }

    /// §FS-fmt.2.4 / §REQ-no-data-loss.2: the rewrite is a splice into the raw
    /// line, so indentation, the delimiters themselves and every line the rule
    /// leaves alone come through byte for byte — including the two silent
    /// apostrophe lines and the string literal on the code line.
    #[test]
    fn fmt_write_expands_docstrings_and_leaves_the_rest_byte_identical() {
        let root = docstring_repo(
            "fmt_write_expands_docstrings_and_leaves_the_rest_byte_identical",
            EVERY_SHAPE,
            "",
        );
        format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            ..FmtOpts::default()
        })
        .expect("fmt --write");

        let written = fs::read_to_string(root.join("src/render.py")).expect("read rewritten file");
        let expected: String = EVERY_SHAPE
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if EVERY_SHAPE_SITES.contains(&(index + 1)) {
                    format!("{}\n", line.replace("§FS-042", "§FS-042-user-login"))
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        assert_eq!(written, expected);
        // …and the tree is clean afterwards: nothing left to report, nothing left
        // to rewrite.
        assert_eq!(reported_lines(&root), Vec::<usize>::new());
        assert_eq!(rewrite_lines(&root).0, Vec::<usize>::new());
    }

    /// §FS-fmt.2.3 / §FS-check.3.13: the half of the ticket that is specified
    /// behaviour and stays. A shorthand inside a `"…"` literal on a **code** line
    /// is a citation — `refs` lists it, it grounds its file — and earns no finding
    /// and no rewrite, because rewriting it would change what the program prints.
    #[test]
    fn a_string_literal_on_a_code_line_stays_silent_and_unrewritten() {
        let source = "BODY = \"Copy citing §FS-042.\"\nOTHER = 'Also §FS-042.'\n";
        let root = docstring_repo(
            "a_string_literal_on_a_code_line_stays_silent_and_unrewritten",
            source,
            "",
        );
        assert_eq!(reported_lines(&root), Vec::<usize>::new());
        assert_eq!(rewrite_lines(&root).0, Vec::<usize>::new());
        // Still a citation, which is what makes the silence a decision rather than
        // a miss: the declaration is cited, so it is not reported unused.
        let findings = scan_findings(&check_run(&root, false).config, &root);
        assert_eq!(
            findings
                .citations
                .iter()
                .filter(|cite| cite.shorthand && !cite.shorthand_rewritable)
                .count(),
            2
        );
    }

    /// §FS-fmt.2.3.1: `docstring_python = false` turns the docstring reading off
    /// entirely, and with it this rule — a `"""` is then just the quote it looks
    /// like, on both surfaces. The unchanged raw-line behaviour: the opening line
    /// and the one-line docstring sit inside a literal, the interior line does
    /// not.
    #[test]
    fn docstring_python_false_keeps_the_raw_line_rule() {
        let source = "\"\"\"Opening cites §FS-042.\n\
                      Interior cites §FS-042.\n\
                      \"\"\"\n\
                      \n\
                      \n\
                      def method():\n\
                      \x20   \"\"\"One-line cites §FS-042.\"\"\"\n\
                      \x20   return 1\n";
        let root = docstring_repo(
            "docstring_python_false_keeps_the_raw_line_rule",
            source,
            "docstring_python = false\n",
        );
        assert_eq!(reported_lines(&root), vec![2]);
        assert_eq!(rewrite_lines(&root).0, vec![2]);
    }

    /// §FS-fmt.2.2 / §FS-check.1.1: the bare-token half of the same split. Off
    /// strict mode the scanner already read a bare ID on a docstring's opening
    /// line against the content and counted it as a citation, while `--marker`
    /// asked the raw line and refused to mark it. Both now read the content, so
    /// `fmt --marker` writes the marker the scanner was already assuming.
    #[test]
    fn marker_pass_marks_a_bare_id_on_a_docstring_opening_line() {
        let root = test_root("marker_pass_marks_a_bare_id_on_a_docstring_opening_line");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [reference]\nstrict = false\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\", \"src\"]\nextensions = [\"md\", \"py\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        write(
            &root.join("src/render.py"),
            "\"\"\"Opening cites FS-042-user-login.\"\"\"\n\
             BODY = \"Copy citing FS-042-user-login.\"\n",
        );
        format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            add_marker: true,
            ..FmtOpts::default()
        })
        .expect("fmt --marker --write");

        assert_eq!(
            fs::read_to_string(root.join("src/render.py")).expect("read rewritten file"),
            "\"\"\"Opening cites §FS-042-user-login.\"\"\"\n\
             BODY = \"Copy citing FS-042-user-login.\"\n",
            "the docstring is marked, the code-line string literal is not"
        );
    }

    /// §FS-fmt.2.3: a declaration heading is one of the two whole lines `fmt`
    /// never rewrites, and the docstring state this fix gives `fmt` is what lets it
    /// see one inside a docstring — where the scanner has always seen it, and where
    /// it records no citation at all. Before, `fmt` read the raw line, found no
    /// heading, and offered to rewrite a shorthand in the heading's own title that
    /// `check` never reported.
    #[test]
    fn fmt_skips_a_declaration_heading_inside_a_docstring() {
        let root = docstring_repo(
            "fmt_skips_a_declaration_heading_inside_a_docstring",
            "class Service:\n\
             \x20   \"\"\"AR-005-service: The service, per §FS-042.\n\
             \n\
             \x20   Body citing §FS-042.\n\
             \x20   \"\"\"\n",
            "",
        );
        // The `AR` home the inline declaration is reached through (§AR-scanner.4).
        write(
            &root.join("docs/architecture/AR-005-service.md"),
            "# AR-005-service: [../../src/render.py](../../src/render.py)\n",
        );
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n\n\
             [[kinds]]\nkind = \"AR\"\nfolder = \"docs/architecture\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\", \"src\"]\nextensions = [\"md\", \"py\"]\n",
        );
        // The heading line is silent on both surfaces; the body line is on neither.
        assert_eq!(reported_lines(&root), vec![4]);
        assert_eq!(rewrite_lines(&root).0, vec![4]);
    }

    /// Replay `typed` one character at a time through `on_type_line_edits` on the
    /// last line of `before`, applying each keystroke's edits as an LSP client
    /// would — the same harness `tests_shorthand_rewrite` uses, kept here because
    /// these cases need the document above the line to say anything at all.
    fn type_after(root: &Path, file: &str, before: &str, typed: &str) -> String {
        let path = root.join(file);
        let declarations = vec![DeclaredId {
            path: root.as_ref(),
            id: "FS-042-user-login",
        }];
        let line_index = before.lines().count();
        let mut line = String::new();
        for ch in typed.chars() {
            line.push(ch);
            let text = format!("{before}{line}");
            let edits = on_type_line_edits(&path, &text, line_index, line.len(), &declarations)
                .expect("on-type edits");
            // Highest offset first, so an earlier edit's span stays valid.
            for edit in edits.iter().rev() {
                line.replace_range(edit.start..edit.end, &edit.text);
            }
        }
        line
    }

    /// §FS-lsp.1.4: the live transform honours every context `grund fmt` refuses
    /// — in both directions. Typing `$$FS-042` and a terminator inside a docstring
    /// lands on the canonical ID, exactly as `fmt --write` would; typing the same
    /// inside a `"…"` literal on a code line converts nothing, exactly as `fmt`
    /// would not. The interior line is here too, because it was already rewritable
    /// and the point is that the opening line now matches it.
    #[test]
    fn on_type_expands_inside_a_docstring_and_refuses_a_code_line_string() {
        let root = docstring_repo(
            "on_type_expands_inside_a_docstring_and_refuses_a_code_line_string",
            "\"\"\"Placeholder.\"\"\"\n",
            "",
        );

        assert_eq!(
            type_after(&root, "src/render.py", "", "\"\"\"Opening $$FS-042 here."),
            "\"\"\"Opening §FS-042-user-login here.",
        );
        assert_eq!(
            type_after(&root, "src/render.py", "\"\"\"\n", "Interior $$FS-042 here."),
            "Interior §FS-042-user-login here.",
        );
        assert_eq!(
            type_after(&root, "src/render.py", "", "BODY = \"Copy $$FS-042 here.\""),
            "BODY = \"Copy $$FS-042 here.\"",
            "a code-line string literal converts neither the trigger nor the shorthand"
        );
    }
}
