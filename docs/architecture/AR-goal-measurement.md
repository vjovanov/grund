# AR-goal-measurement: goal meters live outside goals

Goals say what matters; this page names where each goal is measured. Keep measurement details here, in functional specs, e2e cases, CI specs, and benchmark reports so [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) stays true for the always-read goal page.

## 1. Rule

- `docs/goals.md` owns intent and ordering principles.
- Functional specs own observable behavior.
- Architecture and CI specs own harness shape.
- E2E cases and benchmark reports own examples, baselines, and regression gates.

## 2. Goal meters

| Goal | Meter |
|---|---|
| [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) | Agent entrypoint fixtures ([§FS-init.2.3](../functional-spec/FS-init.md#23-generated-agent-entrypoints)), grounding checks ([§FS-check.3.6](../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), coverage index ([§FS-cover](../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)), and the co-change recipe ([§RM-cochange-gate](../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)). |
| [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) | `check` behavior and broken-reference fixtures ([§FS-check](../functional-spec/FS-check.md#fs-check-grund-validates-every-reference-in-a-repo)). |
| [§GOAL-polyglot-citation](../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) | Scanner doc-comment coverage ([§AR-scanner.4](AR-scanner.md#4-inline-declarations-in-language-doc-comments)) plus positive/negative host-language fixtures. |
| [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) | Instruction-counting benches ([§AR-benchmarks](AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands)), CI bench gate ([§AR-ci.5](AR-ci.md#5-benchmark-job)), catastrophic timeout guard ([§AR-ci.4](AR-ci.md#4-performance-smoke-guard)), and the local snapshot in [benchmarks.md](../benchmarks.md). |
| [§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree) | Init/config behavior ([§FS-init](../functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo), [§FS-config](../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents)) and minimal-conformant fixtures. |
| [§GOAL-multi-language](../goals.md#goal-multi-language-same-engine-three-platforms) | Distribution and binding parity specs ([§FS-distribution](../functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets), [§AR-bindings](AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms)). |
| [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) | Output/error specs ([§FS-errors](../functional-spec/FS-errors.md#fs-errors-grund-emits-messages-in-fixed-shapes), [§FS-output-shapes](../functional-spec/FS-output-shapes.md#fs-output-shapes-machine-readable-output-shapes)), CLI help specs ([§FS-cli](../functional-spec/FS-cli.md#fs-cli-grunds-command-line-surface-conventions)), and deterministic e2e fixtures. |
| [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) | Read/query specs and fixtures ([§FS-show](../functional-spec/FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id), [§FS-refs.3.3](../functional-spec/FS-refs.md#33---summary), [§FS-list.3.3](../functional-spec/FS-list.md#33---summary)); default rationale in [§DF-show-default-token-cheap](../decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in). |
| [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) | Config schema and custom-config fixtures ([§FS-config](../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents)). |
| [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) | Stable output specs, changelog discipline, deprecation fixtures, and release rules ([§FS-errors](../functional-spec/FS-errors.md#fs-errors-grund-emits-messages-in-fixed-shapes), [§FS-distribution.4](../functional-spec/FS-distribution.md#4-release-process)). |
| [§GOAL-small-and-large](../goals.md#goal-small-and-large-start-small-configure-for-big) | Tiny conformant fixtures, configured large fixtures, and the 10k-file benchmark input ([§AR-benchmarks.1](AR-benchmarks.md#1-what-is-benched)). |
