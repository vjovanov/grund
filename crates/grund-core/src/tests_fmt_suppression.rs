/// Test module: the two scopes that suppress `fmt` (§FS-fmt.2.5). The e2e cases
/// pin what a run prints and writes; these pin the two recognizers underneath —
/// which paths `[fmt] exclude` claims (§FS-fmt.2.5.1) and which lines are
/// directives (§FS-fmt.2.5.2) — where a case per spelling would be a fixture
/// tree per spelling. The on-type cases at the end are here for the same reason:
/// the LSP live transform honours both scopes (§FS-lsp.1.4), and what it takes to
/// see one is the document above the cursor, not a tree on disk.
#[cfg(test)]
mod tests_fmt_suppression {
    use super::*;
    use super::tests_support::*;

    fn excluded(root: &Path, patterns: &[&str]) -> FmtExcluded {
        let mut config = legacy_fs_folder_config(root.to_path_buf());
        config.fmt_exclude = patterns.iter().map(|pattern| pattern.to_string()).collect();
        FmtExcluded::new(&config).expect("validated patterns build")
    }

    /// §FS-fmt.2.5.1: the patterns are gitignore-style and config-root-relative.
    /// A bare name matches at any depth, a name with a slash is anchored, and a
    /// directory takes everything under it — the dialect `[scan]
    /// respect_gitignore` already brings to the walk.
    #[test]
    fn exclude_reads_a_path_a_glob_and_a_directory() {
        let root = test_root("exclude_reads_a_path_a_glob_and_a_directory");
        for (pattern, claimed, untouched) in [
            (
                "docs/architecture/AR-001-topology.md",
                "docs/architecture/AR-001-topology.md",
                "docs/AR-001-topology.md",
            ),
            ("AR-*.md", "docs/architecture/AR-001-topology.md", "docs/FS-001-login.md"),
            ("docs/diagrams", "docs/diagrams/nested/topology.md", "docs/topology.md"),
            ("**/generated/*.md", "src/generated/notes.md", "src/notes.md"),
        ] {
            let matcher = excluded(&root, &[pattern]);
            assert!(
                matcher.contains(&root.join(claimed)),
                "`{pattern}` must claim {claimed}"
            );
            assert!(
                !matcher.contains(&root.join(untouched)),
                "`{pattern}` must leave {untouched} alone"
            );
        }
    }

    /// §FS-fmt.2.5.1: the verdict is about the file, not about the spelling of the
    /// path that reached it. The walk's paths are built from the config root and
    /// always strip; an LSP client's are the editor's, and the root was
    /// canonicalized when the config loaded, so the two differ wherever a symlink
    /// stands between them — which is every macOS `$TMPDIR` and any repository
    /// reached through a link — and the rewrite would be let through in the one
    /// file the config named (§FS-lsp.1.4).
    #[cfg(unix)]
    #[test]
    fn exclude_claims_a_file_reached_through_a_symlinked_root() {
        let base = physical_test_root("exclude_claims_a_file_reached_through_a_symlinked_root");
        let project = base.join("project");
        write(&project.join("docs/AR-001-topo.md"), "# Topology\n");
        let link = base.join("through-a-link");
        std::os::unix::fs::symlink(&project, &link).expect("symlink the project root");

        let matcher = excluded(&project, &["docs/AR-001-topo.md"]);
        assert!(
            matcher.contains(&link.join("docs/AR-001-topo.md")),
            "the excluded file is excluded however the path reached it"
        );
        assert!(
            !matcher.contains(&link.join("docs/notes.md")),
            "and the resolution claims no file the list did not name"
        );
    }

    /// A config that set no key pays for nothing and claims nothing — the state
    /// every repository written before §FS-config.3.10 existed is in.
    #[test]
    fn an_empty_exclude_claims_no_file() {
        let root = test_root("an_empty_exclude_claims_no_file");
        assert!(!excluded(&root, &[]).contains(&root.join("docs/functional-spec/FS-001-login.md")));
    }

    /// §FS-config.3.10: a malformed glob is rejected where it was written, so
    /// the config load fails rather than the first `grund fmt`.
    #[test]
    fn a_malformed_glob_is_refused_at_config_load() {
        let message = validate_fmt_exclude(&["docs/a{b".to_string()])
            .expect_err("an unclosed alternate group is not a pattern");
        assert!(
            message.starts_with("[fmt] exclude:") && message.contains("docs/a{b"),
            "the rejection names the key and the pattern: {message}"
        );
        assert!(validate_fmt_exclude(&["docs/**/*.md".to_string()]).is_ok());
    }

    /// §FS-fmt.2.5.2: in Markdown the directive is an HTML comment and nothing
    /// else. The near-misses matter more than the hits — an inline code span is
    /// how this repository's own spec names the directive without writing one.
    #[test]
    fn a_markdown_directive_is_an_html_comment_whose_content_is_exact() {
        let config = legacy_fs_folder_config(test_root("markdown_directive"));
        let directives = FmtDirectives::new(&config, true);
        for (line, expected) in [
            ("<!-- grund:fmt off -->", Some(false)),
            ("<!-- grund:fmt on -->", Some(true)),
            ("   <!--grund:fmt   off-->   ", Some(false)),
            ("`<!-- grund:fmt off -->`", None),
            ("<!-- grund:fmt off please -->", None),
            ("<!-- grund:fmt-off -->", None),
            ("<!-- grund:fmt -->", None),
            ("Prose about grund:fmt off in a sentence.", None),
            ("<!-- grund:fmt off --> and then prose", None),
        ] {
            assert_eq!(
                directives.directive(line, DocstringContent::default()),
                expected,
                "markdown directive verdict for {line:?}"
            );
        }
    }

    /// §FS-fmt.2.5.2: in a source file the directive is a comment line under the
    /// configured `[scan] comment_prefixes`. A string holding the same text is
    /// not a comment, which is the case that would otherwise let a fixture or a
    /// test constant silently open a region.
    #[test]
    fn a_source_directive_is_a_comment_line_and_never_a_string() {
        let config = legacy_fs_folder_config(test_root("source_directive"));
        let directives = FmtDirectives::new(&config, false);
        for (line, expected) in [
            ("// grund:fmt off", Some(false)),
            ("    /// grund:fmt on", Some(true)),
            ("# grund:fmt off", Some(false)),
            ("/* grund:fmt off */", Some(false)),
            ("-- grund:fmt on", Some(true)),
            ("let label = \"grund:fmt off\";", None),
            ("run(); // grund:fmt off", None),
            ("// grund:fmt offline", None),
        ] {
            assert_eq!(
                directives.directive(line, DocstringContent::default()),
                expected,
                "source directive verdict for {line:?}"
            );
        }
    }

    /// §FS-fmt.2.5.2: the state machine itself — `off` runs until `on`, a
    /// redundant directive is a no-op, and every file starts with the rewrite on.
    #[test]
    fn a_region_runs_from_off_until_on_and_redundant_directives_are_no_ops() {
        let config = legacy_fs_folder_config(test_root("region_state"));
        let mut directives = FmtDirectives::new(&config, true);
        assert!(directives.rewriting(), "a file starts with the rewrite on");
        assert!(directives.consume("<!-- grund:fmt on -->", DocstringContent::default()));
        assert!(directives.rewriting(), "a stray `on` changes nothing");
        assert!(directives.consume("<!-- grund:fmt off -->", DocstringContent::default()));
        assert!(!directives.rewriting());
        assert!(!directives.consume("Ordinary prose.", DocstringContent::default()));
        assert!(!directives.rewriting(), "the region runs on until `on`");
        assert!(directives.consume("<!-- grund:fmt off -->", DocstringContent::default()));
        assert!(!directives.rewriting(), "a redundant `off` changes nothing");
        assert!(directives.consume("<!-- grund:fmt on -->", DocstringContent::default()));
        assert!(directives.rewriting());
    }

    /// A repository whose config is `extra` plus the `{kind}-{number}-{slug}`
    /// grammar the shorthand needs, with `FS-042-user-login` declared in it.
    fn suppression_repo(name: &str, extra: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n[id]\nformat = \"{{kind}}-{{number}}-{{slug}}\"\n{extra}"
            ),
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLead.\n",
        );
        root
    }


    /// Replay `typed` one character at a time onto `prefix`, on the line after
    /// `before`, applying each keystroke's edits as an LSP client would — the
    /// harness `tests_shorthand_rewrite` uses, kept here because what these cases
    /// vary is the document *above* the cursor and the config beside it, which is
    /// the only place a suppressed scope is visible from.
    fn type_line_in(root: &Path, file: &str, before: &str, prefix: &str, typed: &str) -> String {
        let path = root.join(file);
        let declarations = vec![DeclaredId {
            path: root.as_ref(),
            id: "FS-042-user-login",
        }];
        let line_index = before.lines().count();
        let mut line = String::from(prefix);
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

    /// §FS-lsp.1.4, §FS-fmt.2.5.2: the live transform refuses a suppressed region
    /// exactly as `grund fmt` does. The case is the one the scopes exist for — a
    /// diagram whose alignment is the document — and the editor is where that
    /// diagram is actually edited, so an on-type expansion here would splice a
    /// slug into the column the region was written to protect.
    #[test]
    fn on_type_refuses_a_shorthand_inside_a_suppressed_region() {
        let root = suppression_repo("on_type_refuses_a_shorthand_inside_a_suppressed_region", "");
        let diagram = "   | balancer | <-- agent --+-- ";

        // Outside any region the expansion fires, as it always has.
        assert_eq!(
            type_line_in(&root, "docs/topo.md", "# Topology\n", diagram, "$$FS-042 ."),
            format!("{diagram}\u{a7}FS-042-user-login .")
        );
        // Inside the region it does not. The trigger still converts the two
        // characters just typed; only the expansion is withheld (§FS-lsp.1.4).
        assert_eq!(
            type_line_in(
                &root,
                "docs/topo.md",
                "# Topology\n<!-- grund:fmt off -->\n",
                diagram,
                "$$FS-042 ."
            ),
            format!("{diagram}\u{a7}FS-042 .")
        );
        // …and fires again once the region has closed.
        assert_eq!(
            type_line_in(
                &root,
                "docs/topo.md",
                "# Topology\n<!-- grund:fmt off -->\ndiagram\n<!-- grund:fmt on -->\n",
                diagram,
                "$$FS-042 ."
            ),
            format!("{diagram}\u{a7}FS-042-user-login .")
        );
    }

    /// §FS-lsp.1.4, §FS-fmt.2.5.1: and the live transform refuses every line of a
    /// file the `[fmt] exclude` list names, which no line of that file can reveal
    /// on its own — the verdict is the config's, so the transform reads it there.
    #[test]
    fn on_type_refuses_a_shorthand_in_an_excluded_file() {
        let root = suppression_repo(
            "on_type_refuses_a_shorthand_in_an_excluded_file",
            "[fmt]\nexclude = [\"docs/AR-001-topo.md\"]\n",
        );

        assert_eq!(
            type_line_in(&root, "docs/AR-001-topo.md", "# Topology\n", "See ", "$$FS-042 ."),
            "See \u{a7}FS-042 ."
        );
        // A sibling the list does not name is rewritten as before: the exclusion
        // is per file, not per project.
        assert_eq!(
            type_line_in(&root, "docs/notes.md", "# Notes\n", "See ", "$$FS-042 ."),
            "See \u{a7}FS-042-user-login ."
        );
    }
}
