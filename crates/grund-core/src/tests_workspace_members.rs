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

    /// §FS-init.2.3.4.15: invoked at the workspace root, the section omits the
    /// root itself and lists every foreign member in alias order, preserving
    /// initialized and not-yet-initialized link grammar. Mirrors
    /// §FS-init-fixtures.6.1.
    #[test]
    fn workspace_members_root_init_omits_self_and_preserves_foreign_rows() {
        let root = test_root("workspace_members_root_init_omits_self_and_preserves_foreign_rows");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"packages/*\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("packages/core")).expect("create core");
        std::fs::create_dir_all(root.join("packages/ui")).expect("create ui");
        write(&root.join("apps/api/AGENTS.md"), "## existing block\n");

        let section = render_workspace_members_section(&root, None, None, "§", true);

        assert_eq!(
            section,
            "\n\n### Workspace members\n\nCross-project citations use §alias/<ID>.\n\n- [`api`](apps/api/AGENTS.md)\n- [`core`](packages/core/) *(not yet initialized)*\n- [`ui`](packages/ui/) *(not yet initialized)*"
        );
    }

    /// §FS-init.2.3.4.15: invoked inside a member, the section omits that member
    /// and keeps the foreign root and siblings in alias order, with link paths
    /// recomputed relative to the member's AGENTS.md. Mirrors §FS-init-fixtures.6.2.
    #[test]
    fn workspace_members_member_init_omits_self_and_preserves_foreign_rows() {
        let root = test_root("workspace_members_member_init_omits_self_and_preserves_foreign_rows");
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

        assert_eq!(
            section,
            "\n\n### Workspace members\n\nCross-project citations use §alias/<ID>.\n\n- [`core`](../../packages/core/) *(not yet initialized)*\n- [`root`](../../) *(not yet initialized)*\n- [`ui`](../../packages/ui/) *(not yet initialized)*"
        );
    }

    /// §FS-init.2.3.4.15: companion-only init omits self by the same canonical
    /// identity rule as canonical-AGENTS.md init.
    #[test]
    fn workspace_members_companion_only_init_omits_self() {
        let root = test_root("workspace_members_companion_only_init_omits_self");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, None, None, "§", false);

        assert!(!section.contains("`api`"), "self row leaked into: {section}");
        assert!(section.contains("- [`root`](../../) *(not yet initialized)*"));
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

    /// §FS-init.2.3.4.15: self is selected by canonical project identity, not
    /// by either the member directory basename or the pending `project_name`.
    #[test]
    fn workspace_members_self_identity_is_canonical_not_pending_alias_text() {
        let root = test_root("workspace_members_self_identity_is_canonical_not_pending_alias_text");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let api_target = root.join("apps/api");

        let section = render_workspace_members_section(&api_target, Some("service"), None, "§", true);

        assert!(!section.contains("`service`"), "pending self alias leaked into: {section}");
        assert!(!section.contains("`api`"), "self basename leaked into: {section}");
        assert!(section.contains("- [`root`](../../) *(not yet initialized)*"));
    }

    /// §FS-init.2.3.4.15: a target reached through a symlink still omits the
    /// resolved project whose canonical root it names.
    #[cfg(unix)]
    #[test]
    fn workspace_members_self_identity_follows_target_symlink() {
        let root = test_root("workspace_members_self_identity_follows_target_symlink");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\"]\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::os::unix::fs::symlink("apps/api", root.join("api-link"))
            .expect("symlink api target");

        let section =
            render_workspace_members_section(&root.join("api-link"), None, None, "§", true);

        assert!(!section.contains("`api`"), "canonical self row leaked into: {section}");
        assert!(section.contains("- [`root`](../../) *(not yet initialized)*"));
    }

    /// §FS-init.2.3.4.15: `include_root = false` still drops the root from a
    /// member entrypoint; self is also omitted, while another foreign member
    /// keeps the section present.
    #[test]
    fn workspace_members_omits_root_when_include_root_false() {
        let root = test_root("workspace_members_omits_root_when_include_root_false");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"apps/api\", \"apps/web\"]\ninclude_root = false\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        std::fs::create_dir_all(root.join("apps/web")).expect("create web");

        let section =
            render_workspace_members_section(&root.join("apps/api"), None, None, "§", true);

        assert!(section.contains("### Workspace members"));
        assert!(!section.contains("`api`"), "self row leaked into: {section}");
        assert!(
            !section.contains("`root`"),
            "include_root = false should suppress the root row entirely"
        );
        assert!(section.contains("- [`web`](../web/) *(not yet initialized)*"));
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
        // Self and its description are omitted together.
        assert!(!section.contains("`root`"), "self description row leaked into: {section}");
        // No config ⇒ no description ⇒ bullet unchanged.
        assert!(section.contains("- [`ui`](packages/ui/) *(not yet initialized)*"));
    }

    /// §FS-init.2.3.4.15: pending self metadata never creates a local row.
    #[test]
    fn workspace_members_member_init_omits_pending_self_description() {
        let root = test_root("workspace_members_member_init_omits_pending_self_description");
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

        assert!(!section.contains("service"), "pending self alias leaked into: {section}");
        assert!(!section.contains("Billing service"), "pending self description leaked into: {section}");
        assert!(section.contains("- [`root`](../../) *(not yet initialized)*"));
    }

    /// §FS-init.2.3.4.15 + §FS-workspace.6.1: in a nested workspace the search
    /// climbs to the *outermost* root, so a member three levels down is taught
    /// every foreign alias CI can resolve — not just its enclosing group's.
    /// Rows carry whole alias paths and links stay relative to the entrypoint.
    #[test]
    fn workspace_members_nested_workspace_lists_the_whole_tree() {
        let root = test_root("workspace_members_nested_workspace_lists_the_whole_tree");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\", \"apps/api\"]\n",
        );
        write(
            &root.join("group/.agents/grund.toml"),
            "project_name = \"group\"\n\n[workspace]\nmembers = [\"alpha\"]\n",
        );
        write(
            &root.join("group/alpha/.agents/grund.toml"),
            "project_name = \"alpha\"\n",
        );
        std::fs::create_dir_all(root.join("apps/api")).expect("create api");
        let alpha_target = root.join("group/alpha");

        let section = render_workspace_members_section(&alpha_target, None, None, "§", true);

        assert!(!section.contains("`group/alpha`"), "self row leaked into: {section}");
        assert!(
            section.contains("- [`group`](../) *(not yet initialized)*"),
            "the grouping node is a project of its own: {section}"
        );
        assert!(
            section.contains("- [`api`](../../apps/api/) *(not yet initialized)*"),
            "a sibling outside the enclosing group is still citable: {section}"
        );
        assert!(
            section.contains("- [`root`](../../) *(not yet initialized)*"),
            "root row: {section}"
        );
    }

    /// §FS-init.2.3.4.15 + §FS-workspace.6.1: `include_root` is read per
    /// `[workspace]` block, so a grouping node that opted out contributes no
    /// alias and therefore no row.
    #[test]
    fn workspace_members_nested_grouping_node_without_include_root_has_no_row() {
        let root = test_root("workspace_members_nested_grouping_node_without_include_root_has_no_row");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\"]\n",
        );
        write(
            &root.join("group/.agents/grund.toml"),
            "project_name = \"group\"\n\n[workspace]\nmembers = [\"alpha\"]\ninclude_root = false\n",
        );
        write(
            &root.join("group/alpha/.agents/grund.toml"),
            "project_name = \"alpha\"\n",
        );

        let section = render_workspace_members_section(&root, None, None, "§", true);

        assert!(
            section.contains("- [`group/alpha`](group/alpha/)"),
            "the leaf keeps its `group/` segment even though `group` is not a project: {section}"
        );
        assert!(!section.contains("`root`"), "root self row leaked into: {section}");
        assert!(!section.contains("`group`]"), "grouping node must not be a row: {section}");
    }

    /// §FS-init.2.3.4.15 + §FS-workspace.6.1: an ancestor `[workspace]` that does
    /// not list the directory below it describes a different workspace, so the
    /// section names the tree the target's own outermost *claiming* block
    /// resolves. Reading the outermost *declarer* instead replaced both real
    /// projects with two aliases that resolve nowhere here and two links outside
    /// the repository — and left `grund check` green, so the wrong block shipped.
    #[test]
    fn workspace_members_ignores_an_ancestor_workspace_that_does_not_claim_the_target() {
        let root = test_root("workspace_members_ignores_an_ancestor_workspace_that_does_not_claim_the_target");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"outer\"\n\n[workspace]\nmembers = [\"unrelated\"]\n",
        );
        std::fs::create_dir_all(root.join("unrelated")).expect("create unrelated");
        write(
            &root.join("repo/.agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"api\"]\n",
        );
        std::fs::create_dir_all(root.join("repo/api")).expect("create api");

        let section = render_workspace_members_section(&root.join("repo"), None, None, "§", true);

        assert!(!section.contains("`root`"), "repository self row leaked into: {section}");
        assert!(
            section.contains("- [`api`](api/) *(not yet initialized)*"),
            "its member is listed relative to the entrypoint: {section}"
        );
        assert!(
            !section.contains("outer") && !section.contains("unrelated"),
            "an unrelated enclosing workspace contributes nothing: {section}"
        );
    }

    /// §FS-init.2.3.4.15 + §FS-workspace.6.1: the same rule one level in — a
    /// nested `[workspace]` its parent does not list is a workspace root in its
    /// own right, so `init` inside it teaches its own aliases rather than the
    /// enclosing tree's, which is what a command run there resolves.
    #[test]
    fn workspace_members_at_a_group_its_parent_does_not_list_names_its_own_tree() {
        let root = test_root("workspace_members_at_a_group_its_parent_does_not_list_names_its_own_tree");
        write(
            &root.join(".agents/grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"listed\"]\n",
        );
        std::fs::create_dir_all(root.join("listed")).expect("create listed");
        write(
            &root.join("stray/.agents/grund.toml"),
            "project_name = \"stray\"\n\n[workspace]\nmembers = [\"leaf\"]\n",
        );
        std::fs::create_dir_all(root.join("stray/leaf")).expect("create leaf");

        let section = render_workspace_members_section(&root.join("stray"), None, None, "§", true);

        assert!(!section.contains("`stray`"), "workspace self row leaked into: {section}");
        assert!(
            section.contains("- [`leaf`](leaf/) *(not yet initialized)*"),
            "its member keeps the one segment its own scope gives it: {section}"
        );
        assert!(
            !section.contains("`root`") && !section.contains("listed"),
            "the enclosing tree is not what a run here resolves: {section}"
        );
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
