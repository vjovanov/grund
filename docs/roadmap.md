# Roadmap

What `grund` plans to ship next, in priority order. Each item has a stable ID — `RM-<slug>` under this repo's `[id] format` ([§FS-config.3.2](functional-spec/FS-config.md#32-id--id-grammar)); `RM` is a configured `[[kinds]]` prefix ([§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes)), so `grund check` validates `§RM-…` citations like any other. Items may be cited from anywhere — commits, PRs, the changelog, other specs. Shipped items move their detail to `docs/changelog.md` and keep a one-line pointer in §"Shipped milestones" below so the citation does not dangle; cancelled items stay in place with a `~~strikethrough~~` title and a one-line reason.

The check engine, the retrieval surface (`grund <ID>`, `grund refs`, including E2E case manifests), the coverage index (`grund cover`), bulk normalization (`grund fmt`, including `--marker` and `--cross-refs`), config loading (`.agents/grund.toml` plus `grund config show` / `grund config validate`), `grund init`, `grund id`, the opt-in grounding floor ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), the token-cheap read surfaces ([§RM-token-cheap-grounding](roadmap.md#rm-token-cheap-grounding-token-cheap-read-surfaces-for-agents)), the e2e corpus, the benchmark baseline/gate ([§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets)), the live registry-name guard ([§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process)), the `grund-core` / `grund-cli` workspace split with data-returning core APIs ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli)), and parallel per-file scanning ([§RM-parallel-scan](roadmap.md#rm-parallel-scan-parallel-per-file-scanning-for-large-repo-throughput)) are all shipped — see `docs/changelog.md`. Two arcs remain. The **distribution arc**: publish on npm and PyPI alongside cargo, ship the optional LSP server, and add `grund check --watch`. And the **grounding arc** (the third layer of [§GOAL-agent-grounding.1](goals.md#1-the-three-layers), diff-gated enforcement): build on [§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) and [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) toward a diff-aware co-change gate — implementation cannot change without the spec it grounds in and without a test of it — via a pre-commit / CI recipe that consumes `grund cover` ([§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)). Four standalone items sit outside both arcs: [§RM-declaration-near-miss](roadmap.md#rm-declaration-near-miss-warn-on-a-heading-that-looks-like-a-declaration-but-does-not-match-id-format) adds a warning for a heading that looks like a declaration but does not match the configured `[id] format`, [§RM-positioning](roadmap.md#rm-positioning-the-lychee-contrast-and-the-instruction-count-framing-in-readme-and-landing-copy) keeps the README/landing pitch paired with the benchmark story, [§RM-gap-report](roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) inverts the [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) index into an orphan / uncovered-ID report, and [§RM-positioning-trace-tools](roadmap.md#rm-positioning-trace-tools-position-grund-against-requirements-traceability-tools-in-readme) extends the README positioning to the requirements-traceability neighbourhood (OFT, Sphinx-Needs, TRLC, Doorstop, Duvet, SARA). The IDed milestones below project both arcs onto reviewable units of work.

## RM-distribution: cargo + npm + pypi from one engine

Per [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) and [§AR-bindings](architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms). Builds on the shipped workspace split ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli)).

### 1. What

napi-rs binding for npm; PyO3 binding for PyPI; CI publish jobs for all three registries (`grund-core` first, in dependency order). Each publish job builds the CLI binary with profile-guided optimization via `scripts/pgo-build.sh` ([§DA-pgo-release](decisions/architectural/DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload), [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process)) — wired for the crates.io `grund` binary already, extended to the prebuilt npm and PyPI binaries here.

### 2. Why now

`grund` is only viable as a CI dependency for non-Rust projects once it ships on their native package manager ([§GOAL-multi-language](goals.md#goal-multi-language-same-engine-three-platforms)).

### 3. Measurable

Integration test runs the same spec corpus through all three bindings and asserts byte-identical reports ([§AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters)).

## RM-lsp: ship the optional LSP server

Per [§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-will-ship-an-optional-lsp-server), [§AR-lsp](architecture/AR-lsp.md#ar-lsp-how-the-lsp-server-is-built), and [§DA-lsp-optional](decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). Adds `crates/grund-lsp/` to the workspace and publishes it as a separate package on cargo, npm, and PyPI. No first-party per-editor wrappers ship; editor configuration is the user's one-time work, with example snippets in the README.

This is half of the live feedback loop. The CLI is solid, but the product promise is strongest when citations fail while the user is editing: diagnostics, hover preview, go-to-definition, and the live `$$ -> §` transform from [§FS-lsp.1](functional-spec/FS-lsp.md#1-capabilities). The editor-less half is [§RM-watch](roadmap.md#rm-watch-implement-grund-check---watch), which gives the same every-save loop without requiring editor setup.

### 1. What

A `grund-lsp` binary that speaks LSP over stdio and serves the four capabilities pinned in [§FS-lsp.1](functional-spec/FS-lsp.md#1-capabilities): diagnostics, hover preview, go-to-definition, and the live `$$ → §` transform (the bulk form of which already ships in `grund fmt`). Holds an in-memory `Findings` per workspace; full re-scan strategy on every change for v1 ([§AR-lsp.3.1](architecture/AR-lsp.md#31-full-re-scan-on-every-change-v1)). Parity with the CLI is enforced by an e2e harness that drives the LSP through the same `e2e/cases/*` corpus and asserts byte-equivalent output ([§AR-lsp.5](architecture/AR-lsp.md#5-determinism-and-parity-tests)).

Distribution: separate package on each registry ([§FS-distribution.1](functional-spec/FS-distribution.md#1-targets)). The CLI install does not pull in `grund-lsp` transitively. README gains a section with example LSP-client snippets for Helix, Neovim, Zed, Emacs, VSCode (generic LSP client extension), and IntelliJ via LSP4IJ.

### 2. Why now

[§GRUND-grund.1](grund.md#1-what-grund-does-about-it) keeps Markdown links peripheral and centers verify/refactor-safe/extract — three pillars all satisfied by CLI-shaped surfaces. Editor integration is then a UX layer over those, and the cheapest non-zero answer is one LSP server every editor can talk to. The workspace split is already shipped ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli)); shipping this after [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) means the engine is factored as a library and the registries are already wired.

### 3. Depends on

- [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) should land first so the registry publishing path exists before `grund-lsp` adds another package to it.

### 4. Measurable

`grund-lsp` installs from each registry. An editor pointed at the binary receives diagnostics, hover bodies, and definition jumps for any conformant repo, and parity tests assert byte-equivalence with `grund check` and ID queries across the e2e corpus. Diagnostic latency on file change is within [§GOAL-fast-feedback.1](goals.md#1-performance-targets)'s per-scan budget.

## RM-watch: implement grund check --watch

Per [§FS-check.6](functional-spec/FS-check.md#6-watch-mode---watch). The editor-less "every save" loop [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) exists for — re-run `grund check` on every change under the scanned tree, clearing prior output each run.

Together with [§RM-lsp](roadmap.md#rm-lsp-ship-the-optional-lsp-server), this ships the live feedback loop: LSP for editor users, `grund check --watch` for terminal users and editor setups that do not speak LSP.

### 1. What

`--watch` on `grund check`: filesystem-notification-driven, debounced, no polling and no configurable interval. Each run is byte-identical to a plain `grund check` on the tree's current state; on Ctrl-C the process exits with the last completed run's exit code. Non-interactive — no TUI, no key bindings ([§FS-non-goals.10](functional-spec/FS-non-goals.md#10-interactive-mode)), no network ([§FS-non-goals.11](functional-spec/FS-non-goals.md#11-network-access-during-a-check)).

### 2. Why now

`grund-lsp` ([§RM-lsp](roadmap.md#rm-lsp-ship-the-optional-lsp-server)) covers editor users; `--watch` covers everyone else with zero editor configuration, and it is small now that the engine is a library ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli)). The watcher calls `grund-core::scan`/`check` rather than re-implementing the walk.

### 3. Measurable

An e2e fixture starts `grund check --watch` on a clean fixture (asserts silent first run), writes a file that introduces a dangling ref (asserts the next run prints it), removes the bad citation (asserts the run goes silent again), then sends SIGINT (asserts exit code matches the last run). A second fixture asserts `--format=json` emits one self-contained report per run.

## RM-cochange-gate: a pre-commit / CI recipe — no impl change without spec and test

The strong form of the discipline ([§GOAL-agent-grounding.1](goals.md#1-the-three-layers), diff-gated enforcement): a changed source file must be grounded ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), and the change must also touch the spec it cites *or* a test of it, with an explicit escape hatch for refactors. This is diff-aware — a function of `(tree, base ref, config)`, not `(tree, config)` — and it leans on `grund cover` ([§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)) plus a git diff, so it lives in the recipe layer, **not** in `grund-core` (a third first-party surface is out of scope, [§FS-non-goals.12](functional-spec/FS-non-goals.md#12-surfaces-outside-grund-core-and-the-lsp-transport); the engine reads no history, [§FS-non-goals.6](functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)). Tiering rationale in [§DF-require-grounding](decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec).

[§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) proves files are grounded at rest; it does not prove that a behavior change came with a spec or test update. The co-change gate is therefore the highest-value remaining "agent discipline" item: use `grund cover` plus git diff to connect changed implementation files to the specs and tests that justify the change.

### 1. What

A documented pre-commit hook / CI step (a recipe alongside the `grund check` hook in the README, and a worked example under `examples/`), not a shipped binary. Given a base ref it: (a) lists changed source files; (b) for each, gets its cited IDs from `grund cover` and fails `ungrounded change` if a changed hunk falls under no citation; (c) requires the diff to also touch the declaring file of one of those IDs *or* a test / `§E2E-` case that cites one of them; (d) honours an escape hatch — a commit trailer (e.g. `Grund-Cochange: refactor`) or a `grund:no-cochange` pragma on a hunk — for legitimate refactors, kept greppable so a reviewer sees every waiver. Which paths count as "source" vs. "test", whether (c) needs spec *and* test or *either*, and how the base ref is chosen are knobs the repo sets in the recipe, not in `grund-core` — so the "two installs agree" contract ([§FS-non-goals.13](functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)) and the no-config-on-severity rule ([§FS-non-goals.9](functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)) are untouched.

### 2. Why now

[§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) makes "every file is grounded" true at rest; this makes "every change stays grounded, and ships with a test" true at the diff. It is unsound by construction — without an AST it cannot tell a behavioral hunk from a cosmetic one — so the escape hatch is mandatory and the gate is advisory-strict, not a proof. That trade is the reason it is a recipe a repo opts into, not engine behavior.

### 3. Measurable

The recipe, run in this repo's CI on a synthetic branch, fails a commit that edits a `src/` file without touching its spec or a test, passes the same commit once a `Grund-Cochange:` trailer is added, and passes a commit that edits the spec and a test together. The `examples/` worked example carries golden output the e2e harness can diff.

## RM-declaration-near-miss: warn on a heading that looks like a declaration but does not match `[id] format`

A non-heuristic onboarding aid: a heading shaped like `# <KIND>-…: <title>` whose ID does not match the configured `[id] format` ([§FS-config.3.2](functional-spec/FS-config.md#32-id--id-grammar)) is silently *not* a declaration today — it does not appear in `grund list`, `grund check` says nothing, and `§…` citations of that shape go unrecognised. The classic stumble is writing `# FS-login: …` under the default `{kind}-{number}-{slug}` (forgetting the `-NNN-`). Serves [§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree) and [§GOAL-friendliness-first](goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — a fresh adopter should not have to discover the grammar by its silence.

### 1. What

`grund check` emits one **warning** (not an error — exit code unchanged, [§FS-check.4](functional-spec/FS-check.md#4-warnings)) at `path:line:` for a heading whose first token after `# ` is `<KIND>-…` for a configured `[[kinds]]` prefix but whose remainder fails the `[id] format` regex — message names the heading, the configured format, and the nearest valid shape. This is a fact about the tree, not a guess at intent: it does not propose the corrected ID ([§FS-non-goals.3](functional-spec/FS-non-goals.md#3-code-ast-parsing) / no heuristics, [§GOAL-agent-grounding.3](goals.md#3-what-this-rules-out)), it only points out that something heading-shaped is being ignored. A line-oriented opt-out (a `grund:not-a-declaration` pragma, or a config glob) for files that legitimately use `# <KIND>-…:` headings as prose is in scope if the warning proves noisy.

### 2. Why now

Caught in the 0.1.0 product review as a real sharp edge: the strict-grammar design is correct, but its failure mode is invisible. The warning is the smallest fix that stays inside the no-guessing rule — it surfaces the mismatch and lets the contributor decide.

### 3. Measurable

An e2e fixture with a numbered-format config and a `# FS-login: …` heading gets exactly one warning naming the heading and the format, `grund check` still exits `0`, and a sibling fixture whose heading *does* match the format gets none. `grund list` is unchanged in both (a near-miss heading is still not a declaration).

## RM-positioning: the Lychee contrast and the instruction-count framing in README and landing copy

`grund` lives next to `lychee` in CI, not against it ([§FS-non-goals.1](functional-spec/FS-non-goals.md#1-markdown-link-validation), [§AR-ci.3](architecture/AR-ci.md#3-current-hooks)). The README now says that in product terms — "Lychee is the link checker; `grund` is the intent checker" — and carries the instruction-count-not-stopwatch framing beside the benchmark badge. What remains is pairing that framing with the committed instruction-count baseline once [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets) lands it. This milestone ships words, not code.

### 1. What

Done: a short "vs. a link checker" block in the README:

- Lychee checks whether Markdown links still open; `grund` checks whether your code still knows why it exists.
- Lychee catches dead links; `grund` catches dead grounding.
- Lychee validates the web of pages; `grund` validates the web of intent.
- Lychee says "this URL broke"; `grund` says "this implementation lost its spec."
- Use Lychee for links out; use `grund` for reasons in.
- Lychee guards navigation; `grund` guards meaning.

…landing on the closing line: **Lychee is the link checker; `grund` is the intent checker. Both belong in CI; they guard different failure modes.**

Done: the README states the benchmark framing next to the local throughput badge and names the committed instruction-count baseline from [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets): **`grund` measures performance by instruction count, not stopwatch time — same binary, same repo, same number — which gives CI a stable regression meter instead of a noisy timing guess** ([§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds), [§AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters)).

### 2. Why now

The 0.1.0 product review found the README explained the *mechanism* well and the *pitch* thinly: a reader who already runs `lychee` could not tell in one line what `grund` adds beside it ([§GRUND-grund.1](grund.md#1-what-grund-does-about-it)). The framing is cheap to write and pays off on every landing. It pairs with [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets) because the instruction-count line earns its full place once there is a committed figure to attach it to.

### 3. Measurable

The README (and landing page, if any) carries a "vs. link checkers" block whose closing line is the "link checker / intent checker" pair. The benchmark section states the instruction-count-not-wall-clock framing alongside the committed baseline from [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets). `grund check` stays clean.

## RM-gap-report: orphan and uncovered ID reports

The inverse of [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file): same scan, but instead of "what does this file cite?" it answers "which declared IDs have nothing climbing into them?" Without it `grund` is a navigation tool; with it, `grund` is a traceability tool — the column every comparable requirements tool already has. The framing comparison lives in [§RM-positioning-trace-tools](roadmap.md#rm-positioning-trace-tools-position-grund-against-requirements-traceability-tools-in-readme).

### 1. What

A new read-only command, `grund gap [--kind <K[,...]>] [--format text|json]`, that re-uses the existing citation graph and reports:

- *orphans*: declared IDs with zero inbound citations, ignoring kinds at the top of the climbing chain (`GRUND`, `GOAL` under the default config).
- *unclimbed*: declared IDs whose only inbound citations come from kinds that violate the climbing rule — e.g. an `FS-` that no `AR-`, `E2E-`, or code site cites.

Output is sorted lexicographically by `(kind, id)` for byte-identical reproducibility ([§FS-errors.4](functional-spec/FS-errors.md#4-determinism)). The command never changes its exit code on found gaps — it is a report, not a check; severity/exit-code customization stays out of the engine ([§FS-non-goals.9](functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)). CI use is a recipe (same shape as [§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)): pipe the JSON, gate on the count. Dangling citations are already `grund check` errors and are not duplicated here.

### 2. Why now

[§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) shipped the index but not the inverted view. Every neighbour tool (OFT, Sphinx-Needs, Doorstop, Duvet) ships a "what's uncovered?" report as the centrepiece feature, and on the comparison matrix in [§RM-positioning-trace-tools](roadmap.md#rm-positioning-trace-tools-position-grund-against-requirements-traceability-tools-in-readme) this is the single line that flips `grund` from "fewer features than OFT" to "different axis from OFT, with parity on the obvious one."

### 3. Measurable

E2E fixtures: a clean tree returns no orphans; deleting an `E2E-` that cited an `FS-` makes that `FS-` show up as `unclimbed` in the next `grund gap`. `--format=json` emits one NDJSON record per gap, sorted as above. Run on this repo, `grund gap` is silent (the repo self-hosts the floor).

## RM-positioning-trace-tools: position grund against requirements-traceability tools in README

[§RM-positioning](roadmap.md#rm-positioning-the-lychee-contrast-and-the-instruction-count-framing-in-readme-and-landing-copy) covers Lychee — link checker vs. intent checker. It does not cover the *other* neighbourhood `grund` lives in: dedicated requirements-traceability tools that already do markdown specs, ID citations, and coverage reports. A reader landing on the README from that world (OFT, Sphinx-Needs, TRLC, Doorstop, Duvet, SARA) cannot tell in one line what `grund` adds beside them. This milestone ships positioning copy, not code.

### 1. What

A new "vs. traceability tools" block in the README and landing page, anchored by a compact comparison matrix and three short positioning lines. The matrix:

| Tool | Since | Markdown-native | Inline code citations | Sectioned IDs `§<ID>.3.1` | Resolver CLI `--brief`/`--toc`/`--full` | Coverage report | Single binary |
|---|---|---|---|---|---|---|---|
| **grund** | 2026 | ✅ | ✅ | ✅ | ✅ | ⏳ [§RM-gap-report](roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) | ✅ |
| [OpenFastTrace](https://github.com/itsallcode/openfasttrace) | 2015 | ✅ | ✅ | ❌ | ❌ | ✅ flagship | ❌ JVM |
| [Sphinx-Needs](https://github.com/useblocks/sphinx-needs) | 2017 | ⚠ RST/MyST | ⚠ via refs | ❌ | ⚠ via Sphinx build | ✅ | ❌ Python+Sphinx |
| [TRLC](https://github.com/bmw-software-engineering/trlc) + [LOBSTER](https://github.com/bmw-software-engineering/lobster) | 2022 | ❌ DSL | ✅ | ❌ | ❌ | ✅ | ❌ Python |
| [Doorstop](https://github.com/doorstop-dev/doorstop) | 2013 | ❌ YAML-per-item | ⚠ links only | ❌ | ❌ | ✅ | ❌ Python |
| [Duvet](https://github.com/awslabs/duvet) | 2021 | ⚠ specs only | ✅ | ⚠ anchors | ❌ | ✅ flagship | ✅ |
| [SARA](https://github.com/cledouarec/sara) | 2026 | ✅ + YAML frontmatter | ❌ | ❌ | ⚠ graph queries | ✅ | ✅ |

The positioning lands on three sentences:

- **OFT, Sphinx-Needs, TRLC, Doorstop, Duvet are traceability tools optimized for a coverage report.** `grund` is a *grounding* tool optimized for an agent reading one specific fact: the sectioned `§<ID>.3.1` citation plus the depth-controlled resolver give a model a one-command path to the smallest text that justifies a line of code ([§GOAL-agent-grounding.1](goals.md#1-the-three-layers)).
- **They model each clause as its own atomic item.** `grund` keeps the clause inside the spec it belongs to and lets the citation point at the heading — fewer files to author, cheaper to read in an agent's context window.
- **Coverage parity is one shipping milestone away.** [§RM-gap-report](roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) inverts the [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) index and answers "which IDs are uncovered?" — the column that today reads ⏳ in the matrix above.

A short "we deliberately don't" footnote points at [§FS-non-goals](functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) and names three features the neighbours have that `grund` will not grow: ReqIF / OFT interchange (would import a foreign citation grammar and break the "two installs agree" contract, [§FS-non-goals.13](functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)), schema-level custom check rules (would require severity / exit-code config, [§FS-non-goals.9](functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)), and HTML/PDF publishing (a third first-party surface, [§FS-non-goals.12](functional-spec/FS-non-goals.md#12-surfaces-outside-grund-core-and-the-lsp-transport)).

### 2. Why now

A reader in the requirements-traceability community currently sees `grund` as "another markdown reqs tool, but with fewer features" — because the README does not name the axis on which `grund` is actually different (sectioned citations + agent-readable resolver, not coverage reports). Writing the positioning before [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) ships gets the framing right before that audience arrives via npm and PyPI. Pairs with [§RM-positioning](roadmap.md#rm-positioning-the-lychee-contrast-and-the-instruction-count-framing-in-readme-and-landing-copy): one block for the "I already run a link checker" reader, one for the "I already run OFT" reader.

### 3. Measurable

The README (and landing page, if any) carries a "vs. traceability tools" section whose matrix names the six tools above with creation year, whose capability columns include the sectioned-citation row, and whose closing sentence is the "traceability tool / grounding tool" pair. The "we deliberately don't" footnote names the three rejected features with [§FS-non-goals](functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) pointers. `grund check` stays clean.

## Shipped milestones

Done milestones leave their full record in `docs/changelog.md` (the `Implemented` block of the latest release). They keep a one-line declaration here so existing `§RM-…` citations still resolve — the changelog has the detail.

## RM-core-cli-split: split grund-core from grund-cli

Shipped. The root manifest is now a virtual workspace, `crates/grund-core` is the shared engine crate, `crates/grund-cli` is the Cargo package named `grund`, and `grund-core` exposes data-returning APIs for the CLI surfaces (`check`, `show`, `refs`, `list`, `cover`, `fmt`, `id`, `init`, dynamic completions, and config inspection) while the CLI owns parsing, rendering, and exit-code mapping — see `docs/changelog.md`, [§AR-bindings](architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms), and [§FS-distribution.3.1](functional-spec/FS-distribution.md#31-rust-grund-core-crate).

### 3. Measurable

The public embedding smoke test calls the `grund-core` APIs directly — no CLI argument parser, no stdout renderer — and obtains the same data shape the CLI later renders. The CLI e2e harness exercises the dedicated `crates/grund-cli` frontend and keeps byte-identical reports. `crates/grund-cli` imports no `grund_core::command_*` symbol; any remaining core command adapters are private compatibility glue for the deprecated `grund_core::main_entry()` path, not the frontend boundary.

## RM-benchmarks: a benchmark harness for the §GOAL-fast-feedback budgets

Shipped. The instruction-counting `cargo bench` harness now covers this repo plus a generated 10k-file conformant fixture, `docs/benchmarks.md` records the baseline, and pull-request CI fails on a >5% Callgrind instruction-count regression — see `docs/changelog.md`, [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands), [§AR-ci.5](architecture/AR-ci.md#5-benchmark-job), and [§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds).

## RM-parallel-scan: parallel per-file scanning for large-repo throughput

Shipped. Large sorted file lists and workspace projects now scan in parallel while merging findings in deterministic path order — see `docs/changelog.md` and [§AR-scanner](architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations).

## RM-self-host: guard the self-host loop in CI

Shipped. CI self-checks this repository, the host scan ignores fixture repos, and the e2e corpus covers nested fixture directories under the canonical default config — see `docs/changelog.md` and [§AR-scanner.6](architecture/AR-scanner.md#6-e2e-case-declarations).

## RM-init-workspace-members: `init` mentions workspace members

Shipped. `grund init` now emits a `### Workspace members` section in the generated `AGENTS.md` whenever the effective config declares `[workspace]`, listing every resolved project (root + members) sorted by alias, with one discoverability line and a `*(not yet initialized)*` marker for members whose `AGENTS.md` does not yet exist — see `docs/changelog.md`, [§FS-init.2.3.4.15](functional-spec/FS-init.md#23415-workspace-members), and [§DISC-init-workspace-members](discussions/proposals/2026-05-17-init-workspace-members.md#disc-init-workspace-members-have-init-mention-workspace-members).

## RM-require-grounding: the opt-in grounding floor

Shipped. `[reference] require_grounding` (and `grund check --require-grounding`), the `ungrounded source file` error class, the inline-declaration exemption, Markdown skipped — see `docs/changelog.md`, [§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in), [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form), and [§DF-require-grounding](decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec). The diff-aware co-change recipe is [§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test).

## RM-e2e-corpus: the e2e/cases/* corpus and CI harness

Shipped. The `e2e/cases/*` corpus, `tests/e2e.rs`, `tests/init.rs`, the per-error-class fixtures, and the byte-for-byte determinism sweep — see `docs/changelog.md`.

## RM-show: ID-query reads

Shipped. Lead-default declaration reads, `--brief`, `--toc`, `--section` and dotted-inline section paths, `--full`, `--format text|md|json`, inline-source extraction, ambiguous-ID / broken-stub query forms — see `docs/changelog.md` and [§FS-show](functional-spec/FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id).

## RM-token-cheap-grounding: token-cheap read surfaces for agents

Shipped. The lead-default `grund <ID>` read, `grund <ID> --brief`, `grund <ID> --toc`, `grund refs --summary`, multi-kind `grund list --kind FS,AR`, `grund list --summary`, and the generated `AGENTS.md` guidance block — see `docs/changelog.md`, [§FS-show.2.1](functional-spec/FS-show.md#21-whole-declaration-default), [§FS-show.2.1.1](functional-spec/FS-show.md#211-brief---brief), [§FS-show.2.1.2](functional-spec/FS-show.md#212-section-map---toc), [§FS-refs.3.3](functional-spec/FS-refs.md#33---summary), [§FS-list.3.3](functional-spec/FS-list.md#33---summary), and [§DF-show-default-token-cheap](decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in).

## RM-config: .agents/grund.toml discovery, parsing, and inspection

Shipped. The line-oriented TOML subset, unknown-key errors with `path:line:` pointers, `grund_config_version` gating, every documented block, plus `grund config validate` / `grund config show` — see `docs/changelog.md` and [§FS-config](functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents).

## RM-marker-fmt: the § marker, the $$ trigger, and grund fmt

Shipped. `grund fmt` with `--check` / `--write` / `--marker`, the deterministic string-literal carve-out, declaration-heading and fenced-block exemptions, and `[reference] strict = true` — see `docs/changelog.md`, [§FS-fmt](functional-spec/FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk), and [§DF-reference-marker](decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger).

## RM-md-link-emission: grund fmt --cross-refs

Shipped. Wraps marker-prefixed citations in `.md` files only, heading-text anchor slugs per the `github` / `gitlab` / `mkdocs` / `pandoc` / `none` profiles, re-derived each pass, fenced/inline-code-span and dangling-citation skips, idempotent — see `docs/changelog.md`, [§FS-fmt.6](functional-spec/FS-fmt.md#6-cross-reference-emission), and [§DF-md-link-anchor-strategy](decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass).

## RM-refs: grund refs <ID>

Shipped. Over the same scan as `check`, sorted by `(path, line, column)`, honouring strict mode and the string-literal carve-out, NDJSON on stdout for `--format=json` — see `docs/changelog.md` and [§FS-refs](functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id).

## RM-cover: grund cover

Shipped. Groups the scanner's citation graph by file, emits text on stdout or one JSON record per scanned file, includes files with zero citations, and stays git/policy-free for the co-change recipe — see `docs/changelog.md` and [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file).
