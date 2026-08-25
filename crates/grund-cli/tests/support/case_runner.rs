// §RM-e2e-corpus: golden CLI cases verify byte-for-byte command behavior.
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

// The citable half of the canonical kind set (§FS-config.3.4). The two default
// test kinds are non-citable, so no `spec.refs` entry can name one.
const CANONICAL_KINDS: &[&str] = &["GRUND", "GOAL", "FS", "AR", "DF", "DA", "RM"];

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

/// What one pass over one case actually did. A skipped case is not a passing case
/// and libtest has no third verdict, so the outcome is returned rather than
/// swallowed: [`assert_every_case_ran`] counts and names the skips at the end of
/// the pass, and fails where the platform had no excuse for one.
pub enum CaseOutcome {
    /// The case ran and its goldens were compared.
    Ran,
    /// This pass does not apply to the case — a mutating case has no
    /// deterministic-rerun contract, because the second run would see the tree the
    /// first one wrote.
    NotApplicable,
    /// The platform could not build the fixture, so nothing was compared.
    Skipped { case: String, why: &'static str },
}

// The `symlinks` manifest half, in a file of its own (§AR-core-module-layout.3).
include!("case_symlinks.rs");

/// Account for every case a pass did not run (§FS-errors.2.2 in spirit: a run says
/// what it did not do). A skip is printed with its reason and counted — never
/// folded into the pass total — and on a platform that can always create a
/// directory symlink it is a **failure**, because there a skipped case means the
/// harness lost the coverage rather than the platform refusing it. That is the
/// shape the old bare `return` hid: an unrelated `TMPDIR` property deleted the
/// member-containment coverage on Linux and macOS and still printed `4 passed`.
pub fn assert_every_case_ran(label: &str, outcomes: &[CaseOutcome]) {
    let skipped = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            CaseOutcome::Skipped { case, why } => Some(format!("  {case}: {why}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    if skipped.is_empty() {
        return;
    }
    let summary = format!(
        "{label}: {} case(s) skipped, not passed:\n{}",
        skipped.len(),
        skipped.join("\n")
    );
    assert!(
        !cfg!(unix),
        "{summary}\nthis platform can create a directory symlink, so a skipped case is lost \
         coverage rather than an unsupported feature"
    );
    eprintln!("{summary}");
}

pub fn discover_e2e_cases(manifest_dir: &Path) -> Vec<PathBuf> {
    let cases_dir = manifest_dir.join("tests/e2e/cases");
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

pub fn assert_case_is_deterministic(manifest_dir: &Path, case: &Path) -> CaseOutcome {
    let name = case_name(case);
    // Two independent questions, one `if` each. Written as one expression they read
    // as `mutating || (has_links && !supported)`, and since a manifest case is
    // always a `{repo_copy}` case (`run_case` requires it) the first clause always
    // won: the platform skip added for those cases decided nothing at all.
    if !case_symlinks(case).is_empty() && !symlinks_supported(manifest_dir) {
        return CaseOutcome::Skipped {
            case: name.to_string(),
            why: SYMLINK_SKIP,
        };
    }
    if is_mutating_case(case) {
        return CaseOutcome::NotApplicable;
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
    CaseOutcome::Ran
}

pub fn run_case(manifest_dir: &Path, case: &Path, kind: CaseKind) -> CaseOutcome {
    let name = case_name(case);
    if kind.requires_spec_refs() {
        assert_spec_refs(case, name);
    }
    // A case whose fixture needs a symlink cannot run where the platform cannot
    // make one: a committed symlink is checked out as a text file on Windows
    // without developer mode, so the fixture is built at run time and the case
    // would otherwise compare a different tree's output (§FS-workspace.6.1). The
    // skip is returned, so the pass counts and names it instead of exiting 0.
    if !case_symlinks(case).is_empty() {
        // Links are created in the copy, and only the `{repo_copy}` branch of
        // `command_args` copies the fixture — so a manifest case written against
        // `{repo}` created no links at all and was green anyway, testing the
        // committed tree while claiming to test a symlinked one.
        assert!(
            case_command(case).contains("{repo_copy}"),
            "{name}: a case with a `symlinks` manifest must run against {{repo_copy}} — \
             its links are created in the copy"
        );
        if !symlinks_supported(manifest_dir) {
            return CaseOutcome::Skipped {
                case: name.to_string(),
                why: SYMLINK_SKIP,
            };
        }
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
        return CaseOutcome::Ran;
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
    CaseOutcome::Ran
}

fn case_name(case: &Path) -> &str {
    case.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<invalid case name>")
}

/// The command a case runs, as written: its `command.args`, or the default every
/// case without one runs. One reader, so "is this a mutating case?", "does it use
/// the copy?", and the argument expansion below cannot disagree about what the
/// case does.
fn case_command(case: &Path) -> String {
    let command_file = case.join("command.args");
    if command_file.exists() {
        read_to_string(command_file)
    } else {
        DEFAULT_COMMAND.to_string()
    }
}

const DEFAULT_COMMAND: &str = "check {repo}";

fn is_mutating_case(case: &Path) -> bool {
    let command = case_command(case);
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
    let command = case_command(case);
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
    for reference in &refs {
        assert!(
            has_canonical_kind_prefix(reference),
            "{name}: spec.refs entry {reference} does not start with a canonical kind ({})",
            CANONICAL_KINDS.join(", ")
        );
    }
    // `[citations.e2e] must = ["FS"]` in grund.toml, held here because `spec.refs`
    // is a manifest the scanner never reads: a case that proves nothing in the
    // spec is not an e2e case (§FS-config.3.4.4).
    assert!(
        refs.iter().any(|reference| reference.starts_with("FS-")),
        "{name}: spec.refs in {} names no FS point; an e2e case cites the spec it proves",
        refs_path.display()
    );
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
