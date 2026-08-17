// §RM-core-cli-split: e2e tests now exercise the dedicated CLI frontend crate.
use std::path::PathBuf;

#[path = "support/case_runner.rs"]
mod case_runner;

use case_runner::CaseKind::{E2e, Example};
use case_runner::{
    assert_case_is_deterministic, assert_every_case_ran, discover_e2e_cases, discover_examples,
    run_case,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// Every pass collects its per-case outcomes and hands them to
// `assert_every_case_ran`: a case the platform could not build is counted and
// named there, never left to look like one of the passes libtest reports.

#[test]
fn e2e_cases_match_expected_reports() {
    let manifest_dir = repo_root();
    let outcomes = discover_e2e_cases(&manifest_dir)
        .iter()
        .map(|case| run_case(&manifest_dir, case, E2e))
        .collect::<Vec<_>>();
    assert_every_case_ran("e2e cases", &outcomes);
}

#[test]
fn e2e_output_is_deterministic() {
    let manifest_dir = repo_root();
    let outcomes = discover_e2e_cases(&manifest_dir)
        .iter()
        .map(|case| assert_case_is_deterministic(&manifest_dir, case))
        .collect::<Vec<_>>();
    assert_every_case_ran("e2e determinism", &outcomes);
}

#[test]
fn examples_are_e2e_cases() {
    let manifest_dir = repo_root();
    let outcomes = discover_examples(&manifest_dir)
        .iter()
        .map(|case| run_case(&manifest_dir, case, Example))
        .collect::<Vec<_>>();
    assert_every_case_ran("examples", &outcomes);
}

#[test]
fn example_output_is_deterministic() {
    let manifest_dir = repo_root();
    let outcomes = discover_examples(&manifest_dir)
        .iter()
        .map(|case| assert_case_is_deterministic(&manifest_dir, case))
        .collect::<Vec<_>>();
    assert_every_case_ran("example determinism", &outcomes);
}
