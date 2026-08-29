/// Test module: what the number-only shorthand *rewrites* — `grund fmt`'s bulk
/// pass and the LSP's live on-type transform (§FS-fmt.2.4, §FS-lsp.1.4,
/// §DF-number-only-citation-shorthand.2.2).
///
/// Split from `tests_shorthand.rs`, which covers what the shorthand *reports*.
/// The two fail for different reasons: a recognition bug shows up as a wrong
/// finding, a rewrite bug as a wrongly edited file, and the second is the one
/// that damages a repository.
#[cfg(test)]
mod tests_shorthand_rewrite {
    use super::tests_support::*;
    use super::*;

    // §FS-fmt.2.4: `fmt` expands what resolves and leaves what does not, and the
    // §FS-fmt.2.3 exclusions still hold — an illustration in inline code and an
    // ID inside a runtime string are not citations to normalize.
    #[test]
    fn fmt_expands_resolvable_shorthands_only() {
        let root = test_root("fmt_expands_resolvable_shorthands_only");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n\n## 1. Inputs\n\nStuff.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let expand = |line: &str, is_md: bool| {
            let mut saw_candidate = false;
            expand_shorthand_citations(
                line,
                DocstringContent::default(),
                &config,
                is_md,
                &ShorthandTargets::new(Some(&findings), None),
                &mut saw_candidate,
                &mut Vec::new(),
            )
            .unwrap_or_else(|| line.to_string())
        };
        assert_eq!(expand("See §FS-042.", true), "See §FS-042-user-login.");
        assert_eq!(expand("See §FS-042.1.", true), "See §FS-042-user-login.1.");
        assert_eq!(expand("Missing §FS-999.", true), "Missing §FS-999.");
        assert_eq!(expand("Already §FS-042-user-login.", true), "Already §FS-042-user-login.");
        assert_eq!(expand("Shown as `§FS-042`.", true), "Shown as `§FS-042`.");
        assert_eq!(
            expand("let s = \"§FS-042\";", false),
            "let s = \"§FS-042\";"
        );
        // Idempotent: a second pass over the expanded line changes nothing.
        assert_eq!(
            expand(&expand("See §FS-042.", true), true),
            "See §FS-042-user-login."
        );
    }

    /// §DF-number-only-citation-shorthand.2.6: the shorthand pattern is anchored at
    /// the start only, so it matches the `FS-042` inside a longer ID-shaped token.
    /// Claiming that prefix and rewriting it splices the canonical slug into the
    /// middle of the author's text and leaves the tail glued on — the token has to
    /// end where the match ends or it is not a shorthand at all.
    ///
    /// Why `/` still expands rather than ending the token: treating it as a
    /// continuation would make the shorthand and the canonical form disagree, and
    /// the shorthand would be the half silently dropped.
    #[test]
    fn a_shorthand_prefix_of_a_longer_token_is_never_rewritten() {
        let root = test_root("a_shorthand_prefix_of_a_longer_token_is_never_rewritten");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let expand = |line: &str, is_md: bool| {
            let mut saw_candidate = false;
            expand_shorthand_citations(
                line,
                DocstringContent::default(),
                &config,
                is_md,
                &ShorthandTargets::new(Some(&findings), None),
                &mut saw_candidate,
                &mut Vec::new(),
            )
            .unwrap_or_else(|| line.to_string())
        };
        // A full ID whose slug this grammar rejects: wrong case, underscores.
        assert_eq!(expand("A §FS-042-User-Login here", true), "A §FS-042-User-Login here");
        assert_eq!(expand("B §FS-042_user_login here", true), "B §FS-042_user_login here");
        // Glued alphanumerics, a version suffix, and a path tail.
        assert_eq!(expand("C §FS-042abc here", true), "C §FS-042abc here");
        assert_eq!(expand("D §FS-042v2 here", true), "D §FS-042v2 here");
        // A section suffix followed by junk is the same case one level in.
        assert_eq!(expand("F §FS-042.1x here", true), "F §FS-042.1x here");

        // `/` is *not* a continuation: it can only precede a kind, never follow a
        // number, and the full-ID pass already reads `§FS-042-user-login/x` as a
        // citation (§DF-number-only-citation-shorthand.2.6).
        assert_eq!(
            expand("E §FS-042/docs/x.md", true),
            "E §FS-042-user-login/docs/x.md"
        );

        // The terminators that *do* end a token still expand: end of line, a
        // sentence period, a space, and a closing bracket.
        assert_eq!(expand("G §FS-042", true), "G §FS-042-user-login");
        assert_eq!(expand("H §FS-042.", true), "H §FS-042-user-login.");
        assert_eq!(expand("I §FS-042 x", true), "I §FS-042-user-login x");
        assert_eq!(expand("J (§FS-042)", true), "J (§FS-042-user-login)");
    }

    /// §FS-fmt.3: a line that expands a shorthand names the text it will write, so
    /// the invention can be reviewed *before* `--write` puts it on disk. The other
    /// rewrites leave the ID token byte-identical and `check` can still catch them;
    /// this one writes the slug into the token, and a wrong one is invisible
    /// afterwards (§DF-shorthand-numeric-run.2.7).
    #[test]
    fn the_report_names_the_text_every_expansion_writes() {
        let root = test_root("the_report_names_the_text_every_expansion_writes");
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

        let mut expansions = Vec::new();
        let mut saw_candidate = false;
        expand_shorthand_citations(
            "See §FS-042.1 and §FS-043.",
            DocstringContent::default(),
            &config,
            true,
            &ShorthandTargets::new(Some(&findings), None),
            &mut saw_candidate,
            &mut expansions,
        )
        .expect("line rewritten");
        // Source order, and the section rides along with the ID it belongs to.
        assert_eq!(
            expansions,
            vec![
                ("§FS-042.1".to_string(), "§FS-042-user-login.1".to_string()),
                ("§FS-043".to_string(), "§FS-043-user-logout".to_string()),
            ]
        );
    }

    // §FS-fmt.2.4: a qualified `§<alias>/FS-042` is rewritten too, against the
    // *aliased* project's declarations — the spec promises the namespace is
    // preserved, and without it `check` would name an error `fmt` never clears.
    #[test]
    fn fmt_expands_a_qualified_shorthand_against_its_own_namespace() {
        let root = test_root("fmt_expands_a_qualified_shorthand_against_its_own_namespace");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"api\", \"web\"]\ninclude_root = false\n",
        );
        write(
            &root.join("api/grund.toml"),
            "grund_config_version = 1\nproject_name = \"api\"\n\n[id]\nformat = \"{kind}-{number}-{slug}\"\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("api/docs/FS-042-session.md"),
            "# FS-042-session: Session\n\nLead.\n",
        );
        write(
            &root.join("web/grund.toml"),
            "grund_config_version = 1\nproject_name = \"web\"\n\n[id]\nformat = \"{kind}-{number}-{slug}\"\n\n[scan]\ninclude = [\"docs\"]\n\n[fmt.cross_refs]\nenabled = false\n",
        );
        write(&root.join("web/docs/notes.md"), "Cross: §api/FS-042\n");

        let output = format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            ..FmtOpts::default()
        })
        .expect("fmt");
        assert_eq!(output.changes.len(), 1, "{:?}", output.changes);
        assert_eq!(
            output.changes[0].label,
            "shorthand \u{2192} canonical: \u{a7}api/FS-042 \u{2192} \u{a7}api/FS-042-session"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("web/docs/notes.md")).expect("read"),
            "Cross: §api/FS-042-session\n"
        );
    }

    // §FS-fmt.2.4 / §FS-lsp.1.4: a typed trigger lands on the canonical form in
    // one pass — the trigger rewrite runs first and the shorthand pass reads its
    // output, so the author never sees an intermediate `§FS-042`.
    #[test]
    fn fmt_expands_a_typed_trigger_shorthand_in_one_pass() {
        let root = test_root("fmt_expands_a_typed_trigger_shorthand_in_one_pass");
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        let config = numbered_config(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");

        let (line, label) = fmt_line(
            "Typed $$FS-042 here.",
            DocstringCursor::default(),
            &root.join("docs/notes.md"),
            &config,
            true,
            &FmtLineOpts {
                add_marker: false,
                cross_refs: false,
                index_entry_ids: None,
                findings: Some(&findings),
                workspace: None,
                shorthand_targets: &ShorthandTargets::new(Some(&findings), None),
            },
            &mut false,
        );
        assert_eq!(line, "Typed §FS-042-user-login here.");
        assert_eq!(
            label,
            "trigger \u{2192} marker: \u{a7}FS-042 \u{2192} \u{a7}FS-042-user-login"
        );

        let (line, label) = fmt_line(
            "Persisted §FS-042 here.",
            DocstringCursor::default(),
            &root.join("docs/notes.md"),
            &config,
            true,
            &FmtLineOpts {
                add_marker: false,
                cross_refs: false,
                index_entry_ids: None,
                findings: Some(&findings),
                workspace: None,
                shorthand_targets: &ShorthandTargets::new(Some(&findings), None),
            },
            &mut false,
        );
        assert_eq!(line, "Persisted §FS-042-user-login here.");
        assert_eq!(
            label,
            "shorthand \u{2192} canonical: \u{a7}FS-042 \u{2192} \u{a7}FS-042-user-login"
        );
    }

    /// Replay `typed` one character at a time through `on_type_line_edits`,
    /// applying the returned edits exactly as an LSP client would. This is the
    /// only honest way to test the live transform: a single call with the whole
    /// token already in place never exercises the keystroke the editor actually
    /// sends, and that gap is what hid the trigger being consumed mid-number.
    fn type_line(path: &Path, prefix: &str, typed: &str, declared: &[(PathBuf, &str)]) -> String {
        type_line_in(path, prefix, typed, declared, "")
    }

    /// As `type_line`, but with `before` standing in for the document above the
    /// edited line — the context a fenced block needs.
    fn type_line_in(
        path: &Path,
        prefix: &str,
        typed: &str,
        declared: &[(PathBuf, &str)],
        before: &str,
    ) -> String {
        let declarations: Vec<DeclaredId<'_>> = declared
            .iter()
            .map(|(path, id)| DeclaredId {
                path: path.as_path(),
                id,
            })
            .collect();
        let line_index = before.lines().count();
        let mut line = String::from(prefix);
        for ch in typed.chars() {
            line.push(ch);
            let text = format!("{before}{line}");
            let edits = on_type_line_edits(path, &text, line_index, line.len(), &declarations)
                .expect("on-type edits");
            // Highest offset first, so an earlier edit's span stays valid.
            for edit in edits.iter().rev() {
                line.replace_range(edit.start..edit.end, &edit.text);
            }
        }
        line
    }

    /// §FS-lsp.1.4: typing `$$FS-042` lands on the canonical ID. The expansion
    /// fires on the keystroke that *ends* the token, not on the one that first
    /// makes it parse — under the default format that is the first digit, and
    /// expanding there rewrites a number the author has not finished typing.
    #[test]
    fn on_type_expands_a_shorthand_when_the_token_ends() {
        let root = test_root("on_type_expands_a_shorthand_when_the_token_ends");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{number}-{slug}\"\n",
        );
        let path = root.join("docs/notes.md");
        write(&path, "\n");
        let home = root.join("docs/functional-spec/FS-042-user-login.md");
        let declared = vec![
            (home.clone(), "FS-042-user-login"),
            (home.clone(), "FS-001-login"),
            (home.clone(), "FS-012-logout"),
        ];

        // Typed left to right and finished with a space — the whole point.
        assert_eq!(
            type_line(&path, "See ", "$$FS-042 x", &declared),
            "See §FS-042-user-login x"
        );
        // A sentence period ends the token just as well, and the trailing `.`
        // survives so a following digit still reads as a section.
        assert_eq!(
            type_line(&path, "See ", "$$FS-042.", &declared),
            "See §FS-042-user-login."
        );
        assert_eq!(
            type_line(&path, "See ", "$$FS-042.1 ", &declared),
            "See §FS-042-user-login.1 "
        );
        // A number whose *prefix* names another declaration must not be rewritten
        // on the way past it: `FS-1` names `FS-001-login`, but the author is
        // typing `FS-12`.
        assert_eq!(
            type_line(&path, "See ", "$$FS-12 x", &declared),
            "See §FS-012-logout x"
        );
        // A full ID is not a shorthand and is never rewritten.
        assert_eq!(
            type_line(&path, "See ", "$$FS-042-user-login x", &declared),
            "See §FS-042-user-login x"
        );
        // Nothing declared: the trigger still converts, so typing never stalls,
        // and §FS-check.3.13 is what names the problem.
        assert_eq!(type_line(&path, "See ", "$$FS-777 x", &[]), "See §FS-777 x");
    }

    /// §FS-fmt.2.4 / §FS-workspace.1: a qualified citation's ID tail is parsed with
    /// the *target* project's grammar, never the citing project's. The scanner
    /// already routes it that way; a rewrite that used the citing grammar would
    /// edit tokens `check` never saw and skip the ones it reported — visible only
    /// in a workspace whose members disagree about `[id] format`.
    #[test]
    fn a_qualified_shorthand_is_matched_with_the_targets_grammar() {
        let root = test_root("a_qualified_shorthand_is_matched_with_the_targets_grammar");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"api\", \"web\"]\ninclude_root = false\n",
        );
        // `api` numbers with dots, `web` with dashes.
        write(
            &root.join("api/grund.toml"),
            "grund_config_version = 1\nproject_name = \"api\"\n\n[id]\nformat = \"{kind}.{number}.{slug}\"\nsection_separator = \"#\"\n\n[scan]\ninclude = [\"docs\"]\n\n[fmt.cross_refs]\nenabled = false\n",
        );
        write(
            &root.join("api/docs/FS.042.session.md"),
            "# FS.042.session: Session\n\nLead.\n",
        );
        write(
            &root.join("web/grund.toml"),
            "grund_config_version = 1\nproject_name = \"web\"\n\n[id]\nformat = \"{kind}-{number}-{slug}\"\n\n[scan]\ninclude = [\"docs\"]\n\n[fmt.cross_refs]\nenabled = false\n",
        );
        write(
            &root.join("web/docs/notes.md"),
            "Target shape: §api/FS.042 here.\nCiting shape: §api/FS-042 here.\n",
        );

        let output = format_references(FmtOpts {
            path: root.clone(),
            path_provided: true,
            write: true,
            ..FmtOpts::default()
        })
        .expect("fmt");
        assert_eq!(output.changes.len(), 1, "{:?}", output.changes);
        assert_eq!(output.changes[0].line, 1);
        // Line 1 is the citation; line 2 is not a token any pass recognizes, so
        // it must survive byte-for-byte.
        assert_eq!(
            std::fs::read_to_string(root.join("web/docs/notes.md")).expect("read"),
            "Target shape: §api/FS.042.session here.\nCiting shape: §api/FS-042 here.\n"
        );
    }

    /// §FS-lsp.1.4: the live transform refuses wherever the bulk pass refuses. A
    /// fenced block and a declaration heading are whole-*line* skips in `fmt`
    /// (§FS-fmt.2.3), which is why the on-type entry point takes the document
    /// rather than one line — without the lines above, the editor would silently
    /// rewrite an illustration inside a fence.
    #[test]
    fn on_type_refuses_the_lines_fmt_refuses() {
        let root = test_root("on_type_refuses_the_lines_fmt_refuses");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{number}-{slug}\"\n",
        );
        let path = root.join("docs/notes.md");
        write(&path, "\n");
        let home = root.join("docs/functional-spec/FS-042-user-login.md");
        let declared = vec![(home.clone(), "FS-042-user-login")];

        // Ordinary prose expands.
        assert_eq!(
            type_line_in(&path, "See ", "$$FS-042 x", &declared, "Intro.\n"),
            "See §FS-042-user-login x"
        );
        // The same keystrokes inside a fenced block do not.
        assert_eq!(
            type_line_in(&path, "See ", "$$FS-042 x", &declared, "Intro.\n```\n"),
            "See §FS-042 x"
        );
        // …and expand again once the fence has closed.
        assert_eq!(
            type_line_in(&path, "See ", "$$FS-042 x", &declared, "```\ncode\n```\n"),
            "See §FS-042-user-login x"
        );
        // A declaration heading is a whole-line skip too.
        assert_eq!(
            type_line_in(&path, "# FS-100-thing: See ", "$$FS-042 x", &declared, ""),
            "# FS-100-thing: See §FS-042 x"
        );
    }

    /// §FS-lsp.1.4: scoping the expansion to the edited file's project compares
    /// paths, and two spellings can name one directory — a symlinked root here,
    /// `/var` vs `/private/var` on macOS, a `\\?\` prefix on Windows. A raw prefix
    /// test silently filters every candidate out and the expansion just never
    /// fires, which is the shape this failed in on two platforms while Linux
    /// passed. Unix-only because it needs a symlink to build the mismatch.
    #[cfg(unix)]
    #[test]
    fn expansion_survives_a_root_reached_by_another_path() {
        let root = test_root("expansion_survives_a_root_reached_by_another_path");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{number}-{slug}\"\n",
        );
        let home = root.join("docs/functional-spec/FS-042-user-login.md");
        write(&home, "# FS-042-user-login: User login\n\nLead.\n");

        // A second spelling of the very same directory.
        let link = root.with_file_name(format!(
            "{}-link",
            root.file_name().expect("test root name").to_string_lossy()
        ));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&root, &link).expect("symlink the test root");

        // Both the document and the declarations are reached through the link,
        // while config discovery resolves the root to its real path — exactly the
        // mismatch an editor URI and a discovered config root can carry.
        let path = link.join("docs/notes.md");
        let declared = vec![(
            link.join("docs/functional-spec/FS-042-user-login.md"),
            "FS-042-user-login",
        )];
        assert_eq!(
            type_line_in(&path, "See ", "$$FS-042 x", &declared, "Intro.\n"),
            "See §FS-042-user-login x"
        );
    }

    // §FS-lsp.1.4: an ambiguous shorthand is never guessed at, and an expansion
    // is scoped to the edited file's own project — a sibling workspace member's
    // declarations must neither supply the answer nor suppress it.
    #[test]
    fn on_type_expansion_is_scoped_and_never_guesses() {
        let root = test_root("on_type_expansion_is_scoped_and_never_guesses");
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[id]\nformat = \"{kind}-{number}-{slug}\"\n",
        );
        let path = root.join("docs/notes.md");
        write(&path, "\n");
        let home = root.join("docs/functional-spec/FS-042-user-login.md");

        // Two declarations share the number: the trigger converts, nothing else.
        let ambiguous = vec![
            (home.clone(), "FS-042-user-login"),
            (home.clone(), "FS-042-user-logout"),
        ];
        assert_eq!(
            type_line(&path, "See ", "$$FS-042 x", &ambiguous),
            "See §FS-042 x"
        );

        // The only candidate lives outside this file's project root, so it is not
        // this file's `FS-042` and must not be substituted for one.
        let foreign = vec![(
            root.parent()
                .expect("test root has a parent")
                .join("other-project/docs/FS-042-elsewhere.md"),
            "FS-042-elsewhere",
        )];
        assert_eq!(
            type_line(&path, "See ", "$$FS-042 x", &foreign),
            "See §FS-042 x"
        );
    }
}
