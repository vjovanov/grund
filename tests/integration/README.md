# integration

Integration tests prove the How: that the parts fit as designed. Each one cites the §AR point whose structure it exercises — `[citations.integration]` in `grund.toml` says this home should cite `AR` — and none carries an ID, because this directory is the home of the non-citable `integration` kind ([§FS-config.3.4.1](../../docs/functional-spec/FS-config.md#341-citable--kinds-that-declare-no-ids), [§FS-config.3.4.4](../../docs/functional-spec/FS-config.md#344-the-default-kinds)). A test belongs here when its subject spans more than one part: two binaries, a crate boundary, the CI and the hook list, the tree and what the binary embeds. Black-box proof of a spec point is an e2e case under `tests/e2e/`; a claim about one module is a unit test beside it.

## Rust

The directory is also the workspace member `grund-integration-tests` (never published), so `cargo test --workspace` builds and runs these; `cargo test -p grund-integration-tests` runs them alone and builds the two binaries on demand (`binaries.rs`).

- `lsp_cli_parity.rs` — for every e2e case that is a plain `check` of a fixture with its own config, the diagnostics `grund-lsp` publishes on `initialized` are the located findings `grund check --format json` prints: the server is the same engine behind a different transport ([§AR-lsp.5](../../docs/architecture/AR-lsp.md#5-determinism-and-parity-tests)).
- `integrations_resolve.rs` — `scripts/try-integrations.sh resolve` run headlessly: the resolver installed into a sandbox HOME from the binary under test resolves every citation form the clickable clients hand it ([§FS-integrations.4](../../docs/functional-spec/FS-integrations.md#4-managed-writes---write)).

## Python

`python -m unittest discover -s tests/integration -p 'test_*.py'`, the same line CI and the pre-commit hook run.

- `test_frontend_isolation.py` — the frontends depend on the engine and on nothing of each other, read from `cargo metadata` ([§AR-bindings.1](../../docs/architecture/AR-bindings.md#1-target-workspace-layout)).
- `test_ci_precommit_parity.py` — CI runs the hook list itself, installs what the hooks need, mirrors every `commit-msg` hook, and spells the Rust commands the hooks spell ([§AR-ci.1](../../docs/architecture/AR-ci.md#1-pre-commit-is-the-source-of-truth)).
- `test_architecture_coverage_table.py` — the test-contracts table of [§AR-workspace.9](../../docs/architecture/AR-workspace.md#9-test-contracts) names cases and test functions that exist, and no architecture page points at a case that is not on disk.
- `test_asset_sync.py` — the `grund-init` skill and every template are byte-identical to the copies the binary embeds ([§FS-init.5](../../docs/functional-spec/FS-init.md#5-agent-setup-instructions)).
- `test_check_changelog_pr_entry.py`, `test_check_no_claude_attribution.py`, `test_file_size_exceptions.py` — the pull-request changelog gate, the attribution gate and the file-size budget registries ([§AR-ci.7](../../docs/architecture/AR-ci.md#7-pull-request-changelog-gate), [§AR-ci.8](../../docs/architecture/AR-ci.md#8-commit-message-attribution-gate), [§AR-ci.9](../../docs/architecture/AR-ci.md#9-file-size-budget-gate)).
- `test_prepare_changelog_release.py`, `test_generate_large_benchmark_fixture.py` — the release script and the benchmark fixture generator ([§FS-distribution.4](../../docs/functional-spec/FS-distribution.md#4-release-process), [§AR-benchmarks.1](../../docs/architecture/AR-benchmarks.md#1-what-is-benched)).

Unit tests live with the code they test (`crates/grund-core/src/tests_*.rs`) and follow `code`'s rule; the crate-level Rust suites under `crates/grund-cli/tests/` and `crates/grund-lsp/tests/` drive one binary each and cite the spec points they prove, so Cargo's own convention keeps them there.
