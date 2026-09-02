# DF-directions-render: the citation-directions wording is chosen once, against a canonical config

**Status:** Accepted
**Date:** 2026-09-02

Fixes the render [§DF-citation-directions.2.7](DF-citation-directions.md#27-generated-agent-entrypoint-section-with-a-drift-check) introduced and [§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions) specifies. Serves [§GOAL-agent-grounding](../../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) (the rule an agent reads before writing a declaration is the rule the checker enforces) and [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (an entrypoint that states rules, not config syntax).

## 1. Context

The generated `### Citation directions` section is what an agent reads *instead of* `grund.toml`, and the first render of it was inexact in five ways. Rendered from the canonical config of §3 on grund 0.12.2:

```markdown
- **AR** may cite FS or GOAL; unlisted citations are forbidden.
- **DA** must cite FS or GOAL and AR.
- **tests/e2e/** must cite FS and GOAL; avoid citing */AR.
```

1. **A mixed rule read the wrong way.** `must = ["FS|GOAL", "AR"]` printed `FS or GOAL and AR`, which English parses as *FS, or (GOAL and AR)*; the rule is *(FS or GOAL) and AR*. A correctness defect in the entrypoint, not a matter of taste.
2. **No bullet stated its unit.** `**FS** should cite GOAL or FS` is per declaration, `**skills/** must cite FS` per file, `**code** … should cite FS or AR` per *source file that cites anything* ([§FS-config.3.9.2](../../functional-spec/FS-config.md#392-the-homeless-kind)). Three units under one verb, and an agent adding a section to an FS file could not tell which one it was in.
3. **Grounding was not rendered at all.** With `[reference] require_grounding = true` the block never said a source file must cite a declared ID; an agent learned that rule from its first `grund check` failure.
4. **Rule grammar leaked into prose.** `*/AR` is config syntax that is never a citation ([§FS-config.3.9.3](../../functional-spec/FS-config.md#393-namespace-matching)), and a closed per-kind default took two clauses to say "only".
5. **No legend, and a closing line that pointed back at the config.** Nothing said which levels gate, and `Unlisted kinds and pairs follow their configured defaults.` sent the reader to `grund.toml` after the defaults had already been rendered above it.

Every wording change is a managed-block version bump ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)), and every adopting repository sees an `agents-init` finding ([§FS-check.3.5](../../functional-spec/FS-check.md#35-invalid-agent-entrypoint-init-block)) until it re-runs `grund init`. So the wording is chosen **once**, against one config that exercises every branch, and pinned by a golden — not tuned bullet by bullet as each defect is noticed.

## 2. Decision

### 2.1 One paragraph, then bullets, then one closing line

The section is a legend and a grounding sentence in one paragraph, one bullet per citing kind with any rule, and one closing line. Rendered from §3:

```markdown
### Citation directions

`must`/`never` are `grund check` errors; `should`/`avoid` are suggestions (`grund check --suggestions`). Every source file must cite a declared ID or declare one inline; every file under skills/ and tests/e2e/ must cite one.

- Each **GOAL** declaration should cite GRUND or GOAL.
- Each **FS** declaration should cite GOAL or FS; never cite AR; avoid citing GRUND.
- Each **AR** declaration may cite only FS or GOAL.
- Each **DA** declaration must cite (FS or GOAL) and AR.
- Each file in **skills/** must cite FS, AR, or RM; never cite api/AR.
- Each file in **tests/e2e/** must cite FS and GOAL; avoid citing AR in any project.
- Each source file outside the Project map (**code**: Gradle build and workflows) that cites anything should cite FS or AR.
Anything not listed above is allowed.
```

The legend is fixed text and renders whenever `[citations]` is declared, including where the config happens to use only one level: a verb whose enforcement is unknown is read as whichever the agent guesses, and the guess is not stable across agents.

### 2.2 The rules the render follows

| Shape | Render |
|---|---|
| 1 alternative | `AR` |
| 2 alternatives | `FS or GOAL` |
| ≥3 alternatives | `FS, AR, or RM` |
| ≥2 entries | joined with ` and `; an entry with ≥2 alternatives is parenthesised |
| `*/K` | `K in any project` |
| `alias/K` | as spelled — that is how a citation is written |
| citable subject | `Each **K** declaration` |
| non-citable folder home | `Each file in **home/**` |
| non-citable single-file home | `The file **home.md**` |
| homeless subject | `Each source file outside the Project map (**code**) that cites anything`; with a `title`, `(**code**: title)` |
| clause order | `must`, `should`, `may`, `must-not`, `should-not`, then the per-kind default |
| leading prohibition | `must not cite …` / `should not cite …`; after another clause, `never cite …` / `avoid citing …` |
| per-kind `must-not` default + `may` alone | `may cite only …`, no trailing clause |
| per-kind `must-not` default beside `must`/`should` | trailing `never cite anything else` |
| per-kind `should-not` default | trailing `avoid citing anything else` |
| per-kind open default under a closed global one | trailing `may cite anything else` |
| a per-kind default and no lists at all | `must not cite anything` / `should not cite anything` / `may cite anything` |
| global `default = "may"`, `"must"`, or `"should"` | closing `Anything not listed above is allowed.` |
| global `default = "must-not"` | closing `Any citation not listed above is forbidden.` |
| global `default = "should-not"` | closing `Any citation not listed above is discouraged.` |
| `[reference] require_grounding` off | no grounding sentence |

### 2.3 Two lines the ticket settled, and why they differ

**The `code` bullet says "that cites anything."** The obligation constrains *what* a source file cites, never *whether* ([§FS-config.3.9.2](../../functional-spec/FS-config.md#392-the-homeless-kind), [§DISC-citation-directions](../../discussions/proposals/2026-06-13-citation-directions.md#disc-citation-directions-encode-citation-directions-as-checked-config)); a util that cites nothing is not a unit, and "each source file" would have agents inventing citations for utils. A non-citable home is the opposite case — a skill without a spec is the defect — so `Each file in **skills/**` states the intent there, and `require_grounding` closes the hole that [§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation)'s unit rule leaves.

**The grounding sentence distinguishes citing from declaring.** A source file grounds by a citation *or* by an inline declaration ([§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)); a declaration inside a non-citable home is misplaced ([§FS-check.3.7](../../functional-spec/FS-check.md#37-misplaced-declaration-configured-kind-home)), so those files can only cite. An unwalked home ([§FS-config.3.4.7](../../functional-spec/FS-config.md#347-scan--a-place-that-is-listed-not-walked)) is left out of the list: nothing in it is scanned. Naming the level per place waits for [§RM-grounding-per-place](../../roadmap.md#rm-grounding-per-place-require_grounding-and-grounding_level-on-the-kinds-row), which needs the same version bump and should not pay for a second one.

### 2.4 Only `must-not` and `should-not` defaults close anything

[§FS-config.3.9.4](../../functional-spec/FS-config.md#394-defaults-and-precedence) makes a `default` of `must`, `should`, or `may` invent no obligation and prohibit nothing — it leaves an unlisted pair exactly as open as silence does. So a global `default = "must"` closes with `Anything not listed above is allowed.`, the same line `"may"` gets, and the old top sentence *By default, unlisted citation pairs are treated as must.* is dropped: it described a key, not a rule an agent could act on. For the same reason a per-kind open default is rendered only where the global default is closed, which is the one case in which it says something — this kind is a hole in that.

The closing line reports the **global** default alone. A per-kind default is folded into its own bullet and is therefore *listed above*, so a closed per-kind default no longer drags the whole section into `follow their configured defaults`.

### 2.5 Folding "only" needs the `may` list to be the whole permission

`may cite only FS or GOAL` is correct when `may` is all the kind may cite. With a `must` or a `should` entry beside it, the permitted set is wider than the `may` list and "only" would name the wrong set, so those bullets end with `never cite anything else` instead — two clauses, but they say two different things rather than one thing twice.

### 2.6 A prohibition that leads its bullet takes the modal

The subject is a noun phrase, so `Each **FS** declaration never cite AR.` is not a sentence. A leading prohibition therefore renders `must not cite` / `should not cite` — the wording [§FS-check.3.12](../../functional-spec/FS-check.md#312-forbidden-citation) already uses in its findings — while a prohibition following another clause keeps the short `never cite` / `avoid citing` the legend names. This is the one place two forms exist for one level, and it is grammar, not a second rule.

### 2.7 One version bump, v7 → v8

The bump is the whole cost ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)): 20 files in this repository carried the v7 marker and every adopting repository gets one `agents-init` finding until it re-runs `grund init`. It is paid once, here, for all five defects plus the grounding sentence, rather than once per fix.

## 3. The canonical config

One config that hits every rendering branch. It is the fixture of the `init-citation-directions-canonical` e2e case, and §2.1 is its golden.

```toml
grund_config_version = 1
project_name = "canon"

[reference]
strict = true
require_grounding = true            # the grounding sentence

[id]
format = "{kind}-{slug}"

[[kinds]]                           # homeless kind declared FIRST: must render last
kind = "code"
citable = false
title = "Gradle build and workflows" # title beside the fixed phrase

[[kinds]]
kind = "GRUND"
file = "docs/grund.md"              # no rules: no bullet
```

…then `GOAL`, `FS`, `AR`, `DA`, `RM` (cited, never citing: no bullet), and the two non-citable homes `skill` and `e2e`, with:

```toml
[citations]
default = "may"                     # open world: the closing line

[citations.GOAL]
should = ["GRUND|GOAL"]             # two alternatives

[citations.FS]
should = ["GOAL|FS"]
must-not = ["AR"]                   # never
should-not = ["GRUND"]              # avoid; three clauses on one bullet

[citations.AR]
default = "must-not"                # closed per-kind default
may = ["FS|GOAL"]                   #   folds into "only"

[citations.DA]
must = ["FS|GOAL", "AR"]            # mixed: the ambiguous one

[citations.skill]
must = ["FS|AR|RM"]                 # ≥3 alternatives
must-not = ["api/AR"]               # pinned alias stays as spelled

[citations.e2e]
must = ["FS", "GOAL"]               # conjunction of singletons
should-not = ["*/AR"]               # any-namespace: prose

[citations.code]
should = ["FS|AR"]
```

The whole file is `tests/e2e/cases/init-citation-directions-canonical/repo/grund.toml`; three sibling cases pin the branches it cannot hold at once — a homeless kind without a `title`, a closed global default, and grounding off.

## 4. Consequences

- Editing `[reference] require_grounding` now drifts the managed block, because the grounding sentence is rendered from it. Flipping the key without re-running `grund init` is an `agents-init` finding, the same way editing `[citations]` already was ([§FS-check.3.5](../../functional-spec/FS-check.md#35-invalid-agent-entrypoint-init-block)).
- The grounding sentence renders whether or not `[citations]` is declared. A repository with the key on and no direction rules gets the sentence appended to the static citation-direction sentence ([§FS-init.2.3.4.10](../../functional-spec/FS-init.md#23410-citation-direction)) — that is where the defect was actually visible, and gating it on an unrelated section would leave it there.
- The homeless kind's fixed phrase *any file outside a kind home* is gone: the subject says it. [§FS-config.3.9.2](../../functional-spec/FS-config.md#392-the-homeless-kind) is updated to match.
- `citation_directions_section` and its helpers moved out of `init_templates.rs` into `init_citation_directions.rs` ([§AR-core-module-layout.1](../../architecture/AR-core-module-layout.md#1-module-categories)) — the payload/renderer split that file's size exception was waiting for.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| Fix the ambiguous conjunction only | Cheapest patch, but each of the other four defects is also a wording change, and each would cost its own version bump (§2.7). |
| Choose the wording per defect as each is noticed | Five bumps and no config that proves the branches interact; the canonical fixture is what makes the render reviewable at all (§3). |
| Keep `By default, unlisted citation pairs are treated as must.` | Describes a config key that changes nothing an agent does (§2.4); the closing line already carries what is true. |
| Render `must not cite` / `should not cite` everywhere | One form instead of two, but it drops the short `never` / `avoid` the legend names and lengthens every multi-clause bullet (§2.6). |
| Always fold a closed per-kind default into `may cite only …` | Names the wrong permitted set whenever a `must` or `should` entry sits beside the `may` list (§2.5). |
| Render the grounding sentence only under `[citations]` | Ties an unrelated key to the section that happens to host it, and leaves the defect standing in exactly the repositories that have no direction rules (§4). |
| Wait for the per-row grounding form of [§RM-grounding-per-place](../../roadmap.md#rm-grounding-per-place-require_grounding-and-grounding_level-on-the-kinds-row) | It needs the same bump; paying one now and one later is the cost this record exists to avoid (§2.3). |
