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

const SCOPE_SUFFIX: &str = " — here, the {scope} subtree means the {scope} project and its descendants; this wording changes in grund 0.14.0";

fn scope_only(alias: &str, scope: &str) -> String {
    format!(
        "unknown project alias {alias}; only the {scope} subtree is in scope here — check from the workspace root for a path outside it{}",
        SCOPE_SUFFIX.replace("{scope}", scope)
    )
}

fn narrowed_text() -> String {
    format!(
        "docs/FS-group.md:3: unknown project alias group/alph; did you mean group/alpha?\n\
         docs/FS-group.md:4: {}\n\
         docs/FS-group.md:6: {}\n\
         docs/FS-group.md:7: {}\n",
        scope_only("alpha", "group"),
        scope_only("outside/alpha", "group"),
        scope_only("grouped/alpha", "group"),
    )
}

fn narrowed_json() -> String {
    format!(
        concat!(
            "{{\"severity\":\"error\",\"path\":\"docs/FS-group.md\",\"line\":3,\"code\":\"unknown-project\",\"message\":\"unknown project alias group/alph; did you mean group/alpha?\",\"sites\":null}}\n",
            "{{\"severity\":\"error\",\"path\":\"docs/FS-group.md\",\"line\":4,\"code\":\"unknown-project\",\"message\":\"{}\",\"sites\":null}}\n",
            "{{\"severity\":\"error\",\"path\":\"docs/FS-group.md\",\"line\":6,\"code\":\"unknown-project\",\"message\":\"{}\",\"sites\":null}}\n",
            "{{\"severity\":\"error\",\"path\":\"docs/FS-group.md\",\"line\":7,\"code\":\"unknown-project\",\"message\":\"{}\",\"sites\":null}}\n",
        ),
        scope_only("alpha", "group"),
        scope_only("outside/alpha", "group"),
        scope_only("grouped/alpha", "group"),
    )
}

/// §FS-check.3.8.1: `group/alph` is safely inside `group`; shorter,
/// outside-prefix, and lexical-prefix aliases remain ineligible, while the equal
/// alias resolves normally. Both root and narrowed text bytes stay exact.
#[test]
fn narrowed_alias_hints_only_rewrite_paths_inside_the_scope() {
    let root = fixture_root();
    let root_arg = root.to_str().expect("fixture path is UTF-8");

    let workspace = run_grund(&["check", root_arg], Path::new(env!("CARGO_MANIFEST_DIR")));
    assert_failed_with(
        &workspace,
        concat!(
            "group/docs/FS-group.md:3: unknown project alias group/alph; did you mean group/alpha?\n",
            "group/docs/FS-group.md:6: unknown project alias outside/alpha; did you mean alpha or group/alpha?\n",
            "group/docs/FS-group.md:7: unknown project alias grouped/alpha; did you mean alpha or group/alpha?\n",
        ),
        "grund check <workspace-root>",
    );

    let narrowed = run_grund(&["check"], &root.join("group"));
    assert_failed_with(&narrowed, &narrowed_text(), "(cd group && grund check)");
}

/// §FS-errors.3: text and JSON carry the same 0.13.2 message bytes in one
/// error diagnostic per citation; stable codes still drive `--only`/`--ignore`.
#[test]
fn narrowed_scope_clarification_preserves_json_fields_and_code_selection() {
    let root = fixture_root().join("group");
    let json = run_grund(&["check", "--format=json"], &root);
    let selected = run_grund(&["check", "--only=unknown-project"], &root);
    let ignored = run_grund(&["check", "--ignore", "unknown-project"], &root);

    assert_failed_with(&json, &narrowed_json(), "grund check --format=json");
    assert_failed_with(
        &selected,
        &narrowed_text(),
        "grund check --only=unknown-project",
    );
    assert_eq!(ignored.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&ignored.stdout), "success\n");
    assert_eq!(String::from_utf8_lossy(&ignored.stderr), "");
}
