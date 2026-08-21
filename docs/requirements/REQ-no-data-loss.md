# REQ-no-data-loss: grund never eats user content

A wrong verdict can be re-run; destroyed content cannot. `grund` writes only what it owns, only when it was asked to — a `grund` invocation must never be the reason a repository lost work. The tool that organizes a project's long-term memory (§GRUND-structure) is the last thing that should be able to destroy it.

## 1. The query surface never writes

`check`, ID queries, `refs`, `list`, `cover`, `config show` / `validate`, `completions`, `agent-setup-instructions`, and `integrations` without `--write` open files read-only. No caches, no lock files, no PID files, no "fixed it for you" rewrites — their only output is the streams (§FS-errors.1).

## 2. Writers touch only what they own

Three invocations write, each bounded by a stated ownership marker: `fmt --write` rewrites the citation tokens it names and no other byte (§FS-fmt.2.3); `init` owns the managed entrypoint block, delimited by its markers, plus the files it scaffolds (§FS-init.3); `integrations --write` owns its comment-delimited blocks in editor and terminal config, its fixed resolver path, and its own extension directory (§FS-integrations.4). The editor transform is held to the same rule in the user's buffer (§FS-lsp.1.4). Ownership is what makes a re-run safe: everything outside the marker survives, however many times the command runs.

## 3. Destructive is opt-in and never a side effect

`init --force` is the one path that replaces whole files — the canonical `AGENTS.md` and the `--docs` scaffold stubs `init` owns end to end (§FS-init.3). That is the user asking for it, and it stays bounded: the config is never overwritten, and companion entrypoints keep their unrelated content. No other flag, and no default run of any command, replaces a file whose content `grund` did not write. Where a spec describes a rule that `--force` reverses, it says so at that point — an unqualified "left unchanged" that is only true without the flag is the wording that loses somebody's `docs/goals.md`.
