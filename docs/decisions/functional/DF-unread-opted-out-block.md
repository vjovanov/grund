# DF-unread-opted-out-block: the unread files of an opted-out block are a conditional warning that never ramps

**Status:** Accepted
**Date:** 2026-09-05

## 1. Context

`include_root = false` says "this block's own root is not a project". The consequence is written down — [§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces) has always said the node's own files are then scanned by **nobody**, not even under `--full` — and until now no run said it. The reproduction in [grund#71](https://github.com/vjovanov/grund/issues/71) is a grouping directory holding two dangling citations over which `check`, `check --full` and `list` are all silent and exit `0`: [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing through one config key, with no feedback at all.

That it should be reported was not in question. Two things were, and the ticket asked both by name. **Whether to warn unconditionally or only when the unscanned tree actually holds files a scan would read** — "the latter is quieter but costs a walk". And **how loudly**, given that the two neighbouring `[workspace]` findings shipped days earlier ([§DF-absorbed-scan-warning](DF-absorbed-scan-warning.md#df-absorbed-scan-warning-a-scan-its-own-members-swallowed-is-a-warning-with-a-named-release-not-an-error), [§DF-unlisted-workspace-block](DF-unlisted-workspace-block.md#df-unlisted-workspace-block-an-unlisted-workspace-block-is-reported-by-the-walk-that-meets-it)) both name the release they become errors in, and this one does not. A reader meeting all three in one week is owed the rule rather than three verdicts, which is why §2.3 below is written as a rule about every workspace finding and not as an excuse for this one.

## 2. Decision

### 2.1 Conditional on the tree, and the walk is what answers it

The warning fires only where the block's own tree actually holds a file a scan would have read, probed as [§FS-check.4.10](../../functional-spec/FS-check.md#410-include_root--false-leaves-the-blocks-own-files-unread) states it and stopping at the first hit.

The unconditional rule is the cheaper one and is rejected on what it does to a correct repository. A grouping directory that only groups — `grund.toml`, its members, nothing else — is exactly what `include_root = false` is for, and it is *the* shape nesting made common. An unconditional warning fires on it with no edit that clears it, and a warning nobody can clear is one people filter; then it is missing when it matters. [[§DF-absorbed-scan-warning](DF-absorbed-scan-warning.md#df-absorbed-scan-warning-a-scan-its-own-members-swallowed-is-a-warning-with-a-named-release-not-an-error) §3](DF-absorbed-scan-warning.md#3-alternatives-considered) already rejected an undated warning on that same reading — "permanent output that tools learn to filter" — and this would be the undated *and* unclearable version of it.

A cheaper condition than a walk does exist and is still not enough. The default `[scan] include` list is materialized rather than absent ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)), so a block that writes no `include` key still has real roots, and keeping only the ones that exist on disk and lie outside the members — which [§FS-workspace.2.1](../../functional-spec/FS-workspace.md#21-a-member-that-swallows-the-blocks-own-scan) already computes — is silent on a pure grouping directory at no cost. It is not enough because an existing `docs/` holding one PNG, one gitignored file, or one hidden file is read by nobody either, and that filter would warn about it with nothing to show. The approved rule subsumes the cheap one and keeps it as its first gate.

The walk is affordable against [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) because of what bounds it. Only a block that wrote `include_root = false` is asked. It is asked once per block per run, where the boundary is populated, not once per file. The roots it looks at are the ones that exist and are outside every member, which on a grouping directory is usually none. And the probe answers an existential question, so it stops at the first file rather than enumerating the tree — the cost is "is there one file here", not the size of what is under it. The one thing that is not negotiable is that the probe prune exactly as the scanner prunes: a prune that disagrees produces a warning on a correct configuration, which is the single outcome this finding cannot afford, and it is why [§FS-check.4.10](../../functional-spec/FS-check.md#410-include_root--false-leaves-the-blocks-own-files-unread) defines the scope by reference to [§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked) and to [§FS-workspace.2.1](../../functional-spec/FS-workspace.md#21-a-member-that-swallows-the-blocks-own-scan)'s own set rather than by a second reading of `[scan]`.

### 2.2 A warning, permanently, with no release it becomes an error in

The finding is a warning; it displaces the `success` marker ([§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format)); it leaves the exit code alone; and unlike its two neighbours it names no release it becomes an error in. There is no version constant, no [§RM](../../roadmap.md#rm-workspace-absorbed-scan-error-flip-the-absorbed-scan-warning-to-an-error)-style milestone that spends it, and no test holding a deadline ahead of the running version, because there is no deadline to hold.

The direct reason is that an error here would fail a run because somebody added a file. The same `grund.toml` is silent on Monday and reportable on Tuesday once `group/docs/notes.md` exists, so the verdict would not be one the config author could act on in advance — and one of the two remedies is to move content rather than to fix a key. The general reason is §2.3.

[§FS-check.2.2.1](../../functional-spec/FS-check.md#221-citation-direction-obligation-applies-to-nothing) is the standing precedent for the shape: a `[workspace]`-adjacent caution shipped in this same release with no exit-status change and no ramp. The undecidable-ancestor warning of [§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces) and the narrowed-`--full` caution of [§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full) are two more.

### 2.3 What makes a workspace finding ramp, and what makes one permanent

Three `[workspace]` findings shipped in one week and one of them behaves differently, so the rule is written here rather than inferred from three verdicts.

**A finding is eligible to become an error only when both of these hold.**

1. **Every repository in that state is wrong** — there is no reading of the state under which the repository is doing what it meant.
2. **The configuration has a way to say "I meant it"** — a key or an edit that removes the state by declaring intent, so the error is a verdict its author could have acted on before it landed.

Only then does [§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly) get asked, and it answers a different question: *how fast*. [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) flips the verdict inside one release where the fix is one command the tool ships; [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) names a release otherwise. That is [§DF-index-compatibility-ramp](DF-index-compatibility-ramp.md#df-index-compatibility-ramp-a-findings-ramp-follows-its-fix-command-not-the-size-of-the-offence)'s whole subject and nothing here disturbs it: it says which ramp, this says whether there is one.

The corpus, against both questions:

| Finding | Every such repository wrong? | A key that says "I meant it" | Verdict |
|---|---|---|---|
| [§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index) declaration missing from its index | yes | `index = false` on the kind | ramps ([§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)) |
| [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link) index entry is not a link | yes | the pass that writes it | error on arrival ([§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations)) |
| [§FS-check.4.8](../../functional-spec/FS-check.md#48-a-workspace-member-swallows-the-blocks-own-scan) a member swallows the block's scan | yes — it claims to be a project and reads nothing | `include_root = false` | ramps |
| [§FS-check.4.9](../../functional-spec/FS-check.md#49-unlisted-workspace-block) unlisted `[workspace]` block | yes — its projects are spelled two ways | list it, or keep it out of the enclosing `[scan]` | ramps |
| [§FS-check.2.2.1](../../functional-spec/FS-check.md#221-citation-direction-obligation-applies-to-nothing) an obligation applies to nothing | **no** — a scaffold nobody has declared in yet is in transit, not wrong | `citable = false` | permanent |
| [§FS-check.2.2](../../functional-spec/FS-check.md#22-empty-scan) empty scan, [§FS-check.4.5](../../functional-spec/FS-check.md#45-nothing-recognized) nothing recognized | **no** | **none** | permanent |
| [§FS-check.4.10](../../functional-spec/FS-check.md#410-include_root--false-leaves-the-blocks-own-files-unread) an opted-out block's unread files | **no** — a grouping directory with a README it does not need checked is correct | **none** | permanent |

This finding fails both tests, which is why it is the one that never ramps. Nothing is wrong: the author wrote `include_root = false` and meant it, and what the run reports is the *cost* of a correct configuration rather than a defect in it. And there is no way to say "I meant it" — the two remedies, making the block a project and pointing another project's `[scan] include` at the directory, both change what the repository **is** rather than record what it already meant. An error would therefore demand a different repository, which is not a demand a checker gets to make.

**The line this replaces, and why it does not hold.** The obvious rule — *a finding ramps when it is a property of the configuration alone, and stays a warning when it is a property of the configuration plus the tree* — is refuted by the first row of the table. A declaration missing from its index is a property of the configuration *and* the tree: the same `grund.toml` is clean until somebody writes a declaration. It ramps anyway. The correlation is real and worth knowing — a state that depends on the tree is more often one a repository can innocently be in — but it is a symptom of the two tests above, not the rule. Reading it as the rule would have made [§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index) a permanent warning, which this repository already decided it is not.

It is also, deliberately, not the size of the mistake. [§DF-index-compatibility-ramp.2.2](DF-index-compatibility-ramp.md#22-the-inversion-is-the-rule-working-not-a-bug-in-it) wrote that down once already for a pair whose greater offence rides the softer ramp; the same holds here, where the finding that silently drops a whole directory out of [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) is the one that stays a warning for good.

Two things this rule does **not** govern. It is about findings, not about syntax deprecations: an old spelling that stops loading — the `[[kinds]] prefix` key, bare `grund`'s historical default — is [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)'s own subject and always has an end date. And it says nothing about severity *within* a release: a finding ineligible to ramp is still a warning that displaces `success`, which is what keeps it catchable by a repository that wants it to be.

### 2.4 The message is short, and names one root

Two departures from [§FS-check.4.8](../../functional-spec/FS-check.md#48-a-workspace-member-swallows-the-blocks-own-scan)'s message, both deliberate.

It names **one** unread root rather than every one. §4.8's claim is universal — every scan root is inside a member — so listing them is the evidence for it; this claim is existential, one unread root is the whole finding, one edit clears all of them, and the probe stops at the first hit rather than paying for roots the answer does not turn on. What it must not name is the *file* it found: which one the walk hands back first is not fixed, and [§FS-errors.4](../../functional-spec/FS-errors.md#4-determinism) is the reason a root is named instead.

It is also **shorter**, at the cost of §4.8's "its declarations are unreachable" half. Unlike its two neighbours this warning fires on runs that are red for something else — a run that also fails on an unknown alias carries it — where it competes for attention with the diagnostic the reader came for, and the e2e corpus caps a non-zero case's stderr line for exactly that reason (`tests/e2e/README.md`). The breadcrumb, the unread tree, the cost, and both remedies all survive the cut; the declaration half does not, because a declaration in a directory nobody reads is the same fact as a citation in it and the citation is the one [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) is about.

### 2.5 It is emitted where the boundary is populated, like §4.8

One CLI-level `warning:` on stderr ([§FS-errors.2.2](../../functional-spec/FS-errors.md#22-cli-level-message)), asked once per block at the two points a run populates a block's member boundary, keeping its text under `--format json` and carrying no diagnostic `code` ([§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format)).

That is [§DF-absorbed-scan-warning.2.4](DF-absorbed-scan-warning.md#24-it-is-emitted-where-the-boundary-is-populated-not-where-check-reports)'s shape rather than [§DF-unlisted-workspace-block.2.4](DF-unlisted-workspace-block.md#24-one-shape-on-every-surface)'s, and the two differ on *when the fact exists*. An unlisted block is knowable only from the walk that meets its config, so that finding can be one of `check`'s report warnings. This one is settled before any walk, and five of its six surfaces have no report to put a diagnostic in — so the report shape is not on offer, and giving `check` alone a different one would make the text a consumer greps for depend on the command that printed it, which is the divergence [§DF-unlisted-workspace-block.2.4](DF-unlisted-workspace-block.md#24-one-shape-on-every-surface) exists to prevent.

The ticket's own reproduction is the argument for the surfaces: it demonstrates the defect with `check` **and** `list`. A finding only `check` produced would leave `list` exactly as silent as it was.

## 3. Alternatives considered

| Option | Why rejected |
|---|---|
| **Warn unconditionally**, as the ticket's first option. Cheapest, no walk. | Fires on a pure grouping directory, which is a correct configuration with no edit that clears it (§2.1). |
| **Keep only the roots that exist and lie outside the members**, with no probe. Nearly free. | Warns about an existing `docs/` holding one PNG or one gitignored file, with nothing to show. The approved rule keeps this as its first gate (§2.1). |
| **An error, on the strength of the hole it names.** A whole directory drops out of [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration). | Fails both tests in §2.3: the state is not a defect, and no key records the intent an error would demand. It would also fail a run because somebody added a file. |
| **A warning with a named release, matching the two siblings.** Three findings, one behaviour, nothing to explain. | Uniform for its own sake. The siblings ramp because they are eligible to; copying the date without the eligibility is what §2.3 exists to stop. |
| **A report warning in `check`, CLI-level elsewhere.** Gives `check` a JSON diagnostic with a `code`. | Two shapes for one fact, and a grep that depends on the command (§2.5). |
| **A `[workspace]` key to silence it** — the "I meant it" §2.3 finds missing, which would make an error eligible later. | A key whose only effect is to suppress a caution is a suppression mechanism, which [§FS-non-goals.9](../../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization) keeps out of the severity model. If such a key is ever wanted it should arrive as a decision of its own, with the ramp it enables argued then. |

## 4. Consequences

A repository with a grouping directory that holds documentation of its own starts printing one warning per such block on every command that walks, and keeps its exit code — permanently, unless it makes the block a project or points another project's scan at the directory. A repository whose grouping directories only group sees nothing, which is most of them.

This closes the last of the three silent-scope holes [[§DF-absorbed-scan-warning](DF-absorbed-scan-warning.md#df-absorbed-scan-warning-a-scan-its-own-members-swallowed-is-a-warning-with-a-named-release-not-an-error) §4](DF-absorbed-scan-warning.md#4-consequences) listed as open.

What stays open, and is recorded rather than papered over: a repository that deliberately keeps prose in an opted-out block and does not want it checked has no way to say so, and takes the warning for good. That is §2.3's second test failing, and it is why the finding cannot be an error; if the case turns out to be common, the key that answers it is the alternative above.
