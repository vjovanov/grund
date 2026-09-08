/// Core compatibility coverage for exact-code check selection (§FS-check.1)
/// and the first-release agents-init message migration (§FS-errors.3).
#[cfg(test)]
mod tests_check_finding_selection {
    use super::*;
    use super::tests_support::*;

    /// `main_entry()` dispatches `check` through this compatibility parser, so
    /// both selector syntaxes must match the dedicated frontend (§FS-cli.3).
    #[test]
    fn issue_49_deprecated_adapter_accepts_check_finding_selectors() {
        let root = test_root("issue_49_deprecated_adapter_accepts_check_finding_selectors");
        write(
            &root.join("grund.toml"),
            concat!(
                "grund_config_version = 1\n\n",
                "[reference]\nmarker = \"\u{a7}\"\nstrict = true\n\n",
                "[id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n\n",
                "[scan]\ninclude = [\".\"]\n\n",
                "[[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n",
            ),
        );
        write(
            &root.join("docs/functional-spec/FS-live.md"),
            "# FS-live: Live behavior\n\nThe behavior cites \u{a7}FS-live.\n",
        );
        let path = root.to_string_lossy().into_owned();

        assert_eq!(
            command_check(&[
                path.clone(),
                "--only=--only=agents-init".to_string(),
            ]),
            ExitCode::from(2),
        );
        assert_eq!(
            command_check(&[
                path.clone(),
                "--ignore=--ignore=agents-init".to_string(),
            ]),
            ExitCode::from(2),
        );
        assert_eq!(
            command_check(&[
                path.clone(),
                "--ignore".to_string(),
                "agents-init".to_string(),
            ]),
            ExitCode::SUCCESS,
        );
        assert_eq!(
            command_check(&[path, "--only=unused".to_string()]),
            ExitCode::SUCCESS,
        );
    }

    /// All five agents-init variants preserve their legacy text as a contiguous
    /// prefix and append one exact maintenance-and-validity tail (§FS-check.3.5).
    #[test]
    fn issue_49_agents_init_compatibility_messages_cover_all_five_variants() {
        const TAIL: &str =
            " — repo maintenance; citation checks still ran; wording changes in grund 0.14.0";

        let stale = current_block().replacen(
            "### Citation directions\n",
            "### Citation directions\nstale generated guidance\n",
            1,
        );
        let cases = [
            (
                "malformed",
                "<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v8)\n",
                "malformed grund managed block: missing `<!-- END GRUND MANAGED BLOCK -->`",
            ),
            (
                "outdated",
                "## Grounding with grund (v3)\n\nlegacy body\n",
                "outdated grund init block v3 (run `grund init` to update to v8)",
            ),
            (
                "unsupported",
                "## Grounding with grund (v99)\n\nfuture body\n",
                "unsupported grund init block v99 (this grund supports v8)",
            ),
            (
                "stale",
                stale.as_str(),
                "stale grund init block: citation directions differ from grund.toml (run `grund init` to refresh)",
            ),
            (
                "missing",
                "# Project instructions\n",
                "missing grund init block v8",
            ),
        ];

        for (name, contents, legacy) in cases {
            let root = test_root(&format!("issue_49_agents_init_{name}"));
            let path = root.join("AGENTS.md");
            write(&path, contents);
            let config = Config::default_for(root);
            let mut report = CheckReport::default();

            check_agent_block_path(&config, &path, &mut report, true);

            assert_eq!(
                report.errors.len(),
                1,
                "{name}: {:?}",
                report
                    .errors
                    .iter()
                    .map(|error| (error.code, error.message.as_str()))
                    .collect::<Vec<_>>()
            );
            assert_eq!(report.errors[0].code, "agents-init", "{name}");
            assert_eq!(report.errors[0].message, format!("{legacy}{TAIL}"), "{name}");
        }
    }
}
