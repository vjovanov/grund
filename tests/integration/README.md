# integration

Integration tests prove the How: that the parts fit as designed. Each one cites the §AR point whose structure it exercises — `[citations.integration]` in `grund.toml` says this home should cite `AR` — and none carries an ID, because this directory is the home of the non-citable `integration` kind ([§FS-config.3.4.1](../../docs/functional-spec/FS-config.md#341-citable--kinds-that-declare-no-ids), [§FS-config.3.4.4](../../docs/functional-spec/FS-config.md#344-the-default-kinds)).

Cargo puts a crate's integration tests in that crate's own `tests/` directory, so grund's live beside the crate they drive rather than here:

- `crates/grund-cli/tests/` — the CLI frontend of [§AR-bindings](../../docs/architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms): `init*.rs`, `fmt_scope_equivalence.rs`, `index_entry_round_trip.rs`, and `e2e.rs`, the harness that runs the corpus under `tests/e2e/`.
- `crates/grund-lsp/tests/` — the server of [§AR-lsp](../../docs/architecture/AR-lsp.md#ar-lsp-how-the-lsp-server-is-built) over its real stdio transport.

Unit tests live with the code they test (`crates/grund-core/src/tests_*.rs`) and follow `code`'s rule; the Python tests for `scripts/` sit one level up at `tests/test_*.py`. A test that fits no single crate — one that drives more than one binary, or the repository's own tooling — belongs in this directory.
