# DF-skill-init-existing-specs: `grund-init` adopts existing specs before scaffolding

**Status:** Accepted
**Date:** 2026-05-20

## 1. Context

`grund init` and the `grund-init` skill have different jobs. The CLI is deliberately non-interactive: it writes or refreshes the managed agent block and creates `.agents/grund.toml` only when absent, while preserving existing entrypoints, user-authored content, and existing config ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints), [§FS-init.2.4](../../functional-spec/FS-init.md#24-generated-grundtoml), [§FS-init.3](../../functional-spec/FS-init.md#3-non-intrusive-guarantees)). The skill is the interactive adoption wrapper: it inspects the repo, recommends configuration with evidence, writes config before invoking `grund init`, and validates the result ([§FS-init.5](../../functional-spec/FS-init.md#5-agent-setup-instructions)).

The ambiguous case is a repo where the user says "set up grund" but the project already has specs, requirements, ADRs, RFCs, plans, or test scenarios. The skill should not treat that repo like a fresh tree. Existing specs are project-owned artifacts; replacing them with canonical scaffold folders would violate the non-intrusive shape of `init` and the configurable-layout goal ([§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable)).

## 2. Decision

When existing specs or spec-like artifacts are found, the skill should adopt them as the source of truth instead of creating competing scaffolds.

1. **Inventory first.** Detect existing spec homes, naming patterns, ID-like headings, citation markers, decisions, roadmaps, changelogs, e2e fixtures, and agent instruction files before recommending any command.
2. **Ask for an artifact model.** When existing specs are present, list the canonical `grund` kinds (`GRUND`, `GOAL`, `FS`, `AR`, `DF`, `DA`, `E2E`, `RM`) beside the detected project-specific sections, tags, or document classes, then ask the user which model to adopt before writing config or refactoring docs.
3. **Prefer configuration over movement.** Represent existing homes in `[[kinds]]`, `[scan].include`, `[scan].exclude`, and ID grammar settings. Do not rename files, move directories, or rewrap existing specs unless the selected artifact model calls for that refactor and the user confirms it.
4. **Default to no `--docs`.** If the repo already has meaningful spec artifacts, recommend `grund init` without `--docs` unless the user selected a canonical-layout migration. The canonical scaffold is for fresh or empty projects; existing specs should be mapped, not shadowed.
5. **Preserve existing grund config.** If `.agents/grund.toml` already exists, treat it as authoritative. The skill may propose edits, but should ask before changing it because `init` itself never overwrites existing config ([§FS-init.2.4](../../functional-spec/FS-init.md#24-generated-grundtoml)).
6. **Preserve existing agent entrypoints.** Let `grund init` append or update only the managed block in whatever known entrypoints already exist. Do not create a competing `AGENTS.md` unless the user explicitly requests the canonical entrypoint ([§FS-init.2.1](../../functional-spec/FS-init.md#21-files-written-updated-or-left-in-place)).
7. **Separate adoption from conversion.** The first setup pass should leave the tree with a valid config, refreshed agent guidance, and a validation report. Adding missing `§` citations, renaming IDs, splitting specs into canonical `FS`/`AR` homes, or adding e2e coverage should follow the selected artifact model, with a visible plan instead of being hidden inside `init`.

## 3. Artifact model prompt

For an existing spec-heavy repo, the skill should show an explicit choice before it writes `.agents/grund.toml`:

| Option | Shape | Result |
|---|---|---|
| Canonical `grund` | Use `GRUND`, `GOAL`, `FS`, `AR`, `DF`, `DA`, `E2E`, and `RM` as the complete artifact model. | The agent may add or refactor docs toward canonical homes, usually with `--docs` only when folders are missing and the user accepts the migration. |
| Canonical core plus project extras | Use `GRUND`, `GOAL`, `FS`, and `AR` as the backbone, then add project-specific kinds for existing sections, tags, or document classes such as `ADR`, `RFC`, `REQ`, `PLAN`, or `RUNBOOK`. | The agent maps existing docs into config, adds missing canonical backbone docs when useful, and keeps project-specific artifacts first-class. |
| Existing structure with citations | Keep the repo's current sections/tags/document classes as the artifact model, adding only `grund` citations, declaration headings, and config around them. | The agent minimizes churn: no canonical refactor unless requested, and `grund init` refreshes only the entrypoints. |

The prompt should include a recommendation grounded in the inventory. For example, a repo with mature ADRs and product requirements but no explicit architecture spec might recommend "canonical core plus project extras"; a repo with a clean internal taxonomy and high migration cost might recommend "existing structure with citations." Once the user selects an option, the agent uses that choice to decide whether it is merely adding config/citations or also refactoring docs into new homes.

## 4. Decision rule

Classify the target before choosing `--docs`:

| Repo shape | Skill recommendation |
|---|---|
| Fresh repo with no meaningful docs/specs | Use `grund init --docs` if the user wants the canonical scaffold. |
| Existing docs/specs but no grund config | Write `.agents/grund.toml` that maps the existing layout, then run `grund init` without `--docs`. |
| Existing `.agents/grund.toml` | Do not rewrite config by default; run `grund init` to refresh entrypoints, then validate. |
| Existing grund citations/declarations but no config | Infer the closest ID grammar and kind homes from the existing declarations, ask for confirmation, then write config before `grund init`. |
| Existing non-grund spec system | Preserve it first; optionally propose a later conversion or dual-citation plan. |

## 5. Boundaries

- The skill may ask questions; the CLI still must not. `grund init` remains flag-driven and deterministic ([§FS-init.1](../../functional-spec/FS-init.md#1-inputs)).
- The skill should not silently add canonical folders when project-specific spec folders already exist.
- The skill should not auto-convert existing documents into `grund` declarations in the setup step. Conversion can be proposed after validation, with its own plan and diffs.
- The skill should not treat a failed `grund check` after initial adoption as proof that the existing specs are wrong. Initial failures may simply mean the selected config is too broad, too strict, or missing legacy artifact homes.

## 6. Consequences

- [§FS-init.5](../../functional-spec/FS-init.md#5-agent-setup-instructions) carries the behavior contract for the printed and packaged `grund-init` instructions.
- `skills/grund-init/SKILL.md` carries the operational decision table because agents need it at setup time without reading this decision first.
- The first setup pass stays conservative: preserve the user's existing spec system, map it into `grund` config, and make the agent entrypoint teach the resulting layout. `--docs` is correct for an empty repo and usually wrong for a repo that already has specs.
