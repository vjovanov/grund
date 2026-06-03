# DF-reference-marker: Use § as the reference marker, with $$ as the typing trigger

**Status:** Accepted
**Date:** 2026-05-08

## 1. Context

The grund reference scheme cites IDs as bare tokens in prose: `§FS-<user-login>.3.1`. This is token-cheap but ambiguous — a stray match in unrelated text (e.g. an issue tracker code, a regex example) can be picked up by the scanner as a citation, producing false positives and false dangling-ref errors.

We want a **single distinguishing character** in front of every citation that:

- Is rare enough that it will not appear by accident in code or prose followed by an ID-shaped token.
- Is aesthetically pleasing — citations are read often.
- Carries the right semantic weight (this is a *reference*, not random punctuation).

We also want a way for users to **type it without leaving the keyboard**, since the whole point of rarity is that no keyboard puts it on a key.

## 2. Decision

### 2.1 Marker

Use **`§`** (U+00A7, "section sign") as the reference marker. A canonical citation is:

```
§FS-user-login.3.1
```

Why:

- Semantic gold standard — the section sign means "section" in legal and academic citation.
- Aesthetically dignified, established typographic tradition.
- Almost never followed by `<KIND>-<digit>` in unrelated text; the regex `§<KIND>-\d+-` produces effectively zero false positives.
- Already supported in every modern font; renders crisply at any size.

### 2.2 Trigger

Default trigger sequence is **`$$`**, transformed to `§` whenever immediately followed by `<KIND>-<digit>`.

Why `$$`:

- Two same-keystrokes; both `$` are shift+4 on US layouts.
- Visually rhymes with `§` (curving stroke + central crossbar).
- The "only when followed by `<KIND>-<digit>`" constraint kills the LaTeX `$$` (display math) false positive entirely.

### 2.3 Trigger ownership

`grund` owns the trigger transformation. It runs in two places:

- **Bulk, via `grund fmt` ([§FS-fmt](../../functional-spec/FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk)).** Walk files and rewrite `<trigger><ID>` to `<marker><ID>`. Idempotent. Used as a pre-commit hook and a CI safety net. This is the canonical, always-available path — every install of `grund` has it.
- **Live, in the optional LSP server ([§FS-lsp.1.4](../../functional-spec/FS-lsp.md#14-live-trigger-transform)).** When `grund-lsp` is installed and configured in the user's editor, typing the trigger before `<KIND>-<digit>` rewrites it to the marker on the fly via `textDocument/onTypeFormatting`. This is the editor-friendly path; users without the LSP rely on the bulk pass.

Editor-native input methods (snippets, Compose, OS Unicode entry) remain available for power users — they bypass the trigger and write `§` directly.

### 2.4 Strict vs optional

**Default: strict.** Bare `FS-<user-login>` is plain text by default. The marker-prefixed form is the citation form; tooling and editor previews use the marker form.

**Opt-in compatibility mode.** Setting `[reference] strict = false` in `grund.toml` recognizes bare tokens as citations for repositories that still rely on the older optional-marker discipline. Repositories can migrate back to the default with `grund fmt --marker`.

### 2.5 Configurability

Both marker and trigger are configurable per [§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable):

```toml
[reference]
marker  = "§"     # default
trigger = "$$"    # default
strict  = true    # default; set false to recognize bare citations
```

Other valid markers we considered: `※` (U+203B, Japanese reference mark), `‡` (U+2021, double dagger), `⁂` (U+2042, asterism). Any of these is a one-line config change.

## 3. Consequences

- The citation rules ([§FS-check.1.1](../../functional-spec/FS-check.md#11-recognized-citations), [§FS-config.3.1](../../functional-spec/FS-config.md#31-reference--citation-form)) recognize only marker-prefixed citations by default, and recognize bare citations only under `strict = false`.
- The optional LSP server ([§FS-lsp.1.4](../../functional-spec/FS-lsp.md#14-live-trigger-transform)) transforms `$$<KIND>-<digit>` to `§<KIND>-<digit>` on the fly when installed and wired into the user's editor.
- A new functional spec, [§FS-fmt](../../functional-spec/FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk), defines `grund fmt` for bulk transformation.
- Existing repos that use bare citations can keep that behavior with `[reference] strict = false`. Migration to marker-prefixed citations is mechanical: `grund fmt --marker --check` reports unconverted citations; `grund fmt --marker` rewrites them.
- The marker becomes the visible signal of a grund citation. A reader scanning a file sees `§FS-...` and immediately knows: this is a reference, follow it.

## 4. Alternatives considered

| Marker | Why rejected |
|---|---|
| `※` (Japanese reference mark) | Strongest rarity, but unfamiliar to most Western readers; harder to learn what it means. |
| `‡`, `†` (daggers) | Established academic citation marks but evoke footnotes more than section refs; double dagger reads as "footnote of footnote." |
| `⁂` (asterism) | Beautiful but ornamental; reads as a section break, not a reference. |
| `¶` (pilcrow) | Word-processor-coded; less serious. |
| `[[...]]` (Obsidian-style) | Familiar but four extra characters per citation; loses token-cheap property. |
| No marker | Status quo; suffers the false-positive problem that motivates this decision. |

| Trigger | Why rejected |
|---|---|
| `::` | Reserved in Rust/C++/Haskell; false transforms in code. |
| `..` | Path operator (`../`) and sentence punctuation. |
| `[[FS-001]]` (full bracket form) | Verbose; defeats the goal of keeping citations short. |
| Editor-native only (no grund trigger) | Inconsistent across editors; contributors on unfamiliar editors get no help. |
