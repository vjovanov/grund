# DF-index-entry-form: an index entry is one full link per ID, and nothing else about the page

**Status:** Accepted
**Date:** 2026-08-25

## 1. Context

A multi-file kind (`folder = "…"`, [§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-prefixes)) conventionally keeps an index README in its home, and `grund init --docs` writes the convention into the one it scaffolds ([§FS-init.2.1](../../functional-spec/FS-init.md#21-files-written-updated-or-left-in-place)). Nothing verified it. Measured on this repository at v0.11.0: `docs/discussions/README.md` announced "Current proposals:" and listed 9 of the 11 that existed, `grund check` was green over it, and the two largest folder kinds — `DF` with 34 declarations and `DA` with 7 — had no index at all.

One direction was already checked: an index row is an ordinary citation, so an index naming a deleted ID dangles ([§FS-check.3.1](../../functional-spec/FS-check.md#31-dangling-citation)). Only the other direction had no rule.

The rule that follows has to be narrow. `docs/functional-spec/README.md` groups 21 entries under six curated headings with hand-shortened descriptions; it is the best index in the tree, and a rule that dictated a table would report it. So the question is not "what should an index look like" but "what is the smallest thing about an index that can be checked without taking the pen away from its author".

## 2. Decision

### 2.1 The entry is a recognized citation of the ID, written as a full Markdown link

Two conditions, and either one unmet is a finding ([§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index), [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link)). The index is a *file* index: its job is to get a reader from the folder to the declaration, and a bare `§<ID>` in it is a promise the reader cannot follow.

### 2.2 "Full link" is the link `fmt` would write here, not "has an anchor"

The required form is the one `grund fmt --cross-refs` emits ([§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)): the relative path to the declaration's home plus the heading anchor under the active `anchor_format`. Stated as "must carry an anchor" the rule would be wrong twice over — a declaration whose home is a source file gets a bare file path with no anchor, and `anchor_format = "none"` drops anchors everywhere. `docs/architecture/README.md` carries the first case today — its collapsed stub-and-inline pair for [§AR-checker](../../../crates/grund-core/src/checker.rs) is a bare link to the Rust file that holds the body, anchorless and correct as written.

Naming `fmt`'s output as the target also keeps the anchor algorithm in one command. `check` therefore requires the *shape* and never re-derives the URL ([§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link)): a heading rename that rots an anchor is a one-line `fmt` diff on the next pass ([§FS-fmt.6.3](../../functional-spec/FS-fmt.md#63-idempotency-and-re-derive)), not a second finding in a second implementation of the same slugger.

### 2.3 One link per ID, not every mention

An ID is satisfied by its first full link; every other occurrence in the index is untouched and is never a finding. `docs/architecture/README.md` explains the stub convention in prose and mentions `§AR-scanner.4` and `§AR-checker` in inline code spans, which `fmt` deliberately never wraps ([§FS-fmt.6.4](../../functional-spec/FS-fmt.md#64-what-is-never-wrapped)). Demanding every occurrence would report a paragraph that is right as written and that the fix command cannot touch.

For the same reason a citation *inside* an inline code span is not an entry at all: it neither satisfies [§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index) nor triggers [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link). Counting it as a bare entry would produce an error whose fix command declines to act — the state [§FS-check.3.13](../../functional-spec/FS-check.md#313-number-only-shorthand-citation) already refuses to create for the number-only shorthand.

### 2.4 Layout is free

No table, no ordering, no headings, no placement. Grouping, recommended reading order and prose around the link set are the author's, and the curated indexes in this tree stay legal exactly as they are.

### 2.5 The walk is the folder's whole subtree, and a stub-and-inline pair collapses

`DISC`'s folder is `docs/discussions` while every proposal lives in `docs/discussions/proposals/`, so a top-level-only rule would check nothing here. And a stub under `folder` pointing at an inline declaration elsewhere is one ID with two sites: it collapses to one entry the way `grund list` collapses it ([§FS-list.2](../../functional-spec/FS-list.md#2-behaviour)), pointing at wherever the body lives.

### 2.6 Where the two findings land

A **missing entry** is anchored at the *declaration's heading*, with the index file named in the message. The subject is the declaration and the fix is an edit to the index, which argues both ways — the tiebreak is uniformity: a folder whose index file does not exist has no line to point at, and every declaration has one, so anchoring at the declaration is the only rule that does not need a second rule beside it for the empty case.

An **unlinked entry** is anchored at *the citation's line in the index*. That finding is about a site that exists, and it is the line `grund fmt --write` rewrites.

## 3. Alternatives considered

**A rendered index** — a `fmt`-written managed block below the curated prose, with `fmt --check` reporting drift. It is the obvious follow-on and is deliberately not this decision: the check is worth having on its own, it does not presume the generator's design, and the hand-curated indexes here want the verification without the writer. The one thing the renderer would buy is a same-release verdict flip for the missing-entry half ([§DF-index-compatibility-ramp](DF-index-compatibility-ramp.md#df-index-compatibility-ramp-a-findings-ramp-follows-its-fix-command-not-the-size-of-the-offence)), which is a reason to build it later, not a dependency to take on now.

**`indexed = false`, or simply "a folder README is the index".** The last one collapses into a claim that is not true of this tree: `e2e/README.md` documents the case layout and names behaviours in English, never `§E2E-` IDs. Naming the file — with `false` as the opt-out — keeps "which file is the index" a fact the config states rather than one the checker guesses ([§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-prefixes)).

**Checking the link target as well as its shape.** Rejected in §2.2: it duplicates the anchor algorithm for no new coverage, and [§AR-ci](../../architecture/AR-ci.md#ar-ci-ci-mirrors-the-local-pre-commit-gate) already runs `grund fmt --write` and `lychee` in the same gate as `grund check --full`.

## 4. Consequences

The four hand-curated indexes in this repository were already conformant: 50 citations across them, 48 already links, and the two exceptions are the inline-code prose §2.3 exempts. The cost of the rule here was zero, which is the evidence that it is narrow enough.
