# REQ-backwards-compatibility: a passing tree keeps passing

Upgrading `grund` must never silently change a verdict: a repository that passed `grund check` before the upgrade passes after it, byte-for-byte, unless the release names the change and its deprecation window (§GOAL-no-silent-breakage).

## 1. What is covered

Everything user-visible: the CLI surface, output bytes on both streams, the JSON schemas, the config schema and its version gate (§FS-config.5), the citation grammar, and the managed agent-entrypoint block (§GOAL-no-silent-breakage.1).

## 2. The only path for change

Release N ships the new form beside the old, with a warning that names the release in which the old form stops working; the old form dies no earlier than N+1 (§GOAL-no-silent-breakage.2). A silent semantic change — same input, different verdict, no warning — is a release blocker, not a bug fix.
