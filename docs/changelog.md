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

### Added

- [§DF-citation-directions](decisions/functional/DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels) / [§FS-config.3.9](functional-spec/FS-config.md#39-citations--citation-direction-rules) / [§FS-check.3.11](functional-spec/FS-check.md#311-missing-required-citation) / [§FS-check.3.12](functional-spec/FS-check.md#312-forbidden-citation) / [§FS-check.2.3](functional-spec/FS-check.md#23-suggestions-channel-opt-in) / [§AR-scanner.2.4](architecture/AR-scanner.md#24-citing-side-classification): add the `[citations]` config section encoding citation directions with RFC-2119 levels — `must`/`must-not` gate `grund check` (`missing-citation` / `forbidden-citation`), `should`/`should-not` surface on a new opt-in suggestions channel (`grund check --suggestions`), and the climbing rule renders into the generated agent entrypoint with a drift check. The scanner now records declaration body ranges and each citation's source kind. An `E2E` `must` is a hard gate satisfiable by the case's `spec.refs` manifest entries ([§FS-check.3.11](functional-spec/FS-check.md#311-missing-required-citation)). PR #43.

### Changed

- [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands): the instruction-count benchmarks now scan generated fixtures instead of this repository's own tree, so the regression gate measures code cost on a stable input rather than conflating it with the repo evolving (e.g. dogfooding `[citations]`). Adds a `check_citations` benchmark (a `--citations` fixture) that exercises the direction-checking passes. PR #43.
- [§FS-distribution.3.0](functional-spec/FS-distribution.md#30-language-neutral-data-shapes) / [§FS-check.2.3](functional-spec/FS-check.md#23-suggestions-channel-opt-in): the `grund-core` `Report` gains a third `suggestions` vector and `CheckOpts` gains `include_suggestions` — a deliberate, documented break in the library surface (the CLI is unaffected). Callers that destructure `Report { errors, warnings }` must add the new field or `..`. PR #43.
- **Schema:** [§FS-init.2.3.5](functional-spec/FS-init.md#235-citation-directions) / [§FS-check.3.5](functional-spec/FS-check.md#35-invalid-agent-entrypoint-init-block): bump the managed `AGENTS.md` init block to **v3** — the hand-written climbing-rule bullet becomes a generated `### Citation directions` section derived from `[citations]`, byte-compared by `grund check` for drift. Migration: run `grund init` to refresh the block. PR #43.
- [§FS-config.2](functional-spec/FS-config.md#2-precedence) / [§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes) / [§FS-config.3.5](functional-spec/FS-config.md#35-scan--what-gets-walked) / [§FS-init.2.1](functional-spec/FS-init.md#21-files-written-updated-or-left-in-place) / [§FS-init.2.2](functional-spec/FS-init.md#22-stdout--stderr) / [§FS-init.2.4](functional-spec/FS-init.md#24-generated-agentsgrundtoml) / [§FS-id.2](functional-spec/FS-id.md#2-outputs) / [§FS-list.3.3](functional-spec/FS-list.md#33---summary): make `requirements.md` the generated default `FS` home for `grund init`, include it in the generated and built-in scan roots, and align `--docs`, `grund id`, `grund list --summary`, e2e README guidance, and next-step guidance; existing configs that omit `[[kinds]]` keep the old implicit `docs/functional-spec` home and scaffold/guidance, so no `grund_config_version` or AGENTS block version bump is required. PR #42.
- [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form) / [§FS-check.1.1](functional-spec/FS-check.md#11-recognized-citations): make `[reference] strict = true` the built-in and generated config default, leaving `strict = false` as the explicit compatibility mode for bare citations. PR #35.
- [§FS-init.2.3.4.15](functional-spec/FS-init.md#23415-workspace-members): workspace member bullets render the alias as the link label (`` - [`api`](apps/api/AGENTS.md): … ``), so the path appears once and the list shares the Project Map's `- [x](y): …` grammar. PR #39.

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
