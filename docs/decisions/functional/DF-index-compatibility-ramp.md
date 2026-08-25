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

The licence has all three of its conditions only because the finding is *defined* as the state that command clears. [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link) reports a bare entry when, and only when, the next `grund fmt --write` would wrap it ([§DF-index-entry-form.2.3](DF-index-entry-form.md#23-one-link-per-id-not-every-mention)); every other unwrapped mention is [§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index)'s warning. Stated the other way round — "any recognized citation that is not wrapped" — the same message would name a command that does nothing in three reachable configurations, and "one documented command the tool ships" would be a sentence the release notes could print but the tool could not keep.

### 2.2 The inversion is the rule working, not a bug in it

This produces an index with no entry at all warning while an index with a bare entry errors — the greater offence on the softer ramp. That is the intended reading of [§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly): what the licence turns on is whether a maintainer can clear the finding with a command the tool hands them, not how wrong the repository is. A verdict a release flips without an escape route is the breakage the requirement exists to prevent, and the size of the offence does not supply one.

It is also self-correcting. When a renderer lands and there *is* a command that writes a missing entry, that half becomes eligible for the same [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) treatment — and until then it is already scheduled to become an error by §2.1 anyway.

### 2.3 Both findings name their versions, and a test keeps the names honest

The warning carries the deadline in its message text: `an index entry becomes an error in grund 0.13.0` ([§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index)). [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) requires the finding to name the release the old form stops working in; a warning that only says "not listed" tells a maintainer they have a problem and not that they have a deadline. The error carries a pair, because [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) asks for the versions a verdict *moved between*: `unchecked in grund 0.11.0, an error in 0.12.0`.

Three literals, then, and all three are wrong the moment someone forgets them at release time — the version is bumped when a release is cut, not when the work lands, which is how the first draft of this ramp came to name the release it was itself shipping in. Two things hold them: [§FS-distribution.4](../../functional-spec/FS-distribution.md#4-release-process) counts them as part of the version bump, and a unit test asserts the three are ordered and that the error release is still ahead of `CARGO_PKG_VERSION`. The bump that reaches the deadline fails CI rather than shipping a warning whose date has passed, which makes [§RM-index-entry-error](../../roadmap.md#rm-index-entry-error-flip-the-missing-index-entry-warning-to-an-error) a milestone the release process cannot walk past.

## 3. Consequences

A repository upgrading into this release sees warnings and keeps its exit code. A repository upgrading into the next one sees errors, having been told the version by every run in between. The link half is red immediately and clears in one `grund fmt --write`, which is the whole argument for treating it differently.

The ramp only had to be argued for repositories that *have* an index obligation, and the `index` default decides who those are. `E2E` defaults to `index = false` for a declared kind exactly as it does for a built-in one ([§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds)); without that, every repository whose `grund.toml` was written by an older `grund init` — which is every repository with a config on disk — would have taken one warning per e2e case on upgrade, for a folder that is exercised rather than navigated. Measured on this repository before the default was applied to declared kinds: 391 warnings, and no `success` line.
