# DISC-declaration-local-shorthand: Declaration-local shorthand for citing sections of the same declaration

## Status

Discussion. No decision yet. This note records declaration-local citation **authoring sugar**: typing a bare section number inside a declaration body and having the editor or `grund fmt` expand it to a full canonical marker. It does not change the stored citation grammar. The companion idea of recognizing language-native section *syntax* (Javadoc HTML, Python reStructuredText headings) was split out to [§DISC-doc-comment-section-syntax](2026-05-24-doc-comment-section-syntax.md#disc-doc-comment-section-syntax-native-looking-section-syntax-inside-doc-comment-declarations).

If accepted, the work would touch [§FS-fmt.2.1](../../functional-spec/FS-fmt.md#21-trigger-to-marker) and [§FS-lsp.1.4](../../functional-spec/FS-lsp.md#14-live-trigger-transform). The key constraint is that persisted citations remain canonical `§<ID>[.<section>]` edges so [§GOAL-polyglot-citation](../../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful), [§FS-refs](../../functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id), and [§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) keep their current graph model: the shorthand is sugar that expands before it is ever stored.

## Context

Inside a code-resident or Markdown declaration, the most common citation target is one of the declaration's *own* sections. Today that still costs the full token: inside the `AR-router` declaration you type `$$AR-router.2` to cite `<§>AR-router.2`, repeating the ID you are already inside.

The portable section model is unchanged from [§AR-scanner.2.2](../../architecture/AR-scanner.md#22-section-detection): a code-resident declaration is conceptually level 1, and `## 1. ...` / `### 1.1 ...` inside it declare section paths `1` and `1.1` ([§FS-config.3.3](../../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)). So a declaration already knows its own section paths; the question this note records is whether `grund` should let the author cite them with a bare number — typing `$$2` inside `AR-router` and having it expand to `<§>AR-router.2`.

## Principles

- **Persist canonical citations.** `§2`, `$$2`, or any other local shorthand should not become a new stored citation grammar. At most, it is authoring sugar that immediately expands to `§<current-ID>.2`.
- **Keep resolution explicit.** A citation in the file should be understandable without remembering the containing declaration. This keeps grep, `refs`, `cover`, and moved prose simple — and is the reason the shorthand must expand eagerly rather than persist.

## Authoring correction

Declaration-local shorthand makes sense as an authoring feature, not as a stored syntax. A user typing this inside the `AR-router` declaration:

```md
See $$2.1 for priority rules.
```

could have the LSP or `grund fmt --write` expand it to:

```md
See <§>AR-router.2.1 for priority rules.
```

The correction should only run when all of these are true:

- the edit site is inside exactly one declaration body, including inline source doc-comments;
- the shorthand is a section path that exists under that declaration;
- the site is not a declaration heading, fenced code block, inline code span, or Markdown link destination;
- the rewrite is minimal and idempotent, matching the existing trigger-to-marker ergonomics ([§FS-fmt.2.1](../../functional-spec/FS-fmt.md#21-trigger-to-marker), [§FS-lsp.1.4](../../functional-spec/FS-lsp.md#14-live-trigger-transform)).

Persisted marker shorthand such as `§2` should not be accepted by `check` as a citation. If supported at all, it should be reported or corrected to the full canonical form.

## Open questions

- Should declaration-local shorthand be LSP-only, `fmt --write` only, or both?
- Should shorthand use only the configured trigger (`$$2`) or also correct marker shorthand (`§2`)?
