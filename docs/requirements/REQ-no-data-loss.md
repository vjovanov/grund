# REQ-no-data-loss: grund never eats user content

A wrong verdict can be re-run; destroyed content cannot. The read commands never write, and the write commands touch only what they own — a `grund` invocation must never be the reason a repository lost work.

## 1. Read commands never write

`check`, ID queries, `refs`, `list`, and `cover` open files read-only. No cache files, no lock files, no "fixed it for you" rewrites — their only output is the streams (§FS-errors.1).

## 2. Write commands touch only what they own

`fmt --write` rewrites exactly the tokens it names — `$$` triggers and citation forms (§FS-fmt) — and leaves every other byte of the file intact. `init` owns the managed entrypoint block and the files it scaffolds; content outside the block is preserved on every re-run (§FS-init.3).
