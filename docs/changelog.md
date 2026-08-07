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

## 2. [0.7.0] — 2026-08-07

### Changed

- **Schema:** [§FS-init.2.3](functional-spec/FS-init.md#23-generated-agent-entrypoints) / [§DF-managed-block-delimiters](decisions/functional/DF-managed-block-delimiters.md#df-managed-block-delimiters-standard-beginend-delimiters-for-the-managed-agent-instructions-block): the managed agent-instructions block bumps v3 → v4 — it is now bounded by explicit `<!-- BEGIN/END GRUND MANAGED BLOCK -->` delimiters, so its end no longer depends on the next heading, and broken delimiter pairs are diagnosed by `grund check` ([§FS-check.3.5](functional-spec/FS-check.md#35-invalid-agent-entrypoint-init-block)) and refused by `grund init` without rewriting the file. Migration: run `grund init` once per repo; the legacy H2-bounded block is migrated in place. PR #57.

### Fixed

- [§FS-init.2.3](functional-spec/FS-init.md#23-generated-agent-entrypoints): the generated block's worked citation example is now `<§>`-escaped, so fresh `grund init` output passes the host repo's own `grund check` unmodified instead of wedging strict repos in a check → init → check loop. PR #57.

## 3. Older releases

- [0.6.0](changelog/0.6.0.md) — 2026-07-02: - [§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server) / [§AR-lsp](architecture/AR-lsp.md#ar-lsp-how-the-lsp-server-is-built): add the optional `grund-lsp` binary with LSP diagnostics (each anchored on the offending citation token, not merely the first citation on the line), `show --toc` hover previews on citations, go-to-definition and find-references that resolve declaration titles and numbered section headings (`<ID>.<section>`) to their citation sites, document links for resolving citations, and live `$$` trigger formatting.
- [0.5.0](changelog/0.5.0.md) — 2026-06-14: - [§DF-citation-directions](decisions/functional/DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels) / [§FS-config.3.9](functional-spec/FS-config.md#39-citations--citation-direction-rules) / [§FS-check.3.11](functional-spec/FS-check.md#311-missing-required-citation) / [§FS-check.3.12](functional-spec/FS-check.md#312-forbidden-citation) / [§FS-check.2.3](functional-spec/FS-check.md#23-suggestions-channel-opt-in) / [§AR-scanner.2.4](architecture/AR-scanner.md#24-citing-side-classification): add the `[citations]` config section encoding citation directions with RFC-2119 levels — `must`/`must-not` gate `grund check` (`missing-citation` / `forbidden-citation`), `should`/`should-not` surface on a new opt-in suggestions channel (`grund check --suggestions`), and the climbing rule renders into the generated agent entrypoint with a drift check.
- [0.4.2](changelog/0.4.2.md) — 2026-06-08: - [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) / [§FS-fmt.6](functional-spec/FS-fmt.md#6-cross-reference-emission): document the measured `o200k_base` token impact of generated Markdown citation links and keep the benchmark-report generator in sync.
- [0.4.1](changelog/0.4.1.md) — 2026-05-25: - [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) / [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets): add the generated 10k-file benchmark fixture, record instruction-count baselines, and make pull-request CI fail on >5% Callgrind instruction-count regressions.
- [0.4.0](changelog/0.4.0.md) — 2026-05-19: - [§FS-inline-citation-style](functional-spec/FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations) / [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form): add configurable inline citation style enforcement for source comments.
- [0.3.0](changelog/0.3.0.md) — 2026-05-18: Default-show release.
- [0.2.0](changelog/0.2.0.md) — 2026-05-17: Workspace and agent-entrypoint release.
- [0.1.0](changelog/0.1.0.md) — 2026-05-14: first published release and baseline CLI surface.
