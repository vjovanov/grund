# AR-core-module-layout: core implementation is split by category

The core implementation lives in `crates/grund-core/src/`, while `crates/grund-cli/src/main.rs` is the published `grund` CLI entrypoint described by [§AR-bindings](AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms). Inside `grund-core`, the source layout should match the same category boundaries the later LSP and binding frontends need. A single large crate root hides ownership and makes spec-to-code citations harder to place.

## 1. Module categories

`crates/grund-core/src/lib.rs` stays the engine crate entrypoint and public Rust API surface (`check`, `show`, `scan`, and the shared data types), while implementation code lives in smaller category files under `crates/grund-core/src/`.

The categories are:

- **model** — shared data types and tiny helpers used across commands.
- **config** — defaults, config discovery, config parsing, and TOML rendering helpers.
- **scanner** — tree walking, per-file scanning, e2e case discovery, and scan error handling.
- **checker** — validation rules that turn scanner findings into diagnostics.
- **output** — shared path formatting, JSON escaping, diagnostics, and report rendering.
- **show** — declaration and section retrieval/rendering.
- **refs** — reverse-reference query rendering.
- **cover** — per-file citation coverage query rendering.
- **list** — declaration catalog query rendering.
- **fmt** — citation normalization and cross-reference planning/writing.
- **id** — ID allocation, slug derivation, and ID rendering.
- **init** — scaffold/template rendering, agent-entrypoint selection ([§FS-init.2.1](../functional-spec/FS-init.md#21-files-written-updated-or-left-in-place): which entrypoints a repository has, and which of them one run writes, appends to, or updates), and managed agent-entrypoint updates.
- **completions** — shell completion scripts and dynamic completion helpers.
- **api** — public embedding API that runs the engine without CLI argument parsing or stdout/stderr rendering.
- **grammar** — the ID grammar and the lexical helpers the scanner, checker and formatter share: fenced-block and comment-line recognition, number-only shorthand, and the inline-note layout rules.
- **workspace** — `[workspace]` expansion, member claims, scope narrowing, and the multi-project scan context.
- **integrations** — the clickable-citation client artifacts and their managed writes.
- **lsp** — the snapshot-backed hover and on-type helpers `grund-lsp` calls, kept here so the server stays a transport.
- **compat** — the deprecated `main_entry()` command adapters kept for 0.4 consumers ([§AR-bindings.2](AR-bindings.md#2-grund-core-the-only-place-logic-lives)); the `*_cmd` files beside `checker`, `config` and `fmt` are the same path's per-command halves.

A file belongs to the category whose prefix its name carries — `scanner_walk.rs` to **scanner**, `init_block.rs` to **init** — and `lib.rs` is the one file outside them, as the crate entrypoint. The prefixes each category owns:

| Category | File-name prefixes |
|---|---|
| **model** | `model` |
| **config** | `config` |
| **scanner** | `scanner` |
| **checker** | `checker` |
| **output** | `output` |
| **show** | `show` |
| **refs** | `refs` |
| **cover** | `cover` |
| **list** | `list` |
| **fmt** | `fmt` |
| **id** | `id` |
| **init** | `init` |
| **completions** | `completions` |
| **api** | `api` |
| **grammar** | `grammar`, `markdown_fence`, `comment_line`, `shorthand`, `inline_note_layout` |
| **workspace** | `workspace` |
| **integrations** | `integrations` |
| **lsp** | `lsp`, `on_type` |
| **compat** | `compat` |

## 2. Refactor boundary

Splitting the core and CLI crates is an architectural refactor only: it must not change CLI output, diagnostics, scan behavior, template bytes, or public entrypoints. The CLI package may keep calling compatibility command adapters while narrower data-returning APIs are introduced, but embedders use the public API in `api.rs`.

## 3. File size

Each implementation file under `src/` stays below 500 lines of code. If a category grows past that limit, split it into smaller category subfiles, or into a category directory with submodules, rather than letting a new monolith form.

## 4. Citation placement

Code moved into a category file keeps the same behavior citations it carried before. When a whole category implements an architectural behavior, the file or module-level comment may cite this spec; narrower functional clauses remain cited on the specific function or branch that implements them.
