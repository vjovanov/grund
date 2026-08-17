# FS-show: grund reads a single declaration body by ID

The `show` subcommand prints a slice of a declaration's body, given an ID. By default it prints the *lead* — the section's prose down to its first child heading — which is the cheap "what is this declaration about?" read. Bigger slices (`--toc`, `--full`) and a smaller one (`--brief`) are one mutually-exclusive flag away. It exists so an agent — human or AI — can pull a single grounded fact into context without loading the whole file. Serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) and [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file).

## 1. Inputs

```
grund [show] <ID> [<path>] [--section <s>] [--brief | --toc | --full] [--format <text|md|json>]
```

- `<ID>` — the full ID without the marker (e.g. `FS-check`). May include an inline section (`FS-check.3.1`). The dotted form uses the configured `[id] section_separator` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)). When the separator is non-default (e.g., `:` or `#`) the inline form may collide with the slug grammar; use `--section` instead. In a repo whose `[id] format` carries both `{number}` and `{slug}`, the number-only shorthand is also accepted — `grund FS-042` and `grund FS-042.1` read `FS-042-user-login` ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)). Nothing is persisted by a query, so the shorthand is convenience here rather than the error it is in a file; a shorthand matching more than one declaration is an ambiguous-ID query failure naming every candidate (§2.2.1), and one matching none is `ID not found` (§3).
- `show` is optional when the first non-flag word is the ID: `grund FS-check`, `grund FS-check --toc`, and `grund --toc FS-check` are byte-for-byte equivalent to their explicit `grund show …` forms ([§FS-cli.1](FS-cli.md#1-the-default-subcommand)).
- `<path>` — directory or file whose tree is scanned to resolve the ID. Defaults to `.`. Discovery is the same as every other subcommand (walk up to a `grund.toml`, else defaults — [§FS-config.1](FS-config.md#1-file-location-and-discovery)). `--path <path>` is an accepted alias for scripts that prefer to pass it as a flag; the two forms are equivalent.
- `--section <s>` — alternative way to specify a section path (`3.1`). Mutually exclusive with the dotted form. Required when `[id] section_separator` makes the dotted form ambiguous. Composes with each `--brief` / `--toc` / `--full` slice exactly as the dotted form does (§2.2).
- `--brief` — print the title (declaration or selected-section heading) plus only the first blank-line-separated paragraph below it. The cheapest "what is this about" view — a hover-preview slice (§2.1.1).
- (no flag, the default) — print the lead: the prose between the heading and the first child section heading. Cut at the first *citable* point (a numbered subsection), so an agent landing on a bare `§<ID>` reads enough to know whether to fetch a deeper section (§2.1).
- `--toc` — print the default *plus* the headings of every nested subsection, in document order. The move when the next step is `grund <ID>.<sec>` and the section number needs to be chosen (§2.1.2).
- `--full` — print the entire body: heading down to the next same-or-shallower ID heading, all subsections recursively included. The escalation when narrower slices are not enough (§2.1.3).
- `--brief`, `--toc`, and `--full` are mutually exclusive — each picks one rung on the "how much" ladder: title + 1 paragraph → lead prose → lead + section map → full body. The rungs are strictly nested (each contains the previous), so escalating is always one more flag.
- `--format` — output shape; defaults to `text` (just the body, no headers).

## 2. Behavior

### 2.1 Whole declaration (default)

`grund FS-check` prints the *lead* — the prose between the declaration heading and the first child section heading (`## 1. ...`). The opening heading is omitted in `text` format and included in `md`. This is the new default: a 1–2 paragraph slice that names what the declaration is about, without paying for the whole body. Decided in [§DF-show-default-token-cheap](../decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in).

If a declaration has no lead paragraph (its body opens directly with `## 1. ...`), the default prints **nothing** and exits `0`. This is not an error: the declaration simply has no lead. Callers (IDE hovers, agents) can detect this case by the empty output and escalate to `--toc` or `--full`. We do not auto-fall-back; the caller knows what it wants.

`grund FS-check.3.1` applies the same cut one level down. It prints the selected section heading (`### 3.1 ...`) and the prose between that heading and the first *child* heading (`#### 3.1.1 ...`). The section heading is kept in both `text` and `md` — only the whole-declaration H1 is omitted by `text` (§3.1). If the section opens directly with a sub-subsection, the output is just the section heading line. A section that does not exist is still a `section not found` error.

#### 2.1.1 Brief (`--brief`)

`grund --brief FS-check` prints the declaration heading and only the first blank-line-separated paragraph below it: a hover-preview slice strictly shorter than the default. "First paragraph" means the first non-blank run of lines after the heading, terminated by the first blank line, the first child heading, or the end of the body — whichever comes first.

`--brief` always includes the heading line so the slice is self-labeled, regardless of `text` vs `md`. This is the one mode where the `text` rule of "omit the H1" yields ([§FS-show.3.1](FS-show.md#31-format-variants)): a single paragraph with no title is unreadable for the hover-preview use case. In `text` the heading is rendered as written, with the leading `#` prefixes preserved.

If the declaration has no lead prose (opens directly with `## 1. ...`), `--brief` prints just the heading line and exits `0`. With `--section` / the dotted form, `--brief` prints the section heading and the first paragraph under it; if the section opens directly with a sub-subsection, just the section heading is printed.

#### 2.1.2 Section map (`--toc`)

`grund --toc FS-check` prints the default lead (§2.1), then a blank line, then every numbered section heading in the declaration body, one per line, in document order, each at the depth it was written (`## 1. Inputs`, `### 2.1 Whole declaration`, `#### 2.1.1 Brief (--brief)`, …). No section bodies. The heading lines are emitted verbatim — the same bytes `--full` would show for those lines — so the section numbers the reader needs are right there to feed back into `grund FS-check.<n>`. No generated summary, ever: `--toc` is a structural slice, as deterministic as the default ([§FS-errors.4](FS-errors.md#4-determinism)).

If the lead is empty (`## 1.` opens the body), the leading blank line is omitted — the output is the section map only. If the body has no numbered headings (a short declaration that is all lead prose, an E2E manifest), `--toc` prints the default and nothing else. If both are empty, `--toc` prints **nothing** and exits `0`.

`grund --toc FS-check.3.1` restricts the map to headings **nested under** the selected section: it prints `### 3.1`'s lead, then a blank line, then `#### 3.1.1 …`, `#### 3.1.2 …`, and so on, stopping at the next sibling-or-shallower heading. A selected section with no nested headings is just its lead — i.e. behaves like the default. A section that does not exist is still a `section not found` error.

#### 2.1.3 Full body (`--full`)

`grund --full FS-check` prints from the heading of `FS-check` to the start of the next ID heading (or end of file). Every subsection and sub-subsection body is included. With `--section` / the dotted form, `--full` prints the selected section's heading and full body — the same slice §2.2 defines. The opening heading is omitted in `text` and included in `md`, as in the default.

`--full` is the escalation path when `--brief`, the default, and `--toc` are not enough. It is also the way to recover today's pre-[§DF-show-default-token-cheap](../decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in) behavior: use `grund <ID> --full`.

### 2.2 Section

`grund FS-check.3.1` selects a section. The flag determines how much of it is printed:

- `--brief`: section heading + first paragraph.
- (default): section heading + prose down to the first child heading.
- `--toc`: section heading + lead + nested heading map.
- `--full`: section heading + full body (everything down to the next sibling-or-shallower heading; nested deeper headings included).

The selected section heading is printed in all four modes — `text` strips only the whole-declaration H1, not section headings (§3.1). For `--brief`, the section heading is the slice's self-label. Arbitrary nesting depth is supported per [§FS-config.3.3](FS-config.md#33-section-paths--arbitrary-nesting-depth).

#### 2.2.1 Ambiguous ID

If an ID has more than one home — the duplicate-declaration error from [§FS-check.3.3](FS-check.md#33-duplicate-declaration) — `show` does not pick one. A stub paired with the inline declaration it points at is *one* home, not two; ambiguity means two or more independent declarations remain after that pairing collapses. When ambiguous, `show` exits 1 with a single bare stderr line (no `<path>:<line>:` prefix, since there is no single site to point at):

```
ambiguous ID: <ID> (declared at <path>:<line>, <path>:<line>[, ...])
```

Sites are listed in lexicographic `path:line` order so the message is stable across runs. The repo must be fixed (run `grund check` first) before `show` will return a body.

A number-only shorthand argument ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)) fails the same way for a different reason — not one ID with two homes, but one abbreviation naming two IDs — so it names the **candidates** rather than the sites:

```
ambiguous ID: FS-042 (matches FS-042-user-login, FS-042-user-logout)
```

Candidates are listed in ID order, and the repo needs no fixing: the caller does, by passing one of the full IDs. Nothing is guessed at ([§DF-number-only-citation-shorthand.2.7](../decisions/functional/DF-number-only-citation-shorthand.md#27-ambiguity-is-reported-never-guessed)). This shape matches the bare-message form used for `ID not found` and `section not found` ([§FS-show.3](FS-show.md#3-outputs)): all three are queries that found something other than exactly one body.

### 2.3 Inline declarations in code and doc-comments

When the ID's home is in code (per [§FS-check.3.4](FS-check.md#34-broken-inline-spec-stub) stub semantics), `show` extracts the comment block surrounding the inline declaration, strips comment markers, and prints the resulting prose. The same section logic applies — and so do the `--brief` / (default) / `--toc` / `--full` slices, computed over the stripped block exactly as over a `.md` body (the lead is what precedes the first `## N.` heading inside the comment; the section map is the numbered headings recorded within it, per §2.3.3).

The scanner recognizes the same doc-comment forms enumerated in [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) — Javadoc, JSDoc/TSDoc, Doxygen, KDoc, Scaladoc, PHPDoc, Rustdoc (`///`, `//!`, `/** … */`), C# XML doc comments, Go's `// …` doc blocks, Ruby `#` comments, and Python `""" … """` docstrings. This means an architectural spec can live directly in the class-level Javadoc, and `grund AR-<event-bus>` returns the rendered Javadoc lead — same content the optional LSP server shows on hover ([§FS-lsp.1.2](FS-lsp.md#12-hover-preview)). The stub at `docs/architecture/AR-<event-bus>.md` is a single-line H1 — `# AR-<event-bus>: [<path>](<path>)` — pointing at the file.

A code-resident declaration is written as `<comment-marker> <ID>: <title>` (or bare `<ID>: <title>` inside a Python docstring), with no markdown `#` prefix. Decided in [§DF-code-declarations-drop-hash](../decisions/functional/DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix).

A single doc-comment may declare **multiple** IDs — most usefully an `AR-` and an `FS-` co-located on the same class — and each gets its own body. The scanner ends each declaration's block at the next declaration line in either direction (§2.3.1.3 below):

```rust
/// AR-router: In-process event router
///
/// Implements the publish-subscribe contract from §FS-events.
///
/// FS-router-priority: Routes are matched in declared priority order
///
/// Ties broken by registration order; see §DF-router-tiebreak.
pub struct Router { ... }
```

`grund AR-router` returns the first body; `grund FS-router-priority` returns the second. Multi-declaration comments compose with every slice flag (`--brief`, default, `--toc`, `--full`) because each is just a normal declaration the scanner happens to have found in the same doc-comment.

#### 2.3.1 What counts as the "comment block"

Extraction is precisely defined so that the implementation has no freedom and the same input produces the same output across editor, CLI, and binding callers.

A declaration is found on a "declaration line" — a line that matches the declaration regex from [AR-scanner.2.1](../architecture/AR-scanner.md#21-declaration-detection) *and* sits inside a comment or docstring. The block surrounding it is computed as follows:

1. **Find the open boundary.** Walk **backwards** from the declaration line over consecutive lines that are part of the same comment construct:
   - For line-style comments (`//`, `///`, `//!`, `#`, `;`, `--`): consecutive lines whose first non-whitespace character matches the same comment prefix family. A blank line ends the block. A line whose first non-whitespace character is not a comment marker ends the block.
   - For block-style comments (`/* … */`, `/** … */`): walk backward until the opener is found (`/*` or `/**`). The opener line itself is part of the block.
   - For Python triple-quoted docstrings: walk backward until the opening `"""` (or `'''`). The opener line is part of the block.

2. **Find the close boundary.** Walk **forwards** from the declaration line by the symmetric rules:
   - Line-style: until a blank line or a non-comment line.
   - Block-style: until the closing `*/`. The closer line is part of the block.
   - Python docstring: until the matching `"""` or `'''`. The closer line is part of the block.

3. **Terminate early on another declaration.** Walking in **either direction**, if another declaration line of any ID is encountered, the block ends at the line before it. This is what allows two adjacent inline declarations to live in the same comment without bleeding into each other — backward termination keeps a later declaration's block from absorbing the previous declaration's tail; forward termination keeps the previous declaration's block from absorbing the next declaration's head.

#### 2.3.2 Stripping comment markers

After the block is selected, comment markers are removed line-by-line so the output is plain prose:

- Leading whitespace is preserved up to the comment marker, then the marker is dropped, then a single space following the marker is dropped if present. The remainder of the line is kept verbatim.
- For block-style continuation lines, a leading ` * ` (with surrounding spaces) is removed if present. Lines that do not have it are kept as-is.
- For Python docstrings, no marker is stripped — docstring content is plain text already; only the surrounding `"""` lines are skipped.
- Trailing comment-close markers (`*/`) on their own line are dropped entirely.
- Blank lines inside the block are preserved.

The result is the markdown that the declaration's author wrote, identical to what would have lived in a `.md` file had the spec been doc-resident instead of inline. This is the property that makes [§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments) round-trip-stable across the in-docs and in-code homes.

#### 2.3.3 Section selection inside a doc-comment

Section selection (`AR-<event-bus>.2`) works the same way inside a doc-comment as inside a markdown file: the scanner records the numbered subsection headings declared within the doc-comment block and `show` slices to the requested section. Section depth is measured relative to the declaration's heading level exactly as in markdown ([AR-scanner.2.2](../architecture/AR-scanner.md#22-section-detection)) — an `AR-<event-bus>:` declaration inside a `///` block is "level 1", so `## 1.` is a depth-1 section. The comment-stripping pass leaves these headings intact.

#### 2.3.4 Broken stub

If the ID's only home is a stub (`# <ID>: [<text>](<path>)`) whose link is broken — the `<path>` does not exist, or the file at `<path>` contains no inline declaration of `<ID>` (the [§FS-check.3.4](FS-check.md#34-broken-inline-spec-stub) error) — `show` has no body to extract. It exits `1` with a bare query-result line ([§FS-errors.2.3](FS-errors.md#23-bare-query-failure)), not a `path:line:` finding:

```
broken stub: <ID> (stub at <path>:<line> points at <target>, which does not exist)
broken stub: <ID> (stub at <path>:<line> points at <target>, which contains no inline declaration of <ID>)
```

This is the same "found something other than exactly one body" family as `ID not found` and `ambiguous ID` (§3). Run `grund check` to see the error in located form; fix the stub or the target before `show` will return a body.

### 2.4 E2E cases

`grund E2E-<name>` returns the case's manifest ([AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations)) in three parts:

```
grund <args…>
expected exit: <code>
fixtures:
- <relative path>
- <relative path>
…
```

The first line is the invocation (`grund check` when the case has no `command.args`); then an `expected exit: <code>` line; then a `fixtures:` line followed by one `- <path>` line per file in the case directory, paths relative to that directory, sorted lexicographically — deterministic for a given tree. `--full` produces this output. The default and `--toc` produce the same output (an E2E manifest has no heading tree, so the default's "lead" *is* the manifest). `--brief` prints only the first line (the invocation). Section paths are not defined for E2E cases (the manifest is not a numbered-heading tree); `grund E2E-<name>.1` is a section-not-found error. `--format=json` emits a single object `{"id":"E2E-<name>","kind":"E2E","path":"e2e/cases/<name>","args":[…],"expected_exit":<code>,"fixtures":[…]}` — `args` is the parsed `command.args` (empty when there is none), `fixtures` the same sorted relative-path list; `--brief` / `--toc` / default over a case do not change this object (the manifest has no headings or lead prose to slice further).

## 3. Outputs

- `0` — printed successfully.
- `1` — ID not found, ambiguous ID (multiple homes — [§FS-show.2.2.1](FS-show.md#221-ambiguous-id)), broken stub ([§FS-show.2.3.4](FS-show.md#234-broken-stub)), or section not found in declaration.
- `2` — I/O error, or a CLI-level failure that stops the query before it runs: the commonest is a qualified ID naming a project this run does not hold, which exits `2` with `error: unknown project alias` however many segments the path has ([§FS-workspace.8.1](FS-workspace.md#81-grund-aliasid)). An ID the grammar rejects is *not* one of these — `invalid ID` is a failed query, `1` (below).

Stdout carries the body (or, with `--format=json`, the result object — one JSON object, never NDJSON, per [§FS-errors.5](FS-errors.md#5-json-format)). Stderr carries errors. Stdout is empty on error.

A failed query (`1`) prints the bare result line and, where the next step is obvious, one extra `hint:` line on stderr below it — never on stdout. With `--format=json`, stderr instead carries one diagnostic JSON object per [§FS-errors.5](FS-errors.md#5-json-format), with `path` and `line` set to `null` because the failure has no single source location:

- `ID not found: <ID>` → `hint: run \`grund list\` to see every declared ID, or \`grund id <KIND> "<title>"\` to propose a new one`
- `section not found: <ID>.<s>` → `hint: run \`grund <ID> --toc\` to print the lead with the section map`
- a `<ID>` argument that does not match the configured `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) is rejected before the scan with `invalid ID \`<arg>\``, followed by `hint: this repo's [id] format is \`<format>\` (run \`grund config show\`); \`grund list\` shows the IDs that exist` — this is the common surprise in a repo whose format differs from the `{kind}-{slug}` `grund` itself uses.

`ambiguous ID` and `broken stub` get no hint: the fix (run `grund check`, then edit the duplicate or the stub) is already stated in §2.2.1 / §2.3.4 and the message names the sites.

### 3.1 Format variants

- `text` (default) — the body only. The whole-declaration H1 (`# FS-<x>: …`) is omitted; section headings inside the slice are kept. Mode-by-mode: the default prints the lead prose (§2.1); `--brief` prints the heading line and the first paragraph (§2.1.1) — the one mode that includes the H1 in `text`, since the slice would otherwise be unlabeled; `--toc` prints the lead plus the numbered heading lines (§2.1.2); `--full` prints the full body (§2.1.3); a selected section is printed with its own section heading in every mode (§2.2). For an inline-source declaration the body is the comment-stripped prose (§2.3.2); for an E2E case it is the manifest (§2.4). A `grund fmt --cross-refs` link wrapper around a citation (`[§FS-<x>.1](FS-<x>.md#1-y)`) is flattened back to the bare `§FS-<x>.1` — §3.2.
- `md` — same as `text` but the opening declaration heading line is **included** verbatim, and `--cross-refs` link wrappers are kept as written — that is the renderable form (§3.2). For the default and `--toc`, the heading is prefixed; for `--brief` it is already included in `text` and stays as written in `md`; for `--full`, the heading is prefixed. The kind's `[[kinds]] title` ([§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)) is *not* injected — it is metadata exposed only in `json`. For an inline-source declaration the included heading is the one written in the doc-comment (`AR-<event-bus>: In-process event broadcaster`), comment-markers stripped.
- `json` — a single object on stdout: `{"id":<ID>,"section":<section-path or null>,"body":<string>,"path":<declaring file or case dir>,"line":<1-indexed>}`. `body` is the same text `text` prints — `--cross-refs` wrappers flattened (§3.2). `section` is `null` when the whole declaration was requested. With `--toc` the object additionally carries `sections` — one `{"path":<section path>,"title":<heading text>,"depth":<integer>}` per numbered heading in the selected outline slice, in document order. For E2E cases the object is the §2.4 shape instead. The wire form is stable per [§GOAL-no-silent-breakage.1](../goals.md#1-what-counts-as-user-visible).

Verbose `show --format=json` examples, including failed-query stream behavior, live in [§FS-output-shapes](FS-output-shapes.md#fs-output-shapes-machine-readable-output-shapes).

### 3.2 Cross-reference links are flattened in `text` and `json`

A repo that has run `grund fmt --cross-refs` ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)) carries each citation in its `.md` files as a Markdown link *wrapping* the citation — `[§FS-check.1](FS-check.md#1-inputs)` instead of `§FS-check.1`. That wrapper is a rendered-view convenience ([§DF-md-link-emission](../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations)), not the canonical form; for an agent pulling a fact into context it is noise, and the relative path inside it is the wrong pointer — the consumer should resolve the citation with `grund <ID>`, not open the file.

So when `show` prints a body in `text` or in the `json` `body` field, it **flattens** every such wrapper back to the bare citation: a `[` immediately before a marker-prefixed citation token and `](…)` immediately after it — exactly the wrap shape `grund fmt --cross-refs` emits and re-derives ([§FS-fmt.6.3](FS-fmt.md#63-idempotency-and-re-derive)) — collapses to just the `§[<alias>/]<ID>[.<section>]` text. This includes qualified workspace citations such as `[<§>api/FS-login](...)`, because they are the same presentation wrapper over the same canonical citation syntax ([§FS-workspace.8.5](FS-workspace.md#85-grund-fmt---cross-refs)). Nothing else changes: an ordinary Markdown link in the prose, a citation that is not wrapped, a body extracted from a source-code doc-comment (cross-references never run on source — [§FS-fmt.6.1](FS-fmt.md#61-scope)), and a `grund <ID> --format md` body (the self-contained markdown fragment, §3.1) are all left exactly as written. The flattening is purely textual — it does not resolve the citation, so a dangling one is flattened just the same and `grund check` still reports it. Decided in [§DF-show-cross-ref-flattening](../decisions/functional/DF-show-cross-ref-flattening.md#df-show-cross-ref-flattening-grund-show-flattens-cross-reference-link-wrappers).

## 4. Why this matters

Without `show`, an agent retrieving a spec section either loads the whole file (token-expensive) or reimplements the parser. With `show`, the canonical way to pull `§FS-check.3.1` into a prompt is exactly:

```
grund FS-check.3.1
```

And when the citation is a bare `§FS-check` with no section, the cheap first move is just `grund FS-check` — the new default prints the lead paragraph, enough to know whether this is the right declaration. If the section needs to be chosen, `grund FS-check --toc` adds the section map; `grund FS-check --full` holds the full body in reserve for when even that is not enough. On this repo that ladder is roughly 0.5 KB → 1 KB → 2 KB → 15 KB: an agent that grounds itself this way pays for the fact it needs, not the file it lives in. `grund FS-check --brief` is the shortest slice of all — heading plus one paragraph — for hover previews and "is this the right ID?" checks before committing to a deeper read.

This is the agent-grounding loop: declarations live in one place, and any agent — at any time — can fetch one, or just its lead, or just its map, with a single command.
