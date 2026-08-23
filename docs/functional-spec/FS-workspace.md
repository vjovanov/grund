# FS-workspace: grund validates cross-project citations in a workspace

`grund` can treat a repository as a workspace of independent project namespaces.
Local citations stay unchanged, while cross-project citations name a stable alias
before the local ID. This keeps the zero-config single-project path intact
([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)) and gives larger repos an explicit resolver for sub-projects
without forcing project names into every ID ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)). The alias
syntax is chosen in [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos).

## 1. Citation syntax

A normal citation still resolves inside the current project:

```text
§FS-login
§FS-login.2.1
```

A cross-project citation writes the target project's alias path, a slash, then
the target ID:

```text
§api/FS-login
§root/GOAL-compatibility
§payments/FS-refunds.3.2
§hardware/sprayer/FS-nozzle
```

The grammar is:

```text
§[alias/…/alias/]ID[.section]
```

Each `alias` is a lowercase slug: it starts with a letter and then uses lowercase
letters, digits, or `-`. The alias path carries **one segment per workspace
level** (§6.1); in a workspace with no nesting it is always a single segment, so
the common form is `<§>alias/<ID>`. The slashes are part of the citation
namespace, not part of the ID — and since an ID never contains `/` (the ID
grammar rejects one on load, [§FS-config.3.2](FS-config.md#32-id--id-grammar)),
the last slash in the token is always the boundary between the two.

For an unqualified citation, the `ID[.section]` part uses the current project's
ID and section grammar. For a qualified citation, it uses the target project's
grammar: `<§>api/FS-001-session` is parsed with `api`'s `[id]` config, even when
the citing/root project uses a different ID format.

The shape is read the same way in **every** repository, including one with no
`[workspace]` at all: what makes a token a citation is the marker
([§FS-check.1.1](FS-check.md#11-recognized-citations)), and a marked token of this shape is a citation whose alias
path resolves against nothing, so it is reported as an unknown project alias at
its site rather than skipped (§5, [§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)) — the same rule that
stops a member-local run from shipping a cross-project citation the workspace
root would reject. The multi-segment path therefore makes one *file path*
readable as a citation: `<§>docs/functional-spec/FS-login.md` is a two-segment
alias path plus an ID, and marking it is what says "resolve this". A path meant
as a path is written without the marker, or outside the citation.

## 2. Workspace configuration

A workspace is declared in the root project's `grund.toml` ([§FS-config.1](FS-config.md#1-file-location-and-discovery)):

```toml
project_name = "root"

[workspace]
members = ["apps/api", "packages/*"]
include_root = true
```

`members` is a list of paths or single-segment trailing globs, resolved relative
to the config root. Member paths must be relative, must not use `.` or `..`, must
not use platform-specific absolute forms or backslash separators, and must not
overlap after glob expansion; one member root cannot contain another member
root. Each must also resolve — through symlinks, which is the only way left to
escape — to a directory *strictly inside* the config root that lists it: not that
root itself, not an ancestor of it, not another tree. Invalid member entries,
missing member paths, escaping member roots, and overlapping expanded
roots are config errors reported at the `members` line per
[§FS-config.4.3](FS-config.md#43-invalid-config-behavior), and they name the entry as the config wrote
it rather than the root it resolved to — a resolved root renders as nothing when
it is the block's own and as an absolute path once it leaves the tree. `packages/*` means
every direct child directory under `packages/`; recursive `**` globs are not
part of v1. `include_root` defaults to `true`; when false, `grund check` at the
workspace root checks only member projects.

Each member is a separate project namespace. If a member has its own config —
either discovery form, `.agents/grund.toml` or a bare `grund.toml`
([§FS-config.1](FS-config.md#1-file-location-and-discovery)) — that file configures the member. If it has neither, the
canonical defaults apply with the member directory as the config root. Root and
members choose independently, so a workspace may mix the two forms.

## 3. Aliases

The root alias is the root config's `project_name`, or `root` when omitted.
A member alias is the member config's `project_name`, or the member directory's
basename when omitted.

Aliases must match the lowercase slug grammar in §1 and must be unique **among
siblings** — the root project and the top-level members share one level, and
each nested `[workspace]` block's members share another (§6.1). A duplicate or
invalid alias is a launch-time error, because a qualified citation would
otherwise have two possible targets. Two projects under different parents may
carry the same alias: their alias paths still differ, so nothing is ambiguous.

A project's optional one-line `project_description` ([§FS-config.3](FS-config.md#3-schema)) follows the
same residency rule as the alias: a member's description comes from the
member's own config, the root row's from the root config, and a member without
its own config has none ([§DF-workspace-member-descriptions](../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists)).
Unlike the alias it is presentation metadata only — generated workspace member
lists render it beside the alias ([§FS-init.2.3.4.15](FS-init.md#23415-workspace-members)), and it never
participates in alias derivation, citation resolution, or `check` semantics.

## 4. Resolution

During `grund check`:

- `<§>ID` resolves only against declarations in the current project.
- `<§>alias/ID` resolves only against declarations in the named workspace project.
- `<§>alias/ID.section` additionally requires that the target declaration contain
  the cited section.
- an unknown alias is an error at the citation site;
- a known alias with no matching declaration is an error at the citation site.
- diagnostics for a known alias render the `<ID>` and section separator with the
  target project's `[id]` config, not the citing project's config. The literal
  source token remains the scanner's evidence, but the diagnostic names the same
  target that resolution attempted.

Cross-project references are deliberately never resolved by path syntax such as
`../FS-login` or `packages/api/FS-login`; aliases are the stable handles.

## 5. Command scope

`grund check` run at a workspace root checks the root project and all configured
members, aggregates diagnostics, and prints the same `path:line: message` shape
as a normal check ([§FS-check.2.1](FS-check.md#21-report-format)). Paths are rendered relative to the workspace
root when `[output] relative_paths = true`.

`grund check <member>` (or `grund check` invoked from inside a member tree)
discovers the member's own config first and validates it as an independent
project. Qualified citations such as `§root/<ID>` or `§sibling/<ID>` cannot be
resolved without the workspace context; each such citation produces an
`unknown project alias <name>` error at the citation site
([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)). This matches [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6 — silent skipping
would let a passing member check ship a cross-project reference that no longer
resolves at the workspace root, which violates [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration). Run
`grund check` at the workspace root to validate cross-project citations.

The member-local scanner recognizes the `<§>alias/` prefix before applying the
member's local ID grammar. That way `<§>root/FS-root` is still an unknown-alias
citation inside a default `{kind}-{number}-{slug}` member, rather than plain
text that disappears from `check`.

Member-local recognition uses a fixed fallback ID shape — `KIND[-NUM]-SLUG`
with an uppercase-or-digit kind and a non-empty slug — because the workspace
catalogue (and therefore each target's `[id] format`) is unreachable at member
scope. A qualified citation whose tail does not match that shape (lowercase
kinds, slug-only ID grammars that don't split on `-`/`_`, kinds with
non-`[A-Z0-9]` characters) is not flagged at member scope; the workspace-root
run, which parses each qualified tail with the target project's grammar, is
the one place that catches every shape. Run `grund check` at the workspace
root for full coverage.

A relaxed standalone mode (downgrade `unknown project alias` from error to
warning on a member-only run) is deferred follow-up; see
[§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6.

## 6. Nested project boundary

Workspace members are namespace boundaries. When the root project is checked as
part of a workspace, the root scan does not scan member project directories or
their descendants, even if the root project's `[scan] include` names a path
inside a member. The member is scanned separately under its own config and alias.

This prevents a child project declaration from accidentally becoming a duplicate
or dependency of the root namespace.

**The boundary is mutual, and it belongs to the directory rather than to the path
that reaches it.** A member's own scan stops at every *other* project in the
workspace exactly as the root scan stops at the members — at a sibling member's
files, and at the root project's. Ordinary descent cannot cross the line, since
no project root contains another except along the workspace tree itself, but a
symlink can, and a symlink is followed ([§FS-config.3.5.1](FS-config.md#351-a-symlink-in-the-tree-is-followed)).
So the directory a link resolves to is asked which project owns it — the
innermost project root that contains it — and a directory owned by another
project is not descended into, whichever project's walk met it and under
whatever name. The harm is the same in every direction: `packages/a/docs/b ->
../../b` files `b`'s declarations under `a`'s namespace and reports them as
duplicates of themselves, which is what this section forbids of the root scan. A
directory that belongs to **no** project in the workspace is not a boundary: it
is outside content the repository deliberately linked into the tree, and it is
read there like any other ([§FS-config.3.5.1](FS-config.md#351-a-symlink-in-the-tree-is-followed)).

A member checked on its own is an independent project (§5) and does not load the
workspace, so it cannot know where the other projects are; this boundary is a
property of a run that loaded them, not of the member's own config.

### 6.1 Nested workspaces

A member may itself declare `[workspace]`. Its `members` are paths under *that*
member's config root, resolved and validated by the rules in §2 exactly as the
outermost root's are. The tree may nest to any depth.

A project is named by its **whole alias path**: one segment per level, read from
the outermost workspace down (§1). The outermost root contributes no segment, so
a workspace with no nesting is spelled exactly as it is today. Given a root with
`members = ["hardware", "final"]` and `hardware` declaring
`members = ["sprayer", "pod"]`:

```text
§root/GOAL-x            the root project        §final/FS-x     a top-level member
§hardware/AR-bus        the node's own decl     §hardware/sprayer/FS-x   a leaf
```

Paths, not globally unique names, are what let two `pod` projects under
different parents coexist ([§DF-nested-workspaces](../decisions/functional/DF-nested-workspaces.md#df-nested-workspaces-a-nested-project-is-named-by-its-whole-alias-path)) — hence the per-level
uniqueness rule in §3. The cost is that a short leaf name no longer resolves on
its own, and `grund check` answers that mistake by naming the projects the
written path could have meant ([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)).

`include_root` is read per `[workspace]` block and governs that block's own
project. Under the default an intermediate node is a project like any other: it
derives an alias (§3), is scanned under its own config minus its members'
subtrees (§6), and is citable at its own path. Under `include_root = false` it
is pure grouping — no project, but still a segment in every path below it, so
`<§>hardware/sprayer/<ID>` is unaffected either way. What it costs is that the
node's own files are then scanned by **nobody**: the enclosing scan stops at the
member boundary (§6) and no other scan covers that directory, so a declaration
there is in no catalog and a citation there is never checked — not even under
`--full` ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full)), which
widens a project's scope and has no project to widen here. `grund check` stays
green over content nothing reads, which is precisely why the default is a project
([§DF-nested-workspaces.3.5](../decisions/functional/DF-nested-workspaces.md#35-the-intermediate-node-reuses-include_root-and-defaults-to-a-project)).
Every block must put at
least one project in scope, so `include_root = false` with no members is a
config error at that block's `members` line — or at its `[workspace]` line when
there is no `members` key to point at, since a tree may hold many blocks and the
error has to say which one is empty.

Members are compared as canonical paths. Two entries in *one* `members` list that
resolve to the same root are the same member — deduped, not rejected, so a glob
may legitimately name a directory an explicit entry also names (`members =
["packages/*", "packages/api"]`); what §2 rejects there is one expanded root
*containing* another. An entry that resolves to a project root **another** block
already holds is a config error at the `members` line that introduced it. What
makes expansion terminate is containment: a member root resolves strictly inside
the block that lists it and no member of one block contains another, so every step
goes strictly downward into a finite tree and no two roots in it can be equal —
which is also what makes that duplicate error a backstop rather than a rule a
config can trip ([AR-workspace.6.1](../architecture/AR-workspace.md#61-nested-workspaces-are-one-recursion-not-a-second-namespace-model)). So is one that escapes
its own block (§2): no lexical ancestor lists such a root, so nothing gives it a
stable alias path, and a root *above* its own block scans nothing at all — every
scan root lies under its own member boundary, so the project's declarations
vanish and its dangling citations pass.

Scope follows §5 unchanged: discovery stops at the *nearest* config
([§FS-config.1](FS-config.md#1-file-location-and-discovery)), so a command invoked at an intermediate node runs that
node's subtree. **Alias paths do not change with scope** for every scope *in* the
**claimed chain** — the `[workspace]` blocks from the outermost root down, each
listing the directory below it among its `members` — so a run started at any of
those blocks resolves a *subset* of the same paths rather than a re-spelled set of
its own. The guarantee is quantified over the **scope**, not over the project: a
block the chain never lists is not one of those scopes even when the projects
*under* it are reached by the chain, which a multi-segment entry (`grp/inner`)
makes routine, and a run started at such a block names every path from itself. A
citation can therefore pass that block's check and fail the run CI does — the
limitation the third rule below records, not a property of a chained scope.
Inside `hardware/`, `<§>hardware/sprayer/<ID>` still names what it names at the
repository root and `<§>final/<ID>` is simply unknown — unknown *here*, which is
what the narrowed run's diagnostic says instead of proposing a project it does
hold ([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)). The alternative — naming a
subtree's projects from the subtree — would make that disagreement the rule at
*every* scope rather than the recorded exception at one: a citation passing a
subtree check and failing the run CI does, which is
[§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing in the one place it has to hold.

Three rules keep one chain readable from every scope. **A path is read from the outermost block that claims a directory:** a multi-segment `members` entry (`grp/inner`) hops a directory that may itself declare `[workspace]` and list the same child, and the outer claim is the one the walk down from the outermost root follows — ordinary nesting has one claim per directory, where the two agree.
**A block that claims a directory and cannot answer** — a missing member, overlapping roots, an invalid alias for the project below it, or a config that does not load at all — fails the run with *its own error*, from its own `members` or `project_name` line — rendered against the root **this run** was launched at, so a block above that root renders with `..` (`../grund.toml:16`) and the reader lands on the file that holds the line rather than on a same-named one inside the subtree ([§FS-errors.4](FS-errors.md#4-determinism)); dropping its segment would let the subtree invent a namespace, and [§FS-check.3.8](FS-check.md#38-cross-project-citation-failure) would then hint the one spelling that fails at the root. The obligation is a *claim's* and a **workspace run's**, and only theirs: climbing happens because a run has an alias path to read, so a run at a project that declares no `[workspace]` block of its own — a leaf member, or any single-project repository — never asks the chain at all. Such a run resolves one project (§5): it has no alias path to get wrong, every qualified citation is an unknown alias whatever an ancestor lists, and no claim above it, answered or not, can fail it. The claim rule is about the scopes that *do* read a path, which are the blocks. A block that does not name this directory is not asked either, so neither a `members` list it could not expand nor a config that would not load at all is an error in a run below it. Otherwise one broken config anywhere above a repository — at any depth up to `/`, in a workspace that never mentions it — would answer every command inside it. The claim is therefore read from the **`members` entries alone**, never from a loaded config: the `members` value is parsed on its own — no other key read, no shape rule applied — so a config that fails to load is still asked whether it claims this directory. Deciding it from a loaded config instead made *every* load failure above a repository silently equal to "claims nothing", which is the collapsed prefix this rule exists to prevent, and two mistakes on one `members` line then behaved oppositely: a member that does not exist failed the subtree run, while an entry the shape rule rejects ([§2](#2-workspace-configuration)) let it re-spell itself. A config whose `members` text cannot be obtained at all — the file cannot be read, or its `members` value is not a list — leaves the claim undecidable in both directions. The run continues, because a stray unreadable `grund.toml` above a repository is not that repository's problem, but it never continues silently: it prints a run-level `warning:` naming that config and saying alias paths below it may be missing a segment ([§FS-errors.2.2](FS-errors.md#22-cli-level-message)).
**A `[workspace]` block that no enclosing block lists is outside the chain:** at the outer scope it is ignored, so its tree belongs to the enclosing project's namespace when that project's scan reaches it and to nobody when it does not, while a run started **at** it names every path from itself — a run started at a block *below* it that the chain does list is back inside the guarantee. `grund check` does not report such a block — finding one needs a walk for config files no pass performs — so it is a known limitation rather than a diagnosed error.

## 7. Neighboring repos

Neighboring repositories in the same organization should use the same external
syntax, for example `<§>payments/FS-refunds`, but external repository resolution
is not part of this first implementation. It requires an explicit cache or
lockfile so ordinary `grund check` remains offline, deterministic, and fast
([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)). Until that cache layer exists, aliases are
workspace-local.

## 8. Other commands

The workspace surface composes through the same resolver `grund check` uses
([AR-workspace.4](../architecture/AR-workspace.md#4-the-resolver-one-function)), so qualified-ID behavior in query commands is a UX layer over
an already-built engine — not new resolution logic. Four shared rules apply to
every command in this section:

- **Discovery follows the same walk-up rule as `grund check`** ([§FS-config.1](FS-config.md#1-file-location-and-discovery),
  §5): from the CWD (or from an explicit `<path>` argument), walk up to the
  nearest `grund.toml` in either discovery form. If the nearest config is a member's own config,
  the command runs member-local — qualified `<alias>/<ID>` cannot resolve, the
  same way `check` errors at the member scope (§5). If the nearest config is
  the workspace root, the command runs workspace-wide. An explicit `<path>`
  argument (e.g. `grund list apps/api`, `grund refs FS-x apps/api`,
  `grund complete ids --path apps/api`) behaves as if the command were invoked
  from that path: a `<path>` inside a member is member-scoped, not
  workspace-aggregate, even when a workspace exists above it.
- **The "current project" is what an unqualified ID resolves against** (§4):
  the root project at the workspace root, the member project inside a member
  tree. When `include_root = false` and the command is invoked at the workspace
  root, there is no current project; commands that accept a single ID reject an
  unqualified `<ID>` and require `<alias>/<ID>` instead. Cross-project lookups
  always require the `<alias>/<ID>` form. There is no `--all-projects` flag; the
  alias *is* the scope handle.
- **`include_root = false`** (§2): when the root project is excluded from the
  workspace, it has no catalog entry and its alias is not known. `<§>root/<ID>`
  (or whatever name `project_name` would have assigned) is treated as any other
  unknown alias by every command in this section — the root alias is not
  silently reserved. Output paths still render from the workspace root, not from
  the first member. Completions, `show`, `refs`, `cover`, and `list --project`
  all agree on this.
- **A malformed alias path is rejected before the scan, and the diagnostic names
  the offending segment** — `Sprayer` for `grund hardware/Sprayer/FS-x`, not the
  whole `hardware/Sprayer`. The path is one slug per level (§1), so the mistake
  is always in a segment; naming the path against a pattern that forbids `/`
  would read as "a namespace may not contain `/`", which is the opposite of the
  rule. An empty segment (`hardware//FS-x`) has nothing to quote, so the
  diagnostic says a segment is empty; a leading `/` (`/FS-x`) says the path is
  empty — never empty backticks. An alias path that is *well formed* but names no
  project is the unknown-alias error of §8.1, not this one.

### 8.1 `grund <alias>/<ID>`

`grund <alias>/<ID>` reads a declaration in another workspace project. The
body is rendered exactly as `grund <ID>` renders it for a local declaration
([§FS-show.2](FS-show.md#2-behavior)) — same slice rules (`--brief`, default, `--toc`, `--full`), same
`text` vs `md` heading behavior, same section selection, same inline-code
extraction ([§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments)). The alias prefix is a routing instruction; it
changes which tree is scanned, not what is printed.

- `grund api/FS-login` — print the lead of `api`'s `FS-login`.
- `grund api/FS-login.3.1 --toc` — print the `3.1` section's lead plus its
  nested heading map, against `api`'s declaration.
- `grund FS-login` invoked from inside `apps/api/` — print the member's
  local `FS-login`. Unchanged from today; no workspace context is needed
  because the citation is local.

Under `--format json` the reported `path` is relative to the config root the
command resolved against, matching `grund list` ([§FS-config.3.6](FS-config.md#36-output--report-format)) — so
`grund api/FS-login --format json` from the workspace root reports
`apps/api/…`, not the member-relative `…`. Consumers that join this path
against the directory they invoked `grund` in, such as the `grund-open`
resolver ([§FS-integrations.3.1](FS-integrations.md#31-terminal-clients-wezterm-kitty-tmux-iterm2)), depend on that base.

Outside a workspace context — including a `<path>` argument that resolves
member-local — `grund api/FS-login` exits `2` with two stderr lines:

```text
error: unknown project alias `api`
note: workspace aliases are defined in the root grund.toml under [workspace]
```

Ambiguity within a project is unchanged ([§FS-show.2.2.1](FS-show.md#221-ambiguous-id)). An ID that exists in
two *different* projects is not ambiguous — they are two declarations in two
namespaces, and the alias picks one.

### 8.2 `grund refs`

`grund refs <alias>/<ID>` lists every citation of that qualified declaration,
across the **whole workspace**, including:

- citations written `<§><alias>/<ID>` anywhere in the workspace (the
  cross-project form), and
- citations written `<§><ID>` **inside the named project's tree** (the
  project's own local form for the same declaration).

The two forms cite the same target (§4), so they appear in the same `refs`
result. This is what makes `refs` a blast-radius answer: an author about to
delete `api`'s `FS-login` learns about both `api`'s own files and every other
project that wrote `<§>api/FS-login`.

- `grund refs api/FS-login` — listed exactly as [§FS-refs.3.1](FS-refs.md#31---format-text-default) specifies; the
  `text` column shows each citation verbatim — `<§>api/FS-login` from sibling
  projects, `<§>FS-login` from inside `api`.
- `grund refs FS-login` invoked at the workspace root — citations of the
  *root's* `FS-login` only (root is the current project). To get cross-project
  occurrences, qualify: `grund refs root/FS-login` is the same query in this
  context and is the canonical form for scripts.
- `--summary` ([§FS-refs.3.3](FS-refs.md#33---summary)) aggregates per file regardless of which member the
  file lives in. Paths render relative to the workspace root when
  `[output] relative_paths = true`.
- `--format json` adds one new field on the per-citation object:
  `"project": "<alias>"` — the alias of the project that *contains* the
  citation site, not the target. The target project is the query argument
  itself: `<alias>` for a qualified lookup, the current project for an
  unqualified lookup. It is not repeated per row; per-row redundancy would
  balloon the wire size of a wide blast-radius scan without adding information
  the caller did not just hand to `refs`. The object's `"id"` field is rendered
  with that target project's `[id]` config; the `"text"` field remains the
  verbatim source citation.

The "neither declared nor cited" stderr note ([§FS-refs.2](FS-refs.md#2-behaviour)) becomes alias-aware:

```text
note: api/FS-login is neither declared nor cited — run `grund list --project api` to see api's declared IDs
```

### 8.3 `grund list`

`grund list` invoked at a workspace root prints the catalog of every project
the workspace covers (root plus members, subject to `include_root` per §8
intro), so the resulting catalog has one row per declaration with no collisions
even when two projects declare the same local ID.

Output changes:

- The ID column renders as `<alias>/<ID>` for every row **whenever workspace
  mode is loaded**, including the current project's and including the narrowed
  catalog under `--project`. Always-qualifying keeps the column self-labeled —
  `FS-login` next to `api/FS-login` in the same dump would read as a third
  project named `(local)` — and keeps script output dependent on what the
  command returned, not on how the user invoked it. The only way to get
  unqualified rows is to invoke `list` member-locally (no workspace context —
  including via a member-local `<path>` argument).
- A new optional filter, `--project <alias>[,<alias>...]`, narrows the catalog
  to one or more named projects. `--project api` returns only `api`'s
  declarations, still rendered with the `api/` prefix per the rule above. An
  unknown alias is a CLI-level error, exit `2`, same shape as `--kind`
  ([§FS-list.4](FS-list.md#4-exit-codes)).
- `--kind` composes with `--project` (intersection):
  `--kind FS --project api,payments` lists FS declarations in those two
  projects.

`grund list --summary` ([§FS-list.3.3](FS-list.md#33---summary)) gains a new variant when a workspace is
loaded: rows are emitted per `(project, kind)` pair, sorted by alias then by
configured kind order. `--project <alias>` narrows the summary to that
project's kinds.

`--format json` adds `"project": "<alias>"` to every object and renders `id` in
the qualified form (`api/FS-login`). The `refs` count is the count under that
project's qualified target, computed exactly as §8.2 defines.

### 8.4 Shell completions

The dynamic helper `grund complete ids` ([§FS-completions.2](FS-completions.md#2-internal-dynamic-helper)) gains workspace
awareness.

Completion grammar (the prefix the user has typed so far):

- No `/` in the prefix → bare-ID candidates from the current project (existing
  behavior, unchanged). When the helper is invoked at the workspace root,
  **also** emit one candidate per known alias path, with a trailing `/` —
  `api/`, `payments/`, `group/alpha/`, and `root/` only when
  `include_root = true` (§8 intro). The trailing slash is the continuation
  signal — see "Shell script adjustments" below.
- Prefix contains a `/` → split on the **last** `/`, the same boundary a
  citation uses (§1). The left side names a project; emit its IDs whose
  qualified form matches the prefix, **and** every known alias path that
  continues past the prefix, again with a trailing `/`. So `group/` offers
  `group/FS-x` alongside `group/alpha/` and `group/beta/`, and one more Tab
  after `group/alpha/` reaches that project's IDs. The prefix itself is never
  re-offered as a candidate: it would stall the shell rather than advance it.
- `--sections` and the implicit section mode (prefix containing the configured
  `[id] section_separator`) compose with the alias prefix in the natural way:
  `api/FS-login.` triggers section candidates against `api`'s declaration of
  `FS-login`.

Helper errors stay quiet ([§FS-completions.2](FS-completions.md#2-internal-dynamic-helper)): a workspace that fails to load
drops back to single-project mode so a user typing in the shell never sees a
stack trace.

**Shell script adjustments.** Alias-as-completion only feels right if a Tab
from `api` advances to `api/` *without inserting a space* — otherwise the user
types `api`-Tab-Backspace-`/` instead of `api`-Tab-Tab. The generated scripts
therefore *do* change for this case (bare-ID completion remains byte-identical
for repos with no workspace):

- `bash` — the completion function calls `compopt -o nospace` for the current
  invocation whenever any candidate in the batch ends in `/`.
- `zsh` — the completion function partitions the candidate batch:
  slash-suffixed candidates are added with `compadd -S ''` (no auto-suffix),
  bare-ID candidates with the default suffix.
- `fish` — candidates are emitted via `complete -f -k`; fish appends no space
  when the candidate's last character is `/`.

These adjustments are testable per shell and must be covered before
`<alias>/`-completion is advertised. Until each shell's no-space behavior is
verified by a fixture, the helper may fall back to emitting full
`<alias>/<ID>` candidates in one shot (one Tab completes both the alias and an
ID), at the cost of needing to type a disambiguating letter first.

### 8.5 `grund fmt --cross-refs`

The link wrapper ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)) becomes workspace-aware when invoked at the
workspace root.

- A qualified citation `<§>api/FS-login` in `docs/index.md` wraps to
  `[<§>api/FS-login](../apps/api/docs/functional-spec/FS-login.md#fs-login-...)` —
  the relative path crosses the workspace into the member's home, the anchor
  is computed from the *member's* declaration heading under the member's
  configured anchor profile ([§FS-fmt.6.7](FS-fmt.md#67-configurability)). The cross-project resolution goes
  through `target_findings_for_citation` ([AR-workspace.4](../architecture/AR-workspace.md#4-the-resolver-one-function)) so the wrapped link
  and `check`'s resolution can never disagree.
- A member-local run (invoked inside `apps/api/`, or via a `<path>` that
  resolves member-local — see §8 intro) **leaves qualified citations
  untouched**: per §5, the member run cannot resolve qualified targets and
  `--cross-refs` does not paper over a citation that `check` would error on
  ([§FS-fmt.6.4](FS-fmt.md#64-what-is-never-wrapped)). A previously-emitted wrapper around a qualified citation —
  e.g. `[<§>root/FS-x](../../docs/FS-x.md#...)` left by an earlier
  workspace-root pass — is **preserved as written**: not stripped (which would
  destroy information the member-local run cannot recompute), not re-derived
  (the resolver has no canonical URL to compare against in this context). It
  will be re-derived the next time `fmt --cross-refs` runs at the workspace
  root. The single canonical way to create or refresh cross-project wrappers
  is the workspace-root run.
- Re-derive ([§FS-fmt.6.3](FS-fmt.md#63-idempotency-and-re-derive)): a heading rename or a file move in `api` triggers a
  one-line `fmt` diff in any *other* project that wrapped a citation of the
  renamed thing, exactly as it triggers a diff in `api`'s own files. The
  workspace-root run is what makes that single pass possible.

`[fmt.cross_refs]` config ([§FS-fmt.6.7](FS-fmt.md#67-configurability)) is read from each member's own config
when wrapping a citation that targets that member — the member's
`anchor_format` wins for its own declarations. Mixed-profile workspaces (rare)
render each project's anchors under its own configured profile, consistent
with the per-member config rule ([AR-workspace.5](../architecture/AR-workspace.md#5-the-config-one-parse-one-validation-pass)).

### 8.6 `grund cover`

`grund cover` invoked at a workspace root indexes **every project the workspace
covers** — root plus members, subject to `include_root` per the §8 intro — with
one entry per scanned file, exactly as §8.3 defines the catalog for `list`. A
member-local invocation (or a `<path>` that resolves member-local) indexes that
member alone, unchanged from a standalone run.

`cover`'s question is "which IDs does this file lean on?" ([§FS-cover.5](FS-cover.md#5-why-this-exists)), and the
answer for a file is the same fact whichever scope the run was launched at. A
per-project index would make the co-change recipe ([§RM-cochange-gate](../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)) read a
changed member file as uncovered, and a coverage index that omits whole
projects while exiting `0` is the silent skip [§REQ-no-missed-citation.1](../requirements/REQ-no-missed-citation.md#1-no-silent-skips) forbids.
Rationale and the discarded project-local alternative: [§DF-cover-workspace-scope](../decisions/functional/DF-cover-workspace-scope.md#df-cover-workspace-scope-cover-indexes-the-whole-run-and-counts-cross-project-citations).

- **Qualified citations count toward the citing file.** A `<§><alias>/<ID>`
  written in `docs/index.md` is one of that file's citations, listed at its
  `(line, column)` like any other. It is what the file leans on; dropping it
  reports a fully grounded file as citing nothing. This holds outside a
  workspace too — a qualified citation in a standalone project is still a
  citation the file carries, and it is `check`'s job, not `cover`'s, to call
  the alias unknown (§8.1).
- **The rendered `id` says what the token says**, canonically: `api/FS-login`
  for a citation written qualified, the bare ID for a local one, rendered under
  the **target** project's `[id]` config exactly as `refs` renders it (§8.2).
  Qualifying a local `<§>FS-login` would report something other than what the
  file wrote, and reporting what the file wrote is the whole job. The target a
  row names is therefore `id` when it carries a `/`, and `<project>/<id>`
  otherwise — one join against the field on the same object. `text` stays the
  verbatim source token either way.
- **Paths render from the workspace root** when a workspace is loaded, so a
  member's file is spelled the way `[workspace] members` spells it and the
  recipe can join it against the same base `git diff` reports
  ([§FS-config.3.6](FS-config.md#36-output--report-format)). Scan errors from any project render against that same
  root.
- **`--format json` adds `"project": "<alias>"`** to the per-file object and to
  each nested citation object whenever workspace mode is loaded — the alias of
  the project that *contains* the file, which is also the citing project. The
  nested objects keep byte parity with `refs --format json` rows
  ([§FS-cover.3.2](FS-cover.md#32---format-json)), including this field. Outside workspace mode no field is
  added and the output is byte-identical to what it was.
- **`include_root = false`** removes the root project's files from the index
  along with its catalog entry, per the §8 intro. Nothing else scans them
  (§6), which is the hole that rule already documents.

### 8.7 Output and exit codes

All six surfaces above keep the exit codes they had:

- `show` — `0` body printed, `1` ID/section not found or ambiguous, `2` CLI/
  scan error. An unknown alias is `2` (it is a CLI-shaped error, not a "found
  something else" error) and matches the standalone-mode shape in §8.1.
- `refs` — `0` always when the scan succeeds; `2` on scan/CLI error.
- `list` — `0` always when the scan succeeds; `2` on scan/CLI error (now
  including unknown `--project`).
- `cover` — `0` always when the scan succeeds; `2` on a scan error in **any**
  loaded project, since the index is then incomplete for the tree the run
  claimed ([§FS-cover.4](FS-cover.md#4-exit-codes)).
- Completion helper — quiet failures, exit `0`, unchanged from [§FS-completions.2](FS-completions.md#2-internal-dynamic-helper).
- `fmt --cross-refs` — unchanged from [§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk).

Paths in every command respect `[output] relative_paths` as `check` already
does (§5).
