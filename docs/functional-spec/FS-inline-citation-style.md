# FS-inline-citation-style: configurable shape of inline code-comment citations

An inline citation in a code comment can carry a short rationale next to the `§<ID>` token — the project explains *why* this clause is grounded in that spec point. This spec defines a project-level house style for that rationale: whether it is allowed at all, and, if allowed, how long it may run. The same configuration drives `grund check` enforcement and the agent-facing copy in `AGENTS.md` / `CLAUDE.md` so the LLM that authors citations and the linter that validates them agree on the rules. Serves [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) and [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible).

## 1. Scope

An **inline citation site** is a comment block — a maximal run of adjacent comment/docstring lines, by the scanner's existing line classes ([AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) — that contains at least one citation token recognized by [§FS-check.1.1](FS-check.md#11-recognized-citations). The recognized block forms are the ones the scanner already normalizes:

- `//` / `///` / `//!` line comments: a run of adjacent lines whose first non-whitespace token is the same line-comment marker.
- `#`, `;`, `--` line comments: same rule per marker (see [§FS-config.3.5](FS-config.md#35-scan--what-gets-walked) for the full prefix set).
- `/* … */` block comments (including JSDoc / Javadoc `/** … */`): from opener to closer.
- Python triple-quoted docstrings (`""" … """` / `''' … '''`): from the opening triple-quote to the matching close.

Adjacency is broken by any line that is not part of the same block: a code line, a blank line, or a different comment style. A site never spans more than one block.

This spec governs inline citation sites only. It does **not** govern:

- Citations inside Markdown spec bodies (prose in `docs/`, `e2e/`, or any other `.md` file the scanner reads). Spec text governs itself; a sentence that needs three lines of context gets three lines of context.
- Declarations themselves — `# FS-foo: …` and `/// FS-foo: …` are declaration headings ([AR-scanner.2.1](../architecture/AR-scanner.md#21-declaration-detection)), and the scanner already excludes a declaration's own heading from the citations it records ([AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection)). A doc-comment whose first line is a declaration heading and whose remaining lines are spec body is a declaration, not a citation site.
- Inline-spec stubs (`# <ID>: [<text>](<path>)`) — a `docs/` shape, not a code-comment shape.
- Bare ID-shaped tokens that the scanner already excludes from citations: tokens inside string literals in source files ([AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection)), and any bare token at all under `[reference] strict = true` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)). If the scanner doesn't see a citation, no site exists.

A *note* is any non-whitespace text inside an inline citation site that is not a comment-prefix character and not part of a `§<ID>[.<section>]` token (workspace-qualified `§<alias>/<ID>` tokens, [§FS-workspace.1](FS-workspace.md#1-citation-syntax), are citation tokens, not notes). Whitespace separating citation tokens is not a note — chained citations (e.g. `// §FS-check.3.1  §FS-config.3.1`) remain pure citation comments.

## 2. Configuration

The schema lives in `[reference]` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)):

```toml
[reference]
inline_style = "citation-with-note"   # default; alt: "citation-only"

# Budgets — apply only when inline_style = "citation-with-note":
inline_note_suggested_lines = 1       # soft cap; advisory unless warn_on_suggested = true
inline_note_max_lines       = 3       # hard cap
inline_note_max_columns     = 100     # hard cap on the longest line at the site

warn_on_suggested = false             # if true, soft-cap overruns surface as `check` warnings
```

### 2.1 Defaults

The zero-config defaults ([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)) are the values shown above:

| key                            | default                |
|--------------------------------|------------------------|
| `inline_style`                 | `"citation-with-note"` |
| `inline_note_suggested_lines`  | `1`                    |
| `inline_note_max_lines`        | `3`                    |
| `inline_note_max_columns`      | `100`                  |
| `warn_on_suggested`            | `false`                |

The defaults preserve the convention this project already follows — a one-line rationale next to each `§<ID>` citation — and never reject sites that an existing conformant tree was already writing.

### 2.2 Load-time invariants

- `inline_note_suggested_lines ≤ inline_note_max_lines` — a soft cap above the hard cap is meaningless. A `grund.toml` that violates this fails on load with the standard config-error shape ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)).
- The three `inline_note_*` keys are valid regardless of `inline_style`; under `inline_style = "citation-only"` they are inert (no note is ever permitted, so the budget never applies). `grund config show` still prints every key — the file is the canonical machine-readable form.
- `warn_on_suggested` is a boolean; any other value is a config error.

### 2.3 Counting lines and columns

- **Lines.** A site's line count is the physical extent of its comment block per §1 — `last_line - first_line + 1`. A single `// …` line counts as 1; a three-line `///` run, `/** … */`, or `""" … """` block counts as 3. Blank intra-block lines (a ` * ` filler inside `/* … */`, an empty `///` line) count toward the total — the rule measures the comment's physical size.
- **Columns.** A site's column width is the byte-column position of the last character on its longest constituent line, counting from column 1 — the same indexing the scanner records on every citation ([AR-scanner.3](../architecture/AR-scanner.md#3-output)). Tabs are one column, not display-width: the cap matches what an editor's column indicator shows in a file, not the visual rendering on any particular tabstop setting.
- **Note presence.** After stripping the line's comment-prefix tokens (`//`, `*`, the opening `/**`, the docstring `"""`, etc.) and every citation token, any non-whitespace character remaining on any line of the site is a note. This is the same line-normalization the scanner already does for declaration detection ([AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) — applied to the whole block instead of one line.

## 3. Styles

### 3.1 `citation-only`

A citation site may contain only its comment prefix(es) and one or more `§<ID>[.<section>]` tokens, separated by whitespace. Any non-citation, non-whitespace text in the site is an error.

Allowed:

```rust
// §FS-check.3.1
// §FS-check.3.1  §FS-config.3.1
```

Rejected:

```rust
// §FS-check.3.1 dangling-ref enforcement entry point
// the per-finding shape comes from §FS-errors.2.1
```

The intended use is repositories that prefer to keep all rationale in the spec — code comments at citation sites become pure pointers. Under this style, `inline_note_*` keys have no effect.

### 3.2 `citation-with-note`

A citation site may contain one or more citation tokens **plus** free-text prose, bounded by `inline_note_max_lines` and `inline_note_max_columns`. The prose may appear before, after, or between citation tokens — what matters is the budgets, not the layout.

Allowed under the defaults (one-line note, ≤ 100 columns):

```rust
// §FS-check.3.1 dangling-ref enforcement entry point
```

Allowed under `inline_note_max_lines = 3`:

```rust
/// §FS-check.3.1 the dangling-ref check.
/// Walks every recognized citation in `findings.citations`, looks the ID up in
/// `findings.declarations`, and emits a finding if the lookup fails.
fn check_dangling(...) { … }
```

Rejected — exceeds `inline_note_max_lines`:

```rust
/// §FS-check.3.1 dangling-ref check entry point.
/// (… four or more comment lines of rationale …)
```

Rejected — exceeds `inline_note_max_columns`:

```rust
// §FS-check.3.1 dangling-ref check — emits a finding for any recognized citation whose ID does not resolve in `findings.declarations`, which is what makes `check` a linter
```

## 4. Enforcement (`grund check`)

Findings are reported using the located-finding shape of [§FS-errors.2.1](FS-errors.md#21-located-finding), anchored at the **first line** of the offending citation site (so a multi-line block with a budget violation lands one diagnostic at its opener, not at every constituent line). The rule is a pure transformation of `Findings` ([AR-checker.4](../../crates/grund-core/src/checker.rs)) — the checker does **not** re-read files; the scanner annotates each recorded citation with its enclosing site's span, max-column width, and note presence so the rule operates from `Findings` alone.

### 4.1 Errors — hard caps

Each of the following is an error and contributes to a non-zero exit code, per [§FS-check.2](FS-check.md#2-outputs):

| condition                                                       | result                                          |
|-----------------------------------------------------------------|-------------------------------------------------|
| `inline_style = "citation-only"` and a note is present          | error: `inline citation must carry no prose`    |
| `lines > inline_note_max_lines`                                 | error: `inline note exceeds N-line maximum`     |
| `max(columns) > inline_note_max_columns`                        | error: `inline note exceeds N-column maximum`   |

A single site that violates more than one cap produces one finding per violated cap (so the author sees every reason in a single pass).

### 4.2 Warnings — opt-in soft cap

`warn_on_suggested = false` (default): soft-cap overruns are **silent** at `check` time. The soft cap is purely guidance for the agent-facing surface (§5); humans get the same guidance through the same rendered copy.

`warn_on_suggested = true`: a site whose line count exceeds `inline_note_suggested_lines` but stays within `inline_note_max_lines` is reported as a **warning**. Warnings never affect the exit code, per [§FS-check.4](FS-check.md#4-warnings).

There is no `suggested_columns` knob; column width is a single hard cap. The motivation is symmetry with how editors and formatters already treat line length — a binary "too long" rather than a layered preference.

### 4.3 `grund fmt`

`grund fmt` does **not** auto-fix style violations under this spec. Prose cannot be safely rewritten or truncated. The formatter continues to handle trigger-to-marker and bare-to-marker rewrites ([§FS-fmt.2.1](FS-fmt.md#21-trigger-to-marker), [§FS-fmt.2.2](FS-fmt.md#22-bare-to-marker-with---marker)) and cross-reference emission ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)) unchanged; an inline citation that violates `inline_style` rules is `check`'s problem, not `fmt`'s.

## 5. Agent-facing rendering

The `init` machinery that writes versioned managed blocks into `AGENTS.md` / `CLAUDE.md` / sibling agent entrypoints ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)) reads the active values and emits one sentence describing the project's house style:

- `inline_style = "citation-only"` → `Inline citations carry no prose — put rationale in the spec.`
- `inline_style = "citation-with-note"`, `suggested_lines == max_lines` → e.g. `Inline notes: ≤ 1 line, ≤ 100 columns.`
- `inline_style = "citation-with-note"`, `suggested_lines < max_lines` → e.g. `Inline notes: ≤ 1 line preferred, hard cap 3 lines; ≤ 100 columns.`

The collapse rule is "if soft and hard are the same number, only mention the number" — the soft/hard distinction is a property of the *config*, not always a useful distinction in the agent prose.

`grund config show` ([§FS-config.4.2](FS-config.md#42-grund-config-show-path)) is the canonical machine-readable form: every key is printed at every value, no collapse, so a human or downstream tool diffing config sees the raw shape.

## 6. Non-goals

- No `suggested_columns` knob. Column width is governed by editor/formatter rules in most repos; one hard cap is enough.
- No auto-rewrite in `grund fmt`. Prose changes need human judgment.
- No scope expansion to Markdown bodies. Spec text is not capped.
- No per-kind or per-file overrides. The style is repo-wide, matching [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) — two correctly-configured `grund` installs must agree on whether a tree is well-formed.
- No "warning for hard-cap miss." A hard-cap miss is always an error; if a project wants the soft tier to nag, it sets `warn_on_suggested = true`.
- No display-width awareness. Tabs count as one column; widening tabstops in an editor does not change whether a comment passes the cap.

## 7. Architecture impact

This rule is additive on top of the existing scanner + checker pipeline:

- **Scanner** ([AR-scanner](../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)). Each recorded `Citation` gains its enclosing site's information: `(first_line, last_line, max_columns, has_note)`. The scanner already knows the comment-block extent on every line (it normalizes `/// …`, ` * …`, docstring interiors for declaration detection in [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) — the addition is recording that extent on the citations the block contains, not new line-classification logic. Multiple citations in the same block carry the same span.
- **Checker** ([AR-checker](../../crates/grund-core/src/checker.rs)). One new rule under [AR-checker.2](../../crates/grund-core/src/checker.rs) — a pure pass over `findings.citations`, grouping by site, comparing line/column counts and note-presence against the `[reference] inline_*` settings, emitting located findings per §4.1 (and §4.2 when `warn_on_suggested = true`). No file I/O.
- **`grund fmt`**, **`grund refs`**, **`grund cover`**, **`grund show`**: unaffected. The added fields are inert for every command except `check`.

A site shape that lies outside what the scanner already records — e.g. "the next code line after the comment" — is **not** part of the site. The rule never grows past the comment block.
