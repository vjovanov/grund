# GRUND-grund: agents stay grounded in the spec

**Keep agents grounded in the spec — fewer bugs, cheaper LLM context, faster onboarding.** On a long-lived codebase with many humans and AIs, the spec drifts: citations rot, decisions get forgotten, e2e tests prove the wrong things, and "the why" lives in someone's head until that someone moves on. Each agent has limited context, and without a shared, mechanically-checked reference frame, knowledge silently fragments.

The **grund reference scheme** — shipped and enforced by the `grund` tool — addresses this by giving every spec, goal, decision, and test a stable ID, and forbidding agents from hoarding context outside the docs. But the scheme is only as strong as its enforcement: a dangling `§FS-<user-login>.3.1` in prose is invisible until something breaks.

Citations live wherever they are useful — including inside Java doc-comments, Rust `///` lines, Python docstrings, Go doc blocks, TypeScript JSDoc — not only inside Markdown. Off-the-shelf Markdown link checkers (`lychee`, `markdown-link-check`) cannot help with those: they walk `.md` files, validate `[text](url)`, and return. A `§FS-<events>.4` cited from `src/bus.rs` is invisible to them. That is the gap `grund` exists to close.

## 1. What grund does about it

`grund` owns the scheme end to end. It defines the IDs and citation grammar, ships the config in `grund.toml`, and scans every `.md` file and every source file in the repo ([§AR-scanner.4](architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) to keep three promises, each its own ground: [§GRUND-understanding](grund.md#grund-understanding-the-why-stays-known), [§GRUND-structure](grund.md#grund-structure-the-projects-long-term-memory-stays-organized), and [§GRUND-consistency](grund.md#grund-consistency-the-structure-stays-consistent).

This serves [§GOAL-agent-grounding](goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) — the headline goal that every other goal exists in service of — and the mechanisms that make it viable: [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration), [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible), [§GOAL-friendliness-first](goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible), and [§GOAL-polyglot-citation](goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful).

## 2. Who it is for

- **Codebases that adopt the specification-driven design** — to verify the spec stays intact across changes, *and across the docs/code boundary*.
- **Polyglot projects** whose specs are cited from source as well as docs — the case off-the-shelf link checkers cannot serve.
- **Agents (human and AI) working in those codebases** — to retrieve grounded spec content cheaply.
- **CI systems** — as a fast pre-merge check.

If a project does not use a grund-style ID scheme, `grund` has nothing to offer it. We deliberately do not generalize.

# GRUND-understanding: the why stays known

Everything in the project — code, docs, decisions, tests — cites the point that says why it is the way it is: a behavior on its doc-comment, a clause inline beside the line that enforces it. The why holds for the work as it stands, not just the change that made it — readable in place, never reconstructed from git history or someone's memory.

# GRUND-structure: the project's long-term memory stays organized

The project's long-term memory — its why, goals, behavior, design, decisions, and proofs — is organized into declarations, each a fact with one stable, location-independent ID: `§FS-<user-login>.3.1` keeps resolving when files move or headings reword — Markdown anchors break; grund citations don't. `grund FS-<user-login>.3.1` returns just that subsection — under 200 lines per [§GOAL-friendliness-first.1](goals.md#1-hard-requirements) — so a human or LLM pulls one fact into context instead of a whole file.

# GRUND-consistency: the structure stays consistent

Every cited ID is checked across prose and code alike — Javadoc, Rustdoc, Python docstrings, Go blocks, JSDoc. Dangling refs, broken section coordinates, duplicate declarations, and broken stub links all fail the build; no dangling reference ships.
