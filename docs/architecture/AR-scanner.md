# AR-scanner: how grund discovers declarations and citations

The scanner is the single tree-walk that produces all of grund's input data. Every check in [§FS-check](../functional-spec/FS-check.md#fs-check-grund-validates-every-reference-in-a-repo) and every retrieval in [§FS-show](../functional-spec/FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id) derives from what the scanner finds. Speed ([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)) is set here.

## 1. Tree walk

Recursive walk from a root path using the `ignore` crate (the same walker that powers `ripgrep`). The walker is chosen specifically because it gives us `.gitignore` support for free — see 1.1 below.

Directory-level skip rules:

- Hidden directories (any name starting with `.`) are skipped — this already covers `.next`, `.venv`, and friends.
- Build/output directories named in the skip list (`target`, `node_modules`, `.git`, `dist`, `build`, `.venv` by default — [§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked)) are skipped at any depth.
- The skip list is configurable per [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) and [§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked).
- Symlinked entries are **followed** — the walker is built with `follow_links`, so a symlinked file arrives as the file it points at and a symlinked directory is descended into ([§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked)). The entry keeps its in-tree path, so the two rules above apply to a followed directory under its **link** name and every finding is reported there rather than at the target.
- The two **boundary** rules are not name rules and are not asked that way: a workspace member root ([§AR-workspace.6](AR-workspace.md#6-the-workspace-boundary)) and an E2E case directory (§6) are properties of the directory itself, not of the name it is reached under. For a directory reached through a link — the link, or anything below it — both are therefore asked of its **canonical** path as well as its in-tree one, and pruned if either says so. Without that, `docs/link -> ../packages` walks a root scan straight into a member namespace [§AR-workspace.6](AR-workspace.md#6-the-workspace-boundary) forbids, and a link onto the E2E cases root scans the fixture repos the manifest pass owns. Only a link-reached directory pays the `canonicalize`, and only directories are asked at all, so the ordinary walk is untouched ([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)).

The roots the walk starts from are `[scan] include` resolved against the config root, or an explicit path argument when one is given, or — under `grund check --full` ([§FS-check.1.3](../functional-spec/FS-check.md#13-the-full-tree-scope---full)) — every `include` root *together with* the config root itself, which is `include` cancelled and nothing more. The skip rules above, the ignore files, and the extension filter below apply identically to all of them, so the only thing that varies is where the descent begins.

The `--full` root list is that pair rather than the config root alone because none of the three directory-level rules can prune a walk root: the `ignore` walker never applies an ignore file or a hidden-name test at depth 0, and the `[scan] exclude` filter skips it too. A gitignored, excluded, or hidden `include` root is therefore read by the ordinary walk — it *is* a root there — and would be pruned as a descendant if the wider walk started only at the config root, making `--full` read fewer files than the plain run. Keeping both means the roots can overlap, so the one sorted file list is deduplicated before scanning: a file reached from two roots is read once, which also removes the duplicate-declaration report an overlapping `include` pair used to produce. The dedup is by path once the list is sorted, which is enough while every root spells its descendants the same way. It is not enough for an *aliased* root — a symlink to a directory inside the config root, or a case alias of one — because the two walks then reach one file under two names. So the roots are canonicalized once before the walk, and only when one of them resolves somewhere other than the path it is written as does the list also get a first-seen-wins pass keyed on file identity. `include` roots come first in the root list precisely so that first-seen is the spelling the plain run reports, which is what makes `--full` purely additive ([§FS-check.1.3](../functional-spec/FS-check.md#13-the-full-tree-scope---full)).

Following links makes a *descendant* alias, not only a root, so the identity pass above is no longer conditioned on an aliased root alone. What turns it on is a **list**, not a flag: while it walks, the scanner records the files that can wear a second name — one that is itself a link, anything below a directory link, and every file of an aliased root — and only those are resolved with `canonicalize`. Their resolved targets are the only paths another file can collide with, so every other file is tested against that small target set by path and is never resolved at all. A tree with no symlink and no aliased root does not run the pass; a tree with one symlink pays one `realpath` rather than one per file. That distinction is the whole of the cost here — a flag makes a single link anywhere charge the entire repository for it, and real repositories have a link — and it is what keeps the ordinary walk at the cost [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) sets. Each root's own file list is sorted before it joins the accumulated one, so "first seen" is a deterministic answer within a root as well as across them ([§FS-errors.4](../functional-spec/FS-errors.md#4-determinism)).

`follow_links` also turns a link the walker cannot resolve into an *error* entry in place of the file — a broken target, or a loop, which the `ignore` crate detects and reports rather than recursing into. Those are collected as the per-file scan failures of [§FS-check.2](../functional-spec/FS-check.md#2-outputs) instead of aborting the walk. The walker applies neither its directory filter nor its ignore files to an error entry, so both are re-applied by hand: the hidden-name and `[scan] exclude` tests before a loop is reported, and, for a broken link, the extension filter below plus a one-level re-walk of the link's own directory — where the link is an ordinary entry and the ignore rules do reach it. Only a positive "the walker filtered this out" suppresses the report; an unreadable parent reports, because the wrong way to be wrong here is silently.

Files are filtered by extension to those that can plausibly contain specs or inline declarations: `.md` and a curated list of source-file extensions. The walk itself produces one sorted file list before scanning starts. Small lists are scanned sequentially; large lists may scan files in parallel, but each file writes into a private result and the merge always happens in sorted path order. That preserves the byte-for-byte report ordering required by [§FS-errors.4](../functional-spec/FS-errors.md#4-determinism) while allowing the hot full-tree commands to use multiple cores once the thread-pool overhead is worth paying ([§RM-parallel-scan](../roadmap.md#rm-parallel-scan-parallel-per-file-scanning-for-large-repo-throughput)).

### 1.1 Respecting `.gitignore` and friends

By default, the walker honors every form of ignore file the `ignore` crate recognizes:

- `.gitignore` files at any depth (nearest-wins precedence, as `git` itself does).
- `.git/info/exclude`.
- The global `core.excludesFile` configured in `git config`.
- `.ignore` files (the ripgrep convention) for `grund`-specific exclusions that are not appropriate for `git`.

This means `grund` will not scan files that `git` would not commit. Generated artefacts, secrets, and vendored dependencies are skipped automatically without any `grund.toml` configuration. A repo's existing `.gitignore` is the source of truth.

The behavior is overridable via `[scan] respect_gitignore` in `grund.toml` (default `true`). Set to `false` only when you genuinely need to scan ignored paths — e.g., a repo that commits both `node_modules/` and its own specs in unusual layouts.

The directory-level skip lists in 1 above are applied **in addition** to ignore-file rules, never instead of them.

## 2. Per-file scan

A single linear pass over each file's lines, performing three jobs simultaneously:

### 2.1 Declaration detection

A regex matches declaration lines in one of two context-specific shapes:

1. **Markdown-form** — `#{1,N} <ID>[:…]`: a `#`-prefixed heading at any markdown level. This is how `.md` files declare (`# FS-foo:`).
2. **Code-form** — `<comment-prefix> <ID>[:…]`, or bare `<ID>[:…]` inside a Python docstring: a doc-comment line with the ID directly after the marker, no markdown `#` prefix. Decided in [§DF-code-declarations-drop-hash](../decisions/functional/DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix). The comment prefix is **required** outside Python docstrings — without a `#` heading in markdown or a doc-comment marker in source, a line `FS-foo: anything` in prose is not a declaration.

`<ID>` is the configured `[id]` grammar ([§FS-config.3.2](../functional-spec/FS-config.md#32-id--id-grammar)) with `{kind}` drawn from a configured `[[kinds]]` prefix. The heading may sit at any markdown level when written in markdown-form: file-form `GRUND`/`FS`/`AR`/`DF`/`DA` declarations are H1 (`# FS-… :`), and `GOAL` and `RM` declarations are H2 inside `docs/goals.md` and `docs/roadmap.md` respectively. Code-form declarations are treated as level 1 *within* the comment block.

When the regex matches, the line opens a new "current declaration" context and the **declaration heading level** `L` is recorded:

- Markdown-form: `L` is the count of `#` on the line (`#` -> 1, `##` -> 2, ...).
- Code-form: `L` defaults to `1`. The declaration is conceptually a "level-1" heading inside the doc-comment block — its sections are still numbered `## 1. …`, `### 1.1 …`, etc., one or more `#` deeper than the declaration line.

Both forms record the same `Declaration` struct downstream; consumers (`grund <ID>`, `grund check`, `grund refs`) do not care which shape the source used. (`E2E` declarations are the exception — they are directories, not heading lines; see §6.)

### 2.2 Section detection

Within a declaration context whose heading is at level `L`, a numbered subsection heading is a line of the form `#{L+1,} <n₁.n₂.….n_d>[.] <title>` — at least one `#` more than the declaration heading, then a dotted number of one or more components, an **optional** trailing `.`, whitespace, and the heading text. The line is recorded on the current declaration as the section path `n₁.n₂.….n_d` together with its `<title>` text, source line, and Markdown heading level (the heading text is needed by [§FS-fmt.6](../functional-spec/FS-fmt.md#6-cross-reference-emission) / [§DF-md-link-anchor-strategy](../decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass) and by `grund <ID> --format=md`; the source line and level are needed by [§FS-check.3.9](../functional-spec/FS-check.md#39-section-heading-level-mismatch)). The dotted number fixes the section's tree position, and the configured `[id] section_heading_levels` mode fixes how strictly the written `#` depth must match it ([§FS-config.3.3](../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)). In `"strict"` mode (the default), the heading level must be exactly `L + d`, where `d` is the number of dotted path components: under an H1 declaration, `## 1. Inputs`, `### 1.1 Recognized citations`, and `#### 3.1.2 Details` are consistent, while `## 1.1 Recognized citations` is recorded but later reported as a check error. `"warn"` records the same mismatch as a warning. In `"loose"` mode, the historical rule applies: the `#` count only has to be deeper than the declaration heading, so `## 1.1` and `### 1.1` both declare section `1.1`. Plain, unnumbered headings and bold labels are just Markdown prose structure and are not recorded as sections. Nesting depth is unbounded ([§FS-config.3.3](../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)); the recorded set is what [§AR-checker.2.3](../../crates/grund-core/src/checker.rs) validates citations against.

### 2.3 Citation detection

The citation regex matches the configured marker ([§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger); default `§`) immediately followed by an `<ID>` token, with an optional `<sep><section-path>` suffix, anywhere in the file. In default `[reference] strict = true` mode only marker-prefixed citations are recognized at all. When `[reference] strict = false` is set for compatibility, the scanner additionally matches bare ID tokens — but, in source files (every extension except `md`), a bare token whose start column lies inside a string literal is **not** treated as a citation, applying the same deterministic left-to-right quote-tracking rule as [§FS-fmt.2.3.1](../functional-spec/FS-fmt.md#231-string-literal-exclusion-rule). This keeps an ID-shaped substring inside a runtime string from raising a false dangling-ref. Marker-prefixed **unqualified** citations are recognized regardless of string context — a `§<ID>` in a string is a deliberate citation, and Markdown files have no string literals so the carve-out never applies there. The **qualified** form `§<alias>/<ID>` carries one added caveat, and only in source files: because `alias/ID` is shaped like a file path, module reference, or URL, a marker-prefixed qualified citation whose start column falls inside an inline-code span or a string literal is **not** treated as a citation there — the same path-collision caution that already rules out an *unmarked* `alias/ID` ([§AR-workspace.3.1](AR-workspace.md#31-the-rule)). Markdown has no string literals and its inline-code spans are prose formatting rather than literal code, so a marker-prefixed qualified citation in a `.md` file — including one wrapped in backticks — is always a citation, exactly like the unqualified form ([§FS-workspace.1](../functional-spec/FS-workspace.md#1-citation-syntax)). A declaration's own heading line is never counted as a citation of the ID it declares.

### 2.4 Citing-side classification

The scanner knows a citation's *cited* kind from its ID, but the citation-direction rules ([§FS-config.3.9](../functional-spec/FS-config.md#39-citations--citation-direction-rules), [§DF-citation-directions](../decisions/functional/DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels)) also need the *citing* kind — what kind of place the citation sits in. This is resolved once, at scan time, because the checker cannot reconstruct it cheaply: doc-comment declaration bodies are narrower than the file, and the same data is what [§FS-cover](../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) and [§RM-gap-report](../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) need.

Two scan-time additions carry it:

- **Declaration body range.** Every `Declaration` records the line span of its body. In a Markdown file the body runs from the declaration heading until the next heading at the **same or higher** level (or end of file) — numbered subsections (§2.2), which are deeper, stay inside it. In a source file the body is bounded by the **comment / docstring block** the declaration line opens (§4); within a multi-ID block the nearest preceding declaration line wins, so an `AR-` and an `FS-` on one class partition the comment between them. A stub heading (§4) and an `E2E` case directory (§6) span their single declaration line only.
- **Citation source kind.** Every `Citation` records the citing kind, resolved by three-step fallback with the bounds above: (1) the kind of the **enclosing declaration** — the nearest preceding declaration whose body range contains the site; else (2) the **kind home of the file** — the reverse lookup from `[[kinds]]` `folder` / `file` ([§FS-config.3.4](../functional-spec/FS-config.md#34-kinds--recognized-prefixes)), used only when exactly one home contains the file; else (3) the reserved `code` pseudo-kind. A citation later in the file than a declaration's body does **not** inherit that declaration's kind — it falls through to step 2 or 3.

The enclosing declaration is also recorded on the citation, so the obligation pass ([§AR-checker.2.9](../../crates/grund-core/src/checker.rs)) can ask "does this declaration's body cite the target?" as a lookup rather than a re-scan.

### 2.5 Escaped-citation illustrations

The `<§>`-escape ([§AR-workspace.3.1](AR-workspace.md#31-the-rule)) writes a citation's *shape* without it being live: the literal `<§>alias/ID` puts a `>` between the marker and the ID, so the citation pass (§2.3) never matches it. That silent inertness is the whole point in prose, but it also hides a slip — a real citation with the marker accidentally bracketed looks identical and raises no dangling error. A dedicated pass therefore records these escapes into a separate, check-inert `escaped_citations` list so the checker can flag one that resolves ([§FS-check.2.3.1](../functional-spec/FS-check.md#231-escaped-citation-resolves)); nothing else reads the list, so it never affects an existing check. The pass is cheap — a line that lacks the literal `<§>` needle short-circuits before any parsing — and runs uniformly in Markdown and source, since the escaped form is inert in both. Both unqualified `<§>ID` and qualified `<§>alias/ID` shapes are collected; the trailing ID is parsed with the citing project's grammar ([§FS-workspace.5](../functional-spec/FS-workspace.md#5-command-scope)'s loose parser is the cross-namespace fallback), so an exotic target grammar can miss a match — only ever costing a suggestion, never a false error. Escapes inside a fenced code block are skipped along with everything else there (§1).

### 2.6 Number-only shorthand citations

Where the configured `[id] format` carries both `{number}` and `{slug}`, a second citation pattern is compiled beside the full one: the format with the `{slug}` placeholder and one adjacent literal separator removed, so `{kind}-{number}-{slug}` yields `{kind}-{number}` and `§FS-042` is recognized ([§FS-check.1.2](../functional-spec/FS-check.md#12-the-number-only-shorthand)). A format missing either placeholder compiles no such pattern and pays nothing anywhere below.

Three properties make the pass safe to add to a grammar that already matches:

- **It runs after the full pass and skips its markers.** The full-ID pass records the marker offset of every citation it matched on the line, whether or not it went on to emit one; the shorthand pass only considers markers outside that set. A configured grammar under which some full ID is also shorthand-shaped therefore resolves as the full ID ([§DF-number-only-citation-shorthand.2.6](../decisions/functional/DF-number-only-citation-shorthand.md#26-the-full-id-always-wins-and-only-a-whole-token-is-a-shorthand)).
- **It requires a trailing boundary.** The pattern is `\A`-anchored at the start only, so it matches the `FS-042` inside any longer ID-shaped token — including a full ID whose slug this grammar rejects, like `FS-042-User-Login`. A match counts only when the character after it cannot continue an ID: an alphanumeric, `_`, or a literal from `format` that is itself followed by a component. `/` is excluded on purpose — it can only precede a kind, and the full-ID pass already reads `§FS-042-user-login/x` as a citation, so counting it here would make the two forms disagree about one boundary and silently drop the shorthand. The `regex` crate has no lookahead, so the rule is a post-match character test rather than part of the pattern. Skipping it is not a missed citation but a *wrong* one — the pass would report a token the file does not contain, and `fmt` would splice the canonical slug into the middle of the author's text ([§FS-fmt.2.4](../functional-spec/FS-fmt.md#24-shorthand-to-canonical)).
- **It records whether the token sits in a numeric run.** A trailing boundary says the token *ended*; it says nothing about whether the token is a citation. `§SPEC-001→SPEC-003` clears the boundary test and is a renumbering table, so the pass also reads forward past the boundary for a delimiter run carrying a second number and flags the site ([§FS-fmt.2.4.1](../functional-spec/FS-fmt.md#241-a-shorthand-in-a-numeric-run-is-not-rewritten)). Two character classes decide it and no punctuation is enumerated, so the test is one forward walk of a few bytes on a token the earlier gates have already accepted — off the hot path of every canonical citation ([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)).
- **It is marker-gated unconditionally.** Unlike the full-ID pass it has no bare branch, so `[reference] strict = false` does not widen it (§2.3). `KIND-NNN` is too common in the wild to recognize unmarked ([§DF-number-only-citation-shorthand.2.4](../decisions/functional/DF-number-only-citation-shorthand.md#24-the-marker-is-required-a-bare-shorthand-is-text)).
- **It emits an ordinary `Citation`,** flagged `shorthand`, carrying an `Id` whose `slug` is `None` and the written token as its text.

A whole-file post-pass then resolves each flagged citation against the declaration set: the declarations sharing its kind and number. Exactly one match rewrites the citation's `Id` to that declaration's, so every downstream consumer — checker, `refs`, `cover`, the unused-declaration warning, the LSP snapshot — reads a canonical `Id` and needs no shorthand awareness at all. Zero or several matches leave the slug `None`, which is the state [§AR-checker.2.12](../../crates/grund-core/src/checker.rs) reports on. The resolution runs against the *project's* declarations, so it lands in the same merge step as the other whole-file post-passes (§2.4) rather than in the per-line loop. The escaped list (§2.5) is resolved with it, so an escape whose shorthand would be live is still caught by [§FS-check.2.3.1](../functional-spec/FS-check.md#231-escaped-citation-resolves).

Every pass that resolves more than one site builds a `(kind, number)` index of the declarations first and looks candidates up in it. Asking the question per site instead — a filter over every declaration in the project — is O(sites × declarations), and the tree where that bites is a repository full of shorthands being migrated to canonical form, which is precisely the tree this rule exists to be run over ([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)). The index is built lazily, so a tree without shorthands builds none: `check` keys one per target namespace on first use, and `fmt` builds one per walk rather than one per line.

Whether a site is *rewritable* is decided here too, not by the checker: a shorthand inside inline code, a Markdown link destination, or a source string literal is recorded as a citation like any other but flagged unrewritable, because [§FS-fmt.2.3](../functional-spec/FS-fmt.md#23-what-is-never-rewritten) forbids `fmt` from touching it. The scanner owns that call because it is the only pass holding the line text, and one predicate serving both keeps `check` from naming a fix `fmt` declines to make ([§FS-check.3.13](../functional-spec/FS-check.md#313-number-only-shorthand-citation)). The numeric-run flag rides along for the same reason and reaches a different verdict: unrewritable-here means say nothing, in-a-run means say something else ([§FS-check.3.15](../functional-spec/FS-check.md#315-shorthand-citation-in-a-numeric-run)).

The predicate is applied to the **raw** source line, not the scanned one. Those differ inside a Python docstring (§4), where the scanned text is the interior with the delimiters stripped: a citation on the opening `"""…` line sits inside a string literal as far as `fmt` is concerned and outside one as far as the scanner's own text is concerned. Asking the scanner's view there marks the site rewritable, and `check` then reports an error `grund fmt --write` silently declines to fix. The question is "what would `fmt` do", so it has to be asked of the bytes `fmt` reads.

An `Id` with a `None` component whose placeholder appears in the format renders as the shorthand — `render_id` drops the placeholder and one adjacent literal, the same reduction the pattern is built from — so the one code path prints both forms and no caller has to special-case a partial ID.

## 3. Output

The scanner produces a `Findings` struct containing:

- `declarations: BTreeMap<Id, Vec<Declaration>>` — keyed by ID, with file/line, stub-info, the recorded sections (each section path paired with its heading text — §2.2) per declaration, and the body line range (§2.4). An `E2E` declaration (§6) carries its case-directory path, fixture list, invocation, and expected exit code instead.
- `citations: Vec<Citation>` — each with the referenced ID, optional section, file, line, and start column, whether it was written marker-prefixed or bare, whether it was written in the number-only shorthand (§2.6), and the resolved source kind plus enclosing declaration (§2.4).
- A citation inside a source comment block also carries that block as an inline citation site ([§FS-inline-citation-style.1](../functional-spec/FS-inline-citation-style.md#1-scope)): its first and last line, its widest column, whether it carries a note, and the ascending list of the lines this run will *report* as failing the configured `[reference] inline_note_layout` ([§FS-inline-citation-style.3.3](../functional-spec/FS-inline-citation-style.md#33-inline_note_layout--where-the-citations-sit)). That list is what lets the checker report a per-line deviation without re-reading the file, and it is a record of the configured verdict rather than a survey of the tree: it holds the lines rule 1 judges, not every line carrying a citation, and it is empty — with no line classified — at the default `inline_note_layout = "any"`, under `inline_style = "citation-only"`, and at `inline_note_layout_check = "off"`, where the verdicts would reach no channel ([§FS-inline-citation-style.4.4](../functional-spec/FS-inline-citation-style.md#44-warnings-and-errors--opt-in-layout-deviations)). So a project that configures no layout, or configures one without gating it, tokenizes no line and classifies no line on the field's account, and pays not even a per-block memo — that allocation belongs to the second reader and is made only where one exists. A future consumer wanting deviations from an ungated tree is asking a question this field does not answer. Every citation in one block carries the same site.

This is the only structured output the scanner produces. Everything downstream (checking, showing, IDE diagnostics) operates on this data structure.

When file scanning runs in parallel, each per-file result is merged as though the sorted file list had been scanned sequentially: declarations for duplicate IDs keep path order, citations keep path/line/column order, and `scanned_files` keeps the sorted file order. Workspace scans use the same per-file rule with the workspace target list already loaded, so `§<alias>/<ID>` citations are still parsed during that one read of the citing file rather than by a second pass.

## 4. Inline declarations in language doc-comments

The scanner is designed so that an inline declaration — most commonly an `AR-NNN-<slug>` for an architectural spec — can live inside the **class, method, module, or package doc-comment** of any major language. This makes class-level documentation a first-class place to put architecture specs: the spec body sits with the code it describes, and a stub under `docs/architecture/` points at it through a single-line H1 of the form `# <ID>: [<path>](<path>)`.

The recognized doc-comment forms (matched as comment prefixes preceding the heading line):

| Language(s)              | Doc-comment form                                  | How the regex sees it                |
|--------------------------|---------------------------------------------------|--------------------------------------|
| Java, Kotlin, Scala      | `/** … */` (Javadoc / KDoc / Scaladoc)            | `/*` opens; ` * ` on continuation    |
| C, C++                   | `/** … */` (Doxygen) or `/// …`                   | `/*` or `//` (covers `///`)          |
| C#                       | `/// <summary>…</summary>` (XML doc)              | `//` (covers `///`)                  |
| Rust                     | `/// …` outer, `//! …` inner, `/** … */` block    | `//` covers `///` and `//!`; `/*` for block |
| TypeScript, JavaScript   | `/** … */` (JSDoc / TSDoc)                        | `/*` opens; ` * ` on continuation    |
| Go                       | `// …` block immediately above the declaration    | `//`                                 |
| Swift                    | `/// …` or `/** … */`                             | `//` or `/*`                          |
| PHP                      | `/** … */` (PHPDoc)                               | `/*` opens; ` * ` on continuation    |
| Ruby                     | `# …` lines (RDoc / YARD)                         | `#` (see note 4.1)                    |
| Python                   | `""" … """` triple-quoted docstring               | special-cased (see note 4.2)         |

This table documents the doc-comment *conventions* for the languages `grund` is built to serve. It is not the gate: the gate is the `[scan] comment_prefixes` list ([§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked)), whose default also recognizes `;` (Lisp / Scheme / Clojure), `--` (SQL / Haskell / Lua / Ada), and bare `*` / `/*` block-comment lines. Any line whose first non-whitespace run is a configured prefix can host a declaration heading or a citation; a language not in the table still works as long as its comment marker is in `comment_prefixes`.

Before declaration, section, or citation detection runs on a source file, the scanner normalizes each eligible comment/docstring line to the content the author meant:

- `//`, `///`, and `//!` line comments strip the full leading comment marker and one following space when present. Therefore `/// AR-001-router: Router`, `//! AR-001-router: Router`, and `// AR-001-router: Router` all expose the same declaration content: `AR-001-router: Router`.
- `#`, `;`, and `--` line comments strip that marker and one following space when present. Therefore Python/Ruby `# AR-001-router: Router` exposes `AR-001-router: Router`; a bare source line `AR-001-router: Router` is not a declaration outside a Python docstring, because it has no comment marker.
- Block comments strip the opener (`/*` or `/**`) and closer (`*/`) when they appear on their own content lines. Continuation lines strip one optional leading `*` plus one following space when present. Therefore ` * AR-001-router: Router` exposes `AR-001-router: Router`.
- Python triple-quoted docstrings in `.py` files enter docstring mode for both `"""` and `'''`. Delimiter-only opening and closing lines are not content; delimiter lines that also contain prose are scanned after stripping the delimiter on that side, so `"""Uses §FS-001-router."""` and an indented multi-line docstring body are both scanned as docstring content. Therefore a class or module docstring containing `AR-001-router: Router` declares `AR-001-router`, and a docstring containing `§FS-001-router` cites it.
- The normalization is line-local and deterministic. It does not parse the host language beyond recognizing the comment/docstring form above; after normalization, the same heading and citation regexes from §2.1 through §2.3 apply. Recorded source positions still point at the original file columns, not the stripped comment or docstring content columns, so LSP ranges and diagnostics cover the token the user sees in the editor.

The following inline declarations are all required to be recognized under the default scan settings:

```rust
/// AR-001-router: Router
/// Routes requests by path.

//! AR-002-module: Module architecture

/**
 * AR-003-block: Block comment spec
 * ## 1. Contract
 */
```

```go
// AR-004-handler: Handler
// Handles HTTP requests.
```

```python
"""
AR-005-service: Service
## 1. Contract
"""
class Service:
    pass
```

```ruby
# AR-006-job: Job
# Runs background work.
```

A canonical example — a Java class whose Javadoc *is* the architectural spec:

```java
/**
 * AR-event-bus: Asynchronous event distribution
 *
 * ## 1. Responsibilities
 * The event bus owns subscription state and …
 *
 * ## 2. Threading model
 * Single-writer, multi-reader …
 */
public final class EventBus { … }
```

Matched by the matching stub `docs/architecture/AR-<event-bus>.md`:

```
# AR-event-bus: [src/main/java/com/example/EventBus.java](src/main/java/com/example/EventBus.java)
```

### 4.1 Ruby and Python edge cases

- **Ruby** uses `#` as the comment marker. The declaration itself starts after that marker, so the canonical Ruby form is `# AR-<event-bus>`, not a markdown heading inside the comment.
- **Python** docstrings are not comments but string literals (`""" … """`). The scanner has a small docstring mode for `.py`: when a triple-quoted string opens, lines inside it are scanned the same way as comment continuation lines until the matching close. This lets a Python class or module docstring be a fully-featured spec home.

## 5. Why regex, not a parser

Specs live in markdown *and* in source-file doc-comments across half a dozen languages. A real parser per language would be far more code and far slower than a single line-oriented regex pass. The scheme is deliberately designed to be regex-recognizable: the heading shape is unambiguous and the citation shape is anchored on word boundaries.

The trade-off: we cannot reason about the surrounding code structure. We do not need to — IDs are syntactic, not semantic. The link in the stub heading is the only structural pointer between a stub and the code that hosts the inline spec, and it is verified by [§AR-checker.2.4](../../crates/grund-core/src/checker.rs).

The marker character recognized in citations follows [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger); the regex shape changes when the marker is reconfigured per [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable).

## 6. E2E case declarations

`E2E` is reserved in the default `[[kinds]]` set, and case-directory discovery follows the directory contract below.

The `E2E` kind is the one kind not declared by a heading line. An `E2E` declaration is a **case directory** directly under the `E2E` kind's `[[kinds]] folder` (default `e2e/cases`). The directory's name is the declared ID with the leading `{kind}` placeholder and its following literal stripped — under the default `[id] format = "{kind}-{number}-{slug}"`, a directory `007-login` declares `E2E-007-login`; under `{kind}-{slug}`, `login` declares `E2E-<login>`; under `{kind}-{number}`, `007` declares `E2E-007`. The directory name must match the format with the kind portion removed; directories that do not (e.g. `.gitkeep`, or a folder with no `expected.exit`) are skipped, so `e2e/cases/` itself never becomes a declaration. The case manifest also records non-empty `spec.refs` lines as cited-kind evidence for E2E citation-direction obligations ([§FS-config.3.9](../functional-spec/FS-config.md#39-citations--citation-direction-rules)); these manifest references do not enter the ordinary citation stream.

A case directory is recognized as a declaration only if it contains an `expected.exit` file (the minimal marker of a real case). The `Declaration` recorded for it carries the directory path with `line = 1`, an empty section set (the fixture file set is not a numbered-heading tree, so any section-bearing citation of an `E2E` ID — a `.2` suffix and so on — is a missing-section error per [§AR-checker.2.3](../../crates/grund-core/src/checker.rs)), and the deterministic, sorted list of the case's fixture files plus the invocation (`command.args` contents, or the implicit `grund check` when absent) and the expected exit code — this is the "body" [§FS-show.2.4](../functional-spec/FS-show.md#24-e2e-cases) prints. E2E declarations are never stubs, are never hosted in code, and are not reported as unused when no spec cites them.

The ordinary file walk treats each direct case directory as an E2E manifest boundary, not as repo content to scan. A root scan over `e2e/` or `e2e/cases/` still registers the case declaration through the E2E manifest pass, but it does not read the nested fixture repo under that case; an explicit path inside the fixture repo remains scannable.

Citations of an `E2E` ID resolve like any other: an `E2E-<name>` cite from a spec ("proven by …") is a dangling-ref error ([§AR-checker.2.2](../../crates/grund-core/src/checker.rs)) when no `e2e/cases/<name>/` case directory exists.
