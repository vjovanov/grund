// §RM-e2e-corpus: golden CLI cases verify byte-for-byte command behavior.
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CANONICAL_KINDS: &[&str] = &["GRUND", "GOAL", "FS", "AR", "DF", "DA", "E2E", "RM"];

#[derive(Clone, Copy)]
pub enum CaseKind {
    E2e,
    Example,
}

impl CaseKind {
    fn requires_spec_refs(self) -> bool {
        matches!(self, CaseKind::E2e)
    }
}

pub fn discover_e2e_cases(manifest_dir: &Path) -> Vec<PathBuf> {
    let cases_dir = manifest_dir.join("e2e/cases");
    let cases = discover_case_dirs(&cases_dir, |_| true);
    assert!(
        !cases.is_empty(),
        "expected at least one e2e case under {}",
        cases_dir.display()
    );
    cases
}

pub fn discover_examples(manifest_dir: &Path) -> Vec<PathBuf> {
    let examples_dir = manifest_dir.join("examples");
    let cases = discover_case_dirs(&examples_dir, |path| path.join("expected.exit").is_file());
    assert!(
        !cases.is_empty(),
        "expected at least one runnable example under {}",
        examples_dir.display()
    );
    cases
}

fn discover_case_dirs(root: &Path, include: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut cases = fs::read_dir(root)
        .unwrap_or_else(|err| panic!("read {}: {err}", root.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir() && include(path))
        .collect::<Vec<_>>();
    cases.sort();
    cases
}

pub fn assert_case_is_deterministic(manifest_dir: &Path, case: &Path) {
    let name = case_name(case);
    if is_mutating_case(case) || !case_symlinks(case).is_empty() && !symlinks_supported() {
        return;
    }
    let args = command_args(manifest_dir, case, name);
    let first = run_grund(manifest_dir, &args, name);
    let second = run_grund(manifest_dir, &args, name);
    assert_eq!(
        first.status.code(),
        second.status.code(),
        "{name}: exit code differs between runs"
    );
    assert_eq!(
        first.stdout, second.stdout,
        "{name}: stdout differs between runs"
    );
    assert_eq!(
        first.stderr, second.stderr,
        "{name}: stderr differs between runs"
    );
}

pub fn run_case(manifest_dir: &Path, case: &Path, kind: CaseKind) {
    let name = case_name(case);
    if kind.requires_spec_refs() {
        assert_spec_refs(case, name);
    }
    // A case whose fixture needs a symlink is skipped where the platform cannot
    // make one: a committed symlink is checked out as a text file on Windows
    // without developer mode, so the fixture is built at run time and the case
    // reports nothing rather than a different tree's output (§FS-workspace.6.1).
    if !case_symlinks(case).is_empty() && !symlinks_supported() {
        return;
    }

    let args = command_args(manifest_dir, case, name);
    let output = run_grund(manifest_dir, &args, name);
    let actual_exit = output.status.code().unwrap_or(-1);
    let actual_stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|err| panic!("{name}: stdout was not UTF-8: {err}"));
    let actual_stderr = String::from_utf8(output.stderr)
        .unwrap_or_else(|err| panic!("{name}: stderr was not UTF-8: {err}"));

    if std::env::var_os("UPDATE_EXPECTED").is_some() {
        write_expected(&case.join("expected.exit"), &format!("{actual_exit}\n"));
        write_expected(&case.join("expected.stdout"), &actual_stdout);
        write_expected(&case.join("expected.stderr"), &actual_stderr);
        return;
    }

    let expected_exit = read_to_string(case.join("expected.exit"));
    let expected_exit = expected_exit
        .trim()
        .parse::<i32>()
        .unwrap_or_else(|err| panic!("{name}: parse expected.exit: {err}"));
    assert_eq!(
        actual_exit, expected_exit,
        "{name}: exit code mismatch\nstdout:\n{actual_stdout}\nstderr:\n{actual_stderr}"
    );

    let expected_stdout = read_expected_output(case.join("expected.stdout"));
    let expected_stderr = read_expected_output(case.join("expected.stderr"));
    assert_expected_errors_are_concise(case, name, &expected_stderr);
    assert_eq!(actual_stdout, expected_stdout, "{name}: stdout mismatch");
    assert_eq!(actual_stderr, expected_stderr, "{name}: stderr mismatch");
    assert_expected_repo(case, manifest_dir, name);
}

fn case_name(case: &Path) -> &str {
    case.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<invalid case name>")
}

fn is_mutating_case(case: &Path) -> bool {
    let command_file = case.join("command.args");
    if !command_file.exists() {
        return false;
    }
    let command = read_to_string(command_file);
    command.contains("--write") || command.contains("{repo_copy}")
}

fn run_grund(manifest_dir: &Path, args: &[String], name: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .unwrap_or_else(|err| panic!("{name}: run grund: {err}"))
}

fn command_args(manifest_dir: &Path, case: &Path, name: &str) -> Vec<String> {
    let repo = case.join("repo");
    let repo_arg = repo
        .strip_prefix(manifest_dir)
        .unwrap_or(&repo)
        .to_string_lossy()
        .into_owned();
    let repo_copy = manifest_dir.join("target/e2e-work").join(name).join("repo");
    let repo_copy_arg = repo_copy
        .strip_prefix(manifest_dir)
        .unwrap_or(&repo_copy)
        .to_string_lossy()
        .into_owned();
    let command_file = case.join("command.args");
    if !command_file.exists() {
        return vec!["check".to_string(), repo_arg];
    }

    let command = read_to_string(command_file);
    if command.contains("{repo_copy}") {
        if let Some(parent) = repo_copy.parent() {
            let _ = fs::remove_dir_all(parent);
            fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("{name}: create {}: {err}", parent.display()));
        }
        copy_dir(&repo, &repo_copy);
        create_case_symlinks(case, &repo_copy, name);
    }

    command
        .split_whitespace()
        .map(|arg| {
            if let Some(suffix) = arg.strip_prefix("{repo}/") {
                PathBuf::from(&repo_arg)
                    .join(suffix)
                    .to_string_lossy()
                    .into_owned()
            } else if let Some(suffix) = arg.strip_prefix("{repo_copy}/") {
                PathBuf::from(&repo_copy_arg)
                    .join(suffix)
                    .to_string_lossy()
                    .into_owned()
            } else if arg == "{repo}" {
                repo_arg.clone()
            } else if arg == "{repo_copy}" {
                repo_copy_arg.clone()
            } else {
                arg.to_string()
            }
        })
        .collect()
}

/// The `symlinks` manifest of a case: one `<link> -> <target>` per line, both
/// relative to the fixture repo. Symlinks are built into the *copy* at run time
/// rather than committed, because git on Windows checks a committed symlink out as
/// a text file holding its target unless developer mode is on — the fixture would
/// then be a different tree, and the golden would fail for a reason the case is not
/// about. §FS-workspace.6.1's containment rule can only be reached through one, so
/// the corpus needs the affordance.
fn case_symlinks(case: &Path) -> Vec<(String, String)> {
    let manifest = case.join("symlinks");
    if !manifest.is_file() {
        return Vec::new();
    }
    read_to_string(manifest)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (link, target) = line.split_once("->").unwrap_or_else(|| {
                panic!("symlinks manifest line is not `<link> -> <target>`: {line}")
            });
            (link.trim().to_string(), target.trim().to_string())
        })
        .collect()
}

/// Whether this platform lets the test process create a directory symlink at all.
/// On Windows that needs developer mode or elevation, so the answer is a probe
/// rather than a `cfg`.
fn symlinks_supported() -> bool {
    let probe =
        std::env::temp_dir().join(format!("grund-e2e-symlink-probe-{}", std::process::id()));
    let _ = fs::remove_file(&probe);
    let made = symlink_dir(Path::new("."), &probe).is_ok();
    let _ = fs::remove_file(&probe);
    made
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

fn create_case_symlinks(case: &Path, repo_copy: &Path, name: &str) {
    for (link, target) in case_symlinks(case) {
        let link_path = repo_copy.join(&link);
        symlink_dir(Path::new(&target), &link_path).unwrap_or_else(|err| {
            panic!("{name}: link {link} -> {target}: {err}");
        });
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap_or_else(|err| panic!("create {}: {err}", to.display()));
    for entry in fs::read_dir(from).unwrap_or_else(|err| panic!("read {}: {err}", from.display())) {
        let entry = entry.unwrap();
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &target);
        } else {
            fs::copy(&source, &target).unwrap_or_else(|err| {
                panic!("copy {} to {}: {err}", source.display(), target.display())
            });
        }
    }
}

fn assert_expected_errors_are_concise(case: &Path, name: &str, stderr: &str) {
    if read_to_string(case.join("expected.exit")).trim() == "0" {
        return;
    }
    assert!(
        !stderr.contains("error(s)") && !stderr.contains("warning(s)"),
        "{name}: stderr should not include aggregate summaries"
    );
    for line in stderr.lines().filter(|line| !line.trim().is_empty()) {
        assert!(
            line.len() <= 180,
            "{name}: stderr line is too long for a concise diagnostic: {line}"
        );
    }
}

fn assert_expected_repo(case: &Path, manifest_dir: &Path, name: &str) {
    let expected = case.join("expected.repo");
    if !expected.exists() {
        return;
    }
    let actual = manifest_dir.join("target/e2e-work").join(name).join("repo");
    assert!(
        actual.exists(),
        "{name}: expected.repo requires command.args to run against {{repo_copy}}"
    );
    let expected_files = relative_files(&expected);
    let actual_files = relative_files(&actual);
    assert_eq!(
        actual_files, expected_files,
        "{name}: final repo file list differs from expected.repo"
    );
    for rel in expected_files {
        let expected_path = expected.join(&rel);
        let actual_path = actual.join(&rel);
        let expected_bytes = fs::read(&expected_path)
            .unwrap_or_else(|err| panic!("{name}: read {}: {err}", expected_path.display()));
        let actual_bytes = fs::read(&actual_path)
            .unwrap_or_else(|err| panic!("{name}: read {}: {err}", actual_path.display()));
        assert_eq!(
            actual_bytes,
            expected_bytes,
            "{name}: final bytes differ for {}",
            rel.display()
        );
    }
}

fn relative_files(root: &Path) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    collect_relative_files(root, root, &mut files);
    files
}

fn collect_relative_files(root: &Path, dir: &Path, files: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display())) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_relative_files(root, &path, files);
        } else {
            files.insert(path.strip_prefix(root).unwrap().to_path_buf());
        }
    }
}

fn assert_spec_refs(case: &Path, name: &str) {
    let refs_path = case.join("spec.refs");
    let refs = read_to_string(&refs_path);
    let refs = refs
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    assert!(
        !refs.is_empty(),
        "{name}: expected at least one spec reference in {}",
        refs_path.display()
    );
    for reference in refs {
        assert!(
            has_canonical_kind_prefix(reference),
            "{name}: spec.refs entry {reference} does not start with a canonical kind ({})",
            CANONICAL_KINDS.join(", ")
        );
    }
}

fn has_canonical_kind_prefix(reference: &str) -> bool {
    CANONICAL_KINDS.iter().any(|k| {
        reference
            .strip_prefix(k)
            .is_some_and(|rest| rest.starts_with('-'))
    })
}

fn write_expected(path: &Path, content: &str) {
    let is_exit = path.extension().and_then(|s| s.to_str()) == Some("exit");
    let body = if content.is_empty() && !is_exit {
        "\n".to_string()
    } else {
        content.to_string()
    };
    fs::write(path, body).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
}

fn read_to_string(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn read_expected_output(path: impl AsRef<Path>) -> String {
    let output = read_to_string(path).replace("\r\n", "\n");
    if output == "\n" {
        String::new()
    } else {
        output
    }
}
