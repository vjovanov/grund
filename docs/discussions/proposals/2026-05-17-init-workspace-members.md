# DISC-init-workspace-members: Have `init` mention workspace members

## Status

Concluded. Ready to draft as additions to [§FS-init](../../functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo).

## Context

`grund init` is workspace-blind today. It writes a single `AGENTS.md` (plus
companion entrypoints) and a single `.agents/grund.toml` for the directory it is
invoked in, and the managed block's Project Map
([§FS-init.2.3.4.4](../../functional-spec/FS-init.md#2344-project-map))
lists only the declaration homes from that one project's effective config.

That is fine for a single-project repo and matches [§FS-init](../../functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo)'s commitment to "no
prompts, no surprises" rooted in
[§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible). It is
weaker for a workspace, though: at a root whose `.agents/grund.toml` declares
`[workspace] members = [...]`
([§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration)),
nothing in the generated `AGENTS.md` tells a reading agent that sibling
project namespaces exist or where to climb to read their rules. An agent
arriving at the root therefore cannot tell — without scanning the toml itself —
that `<§>api/FS-foo` style cross-project citations are even possible, or which
aliases are in scope.

Making `init` *configure* the workspace is the heavier alternative discussed
upstream of this note and was set aside: it requires either prompts or
inference about which directories are members, both of which violate the
no-prompts rule. This proposal is the lighter version — *mention*, do not
*configure*.

## Proposed shape

When the effective `.agents/grund.toml` at the init target has a `[workspace]`
section, append a small "Workspace members" list to the managed block, derived
purely from the existing config:

```markdown
### Workspace members

- `api` → `apps/api/AGENTS.md`
- `core` → `packages/core/AGENTS.md`
- `ui`   → `packages/ui/AGENTS.md`
```

The alias on the left is the member's `project_name` (or the directory's
canonical alias per [§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases));
the path on the right is the member root joined with `AGENTS.md`. Globs from
`members = ["packages/*"]` are expanded the same way `grund check` already
expands them ([§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration)),
so the list reflects the actual filesystem at init time.

The list is emitted from existing configuration only — `init` still does not
prompt, does not infer workspace topology, and does not write to member
directories. Bootstrapping a member's own `AGENTS.md` remains a separate
`grund init` invocation inside that member.

## Boundaries

- Do not write or modify any file under a member directory. This stays a
  root-only edit, scoped to the same managed block [§FS-init](../../functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) already maintains.
- Do not add a `[workspace]` block to a config that does not already have one.
  The section appears only when the config already declares members.
- Do not change the Project Map format
  ([§FS-init.2.3.4.4](../../functional-spec/FS-init.md#2344-project-map)) —
  the members list is a sibling section, not an extension of the existing list,
  so the "declaration homes" semantics stay clean.
- Do not surface `include_root = false` as anything other than the absence of
  the root alias from the members list; the flag's effect on `check` scope is
  not something an agent reads inline.

## Open questions

- Should members whose `AGENTS.md` does not yet exist be listed anyway (more
  honest, surfaces the missing entrypoint) or filtered out (avoids pointing at
  broken paths)? Listing them matches how the Project Map already lists homes
  without checking they are populated.
- Should the section also restate the cross-project citation syntax
  (`§alias/<ID>`), or rely on the reader to follow the link into the member's
  own `AGENTS.md`? The token-cheap-grounding direction
  ([§DISC-token-cheap-grounding](2026-05-12-token-cheap-grounding.md#disc-token-cheap-grounding-token-cheap-grounding-surfaces))
  argues for the briefest possible inline content.
- For very large workspaces (dozens of members), should the list collapse to
  the alias-only form (`- api`, `- core`, …) with a single pointer to "see
  each member's `AGENTS.md`", or stay fully linked?
- Does the member-side `init` need any reciprocal change — e.g. a one-line
  "this project is a member of the workspace at `../..`" pointer — or is the
  one-way root → members link enough?

## Opinions

Overall, this is a strong, low-risk proposal. Adding a "Workspace members" list to the root `AGENTS.md` solves the discoverability problem for AI agents in a multi-project repository while strictly adhering to the "no prompts, no surprises" goal.

Regarding the open questions:

- **Should members without an `AGENTS.md` be listed?** Yes. Listing them accurately reflects the `grund.toml` state and correctly surfaces missing entrypoints, which might prompt the agent to initialize them.
- **Should the syntax be restated?** No. Adhere to token-cheap-grounding. Providing the alias-to-path mapping is sufficient; agents should already know or learn the citation syntax independently.
- **Should large workspaces collapse the list?** No. Keep it fully linked. The mapping of namespaces to paths is the primary value. Stripping paths defeats the purpose, and 50 lines of Markdown is cheap in modern context windows.
- **Does member-side `init` need a reciprocal change?** Yes. A one-line pointer to the workspace root (e.g., `Part of the workspace defined at ../../AGENTS.md`) is highly valuable for agents invoked directly inside a member project.

### Codex opinion

I support this proposal. It fits the shape of `init`: make the agent's starting
context useful, but do not infer or configure workspace topology. Reading
`[workspace] members` from existing config and surfacing resolved aliases is a
low-risk improvement that preserves the no-prompts, no-surprises contract in
[§FS-init](../../functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) while making workspace citation scope visible from the root entrypoint.

My preferred answers to the open questions are:

- List members even when their `AGENTS.md` does not exist. The workspace member
  exists by config; the missing entrypoint is useful information.
- Include one terse syntax line: `Cross-project citations use §alias/<ID>.` The
  point is discoverability, and that sentence earns its tokens.
- Keep full `alias` to `path/AGENTS.md` links for v1. Do not add collapse
  behavior until there is evidence of real bloat.
- Do not add reciprocal member-side output yet. Root-to-member discovery is the
  missing part; member-to-root can be a separate decision.

I would also tighten the proposal into spec language around deterministic
ordering: expand members exactly as workspace resolution does, then emit stable
Markdown sorted by alias or resolved path. That keeps generated blocks
reviewable and avoids churn.

---

A second take: support the proposal, with three refinements on top of the
open-question answers.

- **Mark uninitialized members, don't silently link them.** Listing absent
  members is right, but render them as
  `` `api` → `apps/api/` *(not yet initialized)* `` rather than pointing at
  a file that 404s. Listing is honest; a dead link wastes the next agent
  turn.
- **Sort by alias, not by resolved path.** Path-sort looks arbitrary to a
  reader because depth and prefix dominate. Alias-sort matches how
  citations appear in code (`§api/...`, `§core/...`) and gives the reviewer
  a predictable diff.
- **Be explicit about member-side `init`.** When `grund init` runs inside
  `apps/api`, the effective config still inherits `[workspace]`, so by the
  proposal's trigger the member's own `AGENTS.md` also gets the
  sibling-members list. That is the correct behavior, but it should be
  stated outright in the spec — readers will otherwise intuit that members
  get a different block ("you are a member of X") instead of the same
  list.

On the open questions where the takes diverge:

- **Restate the citation syntax inline.** One line —
  `Cross-project citations use §alias/<ID>.` — earns its tokens. An agent
  landing at the root cold should not have to follow a link to discover
  the namespace syntax exists. The token-cheap-grounding direction is
  about not inlining content that lives elsewhere; the syntax sentence is
  not duplicated content, it is the discoverability hook for the list
  immediately above it.
- **Defer reciprocal member-side pointer to a follow-up.** Root → members
  is the missing discovery direction; member → root is already discoverable
  by climbing `..` and is a separate decision worth its own scope.

## Conclusion

The proposal is accepted. Adopt it as drafted, with the following
resolutions of the open questions and refinements on top.

**Open questions, resolved**

- *List members without `AGENTS.md`?* Yes, with a trailing
  `*(not yet initialized)*` marker; render the right-hand side as the
  member root (e.g. `apps/api/`) rather than a non-existent file path,
  and append `/AGENTS.md` only when the file exists.
- *Restate the citation syntax inline?* Yes. Emit one line:
  `Cross-project citations use §alias/<ID>.`
- *Collapse the list for large workspaces?* No. Keep full
  `alias → path` links in v1. Revisit only if real bloat appears.
- *Reciprocal member-side pointer?* Deferred to a follow-up discussion.
  v1 is root → members only.

**Refinements layered on the proposed shape**

- Members are sorted by alias, not by resolved path, for stable diffs.
- The same "Workspace members" list is emitted when `grund init` runs
  inside a member directory whose effective config inherits `[workspace]`.
  The spec should state this explicitly so the symmetry is not a surprise.

Next step: fold these into [§FS-init](../../functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) alongside the existing managed-block
contract, and update the Project Map section
([§FS-init.2.3.4.4](../../functional-spec/FS-init.md#2344-project-map))
to reference the new sibling section.
