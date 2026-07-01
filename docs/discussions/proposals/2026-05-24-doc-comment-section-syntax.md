# DISC-doc-comment-section-syntax: Native-looking section syntax inside doc-comment declarations

## Status

Discussion. No decision yet. This note records the idea of recognizing language-native or native-looking **section syntaxes** inside code doc-comment declarations — Java Javadoc HTML headings, Python reStructuredText headings — without changing the current scanner contract. The companion idea of declaration-local citation authoring sugar (typing a bare section number that expands to a full marker) was split out to [§DISC-declaration-local-shorthand](2026-05-24-declaration-local-shorthand.md#disc-declaration-local-shorthand-declaration-local-shorthand-for-citing-sections-of-the-same-declaration).

If accepted, the work would touch [§AR-scanner.2.2](../../architecture/AR-scanner.md#22-section-detection), [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments), and [§FS-show.2.3](../../functional-spec/FS-show.md#23-inline-declarations-in-code-and-doc-comments). The key constraint is that persisted citations should remain canonical `§<ID>[.<section>]` edges so [§GOAL-polyglot-citation](../../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful), [§FS-refs](../../functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id), and [§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) keep their current graph model — any native heading form is recognized into the same section map, never a new stored grammar.

## Context

Today `grund` supports native **comment envelopes** but not language-specific **section syntaxes**. The scanner strips `///`, `//!`, `/** ... */`, `//`, `#`, Python docstring delimiters, and the other configured comment forms, then applies the same Markdown-style numbered-heading model everywhere ([§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)). A code-resident declaration is conceptually level 1; inside it, `## 1. ...` and `### 1.1 ...` declare section paths `1` and `1.1` exactly as they do in Markdown files ([§AR-scanner.2.2](../../architecture/AR-scanner.md#22-section-detection), [§FS-config.3.3](../../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)).

That gives a portable form:

```rust
/// AR-router: Router architecture
///
/// Lead text.
///
/// ## 1. Dispatch
/// Route selection behavior.
///
/// ### 1.1 Priority
/// Priority tie-breaks.
pub struct Router;
```

The citations are still full and stable:

```md
§AR-router.1
§AR-router.1.1
```

The question this note records is whether `grund` should additionally recognize language-native or native-looking subsection syntax — such as Java Javadoc `<h2>` headings or Python reStructuredText headings — for declarations whose host renderer does not treat Markdown headings the way Rustdoc/KDoc/JSDoc do.

## Principles

- **One stored section model.** A native heading form must resolve to the same section path a Markdown heading would; it changes recognition only, never the stored `§<ID>.<section>` edge — so `refs`, `cover`, and grep are unaffected.
- **Prefer native renderers when they already use Markdown.** Rustdoc, KDoc, JSDoc/TSDoc, and many Python docstring setups can render Markdown headings, so the current form is already native enough.
- **Only parse deterministic headings.** If a native form is added, it should produce the same section map as Markdown headings and remain line-oriented. Avoid host-language AST parsing ([§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing)).

## Language notes

### Java

The most plausible Java-specific form is Javadoc HTML headings:

```java
/**
 * AR-router: Router architecture
 *
 * <p>Lead text.</p>
 *
 * <h2>1. Dispatch</h2>
 * <p>Route selection behavior.</p>
 *
 * <h3>1.1 Priority</h3>
 * <p>Priority tie-breaks.</p>
 */
public final class Router {
}
```

If supported, `grund` would recognize one-line `<h2>` / `<h3>` / deeper headings after Javadoc comment stripping, use the numeric prefix as the section path, and keep citations as `<§>AR-router.1` / `<§>AR-router.1.1`. Optional attributes such as `<h2 id="dispatch">1. Dispatch</h2>` are worth considering, but multi-line headings should stay out of scope unless there is strong evidence they are needed.

This is preferable to custom `@section` tags: HTML headings are normal Javadoc content, while custom tags can require doclet or doclint configuration.

### Rust

Rustdoc is Markdown, so the current form is already the Rust-native form:

```rust
/// AR-router: Router architecture
///
/// ## 1. Dispatch
/// Route selection behavior.
///
/// ### 1.1 Priority
/// Priority tie-breaks.
pub struct Router;
```

The same applies to inner docs:

```rust
//! AR-module: Module architecture
//!
//! ## 1. Public API
```

No Rust-specific alternate syntax is needed. `#[doc = "..."]` is valid Rust but awkward to author and not a good first-class scanner target.

### Kotlin

KDoc is Markdown-like, so the current Markdown heading form is the natural Kotlin form:

```kotlin
/**
 * AR-router: Router architecture
 *
 * ## 1. Dispatch
 * Route selection behavior.
 *
 * ### 1.1 Priority
 * Priority tie-breaks.
 */
class Router
```

KDoc block tags such as `@param` and `@return` are not a good section grammar. No Kotlin-only syntax is needed unless real Dokka output shows a better deterministic heading convention.

### JavaScript / TypeScript

JSDoc and TSDoc commonly accept Markdown in description prose, so the portable form remains a good fit:

```ts
/**
 * AR-router: Router architecture
 *
 * ## 1. Dispatch
 * Route selection behavior.
 *
 * ### 1.1 Priority
 * Priority tie-breaks.
 */
export class Router {}
```

Avoid `@section` as a stored or parsed section primitive for now. It is not universally standard across JSDoc/TSDoc tools and may require linter or documentation-generator configuration.

### Python

Python has two plausible authoring styles. Markdown headings work today and are increasingly common in MkDocs / pdoc / modern docstring tooling:

```python
class Router:
    """
    AR-router: Router architecture

    ## 1. Dispatch
    Route selection behavior.

    ### 1.1 Priority
    Priority tie-breaks.
    """
```

There is also a stronger case for an optional reStructuredText-style parser because Sphinx/RST is common in Python projects:

```python
class Router:
    """
    AR-router: Router architecture

    1. Dispatch
    -----------
    Route selection behavior.

    1.1 Priority
    ~~~~~~~~~~~~
    Priority tie-breaks.
    """
```

If added, the RST form should be opt-in or carefully scoped: it needs a deterministic mapping from underlined headings to section paths and must not accidentally treat ordinary numbered lists as `grund` sections.

## Open questions

- Should Java Javadoc `<h2>1. ...</h2>` be the only non-Markdown native section form in the first pass?
- Should Python RST headings be supported, and if so behind what config or file-extension rule?
- Should native section parsing apply only inside source doc-comment declarations, or also inside Markdown files that embed HTML headings?
- How should `fmt --cross-refs` link to sections declared with source-native headings when the home is a source file and renderers do not share a stable anchor model?
