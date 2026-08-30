# DF-cli-base-parent-paths: `relative_paths = false` keeps one CLI base and may climb within the loaded root

**Status:** Accepted
**Date:** 2026-08-30

## 1. Context

`relative_paths = false` makes the path argument, or the cwd when none is
given, the report base ([§FS-config.3.6](../../functional-spec/FS-config.md#36-output--report-format)).
In a workspace-wide run from a root-project subdirectory, a member may be a
sibling of that base. It cannot be named both relative to the CLI base and
without `..`; falling back to its canonical absolute path makes the report
machine-specific and breaks consumers that resolve reported paths from the
invocation directory ([§FS-integrations.3.1](../../functional-spec/FS-integrations.md#31-terminal-clients-wezterm-kitty-tmux-iterm2)).

## 2. Decision

The whole report keeps the CLI base. A target that remains inside the loaded
project or workspace may use the minimum `..` components needed to reach it
from that base ([§FS-config.3.6](../../functional-spec/FS-config.md#36-output--report-format)).
The permission is bounded by the loaded root: it does not make paths outside
that root reportable and never licenses an absolute fallback for an in-root
target.

Thus a workspace containing root `docs/FS-root.md` and member
`hw/docs/FS-nozzle.md`, invoked from root `docs/`, reports `FS-root.md` and
`../hw/docs/FS-nozzle.md`. Both names resolve by joining them to the one base
the caller already knows.

## 3. Rejected alternative: rebase the report to the workspace root

Rendering every row from the workspace root would avoid parent components, but
it would silently change what `relative_paths = false` means. Root-project rows
would switch from `FS-root.md` to `docs/FS-root.md`, so one report would no
longer resolve from its documented path-argument/cwd base. A consumer such as
`grund-open` would either open the wrong path or need to infer an unreported
second base.

## 4. Consequences

- Default `relative_paths = true` reports remain workspace-root-relative.
- In-base `relative_paths = false` spellings remain unchanged.
- Workspace aggregation may add `..`, but only to reach a target still inside
  the loaded root; reports remain deterministic and machine-independent.
