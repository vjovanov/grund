//! §REQ-deterministic-output — same input, same bytes, whatever the thread
//! count. The scan is rayon-parallel and merges per-file results afterwards
//! (§AR-scanner), and the e2e determinism pass runs each tiny fixture twice
//! under one pool, so the merge order was never exercised. Every
//! report-producing command over this repository, and `check` over every
//! plain-check fixture, must be byte-identical — stdout, stderr and exit — at
//! one thread and at eight.

#[path = "binaries.rs"]
mod binaries;
#[path = "corpus.rs"]
mod corpus;

use std::path::Path;
use std::process::Command;

const THREADS: &[&str] = &["1", "8"];
const REPO_COMMANDS: &[&[&str]] = &[
    &["check", "--full"],
    &["check", "--full", "--format", "json"],
    &["check", "--full", "--suggestions"],
    &["cover", "--format=json"],
    &["cover"],
    &["list"],
    &["list", "--format", "json"],
    &["list", "--unused"],
];
const MIN_CASES: usize = 80;

#[derive(PartialEq, Eq, Debug)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

fn run(grund: &Path, cwd: &Path, args: &[&str], threads: &str) -> Run {
    let output = Command::new(grund)
        .args(args)
        .current_dir(cwd)
        .env("RAYON_NUM_THREADS", threads)
        .output()
        .unwrap_or_else(|err| panic!("run grund {args:?}: {err}"));
    Run {
        stdout: output.stdout,
        stderr: output.stderr,
        code: output.status.code(),
    }
}

fn differences(grund: &Path, cwd: &Path, args: &[&str]) -> Option<String> {
    let baseline = run(grund, cwd, args, THREADS[0]);
    for threads in &THREADS[1..] {
        let other = run(grund, cwd, args, threads);
        if other != baseline {
            return Some(format!(
                "grund {} in {}: RAYON_NUM_THREADS={} and ={threads} differ\n  exit {:?} vs {:?}\n  stdout {} vs {} bytes\n  stderr {} vs {} bytes",
                args.join(" "),
                cwd.display(),
                THREADS[0],
                baseline.code,
                other.code,
                baseline.stdout.len(),
                other.stdout.len(),
                baseline.stderr.len(),
                other.stderr.len()
            ));
        }
    }
    Some(baseline)
        .filter(|run| !matches!(run.code, Some(0) | Some(1)))
        .map(|run| {
            format!(
                "grund {} in {} exited {:?} — the invocation itself is broken, so it compares nothing:\n{}",
                args.join(" "),
                cwd.display(),
                run.code,
                String::from_utf8_lossy(&run.stderr)
            )
        })
}

#[test]
fn every_report_is_byte_identical_at_one_thread_and_at_eight() {
    let repo = binaries::repo_root();
    let grund = binaries::grund();
    let mut problems = Vec::new();
    for args in REPO_COMMANDS {
        problems.extend(differences(&grund, &repo, args));
    }
    let selection = corpus::plain_check_cases(&repo);
    let mut compared = 0;
    for case in &selection.cases {
        let baseline = run(
            &grund,
            &case.root,
            &["check", ".", "--format", "json"],
            THREADS[0],
        );
        for threads in &THREADS[1..] {
            let other = run(
                &grund,
                &case.root,
                &["check", ".", "--format", "json"],
                threads,
            );
            if other != baseline {
                problems.push(format!(
                    "{}: check differs at {} and {threads} threads",
                    case.name, THREADS[0]
                ));
            }
        }
        compared += 1;
    }
    eprintln!(
        "scan determinism: {} repository command(s) and {compared} fixture(s) compared at {} thread counts",
        REPO_COMMANDS.len(),
        THREADS.len()
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(compared >= MIN_CASES, "only {compared} fixtures compared");
}
