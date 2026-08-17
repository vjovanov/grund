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

    /// §FS-workspace.6.1 / §AR-workspace.6.1: a nested member that resolves to
    /// a project root the workspace already holds is a located config error.
    /// It has no e2e fixture because reaching it needs a symlink, and that is
    /// also the contract that bounds the recursive walk — without it this test
    /// would not fail, it would hang.
    #[test]
    #[cfg(unix)]
    fn nested_workspace_member_cycle_is_rejected() {
        let root = test_root("nested_workspace_member_cycle_is_rejected");
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
            panic!("a member cycle must fail expansion");
        };

        assert!(
            format!("{err:#}").contains("is already a project in this workspace"),
            "a cycling member must be diagnosed, not walked: {err:#}"
        );
    }
}
