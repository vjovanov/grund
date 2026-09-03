/// Test module: the two scopes that suppress `fmt` (§FS-fmt.2.5). The e2e cases
/// pin what a run prints and writes; these pin the two recognizers underneath —
/// which paths `[fmt] exclude` claims (§FS-fmt.2.5.1) and which lines are
/// directives (§FS-fmt.2.5.2) — where a case per spelling would be a fixture
/// tree per spelling.
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
}
