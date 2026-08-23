# DF-cover-workspace-scope: cover indexes the whole run and counts cross-project citations

**Status:** Accepted
**Date:** 2026-08-23

## 1. Context

`grund cover` answers one question — for each scanned file, which IDs does it cite ([§FS-cover.5](../../functional-spec/FS-cover.md#5-why-this-exists)) — and it answered it about a strictly smaller tree than the one the user pointed it at.

Two independent narrowings, both deliberate at the time:

- **Scope.** `cover` scanned the discovered config's own project and stopped there, while `check`, `list`, and `refs` load the root project plus every configured member ([§FS-workspace.5](../../functional-spec/FS-workspace.md#5-command-scope)). At a workspace root every member's files were absent from the index, and the members' scan errors were absent with them, so the run exited `0` on a tree it never read.
- **Cross-project citations.** Every `<§><alias>/<ID>` was dropped at the consumer end. A file whose citations were *all* qualified — the normal shape for a grouping node's docs under nested workspaces ([§FS-workspace.6.1](../../functional-spec/FS-workspace.md#61-nested-workspaces)) — printed as `(no citations)`.

Together they turn the most misleading possible answer into the default one: a coverage index that silently omits whole projects, and reports the files it does reach as ungrounded when they are not.

```console
$ grund cover                       # workspace root, members = ["hardware"]
docs/FS-root.md:
  (no citations)                    # cites §hardware/FS-nozzle, twice
$ echo $?
0                                   # hardware/ never scanned, its broken link never reported
```

The old reasoning for the second narrowing was that attributing a cross-project reference to the citing file's project would "distort the per-file local citation map". That reads the index as a per-project inventory of local IDs. It is not: it is keyed by *file*, and the fact a file leans on `hardware/FS-nozzle` belongs to the file, not to whichever namespace declares the target.

## 2. Decision

### 2.1 `cover` indexes every project the run loaded

[§FS-workspace.8.6](../../functional-spec/FS-workspace.md#86-grund-cover): at a workspace root, root plus members, subject to `include_root`; member-local when the path resolves member-local. The same scope rule, from the same loader, as `show`, `refs`, `list`, completions, and `fmt --cross-refs`.

This is not a new judgement about scope — it is `cover` stopping being the one query command that disagrees with [§FS-workspace.5](../../functional-spec/FS-workspace.md#5-command-scope). A coverage index that omits whole projects while exiting `0` is precisely the silent skip [§REQ-no-missed-citation.1](../../requirements/REQ-no-missed-citation.md#1-no-silent-skips) forbids, and it is the one command where the omission cannot be noticed from the output, because "this file has no citations" and "this file was never read" print as absence either way.

#### 2.1.1 A scope narrower than the config root stays narrow

The aggregate is what a run *at the workspace root* answers. `grund cover src/` is still one narrowed scan of the enclosing project, no workspace loaded — the line [§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full) already draws for `grund check <dir>`, reached through the same `scope_is_config_root` test.

`cover` is the only command in [§FS-workspace.8](../../functional-spec/FS-workspace.md#8-other-commands) whose `<path>` bounds a walk instead of choosing a config, so it is the only one where the two readings differ. `list apps/api/docs` aggregates because its path merely says which project to ask; `cover src/` cannot, because an explicit path bypasses `[scan] include` ([§AR-scanner.1](../../architecture/AR-scanner.md#1-tree-walk)) — the narrowing is the *only* reason those files are in scope, and widening it would both discard them and answer a question the caller did not ask. It would also make the plumbing surface for [§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test) return the whole repository for every narrowed query.

### 2.2 A qualified citation counts toward the citing file

`<§><alias>/<ID>` is listed at its `(line, column)` like any local citation, and the rendered `id` keeps the alias so a caller can hand it back to `grund <ID>` unchanged ([§FS-workspace.8.6](../../functional-spec/FS-workspace.md#86-grund-cover)).

The test is what the consumer does with the row. [§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test) asks "did this changed file's spec change too?" — for a file that cites `hardware/FS-nozzle`, the answer is a fact about `hardware`'s declaration, and the gate cannot ask it if `cover` never named the target. Dropping the row does not make the gate conservative; it makes it read the file as ungrounded, which under [§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) is the opposite verdict from the one `check` reaches on the same file in the same run.

This holds outside a workspace too. A qualified citation in a standalone project is still a citation the file carries; whether the alias names anything is [§FS-workspace.8.1](../../functional-spec/FS-workspace.md#81-grund-aliasid)'s error to raise, and `cover` reporting the graph is not `cover` blessing it.

### 2.3 The `project` field is added, and only in workspace mode

`--format json` gains `"project":"<alias>"` on the per-file object and on each nested citation object when a workspace is loaded, matching what `refs` already emits ([§FS-workspace.8.2](../../functional-spec/FS-workspace.md#82-grund-refs)).

Two consequences are load-bearing. A recipe that maps a changed path back to the project whose config governs it needs the alias, and deriving it from the path prefix would make every consumer re-implement member expansion. And gating the field on workspace mode keeps a single-project repository's bytes exactly as they were — the population that has no use for the field is also the population `cover`'s JSON contract was written for.

### 2.4 Paths and scan errors render from the workspace root

A member's file is spelled the way `[workspace] members` spells it, and a member's unreadable path is reported against that same base, not against the member ([§FS-workspace.8.6](../../functional-spec/FS-workspace.md#86-grund-cover)). A recipe joins `cover` output against the base `git diff` reports; a path rendered against the member names a file that does not exist from where the run was launched.

## 3. Rejected alternative: keep `cover` project-local and add `--all-projects`

Leave today's behavior as the default and put aggregation behind a flag.

It fails on the shape of the mistake. Nobody passes a flag to fix an answer they do not know is wrong — the whole defect is that the incomplete index is indistinguishable from a complete one, so the flag would be discovered only by someone who had already been misled. And it re-opens a question [§FS-workspace.8](../../functional-spec/FS-workspace.md#8-other-commands) closed for every other command: "there is no `--all-projects` flag; the alias *is* the scope handle". `cover` takes no ID, so it has no alias to carry — which makes the invocation path the only scope handle it has, and that is exactly what [§FS-workspace.5](../../functional-spec/FS-workspace.md#5-command-scope) already defines.

A narrowing that a caller genuinely wants is still available and already specified: point `cover` at the member.

## 4. Consequences

- Two verdict changes, both from `0` to `2`, both in workspaces: a member's unreadable file now fails the run at the root, and so does the root's when a member was the reason the root was loaded. Governed by [§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered) as a verdict move the release names — and in both cases exit `2` is the code `check`, `list`, and `refs` already return for the same tree, so the change makes two installs of `grund` agree rather than making them differ ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).
- Text output for a single-project tree with no qualified citations is byte-identical. A single-project tree that *does* carry a qualified citation gains rows it was dropping.
- JSON output for a single-project tree is byte-identical, field for field.
- [§AR-workspace.8](../../architecture/AR-workspace.md#8-downstream-commands-compose-not-duplicate) loses its `cover` carve-out: every query command now composes through `load_workspace_context`, and no consumer filters on `namespace.is_none()`.
- `list`'s reference counts keep their own filter and are untouched. `list` counts citations *of a declaration*, so a `<§>api/FS-login` belongs to `api`'s row and to no other — a different question from "what does this file lean on?", answered correctly today.
