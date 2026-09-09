// §FS-refs.4 / §FS-errors.5: the two resolver rejections share one staged
// exit and wire contract across refs, show, text, JSON, and rendering flags.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const WARNING: &str = "warning: `grund refs` invalid IDs and ambiguous number-only shorthands currently exit 2; they will exit 1 (failed query) in grund 0.15.0\n";
const FORMAT_HINT: &str = "hint: this repo's [id] format is `{kind}-{number}-{slug}` (run `grund config show`); `grund list` shows the IDs that exist\n";
const INVALID: &str = "invalid ID `FS-bar`";
const AMBIGUOUS: &str = "ambiguous ID: FS-042 (matches FS-042-user-login, FS-042-user-logout)";

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "grund-refs-query-failure-{test}-{}-{unique}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale fixture");
        }
        fs::create_dir_all(root.join("docs")).expect("create fixture docs");
        fs::write(
            root.join("grund.toml"),
            "grund_config_version = 1\n\
             project_name = \"root\"\n\n\
             [reference]\n\
             strict = false\n\n\
             [id]\n\
             format = \"{kind}-{number}-{slug}\"\n\n\
             [scan]\n\
             include = [\"docs\"]\n",
        )
        .expect("write fixture config");
        fs::write(
            root.join("docs/FS-042-user-login.md"),
            "# FS-042-user-login: User login\n\nLogin.\n",
        )
        .expect("write login declaration");
        fs::write(
            root.join("docs/FS-042-user-logout.md"),
            "# FS-042-user-logout: User logout\n\nLogout.\n",
        )
        .expect("write logout declaration");
        fs::write(
            root.join("docs/FS-100-empty.md"),
            "# FS-100-empty: No inbound citations\n\nEmpty is a valid answer.\n",
        )
        .expect("write uncited declaration");
        Self { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run(root: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_grund"));
    command.args(args).arg(root);
    command.output().expect("run grund")
}

fn status(output: &Output) -> i32 {
    output.status.code().expect("grund exited by status")
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr is UTF-8")
}

fn version(text: &str) -> (u64, u64, u64) {
    let mut parts = text.split('.').map(|part| {
        part.split(|ch: char| !ch.is_ascii_digit())
            .next()
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("not a version: {text}"))
    });
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn after_flip() -> bool {
    version(env!("CARGO_PKG_VERSION")) >= version("0.15.0")
}

fn expected_refs_text(message: &str, hint: bool) -> (i32, String) {
    let mut rendered = String::new();
    if after_flip() {
        rendered.push_str(message);
        rendered.push('\n');
        if hint {
            rendered.push_str(FORMAT_HINT);
        }
        (1, rendered)
    } else {
        rendered.push_str("error: ");
        rendered.push_str(message);
        rendered.push('\n');
        if hint {
            rendered.push_str(FORMAT_HINT);
        }
        rendered.push_str(WARNING);
        (2, rendered)
    }
}

fn expected_refs_json(message: &str, code: &str, hint: bool) -> (i32, String) {
    if after_flip() {
        (
            1,
            format!(
                "{{\"severity\":\"error\",\"path\":null,\"line\":null,\"code\":\"{code}\",\"message\":\"{message}\",\"sites\":null}}\n"
            ),
        )
    } else {
        expected_refs_text(message, hint)
    }
}

fn assert_run(output: &Output, expected_status: i32, expected_stderr: &str) {
    assert_eq!(stdout(output), "", "stdout must stay empty");
    assert_eq!(
        status(output),
        expected_status,
        "stderr:\n{}",
        stderr(output)
    );
    assert_eq!(stderr(output), expected_stderr);
}

#[test]
fn configured_format_rejection_obeys_both_release_phases_in_text_and_json() {
    let fixture = Fixture::new("invalid-format");
    let text = run(&fixture.root, &["refs", "FS-bar"]);
    let expected_text = expected_refs_text(INVALID, true);
    assert_run(&text, expected_text.0, &expected_text.1);

    // `--summary` is a renderer, not a different operand classification.
    let json = run(
        &fixture.root,
        &["refs", "FS-bar", "--summary", "--format", "json"],
    );
    let expected_json = expected_refs_json(INVALID, "invalid-id", true);
    assert_run(&json, expected_json.0, &expected_json.1);
}

#[test]
fn ambiguous_shorthand_obeys_both_release_phases_in_text_and_json() {
    let fixture = Fixture::new("ambiguous-shorthand");
    // `--section` is applied only after the operand resolves.
    let text = run(&fixture.root, &["refs", "FS-042", "--section", "1"]);
    let expected_text = expected_refs_text(AMBIGUOUS, false);
    assert_run(&text, expected_text.0, &expected_text.1);

    let json = run(&fixture.root, &["refs", "FS-042", "--format", "json"]);
    let expected_json = expected_refs_json(AMBIGUOUS, "ambiguous", false);
    assert_run(&json, expected_json.0, &expected_json.1);
}

#[test]
fn show_bytes_and_status_do_not_move_with_refs() {
    let fixture = Fixture::new("show-seam");
    let invalid = run(&fixture.root, &["show", "FS-bar"]);
    assert_run(&invalid, 1, &format!("{INVALID}\n{FORMAT_HINT}"));
    let ambiguous = run(&fixture.root, &["show", "FS-042"]);
    assert_run(&ambiguous, 1, &format!("{AMBIGUOUS}\n"));

    let invalid_json = run(&fixture.root, &["show", "FS-bar", "--format", "json"]);
    assert_run(
        &invalid_json,
        1,
        &format!(
            "{{\"severity\":\"error\",\"path\":null,\"line\":null,\"code\":\"invalid-id\",\"message\":\"{INVALID}\",\"sites\":null}}\n"
        ),
    );
}

#[test]
fn empty_answer_and_context_failures_keep_their_neighboring_statuses() {
    let fixture = Fixture::new("negative-seams");
    let empty = run(&fixture.root, &["refs", "FS-100-empty"]);
    assert_eq!(status(&empty), 0, "stderr:\n{}", stderr(&empty));
    assert_eq!(stdout(&empty), "");
    assert_eq!(stderr(&empty), "");

    let unknown_alias = run(&fixture.root, &["refs", "other/FS-100-empty"]);
    assert_run(
        &unknown_alias,
        2,
        "error: unknown project alias `other`\n\
         note: workspace aliases are defined in the root grund.toml under [workspace]\n",
    );

    let missing = fixture.root.join("missing-tree");
    let scan_failure = Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(["refs", "FS-100-empty"])
        .arg(&missing)
        .output()
        .expect("run scan failure");
    assert_eq!(
        status(&scan_failure),
        2,
        "stderr:\n{}",
        stderr(&scan_failure)
    );
    assert_eq!(stdout(&scan_failure), "");
    assert!(
        stderr(&scan_failure).starts_with("error: "),
        "scan failure remains a run-level error: {}",
        stderr(&scan_failure)
    );
    assert!(!stderr(&scan_failure).contains(WARNING.trim_end()));
}

#[test]
fn warning_phase_cannot_survive_the_release_it_names() {
    let warning_golden =
        include_str!("../../../tests/e2e/cases/refs-invalid-id-format/expected.stderr");
    if warning_golden.contains("currently exit 2") {
        assert!(
            version(env!("CARGO_PKG_VERSION")) < version("0.15.0"),
            "this tree reached 0.15.0; land §RM-refs-resolver-rejection-exit instead of shipping the warning-phase mapping"
        );
    }
}
