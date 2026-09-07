/// Test module: the declaration near-miss warning (§FS-check.4.6) — a heading
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

    /// §FS-check.4.6 / §RM-off-grammar-declaration-error: the classic stumble
    /// stays a warning with a named deadline before 0.15.0.
    #[test]
    fn off_grammar_heading_missing_the_number_is_reported() {
        let root = near_miss_repo(
            "a_heading_missing_the_number_is_reported",
            "# FS-login: Users can log in\n\nBody.\n",
        );
        let run = check_run(&root, false);
        let finding = only(&run, "declaration-near-miss");
        assert_eq!(
            finding.message,
            "`FS-login` resolves for compatibility but does not match \
             [id] format = \"{kind}-{number}-{slug}\" — rename it or change the \
             effective format; this warning becomes an error in grund 0.15.0"
        );
        assert_eq!(finding.line, Some(1));

        let version = |text: &str| {
            text.trim_end_matches("-dev")
                .split('.')
                .map(|part| part.parse::<u32>().expect("numeric version"))
                .collect::<Vec<_>>()
        };
        assert!(
            version(env!("CARGO_PKG_VERSION")) < version("0.15.0"),
            "this tree reached 0.15.0; land §RM-off-grammar-declaration-error \
             instead of shipping the warning past its deadline"
        );
    }

    /// §FS-config.3.2 / §FS-check.1.1 / §FS-show.1: a mismatch remains a
    /// readable declaration, and only an exact marked candidate backed by that
    /// catalog entry is promoted. Bare and unbacked malformed tokens stay text.
    #[test]
    fn an_off_grammar_declaration_and_its_exact_marked_citation_remain_readable() {
        let root = near_miss_repo(
            "an_off_grammar_declaration_and_its_exact_marked_citation_remain_readable",
            "# FS-security-providers: Security providers\n\nLead.\n\n\
             ## 1. Contract\n\nStable.\n",
        );
        write(
            &root.join("docs/notes.md"),
            "Backed \u{a7}FS-security-providers.1.\n\
             Bare FS-security-providers is prose.\n\
             Unbacked \u{a7}FS-not-declared stays text.\n",
        );

        let shown = show(
            "FS-security-providers.1",
            ShowOpts {
                path: root.clone(),
                mode: ShowMode::Full,
                ..ShowOpts::default()
            },
        )
        .expect("exact persisted declaration resolves");
        assert!(shown.body.contains("Stable."), "{}", shown.body);

        let report = check(&root).expect("check fixture");
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(
            report
                .warnings
                .iter()
                .filter(|finding| finding.code == "declaration-near-miss")
                .count(),
            1
        );
        assert!(
            report
                .warnings
                .iter()
                .all(|finding| !finding.message.contains("never cited")),
            "the backed citation must count as inbound use: {:?}",
            report.warnings
        );
    }

    /// §FS-check.4.6 read from the other side: a heading that *does* match
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

    /// §FS-config.3.2 / §FS-check.4.6: both recognition and the displayed
    /// template come from the candidate kind's effective grammar. A persisted
    /// spelling accepted only by the repository default remains readable when
    /// that kind's authoritative override rejects it.
    #[test]
    fn off_grammar_kind_override_owns_its_near_miss_shape_and_message() {
        let root = test_root("a_kind_override_owns_its_near_miss_shape_and_message");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [[kinds]]\nkind = \"FS\"\nfolder = \"docs/specs\"\nindex = false\n\n\
             [[kinds]]\nkind = \"TICKET\"\nfile = \"docs/tickets.md\"\n\
             format = \"{kind}_{number}\"\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/tickets.md"),
            "# Tickets\n\n## TICKET-old: Persisted default-shaped ticket\n\n\
             Ticket body cites \u{a7}TICKET-old.\n",
        );

        let shown = show(
            "TICKET-old",
            ShowOpts {
                path: root.clone(),
                mode: ShowMode::Full,
                ..ShowOpts::default()
            },
        )
        .expect("persisted override-rejected declaration resolves");
        assert!(shown.body.contains("Ticket body"), "{}", shown.body);

        let run = check_run(&root, false);
        assert!(run.report.errors.is_empty(), "got {:?}", findings(&run));
        let near_misses = run
            .report
            .warnings
            .iter()
            .filter(|finding| finding.code == "declaration-near-miss")
            .collect::<Vec<_>>();
        assert_eq!(near_misses.len(), 1, "got {:?}", findings(&run));
        assert_eq!(near_misses[0].line, Some(3));
        assert_eq!(
            near_misses[0].message,
            "`TICKET-old` resolves for compatibility but does not match \
             [id] format = \"{kind}_{number}\" — rename it or change the \
             effective format; this warning becomes an error in grund 0.15.0"
        );
    }
}
