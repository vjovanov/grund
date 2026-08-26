# DF-unwalked-kind-home: a kind may be a place that is listed but not walked

**Status:** Accepted
**Date:** 2026-08-26

## 1. Context

Configuring this repository's own tree as non-citable kinds ([§FS-config.3.4.1](../../functional-spec/FS-config.md#341-citable--kinds-that-declare-no-ids)) worked for `skills/`, `examples/`, `.github/workflows/` and `scripts/`, and failed on `templates/`. That directory holds the reference copies of what `grund init` writes ([§FS-init.2.1](../../functional-spec/FS-init.md#21-files-written-updated-or-left-in-place)); the binary embeds byte-identical copies, and an integration test keeps the two sets equal. Made a `folder` home under `[reference] require_grounding = true`, all ten files were reported `ungrounded file in kind home templates/` — correctly, by the home rule of [§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) — and none of them can be grounded: a `§` citation in a template is copied into every scaffolded repository as a dangling reference to a declaration that repository does not have.

Naming the directory in `[scan] exclude` changed nothing, by design: a home is a walk root, and [§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked) settled that a config saying both "this directory matters" and "skip it" is read as the first. The workaround that fit the existing keys — a single-file home on a `templates/README.md` explaining the directory — put the README in the Project map where the directory belongs, and asked the sync test to overlook a file that is not a template.

## 2. Decision

### 2.1 `scan = false` on a `[[kinds]]` row

A non-citable kind may set `scan = false` ([§FS-config.3.4.7](../../functional-spec/FS-config.md#347-scan--a-place-that-is-listed-not-walked)): it keeps its home, its title and its Project map row, and its home is not a walk root. The row is the whole point — an agent is told the place exists and what it is for — and nothing in it is read.

### 2.2 Only on a non-citable kind with a home

A citable kind is always walked: `scan = false` on one would make its declarations invisible rather than declared, the trap [§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked) exists to close, so it is a config error. The homeless kind has no home to not walk — what of the complement is read is `[scan] include`'s job — so it is an error there too.

### 2.3 No citation rules on it

A `[citations.<kind>]` table naming an unwalked kind as the citing kind is a config error. [§DF-non-citable-kinds.2.5](DF-non-citable-kinds.md#25-obligations-get-a-per-file-unit-and-grounding-follows-the-home) refused a rule that passes vacuously because it has no unit; a rule on a kind none of whose files is scanned is the same emptiness one level up, and accepting it would let a config carry an instruction the checker never enforces. Without rules there is no directions bullet, which is what a reader should see: the block says where the place is and says nothing about what its files cite, because nothing is asked of them.

### 2.4 `--full` reaches it as territory nobody configured

`grund check --full` walks the whole config root ([§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full)) and reports resolution failures only, so a dangling citation inside an unwalked home is still found there — the run reads past `include` and past this key alike. What it does not do is judge the directory against conventions it never adopted: no grounding, no directions, no misplaced-declaration finding. That is the existing `--full` contract applied unchanged; the key only decides which scope the directory is in.

## 3. Alternatives considered

**A per-kind `require_grounding = false`.** Solves the templates case and nothing else: the files are still read, a `§` illustration in one still resolves or dangles, and a `[citations]` rule still applies. The problem was never that the templates are ungrounded; it is that nothing in them is this repository's to check.

**Let `[scan] exclude` prune a home it names.** [§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked) decided the opposite, for the honest reason that `exclude` is written about descendants. Reversing it would also flip a verdict silently for every repository whose home shares a name with a defensive exclude — `build`, `dist` — which [§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly) forbids.

**A separate `[[places]]` table.** The `[[areas]]` argument of [§DF-non-citable-kinds.2.2](DF-non-citable-kinds.md#22-rejected-a-second-areas-table) again: a second table for a kind that differs by one boolean.

**Leave it unconfigured and mention the directory in the hand-written part of `AGENTS.md`.** Loses the generated row and its title — the one thing the config can keep current — and teaches agents that the map is incomplete.

## 4. Consequences

This repository's `template` kind is `folder = "templates"`, `scan = false`; the README workaround is not needed and the sync test compares the two directories whole. The key is additive ([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)). The managed-block version does not move: the block's content changes only for a config that uses the key, and only by omitting a bullet that config gives it no reason to render.
