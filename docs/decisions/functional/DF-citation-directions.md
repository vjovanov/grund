# DF-citation-directions: encode citation directions as checked config with RFC-2119 levels

**Status:** Accepted
**Date:** 2026-06-13

Decides the design proposed in [§DISC-citation-directions](../../discussions/proposals/2026-06-13-citation-directions.md#disc-citation-directions-encode-citation-directions-as-checked-config). Serves
[§GOAL-agent-grounding](../../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) (the climbing rule is the discipline that keeps the three
layers connected) and [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (one source of truth for the rule an
agent reads and the rule the checker enforces).

## 1. Context

The climbing rule is the load-bearing convention of the whole scheme — *goals cite
reasons, specs cite goals, architecture cites specs, code and tests cite specs* — yet
it exists only as prose in the generated agent entrypoint ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)). Nothing
checks it. [§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports)'s planned "unclimbed" report already presumes a
machine-readable climbing rule. We make the rule data: a `[citations]` section in
`.agents/grund.toml`, keyed by **citing** kind, that `grund check` enforces and
`grund init` renders.

## 2. Decision

### 2.1 A `[citations]` section keyed by citing kind, with RFC-2119 levels

Each `[citations.<KIND>]` subsection names the citing side; its arrays name cited
kinds. Five keys form the level ladder, split into two rule classes and two
surfaces:

| Level | Rule class | Checked per | Surface |
|---|---|---|---|
| `must` | obligation | declaration | `grund check` error (`missing-citation`, [§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation)) |
| `should` | obligation | declaration | suggestion (`suggested-citation`, [§FS-check.2.3](../../functional-spec/FS-check.md#23-suggestions-channel-opt-in)) |
| `may` | permission | — | never; punches a hole in a stricter `default` |
| `should-not` | prohibition | citation site | suggestion (`discouraged-citation`, [§FS-check.2.3](../../functional-spec/FS-check.md#23-suggestions-channel-opt-in)) |
| `must-not` | prohibition | citation site | `grund check` error (`forbidden-citation`, [§FS-check.3.12](../../functional-spec/FS-check.md#312-forbidden-citation)) |

An **obligation** asks whether each top-level declaration of the citing kind
contains ≥1 citation to the target kind anywhere in its body. A list with two entries
is conjunctive (one citation satisfying each); a `|` disjunction inside one entry is
satisfied by any one alternative. A **prohibition** fires per offending citation
site, with exact `file:line`. The schema lives in [§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules).

The direct per-kind table is the **canonical form and the whole of v1** — explicit
tables, no layering DSL. A `climb` shorthand that would generate the edges from an
ordering is deferred (§5.10): it is config-authoring sugar that expands into the same
matrix and changes nothing the checker does.

### 2.2 Errors gate, suggestions report — a fixed split, not a knob

`must` / `must-not` are errors in `grund check`'s default run. `should` / `should-not`
**never** appear in check's standing output. The reason is structural, not a
preference: RFC-2119 "should" means *may be ignored with good reason*; grund has no
per-site suppression mechanism; and [§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format) makes any warning replace the
`success` marker. A single consciously-accepted should-deviation would mean the repo
never prints `success` again. Permanently-ignorable findings are category-incompatible
with a gate's standing output — the same logic that keeps the empty-scan caution
([§FS-check.2.2](../../functional-spec/FS-check.md#22-empty-scan)) off the findings stream.

Suggestions are not downgraded to prose — that would recreate the drift this record
removes. They stay machine-checked in two homes: the rendered agent entrypoint at
write time ([§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions)), where most shoulds are satisfied before a doc is ever
checked, and the report layer on demand (`grund check --suggestions`, [§FS-check.1](../../functional-spec/FS-check.md#1-inputs);
`grund gap`, [§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports)).

The severity mapping is **fixed** across both surfaces (`must`→error,
`should`→suggestion, always), so two installs reading one config agree on what gates
and what is suggested ([§FS-non-goals.9](../../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)).

### 2.3 Suggestions are a third channel, not a third severity

[§FS-config.6](../../functional-spec/FS-config.md#6-what-is-not-configured-here) freezes the severity set at `{error, warning}`. A `severity:
"suggestion"` value would edit that frozen list. Instead suggestions are their own
report channel: `Report { errors, warnings, suggestions }`, JSON `"channel":
"suggestion"`, surfaced only under `--suggestions`, never affecting the exit code
([§FS-check.2.3](../../functional-spec/FS-check.md#23-suggestions-channel-opt-in), [§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format)). The pass/fail contract is untouched, and [§FS-config.6](../../functional-spec/FS-config.md#6-what-is-not-configured-here)'s
frozen set still reads true — suggestions are a non-severity advisory channel.

### 2.4 No `grund_config_version` bump

`[citations]` is additive surface and follows the precedent of `[workspace]`
([§DF-subproject-namespaces](DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos)), `require_grounding` ([§DF-require-grounding](DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec)), and the
reserved `[id] number_width` ([§DF-id-number-width](DF-id-number-width.md#df-id-number-width-grund-id-zero-pads-minted-numbers-to-a-default-width-of-3)): all added at v1 with no bump
([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)). An older binary meeting `[citations]` fails loudly with `unknown
config section` — the accepted additive-change failure mode. `default` (per-kind, or
top-level) sets the level for unlisted targets; the global default is `may`, so
adoption is incremental, with precedence explicit > per-kind `default` > global
`default`.

### 2.5 Namespace matching mirrors citation grammar

Rule entries reuse citation shapes ([§FS-workspace.1](../../functional-spec/FS-workspace.md#1-citation-syntax)): bare `AR` = local only;
`alias/AR` = one pinned member; `*/AR` = the kind in any namespace — new syntax,
valid in rule entries only, never in a citation. Mirroring keeps a rule entry
matching exactly the citation tokens it constrains; the match is textual on
qualifier + prefix, so the direction check never loads a foreign config.

### 2.6 Citing-side classification is a scan-time data-model change

`grund check` learns the cited kind from the ID but must be told what kind of place
is citing. The scanner resolves it by three-step fallback — enclosing declaration's
kind (bounded by the declaration body), else the file's kind home, else the reserved
`code` pseudo-kind — and records it on each citation, alongside declaration body
ranges ([§AR-scanner.2.4](../../architecture/AR-scanner.md#24-citing-side-classification)). This is shared infrastructure: `grund cover` ([§FS-cover](../../functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)) and
[§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) need the same declaration→inbound mapping.

The `code` obligation reuses `require_grounding`'s exact "source file" predicate — a
scanned file whose extension is not `.md` ([§DF-require-grounding.2.2](DF-require-grounding.md#22-grounded-is-defined-syntactically)) — so the two
features police the same file set. Direction rules constrain *how* a file grounds;
whether a file must ground at all stays `require_grounding`'s job.

### 2.7 Generated agent-entrypoint section, with a drift check

The entrypoint block gains a generated Citation directions section ([§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions))
that replaces the hand-written climbing-rule prose. Rendering is deterministic
(`[[kinds]]` order, `code` last, fixed level phrases, `|`→"or"), so `grund check`
re-renders it from the live config and byte-compares against the block — editing
`[citations]` without re-running `grund init` is an `agents-init` finding
([§FS-check.3.5](../../functional-spec/FS-check.md#35-invalid-agent-entrypoint-init-block)). The managed block version bumps to 3. The "Unlisted kinds and pairs
are fine" line is load-bearing — without it an agent over-infers prohibitions from
silence.

## 3. Consequences

- `Config` gains a parsed `[citations]` model; `Citation` gains `source_kind` and
  `enclosing_declaration`; `Declaration` gains a body range; `Report` gains
  `suggestions` and `CheckOpts` gains `include_suggestions`. `grund check`'s default
  output and exit codes are unchanged for any repo without `[citations]`.
- This repo adopts the canonical ruleset ([§DISC-citation-directions](../../discussions/proposals/2026-06-13-citation-directions.md#disc-citation-directions-encode-citation-directions-as-checked-config), dogfooding):
  `E2E must = ["FS"]` and `FS must-not = ["AR"]` gate; everything else is a
  should-level `--suggestions` worklist. The FS→AR pointer sites are downgraded to
  plain Markdown links before the `must-not` hardening activates.
- The checked navigational-reference form is **not** shipped; the sanctioned
  downgrade for a discouraged downward reference is a plain Markdown link, which
  composes with `strict = true` (a bare token in a link is not a citation). The
  checked form stays reserved (§5.9).

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| Keep the climbing rule as entrypoint prose | The status quo: unenforced, drifts from any config change, and blocks [§RM-gap-report](../../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports). |
| `should` as a `grund check` warning | Warnings replace the `success` marker ([§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format)) and grund has no suppression; one accepted deviation buries `success` forever (§2.2). |
| A third `severity: "suggestion"` | Edits the frozen `{error, warning}` set ([§FS-config.6](../../functional-spec/FS-config.md#6-what-is-not-configured-here)); the channel model keeps the pass/fail contract intact (§2.3). |
| Bump `grund_config_version` | `[citations]` is additive; bumping would overturn the `[workspace]`/`require_grounding` precedent (§2.4). |
| Inbound rules (`should-be-cited-by`) first | Inverting an outbound obligation flips the quantifier (every FS cites *some* GOAL vs. *this* GOAL is cited by *some* FS); reserved for the gap report's inbound design. |
| Ship the `climb` shorthand in v1 | Pure authoring sugar that expands into the matrix; designing expansion + a second render mode before any direction is enforced is wasted motion (§5.10). |
| Checked navigational references in v1 | Significant new surface (scanner recognition, anchor check, `fmt` maintenance) not needed to ship directions; plain links are the status-quo escape (§5.9). |

## 5. Resolved questions

1. **Section naming**: `[citations]` — it names what is checked. `code` kept, with
   the noise problem solved by scoping obligations to source-by-extension.
2. **Namespace matching**: mirror citation grammar — bare = local, `alias/` = pinned,
   `*/` = any (new, rule-grammar only).
3. **Inbound rules**: outbound-only first; `should-be-cited-by` stays reserved.
4. **Template defaults**: the canonical ruleset ships commented out in
   `templates/grund.toml`; this repo adopts it live.
5. **Enforcement**: `must` / `must-not` gate; `should` / `should-not` are
   machine-checked suggestions surfaced at write time and on demand, never standing
   check output.
6. **More kinds instead of wider rules?** No — a new kind earns its place when
   lifecycle, audience, or home differ, not when only the citation profile does.
   `[citations]` composes with custom `[[kinds]]` unchanged.
7. **Suggestion output model**: a separate report channel, not a third severity.
8. **Config versioning**: no bump — additive surface, `[workspace]` precedent.
9. **Checked navigational references**: deferred; plain-link downgrades are the
   sanctioned escape.
10. **`climb` shorthand vs direct tables**: v1 is the direct per-kind matrix only;
    `climb` is deferred authoring sugar.
