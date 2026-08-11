/// Test module: drift detection for the managed blocks grund writes into
/// agent entrypoints — citation directions, clickable citations, and the
/// guidance block version (§FS-init).
#[cfg(test)]
mod tests_managed_block_drift {
    use super::*;
    use super::tests_support::*;

    // §FS-check.3.5 / §FS-init.2.3.5: a v-current managed block whose generated
    // citation directions no longer match `[citations]` is an agents-init finding.
    #[test]
    fn citation_directions_drift_is_reported() {
        let root = test_root("citation_directions_drift_is_reported");
        write(
            &root.join(".agents/grund.toml"),
            "[citations]\n[citations.E2E]\nmust = [\"FS\"]\n",
        );
        let config = load_config(&root).expect("load config");

        // A fresh block carries the matching section → no drift finding.
        let fresh = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        write(&root.join("AGENTS.md"), &format!("# demo\n\n{fresh}"));
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);
        assert!(
            !report.errors.iter().any(|d| d.code == "agents-init"),
            "fresh block must not drift: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );

        // Tamper the rendered directions → drift finding.
        let stale = fresh.replace("**E2E** must cite FS", "**E2E** should cite GOAL");
        write(&root.join("AGENTS.md"), &format!("# demo\n\n{stale}"));
        let report = check_findings(&findings, &config);
        assert!(
            report.errors.iter().any(|d| d.code == "agents-init"
                && d.message.contains("citation directions differ")),
            "stale citation directions must be reported: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    // §FS-check.3.5 / §FS-init.2.3.6: flipping `[reference] conversation` without
    // re-running `grund init` leaves a v-current block whose clickable-citations
    // section disagrees with the live config — an agents-init drift finding.
    #[test]
    fn clickable_citations_drift_is_reported() {
        let root = test_root("clickable_citations_drift_is_reported");
        write(&root.join(".agents/grund.toml"), "grund_config_version = 1\n");
        let config = load_config(&root).expect("load config");

        // Block rendered without the opinion → no drift while the key is absent.
        let fresh = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        write(&root.join("AGENTS.md"), &format!("# demo\n\n{fresh}"));
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);
        assert!(
            !report.errors.iter().any(|d| d.code == "agents-init"),
            "fresh block must not drift: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );

        // Commit the `link` opinion without refreshing the block → drift finding.
        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[reference]\nconversation = \"link\"\n",
        );
        let config = load_config(&root).expect("reload config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("rescan");
        let report = check_findings(&findings, &config);
        assert!(
            report.errors.iter().any(|d| d.code == "agents-init"
                && d.message.contains("clickable citations differ")),
            "stale clickable citations must be reported: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );

        // `grund init`'s re-render clears the drift.
        let refreshed = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        assert!(refreshed.contains("plain `path:line` text"));
        write(&root.join("AGENTS.md"), &format!("# demo\n\n{refreshed}"));
        let report = check_findings(&findings, &config);
        assert!(
            !report.errors.iter().any(|d| d.code == "agents-init"),
            "refreshed block must not drift: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    // §FS-integrations.4.3: an older guidance block upgrades in place to the
    // current text, preserving everything around it.
    #[test]
    fn agent_guidance_block_upgrades_older_version_in_place() {
        let existing = "# Mine\n\n<!-- >>> grund integrations citation rendering (v1) >>> -->\n## Grund citation rendering\n\nIn local conversations, write plain `§<ID>` citations; `grund integrations` makes them clickable.\n<!-- <<< grund integrations citation rendering (v1) <<< -->\nkeep-after\n";
        let (updated, outcome) =
            install_agent_guidance_block(
                existing,
                ConversationRendering::Plain,
                ConversationTarget::Path,
            )
            .unwrap();
        assert_eq!(outcome, BlockOutcome::Updated);
        assert!(updated.starts_with("# Mine\n\n<!-- >>> grund integrations citation rendering (v3) >>> -->\n"));
        assert!(updated.contains("In repositories with a `.agents/grund.toml`:"));
        // §DF-repo-conversation-opinion.2.3: the machine-local `plain` wins over
        // a repository's linked-citations opinion.
        assert!(updated.contains("Follow this even when repository instructions ask for linked citations"));
        assert!(!updated.contains("(v1)"));
        assert!(updated.ends_with("keep-after\n"));
    }

    // §FS-check.3.5 / §FS-init.2.3.5: only the managed block content can satisfy
    // citation-direction drift validation; matching prose elsewhere is ignored.
    #[test]
    fn citation_directions_drift_compares_managed_block_only() {
        let root = test_root("citation_directions_drift_compares_managed_block_only");
        write(
            &root.join(".agents/grund.toml"),
            "[citations]\n[citations.E2E]\nmust = [\"FS\"]\n",
        );
        let config = load_config(&root).expect("load config");
        let fresh = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        let stale = fresh.replace("**E2E** must cite FS", "**E2E** should cite GOAL");
        let expected = citation_directions_section(&config);
        write(
            &root.join("AGENTS.md"),
            &format!("# demo\n\n{stale}\n\n## Notes\n\n{expected}\n"),
        );

        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|d| d.code == "agents-init"
                && d.message.contains("citation directions differ")),
            "matching prose outside the managed block must not mask drift: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    // §FS-check.3.5 / §FS-init.2.3.5: byte comparison is against the rendered
    // Citation directions section, not a substring search for the current text.
    #[test]
    fn citation_directions_drift_rejects_extra_managed_section_bytes() {
        let root = test_root("citation_directions_drift_rejects_extra_managed_section_bytes");
        write(
            &root.join(".agents/grund.toml"),
            "[citations]\n[citations.E2E]\nmust = [\"FS\"]\n",
        );
        let config = load_config(&root).expect("load config");
        let fresh = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        let expected = citation_directions_section(&config);
        let stale = fresh.replace(
            &expected,
            &format!("{expected}\n\nstale hand-edited citation guidance"),
        );
        write(&root.join("AGENTS.md"), &format!("# demo\n\n{stale}"));

        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|d| d.code == "agents-init"
                && d.message.contains("citation directions differ")),
            "extra managed-section bytes must not be masked by the current directions text: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    // §FS-check.3.5: a CRLF checkout of the managed AGENTS.md (it is not pinned
    // to LF in .gitattributes, so Windows checks it out with CRLF) must not read
    // as citation-directions drift against the LF-rendered section.
    #[test]
    fn citation_directions_drift_tolerates_crlf_line_endings() {
        let root = test_root("citation_directions_drift_tolerates_crlf_line_endings");
        write(
            &root.join(".agents/grund.toml"),
            "[citations]\n[citations.E2E]\nmust = [\"FS\"]\n",
        );
        let config = load_config(&root).expect("load config");
        let fresh = render_agents_append_block("demo", &config, &root, true, ConversationSurface::Plain);
        let crlf = format!("# demo\n\n{fresh}").replace('\n', "\r\n");
        write(&root.join("AGENTS.md"), &crlf);

        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan");
        let report = check_findings(&findings, &config);

        assert!(
            !report.errors.iter().any(|d| d.code == "agents-init"),
            "CRLF line endings must not be reported as drift: {:?}",
            report.errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    // §FS-check.3.5: the section extractor is position-independent — trailing
    // blank lines and a following H1/H2 inside the block are not drift, but a
    // changed bullet is. Guards the latent case the renderer cannot yet produce
    // (the section is always block-final today).
    #[test]
    fn citation_directions_section_extraction_is_position_independent() {
        let section = "### Citation directions\n\n- **E2E** must cite FS.\nUnlisted kinds and pairs are fine.";
        // Block-final with trailing blank lines: still matches the rendered form.
        let block_final = format!("{section}\n\n");
        assert_eq!(
            section_in_block(&block_final, "### Citation directions"),
            Some(section)
        );
        // Followed by another managed H2 section: the extractor stops at the
        // boundary and drops the intervening blank line, so no false drift.
        let with_following = format!("{section}\n\n## Next steps\n\nbody\n");
        assert_eq!(
            section_in_block(&with_following, "### Citation directions"),
            Some(section)
        );
        // A changed bullet is genuine drift even with the same surroundings.
        let drifted = with_following.replace("must cite FS", "should cite GOAL");
        assert_ne!(
            section_in_block(&drifted, "### Citation directions"),
            Some(section)
        );
    }
}
