# DISC-citation-directions: Encode citation directions as checked config

## Status

Concluded. Tracks [issue #40](https://github.com/vjovanov/grund/issues/40); accepted
as [§DF-citation-directions](../../decisions/functional/DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels) and drafted into the specs listed under "Spec changes
this drafts into" below.

## Context

The climbing rule — *"Citations climb to reasons. Goals cite reasons, specs cite
goals; architecture cites specs; code and executable tests cite specs."* — lives
only as prose in the generated agent entrypoint ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)). Nothing enforces
it, and [§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) already *assumes* a machine-readable climbing rule for its
planned "unclimbed" report ("IDs whose only inbound citations come from kinds that
violate the climbing rule") without one existing.

This proposal encodes citation directions in `.agents/grund.toml` with RFC-2119
levels, checked by `grund check` and rendered into the agent entrypoint, so the
guidance an agent reads and the rule the checker enforces derive from one source.

## A `[citations]` section keyed by citing kind

Each subsection names the **citing** side; arrays name the **cited** kinds. The
canonical ruleset:

```toml
[citations]                  # absent section = no direction checks (backward compatible)
default = "may"              # explicit; this is also the global default

[citations.GOAL]
should = ["GRUND|GOAL"]      # hub-and-spoke: a goal grounds in the reason or in a goal that does

[citations.FS]
should = ["GOAL|FS"]         # sub-specs may ground in a parent spec
must-not = ["AR"]            # hardening: the What may never depend on the How

[citations.AR]
should = ["FS|GOAL"]         # spec-implementing AR cites the spec; structural AR cites a code-health goal

[citations.DF]
should = ["FS|GOAL"]

[citations.DA]
should = ["AR|FS"]

[citations.E2E]
must = ["FS"]                # no second population is conceivable: a scenario tests a spec

[citations.code]             # reserved pseudo-kind: citing sites outside any kind home
should = ["FS|AR"]
```

This is the **canonical form and the whole of v1**: explicit per-kind tables, one
stated pair-set per citing kind, no layering DSL. Every rule is written where it
applies; nothing is derived. `must` is deliberately rare — it is for pairs where no
legitimate second population exists (`E2E→FS`; the `FS→AR` prohibition); everything
with a plausible alternative grounding path is a `should` with a disjunction wide
enough to make the alternative honest. A `climb` shorthand that would *generate*
most of these edges from a single ordering is attractive but deferred (see the last
section); it changes nothing about what the checker does.

## Levels

The five keys form the RFC-2119 ladder, split into two rule classes and two
surfaces:

| Level | Rule class | Checked per | Surface |
|---|---|---|---|
| `must` | obligation | declaration | `grund check` error (`missing-citation`) |
| `should` | obligation | declaration | suggestion (`suggested-citation`) — report layer only |
| `may` | permission | — | never; punches a hole in a stricter `default` |
| `should-not` | prohibition | citation site | suggestion (`discouraged-citation`) — report layer only |
| `must-not` | prohibition | citation site | `grund check` error (`forbidden-citation`) |

**Obligations** ask: does each top-level declaration of this kind contain ≥1
citation to the target kind, anywhere in its body? A list `must = ["GOAL", "GRUND"]`
is conjunctive (one of each); `"GOAL|GRUND"` inside one entry is the disjunction.
**Prohibitions** fire per offending citation site with exact `file:line`.

## Enforcement split: errors gate, suggestions report

`grund check`'s default run reports **only** `must` / `must-not` violations. The
`should` levels never appear in check's standing output, for a structural reason:
RFC-2119 "should" means *may be ignored with good reason*, grund has no per-site
suppression mechanism, and [§FS-check](../../functional-spec/FS-check.md#fs-check-grund-validates-every-reference-in-a-repo)'s output design makes warnings replace the
`success` marker — so one consciously-accepted deviation would mean the repo never
prints `success` again. Permanently-ignorable findings are category-incompatible
with a gate's standing output. This is the same reasoning that keeps the empty-scan
caution out of the findings stream ([§FS-check.2.2](../../functional-spec/FS-check.md#22-empty-scan)).

But suggestions are not prose — that would recreate the exact disease this proposal
cures. They stay machine-checked, in two homes:

1. **Write time — the rendered agent entrypoint.** For grund's primary consumer
   this is the strongest enforcement point: the agent reads "FS should cite GOAL or
   FS" before writing the doc ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)), so most shoulds are satisfied at
   generation and never need checking after the fact. This serves
   [§GOAL-agent-grounding](../../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work).
2. **On demand — the report layer.** `grund check --suggestions` forces the
   suggestion records into one run (the same config-plus-flag pattern as
   `--require-grounding`, [§FS-check.1](../../functional-spec/FS-check.md#1-inputs)), and `grund gap` includes them once
   [§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) ships.

The severity mapping stays **fixed** across both surfaces (`must`→error,
`should`→suggestion, always), keeping this compatible with [§FS-non-goals.9](../../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization): projects
choose the rules, but two grund installs reading the same config always agree both
on what gates and on what is suggested.

## Output and API shape

Suggestions are a **third report channel, not a third severity.** [§FS-config.6](../../functional-spec/FS-config.md#6-what-is-not-configured-here)
freezes the severity set at exactly `{error, warning}` under the same
[§FS-non-goals.9](../../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization) logic this proposal leans on. Adding a `severity: "suggestion"`
value would edit that frozen list; modelling suggestions as their own channel leaves
the gate's pass/fail contract untouched. Concretely ([§FS-check.2.3](../../functional-spec/FS-check.md#23-suggestions-channel-opt-in),
[§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format)):

- **Text** — default `grund check` prints errors and warnings only;
  `grund check --suggestions` additionally prints suggestion lines (the same
  `path:line: message` shape). With the flag absent, the `success` marker still
  appears for a run with zero errors and zero warnings even if suggestions exist.
- **Exit code** — suggestions never affect it (`0`/`1`/`2` unchanged).
- **JSON** — suggestion records carry `"channel": "suggestion"` (not a `severity`),
  so a consumer filtering on `severity ∈ {error, warning}` is unaffected;
  `--format=json` emits them only under `--suggestions`.
- **API** — `Report` gains a third field: `Report { errors, warnings, suggestions }`;
  `CheckOpts` gains `include_suggestions: bool`. The `suggested-citation` /
  `discouraged-citation` codes live on the `suggestions` vector, never on
  `errors` / `warnings`.

## Versioning

Adding `[citations]` does **not** bump `grund_config_version`; it stays `1`. This
follows the established convention: `[workspace]` ([§DF-subproject-namespaces](../../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos)),
`require_grounding` ([§DF-require-grounding](../../decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec)), and the reserved `[id] number_width`
([§DF-id-number-width](../../decisions/functional/DF-id-number-width.md#df-id-number-width-grund-id-zero-pads-minted-numbers-to-a-default-width-of-3)) were all added at v1 with no bump ([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)). The version
key is reserved for *incompatible changes to existing semantics*; new optional
surface is additive. An older binary meeting a config that uses `[citations]` fails
loudly with `unknown config section` — the accepted failure mode for "this config
uses a feature your grund predates."

`default` (per-kind, optionally top-level) sets the level for unlisted targets;
the global default is `may`, so adoption is incremental. Precedence: explicit target
list > per-kind `default` > global `default`.

## Namespace matching mirrors citation grammar

Rule entries use the same shapes citations do ([§FS-workspace.1](../../functional-spec/FS-workspace.md#1-citation-syntax), [§FS-workspace.4](../../functional-spec/FS-workspace.md#4-resolution)): a
bare `AR` matches the **local** namespace only; `alias/AR` pins one member; `*/AR` —
**new syntax, valid in rule entries only, never in a citation** — matches the kind
in **any** namespace, including the local one.

Rationale: under [§FS-workspace.4](../../functional-spec/FS-workspace.md#4-resolution), a bare `AR-foo` citation can only resolve locally, so a
bare rule entry that matched foreign citations would give identical tokens different
scopes. Mirroring makes a rule entry match exactly the citation tokens it
constrains. The check is textual on the qualifier + prefix — resolution failures are
already separate errors ([§FS-check.3.8](../../functional-spec/FS-check.md#38-cross-project-citation-failure)), so the direction check never loads a foreign
config.

## Classifying the citing side

The scanner knows the *cited* kind from the ID but not what kind of place is citing.
Classify each citation site by three-step fallback, with explicit bounds at each
step ([§AR-scanner.2.4](../../architecture/AR-scanner.md#24-citing-side-classification)):

1. **Enclosing declaration's kind — bounded by the declaration body.** In markdown,
   a body runs until the next same-or-higher heading. In a source doc-comment, the
   body is bounded by the comment/docstring block, and within a multi-ID block the
   nearest preceding declaration line wins. Citations later in the *file* do **not**
   inherit the kind.
2. **Kind home of the file** — reverse lookup from `[[kinds]]` `folder` / `file`.
3. **`code`** — reserved lowercase fallback for any other scanned site.

This is a concrete data-model change, not a checker pass over existing data:
`Declaration` gains a body range, `Citation` gains `source_kind` and
`enclosing_declaration`. The checker cannot reconstruct this cheaply after the fact —
doc-comment declaration ranges are narrower than the file — and the same fields are
exactly what `grund cover` ([§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)) and the gap report ([§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports)) need.

### Special cases

- **Stub + inline declaration**: obligations evaluate the inline body; the one-line
  stub (`# <ID>: [<path>](<path>)`) is never the evaluation target.
- **E2E**: declarations are case directories — an obligation evaluates over the
  case's *scanned* files, so fixture trees carved out of `[scan]` stay invisible
  (consistent with [§DF-require-grounding](../../decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec)).
- **`code`**: obligations apply per file, only to files that contain ≥1 citation,
  and only to **source files** under the exact definition `require_grounding`
  already uses — "scanned file whose extension is not `.md`" ([§DF-require-grounding.2.2](../../decisions/functional/DF-require-grounding.md#22-grounded-is-defined-syntactically)).
  Markdown outside a kind home (README, changelog) is therefore prohibition-checked
  but obligation-exempt. Direction rules constrain *how* you ground, never
  *whether* — forcing files to cite remains `require_grounding`'s job.

## AGENTS.md rendering

The entrypoint block (validated by [§FS-check.3.5](../../functional-spec/FS-check.md#35-invalid-agent-entrypoint-init-block)) gains a generated section
replacing the hand-written climbing-rule prose, so agent guidance derives from the
same config `check` enforces ([§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions)). One bullet per citing kind:

```markdown
### Citation directions
- **GOAL** should cite GRUND or GOAL.
- **FS** should cite GOAL or FS; never cite AR.
- **AR** should cite FS or GOAL.
- **DF** should cite FS or GOAL.
- **DA** should cite AR or FS.
- **E2E** must cite FS.
- **code** (any file outside a kind home) should cite FS or AR.
Unlisted kinds and pairs are fine.
```

Rendering is deterministic: `[[kinds]]` order, `code` last, levels render as fixed
phrases (*must cite / should cite / avoid citing / never cite*), `|` → "or",
conjunctive lists → "and". The "unlisted kinds and pairs are fine" line is
load-bearing — without it agents over-infer prohibitions from silence.

**Drift check**: block content now derives from config, so the existing version
check is insufficient — editing `[citations]` without re-running `grund init` would
leave stale guidance under a current version number. `check` re-renders the section
from the live config and byte-compares (rendering is deterministic, so determinism
*is* the hash); a stale block is an `agents-init` finding. The managed block version
bumps to 3.

## Gap report tie-in

An ID is **climbed** when an inbound citation comes from a kind whose own member's
rules oblige (`must` / `should`) citing this kind. Peer, downward, and unlisted-kind
inbound never counts. With the direct per-kind matrix this obligation-edge definition
is complete: every intended upward edge is an explicit `must` / `should`, so
"climbed" reads straight off the rules. `grund gap` is also the standing home for the
suggestion records ([§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports)).

## Dogfooding: the ruleset vs this repo

Dry-run of an earlier, stricter draft (`GOAL must = ["GRUND"]`, `AR must = ["FS"]`,
`FS should = ["GOAL"]`) against `docs/`:

- Most `GOAL` declarations cite nothing upward; the three that climb cite peer goals
  → hence `should = ["GRUND|GOAL"]`.
- `AR-checker` cites nothing — a real gap. `AR-core-module-layout` is structural
  architecture serving every spec, where citing one FS would be ceremony → hence
  `should = ["FS|GOAL"]`, with the structural population grounding in a code-health
  goal.
- 10+ FS docs cite `§AR-` — but sampled sites are navigational pointers to mechanics,
  not the What depending on the How. The `must-not = ["AR"]` hardening activates only
  after those sites are downgraded to plain Markdown links (the sanctioned escape).
- Several FS docs cite no GOAL but ground in parent FS docs — sound sub-spec practice
  → hence `should = ["GOAL|FS"]`.

With the calibrated rules and the enforcement split, migration mostly evaporates:
this repo adopts the full ruleset immediately, because should-level findings become a
`--suggestions` / gap worklist rather than standing check noise. Only two things gate:
`E2E must = ["FS"]` and `FS must-not = ["AR"]` (adopted after the FS→AR pointer sites
are downgraded to plain Markdown links).

## Reserved: navigational references (not in v1)

Prohibitions need a sanctioned downward-pointer. An earlier draft proposed a
link-wrapped bare ID as a third, checked reference category. On reflection that is
significant new surface and is not needed to ship direction rules. **Decision:** the
sanctioned downgrade for a discouraged downward reference is a **plain Markdown
link** — exactly what this repo already did by hand (`49a46de6cb`). The checked
navigational-reference form stays **reserved** as a follow-up, revisited only if
`must-not` prohibitions prove unusable with plain-link downgrades.

## Deferred: the `climb` shorthand

The canonical matrix is mostly the consequence of one ordering — `GRUND ← GOAL ← FS ←
(AR · DF · DA) ← (E2E · code)`. Encoding that ordering and generating the edges from
it is attractive for a large kind set, but it is pure config-authoring sugar: it
expands into the same per-kind matrix before any checking happens, so it changes
nothing the checker does. v1 ships the explicit matrix; `climb` is backward-compatible
to add later (a config that adopts it is just a shorter spelling of a matrix the
engine already understands).

## Spec changes this drafts into

- [§DF-citation-directions](../../decisions/functional/DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels): the levels, the enforcement split, and the resolved
  questions.
- [§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules): the `[citations]` schema; a clarifying edit to [§FS-config.6](../../functional-spec/FS-config.md#6-what-is-not-configured-here) (the
  suggestions channel is non-severity advisory, so the frozen `{error, warning}` set
  still reads true).
- [§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation) / [§FS-check.3.12](../../functional-spec/FS-check.md#312-forbidden-citation): the `missing-citation` / `forbidden-citation`
  errors; [§FS-check.1](../../functional-spec/FS-check.md#1-inputs) the `--suggestions` flag; [§FS-check.2.3](../../functional-spec/FS-check.md#23-suggestions-channel-opt-in) the suggestions channel.
- [§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format): the JSON `"channel"` field.
- [§AR-scanner.2.4](../../architecture/AR-scanner.md#24-citing-side-classification): the citing-side classification rules and declaration body ranges.
- [§AR-checker.2.9](../../../crates/grund-core/src/checker.rs) / [§AR-checker.2.10](../../../crates/grund-core/src/checker.rs): the obligation and prohibition passes.
- [§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions): the generated Citation directions section and its drift check.
