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

`textDocument/hover` on a declaration-side title — a Markdown declaration heading, the same declaration written inline in a doc-comment, a numbered section heading (`<ID>.<section>`, §1.3.1), or an inline-spec stub title — returns the title token and how much of the tree leans on it, with the hover range set to the whole title span. The cursor is already inside the declaration body, so a body preview would only repeat what is on screen; the *usage* is the one fact about a declaration that is visible nowhere on that screen, and reading it used to mean leaving the editor for `grund refs <ID>`. The title hover keeps its original job of giving editors such as Codium a whole-title range for the hover affordance, and the citation sites themselves are still reached on demand through go-to-definition (§1.3) and references (§1.3.1): the hover is the count, not the list.

The body is one line of Markdown — the title token as inline code, then ` — `, then the usage clause:

```
`FS-user-login: Users sign in` — cited at 12 sites across 5 files
```

The token is the title exactly as the document writes it, which on a numbered section heading is the heading text alone — `1. Capabilities`, not a composed `FS-lsp.1: Capabilities`: the popup is drawn over that very heading, so the declaration it belongs to is already on the reader's screen, and a hover that echoed a string written nowhere in the file would be one more place for the file and the editor to disagree. A title that itself contains backticks — many section headings do — is fenced with a backtick run one longer than the longest run inside it, padded with a single space at each end when the title starts or ends with a backtick, because a backslash does not escape a backtick inside a code span and a raw one would close the span early.

The clause is `cited at <n> site(s) across <m> file(s)`, where `site` and `file` take a plural `s` at every count except one, so a lone citation reads `cited at 1 site across 1 file`. Nothing else varies with the numbers — `across` is the preposition at every count, and there is no separate one-citation phrasing — because a hover is skimmed rather than read, and a line whose shape never changes is one where only the digits need looking at. A title with no citations reads `not cited` in place of the entire clause: `cited at 0 sites across 0 files` is a sentence pretending to be a count, and the reader has to parse it to learn a fact two words state.

`<n>` counts citation **sites** and `<m>` the distinct files those sites live in, over exactly the set §1.3.1 returns for that same title — so the number a reader sees and the reference list they open next from the same token can never disagree. For a whole-ID title — a Markdown declaration heading, its doc-comment form, or the stub that points at it — that set is *by definition* the one `grund refs` reports **from the same root the server was started at** ([§FS-refs.2](FS-refs.md#2-behaviour)): citations inside scanned source comments included, `[reference] strict` honoured. Naming the root is not pedantry in a workspace, where the two roots ask different questions: a server rooted at the workspace root counts what `grund refs <alias>/<ID>` reports from there — a member's own `§<ID>` and a sibling's `§<alias>/<ID>` together ([§FS-workspace.8.2](FS-workspace.md#82-grund-refs)) — while a server rooted at the member itself counts what `grund refs <ID>` reports from inside the member, where the sibling's qualified citation lies outside the tree. These are the CLI's numbers rendered in an editor, not a second tally kept beside them (§4).

On a numbered section heading the set is the section-scoped one §1.3.1 already defines: `§<ID>.<section>` and its deeper subsections. A section's blast radius includes its children — the same subtree `grund <ID>.<section> --full` prints ([§FS-show.2.2](FS-show.md#22-section)), and the same set that heading's own definition and references return — so the count is the total of what the reader can navigate to from the very token they are hovering. That is deliberately wider than `grund refs <ID> --section <s>`, which keeps only citations whose section coordinate is *exactly* `<s>` ([§FS-refs.1](FS-refs.md#1-inputs)): the flag answers "who cites this section itself", a declaration-side title answers "who leans on this", and only the whole-ID form is a `refs` invocation counted byte for byte. Where the two readings conflict, the one that keeps a title's hover and its reference list in agreement wins — those are one gesture apart in the same editor, while the terminal comparison is one the user has to go looking for.

The clause is a count, never a finding. An uncited declaration already earns the unused-declaration warning through `publishDiagnostics` (§1.1, [§FS-check.4.1](FS-check.md#41-unused-declaration)), and hover does not restate it: `not cited` is the count at zero, worded as a count, so an editor that renders diagnostics inside the hover popup shows that warning once rather than twice. Where an editor draws both into one popup over an uncited title, that popup carries the warning naming the ID and the count answering the hover — one statement each, not the same sentence twice. Nor is the zero case suppressed in favour of the warning, because the warning does not cover every title that can reach zero — `E2E` declarations are exempt from it ([§FS-check.4.1](FS-check.md#41-unused-declaration)) and section headings never carry one — so a hover that fell silent at zero would go quiet exactly where nothing else speaks, and "no counts shown" is indistinguishable from "this server does not show counts". Determinism is the server's usual promise (§4): the counts are read from the session snapshot, one scan of the workspace shared with diagnostics and navigation, so the same tree and config produce the same bytes and no hover re-scans to answer.

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

Where the typed token is a number-only shorthand ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)) that resolves to exactly one declaration, the server also **expands it**: typing `$$FS-042` leaves `§FS-042-user-login` behind, not a `§FS-042` followed by a diagnostic telling the author to finish the job. This is the live counterpart to [§FS-fmt.2.4](FS-fmt.md#24-shorthand-to-canonical) and the reason the shorthand is worth recognizing at all — it is authoring sugar, so the authoring surface is where it pays ([§DF-number-only-citation-shorthand.2.2](../decisions/functional/DF-number-only-citation-shorthand.md#22-where-the-shorthand-is-accepted-and-where-it-is-an-error)).

The two rewrites fire on **different keystrokes**, and that separation is what makes the expansion correct rather than merely eager. The trigger becomes rewritable the moment the text after `$$` first reads as an ID — under the default format that is the *first digit*, because `FS-0` is already a well-formed shorthand — and converting there is harmless, since it replaces only the `$$` and the author types straight through it. Expanding there is not: the number is unfinished, so `$$FS-12` would be rewritten to whatever `FS-1` happens to name with the remaining digits left trailing behind it. The expansion therefore waits for the keystroke that **ends** the token — a character that cannot continue an ID, which is the same boundary [§FS-check.1.2](FS-check.md#12-the-number-only-shorthand) uses — at which point the typed number is known to be complete. A period is such a character, and the expansion preserves it, so `$$FS-042.1` still lands on `§FS-042-user-login.1`.

The expansion reads the declaration set from the session snapshot the server already maintains, never a fresh scan, so the per-keystroke path stays within [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible). In a workspace the snapshot holds every member's declarations, and only the edited file's own project is consulted: `§FS-042` typed in one member means that member's `FS-042-…` and never a sibling's ([§FS-workspace.5](FS-workspace.md#5-command-scope)). A shorthand that matches no declaration, or more than one, converts the trigger and nothing more: typing never stalls, and the resulting `§FS-042` earns the [§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation) diagnostic that names the problem. The same is true of a token the author never terminates — `grund check` and `grund fmt` are the backstop, and they agree with the editor about what resolves.

Because the expansion can rewrite a citation the author did not just type, it honours every context `grund fmt` refuses ([§FS-fmt.2.3](FS-fmt.md#23-what-is-never-rewritten)) — including the whole-line skips a single line cannot reveal, a fenced code block and a declaration heading. The server therefore hands the core the document, not just the edited line: an editor that silently canonicalized an illustration inside a fence would be doing something no other surface does.

A shorthand citation already in the document is a citation like any other for every other capability — hover (§1.2), go-to-definition (§1.3), references (§1.3.1), document links (§1.3.2), and occurrence highlight (§1.3.3) all treat `§FS-042` as the declaration it resolves to, and the range they return is the written token, not the canonical one it stands for.

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

The folders in an LSP `initialize` request are config-discovery anchors, not scan boundaries. For every `workspaceFolders` entry, the server walks upward with the same discovery rules as the CLI (§3); when it finds a Grund config, it snapshots that config's project root so configured `[scan] include` paths and sibling source trees remain visible even when the editor opened only a nested directory. Entries that discover the same project root share one snapshot. Entries that discover different roots each get a snapshot. If `workspaceFolders` is absent or empty, the deprecated `rootUri` is the anchor, then the server process's current directory as the final fallback. With no discovered config, the anchor itself remains the zero-config scan root.

Each document is answered by at most one of those projects. A project whose root contains the document claims it — the deepest such root when project trees nest, so a member opened as its own folder answers for its own files. A project that merely *scans* the document claims it when no root contains it: `[scan] include` is a scan scope, not a fence ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)), so a symlinked or parent-relative include root reaches outside the project directory, and a document there is checked by the CLI and must stay answerable in the editor. If multiple projects only reach the same external document through their scans, none owns it: choosing by folder order or root shape would guess between independent namespaces, so requests return no result and neither project's diagnostics are published for that file ([§REQ-no-wrong-citation](../requirements/REQ-no-wrong-citation.md#req-no-wrong-citation-a-citation-never-resolves-to-a-guess)). One owner per document is also what keeps diagnostics honest: where nested projects both read a file, the containing owner's verdict is published alone rather than merged with the other's, so a file cannot collect the same finding twice or two projects' disagreeing readings of one citation side by side.

Independent projects are never merged. Identical local IDs in two editor folders are unrelated namespaces, and a reference answered from the wrong one would be a wrong citation ([§REQ-no-wrong-citation](../requirements/REQ-no-wrong-citation.md#req-no-wrong-citation-a-citation-never-resolves-to-a-guess)).

A folder the server cannot turn into a project is skipped, never fatal ([§REQ-never-crashes](../requirements/REQ-never-crashes.md#req-never-crashes-garbage-in-diagnostic-out)). A folder URI with a non-`file:` scheme — editors mix virtual and remote folders into one window — is passed over with a note on stderr. So is a folder whose config will not load: a half-typed `grund.toml` in one folder reports itself and leaves that project on its last good snapshot, while every other folder in the session keeps its diagnostics current. A session with no usable folder left still starts and answers nothing, rather than exiting.

The server advertises workspace-folder support with change notifications. On `workspace/didChangeWorkspaceFolders`, added folders are discovered and included by the same rules, removed folders stop contributing, and diagnostics are republished from the resulting snapshot set. Unusable entries are skipped as above, so the rest of a mixed event still applies. Keeping a nested folder that still resolves to a project keeps that project active even when another folder for the same project is removed. Thus the initial folder order and later add/remove order cannot silently narrow references or diagnostics.

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

The server reads the `grund.toml` via the same discovery logic as `grund check` ([§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up)), walking up from every workspace-folder anchor supplied by the editor's LSP `initialize` request (§2.2). There is no separate LSP config; one source of truth drives both the CLI and the LSP. A workspace folder with no config under either name falls back to the canonical defaults rooted at that folder ([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)).

Editor-side LSP configuration (server arguments, workspace folders) is the user's responsibility per §2.3 and is not part of `grund.toml`.

## 4. Determinism and parity with the CLI

Same input + same config → same diagnostics, same hover body, same definition target, byte-for-byte ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). The implementation enforces this by routing LSP state through `grund-core` snapshot, check, show, refs, and formatting APIs, plus focused LSP tests for linkification, configured trigger handling, workspace member marker resolution, UTF-16 ranges, and document-link targets. The full child-process sweep over `tests/e2e/cases/*` ships as `tests/integration/lsp_cli_parity.rs`: for every plain-`check` case, the diagnostics the server publishes are the located findings the CLI prints, or the build is red.

The LSP server does not have an "interactive" mode, a confirmation prompt, or any user-visible state that the CLI lacks ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)). It is the same engine with a different transport.

## 5. Out of scope

- **Per-editor wrappers**: VSCode/IntelliJ/Vim/Emacs first-party plugins are not shipped ([§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do)). The LSP server is the surface; editor configuration is the user's.
- **Refactoring (rename ID)**: `grund` does not rename IDs; the scheme says IDs are forever ([§FS-non-goals.4](FS-non-goals.md#4-cross-workspace-id-renaming)).
- **Inline editing of declaration bodies from the hover popup**: editors already do this well; `grund-lsp` does not implement it.
- **Network access**: the server performs no network I/O ([§FS-non-goals.11](FS-non-goals.md#11-network-access-during-a-check)). All scanning is local.
