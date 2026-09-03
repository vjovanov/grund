//! §FS-init.4 — the gate `grund init --check` is, and the preview `--dry-run`
//! stays.
//!
//! A managed block can drift in its rendered text while its `(vN)` heading is
//! still current, and `--dry-run` has always seen it: it prints `would-update`
//! and exits `0`, because a preview is a report and not a verdict. `--check`
//! turns that same report into one ([grund#172](https://github.com/vjovanov/grund/issues/172)).
//! These cases pin the pair on one tree, because the whole change is the claim
//! that only the exit code separates them (§FS-init.1, §FS-init.2.2).

use std::fs;
use std::path::Path;
use std::process::Output;

#[path = "support/init_fixture.rs"]
mod init_fixture;

use init_fixture::{manifest_dir, run_grund, workdir};

/// A scaffolded target whose managed block has one rendered line edited and its
/// `## Grounding with grund (vN)` heading left alone — the ticket's own tree.
/// The edit is found through the ownership delimiters rather than by quoting a
/// line of the block, so a block-version bump cannot silently turn this fixture
/// into a tree with no drift in it (§FS-init.2.3.1).
fn drifted_target(name: &str) -> std::path::PathBuf {
    let target = workdir(name);
    let scaffold = run_grund(
        &["init", target.to_str().unwrap(), "--agents-md"],
        manifest_dir(),
    );
    assert!(
        scaffold.status.success(),
        "fixture scaffold failed: {}",
        stderr(&scaffold)
    );

    let path = target.join("AGENTS.md");
    let before = fs::read_to_string(&path).expect("read AGENTS.md");
    let heading = version_heading(&before);
    let edited = drift_last_line_of_block(&before);
    assert_ne!(edited, before, "the fixture edited nothing");
    assert_eq!(
        version_heading(&edited),
        heading,
        "the fixture must drift the rendered text, not the block version"
    );
    fs::write(&path, edited).expect("write drifted AGENTS.md");
    target
}

fn version_heading(contents: &str) -> String {
    contents
        .lines()
        .find(|line| line.starts_with("## Grounding with grund ("))
        .unwrap_or_else(|| panic!("no managed-block version heading in:\n{contents}"))
        .to_string()
}

/// Append a few words to the last rendered line inside the managed block. The
/// delimiters are the boundary `init` owns (§FS-init.2.3.1), so an edit placed
/// by them is drift `init` must report however the block is worded.
fn drift_last_line_of_block(contents: &str) -> String {
    const END: &str = "<!-- END GRUND MANAGED BLOCK -->";
    let mut lines: Vec<String> = contents.lines().map(str::to_string).collect();
    let end = lines
        .iter()
        .position(|line| line.trim() == END)
        .unwrap_or_else(|| panic!("no managed-block end delimiter in:\n{contents}"));
    let target = lines[..end]
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .expect("managed block has no content line");
    lines[target].push_str(" — and a hand edit that no version bump records.");
    let mut out = lines.join("\n");
    if contents.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current).expect("read fixture dir") {
            let path = entry.expect("read fixture entry").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(dir)
                    .expect("path inside the fixture")
                    .display()
                    .to_string();
                files.push((relative, fs::read(&path).expect("read fixture file")));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn check_reports_what_dry_run_reports_and_only_the_exit_code_differs() {
    // The regression guard for both halves of §FS-init.4: `--check` earns `1` on
    // a pending change, `--dry-run` keeps `0` for the caller who was previewing,
    // and neither run may say anything the other does not.
    let target = drifted_target("check_matches_dry_run");
    let path = target.to_str().unwrap();

    let preview = run_grund(&["init", path, "--agents-md", "--dry-run"], manifest_dir());
    let gate = run_grund(&["init", path, "--agents-md", "--check"], manifest_dir());

    assert_eq!(
        preview.status.code(),
        Some(0),
        "--dry-run must keep exit 0 on a pending change (§REQ-backwards-compatibility.1): {}",
        stderr(&preview)
    );
    assert_eq!(
        gate.status.code(),
        Some(1),
        "--check must exit 1 when the run reported a would-… line: {}",
        stderr(&gate)
    );
    assert!(
        stderr(&gate).contains("would-update AGENTS.md"),
        "--check must report the drift it gated on: {}",
        stderr(&gate)
    );
    assert_eq!(
        stderr(&gate),
        stderr(&preview),
        "--check prints the --dry-run report for the same tree, byte for byte"
    );
    assert_eq!(
        stdout(&gate),
        stdout(&preview),
        "stdout stays empty for both"
    );
    assert_eq!(stdout(&gate), "", "init writes nothing to stdout");
}

#[test]
fn check_exits_zero_when_every_reported_path_already_exists() {
    let target = workdir("check_current_tree");
    let path = target.to_str().unwrap();
    let scaffold = run_grund(&["init", path, "--agents-md"], manifest_dir());
    assert!(scaffold.status.success(), "{}", stderr(&scaffold));

    let gate = run_grund(&["init", path, "--agents-md", "--check"], manifest_dir());
    assert_eq!(
        gate.status.code(),
        Some(0),
        "a current tree has no pending change: {}",
        stderr(&gate)
    );
    assert!(
        !stderr(&gate).contains("would-"),
        "a current tree reports only `exists ` lines: {}",
        stderr(&gate)
    );
}

#[test]
fn check_writes_nothing_even_with_force() {
    // §FS-init.1: `--check` composes with every other flag the way `--dry-run`
    // does, and `--force` is the one that would otherwise rewrite the canonical
    // AGENTS.md whole (§FS-init.3).
    let target = drifted_target("check_never_writes");
    let path = target.to_str().unwrap();
    let before = snapshot(&target);

    let gate = run_grund(
        &["init", path, "--agents-md", "--check", "--force"],
        manifest_dir(),
    );

    assert_eq!(
        gate.status.code(),
        Some(1),
        "--check --force still gates rather than writing: {}",
        stderr(&gate)
    );
    assert_eq!(
        snapshot(&target),
        before,
        "--check wrote to the tree under --force"
    );
}
