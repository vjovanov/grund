# FS-lsp: grund ships an optional LSP server

`grund` ships an optional Language Server Protocol server, `grund-lsp`, as a separate binary that any LSP-aware editor can talk to: VSCode, Neovim, Emacs (eglot or lsp-mode), Helix, Zed, Sublime Text, and the IntelliJ family via LSP4IJ. Users who want editor integration install `grund-lsp` and configure their editor once; users who do not — CI pipelines, pre-commit hooks, contributors who only run `grund check` — install nothing extra and pay no dependency cost. The architectural choice (separate binary rather than a Cargo feature or a bundled library) is decided in [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). The implemented Cargo crate and release path are tracked by [§RM-lsp](../roadmap.md#rm-lsp-ship-the-optional-lsp-server); npm and PyPI packaging remain part of [§RM-distribution](../roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine).

`grund` does not ship per-editor wrappers. The only first-party editor surface is the LSP server; per-editor configuration is one-time work the user does, with example snippets in the user-facing LSP setup guide. See [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) for the non-goal that pins this.

## 1. Capabilities

The minimum viable set — everything the server speaks at version 1.0. Diagnostics (§1.1), hover (§1.2), and go-to-definition (§1.3) are illustrated in the project README; each illustration is captured for both light and dark editor themes (paired `<name>-light.png` and `<name>.png` `prefers-color-scheme` sources), so a screenshot refresh updates both variants together.

A single malformed or failing message is not fatal to the session: a request that fails returns an LSP error response, and a notification whose params do not parse is logged and skipped. One bad message from a client cannot drop the connection mid-session — the server keeps serving subsequent messages.

### 1.1 Diagnostics

`textDocument/publishDiagnostics` pushes `grund check` results as the user edits. Each unknown reference, missing section, duplicate declaration, broken stub, and citation-direction violation — a required citation absent ([§FS-check.3.11](FS-check.md#311-missing-required-citation)) or a forbidden one present ([§FS-check.3.12](FS-check.md#312-forbidden-citation)) — becomes a diagnostic with the same `path:line: <message>` content the CLI prints to stdout ([§FS-errors.2.1](FS-errors.md#21-located-finding)). The advisory `should` / `should-not` suggestions channel ([§FS-check.2.3](FS-check.md#23-suggestions-channel-opt-in)) is opt-in on the CLI and is not pushed as diagnostics. Severity follows the engine's severity model ([§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization) — not configurable). The diagnostic position is the start column of the citation the finding concerns — the offending token, not merely the first citation on the line. A single comment can carry several citations, so each finding anchors to its own token. Diagnostics that are line-anchored rather than citation-anchored, such as the opt-in ungrounded-file check ([§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in)), do not borrow a citation range from the same line; otherwise VSCode-style diagnostic hovers would stack unrelated line-level messages onto the citation's own error. Declaration-side findings may still use the declaration, section, or stub title span on that line. For example, with

```
// §FS-check.3.9 / §FS-confg.3.3
//   └ resolves     └ unknown reference (a mistyped FS-config)
```

the `unknown reference FS-confg` diagnostic anchors on the second token; the resolving first citation `FS-check.3.9` is left unmarked. Precise column information is computed once per scan and reused across the open editor session.

### 1.2 Hover preview

`textDocument/hover` on a citation returns the body `grund <ID> --toc` would print ([§FS-show.2.1.2](FS-show.md#212-section-map---toc)), or the `--toc` body of the requested section if the citation includes one ([§FS-show.2.2](FS-show.md#22-section)). When the declaration's home is in source code (a stub points at `src/bus.rs`), the hover body is the comment-stripped prose per [§FS-show.2.3.2](FS-show.md#232-stripping-comment-markers) — the same content the CLI returns. There is no separate "IDE-only" rendering for resolving citations; citation hover and the `show --toc` query produce the same bytes. If that citation has a diagnostic instead (for example an unknown reference with a nearest-ID hint), hover returns nothing: the diagnostic already carries the actionable text — the nearest-ID hint — through `publishDiagnostics`, and an editor that renders diagnostics inside the hover popup (VSCode among them) would otherwise show that text twice. The diagnostic is the single source of the error message; hover stays reserved for previewing citations that resolve.

`textDocument/hover` on a declaration-side title — a Markdown declaration heading, a numbered section heading (`<ID>.<section>`, §1.3.1), or an inline-spec stub title — returns only the title token as Markdown and sets the hover range to the whole title span. The cursor is already on the declaration body, so there is no body preview to show; the title hover exists to give editors such as Codium a whole-title range for the hover affordance. The citation sites that use the title are still reached on demand through go-to-definition (§1.3) and references (§1.3.1).

The citation hover content is Markdown. Any resolving `§<ID>` citation inside that hover body is emitted as a normal link to its declaration target, so users can keep following the grounding graph without closing the hover.

### 1.3 Go-to-definition

`textDocument/definition` on a citation jumps to the declaration's `path:line`. For a stub-and-inline-source pair ([§FS-check.3.4](FS-check.md#34-broken-inline-spec-stub)), the server follows the stub's link and lands on the inline declaration line directly — the user does not stop at the stub. The same is true when definition is invoked anywhere on the stub heading's ID or title text: `# AR-foo: [src/lib.rs](...)` is one navigable title span that jumps to the inline `AR-foo` declaration in the source doc-comment. Normal Markdown declaration headings use the same whole-title span for declaration-side requests: references and definition-as-usages return citations of that ID. A numbered section heading inside a declaration body — `## 1. …`, addressable as `<ID>.<section>` — is itself a declaration-side title with the same behaviour: definition and references on it return the citations of that section (`§<ID>.<section>` and any deeper subsection), scoped to the section rather than the whole ID. Definition results report the whole originating token as their origin span — the entire `§<ID>` citation or declaration, section, or stub title — so the editor underlines that span as one navigable unit on the link gesture rather than only the word under the cursor. The server sends `LocationLink` results unless a client explicitly sets `textDocument.definition.linkSupport = false`; clients that omit the flag still get the richer range-carrying shape because plain `Location` results cannot express the origin span.

#### 1.3.1 References from declarations

`textDocument/references` on a declaration ID or anywhere in its Markdown title returns every citation of that ID, including citations in scanned source-code comments, the same set `grund refs <ID>` reports ([§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id)). On a numbered section heading it returns the citations of that section ID (`§<ID>.<section>` and deeper), the section-scoped counterpart of the whole-ID title. The same request on a citation returns that citation's target ID usages too, so editors can show "find usages" from either side of the relationship.

#### 1.3.2 Document links

`textDocument/documentLink` marks each resolving citation token as a link to its declaration target, and each inline-spec stub title as a link to the source doc-comment declaration it points at. Editors that render LSP document links therefore show `§<ID>` references and stub titles as single navigable units even in source files where Markdown cross-reference emission cannot run ([§FS-fmt.6.1](FS-fmt.md#61-scope)). Ordinary Markdown declaration titles are deliberately **not** document links: a self-pointing link covers the same span the editor's Ctrl-click would otherwise resolve to go-to-definition, so it would shadow that gesture and navigate the title onto its own line instead of showing the declaration's usages. The title stays navigable through go-to-definition, which returns the citation sites (§1.3, §1.3.1). Numbered section headings are likewise not document links and stay reachable through the same declaration-side go-to-definition and references.

The link target is a file URI with a line fragment (`#L<n>`) for the resolved
declaration or source line. Editors that ignore file-URI fragments may open the
linked file without moving the cursor to the line; `textDocument/definition`
remains the exact-position fallback for those clients.

#### 1.3.3 Occurrence highlight

`textDocument/documentHighlight` marks the whole `§<ID>` citation — or declaration, section, or stub title — under the cursor as one span, plus every other occurrence of the same ID in the open document. Without this, an editor that has no highlight provider falls back to its language word pattern when the cursor rests in a token, and a marker-and-punctuation citation such as `§FS-lsp.1.3` is split on `§`, `-`, and `.`, so only the bare word at the cursor (`lsp`) is boxed rather than the whole reference. Returning the token span as a document highlight makes editors mark the entire citation as a unit. Highlights are scoped to the document the request names: the citing token plus same-document declarations, section headings, stub titles, and sibling citations of the same ID; usages in other files are reached through references (§1.3.1).

### 1.4 Live trigger transform

`textDocument/onTypeFormatting` watches the configured trigger sequence (default `$$`, per [§DF-reference-marker.2.2](../decisions/functional/DF-reference-marker.md#22-trigger)) and replaces it with the marker (default `§`) the moment the trigger is followed by a token matching the repo's `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar) — `FS-007` under a numbered format, `FS-login` under the slug-only form). This is the live counterpart to `grund fmt`'s bulk trigger pass ([§FS-fmt.2.1](FS-fmt.md#21-trigger-to-marker)) and is what makes the marker practical to type without leaving the keyboard.

The trigger, marker, and recognized `KIND` set are read from the discovered `grund.toml` so the editor experience matches the project's choices. In a workspace, the replacement uses the config resolved for the edited document, so member-local trigger and marker overrides behave the same as `grund fmt` ([§FS-workspace.5](FS-workspace.md#5-command-scope)). If no config is present, the defaults from [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger) and [§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up) apply.

### 1.5 Capabilities reserved for later

These are out of scope for the first version but compatible with the architecture:

- `textDocument/completion` — autocomplete `§F` to declared `FS-…` IDs from the workspace.
- `textDocument/codeAction` — quick fixes for "unknown reference" (suggest similarly-named IDs) and "section not found" (suggest sibling sections).
- `workspace/symbol` — fuzzy-find IDs across the project.

Each addition is a separate roadmap item if and when it is taken on.

## 2. Installation and lifecycle

### 2.1 Install

`grund-lsp` is a separate Cargo package per [§FS-distribution](FS-distribution.md#fs-distribution-grund-distribution-targets): after a release, users install it with `cargo install grund-lsp`; from a checkout, contributors install it with `cargo install --path crates/grund-lsp`. It is not pulled in by `cargo install grund`. The npm and PyPI `grund-lsp` packages are future distribution targets, so `npm install -g grund-lsp` and `pipx install grund-lsp` are not documented as available until those frontends exist. A user with no editor integration installs the CLI alone.

### 2.2 Lifecycle

Users do not run `grund-lsp` directly. The editor's LSP client spawns it as a child process when a relevant file (markdown or any extension in the configured `[scan] extensions`) is opened in a workspace containing a `grund.toml` (in either discovery location) or `AGENTS.md`, and kills it when the workspace closes. The server speaks LSP over stdio; there is no daemon, no socket, no background service. CI pipelines that happen to have `grund-lsp` installed never invoke it — the only entry point in batch contexts is the CLI.

### 2.3 Editor configuration (one-time, per editor)

The user-facing LSP setup guide ships example LSP-client snippets for the editors most contributors use:

- **Helix** — three lines in `languages.toml`.
- **Neovim** — a built-in LSP snippet, compatible with `nvim-lspconfig`-based setups.
- **Zed** — central LSP registry entry; one config block locally if not yet upstreamed.
- **Emacs** — `eglot-server-programs` or `lsp-mode` registration (~5 lines).
- **VSCode** — install a generic LSP client extension and point it at `grund-lsp`. A first-party VSCode extension is **not** shipped ([§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do)).
- **Sublime Text** — LSP package client configuration for Markdown and scanned source syntaxes.
- **IntelliJ family** — LSP4IJ plugin with a `grund-lsp` server registration.

Adding a new editor's snippet to the user-facing guide is a small contribution; it does not require a release.

## 3. Configuration

The server reads the `grund.toml` via the same discovery logic as `grund check` ([§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up)), walking up from the workspace root supplied by the editor's LSP `initialize` request. There is no separate LSP config; one source of truth drives both the CLI and the LSP. A workspace with no config under either name falls back to the canonical defaults ([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)).

Editor-side LSP configuration (server arguments, workspace folders) is the user's responsibility per §2.3 and is not part of `grund.toml`.

## 4. Determinism and parity with the CLI

Same input + same config → same diagnostics, same hover body, same definition target, byte-for-byte ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). The implementation enforces this by routing LSP state through `grund-core` snapshot, check, show, refs, and formatting APIs, plus focused LSP tests for linkification, configured trigger handling, workspace member marker resolution, UTF-16 ranges, and document-link targets. A full child-process sweep over `e2e/cases/*` is future hardening, not a current shipped harness.

The LSP server does not have an "interactive" mode, a confirmation prompt, or any user-visible state that the CLI lacks ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)). It is the same engine with a different transport.

## 5. Out of scope

- **Per-editor wrappers**: VSCode/IntelliJ/Vim/Emacs first-party plugins are not shipped ([§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do)). The LSP server is the surface; editor configuration is the user's.
- **Refactoring (rename ID)**: `grund` does not rename IDs; the scheme says IDs are forever ([§FS-non-goals.4](FS-non-goals.md#4-cross-workspace-id-renaming)).
- **Inline editing of declaration bodies from the hover popup**: editors already do this well; `grund-lsp` does not implement it.
- **Network access**: the server performs no network I/O ([§FS-non-goals.11](FS-non-goals.md#11-network-access-during-a-check)). All scanning is local.
