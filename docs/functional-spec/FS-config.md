# FS-config: grund reads a TOML config file under .agents/

`grund` is zero-config out of the box ([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)) and fully configurable when a project's conventions diverge ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)). This spec defines the contract: where the config lives, what it contains, what it overrides, and how malformed configs are reported.

## 1. File location and discovery

The config file is **`.agents/grund.toml`** — the `grund.toml` lives inside an `.agents/` directory at the repo root. Discovery walks upward from the working directory until a directory containing `.agents/grund.toml` is found, mirroring how `cargo` finds `Cargo.toml`. The directory **containing `.agents/`** is the **config root**; relative paths inside the config are resolved against it (not against `.agents/`).

`.agents/` is a single-purpose directory: it holds agent-facing tooling configuration that does not belong at the repo root next to the project's own metadata files. Other agent tools may colocate their configuration here in the future; `grund` only owns `.agents/grund.toml`.

If no `.agents/grund.toml` is found, `grund` runs with the built-in defaults defined in this spec. The defaults are the canonical `grund` grammar — they are not stored in any file.

## 2. Precedence

CLI flags > `grund.toml` > built-in defaults. Layering is shallow: a value present in `grund.toml` overrides the entire corresponding default; CLI flags override individual leaf values.

## 3. Schema

The config file is TOML. Every key is optional; omitted keys take the default value. Unknown keys are an **error**, not a warning, per [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — typos in config files are bugs and grund surfaces them loudly.

The recognized surface is the line-oriented subset that the schema below uses: one `key = value` per line, basic (double-quoted) strings, booleans, integers, and single-line `["…", "…"]` arrays of basic strings; `#` comments; `[table]` and `[[array.of.tables]]` headers. Multi-line arrays, inline `{ … }` tables, and other TOML constructs are not parsed — keep each value on one line. A line that does not fit this shape is reported as an error pointing at the offending line, per §4.3.

Top-level keys:

```toml
grund_config_version = 1
project_name = "Example" # optional metadata written by `grund init`
project_description = "One line describing what this project is for" # optional
```

`project_name` is free-form metadata. When the project participates in a workspace (its own config sets `[workspace]`, or its directory is listed as a member by a parent), `project_name` is also the project's workspace alias — but only when it matches the alias grammar in [§FS-workspace.1](FS-workspace.md#1-citation-syntax). A `project_name` that is not a valid alias is not a load-time error; it errors loudly at workspace expansion with `invalid workspace project alias <name>`. Outside any workspace context `project_name` is purely metadata: no checker, scanner, formatter, or query behavior depends on it.

`project_description` is a free-form one-line description of the project, chosen in [§DF-workspace-member-descriptions](../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists). It is presentation metadata only: generated workspace member lists render it next to the project's alias ([§FS-init.2.3.4.15](FS-init.md#23415-workspace-members), [§FS-workspace.3](FS-workspace.md#3-aliases)), and no checker, scanner, formatter, or query behavior depends on it. A value containing a line break (a `\n` or `\r` escape in the TOML string) is a config error at the `project_description` line, reported per §4.3 — the key exists to feed single-line list bullets, so a multi-line value is a bug surfaced loudly.

### 3.1 `[reference]` — citation form

```toml
[reference]
marker            = "§"      # default; rare character that prefixes a citation in prose
trigger           = "$$"     # default; typed sequence rewritten to marker by IDE plugin and `grund fmt`
strict            = true     # default; if false, bare citations are also recognized
require_grounding = false    # default; if true, `check` flags source files that cite no declared ID

# Inline citation style — see [§FS-inline-citation-style](FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations)
inline_style                 = "citation-with-note"   # default; alt: "citation-only"
inline_note_suggested_lines  = 1                       # soft cap; advisory unless warn_on_suggested = true
inline_note_max_lines        = 3                       # hard cap (error)
inline_note_max_columns      = 100                     # hard cap (error)
warn_on_suggested            = false                   # if true, soft-cap overruns surface as `check` warnings
```

Per [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger). `strict = true` requires a non-empty `marker`; `strict = false` is the compatibility mode for repositories that still rely on bare citations.

`require_grounding = true` adds the ungrounded-source-file error ([§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in)): every scanned non-Markdown file must carry at least one resolving citation, or declare an ID inline. `grund check --require-grounding` forces it on for one run. Per [§DF-require-grounding](../decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec); off by default so adopting the discipline is a deliberate step, like `strict`.

`inline_style`, `inline_note_*`, and `warn_on_suggested` govern the shape of inline citations in code comments — whether a `§<ID>` token may be accompanied by a short rationale, and how long that rationale may run. The full contract — modes, enforcement, agent-facing rendering — lives in [§FS-inline-citation-style](FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations). Load-time invariant: `inline_note_suggested_lines ≤ inline_note_max_lines`. Under `inline_style = "citation-only"` the `inline_note_*` keys are inert (no note is ever permitted), but they are still parsed and printed by `grund config show` — the file is the canonical machine-readable form.

### 3.2 `[id]` — ID grammar

```toml
[id]
format             = "{kind}-{number}-{slug}"
section_separator  = "."
section_heading_levels = "strict"
number_pattern     = "\\d+"
slug_pattern       = "[a-z0-9][a-z0-9-]*"
```

`format` is a template: `{kind}`, `{number}`, `{slug}` are placeholders; everything else is literal. `{kind}` is required. `{number}` and `{slug}` are individually optional — but **at least one** of them must appear, because a bare kind would not identify a declaration. The literal characters between placeholders may be anything — `-`, `_`, `.`, `:`, etc.

The three canonical shapes:

| `format`                       | Example ID            | Disambiguator                |
|--------------------------------|-----------------------|------------------------------|
| `"{kind}-{number}-{slug}"`     | `FS-NNN-<slug>`       | number; slug is descriptive  |
| `"{kind}-{number}"`            | `FS-NNN`              | number                       |
| `"{kind}-{slug}"`              | `FS-<slug>`           | slug must be unique per kind |

When `{number}` is omitted, slugs must be unique within each kind — two declarations sharing a kind and slug collide on the same ID and are reported as duplicate declarations (per [§FS-check.3](FS-check.md#3-errors-detected)). When `{number}` is present, slugs are descriptive only and may repeat across declarations with different numbers.

`section_separator` must not collide lexically with any literal in `format` or with `slug_pattern`. grund validates this on load and refuses ambiguous configs.

The chosen format is repo-wide. Mixing styles in one tree (some IDs numbered, others slug-only) is not supported — citations would look identical but resolve differently. Pick one shape per repo and keep it stable.

### 3.3 Section paths — arbitrary nesting depth

Section coordinates are **dotted paths of arbitrary depth**. There is no maximum nesting level. All of the following are valid section references when the corresponding heading exists in the declaration:

```
§FS-check.3
§FS-check.3.1
§FS-check.3.1.2
§FS-check.3.1.2.7.4
```

Section depth in the citation must match a heading at that depth in the declaration. The scanner records every numbered heading inside a declaration body and validates citations against the recorded set, so a project that wants four-deep nesting (`## 1.`, `### 1.1`, `#### 1.1.1`, `##### 1.1.1.1`) is supported with no config changes — the dotted path simply grows.

`section_heading_levels` controls how the Markdown heading depth must line up with the dotted section path. The default, `"strict"`, requires the heading level to equal the declaration heading level plus the number of dotted path components: under an H1 declaration, `## 1. …`, `### 1.1 …`, and `#### 1.1.1 …` are valid, while `## 1.1 …` is a `section heading level mismatch` error in `grund check` ([§FS-check.3.9](FS-check.md#39-section-heading-level-mismatch)). `"warn"` reports the same mismatch as a warning, so CI can stay green while a repo migrates. `"loose"` preserves the historical behavior: any heading deeper than the declaration heading is recorded as a section, and the dotted number alone determines the tree. Plain, unnumbered headings and bold labels are always allowed prose structure; they are not grund section targets. Unknown values are invalid config.

The default `section_separator` is `.`. Projects that prefer `:` (`§FS-check:3.1.2`) or `#` (`RFC-42#3.1.2`) override it; the dotted **components** stay separated by `.` regardless of the outer separator. Example with `section_separator = "#"`:

```
§FS-check#3.1.2     ← outer separator is `#`, intra-section separator is `.`
```

This split keeps the section grammar regular at any depth.

### 3.4 `[[kinds]]` — recognized prefixes

One `[[kinds]]` table per allowed prefix. A kind is either *multi-file* (`folder = "<dir>"`) — each declaration is the H1 of its own file under `<dir>` — or *single-file* (`file = "<path>"`) — every declaration of the kind is an H2 inside that one document. Setting both `folder` and `file` on the same kind is invalid; setting neither leaves the kind with no configured home (`grund id` will print no folder, and the misplaced-declaration check in [§FS-check.3.7](FS-check.md#37-misplaced-declaration-configured-kind-home) only applies when a declaration sits inside some other kind's unique configured home).

`folder` is used by `grund id` ([§FS-id.2.2](FS-id.md#22---format-json) emits it as the `folder` field) and by editor "create new declaration" / "go to home folder" actions; it is also a checker boundary: a declaration inside exactly one configured `folder` home must declare that folder's kind ([§FS-check.3.7](FS-check.md#37-misplaced-declaration-configured-kind-home)). `file` is stricter: declarations of a single-file kind found outside the configured path are reported under [§FS-check.3.7](FS-check.md#37-misplaced-declaration-configured-kind-home), and a different-kind declaration inside that exact file is likewise a unique-home conflict. Declarations are still recognized outside configured homes — including inline source declarations — and declarations in files covered by zero or multiple configured homes are not rejected by the home-kind rule because there is no single expected kind.

`title` is human-readable metadata: it surfaces in `grund <ID> --format=json`, `grund refs --format=json`, and IDE hover previews, and is **not** injected into `grund <ID> --format=md` text (which is the declaration verbatim — [§FS-show.3](FS-show.md#3-outputs)).

The defaults declare the canonical eight, in this order:

```toml
[[kinds]]
prefix = "GRUND"
file   = "docs/grund.md"
title  = "Why: project motivation"

[[kinds]]
prefix = "GOAL"
file   = "docs/goals.md"
title  = "Where: project direction and outcomes"

[[kinds]]
prefix = "FS"
folder = "docs/functional-spec"
title  = "What: behavior, requirements, and constraints"

[[kinds]]
prefix = "AR"
folder = "docs/architecture"
title  = "How: high-level implementation, structure, and design"

[[kinds]]
prefix = "DF"
folder = "docs/decisions/functional"
title  = "Product behavior decisions and tradeoffs"

[[kinds]]
prefix = "DA"
folder = "docs/decisions/architectural"
title  = "Architecture decisions and tradeoffs"

[[kinds]]
prefix = "E2E"
folder = "e2e/cases"
title  = "Executable user scenarios"

[[kinds]]
prefix = "RM"
file   = "docs/roadmap.md"
title  = "Planned milestones and sequencing"
```

`GRUND` is the H1 of the single file `docs/grund.md` (the project's reason for being — one declaration, all of it inline); `GOAL` declarations are H2 headings inside the single file `docs/goals.md` (one file, all goals inline); `RM` declarations are likewise H2 headings inside the single file `docs/roadmap.md` (one file, all milestones inline) — those three are single-file kinds (`file = "<path>"`); `FS`, `AR`, `DF`, and `DA` declarations are the H1 of a file in their `folder` (an `AR` declaration may instead live inline in a source doc-comment with an optional stub in `folder` — [§AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)); `E2E` declarations are case directories under `folder` rather than heading lines — [§AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations). A single-file kind can always be broken up later by swapping `file = "<path>"` for `folder = "<dir>"` and moving the document into that folder — the schema models the transition as exchanging one key for the other, not setting both. A project that overrides this list replaces the defaults entirely — there is no merge. To extend rather than replace, copy the defaults and add to them.

Prefix sets must be unambiguous: no kind's `prefix` may itself be a prefix of another kind's `prefix`. For example, `prefix = "DA"` and `prefix = "DAT"` together are invalid because a token starting with `DAT-` would parse as either kind. grund validates this on load and refuses ambiguous configs with a single error pointing at the offending pair (per §4.3).

### 3.5 `[scan]` — what gets walked

```toml
[scan]
include            = ["docs", "e2e", "src"]
exclude            = ["target", "node_modules", ".git", "dist", "build", ".venv"]
extensions         = ["md", "rs", "go", "java", "kt", "ts", "tsx", "js", "py", "c", "cpp", "swift", "scala", "rb", "php", "cs"]
comment_prefixes   = ["//", "#", ";", "--", "*", "/*"]
docstring_python   = true
respect_gitignore  = true
```

`include` is the set of paths walked **from the config root** — the directory containing `.agents/`, or, when no `.agents/grund.toml` was discovered, the current working directory (never a subdirectory that merely happened to be passed as `grund`'s path argument). So in a config-less repo `grund` (no path) and `grund check .` both walk `docs/`, `e2e/`, `src/` relative to the cwd, while `grund check src/foo` or `grund check lib/` scans exactly the file or directory it is handed — an explicit path argument overrides `include` rather than being filtered by it. A walk that ends up reading no files at all is reported, not silently passed ([§FS-check.2.2](FS-check.md#22-empty-scan)). `exclude` is the set of directory names skipped at any depth. `extensions` filters which files are read. `comment_prefixes` are the markers recognized when looking for inline declarations and citations in source files. `docstring_python` enables Python triple-quoted-string scanning in addition to `#` comments.

The default `comment_prefixes` set is broader than the languages tabulated in [§AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments): it also covers `;` (Lisp / Scheme / Clojure), `--` (SQL / Haskell / Lua / Ada), and `*` / `/*` (block-comment continuation and opener). Any line whose first non-whitespace run is a configured prefix is eligible to host a declaration heading or a citation; the [§AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) table documents the doc-comment *conventions* for the major languages, not the full set of recognized prefixes.

`respect_gitignore` (default `true`) makes the scanner honor every form of ignore file the `ignore` crate recognizes — `.gitignore` at any depth, `.git/info/exclude`, the global `core.excludesFile`, and `.ignore` files. Set to `false` only when you genuinely need to scan ignored paths. The directory-level `exclude` list above is applied **in addition** to ignore-file rules, never instead of them. See [§AR-scanner.1.1](../architecture/AR-scanner.md#11-respecting-gitignore-and-friends).

### 3.6 `[output]` — report format

```toml
[output]
format         = "text"   # text | json
color          = "auto"   # auto | always | never
relative_paths = true     # show paths relative to config root in reports
```

`relative_paths = true` (default) renders every `<path>` in a report relative to the config root (§1). `relative_paths = false` renders them relative to the path argument passed on the command line — or to the current working directory when no path is given — i.e. the same base `grund` uses when no `.agents/grund.toml` is discovered. Either way `grund` **never** emits an absolute path or a path that escapes above the chosen base; this is what keeps the report deterministic per [§FS-errors.4](FS-errors.md#4-determinism). `color` controls ANSI styling once the colored-output feature lands ([§FS-errors.3](FS-errors.md#3-message-text)); until then output is plain bytes regardless of this value, and a change to that default goes through the [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) path.

### 3.7 `[fmt.cross_refs]` — cross-reference emission

```toml
[fmt.cross_refs]
enabled       = true       # default; false opts out of generated Markdown links
anchor_format = "github"   # default; one of github | gitlab | mkdocs | pandoc | none
```

The full contract for this block — what `enabled` does, the named `anchor_format` profiles, and when the cross-reference pass runs — lives in [§FS-fmt.6.7](FS-fmt.md#67-configurability), [§DF-md-link-default-on](../decisions/functional/DF-md-link-default-on.md#df-md-link-default-on-markdown-cross-reference-links-default-on-for-github-review-and-discovery), and [§DF-md-link-anchor-strategy](../decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass). It is part of the schema here because the generated `.agents/grund.toml` ([§FS-init.2.4](FS-init.md#24-generated-agentsgrundtoml)) writes every key in this section explicitly, including `enabled = true`, so the default generated file teaches that `grund fmt --write` emits Markdown inline links in `.md` files. `[fmt.cross_refs]` is the home for cross-reference settings; today `grund fmt --cross-refs` only emits the Markdown inline-link form ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)), so `anchor_format` is the only knob — a future markup family adds its settings under this same block ([§FS-fmt.6.7](FS-fmt.md#67-configurability)), additively, with no `grund_config_version` bump (§5).

### 3.8 `[workspace]` — sub-project namespaces

```toml
[workspace]
members      = ["apps/api", "packages/*"]
include_root = true
```

`members` and `include_root` are specified by [§FS-workspace](FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace). The table is optional; without it the repository is a single project exactly as before. Unknown keys under `[workspace]` are errors like any other config typo.

## 4. Validation and inspection

### 4.1 `grund config validate [path]`

Loads the config discovered by walking up from `path` (or `.` when omitted), checks the schema, and reports problems. Exits 0 on success, 1 on validation errors — the error in the same `error: <path>:<line>: <message>` shape §4.3 defines. No tree scan is performed.

### 4.2 `grund config show [path]`

Prints the **effective** configuration — defaults merged with the config discovered by walking up from `path` (or `.` when omitted), plus CLI flags — as TOML. Useful for debugging "why did grund recognize this citation" or "what does my config actually evaluate to."

### 4.3 Invalid config behavior

A `grund.toml` that fails validation causes every `grund` subcommand to exit with code 2 (code 1 for `grund config validate` itself, §4.1) and a single error message pointing at the first problem, in the form `error: <path>:<line>: <message>` on stderr ([§FS-errors.2.2](FS-errors.md#22-cli-level-message), [§FS-check.2.1.1](FS-check.md#211-cli-level-messages)) — the `error:` prefix marks it a CLI-level failure, the `<path>:<line>:` inside the text points at the offending key or line. Subsequent problems are not reported until the first is fixed — this avoids cascading errors that obscure the root cause.

For concrete stderr examples and the distinction between `config validate` exit `1` and config-blocked command exit `2`, see [§FS-output-shapes.6](FS-output-shapes.md#6-cli-and-config-failures).

## 5. Schema versioning

The TOML file may include a top-level `grund_config_version = N`. The current version is **1**. Future incompatible schema changes increment this; grund refuses to load a config whose version is greater than the grund binary's known maximum, with an error suggesting an upgrade. Configs with no version key are interpreted as version 1.

## 6. What is NOT configured here

Per [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible), the following are deliberately **not** configurable, to avoid the trap of every grund repo behaving differently in surprising ways:

- The set of severity levels (only `error` and `warning` exist).
- The exit code mapping (`0`/`1`/`2` per [§FS-check.2](FS-check.md#2-outputs)).
- The ordering of the report (always deterministic).
- Anything that would let two correctly-configured grund installs disagree on whether a given repo is well-formed.
