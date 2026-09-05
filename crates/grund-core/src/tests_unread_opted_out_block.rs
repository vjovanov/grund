/// Test module: a `[workspace]` block that opted out of being a project and
/// whose own tree still holds files a scan would have read (§FS-check.4.11,
/// §FS-workspace.6.1).
///
/// The behaviour itself is pinned end to end, in `tests/e2e/cases/`, because the
/// warning is a property of a whole run rather than of one function: it is
/// emitted where a run populates a block's member boundary, so `check`, `list`,
/// `refs`, `cover` and `fmt` each have to carry it — and every configuration
/// that must stay *silent* is a whole run too, which is what half those cases
/// are.
///
/// Two things no golden there can keep, and they are this module's job. The
/// first is that the message a reader meets in the spec is the message the
/// binary prints, since a golden is only ever compared against the binary that
/// produced it. The second is the **symlinked** shapes of §FS-workspace.6's
/// boundary: a case directory is a tracked fixture and the corpus holds no
/// symlink at all, so the one direction a member list cannot see — a scope root
/// that resolves into another project of the run — has to be built at runtime.
/// It is the direction this finding got wrong twice, and each time in a shape
/// that reads correctly on paper: the pruning is what the run *would* have done
/// as a project, not what its config says.
///
/// The sibling `tests_workspace_absorbed_scan` carries two tests this one does
/// not, and both exist to hold a named release ahead of the running version.
/// This finding names no release (§DF-unread-opted-out-block.2.3), so there is
/// no deadline to guard and no version constant for a golden to disagree with.
#[cfg(test)]
mod tests_unread_opted_out_block {
    use std::path::{Path, PathBuf};
    // Only the three symlink cases build a fixture on disk, and all three are
    // `cfg(unix)`; what is left on Windows compares two files in the repository.
    #[cfg(unix)]
    use super::tests_support::*;

    /// The case whose golden holds the shipped message byte for byte.
    const GOLDEN: &str =
        "tests/e2e/cases/workspace-include-root-false-unread-check/expected.stderr";

    /// The spec point that documents the same message as a worked example.
    const SPEC: &str = "docs/functional-spec/FS-check.md";

    fn repo_file(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    /// Absent when the tests run from a packaged crate rather than the
    /// workspace; in the repository — where the two can drift — both files are
    /// always present.
    fn repo_text(relative: &str) -> Option<String> {
        std::fs::read_to_string(repo_file(relative)).ok()
    }

    /// §FS-check.4.11: the message the spec shows and the message the binary
    /// prints are one string. Without this the wording could be corrected in the
    /// golden and left stale in the document a reader reaches by citation, and
    /// every e2e case would still be green — the goldens compare the binary
    /// against itself and never against the prose that promised it.
    #[test]
    fn the_documented_message_is_the_shipped_message() {
        let (Some(golden), Some(spec)) = (repo_text(GOLDEN), repo_text(SPEC)) else {
            return;
        };
        let shipped = golden.trim_end_matches('\n');
        assert!(
            spec.lines().any(|line| line == shipped),
            "{SPEC} does not show the warning {GOLDEN} pins:\n{shipped}"
        );
    }

    /// A root project owning `docs/`, with an opted-out block `group` below it
    /// that lists its own member `alpha`. Everything is defaults apart from the
    /// two `[workspace]` blocks, so the block's scope is `[scan] include`'s own
    /// — `requirements.md` and `docs` among them, which is what puts a **file**
    /// root in reach of these tests without configuring one.
    ///
    /// Unix only: every caller is a symlink case and so `#[cfg(unix)]` too.
    #[cfg(unix)]
    fn opted_out_block(name: &str) -> PathBuf {
        let root = test_root(name);
        write(
            &root.join("grund.toml"),
            "project_name = \"root\"\n\n[workspace]\nmembers = [\"group\"]\n",
        );
        write(&root.join("docs/FS-root.md"), "# FS-root: Root\n\nRoot.\n");
        write(&root.join("requirements.md"), "# FS-req: Req\n\nReq.\n");
        write(
            &root.join("group/grund.toml"),
            "project_name = \"group\"\n\n[workspace]\nmembers = [\"alpha\"]\ninclude_root = false\n",
        );
        write(&root.join("group/alpha/grund.toml"), "project_name = \"alpha\"\n");
        write(&root.join("group/alpha/docs/FS-a.md"), "# FS-a: A\n\nA.\n");
        root
    }

    /// How many blocks the run cautioned about, which is the finding itself
    /// (§FS-check.4.11) counted rather than read off stderr — the count is what
    /// decides the `success` marker, so it cannot be right while the lines are
    /// wrong.
    ///
    /// Unix only: every caller is a symlink case and so `#[cfg(unix)]` too.
    #[cfg(unix)]
    fn cautioned_blocks(root: &Path) -> usize {
        crate::check_with_opts(crate::CheckOpts {
            path: root.to_path_buf(),
            path_provided: true,
            ..crate::CheckOpts::default()
        })
        .expect("check the fixture")
        .unread_opted_out_blocks
    }

    /// §FS-check.4.11, §FS-workspace.6: the block's only scope root **is** a
    /// link into the enclosing project's own `docs`. Those files are read — by
    /// the root project — so the answer to "would this block have read
    /// something, had it been a project?" is no, and the caution must not fire.
    ///
    /// The prune cannot come from the walk's directory filter: a walk root is
    /// never pruned at depth 0 (§FS-config.3.5), so the root is gated before the
    /// walk or not at all.
    #[cfg(unix)]
    #[test]
    fn silent_when_the_scope_root_links_into_another_project() {
        let root = opted_out_block("unread_block_scope_root_links_into_another_project");
        std::os::unix::fs::symlink("../docs", root.join("group/docs")).expect("symlink docs");

        assert_eq!(cautioned_blocks(&root), 0);
    }

    /// §FS-check.4.11, §FS-workspace.6: the same, for a scope root that is a
    /// **file**. It reaches the probe without a directory anywhere in it, so a
    /// gate written into the walk alone would miss it — and `requirements.md` is
    /// a default `[scan] include` entry, so no configuration has to ask for this
    /// shape.
    #[cfg(unix)]
    #[test]
    fn silent_when_a_file_scope_root_links_into_another_project() {
        let root = opted_out_block("unread_block_file_scope_root_links_into_another_project");
        std::os::unix::fs::symlink("../requirements.md", root.join("group/requirements.md"))
            .expect("symlink requirements");

        assert_eq!(cautioned_blocks(&root), 0);
    }

    /// §FS-check.4.11: the other direction, and the one that makes the two above
    /// evidence rather than a way of never firing — a link out of the block to
    /// content **no project of the run owns** is content nobody reads, so the
    /// caution is exactly right there.
    ///
    /// The target is a sibling of the workspace rather than an unscanned corner
    /// of it. A directory inside the enclosing project is *owned* by it, and the
    /// block would have stopped at that project's root had it been one, so the
    /// silence there is the same answer read from the same rule — ownership is
    /// per project root, not per scan root (§FS-workspace.6).
    #[cfg(unix)]
    #[test]
    fn warns_when_the_scope_root_links_outside_every_project() {
        let root = opted_out_block("unread_block_scope_root_links_outside_every_project");
        let outside = test_root("unread_block_scope_root_links_outside_every_project_target");
        write(&outside.join("FS-x.md"), "# FS-x: X\n\nX.\n");
        std::os::unix::fs::symlink(&outside, root.join("group/docs")).expect("symlink outside");

        assert_eq!(cautioned_blocks(&root), 1);
    }
}
