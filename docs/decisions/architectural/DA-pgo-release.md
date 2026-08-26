# DA-pgo-release: distributed binaries are PGO-built, trained on the benchmark workload

**Status:** Accepted
**Date:** 2026-05-11

## 1. Context

[§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) makes speed an ordering principle, and `grund` is a branch-heavy program — a scanner walking a tree, line-classifying, and matching regexes — exactly the shape profile-guided optimization helps most (the compiler lays out the hot blocks and inlines along the paths a representative run actually takes). The `[profile.release]` settings already opt into `lto = true` / `codegen-units = 1`; PGO is the next layer. Two questions: how is the PGO pipeline wired (it cannot live in `Cargo.toml`), and what is the training corpus?

[§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) already pins a workload — `grund check`, `list`, `show`, `refs`, `cover`, `fmt --check` over this repo's own conformant tree — chosen as "the commands agents and CI invoke most". That is also a faithful answer to "what should the optimizer profile against".

## 2. Decision

**The distributed `grund` binary is built by `scripts/pgo-build.sh`, with the [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) self-repo hot command list as the training run.** The script does the standard three-phase PGO build: `cargo build --release` with `-Cprofile-generate`, run that workload against this repo's tree to emit `.profraw` profiles, `llvm-profdata merge`, then `cargo build --release` with `-Cprofile-use`. It is a plain shell script (no `cargo-pgo` dependency), consistent with `scripts/check-registry-names.sh`, and requires only the `llvm-tools-preview` rustup component for `llvm-profdata`. The release pipeline ([§RM-distribution](../../roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine)) runs it to produce the cargo-published binary; the `napi-rs` and `PyO3` prebuilt binaries get the same treatment when those land ([§FS-distribution.4](../../functional-spec/FS-distribution.md#4-release-process)). Benchmarking can also run the script when the thing being measured is the optimized release artifact.

This applies to **release and benchmarking** paths only. `cargo install grund` builds from source with a plain `cargo build --release` and no profile — `cargo install` cannot run a custom build script, and that is fine: PGO is a packaging/performance step, not a correctness one. Development builds, tests, and push/PR CI stay non-PGO; a source release build is still LTO-optimized.

## 3. Why this shape

### 3.1 The benchmark workload is already the canonical "hot path"

[§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) picked its self-repo command set because it is what the agent loop and the CI/pre-commit gate actually run. Reusing it as the PGO training corpus means there is **one** definition of "the everyday operations that matter": the thing we benchmark on this repo is the thing we profile against, and when that hot command list changes, both the regression meter and the optimizer's input move together. The generated `check_large_10k` benchmark stays outside PGO training because it exists to enforce the large-repo budget, not to model a release user's typical invocation. The command list is duplicated in two forms (a Rust array in `crates/grund-cli/benches/instructions.rs`, a shell loop in `scripts/pgo-build.sh`) because one is a `cargo bench` target and the other a build script; each carries a "keep in sync" comment pointing at the other.

### 3.2 This repo's tree as the training input

The training run scans `grund`'s own repository — the same input the self-host loop ([§AR-ci.1](../../architecture/AR-ci.md#1-pre-commit-is-the-source-of-truth)) and the benchmarks already use. It is a real conformant tree of the size the small-repo promise targets ([§GOAL-small-and-large.1](../../goals.md#1-small-repo-promise)), with the full spread of file types (Markdown specs, Rust sources, e2e fixture manifests), so the recorded profile generalizes to the common case. A generated large synthetic tree (the one [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) adds) can be folded into the training run later if profiling it ever changes the layout the optimizer picks; starting with the real repo keeps the pipeline simple and the profile honest.

### 3.3 A script, not a `Cargo.toml` profile

`-Cprofile-use` needs the merged `.profdata` file to exist at build time. It does not exist on a fresh clone, and there is no way to express "build this binary, run it, then rebuild it against its own profile" inside `[profile.release]` — PGO is inherently a multi-phase pipeline. Putting a hard-coded profile path in the manifest would break `cargo build --release` for every contributor. A script is the only correct shape; the manifest carries a comment pointing at it.

### 3.4 Hand-rolled, not `cargo-pgo`

`cargo-pgo` automates the same steps and handles some edge cases (locating `llvm-profdata`, the warn-missing flags). But it is one more tool to install in the release pipeline, the steps here are few and explicit, and a transparent script matches how this repo already does release-adjacent automation (`scripts/check-registry-names.sh`). `cargo-pgo` is the obvious upgrade path if the script grows; it is not needed yet.

### 3.5 Release and benchmark verification, not development CI

PGO that is documented but never run rots, but running two optimized builds on every push makes normal development feedback slower for a packaging concern. The script is therefore exercised by the manual pre-release workflow, by the release pipeline that produces distributed artifacts, and by explicit benchmark work when comparing the optimized release artifact. The regular development loop stays on `cargo build`, `cargo test`, `grund check`, and `cargo bench -p grund --features bench --bench instructions`; the benchmark job records the hot workload without turning every push into a PGO release build.

## 4. Consequences

- `scripts/pgo-build.sh` is added; `[profile.release]` in `Cargo.toml` gains a comment pointing at it.
- Normal development CI does not run PGO; [§AR-ci.6](../../architecture/AR-ci.md#6-pgo-stays-out-of-development-ci) pins that boundary and the required pre-release PGO check.
- [§FS-distribution.4](../../functional-spec/FS-distribution.md#4-release-process) (release process) states that the distributed binaries are PGO-built via `scripts/pgo-build.sh` with the [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) self-repo workload as the training run.
- [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) notes that its self-repo workload doubles as the PGO training corpus, with the why here.
- No CLI surface change, no `grund_config_version` change, no `tests/e2e/cases/*` change — this is packaging, outside the [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) user-visible surface. The PGO binary is byte-for-byte behavior-identical to the LTO-only one; only its performance differs.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| **PGO settings in `[profile.release]`.** Self-contained; no script. | Impossible — `-Cprofile-use` needs a merged profile that does not exist on a fresh clone, and the manifest cannot express a generate-run-rebuild pipeline. A hard-coded path would break `cargo build --release` for everyone. |
| **`cargo-pgo` instead of a hand-rolled script.** Automates the phases; handles `llvm-profdata` discovery. | One more pipeline dependency for steps that are few and explicit; a plain script matches `scripts/check-registry-names.sh`. Kept as the upgrade path, not the starting point. |
| **A dedicated PGO training corpus, separate from the self-repo benchmark workload.** Could be tuned independently for the optimizer. | A second source of truth for "the everyday operations that matter" that can drift from [§AR-benchmarks](../../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands). Reusing one workload keeps the self-repo regression meter and the optimizer input in lockstep (§3.1). |
| **PGO `cargo install grund` source builds too.** Every install gets the win. | `cargo install` cannot run a custom build script; PGO is a packaging step. Source builds stay LTO-optimized, which is enough; the prebuilt distributed binaries carry the PGO win. |
| **No PGO; LTO only.** Simpler; one build. | Leaves a measurable, free-at-distribution-time speed-up on the table for a branch-heavy program, against an ordering principle ([§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)) that says to take it. |
