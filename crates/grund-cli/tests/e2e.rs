// §AR-bindings.3: the e2e tests exercise the dedicated CLI frontend crate.
use std::path::PathBuf;

#[path = "support/case_runner.rs"]
mod case_runner;

use case_runner::CaseKind::{E2e, Example};
use case_runner::{
    assert_case_is_deterministic, assert_every_case_passed, discover_e2e_cases, discover_examples,
    golden_form_violations, run_case,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// Every pass collects its per-case outcomes and hands them to
// `assert_every_case_passed`: a case that mismatched its goldens or that the
// platform could not build is counted and named there, never left to look
// like one of the passes libtest reports.

#[test]
fn e2e_cases_match_expected_reports() {
    let manifest_dir = repo_root();
    let outcomes = discover_e2e_cases(&manifest_dir)
        .iter()
        .map(|case| run_case(&manifest_dir, case, E2e))
        .collect::<Vec<_>>();
    assert_every_case_passed("e2e cases", &outcomes);
}

#[test]
fn e2e_output_is_deterministic() {
    let manifest_dir = repo_root();
    let outcomes = discover_e2e_cases(&manifest_dir)
        .iter()
        .map(|case| assert_case_is_deterministic(&manifest_dir, case))
        .collect::<Vec<_>>();
    assert_every_case_passed("e2e determinism", &outcomes);
}

#[test]
fn examples_are_e2e_cases() {
    let manifest_dir = repo_root();
    let outcomes = discover_examples(&manifest_dir)
        .iter()
        .map(|case| run_case(&manifest_dir, case, Example))
        .collect::<Vec<_>>();
    assert_every_case_passed("examples", &outcomes);
}

#[test]
fn example_output_is_deterministic() {
    let manifest_dir = repo_root();
    let outcomes = discover_examples(&manifest_dir)
        .iter()
        .map(|case| assert_case_is_deterministic(&manifest_dir, case))
        .collect::<Vec<_>>();
    assert_every_case_passed("example determinism", &outcomes);
}

/// The goldens are themselves a contract, not just a comparison: every case's
/// are in the one on-disk form the harness writes, so refreshing the case a
/// change is about rewrites no other case's bytes (§AR-workspace.9.1). Judged as
/// bytes and reported all at once — the tree should be fixable from this failure
/// alone.
#[test]
fn goldens_are_in_canonical_form() {
    let manifest_dir = repo_root();
    let mut violations = golden_form_violations(&manifest_dir, &discover_e2e_cases(&manifest_dir));
    violations.extend(golden_form_violations(
        &manifest_dir,
        &discover_examples(&manifest_dir),
    ));
    assert!(
        violations.is_empty(),
        "{} golden file(s) are not in the canonical form of AR-workspace.9.1 — an output \
         golden is never zero bytes and holds no carriage return, an exit golden is the \
         decimal code and exactly one newline. UPDATE_EXPECTED=1 rewrites every one of \
         these, whatever case the change was about:\n{}",
        violations.len(),
        violations.join("\n")
    );
}
