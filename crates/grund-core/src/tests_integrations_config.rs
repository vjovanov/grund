/// Test module: agent overrides, conversation targets, and user config (§FS-integrations)
#[cfg(test)]
mod tests_integrations_config {
    use super::*;
    use super::tests_support::*;

    // §FS-integrations.4.4: `[reference.agents.<agent>]` is a partial of the
    // machine-wide keys — a key present under an agent replaces the base for
    // that agent, an absent key inherits it.
    #[test]
    fn agent_partial_overrides_only_what_it_names() {
        let text = concat!(
            "[reference]\n",
            "conversation = \"link\"\n",
            "conversation_target = \"vscodium\"\n",
            "\n",
            "[reference.agents.codex]\n",
            "conversation_target = \"web\"\n",
        );
        let scan = scan_user_config(text);
        assert_eq!(scan.preference, Some(ConversationRendering::Link));
        assert_eq!(scan.target, Some(ConversationTarget::Vscodium));
        assert_eq!(
            scan.agent_targets,
            vec![("codex".to_string(), ConversationTarget::Web)]
        );
        assert!(scan.problems.is_empty(), "{:?}", scan.problems);

        // Inheritance is the absence of an entry, not a per-agent default:
        // every other agent still resolves to the base.
        assert_eq!(
            scan.agent_targets
                .iter()
                .find(|(name, _)| name == "claude")
                .map(|(_, value)| *value),
            None
        );
    }

    // §FS-integrations.4.4: an override under an unknown agent names the closed
    // set rather than the key — the mistake is nearly always the spelling — and
    // an unknown key inside a known agent is the ordinary unused-key warning.
    #[test]
    fn agent_partial_reports_unknown_agents_and_keys() {
        let scan = scan_user_config(
            "[reference.agents.codx]\nconversation_target = \"web\"\n",
        );
        assert!(scan.agent_targets.is_empty());
        assert_eq!(scan.problems.len(), 1);
        assert!(
            scan.problems[0].1.contains("unknown agent `codx`")
                && scan.problems[0].1.contains("codex, claude, gemini, copilot, zed, pi"),
            "{:?}",
            scan.problems
        );

        let scan = scan_user_config("[reference.agents.codex]\nmarker = \"@\"\n");
        assert!(scan.agent_targets.is_empty());
        assert_eq!(scan.problems.len(), 1);
        assert!(
            scan.problems[0]
                .1
                .contains("unused key `reference.agents.codex.marker`"),
            "{:?}",
            scan.problems
        );

        // An unreadable value costs that agent's override and nothing else.
        let scan = scan_user_config(concat!(
            "[reference]\n",
            "conversation_target = \"vscodium\"\n",
            "[reference.agents.codex]\n",
            "conversation_target = \"emacs\"\n",
        ));
        assert_eq!(scan.target, Some(ConversationTarget::Vscodium));
        assert!(scan.agent_targets.is_empty());
        assert_eq!(scan.problems.len(), 1);
    }

    // §FS-integrations.4.4: a scoped write lands in the agent's own table and
    // leaves the machine-wide base byte-for-byte alone.
    #[test]
    fn agent_override_installs_into_its_own_table() {
        let base = "[reference]\nconversation = \"link\"\nconversation_target = \"vscodium\"\n";
        let (written, outcome) = install_reference_key(
            base,
            &agent_override_table("codex"),
            "conversation_target",
            ConversationTarget::Web.name(),
            false,
        );
        assert_eq!(outcome, BlockOutcome::Appended);
        assert!(written.starts_with(base), "base must be untouched: {written}");
        assert!(written.contains("[reference.agents.codex]\nconversation_target = \"web\"\n"));

        let scan = scan_user_config(&written);
        assert_eq!(scan.target, Some(ConversationTarget::Vscodium));
        assert_eq!(
            scan.agent_targets,
            vec![("codex".to_string(), ConversationTarget::Web)]
        );

        // Recording the same value again is a no-op reporting `exists`.
        let (again, outcome) = install_reference_key(
            &written,
            &agent_override_table("codex"),
            "conversation_target",
            ConversationTarget::Web.name(),
            true,
        );
        assert_eq!(outcome, BlockOutcome::Unchanged);
        assert_eq!(again, written);
    }

    // §FS-integrations.4.4 / §DF-conversation-link-target.2.5: the override sets
    // the request, the gate sets the verdict. Asking for a local scheme under an
    // agent that erases citations still resolves to `path`.
    #[test]
    fn agent_override_cannot_outrank_the_gate() {
        let requested = ConversationTarget::Vscodium;
        assert_eq!(
            LinkSupport::WebOnly.resolve(requested),
            ConversationTarget::Path,
            "an override moves the request, never the verdict"
        );
        // The motivating case needs no such power: `web` is a request the gate
        // already grants Codex.
        assert_eq!(
            LinkSupport::WebOnly.resolve(ConversationTarget::Web),
            ConversationTarget::Web
        );
    }

    // §FS-integrations.4.4: the report names the form each agent received, and
    // why when it is not the one asked for — unreported, an override, a gate
    // downgrade, and an unread key look identical from the outside.
    #[test]
    fn effective_form_describes_override_and_gate() {
        let plain = EffectiveForm {
            rendering: ConversationRendering::Plain,
            target: ConversationTarget::Path,
            requested: ConversationTarget::Vscodium,
            overridden: false,
        };
        assert_eq!(plain.describe(), "plain");

        let taken = EffectiveForm {
            rendering: ConversationRendering::Link,
            target: ConversationTarget::Vscodium,
            requested: ConversationTarget::Vscodium,
            overridden: false,
        };
        assert_eq!(taken.describe(), "link \u{2192} vscodium");

        let overridden = EffectiveForm {
            rendering: ConversationRendering::Link,
            target: ConversationTarget::Web,
            requested: ConversationTarget::Web,
            overridden: true,
        };
        assert_eq!(overridden.describe(), "link \u{2192} web; agent override");

        let gated = EffectiveForm {
            rendering: ConversationRendering::Link,
            target: ConversationTarget::Path,
            requested: ConversationTarget::Vscodium,
            overridden: false,
        };
        assert_eq!(gated.describe(), "link \u{2192} path; vscodium unverified here");
    }

    // §FS-integrations.1 / §FS-integrations.6: `--agent` scopes
    // `--conversation-target` and nothing else.
    #[test]
    fn agent_flag_requires_write_and_a_target() {
        let ok = ["--write", "--agent", "codex", "--conversation-target", "web"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let invocation = parse_integrations_args(&ok).expect("parse scoped write");
        assert_eq!(invocation.agent, Some("codex"));
        assert_eq!(
            invocation.conversation_target,
            Some(ConversationTarget::Web)
        );

        for rejected in [
            // no --write
            vec!["--agent", "codex", "--conversation-target", "web"],
            // nothing to scope
            vec!["--write", "--agent", "codex"],
            // unknown agent
            vec!["--write", "--agent", "codx", "--conversation-target", "web"],
        ] {
            let args = rejected.into_iter().map(str::to_string).collect::<Vec<_>>();
            assert!(
                parse_integrations_args(&args).is_err(),
                "must be rejected: {args:?}"
            );
        }
    }

    // §FS-integrations.4.3: the user preference is installed without rewriting
    // unrelated configuration and an explicit override replaces only its line.
    #[test]
    fn conversation_preference_appends_and_updates() {
        let existing = "keep = true\n";
        let (plain, outcome) =
            install_conversation_preference(existing, ConversationRendering::Plain);
        assert_eq!(outcome, BlockOutcome::Appended);
        assert_eq!(
            plain,
            "keep = true\n\n[reference]\nconversation = \"plain\"\n"
        );
        assert_eq!(
            conversation_preference(&plain),
            Some(ConversationRendering::Plain)
        );

        let (linked, outcome) =
            install_conversation_preference(&plain, ConversationRendering::Link);
        assert_eq!(outcome, BlockOutcome::Updated);
        assert!(linked.starts_with("keep = true\n\n[reference]\n"));
        assert!(linked.ends_with("conversation = \"link\"\n"));
        let (again, outcome) =
            install_conversation_preference(&linked, ConversationRendering::Link);
        assert_eq!(outcome, BlockOutcome::Unchanged);
        assert_eq!(again, linked);
    }

    // §FS-integrations.4.3: every TOML spelling of the key is the same setting.
    // A spelling grund failed to *see* would be silently reversed to the default
    // and written back beside the original, so these are read, not ignored.
    #[test]
    fn conversation_preference_reads_equivalent_toml_spellings() {
        for text in [
            "[reference]\nconversation = \"link\"\n",
            "[ reference ]\nconversation = \"link\"\n",
            "reference.conversation = \"link\"\n",
            "[reference]\nconversation = 'link'\n",
            "[reference]\nconversation = \"link\" # chosen\n",
            "[output]\nformat = \"json\"\n\n[reference]\nconversation  =  \"link\"\n",
        ] {
            assert_eq!(
                conversation_preference(text),
                Some(ConversationRendering::Link),
                "spelling not recognized: {text:?}"
            );
            // Already recorded: the bytes are left exactly as the user wrote
            // them, comment and all, and the run reports `exists`.
            let (same, outcome) =
                install_conversation_preference(text, ConversationRendering::Link);
            assert_eq!(outcome, BlockOutcome::Unchanged);
            assert_eq!(same, text);
        }
    }

    // A rewrite keeps the key exactly as written: a dotted root key re-spelled
    // as a bare `conversation` would land in whatever table precedes it.
    #[test]
    fn conversation_preference_rewrite_preserves_key_spelling() {
        let (updated, outcome) = install_conversation_preference(
            "reference.conversation = \"link\"\n",
            ConversationRendering::Plain,
        );
        assert_eq!(outcome, BlockOutcome::Updated);
        assert_eq!(updated, "reference.conversation = \"plain\"\n");
        assert_eq!(
            conversation_preference(&updated),
            Some(ConversationRendering::Plain)
        );
    }

    // §FS-integrations.4.3: nothing in this file fails. A value grund cannot
    // interpret leaves it with no preference — the state of a machine that never
    // wrote one — and a duplicate resolves to the first, which is the occurrence
    // a write rewrites, so a read and a write cannot disagree.
    #[test]
    fn user_config_reports_bad_values_and_duplicates_without_failing() {
        let scan = scan_user_config("[reference]\nconversation = \"neither\"\n");
        assert_eq!(scan.preference, None);
        assert_eq!(scan.problems.len(), 1);
        assert!(scan.problems[0].1.contains("must be one of plain | link"));

        let scan = scan_user_config("[reference]\nconversation = bare\n");
        assert_eq!(scan.preference, None);
        assert!(scan.problems[0].1.contains("must be a quoted"));

        let scan = scan_user_config("[reference]\nconversation = \"link\"\nconversation = \"plain\"\n");
        assert_eq!(scan.preference, Some(ConversationRendering::Link));
        assert_eq!(scan.problems, vec![(3, "ignoring duplicate `reference.conversation`; the first one is used".to_string())]);
        // The write targets the same occurrence the read used, so an unchanged
        // preference stays a no-op instead of rewriting the other line.
        let (same, outcome) = install_conversation_preference(
            "[reference]\nconversation = \"link\"\nconversation = \"plain\"\n",
            ConversationRendering::Link,
        );
        assert_eq!(outcome, BlockOutcome::Unchanged);
        assert_eq!(same, "[reference]\nconversation = \"link\"\nconversation = \"plain\"\n");
    }

    // §FS-integrations.4.3: nothing else in this file has any effect, so every
    // unconsumed key is reported with its line — a typo, a retired spelling, and
    // a repository-only key set here all read as "configured" otherwise.
    #[test]
    fn user_config_reports_every_unused_key() {
        let scan = scan_user_config(
            "[reference]\n\
             conversation = \"link\"\n\
             converstaion = \"plain\"\n\
             marker = \"@\"\n\
             \n\
             [render.links]\n\
             conversation = \"plain\"\n",
        );
        // The recognized key still resolves; the rest are named, not guessed at.
        assert_eq!(scan.preference, Some(ConversationRendering::Link));
        assert_eq!(
            scan.problems
                .iter()
                .map(|(line, message)| (*line, message.split(';').next().unwrap().to_string()))
                .collect::<Vec<_>>(),
            vec![
                (3, "unused key `reference.converstaion`".to_string()),
                (4, "unused key `reference.marker`".to_string()),
                (7, "unused key `render.links.conversation`".to_string()),
            ]
        );
    }

    // A file grund fully consumes warns about nothing.
    #[test]
    fn user_config_reports_nothing_when_every_key_is_read() {
        for text in [
            "[reference]\nconversation = \"plain\"\n",
            "reference.conversation = \"link\"\n",
            "# only a comment\n\n[reference]\n",
        ] {
            assert!(
                scan_user_config(text).problems.is_empty(),
                "spurious warning in {text:?}"
            );
        }
    }

    // §FS-integrations.4.3: global agent guidance is versioned, idempotent, and
    // preserves user-authored text around its managed block.
    #[test]
    fn agent_guidance_block_tracks_user_preference() {
        let (plain, outcome) =
            install_agent_guidance_block("# My instructions\n", ConversationRendering::Plain, ConversationTarget::Path)
                .unwrap();
        assert_eq!(outcome, BlockOutcome::Appended);
        assert!(plain.starts_with("# My instructions\n\n<!-- >>>"));
        assert!(plain.contains("write citations bare in local conversations"));

        let with_tail = format!("{plain}keep-after\n");
        let (linked, outcome) =
            install_agent_guidance_block(
                &with_tail,
                ConversationRendering::Link,
                ConversationTarget::Path,
            )
            .unwrap();
        assert_eq!(outcome, BlockOutcome::Updated);
        assert!(linked.starts_with("# My instructions\n"));
        assert!(linked.ends_with("keep-after\n"));
        assert!(linked.contains("follow each citation with its declaration location as plain `path:line` text"));
        assert!(!linked.contains("write citations bare in local conversations"));
        // §FS-integrations.4.3: user-global and written once, so neither text may
        // hardcode a marker — `[reference] marker` is per-repo, and a `§` here
        // would be wrong in every repository configured with another one.
        for text in [
            ConversationRendering::Plain.instruction(ConversationTarget::Path),
            ConversationRendering::Link.instruction(ConversationTarget::Path),
        ] {
            assert!(!text.contains('\u{a7}'), "global block must name no marker: {text}");
        }

        let (again, outcome) =
            install_agent_guidance_block(
                &linked,
                ConversationRendering::Link,
                ConversationTarget::Path,
            )
            .unwrap();
        assert_eq!(outcome, BlockOutcome::Unchanged);
        assert_eq!(again, linked);
    }

    #[test]
    fn agent_guidance_block_rejects_newer_version() {
        let newer = "<!-- >>> grund integrations citation rendering (v99) >>> -->\nx\n<!-- <<< grund integrations citation rendering (v99) <<< -->\n";
        assert!(
            install_agent_guidance_block(
                newer,
                ConversationRendering::Plain,
                ConversationTarget::Path
            )
            .is_err()
        );
    }

    // §FS-integrations.4.2: a matching marker alone does not hide a missing or
    // damaged extension file from the repair path.
    #[test]
    fn vscode_installation_state_checks_owned_files() {
        let root = test_root("vscode_installation_state_checks_owned_files");
        write(&root.join(".grund-version"), &INTEGRATIONS_BLOCK_VERSION.to_string());
        write(&root.join("package.json"), VSCODE_PACKAGE_JSON);
        write(&root.join("extension.js"), VSCODE_EXTENSION_JS);
        assert!(vscode_integration_is_current(&root));

        std::fs::remove_file(root.join("extension.js")).expect("remove provider");
        assert!(!vscode_integration_is_current(&root));
    }

    // §FS-integrations.5: the machine detection plan distinguishes ambient
    // detection from actual installation state, and carries each client's
    // `install_kind` so a manual client's permanent `installed: false` reads as
    // "not knowable" rather than "not installed" (§FS-integrations.3.4).
    #[test]
    fn integrations_detection_json_reports_installed_state() {
        let json = detection_plan_json(&[IntegrationClient::Wezterm]);
        assert!(json.contains("\"client\":\"wezterm\",\"detected\":true,\"installed\":"));
        assert!(json.contains("\"client\":\"kitty\",\"detected\":false,\"installed\":"));
        assert!(json.contains("\"install_kind\":\"block\""));
        assert!(json.contains("\"install_kind\":\"extension\""));
        assert!(json.contains(
            "\"client\":\"iterm2\",\"detected\":false,\"installed\":false,\"install_kind\":\"manual\""
        ));
    }

    // §FS-integrations.2: `codium` is marked when a VSCODE_* value *names*
    // VSCodium's application directory — a path segment, not a substring, so a
    // workspace that merely mentions codium does not flag the client.
    #[test]
    fn codium_detection_requires_a_vscodium_path_segment() {
        for named in [
            "/usr/share/codium/resources/app/out/cli.js",
            "/applications/vscodium.app/contents/resources",
            "c:\\users\\u\\appdata\\local\\programs\\vscodium\\codium.exe",
            "/var/lib/flatpak/app/com.vscodium.codium/current",
            "/opt/vscodium-bin/codium",
        ] {
            assert!(value_names_codium(named), "should mark: {named}");
        }
        for bystander in [
            "/home/u/codium-notes/project",
            "/home/u/my-codium/workspace",
            "/usr/share/code/resources/app/out/cli.js",
        ] {
            assert!(!value_names_codium(bystander), "must not mark: {bystander}");
        }
    }
}
