/// Test module: the `grund check --full` scope layer — the walk past
/// `[scan] include`, the out-of-scope reference tier, and the promise that the
/// in-scope report is what a run without the flag prints (§FS-check.1.3,
/// §FS-check.3.14, §DF-check-full-scope).
#[cfg(test)]
mod tests_check_full {
    use super::*;
    use super::tests_support::*;

    /// A repo whose code moved out from under `[scan] include`: specs and `src/`
    /// are configured, `sim/` is not — the shape the issue behind
    /// §DF-check-full-scope reports.
    fn drifted_repo(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\", \"src\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: A user can log in\n\n## 1. Rules\n\nThe login behavior.\n",
        );
        write(&root.join("src/auth.rs"), "// Implements §FS-001-login.1\n");
        root
    }

    fn located<'a>(
        config: &Config,
        diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
    ) -> Vec<String> {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}: {}",
                    diagnostic
                        .path
                        .as_ref()
                        .map(|path| display_path(config, path))
                        .unwrap_or_default(),
                    diagnostic.line.unwrap_or(0),
                    diagnostic.message
                )
            })
            .collect()
    }

    fn check(root: &Path, full: bool) -> CheckRun {
        run_check(root, true, false, full).expect("check run")
    }

    #[test]
    fn full_scope_reports_a_dangling_citation_outside_include() {
        let root = drifted_repo("full_scope_reports_a_dangling_citation_outside_include");
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");

        let scoped = check(&root, false);
        assert!(
            scoped.report.errors.is_empty(),
            "§FS-check.1.3: a citation outside `[scan] include` is invisible to the ordinary run"
        );

        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            vec!["sim/world.py:1: unknown reference FS-999-missing (outside [scan] include)"],
            "§FS-check.3.14: --full reports it, naming the key that hid it"
        );
        assert_eq!(full.report.errors[0].code, "out-of-scope-reference");
    }

    #[test]
    fn full_scope_withholds_style_and_grounding_outside_include() {
        let root = drifted_repo("full_scope_withholds_style_and_grounding_outside_include");
        write(
            &root.join("sim/world.py"),
            "\"\"\"Simulation world.\n\nA module docstring citing §FS-001-login that runs well past the\nthree-line inline-note budget, because it was never written to that\nconvention in the first place.\n\"\"\"\n",
        );
        write(&root.join("sim/plain.py"), "def run():\n    pass\n");
        // `require_grounding` on, so the out-of-scope tier is also declining to
        // report every ungrounded source file it just walked past (§FS-check.3.6).
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[reference]\nrequire_grounding = true\n\n[scan]\ninclude = [\"docs\", \"src\"]\n",
        );

        let full = check(&root, true);
        assert!(
            full.report.errors.is_empty(),
            "§FS-check.3.14: only resolution is judged out of scope, not inline-note budgets — got {:?}",
            located(&full.config, &full.report.errors)
        );
        let scoped_style = check(&root.join("sim"), false);
        assert!(
            scoped_style
                .report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == "inline-citation-style"),
            "the same tree checked as an explicit scope does report the style violation"
        );
    }

    #[test]
    fn full_scope_leaves_the_in_scope_report_unchanged() {
        let root = drifted_repo("full_scope_leaves_the_in_scope_report_unchanged");
        // The declaration lives outside `include`; the citation inside it.
        write(
            &root.join("sim/engine.py"),
            "# AR-002-engine: The simulation engine\n",
        );
        write(&root.join("src/engine_client.rs"), "// Uses §AR-002-engine\n");

        let scoped = check(&root, false);
        let full = check(&root, true);
        assert_eq!(
            located(
                &full.config,
                full.report
                    .errors
                    .iter()
                    .filter(|diagnostic| diagnostic.code != "out-of-scope-reference")
            ),
            located(&scoped.config, &scoped.report.errors),
            "§DF-check-full-scope.2.4: --full is purely additive — the wider walk never makes an in-scope citation resolve"
        );
        assert_eq!(
            located(&scoped.config, &scoped.report.errors),
            vec!["src/engine_client.rs:1: unknown reference AR-002-engine"]
        );
    }

    #[test]
    fn full_scope_resolves_against_the_whole_walk() {
        let root = drifted_repo("full_scope_resolves_against_the_whole_walk");
        write(
            &root.join("sim/engine.py"),
            "# AR-002-engine: The simulation engine\n",
        );
        write(&root.join("sim/world.py"), "# Drives §AR-002-engine\n");

        let full = check(&root, true);
        assert!(
            full.report.errors.is_empty(),
            "§FS-check.3.14: the tier reports references that point at nothing, not references that point outside the scope — got {:?}",
            located(&full.config, &full.report.errors)
        );
    }

    #[test]
    fn full_scope_keeps_the_empty_scan_caution() {
        let root = test_root("full_scope_keeps_the_empty_scan_caution");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"nowhere\"]\n",
        );
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");

        let full = check(&root, true);
        assert!(
            full.report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.code == "empty-scan"),
            "§FS-check.1.3: the tier says where the citations are, the caution says the config has not been told"
        );
        assert_eq!(full.report.errors.len(), 1);
    }

    #[test]
    fn an_explicit_path_argument_is_not_widened_by_full() {
        let root = drifted_repo("an_explicit_path_argument_is_not_widened_by_full");
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");
        write(&root.join("render/prompts.md"), "Cites §FS-998-absent\n");

        let scoped = check(&root.join("sim"), false);
        let full = check(&root.join("sim"), true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            located(&scoped.config, &scoped.report.errors),
            "§FS-check.1.3: --full cancels `include`, never a path the caller typed"
        );
        assert!(
            full.report
                .errors
                .iter()
                .all(|diagnostic| diagnostic.code != "out-of-scope-reference"),
            "an explicit scope has no out-of-scope tier"
        );
    }

    #[test]
    fn full_scope_withholds_the_mechanical_shorthand_rewrite() {
        let root = test_root("full_scope_withholds_the_mechanical_shorthand_rewrite");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[id]\nformat = \"{kind}-{number}-{slug}\"\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-042-user-login.md"),
            "# FS-042-user-login: A user can log in\n\nBody.\n",
        );
        write(
            &root.join("sim/world.py"),
            "# Resolvable shorthand §FS-042\n# Unknown shorthand §FS-777\n",
        );

        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            vec![
                "sim/world.py:2: shorthand citation §FS-777 matches no declaration (outside [scan] include)"
            ],
            "§FS-check.3.14: the mechanical rewrite is withheld because `fmt` scopes by `include` too; a shorthand matching nothing is still a resolution failure"
        );
    }

    #[test]
    fn full_scope_does_not_resolve_an_in_scope_shorthand_against_the_wider_walk() {
        let root = test_root("full_scope_does_not_resolve_an_in_scope_shorthand_against_the_wider_walk");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[id]\nformat = \"{kind}-{number}-{slug}\"\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nCites the shorthand §FS-042.\n",
        );
        // The only declaration the shorthand could name lives outside `include`.
        write(
            &root.join("sim/login.py"),
            "# FS-042-user-login: A user can log in\n",
        );

        let scoped = check(&root, false);
        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            located(&scoped.config, &scoped.report.errors),
            "§FS-check.3.13: the wider walk must not leave the site holding a canonical ID whose declaration was narrowed away — one cause, one finding"
        );
        assert!(
            located(&scoped.config, &scoped.report.errors)
                .iter()
                .any(|line| line.contains("shorthand citation §FS-042 matches no declaration"))
        );
    }

    #[test]
    fn full_scope_widens_every_workspace_member() {
        let root = test_root("full_scope_widens_every_workspace_member");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"api\"]\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-root.md"),
            "# FS-001-root: Root spec\n\nBody.\n",
        );
        write(
            &root.join("api/grund.toml"),
            "grund_config_version = 1\nproject_name = \"api\"\n\n[scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("api/docs/functional-spec/FS-002-api.md"),
            "# FS-002-api: Api spec\n\nCites §root/FS-001-root.\n",
        );
        write(&root.join("api/sim/model.py"), "# Cites §FS-404-nope\n");

        let scoped = check(&root, false);
        assert!(scoped.report.errors.is_empty());

        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            vec!["api/sim/model.py:1: unknown reference FS-404-nope (outside [scan] include)"],
            "§FS-check.1.3: `include` is a per-project statement, so every member widens past its own"
        );
    }
}
