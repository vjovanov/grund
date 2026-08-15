# Editor Support via LSP

`grund-lsp` is the optional editor server for `grund`. It provides diagnostics, hover previews, usage counts on declaration titles, go-to-definition, references, document links, and live `$$` to `§` formatting from the same engine as the CLI ([§FS-lsp](../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)).

## Install

Install the CLI first:

```bash
cargo install grund
```

Then install the LSP server separately:

```bash
cargo install grund-lsp
grund-lsp --version
```

When testing from this repository before a release, install the workspace crate instead:

```bash
cargo install --path crates/grund-lsp
grund-lsp --version
```

`grund-lsp` speaks LSP over stdio. Configure your editor to launch `grund-lsp` from the workspace root; there is no daemon, socket, or long-running service outside the editor process ([§FS-lsp.2.2](../functional-spec/FS-lsp.md#22-lifecycle)).

Use the same file types you scan in `.agents/grund.toml`. Markdown is the usual minimum; add Rust, Python, Go, JavaScript, TypeScript, or any other source languages in your `[scan] extensions`.

## VSCode / VSCodium

Install a generic LSP client extension and configure it to launch `grund-lsp` for Markdown plus the source file types in your `[scan] extensions`. Use one that is available on your editor's marketplace: VSCodium installs from Open VSX, where "Simple LSP Client" (`wdomitrz.simple-lsp-client`) is published — the older `zsol.vscode-glspc` is only on the Microsoft Marketplace and cannot be installed in VSCodium.

With "Simple LSP Client", add this to your settings (workspace `.vscode/settings.json` or user settings):

```json
{
  "simpleLspClient.servers": {
    "grund-lsp": {
      "cmd": ["${userHome}/.cargo/bin/grund-lsp"],
      "filetypes": [
        "markdown",
        "rust",
        "python",
        "go",
        "javascript",
        "typescript"
      ]
    }
  }
}
```

The `cmd` is the path to the installed binary; `${userHome}/.cargo/bin/grund-lsp` is where `cargo install` places it. If `grund-lsp` is already on the editor's `PATH`, `["grund-lsp"]` works too. The `filetypes` are VS Code language IDs and must match the languages you scan. To get the live `$$` → `§` transform, also enable `"editor.formatOnType": true` for those languages.

**Prefer user (global) settings over a per-repo `.vscode/settings.json`.** Put the `simpleLspClient.servers` block above in your user settings once and `grund-lsp` runs in every project you open. A per-repo `.vscode/settings.json` only wires the server for that one repo — open any repo without it and VSCode silently falls back to its built-in Markdown behavior: Ctrl-click underlines a single hyphen-delimited word instead of the whole `§<ID>` token, and find-references misses the `grund` citation sites. Use a workspace `.vscode/settings.json` only for a deliberate per-project override.

A first-party VSCode extension is intentionally not shipped ([§FS-non-goals](../functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do)).

## IntelliJ family

Install LSP4IJ, then add a server named `grund-lsp`:

- Command: `grund-lsp`
- Working directory: the project root
- File mappings: Markdown plus the source file patterns in your `[scan] extensions`

Apply the server to the project and open a file containing a `§` citation. A first-party JetBrains plugin is intentionally not shipped ([§FS-non-goals](../functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do)).

## Vim / Neovim

Use the built-in LSP client from your Neovim config:

```lua
vim.api.nvim_create_autocmd({ "BufReadPost", "BufNewFile" }, {
  pattern = { "*.md", "*.rs", "*.py", "*.go", "*.js", "*.ts" },
  callback = function(args)
    vim.lsp.start({
      name = "grund-lsp",
      cmd = { "grund-lsp" },
      root_dir = vim.fs.root(args.buf, { ".agents/grund.toml", "AGENTS.md", ".git" }),
    })
  end,
})
```

If you use `nvim-lspconfig`, keep your usual language servers and add `grund-lsp` as a separate client for the same buffers; it should not replace `rust_analyzer`, `pyright`, `gopls`, `ts_ls`, or other language-specific servers.

For Vim, use an LSP client plugin that can launch a stdio server and point it at `grund-lsp` for Markdown plus the source file types in your `[scan] extensions`.

## Emacs

With `eglot`, register `grund-lsp` for Markdown and scanned source modes:

```elisp
(add-to-list 'eglot-server-programs
             '((markdown-mode rust-mode python-mode go-mode js-mode typescript-mode)
               . ("grund-lsp")))
```

Then run `M-x eglot` in a project buffer, or enable your normal project hook.

With `lsp-mode`, add a client registration:

```elisp
(with-eval-after-load 'lsp-mode
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection "grund-lsp")
    :activation-fn (lsp-activate-on "markdown" "rust" "python" "go" "javascript" "typescript")
    :server-id 'grund-lsp)))
```

Start it with `M-x lsp` in a project buffer.

## Helix

Add the server to `languages.toml`:

```toml
[language-server.grund-lsp]
command = "grund-lsp"
```

Attach it to Markdown:

```toml
[[language]]
name = "markdown"
language-servers = ["grund-lsp"]
```

Attach it to scanned source languages too. For example, if Rust files are scanned:

```toml
[[language]]
name = "rust"
language-servers = ["rust-analyzer", "grund-lsp"]
```

## Zed

Add a local language server entry to `settings.json`:

```json
{
  "lsp": {
    "grund-lsp": {
      "binary": {
        "path": "grund-lsp"
      }
    }
  },
  "languages": {
    "Markdown": {
      "language_servers": ["grund-lsp"]
    },
    "Rust": {
      "language_servers": ["rust-analyzer", "grund-lsp"]
    }
  }
}
```

Add the same `grund-lsp` entry to each scanned source language you want checked while editing.

## Sublime Text

Install the Sublime `LSP` package, then add a client configuration for `grund-lsp`:

```json
{
  "clients": {
    "grund-lsp": {
      "enabled": true,
      "command": ["grund-lsp"],
      "selector": "text.html.markdown | source.rust | source.python | source.go | source.js | source.ts"
    }
  }
}
```

Adjust the selector to match the syntaxes you scan in `.agents/grund.toml`.

## Check the wiring

Open a file containing a resolving citation such as `§FS-check`.

- Hover should show the same body as `grund FS-check --toc` ([§FS-lsp.1.2](../functional-spec/FS-lsp.md#12-hover-preview)).
- Hover a whole-ID title — the `# FS-check: …` heading, the same declaration written inline in a doc-comment, or the stub that points at it — and the popup reads `` `FS-check: …` — cited at 12 sites across 5 files ``: the same sites `grund refs FS-check` lists, counted. An uncited title reads `not cited` ([§FS-lsp.1.2](../functional-spec/FS-lsp.md#12-hover-preview)). Compare from the root the editor opened — in a workspace that is `grund refs <alias>/FS-check` from the workspace root and `grund refs FS-check` from inside the member — or the two are counting different trees.
- Hover a numbered section heading and the count is that section **and everything under it** — the same set the heading's own references return, and one no `grund refs` invocation prints, since `--section` keeps only citations whose coordinate is exactly the one asked for. The divergence is deliberate and stated in [§FS-lsp.1.2](../functional-spec/FS-lsp.md#12-hover-preview).
- Go-to-definition should jump to the declaration ([§FS-lsp.1.3](../functional-spec/FS-lsp.md#13-go-to-definition)).
- Find references from a declaration title should list citation sites ([§FS-lsp.1.3.1](../functional-spec/FS-lsp.md#131-references-from-declarations)).
- Clickable citation links target the declaration line; editors that ignore file URI `#L<n>` fragments should still use go-to-definition for the exact jump ([§FS-lsp.1.3.2](../functional-spec/FS-lsp.md#132-document-links)).
- Typing `$$FS-check` should rewrite the trigger to `§FS-check` ([§FS-lsp.1.4](../functional-spec/FS-lsp.md#14-live-trigger-transform)).
