# AR-lsp: how the LSP server is built

Implements [§FS-lsp](../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server). The LSP server is a separate crate (`grund-lsp`) in the workspace defined by [§AR-bindings.1](AR-bindings.md#1-target-workspace-layout), depending only on `grund-core`. It has no shared runtime with `grund-cli`, no shared state with the bindings, and no own engine logic — everything it does delegates to `grund-core`.

## 1. Crate boundary

`grund-lsp` is a binary crate with one job: speak LSP over stdio and translate each request into a `grund-core` call. The crate has:

- No scanner, no checker, no `show` extraction, no `fmt` planning. All four are imports from `grund-core`.
- No `lsp-server`/`lsp-types` references in `grund-core`. The JSON-RPC loop and LSP protocol data types live entirely in `grund-lsp`. `grund-cli` continues to be synchronous and pulls none of this in.
- No filesystem walking outside what `grund-core::scan` already does. The LSP server does not invent its own walker.

This is the architectural shape that lets the LSP be optional ([§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary)): the dependency cost stays in `grund-lsp`, and a user installing only `grund` (the CLI) pays none of it.

## 2. State

The server holds one in-memory `LspSnapshot` per discovered Grund project. The snapshot is built by `grund-core` from the same scan/check data [§AR-scanner.3](AR-scanner.md#3-output) produces, and adds resolved declaration, section-heading, stub, citation, and link ranges for editor requests:

- On `initialize`, the server records every `workspaceFolders` URI (falling back to `rootUri`, then the process current directory), discovers each folder's enclosing config, and deduplicates folders that resolve to the same project root ([§FS-lsp.2.2](../functional-spec/FS-lsp.md#22-lifecycle)). A folder with no config keeps itself as its project root rather than inheriting the server process's current directory. A folder with a non-`file:` URI, or one whose config will not load, is skipped with a note on stderr instead of ending the session.
- During startup and again on `initialized`, the server runs a full scan for every distinct project root and stores the resulting snapshots. Each project is built on its own: one that fails keeps its last good snapshot and the others still refresh, so a single unreadable config cannot freeze the session's diagnostics.
- On `textDocument/didChange`, the server updates the in-memory copy of the changed file (LSP delivers the new text), then re-runs the scan for every project that can see that file. A project whose scan never reads the document cannot have changed verdicts, so it is left standing — the full-scan-per-edit cost stays a function of the edited project, not of how many folders the editor happens to have open. A document *no* project claims rebuilds everything: that is what a file newly created under an external include root looks like, and treating it as nobody's would leave the editor silent on a file the CLI checks.
- On `textDocument/didSave`, the server reconciles the in-memory copy against disk (handles cases where another tool wrote the file).
- On `textDocument/didClose`, the server drops the in-memory overlay and re-runs the scan against disk.
- On `workspace/didChangeWatchedFiles`, the server re-runs every project's scan to pick up creates and deletes the editor reported — a path no project has read yet belongs to none of them, so this one cannot be narrowed to the changed document.
- On `workspace/didChangeWorkspaceFolders`, the server updates the recorded folder set, rediscovers and deduplicates project roots, rebuilds their snapshots, and republishes diagnostics. Discovery is recomputed from the remaining folder anchors so removing one of two folders for the same project does not remove the project.

The snapshots are the cache for everything else: hover, definition, references, document links, and diagnostics all answer from the request document's project snapshot. Independent projects are not merged, because identical local IDs in two editor folders are unrelated namespaces.

Which snapshot that is comes from one resolution used by every surface, requests and diagnostics alike, so an editor cannot navigate against one project's view while reading another's errors. A project whose root contains the document owns it, the deepest root winning among nested trees; failing that, a project whose scan reached the document owns it. The second half is why `LspSnapshot` carries the set of files its scan read: a project's reach is not bounded by its root — a symlinked `[scan] include` canonicalizes outside it, and an include path may be parent-relative — so a root prefix alone would answer "not mine" for files the CLI checks, and the editor would go quiet on exactly the citations [§FS-lsp.1](../functional-spec/FS-lsp.md#1-capabilities) exists to surface. Diagnostics are then published by the owner alone; the same finding reaching two snapshots is reported once.

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
- `textDocument/hover` previews citation bodies by calling the same `show` engine used by `grund <ID> --toc` with open-document overlays applied. Declaration-side title spans (Markdown declaration headings, numbered section headings, and inline-spec stub titles) return the whole-title range plus the title's usage counts ([§FS-lsp.1.2](../functional-spec/FS-lsp.md#12-hover-preview)), so editors can underline the complete title while the sites themselves stay behind go-to-definition and references.
- Those counts are read from the snapshot, not from a fresh `refs` query. `grund_core::refs` is the CLI's entry point and loads its own workspace context — one full scan per call — so calling it per hover would re-walk the tree on a keystroke-adjacent request and break [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible). The snapshot is already built from the same scan `refs` and `check` run (§2), so counting its citations *is* reusing the refs machinery, one scan earlier. What is shared is the rule rather than the walk: `grund_core::citation_under_title` is the single definition of which citations belong to a declaration-side title, and `LspSnapshot::title_usage` and `LspSnapshot::title_citations` are the count and the list built from it, so the hover number and the `textDocument/references` result cannot drift apart ([§FS-lsp.1.3.1](../functional-spec/FS-lsp.md#131-references-from-declarations)). Parity with the CLI is held by a test that runs `grund_core::refs` over the same tree and compares.
- The hover body bytes are formed in `grund_core::lsp_title_hover_body`, beside the counts rather than in the transport, so the singular/plural and zero wording [§FS-lsp.1.2](../functional-spec/FS-lsp.md#12-hover-preview) fixes is unit-testable without a server process and cannot be re-worded by a second frontend.
- `textDocument/onTypeFormatting` calls the same configured trigger/marker and ID-grammar checks as `grund fmt`.
- Focused `grund-lsp` tests cover UTF-16 range conversion, hover linkification, configured trigger punctuation, member-local trigger/marker overrides, section-heading definition and references, declaration-title usage counts end to end (with the citation hover left as the `--toc` body), whole-title stub document links, the absence of a self-pointing link on ordinary declaration titles (so the click resolves to go-to-definition usages, [§FS-lsp.1.3.2](../functional-spec/FS-lsp.md#132-document-links)), whole-token occurrence highlight ([§FS-lsp.1.3.3](../functional-spec/FS-lsp.md#133-occurrence-highlight)), and citation document-link line fragments. The declaration/citation matcher itself is no longer unit-tested in this crate: it moved to `grund-core` with `citation_under_title`, and its cases live beside the counts they feed in `crates/grund-core/src/tests_lsp_hover.rs`. This crate still covers it end to end, over a real server, in the navigation case.

The broader child-process sweep is `tests/integration/lsp_cli_parity.rs`: for every e2e case that is a plain `check` of a fixture carrying its own config, the diagnostics the server publishes on `initialized` are compared with the located findings `grund check --format json` prints for the same tree — path, line, code, severity and message — and the number of compared cases has a floor, so the sweep cannot shrink unnoticed. Cases the CLI refuses and fixtures with no config at their root are counted, never silently skipped.

This is what makes the LSP "the same engine with a different transport" rather than a parallel implementation that could drift.

## 6. What this does not contain

- No editor-specific code. Per [§FS-lsp](../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server) and [§FS-non-goals](../functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do), no first-party VSCode/IntelliJ/Vim/Emacs wrappers ship; this crate is the only editor-facing surface.
- No process supervision. The editor owns the lifecycle ([§FS-lsp.2.2](../functional-spec/FS-lsp.md#22-lifecycle)); `grund-lsp` does not respawn itself, does not background, does not write a PID file.
- No telemetry, no auto-update, no crash reporter ([§FS-non-goals.11](../functional-spec/FS-non-goals.md#11-network-access-during-a-check) — no network I/O).
