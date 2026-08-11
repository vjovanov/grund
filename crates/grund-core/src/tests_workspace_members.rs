/// Test module: workspace member listing and descriptions (§FS-workspace)
#[cfg(test)]
mod tests_workspace_members {
    use super::*;
    use super::tests_support::*;

    /// §FS-init.2.3.4.15: `render_workspace_members_section` returns the empty
    /// string for a target that is not inside a workspace. The Project Map
    /// section is unchanged from the no-workspace fixture (§FS-init-fixtures.6.3).
    #[test]
    fn workspace_members_empty_when_no_workspace_declared() {
        let root = test_root("workspace_members_empty_when_no_workspace_declared");
        // No `.agents/grund.toml` at all — fall through to defaults.
        assert_eq!(render_workspace_members_section(&root, None, None, "§", true), "");
        // And the rendered AGENTS.md contains neither the section heading nor
        // the discoverability line.
        let config = Config::default_for(root.clone());
        let rendered = render_agents_md("demo", &config, &root, true);
        assert!(!rendered.contains("### Workspace members"));
        assert!(!rendered.contains("Cross-project citations"));
    }

    /// §FS-init.2.3.4.15: invoked at the workspace root, the section lists
    /// every member sorted by alias, marks uninitialized members with
    /// `*(not yet initialized)*`, and includes the root row when
    /// `include_root = true` (the default). Mirrors §FS-init-fixtures.6.1.
    #[test]
    fn workspace_members_root_init_lists_aliases_and_initialization_state() {
        let root = test_root("workspace_members_root_init_lists_aliases_and_initialization_state");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"packages/*\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("packages/core")).expect("create core");
        std::fs::create_dir_all(root.join("packages/ui")).expect("create ui");
        write(&root.join("apps/api/AGENTS.md"), "## existing block\n");

        let section = render_workspace_members_section(&root, None, None, "§", true);

        assert!(section.contains("### Workspace members"));
        assert!(section.contains("Cross-project citations use §alias/<ID>."));
        assert!(section.contains("- [`api`](apps/api/AGENTS.md)"));
        assert!(
            section.contains("- [`core`](packages/core/) *(not yet initialized)*")
        );
        assert!(section.contains("- [`ui`](packages/ui/) *(not yet initialized)*"));
        // `include_root = true` (default), and the root row is rendered with
        // the uniform `alias → AGENTS.md` shape — self counts as initialized
        // even though `root/AGENTS.md` does not yet exist on disk.
        assert!(section.contains("- [`root`](AGENTS.md)"));
        // Alias-sorted: api < core < root < ui.
        let api = section.find("`api`").unwrap();
        let core = section.find("`core`").unwrap();
        let root_pos = section.find("`root`").unwrap();
        let ui = section.find("`ui`").unwrap();
        assert!(api < core && core < root_pos && root_pos < ui);
    }

    /// §FS-init.2.3.4.15: invoked inside a member, the section has the same
    /// alias list and ordering as the root run, the member-being-initialized
    /// is marked as `self` (initialized even before the write completes), and
    /// link paths are recomputed relative to the member's AGENTS.md. Mirrors
    /// §FS-init-fixtures.6.2.
    #[test]
    fn workspace_members_member_init_uses_self_exception_and_relative_paths() {
        let root = test_root("workspace_members_member_init_uses_self_exception_and_relative_paths");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"packages/*\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("packages/core")).expect("create core");
        std::fs::create_dir_all(root.join("packages/ui")).expect("create ui");
        // None of the members are initialized — root/AGENTS.md absent too.
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, None, None, "§", true);

        // Self counts as initialized — `api` row is the uniform-shape link.
        assert!(section.contains("- [`api`](AGENTS.md)"));
        // Sibling members and the workspace root all carry the marker.
        assert!(section
            .contains("- [`core`](../../packages/core/) *(not yet initialized)*"));
        assert!(section
            .contains("- [`ui`](../../packages/ui/) *(not yet initialized)*"));
        // Root row points at the workspace root *directory* because its
        // AGENTS.md does not exist.
        assert!(section.contains("- [`root`](../../) *(not yet initialized)*"));
        // Alias list and ordering are independent of which project is self.
        let api = section.find("`api`").unwrap();
        let core = section.find("`core`").unwrap();
        let root_pos = section.find("`root`").unwrap();
        let ui = section.find("`ui`").unwrap();
        assert!(api < core && core < root_pos && root_pos < ui);
    }

    /// §FS-init.2.3.4.15: companion-only init does not create the canonical
    /// AGENTS.md, so the self row must still point at the project directory when
    /// AGENTS.md is absent.
    #[test]
    fn workspace_members_companion_only_init_marks_missing_self_agents_md() {
        let root = test_root("workspace_members_companion_only_init_marks_missing_self_agents_md");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, None, None, "§", false);

        assert!(section.contains("- [`api`](./) *(not yet initialized)*"));
        assert!(!section.contains("- [`api`](AGENTS.md)"));
    }

    /// §FS-init.2.3.4.15: the discoverability line uses the target project's
    /// configured marker, not a hard-coded `§`.
    #[test]
    fn workspace_members_discoverability_line_uses_configured_marker() {
        let root = test_root("workspace_members_discoverability_line_uses_configured_marker");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");

        let section = render_workspace_members_section(&root, None, None, "@", true);

        assert!(section.contains("Cross-project citations use @alias/<ID>."));
        assert!(!section.contains("Cross-project citations use §alias/<ID>."));
    }

    /// §FS-init.2.3.4.15: when a member has no local config yet, its self row
    /// uses the `project_name` that `init` is about to write instead of the
    /// directory basename, so the generated block matches later workspace
    /// resolution.
    #[test]
    fn workspace_members_member_init_uses_pending_name_for_self_alias() {
        let root = test_root("workspace_members_member_init_uses_pending_name_for_self_alias");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, Some("service"), None, "§", true);

        assert!(section.contains("- [`service`](AGENTS.md)"));
        assert!(
            !section.contains("`api`"),
            "the basename fallback must not leak into the generated block"
        );
    }

    /// §FS-init.2.3.4.15: `include_root = false` drops the root row entirely;
    /// the section still emits when there is at least one member to list.
    #[test]
    fn workspace_members_omits_root_when_include_root_false() {
        let root = test_root("workspace_members_omits_root_when_include_root_false");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\ninclude_root = false\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");

        let section = render_workspace_members_section(&root, None, None, "§", true);

        assert!(section.contains("### Workspace members"));
        assert!(section.contains("`api`"));
        assert!(
            !section.contains("`root`"),
            "include_root = false should suppress the root row entirely"
        );
    }

    /// §FS-init.2.3.4.15 + §FS-workspace.6: a configured-but-misconfigured
    /// workspace (e.g. a member directory that does not exist) silently
    /// suppresses the section so `init` does not fail. `grund check` will
    /// surface the configuration error separately.
    #[test]
    fn workspace_members_silently_skipped_on_workspace_config_error() {
        let root = test_root("workspace_members_silently_skipped_on_workspace_config_error");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        // `apps/api` directory missing — `expand_workspace_members` errors,
        // but `render_workspace_members_section` must degrade gracefully.

        assert_eq!(render_workspace_members_section(&root, None, None, "§", true), "");
    }

    /// §FS-init.2.3.4.15: duplicate aliases are a workspace configuration
    /// error, so `init` must suppress the section instead of rendering
    /// ambiguous bullets with the same alias.
    #[test]
    fn workspace_members_suppresses_duplicate_aliases() {
        let root = test_root("workspace_members_suppresses_duplicate_aliases");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"services/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create apps/api");
        std::fs::create_dir_all(root.join("services/api")).expect("create services/api");

        assert_eq!(render_workspace_members_section(&root, None, None, "§", true), "");
    }

    /// §FS-init.2.3.4.15 + §DF-workspace-member-descriptions: a project's
    /// `project_description` renders after its link (before any trailing
    /// marker), and a project without one keeps the link-only bullet. Mirrors
    /// §FS-init-fixtures.6.4.
    #[test]
    fn workspace_members_renders_configured_descriptions() {
        let root = test_root("workspace_members_renders_configured_descriptions");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\nproject_description = \"Workspace root: shared specs\"\n\n[workspace]\nmembers = [\"apps/api\", \"packages/*\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("packages/core")).expect("create core");
        std::fs::create_dir_all(root.join("packages/ui")).expect("create ui");
        write(
            &root.join("apps/api/.agents/grund.toml"),
            "project_name = \"api\"\nproject_description = \"Payment API service\"\n",
        );
        write(&root.join("apps/api/AGENTS.md"), "## existing block\n");
        write(
            &root.join("packages/core/.agents/grund.toml"),
            "project_name = \"core\"\nproject_description = \"Core domain library\"\n",
        );

        let section = render_workspace_members_section(&root, None, None, "§", true);

        // Initialized member: description after the link.
        assert!(section
            .contains("- [`api`](apps/api/AGENTS.md): Payment API service"));
        // Uninitialized member: description before the trailing marker.
        assert!(section.contains(
            "- [`core`](packages/core/): Core domain library *(not yet initialized)*"
        ));
        // Root row: description from the root config.
        assert!(section.contains("- [`root`](AGENTS.md): Workspace root: shared specs"));
        // No config ⇒ no description ⇒ bullet unchanged.
        assert!(section.contains("- [`ui`](packages/ui/) *(not yet initialized)*"));
    }

    /// §FS-init.2.3.4.15: when a member has no local config yet, its self row
    /// uses the pending `project_description` from `--description`, mirroring
    /// the pending `project_name` behavior.
    #[test]
    fn workspace_members_member_init_uses_pending_description_for_self_row() {
        let root = test_root("workspace_members_member_init_uses_pending_description_for_self_row");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(
            &api_target,
            Some("service"),
            Some("Billing service"),
            "§",
            true,
        );

        assert!(section.contains("- [`service`](AGENTS.md): Billing service"));
    }

    /// §FS-init.2.4 + §DF-workspace-member-descriptions: the generated config
    /// teaches `project_description` with a commented line by default, and
    /// `--description` turns it into the real key.
    #[test]
    fn grund_toml_description_teaching_line_and_substitution() {
        let teaching_line =
            "# project_description = \"<one line shown next to this project in workspace member lists>\"";
        let without = render_grund_toml("demo", None);
        assert!(without.contains(teaching_line));
        assert!(!without.contains("\nproject_description = "));

        let with = render_grund_toml("demo", Some("Demo \"quoted\" service"));
        assert!(!with.contains(teaching_line));
        assert!(with.contains("project_description = \"Demo \\\"quoted\\\" service\""));
    }
}
