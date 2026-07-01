# DA-lsp-optional: LSP server ships as a separate, optional binary

**Status:** Accepted
**Date:** 2026-05-09

## 1. Context

`grund`'s editor-integration story ([§FS-lsp](../../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)) is delivered through the Language Server Protocol. There are several plausible ways to package the LSP capability into the project:

1. **Bundle into `grund-cli`.** A single `grund lsp` subcommand on the existing CLI; one binary, one install.
2. **Cargo feature flag on `grund-cli`.** `cargo install grund --features lsp` opts in; default install excludes it.
3. **Separate binary in the same crate.** `grund-cli` produces both `grund` and `grund-lsp` from one Cargo manifest.
4. **Separate crate, separate binary, separate published package.** `grund-lsp` is its own crate in the workspace; installing the LSP package is the only normal release install path for editor integration.

This decision picks among the four and pins the consequences for [§FS-distribution](../../functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) and [§AR-bindings](../../architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms).

## 2. Decision

**Option (4): separate crate, separate binary, separate published package.**

`grund-lsp` lives at `crates/grund-lsp/` in the workspace defined by [§AR-bindings.1](../../architecture/AR-bindings.md#1-target-workspace-layout), depends only on `grund-core`, and is published as a standalone package. The implemented Cargo package is `grund-lsp`; the planned npm and PyPI packages use the same name once those distribution frontends exist. The CLI install path (`cargo install grund`, and later `npm install grund-cli` / `pipx install grund`) does not transitively pull in the LSP server; users who want editor integration install it explicitly.

## 3. Why this shape

### 3.1 Dependency cost

`grund-lsp` depends on `lsp-server`, `lsp-types`, `serde_json`, and URL/protocol support crates. None of those are needed by `grund check`, `grund show`, `grund fmt`, or `grund init`, all of which are synchronous batch operations. Bundling (option 1) or feature-flagging on the CLI crate (option 2) leaks those dependencies into the resolved dependency graph in some installs (option 1) or risks accidental inclusion (option 2 — feature unification across a workspace can force `lsp` features on the CLI even when no consumer asked for them). A separate crate keeps the CLI's dependency tree small for the audience that runs `grund` in CI without ever opening an editor — the largest audience by usage volume.

### 3.2 CI binary size and start-up

CI pipelines and pre-commit hooks call `grund check` thousands of times more often than any editor opens `grund-lsp`. Keeping the CLI binary small and synchronous matters for both raw start-up cost (every invocation pays binary-load overhead) and download size (every CI cache pull moves the binary across the network). A bundled binary that includes a LSP server most users will never invoke is a tax on the common case. [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) explicitly endorses paying engineering cost to keep the CLI fast; this is part of that pattern.

### 3.3 Distribution parallel to industry practice

`rust-analyzer`, `gopls`, `pyright`, and `typescript-language-server` are all distributed as separate binaries, not subcommands of the language toolchain. This is the established expectation: editors point their LSP client at a binary named by convention, not at a subcommand of a CLI. Following the convention means contributors who already know how to wire up `rust-analyzer` know how to wire up `grund-lsp` — no new mental model.

### 3.4 Composition with [§FS-non-goals.12.1](../../functional-spec/FS-non-goals.md#121-plugins-or-scripting-hooks-inside-the-engine)

[§FS-non-goals.12.1](../../functional-spec/FS-non-goals.md#121-plugins-or-scripting-hooks-inside-the-engine) forbids a plugin system inside the engine. Keeping `grund-lsp` as a separate process that talks to the engine through a defined transport (LSP over stdio) — rather than as a feature flag that links against the engine in-process — preserves the "no plugins inside" property at the architectural level. The LSP is a *consumer* of `grund-core`, on equal footing with `grund-cli`, `grund-node`, and `grund-py`.

### 3.5 Composition with [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)

A separate package means the LSP's release cadence can decouple from the CLI's when needed. A bug fix to the LSP (say, a hover formatting change) does not require a CLI version bump. Conversely, an engine surface change that breaks the LSP's assumptions surfaces as a build break in the LSP crate before release, not as a runtime mismatch in editors. The compile-time link from `grund-lsp` to a pinned `grund-core` version is what enforces this.

## 4. Consequences

- The workspace gains a third crate: `crates/grund-lsp/`. [§AR-bindings.1](../../architecture/AR-bindings.md#1-target-workspace-layout) is updated to list it alongside `grund-core`, `grund-cli`, `grund-node`, and `grund-py`.
- The Cargo package `grund-lsp` is published independently from the CLI. The npm and PyPI packages remain planned targets in [§FS-distribution.1](../../functional-spec/FS-distribution.md#1-targets).
- [§FS-lsp.2.1](../../functional-spec/FS-lsp.md#21-install) ("Install") states that the CLI install does not pull in `grund-lsp` transitively; the inverse is also true (`grund-lsp` does not pull in the CLI binary).
- The roadmap item [§RM-lsp](../../roadmap.md#rm-lsp-ship-the-optional-lsp-server) owns shipping the crate and Cargo package. The remaining npm/PyPI packaging rides with [§RM-distribution](../../roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine).
- [§FS-non-goals](../../functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) adds an explicit entry: first-party per-editor plugins (VSCode/IntelliJ/Vim/Emacs wrappers) are out of scope. The LSP server is the only editor surface; editor configuration is the user's one-time work.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| **(1) Bundle into `grund-cli` as a `grund lsp` subcommand.** Single binary, single install; the simplest install story. | Forces every CLI install — including CI — to carry the LSP's transitive dependencies (`lsp-server`, `lsp-types`, JSON-RPC/protocol support). Increases binary size and slows start-up. The largest audience pays for a feature it never invokes. |
| **(2) Cargo feature on `grund-cli`.** `cargo install grund --features lsp` opts in; default install excludes the LSP. Avoids the dependency cost for the default case. | Cargo's feature unification can force LSP features on the CLI when a consumer in the same workspace requests them — a transitive `grund-cli` dependency with `features = ["lsp"]` would impose them on every consumer of `grund-cli`. Not airtight isolation. Also loses the per-binary version cadence in §3.5. |
| **(3) Separate binary in the same crate.** One Cargo manifest, two `[[bin]]` entries (`grund` and `grund-lsp`). Avoids a new crate. | The binaries would share a Cargo manifest, which means they share a dependency declaration — the LSP protocol crates would be present in the manifest even if optional. `cargo install grund` would still need to compile the dependency graph including the LSP's deps to produce only the CLI binary. Separating crates is the one mechanism Cargo provides for fully partitioning compiled artifacts. |
| **(4) — chosen** | See §2 and §3 above. |
