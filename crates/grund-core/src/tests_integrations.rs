/// Test module: integration blocks and per-client install (§FS-integrations)
#[cfg(test)]
mod tests_integrations {
    use super::*;
    // Only the symlink and resolver-peek cases build a fixture on disk, and
    // both are `cfg(unix)`; the rest of this module works on strings.
    #[cfg(unix)]
    use super::tests_support::*;

    // §FS-integrations.4.1: managed dotfile block splice is idempotent.
    #[test]
    fn integrations_block_appends_then_is_idempotent() {
        let (appended, outcome) = install_managed_block("#", false, "# my config\n", "SNIPPET").unwrap();
        assert_eq!(outcome, BlockOutcome::Appended);
        assert!(appended.starts_with("# my config\n"));
        assert!(appended.contains("# >>> grund integrations (v1) >>>\nSNIPPET\n# <<< grund integrations (v1) <<<\n"));

        let (again, outcome) = install_managed_block("#", false, &appended, "SNIPPET").unwrap();
        assert_eq!(outcome, BlockOutcome::Unchanged, "re-applying the same snippet is a no-op");
        assert_eq!(again, appended);
    }

    // §FS-integrations.4.1: a changed snippet updates only the marked region.
    #[test]
    fn integrations_block_updates_in_place() {
        let (first, _) = install_managed_block("#", false, "keep-before\n", "OLD").unwrap();
        let with_tail = format!("{first}keep-after\n");
        let (updated, outcome) = install_managed_block("#", false, &with_tail, "NEW").unwrap();
        assert_eq!(outcome, BlockOutcome::Updated);
        assert!(updated.starts_with("keep-before\n"));
        assert!(updated.ends_with("keep-after\n"), "content after the block is preserved");
        assert!(updated.contains("NEW"));
        assert!(!updated.contains("OLD"));
    }

    // §FS-integrations.4.1: a block newer than this binary is a hard error.
    #[test]
    fn integrations_block_rejects_newer_version() {
        let newer = "# >>> grund integrations (v99) >>>\nx\n# <<< grund integrations (v99) <<<\n";
        assert!(install_managed_block("#", false, newer, "SNIPPET").is_err());
    }

    // §FS-integrations.4.1: a begin marker with no matching end marker is a hard
    // error, not an append — appending would let the next --write splice from the
    // orphan begin to the appended end and delete the user config in between.
    #[test]
    fn integrations_block_rejects_orphan_begin_marker() {
        let orphan = "# >>> grund integrations (v1) >>>\nkeep-me\nmore-user-config\n";
        let result = install_managed_block("#", false, orphan, "SNIPPET");
        assert!(result.is_err(), "orphan begin marker must not append");
        // The user's content is never touched: the error path returns before any
        // rewrite, so a caller that surfaces the error leaves the file intact.
        assert!(result.unwrap_err().contains("incomplete"));
    }

    // §FS-integrations.4.1: an older supported block is upgraded in place rather
    // than left active beside a newly appended current block.
    #[test]
    fn integrations_block_upgrades_older_version_in_place() {
        let old = "before\n# >>> grund integrations (v0) >>>\nOLD\n# <<< grund integrations (v0) <<<\nafter\n";
        let (updated, outcome) = install_managed_block("#", false, old, "NEW").expect("upgrade old block");

        assert_eq!(outcome, BlockOutcome::Updated);
        assert_eq!(updated.matches("# >>> grund integrations").count(), 1);
        assert!(updated.contains("# >>> grund integrations (v1) >>>\nNEW\n"));
        assert!(updated.starts_with("before\n"));
        assert!(updated.ends_with("after\n"));
    }

    // §FS-integrations.4.1: indentation accepted by marker recognition is part
    // of the marker line and must be consumed during replacement.
    #[test]
    fn integrations_block_consumes_complete_indented_marker_lines() {
        let indented = "before\n  # >>> grund integrations (v1) >>>  \nOLD\n  # <<< grund integrations (v1) <<<  \nafter\n";
        let (updated, _) = install_managed_block("#", false, indented, "NEW").expect("replace block");

        assert!(!updated.contains("  >>>"));
        assert!(!updated.contains("  <<<"));
        assert_eq!(updated, "before\n# >>> grund integrations (v1) >>>\nNEW\n# <<< grund integrations (v1) <<<\nafter\n");
    }

    #[test]
    fn integrations_block_rejects_multiple_blocks() {
        let block = "# >>> grund integrations (v1) >>>\nx\n# <<< grund integrations (v1) <<<\n";
        assert!(install_managed_block("#", false, &format!("{block}{block}"), "NEW").is_err());
    }

    // §FS-integrations.3.3: `--peek` renders the declaration instead of opening
    // an editor, through the *same* resolution path — a peek and a click must
    // never disagree about where a citation points. The output leads with the
    // resolved `path:line` so the peek is actionable, and never launches an
    // editor even when one is configured.
    #[cfg(unix)]
    #[test]
    fn resolver_peek_prints_the_declaration_without_opening_an_editor() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_root("resolver_peek_prints_the_declaration_without_opening_an_editor");
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).expect("create mock bin");
        write(&root.join(".agents/grund.toml"), "[project]\n");
        let opened = root.join("must-not-exist");
        // `--format json` resolves; the bare call renders the body to show.
        write(
            &bin.join("grund"),
            "#!/bin/sh\nif [ \"${2:-}\" = \"--format\" ]; then\n\
             printf '%s\\n' '{\"id\":\"FS-t\",\"section\":\"2\",\"body\":\"\",\"path\":\"docs/t.md\",\"line\":12}'\n\
             else\n printf '%s\\n' 'RENDERED BODY'\nfi\n",
        );
        write(
            &bin.join("opener"),
            &format!("#!/bin/sh\ntouch {}\n", opened.display()),
        );
        let resolver = root.join("grund-open");
        write(&resolver, GRUND_OPEN_RESOLVER);
        for path in [&bin.join("grund"), &bin.join("opener"), &resolver] {
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }
        let output = std::process::Command::new(&resolver)
            .arg("--peek")
            .arg(format!("{}FS-t.2", '\u{a7}'))
            .current_dir(&root)
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default()),
            )
            .env("PAGER", "cat")
            .env("GRUND_OPEN_CMD", bin.join("opener"))
            .env_remove("EDITOR")
            .output_unbusy();

        assert!(
            output.status.success(),
            "peek failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("docs/t.md:12"),
            "peek must lead with the resolved location, got: {stdout}"
        );
        assert!(stdout.contains("RENDERED BODY"), "peek must show the declaration");
        assert!(
            !opened.exists(),
            "peek must not launch the editor even with GRUND_OPEN_CMD set"
        );
    }

    // §FS-integrations.3.2: VSCodium is a separate application with a separate
    // extensions root. Installing the extension into `~/.vscode` for a VSCodium
    // user fails *silently* — the write reports success and no link ever appears
    // — so the two clients must never share a target.
    #[test]
    fn codium_installs_into_its_own_extensions_root() {
        let vscode = IntegrationClient::Vscode.config_target();
        let codium = IntegrationClient::Codium.config_target();
        assert_ne!(vscode, codium);
        assert!(codium.starts_with("~/.vscode-oss/"), "got {codium}");
        assert!(vscode.starts_with("~/.vscode/"), "got {vscode}");
        // Same install machinery, same artifact — only the destination differs.
        assert!(matches!(
            IntegrationClient::Codium.install_kind(),
            InstallKind::Vscode
        ));
        assert!(!IntegrationClient::Codium.is_terminal());
        assert!(IntegrationClient::Codium.snippet().is_none());
    }

    // §FS-integrations.3.4: iTerm2 keeps its rules in a binary plist, so there is
    // nothing to splice and nothing to read back. It must never claim installed —
    // a guess there is worse than reporting nothing — and the detection plan has
    // to say *why*, so a caller can tell "not installed" from "not knowable".
    #[test]
    fn iterm2_is_a_manual_client_that_never_claims_installed() {
        assert!(matches!(
            IntegrationClient::Iterm2.install_kind(),
            InstallKind::Manual
        ));
        assert!(!integration_is_current(IntegrationClient::Iterm2));
        let descriptor = client_descriptor_json(IntegrationClient::Iterm2);
        assert!(descriptor.contains("\"install_kind\":\"manual\""));
        // It still uses the shared resolver, so it counts as a terminal client
        // and its printed artifact carries grund-open.
        assert!(IntegrationClient::Iterm2.is_terminal());
        // The rule it prints must carry the same matcher the other clients use,
        // or a citation clickable in kitty would be inert in iTerm2.
        let snippet = IntegrationClient::Iterm2.snippet().expect("iterm2 artifact");
        assert!(snippet.contains("[A-Z][A-Z0-9]*-[a-z0-9][a-z0-9-]*"));
        assert!(snippet.contains("grund-open \\0"));
    }

    // §FS-integrations.4.1: the markers are comments *in the host file's
    // language*. `#` is a comment in kitty.conf and .tmux.conf but the length
    // operator in Lua, so a `#` marker in wezterm.lua is a syntax error that
    // costs the user their entire WezTerm config — the block loads, and nothing
    // else in the file does.
    #[test]
    fn integrations_block_markers_match_the_host_language() {
        assert_eq!(IntegrationClient::Kitty.comment_prefix(), "#");
        assert_eq!(IntegrationClient::Tmux.comment_prefix(), "#");
        assert_eq!(IntegrationClient::Wezterm.comment_prefix(), "--");

        let (lua, _) = install_managed_block("--", true, "", "SNIPPET").expect("install into lua");
        for line in lua.lines() {
            assert!(
                !line.trim_start().starts_with('#'),
                "wezterm.lua must contain no `#` line, found: {line}"
            );
        }
        assert!(lua.starts_with("-- >>> grund integrations (v"));
    }

    // A block written with one dialect's markers must not be found by another's,
    // or an update would splice a Lua block using `#` markers and corrupt it.
    #[test]
    fn integrations_block_lookup_is_dialect_scoped() {
        let (lua, _) = install_managed_block("--", true, "", "SNIPPET").unwrap();
        assert!(find_managed_block("--", &lua).unwrap().is_some());
        assert!(find_managed_block("#", &lua).unwrap().is_none());
    }

    // §FS-integrations.4.1: WezTerm applies hyperlink rules only from the config
    // object the file returns, so a from-scratch install that stopped at the
    // block would parse and register nothing. The scaffold is what makes a fresh
    // install work without hand-editing.
    #[test]
    fn wezterm_fresh_install_is_a_working_config() {
        let scaffold = IntegrationClient::Wezterm
            .fresh_config_scaffold()
            .expect("wezterm ships a starter config");
        assert!(scaffold.contains("grund_apply_hyperlink_rule(config)"));
        assert!(scaffold.trim_end().ends_with("return config"));
        // The helper the scaffold calls is defined by the block above it.
        assert!(WEZTERM_SNIPPET.contains("function grund_apply_hyperlink_rule(config)"));
        // Clients whose config is not a program get no scaffold.
        assert!(IntegrationClient::Kitty.fresh_config_scaffold().is_none());
        assert!(IntegrationClient::Tmux.fresh_config_scaffold().is_none());
    }

    // §FS-integrations.4.1: the wiring step is reported, and the test is the
    // unmanaged remainder alone. A whole-file search would match the block's own
    // definition and comments on every config and so report nothing, ever.
    #[test]
    fn wezterm_wiring_note_reads_only_outside_the_block() {
        let block_only =
            install_managed_block("--", true, "", WEZTERM_SNIPPET).expect("install block");
        assert!(
            block_only.0.contains("function grund_apply_hyperlink_rule(config)"),
            "the block defines the helper it is checked for"
        );
        assert!(
            needs_wezterm_wiring(IntegrationClient::Wezterm, &block_only.0),
            "a config that is only the block calls nothing and must be reported"
        );

        // A from-scratch write appends the scaffold, which calls the helper.
        let mut fresh = block_only.0.clone();
        fresh.push_str(
            IntegrationClient::Wezterm
                .fresh_config_scaffold()
                .expect("wezterm ships a starter config"),
        );
        assert!(!needs_wezterm_wiring(IntegrationClient::Wezterm, &fresh));

        // A user config that already wires it up is silent too.
        let wired = install_managed_block(
            "--",
            true,
            "local config = wezterm.config_builder()\ngrund_apply_hyperlink_rule(config)\nreturn config\n",
            WEZTERM_SNIPPET,
        )
        .expect("install block into a wired config");
        assert!(!needs_wezterm_wiring(IntegrationClient::Wezterm, &wired.0));

        // An existing config that never calls it is the reported case.
        let unwired = install_managed_block(
            "--",
            true,
            "local config = wezterm.config_builder()\nreturn config\n",
            WEZTERM_SNIPPET,
        )
        .expect("install block into an unwired config");
        assert!(needs_wezterm_wiring(IntegrationClient::Wezterm, &unwired.0));

        // No other client has a wiring step, so none of them can be reported.
        for client in [
            IntegrationClient::Kitty,
            IntegrationClient::Tmux,
            IntegrationClient::Vscode,
            IntegrationClient::Codium,
            IntegrationClient::Iterm2,
        ] {
            assert!(!needs_wezterm_wiring(client, ""));
        }
    }

    // §FS-integrations.2: detection closes on the preview line and the setup
    // guide, in both the detected and the nothing-detected form — the guide
    // carries the prerequisites and manual steps `--write` cannot perform.
    #[test]
    fn detection_names_the_setup_guide() {
        assert!(SETUP_GUIDE_URL.starts_with("https://"));
        assert!(SETUP_GUIDE_URL.ends_with("docs/user-facing/clickable-citations.md"));
    }

    // §FS-integrations.1 / §FS-integrations.4.3: an explicit conversation
    // preference is a complete clientless write target.
    #[test]
    fn integrations_accepts_preference_only_write() {
        let args = ["--write", "--conversation", "link"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let invocation = parse_integrations_args(&args).expect("parse preference-only write");
        assert!(invocation.client.is_none());
        assert!(invocation.write);
        assert_eq!(invocation.conversation, Some(ConversationRendering::Link));
    }

    // §FS-integrations.1: `--conversation-target` alone is also a complete
    // clientless write target, and an unknown value is a CLI error listing the
    // accepted set — a value the caller typed, not a stale line in a file.
    #[test]
    fn integrations_accepts_target_only_write() {
        let args = ["--write", "--conversation-target", "vscodium"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let invocation = parse_integrations_args(&args).expect("parse target-only write");
        assert!(invocation.client.is_none());
        assert!(invocation.write);
        assert_eq!(invocation.conversation, None);
        assert_eq!(
            invocation.conversation_target,
            Some(ConversationTarget::Vscodium)
        );

        let joined = ["--write", "--conversation-target=web"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            parse_integrations_args(&joined)
                .expect("parse joined form")
                .conversation_target,
            Some(ConversationTarget::Web)
        );

        for rejected in [
            vec!["--write", "--conversation-target", "emacs"],
            vec!["--conversation-target", "file"],
        ] {
            let args = rejected
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            assert!(
                parse_integrations_args(&args).is_err(),
                "must be rejected: {args:?}"
            );
        }
    }

    // §DF-conversation-link-target.2.4: an agent is instructed only in the form
    // its renderer is verified to honor. The gate can hold a target where it
    // was, never make one worse — every downgrade lands on `path`, the form
    // that surface already had.
    #[test]
    fn link_support_gates_unverified_targets_to_path() {
        for target in ConversationTarget::ALL {
            assert_eq!(
                LinkSupport::Every.resolve(target),
                target,
                "a verified renderer takes every target"
            );
            assert_eq!(
                LinkSupport::Unverified.resolve(target),
                ConversationTarget::Path
            );
        }
        // Pi's labels always survive and its clicks come from the terminal's own
        // URL matcher, which knows `file:` and `https:` and no editor scheme.
        for kept in [ConversationTarget::File, ConversationTarget::Web] {
            assert_eq!(LinkSupport::FileAndWeb.resolve(kept), kept);
        }
        for dropped in [
            ConversationTarget::Vscode,
            ConversationTarget::Vscodium,
            ConversationTarget::Cursor,
        ] {
            assert_eq!(
                LinkSupport::FileAndWeb.resolve(dropped),
                ConversationTarget::Path
            );
        }
        // Codex hyperlinks web URLs and renders a *local* destination in place
        // of the label, erasing the citation — so web survives and the local
        // schemes do not.
        assert_eq!(
            LinkSupport::WebOnly.resolve(ConversationTarget::Web),
            ConversationTarget::Web
        );
        for local in [
            ConversationTarget::File,
            ConversationTarget::Vscode,
            ConversationTarget::Vscodium,
            ConversationTarget::Cursor,
        ] {
            assert_eq!(
                LinkSupport::WebOnly.resolve(local),
                ConversationTarget::Path
            );
        }
    }

    // §FS-integrations.4.3: the `link` block addresses the declaration through
    // the effective target, and `path` keeps the plain-location sentence.
    #[test]
    fn link_instruction_names_the_effective_target() {
        let path_form = ConversationRendering::Link.instruction(ConversationTarget::Path);
        assert!(path_form.contains(
            "follow each citation with its declaration location as plain `path:line` text"
        ));
        assert!(!path_form.contains("Markdown link"));

        for (target, phrase) in [
            (
                ConversationTarget::File,
                "`file://<absolute path>#L<line>` for the declaration",
            ),
            (
                ConversationTarget::Vscodium,
                "`vscodium://file<absolute path>:<line>` for the declaration",
            ),
            (
                ConversationTarget::Web,
                "the declaration's forge URL at the current commit",
            ),
        ] {
            let rendered = ConversationRendering::Link.instruction(target);
            assert!(
                rendered.contains(
                    "render each citation as a Markdown link whose visible text is the citation itself"
                ),
                "{target:?} must teach the link form: {rendered}"
            );
            assert!(rendered.contains(phrase), "{target:?}: {rendered}");
            // Self-scoping, like every other block text (§FS-integrations.4.3).
            assert!(rendered.starts_with("In repositories with a `grund.toml` (at the root or under `.agents/`):"));
            assert!(rendered.ends_with("Elsewhere, ignore this."));
        }

        // `plain` ignores the target entirely: there is no location to address.
        assert_eq!(
            ConversationRendering::Plain.instruction(ConversationTarget::Vscodium),
            ConversationRendering::Plain.instruction(ConversationTarget::Path)
        );
    }

    // §FS-integrations.4.3: the two keys are independent — an unreadable target
    // never costs the `plain`/`link` preference recorded beside it, and both are
    // recorded even when the target is inert under `plain`.
    #[test]
    fn conversation_target_is_recorded_independently() {
        let (written, outcome) = install_reference_key(
            "[reference]\nconversation = \"link\"\n",
            "reference",
            "conversation_target",
            ConversationTarget::Vscodium.name(),
            false,
        );
        assert_eq!(outcome, BlockOutcome::Updated);
        assert_eq!(
            conversation_target_preference(&written),
            Some(ConversationTarget::Vscodium)
        );
        assert_eq!(
            conversation_preference(&written),
            Some(ConversationRendering::Link)
        );

        // An unreadable target is a warning that leaves no target from this
        // file; the preference beside it still reads.
        let broken = "[reference]\nconversation = \"link\"\nconversation_target = \"emacs\"\n";
        let scan = scan_user_config(broken);
        assert_eq!(scan.preference, Some(ConversationRendering::Link));
        assert_eq!(scan.target, None);
        assert_eq!(scan.problems.len(), 1);
        assert!(
            scan.problems[0].1.contains("must be one of file | path | web"),
            "{:?}",
            scan.problems
        );

        // Neither key is reported as unused, and every other one still is.
        let stray = "[reference]\nconversation = \"plain\"\nconversation_target = \"file\"\nmarker = \"§\"\n";
        let scan = scan_user_config(stray);
        assert_eq!(scan.problems.len(), 1);
        assert!(scan.problems[0].1.contains("unused key `reference.marker`"));
        assert!(
            scan.problems[0]
                .1
                .contains("`reference.agents.<agent>.conversation_target`")
        );
    }

    // §FS-init.2.3.4.17: a Claude entrypoint that is a symlink to the canonical
    // `AGENTS.md` resolves to that one file, which every other agent reads too,
    // so the linked form cannot reach Claude through it. `init` must say so.
    // Unix-only: the fixture needs a real symlink (§FS-init.2.3.4.17).
    #[cfg(unix)]
    #[test]
    fn claude_symlink_to_agents_md_is_detected() {
        let root = test_root("claude_symlink_to_agents_md_is_detected");
        write(&root.join("AGENTS.md"), "# demo\n");
        std::os::unix::fs::symlink("AGENTS.md", root.join("CLAUDE.md")).expect("symlink");
        assert_eq!(
            claude_entrypoints_shadowed_by_symlink(&root),
            vec!["CLAUDE.md".to_string()]
        );

        // A real file is not shadowed — it carries its own block.
        let other = test_root("claude_real_file_is_not_shadowed");
        write(&other.join("AGENTS.md"), "# demo\n");
        write(&other.join("CLAUDE.md"), "# demo\n");
        assert!(claude_entrypoints_shadowed_by_symlink(&other).is_empty());
    }

    // §FS-init.2.3.4.17: the note is emitted only when the repository actually
    // commits the opinion — without it there is no linked form to be shadowed,
    // and a note would be noise on every run in a repo that never opted in.
    // Unix-only: the fixture needs a real symlink (§FS-init.2.3.4.17).
    #[cfg(unix)]
    #[test]
    fn symlinked_claude_entrypoint_is_reported_only_under_the_link_opinion() {
        let root = test_root("symlinked_claude_entrypoint_is_reported");
        write(&root.join("AGENTS.md"), "# demo\n");
        std::os::unix::fs::symlink("AGENTS.md", root.join("CLAUDE.md")).expect("symlink");

        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n",
        );
        let output = init(InitOpts {
            target: root.clone(),
            dry_run: true,
            // §FS-init.1.2: a bare temp root no VCS marker covers.
            no_vcs: true,
            ..InitOpts::default()
        })
        .expect("init without the opinion");
        assert!(output.notes.is_empty(), "{:?}", output.notes);

        write(
            &root.join(".agents/grund.toml"),
            "grund_config_version = 1\n[reference]\nconversation = \"link\"\n",
        );
        let output = init(InitOpts {
            target: root.clone(),
            dry_run: true,
            // §FS-init.1.2: a bare temp root no VCS marker covers.
            no_vcs: true,
            ..InitOpts::default()
        })
        .expect("init with the opinion");
        assert_eq!(output.notes.len(), 1, "{:?}", output.notes);
        assert!(
            output.notes[0].starts_with("CLAUDE.md is a symlink to AGENTS.md"),
            "{:?}",
            output.notes
        );
        assert!(output.notes[0].contains("grund init --claude"));
    }
}
