# DF-note-columns-are-characters: a note column is one character, not one byte and not one display cell

**Status:** Accepted
**Date:** 2026-08-23

## 1. Context

`[reference] inline_note_max_columns` ([§FS-inline-citation-style.2.3](../../functional-spec/FS-inline-citation-style.md#23-counting-lines-and-columns)) bounds the longest line at an inline citation site. The key says *columns*, the finding says `inline note exceeds N-column maximum`, and the sentence rendered into the managed entrypoint block says `≤ 100 columns` ([§FS-inline-citation-style.5](../../functional-spec/FS-inline-citation-style.md#5-agent-facing-rendering)). Until this decision the rule measured **bytes**, and §2.3 said so in as many words.

The measures agree on ASCII and diverge everywhere else, because UTF-8 spends two or three bytes on exactly the characters technical prose reaches for: an em dash, an accented letter, `×`, and the `§` marker the citation itself is made of. Two notes of identical length are therefore judged differently:

```rust
// xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx (§SPEC-001-checkout)
// éééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééé (§SPEC-001-checkout)
```

Both lines are 90 characters. The first is 91 bytes and passes at `inline_note_max_columns = 100`; the second is 157 bytes and fails. In a real tree with European prose the effective cap sat near 85, and every one of the 23 findings it produced was on a note comfortably under the configured 100 — so the fix an author reached for was to shorten prose that was never too long, leaving notes visibly clipped against the rest of the file. That is the shape of a limit nobody can predict: an author cannot count a line's bytes while writing it, no editor's column indicator reports them for UTF-8 text, and nothing in the key's name, the finding, or the entrypoint sentence hints that the number means anything but characters.

The old bullet's own justification argued against its own rule. It said the cap "matches what an editor's column indicator shows in a file, not the visual rendering on any particular tabstop setting" — a correct goal, reached for by measuring the one quantity no editor's indicator shows.

## 2. Decision

### 2.1 A column is a character

The width of a line is its count of Unicode scalar values. `é` is one column, `—` is one column, `§` is one column, and a tab is one column ([§FS-inline-citation-style.2.3](../../functional-spec/FS-inline-citation-style.md#23-counting-lines-and-columns)). A project that configures `100` gets 100 columns in every language it writes, which is what [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) asks of a limit an author is expected to write under, and what [§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable) already promised by exposing the number at all — a knob whose effective value depends on the alphabet is not the knob the key names.

The character is also the only one of the three candidate units that stays a property of the *file*. Bytes are a property of the encoding, display cells a property of the renderer; the number of characters an author typed is the same fact on every machine that opens the file, which is what [§REQ-deterministic-output](../../requirements/REQ-deterministic-output.md#req-deterministic-output-same-input-same-bytes) wants of anything a verdict depends on.

### 2.2 The scanner's recorded citation column stays a byte offset

The citation position the scanner records ([AR-scanner.3](../../architecture/AR-scanner.md#3-output)) is unchanged, and §2.3 now says the two are different measures rather than the same one. They answer different questions: that column addresses a place in a file so an editor or an LSP client can jump to a token, and a byte offset is the honest unit for addressing; this one counts how much an author wrote so a budget can bound it. Re-basing the recorded position would change every consumer of a citation's location — `refs --format json`, the LSP ranges, the report ordering — to fix a defect in none of them.

### 2.3 Not display width

Display width is rejected as the unit, and the previous bullet was right to reject it. It would make the cap depend on a font, an East Asian width table, and a tabstop setting, so two installs could disagree about one file — which [§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree) forbids outright — and it would cost a Unicode width table to compute what `chars().count()` gives for nothing. "Tabs are one column" survives this decision verbatim; only the byte reading is retired.

### 2.4 Nothing that passed stops passing

A UTF-8 line's character count is never greater than its byte length, so the measured width of every line in every tree either falls or stays equal, and the finding fires on a strict subset of the sites it fired on before. No repository that passes `grund check` today can fail it after this change, and the flip in the other direction — a site that errored and now passes — is the defect being repaired.

This is why the change ships without the deprecation window of [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) and does not need the loud-migration licence of [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations). That requirement is about not turning a green repository red behind the maintainer's back; a rule that only ever stops rejecting is outside the risk it guards. A project that *wants* the old effective limit sets a smaller number, which it could not do before, because the old limit was not a number it could name.

## 3. Rejected alternative: keep bytes and rename the key

Rename `inline_note_max_columns` to `inline_note_max_bytes`, leave the arithmetic alone, and let the name tell the truth.

It is honest, and it is the wrong trade. The quantity an author controls while writing a comment is characters; bytes are an artifact of the encoding that no one composing prose is tracking, so a truthful name would just relocate the surprise from "why did this fail?" to "why is my limit measured in a unit I can't count?". It would also spend the whole cost of the fix — a key rename is a config-schema change under [§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning), with an alias to carry and an entrypoint sentence to re-render — to leave the reported behaviour exactly as unhelpful as it is now. The name was never the part that was wrong.

## 4. Consequences

- `InlineCitationSite::max_columns` is a character count ([AR-scanner.3](../../architecture/AR-scanner.md#3-output)). The scanner computes it with `chars().count()` over the site's lines; the checker rule that compares it against the configured cap ([§FS-inline-citation-style.4.1](../../functional-spec/FS-inline-citation-style.md#41-errors--hard-caps)) is unchanged, since only the measure moved.
- No `grund_config_version` bump and no `AGENTS.md` block-version bump. The key, its default, its finding text, and the rendered `≤ N columns` sentence are all byte-identical; a repository's managed block does not drift.
- Findings only disappear, never appear (§2.4). A tree carrying notes in non-ASCII prose sees its `inline-citation-style` column errors fall, and the ones that remain name lines that really are over the configured number of characters.
