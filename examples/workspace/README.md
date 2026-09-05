# Workspace: cross-project citation in a monorepo

A tiny three-project workspace: the root `root` declares `FS-platform`, and
two members `api` and `web` each declare their own `FS-session`. Both members
cite back to the root, and the root cites into each member. Every citation
that crosses a project boundary writes the target alias before the ID.

```toml
# repo/grund.toml
project_name = "root"

[workspace]
members      = ["packages/*"]
include_root = true
```

```markdown
# repo/docs/functional-spec/FS-platform.md
The root project pulls together its sub-projects: the API in §api/FS-session
and the web app in §web/FS-session.
```

```markdown
# repo/packages/api/docs/functional-spec/FS-session.md
The platform-level coordinator is §root/FS-platform.
```

## What this teaches

- **Workspace = one citation universe per member.** Both `packages/api` and
  `packages/web` declare `FS-session` independently; neither is a duplicate.
  Local citations stay short — inside `api`, `<§>FS-session` resolves to the
  api project; inside `web`, the same form resolves to the web project.
  (Every citation outside the fenced blocks above is written `<§>…`: the
  fixture this page describes lives under `[scan] exclude`, so these IDs
  resolve nowhere from *this* repository and a live marker would be a
  dangling citation under `grund check --full`.)
- **Cross-project citations are explicit.** Every reference that crosses a
  project boundary names the target's alias: `<§>api/FS-session`,
  `<§>root/FS-platform`. There is no `<§>../FS-platform` or implicit
  parent-of-parent lookup.
- **`grund check` orchestrates the whole tree.** At the workspace root, one
  `grund check examples/workspace/repo` validates the root and every member,
  with each member as its own namespace.

## Run it

```bash
grund examples/workspace/repo
echo $?    # 0
```

Then drop a dangling cross-project citation into any file (e.g. change
`<§>api/FS-session` to `<§>api/FS-missing`) and re-run. `grund` reports an
`unknown reference api/FS-missing` at the citation site, with the same
`path:line:` shape it uses for single-project checks.

The neighboring-repo form `<§>payments/FS-refunds` resolving to another
repository is **not** implemented yet — see [[§FS-workspace.7](../../docs/functional-spec/FS-workspace.md#7-neighboring-repos)]
(../../docs/functional-spec/FS-workspace.md#7-neighboring-repos).
