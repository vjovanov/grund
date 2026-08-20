/// Test module: where the config file is found — the two names probed at every
/// level of the upward walk, the tie-break between them, and the redundant-pair
/// warning that names the loser (§FS-config.1, §FS-config.1.1, §FS-check.4.3,
/// §DF-config-file-location).
#[cfg(test)]
mod tests_config_discovery {
    use super::*;
    use super::tests_support::*;

    const MARKER_AT: &str = "[reference]\nmarker = \"@\"\n";
    const MARKER_HASH: &str = "[reference]\nmarker = \"#\"\n";

    // §FS-config.1: the bare root-visible `grund.toml` is a config root on its
    // own — the form `grund init` now generates (§FS-init.2.4).
    #[test]
    fn bare_root_config_is_discovered() {
        let root = test_root("bare_root_config_is_discovered");
        write(&root.join("grund.toml"), MARKER_AT);

        let config = load_config(&root).expect("load bare config");

        assert_eq!(config.marker, "@");
        assert_eq!(config.root, canonical_test_path(&root));
        assert_eq!(config.config_file.as_deref(), Some(Path::new("grund.toml")));
        assert_eq!(config.redundant_config_file, None);
    }

    // §FS-config.1: discovery walks upward from the working directory for either
    // name, the way `cargo` finds `Cargo.toml` — a bare config must climb just as
    // the `.agents/` one always did.
    #[test]
    fn bare_config_is_discovered_from_a_subdirectory() {
        let root = test_root("bare_config_is_discovered_from_a_subdirectory");
        write(&root.join("grund.toml"), MARKER_AT);
        write(&root.join("docs/deep/note.md"), "note\n");

        let config = load_config(&root.join("docs/deep")).expect("load from subdir");

        assert_eq!(config.marker, "@");
        assert_eq!(config.root, canonical_test_path(&root));
    }

    // §FS-config.1.1 / §DF-config-file-location.2.2: the bare `grund.toml` wins a
    // tie — the form `grund init` generates is the form that governs, so a project
    // never holds one rule for the file grund writes and another for what it reads.
    #[test]
    fn the_bare_form_wins_a_tie_and_the_loser_is_recorded() {
        let root = test_root("the_bare_form_wins_a_tie_and_the_loser_is_recorded");
        write(&root.join("grund.toml"), MARKER_AT);
        write(&root.join(".agents/grund.toml"), MARKER_HASH);

        let config = load_config(&root).expect("load tied config");

        assert_eq!(config.marker, "@", "the bare file is the one that is read");
        assert_eq!(config.config_file.as_deref(), Some(Path::new("grund.toml")));
        assert_eq!(
            config.redundant_config_file.as_deref(),
            Some(Path::new(".agents/grund.toml")),
            "the ignored file is recorded so every reporting surface can name it"
        );
    }

    // §FS-check.4.3: the pair is a `warning:` — it never blocks a run, because it
    // is the ordinary transient state of a migration between the two forms.
    #[test]
    fn redundant_pair_is_reported_as_a_warning() {
        let root = test_root("redundant_pair_is_reported_as_a_warning");
        write(&root.join("grund.toml"), MARKER_AT);
        write(&root.join(".agents/grund.toml"), MARKER_HASH);

        let config = load_config(&root).expect("load tied config");

        assert_eq!(
            config_warnings(&config),
            vec![
                ".agents/grund.toml is ignored — grund.toml takes precedence; delete one"
                    .to_string()
            ]
        );
    }

    // §FS-check.4.3: one config is not a pair. A repository on either form alone
    // must stay silent, or the warning would fire on every well-formed project.
    #[test]
    fn a_single_config_warns_about_nothing() {
        for (name, rel) in [
            ("single_config_bare", "grund.toml"),
            ("single_config_agents", ".agents/grund.toml"),
        ] {
            let root = test_root(name);
            write(&root.join(rel), MARKER_AT);

            let config = load_config(&root).expect("load single config");

            assert!(
                config_warnings(&config).is_empty(),
                "{rel} alone must not warn"
            );
        }
    }

    // §DF-config-file-location.2.1: both names are probed at *every* level before
    // the walk climbs, so a bare member config shadows an ancestor's `.agents/`
    // one exactly as a nested `.agents/grund.toml` always did. Getting this wrong
    // is the failure that would make the two forms non-interchangeable.
    #[test]
    fn a_bare_member_config_shadows_an_ancestor_agents_config() {
        let root = test_root("a_bare_member_config_shadows_an_ancestor_agents_config");
        write(&root.join(".agents/grund.toml"), MARKER_AT);
        write(&root.join("packages/app/grund.toml"), MARKER_HASH);

        let config = load_config(&root.join("packages/app")).expect("load member config");

        assert_eq!(config.marker, "#");
        assert_eq!(config.root, canonical_test_path(&root.join("packages/app")));
    }

    // The mirror of the case above: an `.agents/` member under a bare root. The
    // rule is uniform, so a workspace may mix the two forms (§FS-workspace.2).
    #[test]
    fn an_agents_member_config_shadows_a_bare_root_config() {
        let root = test_root("an_agents_member_config_shadows_a_bare_root_config");
        write(&root.join("grund.toml"), MARKER_AT);
        write(&root.join("packages/app/.agents/grund.toml"), MARKER_HASH);

        let config = load_config(&root.join("packages/app")).expect("load member config");

        assert_eq!(config.marker, "#");
        assert_eq!(config.root, canonical_test_path(&root.join("packages/app")));
    }

    // §FS-config.1: relative paths inside a bare config resolve against the
    // directory holding it — the same config root the `.agents/` form gets, never
    // one level down.
    #[test]
    fn bare_config_resolves_scan_paths_against_its_own_directory() {
        let root = test_root("bare_config_resolves_scan_paths_against_its_own_directory");
        write(
            &root.join("grund.toml"),
            "[scan]\ninclude = [\"specs\"]\n",
        );
        write(
            &root.join("specs/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n\nBody.\n",
        );
        write(
            &root.join("elsewhere/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\nBody.\n",
        );

        let config = load_config(&root).expect("load bare config");
        let (findings, _) = scan_tree(&config, Some(&root), false).expect("scan");

        let ids = findings.declarations.keys().collect::<Vec<_>>();
        assert_eq!(ids.len(), 1, "only `specs/` is in scope: {ids:?}");
        assert_eq!(ids[0].slug.as_deref(), Some("alpha"));
    }

    // §GOAL-zero-config: neither name anywhere up the walk is still a valid tree,
    // and the added probe must not turn that into a discovered config.
    #[test]
    fn no_config_under_either_name_stays_zero_config() {
        let root = test_root("no_config_under_either_name_stays_zero_config");
        write(&root.join("docs/FS-001-alpha.md"), "# FS-001-alpha: Alpha\n");

        let config = load_config(&root).expect("load defaults");

        assert_eq!(config.config_file, None);
        assert_eq!(config.redundant_config_file, None);
        assert_eq!(config.marker, "§", "§FS-config.2: the built-in default");
    }

    // §FS-init.2.4: `init` generates the bare form, and probes both before it
    // writes — a repository already on `.agents/` is reported at the name it was
    // found under and never grows the pair §FS-check.4.3 warns about.
    #[test]
    fn init_generates_the_bare_form_and_leaves_an_agents_config_alone() {
        let fresh = test_root("init_generates_the_bare_form");
        let output = init(InitOpts {
            target: fresh.clone(),
            // §FS-init.1.2: a bare temp root no VCS marker covers.
            no_vcs: true,
            ..InitOpts::default()
        })
        .expect("init fresh repo");
        assert!(
            output
                .events
                .iter()
                .any(|event| event.verb == "wrote" && event.path == "grund.toml"),
            "fresh init writes the bare form: {:?}",
            output.events
        );
        assert!(fresh.join("grund.toml").is_file());
        assert!(!fresh.join(".agents/grund.toml").exists());

        let existing = test_root("init_keeps_an_agents_config");
        write(&existing.join(".agents/grund.toml"), MARKER_AT);
        let output = init(InitOpts {
            target: existing.clone(),
            // §FS-init.1.2: a bare temp root no VCS marker covers.
            no_vcs: true,
            ..InitOpts::default()
        })
        .expect("init configured repo");
        assert!(
            output
                .events
                .iter()
                .any(|event| event.verb == "exists" && event.path == ".agents/grund.toml"),
            "an existing config is reported at the name it was found under: {:?}",
            output.events
        );
        assert!(
            !existing.join("grund.toml").exists(),
            "init must not create the redundant pair"
        );
    }
}
