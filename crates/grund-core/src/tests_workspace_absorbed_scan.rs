/// Test module: a `[workspace]` block whose members cover every one of its own
/// walk roots (§FS-workspace.2.1), and the deprecation ramp the finding rides
/// (§FS-check.4.7).
///
/// The behaviour itself is pinned end to end, in `tests/e2e/cases/`, because the
/// warning is a property of a whole run rather than of one function: it is
/// emitted where a run populates a block's member boundary, so `list`, `check`,
/// `refs`, `cover` and `fmt` each have to carry it. What is left for a unit test
/// is the promise inside the message — the release the finding becomes an error
/// in — which no golden can keep honest on its own, since a golden is only ever
/// compared against the binary that produced it.
#[cfg(test)]
mod tests_workspace_absorbed_scan {
    use std::path::{Path, PathBuf};

    /// The case whose golden holds the shipped message byte for byte.
    const GOLDEN: &str = "tests/e2e/cases/workspace-member-absorbs-scan-list/expected.stderr";

    /// The spec point that documents the same message as a worked example.
    const SPEC: &str = "docs/functional-spec/FS-check.md";

    /// The clause §REQ-backwards-compatibility.2 requires the warning to carry.
    const DEADLINE: &str = "becomes an error in grund ";

    fn repo_file(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)
    }

    /// Absent when the tests run from a packaged crate rather than the
    /// workspace; in the repository — where the promise can be broken — both
    /// files are always present.
    fn repo_text(relative: &str) -> Option<String> {
        std::fs::read_to_string(repo_file(relative)).ok()
    }

    /// The release named in a message, e.g. `0.14.0` out of `… an error in
    /// grund 0.14.0.` — read as the digits-and-dots run after the clause, with
    /// the sentence's full stop trimmed off the end.
    fn named_release(text: &str) -> Option<String> {
        let tail = text.split(DEADLINE).nth(1)?;
        let run: String = tail
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
            .collect();
        let release = run.trim_end_matches('.');
        (!release.is_empty()).then(|| release.to_string())
    }

    fn version(text: &str) -> Vec<u32> {
        text.split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect()
    }

    /// §FS-check.4.7, §REQ-backwards-compatibility.2: the warning names the
    /// release it becomes an error in, and a named release that has already
    /// passed is a promise grund broke. Held ahead of the running version so the
    /// bump that reaches the deadline fails the build rather than shipping a
    /// message the binary is behind — the guard §RM-workspace-absorbed-scan-error
    /// is spent against, the same one `index_entry_ramp_releases_are_ordered`
    /// keeps for its ramp.
    #[test]
    fn the_absorbed_scan_error_release_is_still_ahead() {
        let Some(golden) = repo_text(GOLDEN) else {
            return;
        };
        let release = named_release(&golden).unwrap_or_else(|| {
            panic!("{GOLDEN} no longer names the release the warning becomes an error in")
        });
        assert!(
            version(env!("CARGO_PKG_VERSION")) < version(&release),
            "this tree is {}, which has reached the release §FS-check.4.7 promised the \
             absorbed-scan warning would become an error in ({release}). Land \
             §RM-workspace-absorbed-scan-error rather than moving the date.",
            env!("CARGO_PKG_VERSION")
        );
    }

    /// §FS-check.4.7, §RM-workspace-absorbed-scan-error: the one place the
    /// release is written in the source is the release the shipped message names.
    /// The guard above reads the bytes a user sees and holds them ahead of the
    /// running version; this ties those bytes to the constant, so a ramp moved in
    /// `ABSORBED_SCAN_ERROR_RELEASE` alone fails here rather than shipping a
    /// message that disagrees with it.
    #[test]
    fn the_release_constant_is_the_release_the_message_names() {
        let Some(golden) = repo_text(GOLDEN) else {
            return;
        };
        assert_eq!(
            named_release(&golden).as_deref(),
            Some(super::ABSORBED_SCAN_ERROR_RELEASE),
            "{GOLDEN} names a different release from the constant the message is built from"
        );
    }

    /// §FS-check.4.7: the whole sentence, assembled from the covered pairs the
    /// rule found — the golden with its `members`-line breadcrumb taken off the
    /// front. Held here as well as end to end because this is where a failure
    /// names the sentence rather than a whole run's stderr.
    #[test]
    fn the_message_is_assembled_from_the_covered_pairs() {
        let Some(golden) = repo_text(GOLDEN) else {
            return;
        };
        let shipped = golden.trim_end_matches('\n');
        let sentence = shipped
            .strip_prefix("warning: grund.toml:16: ")
            .unwrap_or_else(|| panic!("{GOLDEN} is no longer a located warning:\n{shipped}"));
        assert_eq!(
            super::absorbed_scan_warning(&["`docs` in `docs`".to_string()]),
            sentence
        );
    }

    /// §FS-check.4.7: the message the spec shows and the message the binary
    /// prints are one string. Without this the deadline could be kept in the
    /// golden and stale in the document a reader reaches by citation — and the
    /// guard above would still pass, because it only ever reads the golden.
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
