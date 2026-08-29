// The comparison-and-report half of the case runner: turning a golden mismatch
// into data, and rendering every mismatched case and surface into one verdict
// per pass. Split out of `case_runner.rs` along the same seam as
// `case_symlinks.rs` / `case_stderr.rs` — that file discovers and runs cases,
// this one decides what a run means and reports it. Included into the same
// module, so both halves still share `case_name`, `CaseOutcome`, and the
// manifest readers.

/// Render every [`CaseOutcome::Failed`] in `outcomes` (= discovery order, since
/// that is the order a pass built the vector in) as one summary naming each
/// mismatched case and, under it, each surface that differed — a case's own
/// `mismatches` are already in the fixed exit/stdout/stderr/repo order, so this
/// only orders the cases (§AR-workspace.9).
fn mismatch_summary(label: &str, outcomes: &[CaseOutcome]) -> Option<String> {
    let failed = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            CaseOutcome::Failed { case, mismatches } => Some((case, mismatches)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if failed.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    for (case, mismatches) in &failed {
        for mismatch in mismatches.iter() {
            for (index, line) in mismatch.lines().enumerate() {
                if index == 0 {
                    lines.push(format!("  {case}: {line}"));
                } else {
                    lines.push(format!("  {line}"));
                }
            }
        }
    }
    Some(format!(
        "{label}: {} case(s) mismatched their goldens:\n{}",
        failed.len(),
        lines.join("\n")
    ))
}

/// Account for every case a pass did not run or did not pass (§FS-errors.2.2 in
/// spirit: a run says what it did not do). [`run_case`] and
/// [`assert_case_is_deterministic`] collect a mismatch as data instead of
/// panicking at the first one, so this is where the pass actually decides: if
/// any case mismatched, it panics once, naming every mismatched case and surface
/// and, next to them, any skip — so a skip is never lost among the mismatches
/// (§AR-workspace.9). With no mismatch, a skip is printed with its reason and
/// counted — never folded into the pass total — and on a platform that can
/// always create a directory symlink it is a **failure**, because there a
/// skipped case means the harness lost the coverage rather than the platform
/// refusing it. That is the shape the old bare `return` hid: an unrelated
/// `TMPDIR` property deleted the member-containment coverage on Linux and macOS
/// and still printed `4 passed`.
pub fn assert_every_case_passed(label: &str, outcomes: &[CaseOutcome]) {
    let skipped = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            CaseOutcome::Skipped { case, why } => Some(format!("  {case}: {why}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    let skip_summary = if skipped.is_empty() {
        None
    } else {
        Some(format!(
            "{label}: {} case(s) skipped, not passed:\n{}",
            skipped.len(),
            skipped.join("\n")
        ))
    };

    if let Some(mismatch_summary) = mismatch_summary(label, outcomes) {
        panic!(
            "{mismatch_summary}{}",
            skip_summary.map_or(String::new(), |skip| format!("\n{skip}"))
        );
    }

    let Some(skip_summary) = skip_summary else {
        return;
    };
    assert!(
        !cfg!(unix),
        "{skip_summary}\nthis platform can create a directory symlink, so a skipped case is lost \
         coverage rather than an unsupported feature"
    );
    eprintln!("{skip_summary}");
}

/// The `repo` surface: whether `command.args` runs against `{repo_copy}` is a
/// fixture-authoring precondition and stays an `assert!` (like the `symlinks`
/// manifest's own `{repo_copy}` requirement in [`run_case`]); the file list and
/// the per-file bytes are golden comparisons, so a difference is pushed onto
/// `mismatches` — as one `final repo mismatch` entry — instead of panicking.
fn assert_expected_repo(case: &Path, manifest_dir: &Path, name: &str, mismatches: &mut Vec<String>) {
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
    let mut repo_lines = Vec::new();
    if actual_files != expected_files {
        let list_diff = expected_files
            .difference(&actual_files)
            .map(|rel| format!("  - {} (expected, not actual)", rel.display()))
            .chain(
                actual_files
                    .difference(&expected_files)
                    .map(|rel| format!("  - {} (actual, not expected)", rel.display())),
            )
            .collect::<Vec<_>>()
            .join("\n");
        repo_lines.push(format!("file list differs:\n{list_diff}"));
    }
    for rel in expected_files.intersection(&actual_files) {
        let expected_path = expected.join(rel);
        let actual_path = actual.join(rel);
        let expected_bytes = fs::read(&expected_path)
            .unwrap_or_else(|err| panic!("{name}: read {}: {err}", expected_path.display()));
        let actual_bytes = fs::read(&actual_path)
            .unwrap_or_else(|err| panic!("{name}: read {}: {err}", actual_path.display()));
        if actual_bytes != expected_bytes {
            repo_lines.push(format!(
                "{}:\n{}",
                rel.display(),
                text_mismatch(
                    "bytes differ",
                    "expected",
                    &String::from_utf8_lossy(&expected_bytes),
                    "actual",
                    &String::from_utf8_lossy(&actual_bytes),
                )
            ));
        }
    }
    if !repo_lines.is_empty() {
        mismatches.push(format!(
            "final repo mismatch\n{}",
            indent(&repo_lines.join("\n"), 2)
        ));
    }
}

/// Indent every line of `text` by `spaces`, so a multi-line golden payload
/// nests under the header line above it instead of being dumped as one escaped
/// line.
fn indent(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn exit_mismatch(expected: i32, actual: i32) -> String {
    format!("exit code mismatch: expected {expected}, actual {actual}")
}

/// One surface's report: `headline`, then `label_a`/`label_b` and their raw
/// (not `Debug`-escaped) text, each indented under its own header — the shape
/// [`mismatch_summary`] renders for every surface but the exit code.
fn text_mismatch(headline: &str, label_a: &str, a: &str, label_b: &str, b: &str) -> String {
    format!(
        "{headline}\n  {label_a}:\n{}\n  {label_b}:\n{}",
        indent(a, 4),
        indent(b, 4)
    )
}

fn exit_code_text(code: Option<i32>) -> String {
    code.map_or_else(|| "<terminated by signal>".to_string(), |code| code.to_string())
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

