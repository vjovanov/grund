# e2e

End-to-end tests for `grund`. Each case is a tiny repository plus golden command results. The Rust integration test in `tests/e2e.rs` discovers every directory under `e2e/cases/` and runs the built `grund` binary.

## Case layout

```
e2e/cases/<case-name>/
├── repo/
│   └── ... files scanned by grund ...
├── spec.refs
├── expected.exit
├── expected.stdout
└── expected.stderr
```

`spec.refs` is required. Every non-empty line must cite a functional spec ID such as `FS-001-check.3.1`; the harness rejects cases that do not cite the behavior they exercise.

`expected.exit` contains `0`, `1`, or `2`. `expected.stdout` and `expected.stderr` are compared byte-for-byte, except that a file containing only one newline is treated as empty so empty golden files can be represented cleanly in patches.

Most cases run `grund check <repo>`. A case may override the command with `command.args`; use `{repo}` for the fixture repo path. For write-mode tests, use `{repo_copy}` so the harness copies the fixture under `target/e2e-work/` before running the command.

Error output is part of the contract. Non-zero cases should keep `expected.stderr` concise: one actionable diagnostic per line, no aggregate footer, and no long explanatory prose that makes editor and agent consumption harder.

## Current coverage

- basic Markdown valid references
- dangling Markdown citation
- missing Markdown section
- duplicate Markdown declaration
- fenced Markdown examples ignored
- marker-prefixed citations
- optional-mode bare citations
- strict-mode bare tokens ignored
- strict-mode marker citations accepted
- config unknown-key failure
- config unsupported-version failure (newer `grund_config_version` refused, with upgrade hint)
- config custom marker in strict mode
- config discovered as a bare root `grund.toml` from a subdirectory
- config redundant pair (the bare `grund.toml` wins, the `.agents/` file is warned about)
- workspace mixing both config discovery forms across its members
- config include/exclude/extensions
- explicit `check` subcommand
- default `show` shorthand and mistyped-path failure with explicit-check hint
- top-level help output
- per-subcommand help (`grund help check`, `grund help show`, `grund help list`)
- `grund help <unknown>` failure
- `list` ID catalog (text), comma and repeated multi-kind `--kind`, `--unused`, `--summary`, summary composition with `--kind` / `--unused`, `--format json`
- JSON report output
- `fmt --check` trigger-to-marker report
- `fmt` custom trigger and marker from config
- `fmt --write` trigger-to-marker mutation path
- `fmt --marker --check` bare-to-marker report
- `fmt` idempotence
- `fmt` skips declaration headings and fenced Markdown
- `show` full Markdown declaration
- `show` Markdown section extraction
- `show` lead default
- `show --toc` / `show --brief` in text, Markdown, and JSON, including empty lead handling, empty output, E2E manifests, and mode mutex errors
- `show` missing ID failure (with recovery hint)
- `show` missing section failure (with recovery hint)
- `refs --summary` in text and JSON, including duplicate citations on one line and section-filtered summaries
- `name --explain` next-step hint
- `show` Rust inline declaration extraction
- Markdown stub to Rust inline declaration
- broken Markdown-to-Rust inline stub
- Rust source comment to Markdown citation
- Rust `///` doc-comment declaration
- Rust block doc-comment declaration
- Go line doc-comment declaration
- Python docstring declaration
- missing stub-link target
- stub-link target is a directory
- stub-link target has an unsupported extension
- skipped output/hidden directories
- nested e2e fixture repos ignored during ordinary scans
- unsupported extension ignored
- deterministic multiple-error output
- `check --full` reporting a dangling citation outside `[scan] include`, the same tree staying silent without the flag, and style / grounding findings withheld out there
- `check --full` keeping an `[scan] include` root that `[scan] exclude` names, and one whose name is hidden, inside the ordinary scope
- `check --full` resolving an out-of-scope citation against an out-of-scope declaration
- `check --full` compound out-of-scope diagnostic codes in `--format json`
- `check --full` cautioning on stderr when an explicit path leaves it nothing to widen
- `check --full` reporting a cross-member number-only shorthand once in a workspace
- inline citation style: a citation-only site carrying prose, and a soft-cap overrun surfacing as a warning
- inline note layout (`citation-first-colon`): one error per nonconforming line under `inline_note_layout_check = "error"`, the same lines as warnings under `warn`, silence at the default `off`, and silence under `inline_note_layout = "any"` whatever the check level
- config invalid-value failures for `inline_note_layout` and `inline_note_layout_check`, and for a soft cap above the hard cap

Warnings are intentionally not covered here yet. They are lower priority than the error, retrieval, formatting, and configuration contracts.
