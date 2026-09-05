# DF-absorbed-scan-warning: a scan its own members swallowed is a warning with a named release, not an error

**Status:** Accepted
**Date:** 2026-09-04

## 1. Context

A `[workspace]` block whose `members` cover every one of its own walk roots reads nothing: its declarations reach no catalog, and its dangling citations pass a `grund check` that says `success` ([§FS-workspace.2.1](../../functional-spec/FS-workspace.md#21-a-member-that-swallows-the-blocks-own-scan)). Reporting it at all is not in question — that hole is [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing silently, and the workaround, "point `[scan] include` somewhere else", is only findable by someone who already knows what happened.

What is in question is how loudly. The neighbouring rule is an **error**: a member root that resolves *to* or *outside* the block that lists it is refused at load, and [§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces) gives this very consequence as one of its two reasons — "a root *above* its own block scans nothing at all". This finding is that sentence one step weaker. The temptation is to make it the same error and be done.

[§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered) governs the choice, and governs it even at warning severity: any warning stands in place of the `success` marker ([§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format)), so a run's bytes move either way. Two licences are on offer. [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) is the deprecation path: release `N` ships the warning naming the release the old form stops working in, and the error lands no earlier than `N+1`. [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) permits the verdict to flip inside one release — but only where "the fix is one documented command the tool ships".

## 2. Decision

### 2.1 A warning, because the repair is a judgement rather than a command

The finding takes [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path). §3's precondition is simply false here: the repair is a choice between two different configurations that mean different things — repoint `[scan] include` at a directory that is not also a member, or set `include_root = false` and accept that the block is grouping rather than a project ([§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces)) — and no command `grund` ships can pick between them, because the answer is what the repository meant, not what its files say. `grund fmt` rewrites citations, not config, and rendering a `[scan]` key nobody wrote would be the tool guessing at intent.

An error would therefore stop every command in a repository that works today with no route out but a hand edit, which is precisely the silent-upgrade breakage [§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly) exists to prevent — and it would do it at load, so `grund list`, `refs` and `show` would fail alongside `check` rather than merely reporting.

### 2.2 Why the sibling containment case is an error and this one is not

`workspace_member_root` refuses a member that resolves to its own block root, or outside it, as a hard error. The two look alike and are not, on the test §3 sets. That entry has *no* meaning to preserve: no lexical ancestor lists such a root, so nothing gives it a stable alias path, and it is broken at every scope rather than merely unproductive. A configuration that reaches it never worked. A block whose members happen to cover its scan roots, by contrast, is a configuration that loads, resolves, and answers every query it is asked — it just answers about less than its author thinks. Refusing it is a verdict flip on a working repository; refusing the other is refusing to pretend.

The size of the mistake is not what decides the ramp, and this repository has already written that down once ([§DF-index-compatibility-ramp.2.2](DF-index-compatibility-ramp.md#22-the-inversion-is-the-rule-working-not-a-bug-in-it)): what the licence turns on is whether a maintainer can clear the finding with a command the tool hands them.

### 2.3 The release is named in the message, not only in the changelog

[§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) requires the warning to name the release the old form stops working in, and the message carries it: `This becomes an error in grund 0.14.0` ([§FS-check.4.7](../../functional-spec/FS-check.md#47-a-workspace-member-swallows-the-blocks-own-scan)). A deadline that lives only in release notes is one nobody reads at the moment they meet the warning, and a warning with no stated end date is one people learn to skip.

`0.14.0` is the next minor after the release this ships in. The tree carries `0.12.4-dev`, and both ramps then on the books — the missing-index-entry warning and the deprecated `[[kinds]] prefix` key — promised their flips *in* `0.13.0`, whose guard tests failed the version bump that reached it; so `0.13.0` is the release those two come due in and the one this warning ships beside them in, and §2's "no earlier than `N+1`" puts the error at `0.14.0`. One of those two has since landed and the other has not — the missing-index-entry flip is in the tree, while the `prefix` key still loads and still promises `0.13.0` — and neither fact moves this deadline, which is measured from the release *this* warning ships in rather than from theirs. Should the bump that ships this instead be a patch, the deadline is later than the requirement asks rather than earlier, which is the safe direction to be wrong in.

Three things hold the literal honest, the same three [§DF-index-compatibility-ramp.2.3](DF-index-compatibility-ramp.md#23-both-findings-name-their-versions-and-a-test-keeps-the-names-honest) uses: the release process counts ramp constants as part of the version bump ([§FS-distribution.4](../../functional-spec/FS-distribution.md#4-release-process)), a test asserts the named release is still ahead of `CARGO_PKG_VERSION` so the bump that reaches the deadline fails CI, and [§RM-workspace-absorbed-scan-error](../../roadmap.md#rm-workspace-absorbed-scan-error-flip-the-absorbed-scan-warning-to-an-error) is the milestone that spends it.

### 2.4 It is emitted where the boundary is populated, not where `check` reports

The whole complaint is that the other surfaces look fine: `grund list` printed nothing and exited `0`. A finding only `check` produced would leave that untouched. So the question is asked once per block at the point a run resolves that block's member boundary, which every walking command shares, and the message is a CLI-level `warning:` on stderr ([§FS-errors.2.2](../../functional-spec/FS-errors.md#22-cli-level-message)) — the shape the undecidable-ancestor warning of [§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces) already uses for a load-time fact that must not stop the run.

## 3. Alternatives considered

**An error, as the issue's first option.** Rejected on §2.1: it flips the verdict of repositories that load today, with no command to clear it, and it does so at load so no command survives it.

**A warning with no named release.** Rejected on [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path), which asks for the release by name, and on what an undated warning becomes: permanent output that tools learn to filter.

**Reporting a partly covered `[scan] include` too.** Rejected because [§FS-workspace.6](../../functional-spec/FS-workspace.md#6-nested-project-boundary) already specifies that shape — the root scan stops at a member "even if the root project's `[scan] include` names a path inside a member" — so a repository doing it deliberately would take a warning for behaviour the spec promises. Reporting only the total case keeps the finding at "this project reads nothing", which is a fact rather than a judgement about someone's layout.

**Widening the existing empty-scan caution ([§FS-check.2.2](../../functional-spec/FS-check.md#22-empty-scan)) instead of adding a finding.** Rejected twice over: its text is stable phrasing ([§FS-errors.3](../../functional-spec/FS-errors.md#3-message-text)) that repositories grep for, and it belongs to `check` alone, so widening it would leave `list` — the ticket's own repro — exactly as silent as before.

## 4. Consequences

A repository whose members cover its scan roots starts printing one warning per affected block on every walking command, and keeps its exit code until `0.14.0`. A repository that meant it says so by setting `include_root = false`, and a repository that did not repoints `[scan] include`; both clear the warning by stating what they meant, which is the outcome an error could have compelled but not explained.

Two adjacent silent-scope holes were left open here and are not this decision's; both have since been closed on their own terms. A `[workspace]` block no enclosing block lists is [§DF-unlisted-workspace-block](DF-unlisted-workspace-block.md#df-unlisted-workspace-block-an-unlisted-workspace-block-is-reported-by-the-walk-that-meets-it), which took this same ramp. A node excluded by `include_root = false` whose own files are then scanned by nobody ([§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces)) is [§DF-unread-opted-out-block](DF-unread-opted-out-block.md#df-unread-opted-out-block-the-unread-files-of-an-opted-out-block-are-a-conditional-warning-that-never-ramps), which did not: no repository in that state is doing anything wrong and no key records the intent, so it stays a warning for good. [§DF-unread-opted-out-block.2.3](DF-unread-opted-out-block.md#23-what-makes-a-workspace-finding-ramp-and-what-makes-one-permanent) is where the three findings are held against one rule, so a reader meeting all of them is not left to infer which behaviour is the accident.
