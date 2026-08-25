# DISC-id-less-kinds: Kinds that declare no IDs

## Status

**Closed — decided in [§DF-non-citable-kinds](../../decisions/functional/DF-non-citable-kinds.md#df-non-citable-kinds-a-kind-may-declare-no-ids-and-stays-one-kinds-table-when-it-does)**, shipped as
[§FS-config.3.4.1](../../functional-spec/FS-config.md#341-citable--kinds-that-declare-no-ids). Raised 2026-08-25 while trying to give this repository's
`skills/` directory a place in the generated agent entrypoint.

The argument below stands as written; four of its open questions were answered
differently from the shape it proposes, and the text was left alone rather than
rewritten, because a proposal edited to match its own outcome stops being
evidence of how the outcome was reached. What shipped instead:

- **The key is `citable = false`, not `ids = false`** (open question 1).
  *Citable* is already the docs' word for "can be the target of a `§` citation".
- **`prefix` became `kind`** (open question 2), which is what made the
  collision-rule carve-out (open question 3) read as the rule's natural scope
  rather than an exception: prefix-freedom is a property of citable kinds.
- **`code` became a *default* name rather than a fixed one**, which is close to
  what this proposal asked for and reached by a different route. The row is
  **optional**: declaring `citable = false` with no `folder` and no `file` *is*
  the declaration of the complement kind, so a project may name it `src`,
  `modules`, or keep `code` and give it a `title`. Exactly one row may be it,
  and `code` is reserved to that row. Because the row is optional rather than
  always materialised, a config that never declared it prints nothing for it and
  `grund config show` still round-trips — the objection that briefly argued for
  keeping it unwritable.
- **It gets no Project map row** (against this proposal's "renders
  unconditionally"): every row there links a place, and the complement of every
  home is the one kind that has none. So the managed block does not move for any
  existing repository, and there is **no block version bump** — against the v8
  this proposal expected.
- **Non-citable kinds render by place, never by name** — `- [skills/](skills)`
  — a question this text did not reach.
- **A configured kind home is always in the scan scope** (open questions 4 and
  5, answered together and for every kind, not only ID-less ones).
- **Open question 6 is closed by `require_grounding` following the home**
  rather than the file extension, so a Markdown file in such a home can be
  required to cite something.
- **The default `[[kinds]]` table dropped `E2E`** for non-citable `e2e` and
  `integration`, which this text did not propose at all.

## Context

Some directories hold agent-facing content rather than specification: skills,
prompt libraries, runbooks, templates. An agent has to be told they exist and
what they are for, and citations inside them should be checked like citations
anywhere else — but their files are not declarations and should not carry grund
IDs. The request that raised this was literally:

```toml
[[kinds]]
prefix = "SKILL"
folder = "skills"
title = "Agent review and automation skills"
```

*"Add it to AGENTS.md, scan it, but no ID."*

Two of the three asks need nothing new. "No ID" is the status quo for any
directory that is not a kind. "Scan it" is a one-line `[scan] include` edit:
`include` is independent of `[[kinds]]` — its default is a hardcoded
`["requirements.md", "docs", "e2e", "src"]` (`grammar.rs:2`) that covers the
default kind homes only because the two lists were written to agree, a trap in
its own right (open question 5).

"Add it to AGENTS.md" is the gap: **the generated Project map cannot list a
place that has no ID namespace.** It has to be config rather than hand-written
prose below the managed block, because the block is spliced into every agent
surface — `CLAUDE.md`, `GEMINI.md`, `.github/copilot-instructions.md`, and every
workspace member ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints), [§REQ-agents-md](../../requirements/REQ-agents-md.md#req-agents-md-the-agent-entrypoint-stays-managed-and-grounded)). Prose outside the block reaches
`AGENTS.md` alone.

## What a `[[kinds]]` entry does today

One entry feeds six consumers:

1. the ID grammar — `SKILL-<slug>` becomes a legal, citable ID ([§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds))
2. the `KIND ∈ {…}` vocabulary line in the managed block (`init_templates.rs:70`)
3. the **Project map** bullet in that block (`declaration_map`, `init_templates.rs:386`)
4. `grund list` grouping, `grund id`, and hover titles ([§FS-list](../../functional-spec/FS-list.md#fs-list-grund-lists-every-declared-id), [§FS-id](../../functional-spec/FS-id.md#fs-id-grund-proposes-ids-for-new-declarations))
5. `[citations.<KIND>]` direction rules ([§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules)) and the prefix-collision
   rule (`config.rs:416`)
6. the declaration-home boundary of [§FS-check.3.7](../../functional-spec/FS-check.md#37-misplaced-declaration-configured-kind-home) (`KindHomeIndex`, `checker.rs:1238`)

Only #3 is wanted here. On a first reading the other five go dead or go wrong;
on a second, #5 turns out to be the hinge, and the next section says why. It
helps that `[[kinds]]` is *already* a registry keyed by prefix rather than a
pure ID vocabulary: the scanner locates the e2e case root with
`find(|kind| kind.prefix == "E2E")` (`scanner_walk.rs:128`, `scanner.rs:1447`),
and `init.rs:571` / `model.rs:543` special-case `"FS"`.

## `code` is already an ID-less kind

grund already has a kind with no IDs, and it is `code`. It has a name; no
declarations; its own `[citations.code]` table ([§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules)); a rendered
row in the managed block's citation-directions section (`init_templates.rs:206`);
and it is a reserved word that `config.rs:408` refuses as a `[[kinds]]` prefix.

So the wiring an ID-less kind needs already exists. `scanner.rs:503` picks a
citation's citing side in three steps:

1. inside a declaration body → that declaration's kind;
2. otherwise → `file_home_kind(path, config)`, the kind whose configured home
   uniquely contains the file;
3. otherwise → `code`.

Step 2 requires no declaration: a file inside a kind's home already classifies
as that kind. Point a named, ID-less kind at `skills/` and every citation in
those files gets `source_kind = "SKILL"` through existing code, so
`[citations.SKILL]` works and renders beside the `code` row with no new
mechanism.

That answers *"how would we say what it needs to cite?"* — `[citations.*]` keys
on a **name**, not on an ID prefix. The two coincide for ordinary kinds, which
is what made them look like one thing.

## Proposal

Admit a third row to a family that already has two.

| kind | IDs | home | Project map | `[citations.*]` | `KIND ∈ {…}` |
|---|---|---|---|---|---|
| `FS`, `AR`, `E2E`, … | yes | yes | yes | yes | yes |
| `SKILL` (new) | no | yes | yes | yes | no |
| `code` | no | **forbidden** | yes | yes | no |

```toml
[[kinds]]
prefix = "SKILL"
folder = "skills"
title  = "Agent review and automation skills"
ids    = false

[citations.SKILL]
should = ["FS"]
```

`ids = false` drops the entry from `{KINDS_SET}` exactly as `code` is dropped,
keeps it in the Project map, keeps it addressable in `[citations.*]`, and claims
`skills/` as a declaration home in which **nothing may be declared** — which is
[§FS-check.3.7](../../functional-spec/FS-check.md#37-misplaced-declaration-configured-kind-home) working as designed rather than a gap in it.

The name stays mandatory. `[citations.*]` and `grund list --kind` key on it, so
the existing "every `[[kinds]]` entry must declare a `prefix`" error
(`config.rs:381`) survives untouched, and a forgotten prefix line is still
caught rather than silently producing an ID-less kind whose every `§`-citation
then dangles.

### `code` becomes an entry

`code` is the same species and belongs in the same table. It is homeless *by
construction*, not by omission — `scanner.rs:518` defines it as the fallback
when `file_home_kind` returns `None` — and giving it a `folder` would make the
complement nameless again. So the reserved name may carry a `title` and a
`[citations.code]` table, never `folder` / `file`.

Treating it as an implicit ID-less kind collapses three special cases:

- `config.rs:736` — `if citing != CODE_SOURCE_KIND && !known.contains(...)`
  becomes a plain `known` lookup, because `code` is in `known`.
- `init_templates.rs:206` — appending `code` after the kinds in the directions
  section goes away; it is already in the list.
- `config.rs:408` — the reserved-prefix rejection softens from "`code` may not
  be a `[[kinds]]` prefix" to "`code` may only be an ID-less, homeless one",
  which also lets a project retitle it (a Terraform or SQL repo may not want to
  call it "code").

Its Project map row renders **unconditionally**, including in a repo with no
source files and no `[citations.code]`. The tempting gate — show the row only
when the scan read a non-`.md` file — is unavailable: `declaration_map(config)`
takes config alone, and has to, because `grund check` byte-compares the block
and anything tree-dependent would drift as files are added. Unconditional is
also the better answer: today `code` is invisible in the block unless directions
happen to be configured, even though it is the classification most citations in
a typical repo receive.

`declaration_map` already has a homeless branch (`init_templates.rs:396`), but
its parenthetical — *"(inline / configured by convention)"* — is wrong for
`code`. The directions section already phrases it well, *any file outside a kind
home*, and that plus a default title is what the row needs.

### The field is a kind, not a prefix

Once `code` is an entry the field holds `"code"`, which never prefixes anything
— [§FS-config.3.9.2](../../functional-spec/FS-config.md#392-the-homeless-kind) already calls it a *pseudo-kind*. `prefix` is accurate for
the first row of the table above and for no other; `kind` is accurate for all
three.

`kind` is also what the rest of grund already calls this value. Three surfaces
name it and all three say *kind*: the `{kind}` placeholder of `[id] format`
([§FS-config.3.2](../../functional-spec/FS-config.md#32-id--id-grammar)), the `--kind <KIND>` selector of `grund list` ([§FS-list.1](../../functional-spec/FS-list.md#1-inputs)), and
the `[citations.<KIND>]` table key ([§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules)). So does the parsed ID's own
field (`Id.kind`, `model.rs:5`) and the internals around it — `citing_kind`,
`file_home_kind`, `declaration_home_kind`, `CODE_SOURCE_KIND`. Across `docs/`
the word *kind* outnumbers *prefix* 646 to 226, and [§FS-list.1](../../functional-spec/FS-list.md#1-inputs) shows the strain
in a single clause: "*declarations whose ID has one of the named **kind
prefixes** (each a configured `[[kinds]]` **prefix**)*" — the doc has to
translate between the two names mid-sentence. `prefix` is a local alias for
*kind*, used in exactly one config key.

Under the rename, prefix-ness becomes a *derived* property of ID-declaring
kinds, and the collision rule reads as "a kind that declares IDs contributes its
name as the literal prefix of every ID in it, and no such prefix may be a prefix
of another" rather than as a carve-out (open question 3).

The cost is a stutter — `[[kinds]] kind = "SKILL"`. `name` avoids it and is the
runner-up, but it is generic where `kind` is the term the schema is already
committed to; the stutter is paid once at authoring time, the mismatch on every
read of the three surfaces above. `id` is worst of the four: the value is not an
ID (`SKILL-grund-init` is), `id = "code"` is worse than `prefix = "code"`, and
`id` sits one letter from the `ids` opt-out in the same table. Its one
attraction — that *absence* would read as "no ID" — does not apply, because the
opt-out here is explicit rather than by absence.

Blast radius is small: one parse arm (`config.rs:233`), two emitters
(`cli_config.rs:110`, `config_cmd.rs:130`), two shipped templates,
[§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds), and 17 e2e fixtures. Accepting both keys during a deprecation
window is one more match arm plus a "both set" error, so
[§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)'s ordinary path is affordable and the pre-1.0
licence of [§REQ-backwards-compatibility.4](../../requirements/REQ-backwards-compatibility.md#4-what-was-never-a-promise) need not be invoked.

The rename is filed separately as
[issue #129](https://github.com/vjovanov/grund/issues/129), to be decided on its
own — but landed in the same release if at all: the block already goes to v8 and
[§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds) is being rewritten, so a later rename costs a second
disruption.

## Worked example

This repository's `grund.toml` under the proposal — the tail of the `[[kinds]]`
table and the tables it touches. The three additions are marked `# NEW`;
everything else is as committed today.

```toml
[[kinds]]
prefix = "DISC"
folder = "docs/discussions"
title = "Design discussions and proposals"

# ---- kinds that declare no IDs ----------------------------------------------

[[kinds]]                                            # NEW
prefix = "SKILL"
folder = "skills"
title = "Agent review and automation skills"
ids = false

[[kinds]]                                            # NEW
prefix = "code"
title = "Implementation: any file outside a kind home"
ids = false
# No `folder` / `file`: `code` is the complement of every other home, so setting
# one is a config error. Last in the table, because the Project map and the
# citation-directions section both render in `[[kinds]]` order.

[scan]                                               # "skills" is NEW
include = ["docs", "e2e", "src", "crates", "skills", ".github/workflows", "README.md", "AGENTS.md"]

[citations.SKILL]                                    # NEW
should = ["FS"]
must-not = ["AR"]

[citations.code]
should = ["FS|AR"]
```

`grund init` renders that into the managed block as:

```markdown
### Project map

- [GRUND](docs/grund.md): Why: project motivation
- [GOAL](docs/goals.md): Where: project direction and outcomes
- [FS](docs/functional-spec): What: behavior, requirements, and constraints
- [REQ](docs/requirements): Hard requirements: what grund must never break
- [AR](docs/architecture): How: high-level implementation, structure, and design
- [DF](docs/decisions/functional): Product behavior decisions and tradeoffs
- [DA](docs/decisions/architectural): Architecture decisions and tradeoffs
- [E2E](e2e/cases): Executable user scenarios
- [RM](docs/roadmap.md): Planned milestones and sequencing
- [DISC](docs/discussions): Design discussions and proposals
- [SKILL](skills): Agent review and automation skills
- `code`: Implementation: any file outside a kind home
```

```markdown
### Citation directions

- **GOAL** should cite GRUND or GOAL.
- **FS** should cite GOAL or FS; never cite AR.
- **REQ** should cite GRUND or GOAL; never cite AR.
- **AR** should cite FS or GOAL.
- **DF** should cite FS or GOAL.
- **DA** should cite AR or FS.
- **E2E** must cite FS.
- **SKILL** should cite FS; never cite AR.
- **code** (any file outside a kind home) should cite FS or AR.
Unlisted kinds and pairs are fine.
```

The vocabulary line is **unchanged** — `KIND ∈ {GRUND, GOAL, FS, REQ, AR, DF, DA,
E2E, RM, DISC}` — because `ids = false` keeps both new entries out of
`{KINDS_SET}` exactly as `code` is kept out today. Under the rename, only the
two new entries read differently: `kind = "SKILL"` and `kind = "code"`.

## What the example exposes: obligations have no unit

`[citations.SKILL] should = ["FS"]` above is **inert as written**, and that is
the sharpest thing the worked example surfaces.

Prohibitions and obligations are checked on different sides. `must-not` /
`should-not` are per **citation site** ([§FS-check.3.12](../../functional-spec/FS-check.md#312-forbidden-citation)), so they work on an
ID-less kind today with no change — the citing side is already `SKILL` via
`file_home_kind`. But `must` / `should` are per **declaration**
([§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation)), and `obligation_units` (`checker.rs:640`) builds them by
iterating `findings.declarations` filtered to the citing kind. An ID-less kind
has none, so it yields zero units and the obligation silently never fires.

`code` escapes this through a special-cased per-file branch (`checker.rs:633`)
— the same shape of exception that admitting `code` to the table was supposed to
remove. Generalizing that branch to every ID-less kind is the obvious fix, but
it is not a literal copy; two decisions come with it:

- **`.md` must count.** `code_by_file` (`checker.rs:532`) deliberately excludes
  Markdown, on the same reasoning that exempts it from `require_grounding`
  ([§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)): documents are not implementation. A `SKILL` home is
  usually *all* Markdown, so inheriting that filter would make the obligation
  inert a second time. An ID-less kind's home is declared to matter, so every
  scanned file in it should be a unit.
- **Zero-citation files still escape.** Units are built from citations, so a
  file carrying none produces no unit and `must` cannot fire on it —
  [§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation) states this for `code` explicitly ("a source file that
  contains at least one citation but none satisfying the obligation is the
  error"). `require_grounding` does not cover the hole for Markdown homes
  either, since it exempts `.md`. So as things stand there is **no way to
  require that a skill file cite anything at all**. Whether to close that — a
  grounding rule that follows an ID-less home rather than a file extension — is
  a real question, not a detail (open question 6).

This is implementation work the proposal requires, not an optional extra.

## Rejected: a second table

The alternative was to leave `[[kinds]]` alone and add `[[areas]]` / `[[map]]`:
`path` + `title`, no prefix, no ID grammar. It was rejected once the citation
question was asked of it. Four ways it could answer, three of which fail:

- **B1 — the area carries a `name`, `[citations.<name>]` keys on it.** Works,
  and is the only viable one. The table then carries `name`, `path`, `title` and
  participates in `[citations.*]` — `[[kinds]]` minus one boolean.
- **B2 — rules inline on the entry** (`should = ["FS|AR"]` inside `[[areas]]`).
  The only variant needing no name, which is why it tempts. But it forks the
  citation-direction grammar into two syntaxes: namespace qualifiers
  (`alias/KIND`, `*/KIND`), the RFC-2119 ladder, and the overlap-validation rule
  all get implemented and documented twice, and `init_templates.rs:195` renders
  from two shapes. It also breaks the property that `[citations]` is where
  citation policy lives.
- **B3 — key on the path** (`[citations."skills"]`). Makes `[citations.*]` keys
  heterogeneous — prefixes, `code`, and paths — with `[citations.docs]` genuinely
  ambiguous between a kind named `docs` and a folder. And paths move.
- **B4 — no directions at all; areas fall through to `code`.** Coherent and
  cheapest: the area is not a home, so its citations stay `code` and
  `[citations.code]` governs them as it governs `src/`. It fails only when an
  area should differ from code (*"code should cite FS or AR, skills must cite
  FS"*), and it degrades by silence — that can never be expressed, and upgrading
  means schema-changing a table already shipped.

So B reduces to B1, which differs from this proposal by a single boolean. Two
tables that differ by `ids` will spend their lives being asked why they are two
tables.

**Where B1 genuinely wins.** Under this proposal an ID-less entry's `prefix`
passes through the ID-collision rule at `config.rs:416` — no prefix may be a
prefix of another. That rule exists for *tokenization*: `DA` and `DAT` are
ambiguous because `DAT-foo` parses either way. A name that never appears in an
ID never tokenizes, so `SKILL` alongside a hypothetical `SKI` kind would be
**falsely rejected** — a config that should load, does not. Under B1 the split
falls out naturally (areas need uniqueness, kinds need prefix-freedom); here it
needs an explicit carve-out, or a deliberate decision to keep the stricter rule.

**What decided it.** Roughly seven sites would have to consider both vectors —
the `known` set (`config.rs:736`), `file_home_kind` (`scanner.rs:539`, the
citing-side hook itself), `KindHomeIndex` (`checker.rs:1238`),
`declaration_map` (`init_templates.rs:386`), the directions render order
(`init_templates.rs:205`), and both `grund config` round-trips
(`cli_config.rs:110`, `config_cmd.rs:130`). That is fewer sites than the ~28
`.kinds` consumers this proposal touches, but the worse failure mode: **a missed
boolean gives a wrong answer at a site you were looking at; a missed union gives
silence** — an area quietly absent from a feature nobody thought to extend. A
`bool` / `Option` on an existing struct forces a decision at every use site; a
second `Vec` field forces nothing, forever.

## Consequences

**Adopting an entry moves citations from `code` to its name.** A repo's existing
`[citations.code]` rules stop applying inside the new home. It is a verdict
change, but one the maintainer causes with a config edit — fine under
[§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly), and worth stating so it is not discovered.

**`require_grounding` still applies** to any non-`.md` file in the new home
([§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)). Point an ID-less kind at a folder of `.py` prompts and each
file must carry a resolving citation. Probably desirable; not obvious.

**Obligations need a per-file unit rule**, or `must` / `should` on an ID-less
kind never fire — see the previous section.

**The managed block goes to v8.** The ID-less kind alone needs no bump — only
repos that add such an entry see their block change, and they caused it. Putting
`code` in the map changes `{DECLARATION_MAP}` for **every** grund repo, so every
one reports an outdated block until `grund init` re-renders. That is licensed
under [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) as a loud mechanical migration, and it
means landing both changes in the *same* bump: one v8, not a v8 and a v9.

**No `grund_config_version` bump.** `ids` is a new optional key, which
[§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning) classifies as additive; an older binary meeting it fails loudly
through the closed key allow-list (`config.rs:357`).

## Open questions

1. **Key spelling of the opt-out.** `ids = false` versus `declares = false` /
   `citable = false`.
2. **`prefix` → `kind`.** Argued above; decided on its own as
   [issue #129](https://github.com/vjovanov/grund/issues/129).
3. **The prefix-collision carve-out.** Exempt ID-less kinds from the
   prefix-freedom rule at `config.rs:416`, keeping only uniqueness — or keep the
   stricter rule deliberately and say why. Under the rename the exemption is
   the rule's natural reading rather than a carve-out.
4. **Does an ID-less kind imply `[scan]` inclusion?** Its whole purpose is "this
   directory matters", so implying it is tempting. Against: `include` is
   currently the single answer to what gets walked, and making *only* ID-less
   kinds imply inclusion is a shape-dependent rule that is hard to reason about.
5. **A kind home outside the scan scope is silent today.** Independent of this
   proposal: a configured `folder` / `file` outside `[scan] include` is never
   walked, its declarations do not exist, and its citations are *invisible*
   rather than dangling ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)). Nothing warns. A `check` warning
   ("configured home `skills` is outside the scan scope") would remove the trap
   for ordinary kinds and make question 4 moot. Possibly its own proposal.
6. **Should an ID-less home be groundable?** There is no way to require that a
   Markdown file in such a home cite anything at all — obligations need at least
   one citation to attach to, and `require_grounding` exempts `.md`.
7. **`code`'s default title**, and whether a project may override it.
8. **Ordering.** `code` should render last in the Project map, as it already
   does in the directions section. Forced, or is explicit placement in
   `[[kinds]]` honoured?

## Spec changes this drafts into (if accepted)

- [§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-kinds) — the `ids` key, the ID-less shape, the reserved `code` entry,
  and the outcome of the collision-rule question.
- [§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules) — that `[citations.<KIND>]` keys on a name, of which `code` and
  ID-less kinds are instances.
- [§FS-check.3.7](../../functional-spec/FS-check.md#37-misplaced-declaration-configured-kind-home) — a home whose kind declares no IDs admits no declaration of any
  kind, and the message shape for it.
- [§FS-check.3.11](../../functional-spec/FS-check.md#311-missing-required-citation) — the per-file obligation unit for ID-less kinds.
- [§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints) — the Project map gains ID-less rows and the `code` row; managed
  block v8.
- [§FS-list](../../functional-spec/FS-list.md#fs-list-grund-lists-every-declared-id), [§FS-id](../../functional-spec/FS-id.md#fs-id-grund-proposes-ids-for-new-declarations) — behavior for a kind that can never have declarations.
- A `DF-` record for the rejected second table, carrying B1's collision-rule
  advantage and the silent-union failure mode that outweighed it.
