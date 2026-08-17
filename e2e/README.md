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
- nested workspaces: whole-alias-path naming, per-level alias uniqueness, the grouping node as a project, subtree scope, the short-leaf-name hint, an enclosing workspace whose own member list fails to expand, one cross-branch citation checked at both scopes, and an empty nested block with no `members` key at all
- `include_root = false` leaving the excluded root's own files scanned by nobody: a dangling citation there passes even `check --full`
- `include_root = false` at the outermost root of a nested tree: no catalog row for the root, member paths still rendered from the workspace root, the root alias unknown to `show` and to `list --project`, and completions offering no `root/`
- `fmt --cross-refs --write` wrapping citations that cross two workspace levels in both directions, and the `--check` re-run staying silent on the result
- a nested qualified ID as a CLI argument: `grund group/alpha/FS-x`, `refs group/alpha/FS-x`, and `list --project` for both a leaf and the grouping node it sits under (an exact alias match, never a prefix one)
- a `[citations]` rule entry qualified by a whole alias path (`must = ["group/alpha/FS"]`), satisfied by a nested-path line in an `spec.refs` manifest and unsatisfied by the leaf name alone
- a malformed nested alias path as a CLI argument, naming the segment that failed rather than the whole path
- the unknown-alias hint at the workspace root (the two `FS-check.3.8` worked examples, byte-exactly): a dropped prefix naming one project, and a leaf name two projects share naming both, joined as `a or b`
- config include/exclude/extensions
- explicit `check` subcommand
- default `show` shorthand and mistyped-path failure with explicit-check hint
- top-level help output
- per-subcommand help (`grund help check`, `grund help show`, `grund help list`)
- `grund help <unknown>` failure
- nested-workspace shell completions: the alias-path candidates a nested tree offers with no prefix, a mid-path prefix offering the grouping node's own ID beside its members' deeper paths, and one more Tab reaching a leaf's IDs — the typed prefix never re-offered
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
- config ID-grammar failure for a `slug_pattern` that admits the alias separator `/`

Warning coverage is partial. The inline-citation-style family pins its warning channel here — the soft-cap overrun and the `inline_note_layout_check = "warn"` case both assert the warning text and the exit code it must not move. Other warning tiers are not covered yet; they are lower priority than the error, retrieval, formatting, and configuration contracts.
