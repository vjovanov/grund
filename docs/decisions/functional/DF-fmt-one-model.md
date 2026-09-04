# DF-fmt-one-model: `fmt` is the shared verified model plus a write step, and completeness is a precondition rather than a convention

**Status:** Accepted
**Date:** 2026-09-05

## 1. Context

Twelve defects in `grund fmt` have been fixed as twelve bugs. Reading them together, they are one:

| What went wrong | Root cause, not symptom |
|---|---|
| A shorthand every reader took for a citation was dropped by every pass while its declaration was reported "never cited" | A token shape with no defined verdict anywhere |
| `--write` spliced the canonical slug into a longer token (`<§>FS-042-User-Login` → `<§>FS-042-user-login-User-Login`) | `fmt`'s token recognizer was a second grammar, boundary-anchored differently from the scanner's |
| LSP live expansion rewrote to the wrong declaration at the first digit | A semantic rewrite committed on an incomplete token |
| `check` reported sites `fmt` was forbidden to rewrite, leaving a tree red with nothing to run | Two independently built predicates for one question, evaluated on different inputs |
| `<§>SPEC-001→SPEC-003` expanded into a citation of the wrong live declaration, `check` green after | The rewrite asked a lexical question ("does the token end here?") and never "is this token used as a citation?" |
| `--check` was not a preview of `--write` | The dry run was a separate computation, not the same one with the write withheld |
| `--write` rewrote a file through an out-of-root symlink | Ownership judged by the in-tree name, never by the target's identity |
| `fmt` alone was silent about unreadable paths while `check`, `refs` and `list` exited `2` | A private walk wrapper discarded errors — "nowhere to put a scan error" — against a contract [§FS-fmt.3](../../functional-spec/FS-fmt.md#3-outputs) already stated |
| The dry run listed a rewrite `--write` refuses, so `--check` could never go green | The refusal lived only on the write path |
| A member's unreadable path rendered as a different real file in the run's output | `fmt`'s private render base |
| The strict path named only the first unreadable file; the plain path named them all | The refusal was built beside the reporting path, not from it |
| `--write` with no path rewrote against a partial set; with a path it refused | The completeness invariant was a property of one producer, not a precondition of the consumer |

`fmt` was built as a private text-transformation pipeline — its own lexer, walker, error plumbing, predicates, render base and report — when its contract is the shared verified model plus a write step. Seven of the twelve are that private path disagreeing with the shared one; four are a write committed on evidence short of proof, and one is both.

The evidence that this is a single thing is in the fix commits themselves, which keep independently rediscovering fragments of one sentence: *"one predicate now serves both"*, *"the same routing the scanner uses"*, *"fmt walks the tree check walks and owes the same account"*, *"both now render where every other path fmt prints does"*, *"`--check` predicts `--write` again"*. Five fixes, five clauses of a contract nobody had written.

The write half carries more weight, because `fmt` is the one command whose mistakes are structurally invisible afterwards. A wrong expansion is a well-formed citation of a real declaration, so no later pass can see it — [issue #81](https://github.com/vjovanov/grund/issues/81) was found by a human re-reading a 5,750-line diff. That is [§DF-shorthand-numeric-run.2.7](DF-shorthand-numeric-run.md#27-invention-is-reported-in-full-whatever-the-rule-decides) and [§REQ-no-wrong-citation.3](../../requirements/REQ-no-wrong-citation.md#3-no-wrong-write) stated as a defect class.

**Writing the principle in the spec has already been tried, twice, and failed twice.** The exit-`2` contract was in [§FS-fmt.3](../../functional-spec/FS-fmt.md#3-outputs) *before* the silent-`fmt` defect, and a code comment shows its author knew of the contract and chose plumbing convenience over it. [Issue #105](https://github.com/vjovanov/grund/issues/105) violates "an unreadable path is fatal up front" — a sentence added to [§FS-fmt.3](../../functional-spec/FS-fmt.md#3-outputs) by the very pull request whose code violated it. A principle enforced per code path by hand does not survive the next code path, and this one did not survive its own change.

## 2. Decision

### 2.1 The principle is a spec point with four checkable rules, not a paragraph

[§FS-fmt.7](../../functional-spec/FS-fmt.md#7-one-model-and-a-write-step) states the contract, and states it as four numbered rules so each is citable and each has a command that decides it: scope-equivalence ([§FS-fmt.7.1](../../functional-spec/FS-fmt.md#71-scope-equivalence)), reader-equivalence ([§FS-fmt.7.2](../../functional-spec/FS-fmt.md#72-reader-equivalence)), preview-equivalence ([§FS-fmt.7.3](../../functional-spec/FS-fmt.md#73-preview-equivalence)), and completeness as a precondition ([§FS-fmt.7.4](../../functional-spec/FS-fmt.md#74-no-write-without-a-complete-model)).

The load-bearing part is the rules, not the prose. Prose is what was tried twice. A rule earns its place here only by naming the artifact that fails when it is broken.

### 2.2 The equivalences are general properties over a corpus, not one case each

Every rule above already has instance coverage — written by the individual fix that discovered it, pinning the one tree shape that defect had. That coverage is why all three hold today and none of them is a rule: the next code path is not one of the shapes.

So rules 7.1 to 7.3 are asserted as properties over a corpus of tree shapes — clean, strict-aborting, partial, scoped, and workspace — and a new form of the command joins the corpus rather than earning a case of its own. These are cross-run comparisons, so they are integration tests: a golden compares one run's bytes to a file and can state no equality between two runs ([§FS-fmt.7.5](../../functional-spec/FS-fmt.md#75-measurable)).

### 2.3 Reader-equivalence is set equality, not byte equality

The rule is equality of the **(path, reason)** pairs, with `nothing was rewritten: ` named as the one licensed difference in the line.

Byte equality would be the stronger claim and it is the wrong one: [§FS-fmt.3](../../functional-spec/FS-fmt.md#3-outputs) requires that prefix precisely so two exit `2`s that mean opposite things are not spelled the same — one says the tree was edited and the view of it was short, the other that the tree was not touched. A reader deciding whether to re-run, revert, or fix a link needs to know which. A §7 demanding byte equality would contradict §3 on the day it landed, and the contradiction would be resolved by whichever of the two the next author read first.

### 2.4 The rule binds `fmt` to `check`'s model, not `check`'s findings to `fmt`'s capability

This is the answer to the cost this decision has already incurred once. Coupling the two commands in the second direction — narrowing what `check` may report to what `fmt` is able to rewrite, which is what [§FS-fmt.2.4](../../functional-spec/FS-fmt.md#24-shorthand-to-canonical) does for [§FS-check.3.13](../../functional-spec/FS-check.md#313-number-only-shorthand-citation) — is what created [issue #83](https://github.com/vjovanov/grund/issues/83): a citation inside a string literal can never be flagged at all, by any command, because one consumer of the model cannot act on it.

Both directions read as "one model, one answer", which is why the second one was easy to write. They are opposite trades. A reader is owed every true finding about the tree. A writer is owed only the findings it can safely act on. [§FS-fmt.7](../../functional-spec/FS-fmt.md#7-one-model-and-a-write-step) takes the first direction and licenses no more of the second; #83 stays a live defect with a decided shape, rather than a precedent.

### 2.5 Completeness is a type the consumer demands, because prose has failed twice

Rules 7.1 to 7.3 are behavioral and testable. Rule 7.4 is structural: the code that consumes a declaration set accepts only a value carrying the proof that the scan producing it met no error, obtainable one way — a construction that checks — so the invariant cannot be restated, forgotten, or optimized away at a call site.

The asymmetry is deliberate and is the whole argument of this record. A guard written beside each call site is re-decided by every new call site, by whoever writes it, in the one moment least able to judge it: #105 is exactly that, a caller that had a declaration set at hand and used it. A precondition on the consumer is re-decided by nobody.

Two facts about this repository decide the shape rather than the taste of whoever implements it. `grund-core` is assembled by `include!()` — `lib.rs` is a list of includes and the crate is one flat module — so a newtype with a "private" field written in any of those files is constructible by every sibling file, and the rule would be a convention again in a costume. The guarantee needs a real `mod` whose only public way in is the checking constructor. And the two files that would obviously host it cannot: `fmt.rs` sits one line under its file-size budget and `api.rs` sits under an accepted ceiling, so the code goes in a sibling file with its own `include!()` line, which is the move this repository's own tooling prescribes.

### 2.6 The compile-time rule is checked by compiling the wrong program

Rule 7.4 has no runtime test, because there is nothing to run: the assertion is that a program does not build. Nor can a `compile_fail` doctest state it — a doctest compiles as a separate crate against `grund-core`'s public API, so it can only exercise the *external* bypass, while the mistake #105 actually was is a sibling file *inside* the crate. It would also need a `cargo test --doc` step added to two gate files, since the gate runs `--all-targets`, which excludes doctests: a new gate step that proves an adjacent property is worse than no step, because it reads as coverage.

`scripts/check_fmt_complete_findings.sh` is the check instead. It makes the one substitution that was #105 — the workspace formatter handing the consumer a project's raw findings — compiles the crate, restores the tree, and fails when the compile succeeded. It is deliberately **not** in the pre-commit gate: it edits tracked source, and a step that leaves the working tree modified when interrupted is the wrong thing to run before every commit. It is the acceptance check a reviewer runs, and it is named in [§FS-fmt.7.5](../../functional-spec/FS-fmt.md#75-measurable) so it is findable from the rule it decides.

## 3. Consequences

- Four `fmt` rules become citable, so code that upholds one says which one, and a future defect in this class is a violation of a named rule rather than a new bug.
- The equivalence suites grow a fixture corpus rather than a case per defect. A new tree shape is added once and every rule is asserted over it.
- The declaration-set consumer takes a proof-carrying type. Every producer of that type pays a checked construction, and the workspace path that reused a member's scan pays one too — a check it should always have made.
- `grund-core` grows one file and one `include!()` line, and gains its first real `mod` inside the flat include tree. That is a small departure from the crate's layout and is justified by the one thing a `mod` provides that an `include!` does not: privacy from siblings, which is the entire mechanism here.
- **The cost this does not pay off.** #83 is left standing (§2.4). A shared model is slower to change than a private one, and a `fmt` behavior that needs a walk `check` does not do now has to argue for it in the spec first rather than growing a private pass. That is the intended friction; it is still friction, and the twelve defects above are what it is priced against.
- Rule 7.4 is enforced by the compiler and by a script nobody's gate runs (§2.6). Its weakest link is a reviewer who does not run the script; the alternatives were weaker or dishonest.

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| State the principle in [§FS-fmt.3](../../functional-spec/FS-fmt.md#3-outputs) and stop | Tried twice, failed twice — once against a contract the violating author had read, once in the same pull request that wrote the sentence (§1). |
| Rules 7.1 to 7.3 only; leave completeness as a documented invariant | That is the state that produced #105. The three equivalences hold today; the invariant is the one of the four that does not, and it is the one guarding the write half. |
| Byte equality for reader-equivalence, as the issue words it | Contradicts [§FS-fmt.3](../../functional-spec/FS-fmt.md#3-outputs)'s deliberate `nothing was rewritten: ` prefix on the day it lands (§2.3). |
| One equivalence case per defect, as before | It is what the repository already has, and it is why all three rules hold today and none is a rule. A case pins the shape that broke; a property pins the next one. |
| `trybuild` for the compile-fail assertion | A new dev-dependency whose `.stderr` goldens track compiler versions, against a CI matrix of several operating systems — a recurring maintenance cost for an assertion about the *external* API, which is not where #105 lived (§2.6). |
| A `compile_fail` doctest plus a `cargo test --doc` gate step | Same wrong surface, and it needs a new step in two gate files to run at all. A gate step that proves an adjacent property is read as coverage of the real one (§2.6). |
| Put the completeness check in the producer and audit callers in review | Every defect in §1's table passed review. |
