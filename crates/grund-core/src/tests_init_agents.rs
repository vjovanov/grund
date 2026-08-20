/// Test module: init scaffolding and the managed agent block (§FS-init)
#[cfg(test)]
mod tests_init_agents {
    use super::*;
    use super::tests_support::*;

    /// §FS-init.5: the distributable skill and the binary-embedded copy the CLI
    /// prints must be byte-identical, and a release that edits one surface
    /// without the other is invalid. Nothing enforced that, so an edit to the
    /// repository copy alone shipped stale setup instructions to every agent
    /// reaching grund through `agent-setup-instructions` rather than the repo.
    #[test]
    fn agent_setup_instructions_match_the_distributable_skill() {
        let distributable = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../skills/grund-init/SKILL.md");
        // Absent when the tests run from a packaged crate rather than the
        // workspace; in the repository — where the invariant can be violated —
        // it is always present.
        let Ok(text) = std::fs::read_to_string(&distributable) else {
            return;
        };
        assert_eq!(
            text, AGENT_SETUP_INSTRUCTIONS,
            "skills/grund-init/SKILL.md and crates/grund-core/assets/skills/grund-init/SKILL.md must be byte-identical (§FS-init.5)"
        );
    }

    #[test]
    fn embedded_templates_are_lf_canonical() {
        assert_eq!(
            canonical_template_text("alpha\r\nbeta\rgamma\n"),
            "alpha\nbeta\ngamma\n"
        );

        let config = Config::default_for(PathBuf::from("."));
        assert!(!render_agents_md("demo", &config, Path::new("."), true).contains('\r'));
        assert!(!render_grund_toml("demo", None).contains('\r'));
        assert!(!canonical_template_text(AGENT_SETUP_INSTRUCTIONS).contains('\r'));
        let fs_home = init_fs_home(&config);
        for (_, contents) in docs_scaffold(&fs_home) {
            assert!(!contents.contains('\r'));
        }
    }

    #[test]
    fn agents_guidance_uses_configured_section_separator() {
        let mut config = Config::default_for(PathBuf::from("."));
        config.section_separator = "#".to_string();

        let rendered = render_agents_md("demo", &config, Path::new("."), true);

        assert!(
            rendered.contains("§<ID>#1` / `§<ID>#1.1"),
            "section examples should use the configured outer separator: {rendered}"
        );
        assert!(
            !rendered.contains("§<ID>.1` / `§<ID>.1.1"),
            "section examples must not hard-code dot as the outer separator"
        );
    }

    #[test]
    fn agents_update_appends_managed_block_when_missing() {
        let (updated, result) =
            update_agents_text("# Existing agents\n", &current_block(), "AGENTS.md")
                .expect("append block");

        assert_eq!(result, AgentsUpdateResult::Appended);
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_does_not_append_current_block_twice() {
        // §FS-init.2.2: a file already holding the current rendered block is left
        // untouched (`Unchanged` → `exists `), not rewritten and reported `updated `.
        let existing = current_block();
        let (updated, result) =
            update_agents_text(&existing, &current_block(), "AGENTS.md").expect("current block");

        assert_eq!(result, AgentsUpdateResult::Unchanged);
        assert_eq!(updated, existing);
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_rewrites_current_block_from_rendered_template() {
        // A block that differs from the current render (here: an extra hand-added
        // line inside the delimiters) is replaced and reported `Updated`.
        let mut stale = current_block();
        let insert_at = stale
            .find("<!-- END GRUND MANAGED BLOCK -->")
            .expect("rendered block carries the END delimiter");
        stale.insert_str(insert_at, "hand-edited line\n");
        let existing = format!("# Local notes\n\n{stale}");

        let (updated, result) = update_agents_text(&existing, &current_block(), "AGENTS.md")
            .expect("rewrite current block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(updated.starts_with("# Local notes\n\n"));
        assert!(!updated.contains("hand-edited line"));
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_keeps_current_block_in_middle_position() {
        // §FS-init.2.3.1 / §FS-init.2.2: a block already current and already
        // sitting between user-authored sections is left byte-for-byte untouched
        // (`Unchanged` → `exists `) — nothing around it moves, nothing is rewritten.
        let existing = format!(
            "# Existing agents\n\n{}\n# Local notes\n",
            current_block()
        );
        let (updated, result) = update_agents_text(&existing, &current_block(), "AGENTS.md")
            .expect("non-EOF current block");

        assert_eq!(result, AgentsUpdateResult::Unchanged);
        assert_eq!(
            updated, existing,
            "an already-current block preserves every byte, inside and out"
        );
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert!(updated.ends_with("\n# Local notes\n"));
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_handles_crlf_line_endings() {
        // §FS-init.2.3.2: a CRLF-encoded AGENTS.md whose managed block is stale
        // (same version, different body) must still be detected and rewritten,
        // with the surrounding CRLF preserved verbatim.
        let existing = format!(
            "# Existing agents\r\n\r\n{}\r\n\r\nstale body line\r\n\r\n# Local notes\r\n",
            current_marker()
        );
        let (updated, result) = update_agents_text(&existing, &current_block(), "AGENTS.md")
            .expect("update CRLF stale block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(
            updated.starts_with("# Existing agents\r\n\r\n"),
            "CRLF prefix must be preserved verbatim"
        );
        assert!(
            updated.ends_with("\n# Local notes\r\n"),
            "CRLF suffix must be preserved verbatim"
        );
        assert_eq!(updated.matches(current_marker()).count(), 1);
        assert!(!updated.contains("stale body line"));
    }

    #[test]
    fn agents_update_migrates_legacy_block_to_delimited_form() {
        // §FS-init.2.3 / §DF-managed-block-delimiters: a legacy H2-bounded block
        // sandwiched between user sections is replaced in place by the delimited
        // render, with both neighbors byte-identical.
        let existing = "# Existing agents\n\n## Grounding with grund (v3)\n\nlegacy body\n\n## Local notes\n";
        let (updated, result) = update_agents_text(existing, &current_block(), "AGENTS.md")
            .expect("migrate legacy block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(updated.starts_with("# Existing agents\n\n<!-- BEGIN GRUND MANAGED BLOCK -->\n"));
        assert!(updated.ends_with("<!-- END GRUND MANAGED BLOCK -->\n\n## Local notes\n"));
        assert!(!updated.contains("legacy body"));
        assert_eq!(updated.matches(current_marker()).count(), 1);
    }

    #[test]
    fn agents_update_preserves_non_heading_content_after_delimited_block() {
        // §FS-init.2.3 / §DF-managed-block-delimiters: the managed region ends at
        // the END delimiter, so a third-party managed marker right after the
        // block — not an H1/H2, invisible to the legacy boundary — survives.
        let existing = format!(
            "{}\n<!-- rhei:begin -->\nother tool's region\n<!-- rhei:end -->\n",
            current_block()
        );
        let (updated, result) = update_agents_text(&existing, &current_block(), "AGENTS.md")
            .expect("update delimited block");

        assert_eq!(result, AgentsUpdateResult::Unchanged);
        assert!(updated.contains("<!-- rhei:begin -->\nother tool's region\n<!-- rhei:end -->\n"));
    }

    #[test]
    fn agents_update_refuses_malformed_delimiters() {
        // §FS-init.2.3: splicing against broken delimiters risks eating user
        // content, so init errors out and leaves the text alone.
        for (existing, defect) in [
            (
                "<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v4)\n\nbody\n",
                "missing `<!-- END GRUND MANAGED BLOCK -->`",
            ),
            (
                "notes\n\n<!-- END GRUND MANAGED BLOCK -->\n",
                "`<!-- END GRUND MANAGED BLOCK -->` without a begin delimiter",
            ),
            (
                "<!-- BEGIN GRUND MANAGED BLOCK -->\n<!-- BEGIN GRUND MANAGED BLOCK -->\n<!-- END GRUND MANAGED BLOCK -->\n",
                "duplicate `<!-- BEGIN GRUND MANAGED BLOCK -->`",
            ),
            (
                "<!-- BEGIN GRUND MANAGED BLOCK -->\nbody without a version heading\n<!-- END GRUND MANAGED BLOCK -->\n",
                "no `## Grounding with grund (vN)` heading between the delimiters",
            ),
        ] {
            let err = update_agents_text(existing, &current_block(), "AGENTS.md")
                .expect_err("malformed delimiters must refuse the update");
            let message = format!("{err:#}");
            assert!(
                message.contains("malformed grund managed block") && message.contains(defect),
                "unexpected error for {existing:?}: {message}"
            );
        }
    }

    #[test]
    fn check_reports_malformed_agents_block() {
        // §FS-check.3.5: broken delimiters are an agents-init error anchored at
        // the offending delimiter line, and the file is never rewritten.
        let root = test_root("check_reports_malformed_agents_block");
        write(
            &root.join("AGENTS.md"),
            "# Title\n\n<!-- BEGIN GRUND MANAGED BLOCK -->\n## Grounding with grund (v4)\n\nbody\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report.errors.iter().any(|error| error.code == "agents-init"
                && error.line == Some(3)
                && error
                    .message
                    .contains("malformed grund managed block: missing `<!-- END GRUND MANAGED BLOCK -->`")),
            "malformed delimiters should be a line-anchored agents-init error: {:?}",
            report
                .errors
                .iter()
                .map(|error| (&error.line, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rendered_block_citation_example_is_escaped() {
        // §FS-init.2.3: the worked example must be the `<§>`-escaped illustration
        // form — a live `§` would make freshly generated output fail the host
        // repo's own `grund check` as a dangling reference.
        let block = current_block();
        assert!(
            block.contains("`<§>FS-042-user-login.3.1`"),
            "worked example should be escaped: {block}"
        );
        assert!(
            !block.contains("`§FS-042-user-login"),
            "worked example must not be a live citation: {block}"
        );
    }

    #[test]
    fn discovers_known_companion_agent_entrypoints() {
        let root = test_root("discovers_known_companion_agent_entrypoints");
        write(&root.join("AGENTS.override.md"), "# Codex override notes\n");
        write(&root.join("CLAUDE.md"), "# Claude notes\n");
        write(&root.join(".claude/CLAUDE.md"), "# Claude project notes\n");
        write(&root.join("GEMINI.md"), "# Gemini notes\n");
        write(&root.join(".pi/AGENTS.md"), "# Pi notes\n");
        write(
            &root.join(".github/copilot-instructions.md"),
            "# Copilot notes\n",
        );

        let companions = companion_agent_entrypoints(&root).expect("discover companions");
        let rels = companions
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            rels,
            vec![
                "AGENTS.override.md",
                "CLAUDE.md",
                ".claude/CLAUDE.md",
                "GEMINI.md",
                ".pi/AGENTS.md",
                ".github/copilot-instructions.md"
            ]
        );
    }

    #[test]
    fn init_discovers_missing_aliases_for_existing_agent_workspaces() {
        // §FS-init.2.1.1: `.claude/` proves Claude is in use, which is one fact
        // and so one alias — the root-visible `CLAUDE.md`, not both of Claude's
        // entrypoints.
        let root = test_root("init_discovers_missing_aliases_for_existing_agent_workspaces");
        fs::create_dir_all(root.join(".claude")).expect("create .claude");
        fs::create_dir_all(root.join(".gemini")).expect("create .gemini");
        fs::create_dir_all(root.join(".pi")).expect("create pi");
        fs::create_dir_all(root.join(".github/workflows")).expect("create github metadata");

        let companions =
            workspace_init_companion_agent_entrypoints(&root, CanonicalSurfaceReach::EveryEntrypoint)
                .expect("discover workspace aliases");
        let rels = companions
            .iter()
            .map(|entrypoint| match entrypoint {
                InitCompanionAgentEntrypoint::Existing(path)
                | InitCompanionAgentEntrypoint::MissingAlias(path) => path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            })
            .collect::<Vec<_>>();

        assert_eq!(rels, vec!["CLAUDE.md", "GEMINI.md", ".pi/AGENTS.md"]);
    }

    #[test]
    fn an_unclaimed_generic_file_is_not_its_agent_s_entrypoint() {
        // §FS-init.2.1.1 / §FS-init.2.1: `.rules` is too generic to attribute to
        // Zed by filename alone, so a build-rules file that no `.zed/` and no
        // managed block claims is somebody else's. Answering *does this agent
        // have an entrypoint* on looser evidence than the update set and
        // `grund check` use is how the two come to disagree about what an
        // entrypoint is (§FS-check.3.5).
        let root = test_root("an_unclaimed_generic_file_is_not_its_agent_s_entrypoint");
        write(&root.join(".rules"), "# somebody else's build rules\n");

        let covered = agents_with_own_entrypoint(&root, CanonicalSurfaceReach::EveryEntrypoint)
            .expect("inspect entrypoints");
        assert!(
            !covered.contains(&AgentEntrypoint::Zed),
            "an unclaimed .rules is not Zed's entrypoint"
        );

        // The `.zed/` workspace is the evidence that settles it.
        fs::create_dir_all(root.join(".zed")).expect("create .zed");
        let covered = agents_with_own_entrypoint(&root, CanonicalSurfaceReach::EveryEntrypoint)
            .expect("inspect entrypoints");
        assert!(
            covered.contains(&AgentEntrypoint::Zed),
            "with .zed/ present the same file is Zed's entrypoint"
        );
    }

    #[test]
    fn init_requests_one_entrypoint_per_agent() {
        // §FS-init.2.1.1: an explicit flag updates every entrypoint the agent has
        // and creates one only for an agent that has none — the block is the same
        // bytes in each, so a second file is the same guidance read twice.
        let root = test_root("init_requests_one_entrypoint_per_agent");
        write(&root.join(".claude/CLAUDE.md"), "# Claude project notes\n");
        let selection = InitAgentEntrypointSelection {
            claude: true,
            gemini: true,
            ..InitAgentEntrypointSelection::default()
        };

        let (canonical_symlinks, companions) =
            requested_init_companion_agent_entrypoints(
                &root,
                &selection,
                CanonicalSurfaceReach::EveryEntrypoint,
            )
            .expect("select requested");

        assert!(canonical_symlinks.is_empty());
        let planned = companions
            .iter()
            .map(|entrypoint| {
                let rel = entrypoint
                    .path()
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let created = matches!(entrypoint, InitCompanionAgentEntrypoint::MissingAlias(_));
                (rel, created)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            planned,
            vec![
                (".claude/CLAUDE.md".to_string(), false),
                ("GEMINI.md".to_string(), true),
            ],
            "the Claude entrypoint on disk should be updated and no second one created"
        );
    }

    #[test]
    fn check_ignores_companion_agent_entrypoints_without_canonical_agents_md() {
        let root =
            test_root("check_ignores_companion_agent_entrypoints_without_canonical_agents_md");
        write(&root.join("CLAUDE.md"), "# Project agent notes\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);

        assert!(
            report
                .errors
                .iter()
                .all(|error| error.code != "agents-init"),
            "project-owned AGENTS.md should not require a managed block without canonical AGENTS.md"
        );
    }

    #[test]
    fn check_validates_managed_companion_without_canonical_agents_md() {
        let root =
            test_root("check_validates_managed_companion_without_canonical_agents_md");
        write(
            &root.join("CLAUDE.md"),
            "## Grounding with grund (v99)\n\nold block\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let expected_path = root.join("CLAUDE.md");

        assert!(
            report.errors.iter().any(|error| error.code == "agents-init"
                && error.path.as_deref() == Some(expected_path.as_path())
                && error.message.contains("unsupported grund init block v99")),
            "managed companion entrypoint should be version-checked without AGENTS.md: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_validates_managed_zed_rules_without_canonical_agents_md() {
        // §FS-check.3.5 / §FS-init.2.1: `.rules` is not discovered by filename
        // alone, but a managed block proves it is a grund-owned Zed companion
        // and must still get init-block drift detection.
        let root = test_root("check_validates_managed_zed_rules_without_canonical_agents_md");
        write(
            &root.join(".rules"),
            "## Grounding with grund (v99)\n\nold block\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let expected_path = root.join(".rules");

        assert!(
            report.errors.iter().any(|error| error.code == "agents-init"
                && error.path.as_deref() == Some(expected_path.as_path())
                && error.message.contains("unsupported grund init block v99")),
            "managed .rules should be version-checked without AGENTS.md: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_validates_zed_workspace_rules_when_canonical_exists() {
        // §FS-check.3.5 / §FS-init.2.1: in a Zed workspace, `.rules` is owned
        // by the Zed companion path and must be validated when AGENTS.md exists.
        let root = test_root("check_validates_zed_workspace_rules_when_canonical_exists");
        write(&root.join("AGENTS.md"), &current_block());
        write(&root.join(".zed/settings.json"), "{}\n");
        write(&root.join(".rules"), "# Zed notes without a managed block\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let expected_path = root.join(".rules");

        assert!(
            report.errors.iter().any(|error| error.code == "agents-init"
                && error.path.as_deref() == Some(expected_path.as_path())
                && error.message.contains("missing grund init block v7")),
            "Zed workspace .rules should be required to carry the managed block: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_ignores_unmanaged_generic_rules_without_zed_workspace() {
        // §FS-init.2.1: `.rules` is too generic to attribute to Zed by file
        // existence alone, so a generic unmanaged file outside a `.zed/`
        // workspace must not become a companion check target.
        let root = test_root("check_ignores_unmanaged_generic_rules_without_zed_workspace");
        write(&root.join("AGENTS.md"), &current_block());
        write(&root.join(".rules"), "# Build rules, not Zed\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check_findings(&findings, &config);
        let generic_rules = root.join(".rules");

        assert!(
            report.errors.iter().all(|error| {
                error.code != "agents-init"
                    || error.path.as_deref() != Some(generic_rules.as_path())
            }),
            "generic .rules must not be validated as a Zed companion: {:?}",
            report.errors
                .iter()
                .map(|error| (&error.path, &error.message))
                .collect::<Vec<_>>()
        );
    }
}
