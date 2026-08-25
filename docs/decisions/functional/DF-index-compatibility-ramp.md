# DF-index-compatibility-ramp: a finding's ramp follows its fix command, not the size of the offence

**Status:** Accepted
**Date:** 2026-08-25

## 1. Context

Checking a kind's index ([§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index), [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link)) turns two states that were green into findings, on every repository that already has a folder kind. That is a verdict change, and [§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered) governs it as one even at warning severity, because any warning stands in place of the `success` marker ([§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format)).

Two licences exist for such a change, and they cost different things. [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) is the deprecation path: release `N` ships the finding as a warning naming the release in which it becomes an error, and `N+1` makes it one. [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) is the loud mechanical migration: the verdict may flip in the same release, but only where "the fix is one documented command the tool ships".

## 2. Decision

### 2.1 The two halves ride different ramps

A **declaration missing from the index** takes [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path): a warning in this release, naming the release in which it becomes an error. Nothing `grund` ships writes the entry — rendering an index is deliberately out of scope ([§DF-index-entry-form.3](DF-index-entry-form.md#3-alternatives-considered)) — so [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations)'s precondition is simply false, and taking that route would make the renderer a dependency of this check rather than a follow-on to it.

An **entry present but not a link** takes [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) and is an error on arrival. Its fix is one documented command the tool ships and already recommends: `[fmt.cross_refs] enabled = true` is the built-in default and what generated configs write ([§FS-fmt.6.6](../../functional-spec/FS-fmt.md#66-why-generated-configs-enable-cross-references)), and `grund fmt --write` runs the pass on any scope holding Markdown without a flag. [§DF-index-always-linkified](DF-index-always-linkified.md#df-index-always-linkified-the-cross-reference-pass-always-runs-on-a-kinds-index-file) closes the one gap in that claim, so the fix is `grund fmt --write` in every configuration rather than in most of them.

### 2.2 The inversion is the rule working, not a bug in it

This produces an index with no entry at all warning while an index with a bare entry errors — the greater offence on the softer ramp. That is the intended reading of [§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly): what the licence turns on is whether a maintainer can clear the finding with a command the tool hands them, not how wrong the repository is. A verdict a release flips without an escape route is the breakage the requirement exists to prevent, and the size of the offence does not supply one.

It is also self-correcting. When a renderer lands and there *is* a command that writes a missing entry, that half becomes eligible for the same [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) treatment — and until then it is already scheduled to become an error by §2.1 anyway.

### 2.3 The warning names the release

`an index entry becomes an error in grund 0.12.0`, carried in the message text itself ([§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index)). [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) requires the finding to name the release the old form stops working in; a warning that only says "not listed" tells a maintainer they have a problem and not that they have a deadline.

## 3. Consequences

A repository upgrading into this release sees warnings and keeps its exit code. A repository upgrading into the next one sees errors, having been told the version by every run in between. The link half is red immediately and clears in one `grund fmt --write`, which is the whole argument for treating it differently.
