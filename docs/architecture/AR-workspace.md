# AR-workspace: how the resolver, config loader, and scanner compose across projects

The workspace surface ([§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos), [§FS-workspace](../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace)) adds **one
extra dimension** to the existing single-project pipeline: a citation may now
carry a namespace. Every other moving part — the scanner, the config loader,
the checker — must keep its single-project contract intact and let the new
dimension flow through unchanged. This document fixes the layering so that the
next command to gain qualified-ID behaviour (`show`, `refs`, `list`,
completions) composes with it, instead of re-implementing it.

The bug cluster that motivated this doc was three slips of the same kind:
two scanner modes that disagreed on what `path/ID` means; alias validation
gated on which section happened to be present in a config file; a
silent-skip behaviour at member scope that contradicted [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6.
All three came from the same shape — *workspace-only branches sitting
alongside the single-project path* — and all three are ruled out by the
invariants below.

## 1. Layering

The pipeline is four layers, top to bottom:

```text
┌────────────────────────────────────────────────────────────────┐
│ CLI (check_cmd, refs, show, list, …)                           │
│   - decides workspace vs single-project run                    │
│   - assembles the project map and current alias                │
├────────────────────────────────────────────────────────────────┤
│ Resolver (target_findings_for_citation)                        │
│   - one function that maps a Citation to its target Findings   │
│   - the only place that knows what "qualified" means at runtime│
├────────────────────────────────────────────────────────────────┤
│ Checker (check_with_workspace)                                 │
│   - rules from §FS-check / §AR-checker, namespace-agnostic     │
│   - calls the resolver; does not branch on "is workspace?"    │
├────────────────────────────────────────────────────────────────┤
│ Scanner (scan_file, scan_tree) — §AR-scanner                   │
│   - one citation regex; emits Citation { namespace, … }        │
│   - one tree walk; obeys workspace_boundary_roots              │
└────────────────────────────────────────────────────────────────┘
```

No layer reads a layer above it. The scanner never asks "am I in a
workspace?"; the checker never asks "what alias am I?"; the CLI never reaches
into a regex.

## 2. Single citation grammar

There is exactly one citation regex in the engine, defined in `grammar.rs`:

```text
\b(?:(?P<namespace>[a-z][a-z0-9-]*(?:/[a-z][a-z0-9-]*)*)/)?<ID>(?:<sep><section>)?
```

The optional `<namespace>` capture is part of the grammar ([§FS-workspace.1](../functional-spec/FS-workspace.md#1-citation-syntax)),
not a second parser pass. A scanner that toggles between "qualified" and
"unqualified" regexes invites the bug we hit on the first slice: two modes that
look equivalent on the happy path, but disagree on the edges (a bare
`path/<ID>` token; a literal slash inside a string; a markdown link
destination). One regex, one capture group, one decision rule downstream.

Nesting widened the capture to a `/`-joined run of alias segments (§6.1) and
changed nothing else: still one capture holding one string, so no consumer
learned a second shape. It stays unambiguous because an ID never contains `/`.

In a workspace run, the alias still controls the ID grammar: a qualified
citation's `ID[.section]` tail is parsed with the target project's config, not
the citing project's config. The scanner may first recognize the marker and
alias with the citing project's config, but once all member configs are loaded,
workspace-aware paths normalize known qualified citations against the target
grammar so `check`, `refs`, `list`, and `fmt --cross-refs` see the
target-shaped `Id`.

## 3. The scanner: marker-anchored qualification

### 3.1 The rule

A qualified citation requires the citation marker `§`. The scanner emits
`Citation { namespace: Some(_) }` only when the regex captured a `<namespace>`
group *and* the byte immediately preceding the match is the marker.

|                              | marker present (`§…`) | marker absent      |
|------------------------------|-----------------------|--------------------|
| `<§>alias/<ID>` / `alias/<ID>`| qualified citation    | text (skip)        |
| `<§><ID>` / `<ID>`            | unqualified citation  | bare ID in non-strict mode only; otherwise skip ([§AR-scanner.2.3](AR-scanner.md#23-citation-detection)) |

Throughout the specs, `<§>` — angle-bracketed like `<ID>` — is schematic for the marker itself: it shows a citation's *shape* without being a live citation, because the literal characters `<§>` do not end with the marker and so never match. Use it (or drop the marker entirely) to write an illustrative citation in prose that `grund check` must not resolve.

This is the rule that rules out the v1 regression where, under
`[reference] strict = false`, the bare token `path/<ID>` in prose was promoted
to a qualified citation and produced a spurious `unknown project alias`
error. The marker is the only signal the scanner uses for "the writer meant
this as a citation."

### 3.2 The scanner never branches on workspace

The scanner does not know whether it is running for a single-project repo or
a workspace member. It produces a uniform stream of `Citation` records, and
the workspace machinery lives one layer up.

The only workspace-shaped knob the scanner reads is
`workspace_boundary_roots` — a list of canonical paths the tree walk must
*not* descend into ([§AR-workspace.6](AR-workspace.md#6-the-workspace-boundary)). It exists because a root-project scan
must not absorb member declarations; it is consulted as a directory filter
during the walk, never as a per-citation rule.

## 4. The resolver: one function

`target_findings_for_citation(cite, local, workspace) -> Option<&Findings>` is
the single function any command calls to map a citation to the project it
resolves against:

- `cite.namespace == None` → resolves against `local` (the current project).
- `cite.namespace == Some(name)` → resolves against `workspace[name]`, or
  `None` if the alias is unknown.

Every consumer of citations — the checker (dangling, missing-section,
ungrounded), and any future qualified `show` / `refs` / `list` — must go
through this function. The bug shape it rules out is a command that learns
qualified IDs but resolves them slightly differently from `check`, leaving the
editor jump and the CI verdict out of sync.

A `None` return value at the resolver is never a silent skip; the calling
rule turns it into a located diagnostic (`unknown project alias <name>` at
the citation site).

## 5. The config: one parse, one validation pass

### 5.1 One loader, one parser

There is one `parse_config_file(read_path, report_path, config)` and one
`load_config_at(root) -> Config` helper. Every entry point — upward discovery
from `cli_base` ([§FS-config.1](../functional-spec/FS-config.md#1-file-location-and-discovery)) and direct loading at a known workspace member
root ([§FS-workspace.2](../functional-spec/FS-workspace.md#2-workspace-configuration)) — funnels through them. There is no separate "load a
member config" path that duplicates parsing logic; the only difference
between the two entry points is whether they walk upward first.

### 5.2 Validation runs once, at the right layer

Every post-parse invariant runs on every config load, never gated on
"did this section appear in the same file." The bug this rules out is the v1
slip where the `project_name` slug check fired only when `[workspace]` was
also present in the same file, so a member's `project_name` slipped through
load and only blew up in a different command.

For an invariant to live at the loader, the constraint must be universal — it
must hold for *every* shape of config the loader can return:

- The `[workspace] members` list shape (no native absolute paths, no
  Windows-style drive/UNC/backslash forms, no `..`, no multi-segment globs) is
  universal: a member entry never has a valid alternative reading. Checked at
  load, with diagnostics anchored at the `members` line.
- Expanded member roots must not overlap. If `apps/api` and `apps/api/sub` are
  both members, scanning the parent can absorb the child namespace unless every
  member scan also carries a bespoke boundary table. Rejecting overlap keeps
  workspace expansion simple, makes namespace boundaries explicit, and reports
  at the `members` line that introduced the conflicting roots.
- `project_name` is *not* universal. [§FS-config.3](../functional-spec/FS-config.md#3-schema) makes it free-form metadata
  when the project is standalone, and only an alias when it participates in a
  workspace. The slug check therefore lives in `derive_alias`
  (§5.3) — the one place that already needs to know "this is being used as
  an alias." Putting it earlier would force `grund init`'s `--name`, which
  predates workspaces, to write only sluggable names.

The principle: an invariant runs *once*, at the layer where its precondition
is universally true — never duplicated across layers, never gated on a sibling
section's presence.

### 5.3 Alias derivation has one canonical source

The workspace alias for a project is, in order:

1. The project's own `project_name` (validated as a slug at config load).
2. For a member, the basename of the member directory (also validated as a slug
   before use).
3. For the workspace root with no `project_name`, the literal `root`
   ([§FS-workspace.3](../functional-spec/FS-workspace.md#3-aliases), [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.3).

This ordering is implemented in *one* place. Commands ask for "the alias of
this `ProjectScan`" rather than re-deriving it from a config + path pair on
their own. Alias errors still obey [§GOAL-friendliness-first.1](../goals.md#1-hard-requirements): a bad explicit
`project_name` points at that key, while a bad basename fallback or duplicate
member alias points at the workspace `members` line that introduced the member.

## 6. The workspace boundary

When `grund check` runs at a workspace root, the **root scan must not enter
member namespaces**. That means both ordinary descent into a member directory
and a root `[scan] include` that names a path inside a member are skipped.
Otherwise the root absorbs the member's declarations into its own namespace,
breaks alias uniqueness, and silently re-creates the cross-project dependency
model that [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) rejected (§3.2).

The mechanism is a list of canonical member roots set on the root `Config`
before the walk; the scanner skips any scan root at or below one of those
boundaries, and its `WalkBuilder` filter prunes entries whose relative path
matches a boundary. Boundary roots are computed once per workspace run, not per
directory entry, so the per-entry cost is one path comparison and no
`canonicalize` syscall.

Members are scanned recursively as independent projects. A member that declares
its own `[workspace]` block contributes its whole subtree instead of one
project (§6.1).

### 6.1 Nested workspaces are one recursion, not a second namespace model

A member config carrying its own `[workspace]` block is expanded recursively:
its members load under that member's config root, and every project the walk
reaches — at any depth — becomes one entry in the same `alias → project` map
the single-level case builds. The only difference is the key: a project's alias
is its whole path, one segment per level ([§FS-workspace.6.1](../functional-spec/FS-workspace.md#61-nested-workspaces)).

That key is what keeps nesting out of every layer below the expansion step. The
namespace stays **one string**, so the resolver (§4) still does a map lookup,
the grammar (§2) still has one optional capture, and alias derivation (§5.3)
still yields one segment that the walk composes. Had it become a *list*, the
resolver, the citation regex, the completion candidates, the `--project` filter,
and the `refs`/`list` JSON keys would each have grown a second shape
([§DF-nested-workspaces](../decisions/functional/DF-nested-workspaces.md#df-nested-workspaces-a-nested-project-is-named-by-its-whole-alias-path)). An ID never contains `/`, so the path/ID split is the
last separator — one rule, applied identically by the scanner, the CLI argument
parser, and the `[citations]` rule parser.

The path is absolute with respect to the **outermost** workspace, not the one the command started in, so a narrowed run resolves a subset of the same names instead
of a re-spelled set of its own. `enclosing_alias_prefix` recovers it by climbing the ancestors that both declare `[workspace]` and list the directory below them, and
takes the **outermost** claim: a multi-segment `members` entry may hop a block that lists the same child, and the top-down walk composes the path through the outer one.
The climb is fallible — a claiming block that cannot expand its members or name the project below it raises that block's own error, since dropping it would silently
re-spell the subtree, while an ancestor whose config does not even load is no claim at all and is climbed past.

That chain of mutual claims is also the boundary of the guarantee: a `[workspace]` block no enclosing block lists is outside it — absorbed into the enclosing namespace at the outer scope, a root of its own from the inside, and diagnosed by no pass ([§FS-workspace.6.1](../functional-spec/FS-workspace.md#61-nested-workspaces)).

Expansion is bounded by the canonical project roots already collected: each is
recorded as it is added, and a member resolving to one already present is a
config error at the `members` line that introduced it — never a silent skip,
never an unbounded walk. Depth needs no separate limit because each recursion
step consumes a distinct root.

Three invariants are per-block, because every `[workspace]` block is a workspace
root in its own right (§5.1 — one loader, whatever the depth):
`workspace_boundary_roots` (§6) comes from that block's own member list, so each
scan stops at its own members; `include_root` decides that block's own project
only; and alias uniqueness is checked within one sibling set, since paths under
different parents cannot collide.

## 7. Standalone members fail loud, not silent

A `grund check` invoked at a member root cannot resolve qualified citations
to the workspace or to siblings — there is no project map. Per
[§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6 and [§FS-workspace.5](../functional-spec/FS-workspace.md#5-command-scope), every such unresolved
qualified citation is an `unknown project alias <name>` error at the
citation site.

This is the architecturally honest default: the resolver returns `None` for
the unknown alias, and the caller — a single rule, in one place — turns
`None` into a diagnostic. The opt-in to downgrade these to warnings
(`[reference] cross_project_when_standalone = "warn"`) is deferred follow-up
([§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6); when it lands, it changes one branch in
the checker, not the scanner, not the loader, not the resolver shape.

## 8. Downstream commands compose, not duplicate

Query commands (`show`, `refs`, `list`, completions) and the formatter
(`fmt --cross-refs`) consume the qualified-citation shape through a single
shared loader, `load_workspace_context`
([§FS-workspace.8](../functional-spec/FS-workspace.md#8-other-commands)).
That loader funnels through `resolve_workspace_config` so workspace
discovery and member-scope rewriting stay in one place (§5.1), and it
exposes:

1. The list of projects in scope (root + members in workspace mode; a
   single project member-local or standalone).
2. The "current" project for unqualified IDs (root at the workspace
   root; `None` when `include_root = false`, so unqualified queries are
   forced to qualify or fail loud).
3. `project_by_alias` for routing a qualified `<§>alias/<ID>` to the
   right config + findings; `aliases()` for completion candidates.

Each command then applies its own filter — `grund refs FS-x` invoked at
the workspace root scopes to the current (root) project; `grund list
--project api` narrows the catalog; `grund fmt --cross-refs` from a
member tree preserves any pre-existing qualified wraps as-is and emits
no new ones ([§FS-workspace.8.5](../functional-spec/FS-workspace.md#85-grund-fmt---cross-refs)). No command re-implements the resolver,
the citation regex, or the alias derivation. `grund cover` deliberately
stays project-local — its answer is "which files in this project carry
citations?" — and filters at the consumer end on `cite.namespace.is_none()`,
never by switching the scanner into a different mode.

## 9. Test contracts

The architecture is observable. Each invariant above has a fixture or unit
test that fails if the invariant is broken:

| Invariant                                        | Test or fixture |
|--------------------------------------------------|---|
| Single regex, marker-anchored                    | `marked_qualified_citation_is_recognised_unmarked_one_is_text` (`crates/grund-core/src/tests.rs`); `e2e/cases/non-strict-bare-slash-not-citation` |
| Resolver returns `None` ⇒ diagnostic, never skip | `e2e/cases/workspace-unknown-alias`; `e2e/cases/workspace-standalone-cross-project` |
| Alias check fires at use, both for `project_name` and the basename fallback | `e2e/cases/workspace-invalid-auto-alias`; `e2e/cases/workspace-duplicate-auto-alias` |
| Missing section on a qualified citation reports at the citation site | `e2e/cases/workspace-cross-project-missing-section` |
| Nested projects are named by their whole alias path | `e2e/cases/workspace-nested-members` |
| The intermediate node is a project, scanned under its own config | `e2e/cases/workspace-nested-group-is-scanned` |
| `include_root = false` drops the node but keeps its segment | `e2e/cases/workspace-nested-group-include-root-false` |
| Same alias under two parents is not a duplicate  | `e2e/cases/workspace-nested-same-alias-different-parents` |
| Duplicate alias within one sibling set rejected  | `e2e/cases/workspace-nested-duplicate-alias` |
| Nested block with nothing in scope rejected      | `e2e/cases/workspace-nested-empty-scope` |
| A short leaf name names the project it could have meant | `e2e/cases/workspace-nested-short-alias-hint` |
| A narrowed run resolves a subset, not a re-spelling | `e2e/cases/workspace-nested-subtree-scope` |
| Nested member cycle rejected, expansion terminates | `nested_workspace_member_cycle_is_rejected` (`crates/grund-core/src/tests_workspace.rs`) |
| Boundary skips member from root scan in `check`  | `workspace_boundary_root_is_not_scanned_as_parent_content` / `workspace_boundary_nested_scan_root_is_not_scanned_as_parent_content` (`crates/grund-core/src/tests.rs`); `e2e/cases/workspace-cross-project-valid` |
| Boundary skips member from root scan in non-`check` commands | `e2e/cases/workspace-list-respects-boundary` |
| Member without its own config falls back to canonical defaults | `e2e/cases/workspace-member-without-config` |
| Glob expansion                                   | `e2e/cases/workspace-glob-members` |
| Cross-member citations resolve in both directions | `e2e/cases/workspace-cross-member-citations` |
| Same local ID in two members is not a duplicate  | `e2e/cases/workspace-same-id-different-projects` |
| Single-project repo flags stray `<§>alias/<ID>`  | `e2e/cases/cross-project-citation-without-workspace` |
| `config show` round-trips `[workspace]`          | `e2e/cases/config-show-workspace-roundtrip` |
| `check --format json` shape in a workspace       | `e2e/cases/workspace-check-json` |
| `cover` / `list` skip qualified citations        | `e2e/cases/cover-ignore-qualified-project-local`; `e2e/cases/list-ignore-qualified-project-local`; `e2e/cases/refs-ignore-qualified-project-local`; `e2e/cases/fmt-cross-refs-ignore-qualified-project-local` |
| `[workspace] members` shape rejected at load     | `e2e/cases/workspace-member-absolute-path`; `e2e/cases/workspace-member-parent-segment`; `e2e/cases/workspace-member-windows-drive`; `e2e/cases/workspace-member-windows-path`; `e2e/cases/workspace-member-multi-glob` |
| Overlapping workspace member roots rejected      | `e2e/cases/workspace-member-overlap` |
| `[[workspace]]` array-table form rejected        | `e2e/cases/workspace-section-as-array-table` |
| Unknown key under `[workspace]` rejected         | `e2e/cases/workspace-unknown-key` |
| Member missing on disk fails workspace expansion | `e2e/cases/workspace-member-missing-on-disk` |

These are the contracts a future change must keep green; if a change cannot
keep one of them green, the change is breaking the layering — not the test.
