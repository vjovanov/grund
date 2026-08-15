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
            vec!["sim/world.py:1: outside [scan] include: unknown reference FS-999-missing"],
            "§FS-check.3.14: --full reports it, naming the key that hid it"
        );
        assert_eq!(
            full.report.errors[0].code, "out-of-scope-dangling",
            "§FS-check.3.14: the tier code is the in-scope rule's code under an `out-of-scope-` prefix"
        );
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
                    .filter(|diagnostic| !diagnostic.code.starts_with("out-of-scope-"))
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
                .all(|diagnostic| !diagnostic.code.starts_with("out-of-scope-")),
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
                "sim/world.py:2: outside [scan] include: shorthand citation §FS-777 matches no declaration"
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
            vec!["api/sim/model.py:1: outside [scan] include: unknown reference FS-404-nope"],
            "§FS-check.1.3: `include` is a per-project statement, so every member widens past its own"
        );
    }

    /// §FS-check.1.3: `.gitignore` prunes descendants, never the directory a
    /// walk starts at — so an `[scan] include` root the ignore files hide is read
    /// by the ordinary run and must be read by the wider one too. Without the
    /// exemption `--full` reads *fewer* files than `grund check` and the finding
    /// in `generated/` disappears under the flag meant to find more.
    #[test]
    fn full_scope_still_reads_a_gitignored_include_root() {
        let root = test_root("full_scope_still_reads_a_gitignored_include_root");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"generated\", \"docs\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: A user can log in\n\nCites §AR-002-gen.\n",
        );
        write(&root.join("generated/notes.md"), "# Notes\n\nCites §FS-999-missing.\n");
        write(
            &root.join("generated/AR-002-gen.md"),
            "# AR-002-gen: Generated architecture\n\nBody.\n",
        );
        write(&root.join(".gitignore"), "generated/\n");
        // The `ignore` crate only consults ignore files inside a git repository;
        // an empty `.git` directory is what makes this fixture one.
        std::fs::create_dir_all(root.join(".git")).expect("create .git");

        let scoped = check(&root, false);
        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            located(&scoped.config, &scoped.report.errors),
            "§FS-check.1.3: the wider walk reads a superset — a gitignored `include` root is still an `include` root"
        );
        assert_eq!(
            located(&scoped.config, &scoped.report.errors),
            vec!["generated/notes.md:3: unknown reference FS-999-missing"]
        );
    }

    /// The same shape for the other two prune rules: a `[scan] include` root that
    /// `[scan] exclude` names, and one whose name makes it hidden (§FS-check.1.3).
    #[test]
    fn full_scope_still_reads_an_excluded_or_hidden_include_root() {
        for (name, include, exclude, dir) in [
            ("excluded", "[\"vendor\", \"docs\"]", "\nexclude = [\"vendor\"]", "vendor"),
            ("hidden", "[\".specs\", \"docs\"]", "", ".specs"),
        ] {
            let root = test_root(&format!(
                "full_scope_still_reads_an_excluded_or_hidden_include_root_{name}"
            ));
            write(
                &root.join("grund.toml"),
                &format!("grund_config_version = 1\n\n[scan]\ninclude = {include}{exclude}\n"),
            );
            write(
                &root.join("docs/functional-spec/FS-001-login.md"),
                "# FS-001-login: A user can log in\n\nBody.\n",
            );
            write(&root.join(dir).join("notes.md"), "# Notes\n\nCites §FS-999-missing.\n");

            let scoped = check(&root, false);
            let full = check(&root, true);
            assert_eq!(
                located(&full.config, &full.report.errors),
                located(&scoped.config, &scoped.report.errors),
                "§FS-check.1.3: `--full` must not lose the {name} `include` root"
            );
            assert!(
                located(&scoped.config, &scoped.report.errors)
                    .iter()
                    .any(|line| line.starts_with(&format!("{dir}/notes.md:3:"))),
                "the plain run reads it, so the wider one must"
            );
        }
    }

    /// The exemption is for the roots `[scan] include` names, not for the rules:
    /// a directory those same three rules prune *below* a scanned root stays
    /// unread under `--full` (§FS-check.1.3).
    #[test]
    fn full_scope_still_prunes_excluded_hidden_and_ignored_descendants() {
        let root = test_root("full_scope_still_prunes_excluded_hidden_and_ignored_descendants");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\"]\nexclude = [\"vendor\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: A user can log in\n\nBody.\n",
        );
        write(&root.join("sim/vendor/a.py"), "# Cites §FS-901-nope\n");
        write(&root.join("sim/.cache/b.py"), "# Cites §FS-902-nope\n");
        write(&root.join("sim/build/c.py"), "# Cites §FS-903-nope\n");
        write(&root.join("sim/world.py"), "# Cites §FS-904-nope\n");
        write(&root.join(".gitignore"), "build/\n");
        std::fs::create_dir_all(root.join(".git")).expect("create .git");

        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            vec!["sim/world.py:1: outside [scan] include: unknown reference FS-904-nope"],
            "§FS-check.1.3: `--full` cancels `include` and nothing else — exclude, hidden dirs, and the ignore files still prune"
        );
    }

    /// §FS-check.1.3: overlapping roots name one file once. `include` may already
    /// nest one root inside another, and under `--full` every root is walked
    /// beside the config root that contains them all — a second read would report
    /// each declaration as a duplicate of itself (§FS-check.3.3).
    #[test]
    fn full_scope_reads_each_file_once_across_overlapping_roots() {
        let root = test_root("full_scope_reads_each_file_once_across_overlapping_roots");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\", \"docs/functional-spec\", \".\"]\n",
        );
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: A user can log in\n\nCited by §FS-001-login.\n",
        );

        for full in [false, true] {
            let run = check(&root, full);
            assert!(
                run.report
                    .errors
                    .iter()
                    .all(|diagnostic| diagnostic.code != "duplicate"),
                "one file read twice would duplicate its own declaration (full = {full}) — got {:?}",
                located(&run.config, &run.report.errors)
            );
        }
    }

    /// §FS-check.3.8 through the tier: an unknown namespace alias out of scope is
    /// a resolution failure like any other, and carries the alias's own code.
    #[test]
    fn full_scope_reports_an_unknown_alias_outside_include() {
        let root = drifted_repo("full_scope_reports_an_unknown_alias_outside_include");
        write(&root.join("sim/world.py"), "# Cites §nope/FS-001-login\n");

        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            vec!["sim/world.py:1: outside [scan] include: unknown project alias nope"]
        );
        assert_eq!(full.report.errors[0].code, "out-of-scope-unknown-project");
    }

    /// §FS-check.3.13: the qualified cross-member shorthand. The workspace pass
    /// resolves `§api/AR-900` against `api`'s *whole* walk, which under `--full`
    /// includes a declaration `api`'s own `include` excludes; narrowing then drops
    /// it. One cause must still be one finding.
    #[test]
    fn full_scope_reports_a_qualified_shorthand_once() {
        let root = test_root("full_scope_reports_a_qualified_shorthand_once");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"api\", \"web\"]\ninclude_root = false\n",
        );
        let member = "grund_config_version = 1\nproject_name = \"{alias}\"\n\n[id]\nformat = \"{kind}-{number}-{slug}\"\n\n[scan]\ninclude = [\"docs\"]\n";
        write(&root.join("api/grund.toml"), &member.replace("{alias}", "api"));
        write(&root.join("web/grund.toml"), &member.replace("{alias}", "web"));
        write(&root.join("api/docs/FS-042-session.md"), "# FS-042-session: Session\n\nLead.\n");
        write(&root.join("api/internal/AR-900-hidden.md"), "# AR-900-hidden: Hidden\n\nLead.\n");
        write(&root.join("web/docs/notes.md"), "Qualified shorthand: §api/AR-900\n");

        let scoped = check(&root, false);
        let full = check(&root, true);
        assert_eq!(
            located(&full.config, &full.report.errors),
            located(&scoped.config, &scoped.report.errors),
            "§FS-check.3.13: at most one shorthand finding per site, and never a dangling one beside it"
        );
        assert!(
            located(&scoped.config, &scoped.report.errors)
                .iter()
                .any(|line| line.contains("shorthand citation §api/AR-900 matches no declaration"))
        );
    }

    /// §FS-check.1.3: `--full` with an explicit path that is not the config root
    /// has nothing left to cancel. The run is the ordinary one and says so.
    #[test]
    fn full_scope_warns_when_an_explicit_path_leaves_it_nothing_to_cancel() {
        let root = drifted_repo("full_scope_warns_when_an_explicit_path_leaves_it_nothing_to_cancel");
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");

        let full = check(&root.join("sim"), true);
        let caution = full
            .report
            .warnings
            .iter()
            .find(|diagnostic| diagnostic.code == "full-scope-ignored")
            .expect("§FS-check.1.3: the redundant flag earns a caution");
        assert_eq!(
            caution.message,
            "--full has no effect with an explicit PATH — it cancels [scan] include, and sim already bypasses it"
        );
        assert!(caution.line.is_none(), "a run-level caution goes to stderr, not the findings");
        // The root scope is where the flag does apply, so it stays silent there.
        assert!(
            check(&root, true)
                .report
                .warnings
                .iter()
                .all(|diagnostic| diagnostic.code != "full-scope-ignored")
        );
    }

    /// §FS-check.3.14: the wider walk reaches files the configured scope never
    /// touched, so one it cannot read is the §FS-check.2 scan failure and exit 2 —
    /// on a tree whose plain `check` exits 0.
    #[cfg(unix)]
    #[test]
    fn full_scope_exits_two_on_an_unreadable_file_outside_include() {
        use std::os::unix::fs::PermissionsExt;

        let root = drifted_repo("full_scope_exits_two_on_an_unreadable_file_outside_include");
        let unreadable = root.join("sim/world.py");
        write(&unreadable, "# Cites §FS-001-login\n");
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))
            .expect("chmod the fixture");
        if std::fs::read_to_string(&unreadable).is_ok() {
            // Running as root: the mode bits say nothing about readability.
            return;
        }

        assert!(!check(&root, false).had_scan_errors);
        let full = check(&root, true);
        assert!(
            full.had_scan_errors,
            "§FS-check.3.14: a file the wider walk cannot read is a scan failure, and the run exits 2"
        );
        assert!(
            full.report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == "io"),
            "reported in the CLI-level `error: <path>: <reason>` shape"
        );
        let _ = std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644));
    }
}
