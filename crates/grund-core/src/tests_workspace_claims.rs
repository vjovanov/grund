/// Test module: which `[workspace]` block claims a directory, and what a run at
/// any scope therefore calls the projects below it (§FS-workspace.6.1,
/// §AR-workspace.6.1).
///
/// Split from `tests_workspace_nested.rs` because these cases fail together for
/// one reason: the *claimed chain* — which ancestor names this directory, and
/// which scopes the alias-path guarantee therefore covers. What answering a claim
/// obliges a block to is `tests_workspace_claim_answers.rs`; what a single
/// `members` list may contain is `tests_workspace_nested.rs`.
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
