# REQ-backwards-compatibility: an upgrade never changes a verdict quietly

Upgrading `grund` must not turn a passing repository into a failing one behind the maintainer's back. The guarantee is about *silence*, not about stasis (§GOAL-no-silent-breakage): a verdict may move, but only where the release says it will and the finding says what to do.

## 1. What is covered

Everything user-visible: the CLI surface, the exit-code mapping (§FS-cli.5), the JSON schemas, the config schema and its version gate (§FS-config.5), the citation grammar, and the managed agent-entrypoint block.

Two guarantees of different strength live inside that list. The **verdict** — whether a tree passes — moves only by §2 or §3. The **bytes** are narrower: the text of an existing finding is stable phrasing that tools grep on and changes only through §2 (§FS-errors.3). Adding a new finding necessarily changes a run's bytes, since any warning stands in place of the `success` marker (§FS-check.2.1); that is governed as a verdict change, not forbidden as a byte change.

## 2. The deprecation path

The default path for anything in §1: release `N` ships the new form beside the old, with a warning naming the release in which the old form stops working, and the old form dies no earlier than `N+1`. Bare `grund` keeping its historical `check .` behavior through a named window is the worked example (§FS-cli.1).

## 3. Loud, mechanical migrations

A verdict may flip in the release that introduces the change when all three hold: the finding **names the versions** it moved between, the fix is **one documented command** the tool ships, and the release notes it. The managed block is the standing case — an older block is reported outdated until `grund init` re-renders it (§FS-init.2.3.6), which is a migration the repository can complete without reading a changelog.

This is a narrow licence, and it carries one obligation back: a change to a byte-compared block section must move the block version (§FS-init.2.3.5), because a mismatch that names no version tells the reader a file is wrong without telling them what changed.

## 4. What was never a promise

Two things sit outside the guarantee, and both must be argued in a decision record rather than assumed. A construct that had **no defined meaning** and produced no output was not a working feature, so giving it one is not a break (§DF-number-only-citation-shorthand). And before `1.0`, a surface may change without an alias where carrying the alias would cost more than the rename (§DF-show-default-token-cheap.4) — the pre-release licence, which expires at `1.0` and is not a general escape.
