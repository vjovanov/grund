# DF-inline-note-layout: inline note layout is a configured house style, checked per line and never normalized

**Status:** Accepted
**Date:** 2026-08-16

## 1. Context

`[reference] inline_style = "citation-with-note"` ([§FS-inline-citation-style.3.2](../../functional-spec/FS-inline-citation-style.md#32-citation-with-note)) says a citation may carry prose and how much of it; it says nothing about where the citation sits relative to that prose. All three of these pass today, and a repository that wants exactly one of them has no way to say so:

```java
// §FS-user-login.2: Reject an expired credential.
// Reject an expired credential. §FS-user-login.2
// §FS-user-login.2 Reject an expired credential.
```

Two costs follow. The first is that the convention becomes repository prose: the generated entrypoint tells an agent that a note is allowed and how long it may be, and then the agent has to *infer* the arrangement from the surrounding files — which is the guessing [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) exists to remove. The second is specific to the citation-first form with a colon: it is the one arrangement that cannot be confused with an inline declaration. Under `[reference] strict = false` a bare `// FS-x: note` line *is* a declaration heading ([§FS-inline-citation-style.1](../../functional-spec/FS-inline-citation-style.md#1-scope)), so writing the marker and the colon is what makes "this cites `FS-x`" and "this declares `FS-x`" two visibly different lines rather than one shape read two ways.

The budgets already have the machinery this needs: a config key, a load-time enum check, a scanner-recorded fact per site, a checker rule, and a rendered sentence in the managed block. What is missing is the axis.

## 2. Decision

### 2.1 Two keys: the style, and the level it is enforced at

Add `[reference] inline_note_layout` (default `"any"`) and `[reference] inline_note_layout_check` (default `"off"`) ([§FS-config.3.1](../../functional-spec/FS-config.md#31-reference--citation-form)). The first names the project's house style; the second says whether `grund check` reports a deviation and through which channel — `off`, `warn`, or `error`.

They are two keys and not one because they answer two questions that a project answers at different times. A team adopts the style first (the entrypoint starts teaching it on the next `grund init` — the sentence is rendered into the managed block, not read live, so adopting the key and refreshing the block are two steps, [§FS-inline-citation-style.5](../../functional-spec/FS-inline-citation-style.md#5-agent-facing-rendering)), migrates the tree with `warn` as the worklist, and gates on `error` once the tree is clean. Folding the level into the layout value — `citation-first-colon-warn` — would multiply the enum by the level set and make "same style, different gate" unstateable.

This is **not** configurable severity. [§FS-non-goals.9](../../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization) freezes what `error` and `warning` *mean* and how the exit code follows from them, and [§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree) requires two correctly-configured installs to agree on a tree — both still hold, because the level is read from the same committed `grund.toml` both installs read. Choosing which channel one opt-in rule speaks through is existing precedent, not a new power: `warn_on_suggested` ([§FS-inline-citation-style.4.2](../../functional-spec/FS-inline-citation-style.md#42-warnings--opt-in-soft-cap)) moves the soft cap between silent and warning, and `[citations]` `must` / `should` ([§FS-config.3.9.1](../../functional-spec/FS-config.md#391-levels)) picks error or suggestion per obligation. What is ruled out is a general remap — there is no key that re-levels the dangling-citation error, and this one names its own rule.

### 2.2 Per line, not per site

A site is a whole comment block, and blocks in every doc-comment convention mix kinds of line: a Rustdoc or Javadoc block opens with a summary sentence and carries its `§<ID>: …` lines below it. Judging the block as a unit would either reject that shape or accept a block whose first citation line is malformed because a later one is fine. So every line that carries a recognized citation is judged on its own, lines without a citation are unconstrained, and the finding is anchored at the offending line — which is also the line the author has to edit ([§FS-inline-citation-style.4.4](../../functional-spec/FS-inline-citation-style.md#44-warnings-and-errors--opt-in-layout-deviations)).

A site with no note at all is exempt: `// §A  §B` and `// §A, §B` are pure pointers — the separator joining two citations of one run is not a note ([§FS-inline-citation-style.1](../../functional-spec/FS-inline-citation-style.md#1-scope)) — and a layout is a relation between a citation and a note. The exemption is per site rather than per line, which has one consequence worth stating outright — a `// §<ID>` line followed by a prose line in the *same* block is a site with a note, so the citation line is judged and fails. That is the intended reading: the two lines are one comment, and the note is not laid out behind its citation.

### 2.3 One exact canonical form

`citation-first-colon` accepts exactly one spelling — a run of citations joined by `, `, a colon, then a space and the note (or end of line) — as specified in [§FS-inline-citation-style.3.3](../../functional-spec/FS-inline-citation-style.md#33-inline_note_layout--where-the-citations-sit). Near misses are deviations: a space where the comma belongs, a comma with no space, a space before the colon.

Being exact is the point of asking for a form at all. A tolerant matcher would accept several spellings, and the project would end up with the variation it configured the key to remove — while an agent reading the rendered sentence would still have to guess which of the tolerated spellings the humans actually write. Exactness also keeps the rule cheap to state, cheap to implement, and identical across installs, and it is what makes the deviation list a mechanical worklist rather than a matter of taste.

### 2.4 A closed enum with one member, widenable

`inline_note_layout` ships with `any` and `citation-first-colon` only. Other arrangements are plausible and were considered — `citation-first-dash` (`§A — note`), a bare `citation-first` with no delimiter, a bracketed `[§A] note`, `note-first-parenthesized` (`note (§A)`), a bare `note-first`, and `citation-line` (citations alone on their own line above the prose). None is ruled out; none has a demand behind it yet, and each would need its own passing and failing examples in the spec, its own rendered sentence, and its own e2e coverage. Adding a value later is additive and needs no `grund_config_version` bump (§2.6), so the cost of waiting is zero and the cost of guessing wrong is a value nobody uses and nobody can remove.

A free-form template — `inline_note_format = "{cites}: {note}"` — was the tempting generalization and is rejected for now. It turns a closed enum into a small language: every template needs a parser, an inverse matcher, an escaping rule for a literal `{`, and a validation pass that rejects the templates that cannot be matched unambiguously. It also makes the *rendered* agent sentence unwritable in general, since grund would have to describe a form it was handed rather than one it knows. A named enum keeps one canonical description per value, in the spec and in the entrypoint alike.

### 2.5 Check-only: `grund fmt` is not involved

The layout is never normalized, in `grund fmt --check` or in `--write` ([§FS-inline-citation-style.4.3](../../functional-spec/FS-inline-citation-style.md#43-grund-fmt)). This is the one part of the original report ([issue #52](https://github.com/vjovanov/grund/issues/52)) that is declined, and the reason is that the rewrite is a prose edit wearing a token edit's clothes. Turning `// note (§A).` into `// §A: note.` requires deciding where the sentence ends, whether the trailing parenthetical was the citation or part of the sentence, and what punctuation the remainder now needs — judgment `grund` deliberately does not have, and cannot get without the AST parsing [§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing) rules out. A formatter that is right most of the time is worse here than one that never runs: the failure is a silently mangled comment in a commit nobody reviewed line by line.

What the report actually needs is served without it. The fix is one token typed by whoever is already editing the line, and the migration of an existing tree is served by `warn`, which produces the exact list of lines to visit.

### 2.6 Default off, and no version bumps

Both keys default to the inert value, so no existing repository gains a finding on upgrade ([§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)). `grund_config_version` stays at 1: the keys are additive, and a v1 config that omits them is read exactly as before. The `AGENTS.md` managed-block version does not move either — under `any` the rendered block is byte-identical to what the previous release produced, so no repository's block goes stale and no one is handed an `agents-init` error for a rule that changes nothing about what they should write.

## 3. Consequences

- `Config` gains `inline_note_layout: String` and `inline_note_layout_check: String`, both validated at load and both printed by `grund config show` at every value, inert or not ([§FS-config.4.2](../../functional-spec/FS-config.md#42-grund-config-show-path)). `templates/grund.toml` carries both with their accepted-value comments, so the generated config still teaches the whole schema ([§FS-init.2.4](../../functional-spec/FS-init.md#24-generated-grundtoml)).
- The scanner records, per inline citation site, the lines that fail the configured layout ([§FS-inline-citation-style.7](../../functional-spec/FS-inline-citation-style.md#7-architecture-impact)); the checker stays a pure pass over `Findings` and reads the list rather than re-reading the file. Under `any` no line is tokenized or classified on the field's account — the default path pays one comparison per site and one empty per-block memo, nothing more ([§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)).
- A repository on `citation-first-colon` gains one more reason for its comments to be mechanically checkable: the citation-first line is the only inline form that cannot be read as an inline declaration, which is what the ambiguity in §1 costs today under `strict = false`.
- The `warn` level is a genuine migration surface and not decoration: it never moves the exit code ([§FS-check.4](../../functional-spec/FS-check.md#4-warnings)), so a team can turn it on in CI on the day it adopts the style.

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| One key, values `any` / `citation-first-colon` / `citation-first-colon-strict` | Multiplies the style enum by the enforcement level and makes "this style, not yet gated" unstateable — the state every adopting repository is in first. |
| Enforce the layout unconditionally under `citation-with-note` | Every existing repository would gain findings on upgrade for a house style it never chose, which is exactly what [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) forbids; and there is no single form the ecosystem agrees on. |
| Judge the whole site instead of each line | Rejects the summary-then-citation doc-comment that Rustdoc, Javadoc, and JSDoc all encourage, or lets a malformed line hide behind a well-formed sibling. Neither is a useful verdict. |
| Tolerant matching (any whitespace, `:` or `—`, citation anywhere in the first half) | Leaves the project with the variation the key exists to remove, and leaves the agent guessing which tolerated spelling the humans write. |
| A free-form `{cites}: {note}` template | A template is a small language: parser, inverse matcher, escaping, validation, and no way to render one canonical sentence into the entrypoint. Deferred, not foreclosed — the enum can gain values without one. |
| Normalize in `grund fmt --write` | The rewrite needs sentence-level judgment (where the prose ends, whether the trailing parenthetical was the citation) that no line-oriented formatter has; a mangled comment is worse than a reported one. §2.5. |
| Extend the rule to Markdown prose citations | Spec text governs itself; the same argument that keeps the line and column budgets off Markdown ([§FS-inline-citation-style.6](../../functional-spec/FS-inline-citation-style.md#6-non-goals)) keeps the layout off it. |
