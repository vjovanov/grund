//! §FS-init.1.2 — the targets `grund init` declines to scaffold into, and the
//! one flag that lifts the rule that has a legitimate other side.
//!
//! These cases live outside `tests/init.rs` and outside the `e2e/cases` corpus
//! because both run inside this repository's own git tree, where the
//! version-control rule can never fire: the condition under test is the state
//! every other init fixture takes for granted. Each case here therefore builds
//! its target under the system temp directory and, for the home-directory rule,
//! points `$HOME` at it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The personal file from the issue report: three lines that have nothing to do
/// with any project, which a refused run must leave byte-for-byte alone.
const PERSONAL_INSTRUCTIONS: &str = "# Personal instructions\n\nNothing to do with any project.\n";

/// A fresh directory under the system temp root — deliberately *not* under
/// `target/`, which is inside this repository's git tree and would satisfy the
/// version-control rule through the walk-up.
fn outside_repo_dir(suffix: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("grund-init-refused-targets")
        .join(suffix);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create target outside the repo");
    assert!(
        !dir.join(".git").exists(),
        "fixture root must not be version-controlled"
    );
    dir
}

fn run_init(args: &[&str], home: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_grund"));
    command.arg("init").args(args);
    if let Some(home) = home {
        command.env("HOME", home);
    }
    command.output().expect("spawn grund")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn home_directory_target_is_refused_and_leaves_the_personal_file_alone() {
    // The reported accident: a shell slip leaves the user in `$HOME`, and
    // `--claude` appends the managed block to the machine-global instruction
    // file every session in every project loads (§FS-init.1.2).
    let home = outside_repo_dir("home_target");
    fs::create_dir_all(home.join(".claude")).expect("create .claude");
    fs::write(home.join(".claude/CLAUDE.md"), PERSONAL_INSTRUCTIONS).expect("write personal file");

    let output = run_init(&["--claude", home.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(
        message.contains("refusing to scaffold into the home directory"),
        "the home rule must be the one that answers, not the version-control rule \
         that also applies to this target: {message}"
    );
    assert_eq!(
        fs::read_to_string(home.join(".claude/CLAUDE.md")).expect("read personal file"),
        PERSONAL_INSTRUCTIONS,
        "a refused run must not append the managed block"
    );
    // A refusal is total — not even the files that would have been fine.
    assert!(!home.join("CLAUDE.md").exists(), "CLAUDE.md was written");
    assert!(!home.join("grund.toml").exists(), "grund.toml was written");
}

#[test]
fn home_directory_refusal_is_lifted_by_no_flag() {
    let home = outside_repo_dir("home_unconditional");
    for flags in [
        vec!["--no-vcs"],
        vec!["--force"],
        vec!["--force", "--no-vcs"],
        vec!["--dry-run", "--no-vcs"],
    ] {
        let mut args = flags.clone();
        args.push(home.to_str().unwrap());
        let output = run_init(&args, Some(&home));
        assert_eq!(
            output.status.code(),
            Some(2),
            "{flags:?} must not lift the home rule: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("refusing to scaffold into the home directory"),
            "{flags:?}: {}",
            stderr(&output)
        );
    }
    assert!(!home.join("AGENTS.md").exists(), "AGENTS.md was written");
    assert!(!home.join("grund.toml").exists(), "grund.toml was written");
}

#[test]
fn target_outside_version_control_is_refused_until_no_vcs() {
    let target = outside_repo_dir("no_vcs");
    // Not $HOME — only the version-control rule applies here.
    let home = outside_repo_dir("no_vcs_home");

    let refused = run_init(&[target.to_str().unwrap()], Some(&home));
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("is not inside a version-controlled tree")
            && stderr(&refused).contains("pass --no-vcs to scaffold anyway"),
        "the refusal must name the flag that proceeds: {}",
        stderr(&refused)
    );
    assert!(
        !target.join("AGENTS.md").exists() && !target.join("grund.toml").exists(),
        "a refused run wrote files"
    );

    let allowed = run_init(&["--no-vcs", target.to_str().unwrap()], Some(&home));
    assert_eq!(allowed.status.code(), Some(0), "{}", stderr(&allowed));
    assert!(target.join("AGENTS.md").is_file(), "AGENTS.md not written");
    assert!(
        target.join("grund.toml").is_file(),
        "grund.toml not written"
    );
}

#[test]
fn dry_run_reports_the_refusal_rather_than_a_preview() {
    let target = outside_repo_dir("dry_run");
    let home = outside_repo_dir("dry_run_home");

    let output = run_init(&["--dry-run", target.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(
        message.contains("is not inside a version-controlled tree"),
        "{message}"
    );
    assert!(
        !message.contains("would-write"),
        "a preview of a run that would be refused is that refusal: {message}"
    );
}

#[test]
fn a_marker_anywhere_above_the_target_satisfies_the_rule() {
    let root = outside_repo_dir("marker_above");
    let home = outside_repo_dir("marker_above_home");
    // A linked worktree and a submodule both write `.git` as a *file*, so the
    // rule tests presence rather than type (§FS-init.1.2).
    fs::write(root.join(".git"), "gitdir: /elsewhere\n").expect("write .git file");
    let nested = root.join("packages/service");
    fs::create_dir_all(&nested).expect("create nested target");

    let output = run_init(&[nested.to_str().unwrap()], Some(&home));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(nested.join("AGENTS.md").is_file(), "AGENTS.md not written");
}

#[test]
fn each_supported_marker_satisfies_the_rule() {
    let home = outside_repo_dir("markers_home");
    for marker in [".git", ".hg", ".jj", ".svn"] {
        let target = outside_repo_dir(&format!("marker{marker}"));
        fs::create_dir_all(target.join(marker)).expect("create marker");
        let output = run_init(&[target.to_str().unwrap()], Some(&home));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{marker} should satisfy the rule: {}",
            stderr(&output)
        );
    }
}
