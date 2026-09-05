# DF-nothing-recognized: a run that recognized nothing says so, and says it as a warning

**Status:** Accepted
**Date:** 2026-08-23

## 1. Context

A project whose headings are written for a different `[id] format` than the one configured ([§FS-config.3.2](../../functional-spec/FS-config.md#32-id--id-grammar)) declares nothing. The classic case is a docs tree written slug-only — `# FS-login: …` — under the default `{kind}-{number}-{slug}`: every heading in it is heading-shaped and none of them is a declaration.

Until this decision, that tree's verdict was one word:

```console
$ ls docs
FS-alpha.md  FS-beta.md  FS-gamma.md
$ grund check
success
$ grund list
$ echo $?
0
```

Three files read, three headings that look exactly like declarations, and nothing on screen separating that run from a run over a fully grounded tree. The empty-scan caution ([§FS-check.2.2](../../functional-spec/FS-check.md#22-empty-scan)) does not fire — files *were* scanned — and no citation dangles, because a tree that declares nothing usually cites nothing either. `grund list` is empty, which is the honest answer to the question it was asked and no answer at all to the question the user has.

This is the worst first run `grund` can give. Every other way of getting the config wrong is loud: a scope that matches no files is [§FS-check.2.2](../../functional-spec/FS-check.md#22-empty-scan), a config that does not load is [§FS-config.4.3](../../functional-spec/FS-config.md#43-invalid-config-behavior), a citation to a missing ID is [§FS-check.3.1](../../functional-spec/FS-check.md#31-dangling-citation). Only this one is silent, and it is the one a first-time adopter is most likely to hit, because the `[id] format` is the single setting they have to match before anything else in the tool works.

## 2. Decision

### 2.1 A run that read files and recognized nothing in them emits a caution

[§FS-check.4.5](../../functional-spec/FS-check.md#45-nothing-recognized): a walk that read at least one file and found **no declaration and no citation** names that fact on stderr, with the shape a declaration and a citation take under the configured format.

The condition is "recognized nothing", not "declared nothing". A project that only cites — a workspace member whose code points at another member's specs ([§FS-workspace.1](../../functional-spec/FS-workspace.md#1-citation-syntax)) — declares nothing and is working exactly as intended, so declaration count alone would report a healthy project as broken. A file the grammar matched *somewhere* proves the grammar and the tree agree, which is the whole question this caution asks.

### 2.2 It is a warning, not an error

The exit code stays `0`. Two reasons, and the second is the load-bearing one.

A tree with nothing in it yet is the ordinary state of a repository on its first day, and of any project that adopts `grund` before it writes its first spec. An error would make `grund check` fail on `grund init` output until the first declaration lands, which turns the tool's own scaffolding into a broken build.

And an error would flip the verdict of every repository in that state, silently, on upgrade — which [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) reserves for a deprecation window and [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) for a migration one command can complete. Neither applies: there is no old form to carry beside a new one, and the fix — write the headings to match `[id] format`, or set `[id] format` to match the headings — is a judgement about the project, not a command `grund` can run for it.

What the warning actually buys is the word `success`. A warning stands in place of the success marker ([§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format)), so the run that recognized nothing stops printing the same word as the run that checked everything. That was the defect: not the exit code, which was defensible, but a green line that claimed a verdict the run had not reached.

### 2.3 The caution names shapes, never a corrected ID

The message renders `<KIND>-<NNN>-<slug>` from the configured `[id] format` — the substitution [§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints) already makes for the managed entrypoint block — and lists the configured `[[kinds]]` prefixes. It does not build an example ID out of `[id] number_pattern` and `[id] slug_pattern`, and it does not propose what any heading in the tree should have been called.

An ID assembled from those patterns is a guess about what they accept: `number_pattern = "[A-Z]{2}"` makes `FS-001-example` a lie, printed by the tool, in the message whose whole job is to tell the user what the grammar wants. The shape is derived from the format template alone, which is a literal fact about the config, so it is right for every pattern.

### 2.4 It is withheld from a run that has any other finding

Exactly the gate [§FS-check.2.2](../../functional-spec/FS-check.md#22-empty-scan) uses. A report that already says something is not the silent verdict this rule exists to break, and a repository with a stale entrypoint block does not need a second message about the same run.

## 3. Rejected alternative: warn on each heading that looks like a declaration

Report every `# <KIND>-…: <title>` heading whose ID fails `[id] format`, at its own `path:line`. This is the near miss [§FS-check.5](../../functional-spec/FS-check.md#5-what-grund-does-not-check) already records as unflagged, and it shipped as [§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-near-miss).

It is a better message and a different rule, and it is not a substitute for this one. It requires deciding what "looks like a declaration" means — which is a heuristic, of the kind [§GOAL-agent-grounding.3](../../goals.md#3-what-this-rules-out) is careful about — plus an opt-out for files that legitimately write such headings as prose. This decision needs none of that: "the run recognized nothing" is arithmetic over what the scan already recorded, with no line to judge and nothing to suppress. When the per-heading rule ships it makes this warning more actionable, not redundant: the count is the diagnosis, the sites are the fix.

## 4. Consequences

- One new warning code, `nothing-recognized` ([§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format)), `path`/`line` null and on stderr like the empty-scan caution it is the sibling of.
- Per project, not per run ([§FS-check.4.5](../../functional-spec/FS-check.md#45-nothing-recognized)): in a workspace each member is asked the question against its own config, because one member's grammar mismatch says nothing about another's.
- No repository's exit code changes. A repository that recognized nothing and printed `success` now prints the caution instead — a byte change to a passing run, governed as the addition of a finding by [§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered).
- `grund list` is unchanged. An empty listing is the correct answer to "list the declarations" in a tree that has none, and it is the read a user reaches for to confirm this warning — [§FS-check.4.3](../../functional-spec/FS-check.md#43-redundant-config-pair) draws the same line about which surfaces repeat a caution.
