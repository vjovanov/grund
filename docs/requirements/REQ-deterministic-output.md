# REQ-deterministic-output: same input, same bytes

Same tree plus same config produces byte-identical output — across runs, across supported operating systems, and across the cargo, npm, and PyPI bindings (§GOAL-multi-language.1). Agents cache and diff `grund` output, and CI compares it; a report that wobbles is a report nobody can build on (§GOAL-friendliness-first.1).

## 1. What determinism requires

Fixed report ordering (§FS-errors.4), stable JSON shapes, no timestamps, and no dependence on directory walk order, thread scheduling, or locale.

## 2. No environment leaks

The compared bytes never vary with anything outside the tree and the config. Where the terminal adds presentation — `color = "auto"` — piped output stays the plain, stable form; decoration is rendering, never content.
