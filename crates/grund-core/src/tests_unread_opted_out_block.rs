/// Test module: a `[workspace]` block that opted out of being a project and
/// whose own tree still holds files a scan would have read (§FS-check.4.10,
/// §FS-workspace.6.1).
///
/// The behaviour itself is pinned end to end, in `tests/e2e/cases/`, because the
/// warning is a property of a whole run rather than of one function: it is
/// emitted where a run populates a block's member boundary, so `check`, `list`,
/// `refs`, `cover` and `fmt` each have to carry it — and every configuration
/// that must stay *silent* is a whole run too, which is what half those cases
/// are. What no golden can keep on its own is that the message a reader meets in
/// the spec is the message the binary prints, since a golden is only ever
/// compared against the binary that produced it. That is this module's one job.
///
/// The sibling `tests_workspace_absorbed_scan` carries two tests this one does
/// not, and both exist to hold a named release ahead of the running version.
/// This finding names no release (§DF-unread-opted-out-block.2.3), so there is
/// no deadline to guard and no version constant for a golden to disagree with.
#[cfg(test)]
mod tests_unread_opted_out_block {
    use std::path::{Path, PathBuf};

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

    /// §FS-check.4.10: the message the spec shows and the message the binary
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
}
