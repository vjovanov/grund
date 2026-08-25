# FS-init-fixtures: concrete init fixtures

This file is the verbose fixture companion to [§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo). It does not add a new command; it gives implementers concrete states and transcripts for the `init` behavior that [§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) defines in prose.

The fixtures use `{repo}` for an existing target directory and `{repo_copy}` for a mutable copy of that directory. Every success case has empty stdout. Every path printed by `init` is relative to the target directory. The stderr blocks below are exact apart from the placeholder path in the missing-target case.

## 1. Default form

Command:

```text
grund init {repo_copy}
```

Precondition: `{repo_copy}` exists and contains no `AGENTS.md` and no config in either discovery form ([§FS-config.1](FS-config.md#1-file-location-and-discovery)).

Exit `0`, stdout empty, stderr:

```text
wrote AGENTS.md
wrote grund.toml

next:
  1. re-run with --docs to scaffold the FS home (requirements.md), docs/, and e2e/ (or create them yourself) — until then `grund check` has nothing to scan
  2. run `grund check` — a scaffolded tree is clean
  3. allocate an ID:  ID=$(grund id FS "…")  then add it to requirements.md
see AGENTS.md for the full workflow.
```

Final tree:

```text
AGENTS.md
grund.toml
```

`AGENTS.md` contains exactly one delimiter-bounded managed block — `<!-- BEGIN GRUND MANAGED BLOCK -->` through `<!-- END GRUND MANAGED BLOCK -->` ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)) — headed `## Grounding with grund (vN)` at the current schema version. `grund.toml` is the default generated config from [§FS-init.2.4](FS-init.md#24-generated-grundtoml), including `grund_config_version = 1`, `project_name`, `[reference]` (including the inline citation style keys from [§FS-inline-citation-style.2](FS-inline-citation-style.md#2-configuration)), `[id]`, every default `[[kinds]]`, `[scan]`, `[output]`, and `[fmt.cross_refs]`. Constrained keys include the inline value-set comments required by [§FS-init.2.4](FS-init.md#24-generated-grundtoml).

## 2. Docs form

Command:

```text
grund init {repo_copy} --docs
```

Precondition: `{repo_copy}` exists and contains no generated files.

Exit `0`, stdout empty, stderr:

```text
wrote AGENTS.md
wrote grund.toml
wrote requirements.md
wrote docs/grund.md
wrote docs/goals.md
wrote docs/roadmap.md
wrote docs/changelog.md
wrote docs/architecture/README.md
wrote docs/decisions/architectural/README.md
wrote docs/decisions/functional/README.md
wrote e2e/README.md
wrote e2e/cases/.gitkeep

next:
  1. run `grund check` — a freshly scaffolded tree is clean
  2. allocate an ID:  ID=$(grund id FS "…")  then add it to requirements.md
     (H2: `## <ID>: <one-line statement of the behavior>`)
  3. cite it as §<ID> from the docs and e2e tests that depend on it, then `grund check` again
see AGENTS.md for the full workflow.
```

Final tree includes the default-form files plus exactly these docs/e2e scaffold paths:

```text
requirements.md
docs/grund.md
docs/goals.md
docs/roadmap.md
docs/changelog.md
docs/architecture/README.md
docs/decisions/architectural/README.md
docs/decisions/functional/README.md
e2e/README.md
e2e/cases/.gitkeep
```

## 3. Existing files

When the managed block and config are already current, `init` is a no-op on disk:

```text
exists AGENTS.md
exists .agents/grund.toml
```

The `next:` block is suppressed in this case ([§FS-init.2.2](FS-init.md#22-stdout--stderr)) — every reported path is `exists `, so the user already has a complete grund setup and there is no next step to teach.

When `AGENTS.md` exists without a managed block and `--force` is not passed, `init` appends the block and reports:

```text
appended AGENTS.md
wrote grund.toml
```

When `AGENTS.md` exists and `--force` is passed, `init` rewrites the canonical file and reports `wrote AGENTS.md`. When a config already exists, `init --force` still preserves it and reports it with `exists ` under the name it was found at — `exists .agents/grund.toml` for the fixture below; config is never clobbered once present, in either discovery form ([§FS-config.1](FS-config.md#1-file-location-and-discovery)).

## 4. Target and flag failures

A missing target directory is a CLI-level failure: exit `2`, stdout empty, stderr:

```text
error: target directory does not exist: <path>
```

An unknown flag is a CLI-level failure: exit `2`, stdout empty, stderr:

```text
error: unknown flag `<flag>`
```

These failures leave the target tree unchanged.

## 5. Dry-run preview

`grund init --dry-run` reports exactly what a real run would do, without writing anything. Every line that a real run would prefix with `wrote `, `appended `, or `updated ` is reported with `would-write `, `would-append `, or `would-update ` instead; `exists ` lines are unchanged. The `next:` block prints under the same rule as a real run — suppressed when every reported path is `exists ` and no `would-…` lines were emitted ([§FS-init.2.2](FS-init.md#22-stdout--stderr)). Exit code is `0` for a clean preview, `2` for a CLI-level error (e.g. missing target), the same as a real run.

## 6. Workspace members

These fixtures together cover [§FS-init.2.3.4.15](FS-init.md#23415-workspace-members): a workspace root with a mix of initialized and uninitialized members, a member-side run against the same workspace, a non-workspace repo whose generated block is unchanged, and a workspace whose projects carry `project_description` metadata.

### 6.1 Workspace root init

Precondition: `{repo_copy}` exists with `.agents/grund.toml`:

```toml
project_name = "root"

[workspace]
members = ["apps/api", "packages/*"]
```

and the on-disk layout:

```text
.agents/grund.toml
apps/api/AGENTS.md
packages/core/
packages/ui/
```

`packages/core/` and `packages/ui/` are real directories with no `AGENTS.md`. `apps/api/AGENTS.md` already exists from a prior member-side `grund init`.

Command: `grund init {repo_copy}`.

The generated `{repo_copy}/AGENTS.md` contains, between `### Project map` and `### Declarations and citations`, exactly this block — byte-identical, including blank lines:

```markdown
### Workspace members

Cross-project citations use §alias/<ID>.

- [`api`](apps/api/AGENTS.md)
- [`core`](packages/core/) *(not yet initialized)*
- [`root`](AGENTS.md)
- [`ui`](packages/ui/) *(not yet initialized)*
```

The list is sorted lexicographically by alias. `api` is initialized so its bullet links to the existing `AGENTS.md`. `root` is included because `include_root` defaults to `true` (the row appears even though linking the file to itself is mildly noisy — the spec keeps the row shape uniform). `core` and `ui` are present in the workspace by glob expansion of `packages/*` but have no `AGENTS.md` yet, so they render with the trailing `*(not yet initialized)*` marker and link to the member root.

### 6.2 Workspace member init

Same precondition as §6.1, but with `apps/api/AGENTS.md` removed so the member is uninitialized.

Command: `grund init {repo_copy}/apps/api`.

The generated `{repo_copy}/apps/api/AGENTS.md` contains a `### Workspace members` section whose bullets refer to the same logical workspace as §6.1 — verifying [§FS-init.2.3.4.15](FS-init.md#23415-workspace-members)'s symmetry guarantee. Paths are rewritten relative to the AGENTS.md being written:

```markdown
### Workspace members

Cross-project citations use §alias/<ID>.

- [`api`](AGENTS.md)
- [`core`](../../packages/core/) *(not yet initialized)*
- [`root`](../../) *(not yet initialized)*
- [`ui`](../../packages/ui/) *(not yet initialized)*
```

`api`'s row is the freshly written file (the "self" exception in [§FS-init.2.3.4.15](FS-init.md#23415-workspace-members) — `api` counts as initialized in its own block even before the write completes). `root` is marked uninitialized because `{repo_copy}/AGENTS.md` does not exist, and the link points at the workspace root directory rather than the file that would 404. The aliases, the discoverability line, and the alias ordering match §6.1 exactly — what differs is the per-row "self" flag and the link paths, both of which are local-perspective renderings.

### 6.3 Non-workspace repo

Precondition: `{repo_copy}` exists and contains no `[workspace]` block in its config (or no config at all, in which case the defaults apply per [§FS-init.2.4](FS-init.md#24-generated-grundtoml)).

Command: `grund init {repo_copy}`.

The generated `AGENTS.md` contains no `### Workspace members` section anywhere. The `### Project map` block is byte-identical to the default-form fixture (§1) — surfacing workspace mode is gated on `[workspace]` being declared, so a single-project repo's block is unchanged from before [§RM-init-workspace-members](../roadmap.md#rm-init-workspace-members-init-mentions-workspace-members) landed.

### 6.4 Workspace member descriptions

Same shape as §6.1, but the root and two members carry `project_description` metadata ([§FS-config.3](FS-config.md#3-schema), [§FS-workspace.3](FS-workspace.md#3-aliases), [§DF-workspace-member-descriptions](../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists)). Precondition: `{repo_copy}` exists with `.agents/grund.toml`:

```toml
project_name = "root"
project_description = "Workspace root: shared specs and tooling"

[workspace]
members = ["apps/api", "packages/*"]
```

`apps/api/.agents/grund.toml` sets `project_name = "api"` and `project_description = "Payment API service"`, and `apps/api/AGENTS.md` exists from a prior member-side `grund init`. `packages/core/.agents/grund.toml` sets `project_name = "core"` and `project_description = "Core domain library"` but `packages/core/AGENTS.md` does not exist. `packages/ui/` is a real directory with no config and no `AGENTS.md`.

Command: `grund init {repo_copy}`.

The generated `{repo_copy}/AGENTS.md` contains exactly this block:

```markdown
### Workspace members

Cross-project citations use §alias/<ID>.

- [`api`](apps/api/AGENTS.md): Payment API service
- [`core`](packages/core/): Core domain library *(not yet initialized)*
- [`root`](AGENTS.md): Workspace root: shared specs and tooling
- [`ui`](packages/ui/) *(not yet initialized)*
```

Each described project appends `: <description>` after its link; `core` shows the description rendering *before* the trailing `*(not yet initialized)*` marker; `ui` has no config, therefore no description, and its bullet is byte-identical to the §6.1 form.
