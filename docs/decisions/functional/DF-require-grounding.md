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

The check is per file: one resolving citation anywhere in the file satisfies it. A finer "every doc-comment block must cite something" rule is conceivable from the same scan data, but file granularity is the cheap, sound floor and is what the diff-aware gate ([§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)) refines against — there is no need to bake the finer rule into `grund-core` first. **Amended by §4**, which keeps this as the default and makes the finer rule a per-place opt-in rather than a second gate.

### 2.4 Off by default

Like `strict`, grounding is a discipline a repo opts into once it is ready (and once its source tree — including any fixture trees under the `E2E` folder — is either grounded or carved out of `[scan]`). A repo that has never adopted the marker should not start failing `check` on upgrade.

## 3. Consequences

- `Config` gains a `require_grounding: bool`; `check` gains the [§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) loop over the scanner's file list (a new `Findings.scanned_files`); `grund config show` prints the key; `grund check --help` lists the flag; `templates/grund.toml` carries `require_grounding = false` so the generated config still documents every key ([§FS-init.2.4](../../functional-spec/FS-init.md#24-generated-grundtoml)).
- No `grund_config_version` bump: a v1 config without the key keeps working, and a v1 config that sets it is only understood by a `grund` new enough to have this record — an additive change, like `[fmt.cross_refs]`.
- The reverse-lookup story tightens: in a `require_grounding` repo, `grund refs <ID>` over the source tree is complete by construction, because an ungrounded file cannot land.
- Tiers 2 and 3 ([§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file), [§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)) build on this; the co-change gate in particular lives in the pre-commit / CI recipe layer, not in `grund-core` — a third first-party surface is out of scope ([§FS-non-goals.12](../../functional-spec/FS-non-goals.md#12-surfaces-outside-grund-core-and-the-lsp-transport)).

## 4. Grounding per place and per level

**Status:** Accepted
**Date:** 2026-09-02

Two things this record settled in 2026-05 turned out to be one setting each where the repositories that adopted grounding wanted one per place.

**The switch was global.** `require_grounding` reaches every scanned source file and every non-citable home at once (§2.1), while *whether* a file must cite is already reasoned about per place: direction rules constrain how you ground and never whether ([§DISC-citation-directions](../../discussions/proposals/2026-06-13-citation-directions.md#disc-citation-directions-encode-citation-directions-as-checked-config)), and for a non-citable kind the rule follows the home rather than the extension ([§DISC-id-less-kinds](../../discussions/proposals/2026-08-25-id-less-kinds.md#disc-id-less-kinds-kinds-that-declare-no-ids)). A repository that wants "every skill file must cite" cannot say it without also demanding a citation from every workflow and build script in the scan, so it says neither and the hole stays open.

**The unit was the file** (§2.3). One citation anywhere grounds a 300-line skill, and a repository that wants every `##` of one to name what it implements has no way to say so.

### 4.1 The decision

`require_grounding` and a new `grounding_level` become **`[[kinds]]` row keys**, each with its `[reference]` twin as the default for rows that do not say — the shape `index` already has ([§FS-config.3.4.2](../../functional-spec/FS-config.md#342-index--the-kinds-index-file)). The row wins. `require_grounding` stays the boolean it is; `grounding_level` is an integer in Markdown heading levels, `1` being the file and therefore §2.3's rule under a name. The full contract is [§FS-config.3.4.8](../../functional-spec/FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level) and [§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in).

The level is stated in heading levels rather than in a vocabulary of its own because authors already think in `##`, and because `[id] section_heading_levels` already uses *level* for the same count. A source file has no headings, so it gets the two ranks grund can see without parsing code ([§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing)) — unindented doc-comment blocks, and all of them — read by indentation rather than by syntax.

`[citations]` obligations follow the row's unit ([§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation)), so *whether* a place's files must cite and *what* they must cite are asked of the same thing rather than of two units that can disagree.

### 4.2 The global keys are kept, not deprecated

Every existing `require_grounding = true` config keeps its exact meaning with no edit: the default for every row is what the global key was, and the default level is the file. `--require-grounding` needs a global meaning regardless — deprecating the key would leave the flag pointing at nothing — and the flag and the key stay **one knob**, so an explicit row `false` wins over the flag ([§FS-check.1](../../functional-spec/FS-check.md#1-inputs)). "Which wins" then has the answer the config already gives for `index`, and `grund config show` prints the effective values per row. Moving the keys out of `[reference]` into a section of their own would be tidier and is not worth a deprecation cycle; `prefix` → `kind` is already one in flight ([§FS-config.3.4.6](../../functional-spec/FS-config.md#346-prefix-the-former-spelling-of-kind-removed-in-0130)).

### 4.3 Consequences

- `KindConfig` gains `require_grounding: Option<bool>` and `grounding_level: Option<usize>`; `Config` gains the global `grounding_level`. Both are additive, so `grund_config_version` stays at 1 ([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)) and no config has to be edited to keep the meaning it had ([§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered)).
- **One finding is added to unchanged level-1 configs**, and it is the only verdict-relevant movement in this change: the inline-declaration escape of §2.2 no longer grounds a file in a **non-citable** home ([§FS-check.3.6.2](../../functional-spec/FS-check.md#362-the-unit)), because a declaration there is misplaced to begin with ([§FS-check.3.7](../../functional-spec/FS-check.md#37-misplaced-declaration-configured-kind-home)) — the escape was letting a misplaced declaration excuse the file it sat in. Who sees it: a repository with grounding on, a non-citable home, and a file in that home declaring an ID — which already earns a `must not be declared in <home>` error at the same line, so the tree was red before the finding and is red after it. [§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered) governs an added finding as a verdict change; no tree's verdict moves, so neither the deprecation path of [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) nor the loud migration of [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) is engaged. The changelog says it in words anyway, because a run's bytes do change.
- Five config errors close the states the keys cannot describe ([§FS-config.3.4.8](../../functional-spec/FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level)), each anchored at the offending line. Both keys are rejected on a `file` row only where that kind is **citable**: a citable single-file kind's document holds its declarations and the grounding rule leaves it alone, while a *non-citable* `file` home is a place like any other and its row is exactly where its one document's grounding belongs ([§FS-check.3.6.1](../../functional-spec/FS-check.md#361-which-files-a-row-governs)).
- The scanner records each governed file's heading list and doc-comment blocks so the checker still re-reads nothing ([AR-scanner.2](../../architecture/AR-scanner.md#2-per-file-scan)); the recording is taken per file, only where that file's own row asks for a unit finer than the file, so one fine-grained place does not describe the whole tree and a level-1 tree pays nothing for the key.
- The warning of [§FS-check.2.2.1](../../functional-spec/FS-check.md#221-citation-direction-obligation-applies-to-nothing) now points at the row's key rather than the global one, which is the setting its reader can act on.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| Make it part of `grund check` unconditionally | Would start failing every existing repo on upgrade, and conflates "well-formed references" with "fully adopted discipline" — the same reason `strict` is opt-in ([§DF-reference-marker.2.4](DF-reference-marker.md#24-strict-vs-optional)). |
| Fold it into `[reference] strict` | `strict` is about whether bare tokens are citations; grounding is about whether files cite at all. Two independent axes; a repo may want one without the other. |
| Diff-aware from the start (Tier 3 only) | Needs a base revision and a git diff — a different contract than `grund check` and a dependency the engine avoids ([§FS-non-goals.6](../../functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)). The static floor is useful on its own and is the substrate the gate refines. |
| Hunk-level grounding ("every doc-comment block cites something") | More precise but more machinery; the diff-aware gate is the right place to get hunk precision, against an actual change set. File level is the sound, cheap floor. |
| Require a *test* co-change too (in `grund-core`) | Cannot be done soundly without diffing and without distinguishing behavioral from cosmetic changes (no AST) — belongs in the [§RM-cochange-gate](../../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test) recipe with its escape hatch, not in the engine. |
