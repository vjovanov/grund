# DF-subproject-namespaces: alias-namespace model for sub-projects and external repos

**Status:** Accepted
**Date:** 2026-05-16
**Authors:** GPT-5.5, Claude Opus 4.7, Vojin Jovanovic

## 1. Context

`grund` was designed around a single project rooted at one `.agents/grund.toml` ([§FS-config.1](../../functional-spec/FS-config.md#1-file-location-and-discovery)). Real adopters now want two adjacent capabilities:

- **Sub-projects in one tree.** Monorepos that hold several independent grund projects under one repository root, where each sub-project should keep its own short, local citation form but the whole tree should still check cleanly in CI.
- **Cross-repo citation.** Neighboring repositories in the same organization should be referenceable by stable name, not by URL or filesystem path, with the same correctness guarantees as local citations.

Before this decision, the design space had four viable shapes:

1. **One global namespace** with project prefixes baked into every ID (`api-FS-login`).
2. **Kind-prefix renaming** that overloads the existing ID grammar (`§PM-FS-refunds`).
3. **Path-based citations** that point at the file system (`§../GOAL-x`, `§./packages/api/FS-x`).
4. **An alias-namespace model** with explicit qualified citations (`<§>api/FS-login`).

This decision picks among them and pins the first user-visible surface: the citation grammar and workspace-local alias resolution. External repository resolution, lockfiles, `grund sync`, and qualified query commands are intentionally downstream; they need their own specs before they become user-visible behavior.

The discussion was a three-way iteration: GPT-5.5 produced the initial proposal and the four-syntax exploration, Claude Opus 4.7 reviewed the recommendation and proposed the final form (drop reserved `root`, ban branch-pinned externals, unify the resolver across commands, fail loud in standalone mode), and Vojin Jovanovic adjudicated and accepted.

## 2. Decision

Adopt the **alias-namespace model** with two citation forms:

```
§ID[.section]               local
<§>alias/ID[.section]         qualified
```

and three eventual project shapes — **standalone**, **workspace**, **member**. The first implementation composes them via a new optional `[workspace]` block on `.agents/grund.toml` and validates qualified citations in `grund check`.

The current contract is [§FS-workspace](../../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace). Neighboring repositories and qualified query commands remain follow-up work, not part of this accepted slice.

## 3. Why this shape

### 3.1 Local ergonomics stay byte-identical

`§FS-<login>` keeps working in every existing repo. Adopting the workspace block changes nothing about how local citations are written, scanned, or formatted. [§GOAL-zero-config](../../goals.md#goal-zero-config-works-on-any-conformant-tree) holds: a canonical single-project tree needs no config and no qualified citations to function. The whole multi-project surface is opt-in.

### 3.2 Cross-project references are always explicit

The slash separator makes "this citation crosses a project boundary" visible at the citation site. A reader who sees `<§>api/FS-login` knows immediately that `api` is a different project; the eye does not have to disambiguate from kind prefixes. The dash-form alternative `§PM-FS-refunds` considered in the discussion reads as one long ID and forces the parser to know that `PM` is a project rather than a kind. That ambiguity is solvable, but it lives inside the load-bearing surface of [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) — making it the citation grammar's problem instead of a config-load problem is the wrong tradeoff.

### 3.3 No reserved alias for the workspace project

Reserving `root` (or any other name) would shadow any real project actually named `root` and would mean one piece of the citation grammar lived outside config, against [§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable). Letting the workspace project's `project_name` drive how members cite it keeps everything in one configurable surface and costs nothing — projects that want the discussion's suggested `<§>root/…` form set `project_name = "root"` and get it.

### 3.4 External repos require a separate offline cache design

`[projects.payments] ref = "main"` would mean a passing `check` is a promise about whichever commit was HEAD at the last `sync` — silently invalidated by any upstream push. That is a [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) hazard packaged as a feature. The first implementation therefore stops at workspace-local aliases. External repos should be added only with a committed lockfile or cached declaration index that keeps `grund check` offline and reproducible.

### 3.5 Network I/O must be one explicit verb

External references, when implemented, must be validated against local state, never the network. This keeps `grund check` deterministic across machines and fast enough for [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) — per-keystroke in an IDE, per-save in a watcher, per-commit in CI. A future `grund sync`-style verb is the right place for network behavior. A single network verb is easier to audit, sandbox, and reason about than network behavior diffused across commands.

### 3.6 Standalone members fail loud, not silent

A member checked outside its workspace context cannot resolve qualified citations back to the workspace or to siblings. Two ways to handle this exist: silently skip the unresolvable references, or fail. Skipping would mean a passing `grund check` in `apps/api/` could ship a `<§>root/GOAL-x` that no longer exists upstream — a false negative on the core promise. Failing by default preserves [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) without forcing every contributor to clone the whole monorepo. The current contract is therefore "every unresolved qualified citation is an `unknown project alias` error" ([§FS-workspace.5](../../functional-spec/FS-workspace.md#5-command-scope), [§FS-check.3.8](../../functional-spec/FS-check.md#38-cross-project-citation-failure)) — never silently skipped.

A future relaxation knob — e.g. `[reference] cross_project_when_standalone = "warn"` for workflows that genuinely want the looser mode (a sub-project cloned in isolation for a quick edit) — is **deferred follow-up**. The space is *error* vs *warning* only; *skipped* is not on the table. The knob is intentionally not part of this first slice because it adds a third state to the resolver, an interaction with `--require-grounding` and `[output] format = json`, and a new failure mode for CI that we want to design once, separately, rather than retrofit.

### 3.7 Check comes first; query commands follow

Making `check` workspace-aware first is the narrowest correctness slice: it lets CI catch broken cross-project citations without changing every query command at once. The long-term direction is still one resolver across `show`, `refs`, `list`, completions, and editor integrations; otherwise editor jumps would differ from CI results. That follow-up should be specified separately so each command's ambiguity and output behavior is pinned.

## 4. Consequences

- A new functional spec, [§FS-workspace](../../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace), ships with this decision and defines the current contract.
- [§FS-config](../../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up) gains the optional `[workspace]` schema section. The addition is under the current `grund_config_version = 1`; no schema bump.
- [§FS-check](../../functional-spec/FS-check.md#fs-check-grund-validates-every-reference-in-a-repo) gains workspace aggregation and alias-qualified citation validation. The existing exit-code mapping ([§FS-check.2](../../functional-spec/FS-check.md#2-outputs)) and finding-line shape ([§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format)) are preserved.
- Qualified `show`, `refs`, `list`, completions, neighboring repositories, lockfiles, and any `grund sync`-style command remain follow-up work.
- The `[reference] cross_project_when_standalone` opt-in (§3.6) remains follow-up work; today the only configurable axis is "error". Any future relaxation must explicitly remain between *error* and *warning*.
- The reusable workspace surface is fixed in [§FS-workspace](../../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace) — one citation grammar, one config loader, one resolver, one validation pass. Subsequent commands that learn qualified IDs must compose with that layering rather than re-implement it.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| **Single global namespace** with project prefixes baked into every ID (`api-FS-login`). | Forces every existing repo to migrate; defeats [§GOAL-zero-config](../../goals.md#goal-zero-config-works-on-any-conformant-tree); makes the local citation form bear the cost of the rare cross-project case. The chosen model lets locals stay locals. |
| **Dash-form qualified prefix `§PM-FS-refunds`.** Compact; no new separator. | Reads as one long ID, not "project + ID." Requires the parser to know which prefixes are projects vs kinds; collision rules become a config-load minefield (`PM` vs `PM-FS` vs hypothetical `PMA` kind). The `/` is one character and removes the ambiguity at the grammar level. |
| **`@`-prefix form `§@api/FS-login`.** Visually marks the cross-project case. | The `§` marker already says "this is a citation" and the `/` already says "this is qualified." The `@` is a third signal for the same fact. Drops the `@`; the form is `<§>alias/ID`. |
| **Relative or path-based citations** (`§../GOAL-x`, `§./packages/api/FS-x`). No alias declaration needed. | Paths are not stable — projects move; directories get renamed; a citation written against `../api` breaks when `api` becomes `packages/api`. Aliases are stable handles; the alias table is configuration, the citation is content. |
| **Reserve `root` as the workspace alias.** Matches the discussion's suggestion that the common case write `<§>root/…`. | Hard-codes a name into the grammar; a project literally named `root` cannot then be a member; the reservation lives outside config against [§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable). Setting `project_name = "root"` recovers the same form without the reservation. |
| **Allow branch-pinned externals (`ref = "main"`).** Familiar from package managers and dependency tools. | Deferred. A `check` that passes today can stop passing tomorrow with zero local change — a false negative on [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration). Any external design needs a reproducible local index first. |
| **`check` workspace-aware, other commands project-local forever.** Cheaper to implement; the resolver only has to learn workspaces in one place. | Accepted only as the first slice. Long-term, a two-tier UX is surprising: an editor jump from `grund show <§>api/FS-login` would fail while CI's `grund check` succeeded on the same file. |
| **Silently skip unresolvable cross-project refs in standalone members.** Cheapest CI story for sub-project clones; nothing to configure. | Silent false negatives on the core promise. The chosen design errors by default and offers an explicit `cross_project_when_standalone = "warn"` opt-in — the choice is between *error* and *warning*, never *skipped*. |
| **Per-member dependency policy in v1** (`[workspace.references] api = ["root", "db"]`). | Premature. The resolver and the alias namespace are the load-bearing pieces; restricting which projects may cite which is a policy layer that does not change the citation grammar. Deferrable without affecting any committed citation. |
