//! Black-box contract for one-context batch declaration reads
//! (§FS-show.1, §FS-show.2.6, §FS-output-shapes.4.1, §AR-workspace.8).

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new(name: &str) -> Self {
        let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/show-batch-contract")
            .join(format!("{name}-{}-{serial}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).expect("create batch fixture");
        fs::create_dir_all(root.join("member/docs")).expect("create batch member fixture");

        write(
            &root.join("grund.toml"),
            "grund_config_version = 1\nproject_name = \"root\"\n\n\
             [reference]\nstrict = false\n\n\
             [id]\nformat = \"{kind}-{slug}\"\nnamed_sections = true\n\n\
             [scan]\ninclude = [\"docs\"]\n\n\
             [workspace]\nmembers = [\"member\"]\n",
        );
        write(
            &root.join("member/grund.toml"),
            "grund_config_version = 1\nproject_name = \"member\"\n\n\
             [reference]\nstrict = false\n\n\
             [id]\nformat = \"{kind}-{slug}\"\nnamed_sections = true\n\n\
             [scan]\ninclude = [\"docs\"]\n",
        );
        write(
            &root.join("docs/FS-alpha.md"),
            "# FS-alpha: Alpha\n\nAlpha lead.\n\nAlpha second paragraph.\n\n\
             ## 1. Numeric\n\nNumeric lead.\n\n\
             ### 1.1 Child\n\nChild body.\n\n\
             ## goals: Named\n\nNamed body.\n",
        );
        write(
            &root.join("member/docs/FS-beta.md"),
            "# FS-beta: Beta\n\nBeta lead.\n\n## 2. Member section\n\nMember body.\n",
        );
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn load_log(&self, name: &str) -> PathBuf {
        self.0.join(format!("{name}.loads"))
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, body: &str) {
    fs::write(path, body).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

fn run_batch(repo: &Repo, extra_args: &[&str], stdin: &str, load_log: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_grund"));
    command
        .args(["show", "--batch", "--format=json"])
        .args(extra_args)
        .current_dir(repo.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = load_log {
        command.env("GRUND_TEST_WORKSPACE_LOAD_LOG", path);
    }
    let mut child = command.spawn().expect("run grund show --batch");
    let write_result = child
        .stdin
        .take()
        .expect("batch stdin")
        .write_all(stdin.as_bytes());
    if let Err(error) = write_result
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        panic!("write batch stdin: {error}");
    }
    child.wait_with_output().expect("read batch output")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn records(output: &Output) -> Vec<Value> {
    stdout(output)
        .lines()
        .map(|line| serde_json::from_str(line).expect("batch stdout record is JSON"))
        .collect()
}

fn loads(path: &Path) -> usize {
    fs::read_to_string(path).unwrap_or_default().lines().count()
}

#[test]
fn show_batch_all_four_modes_use_single_show_rendering() {
    let repo = Repo::new("modes");
    let input = "{\"id\":\"FS-alpha\",\"section\":null}\n";
    let cases = [
        (None, "Alpha lead.\n\nAlpha second paragraph.\n", false),
        (Some("--brief"), "# FS-alpha: Alpha\n\nAlpha lead.\n", false),
        (
            Some("--toc"),
            "Alpha lead.\n\nAlpha second paragraph.\n\n## 1. Numeric\n### 1.1 Child\n## goals: Named\n",
            true,
        ),
        (
            Some("--full"),
            "Alpha lead.\n\nAlpha second paragraph.\n\n## 1. Numeric\n\nNumeric lead.\n\n### 1.1 Child\n\nChild body.\n\n## goals: Named\n\nNamed body.\n",
            false,
        ),
    ];

    for (flag, body, has_sections) in cases {
        let args = flag.into_iter().collect::<Vec<_>>();
        let output = run_batch(&repo, &args, input, None);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{flag:?} stderr:\n{}",
            stderr(&output)
        );
        let records = records(&output);
        assert_eq!(records.len(), 1, "{flag:?}");
        assert_eq!(records[0]["result"]["body"], body, "{flag:?}");
        assert_eq!(
            records[0]["result"].get("sections").is_some(),
            has_sections,
            "{flag:?}"
        );
    }
}

#[test]
fn show_batch_rejects_the_whole_malformed_stream_before_scanning() {
    let repo = Repo::new("malformed");
    let log = repo.load_log("malformed");
    let input = concat!(
        "{\"id\":\"FS-alpha\"}\n",
        "{\"id\":\"FS-alpha\",\"extra\":true}\n",
    );
    let output = run_batch(&repo, &[], input, Some(&log));

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "error: batch input line 2: unknown field `extra`\n"
    );
    assert_eq!(loads(&log), 0, "malformed input must be rejected pre-scan");
}

#[test]
fn show_batch_empty_input_is_a_successful_no_scan_noop() {
    let repo = Repo::new("empty");
    let log = repo.load_log("empty");
    let output = run_batch(&repo, &[], "", Some(&log));

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
    assert_eq!(loads(&log), 0);
}

#[test]
fn show_batch_loads_one_workspace_for_many_queries_and_for_all() {
    let repo = Repo::new("load-count");
    let explicit_log = repo.load_log("explicit");
    let all_log = repo.load_log("all");
    let explicit = run_batch(
        &repo,
        &[],
        concat!(
            "{\"id\":\"FS-alpha\"}\n",
            "{\"id\":\"FS-alpha\",\"section\":\"1\"}\n",
            "{\"id\":\"member/FS-beta\"}\n",
        ),
        Some(&explicit_log),
    );
    let all = run_batch(&repo, &["--all"], "", Some(&all_log));

    assert_eq!(
        (loads(&explicit_log), loads(&all_log)),
        (1, 1),
        "expected one workspace-loader entry for explicit and exhaustive batches; \
         statuses were {:?} and {:?}; stderr was {:?} and {:?}",
        explicit.status.code(),
        all.status.code(),
        stderr(&explicit),
        stderr(&all)
    );
    assert_eq!(explicit.status.code(), Some(0));
    assert_eq!(all.status.code(), Some(0));
}
