# Changelog

Records every notable change to `grund`. Versions follow semver; the **latest release is inline** in this file, and **older releases live one-per-file under `docs/changelog/`** so a reader (human or agent) only loads the history they ask for. Each entry cites the FS/AR/G/DF IDs it touches, so the changelog is itself part of the conformant tree (`grund .` validates the citations).

Schema-version bumps are called out explicitly: `grund_config_version` ([§FS-config.5](functional-spec/FS-config.md#5-schema-versioning)) and the `AGENTS.md` init block version ([§FS-init.2](functional-spec/FS-init.md#2-outputs)). A bump to either is a breaking change for the consumer and must appear under **Changed** with a migration note.

## 1. Conventions

### 1.1 Sections per release

`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security` — the Keep-a-Changelog set; omit any with no entries. A large entry (the first release, folding in pre-history, is the case in point) may add narrative subsection headings — e.g. `Baseline`, `Renamed`, `Implemented`, `Distribution and bindings` — for readability when the standard six would bury the structure; the semver-relevant changes still live under the standard names.

### 1.2 Schema version callouts

Any change to `grund_config_version` or the `AGENTS.md` block version goes under **Changed** with the prefix `**Schema:**` and a one-line migration pointer.

### 1.3 Entry style

One bullet per change, present tense, leading with the affected ID. Example: `§FS-show: add --head mode for truncated output`.

### 1.4 Progressive discovery

Only **Unreleased** and the **most recent release** are inline. When a new release ships, the previous "latest" section is moved verbatim to `docs/changelog/<version>.md` and a one-line link is added under [§3 Older releases](#3-older-releases). The most recent release stays inline so the common reader and agent path — "what changed lately?" — is one file deep.

## Unreleased

### Fixed

- [§FS-check.3.7](functional-spec/FS-check.md#37-misplaced-declaration-configured-kind-home) / [§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes): `grund check` now rejects declarations whose kind conflicts with the containing unique configured kind home, while preserving cross-kind citations and ambiguous-home cases. PR #30.

## 2. [0.4.1] — 2026-05-25

### Added

- [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) / [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets): add the generated 10k-file benchmark fixture, record instruction-count baselines, and make pull-request CI fail on >5% Callgrind instruction-count regressions. PR #19.
- [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli) / [§FS-distribution.3.1](functional-spec/FS-distribution.md#31-rust-grund-core-crate): add the dedicated `crates/grund-cli` frontend package, make the root manifest a virtual workspace, expose the initial `grund-core` embedding API (`check`, `show`, `scan`, `Report`, `Findings`, `ShowOpts`), and move the remaining CLI surfaces (`refs`, `list`, `cover`, `fmt`, `id`, `init`, config inspection) onto data-returning core APIs with text/JSON rendering and exit-code mapping in `grund-cli`. PR #20, PR #21.
- [§RM-parallel-scan](roadmap.md#rm-parallel-scan-parallel-per-file-scanning-for-large-repo-throughput) / [§AR-scanner](architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations): scan large sorted file lists and workspace projects in parallel while merging findings in deterministic path order, and publish updated benchmark results for the parallel path. PR #22.

### Changed

- [§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes) / [§FS-init.2.3.4.4](functional-spec/FS-init.md#2344-project-map): generated configs and project maps use clearer core kind titles (`Why`, `Where`, `What`, `How`) without adding a configurable document-structure TOML surface. PR #15.
- [§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes) / [§FS-init.5](functional-spec/FS-init.md#5-agent-setup-instructions) / [§DF-skill-init-existing-specs](decisions/functional/DF-skill-init-existing-specs.md#df-skill-init-existing-specs-grund-init-adopts-existing-specs-before-scaffolding): rename the default grounding kind tag to `GRUND`, and clarify that the `grund-init` skill asks users to choose an artifact model before adopting repos that already have specs. PR #17.
- [§FS-init.2.3.4.16](functional-spec/FS-init.md#23416-project-namespaces) / [§FS-workspace.1](functional-spec/FS-workspace.md#1-citation-syntax): generated `AGENTS.md` now teaches agents when to keep citations local, when to create or use a project namespace, and how to cite and validate cross-namespace references. PR #27.
- [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli) / [§AR-bindings](architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms): finish the core/CLI boundary for the LSP and binding work by moving `check`, `show`, and shell-completion argument parsing/rendering into `crates/grund-cli`, adding `grund-core` data APIs for optioned checks and dynamic ID completion, and leaving only private compatibility command adapters behind `grund_core::main_entry()`. PR #24.
- [§RM-parallel-scan](roadmap.md#rm-parallel-scan-parallel-per-file-scanning-for-large-repo-throughput) / [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process): move completed roadmap work out of the active queue and keep package-name verification documented as part of the release process rather than as a standalone roadmap milestone. PR #23.

### Fixed

- [§FS-check.3.1](functional-spec/FS-check.md#31-dangling-citation): dangling-reference diagnostics now suggest the nearest declared same-kind ID when the misspelling is close enough, while unrelated missing IDs keep the plain `unknown reference <ID>` message. PR #28.
- [§AR-ci](architecture/AR-ci.md#ar-ci-ci-mirrors-the-local-pre-commit-gate): CI cache restore/save failures no longer abort a matrix job before the actual checks run. PR #22.
- [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process): release recovery runs for pre-split tags now accept the historical root `[package] version` manifest shape as well as the new workspace-package version. PR #20.
- [§AR-scanner.6](architecture/AR-scanner.md#6-e2e-case-declarations) / [§RM-self-host](roadmap.md#rm-self-host-guard-the-self-host-loop-in-ci): ordinary scans now treat direct E2E case directories as manifest boundaries, so nested fixture repos do not pollute the outer report under the canonical default config. PR #18.
- [§AR-ci.7](architecture/AR-ci.md#7-pull-request-changelog-gate) / [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process): the local pre-push hook now runs the PR changelog-entry gate once `gh` can resolve the current branch's pull request number. PR #18.

## 3. Older releases

- [0.4.0](changelog/0.4.0.md) — 2026-05-19: - [§FS-inline-citation-style](functional-spec/FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations) / [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form): add configurable inline citation style enforcement for source comments.
- [0.3.0](changelog/0.3.0.md) — 2026-05-18: Default-show release.
- [0.2.0](changelog/0.2.0.md) — 2026-05-17: Workspace and agent-entrypoint release.
- [0.1.0](changelog/0.1.0.md) — 2026-05-14: first published release and baseline CLI surface.
