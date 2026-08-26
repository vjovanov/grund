# Goals

Goals state direction. Measurement details live in specs, e2e, CI, and benchmark pages; the map is [AR-goal-measurement](architecture/AR-goal-measurement.md#ar-goal-measurement-goal-and-requirement-meters-live-outside-goals).

## GOAL-agent-grounding: agents stay cited as they work

Keep specs, decisions, tests, and code cited while work happens — the why understood ([§GRUND-understanding](grund.md#grund-understanding-the-why-stays-known)), out of an organized long-term memory ([§GRUND-structure](grund.md#grund-structure-the-projects-long-term-memory-stays-organized)), held consistent by the check ([§GRUND-consistency](grund.md#grund-consistency-the-structure-stays-consistent)). This is the headline outcome from [§GRUND-grund](grund.md#grund-grund-agents-stay-grounded-in-the-spec); every other goal exists to keep that loop correct, cheap, and easy.

### 1. The three layers

Instruction (`init` writes agent entrypoints), verification at rest (`check`), and diff-gated co-change enforcement (`cover` plus recipe).

### 2. What "grounded" requires of a diff

New declarations, code, decisions, and e2e cases cite the most-specific ID they realize or prove.

### 3. What this rules out

No guessed citations, separate lint surface, or hard-coded code-unit heuristic.

### 4. Composition with other goals

Grounding depends on resolving citations, fast checks, readable output, polyglot scan coverage, and zero-config defaults.

### 5. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-no-dangling-refs: every cited ID resolves to a declaration

A passing repo has zero dangling references and zero broken section coordinates. False negatives are bugs. This is the correctness floor under [§GRUND-consistency](grund.md#grund-consistency-the-structure-stays-consistent): a citation an agent cannot trust grounds nothing.

### 1. What "resolves" means

A citation resolves when its declaration exists, its section path exists, and any stub points at an inline declaration of the same ID.

### 2. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-polyglot-citation: IDs cite cleanly from anywhere they are useful

One citation grammar works in Markdown and source comments, across docs/code boundaries, through one resolver. This is the reason `grund` is more than a Markdown link checker: [§GRUND-understanding](grund.md#grund-understanding-the-why-stays-known) has to hold in every language a line is written in.

### 1. What "cleanly" means

Same marker, same section grammar, same resolver, same line-located errors in every supported host language.

### 2. Why this is a goal, not a side effect

Markdown links degrade outside rendered Markdown; `§<ID>` citations must stay useful wherever implementation intent lives.

### 3. Composition with other goals

This is coverage; [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) is correctness.

### 4. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-fast-feedback: grund must be as fast as possible

Speed is an ordering principle. `grund` runs in editors, save loops, commits, and CI; anything slower than the loop gets routed around. A consistency check ([§GRUND-consistency](grund.md#grund-consistency-the-structure-stays-consistent)) that is too slow to run on every save is one that stops being run.

### 1. Performance targets

- Under 100 ms on this repo.
- Under 1 s on a 10k-file repo.
- At most one allocation per file; zero on hot regex paths where possible.

### 2. How we get there

Linear scans, streaming reads, shared compiled regexes, skipped dead directories, and parallelism only when the simple path stops winning.

### 3. Measurable

See [AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands), [AR-ci.5](architecture/AR-ci.md#5-benchmark-job), and [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-zero-config: works on any conformant tree

A canonical repo works with no config and no flags. Divergent layouts configure the difference; empty scans fail loud.

### 1. What "canonical layout" means

Root agent entrypoint, `grund.toml` when needed, `docs/`, `e2e/`, `src/`, configured kinds, and the canonical citation grammar.

### 2. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

### 3. Composition with [§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable)

Zero-config owns the default; configurability owns deliberate divergence.

## GOAL-multi-language: same engine, three platforms

Cargo, npm, and PyPI ship the same engine with idiomatic host bindings and byte-identical behavior. The grounding loop ([§GRUND-grund](grund.md#grund-grund-agents-stay-grounded-in-the-spec)) must hold wherever an agent works, so every host platform runs the same engine.

### 1. Identical behavior

Same tree plus same config produces the same report across bindings and supported operating systems.

### 2. Idiomatic surfaces

Rust returns `Result`, Node returns promises, Python raises exceptions. Behavior stays identical; APIs fit the host.

### 3. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-friendliness-first: as user- and agent-friendly as possible

Friendliness is an ordering principle beside speed: prefer output and workflows humans and agents can act on directly. Grounding ([§GRUND-understanding](grund.md#grund-understanding-the-why-stays-known)) only sticks if staying cited is the path of least resistance.

### 1. Hard requirements

- Errors point at `path:line`.
- JSON output has stable shapes.
- `grund <ID>` returns the smallest useful grounded read.
- Top-level help fits one screen.
- Frequent commands are one token after `grund`.
- Same input produces byte-identical output.
- Passing text `check` prints exactly `success`.

### 2. What this rules out

No configurable severity, report ordering, exit-code mapping, hidden prompts, or extra verbs in frequent workflows.

### 3. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-token-economy: give an agent the right amount of spec, not the whole file

Return the smallest deterministic slice that answers the grounding question; make escalation explicit. Cheap reads keep the organized memory ([§GRUND-structure](grund.md#grund-structure-the-projects-long-term-memory-stays-organized)) affordable enough that an agent grounds every change, not just the cheap ones.

### 1. What this requires

Bare `grund <ID>` is the cheap lead read; `--brief`, `--toc`, section reads, `--full`, `refs --summary`, and narrowed `list` form the escalation ladder.

### 2. What this rules out

No forced full-body reads, generated summaries, abridged diagnostics, or token saving that changes facts.

### 3. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

### 4. Research notes

Evidence for the cheap default lives in [DF-show-default-token-cheap](decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in).

## GOAL-configurable: every default is overridable

Defaults fit canonical `grund`; config makes different project conventions first-class. A project can only keep its work grounded ([§GRUND-grund](grund.md#grund-grund-agents-stay-grounded-in-the-spec)) if the scheme bends to its layout instead of the reverse.

### 1. What is configurable

Kinds, ID format, marker/trigger, strictness, scan scope, comment prefixes, and output defaults per [FS-config](functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up).

### 2. What is NOT configurable

Invariants that decide pass/fail: severity, exit codes, report ordering, and other cross-install agreement rules.

### 3. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-no-silent-breakage: changes ship through a deprecation path

Anything user-visible stays backward-compatible or crosses a named deprecation window. Silent semantic change is a release blocker. A grounding contract ([§GRUND-consistency](grund.md#grund-consistency-the-structure-stays-consistent)) that shifts under a repo without warning is one nobody can rely on.

### 1. What counts as user-visible

CLI surface, output bytes, JSON schema, config schema/version, citation grammar, and managed agent-entrypoint block content.

### 2. The deprecation path

Release N adds the new form while the old form warns; release N+1 or later may remove it after the named horizon.

### 3. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).

## GOAL-small-and-large: start small, configure for big

One binary serves small repos and large monorepos; scale changes layout depth, not the citation contract.

### 1. Small-repo promise

A tiny flat repo works without ceremony.

### 2. Large-repo promise

A large repo can organize specs by component without changing citation syntax or resolver invariants.

### 3. Layout knobs live in config

Scale features are opt-in `grund.toml` settings, not implicit mode switches.

### 4. Composition with [§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree) and [§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable)

Flat defaults keep small repos zero-config; config carries large layouts.

### 5. Measurable

See [AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters).
