//! §FS-workspace.8.7 / §FS-check.2 — a member's unreadable file earns the
//! same `<path>: <reason>` whichever query command meets it first, spelled
//! from the workspace root like `check` spells it. `fmt` adds its
//! §FS-fmt.3 strict-refusal prefix when automatic cross-references need the
//! complete tree. Issue
//! #103: `list`, `refs`, and `show` used to render that line against the
//! scanning member's own config instead, naming a path that does not exist
//! from where the run was launched. This proves the commands agree on the
//! path rather than pinning one command's bytes in isolation — a case per command
//! lives under `tests/e2e/cases/workspace-{list,refs,show}-member-scan-error`.
//!
//! Unix only: the fixture needs a real broken symlink.

#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A workspace with a root and one member, a citation crossing the boundary
/// in each direction so neither declaration is "never cited", and a broken
/// symlink only the member's walk reaches.
fn build_fixture(name: &str) -> PathBuf {
    let dir = manifest_dir()
        .join("target/workspace-scan-error-tests")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("docs")).expect("create root docs dir");
    fs::create_dir_all(dir.join("packages/sub/docs")).expect("create member docs dir");
    fs::write(
        dir.join("grund.toml"),
        "grund_config_version = 1\nproject_name = \"root\"\n\n\
         [reference]\nmarker = \"\u{a7}\"\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{slug}\"\n\n\
         [scan]\ninclude = [\"docs\"]\n\n\
         [workspace]\nmembers = [\"packages/sub\"]\n",
    )
    .expect("write root grund.toml");
    fs::write(
        dir.join("docs/FS-root-thing.md"),
        "# FS-root-thing: Root concern\n\nRoot leans on \u{a7}sub/FS-sub-thing.\n",
    )
    .expect("write FS-root-thing.md");
    fs::write(
        dir.join("packages/sub/grund.toml"),
        "grund_config_version = 1\nproject_name = \"sub\"\n\n\
         [reference]\nmarker = \"\u{a7}\"\nstrict = true\n\n\
         [id]\nformat = \"{kind}-{slug}\"\n\n\
         [scan]\ninclude = [\"docs\"]\n",
    )
    .expect("write member grund.toml");
    fs::write(
        dir.join("packages/sub/docs/FS-sub-thing.md"),
        "# FS-sub-thing: A thing sub provides\n\nSub leans on \u{a7}root/FS-root-thing.\n",
    )
    .expect("write FS-sub-thing.md");
    std::os::unix::fs::symlink("nowhere.md", dir.join("packages/sub/docs/FS-gone.md"))
        .expect("create broken symlink");
    dir
}

fn run_grund(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

fn error_lines(output: &Output) -> BTreeSet<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter(|line| line.starts_with("error: "))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_query_command_and_strict_fmt_name_a_member_scan_error_the_same_way() {
    let root = build_fixture("query_equivalence");
    const EXPECTED_ERROR: &str =
        "error: packages/sub/docs/FS-gone.md: broken symlink: the target does not exist";

    let commands: &[&[&str]] = &[
        &["check", "."],
        &["list"],
        &["refs", "root/FS-root-thing"],
        &["cover", "."],
        &["show", "sub/FS-sub-thing"],
    ];
    let expected: BTreeSet<String> = [EXPECTED_ERROR.to_string()].into_iter().collect();

    for args in commands {
        let output = run_grund(args, &root);
        assert_eq!(
            output.status.code(),
            Some(2),
            "`grund {}`: exit code, stderr was: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            error_lines(&output),
            expected,
            "`grund {}`: error lines on stderr",
            args.join(" ")
        );
    }

    // §FS-fmt.3 / §FS-fmt.6.6: automatic cross-references make the complete
    // declaration scan ordinary in check mode too. The path stays identical;
    // the prefix says this formatter run aborted before reporting any rewrite.
    let fmt = run_grund(&["fmt", "--check", "."], &root);
    assert_eq!(fmt.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&fmt.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&fmt.stderr),
        "error: nothing was rewritten: packages/sub/docs/FS-gone.md: broken symlink: the target does not exist\n"
    );
}

#[test]
/// §FS-fmt.3 / §FS-fmt.6.6: the automatically enabled pass and the explicit
/// flag take the same strict workspace path and preserve root-then-member order.
fn automatic_and_explicit_cross_refs_report_workspace_errors_in_scan_order() {
    let root = build_fixture("strict_complete_abort");
    std::os::unix::fs::symlink("nowhere.md", root.join("docs/FS-gone.md"))
        .expect("create root broken symlink");

    let automatic = run_grund(&["fmt", "--check", "."], &root);
    let explicit = run_grund(&["fmt", "--check", "--cross-refs", "."], &root);

    assert_eq!(automatic.status.code(), Some(2));
    assert_eq!(explicit.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&automatic.stdout), "");
    assert_eq!(String::from_utf8_lossy(&explicit.stdout), "");
    assert_eq!(automatic.stderr, explicit.stderr);
    assert_eq!(
        String::from_utf8_lossy(&automatic.stderr),
        concat!(
            "error: nothing was rewritten: docs/FS-gone.md: broken symlink: the target does not exist\n",
            "error: nothing was rewritten: packages/sub/docs/FS-gone.md: broken symlink: the target does not exist\n",
        )
    );
}

#[test]
/// §FS-fmt.3: a later member's strict refusal is discovered before a
/// write-capable pass can mutate a readable file in the root project.
fn strict_workspace_write_does_not_mutate_root_before_member_abort() {
    let root = build_fixture("strict_write_abort");
    let path = root.join("docs/FS-root-thing.md");
    let before = fs::read(&path).expect("read root document before fmt");

    let output = run_grund(&["fmt", "--write", "."], &root);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: nothing was rewritten: packages/sub/docs/FS-gone.md: broken symlink: the target does not exist\n"
    );
    assert_eq!(
        fs::read(path).expect("read root document after fmt"),
        before,
        "the earlier root project was rewritten before the member refused"
    );
}
