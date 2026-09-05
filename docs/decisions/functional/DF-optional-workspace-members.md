# DF-optional-workspace-members: an absent member is declared in a sibling list, and the run announces the namespace it did not check

**Status:** Accepted
**Date:** 2026-09-05

## 1. Context

A repository whose hardware namespaces live in a private git submodule had no way
to run `grund check` in the checkout CI makes ([grund#85](https://github.com/vjovanov/grund/issues/85)).
Both obvious routes fail, and both fail correctly. Leaving the submodule in
`[workspace] members` is a config error at that line, because [§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration)
makes a missing member path fatal. Taking it out unregisters the alias, so every
`§vendored/…` citation becomes an unknown-alias error at its own site
([§FS-workspace.4](../../functional-spec/FS-workspace.md#4-resolution), [§FS-check.3.8](../../functional-spec/FS-check.md#38-cross-project-citation-failure)) — thousands of lines across the reporting tree.

The strictness is not in question. [§FS-workspace.5](../../functional-spec/FS-workspace.md#5-command-scope) argues it at length, and the
issue's author argues it back: the release before this one skipped citations into
an unresolvable alias silently, and that is how `§pod/…` references into a
deleted namespace survived two weeks in the tree this ticket comes from. What is
missing is a way to say *this one may be absent*, and to be told when it is.

Three shapes were on the table, in the issue's own order of preference:

1. **A structured member entry** — `members = [{ path = "vendored", optional = true }]`,
   so the intent sits on the member it describes.
2. **A run flag** — `grund check --allow-missing-members`, degrading the config
   error to a warning for the run.
3. **A better error message** — the refusal naming an escape hatch, if there is
   one to name.

## 2. Decision

Adopt a **sibling list**: an optional `optional_members` under `[workspace]`,
with the same grammar as `members` and globs barred.

```toml
[workspace]
members          = ["apps/api", "packages/*"]
optional_members = ["vendored"]
```

An entry that is present is an ordinary member. An entry that is absent is
skipped, its namespace is recorded as unverified rather than unknown, and the run
names it in the report and withholds `success`. The contract lives in
[§FS-workspace.2.2](../../functional-spec/FS-workspace.md#22-a-member-that-may-be-legitimately-absent), [§FS-workspace.4](../../functional-spec/FS-workspace.md#4-resolution) and [§FS-check.4.9](../../functional-spec/FS-check.md#49-a-workspace-member-declared-optional-is-absent); the blind spot it creates
is enrolled in [§REQ-no-missed-citation.2](../../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded). The third option ships regardless: the
`workspace member does not exist` refusal now names the key.

### 2.1 A sibling list, because this repository has already refused the other shape

The structured member entry was rejected here once, on grounds that apply verbatim
a second time. [§DF-workspace-member-descriptions.3.2](DF-workspace-member-descriptions.md#32-the-root-alternatives-fight-existing-semantics) turned down
`members = [{ path = "apps/api", description = "…" }]` because it "kill[s] the
`packages/*` glob ergonomics of [§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration) and force[s] a second parse shape
for `members`". Both are still true, and the author reaffirmed both when
approving this: reusing the existing grammar, keeping `grund_config_version` at
`1`, and letting an older binary fail loudly on the unknown key is the better
shape.

The mechanical half is worth stating because it is what makes the change small.
`members` is read by one comma-split over a bracketed line of quoted strings, and
`optional_members` reuses it whole — the same list parser, the same per-entry path
validation, the same renderer for `grund config show`, and the same
unknown-key rejection under `[workspace]` that gives an older binary its loud
failure. An inline table would have needed a second parse shape, a second render
shape, and a migration story for every config already written.

What the sibling list gives up is adjacency: the intent sits one line away from
the member rather than on it, and a reader has to look at two lists to know what
a `[workspace]` block contains. That is the cost, and it is paid once per block
rather than once per entry.

### 2.2 Not a run flag, because a flag records no intent

`--allow-missing-members` was rejected for two reasons, and the author agreed
with both. It records nothing: the next reader of the config learns that a member
exists and not that it may be absent, so the fact lives in a CI file, or in
nobody's head. And it is indiscriminate — it excuses every missing member, so the
day one of them goes missing by accident, the flag that was added for the
submodule swallows it too. A key in the config is a claim the repository makes
once, per member, in the file the member is listed in.

### 2.3 The alias comes from the entry text, and disagreement is an error

An absent member has no config, so [§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases)'s rule — the alias is the
member's `project_name`, or the directory basename when omitted — has nothing to
read. The only name the block can recover is the entry's last path segment, so
that is the alias ([§FS-workspace.2.2.2](../../functional-spec/FS-workspace.md#222-the-alias-of-an-optional-member)).

The consequence is the important half, and it is the reason the decision is not
simply "skip it". If a *present* optional member's `project_name` were allowed to
differ from that segment, the same citation text would resolve in a full checkout
and name nothing in a partial one — a divergence visible only in the checkout
least able to notice it, which is strictly worse than the failure this decision
removes. So the disagreement is a config error at the `optional_members` line,
naming both names and choosing neither: either may be the one the tree's existing
citations already write.

Barring the `/*` glob follows from the same place. A glob discovers namespaces by
reading a directory; an absent directory yields none, so an optional glob would
be a key that appears to work and does nothing. It is refused, and the refusal
names the shape to write instead, one concrete entry per namespace — a user
meeting a refusal needs the form that works.

### 2.4 Exit `0`, and the announcement is what buys it

A run that still exits non-zero has not unblocked the CI job the ticket is about,
so an absent optional member leaves the exit code alone. That is a green run over
a region grund did not check, which is the failure class [§REQ-no-missed-citation](../../requirements/REQ-no-missed-citation.md#req-no-missed-citation-every-citation-the-run-reads-is-checked)
exists to prevent, and three things are what make this instance legitimate rather
than a regression to the silent skipping of the release before it: the skip is
opt-in, it is declared in the config, and every run announces it.

The announcement therefore carries the weight, and it is specified as a **located
warning on stdout** rather than as the CLI-level stderr caution its two nearest
neighbours use ([§FS-check.4.9](../../functional-spec/FS-check.md#49-a-workspace-member-declared-optional-is-absent)). The argument is in that section; the decision it
rests on is that this is the one finding in grund whose subject is the *coverage
of the report itself*, in the one case where an incomplete run still exits `0`.
With the exit code carrying nothing, an announcement on the stream a quieted log
drops would leave stdout signalling only that `success` was withheld — an absence,
which reads exactly like silence.

## 3. Alternatives considered

**The inline table, `members = [{ path, optional }]`.** Rejected on §2.1, which
is [§DF-workspace-member-descriptions.3.2](DF-workspace-member-descriptions.md#32-the-root-alternatives-fight-existing-semantics) applied a second time. It is the shape
the issue asked for first, and the author withdrew it on reading the precedent.

**`--allow-missing-members`.** Rejected on §2.2.

**Deriving the alias from the last segment only when the member is absent, and
from `project_name` when it is present.** Rejected on §2.3: it is the shape that
makes one citation text mean two things, and it fails silently in the direction
nobody is watching.

**Allowing `optional_members = ["hardware/*"]`.** Rejected on §2.3. A tree that
spells its members with a glob expands it by hand for the ones that may go
missing; that cost is real and was accepted knowingly, because the alternative is
a key that silently contributes nothing.

**Widening "absent" to cover an empty directory.** Rejected in
[§FS-workspace.2.2.1](../../functional-spec/FS-workspace.md#221-what-absent-means). An uninitialized submodule materializes as an empty
directory, so this looks like the more helpful rule; it would put a typo'd or
half-created directory on the unverified path and move the boundary of the blind
spot somewhere no line of the repository records. The repository meeting that
case names the namespaces under the submodule instead.

**A stderr `warning:` for the announcement, matching [§FS-check.4.7](../../functional-spec/FS-check.md#47-a-workspace-member-swallows-the-blocks-own-scan) and §4.8.**
Rejected on §2.4. It is the consistent shape for a fact about a `[workspace]`
block, and consistency is a real argument; it loses to the author's condition
that the announcement survive whatever quieting a CI log applies, which a stderr
line does not.

## 4. Scope

In scope: the config key and its validation, member expansion, the unverified
resolution state, the announcement, the `optional_members` mention in the
`workspace member does not exist` refusal, and `grund config show`.

Out of scope for this slice, and named so the gaps are not mistaken for
oversights. The single-ID read of [§FS-workspace.8.1](../../functional-spec/FS-workspace.md#81-grund-aliasid) still answers
`grund vendored/SPEC-007` with `unknown project alias`, which is the wrong word
for a name the config declares — `check` is the only surface this decision
changes, because `check` is the only one that renders a verdict about whether
every citation resolves. The generated `templates/grund.toml` does not teach the
key with a commented line the way it teaches `project_description`: a description
is something most repositories want and this is a key most will never need, so
the line would be noise in every generated config to spare one reader a
documentation lookup. And an absent member's namespace is skipped whole rather
than checked against a cached catalog of what it used to hold, which is the
shape [§FS-workspace.7](../../functional-spec/FS-workspace.md#7-neighboring-repos) would need for neighbouring repositories and is a different
feature.
