# AR-ci: CI mirrors the local pre-commit gate

The CI workflow is the remote form of the local pre-commit gate. Anything that can abort a local commit must also abort CI, so a contributor cannot bypass the repository's local checks by skipping hooks or editing through a web UI. This supports [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) and keeps the link-checking boundary from [§FS-non-goals.1](../functional-spec/FS-non-goals.md#1-markdown-link-validation) enforced alongside `grund check`.

## 1. Pre-commit is the source of truth

The hook list lives in `.pre-commit-config.yaml`. CI must invoke that list directly with `pre-commit run --all-files`, rather than hand-copying each hook into separate workflow steps. The workflow may install hook prerequisites first, but the set of checks is defined by the pre-commit config.

When a new pre-commit hook is added, the same change must ensure CI can run it. If the hook needs an external binary, the CI workflow installs that binary before the pre-commit step. If the hook is intentionally local-only, it does not belong in `.pre-commit-config.yaml`; put it in a developer-local hook instead.

## 2. Platform scope

The full Rust build and test matrix still runs on every configured operating system. The pre-commit gate may run on one representative CI platform when the hooks are platform-independent, because its job is policy parity with local commits, not cross-platform behavior coverage. Platform-specific behavior belongs in the build and test jobs.

CI dependency and build caches are performance optimizations only. A cache restore/save failure must not abort the matrix before the actual checks run; the job should continue cold and let `fmt`, the changelog gate, Python tests, pre-commit, build, self-check, tests, or benchmarks decide pass/fail.

## 3. Current hooks

The current pre-commit gate runs the same Rust format/build/test commands that development CI runs: `cargo fmt --all -- --check`, `cargo build --workspace --all-targets --locked` with warnings denied, and `cargo test --workspace --all-targets --locked`. The test hook also runs at `pre-push`, so a contributor who commits while a test is transiently broken still gets the same local stop before sending the branch. The changelog PR-entry gate also runs at `pre-push` when the current branch already has a GitHub pull request number, so follow-up pushes cannot miss the `docs/changelog.md` `## Unreleased` entry CI will require.

The gate also runs `grund check`, including the grounding floor from [§FS-check.3.6](../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in), `grund fmt --write` for canonical citation links, and `lychee` for Markdown links. `grund` owns ID citations across docs and source; `lychee` owns regular Markdown links and URLs. Running both in CI preserves that boundary.

## 4. Performance smoke guard

CI carries a cheap floor on [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) that does not depend on any benchmarking toolchain: the `grund .` self-check step runs the already-built binary under a generous wall-clock `timeout` — long enough never to flake on a loaded runner, short enough to fail the build on a catastrophic regression such as an accidental quadratic walk or a second read pass over every file. It is not the budget itself (the budget is tens of milliseconds; the ceiling is tens of seconds) — it is the difference between "we'd notice eventually" and "the build is red on the commit that did it". The precise per-commit meter is the §5 benchmark job, and this timeout stays as its catastrophic backstop.

## 5. Benchmark job

A separate `bench` job runs the instruction-counting harness ([§AR-benchmarks](AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands)). Because the harness runs `grund` under Callgrind, the job first installs `valgrind` and `cargo install`s `iai-callgrind-runner` pinned to the same version as the `iai-callgrind` dev-dependency. It runs on one representative platform (the harness measures instruction counts, not cross-platform behavior — §2's reasoning applies) and on every push and pull request, so the numbers are recorded per commit.

On pull requests, the job checks out the PR base branch and runs `cargo bench -p grund --features bench --locked --bench instructions -- --save-baseline=main --save-summary=json`; then it checks out the PR commit and runs `cargo bench -p grund --features bench --locked --bench instructions -- --baseline=main --save-summary=json`. The named baseline comes from the current base branch rather than a stale generated artifact, while the committed baseline figures in `docs/benchmarks.md` remain the human-readable reference point. The harness owns the regression limits: 5% `Ir` for the generated 10k-file fixture, whose size is stable, and 12.5% `Ir` for self-repo hot commands, whose input intentionally grows with checked-in specs, docs, and source. Growth beyond those limits is a build failure for [§GOAL-fast-feedback.3](../goals.md#3-measurable). On pushes to `main`, the job records the current counts and saves JSON summaries without comparing against itself.

The harness body is gated behind the `bench` Cargo feature, so the regular build and test matrix compiles only a no-op bench target and stays free of the Valgrind dependency — only this job needs it; that gate is also why §1's "CI installs hook prerequisites" pattern is followed here as a dedicated job rather than folded into the test matrix.

## 6. PGO stays out of development CI

Development CI does **not** run the profile-guided-optimization pipeline. `scripts/pgo-build.sh` does an instrumented release build, runs the [§AR-benchmarks](AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) self-repo hot command list as the training run, merges profiles with `llvm-profdata`, and rebuilds the optimized release artifact; that cost and toolchain belong to release packaging ([§FS-distribution.4](../functional-spec/FS-distribution.md#4-release-process), [§DA-pgo-release](../decisions/architectural/DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload)) and explicit benchmark work, not ordinary push/PR feedback. The manual **Pre-release checks** workflow runs the PGO script and self-checks the resulting binary before publish; a release is blocked unless that job passes. The dev loop remains `fmt`, pre-commit hooks, build, self-check, tests, and the instruction-counting benchmark job (§5).

## 7. Pull-request changelog gate

On `pull_request` events, CI runs `scripts/check_changelog_pr_entry.py` before the Python unit tests. The gate is intentionally local and deterministic: it reads the current PR number from the workflow context and scans only `docs/changelog.md`'s `## Unreleased` body for `PR #N`, `pull request #N`, or a `/pull/N` URL. It does not call the GitHub API in CI, so forks and restricted-token runs get the same result as owner branches. Push CI skips the gate because there is no current PR number. The local pre-push hook may ask `gh pr view` for the current branch's PR number and then runs the same check; if no current-branch PR exists yet, it skips and leaves the first enforceable check to pull-request CI. This implements the release-note discipline in [§FS-distribution.4](../functional-spec/FS-distribution.md#4-release-process).
