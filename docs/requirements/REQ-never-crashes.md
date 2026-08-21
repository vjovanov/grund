# REQ-never-crashes: garbage in, diagnostic out

`grund` runs in save hooks, pre-commit hooks, and CI; a panic there blocks work that has nothing to do with `grund`. Malformed input is the tool's daily bread, never a crash: broken UTF-8, unreadable files, pathological markdown, and huge inputs produce located errors and truthful exit codes.

## 1. Every failure is a diagnostic

An input the scanner cannot handle yields an error naming the path (§FS-errors.2.1), and a run that could not complete exits `2` (§FS-check.2) — the failure is reported in-band, never thrown as a stack trace.

## 2. Exit codes are the API

The mapping is frozen and not configurable (§FS-cli.5): `0` clean or printed, `1` findings or a failed query — a well-formed request that yielded nothing (§FS-errors.5) — `2` a scan or CLI-level failure. A command may leave a code unused, never redefine one. CI trusts the exit code before it reads a byte of output, so a wrong `0` is the worst bug the tool can ship; the one place a failed run is allowed to exit `0` is the shell completion helper, where a hidden hot-path command must stay silent on a keystroke rather than spray errors into the user's prompt (§FS-completions.2).
