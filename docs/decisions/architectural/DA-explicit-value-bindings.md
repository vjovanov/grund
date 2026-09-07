# DA-explicit-value-bindings: compare only authored delimited value bindings

**Status:** Accepted
**Date:** 2026-09-07

## 1. Context

First-class values must catch content drift without asking a line-oriented citation tool to infer intent from arbitrary prose. The contract also needs one source that application code can read without introducing generated artifacts or historical state. This decision narrows [§FS-non-goals.2](../../functional-spec/FS-non-goals.md#2-spelling-grammar-prose-quality) only as far as [§FS-values](../../functional-spec/FS-values.md#fs-values-opted-in-kinds-bind-authored-components-to-one-declared-value) requires and extends the scanner/checker boundary of [§AR-scanner.3](../../architecture/AR-scanner.md#3-output) and [§AR-checker.2](../../../crates/grund-core/src/checker.rs).

## 2. Decision

`grund` compares only the explicit, single-line, backtick-delimited binding grammar in [§FS-values.3.1](../../functional-spec/FS-values.md#31-the-only-binding-grammar). Unbackticked adjacency and bare value citations stay invisible to value comparison. A binding's marked citation remains an ordinary citation and resolves through the existing resolver before the independent value-checker pass runs.

JSON in the opted-in kind's own home is the language-neutral runtime surface. It joins the same declaration catalog as Markdown, preserving exact spans and duplicates; applications read it directly. No separately configured value-source list or generated module exists.

Checks compare the current tree only. The feature stores no prior fingerprint, reconciliation marker, audit record, or generated freshness state.

## 3. Why

Backticks and the exact adjacency grammar record author intent without guessing which nearby token belongs to a citation. Keeping unbackticked prose ordinary preserves byte-identical behavior until a kind and a use both opt in. Keeping the binding citation in the graph prevents a second resolution model from disagreeing with `check`, queries, workspaces, or the LSP.

Home JSON already gives code and prose one readable source. Code generation would create a second artifact whose freshness must be managed, while history would change a deterministic current-tree check into a stateful database. Neither is needed to catch the reported mismatch.

## 4. Consequences

- Authors must write the explicit binding form when they want comparison; surrounding prose is never inferred or linted.
- The scanner records declarations, exact components, and bindings once; a focused checker pass resolves then compares them.
- Markdown and JSON are equivalent declaration formats, but only JSON is directly reusable as runtime data.
- Rendering/interpolation, bare-literal lint, generated modules, and persisted staleness remain outside v1.
