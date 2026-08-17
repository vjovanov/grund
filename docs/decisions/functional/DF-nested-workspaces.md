# DF-nested-workspaces: a nested project is named by its whole alias path

**Status:** Accepted
**Date:** 2026-08-17
**Authors:** Claude Opus 5, Vojin Jovanovic

## 1. Context

The workspace surface shipped with a flat member list: a root `grund.toml`
names every project, and a member that declared its own `[workspace]` was a
load-time error ([§FS-workspace.6](../../functional-spec/FS-workspace.md#6-nested-project-boundary), before this decision). Real repositories group
related sub-projects under a directory — four independently checked hardware
projects under `hardware-current/`, beside a `hardware-final/` — and had to
enumerate every leaf at the root, so the grouping directory could not own its
own member list and the hierarchy survived only in human-facing prose
([issue #47](https://github.com/vjovanov/grund/issues/47)).

Lifting the restriction is one line of expansion code. What needed deciding is
what the lifted configuration *means*, and the issue named the three open
questions: whether aliases are global across the tree or scoped per parent, how
duplicates and cycles are detected across levels, and whether an intermediate
grouping node is itself a citable project.

## 2. Decision

A workspace member may declare its own `[workspace]`, to any depth. A project is
named by its **whole alias path** — one segment per workspace level, read from
the outermost workspace down ([§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces)):

```text
§hardware/sprayer/FS-nozzle    a leaf, named through its parent
§hardware/AR-bus               the intermediate node's own declaration
§final/FS-spec                 a top-level member
```

- **Uniqueness is per level.** Two projects under different parents may share a
  segment; their paths still differ. A collision *within* one sibling set is the
  existing duplicate-alias error.
- **The outermost root contributes no segment**, so a workspace with no nesting
  is spelled exactly as it is today.
- **Paths are absolute with respect to the outermost workspace**, never the
  scope a command started in.
- **`include_root` is per block.** An intermediate node is a project — alias,
  scan, citable — unless its own block sets `include_root = false`, and then it
  is pure grouping that still contributes its segment to the paths below it.
  Every block must put at least one project in scope.
- **Cycles are a config error, by canonical path.** A member resolving to a
  project root the workspace already holds fails at the `members` line that
  introduced it.

## 3. Why this shape

### 3.1 The alternative was flat, globally-unique aliases

The competing model — one segment, unique across the whole tree — is cheaper in
every mechanical respect: the grammar is untouched, citations stay short, and
nesting or un-nesting a group never rewrites a citation. Its cost is that leaf
names must be globally unique, and the escape hatch is real but manual: a
collision is a located load-time error that `project_name` fixes in one line.

Paths won because that escape hatch does not scale with the thing nesting is
*for*. A repository reaches for grouping precisely when it has many
sub-projects, and many sub-projects are exactly when independent groups start
picking the same obvious names — `api`, `core`, `pod`, `docs`. Under flat
aliases the second group to want `api` must invent `hardware-current-api`,
which is the alias path spelled by hand, in a config key, with no checking that
it stays true when the directory moves. If the answer to a collision is going to
be a qualified name, the qualified name should be the model rather than a naming
convention the tool cannot enforce.

The counter-argument — "an alias path is a path, and [§DF-subproject-namespaces](DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §5
rejected path-based citations" — does not survive inspection. What that decision
rejected was citing through the *filesystem*: `../api`, `packages/api/FS-x`,
which break when a directory is renamed. An alias path is a chain of configured
names; renaming the `hardware-current/` directory changes no citation, because
the segment comes from `project_name`. It is a namespace path, the way
`crate::module::Item` is, not a file path.

### 3.2 The real cost is short names, and it is paid at the diagnostic

Full paths make one mistake likely: writing `<§>sprayer/FS-x` where
`<§>hardware/sprayer/FS-x` is required — especially for an agent editing one
file without the tree in view. So `check` answers that mistake specifically. An
unresolved path is matched against the projects in scope by path suffix first (a
dropped prefix), then by last segment (a wrong prefix), then by edit distance (a
typo), and the diagnostic names what it found ([§FS-check.3.8](../../functional-spec/FS-check.md#38-cross-project-citation-failure)):

```text
docs/FS-root.md:3: unknown project alias sprayer; did you mean hardware/sprayer?
```

This is what makes the tradeoff acceptable rather than merely defensible: the
failure mode of the model we chose is a compile-time error carrying its own fix,
not a silent misresolution and not a search through the config.

### 3.3 One string, not a list of segments

The namespace stays a single string with `/` inside it. That is the difference
between widening one regex capture and teaching a second shape to the resolver,
the scanner, the completion helper, the `--project` filter, and the `refs` and
`list` JSON keys. The split between path and ID needs no new rule either: an ID
never contains `/`, so the last separator is the boundary, applied identically
by the scanner, the CLI argument parser, and the `[citations]` rule parser
([§AR-workspace.6.1](../../architecture/AR-workspace.md#61-nested-workspaces-are-one-recursion-not-a-second-namespace-model)).

### 3.4 A path means the same thing at every scope

A run narrowed to a subtree resolves a *subset* of the same paths, never a
re-spelled set of its own: inside `hardware/`, `<§>hardware/sprayer/<ID>` still
names what it names at the repository root, and `<§>final/<ID>` is simply
unknown there. The alternative — naming a subtree's projects from the subtree —
would let a citation pass a local check and fail the run CI does, which is
[§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing quietly in the one place it is supposed to
hold. The prefix is recovered by climbing the chain of ancestors that each
declare `[workspace]` and list the directory below it.

The guarantee reaches exactly that chain, and no further
([§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces)). A `[workspace]` block no enclosing block
lists is claimed by nobody: the outer run ignores it and absorbs its tree into
the enclosing namespace, a run inside it names itself from itself, and the two
disagree. Diagnosing that needs a walk for config files no pass performs, so it
is recorded as a known limitation there rather than designed around here.

### 3.5 The intermediate node reuses `include_root`, and defaults to a project

A grouping directory usually holds something — a README, a roadmap, shared docs
about why the sub-projects sit together. If the node were never a project, that
content would be unreachable: the outermost scan stops at the member boundary
([§FS-workspace.6](../../functional-spec/FS-workspace.md#6-nested-project-boundary)) and no other scan covers it, so citations in it would
silently go unchecked — a [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) hole opened by a config key.

`include_root` already means "is this block's own root a project?", and an
intermediate node is a workspace root, so reusing it adds no config surface.
Opting out drops the project but not the segment: `<§>hardware/sprayer/<ID>` is
unaffected by whether `hardware` is itself checked, which keeps the naming model
independent of a scan decision.

### 3.6 Termination is a duplicate check, not a depth limit

Member paths are relative and reject `..` ([§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration)), so a cycle needs a
symlink. Comparing canonical roots against the ones already collected catches
exactly that, reports it at the line that caused it, and bounds the walk as a
side effect: each recursion step consumes a root that can never be consumed
again. A depth cap would be a second, arbitrary number that fires on legitimate
deep trees and still would not name the offending line.

## 4. Consequences

- [§FS-workspace.1](../../functional-spec/FS-workspace.md#1-citation-syntax) grammar becomes `<§>[alias/…/alias/]<ID>[.section]`, and
  [§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces) / [§AR-workspace.6.1](../../architecture/AR-workspace.md#61-nested-workspaces-are-one-recursion-not-a-second-namespace-model) replace the rejection rule. No
  `grund_config_version` bump: `[workspace]` is unchanged as a schema, and a
  config that nests is only ever written for a binary that accepts it.
- Existing single-level workspaces are unaffected — one level means one segment,
  which is what they already write.
- [§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules) citation-direction rules take the same path form
  (`group/api/AR`), split at the last `/` like every other consumer.
- [§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members) walks to the *outermost* workspace root and renders
  full paths, so a generated entrypoint anywhere in the tree lists the alias
  paths CI resolves.
- Converting the issue's flat leaf list into a nested tree **does** rewrite its
  cross-project citations, once, from `<§>sprayer/<ID>` to
  `<§>hardware-current/sprayer/<ID>`. §3.2's diagnostic is what makes that
  migration mechanical rather than a search.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| **Flat, globally-unique aliases.** Grammar untouched; citations stay short; nesting and un-nesting a group never rewrite a citation. | The escape hatch for a collision is renaming a project to a hand-written qualified name (`hardware-current-api`) that the tool cannot keep true when the directory moves. Nesting is adopted exactly when many sub-projects make collisions likely, so the model should carry the qualification (§3.1). |
| **Flat aliases, qualify only on ambiguity.** Short form when unique, full path only when two projects share a name; ambiguity fails loudly. | Best ergonomics, but a citation's required spelling then depends on what *else* exists in the tree: adding a second `api` anywhere re-spells citations in projects that never changed. It also forces a canonical-form choice on `grund list`, `refs --format json`, `--project`, and completions that the other two models get for free. |
| **Auto-qualify only on collision, silently.** No new syntax; no user-visible change until it is needed. | Same dependence on siblings as above, without the loud failure — the worst of the three. |
| **Namespace as a list of segments** rather than one `/`-joined string. | Structurally tidier, but it teaches a second shape to the resolver, the scanner, completions, the `--project` filter, and the JSON keys. An ID never contains `/`, so one string with a last-separator split is unambiguous and touches nothing (§3.3). |
| **Intermediate node is never a project** (grouping only). Simplest to explain. | Its files would be scanned by nobody — the outermost scan stops at the boundary — so citations in a grouping directory silently stop being checked (§3.5). |
| **A new `[workspace] nested = true` opt-in.** Nesting stays off unless asked for. | The nested config is already explicit — a member either declares `[workspace]` or does not. A second key to confirm the first only refuses configurations that are already unambiguous. |
| **Name a subtree's projects from the subtree** when a run is narrowed. | A citation could pass `grund check` inside `hardware/` and fail the run CI does at the repository root (§3.4). |
| **Depth limit (e.g. 8 levels).** Cheap termination guard. | An arbitrary number that rejects legitimate trees and, when it fires, names no offending line. The canonical-root duplicate check terminates the walk *and* diagnoses the actual mistake. |
