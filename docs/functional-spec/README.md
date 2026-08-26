# Functional spec

This is the external behavior of `grund` — *what* it does, not how it's built. Each spec lives in its own file. The H1 of that file declares an `FS-<slug>` ID, and the body is its contract. Anywhere else in the tree, a citation like `§FS-<slug>.<section>` resolves back into one of these files.

## CLI commands

The subcommands a user runs on the command line.

- [§FS-check](FS-check.md#fs-check-grund-validates-every-reference-in-a-repo) — grund validates every reference in a repo
- [§FS-show](FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id) — grund reads a single declaration body by ID
- [§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id) — grund lists every declared ID
- [§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id) — grund lists every citation of an ID
- [§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) — grund groups citations by scanned file
- [§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk) — grund normalizes references in bulk
- [§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) — grund bootstraps a new grund-conformant repo
- [§FS-id](FS-id.md#fs-id-grund-proposes-ids-for-new-declarations) — grund proposes IDs for new declarations
- [§FS-completions](FS-completions.md#fs-completions-grund-completes-declared-ids-in-shells) — grund completes declared IDs in shells

## Editor integration

The editor surface — an optional LSP server that any LSP-aware editor can talk to. No first-party per-editor plugins ship; configuration is the user's one-time work.

- [§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server) — grund ships an optional LSP server
- [§FS-integrations](FS-integrations.md#fs-integrations-grund-prints-and-installs-its-rendering-layer-integrations) — grund prints and installs its rendering-layer integrations

## Packaging

How `grund` is shipped.

- [§FS-distribution](FS-distribution.md#fs-distribution-grund-distribution-targets) — grund distribution targets

## Cross-cutting

Behavior every subcommand inherits.

- [§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions) — grund's command-line surface conventions
- [§FS-errors](FS-errors.md#fs-errors-grund-emits-messages-in-fixed-shapes) — grund emits messages in fixed shapes
- [§FS-output-shapes](FS-output-shapes.md#fs-output-shapes-machine-readable-output-shapes) — machine-readable output shapes

## Verbose fixtures

Concrete fixtures that keep the command specs readable while pinning exact examples.

- [§FS-examples](FS-examples.md#fs-examples-examples-teach-canonical-user-workflows) — examples teach canonical user workflows
- [§FS-init-fixtures](FS-init-fixtures.md#fs-init-fixtures-concrete-init-fixtures) — concrete init fixtures

## Configuration and scope

- [§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up) — grund reads a TOML config file found by walking up
- [§FS-inline-citation-style](FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations) — configurable shape of inline code-comment citations
- [§FS-workspace](FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace) — grund validates cross-project citations in a workspace
- [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) — what grund will deliberately not do

---

This index is navigational only. Citations should target the spec ID directly, never this file.
