# DISC-markup-format-declarations: Declarations in AsciiDoc, reStructuredText, LaTeX, and similar markup document formats

## Status

Discussion. No decision yet. This note records how a `grund` **declaration** could live in a prose document format other than Markdown — AsciiDoc, reStructuredText, LaTeX, Org-mode, Typst — so that a spec authored in a project's house format is a first-class declaration home, not just a file that happens to contain `§` citations. It is the prose-document analog of [§DISC-doc-comment-section-syntax](2026-05-24-doc-comment-section-syntax.md#disc-doc-comment-section-syntax-native-looking-section-syntax-inside-doc-comment-declarations), which asks the same recognition question for native section syntax *inside code doc-comments*. The companion authoring-sugar idea is [§DISC-declaration-local-shorthand](2026-05-24-declaration-local-shorthand.md#disc-declaration-local-shorthand-declaration-local-shorthand-for-citing-sections-of-the-same-declaration).

If accepted, the work would touch [§AR-scanner.2.1](../../architecture/AR-scanner.md#21-declaration-detection), [§AR-scanner.2.2](../../architecture/AR-scanner.md#22-section-detection), [§AR-scanner.2.3](../../architecture/AR-scanner.md#23-citation-detection), and the [§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked) `[scan]` surface. The key constraint is the same as in the sibling notes: persisted citations stay canonical `§<ID>[.<section>]` edges so [§GOAL-polyglot-citation](../../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful), [§FS-refs](../../functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id), and [§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) keep their current graph model — a native heading form is *recognized* into the same declaration and section map, never stored as a new grammar.

## Context

`grund` recognizes a declaration in exactly two shapes today ([§AR-scanner.2.1](../../architecture/AR-scanner.md#21-declaration-detection)):

1. **Markdown-form** — a `#`-prefixed heading at any level: `# FS-foo: …`, `## GOAL-bar: …`. This is the `.md` declaration.
2. **Code-form** — a configured comment prefix immediately before the ID (`/// AR-foo: …`, `# AR-foo: …`), or a bare `AR-foo: …` inside a Python docstring. The comment prefix was allowed to drop the `#` in [§DF-code-declarations-drop-hash](../../decisions/functional/DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix).

For source files the scanner strips the comment envelope and then reuses the *same* Markdown numbered-heading regexes everywhere ([§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)): a code-resident declaration is conceptually level 1, and `## 1. …` / `### 1.1 …` inside its comment block declare section paths `1` and `1.1` ([§AR-scanner.2.2](../../architecture/AR-scanner.md#22-section-detection), [§FS-config.3.3](../../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)).

A prose markup format is neither of those. AsciiDoc, RST, and LaTeX are documentation formats like Markdown — but their **headings** are not `#`-prefixed and their **comments** are not in the default `comment_prefixes`. So today:

- Citations already work *if the file is read*. The `§<ID>` regex is format-agnostic ([§AR-scanner.2.3](../../architecture/AR-scanner.md#23-citation-detection)); a `<§>FS-foo` in `spec.adoc` is a citation as soon as `adoc` is in `[scan] extensions`.
- Declarations mostly do **not** work. `== FS-foo: title` (AsciiDoc) and `\section{FS-foo: title}` (LaTeX) match neither declaration shape, so the spec is invisible to `grund <ID>`, `grund check`, and `grund refs`.

The question this note records is how a declaration should live in these formats — and the answer splits into a *home* question and a *level model* question.

## Principles

- **One stored section model.** Whatever native heading form is recognized must resolve to the same `§<ID>.<section>` edge a Markdown heading would. Recognition changes; the stored graph, `refs`, `cover`, and grep do not. This is the load-bearing invariant from both sibling notes.
- **Prose formats are classified like Markdown, not like source.** The string-literal carve-out in [§AR-scanner.2.3](../../architecture/AR-scanner.md#23-citation-detection) keys off *"every extension except `md`"*: in a source file a bare ID inside a string literal is not a citation. AsciiDoc/RST/LaTeX have no string literals; treating them as source would mis-fire that rule. Adding them forces the current binary *markdown vs. source* split to become a three-way *markdown · other-prose · source* classification — the prose class shares Markdown's "no string literals, bare tokens are plain prose" behavior.
- **Line-oriented and deterministic; no host-format parsing.** A heading must be recognizable from a single line by regex, the way Markdown and stripped comments are. No macro expansion, no brace-balanced multi-line argument parsing, no document-order inference. This keeps faith with [§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing) (no AST parsing) and is the test that decides which formats are easy and which are hard.
- **Config-first where it already suffices.** The *comment* home (below) needs only `[scan]` additions, no scanner change. Reserve scanner work for the *native-heading* home, where it actually buys rendered specs.
- **Breadth of formats, not generalization of the scheme.** Reading more document formats is still about the one ID scheme — it does not weaken [§FS-non-goals.8](../../functional-spec/FS-non-goals.md#8-generalization-beyond-the-id-scheme). And `grund` still does not *render* these files ([§FS-non-goals.5](../../functional-spec/FS-non-goals.md#5-documentation-generation)); it reads and validates them.

## Two homes per format

### Home A — the declaration is a comment (cheap, invisible)

Put the declaration and its numbered sections inside the format's comment syntax, exactly as a code doc-comment does. This needs **no scanner change** — only `[scan] extensions` plus a `comment_prefixes` entry ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)) — because the existing comment-stripping + Markdown-heading pipeline ([§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) already does the rest.

```asciidoc
// FS-foo: Foo behavior
//
// ## 1. Inputs
// Accepted inputs.
```

The cost: the spec does not render in the published document. For *code* that is correct (the doc-comment renders elsewhere); for a *prose* document it is usually the wrong default — the whole point of authoring in AsciiDoc is that the spec is part of the rendered text. So Home A is the zero-effort fallback, not the recommended shape.

### Home B — the declaration is a native rendered heading (the goal)

The declaration *is* the document's own section heading, so the spec renders as structure — the prose analog of Markdown's `# FS-foo:`. This is what authors actually want, and it is what requires the scanner to learn each format's heading grammar. The three formats differ sharply in how hard that is, entirely along the **level model** axis: how a heading's depth is determined.

## Format notes

### AsciiDoc — the clean case (marker-count levels)

AsciiDoc ATX-style headings are `=`-prefixed, and the `=` count *is* the level — a near-exact rename of Markdown's `#` model:

```asciidoc
== FS-router: Router spec

Lead text.

=== 1. Dispatch
Route selection.

==== 1.1 Priority
Priority tie-breaks.
```

`==` → level 1 declaration, `=== 1.` → section `1`, `==== 1.1` → section `1.1`. The strict `section_heading_levels` rule ([§FS-config.3.3](../../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)) maps directly: required level = declaration level + dotted-path depth, counting `=` instead of `#`. Line-oriented, deterministic, no parsing. AsciiDoc is the strongest first target, and AsciiDoc's own `xref:` was already named as the motivating second cross-reference family when `[fmt.cross_refs]` was generalized (see [§FS-config.3.7](../../functional-spec/FS-config.md#37-fmtcross_refs--cross-reference-emission), [§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission)). (AsciiDoc also has a legacy setext-style underlined heading; the `=`-prefix form is canonical and the only one worth recognizing first.)

### reStructuredText — the hard case (order-dependent levels)

RST headings are a title line **underlined** (optionally overlined) by a run of punctuation:

```rst
FS-router: Router spec
======================

1. Dispatch
-----------
Route selection.

1.1 Priority
~~~~~~~~~~~~~
Priority tie-breaks.
```

Two properties fight the model directly:

- **A heading is two lines**, not one. The title and its underline must be read together — a single-line regex cannot see the level. This breaks the line-oriented assumption that Markdown and stripped comments rely on.
- **No character has a fixed level.** RST assigns levels by *order of first appearance* in the document: whatever underline style appears first is level 1, the next new style is level 2, and so on. A fixed strict mapping from depth to dotted path (the [§FS-config.3.3](../../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth) contract) has nothing to anchor to without scanning document order — which is exactly the per-document inference the determinism principle rules out.

So RST native headings are the format that most resists Home B. The pragmatic options: (a) support RST only via Home A (comments — RST's `.. ` comment form), or (b) require an explicit, configured underline-character → level table so the mapping is deterministic rather than order-inferred, accepting that this diverges from how RST renderers themselves assign levels. RST is the reason this note is separate from the AsciiDoc-only easy path.

### LaTeX — the middle case (keyword-hierarchy levels)

LaTeX headings are sectioning commands with a fixed hierarchy — `\part`, `\chapter`, `\section`, `\subsection`, `\subsubsection`, `\paragraph`, `\subparagraph` — and the title is a brace argument:

```latex
\section{FS-router: Router spec}

Lead text.

\subsection{1. Dispatch}
Route selection.

\subsubsection{1.1 Priority}
Priority tie-breaks.
```

The level comes from the *command keyword*, not a repeated marker, so the scanner needs a fixed keyword → level table (a small, deterministic addition — not document-order inference, unlike RST). Two frictions:

- **The title is a brace argument.** A single-line `\section{…}` is line-local and fine; a title that spans lines or nests braces edges toward the parsing that [§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing) forbids. Single-line headings should be the only supported form.
- **The section number is redundant with LaTeX's auto-numbering.** Writing `\subsection{1. Dispatch}` duplicates the number LaTeX already emits. It still works for `grund`'s purposes (the dotted prefix is what `grund` reads), but it will look odd to LaTeX authors and is worth calling out. LaTeX's comment form (`%`, not in default `comment_prefixes`) makes Home A available as well.

### Org-mode and Typst — same axis, noted for completeness

Org-mode (`*`, `**`, … — marker-count, like AsciiDoc) and Typst (`=`, `==`, … — also marker-count) both fall on the easy end of the level axis and would follow the AsciiDoc treatment if demand appears. They are listed so the framing is visibly *"levels by marker-count (easy) vs. keyword-hierarchy (LaTeX) vs. order-dependent (RST)"*, not a per-format zoo.

## Open questions

- Is **AsciiDoc-only, native-heading (Home B)** the right first pass, with everything else deferred — given it is the one format that maps onto the existing model with no new level notion?
- Should RST be supported only through **Home A (comments)**, or is a configured underline → level table worth the divergence from RST's own order-dependent leveling?
- Does the prose-format class get its own `[scan]` toggle, or is being listed in `extensions` enough — and how is the *markdown · other-prose · source* classification ([§AR-scanner.2.3](../../architecture/AR-scanner.md#23-citation-detection) string-literal carve-out) configured or inferred from extension?
- Should the LaTeX keyword → level table be fixed in the scanner or exposed in `[scan]` config for projects with custom sectioning commands (`\addsec`, KOMA, custom macros)?
- How should `fmt --cross-refs` ([§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission)) emit links into these formats — AsciiDoc `xref:`, RST `:ref:`, LaTeX `\ref`/`\hyperref` — given each renderer's anchor algorithm is its own work ([§DF-github-anchor-fidelity](../../decisions/functional/DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly))? This is the same anchor-model open question raised in [§DISC-doc-comment-section-syntax](2026-05-24-doc-comment-section-syntax.md#disc-doc-comment-section-syntax-native-looking-section-syntax-inside-doc-comment-declarations).
