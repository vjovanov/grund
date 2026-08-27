//! Instruction-counting benches for the hot CLI commands. Each runs the freshly
//! built `grund` binary under Callgrind (`iai-callgrind`) for a deterministic,
//! CI-stable instruction count.
//!
//! §AR-benchmarks: every benchmark scans a *generated fixture*, never this repo —
//! a fixed input isolates a genuine code slowdown (what the gate must fail on)
//! from the repo merely doing more work as its own docs/config evolve. See the
//! spec for the full rationale, the fixture set, and why one fixture enables
//! `[citations]` while the rest omit it.
//!
//! The command list also drives the PGO training run in `scripts/pgo-build.sh`;
//! keep them in sync. Run with
//! `cargo bench -p grund --features bench --bench instructions` (needs Valgrind and
//! `iai-callgrind-runner` on `PATH`).

#[cfg(feature = "bench")]
use iai_callgrind::{Command, binary_benchmark, binary_benchmark_group, main};
#[cfg(feature = "bench")]
use std::{
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

/// The freshly built `grund` binary under test (Cargo exports this env var).
#[cfg(feature = "bench")]
const GRUND: &str = env!("CARGO_BIN_EXE_grund");
/// Repository root — used only to locate the fixture generator script, never
/// scanned by a benchmark.
#[cfg(feature = "bench")]
const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
/// Generated canonical fixture for the per-command suite — large enough to be
/// representative, small enough to keep the suite quick.
#[cfg(feature = "bench")]
const CANONICAL_REPO_REL: &str = "target/bench-fixtures/canonical-repo";
#[cfg(feature = "bench")]
const CANONICAL_FILE_COUNT: usize = 1_500;
/// Generated large conformant fixture for the `grund check` budget benchmark.
#[cfg(feature = "bench")]
const LARGE_REPO_REL: &str = "target/bench-fixtures/large-conformant-repo";
#[cfg(feature = "bench")]
const LARGE_REPO_FILE_COUNT: usize = 10_000;

/// A declared ID the canonical fixture always contains (`id_for(1)` in the
/// generator) — the subject of the `show` ladder and `refs` benchmarks. The
/// fixture's bodies are uniform, so these measure the scan + render path that
/// dominates, not body size.
#[cfg(feature = "bench")]
const FIXTURE_ID: &str = "FS-00001-feature-00001";

/// Generated canonical fixture that declares `[citations]`, so `check_citations`
/// exercises the citing-side classification + direction passes (§FS-config.3.9).
#[cfg(feature = "bench")]
const CITATIONS_REPO_REL: &str = "target/bench-fixtures/canonical-citations-repo";

#[cfg(feature = "bench")]
fn ensure_fixture(rel: &str, file_count: usize, citations: bool) -> PathBuf {
    let root = Path::new(REPO).join(rel);
    let script = Path::new(REPO).join("scripts/generate_large_benchmark_fixture.py");
    let mut command = ProcessCommand::new("python3");
    command
        .arg(script)
        .arg("--root")
        .arg(&root)
        .arg("--files")
        .arg(file_count.to_string());
    if citations {
        command.arg("--citations");
    }
    let status = command.status().expect("run benchmark fixture generator");
    assert!(status.success(), "benchmark fixture generator failed");
    root
}

#[cfg(feature = "bench")]
fn canonical_repo() -> PathBuf {
    ensure_fixture(CANONICAL_REPO_REL, CANONICAL_FILE_COUNT, false)
}

// `grund check <fixture>` — validate every citation in the tree.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn check() -> Command {
    Command::new(GRUND)
        .arg("check")
        .arg(canonical_repo())
        .build()
}

// `grund check <large-fixture>` — the 10k-file budget input.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn check_large_10k() -> Command {
    let root = ensure_fixture(LARGE_REPO_REL, LARGE_REPO_FILE_COUNT, false);
    Command::new(GRUND).arg("check").arg(root).build()
}

// `grund check <fixture-with-citations>` — the citation-direction code path
// (classify + obligation + prohibition passes, §FS-config.3.9). Same file count
// as `check`, so the delta against it is the direction-checking overhead.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn check_citations() -> Command {
    let root = ensure_fixture(CITATIONS_REPO_REL, CANONICAL_FILE_COUNT, true);
    Command::new(GRUND).arg("check").arg(root).build()
}

// `grund list <fixture>` — every declared ID.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn list() -> Command {
    Command::new(GRUND)
        .arg("list")
        .arg(canonical_repo())
        .build()
}

// `grund <ID> --brief <fixture>` — title plus first paragraph.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn show_brief() -> Command {
    Command::new(GRUND)
        .args(["show", FIXTURE_ID, "--brief"])
        .arg(canonical_repo())
        .build()
}

// `grund <ID> <fixture>` — the lead-default declaration read.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn show() -> Command {
    Command::new(GRUND)
        .args(["show", FIXTURE_ID])
        .arg(canonical_repo())
        .build()
}

// `grund <ID> --full <fixture>` — one full declaration body.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn show_full() -> Command {
    Command::new(GRUND)
        .args(["show", FIXTURE_ID, "--full"])
        .arg(canonical_repo())
        .build()
}

// `grund refs <ID> <fixture>` — every citation site of an ID.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn refs() -> Command {
    Command::new(GRUND)
        .args(["refs", FIXTURE_ID])
        .arg(canonical_repo())
        .build()
}

// `grund cover <fixture>` — the citation graph grouped by scanned file.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn cover() -> Command {
    Command::new(GRUND)
        .arg("cover")
        .arg(canonical_repo())
        .build()
}

// `grund fmt --check <fixture>` — report (without writing) any non-canonical citation.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn fmt_check() -> Command {
    Command::new(GRUND)
        .args(["fmt", "--check"])
        .arg(canonical_repo())
        .build()
}

#[cfg(feature = "bench")]
binary_benchmark_group!(
    name = commands;
    benchmarks = check, check_large_10k, check_citations, list, show_brief, show, show_full, refs, cover, fmt_check
);

#[cfg(feature = "bench")]
main!(binary_benchmark_groups = commands);

#[cfg(not(feature = "bench"))]
fn main() {}
