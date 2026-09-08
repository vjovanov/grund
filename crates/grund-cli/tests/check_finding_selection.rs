//! Binary-level contract for exact-code `check` selection (§FS-check.1), its
//! selected output and exit decision (§FS-check.2.1), and unhideable incomplete
//! scans (§FS-check.2). Each assertion invokes the shipped `grund` frontend.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MAINTENANCE_TAIL: &str =
    " — repo maintenance; citation checks still ran; wording changes in grund 0.14.0";
const OUTDATED: &str = "outdated grund init block v3 (run `grund init` to update to v8)";

fn fixture_root(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/issue-49-finding-selection")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create fixture tree");
    fs::write(
        root.join("grund.toml"),
        concat!(
            "grund_config_version = 1\n\n",
            "[reference]\nmarker = \"\u{a7}\"\nstrict = true\n\n",
            "[id]\nformat = \"{kind}-{slug}\"\nslug_pattern = \"[a-z][a-z0-9-]*\"\n\n",
            "[scan]\ninclude = [\".\"]\n\n",
            "[[kinds]]\nkind = \"FS\"\nfolder = \"docs/functional-spec\"\nindex = false\n",
        ),
    )
    .expect("write grund.toml");
    root
}

fn write(path: impl AsRef<Path>, text: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, text).expect("write fixture file");
}

fn stale_fixture(name: &str, dangling: bool) -> PathBuf {
    let root = fixture_root(name);
    write(
        root.join("AGENTS.md"),
        "## Grounding with grund (v3)\n\nlegacy block\n",
    );
    write(
        root.join("docs/functional-spec/FS-live.md"),
        "# FS-live: Live behavior\n\nThe behavior cites \u{a7}FS-live.\n",
    );
    if dangling {
        write(
            root.join("docs/notes.md"),
            "This content cites \u{a7}FS-missing.\n",
        );
    }
    root
}

fn run(root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .arg("check")
        .arg(root)
        .args(extra)
        .output()
        .expect("run grund check")
}

fn text(output: &Output) -> (&str, &str) {
    (
        std::str::from_utf8(&output.stdout).expect("stdout is UTF-8"),
        std::str::from_utf8(&output.stderr).expect("stderr is UTF-8"),
    )
}

fn assert_run(output: &Output, exit: i32, stdout: &str, stderr: &str) {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stderr: {}",
        text(output).1
    );
    assert_eq!(text(output), (stdout, stderr));
}

fn maintenance_line() -> String {
    format!("AGENTS.md:1: {OUTDATED}{MAINTENANCE_TAIL}\n")
}

#[test]
fn issue_49_default_stays_red_while_selected_empty_text_and_json_are_clean() {
    let root = stale_fixture("selected-empty", false);

    assert_run(&run(&root, &[]), 1, &maintenance_line(), "");
    assert_run(
        &run(&root, &["--ignore", "agents-init"]),
        0,
        "success\n",
        "",
    );
    assert_run(
        &run(&root, &["--ignore=agents-init", "--format=json"]),
        0,
        "",
        "",
    );
    assert_run(&run(&root, &["--only", "unused"]), 0, "success\n", "");
}

#[test]
fn issue_49_only_ignore_unions_precedence_duplicates_and_order_are_exact() {
    let root = stale_fixture("set-composition", true);
    let maintenance = maintenance_line();
    let dangling = "docs/notes.md:1: unknown reference FS-missing\n";
    let both = format!("{maintenance}{dangling}");

    assert_run(&run(&root, &["--only", "agents-init"]), 1, &maintenance, "");
    assert_run(&run(&root, &["--only=dangling"]), 1, dangling, "");
    assert_run(&run(&root, &["--ignore", "agents-init"]), 1, dangling, "");
    assert_run(
        &run(
            &root,
            &[
                "--only=dangling",
                "--only",
                "agents-init",
                "--only",
                "dangling",
            ],
        ),
        1,
        &both,
        "",
    );
    assert_run(
        &run(
            &root,
            &[
                "--only",
                "agents-init",
                "--only=dangling",
                "--ignore=agents-init",
                "--ignore",
                "agents-init",
            ],
        ),
        1,
        dangling,
        "",
    );
    assert_run(
        &run(
            &root,
            &[
                "--ignore",
                "agents-init",
                "--ignore=dangling",
                "--ignore",
                "dangling",
            ],
        ),
        0,
        "success\n",
        "",
    );
}

#[test]
fn issue_49_retained_json_is_the_ordinary_ndjson_object() {
    let root = stale_fixture("retained-json", true);
    assert_run(
        &run(&root, &["--ignore=agents-init", "--format", "json"]),
        1,
        concat!(
            "{\"severity\":\"error\",\"path\":\"docs/notes.md\",\"line\":1,",
            "\"code\":\"dangling\",\"message\":\"unknown reference FS-missing\",",
            "\"sites\":null}\n",
        ),
        "",
    );
}

#[test]
fn issue_49_warnings_and_enabled_suggestions_are_selectable_but_do_not_fail() {
    let warning_root = fixture_root("warning-channel");
    write(
        warning_root.join("docs/functional-spec/FS-unused.md"),
        "# FS-unused: Unused behavior\n",
    );
    assert_run(
        &run(&warning_root, &["--only=unused"]),
        0,
        "docs/functional-spec/FS-unused.md:1: declared but never cited: FS-unused\n",
        "",
    );
    assert_run(
        &run(&warning_root, &["--ignore", "unused"]),
        0,
        "success\n",
        "",
    );

    let suggestion_root = fixture_root("suggestion-channel");
    write(
        suggestion_root.join("docs/functional-spec/FS-session.md"),
        "# FS-session: Session behavior\n\nThe session lifecycle.\n",
    );
    write(
        suggestion_root.join("docs/guide.md"),
        "The session is \u{a7}FS-session. An example is `<\u{a7}>FS-session`.\n",
    );
    let suggestion = concat!(
        "docs/guide.md:1: escaped citation <\u{a7}>FS-session resolves to a declaration; ",
        "write \u{a7}FS-session for a live citation, or leave it escaped if it is only an illustration\n",
    );
    assert_run(
        &run(
            &suggestion_root,
            &["--suggestions", "--only", "escaped-citation-resolves"],
        ),
        0,
        suggestion,
        "",
    );
    assert_run(
        &run(&suggestion_root, &["--only=escaped-citation-resolves"]),
        0,
        "success\n",
        "",
    );
}

#[test]
fn issue_49_selector_errors_are_exact_and_happen_before_scanning() {
    let absent = fixture_root("validation-order").join("does-not-exist");
    let cases: &[(&[&str], &str)] = &[
        (&["--only"], "error: --only requires a finding code\n"),
        (&["--only="], "error: --only requires a finding code\n"),
        (&["--ignore"], "error: --ignore requires a finding code\n"),
        (&["--ignore="], "error: --ignore requires a finding code\n"),
        (
            &["--only", "Agents_Init"],
            "error: invalid finding code \"Agents_Init\" (expected lowercase kebab-case)\n",
        ),
        (
            &["--ignore=agents-init,dangling"],
            "error: invalid finding code \"agents-init,dangling\" (expected lowercase kebab-case)\n",
        ),
        (
            &["--only=--only=agents-init"],
            "error: invalid finding code \"--only=agents-init\" (expected lowercase kebab-case)\n",
        ),
        (
            &["--ignore=--ignore=agents-init"],
            "error: invalid finding code \"--ignore=agents-init\" (expected lowercase kebab-case)\n",
        ),
        (
            &["--only=not-real"],
            concat!(
                "error: unknown check finding code \"not-real\"; ",
                "run `grund check --help` for supported codes\n",
            ),
        ),
        (
            &["--ignore", "not-real"],
            concat!(
                "error: unknown check finding code \"not-real\"; ",
                "run `grund check --help` for supported codes\n",
            ),
        ),
    ];

    for (args, stderr) in cases {
        assert_run(&run(&absent, args), 2, "", stderr);
    }
}

#[test]
fn issue_49_check_help_exposes_the_sorted_public_code_catalog() {
    const CODES: &[&str] = &[
        "agents-init",
        "broken-stub",
        "dangling",
        "declaration-near-miss",
        "deprecated-config-location",
        "discouraged-citation",
        "duplicate",
        "duplicate-section",
        "empty-citation-obligation",
        "empty-scan",
        "escaped-citation-resolves",
        "forbidden-citation",
        "full-scope-ignored",
        "inline-citation-style",
        "invalid-value-binding",
        "invalid-value-declaration",
        "io",
        "misplaced-declaration",
        "missing-citation",
        "missing-index-entry",
        "missing-section",
        "missing-snapshot",
        "nothing-recognized",
        "optional-member-absent",
        "orphan-section",
        "out-of-scope-dangling",
        "out-of-scope-missing-section",
        "out-of-scope-shorthand-citation",
        "out-of-scope-unknown-project",
        "redundant-config",
        "section-heading-level",
        "shorthand-citation",
        "shorthand-numeric-run",
        "suggested-citation",
        "ungrounded",
        "unknown-project",
        "unlinked-index-entry",
        "unlisted-workspace-block",
        "unused",
        "value-mismatch",
    ];
    let output = Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(["check", "--help"])
        .output()
        .expect("run check help");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output).1, "");
    let stdout = text(&output).0;
    assert!(stdout.contains("--only <code>"));
    assert!(stdout.contains("--ignore <code>"));
    assert!(stdout.contains("--ignore agents-init"));
    assert!(stdout.contains("operational failures remain visible and exit 2"));
    let expected_catalog = format!(
        "Supported check finding codes:\n{}\n",
        CODES
            .iter()
            .map(|code| format!("  {code}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        stdout.contains(&expected_catalog),
        "help did not contain the sorted catalog:\n{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn issue_49_selector_cannot_hide_an_incomplete_scan() {
    use std::os::unix::fs::symlink;

    let root = fixture_root("incomplete-scan");
    write(
        root.join("docs/functional-spec/FS-live.md"),
        "# FS-live: Live behavior\n\nThe behavior cites \u{a7}FS-live.\n",
    );
    symlink(
        "missing-target.md",
        root.join("docs/functional-spec/FS-gone.md"),
    )
    .expect("create broken symlink");

    let output = run(&root, &["--ignore", "io"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(text(&output).0, "");
    assert_eq!(
        text(&output).1,
        "error: docs/functional-spec/FS-gone.md: broken symlink: the target does not exist\n"
    );
}
