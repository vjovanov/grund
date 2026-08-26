# DF-require-grounding: an opt-in check that every source file cites a spec

**Status:** Accepted
**Date:** 2026-05-10

## 1. Context

The reference scheme already proves that every *citation* resolves ([§FS-check.3.1](../../functional-spec/FS-check.md#31-dangling-citation)) and that every section coordinate exists ([§FS-check.3.2](../../functional-spec/FS-check.md#32-missing-section)). It does **not** prove the converse: that every piece of implementation actually *points* at the spec it realizes. A new module can land carrying no citation at all; a reviewer changing a spec runs `grund refs` on it and only sees the files that already chose to cite it.

The stronger discipline we want — "implementation cannot change without the spec it grounds in, and without the tests" — is naturally diff-aware: it compares a change against a base revision. That is a different contract from `grund check`, which is a pure function of `(tree, config)` ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)), and it leans on a git diff, which the engine deliberately does not read ([§FS-non-goals.6](../../functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)). So the full idea has to be tiered:

1. **Grounded implementation** — every source file carries at least one citation to a declared ID. Static; no git; no AST.
2. **A `grund cover` plumbing surface** — the scan exposed as data: for each file, the IDs it cites and their line ranges; for each test / `§E2E-` case, the IDs it cites. Still static.
3. **A co-change gate** — diff-aware: a changed source file must be grounded, and the diff must also touch the cited spec *or* a test of it, with an explicit, greppable escape hatch for refactors.

Tier 1 is most of the value and the only part that fits inside `grund-core` without bending a bright line. This record covers Tier 1; Tiers 2–3 are tracked under [§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) and [§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test).

## 2. Decision

### 2.1 A new opt-in error class

Add `[reference] require_grounding` ([§FS-config.3.1](../../functional-spec/FS-config.md#31-reference--citation-form)), default `false`, plus `grund check --require-grounding` to force it on for one run. When set, `check` reports an `ungrounded source file` error ([§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)) for every scanned file whose extension is not `.md` and that is not *grounded*.

### 2.2 "Grounded" is defined syntactically

A source file is grounded if **either**:

- it contains at least one recognized citation ([§FS-check.1.1](../../functional-spec/FS-check.md#11-recognized-citations) — so a bare token counts only when `strict = false`) whose ID resolves to a declaration in the tree; **or**
- it itself declares an ID inline ([§FS-show.2.3](../../functional-spec/FS-show.md#23-inline-declarations-in-code-and-doc-comments)) as a non-stub home — a class that carries its own inline spec is grounded in that spec.

A file whose only citation is dangling is not grounded; it earns both the `dangling` and the `ungrounded` finding, and fixing the citation clears both. "Source file" is decided purely by extension (not by parsing the file), so the rule adds no language awareness ([§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing)) and reads no history ([§FS-non-goals.6](../../functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)) — it stays a pure function of `(tree, config)` ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).

### 2.3 File granularity, not hunk granularity

The check is per file: one resolving citation anywhere in the file satisfies it. A finer "every doc-comment block must cite something" rule is conceivable from the same scan data, but file granularity is the cheap, sound floor and is what the diff-aware gate ([§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)) refines against — there is no need to bake the finer rule into `grund-core` first.

### 2.4 Off by default

Like `strict`, grounding is a discipline a repo opts into once it is ready (and once its source tree — including any fixture trees under the `E2E` folder — is either grounded or carved out of `[scan]`). A repo that has never adopted the marker should not start failing `check` on upgrade.

## 3. Consequences

- `Config` gains a `require_grounding: bool`; `check` gains the [§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) loop over the scanner's file list (a new `Findings.scanned_files`); `grund config show` prints the key; `grund check --help` lists the flag; `templates/grund.toml` carries `require_grounding = false` so the generated config still documents every key ([§FS-init.2.4](../../functional-spec/FS-init.md#24-generated-grundtoml)).
- No `grund_config_version` bump: a v1 config without the key keeps working, and a v1 config that sets it is only understood by a `grund` new enough to have this record — an additive change, like `[fmt.cross_refs]`.
- The reverse-lookup story tightens: in a `require_grounding` repo, `grund refs <ID>` over the source tree is complete by construction, because an ungrounded file cannot land.
- Tiers 2 and 3 ([§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file), [§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)) build on this; the co-change gate in particular lives in the pre-commit / CI recipe layer, not in `grund-core` — a third first-party surface is out of scope ([§FS-non-goals.12](../../functional-spec/FS-non-goals.md#12-surfaces-outside-grund-core-and-the-lsp-transport)).

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| Make it part of `grund check` unconditionally | Would start failing every existing repo on upgrade, and conflates "well-formed references" with "fully adopted discipline" — the same reason `strict` is opt-in ([§DF-reference-marker.2.4](DF-reference-marker.md#24-strict-vs-optional)). |
| Fold it into `[reference] strict` | `strict` is about whether bare tokens are citations; grounding is about whether files cite at all. Two independent axes; a repo may want one without the other. |
| Diff-aware from the start (Tier 3 only) | Needs a base revision and a git diff — a different contract than `grund check` and a dependency the engine avoids ([§FS-non-goals.6](../../functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)). The static floor is useful on its own and is the substrate the gate refines. |
| Hunk-level grounding ("every doc-comment block cites something") | More precise but more machinery; the diff-aware gate is the right place to get hunk precision, against an actual change set. File level is the sound, cheap floor. |
| Require a *test* co-change too (in `grund-core`) | Cannot be done soundly without diffing and without distinguishing behavioral from cosmetic changes (no AST) — belongs in the [§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test) recipe with its escape hatch, not in the engine. |
