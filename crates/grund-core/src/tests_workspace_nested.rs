/// Test module: nested workspaces — what one `[workspace]` block's `members` list
/// may contain, and how a bad entry is named (§FS-workspace.6.1,
/// §AR-workspace.6.1).
///
/// Split from `tests_workspace.rs` because these cases fail together and for one
/// reason: the shape of the alias path a nested tree produces. The claimed chain
/// that decides which block a path is read from lives in
/// `tests_workspace_claims.rs`.
#[cfg(test)]
mod tests_workspace_nested {
    use super::*;
    use super::tests_support::*;

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

    /// §FS-workspace.6.1: the containment rule is about where an entry *resolves*,
    /// not which of its segments carries the symlink — a symlinked **parent**
    /// (`members = ["pkgs/api"]` with `pkgs -> ../store/pkgs`) leaves the tree just
    /// as surely as a symlinked member does, and the migration note says both.
    #[test]
    #[cfg(unix)]
    fn a_member_reached_through_a_symlinked_parent_is_rejected() {
        let root = physical_test_root("a_member_reached_through_a_symlinked_parent_is_rejected");
        write(
            &root.join("repo/grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n[workspace]\nmembers = [\"pkgs/api\"]\n",
        );
        write(
            &root.join("store/pkgs/api/grund.toml"),
            "grund_config_version = 1\nproject_name = \"api\"\n",
        );
        std::os::unix::fs::symlink(root.join("store/pkgs"), root.join("repo/pkgs"))
            .expect("point a member's parent segment out of the workspace tree");

        let mut config = load_config(&root.join("repo")).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a member reached through a symlinked parent must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:5: workspace member `pkgs/api` resolves outside the workspace root that lists it",
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

    /// §FS-workspace.2 / §FS-errors.4: a nested block's overlap error names both
    /// entries as the config wrote them, at that block's own `members` line. It
    /// used to print `display_path` of the canonical roots instead, which is a
    /// different string from the one the author wrote. This fixture cannot see the
    /// difference — written and canonical agree in it — so it pins the message
    /// shape; the case below is the one that bites.
    #[test]
    fn nested_workspace_member_overlap_names_both_entries_as_written() {
        let root = test_root("nested_workspace_member_overlap_names_both_entries_as_written");
        for (dir, body) in [
            (
                "",
                "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\"]\n",
            ),
            (
                "group",
                "project_name = \"group\"\n\n[workspace]\nmembers = [\"packages\", \"packages/api\"]\n",
            ),
            ("group/packages/api", "project_name = \"api\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let mut config = load_config(&root).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("one member root containing another must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            "group/grund.toml:4: workspace members overlap: `packages` contains `packages/api`",
            "both entries are named as written, at the line that listed them"
        );
    }

    /// §FS-errors.4, the same rule where the two strings differ: one of the
    /// overlapping entries is a **symlink**, so its canonical root
    /// (`packages/api`) is spelled nothing like the entry that named it (`link`).
    /// Printing `display_path` of the canonical roots — the regression the case
    /// above cannot see, because there the two agree — names a path the author
    /// never wrote and cannot search for.
    #[test]
    #[cfg(unix)]
    fn member_overlap_names_the_entry_not_the_canonical_root() {
        let root = physical_test_root("member_overlap_names_the_entry_not_the_canonical_root");
        for (dir, body) in [
            (
                "",
                "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\"]\n",
            ),
            (
                "group",
                "project_name = \"group\"\n\n[workspace]\nmembers = [\"packages\", \"link\"]\n",
            ),
            ("group/packages/api", "project_name = \"api\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }
        std::os::unix::fs::symlink(root.join("group/packages/api"), root.join("group/link"))
            .expect("name one member through a symlink");

        let mut config = load_config(&root).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("one member root containing another must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            "group/grund.toml:4: workspace members overlap: `packages` contains `link`",
            "the entries as written, even though the second one's canonical root reads differently"
        );
    }

    /// §FS-workspace.2 / §FS-errors.4: a glob whose parent directory is missing
    /// names the entry **as written**, like every other member error. It used to
    /// render the joined path against the block's own root — a base no report uses
    /// — which under `[output] relative_paths = false` could not be made relative
    /// at all and printed an absolute path.
    #[test]
    fn a_missing_glob_parent_names_the_entry_as_written() {
        let root = test_root("a_missing_glob_parent_names_the_entry_as_written");
        write(
            &root.join("grund.toml"),
            "project_name = \"root\"\n\n[output]\nrelative_paths = false\n\n[workspace]\nmembers = [\"packages/*\"]\n",
        );

        let mut config = load_config(&root).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a glob naming a directory that does not exist must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            "grund.toml:7: workspace member glob parent does not exist: packages",
            "the entry is named as written, at the line that listed it"
        );
    }

    /// §FS-workspace.2: an existing glob parent that cannot be read is a
    /// located config error at the `members` line, naming the glob as written
    /// and retaining the operating-system reason.
    #[test]
    #[cfg(unix)]
    fn an_unreadable_glob_parent_is_a_located_members_error() {
        use std::os::unix::fs::PermissionsExt;

        struct RestorePermissions {
            path: PathBuf,
            permissions: std::fs::Permissions,
        }

        impl Drop for RestorePermissions {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.path, self.permissions.clone());
            }
        }

        let root = test_root("an_unreadable_glob_parent_is_a_located_members_error");
        write(
            &root.join("grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\"]\n",
        );
        write(
            &root.join("group/grund.toml"),
            "project_name = \"group\"\n\n[workspace]\nmembers = [\"packages/*\"]\n",
        );
        let packages = root.join("group/packages");
        std::fs::create_dir_all(&packages).expect("create glob parent");
        let permissions = std::fs::metadata(&packages)
            .expect("read original glob-parent permissions")
            .permissions();
        let _restore = RestorePermissions {
            path: packages.clone(),
            permissions,
        };
        std::fs::set_permissions(&packages, std::fs::Permissions::from_mode(0o000))
            .expect("make glob parent unreadable");
        let read_error = match std::fs::read_dir(&packages) {
            Ok(_) => return, // Root/elevated identities can still read mode 000.
            Err(err) => err,
        };

        let mut config = load_config(&root).expect("load workspace root config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("an unreadable glob parent must fail expansion");
        };

        assert_eq!(
            format!("{err:#}"),
            format!(
                "group/grund.toml:4: cannot read workspace member glob `packages/*`: {read_error}"
            )
        );
    }

    /// §FS-workspace.6.1: an unmatched glob is diagnosed only when the
    /// whole block is empty, and multiple unmatched globs name the first entry
    /// in config order so the output is deterministic.
    #[test]
    fn unmatched_globs_only_error_when_the_workspace_is_empty() {
        let valid = test_root("an_unmatched_glob_beside_a_member_is_valid");
        write(
            &valid.join("grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"empty/*\", \"member\"]\ninclude_root = false\n",
        );
        std::fs::create_dir_all(valid.join("empty")).expect("create empty glob parent");
        write(
            &valid.join("member/grund.toml"),
            "project_name = \"member\"\n",
        );
        let mut config = load_config(&valid).expect("load valid workspace config");
        let aliases = expand_workspace_tree(&mut config)
            .expect("another member keeps the workspace in scope")
            .into_iter()
            .map(|entry| entry.alias)
            .collect::<Vec<_>>();
        assert_eq!(aliases, vec!["member"]);

        let empty = test_root("multiple_unmatched_globs_name_the_first_in_config_order");
        write(
            &empty.join("grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"z/*\", \"a/*\"]\ninclude_root = false\n",
        );
        std::fs::create_dir_all(empty.join("z")).expect("create first empty glob parent");
        std::fs::create_dir_all(empty.join("a")).expect("create second empty glob parent");
        let mut config = load_config(&empty).expect("load empty workspace config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a workspace with only unmatched globs must fail");
        };
        assert_eq!(
            format!("{err:#}"),
            "grund.toml:4: the glob `z/*` matched no directories"
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

    /// §FS-workspace.8: an alias path that is not one slug per level names the
    /// **segment** that failed. Before this the message quoted the whole path
    /// against a pattern forbidding `/`, which reads as "a namespace may not
    /// contain `/`" — the opposite of §FS-workspace.1, and in a nested tree the
    /// path is usually mostly right.
    #[test]
    fn an_invalid_alias_path_names_the_offending_segment() {
        let message = |arg: &str| {
            format!(
                "{:#}",
                split_qualified_id_arg(arg).expect_err("malformed alias path is rejected")
            )
        };

        assert_eq!(
            message("group/Alpha/FS-x"),
            "invalid project alias segment `Alpha` in `group/Alpha` (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );
        assert_eq!(
            message("group//FS-x"),
            "invalid project alias `group/`: a segment is empty (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );
        assert_eq!(
            message("/FS-x"),
            "invalid project alias: the path before the ID is empty (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );
        // A single-segment path *is* its own segment, so it is named plainly.
        assert_eq!(
            message("Group/FS-x"),
            "invalid project alias `Group` (expected [a-z][a-z0-9-]*, one segment per workspace level)"
        );

        let (alias, id) = split_qualified_id_arg("group/alpha/FS-x").expect("a valid path splits");
        assert_eq!((alias.as_deref(), id), (Some("group/alpha"), "FS-x"));
    }
}
