/// Test module: which `[workspace]` block claims a directory, and what a run at
/// any scope therefore calls the projects below it (§FS-workspace.6.1,
/// §AR-workspace.6.1).
///
/// Split from `tests_workspace_nested.rs` because these cases fail together for
/// one reason: the *claimed chain* — which ancestor names this directory, what
/// answering that claim obliges it to, and which scopes the alias-path guarantee
/// covers. What a single `members` list may contain is the other file's subject.
#[cfg(test)]
mod tests_workspace_claims {
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

    /// §FS-workspace.6.1, the recorded limitation of the scope guarantee, pinned so
    /// the spec stays honest about it: alias paths are stable for every scope *in*
    /// the claimed chain, and `grp` — hopped by `mid`'s multi-segment entry
    /// `grp/inner`, listed by nobody — is not one of them. A run started there
    /// names its tree from itself, even though the projects below it *are* reached
    /// by the chain, so `§inner/leaf/…` passes here and fails at the root.
    #[test]
    fn a_block_the_chain_never_lists_respells_its_own_subtree() {
        let root = test_root("a_block_the_chain_never_lists_respells_its_own_subtree");
        for (dir, body) in [
            ("", "project_name = \"skip\"\n\n[workspace]\nmembers = [\"mid\"]\n"),
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
            "the outermost root reaches the leaf through the hopped block"
        );
        assert_eq!(
            aliases_at(&root.join("mid/grp")),
            vec!["grp", "inner", "inner/leaf"],
            "a scope outside the claimed chain names its whole tree from itself"
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
        assert_eq!(
            err, "../grund.toml:4: workspace member does not exist: missing",
            "the enclosing block's own error, at the file that holds the line: \
             rendered against the root this run was launched at, so it climbs out \
             of the subtree with `..` instead of naming a same-shaped file inside it"
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
        assert_eq!(
            err,
            "../grund.toml:1: invalid workspace project alias `My_Group` (expected [a-z][a-z0-9-]*) for workspace member `mid`",
            "the ancestor's own alias error, at its own `project_name` line: {err}"
        );
    }

    /// §FS-workspace.6.1: every obligation of the ancestor climb is scoped to a
    /// **claim**. A block four levels up that declares `[workspace]`, lists one
    /// directory that does not exist, and never lists this repository says nothing
    /// about this tree — so its error is not this run's error. Expanding every
    /// declaring ancestor instead made one broken `members` list anywhere above a
    /// repository, at any depth up to `/`, the answer to every command inside it.
    #[test]
    fn an_ancestor_that_claims_nothing_here_cannot_break_the_run() {
        let root = test_root("an_ancestor_that_claims_nothing_here_cannot_break_the_run");
        for (dir, body) in [
            (
                "outer",
                "project_name = \"outer\"\n\n[workspace]\nmembers = [\"gone\"]\n",
            ),
            (
                "outer/deep/a/b/repo",
                "project_name = \"repo\"\n\n[workspace]\nmembers = [\"api\"]\n",
            ),
            ("outer/deep/a/b/repo/api", "project_name = \"api\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let repo = root.join("outer/deep/a/b/repo");
        let mut config = load_config(&repo).expect("load the repository config");
        let aliases = expand_workspace_tree(&mut config)
            .expect("a broken ancestor that claims nothing must not fail this run")
            .into_iter()
            .map(|entry| entry.alias)
            .collect::<Vec<_>>();

        assert_eq!(
            aliases,
            vec!["repo", "api"],
            "an unclaimed repository is named from itself, exactly as it was"
        );
    }

    /// §FS-workspace.6.1, the same rule against a different expansion failure:
    /// overlapping members. The class is what matters — a non-claiming ancestor is
    /// never expanded, so *no* error its member list could earn reaches a run
    /// below it.
    #[test]
    fn an_ancestor_with_overlapping_members_that_claims_nothing_is_climbed_past() {
        let root = test_root("an_ancestor_with_overlapping_members_that_claims_nothing_is_climbed_past");
        for (dir, body) in [
            (
                "outer",
                "project_name = \"outer\"\n\n[workspace]\nmembers = [\"packages\", \"packages/api\"]\n",
            ),
            ("outer/packages/api", "project_name = \"api\"\n"),
            ("outer/repo", "project_name = \"repo\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let mut config = load_config(&root.join("outer/repo")).expect("load the repository config");
        let entries = expand_workspace_tree(&mut config)
            .expect("an unclaimed ancestor's overlap is not this run's error");
        assert_eq!(
            entries.into_iter().map(|entry| entry.alias).collect::<Vec<_>>(),
            vec!["repo"]
        );

        // The claim, not the declaration, is what makes the ancestor's error this
        // run's error: list the same directory and the run fails with it.
        write(
            &root.join("outer/grund.toml"),
            "project_name = \"outer\"\n\n[workspace]\nmembers = [\"repo\", \"packages\", \"packages/api\"]\n",
        );
        let mut config = load_config(&root.join("outer/repo")).expect("load the repository config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a claiming ancestor that cannot expand must fail the run");
        };
        assert!(
            format!("{err:#}").contains("workspace members overlap: `packages` contains `packages/api`"),
            "the claimed chain reports the enclosing block's own error: {err:#}"
        );
    }

    /// §FS-workspace.6.1 / §AR-workspace.5.3: a glob claims the directories under
    /// its parent, so a block whose only mention of this repository is
    /// `deep/*` still owes it an answer — and a `members` list it cannot expand
    /// still fails the run. The claim is read from the entry text, and the entry
    /// text here names a set.
    #[test]
    fn an_ancestor_glob_claims_the_child_and_still_owes_it_an_answer() {
        let root = test_root("an_ancestor_glob_claims_the_child_and_still_owes_it_an_answer");
        for (dir, body) in [
            (
                "outer",
                "project_name = \"outer\"\n\n[workspace]\nmembers = [\"deep/*\", \"gone\"]\n",
            ),
            ("outer/deep/repo", "project_name = \"repo\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let mut config = load_config(&root.join("outer/deep/repo")).expect("load the member config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a glob that claims this directory must not drop its segment");
        };
        assert!(
            format!("{err:#}").contains("workspace member does not exist: gone"),
            "the claiming block's own error, from its own members line: {err:#}"
        );
    }

    /// §FS-workspace.6.1: a `members` entry may reach this directory through a
    /// symlink, and then the entry text names it only once resolved. The claim
    /// therefore compares canonical paths too — otherwise the prefix is dropped
    /// and the subtree names itself, which is the re-spelling §FS-check.3.8 would
    /// then hint at.
    #[test]
    #[cfg(unix)]
    fn an_ancestor_claim_through_a_symlinked_entry_keeps_the_prefix() {
        let root = physical_test_root("an_ancestor_claim_through_a_symlinked_entry_keeps_the_prefix");
        for (dir, body) in [
            (
                "",
                "project_name = \"outer\"\n\n[workspace]\nmembers = [\"link\"]\n",
            ),
            (
                "pkgs/kid",
                "project_name = \"kid\"\n\n[workspace]\nmembers = [\"leaf\"]\n",
            ),
            ("pkgs/kid/leaf", "project_name = \"leaf\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }
        std::os::unix::fs::symlink(root.join("pkgs/kid"), root.join("link"))
            .expect("name the member through a symlink");

        let mut config = load_config(&root.join("pkgs/kid")).expect("load the member config");
        let aliases = expand_workspace_tree(&mut config)
            .expect("expand the narrowed run")
            .into_iter()
            .map(|entry| entry.alias)
            .collect::<Vec<_>>();

        assert_eq!(
            aliases,
            vec!["kid", "kid/leaf"],
            "the symlinked claim is found, so the narrowed run keeps the outer spelling"
        );
    }
}
