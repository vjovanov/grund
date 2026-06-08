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

## 2. [0.4.2] — 2026-06-08

### Added

- [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) / [§FS-fmt.6](functional-spec/FS-fmt.md#6-cross-reference-emission): document the measured `o200k_base` token impact of generated Markdown citation links and keep the benchmark-report generator in sync. PR #31.

### Fixed

- [§GOAL-token-economy](goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) / [§AR-goal-measurement](architecture/AR-goal-measurement.md#ar-goal-measurement-goal-meters-live-outside-goals): keep `docs/goals.md` minimal by moving goal measurement detail to a dedicated architecture map. PR #32.
- [§FS-check.3.7](functional-spec/FS-check.md#37-misplaced-declaration-configured-kind-home) / [§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes): `grund check` now rejects declarations whose kind conflicts with the containing unique configured kind home, while preserving cross-kind citations and ambiguous-home cases. PR #30.

## 3. Older releases

- [0.4.1](changelog/0.4.1.md) — 2026-05-25: - [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) / [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets): add the generated 10k-file benchmark fixture, record instruction-count baselines, and make pull-request CI fail on >5% Callgrind instruction-count regressions.
- [0.4.0](changelog/0.4.0.md) — 2026-05-19: - [§FS-inline-citation-style](functional-spec/FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations) / [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form): add configurable inline citation style enforcement for source comments.
- [0.3.0](changelog/0.3.0.md) — 2026-05-18: Default-show release.
- [0.2.0](changelog/0.2.0.md) — 2026-05-17: Workspace and agent-entrypoint release.
- [0.1.0](changelog/0.1.0.md) — 2026-05-14: first published release and baseline CLI surface.
