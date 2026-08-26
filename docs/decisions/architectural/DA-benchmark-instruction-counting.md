# DA-benchmark-instruction-counting: the performance harness counts instructions, not wall-clock seconds

**Status:** Accepted
**Date:** 2026-05-11

## 1. Context

[§GOAL-fast-feedback.1](../../goals.md#1-performance-targets) writes down performance targets in wall-clock units — under 100 ms on this repo, under 1 s on a 10k-file repo — while [§AR-goal-measurement.2](../../architecture/AR-goal-measurement.md#2-goal-meters) and [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) own the meter. The original roadmap text for the benchmark milestone named a `cargo bench` **criterion** (wall-clock) harness. When it came time to build it, the question was which kind of meter actually makes "fail CI on regression" work, given that the harness runs on shared GitHub-hosted runners.

There are three plausible shapes:

1. **Wall-clock micro-benchmarks (criterion / divan).** Measure elapsed time directly, in the units the budget is written in.
2. **Instruction counting under Valgrind/Callgrind (iai-callgrind).** Measure the number of CPU instructions executed (plus cache-miss estimates) — a deterministic proxy for cost.
3. **Both** — criterion for the headline ms figure, iai-callgrind for the regression gate.

And an orthogonal axis: benchmark the **library** (`grund::` calls) or the **binary** (the actual `grund` process under test).

## 2. Decision

**Instruction counting (option 2), against the binary.** The harness is `crates/grund-cli/benches/instructions.rs`; it uses `iai-callgrind` to run the freshly built `grund` binary as a subprocess under Callgrind, on this repository's own conformant tree, for the subcommands the agent loop and the CI/pre-commit gate run most (`check`, `list`, `show`, `refs`, `cover`, `fmt --check`). Its real benchmark body is gated behind a `bench` Cargo feature so `cargo test --all-targets` compiles only a no-op bench target and never tries to run it without Valgrind installed. The contract this realizes is [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands). The roadmap text is updated to match (the "criterion" wording is superseded by this record).

## 3. Why this shape

### 3.1 Determinism is what makes "fail on regression" implementable

A Callgrind instruction count is the same number every run for a given binary and input — it does not move with runner load, neighbour-VM noise, CPU frequency scaling, or kernel scheduling. A wall-clock measurement on a GitHub-hosted runner varies enough that a regression gate built on it has to choose between flaking (a threshold tight enough to catch a real regression also trips on a noisy run) and uselessness (a threshold loose enough never to flake also lets real regressions through). Instruction count sidesteps the dilemma: the regressions [§GOAL-fast-feedback.2](../../goals.md#2-how-we-get-there) is designed against — an accidental quadratic walk, a second read pass over every file, a per-line allocation that should have been hoisted — move the instruction count by far more than any scheduling noise could mask, so a modest fixed threshold is both stable and meaningful. This is the standard reason projects that need a *CI-gating* performance check (rather than a developer-facing profiler) reach for iai-callgrind.

### 3.2 The ms budget is still covered — twice

Choosing instruction counts does not abandon the millisecond budget [§GOAL-fast-feedback.1](../../goals.md#1-performance-targets) is written in. It is backstopped from below by the cheap wall-clock guard already in place ([§AR-ci.4](../../architecture/AR-ci.md#4-performance-smoke-guard): the built `grund .` self-check under a generous `timeout` — tens of seconds, against a tens-of-milliseconds budget), which trips on a catastrophic blowup. And the instruction count is a tight enough proxy that a regression in it is a regression in time; the absolute ms figure for the local wall-clock report is recorded separately, in `docs/benchmarks.md`, for the record, not re-measured in the gate. So: instruction count is the precise regression meter, the `timeout` is the catastrophic-blowup floor, and the recorded ms baseline is the human-readable headline. Nothing is lost.

### 3.3 Benchmark the binary, because that is what gets invoked

The audience for this budget is `grund` running on every save, commit, and push — a process, not a function call. Process start-up, argument parsing, config discovery, the walk, and output formatting are all part of the cost an agent or CI pays, and a library-only benchmark would miss the ones that live in `main`. iai-callgrind's binary-benchmark mode runs the real `target/.../grund` under Callgrind, so the figure is the whole invocation. The public `grund-core` API of [§AR-bindings.2](../../architecture/AR-bindings.md#2-grund-core-the-only-place-logic-lives) gives embedders a separate measurement surface, but this benchmark deliberately stays on the binary because that is what the agent loop invokes.

### 3.4 Not "both", for now

Running criterion *and* iai-callgrind doubles the harness, the CI time, and the maintenance surface for a marginal gain — the recorded ms baseline (§3.2) already supplies the headline figure without a second framework. If a future need appears for continuous wall-clock tracking (say, to catch a regression that is invisible in instruction count because it is I/O- or syscall-bound), criterion can be added then as a second `[[bench]]`; the decision here is not to start with it.

## 4. Consequences

- `crates/grund-cli/benches/instructions.rs` is added, with `iai-callgrind` as a `[dev-dependency]` and a `bench` Cargo feature gating the benchmark body (so `cargo test --all-targets` / `cargo build --all-targets` compile only a no-op bench target unless the feature is enabled).
- A `bench` CI job installs `valgrind` and the version-matched `iai-callgrind-runner` and runs `cargo bench -p grund --features bench --bench instructions` — [§AR-ci.5](../../architecture/AR-ci.md#5-benchmark-job). With [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands)'s committed baseline and regression threshold, it is the authoritative [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) meter and [§AR-ci.4](../../architecture/AR-ci.md#4-performance-smoke-guard)'s timeout stays as a backstop.
- [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) is added as the architectural spec for the harness; the roadmap milestone `RM-benchmarks` is updated so its "What" / "Measurable" describe an instruction-counting harness rather than a criterion one, and cites this record for the why.
- No `tests/e2e/cases/*` change, no CLI surface change, no `grund_config_version` change — this is build-and-CI tooling, outside the [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) user-visible surface.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| **(1) Wall-clock micro-benchmarks (criterion / divan).** Measures the budget's actual units (ms); familiar; great developer-facing profiler. | Variance on shared CI runners forces the regression gate to choose between flaking and uselessness (§3.1). Good for ad-hoc local profiling, wrong for a CI-gating check. |
| **(3) Both criterion and iai-callgrind.** Headline ms figure *and* a stable regression gate. | Doubles harness, CI time, and maintenance for a gain the recorded-baseline ms figure (§3.2) already covers. Deferred, not foreclosed — criterion can be added as a second `[[bench]]` if a wall-clock-only regression ever needs continuous tracking. |
| **Library benchmarks instead of binary benchmarks.** Slightly faster to run; no subprocess. | Misses the cost in `main` (start-up, arg parsing, config discovery, formatting) that an actual invocation pays; and as of 0.1.0 there is no stable public library surface to bench anyway (§3.3). |
| **(2), against the binary — chosen.** | See §2 and §3. |
