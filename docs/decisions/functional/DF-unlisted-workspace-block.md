# DF-unlisted-workspace-block: an unlisted workspace block is reported by the walk that meets it

**Status:** Accepted
**Date:** 2026-09-04

## 1. Context

A directory that declares `[workspace]` and that no enclosing block lists among its `members` is claimed by nobody ([§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces)). The outer run ignores the block and absorbs its subtree into the enclosing project's namespace; a run started *at* it names every project from itself. The two scopes spell the same projects differently, so a citation passes the inner check and fails the run CI does — [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing in the one place the alias-path model exists to hold it.

Both [§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces) and [§DF-nested-workspaces.3.4](DF-nested-workspaces.md#34-a-path-means-the-same-thing-at-every-scope) were honest that this was unreported, and gave a reason: finding such a block needs a walk for config files, and `grund.toml` is not in `[scan] extensions`, so no pass looked for one. That reason no longer holds up. The walk already *enumerates* the directories; what it does not do is ask each one which config it carries. So the question is not whether to build a second traversal, it is what to do with a fact the existing one can be asked for at the cost of one probe per directory.

Three things had to be settled before the finding could be written: how loudly it fires, which trees it looks at, and which commands carry it. Nesting is what made the question urgent — adding `[workspace]` to a directory and forgetting to list it above is an easy thing to end up with, where before nested blocks were rejected outright.

## 2. Decision

### 2.1 A warning that names the release it becomes an error in

The finding is a **warning**, and its message names the release in which it becomes an error: [§RM-unlisted-workspace-error](../../roadmap.md#rm-unlisted-workspace-error-flip-the-unlisted-workspace-warning-to-an-error).

[§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations)'s licence to flip a verdict inside one release wants the fix to be "one documented command the tool ships". Neither remedy here is one. Listing the block as a member and excluding it from `[scan]` are opposite decisions about whether the subtree is one of ours, and no `grund` command can make that call — writing either would be `grund` deciding what a repository is. So the precondition is false and [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)'s deprecation path is what is on offer, exactly as it was for the missing index entry ([§DF-index-compatibility-ramp.2.1](DF-index-compatibility-ramp.md#21-the-two-halves-ride-different-ramps)). This is the same ramp for the same reason, and copying it rather than inventing a third path is deliberate: two neighbouring deprecations that behave differently teach a reader that the behaviour is arbitrary.

A permanent warning was the cheaper option and is rejected. Every firing is cleared by one of two one-line edits, and the thing it protects is the alias-path guarantee — the property [§FS-workspace](../../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace) exists to keep. A warning with no end date leaves that guarantee quietly breakable for as long as anyone is willing to skip a line of output, and a warning people learn to skip is what an undated one becomes.

The release goes **in the warning text**, not only in the changelog, and a unit test holds it ahead of `CARGO_PKG_VERSION` so the deadline fails the build rather than passing unnoticed — the forcing function [§DF-index-compatibility-ramp.2.3](DF-index-compatibility-ramp.md#23-both-findings-name-their-versions-and-a-test-keeps-the-names-honest) put behind the same promise.

### 2.2 The scope is the walk the run already makes, and the residue stays recorded

The rule looks at the directories this run's own walk reached — the `[scan] include` roots and their subtrees, minus `[scan] exclude`, the ignore files, hidden directories, and member boundaries. Not a second full-subtree traversal for config files.

Two reasons, and they point the same way. It is proportionate: the entries are enumerated already, so the added work is one config probe per walked directory, which is what keeps [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) affordable at the seam between the scanner and the workspace machinery. And it is the *right* tree rather than merely the cheap one — the harm this finding names is a subtree being absorbed into the enclosing namespace, and a block the scan never reaches is absorbed into nothing.

What that leaves is a real limitation and it stays written down in both documents that recorded the old one: an unlisted block outside the walk is still unreported. That is the same stance [§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index) takes for an index the run did not scan and [§FS-check.4.5](../../functional-spec/FS-check.md#45-nothing-recognized) takes for a narrowed run — a run that cannot see something does not judge it — and it is the honest half of a rule whose other half is now diagnosed.

### 2.3 Every command that walks carries it, not `check` alone

The warning is emitted by every command whose run walks a project tree, not by `check` alone.

This is the point on which the first draft of the rule was wrong, and it is recorded because the reader is owed the choice rather than the inference: [§FS-check.4.3](../../functional-spec/FS-check.md#43-redundant-config-pair) draws the opposite line one page up, where a redundant config pair is reported by `check` and the config commands and by nothing else.

That line does not reach this fact. A redundant pair is about *which file the run read*; this is about *how every command in the tree spells its projects*. The ticket's own reproduction makes the difference concrete: it demonstrates the defect with `grund list`, which prints `root/FS-c` at the repository root and `c/FS-c` inside the block. A `list` that prints the absorbed spelling and says nothing is the half-fix — the surfaces that look fine are the whole complaint. `refs` and `fmt --cross-refs` resolve qualified citations against the same project map and are equally wrong under an absorbed block.

It also keeps this warning and the neighbouring `[workspace]` cautions obeying one rule rather than two by accident, which matters more than either choice on its own: two workspace warnings shipped together with different surfaces would teach a reader that the surface is arbitrary.

**One honest difference, stated rather than left implicit.** A fact about the workspace *configuration* is knowable at boundary population, before any walk, so every command that merely loads the workspace can carry it. This fact is knowable only from the walk that meets the nested config. So it is a property of what the run walked: carried by every command that walks, and silent wherever the walk does not reach the block. That is the earliest point at which the fact exists, not a second rule chosen for convenience.

### 2.4 One shape on every surface

The finding takes the CLI-level `warning:` shape on stderr ([§FS-errors.2.2](../../functional-spec/FS-errors.md#22-cli-level-message)), with the block's config path and its `[workspace]` line rendered inside the message text — the shape the undecidable-claim warning of [§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces) already prints for the neighbouring fact about a `[workspace]` block.

A located finding on stdout for `check` plus a CLI-level line elsewhere was considered and rejected. Two shapes for one fact is the divergence this decision exists to prevent, and it would make the text a consumer greps for depend on which command produced it. The cost is paid under `--format=json`, where a CLI-level diagnostic carries `path` and `line` `null` ([§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format)) and the location lives in the message text. That is the same cost [§FS-check.4.3](../../functional-spec/FS-check.md#43-redundant-config-pair) and the deprecated-config-key warning already pay, and it buys one grep across six commands.

In `check` the warning is one of the report's warnings rather than a line printed past it. That is load-bearing twice over: it is what makes the warning stand in place of the `success` marker ([§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format)), so the verdict is catchable, and a catchable verdict is what makes §2.1's deprecation-path reasoning correct in the first place — [§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered) governs this as a verdict change precisely because a warning displaces `success`.

### 2.5 `include_root = false` on the block changes nothing

An unlisted block that opts its own root out of being a project still contributes a segment to every alias path below it ([§DF-nested-workspaces.3.5](DF-nested-workspaces.md#35-the-intermediate-node-reuses-include_root-and-defaults-to-a-project)), so the two scopes still disagree about how the projects under it are spelled. The key answers "is this block's root a project?"; the finding asks "does anything claim this block?". Same finding, same message.

## 3. Consequences

A repository that has an unlisted block sees one new warning per outermost such block and keeps its exit code, until the release [§RM-unlisted-workspace-error](../../roadmap.md#rm-unlisted-workspace-error-flip-the-unlisted-workspace-warning-to-an-error) names. Because a warning displaces `success`, a repository gating on that marker rather than on the exit code sees the change immediately — which is the point of a ramp that gives it a release to act in.

A repository that vendors a project carrying its own `[workspace]` inside its scan is told about it. That is a true report rather than a false positive: the vendored tree *is* being read into the host namespace under the host's grammar and kinds, and `[scan] exclude` is the edit [§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full) already recommends for it.

`grund config validate` is a natural second home and is deliberately left out. If that line moves it should move for every `[workspace]` warning at once rather than for this one alone.

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| **An error on arrival.** The strongest reading of the guarantee: a namespace nobody claims fails the run. | It flips a verdict on configurations that load today, with no command the tool ships to clear them ([§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations)). The ramp reaches the same end state one release later, having told every user first. |
| **A warning with no deadline.** Cheapest, and never breaks anybody. | Leaves the alias-path guarantee breakable indefinitely, and a dateless deprecation is the kind of line people learn to skip ([§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)). |
| **A dedicated walk for config files over the whole member subtree.** Catches blocks behind `[scan] exclude` and ignore files too. | Pays a second traversal for trees the run has deliberately been told not to read, to report an absorption that is not happening in them. The recorded limitation is the smaller cost. |
| **`check` only, on the [§FS-check.4.3](../../functional-spec/FS-check.md#43-redundant-config-pair) analogy.** One command pays for the claim resolution. | The analogy fails: a redundant pair is a fact about which file the run read, this is a fact about how every command spells its projects — and the ticket's own repro is a `grund list` (§2.3). |
| **A located finding on stdout for `check`, CLI-level elsewhere.** Gives `check` a `path:line:` an editor can jump to. | Two shapes for one fact, and a message whose text depends on the command that printed it (§2.4). |
