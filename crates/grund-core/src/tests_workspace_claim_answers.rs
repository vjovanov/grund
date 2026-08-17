/// Test module: what answering a claim obliges an enclosing `[workspace]` block
/// to, and which runs it can reach (§FS-workspace.6.1, §AR-workspace.6.1).
///
/// Split from `tests_workspace_claims.rs`, which keeps the naming half — which
/// block claims a directory and what the projects below it are therefore called.
/// These cases fail together for the other reason: whether an error a block's own
/// `members` list earns reaches a run below it, and whether a run reads an alias
/// path at all. The two halves move independently — one is about spelling, the
/// other about propagation — and only this one has fixtures whose run root has to
/// declare a block for the outcome to exist.
#[cfg(test)]
mod tests_workspace_claim_answers {
    use super::*;
    use super::tests_support::*;

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
    ///
    /// The run root declares `[workspace]` because that is the only run that climbs
    /// at all: a project with no block of its own is one project with no alias path
    /// to read, so nothing above it is consulted (§FS-workspace.5). A fixture
    /// without it made the *second* half below assert an outcome no command could
    /// produce.
    #[test]
    fn an_ancestor_with_overlapping_members_that_claims_nothing_is_climbed_past() {
        let root = test_root("an_ancestor_with_overlapping_members_that_claims_nothing_is_climbed_past");
        for (dir, body) in [
            (
                "outer",
                "project_name = \"outer\"\n\n[workspace]\nmembers = [\"packages\", \"packages/api\"]\n",
            ),
            ("outer/packages/api", "project_name = \"api\"\n"),
            (
                "outer/repo",
                "project_name = \"repo\"\n\n[workspace]\nmembers = [\"api\"]\n",
            ),
            ("outer/repo/api", "project_name = \"api\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        let mut config = load_config(&root.join("outer/repo")).expect("load the repository config");
        let entries = expand_workspace_tree(&mut config)
            .expect("an unclaimed ancestor's overlap is not this run's error");
        assert_eq!(
            entries.into_iter().map(|entry| entry.alias).collect::<Vec<_>>(),
            vec!["repo", "api"]
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
    ///
    /// The claimed directory declares `[workspace]` for the same reason as the case
    /// above: only a run that reads an alias path climbs for one (§FS-workspace.5).
    #[test]
    fn an_ancestor_glob_claims_the_child_and_still_owes_it_an_answer() {
        let root = test_root("an_ancestor_glob_claims_the_child_and_still_owes_it_an_answer");
        for (dir, body) in [
            (
                "outer",
                "project_name = \"outer\"\n\n[workspace]\nmembers = [\"deep/*\", \"gone\"]\n",
            ),
            (
                "outer/deep/repo",
                "project_name = \"repo\"\n\n[workspace]\nmembers = [\"leaf\"]\n",
            ),
            ("outer/deep/repo/leaf", "project_name = \"leaf\"\n"),
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

    /// §FS-workspace.6.1: a claiming ancestor whose config does not **load** owes
    /// this run the same answer as one that cannot expand its members — its own
    /// error, from its own `members` line. The claim is read from the `members`
    /// entries on their own for exactly this reason, so it survives a file the
    /// loader rejects. Read off a *loaded* config instead, two mistakes on one
    /// `members` line behaved oppositely: `"missing"` failed the subtree run
    /// (above), while `"/abs"` — which the shape rule rejects, so nothing loads —
    /// read as "claims nothing", dropped the whole segment chain, and let the
    /// subtree spell itself `alpha` where the root says `group/alpha`.
    #[test]
    fn an_enclosing_workspace_whose_config_does_not_load_fails_the_narrowed_run() {
        let root = test_root("an_enclosing_workspace_whose_config_does_not_load_fails_the_narrowed_run");
        for (dir, body) in [
            (
                "",
                "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\", \"/abs\"]\n",
            ),
            (
                "group",
                "project_name = \"group\"\n\n[workspace]\nmembers = [\"alpha\"]\n",
            ),
            ("group/alpha", "project_name = \"alpha\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        // The members-only read answers for a file the loader will not accept:
        // that is what keeps the claim decidable here.
        assert!(
            load_config_at(&root, &root).is_err(),
            "the fixture's enclosing config must be one that does not load"
        );
        assert_eq!(
            ancestor_member_entries(&root.join("grund.toml")).expect("read the members entries"),
            vec!["group".to_string(), "/abs".to_string()],
            "the entries are read from the file text, shape rule and all other keys aside"
        );

        let mut config = load_config(&root.join("group")).expect("load the group config");
        let Err(err) = expand_workspace_tree(&mut config) else {
            panic!("a claiming ancestor whose config does not load must fail the run");
        };
        assert_eq!(
            format!("{err:#}"),
            "../grund.toml:4: invalid [workspace] member `/abs` (expected relative path or trailing /* glob)",
            "the ancestor's own load error, rendered against the root this run was launched at"
        );
    }

    /// §FS-workspace.6.1, the half above must not cost: a config that does not load
    /// and claims **nothing** here is still climbed past, whatever is wrong with it.
    /// A stray `grund.toml` above a repository — in a workspace that never mentions
    /// it — is not that repository's problem, at any depth up to `/`.
    #[test]
    fn an_ancestor_that_does_not_load_and_claims_nothing_here_is_climbed_past() {
        let root = test_root("an_ancestor_that_does_not_load_and_claims_nothing_here_is_climbed_past");
        for (dir, body) in [
            (
                "outer",
                "project_name = \"outer\"\nthis is not a key\n\n[workspace]\nmembers = [\"other\"]\n",
            ),
            ("outer/other", "project_name = \"other\"\n"),
            (
                "outer/deep/a/b/repo",
                "project_name = \"repo\"\n\n[workspace]\nmembers = [\"api\"]\n",
            ),
            ("outer/deep/a/b/repo/api", "project_name = \"api\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }

        assert!(
            load_config_at(&root.join("outer"), &root).is_err(),
            "the fixture's ancestor config must be one that does not load"
        );
        let mut config =
            load_config(&root.join("outer/deep/a/b/repo")).expect("load the repository config");
        let aliases = expand_workspace_tree(&mut config)
            .expect("a config that does not load and claims nothing must not fail this run")
            .into_iter()
            .map(|entry| entry.alias)
            .collect::<Vec<_>>();

        assert_eq!(
            aliases,
            vec!["repo", "api"],
            "an unclaimed repository is named from itself, exactly as it was"
        );
    }

    /// §FS-workspace.6.1: the residue of the members-only read — a config whose
    /// `members` text cannot be obtained at all, so the claim is undecidable in
    /// *both* directions. Failing would let one unreadable `grund.toml` above a
    /// repository break every run inside it, so the run continues; staying silent
    /// is what let a claiming ancestor re-spell the subtree, so it warns, naming
    /// the file against this run's own root and what the reader stands to lose.
    #[test]
    fn an_ancestor_whose_members_text_cannot_be_read_warns_and_lets_the_run_through() {
        let root = test_root("an_ancestor_whose_members_text_cannot_be_read_warns_and_lets_the_run_through");
        for (dir, body) in [
            (
                "outer",
                "project_name = \"outer\"\n\n[workspace]\nmembers = [\"repo\"\n",
            ),
            (
                "outer/repo",
                "project_name = \"repo\"\n\n[workspace]\nmembers = [\"api\"]\n",
            ),
            ("outer/repo/api", "project_name = \"api\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }
        let ancestor_config = root.join("outer/grund.toml");

        let reason = ancestor_member_entries(&ancestor_config)
            .expect_err("a members value that is not a list cannot decide a claim");
        assert_eq!(reason, "`members` is not a list of strings");
        let repo = root.join("outer/repo");
        assert_eq!(
            undecidable_ancestor_claim_warning(&ancestor_config, &repo, &reason),
            "../grund.toml: cannot read [workspace] members (`members` is not a list of strings); \
             alias paths below it may be missing a segment",
            "the file is named from the root this run was launched at, so it climbs out with `..`"
        );

        let mut config = load_config(&repo).expect("load the repository config");
        let aliases = expand_workspace_tree(&mut config)
            .expect("an undecidable claim above the run must not fail it")
            .into_iter()
            .map(|entry| entry.alias)
            .collect::<Vec<_>>();
        assert_eq!(aliases, vec!["repo", "api"]);
    }

    /// §FS-workspace.6.1: the other way the residue is reached — a config file that
    /// cannot be read as text at all. Bytes that are not UTF-8 are the portable
    /// case (a permission bit is not one: `root` can read anything), and they reach
    /// the same warning, because the reason is whatever the read failure said.
    #[test]
    fn an_ancestor_config_that_is_not_text_is_reported_and_climbed_past() {
        let root = test_root("an_ancestor_config_that_is_not_text_is_reported_and_climbed_past");
        for (dir, body) in [
            (
                "outer/repo",
                "project_name = \"repo\"\n\n[workspace]\nmembers = [\"api\"]\n",
            ),
            ("outer/repo/api", "project_name = \"api\"\n"),
        ] {
            write(&root.join(dir).join("grund.toml"), body);
        }
        let ancestor_config = root.join("outer/grund.toml");
        std::fs::write(&ancestor_config, [0x5b, 0x77, 0xff, 0x5d]).expect("write a non-text config");

        assert!(
            ancestor_member_entries(&ancestor_config).is_err(),
            "a config that is not text cannot answer whether it claims anything"
        );
        let mut config = load_config(&root.join("outer/repo")).expect("load the repository config");
        let aliases = expand_workspace_tree(&mut config)
            .expect("an unreadable config above the run must not fail it")
            .into_iter()
            .map(|entry| entry.alias)
            .collect::<Vec<_>>();
        assert_eq!(aliases, vec!["repo", "api"]);
    }

    /// §FS-workspace.5 / §FS-workspace.6.1: the asymmetry every claim case above
    /// rests on — a run climbs only when it has an alias path to read. A project
    /// that declares no `[workspace]` of its own is a single project, resolving its
    /// own IDs and nothing else, so no enclosing claim is consulted and none can
    /// fail it; the same broken claim fails a run one directory up, at the block
    /// whose members it names. Both halves through the run-level loader, because
    /// the asymmetry *is* the run: `expand_workspace_tree` is reached only for a
    /// config that carries a block.
    #[test]
    fn a_run_at_a_project_with_no_workspace_block_never_climbs() {
        let root = test_root("a_run_at_a_project_with_no_workspace_block_never_climbs");
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

        let context = load_workspace_context(&root.join("group/alpha"), true)
            .expect("a project with no [workspace] block of its own must not climb");
        assert!(!context.workspace_loaded);
        assert_eq!(context.projects.len(), 1, "one project, and no alias path");

        let Err(err) = load_workspace_context(&root.join("group"), true) else {
            panic!("the block one level up reads an alias path, so the claim must fail it");
        };
        assert_eq!(
            format!("{err:#}"),
            "../grund.toml:4: workspace member does not exist: missing"
        );
    }
}
