/// Test module: nested workspaces — alias paths, per-level uniqueness, and the
/// bounds on recursive member expansion (§FS-workspace.6.1, §AR-workspace.6.1).
///
/// Split from `tests_workspace.rs` because these cases fail together and for
/// one reason: the shape of the alias path a nested tree produces.
#[cfg(test)]
mod tests_workspace_nested {
    use super::*;
    use super::tests_support::*;

    /// §FS-workspace.6.1: an alias path is read from the outermost workspace at
    /// every scope, three levels deep — so a run narrowed to the middle block
    /// resolves a *subset* of the outer run's paths, never a re-spelled set of
    /// its own. Without that, a citation could pass a subtree check and fail
    /// the run CI does.
    #[test]
    fn nested_workspace_alias_paths_are_stable_across_scopes() {
        let root = test_root("nested_workspace_alias_paths_are_stable_across_scopes");
        for (dir, body) in [
            ("", "project_name = \"root\"\n\n[workspace]\nmembers = [\"a\"]\n"),
            ("a", "project_name = \"a\"\n\n[workspace]\nmembers = [\"b\"]\n"),
            ("a/b", "project_name = \"b\"\n\n[workspace]\nmembers = [\"c\"]\n"),
            ("a/b/c", "project_name = \"c\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let aliases_at = |start: &Path| {
            let mut config = load_config(start).expect("load config");
            expand_workspace_tree(&mut config)
                .expect("expand workspace")
                .into_iter()
                .map(|entry| entry.alias)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            aliases_at(&root),
            vec!["root", "a", "a/b", "a/b/c"],
            "the outermost run names every project by its whole path"
        );
        assert_eq!(
            aliases_at(&root.join("a/b")),
            vec!["a/b", "a/b/c"],
            "a run narrowed to the middle block keeps the outer spelling"
        );
    }

    /// §FS-workspace.6.1: two blocks may claim one directory — a multi-segment
    /// `members` entry (`grp/inner`) hops `grp`, which declares `[workspace]` and
    /// lists the same child — and the alias path is read from the **outermost**
    /// claim, the one the top-down walk follows. Taking the nearest instead
    /// stopped at `grp`, which nothing lists, and lost every segment above it: the
    /// citation passed the subtree check and failed the run CI does.
    #[test]
    fn alias_paths_follow_the_outermost_claim_of_a_member() {
        let root = test_root("alias_paths_follow_the_outermost_claim_of_a_member");
        for (dir, body) in [
            (
                "",
                "project_name = \"skip\"\n\n[workspace]\nmembers = [\"mid\"]\n",
            ),
            (
                "mid",
                "project_name = \"mid\"\n\n[workspace]\nmembers = [\"grp/inner\"]\n",
            ),
            (
                "mid/grp",
                "project_name = \"grp\"\n\n[workspace]\nmembers = [\"inner\"]\n",
            ),
            (
                "mid/grp/inner",
                "project_name = \"inner\"\n\n[workspace]\nmembers = [\"leaf\"]\n",
            ),
            ("mid/grp/inner/leaf", "project_name = \"leaf\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let aliases_at = |start: &Path| {
            let mut config = load_config(start).expect("load config");
            expand_workspace_tree(&mut config)
                .expect("expand workspace")
                .into_iter()
                .map(|entry| entry.alias)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            aliases_at(&root),
            vec!["skip", "mid", "mid/inner", "mid/inner/leaf"],
            "the hopped directory contributes no segment at the outermost scope"
        );
        assert_eq!(
            aliases_at(&root.join("mid/grp/inner")),
            vec!["mid/inner", "mid/inner/leaf"],
            "a run narrowed to the twice-claimed member keeps the outer spelling"
        );
    }

    /// §FS-workspace.6.1: a block that claims this directory and cannot expand
    /// its own member list fails the narrowed run with that block's error. The
    /// alternative shipped: the failure read as "does not claim this tree", the
    /// climb walked past, and the subtree named itself — `alpha` where the root
    /// says `group/alpha` — so §FS-check.3.8 hinted the one spelling that fails
    /// in CI.
    #[test]
    fn enclosing_workspace_that_cannot_expand_fails_the_narrowed_run() {
        let root = test_root("enclosing_workspace_that_cannot_expand_fails_the_narrowed_run");
        for (dir, body) in [
            (
                "",
                "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\", \"missing\"]\n",
            ),
            (
                "group",
                "project_name = \"group\"\n\n[workspace]\nmembers = [\"alpha\"]\n",
            ),
            ("group/alpha", "project_name = \"alpha\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let mut config = load_config(&root.join("group")).expect("load the group config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("an enclosing workspace that cannot expand must fail the run");
        };
        let err = format!("{err:#}");
        assert!(
            err.contains("grund.toml:4: workspace member does not exist: missing"),
            "the narrowed run must report the enclosing block's own error: {err}"
        );
    }

    /// §FS-workspace.6.1: the truncation half of the same rule — an ancestor in
    /// the chain whose alias is invalid is an error, not a dropped segment. It
    /// used to `break` the climb, so the root run exited 2 while the subtree run
    /// exited 0 with every project renamed one level short.
    #[test]
    fn enclosing_workspace_with_an_invalid_alias_fails_the_narrowed_run() {
        let root = test_root("enclosing_workspace_with_an_invalid_alias_fails_the_narrowed_run");
        for (dir, body) in [
            (
                "",
                "project_name = \"root\"\n\n[workspace]\nmembers = [\"mid\"]\n",
            ),
            (
                "mid",
                "project_name = \"My_Group\"\n\n[workspace]\nmembers = [\"group\"]\n",
            ),
            (
                "mid/group",
                "project_name = \"group\"\n\n[workspace]\nmembers = [\"alpha\"]\n",
            ),
            ("mid/group/alpha", "project_name = \"alpha\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let mut config = load_config(&root.join("mid/group")).expect("load the group config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("an invalid alias in the claimed chain must fail the run");
        };
        let err = format!("{err:#}");
        assert!(
            err.contains("grund.toml:1: invalid workspace project alias `My_Group`"),
            "the narrowed run must report the ancestor's own alias error: {err}"
        );
    }

    /// §FS-workspace.6.1: a member that resolves to an ancestor of the block that
    /// lists it is a located config error naming the entry as written. It has no
    /// e2e fixture because reaching it needs a symlink, and it is also what bounds
    /// the recursive walk — without it this test would not fail, it would hang.
    ///
    /// The assertion is the whole message on purpose: the earlier
    /// `contains("is already a project in this workspace")` passed while the
    /// message named the member `` — `display_path` renders a canonical root that
    /// equals the render base as the empty string.
    #[test]
    #[cfg(unix)]
    fn nested_workspace_member_pointing_at_an_ancestor_is_rejected() {
        let root = test_root("nested_workspace_member_pointing_at_an_ancestor_is_rejected");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"group\"]\n",
        );
        write(
            &root.join("group/grund.toml"),
            "grund_config_version = 1\nproject_name = \"group\"\n\n[workspace]\nmembers = [\"back\"]\n",
        );
        std::os::unix::fs::symlink(&root, root.join("group/back"))
            .expect("point a nested member back at the workspace root");

        let mut config = load_config(&root).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a member resolving outside its own block must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            "group/grund.toml:5: workspace member `back` resolves outside the workspace root that lists it",
            "the entry is named as written, at the line that listed it"
        );
    }

    /// §FS-workspace.6.1: the same rule outward. A member whose canonical root
    /// leaves the tree has no lexical ancestor listing it, so the two scopes that
    /// can see it demand opposite spellings — `grund check` at the root wanted
    /// `real/leaf`, a run at the member wanted `leaf`, and no citation text passed
    /// both. Resolving an external repository is out of contract until a cache
    /// layer exists (§DF-subproject-namespaces.3.4), so this is an error, not a
    /// second naming model.
    #[test]
    #[cfg(unix)]
    fn workspace_member_resolving_out_of_the_tree_is_rejected() {
        let root = test_root("workspace_member_resolving_out_of_the_tree_is_rejected");
        write(
            &root.join("repo/grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"link\"]\n",
        );
        write(
            &root.join("elsewhere/real/grund.toml"),
            "grund_config_version = 1\nproject_name = \"real\"\n",
        );
        std::os::unix::fs::symlink(root.join("elsewhere/real"), root.join("repo/link"))
            .expect("point a member out of the workspace tree");

        let mut config = load_config(&root.join("repo")).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a member outside the tree must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: workspace member `link` resolves outside the workspace root that lists it",
            "the entry is named as written, at the line that listed it"
        );
    }

    /// §FS-workspace.6.1: a member that resolves *to* the block listing it — the
    /// `self` symlink — is the boundary case of the same rule, and stays rejected.
    #[test]
    #[cfg(unix)]
    fn workspace_member_resolving_to_its_own_block_is_rejected() {
        let root = test_root("workspace_member_resolving_to_its_own_block_is_rejected");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"self\"]\n",
        );
        std::os::unix::fs::symlink(&root, root.join("self"))
            .expect("point a member at its own block");

        let mut config = load_config(&root).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a member resolving to its own block must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: workspace member `self` resolves to the workspace root that lists it",
            "the entry is named as written, at the line that listed it"
        );
    }

    /// §FS-workspace.6.1: the rule above bites only on an escape — a member that
    /// really is nested inside the block that lists it still loads, including the
    /// multi-segment form (`grp/alpha`) whose canonical root is two levels down.
    #[test]
    fn nested_member_inside_the_block_that_lists_it_loads() {
        let root = test_root("nested_member_inside_the_block_that_lists_it_loads");
        for (dir, body) in [
            (
                "",
                "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\"]\n",
            ),
            (
                "group",
                "project_name = \"group\"\n\n[workspace]\nmembers = [\"alpha\", \"grp/beta\"]\n",
            ),
            ("group/alpha", "project_name = \"alpha\"\n"),
            ("group/grp/beta", "project_name = \"beta\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let mut config = load_config(&root).expect("load workspace root config");
        let aliases = expand_workspace_tree(&mut config)
            .expect("legitimate nesting must still expand")
            .into_iter()
            .map(|entry| entry.alias)
            .collect::<Vec<_>>();

        assert_eq!(aliases, vec!["root", "group", "group/alpha", "group/beta"]);
    }
}
