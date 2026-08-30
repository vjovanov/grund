/// Test module: the `grund check --full` scope layer — the walk past
/// `[scan] include`, and the promise that everything inside it still reads
/// exactly as a run without the flag (§FS-check.1.3, §DF-check-full-scope).
/// What the wider walk then *reports* out there is in `tests_check_full.rs`.
#[cfg(test)]
mod tests_check_full_scope {
    // Only the shared fixtures are needed here, and they carry their own
    // imports: a `use super::*` would be unused wherever the `#[cfg(unix)]`
    // cases below are compiled out.
    use super::tests_support::*;

    #[test]
    fn full_scope_leaves_the_in_scope_report_unchanged() {
        let root = drifted_include_repo("full_scope_leaves_the_in_scope_report_unchanged");
        // The declaration lives outside `include`; the citation inside it.
        write(
            &root.join("sim/engine.py"),
            "# AR-002-engine: The simulation engine\n",
        );
        write(&root.join("src/engine_client.rs"), "// Uses §AR-002-engine\n");

        let scoped = check_run(&root, false);
        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(
                &full.config,
                full.report
                    .errors
                    .iter()
                    .filter(|diagnostic| !diagnostic.code.starts_with("out-of-scope-"))
            ),
            located_diagnostics(&scoped.config, &scoped.report.errors),
            "§DF-check-full-scope.2.4: --full is purely additive — the wider walk never makes an in-scope citation resolve"
        );
        assert_eq!(
            located_diagnostics(&scoped.config, &scoped.report.errors),
            vec!["src/engine_client.rs:1: unknown reference AR-002-engine"]
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

        let full = check_run(&root, true);
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
        let root = drifted_include_repo("an_explicit_path_argument_is_not_widened_by_full");
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");
        write(&root.join("render/prompts.md"), "Cites §FS-998-absent\n");

        let scoped = check_run(&root.join("sim"), false);
        let full = check_run(&root.join("sim"), true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
            located_diagnostics(&scoped.config, &scoped.report.errors),
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

    /// §FS-check.1.3: `--full` with an explicit path that is not the config root
    /// has nothing left to cancel. The run is the ordinary one and says so.
    #[test]
    fn full_scope_warns_when_an_explicit_path_leaves_it_nothing_to_cancel() {
        let root = drifted_include_repo("full_scope_warns_when_an_explicit_path_leaves_it_nothing_to_cancel");
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");

        let full = check_run(&root.join("sim"), true);
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
            check_run(&root, true)
                .report
                .warnings
                .iter()
                .all(|diagnostic| diagnostic.code != "full-scope-ignored")
        );
    }

    /// §FS-config.3.6: an OS-level alias for the config root must not make a
    /// lexical explicit scope fall back to its absolute spelling.
    #[cfg(unix)]
    #[test]
    fn full_scope_warning_keeps_a_lexical_root_alias() {
        let root = drifted_include_repo("full_scope_warning_keeps_a_lexical_root_alias");
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");
        let alias = root.with_extension("alias");
        symlink(root.to_str().expect("UTF-8 test path"), &alias);

        let full = check_run(&alias.join("sim"), true);
        let caution = full
            .report
            .warnings
            .iter()
            .find(|diagnostic| diagnostic.code == "full-scope-ignored")
            .expect("redundant --full warning");
        assert!(caution.message.ends_with("and sim already bypasses it"));
        assert!(!caution.message.contains(alias.to_str().expect("UTF-8 test path")));
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

        let scoped = check_run(&root, false);
        assert!(scoped.report.errors.is_empty());

        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
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

        let scoped = check_run(&root, false);
        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
            located_diagnostics(&scoped.config, &scoped.report.errors),
            "§FS-check.1.3: the wider walk reads a superset — a gitignored `include` root is still an `include` root"
        );
        assert_eq!(
            located_diagnostics(&scoped.config, &scoped.report.errors),
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

            let scoped = check_run(&root, false);
            let full = check_run(&root, true);
            assert_eq!(
                located_diagnostics(&full.config, &full.report.errors),
                located_diagnostics(&scoped.config, &scoped.report.errors),
                "§FS-check.1.3: `--full` must not lose the {name} `include` root"
            );
            assert!(
                located_diagnostics(&scoped.config, &scoped.report.errors)
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

        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
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
            let run = check_run(&root, full);
            assert!(
                run.report
                    .errors
                    .iter()
                    .all(|diagnostic| diagnostic.code != "duplicate"),
                "one file read twice would duplicate its own declaration (full = {full}) — got {:?}",
                located_diagnostics(&run.config, &run.report.errors)
            );
        }
    }

    /// A repo whose `[scan] include` root is a **symlink** to a directory inside
    /// the config root: the plain walk starts at `<link>` and reaches every file
    /// under that spelling, while the config root `--full` adds reaches the same
    /// files under the real one. Unix only — a fixture cannot carry a symlink to
    /// a Windows runner that checks the repository out without them, so this
    /// case stays here rather than in `e2e/cases/` (§FS-check.1.3).
    #[cfg(unix)]
    fn aliased_include_root_repo(name: &str, real: &str, link: &str) -> std::path::PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            &format!("grund_config_version = 1\n\n[scan]\ninclude = [\"{link}\"]\n"),
        );
        std::fs::create_dir_all(root.join(real)).expect("create the real include root");
        std::os::unix::fs::symlink(real, root.join(link)).expect("alias the include root");
        root
    }

    /// §FS-check.1.3: the wider walk reads each *file* once, not each path once.
    /// An aliased root hands the same file to two walks under two spellings, so
    /// the byte-identical dedup cannot see the reread — and the declaration the
    /// plain run reads once would be reported as a duplicate of itself (§3.3),
    /// turning a green run red on a tree nobody changed.
    #[cfg(unix)]
    #[test]
    fn full_scope_reads_an_aliased_include_root_once() {
        let root = aliased_include_root_repo(
            "full_scope_reads_an_aliased_include_root_once",
            "docs",
            "docslink",
        );
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: A user can log in\n\n## 1. Rules\n\nSee §FS-001-login.1.\n",
        );

        let scoped = check_run(&root, false);
        let full = check_run(&root, true);
        assert!(
            scoped.report.errors.is_empty(),
            "the plain run reads the declaration once, through the `include` spelling"
        );
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
            located_diagnostics(&scoped.config, &scoped.report.errors),
            "§FS-check.1.3: --full is purely additive — no duplicate declaration of itself"
        );
    }

    /// §FS-check.1.3: and the spelling it keeps is the `include` one. Reporting
    /// the same site a second time under the real path would be an in-scope line
    /// the plain run does not print — and an untagged one, since the configured
    /// scope covers both names — so the two reports must be identical text.
    #[cfg(unix)]
    #[test]
    fn full_scope_keeps_the_include_spelling_of_an_aliased_root() {
        let root = aliased_include_root_repo(
            "full_scope_keeps_the_include_spelling_of_an_aliased_root",
            "real",
            "speclink",
        );
        write(&root.join("real/notes.md"), "# Notes\n\nCites §FS-999-missing.\n");

        let scoped = check_run(&root, false);
        assert_eq!(
            located_diagnostics(&scoped.config, &scoped.report.errors),
            vec!["speclink/notes.md:3: unknown reference FS-999-missing"],
            "the plain run reports the site under the root `[scan] include` names"
        );
        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
            located_diagnostics(&scoped.config, &scoped.report.errors),
            "§FS-check.1.3: --full only appends `outside [scan] include:` lines"
        );
    }

    /// §FS-check.3.14: the wider walk reaches files the configured scope never
    /// touched, so one it cannot read is the §FS-check.2 scan failure and exit 2 —
    /// on a tree whose plain `check` exits 0.
    #[cfg(unix)]
    #[test]
    fn full_scope_exits_two_on_an_unreadable_file_outside_include() {
        use std::os::unix::fs::PermissionsExt;

        let root = drifted_include_repo("full_scope_exits_two_on_an_unreadable_file_outside_include");
        let unreadable = root.join("sim/world.py");
        write(&unreadable, "# Cites §FS-001-login\n");
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))
            .expect("chmod the fixture");
        if std::fs::read_to_string(&unreadable).is_ok() {
            // Running as root: the mode bits say nothing about readability.
            return;
        }

        assert!(!check_run(&root, false).had_scan_errors);
        let full = check_run(&root, true);
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
