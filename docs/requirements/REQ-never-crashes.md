# REQ-never-crashes: garbage in, diagnostic out

`grund` runs in save hooks, pre-commit hooks, and CI; a panic there blocks work that has nothing to do with `grund`. Malformed input is the tool's daily bread, never a crash: broken UTF-8, unreadable files, pathological markdown, and huge inputs produce located errors and truthful exit codes.

## 1. Every failure is a diagnostic

An input the scanner cannot handle yields an error naming the path (§FS-errors.2.1), and a run that could not complete exits `2` (§FS-check.2) — the failure is reported in-band, never thrown as a stack trace.

## 2. Exit codes are the API

`0` means checked and clean, `1` means findings, `2` means the run cannot be trusted — and nothing else ever maps onto them (§FS-cli.5). CI trusts the exit code before it reads a byte of output; a wrong `0` is the worst bug the tool can ship.
