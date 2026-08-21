# REQ-never-crashes: garbage in, diagnostic out

`grund` runs in save hooks, pre-commit hooks, and CI ([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)); a panic there blocks work that has nothing to do with `grund`, and a stack trace is the opposite of an error a reader can act on ([§GOAL-friendliness-first.1](../goals.md#1-hard-requirements)). Malformed input is the tool's daily bread, never a crash: broken UTF-8, unreadable files, pathological markdown, and huge inputs produce located errors and truthful exit codes.

## 1. Every failure is a diagnostic

An input the scanner cannot handle yields an error naming the path ([§FS-errors.2.1](../functional-spec/FS-errors.md#21-located-finding)), and a run that could not complete exits `2` ([§FS-check.2](../functional-spec/FS-check.md#2-outputs)) — the failure is reported in-band, never thrown as a stack trace.

## 2. Exit codes are the API

The mapping is frozen and not configurable ([§FS-cli.5](../functional-spec/FS-cli.md#5-exit-code-mapping-is-fixed)): `0` clean or printed, `1` findings or a failed query — a well-formed request that yielded nothing ([§FS-errors.5](../functional-spec/FS-errors.md#5-json-format)) — `2` a scan or CLI-level failure. A command may leave a code unused, never redefine one. CI trusts the exit code before it reads a byte of output, so a wrong `0` is the worst bug the tool can ship; the one place a failed run is allowed to exit `0` is the shell completion helper, where a hidden hot-path command must stay silent on a keystroke rather than spray errors into the user's prompt ([§FS-completions.2](../functional-spec/FS-completions.md#2-internal-dynamic-helper)).
