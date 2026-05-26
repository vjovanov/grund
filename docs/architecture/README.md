# Architectural spec

Internals — *how* `grund` is built. One file per spec; each H1 is the declaration of an `AR-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§AR-<slug>.<section>`) resolve into these files.

An architectural spec may live inline in the class- or module-level doc-comment of the file it describes. A one-line stub here whose H1 is `# AR-<slug>: [<path>](<path>)` is **optional** — add it when you want the inline spec listed in this index alongside the file-form ones; omit it when the doc-comment alone is enough. `grund <ID>` resolves the ID either way; with a stub it follows the link and strips comment markers. See `§AR-scanner.4` for the supported doc-comment forms. `§AR-checker` is the worked example: it lives in the doc-comment of `fn check` in [`crates/grund-core/src/checker.rs`](../../crates/grund-core/src/checker.rs), with the one-line stub at [`AR-checker.md`](AR-checker.md).

| ID | Subject |
|---|---|
| [§AR-scanner](AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations) | how `grund` discovers declarations and citations |
| [§AR-checker](../../crates/grund-core/src/checker.rs) | how `grund` validates the scanner's findings — declared inline in `crates/grund-core/src/checker.rs` (stub: `AR-checker.md`) |
| [§AR-workspace](AR-workspace.md#ar-workspace-how-the-resolver-config-loader-and-scanner-compose-across-projects) | how the resolver, config loader, and scanner compose across projects |
| [§AR-core-module-layout](AR-core-module-layout.md#ar-core-module-layout-core-implementation-is-split-by-category) | how the current Rust implementation is split into category files |
| [§AR-bindings](AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms) | how the same Rust engine is exposed on three platforms |
| [§AR-lsp](AR-lsp.md#ar-lsp-how-the-lsp-server-is-built) | how the optional LSP server is built |
| [§AR-ci](AR-ci.md#ar-ci-ci-mirrors-the-local-pre-commit-gate) | how CI mirrors the local pre-commit gate |
| [§AR-goal-measurement](AR-goal-measurement.md#ar-goal-measurement-goal-meters-live-outside-goals) | where goal measurement lives outside `docs/goals.md` |
| [§AR-benchmarks](AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) | the instruction-counting benchmark harness for the [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) budgets |

This index is navigational — citations should target the spec ID directly, never this file.
