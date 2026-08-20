/// §FS-init.1.2 — the rules that decide whether a target may be scaffolded at
/// all, tested at the unit the CLI cannot reach.
///
/// The user-global instruction rule is deliberately unreachable through the
/// command line today: `~/.claude/CLAUDE.md` is the only path the repository
/// entrypoints and §FS-integrations.4.3's table share, so the home-directory
/// rule answers first for every argument that could produce it. That is a fact
/// about the two path sets, not about this rule being redundant — it is the one
/// that stays true if either table grows an entry, which is what these cases
/// pin down.
///
/// Both rules are pure path questions: they read the environment and the
/// filesystem and write nothing, so these cases use the real `$HOME` rather
/// than pointing the variable somewhere else. Mutating it would race every
/// other case in this binary that reads it.
#[cfg(test)]
mod tests_init_target {
    use super::tests_support::*;
    use super::*;

    /// The real `$HOME`, which every rule under test resolves against. Absent,
    /// there is nothing to assert rather than something that passes vacuously.
    fn home() -> PathBuf {
        PathBuf::from(
            std::env::var_os("HOME").expect("these cases assert against the real $HOME; it is unset"),
        )
    }

    #[test]
    fn user_global_instruction_files_are_refused() {
        let home = home();
        for global in [
            ".codex/AGENTS.md",
            ".claude/CLAUDE.md",
            ".gemini/GEMINI.md",
            ".copilot/copilot-instructions.md",
            ".config/zed/AGENTS.md",
            ".pi/agent/AGENTS.md",
        ] {
            let refusal = refuse_init_global_instruction_paths(&[
                InitCompanionAgentEntrypoint::MissingAlias(home.join(global)),
            ]);
            let message = refusal
                .unwrap_or_else(|| panic!("{global} is a user-global file and was allowed"));
            assert!(
                message.contains("machine-global agent instruction file")
                    && message.contains("grund integrations --write"),
                "the refusal must name the command that owns the file: {message}"
            );
        }
    }

    #[test]
    fn a_repository_entrypoint_of_the_same_name_is_not_refused() {
        // The rule is about *which file*, not which basename: `.claude/CLAUDE.md`
        // inside a project is the entrypoint `init` exists to write.
        let project = test_root("init_target_project_entrypoint");
        let allowed = refuse_init_global_instruction_paths(&[
            InitCompanionAgentEntrypoint::MissingAlias(project.join(".claude/CLAUDE.md")),
            InitCompanionAgentEntrypoint::Existing(project.join("CLAUDE.md")),
        ]);
        assert!(
            allowed.is_none(),
            "a repository entrypoint was refused: {allowed:?}"
        );
    }

    #[test]
    fn the_rule_holds_for_a_path_that_does_not_exist_yet() {
        // `init` refuses *before* creating anything, so the comparison has to
        // work on a path with no file behind it (§FS-init.1.2).
        let home = home();
        let global = home.join(".claude/CLAUDE.md");
        assert!(
            refuse_init_global_instruction_paths(&[
                InitCompanionAgentEntrypoint::MissingAlias(global.clone()),
            ])
            .is_some(),
            "{} was allowed",
            global.display()
        );
        assert!(
            refuse_init_global_instruction_paths(&[
                InitCompanionAgentEntrypoint::MissingAlias(
                    home.join(".pi/agent/AGENTS.md")
                ),
            ])
            .is_some(),
            "a user-global path whose directory the machine may not have was allowed"
        );
    }

    #[test]
    fn a_home_directory_target_is_refused_whatever_the_flags_say() {
        let home = home();
        for no_vcs in [false, true] {
            let message = refuse_init_target(&home, no_vcs)
                .unwrap_or_else(|| panic!("the home directory was allowed with no_vcs={no_vcs}"));
            assert!(
                message.contains("refusing to scaffold into the home directory"),
                "no_vcs={no_vcs}: {message}"
            );
        }
    }

    #[test]
    fn a_missing_or_non_directory_target_answers_before_the_refusals() {
        let root = test_root("init_target_shape");
        let missing = root.join("nowhere");
        assert_eq!(
            refuse_init_target(&missing, true),
            Some(format!(
                "target directory does not exist: {}",
                missing.display()
            )),
        );

        let file = root.join("a-file");
        fs::write(&file, "not a directory\n").expect("write fixture file");
        assert_eq!(
            refuse_init_target(&file, true),
            Some(format!("target is not a directory: {}", file.display())),
        );
    }
}
