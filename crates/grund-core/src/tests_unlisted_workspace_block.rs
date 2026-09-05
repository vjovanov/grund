/// Test module: the `[workspace]` block no enclosing block lists, and the warning
/// a run that walks into one earns (§FS-check.4.9, §FS-workspace.6.1).
///
/// Split from `tests_workspace_claims.rs`, which keeps the naming half — which
/// block claims a directory and what the projects below it are therefore called.
/// These cases fail together for one reason: whether a run that *meets* an
/// unclaimed block says so. They are about the walk, not about the chain, which is
/// also why their fixtures put a config inside a scanned tree rather than above
/// the run's root (§DF-unlisted-workspace-block.2.3) — except the one case that is
/// about what an ancestor above that root can and cannot answer.
#[cfg(test)]
mod tests_unlisted_workspace_block {
    use super::*;
    use super::tests_support::*;

    /// The ticket's own tree (grund#72): a root listing `a` and scanning `docs`
    /// and `b`, a `b` that declares `[workspace] members = ["c"]` and that the
    /// root does not list, and a citation inside `b` that only resolves at `b`'s
    /// own scope. `[id] format` is slug-only because the ticket's IDs carry no
    /// number, and every folder kind opts out of an index the cases never assert
    /// on (§FS-config.3.4).
    fn unlisted_block_repo(name: &str) -> PathBuf {
        unlisted_block_repo_at(test_root(name))
    }

    /// The same tree at a root the caller chose, for the one case whose fixture
    /// needs a directory *above* the run's root to put an ancestor config in.
    fn unlisted_block_repo_at(root: PathBuf) -> PathBuf {
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [workspace]\nmembers = [\"a\"]\n\n\
             [scan]\ninclude = [\"docs\", \"b\"]\n",
        );
        write(
            &root.join("a/grund.toml"),
            "grund_config_version = 1\nproject_name = \"a\"\n\n[id]\nformat = \"{kind}-{slug}\"\n",
        );
        write(
            &root.join("b/grund.toml"),
            "grund_config_version = 1\nproject_name = \"b\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [workspace]\nmembers = [\"c\"]\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("b/c/grund.toml"),
            "grund_config_version = 1\nproject_name = \"c\"\n\n[id]\nformat = \"{kind}-{slug}\"\n",
        );
        write(&root.join("docs/FS-001-root.md"), "# FS-001-root: Root\n\nThe root project, cited as §FS-001-root\n");
        write(&root.join("a/docs/FS-001-alpha.md"), "# FS-001-alpha: A\n\nA listed member, cited as §FS-001-alpha\n");
        write(
            &root.join("b/docs/FS-001-beta.md"),
            "# FS-001-beta: B\n\nInside the unlisted block, cited as §FS-001-beta, and citing §c/FS-001-gamma\n",
        );
        write(&root.join("b/c/docs/FS-001-gamma.md"), "# FS-001-gamma: C\n\nUnder the unlisted block, cited as §FS-001-gamma\n");
        root
    }

    /// The message the ticket's tree earns, in full. `b/grund.toml`'s `[workspace]`
    /// sits on line 7 of the fixture above.
    const TICKET_MESSAGE: &str = "b/grund.toml:7: this [workspace] is listed by no enclosing workspace \
         — the projects under it are absorbed into `root` instead of named under their own alias path; \
         add \"b\" to [workspace] members in grund.toml, or keep it out of that project's [scan] \
         — an unlisted [workspace] becomes an error in grund 0.14.0";

    /// Every message this rule reported, in report order — what a case about *how
    /// many* findings one block earns asserts on.
    fn block_findings(run: &CheckRun) -> Vec<String> {
        run.report
            .warnings
            .iter()
            .filter(|diagnostic| diagnostic.code == "unlisted-workspace-block")
            .map(|diagnostic| diagnostic.message.clone())
            .collect()
    }

    /// §FS-check.4.9: the finding itself. One warning, at the unlisted block's own
    /// `[workspace]` line, saying its projects are absorbed rather than named under
    /// their own alias path — which is the fact `grund check` used to leave to
    /// `unknown project alias c` and nothing else.
    #[test]
    fn an_unlisted_workspace_block_is_reported_at_its_own_workspace_line() {
        let root = unlisted_block_repo("an_unlisted_workspace_block_is_reported_at_its_own_workspace_line");
        let run = check_run(&root, false);
        let diagnostic = only(&run, "unlisted-workspace-block");
        assert_eq!(diagnostic.message, TICKET_MESSAGE);
    }

    /// §FS-check.4.9: it is a *warning*, and a CLI-level one — `path` and `line`
    /// null, the location inside the message text (§FS-errors.2.2, §FS-errors.5).
    /// Being one of the report's warnings is what stands it in place of the
    /// `success` marker (§FS-check.2.1), which is what makes the deprecation path
    /// of §DF-unlisted-workspace-block.2.1 the right one; a line printed past the
    /// report would leave `check` printing `success` beside its own caution.
    #[test]
    fn the_finding_is_a_report_warning_with_no_path_or_line() {
        let root = unlisted_block_repo("the_finding_is_a_report_warning_with_no_path_or_line");
        let run = check_run(&root, false);
        assert!(
            run.report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.code == "unlisted-workspace-block"),
            "the finding is a warning, so it displaces `success`: {:?}",
            codes(&run)
        );
        let diagnostic = only(&run, "unlisted-workspace-block");
        assert_eq!(diagnostic.path, None, "a CLI-level diagnostic carries no path");
        assert_eq!(diagnostic.line, None, "a CLI-level diagnostic carries no line");
        assert!(diagnostic.sites.is_empty(), "one block, one site");
        assert!(
            run.report
                .errors
                .iter()
                .all(|diagnostic| diagnostic.code != "unlisted-workspace-block"),
            "a warning, so the exit code is unchanged"
        );
    }

    /// §FS-check.4.9: the block's config is found under both discovery names
    /// (§FS-config.1). The walk prunes hidden directories, so `.agents/` is never
    /// met as a walked entry — the probe has to ask each walked *directory* which
    /// config it carries, or half the blocks in the wild go unreported.
    #[test]
    fn an_unlisted_block_configured_under_agents_is_found() {
        let root = unlisted_block_repo("an_unlisted_block_configured_under_agents_is_found");
        std::fs::rename(root.join("b/grund.toml"), root.join("b/.agents-tmp"))
            .expect("move the block config aside");
        std::fs::create_dir_all(root.join("b/.agents")).expect("create .agents");
        std::fs::rename(root.join("b/.agents-tmp"), root.join("b/.agents/grund.toml"))
            .expect("put the block config under .agents");
        let run = check_run(&root, false);
        let diagnostic = only(&run, "unlisted-workspace-block");
        assert!(
            diagnostic.message.starts_with("b/.agents/grund.toml:7:"),
            "the `.agents/` form is named as the file it is: {}",
            diagnostic.message
        );
    }

    /// §FS-check.4.9: `include_root = false` on the unlisted block changes nothing.
    /// The key answers "is this block's own root a project?"; the finding asks
    /// "does anything claim this block?" — and the block still contributes a
    /// segment to every alias path below it, so the two scopes still disagree.
    #[test]
    fn include_root_false_on_the_unlisted_block_reports_the_same_finding() {
        let root = unlisted_block_repo("include_root_false_on_the_unlisted_block_reports_the_same_finding");
        write(
            &root.join("b/grund.toml"),
            "grund_config_version = 1\nproject_name = \"b\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [workspace]\nmembers = [\"c\"]\ninclude_root = false\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        std::fs::remove_dir_all(root.join("b/docs")).expect("the opted-out root scans nothing");
        let run = check_run(&root, false);
        assert_eq!(only(&run, "unlisted-workspace-block").message, TICKET_MESSAGE);
    }

    /// §FS-check.4.9: only the outermost block of a chain. A block below an
    /// unlisted one *is* claimed — by the unlisted block — so listing the outer one
    /// puts the whole chain back in the claimed chain. Two lines for one edit is
    /// what this rule refuses.
    #[test]
    fn only_the_outermost_unlisted_block_of_a_chain_is_reported() {
        let root = unlisted_block_repo("only_the_outermost_unlisted_block_of_a_chain_is_reported");
        write(
            &root.join("b/c/grund.toml"),
            "grund_config_version = 1\nproject_name = \"c\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [workspace]\nmembers = [\"d\"]\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("b/c/d/grund.toml"),
            "grund_config_version = 1\nproject_name = \"d\"\n\n[id]\nformat = \"{kind}-{slug}\"\n",
        );
        write(&root.join("b/c/d/docs/FS-001-delta.md"), "# FS-001-delta: D\n\nTwo levels down, cited as §FS-001-delta\n");
        let run = check_run(&root, false);
        assert_eq!(
            block_findings(&run),
            vec![TICKET_MESSAGE.to_string()],
            "`b/c` is claimed by `b`, so the chain is one finding and one edit"
        );
    }

    /// §FS-check.4.9: a nested directory carrying a plain `grund.toml` with no
    /// `[workspace]` table declares no projects to absorb. It is ordinary tree to
    /// the enclosing walk (§FS-check.1.3) and this rule says nothing about it.
    #[test]
    fn a_nested_config_with_no_workspace_table_is_not_reported() {
        let root = test_root("a_nested_config_with_no_workspace_table_is_not_reported");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [workspace]\nmembers = [\"a\"]\n\n\
             [scan]\ninclude = [\"docs\", \"b\"]\n",
        );
        write(
            &root.join("a/grund.toml"),
            "grund_config_version = 1\nproject_name = \"a\"\n\n[id]\nformat = \"{kind}-{slug}\"\n",
        );
        write(
            &root.join("b/grund.toml"),
            "grund_config_version = 1\nproject_name = \"b\"\n\n[id]\nformat = \"{kind}-{slug}\"\n",
        );
        write(&root.join("docs/FS-001-root.md"), "# FS-001-root: Root\n\nThe root project, cited as §FS-001-root\n");
        write(&root.join("a/docs/FS-001-alpha.md"), "# FS-001-alpha: A\n\nA listed member, cited as §FS-001-alpha\n");
        write(&root.join("b/docs/FS-001-beta.md"), "# FS-001-beta: B\n\nOrdinary tree, cited as §FS-001-beta\n");
        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlisted-workspace-block".to_string()),
            "no [workspace] table, nothing claimed away: {:?}",
            codes(&run)
        );
    }

    /// §FS-check.4.9: a block that *is* listed is inside the claimed chain, so
    /// listing it is one of the two edits that clear the finding.
    #[test]
    fn a_listed_block_is_not_reported() {
        let root = unlisted_block_repo("a_listed_block_is_not_reported");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [workspace]\nmembers = [\"a\", \"b\"]\n\n\
             [scan]\ninclude = [\"docs\", \"b\"]\n",
        );
        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlisted-workspace-block".to_string()),
            "listing the block puts it back in the claimed chain: {:?}",
            codes(&run)
        );
    }

    /// §FS-check.4.9: a run started *at* the block is the block. Its own project
    /// roots are never candidates — they are the scopes it names everything else
    /// from — so the run that has the disagreement to itself says nothing about it.
    #[test]
    fn a_run_started_at_the_unlisted_block_reports_nothing() {
        let root = unlisted_block_repo("a_run_started_at_the_unlisted_block_reports_nothing");
        let run = check_run(&root.join("b"), false);
        assert!(
            !codes(&run).contains(&"unlisted-workspace-block".to_string()),
            "at `b`, `b` is the root: {:?}",
            codes(&run)
        );
    }

    /// §FS-check.4.9: the same exemption where it would bite hardest. `--full`
    /// makes the config root a walk root (§FS-check.1.3), so a rule that did not
    /// exempt the run's own project roots would report every workspace repository
    /// that sits under no enclosing one — which is nearly all of them.
    #[test]
    fn the_runs_own_workspace_root_is_never_reported_under_full() {
        let root = test_root("the_runs_own_workspace_root_is_never_reported_under_full");
        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [id]\nformat = \"{kind}-{slug}\"\n\n\
             [workspace]\nmembers = [\"a\"]\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("a/grund.toml"),
            "grund_config_version = 1\nproject_name = \"a\"\n\n[id]\nformat = \"{kind}-{slug}\"\n",
        );
        write(&root.join("docs/FS-001-root.md"), "# FS-001-root: Root\n\nThe root project, cited as §FS-001-root\n");
        write(&root.join("a/docs/FS-001-alpha.md"), "# FS-001-alpha: A\n\nA listed member, cited as §FS-001-alpha\n");
        let run = check_run(&root, true);
        assert!(
            !codes(&run).contains(&"unlisted-workspace-block".to_string()),
            "the run's own root is the namespace, not a block absorbed into one: {:?}",
            codes(&run)
        );
    }

    /// §FS-check.4.9: the scope is the walk the run already makes, so a narrowed
    /// run that never reaches the block says nothing — the same stance §FS-check.4.6
    /// takes for an index the run did not scan. This is the residue the spec keeps
    /// recording rather than papering over.
    #[test]
    fn a_run_narrowed_away_from_the_block_reports_nothing() {
        let root = unlisted_block_repo("a_run_narrowed_away_from_the_block_reports_nothing");
        let run = check_run(&root.join("docs"), false);
        assert!(
            !codes(&run).contains(&"unlisted-workspace-block".to_string()),
            "a run that cannot see the block does not judge it: {:?}",
            codes(&run)
        );
    }

    /// §FS-check.4.9 "one finding for one edit": the walk reaches `b` twice, once
    /// as itself and once through a directory symlink inside `docs`, and one edit —
    /// listing `b` — clears both. The claim test resolves symlinks, so the second
    /// spelling answers exactly as the first did; only the report has to agree, and
    /// the walk's *file* half already de-aliases in the same run.
    #[cfg(unix)]
    #[test]
    fn one_block_the_walk_reached_twice_is_one_finding() {
        let root = unlisted_block_repo("one_block_the_walk_reached_twice_is_one_finding");
        symlink("../b", &root.join("docs/blink"));
        let run = check_run(&root, false);
        assert_eq!(
            block_findings(&run),
            vec![TICKET_MESSAGE.to_string()],
            "one block and one edit, whatever spelling the walk reached it under"
        );
    }

    /// The likeliest tree in the wild: a repository that is no workspace at all,
    /// carrying a `[workspace]` block somewhere inside its scan. Every other fixture
    /// here gives the root a `[workspace]` table, so this is the shape that exercises
    /// the message's fallback name.
    fn plain_repo_absorbing_the_block(name: &str, project_name: Option<&str>) -> PathBuf {
        let root = unlisted_block_repo_at(test_root(name));
        let named = project_name.map_or_else(String::new, |name| format!("project_name = \"{name}\"\n"));
        write(
            &root.join("grund.toml"),
            &format!(
                "grund_config_version = 1\n{named}\n\
                 [id]\nformat = \"{{kind}}-{{slug}}\"\n\n\
                 [scan]\ninclude = [\"docs\", \"b\"]\n"
            ),
        );
        root
    }

    /// §FS-check.4.9: a run that loaded no workspace has no alias path to quote, so
    /// the absorbing project is named the way the reader would see it the moment the
    /// block is listed — the root's `project_name` (§AR-workspace.5.3), which is what
    /// §FS-list prints for it once the namespace becomes a workspace.
    #[test]
    fn a_repo_with_no_workspace_table_is_named_by_its_project_name() {
        let root = plain_repo_absorbing_the_block(
            "a_repo_with_no_workspace_table_is_named_by_its_project_name",
            Some("myrepo"),
        );
        let run = check_run(&root, false);
        assert_eq!(
            only(&run, "unlisted-workspace-block").message,
            TICKET_MESSAGE.replace("`root`", "`myrepo`"),
        );
    }

    /// The same tree with no `project_name` either: the root alias falls back to
    /// `root` (§AR-workspace.5.3), which is again the spelling listing the block
    /// would produce.
    #[test]
    fn a_repo_with_no_project_name_is_named_root() {
        let root = plain_repo_absorbing_the_block("a_repo_with_no_project_name_is_named_root", None);
        let run = check_run(&root, false);
        assert_eq!(only(&run, "unlisted-workspace-block").message, TICKET_MESSAGE);
    }

    /// The ticket's tree one level down, under an ancestor `[workspace]` whose
    /// `members` the caller writes — the only fixture here with a config *above* the
    /// run's root, because the claim it tests is one only an ancestor can make.
    fn repo_under_an_ancestor_listing(name: &str, members: &str) -> PathBuf {
        let outer = test_root(name);
        let root = unlisted_block_repo_at(outer.join("repo"));
        write(
            &outer.join("grund.toml"),
            &format!(
                "grund_config_version = 1\nproject_name = \"outer\"\n\n\
                 [workspace]\nmembers = {members}\n"
            ),
        );
        root
    }

    /// §FS-check.4.9: a claim an ancestor *names* and then cannot answer — here
    /// `repo/b` listed beside a member that does not exist, so the list will not
    /// expand — leaves the block undecidable in both directions (§FS-workspace.6.1)
    /// and unreported. No answer is not the answer that nothing claims it.
    #[test]
    fn a_claim_the_ancestor_cannot_answer_leaves_the_block_unreported() {
        let root = repo_under_an_ancestor_listing(
            "a_claim_the_ancestor_cannot_answer_leaves_the_block_unreported",
            "[\"repo/b\", \"nope\"]",
        );
        let run = check_run(&root, false);
        assert!(
            !codes(&run).contains(&"unlisted-workspace-block".to_string()),
            "the ancestor names `repo/b` and cannot say what it is: {:?}",
            codes(&run)
        );
    }

    /// The floor under that silence: the claim is read off the entry text before
    /// anything is expanded, so an ancestor that names nothing here is climbed past
    /// however broken it is, and the finding fires as it does with no ancestor at all.
    #[test]
    fn an_ancestor_that_names_nothing_here_silences_nothing() {
        let root = repo_under_an_ancestor_listing(
            "an_ancestor_that_names_nothing_here_silences_nothing",
            "[\"other\"]",
        );
        let run = check_run(&root, false);
        assert_eq!(only(&run, "unlisted-workspace-block").message, TICKET_MESSAGE);
    }

    /// §REQ-backwards-compatibility.2, §DF-unlisted-workspace-block.2.1: the
    /// warning names the release it becomes an error in, and that release is held
    /// ahead of the running version so the deadline fails the build rather than
    /// passing unnoticed — the forcing function §RM-index-entry-error uses, and the
    /// one that carried `[[kinds]] prefix` to the removal §FS-config.3.4.6 records.
    /// Read out of the message rather than off the constant on purpose: the message
    /// is the promise a user was given.
    #[test]
    fn the_named_error_release_is_0_14_0_and_still_ahead() {
        let root = unlisted_block_repo("the_named_error_release_is_0_14_0_and_still_ahead");
        let run = check_run(&root, false);
        let message = only(&run, "unlisted-workspace-block").message.clone();
        let marker = "becomes an error in grund ";
        let named = message
            .split(marker)
            .nth(1)
            .unwrap_or_else(|| panic!("the warning must name its release: {message}"))
            .trim()
            .to_string();
        assert_eq!(named, "0.14.0", "#72 and #78 ramp into the same release");
        let version = |text: &str| {
            text.split('.')
                .map(|part| part.trim_end_matches(|c: char| !c.is_ascii_digit()))
                .map(|part| part.parse::<u32>().unwrap_or(0))
                .collect::<Vec<_>>()
        };
        assert!(
            version(env!("CARGO_PKG_VERSION")) < version(&named),
            "this tree is {}, which has reached the release §FS-check.4.9 promised the warning \
             would become an error in ({named}). Ship §RM-unlisted-workspace-error rather than \
             moving the date.",
            env!("CARGO_PKG_VERSION")
        );
    }
}
