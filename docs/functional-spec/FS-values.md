# FS-values: opted-in kinds bind authored components to one declared value

`grund` lets a repository make numbered components authoritative values. The feature is explicit and current-tree-only: either a kind opts whole declarations in or an author marks one citable section, a use names one exact component, and `grund check` compares the authored component with that declaration. Repositories that use neither authority retain byte-identical behavior. This serves [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) and [§GOAL-polyglot-citation](../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful).

## 1. Per-kind opt-in and identity

A `[[kinds]]` row opts whole declarations in only with `values = true` ([§FS-config.3.4.9](FS-config.md#349-values--first-class-value-declarations)). The row must be citable, must declare exactly one existing `file` or `folder` home, and that normalized home must be inside the project root. `values` is absent and false by default. It does not create a kind, change the configured ID grammar, or make any whole declaration outside the row's existing home authoritative.

Independently, the exact section marker in §2.4 opts that one numbered section into value authority in any supported scanned Markdown declaration or source doc-comment. It needs no `values = true`, does not make its enclosing declaration or sibling sections authoritative, and adds no configuration key or identifier grammar.

The declaration ID is a normal full local ID of that kind. Its slug carries stable identity rather than its value: for example, `CONST-field-price`, not `V-1200-USD`. The feature adds no second identifier grammar and no `value_sources` key.

## 2. Value declarations

### 2.1 Markdown declarations

A whole-declaration Markdown value is an ordinary declaration in its opted-in kind home. Its value fields are the declaration's immediate citable child headings and must be one nonempty contiguous run `.1` through `.N`, where `N >= 1`. Each heading uses the configured strict depth for that coordinate; gaps, zero or leading-zero coordinates, nested or named citable sections, and duplicate fields make the declaration invalid. The separately marked section form is §2.4; its descendants do not weaken this declaration-rooted grammar.

The component is the entire heading title after the numeric coordinate. It must fit on that physical line, be nonempty, and have no leading or trailing whitespace, backtick, or control character. Lead prose, bodies below component headings, and plain non-citable headings carry no value. A component is numeric only when its complete text matches JSON number grammar; otherwise it is a string.

### 2.2 JSON declarations from the kind home

JSON is an equivalent declaration format at the existing kind home, never a separately configured source. When the row's `file` ends in `.json`, that file is its only JSON declaration source. For a `folder` home, every normalized direct child whose extension is `.json` is a source, in bytewise path order; nested JSON, JSON outside the home, and a non-JSON `file` are not sources.

Home JSON is project catalog input, not general scan input. It is read once regardless of `[scan].extensions`, ignore/include/exclude, an explicit command path, or `--full`; those controls neither suppress home JSON nor discover another source. JSON sources never contribute citations.

A source is one top-level object. Every ordered member key is a full, unaliased local ID of the owning kind and every member value is a nonempty array. Array element `i` declares component `i + 1`; only JSON numbers and strings are valid elements. A decoded string component must be nonempty and contain no edge whitespace, backtick, or control character, matching the Markdown component boundary. Empty or non-object roots; wrong-kind, invalid, or qualified keys; empty arrays; and null, boolean, object, nested-array, or non-finite/malformed number components are invalid declarations.

The reader preserves member order, duplicate keys, raw number and string spellings, decoded strings, and exact key/member/element spans before building lookup maps. It never applies a JSON library's “last key wins” behavior. A missing, unreadable, malformed-UTF-8, or syntactically incomplete source is a scan-incomplete failure ([§FS-errors.5](FS-errors.md#5-json-format)); readable semantic violations are ordinary value errors.

### 2.3 Duplicates and ownership

A JSON key repeated in one object, the same ID declared across JSON files, a Markdown/JSON collision, or the same JSON path claimed by two opted-in homes is an ambiguous duplicate declaration even when the components agree. The existing duplicate rule wins; neither source nor kind takes precedence ([§FS-check.3.3](FS-check.md#33-duplicate-declaration)).

### 2.4 Embedded section value roots

An author marks one existing citable numeric section as a value root by ending its heading content with one ASCII space and the byte-exact lowercase suffix `<!-- grund:value -->`, followed only by optional trailing whitespace. The section may occur at any numeric depth inside any scanned Markdown declaration or supported source doc-comment, independently of the enclosing kind's `values` setting and configured home. Its canonical identity remains the declaration ID plus its existing dotted section path: a root at `FS-pricing.2.3` owns components `FS-pricing.2.3.1` through `.N`. No synthetic declaration, ID, section, or resolver is created.

For source declarations the scanner recognizes the marker after removing the same configured comment wrapper used for headings: `//`, `///`, `//!`, `#`, `;`, and `--` line comments; `/* ... */` and `*` continuation lines used by Javadoc/JSDoc and other supported block doc-comments; and content inside enabled Python `"""` and `'''` docstrings. The language adds no alternate spelling. A misspelling, different case, missing or extra space inside the suffix, incomplete suffix, or other lookalike is opaque prose and grants no value authority or value diagnostic.

The marker is removed only from semantic heading content. The section title, derived Markdown anchor, LSP title and hover token, and semantic title range exclude the separating space and marker. Raw source spans remain unchanged: `show` includes the authored marker on a verbatim root heading, formatter input and output retain it byte for byte, and diagnostics may select the marker or offending source line.

A valid marked root owns exactly one nonempty, physical-order run of immediate numeric children, with relative coordinates `.1` through `.N` contiguous and in order. Each child is written exactly one heading level below the root and satisfies §2.1's one-line component-title grammar; its existing section record and exact value span are the component. Blank lines and the outer delimiters of a source block comment are harmless. Every other nonblank line within the root is invalid: root lead prose, a component body, a named or plain child heading, a grandchild, and a child with invalid component text are reported at that line. Zero components is reported at the root marker. A gap or out-of-order coordinate is reported at each heading that differs from the next expected index. A duplicate retains [§FS-check.3.3](FS-check.md#33-duplicate-declaration) or the ordinary duplicate-section finding and also receives `invalid-value-declaration` at its duplicate heading.

An exact marker on a plain or named heading, an empty or malformed numeric heading, or a heading outside a declaration grants no authority and is `invalid-value-declaration` at the marker; the same bytes in ordinary prose are inert. Ordinary section-depth findings own a malformed root heading and run before value comparison. Multiple marked roots in one declaration are valid only when neither root path is an ancestor of the other. Nested or overlapping marks invalidate both roots and report the inner marker. A marker inside a whole-declaration Markdown value likewise invalidates the marked root while the existing declaration-root authority wins.

## 3. Explicit value bindings

### 3.1 The only binding grammar

The sole binding form is a nonempty single-backtick-delimited authored literal, one ASCII space, then a parenthesized marker-prefixed citation to either an opted-in whole-value ID with one positive numeric component or a valid marked-root path followed by one positive numeric immediate-component coordinate, all on one physical line:

```text
`1200` (§CONST-field-price.1)
```

The marker is mandatory even when `[reference] strict = false`. The literal may not be empty or multiline and may not have edge whitespace, a backtick, or a control character. Tabs or extra/missing spaces, absent parentheses, a missing marker or field, and zero, leading-zero, named, or otherwise nonnumeric component coordinates are invalid attempted bindings. A delimited form aimed at a marked root itself or below one of its components is also `invalid-value-binding`. An unbackticked adjacent token, a bare value citation, or the same delimited shape aimed at an ordinary unmarked section or a whole declaration whose kind lacks `values = true` remains ordinary prose and one ordinary citation; it is not inferred as an attempted binding and is not compared.

### 3.2 Recognized text contexts

Markdown bindings are recognized outside fenced code blocks. In a scanned source file, a binding is recognized only when the entire form is inside the same comment or doc-comment line that the scanner already recognizes; it is never read from a host-language expression or string. The citation inside a binding remains one ordinary citation for resolution, `refs`, `cover`, citation directions, grounding, and unused-declaration counting ([§FS-check.1.1](FS-check.md#11-recognized-citations)).

## 4. Exact equality

When both the authored and declared components completely match JSON number grammar, `grund` compares their arbitrary-precision decimal values without binary floating point or exponent expansion. Sign, coefficient, and exponent are normalized so `1200`, `1200.0`, and `1.2e3` are equal and negative zero equals zero. There is no implementation-size limit on a written coefficient or exponent beyond available input and memory.

Otherwise both components must be strings and their decoded Unicode scalar sequences must be exactly equal. Comparison is case-sensitive and performs no trimming, Unicode or locale normalization, separator stripping, unit conversion, or numeric/string coercion. Thus `1,200`, `+1200`, and `1_200` are strings. Ranges and units are merely successive components by convention; they have no schema, algebra, ordering, or conversion semantics.

## 5. Resolution, diagnostics, and exit status

### 5.1 Resolve before comparison

Every binding citation uses the existing local/workspace declaration and dotted-section resolver ([§FS-workspace.4](FS-workspace.md#4-resolution)). For an embedded binding, its final numeric segment selects the component and its parent path must resolve uniquely to one valid marked root that records that immediate child. Unknown alias, dangling or duplicate declaration, duplicate or missing section, invalid root, and noncanonical shorthand findings run first and suppress value comparison at that site. Invalid declaration or resolution also suppresses mismatch; malformed binding syntax remains independently reportable. A mismatch exists only after one unique valid authority and component resolve.

### 5.2 Fixed value errors

Three fixed-severity errors are reported and exit `1`:

- `invalid-value-declaration` identifies a readable whole declaration or marked section root whose value grammar or marker location is invalid.
- `invalid-value-binding` identifies an attempted delimited binding whose grammar or marked-root relationship is invalid.
- `value-mismatch` identifies a valid binding whose authored and declared components are unequal under §4.

They are errors regardless of `strict` or `--suggestions` and are never suggestions. A mismatch uses this canonical lowercase, no-period text, with the declaration site in both text and structured forms:

```text
docs/offer.md:7: value mismatch for CONST-field-price.1: bound `1250`, declared `1200` at values/field-price.md:2
```

### 5.3 Incomplete input and deterministic output

Invalid config stops before scanning. A missing, unreadable, or syntactically incomplete home JSON file preserves the existing incomplete-tree exit `2`; readable semantic JSON errors use exit `1`. For readable input, declaration and citation-resolution findings precede comparison as §5.1 specifies.

Text and NDJSON use the existing streams, schema, and deterministic `(path, line, message)` ordering ([§FS-output-shapes.1](FS-output-shapes.md#1-diagnostic-object), [§FS-output-shapes.3](FS-output-shapes.md#3-text-report-ordering)). The NDJSON `code`, message, primary location, and declaration `sites` describe the same finding as text. A clean text check prints `success`; a clean JSON check remains empty.

## 6. Shared catalog consumers

Markdown, JSON, and embedded roots enter one declaration/section catalog. An embedded root and its components keep their ordinary dotted section identities: `show` returns the queried verbatim section slice, `refs` and `cover` count the binding citation once, and shell completion continues to offer the recorded section paths without a value-specific candidate. `list` retains one row for the enclosing declaration and exposes root metadata on that row rather than inventing another row. Text appends ` [value roots: <ID.path>, <ID.path> (invalid)]` only when roots exist, ordered by canonical section path. NDJSON likewise adds an optional `"value_roots":[{"id":"<ID.path>","valid":true}]` member only on such rows; rows without roots omit it and `--summary` is unchanged ([§FS-list.2](FS-list.md#2-behaviour), [§FS-list.3](FS-list.md#3-outputs)).

`list` reports JSON IDs and locations without inventing a title; `refs` and `cover` include binding citations as ordinary citations; completion offers whole-value IDs and numbered fields; and unused and duplicate checks apply normally. `id` includes JSON IDs in collision checks but remains a Markdown-oriented allocator and never writes JSON. `refs <ID.path>` and `--section <path>` retain their exact ordinary-section meanings; no root aggregate is added.

For a JSON declaration, `show` returns the exact source member slice for an ID and exact element slice for a section. `--brief`, default, `--toc`, and `--full` collapse to that available slice, and no command synthesizes Markdown. A folder kind's generated index includes JSON IDs by linking to their JSON file without an invented Markdown anchor; a single-file JSON kind has no index, matching the existing single-file rule.

## 7. Workspaces and editor consumers

An unqualified binding resolves in its local project. The qualified form `<§>alias/CONST-field-price.1` resolves under the target project's grammar, authority, declaration, and equality rules through the same workspace resolver; member-local unknown aliases retain their existing diagnostic. Core value records and exact spans feed the public report and LSP: diagnostics equal the CLI finding, binding hover uses the same exact `show --toc` slice, and go-to-definition lands on the existing Markdown/source component heading or JSON key/element. A declaration-side root hover remains the ordinary section title and usage count, excluding invisible marker bytes from its title range; raw previews include the marker. References, highlights, and document links use the written dotted citation, with no badge, rendered substitution, or value-specific LSP completion ([§FS-lsp.1](FS-lsp.md#1-capabilities)).

## 8. Formatting stability

`fmt --cross-refs` may perform its existing trigger and safe shorthand rewrites, but it never wraps or otherwise rewrites the citation bytes inside a recognized binding: replacing `§ID` with a Markdown link would destroy the only accepted binding form. It never inserts, canonicalizes, moves, or removes an embedded value marker, and preserves its authored bytes and trailing whitespace on both `--check` and `--write` passes. Every other citation keeps the existing formatter behavior.

## 9. Compatibility and explicit exclusions

Without `values = true`, whole-declaration and JSON discovery retain their prior absence. Without the exact §2.4 marker, no embedded root, binding record, value diagnostic, output field, or extra read occurs; unmarked sections, malformed lookalikes, citations, and prose retain byte-identical behavior and never gain authority by inference ([§FS-non-goals.2](FS-non-goals.md#2-spelling-grammar-prose-quality)). The exact previously inert marker now has the explicit meaning §2.4 assigns it. Applications may reuse opted-in JSON by reading that source directly. `grund` neither generates nor freshness-checks language modules.

The feature adds no rendering or interpolation, inferred adjacent-value or bare-literal lint, range/unit semantics, derived arithmetic, generated artifact, target fingerprint, history, `reconcile` or `stale` command, excluded-path hint, value-aware search, generated-file policy, embedded JSON root, nested value root, or orphan relief. It does not change `[reference] strict`, scan scope or filters, custom citation markers or separators, named-section authority, whole-value Markdown/JSON behavior, or the history, AST, documentation-generation, offline, and deterministic-install non-goals ([§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do)).
