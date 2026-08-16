# FS-inline-citation-style: configurable shape of inline code-comment citations

An inline citation in a code comment can carry a short rationale next to the `§<ID>` token — the project explains *why* this clause is grounded in that spec point. This spec defines a project-level house style for that rationale: whether it is allowed at all, how long it may run, and where the citation sits inside it. The same configuration drives `grund check` enforcement and the agent-facing copy in `AGENTS.md` / `CLAUDE.md` so the LLM that authors citations and the linter that validates them agree on the rules. Serves [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) and [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible).

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

A *note* is any non-whitespace text inside an inline citation site that is not a comment-prefix character and not part of a `§<ID>[.<section>]` token (workspace-qualified `§<alias>/<ID>` tokens, [§FS-workspace.1](FS-workspace.md#1-citation-syntax), are citation tokens, not notes). What separates two citation tokens of one chain is not a note either: whitespace, or a single comma with optional whitespace around it. So `// §FS-check.3.1  §FS-config.3.1` and `// §FS-check.3.1, §FS-config.3.1` are both pure citation comments — the second spells the chain the way §3.3 requires a note's citation run to be spelled, and writing it must not turn a pointer into prose. Anything else between two tokens is a note: a second comma, a ` + `, a ` / `, an `and`.

## 2. Configuration

The schema lives in `[reference]` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)):

```toml
[reference]
inline_style = "citation-with-note"   # default; alt: "citation-only"

# Budgets — apply only when inline_style = "citation-with-note":
inline_note_suggested_lines = 1       # soft cap; advisory unless warn_on_suggested = true
inline_note_max_lines       = 3       # hard cap
inline_note_max_columns     = 100     # hard cap on the longest line at the site

# Layout — applies only when inline_style = "citation-with-note":
inline_note_layout       = "any"      # default; alt: "citation-first-colon" (§3.3)
inline_note_layout_check = "off"      # off | warn | error — how `check` reports a deviation

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
| `inline_note_layout`           | `"any"`                |
| `inline_note_layout_check`     | `"off"`                |
| `warn_on_suggested`            | `false`                |

The defaults preserve the convention this project already follows — a one-line rationale next to each `§<ID>` citation — and never reject sites that an existing conformant tree was already writing. `inline_note_layout = "any"` is that promise for the layout axis in particular: it imposes no shape at all, so a tree that never had a house style gains no findings and pays no classification work.

### 2.2 Load-time invariants

- `inline_note_suggested_lines ≤ inline_note_max_lines` — a soft cap above the hard cap is meaningless. A `grund.toml` that violates this fails on load with the standard config-error shape ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)).
- The three `inline_note_*` keys are valid regardless of `inline_style`; under `inline_style = "citation-only"` they are inert (no note is ever permitted, so the budget never applies). `grund config show` still prints every key — the file is the canonical machine-readable form.
- `warn_on_suggested` is a boolean; any other value is a config error.
- `inline_note_layout` is a closed enum — `any` or `citation-first-colon` — and `inline_note_layout_check` is a closed enum — `off`, `warn`, or `error`. Any other value fails on load with the standard config-error shape ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)), the same way an unknown `inline_style` does. Both sets are widenable later without a `grund_config_version` bump ([§FS-config.5](FS-config.md#5-schema-versioning)).
- The two layout keys are independent of each other and of the budgets. `inline_note_layout_check` is legal at every layout and is simply inert under `inline_note_layout = "any"` — there is no shape to deviate from — and both keys are inert under `inline_style = "citation-only"`, where no note is ever permitted and so no note has a layout. Inert keys are still parsed and still printed by `grund config show`, exactly as the `inline_note_*` budgets are.

### 2.3 Counting lines and columns

- **Lines.** A site's line count is the physical extent of its comment block per §1 — `last_line - first_line + 1`. A single `// …` line counts as 1; a three-line `///` run, `/** … */`, or `""" … """` block counts as 3. Blank intra-block lines (a ` * ` filler inside `/* … */`, an empty `///` line) count toward the total — the rule measures the comment's physical size.
- **Columns.** A site's column width is the byte-column position of the last character on its longest constituent line, counting from column 1 — the same indexing the scanner records on every citation ([AR-scanner.3](../architecture/AR-scanner.md#3-output)). Tabs are one column, not display-width: the cap matches what an editor's column indicator shows in a file, not the visual rendering on any particular tabstop setting.
- **Note presence.** After stripping the line's comment-prefix tokens (`//`, `*`, the opening `/**`, the docstring `"""`, etc.), every citation token, and the separator joining two consecutive citation tokens where that separator is whitespace with at most one comma (§1), any non-whitespace character remaining on any line of the site is a note. This is the same line-normalization the scanner already does for declaration detection ([AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) — applied to the whole block instead of one line.

## 3. Styles

### 3.1 `citation-only`

A citation site may contain only its comment prefix(es) and one or more `§<ID>[.<section>]` tokens, separated by whitespace or by a single comma (§1). Any non-citation, non-whitespace text in the site is an error.

Allowed:

```rust
// §FS-check.3.1
// §FS-check.3.1  §FS-config.3.1
// §FS-check.3.1, §FS-config.3.1
```

Rejected:

```rust
// §FS-check.3.1 dangling-ref enforcement entry point
// the per-finding shape comes from §FS-errors.2.1
```

The intended use is repositories that prefer to keep all rationale in the spec — code comments at citation sites become pure pointers. Under this style, `inline_note_*` keys have no effect.

### 3.2 `citation-with-note`

A citation site may contain one or more citation tokens **plus** free-text prose, bounded by `inline_note_max_lines` and `inline_note_max_columns`. The prose may appear before, after, or between citation tokens — the budgets are this style's only constraint. A project that also wants one canonical arrangement of citation and prose sets `inline_note_layout` (§3.3); at its default `any` the style is exactly as permissive as it reads here.

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

### 3.3 `inline_note_layout` — where the citations sit

`inline_style` says whether a note may exist and the budgets say how big it may be; neither says where the `§<ID>` tokens sit inside it. `inline_note_layout` is that third axis, and it is orthogonal to the other two: it constrains arrangement only, never presence and never size.

`inline_note_layout = "any"` (the default) imposes nothing — §3.2 as written. `inline_note_layout = "citation-first-colon"` requires the canonical form:

```
<cite>[, <cite>]*: <note>
```

read on the line's content **after** the comment prefix (`//`, `///`, `//!`, `#`, `;`, `--`, ` * `, `/**`, a docstring quote, …) and any block closer (`*/`, a closing docstring quote) have been stripped — the same normalization §2.3 already applies to decide note presence.

Precisely: let `L` be a run of one or more recognized citation tokens joined by exactly `, ` (comma, one space), `W` one or more spaces, `T` any non-empty text, and `ε` the end of the content. A line **conforms** when its content matches

```
L ":" ( W T | ε )
```

Seven rules complete the definition:

1. **Per line, not per site.** Every line of the site that carries at least one recognized citation token must conform. A line with no citation is unconstrained, so a doc-comment may open with a summary sentence and carry its `/// §<ID>: …` lines below it — the shape Rustdoc, Javadoc, and JSDoc all encourage.
2. **Only sites that carry a note.** A site whose note presence is false (§2.3) is exempt: pure citation comments have no note, and a layout is a relation between a citation and a note. Both spellings of a chain qualify — `// §FS-check.3.1  §FS-config.3.1` and the comma-joined `// §FS-check.3.1, §FS-config.3.1`, which is the very run this layout mandates in front of a colon; a project that adopts the layout must not be told its noteless pointers are now malformed for lacking one. The consequence is deliberate — a `// §<ID>` line followed by a prose-only line **in the same block** is one site *with* a note, so the citation line is judged and fails.
3. **One edge only.** The rule constrains what *opens* the line. Citations later on the line are free, so a note may name a second spec point in passing (`// §<ID>: note (see also §<other>)`) and still conform.
4. **Exact.** Whitespace and punctuation deviations are deviations. A space instead of `, ` between two citations, a comma with no space, a space before the colon, a missing colon, a citation written last inside the prose, and a dash used where the colon belongs all fail. A citation run followed by a colon and nothing else conforms — the colon may end the line.
5. **Recognized tokens only.** "Citation token" means exactly what the scanner already recognizes on that line ([§FS-check.1.1](FS-check.md#11-recognized-citations)): the configured marker, `[reference] strict`, workspace-qualified `§<alias>/<ID>` tokens ([§FS-workspace.1](FS-workspace.md#1-citation-syntax)), and the string-literal exclusion. Under `strict = false` a bare `// FS-x: note` line is claimed by the *declaration* recognizer before it reaches this rule ([AR-scanner.2.1](../architecture/AR-scanner.md#21-declaration-detection)) — an inline declaration heading is not a citation site at all (§1) — which is precisely the ambiguity the canonical form removes: with the marker written, `// <§>FS-x: note` reads as a citation carrying a rationale and can never be mistaken for a declaration of the same ID.
6. **Same scope as the rest of this spec.** Markdown bodies have no inline citation sites and are untouched (§1), and a comment trailing code on the same line (`foo(); // §<ID>: note`) is not a site today and does not become one here.
7. **The budgets still apply.** Layout and size are judged independently; a line may deviate from the layout, exceed the column cap, or both, and each is its own finding.

Conforming:

```rust
// §FS-check.3.1: dangling-ref enforcement entry point.
// §FS-check.3.1, §FS-config.3.1: the rule and the key that turns it on.
// §FS-check.3.1: the rule (see also §FS-config.3.1).
/// Walks every recognized citation and resolves it.
/// §FS-check.3.1: one error per unresolved ID.
```

Nonconforming:

```rust
// §FS-check.3.1 dangling-ref enforcement entry point
// dangling-ref enforcement entry point (§FS-check.3.1)
// §FS-check.3.1 §FS-config.3.1: the rule and its key
// §FS-check.3.1,§FS-config.3.1: the rule and its key
// §FS-check.3.1 — dangling-ref enforcement entry point
```

The default is `any` because a layout is a house style, not a correctness property: two projects may reasonably disagree, and a tree that adopts `grund` mid-life should not be told its comments are wrong on the day it upgrades ([§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)). Choosing a value, and why the enforcement level is a second key, is decided in [§DF-inline-note-layout](../decisions/functional/DF-inline-note-layout.md#df-inline-note-layout-inline-note-layout-is-a-configured-house-style-checked-per-line-and-never-normalized).

## 4. Enforcement (`grund check`)

Findings are reported using the located-finding shape of [§FS-errors.2.1](FS-errors.md#21-located-finding), anchored at the **first line** of the offending citation site (so a multi-line block with a budget violation lands one diagnostic at its opener, not at every constituent line). The one exception is the layout rule of §4.4, which judges a single line and therefore anchors at it. The rule is a pure transformation of `Findings` ([AR-checker.4](../../crates/grund-core/src/checker.rs)) — the checker does **not** re-read files; the scanner annotates each recorded citation with its enclosing site's span, max-column width, and note presence so the rule operates from `Findings` alone.

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

`grund fmt` does **not** auto-fix style violations under this spec — budgets and layout alike. Prose cannot be safely rewritten or truncated, and moving a citation across the prose that surrounds it is a prose edit, not a token rewrite: the formatter would have to decide where a sentence ends, whether a trailing `(§<ID>)` was parenthetical, and what punctuation the remainder now needs. The fix for a layout deviation is one token in the author's own editing loop, and migrating a tree is served by `inline_note_layout_check = "warn"` (§4.4), which produces the worklist without touching a byte. The formatter continues to handle trigger-to-marker and bare-to-marker rewrites ([§FS-fmt.2.1](FS-fmt.md#21-trigger-to-marker), [§FS-fmt.2.2](FS-fmt.md#22-bare-to-marker-with---marker)) and cross-reference emission ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)) unchanged; an inline citation that violates `inline_style` rules is `check`'s problem, not `fmt`'s.

### 4.4 Warnings and errors — opt-in layout deviations

Off by default, twice over: `inline_note_layout = "any"` means there is no layout to deviate from, and `inline_note_layout_check = "off"` means a layout that *is* configured stays documentation. A project that sets only `inline_note_layout` has told its agents the house style through §5 and asked `check` for nothing — the same standing the soft cap has under `warn_on_suggested = false`.

With `inline_note_layout = "citation-first-colon"`:

| `inline_note_layout_check` | result                                                                     |
|----------------------------|----------------------------------------------------------------------------|
| `off` (default)            | silent; the layout is agent-facing guidance only (§5)                       |
| `warn`                     | one **warning** per nonconforming line; the exit code is untouched (§4.2)   |
| `error`                    | one **error** per nonconforming line; the exit code becomes 1               |

Three properties are fixed at both levels:

- **One finding per nonconforming line, anchored at that line** — not at the site's opener. A layout deviation is a property of the line the author has to edit, and a five-line doc-comment with two bad lines is two edits. This is the one rule in this spec that does not anchor at `first_line`; the budgets measure the site as a whole and keep their opener anchor.
- **The message is the same at both levels**, so moving a project from `warn` to `error` changes the exit code and nothing a reader has to re-learn. It names the canonical shape with the configured marker, e.g. ``inline note must open with its citations and a colon (§<ID>: note)``.
- **Report order is the existing deterministic order** ([§FS-errors.4](FS-errors.md#4-determinism)) — the level chooses the channel, never the sequence.

The two levels exist so a repository can adopt the style in the order adoption actually happens: turn on `warn`, migrate the tree with the report as the worklist, then turn on `error` to keep it migrated. That is the same ladder [§DF-require-grounding.2.4](../decisions/functional/DF-require-grounding.md#24-off-by-default) describes for the grounding floor, and choosing which channel a rule speaks through is a per-project configuration choice, not a redefinition of what a warning or an error *means* — those stay fixed by [§FS-check.2](FS-check.md#2-outputs).

## 5. Agent-facing rendering

The `init` machinery that writes versioned managed blocks into `AGENTS.md` / `CLAUDE.md` / sibling agent entrypoints ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)) reads the active values and emits one sentence describing the project's house style:

- `inline_style = "citation-only"` → `Inline citations carry no prose — put rationale in the spec.`
- `inline_style = "citation-with-note"`, `suggested_lines == max_lines` → e.g. `Inline notes: ≤ 1 line, ≤ 100 columns.`
- `inline_style = "citation-with-note"`, `suggested_lines < max_lines` → e.g. `Inline notes: ≤ 1 line preferred, hard cap 3 lines; ≤ 100 columns.`

When `inline_note_layout = "citation-first-colon"` is set, one further sentence is appended to whichever line above applies, naming the canonical form with the configured marker and placeholder IDs — e.g. ``Lay each note out citation-first: `// §<ID>: <note>` (several citations: `// §<ID>, §<ID>: <note>`).`` Under `inline_note_layout = "any"` nothing is appended and the rendered text is byte-identical to what a `grund` without this key produced, so no repository's managed block drifts on upgrade ([§FS-check.3.5](FS-check.md#35-invalid-agent-entrypoint-init-block)).

`inline_note_layout_check` does **not** change the sentence. The house style is what the agent is asked to write; whether `check` reports a deviation as a warning, as an error, or not at all is a fact about the project's gate, not about the form. An agent told the form and then told it is only advisory would have been given a reason to ignore it.

The collapse rule is "if soft and hard are the same number, only mention the number" — the soft/hard distinction is a property of the *config*, not always a useful distinction in the agent prose.

`grund config show` ([§FS-config.4.2](FS-config.md#42-grund-config-show-path)) is the canonical machine-readable form: every key is printed at every value, no collapse, so a human or downstream tool diffing config sees the raw shape.

## 6. Non-goals

- No `suggested_columns` knob. Column width is governed by editor/formatter rules in most repos; one hard cap is enough.
- No auto-rewrite in `grund fmt`. Prose changes need human judgment.
- No scope expansion to Markdown bodies. Spec text is not capped.
- No per-kind or per-file overrides. The style is repo-wide, matching [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) — two correctly-configured `grund` installs must agree on whether a tree is well-formed.
- No "warning for hard-cap miss." A hard-cap miss is always an error; if a project wants the soft tier to nag, it sets `warn_on_suggested = true`.
- No display-width awareness. Tabs count as one column; widening tabstops in an editor does not change whether a comment passes the cap.
- No `grund fmt` normalization of layout, in `--check` or in `--write` (§4.3). Layout is check-only.
- No per-rule severity remap. `inline_note_layout_check` selects which channel *this* rule speaks through, from a fixed set; it does not let a project re-level any other rule, and it does not change what an error or a warning means ([§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization), [§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).
- No scope growth. Layout is judged on inline citation sites only — never in Markdown bodies, never on a comment trailing code, never on the code line below the comment (§3.3, rule 6).

## 7. Architecture impact

This rule is additive on top of the existing scanner + checker pipeline:

- **Scanner** ([AR-scanner](../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)). Each recorded `Citation` gains its enclosing site's information: `(first_line, last_line, max_columns, has_note)`, plus the ascending list of the site's lines that fail the configured layout (§3.3) — computed only when a layout is configured, so `any` costs one comparison per site and nothing else ([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)). The scanner already knows the comment-block extent on every line (it normalizes `/// …`, ` * …`, docstring interiors for declaration detection in [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) — the addition is recording that extent on the citations the block contains, not new line-classification logic. Multiple citations in the same block carry the same span.
- **Checker** ([AR-checker](../../crates/grund-core/src/checker.rs)). One new rule under [AR-checker.2](../../crates/grund-core/src/checker.rs) — a pure pass over `findings.citations`, grouping by site, comparing line/column counts and note-presence against the `[reference] inline_*` settings, emitting located findings per §4.1 (and §4.2 when `warn_on_suggested = true`, §4.4 when `inline_note_layout_check` is not `off`). No file I/O: the per-line layout verdicts arrive on the site the scanner recorded, so the checker never re-reads a line to decide its shape.
- **`grund fmt`**, **`grund refs`**, **`grund cover`**, **`grund show`**: unaffected. The added fields are inert for every command except `check`.

A site shape that lies outside what the scanner already records — e.g. "the next code line after the comment" — is **not** part of the site. The rule never grows past the comment block.
