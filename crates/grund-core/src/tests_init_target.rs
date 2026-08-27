/// §FS-init.1.2 — the rules that decide whether a target may be scaffolded at
/// all, tested at the unit the CLI cannot reach.
///
/// The user-global instruction rule is not the home-directory rule restated.
/// The home rule answers when `<path>` *is* `$HOME`; this one answers for the
/// planned paths, and `<path>` is arbitrary — `<path>/AGENTS.md` under
/// `~/.codex` and `<path>/GEMINI.md` under `~/.gemini` are that table's files
/// with `<path>` at a directory the home rule says nothing about. Those
/// arguments reach the rule from the command line, which is where
/// `grund-cli/tests/init_refused_targets.rs` pins them down; what is left for
/// this module is the rule itself, on the paths a CLI argument cannot produce
/// one at a time.
///
/// Both rules are pure path questions: they read the environment and the
/// filesystem and write nothing, so these cases use the real home directory
/// rather than pointing `$HOME` somewhere else. Mutating it would race every
/// other case in this binary that reads it.
#[cfg(test)]
mod tests_init_target {
    use super::tests_support::*;
    use super::*;

    /// The real home directory, resolved the way the rules under test resolve
    /// it. Absent, there is nothing to assert rather than something that passes
    /// vacuously.
    fn home() -> PathBuf {
        std::env::home_dir()
            .expect("these cases assert against the real home directory; it is unset")
    }

    /// The user-global table is `~`-rooted and resolved through `$HOME`
    /// (§FS-integrations.4), so on a platform that does not set that variable the
    /// rule has no path to compare against and there is nothing to assert. The
    /// home-directory rule below resolves the platform home directory instead and
    /// runs everywhere, which is what keeps the reported accident refused.
    #[cfg(unix)]
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
            let refusal = refuse_init_global_instruction_paths(&[home.join(global)]);
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
            project.join(".claude/CLAUDE.md"),
            project.join("CLAUDE.md"),
        ]);
        assert!(
            allowed.is_none(),
            "a repository entrypoint was refused: {allowed:?}"
        );
    }

    /// The user-global table is `~`-rooted and resolved through `$HOME`
    /// (§FS-integrations.4), so on a platform that does not set that variable the
    /// rule has no path to compare against and there is nothing to assert. The
    /// home-directory rule below resolves the platform home directory instead and
    /// runs everywhere, which is what keeps the reported accident refused.
    #[cfg(unix)]
    #[test]
    fn the_rule_holds_for_a_path_that_does_not_exist_yet() {
        // `init` refuses *before* creating anything, so the comparison has to
        // work on a path with no file behind it (§FS-init.1.2).
        let home = home();
        let global = home.join(".claude/CLAUDE.md");
        assert!(
            refuse_init_global_instruction_paths(std::slice::from_ref(&global)).is_some(),
            "{} was allowed",
            global.display()
        );
        assert!(
            refuse_init_global_instruction_paths(&[home.join(".pi/agent/AGENTS.md")]).is_some(),
            "a user-global path whose directory the machine may not have was allowed"
        );
    }

    #[test]
    fn a_home_directory_target_is_refused_whatever_the_flags_say() {
        let home = home();
        // `refuse_init_target` answers "does not exist" before this rule, on
        // purpose (the case below pins that order), so a sandbox whose `HOME`
        // names no directory — a Nix builder's `/homeless-shelter`, some
        // container images — would turn this into a red case about a message
        // rather than about the rule. There is nothing to assert there, and
        // creating the real home directory is not this case's to do.
        if !home.is_dir() {
            eprintln!(
                "skipped: the home directory {} does not exist on this machine",
                home.display()
            );
            return;
        }
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
