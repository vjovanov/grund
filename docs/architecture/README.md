# Architectural spec

Internals — *how* `grund` is built. One file per spec; each H1 is the declaration of an `AR-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§AR-<slug>.<section>`) resolve into these files.

An architectural spec may live inline in the class- or module-level doc-comment of the file it describes. Its canonical link in this index enrolls it directly, with no otherwise-empty stub file required ([§FS-check.4.6](../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index)); a legacy or deliberately navigational one-line stub whose H1 is `# AR-<slug>: [<path>](<path>)` remains valid. `grund <ID>` resolves the source declaration either way and strips its comment markers. See `§AR-scanner.4` for the supported doc-comment forms. `§AR-checker` is the worked example: its only declaration lives in the doc-comment of `fn check` in [`crates/grund-core/src/checker.rs`](../../crates/grund-core/src/checker.rs), and the canonical index row below enrolls it here.

| ID | Subject |
|---|---|
| [§AR-scanner](AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations) | how grund discovers declarations and citations |
| [§AR-checker](../../crates/grund-core/src/checker.rs) | how grund validates the scanner's findings — declared and enrolled directly from `crates/grund-core/src/checker.rs` |
| [§AR-workspace](AR-workspace.md#ar-workspace-how-the-resolver-config-loader-and-scanner-compose-across-projects) | how the resolver, config loader, and scanner compose across projects |
| [§AR-core-module-layout](AR-core-module-layout.md#ar-core-module-layout-core-implementation-is-split-by-category) | core implementation is split by category |
| [§AR-bindings](AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms) | target shape for exposing the Rust engine on three platforms |
| [§AR-lsp](AR-lsp.md#ar-lsp-how-the-lsp-server-is-built) | how the LSP server is built |
| [§AR-ci](AR-ci.md#ar-ci-ci-mirrors-the-local-pre-commit-gate) | CI mirrors the local pre-commit gate |
| [§AR-goal-measurement](AR-goal-measurement.md#ar-goal-measurement-goal-and-requirement-meters-live-outside-goals) | goal and requirement meters live outside goals |
| [§AR-benchmarks](AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) | instruction-counting benchmarks for the hot CLI commands |

This index is navigational — citations should target the spec ID directly, never this file.
