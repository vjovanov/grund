# REQ-deterministic-output: same input, same bytes

The tree-reading commands — `check`, ID queries, `list`, `refs`, `cover`, `fmt`, `id` — produce byte-identical output for the same input, across runs, across supported operating systems, and across the cargo, npm, and PyPI bindings of the same release ([§GOAL-multi-language.1](../goals.md#1-identical-behavior), [§FS-non-goals.13](../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). Agents cache and diff `grund` output and CI compares it; a report that wobbles is a report nobody can build on ([§GOAL-friendliness-first.1](../goals.md#1-hard-requirements)).

## 1. What determinism requires

Fixed report ordering ([§FS-errors.4](../functional-spec/FS-errors.md#4-determinism)), stable JSON shapes, no timestamps, process IDs or hostnames, and no dependence on directory walk order or thread scheduling — however the work is divided, the report must read as though the tree had been walked in sorted order. Sorting is over the bytes of the path, not a locale collation.

## 2. "Input" is the tree, the config, and the invocation

Three inputs decide the bytes, and all three must be held fixed before two runs are compared. The **tree** includes the ignore state that selects what is walked, which reaches `.git/info/exclude` and git's global `core.excludesFile` under the default `respect_gitignore = true` ([§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked)). The **config** is the one discovery found, which walks upward and may be found above the tree ([§FS-config.1](../functional-spec/FS-config.md#1-file-location-and-discovery)). The **invocation** matters wherever paths are rendered relative to it: under `relative_paths = false` the base is the path argument or the working directory ([§FS-config.3.6](../functional-spec/FS-config.md#36-output--report-format)), so two runs compared from different directories are not the same input.

Naming these is the point. Each is a legitimate input; none may be a *hidden* one, and a difference that cannot be traced to one of the three is a bug.

## 3. Presentation is not content

Output carries no ANSI styling today ([§FS-config.3.6](../functional-spec/FS-config.md#36-output--report-format)). When colored output lands, `auto` must mean "only when the stream is a terminal", so piped and redirected bytes stay the plain, stable form that tooling compares; `always` is the caller asking for decoration and owning the consequence. Environment-driven commands are outside this requirement by construction: `grund integrations` detects from ambient variables and says so — only its explicit-`<client>` form is byte-stable across machines ([§FS-integrations.6](../functional-spec/FS-integrations.md#6-determinism-and-exit-codes)).
