# AR-lsp: how the LSP server is built

Implements [§FS-lsp](../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server). The LSP server is a separate crate (`grund-lsp`) in the workspace defined by [§AR-bindings.1](AR-bindings.md#1-target-workspace-layout), depending only on `grund-core`. It has no shared runtime with `grund-cli`, no shared state with the bindings, and no own engine logic — everything it does delegates to `grund-core`.

## 1. Crate boundary

`grund-lsp` is a binary crate with one job: speak LSP over stdio and translate each request into a `grund-core` call. The crate has:

- No scanner, no checker, no `show` extraction, no `fmt` planning. All four are imports from `grund-core`.
- No `lsp-server`/`lsp-types` references in `grund-core`. The JSON-RPC loop and LSP protocol data types live entirely in `grund-lsp`. `grund-cli` continues to be synchronous and pulls none of this in.
- No filesystem walking outside what `grund-core::scan` already does. The LSP server does not invent its own walker.

This is the architectural shape that lets the LSP be optional ([§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary)): the dependency cost stays in `grund-lsp`, and a user installing only `grund` (the CLI) pays none of it.

## 2. State

The server holds an in-memory `LspSnapshot` per workspace. The snapshot is built by `grund-core` from the same scan/check data [§AR-scanner.3](AR-scanner.md#3-output) produces, and adds resolved declaration, section-heading, stub, citation, and link ranges for editor requests:

- On `initialize`, the server records the workspace root from the client's `rootUri`.
- During startup and again on `initialized`, the server runs a full scan and stores the resulting snapshot.
- On `textDocument/didChange`, the server updates the in-memory copy of the changed file (LSP delivers the new text), then re-runs the scan over the workspace.
- On `textDocument/didSave`, the server reconciles the in-memory copy against disk (handles cases where another tool wrote the file).
- On `textDocument/didClose`, the server drops the in-memory overlay and re-runs the scan against disk.
- On `workspace/didChangeWatchedFiles`, the server re-runs the scan to pick up creates and deletes the editor reported.

The snapshot is the cache for everything else: hover, definition, references, document links, and diagnostics all answer from it.

## 3. Scan strategy

### 3.1 Full re-scan on every change (v1)

Initial implementation: every `didChange` triggers `grund-core::scan(workspace_root)` and a fresh `grund-core::check`. This is simple and correct. Per [§GOAL-fast-feedback.1](../goals.md#1-performance-targets), a scan completes in under 100 ms on the grund repo and under 1 s on a 10k-file repo — fast enough that a full re-scan per keystroke is invisible on small and medium projects, and acceptable per-save on large ones.

### 3.2 Incremental scan (v2, when budget breaks)

When the full-scan budget breaks (typically: large monorepos, slow disks, or per-keystroke debounce too tight), switch to incremental: rescan only the changed file and re-validate citations whose targets touch the changed file's declarations. This is the same gradient [§GOAL-fast-feedback.2](../goals.md#2-how-we-get-there) endorses for the CLI's parallel walk — incremental is added when the simple version stops winning, not before.

The incremental path keeps the single source of truth in `grund-core::scan`; `grund-lsp` adds a thin "what changed" diff over scan inputs and reuses the rest.

## 4. Transport

LSP over **stdio only**. No TCP, no Unix socket, no named pipe. Reasoning: stdio is what every LSP-aware editor expects by default, has no port-conflict surface, and avoids the need for any local listener that could be reached by another process. The server is invoked by the editor's LSP client as a child process and reads/writes JSON-RPC framed messages on stdin/stdout. Diagnostic logging goes to stderr in the LSP-canonical `[LEVEL] message` form; editors that surface server logs render it as-is.

## 5. Determinism and parity tests

The LSP must produce the same diagnostics for the same workspace state as `grund check` does — byte-for-byte on the message text, position-for-position on the line numbers ([§FS-non-goals.13](../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). Current parity is enforced by keeping all engine work in `grund-core` and limiting `grund-lsp` to transport/range translation:

- `grund-core::lsp_snapshot` returns the report, declaration ranges, section-heading ranges, stub ranges, citation ranges, and resolved targets from one scan/check pass.
- `textDocument/hover` answers only on citations, calling the same `show` engine used by `grund <ID> --toc` with open-document overlays applied; declaration-side title spans (Markdown declaration headings, numbered section headings, and inline-spec stub titles) carry no hover and expose their usages through go-to-definition and references instead ([§FS-lsp.1.2](../functional-spec/FS-lsp.md#12-hover-preview)).
- `textDocument/onTypeFormatting` calls the same configured trigger/marker and ID-grammar checks as `grund fmt`.
- Focused `grund-lsp` tests cover UTF-16 range conversion, hover linkification, configured trigger punctuation, member-local trigger/marker overrides, declaration/reference matching, section-heading definition and references, whole-title stub document links, the absence of a self-pointing link on ordinary declaration titles (so the click resolves to go-to-definition usages, [§FS-lsp.1.3.2](../functional-spec/FS-lsp.md#132-document-links)), and citation document-link line fragments.

A broader child-process sweep over `e2e/cases/*` remains a useful hardening step, but it is not part of the current shipped test harness.

This is what makes the LSP "the same engine with a different transport" rather than a parallel implementation that could drift.

## 6. What this does not contain

- No editor-specific code. Per [§FS-lsp](../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server) and [§FS-non-goals](../functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do), no first-party VSCode/IntelliJ/Vim/Emacs wrappers ship; this crate is the only editor-facing surface.
- No process supervision. The editor owns the lifecycle ([§FS-lsp.2.2](../functional-spec/FS-lsp.md#22-lifecycle)); `grund-lsp` does not respawn itself, does not background, does not write a PID file.
- No telemetry, no auto-update, no crash reporter ([§FS-non-goals.11](../functional-spec/FS-non-goals.md#11-network-access-during-a-check) — no network I/O).
