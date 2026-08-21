# REQ-readme: the README is the grounded shop window

The root `README.md` is the first surface a human or an agent reads, and it sells a discipline — so it must practice that discipline on itself. This serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — the common path is legible before anyone opens the spec tree — and [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) — the first thing a reader learns is the loop they are expected to keep.

## 1. What it must say

The README opens with what `grund` is: the three promises, one bullet each, every bullet citing the GRUND declaration that makes it (§GRUND-understanding, §GRUND-structure, §GRUND-consistency). The walkthrough then follows the workflow in order — specify, cite, re-read, check — one numbered section per step, and ends with how to install. The README is read whole, so depth lives in `docs/` behind links, not inline (§GOAL-token-economy); it stays within the repository's entrypoint line budget.

## 2. Every example is real

Code excerpts are verbatim from this repository, with elisions marked. Command output is captured from actually running the command against this tree — no invented IDs, no invented paths, no invented output. A change that invalidates a captured excerpt or output updates the README in the same change, per the co-change contract in §FS-examples.4.

## 3. The README's citations are checked

`README.md` is named in `[scan] include` (§FS-config.3.5), so every `§`-marked citation in its prose resolves under `grund check` like any other scanned file's. Illustrations that must not resolve stay inside fenced code blocks, which the scanner ignores.
