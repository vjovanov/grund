//! §FS-check.3.8.1 — a narrowed run may hint only for a written alias that is a
//! strict segment-wise extension of its scope. One repository is checked from
//! both the workspace root and the nested `group` scope so paths, ordering,
//! streams, exit status, the admitted hint, and the suppressed controls stay one
//! black-box contract.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/narrowed_alias_hint/repo")
}

fn run_grund(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

fn assert_failed_with(output: &Output, expected_stdout: &str, command: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{command}: exit status; stderr was {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "{command}: stderr"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected_stdout,
        "{command}: stdout"
    );
}

/// §FS-check.3.8.1: `group/alph` is safely inside `group`; the shorter `alpha`
/// and outside `outside` aliases remain ineligible and keep their exact bytes.
#[test]
fn narrowed_alias_hints_only_rewrite_paths_inside_the_scope() {
    let root = fixture_root();
    let root_arg = root.to_str().expect("fixture path is UTF-8");

    let workspace = run_grund(&["check", root_arg], Path::new(env!("CARGO_MANIFEST_DIR")));
    assert_failed_with(
        &workspace,
        "group/docs/FS-group.md:3: unknown project alias group/alph; did you mean group/alpha?\n",
        "grund check <workspace-root>",
    );

    let narrowed = run_grund(&["check"], &root.join("group"));
    assert_failed_with(
        &narrowed,
        concat!(
            "docs/FS-group.md:3: unknown project alias group/alph; did you mean group/alpha?\n",
            "docs/FS-group.md:4: unknown project alias alpha; only the group subtree is in scope here — check from the workspace root for a path outside it\n",
            "docs/FS-group.md:5: unknown project alias outside; only the group subtree is in scope here — check from the workspace root for a path outside it\n",
        ),
        "(cd group && grund check)",
    );
}
