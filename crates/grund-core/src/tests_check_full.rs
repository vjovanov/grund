/// Test module: the out-of-scope reference tier of `grund check --full` — what
/// it reports past `[scan] include`, and the four rules that are the only ones
/// it judges out there (§FS-check.3.14, §DF-check-full-scope). The walk that
/// produces the tier, and the scope layer that splits its findings in two, are
/// in `tests_check_full_scope.rs`.
#[cfg(test)]
mod tests_check_full {
    // Every fixture and shape helper these cases need is shared with
    // `tests_check_full_scope.rs` and lives in `tests_support`.
    use super::tests_support::*;

    #[test]
    fn full_scope_reports_a_dangling_citation_outside_include() {
        let root = drifted_include_repo("full_scope_reports_a_dangling_citation_outside_include");
        write(&root.join("sim/world.py"), "# Cites §FS-999-missing\n");

        let scoped = check_run(&root, false);
        assert!(
            scoped.report.errors.is_empty(),
            "§FS-check.1.3: a citation outside `[scan] include` is invisible to the ordinary run"
        );

        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
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
        let root = drifted_include_repo("full_scope_withholds_style_and_grounding_outside_include");
        write(
            &root.join("sim/world.py"),
            "# Simulation world.\n#\n# A module comment citing §FS-001-login that runs well past the\n# three-line inline-note budget, because it was never written to that\n# convention in the first place.\n",
        );
        write(&root.join("sim/plain.py"), "def run():\n    pass\n");
        // `require_grounding` on, so the out-of-scope tier is also declining to
        // report every ungrounded source file it just walked past (§FS-check.3.6).
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\n\n[reference]\nrequire_grounding = true\n\n[scan]\ninclude = [\"docs\", \"src\"]\n",
        );

        let full = check_run(&root, true);
        assert!(
            full.report.errors.is_empty(),
            "§FS-check.3.14: only resolution is judged out of scope, not inline-note budgets — got {:?}",
            located_diagnostics(&full.config, &full.report.errors)
        );
        let scoped_style = check_run(&root.join("sim"), false);
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
    fn full_scope_resolves_against_the_whole_walk() {
        let root = drifted_include_repo("full_scope_resolves_against_the_whole_walk");
        write(
            &root.join("sim/engine.py"),
            "# AR-002-engine: The simulation engine\n",
        );
        write(&root.join("sim/world.py"), "# Drives §AR-002-engine\n");

        let full = check_run(&root, true);
        assert!(
            full.report.errors.is_empty(),
            "§FS-check.3.14: the tier reports references that point at nothing, not references that point outside the scope — got {:?}",
            located_diagnostics(&full.config, &full.report.errors)
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

        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
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

        let scoped = check_run(&root, false);
        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
            located_diagnostics(&scoped.config, &scoped.report.errors),
            "§FS-check.3.13: the wider walk must not leave the site holding a canonical ID whose declaration was narrowed away — one cause, one finding"
        );
        assert!(
            located_diagnostics(&scoped.config, &scoped.report.errors)
                .iter()
                .any(|line| line.contains("shorthand citation §FS-042 matches no declaration"))
        );
    }

    /// §FS-check.3.8 through the tier: an unknown namespace alias out of scope is
    /// a resolution failure like any other, and carries the alias's own code.
    #[test]
    fn full_scope_reports_an_unknown_alias_outside_include() {
        let root = drifted_include_repo("full_scope_reports_an_unknown_alias_outside_include");
        write(&root.join("sim/world.py"), "# Cites §nope/FS-001-login\n");

        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
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

        let scoped = check_run(&root, false);
        let full = check_run(&root, true);
        assert_eq!(
            located_diagnostics(&full.config, &full.report.errors),
            located_diagnostics(&scoped.config, &scoped.report.errors),
            "§FS-check.3.13: at most one shorthand finding per site, and never a dangling one beside it"
        );
        assert!(
            located_diagnostics(&scoped.config, &scoped.report.errors)
                .iter()
                .any(|line| line.contains("shorthand citation §api/AR-900 matches no declaration"))
        );
    }
}
