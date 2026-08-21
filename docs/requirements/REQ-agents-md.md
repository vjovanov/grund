# REQ-agents-md: the agent entrypoint stays managed and grounded

`AGENTS.md` is where an agent's session starts; if it drifts, every session starts wrong. This serves [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) — instruction is the first of its three layers — under the entrypoint contract that §FS-init ships.

## 1. One source, symlinked companions

`AGENTS.md` is the single source of agent instructions; `CLAUDE.md` is a symlink to it, the companion arrangement §FS-init.2.1 maintains. Edits land in `AGENTS.md` only — never in a companion copy.

## 2. The managed block stays current

The versioned `grund init` block is kept at the version the pinned `grund` binary writes; a missing, older, or unsupported block fails the build (§FS-check.3.5).

## 3. Rules outside the block are grounded

Every repository rule below the managed block cites the spec point that owns it — the changelog gate cites §FS-distribution.4, the layout contract is §REQ-grund — so an agent resolves the why of an instruction the same way it resolves the why of a line of code.

## 4. Scanned like everything else

`AGENTS.md` is named in `[scan] include` (§FS-config.3.5), so the citations its rules carry are checked, not decorative.
