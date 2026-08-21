# REQ-grund: the repository practices what it ships

`grund`'s own repository is the first grund-conformant tree: if the scheme is not worth keeping here, it is not worth shipping. This holds the tree to [§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree) — this repo is the canonical layout the tool assumes — in service of [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work).

## 1. Canonical layout

Every kind lives in its configured home: GRUND, GOAL, and RM as single files (`docs/grund.md`, `docs/goals.md`, `docs/roadmap.md`), FS, AR, DF, DA, and DISC as folders under `docs/`, and E2E cases under `e2e/cases`. Scale changes layout depth, never the citation contract (§GOAL-small-and-large).

## 2. Every citing surface is scanned

`[scan] include` (§FS-config.3.5) names every surface that carries citations: `docs`, `e2e`, `src`, `crates`, plus the root files `README.md` (§REQ-readme.3) and `AGENTS.md` (§REQ-agents-md.4). A citation in an unscanned file neither resolves nor dangles, so leaving a citing surface out of the set is the silent failure mode; `grund check --full` (§FS-check.1.3) is the periodic net for strays.

## 3. Grounded source

`[reference] require_grounding = true`: every scanned source file either cites a spec point or declares one inline (§FS-check.3.6). New code starts grounded; there is no grandfather list.

## 4. Ship discipline

Behavior and design changes update the most-specific spec point before code. Every pull request carries a `docs/changelog.md` `## Unreleased` bullet naming it (§FS-distribution.4).
