# DF-non-citable-kinds: a kind may declare no IDs, and stays one `[[kinds]]` table when it does

**Status:** Accepted
**Date:** 2026-08-25

## 1. Context

Some directories hold agent-facing content rather than specification: skills, prompt libraries, runbooks, test suites. An agent has to be told they exist and what they are for, and the citations inside them should be checked like citations anywhere else — but their files are not declarations and should not carry IDs. The request that raised this, about this repository's own `skills/`, was literally *"add it to `AGENTS.md`, scan it, but no ID."*

Two of the three asks needed nothing new. "No ID" is the status quo for any directory that is not a kind. "Scan it" was a `[scan] include` edit. "Add it to `AGENTS.md`" was the gap: the Project map is generated from `[[kinds]]` and spliced into every agent surface ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints), [§REQ-agents-md](../../requirements/REQ-agents-md.md#req-agents-md-the-agent-entrypoint-stays-managed-and-grounded)), so prose written below the managed block reaches `AGENTS.md` alone. **The map could not list a place that has no ID namespace.**

The full analysis, including the four ways the alternative could have answered the citation question, is [§DISC-id-less-kinds](../../discussions/proposals/2026-08-25-id-less-kinds.md#disc-id-less-kinds-kinds-that-declare-no-ids).

## 2. Decision

### 2.1 `citable = false` on the existing `[[kinds]]` table

A kind has two independent properties — *has a home* and *declares IDs* — and all four cells are real:

| | home | no home |
|---|---|---|
| **citable** | `FS`, `AR`, … | inline-declared kinds |
| **not citable** | `skill`, `e2e` (new) | `code` |

The bottom-left cell is the one that had no spelling. `citable = false` gives it one ([§FS-config.3.4.1](../../functional-spec/FS-config.md#341-citable--kinds-that-declare-no-ids)) without adding a mechanism: the scanner's citing-side classification already reaches a file by its home and asks nothing about declarations ([AR-scanner.2.4](../../architecture/AR-scanner.md#24-citing-side-classification)), so `[citations.<kind>]` governs the new kind through code that already existed.

*Citable* is the word, not `ids` or `declares`, because it is already this spec's word for "can be the target of a `§` citation" — citable sections, citable IDs. A citable kind is one whose IDs you can cite.

### 2.2 Rejected: a second `[[areas]]` table

The alternative was to leave `[[kinds]]` alone and add `[[areas]]`: `path` + `title`, no name, no ID grammar. It was rejected once the citation question was asked of it. Of the four ways it could answer, three fail — rules inline on the entry fork the citation-direction grammar into two syntaxes, keying `[citations]` on a path makes its keys heterogeneous and `[citations.docs]` ambiguous, and falling through to `code` can never express *"code should cite FS or AR, skills must cite FS"* and degrades by silence. The fourth works, and it is `[[kinds]]` carrying `name`, `path`, `title` and participating in `[citations.*]` — **`[[kinds]]` minus one boolean**. Two tables that differ by one boolean spend their lives being asked why they are two tables.

**What the second table would genuinely have won**, and what outweighed it, is the same fact twice. Roughly seven sites would have had to consider both vectors — the known-kind set, `file_home_kind`, the declaration-home index, the Project map, the directions render order, and both `grund config` round-trips. That is fewer sites than the ~28 `.kinds` consumers this decision touches, but the worse failure mode: **a missed boolean gives a wrong answer at a site you were looking at; a missed union gives silence** — an area quietly absent from a feature nobody thought to extend. A `bool` on an existing struct forces a decision at every use site; a second `Vec` field forces nothing, forever.

### 2.3 A non-citable kind is rendered by place, never by name

Its name exists to key `[citations.*]` on and to be unique. There is nothing to cite, so `- [skills/](skills): …` in the Project map and `- **skills/** must cite FS.` in the directions ([§FS-init.2.3.4.4](../../functional-spec/FS-init.md#2344-project-map), [§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions)), and the same label in every finding about it ([§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation), [§FS-check.3.12](../../functional-spec/FS-check.md#312-forbidden-citation)). Printing the name would teach an agent a token it must never write.

The **homeless kind** keeps its name, because it is the one non-citable kind with no place — the complement of every home there is, and there is nothing else to call it by.

`code` is that name by default and not by fiat. The complement is a category, and which word fits it is a property of the repository: `code` is right for most and wrong for a Terraform, SQL, or prose tree. So a project may declare the kind and name it — `citable = false` with no `folder` and no `file` is the declaration, since that shape *is* what "the complement of every home" means ([§FS-config.3.9.2](../../functional-spec/FS-config.md#392-the-homeless-kind)). What survives of the old rule is narrower and still load-bearing: exactly one row may be the complement, and the name `code` is available only to it, because any other row wearing it would collide with the kind every citation outside a home resolves to.

An earlier draft of this decision kept `code` unwritable, on the grounds that a row for it would be printed by `grund config show` and the printed config has to load back as itself. That argument confused *materialising* the row with *permitting* it. The row is optional: absent means the reserved default, so a config that never declared it prints nothing for it and round-trips exactly as before.

### 2.4 The field is a kind, not a prefix

`prefix` was accurate while every row of the table declared IDs. `citable = false` makes it wrong for half the table, and *kind* is the word the rest of grund already uses — the `{kind}` placeholder of `[id] format`, the `--kind` selector, the `[citations.<kind>]` key, and the parsed ID's own field. Across `docs/` the word *kind* outnumbers *prefix* by roughly three to one, and [§FS-list.1](../../functional-spec/FS-list.md#1-inputs) used to have to translate between the two names mid-sentence.

Under the rename, prefix-freedom becomes a *derived* property of citable kinds rather than a rule about the schema's own vocabulary ([§FS-config.3.4.5](../../functional-spec/FS-config.md#345-name-rules)) — which is what lets it be scoped correctly. The rule exists because `DAT-foo` parses as either `DA` or `DAT`; a name that never appears in an ID never tokenizes, so `skill` beside a citable `SKI` is a config that should load, and under the old framing it was rejected.

The cost is a stutter — `[[kinds]] kind = "skill"`. `name` avoids it and was the runner-up, but it is generic where `kind` is the term the schema is already committed to: the stutter is paid once at authoring time, the mismatch on every read of the three surfaces above. `prefix` still loads through the deprecation window of [§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) ([§FS-config.3.4.6](../../functional-spec/FS-config.md#346-prefix-the-former-spelling-of-kind-removed-in-0130)) — one match arm and a "both set" error, which is cheaper than invoking the pre-1.0 licence of [§REQ-backwards-compatibility.4](../../requirements/REQ-backwards-compatibility.md#4-what-was-never-a-promise).

Consequence: that window closed. `prefix` stopped loading in 0.13.0, the match arm and the "both set" error went with it, and [§FS-config.3.4.6](../../functional-spec/FS-config.md#346-prefix-the-former-spelling-of-kind-removed-in-0130) is now the record of a removed key.

### 2.5 Obligations get a per-file unit, and grounding follows the home

`must` / `should` are checked per declaration, so on a kind with none they would have yielded zero units and passed vacuously — the rule would have been accepted by the config validator and then never fired. The unit is therefore every scanned file in the home that carries a citation, **`.md` included** ([§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation)).

`code` escapes the same problem through a per-file branch that excludes Markdown, and inheriting that filter here would have made the obligation inert a second time: a non-citable home is usually *all* Markdown. The exclusion reasons about implementation versus document ([§DF-require-grounding.2.2](DF-require-grounding.md#22-grounded-is-defined-syntactically)); a home the maintainer declared matters is neither guess.

Units are built from citations, so a file with none produces no unit and `must` cannot reach it. `[reference] require_grounding` closes that hole in a non-citable home, over every scanned file in it including Markdown ([§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)) — so "cite something" and "cite an `FS`" are two keys that compose instead of one rule with a hole.

### 2.6 A configured home is in the scan scope by construction

`include` was the single answer to what gets walked, and a `folder` outside it was never read: its declarations did not exist and its citations were **invisible rather than dangling** ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)). That trap predates this decision and applies to every kind, but a kind whose entire content is "this directory matters" would have hit it on its first line of config, so it is closed here rather than filed as a separate rule nobody would reach for. `include` keeps its job: the extra roots.

## 3. Consequences

**Adopting an entry moves its files' citations from `code` to the new kind**, so `[citations.code]` stops applying inside the home. A verdict change the maintainer causes with a config edit — fine under [§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly), and worth stating so it is not discovered.

**A repository with a kind home outside `[scan] include` starts seeing that home's findings.** They were always true of the tree; the run was not reading it. Loud rather than quiet is the direction the requirement cares about.

**The default kind set drops `E2E` for non-citable `e2e` and `integration`** ([§FS-config.3.4.4](../../functional-spec/FS-config.md#344-the-default-kinds)). Nothing in this repository cites an `E2E-` ID — the ID names the per-case obligation unit and nothing else — and a test that is a citation *target* inverts the direction the rest of the model runs in. `E2E` stays fully supported. This repository declared it when this was decided and moved to the default pair right after 0.12.0 shipped — the corpus to `tests/e2e/cases/`, the fixture repositories kept out of the host scan by `[scan] exclude` instead of the case pass, and the per-case `must cite FS` into the harness's `spec.refs` gate; the case machinery is exercised by the e2e cases that declare the kind.

It carries one consequence that is not a config edit, and it is the sharpest thing here. The e2e case machinery follows the configured `E2E` home ([AR-scanner.6](../../architecture/AR-scanner.md#6-e2e-case-declarations)), and fixture-tree pruning is part of it — a nested case repo under `e2e/cases/` is not scanned because the case pass owns it. A repository that has **no `[[kinds]]` block at all** *and* an `e2e/cases/` corpus therefore starts scanning its fixtures on upgrade, and a deliberately-broken fixture there becomes a finding. Every config `grund init` has ever written declares `[[kinds]]` in full, including `E2E`, so the affected set is hand-written minimal configs; the fix is one `[[kinds]]` block naming the kind the repository was relying on by default, and the release notes it. This is the pre-1.0 licence of [§REQ-backwards-compatibility.4](../../requirements/REQ-backwards-compatibility.md#4-what-was-never-a-promise) being used, which is why it is argued here rather than assumed.

**No managed-block version bump.** The block's bytes change only for a repository that adds a non-citable kind — and it caused that. `code` gains no map row, so no existing block moves under it ([§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions)); the default-schema change reaches newly generated configs only.

**No `grund_config_version` bump.** `citable` and `kind` are additive keys, which [§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning) classifies as not moving the integer; an older binary meeting either fails loudly through the closed key allow-list.
