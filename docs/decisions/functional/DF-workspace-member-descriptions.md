# DF-workspace-member-descriptions: member-side `project_description` for workspace member lists

**Status:** Accepted
**Date:** 2026-06-12
**Authors:** Claude Fable 5, Vojin Jovanovic

## 1. Context

The generated "Workspace members" section ([§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members)) maps each
workspace alias to its `AGENTS.md`, but carries no semantic hint about what
each namespace is for — an agent picking an alias for a cross-project citation
must open each member's entrypoint or guess from path names
([issue #36](https://github.com/vjovanov/grund/issues/36)). The list is
high-traffic grounding context, so a one-line description per member makes
alias selection cheaper and less error-prone, the same argument as
[§DISC-token-cheap-grounding](../../discussions/proposals/2026-05-12-token-cheap-grounding.md#disc-token-cheap-grounding-token-cheap-grounding-surfaces). The design space was explored in
[§DISC-workspace-member-descriptions](../../discussions/proposals/2026-06-12-workspace-member-descriptions.md#disc-workspace-member-descriptions-describe-workspace-members-in-generated-lists); this decision pins where the description
lives and how it renders.

Three shapes were considered for where the text comes from:

1. **Structured member entries at the root** —
   `members = [{ path = "apps/api", description = "…" }]`.
2. **An alias-keyed root table** — `[workspace.descriptions] api = "…"`.
3. **A member-side key** — an optional `project_description` next to
   `project_name` in each project's own `.agents/grund.toml`, with the root
   config describing the root row.

A fourth source — deriving the line from the member's `GRUND` lead — was
considered as a fallback rather than a competing home for the configured text.

## 2. Decision

Adopt the **member-side key**: an optional single-line top-level
`project_description`, a sibling of `project_name`, read from each project's
own config.

```toml
project_name = "gradle"
project_description = "Gradle plugin that builds native images from JVM projects"
```

The [§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members) renderer appends the description after the bullet's
link as `: <description>`, before any `*(not yet initialized)*` marker; a
project without a description keeps the link-only bullet. Adopting
descriptions also tightened the bullet itself: the alias is now the link's
*label* — `` - [`api`](apps/api/AGENTS.md): Payment API service `` — so the
destination path appears once instead of twice, sharing the Project Map's
`- [x](y): …` grammar and superseding the `alias → path` bullet shape that
[§DISC-init-workspace-members](../../discussions/proposals/2026-05-17-init-workspace-members.md#disc-init-workspace-members-have-init-mention-workspace-members) originally proposed. The
generated `.agents/grund.toml` teaches the key with a commented line, and
`grund init --description <text>` sets it at bootstrap time with the same
pending-config self-exception as `--name`. The contract lives in
[§FS-config.3](../../functional-spec/FS-config.md#3-schema), [§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases), and [§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members).

## 3. Why this shape

### 3.1 The description is a member fact, like the alias

[§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration) already commits to "if a member has its own
`.agents/grund.toml`, that file configures the member", and the alias is the
member's own `project_name` ([§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases)). A description names what the
member is for; it belongs next to the name it describes, moves with the
member on rename, and never requires editing the root to describe a sibling.

### 3.2 The root alternatives fight existing semantics

Structured member entries kill the `packages/*` glob ergonomics of
[§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration) and force a second parse shape for `members`. An alias-keyed
table breaks silently when a member renames its `project_name`, invites
dangling keys that need new diagnostics, and duplicates member facts at the
root. Both were rejected.

### 3.3 Derivation is a fallback, not the mechanism

The `GRUND` lead is multi-sentence motivation prose written for a different
audience; deriving a one-liner from it needs truncation heuristics and gives
authors no control over the line agents actually read. It stays open as a
possible later fallback (configured string wins), not as the v1 mechanism.

### 3.4 Presentation metadata only

The key never participates in alias derivation, citation resolution, or
`check` semantics. Omitting it changes nothing but the rendered bullet, so
the zero-config path ([§GOAL-zero-config](../../goals.md#goal-zero-config-works-on-any-conformant-tree)) is untouched.

## 4. Scope

In scope: the config key and its single-line validation, the workspace-member
bullet rendering, the generated-config teaching line, and the
`init --description` flag. Out of scope for this slice: deriving descriptions
from member metadata, surfacing the description in `grund list` or other
query commands, and any length lint beyond the single-line rule.
