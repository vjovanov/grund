// The golden-form half of the case runner: the one on-disk spelling a golden may
// have, and the property the writer owes the reader. Split out of
// `case_runner.rs` along the same seam as `case_report.rs` — that file writes
// goldens and reads them back, this one says which bytes are allowed to be
// there. Included into the same module, so the property tests below drive
// `write_expected` and `read_expected_output` themselves rather than a copy of
// their rules.

/// The goldens one case carries, in the order a run compares them.
const GOLDEN_SURFACES: [&str; 3] = ["expected.exit", "expected.stdout", "expected.stderr"];

/// Every golden under `cases` whose bytes are not the canonical form
/// (§AR-workspace.9.1), named relative to `manifest_dir` with what is wrong with
/// it. Two things this does on purpose: it reads bytes rather than
/// [`read_expected_output`], because the reader normalizes away the very
/// difference under test; and it judges every golden of every case before
/// returning, so the caller names them all in one failure instead of aborting at
/// the first (§AR-workspace.9).
pub fn golden_form_violations(manifest_dir: &Path, cases: &[PathBuf]) -> Vec<String> {
    let mut violations = Vec::new();
    for case in cases {
        for surface in GOLDEN_SURFACES {
            let path = case.join(surface);
            // Whether a golden is *there* is the runner's business — it fails the
            // case it cannot read. This pass judges the bytes of the ones that are.
            if !path.is_file() {
                continue;
            }
            let bytes =
                fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
            if let Some(why) = golden_defect(surface, &bytes) {
                // `/`-separated whatever the platform, so the name reads as the
                // repository path a contributor has to go and fix.
                let named = path.strip_prefix(manifest_dir).unwrap_or(&path);
                let named = named.to_string_lossy().replace('\\', "/");
                violations.push(format!("{named}: {why}"));
            }
        }
    }
    violations
}

/// What is wrong with one golden's bytes, or `None` when they are canonical.
fn golden_defect(surface: &str, bytes: &[u8]) -> Option<String> {
    if surface == "expected.exit" {
        return exit_golden_defect(bytes);
    }
    if bytes.is_empty() {
        return Some(
            "zero bytes; empty output is written as a single newline, so a refresh rewrites this \
             file"
                .to_string(),
        );
    }
    let returns = bytes.iter().filter(|byte| **byte == b'\r').count();
    if returns > 0 {
        return Some(format!(
            "{returns} carriage return(s); an output golden is LF-only, so a refresh rewrites \
             this file"
        ));
    }
    None
}

/// An exit golden is the decimal code and exactly one newline. `0` unterminated
/// and `0\n` parse to the same code, which is why the reader never noticed the
/// difference and a refresh rewrote the file anyway.
fn exit_golden_defect(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes).into_owned();
    match text.trim().parse::<i32>() {
        Ok(code) if text == format!("{code}\n") => None,
        Ok(code) => Some(format!(
            "{text:?}, not the decimal exit code and exactly one newline ({:?}), so a refresh \
             rewrites this file",
            format!("{code}\n")
        )),
        Err(err) => Some(format!("{text:?}, which is not an exit code at all: {err}")),
    }
}

/// The property the writer owes the reader (§AR-workspace.9.1): the bytes a
/// golden is written with are the bytes a refresh writes again, so writing one
/// twice — with the reader's own view of it in between — changes nothing. Driven
/// over representative outputs rather than over the corpus, because the corpus
/// only holds what the writer happens to have produced.
#[cfg(test)]
mod golden_form_tests {
    use super::{golden_form_violations, read_expected_output, read_to_string, write_expected};
    use std::fs;
    use std::path::{Path, PathBuf};

    /// A run's output, the bytes its golden must hold, and the value the reader
    /// hands the comparison. The third row is the recorded limitation of
    /// §AR-workspace.9.1 — a whole-file newline means "no output", so a case whose
    /// real output is one newline reads back as empty and cannot be pinned. The
    /// fourth is the one the writer gets wrong today: it emits the run's line
    /// endings while the reader folds them.
    const OUTPUTS: [(&str, &str, &str); 4] = [
        ("", "\n", ""),
        ("x\n", "x\n", "x\n"),
        ("\n", "\n", ""),
        ("a\r\nb\n", "a\nb\n", "a\nb\n"),
    ];

    const EXIT_CODES: [i32; 2] = [0, 1];

    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target/e2e-harness-tests")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("create {}: {err}", dir.display()));
        dir
    }

    fn read_bytes(path: &Path) -> Vec<u8> {
        fs::read(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
    }

    /// The form check over one case directory, with the case as its own root so a
    /// violation is named by the golden's file name alone.
    fn violations(case: &Path) -> Vec<String> {
        golden_form_violations(case, &[case.to_path_buf()])
    }

    #[test]
    fn what_the_writer_writes_is_the_canonical_form() {
        let case = scratch("what_the_writer_writes_is_the_canonical_form/outputs");
        let path = case.join("expected.stdout");
        for (content, canonical, _) in OUTPUTS {
            write_expected(&path, content);
            assert_eq!(
                canonical,
                String::from_utf8_lossy(&read_bytes(&path)),
                "writing the output {content:?} must put {canonical:?} on disk"
            );
            assert_eq!(
                Vec::<String>::new(),
                violations(&case),
                "the golden written from {content:?} must already be in canonical form"
            );
        }
        let case = scratch("what_the_writer_writes_is_the_canonical_form/exits");
        let path = case.join("expected.exit");
        for code in EXIT_CODES {
            write_expected(&path, &format!("{code}\n"));
            assert_eq!(
                format!("{code}\n"),
                String::from_utf8_lossy(&read_bytes(&path)),
                "an exit golden is the code and exactly one newline"
            );
            assert_eq!(
                Vec::<String>::new(),
                violations(&case),
                "the exit golden written for {code} must already be in canonical form"
            );
        }
    }

    #[test]
    fn rewriting_a_golden_after_reading_it_changes_no_bytes() {
        let case = scratch("rewriting_a_golden_after_reading_it_changes_no_bytes");
        let path = case.join("expected.stdout");
        for (content, _, _) in OUTPUTS {
            write_expected(&path, content);
            let first = read_bytes(&path);
            let seen = read_expected_output(&path);
            write_expected(&path, &seen);
            assert_eq!(
                String::from_utf8_lossy(&first),
                String::from_utf8_lossy(&read_bytes(&path)),
                "a golden written from {content:?} must survive a refresh byte-identical"
            );
        }
        let path = case.join("expected.exit");
        for code in EXIT_CODES {
            write_expected(&path, &format!("{code}\n"));
            let first = read_bytes(&path);
            let seen = read_to_string(&path)
                .trim()
                .parse::<i32>()
                .unwrap_or_else(|err| panic!("parse the exit golden for {code}: {err}"));
            write_expected(&path, &format!("{seen}\n"));
            assert_eq!(
                String::from_utf8_lossy(&first),
                String::from_utf8_lossy(&read_bytes(&path)),
                "an exit golden for {code} must survive a refresh byte-identical"
            );
        }
    }

    #[test]
    fn a_written_golden_reads_back_as_the_value_a_run_compares() {
        let case = scratch("a_written_golden_reads_back_as_the_value_a_run_compares");
        let path = case.join("expected.stdout");
        for (content, _, seen) in OUTPUTS {
            write_expected(&path, content);
            assert_eq!(
                seen,
                read_expected_output(&path),
                "the golden written from {content:?} must read back as {seen:?}"
            );
        }
        let path = case.join("expected.exit");
        for code in EXIT_CODES {
            write_expected(&path, &format!("{code}\n"));
            assert_eq!(
                Ok(code),
                read_to_string(&path).trim().parse::<i32>(),
                "the exit golden written for {code} must read back as {code}"
            );
        }
    }

    #[test]
    fn the_form_check_names_every_offending_golden() {
        let root = scratch("the_form_check_names_every_offending_golden");
        let cases = ["case-a", "case-b"].map(|name| root.join(name));
        for case in &cases {
            fs::create_dir_all(case).unwrap_or_else(|err| panic!("create the case: {err}"));
        }
        fs::write(cases[0].join("expected.exit"), "0").expect("write an unterminated exit golden");
        fs::write(cases[0].join("expected.stdout"), "").expect("write a zero-byte golden");
        fs::write(cases[0].join("expected.stderr"), "ok\n").expect("write a canonical golden");
        fs::write(cases[1].join("expected.exit"), "1\n").expect("write a canonical exit golden");
        fs::write(cases[1].join("expected.stdout"), "a\r\nb\n").expect("write a CRLF golden");
        // `case-b` carries no `expected.stderr`: a golden that is not there is the
        // runner's verdict to give, not a defect in the form of one that is.

        let violations = golden_form_violations(&root, &cases);
        assert_eq!(3, violations.len(), "{violations:#?}");
        assert!(violations[0].starts_with("case-a/expected.exit: "), "{violations:#?}");
        assert!(violations[1].starts_with("case-a/expected.stdout: "), "{violations:#?}");
        assert!(violations[2].starts_with("case-b/expected.stdout: "), "{violations:#?}");
        assert!(
            violations.iter().all(|violation| violation.contains("refresh")),
            "each violation says what it costs:\n{violations:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
