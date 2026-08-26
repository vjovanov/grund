/// Test module: the declaration near-miss warning (§FS-check.4.7) — a heading
/// that opens like a declaration and parses as none.
#[cfg(test)]
mod tests_declaration_near_miss {
    use super::tests_support::*;
    use super::*;

    fn near_miss_repo(name: &str, heading: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs\"\nindex = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(&root.join("docs/spec.md"), heading);
        root
    }

    /// §FS-check.4.7: the classic stumble — the `-NNN-` left out under the
    /// default numbered format.
    #[test]
    fn a_heading_missing_the_number_is_reported() {
        let root = near_miss_repo(
            "a_heading_missing_the_number_is_reported",
            "# FS-login: Users can log in\n\nBody.\n",
        );
        let run = check_run(&root, false);
        let finding = only(&run, "declaration-near-miss");
        assert_eq!(
            finding.message,
            "`FS-login` is heading-shaped and declares nothing — \
             [id] format = \"{kind}-{number}-{slug}\" reads `# <KIND>-<NNN>-<slug>: <title>`"
        );
        assert_eq!(finding.line, Some(1));
    }

    /// §FS-check.4.7 read from the other side: a heading that *does* match
    /// gets none.
    #[test]
    fn a_heading_that_matches_is_not_reported() {
        let root = near_miss_repo(
            "a_heading_that_matches_is_not_reported",
            "# FS-001-login: Users can log in\n\nBody.\n",
        );
        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"declaration-near-miss".to_string()),
            "a declaration is not a near miss: {:?}",
            findings(&run)
        );
    }
}
