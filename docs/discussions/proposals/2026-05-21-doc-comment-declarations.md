# DISC-doc-comment-declarations: Declarations live only in class/method doc-comments, never inline

## Status

Proposed. Roadmapped as [§RM-doc-comment-declarations](../../roadmap.md#rm-doc-comment-declarations-declarations-only-in-classmethod-doc-comments). If accepted, folds into [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) (the recognizer), [§DF-code-declarations-drop-hash](../../decisions/functional/DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix) (the form table), and [§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked) (the new `[scan]` switch).

## Context

Today a code-resident declaration is recognized behind *any* configured `[scan] comment_prefixes` marker. [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) states it outright: `///`, `//!`, and a plain `//` line comment "all expose the same declaration content." The recognizer is `decl_re` in `crates/grund-core/src/grammar.rs`, and `comment_prefix_regex` widens `//` to `//[/!]?` — the third character is *optional*, so a bare `//` matches.

The consequence is a phantom declaration. A line like

```rust
fn check() {
    // FS-check.3.9 / something: in strict mode the heading level must match
    ...
}
```

is read as a declaration of `FS-check`, sitting *inside a function body*, colliding with the real `# FS-check` declaration under `docs/functional-spec/` and producing a `duplicate declaration` diagnostic. The usual way to hit it: write a citation note in a plain comment and forget the `§` marker on the first token. In strict mode a bare ID-shaped token is correctly ignored as a *citation* ([§FS-config.3.1](../../functional-spec/FS-config.md#31-reference--citation-form)), but the *declaration* recognizer has no equivalent marker gate.

This is wrong twice over against [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)'s own framing. That section already describes an inline declaration as living in "the **class, method, module, or package doc-comment**" — a `///`/`/** */`/docstring that documents the thing it sits on. A plain `//` is a *regular* comment, not documentation; and a comment inside a function body documents nothing. The recognizer is simply looser than the spec it implements: it accepts the regular-comment marker, and it does not require the comment to be attached to a definition.

The fix is to make the recognizer match the framing: **a code declaration is recognized only inside a doc-comment that documents the immediately-following definition — never an inline or trailing comment.** Citations are untouched; only declaration recognition tightens.

## Proposed shape

### The rule

A code-resident declaration is recognized only when it sits in a **doc-comment that documents the immediately-following definition** — a class, struct, enum, trait/interface, function, method, module, or package. A plain inline comment or a trailing `// note` never hosts a declaration. A `§<ID>` *citation* may still appear in any comment, including a plain `//` note ([§FS-config.3.1](../../functional-spec/FS-config.md#31-reference--citation-form) already gates citations on the marker; this proposal does not change that).

Languages split into two recognizer families by how they mark a doc-comment:

1. **Marker languages** — a *distinct* doc-comment marker exists, so the marker alone separates documentation from a regular comment. Gate on the marker: a declaration is recognized only behind `///`/`//!` (Rust, C#, Swift, Doxygen) or a `/** … */` / `/*! … */` block (Javadoc, JSDoc, KDoc, Scaladoc, PHPDoc, Doxygen); a bare `//` or `/* … */` is a regular comment and never declares. This is the majority of the top 20.

2. **Position languages** — the regular comment marker *is* the doc-comment marker (Go `//`, Ruby `#`, shell `#`, …), so the marker cannot separate the two. Gate on **position**: the comment block hosts a declaration only when it is immediately followed (no blank line) by a *definition-starter* line. Definition-starters are a small per-language set of line-anchored prefixes — Go `func|type|var|const|package`, Ruby `class|module|def`, and so on — matched as one anchored regex per line. This is recognition, not parsing ([§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing), [§AR-scanner](../../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)): it is line-local, deterministic, spans no scope, and a missed keyword is fixed in config, not by growing a parser.

Python docstrings are already special-cased by [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) and stay as-is: a `""" … """` is by construction the documentation of its module, class, or def, so a docstring declaration needs no extra gate.

### Top-20 language coverage

| Language | Extensions | Doc-comment (declaration host) | Plain comments (citation only) | Gate |
|---|---|---|---|---|
| Rust | `rs` | `///`, `//!`, `/** */`, `/*! */` | `//`, `/* */` | marker |
| C# | `cs` | `///` (XML doc) | `//`, `/* */` | marker |
| C / C++ / Objective-C | `c` `h` `cpp` `cc` `hpp` `hh` `m` `mm` | `/** */`, `/*! */`, `///`, `//!` (Doxygen) | `//`, `/* */` | marker |
| Java | `java` | `/** */` (Javadoc) | `//`, `/* */` | marker |
| Kotlin | `kt` | `/** */` (KDoc) | `//`, `/* */` | marker |
| Scala | `scala` | `/** */` (Scaladoc) | `//`, `/* */` | marker |
| Swift | `swift` | `///`, `/** */` | `//`, `/* */` | marker |
| JavaScript | `js` `jsx` `mjs` | `/** */` (JSDoc) | `//`, `/* */` | marker |
| TypeScript | `ts` `tsx` | `/** */` (TSDoc) | `//`, `/* */` | marker |
| PHP | `php` | `/** */` (PHPDoc) | `//`, `#`, `/* */` | marker |
| Dart | `dart` | `///`, `/** */` | `//`, `/* */` | marker |
| Python | `py` | `""" """`, `''' '''` docstring | `#` | marker (docstring; already special-cased) |
| Go | `go` | `//` / `/* */` immediately above a definition | `//`, `/* */` elsewhere | position |
| Ruby | `rb` | `#` immediately above a definition (RDoc/YARD) | `#` elsewhere | position |
| Lua | `lua` | `---`, `--[[ ]]` (LDoc) | `--` | marker (`---`) |
| Haskell | `hs` | `-- \|`, `-- ^`, `{-\| -}` (Haddock) | `--`, `{- -}` | marker |
| Shell / Bash | `sh` `bash` | `#` immediately above a definition | `#` elsewhere | position |
| SQL | `sql` | `--` immediately above a `CREATE` | `--`, `/* */` elsewhere | position |
| Lisp / Clojure / Elisp | `clj` `el` `lisp` | docstring forms / `#` block above a `defn`/`def` | `;` | position |

The table documents *conventions*; the gate stays config-driven ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)), exactly as [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) already says the existing prefix list — not the table — is the real gate. A language not listed still works: marker-style if its doc marker is in the doc-comment set, position-style if its regular comment marker is, falling back to "any comment declares" when the switch is off.

### The disable switch

A new `[scan]` key, default on:

```toml
[scan]
declarations_in_doc_comments = true   # default
```

When `false`, the legacy recognizer is restored verbatim — any `comment_prefixes` marker hosts a declaration, i.e. exactly the [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) behavior shipping today. It lives in `[scan]` beside `comment_prefixes` ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)). It is a *recognizer* toggle, not a severity or exit-code knob ([§FS-non-goals.9](../../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)), and both installs read the same toml and agree on the same result ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)), so the "two installs agree" contract is untouched.

### How to build it

- **Split the prefix alternation.** Today one `comment_prefix_regex` feeds both `decl_re` and citation detection in `grammar.rs`. Introduce a separate *declaration-prefix* regex, selected by file extension, holding only doc-comment markers; citation detection keeps using the existing any-comment set unchanged.
- **Marker languages:** `decl_re` uses `///|//!` and the `/**`/`/*!` block openers (plus the ` * ` continuation that [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) already normalizes). The current `//` → `//[/!]?` widening becomes `//[/!]` — the doc third character is mandatory; bare `//`/`/*` drop out of the declaration alternation.
- **Position languages:** keep the bare marker in the declaration-prefix set, but add a post-match positional check — emit the declaration only if the candidate comment block (the boundaries `inline_citation_sites` already computes in `scanner.rs`) is immediately followed by a definition-starter line. Definition-starters are built-in per-extension defaults plus a config override.
- **Switch off:** `decl_re` uses today's widened alternation; nothing else changes.

## Boundaries

- **Citations are untouched.** Only declaration recognition tightens. A `§<ID>` in a plain `//` note still resolves and still climbs ([§FS-check](../../functional-spec/FS-check.md#fs-check-grund-validates-every-reference-in-a-repo)). The screenshot scenario stops being a *declaration*; if it carried a real `§` marker it stays a *citation*.
- **No AST.** The definition-starter check is one line-anchored regex per language — line-local, deterministic, no scope, no multi-line state ([§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing), [§AR-scanner](../../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)). False negatives are fixed in config, never by parsing.
- **Markdown is unchanged.** `.md` declarations still require `# <ID>:` for the H1 ([§DF-code-declarations-drop-hash](../../decisions/functional/DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix)); this proposal only touches code doc-comments.
- **Multi-declaration doc-comments still work.** Co-located declarations share one doc-comment ([§DF-code-declarations-drop-hash.2.1](../../decisions/functional/DF-code-declarations-drop-hash.md#21-multi-declaration-doc-comments), [§FS-show.2.3](../../functional-spec/FS-show.md#23-inline-declarations-in-code-and-doc-comments)); the gate applies once to the whole block, so an `AR-`/`FS-` pair in one `///` block is unaffected.

## Migration / blast radius

- [§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) table and normalization paragraph split into "declaration host (doc-comment)" vs "citation host (any comment)".
- [§DF-code-declarations-drop-hash](../../decisions/functional/DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix) form table gains a "plain `//` is not a declaration host" note, and its Go row gains the positional caveat.
- Two Rust e2e fixtures that currently declare via plain `//` (`e2e/cases/show-inline-rust`, `e2e/cases/markdown-to-rust-inline-valid`) move `//` → `///`.
- New e2e cases: a plain-`//` ID note is *not* a declaration (the screenshot); the switch off restores legacy recognition; Go positional accept and reject; a marker-language reject.

Turning the gate on by default changes what an existing tree recognizes — a tree that declared via plain `//` loses those declarations. Per [§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning) this rides a `grund_config_version` bump with a migration note, and the switch is the escape hatch. (See open questions on whether default-on without a bump is acceptable, since idiomatic trees are unaffected.)

## Relation to other work

- [§FS-check.4.7](../../functional-spec/FS-check.md#47-declaration-near-miss) warns on a heading shaped like a declaration that the recognizer ignores. A plain-`//` ID line is exactly such a near-miss once this gate drops it, so the two compose: the gate stops the phantom declaration, the near-miss optionally surfaces "this looks like a declaration but is being ignored."
- [§GOAL-multi-language](../../goals.md#goal-multi-language-same-engine-three-platforms): the gate and the language table must produce byte-identical results across all three bindings.
- [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) / [§GOAL-zero-config](../../goals.md#goal-zero-config-works-on-any-conformant-tree): default-on, no config to write; the switch is only for trees that relied on the old behavior.

## Open questions

- Config key name and namespace: `[scan] declarations_in_doc_comments` as proposed, or something shorter, or under `[reference]`?
- Default-on *with* a `grund_config_version` bump ([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)), or without — given idiomatic trees (`///` in Rust, `/** */` in Java) are unaffected and only plain-`//` declarers break?
- Are position-language definition-starters worth shipping in v1, or should position languages keep "any comment declares" until the starters are proven on real corpora — i.e. gate marker languages first, position languages second?
- Should a `///` doc-comment that is *not* followed by a definition (a floating doc comment) also be rejected, or is marker-presence enough? Rust's own compiler already warns on unattached `///`, which argues for "enough."
- Block-comment opener disambiguation (`/**`/`/*!` doc vs `/*` regular) plus the ` * ` continuation line — confirm multi-line Javadoc/JSDoc declarations still normalize correctly ([§AR-scanner.4](../../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)).
- The definition-starter sets risk drift as languages evolve. Built-in defaults plus config override, or config-only with conservative defaults?
