//! §FS-fmt.3 — `grund fmt --write` with no `<path>` argument and `grund fmt
//! --write .` name the same scope (the current directory) and must refuse
//! alike on a tree the whole-declaration-set completeness check would
//! reject. Regression coverage for issue #105: the no-path form reused a
//! project's already-computed `Findings` without checking whether the scan
//! that produced them met an error, so it silently rewrote a tree the
//! explicit-path form correctly refused — a well-formed citation of a real
//! declaration that no later `check` run could tell apart from one the
//! author wrote. `symlink-fmt-write-abort` (`tests/e2e/cases/`) pins the
//! explicit-path refusal this suite compares the no-path form against.
//!
//! Unix only: the fixture needs real broken symlinks, and everything below
//! exists to serve the three `#[test]`s, so the whole file is gated rather than
//! leaving helpers unused (and `-D warnings`-fatal) on a platform that never
//! compiles the tests that call them.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh fixture under this repository's own tree: a project whose
/// `[fmt.cross_refs] enabled = true` needs the whole declaration set before
/// it can rewrite anything, holding one rewritable citation and two broken
/// symlinks the completed scan must report (§FS-fmt.3).
fn build_fixture(name: &str) -> PathBuf {
    let dir = manifest_dir().join("target/fmt-tests").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("docs")).expect("create docs dir");
    fs::write(
        dir.join("grund.toml"),
        "grund_config_version = 1\n\n[scan]\ninclude = [\"docs\"]\n\n[fmt.cross_refs]\nenabled = true\n",
    )
    .expect("write grund.toml");
    fs::write(
        dir.join("docs/FS-001-alpha.md"),
        "# FS-001-alpha: Alpha\n\nCites $$FS-001-alpha.\n",
    )
    .expect("write FS-001-alpha.md");
    fs::write(
        dir.join("docs/notes.md"),
        "# Notes\n\nSee $$FS-001-alpha for the rest.\n",
    )
    .expect("write notes.md");
    std::os::unix::fs::symlink("nowhere.md", dir.join("docs/FS-002-gone.md"))
        .expect("create broken symlink");
    std::os::unix::fs::symlink("nowhere-either.md", dir.join("docs/FS-003-also-gone.md"))
        .expect("create second broken symlink");
    dir
}

fn run_grund(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

const ORIGINAL_NOTES: &str = "# Notes\n\nSee $$FS-001-alpha for the rest.\n";
const ABORT_MESSAGE: &str = concat!(
    "error: nothing was rewritten: docs/FS-002-gone.md: broken symlink: the target does not exist\n",
    "error: nothing was rewritten: docs/FS-003-also-gone.md: broken symlink: the target does not exist\n",
);

#[test]
fn fmt_check_cross_refs_reports_every_unreadable_path_before_any_report() {
    let root = build_fixture("check_cross_refs_complete_abort");

    let output = run_grund(&["fmt", "--check", "--cross-refs", "."], &root);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(stderr(&output), ABORT_MESSAGE);
    assert_eq!(
        stdout(&output),
        "",
        "an aborted run emitted a rewrite report"
    );
}

#[test]
fn fmt_write_refuses_alike_with_and_without_a_path_argument() {
    let omitted = build_fixture("write_path_omitted");
    let explicit = build_fixture("write_path_explicit");

    let without_path = run_grund(&["fmt", "--write"], &omitted);
    let with_path = run_grund(&["fmt", "--write", "."], &explicit);

    assert_eq!(
        without_path.status.code(),
        Some(2),
        "no-path form: {}",
        stderr(&without_path)
    );
    assert_eq!(
        with_path.status.code(),
        Some(2),
        "explicit-path form: {}",
        stderr(&with_path)
    );
    assert_eq!(stderr(&without_path), ABORT_MESSAGE);
    assert_eq!(stderr(&with_path), ABORT_MESSAGE);
    assert_eq!(stdout(&without_path), "");
    assert_eq!(stdout(&with_path), "");

    // The completeness check is fatal *before* any rewrite — reusing a
    // project's already-scanned findings must not let the no-path form skip
    // that guard and write anyway.
    assert_eq!(
        fs::read_to_string(omitted.join("docs/notes.md")).expect("read notes.md"),
        ORIGINAL_NOTES,
        "the no-path form rewrote a tree the explicit-path form refuses"
    );
    assert_eq!(
        fs::read_to_string(explicit.join("docs/notes.md")).expect("read notes.md"),
        ORIGINAL_NOTES
    );
}

#[test]
fn fmt_check_previews_the_same_refusal_with_and_without_a_path_argument() {
    let omitted = build_fixture("check_path_omitted");
    let explicit = build_fixture("check_path_explicit");

    let without_path = run_grund(&["fmt", "--check"], &omitted);
    let with_path = run_grund(&["fmt", "--check", "."], &explicit);

    assert_eq!(without_path.status.code(), with_path.status.code());
    assert_eq!(stdout(&without_path), stdout(&with_path));
    assert_eq!(stderr(&without_path), stderr(&with_path));
}
