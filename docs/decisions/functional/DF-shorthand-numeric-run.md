# DF-shorthand-numeric-run: a marked shorthand glued to another number is a numeral, not a citation

**Status:** Accepted
**Date:** 2026-08-19

## 1. Context

[§DF-number-only-citation-shorthand](DF-number-only-citation-shorthand.md#df-number-only-citation-shorthand-the-number-only-shorthand-is-authoring-sugar-and-a-persisted-one-is-a-check-error) made `§FS-042` recognizable, an error where it persists, and rewritable in bulk by `grund fmt --write` ([§FS-fmt.2.4](../../functional-spec/FS-fmt.md#24-shorthand-to-canonical)). It bounded the rewrite with one question — *does this token end here?* — because it had one threat in view: the shorthand is a lexical **prefix** of every longer ID-shaped token, so `§FS-042-User-Login` must not be read as `§FS-042` with a tail glued on.

That question is necessary and it is not sufficient. A changelog recording an ID remapping writes the old numbers as glued runs:

```
Renumbered on import: §SPEC-001→SPEC-003, and §SPEC-001/003 both moved.
```

Every character after `SPEC-001` is one that cannot continue an ID, so the token *does* end there, and the rewrite fires:

```
Renumbered on import: §SPEC-001-checkout→SPEC-003, and §SPEC-001-checkout/003 both moved.
```

The sentence now names a declaration it never meant, and the reader can no longer tell the numbers were a mapping. Reported from a real tree ([issue #81](https://github.com/vjovanov/grund/issues/81)), where it repointed a renumbering table for a folded-away namespace at live, unrelated declarations.

**Nothing downstream can see it.** `§SPEC-001-checkout` is a well-formed citation of a real declaration, so `grund check` passes and the run reports success. Every other `fmt` rewrite leaves the ID token byte-identical and only moves markup around it — a wrong trigger→marker, bare→marker, or link wrap dangles, duplicates, or fails to resolve, and `check` names it. Shorthand→canonical is the one pass that writes characters *into* the ID, which is information the source did not carry, so a wrong one is indistinguishable from a right one by any later pass. Only a human reading the prose can catch it, and only if they happen to read that line. That is the failure class [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) forbids, arriving from the side no rule was watching: not a citation that resolves to nothing, but a citation that resolves to the wrong thing while reporting green.

## 2. Decision

### 2.1 The marker proves the sentence is about IDs; it does not pick the citation out of it

[§DF-number-only-citation-shorthand.2.4](DF-number-only-citation-shorthand.md#24-the-marker-is-required-a-bare-shorthand-is-text) requires the marker on a shorthand for exactly the right reason: `KIND-NNN` is a shape that occurs constantly in the wild as issue keys, part numbers, and standards references, and unlike a full ID it carries no slug to make an accidental match unlikely, so the marker is what supplies the missing intent. It then treats the marker as *sufficient* evidence, and that is where this defect was decided.

A `§` in front of `SPEC-001→SPEC-003` says *this sentence is about grund IDs*. It does not say *this one token is the citation and the rest is prose*. For a full ID the two readings coincide, because the slug makes any other reading vanishingly unlikely. For a shorthand — which is precisely the ambiguous shape — they come apart, and the marker cannot tell them apart on its own.

So the shorthand needs one more piece of evidence than the marker, and §2.2 says which.

### 2.2 The evidence for a run is the run

A marked shorthand sits in a **numeric run** when the text immediately following its token is a delimiter sequence that carries a second number.

Precisely, reading forward from the end of the shorthand token:

1. Take the maximal run of characters that can neither continue an ID nor start one — anything that is not alphanumeric and not `_`. The run must be **non-empty**, and must contain **no whitespace**, **no marker**, and **no bracket or quote**.
2. What follows it must begin with either a `[id] number_pattern` match or the whole shorthand shape (`{kind}` and number) of the grammar that parsed the token ([§FS-config.3.2](../../functional-spec/FS-config.md#32-id--id-grammar)), **unqualified**: an `<alias>/` namespace prefix ([§FS-workspace.1](../../functional-spec/FS-workspace.md#1-citation-syntax)) precedes a citation and is never the second number of a run, so admitting it would make every path ending in an ID-shaped segment count as one.

Both conditions hold in `§SPEC-001→SPEC-003`, `§SPEC-001/003`, `§SPEC-001..003`, and `§COMP-047/046/049`; neither the delimiter list nor the count of elements needs enumerating, because the discriminator is not the punctuation — it is **the second number**.

The three exclusions are the whole safety margin:

- **No whitespace.** The gluing *is* the evidence. A run is written as one unbroken string precisely because its parts belong together; a citation followed by prose is not. This is what keeps `§FS-042 (2024)` a citation under the default `number_pattern = "\d+"`, which matches a year as readily as an ID number.
- **No marker.** `§FS-042, §FS-043` is two citations the author marked one at a time, and marking each is the clearest statement of intent available in this grammar. `§SPEC-001→SPEC-003` is one marked run.
- **No bracket or quote.** `(`, `)`, `[`, `]`, `{`, `}`, `"`, `'`, and `` ` `` bound a construct; they do not glue two numerals. Without this the characters *closing* the construct the citation sits in and *opening* the next one read as one delimiter run, and whatever number the next construct carries becomes the second number. That misreads two shapes this project writes constantly: `[§FS-042](FS-042-user-login.md)` — the Markdown link [§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission) itself produces — and the footnote reference `§FS-042[^1]`. Both are ordinary citations, and refusing them leaves a §2.5 error naming an edit `fmt` will never make, which is the one thing [§DF-number-only-citation-shorthand.2.2](DF-number-only-citation-shorthand.md#22-where-the-shorthand-is-accepted-and-where-it-is-an-error) forbids. `|` is deliberately not in the set — it is a delimiter people write between numbers (§3), and a Markdown table cell writes `| §FS-042 |` with the spaces the whitespace exclusion already covers.

`-<digits>` needs no clause: under a `slug_pattern` that admits digits the full-ID pass claims `SPEC-001-003` and it dangles loudly, and under one that does not, the trailing-boundary rule of [§DF-number-only-citation-shorthand.2.6](DF-number-only-citation-shorthand.md#26-the-full-id-always-wins-and-only-a-whole-token-is-a-shorthand) already refuses it. The rule here adds only what that one cannot see.

### 2.3 The rule reads forward, not backward

`SPEC-001→§SPEC-003` — a run whose *tail* carries the marker — is not covered, deliberately.

A run is written head-first and the marker lands on the head, which is where every reported instance put it. Reading backward as well would cost more than it buys: a left-hand neighbor that is number-shaped and glued is a common shape in ordinary prose — `2026-08-19/§FS-042`, `v1.2/§FS-042` — and refusing those would withhold the rewrite from real citations for no evidence at all. §2.5's report is what covers the residue.

### 2.4 `fmt` does not guess about intent, exactly as it does not guess about resolution

[§FS-fmt.2.4](../../functional-spec/FS-fmt.md#24-shorthand-to-canonical) already states the principle — *`fmt` normalizes, it does not guess* — and already applies it to **resolution** ambiguity: a shorthand matching zero or several declarations is left byte-for-byte and `check` reports it. This record extends the same sentence to **intent** ambiguity, which is the case it never named: a shorthand that resolves perfectly well but is not being used as a citation.

The two are the same rule about the same pass and cost the same thing. A withheld rewrite leaves a persisted shorthand and a finding; a wrong rewrite leaves a false sentence and silence. The costs are not close, so the tie goes to withholding.

### 2.5 The site is reported, and the report names both exits

Skipping silently would fix the corruption and leave the author with no way to learn the line needs attention. So the site earns a finding of its own ([§FS-check.3.15](../../functional-spec/FS-check.md#315-shorthand-citation-in-a-numeric-run)):

```
docs/changelog.md:3: shorthand §SPEC-001 sits in a numeric run and was not rewritten;
write §SPEC-001-checkout, or <§>SPEC-001 if these are old numbers
```

Both exits are named because `grund` cannot know which one is meant, and the author knows immediately. If it was a citation, the canonical text is right there to paste. If it was a mapping, `<§>` is the escape this grammar already has for writing an ID without citing it ([§FS-check.2.3.1](../../functional-spec/FS-check.md#231-escaped-citation-resolves)), and the message says so in the same shape §3.1 already uses for a dangling citation that might be an illustration.

An **error**, on the same argument [§DF-number-only-citation-shorthand.2.3](DF-number-only-citation-shorthand.md#23-it-is-an-error-not-a-warning-or-a-suggestion) makes: a warning leaves the exit code alone, and a finding no CI run fails on is one a repository accumulates behind forever. Upgrading costs nothing, because these sites are **already** errors — `shorthand citation §SPEC-001; write §SPEC-001-checkout` fires on them today. What changes is that the message stops advising an edit that would corrupt the line. No tree turns from green to red ([§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)); some turn from red-and-wrong to red-and-right.

This is the one place [§DF-number-only-citation-shorthand.2.2](DF-number-only-citation-shorthand.md#22-where-the-shorthand-is-accepted-and-where-it-is-an-error) has to be read more narrowly than it was written. It holds that a finding whose named fix the tool declines to perform is one a repository can never clear, and therefore withholds the error wherever [§FS-fmt.2.3](../../functional-spec/FS-fmt.md#23-what-is-never-rewritten) forbids the rewrite. That is right where the text is legitimate as it stands — a citation illustrated in inline code needs no edit at all. It is wrong here, where the line does need an edit and the author can make it in one keystroke. The commitment worth keeping is that **every finding names a fix**, not that every fix is `fmt`'s.

### 2.6 Recognition is unchanged; only the rewrite is withheld

The site stays a citation for every graph question — it resolves, `refs` lists it, `cover` groups it, it grounds its file, it counts for citation directions, and its declaration is not reported unused ([§DF-number-only-citation-shorthand.2.8](DF-number-only-citation-shorthand.md#28-a-resolved-shorthand-is-a-real-edge)).

Dropping the edge would be the tidier model — the token is not really a citation — and it is the wrong trade. It can only *remove* findings: the site would resolve to nothing, report nothing, and the declaration would go back to being reported uncited, which is the exact false negative [§DF-number-only-citation-shorthand](DF-number-only-citation-shorthand.md#df-number-only-citation-shorthand-the-number-only-shorthand-is-authoring-sugar-and-a-persisted-one-is-a-check-error) exists to end. Keeping the edge and reporting the site keeps the run loud.

For the same reason the rule touches only the finding that names the rewrite. A shorthand in a run that matches **zero or several** declarations keeps its existing message: that is a resolution failure, reported on its own terms, and a run is not a reason to say less about it.

### 2.7 Invention is reported in full, whatever the rule decides

Independent of the rule above, and the part that outlives it: `fmt` prints the text of every shorthand it expands ([§FS-fmt.3](../../functional-spec/FS-fmt.md#3-outputs)).

```
docs/notes.md:1: shorthand → canonical: §FS-042 → §FS-042-user-login
```

The old report named the rewrite class and the line, which is enough to review a rewrite that only moves markup and not enough to review one that invents an identity. The reporter of #81 found this defect by re-reading a 5,750-line diff after the fact, which is the only review `grund` left available. A `--check` run that shows what it will write is a review before the fact, it costs one string, and it is the only generic defense against the next pass that writes characters the source did not carry.

## 3. Rejected alternative: a list of run punctuation

Skip a shorthand immediately followed by `→`, `/<digits>`, `…<digits>`, or `-<digits>` — the shape the report proposed.

It fixes every reported case and was rejected for being a list. The bracket-and-quote exclusion in §2.2 is not that list returning: this one enumerates what *makes* a run, an allowlist that has to be complete to be correct and grows by one entry every time a human notices a false sentence; that one enumerates what makes it not a run, and every member is there for one stated property — it bounds a construct — so the set is closed by the property rather than by observation, and a missing member costs a withheld rewrite rather than a corrupted line. `..`, `—`, `|`, `>`, `»`, and `:` are all run delimiters somebody writes, and each would be a separate defect found the same way — by a human noticing a false sentence. Worse, the list encodes the symptom: the reason `→` is suspicious is not that it is `→`, it is that a number follows it. Naming the evidence directly derives all four listed cases and the ones nobody has hit yet, and it leaves a rule a reader can check against a line by eye.

## 4. Rejected alternative: refuse every shorthand rewrite

Withdraw [§FS-fmt.2.4](../../functional-spec/FS-fmt.md#24-shorthand-to-canonical) and make the canonical form a hand edit.

That trades a rare silent corruption for a rule nobody can clear in bulk, and [§DF-number-only-citation-shorthand.2.2](DF-number-only-citation-shorthand.md#22-where-the-shorthand-is-accepted-and-where-it-is-an-error) is explicit that the [§FS-check.3.13](../../functional-spec/FS-check.md#313-number-only-shorthand-citation) error is only worth having because one command clears it. The pass is safe on everything but a shape with evidence against it; deleting it would answer a bounded defect by removing the feature.

## 5. Consequences

- A new `shorthand-numeric-run` error code ([§FS-check.3.15](../../functional-spec/FS-check.md#315-shorthand-citation-in-a-numeric-run)), replacing the mechanical `shorthand-citation` message at these sites only. Withheld out of scope under `--full` for the reason [§FS-check.3.14](../../functional-spec/FS-check.md#314-out-of-scope-unresolvable-citation---full-only) withholds the mechanical form, and withheld where [§FS-fmt.2.3](../../functional-spec/FS-fmt.md#23-what-is-never-rewritten) already forbids every rewrite: an illustration in inline code needs no edit.
- `Citation` gains a `numeric_run` flag, set by the scanner beside `shorthand_rewritable` — the scanner is the only pass holding the line text, so it is the only one that can see the run ([§AR-scanner.2.6](../../architecture/AR-scanner.md#26-number-only-shorthand-citations)).
- `fmt`'s per-line report label becomes a `String` rather than a fixed set of four, so the expansion can carry its text (§2.7).
- The LSP's live transform ([§FS-lsp.1.4](../../functional-spec/FS-lsp.md#14-live-trigger-transform)) applies the same rule, and has a residue the bulk pass does not: an author typing a fresh run keystroke by keystroke has not yet written the second number when the keystroke that ends the token fires, so the expansion happens and is then visible and undoable in the editor. That is the loud failure, not the silent one, and it is the same order of surprise as any on-type transform. A run already on the line — the paste or edit case — is refused there exactly as it is in `fmt`.
- No `grund_config_version` bump, no `[id]` key, and no `AGENTS.md` block bump. The rule is derived from `[id] format` like the shorthand itself; a knob would let two installs disagree about what a citation *is* ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)), and the block already tells an agent to write canonical citations, which stays exactly right.
