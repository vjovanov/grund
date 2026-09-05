# DF-index-entry-form: an index entry is one full link per ID, and nothing else about the page

**Status:** Accepted
**Date:** 2026-08-25

## 1. Context

A multi-file kind (`folder = "…"`, [§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds)) conventionally keeps an index README in its home, and `grund init --docs` writes the convention into the one it scaffolds ([§FS-init.2.1](../../functional-spec/FS-init.md#21-files-written-updated-or-left-in-place)). Nothing verified it. Measured on this repository at v0.11.0: `docs/discussions/README.md` announced "Current proposals:" and listed 9 of the 11 that existed, `grund check` was green over it, and the two largest folder kinds — `DF` with 34 declarations and `DA` with 7 — had no index at all.

One direction was already checked: an index row is an ordinary citation, so an index naming a deleted ID dangles ([§FS-check.3.1](../../functional-spec/FS-check.md#31-dangling-citation)). Only the other direction had no rule.

The rule that follows has to be narrow. `docs/functional-spec/README.md` groups 21 entries under six curated headings with hand-shortened descriptions; it is the best index in the tree, and a rule that dictated a table would report it. So the question is not "what should an index look like" but "what is the smallest thing about an index that can be checked without taking the pen away from its author".

## 2. Decision

### 2.1 The entry is a recognized citation of the ID, written as a full Markdown link

Two conditions, and either one unmet is a finding ([§FS-check.3.18](../../functional-spec/FS-check.md#318-declaration-missing-from-its-kinds-index), [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link)). The index is a *file* index: its job is to get a reader from the folder to the declaration, and a bare `§<ID>` in it is a promise the reader cannot follow.

### 2.2 "Full link" is the link `fmt` would write here, not "has an anchor"

The required form is the one `grund fmt --cross-refs` emits ([§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)): the relative path to the declaration's home plus the heading anchor under the active `anchor_format`. Stated as "must carry an anchor" the rule would be wrong twice over — a declaration whose home is a source file gets a bare file path with no anchor, and `anchor_format = "none"` drops anchors everywhere. `docs/architecture/README.md` carries the first case today — its directly enrolled source declaration for [§AR-checker](../../../crates/grund-core/src/checker.rs) is a bare link to the Rust file that holds the body, anchorless and correct as written.

Naming `fmt`'s output as the target also keeps the anchor algorithm in one command. `check` therefore requires the *shape* and never re-derives the URL ([§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link)): a heading rename that rots an anchor is a one-line `fmt` diff on the next pass ([§FS-fmt.6.3](../../functional-spec/FS-fmt.md#63-idempotency-and-re-derive)), not a second finding in a second implementation of the same slugger.

### 2.3 One link per ID, not every mention

An ID is satisfied by its first full link; every other occurrence in the index is untouched and is never a finding. `docs/architecture/README.md` explains the stub convention in prose and mentions `§AR-scanner.4` and `§AR-checker` in inline code spans, which `fmt` deliberately never wraps ([§FS-fmt.6.4](../../functional-spec/FS-fmt.md#64-what-is-never-wrapped)). Demanding every occurrence would report a paragraph that is right as written and that the fix command cannot touch.

For the same reason a citation *inside* an inline code span is not an entry at all: it neither satisfies [§FS-check.3.18](../../functional-spec/FS-check.md#318-declaration-missing-from-its-kinds-index) nor triggers [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link). Counting it as a bare entry would produce an error whose fix command declines to act — the state [§FS-check.3.13](../../functional-spec/FS-check.md#313-number-only-shorthand-citation) already refuses to create for the number-only shorthand.

And inline code is not the only such place, which is why the rule is stated as the predicate and not as a list of exemptions. **A citation is a bare entry exactly when the next `grund fmt --write` would wrap it**, and the cross-reference pass wraps a marker-prefixed citation ([§FS-fmt.6.5](../../functional-spec/FS-fmt.md#65-interaction-with---marker)), in a `.md` file ([§FS-fmt.6.1](../../functional-spec/FS-fmt.md#61-scope)), outside [§FS-fmt.2.3](../../functional-spec/FS-fmt.md#23-what-is-never-rewritten)'s never-rewrite zones and [§FS-fmt.6.4](../../functional-spec/FS-fmt.md#64-what-is-never-wrapped)'s exemptions. Two configurations reach the difference, and each of them was a permanent error under the list-of-exemptions reading:

- a repository on `[reference] strict = false`, whose index lists IDs as bare tokens with no marker — `fmt --write` writes nothing there, because without `--marker` a bare token stays bare;
- an `index` naming a file the cross-reference pass does not run on at all — closed a second way, by [§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds) requiring `index` to name a `.md` file, since an error class whose licence is that `fmt` fixes it cannot apply where `fmt` never runs.

An ordinary Markdown link whose destination happens to have an ID-shaped file name, `See the [login spec](FS-login.md)`, does not reach this predicate at all: the token in the *destination* is not a citation off strict mode either ([§FS-check.1.1](../../functional-spec/FS-check.md#11-recognized-citations)), so there is nothing here for `fmt --write` to ever be asked to wrap. That the scanner once saw a citation there was a separate, older defect ([grund#131](https://github.com/vjovanov/grund/issues/131)), fixed ahead of this rule rather than worked around by it.

`Citation::shorthand_rewritable` is the same predicate, written for the same reason one rule earlier; the index rules reuse the machinery under it rather than growing a second copy that could drift.

The predicate runs one way only. It withholds the *error*; it does not withhold credit for an entry that is already a link. A hand-written `[FS-login](FS-login.md)` around an unmarked token is a link a reader can follow, `fmt` would leave it alone, and [§FS-check.3.18](../../functional-spec/FS-check.md#318-declaration-missing-from-its-kinds-index) is satisfied by it.

### 2.4 Layout is free

No table, no ordering, no headings, no placement. Grouping, recommended reading order and prose around the link set are the author's, and the curated indexes in this tree stay legal exactly as they are.

### 2.5 The walk is the folder's whole subtree, and a stub-and-inline pair collapses

`DISC`'s folder is `docs/discussions` while every proposal lives in `docs/discussions/proposals/`, so a top-level-only rule would check nothing here. And a stub under `folder` pointing at an inline declaration elsewhere is one ID with two sites: it collapses to one entry the way `grund list` collapses it ([§FS-list.2](../../functional-spec/FS-list.md#2-behaviour)), pointing at wherever the body lives.

### 2.6 Where the two findings land

A **missing entry** is anchored at the *declaration's heading*, with the index file named in the message. The subject is the declaration and the fix is an edit to the index, which argues both ways — the tiebreak is uniformity: a folder whose index file does not exist has no line to point at, and every declaration has one, so anchoring at the declaration is the only rule that does not need a second rule beside it for the empty case.

An **unlinked entry** is anchored at *the citation's line in the index*. That finding is about a site that exists, and it is the line `grund fmt --write` rewrites.

### 2.7 A canonical bare-ID link enrolls an external inline declaration

An inline declaration of the indexed kind whose only home is a non-Markdown source file outside `folder` may join the kind's index without a stub. Its enrollment is the exact link `grund fmt --cross-refs` writes for a marker-prefixed, unqualified citation of the bare ID from that index to the source home ([§FS-check.3.18](../../functional-spec/FS-check.md#318-declaration-missing-from-its-kinds-index), [§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)). The link is both membership and entry. It creates no declaration, so `show`, `list`, duplicate detection, and every other declaration consumer keep the source doc-comment as the canonical home.

This is intentionally stricter than §2.1's entry for a declaration already under `folder`, where `check` requires the wrapper shape and leaves the target to `fmt`. Here the target is the discriminator: without exact equality to `fmt`'s derived destination, a same-kind citation in surrounding index prose would silently become structural membership. The citation must name the whole ID rather than a section, must be unqualified, and must carry the marker; section references, cross-project references, custom links, and links to external Markdown declarations stay ordinary references. A bare marker-prefixed whole-ID mention becomes enrollment if a formatting pass writes the canonical wrapper around it — the persisted canonical link, not guessed layout or prose intent, is the signal.

No table row, list item, heading, or managed region is required. That preserves §2.4's layout freedom and makes existing table and list indexes equally capable of enrollment. No new citation grammar or config key is required either: both would spread a one-file membership question into the scanner, LSP, formatter, completion, and configuration surfaces. The one extra comparison reuses the formatter's canonical target function, so path and anchor semantics still have one owner.

## 3. Alternatives considered

**A rendered index** — a `fmt`-written managed block below the curated prose, with `fmt --check` reporting drift. It is the obvious follow-on and is deliberately not this decision: the check is worth having on its own, it does not presume the generator's design, and the hand-curated indexes here want the verification without the writer. The one thing the renderer would buy is a same-release verdict flip for the missing-entry half ([§DF-index-compatibility-ramp](DF-index-compatibility-ramp.md#df-index-compatibility-ramp-a-findings-ramp-follows-its-fix-command-not-the-size-of-the-offence)), which is a reason to build it later, not a dependency to take on now.

**`indexed = false`, or simply "a folder README is the index".** The last one collapses into a claim that is not true of this tree: `tests/e2e/README.md` documents the case layout and names behaviours in English, never `§E2E-` IDs. Naming the file — with `false` as the opt-out — keeps "which file is the index" a fact the config states rather than one the checker guesses ([§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds)).

**Checking the link target as well as its shape.** Rejected in §2.2: it duplicates the anchor algorithm for no new coverage, and [§AR-ci](../../architecture/AR-ci.md#ar-ci-ci-mirrors-the-local-pre-commit-gate) already runs `grund fmt --write` and `lychee` in the same gate as `grund check --full`.

## 4. Consequences

The four hand-curated indexes in this repository were already conformant: 50 citations across them, 48 already links, and the two exceptions are the inline-code prose §2.3 exempts. The cost of the rule here was zero, which is the evidence that it is narrow enough.
