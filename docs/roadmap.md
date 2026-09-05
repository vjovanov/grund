# Roadmap

What `grund` plans to ship next, in priority order. Each item has a stable ID — `RM-<slug>` under this repo's `[id] format` ([§FS-config.3.2](functional-spec/FS-config.md#32-id--id-grammar)); `RM` is a configured `[[kinds]]` prefix ([§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-kinds)), so `grund check` validates `§RM-…` citations like any other. Items may be cited from anywhere — commits, PRs, the changelog, other specs. A shipped item is removed: its record is the changelog and the spec it landed in, and whatever cited the milestone cites that spec instead. A cancelled item stays in place with a `~~strikethrough~~` title and a one-line reason. Where an item has a GitHub issue, the item names it.

The check engine, the retrieval surface (`grund <ID>`, `grund refs`, including E2E case manifests), the coverage index (`grund cover`), bulk normalization (`grund fmt`, including `--marker` and `--cross-refs`), config loading (`grund.toml` plus `grund config show` / `grund config validate`), `grund init`, `grund id`, the opt-in grounding floor ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), the token-cheap read surfaces ([§DF-show-default-token-cheap](decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in)), the e2e corpus, the benchmark baseline/gate ([§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands)), the live registry-name guard ([§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process)), the `grund-core` / `grund-cli` workspace split with data-returning core APIs ([§AR-bindings.2](architecture/AR-bindings.md#2-grund-core-the-only-place-logic-lives)), the optional Cargo LSP server ([§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)), and parallel per-file scanning ([§AR-scanner.1](architecture/AR-scanner.md#1-tree-walk)) are all shipped — see `docs/changelog.md`. Two arcs remain. The **distribution arc**: publish on npm and PyPI alongside cargo, including the npm/PyPI LSP packages, and add `grund check --watch`. And the **grounding arc** (the third layer of [§GOAL-agent-grounding.1](goals.md#1-the-three-layers), diff-gated enforcement): build on [§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) and [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) toward a diff-aware co-change gate — implementation cannot change without the spec it grounds in and without a test of it — via a pre-commit / CI recipe that consumes `grund cover` ([§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)). Six standalone items sit outside both arcs: [§RM-doc-comment-declarations](roadmap.md#rm-doc-comment-declarations-declarations-only-in-classmethod-doc-comments) tightens code-declaration recognition so a declaration is only seen inside a class/method doc-comment and never a plain inline comment, [§RM-lsp-completion-tab](roadmap.md#rm-lsp-completion-tab-lsp-id-autocomplete-accepted-with-tab) adds LSP ID completion that works with editor Tab acceptance, [§RM-lsp-trigger-conversion-fix](roadmap.md#rm-lsp-trigger-conversion-fix-fix-the-lsp-trigger-conversion) fixes the LSP `$$` trigger conversion, [§RM-positioning](roadmap.md#rm-positioning-the-lychee-contrast-and-the-instruction-count-framing-in-readme-and-landing-copy) keeps the README/landing pitch paired with the benchmark story, [§RM-gap-report](roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports) inverts the [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) index into an orphan / uncovered-ID report, and [§RM-positioning-trace-tools](roadmap.md#rm-positioning-trace-tools-position-grund-against-requirements-traceability-tools-in-readme) extends the README positioning to the requirements-traceability neighbourhood (OFT, Sphinx-Needs, TRLC, Doorstop, Duvet, SARA). Two deadline items, [§RM-workspace-absorbed-scan-error](roadmap.md#rm-workspace-absorbed-scan-error-flip-the-absorbed-scan-warning-to-an-error) and [§RM-unlisted-workspace-error](roadmap.md#rm-unlisted-workspace-error-flip-the-unlisted-workspace-warning-to-an-error), expire deprecation ramps at the release their warnings already name. The IDed milestones below project both arcs onto reviewable units of work.

## RM-distribution: cargo + npm + pypi from one engine

Per [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) and [§AR-bindings](architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms). Builds on the shipped workspace split ([§AR-bindings.2](architecture/AR-bindings.md#2-grund-core-the-only-place-logic-lives)).

### 1. What

napi-rs binding for npm; PyO3 binding for PyPI; CI publish jobs for all three registries (`grund-core` first, in dependency order). Each publish job builds the CLI binary with profile-guided optimization via `scripts/pgo-build.sh` ([§DA-pgo-release](decisions/architectural/DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload), [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process)) — wired for the crates.io `grund` and `grund-lsp` packages already, extended to the prebuilt npm and PyPI CLI/LSP binaries here.

### 2. Why now

`grund` is only viable as a CI dependency for non-Rust projects once it ships on their native package manager ([§GOAL-multi-language](goals.md#goal-multi-language-same-engine-three-platforms)).

### 3. Measurable

Integration test runs the same spec corpus through all three bindings and asserts byte-identical reports ([§AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters)).

## RM-watch: implement grund check --watch

Per [§FS-check.6](functional-spec/FS-check.md#6-watch-mode---watch). The editor-less "every save" loop [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) exists for — re-run `grund check` on every change under the scanned tree, clearing prior output each run.

Together with [§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server), this ships the live feedback loop: LSP for editor users, `grund check --watch` for terminal users and editor setups that do not speak LSP.

### 1. What

`--watch` on `grund check`: filesystem-notification-driven, debounced, no polling and no configurable interval. Each run is byte-identical to a plain `grund check` on the tree's current state; on Ctrl-C the process exits with the last completed run's exit code. Non-interactive — no TUI, no key bindings ([§FS-non-goals.10](functional-spec/FS-non-goals.md#10-interactive-mode)), no network ([§FS-non-goals.11](functional-spec/FS-non-goals.md#11-network-access-during-a-check)).

### 2. Why now

`grund-lsp` ([§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)) covers editor users; `--watch` covers everyone else with zero editor configuration, and it is small now that the engine is a library ([§AR-bindings.2](architecture/AR-bindings.md#2-grund-core-the-only-place-logic-lives)). The watcher calls `grund-core::scan`/`check` rather than re-implementing the walk.

### 3. Measurable

An e2e fixture starts `grund check --watch` on a clean fixture (asserts silent first run), writes a file that introduces a dangling ref (asserts the next run prints it), removes the bad citation (asserts the run goes silent again), then sends SIGINT (asserts exit code matches the last run). A second fixture asserts `--format=json` emits one self-contained report per run.

## RM-lsp-completion-tab: LSP ID autocomplete accepted with Tab

Implements the reserved `textDocument/completion` capability from [§FS-lsp.1.5](functional-spec/FS-lsp.md#15-capabilities-reserved-for-later), building on the shipped LSP server [§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server). The result is the expected editor loop: type a marker or trigger prefix, narrow to the ID, accept the selected completion with the editor's normal Tab binding, and get the canonical citation inserted.

### 1. What

`grund-lsp` returns completion items for declared IDs in the document's resolved project config. Completion triggers include the configured marker prefix (`§F` under defaults), the configured typing trigger prefix (`$$F` under defaults), and a partially typed ID immediately after either prefix. Applying a completion replaces only the active prefix/token range with the configured marker plus the chosen ID; it does not touch surrounding prose, existing markdown links, or another citation on the same line. Completion details show the declaration title and source path, and items sort by exact prefix match, then kind order, then ID for deterministic output ([§FS-errors.4](functional-spec/FS-errors.md#4-determinism)). The server cannot own each client's Tab key, so the contract is that completion items use plain text edits and ranges that work with standard Tab acceptance in Helix, Neovim, Zed, VSCode, and eglot/lsp-mode; README snippets add the client-side Tab mapping only where the editor requires it.

### 2. Why now

The shipped LSP already gives diagnostics, navigation, hover, links, and trigger formatting ([§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)). The remaining daily friction is remembering exact IDs while writing a citation. Completion turns the LSP from a checker into an authoring aid without changing the CLI contract.

### 3. Measurable

LSP tests open a fixture workspace, request completions after `§F`, `$$F`, and a longer prefix, and assert the returned labels, sort order, and text-edit ranges. Applying the edit produces exactly one canonical `§<ID>` citation, `grund check` resolves it, and the same fixture covers a workspace member with a non-default marker/trigger.

## RM-lsp-trigger-conversion-fix: fix the LSP trigger conversion

Fixes and hardens the shipped live trigger transform [§FS-lsp.1.4](functional-spec/FS-lsp.md#14-live-trigger-transform). The existing LSP milestone is shipped ([§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)), but the `$$` authoring path needs to be reliable before completion and normal editing can depend on it.

### 1. What

`textDocument/onTypeFormatting` converts the configured typing trigger (`$$` by default) to the configured marker only when the text after the trigger is a valid citation start for the document's resolved config. It must handle the common typing paths: `$$FS-foo` typed continuously, `$$` followed by a completion choice, trigger text at the start of a line, trigger text inside a comment, and trigger text next to another citation on the same line. The returned edit is minimal, UTF-16-correct, idempotent, and never rewrites literal money/prose `$$` that is not followed by a recognized ID prefix. Workspace-member config and non-default markers/triggers follow the same lookup path as `grund fmt` and diagnostics ([§FS-workspace.5](functional-spec/FS-workspace.md#5-command-scope)).

### 2. Why now

[§FS-lsp.1.4](functional-spec/FS-lsp.md#14-live-trigger-transform) is what makes `§` practical to type without leaving the keyboard. If the conversion is flaky, the LSP's most basic authoring workflow feels broken even when diagnostics and navigation are correct.

### 3. Measurable

Focused LSP tests cover continuous typing, completion-adjacent typing, line-start and comment positions, UTF-16 ranges, adjacent citations, non-default trigger/marker config, and negative `$$` prose cases. The same fixture should pass `grund fmt --check` after applying the LSP edit, proving live conversion and bulk normalization agree.

## RM-cochange-gate: a pre-commit / CI recipe — no impl change without spec and test

The strong form of the discipline ([§GOAL-agent-grounding.1](goals.md#1-the-three-layers), diff-gated enforcement): a changed source file must be grounded ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), and the change must also touch the spec it cites *or* a test of it, with an explicit escape hatch for refactors. This is diff-aware — a function of `(tree, base ref, config)`, not `(tree, config)` — and it leans on `grund cover` ([§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)) plus a git diff, so it lives in the recipe layer, **not** in `grund-core` (a third first-party surface is out of scope, [§FS-non-goals.12](functional-spec/FS-non-goals.md#12-surfaces-outside-grund-core-and-the-lsp-transport); the engine reads no history, [§FS-non-goals.6](functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)). Tiering rationale in [§DF-require-grounding](decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec).

GitHub: [#26](https://github.com/vjovanov/grund/issues/26).

[§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) proves files are grounded at rest; it does not prove that a behavior change came with a spec or test update. The co-change gate is therefore the highest-value remaining "agent discipline" item: use `grund cover` plus git diff to connect changed implementation files to the specs and tests that justify the change.

### 1. What

A documented pre-commit hook / CI step (a recipe alongside the `grund check` hook in the README, and a worked example under `examples/`), not a shipped binary. Given a base ref it: (a) lists changed source files; (b) for each, gets its cited IDs from `grund cover` and fails `ungrounded change` if a changed hunk falls under no citation; (c) requires the diff to also touch the declaring file of one of those IDs *or* a test / `§E2E-` case that cites one of them; (d) honours an escape hatch — a commit trailer (e.g. `Grund-Cochange: refactor`) or a `grund:no-cochange` pragma on a hunk — for legitimate refactors, kept greppable so a reviewer sees every waiver. Which paths count as "source" vs. "test", whether (c) needs spec *and* test or *either*, and how the base ref is chosen are knobs the repo sets in the recipe, not in `grund-core` — so the "two installs agree" contract ([§FS-non-goals.13](functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)) and the no-config-on-severity rule ([§FS-non-goals.9](functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)) are untouched.

### 2. Why now

[§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) makes "every file is grounded" true at rest; this makes "every change stays grounded, and ships with a test" true at the diff. It is unsound by construction — without an AST it cannot tell a behavioral hunk from a cosmetic one — so the escape hatch is mandatory and the gate is advisory-strict, not a proof. That trade is the reason it is a recipe a repo opts into, not engine behavior.

### 3. Measurable

The recipe, run in this repo's CI on a synthetic branch, fails a commit that edits a `src/` file without touching its spec or a test, passes the same commit once a `Grund-Cochange:` trailer is added, and passes a commit that edits the spec and a test together. The `examples/` worked example carries golden output the e2e harness can diff.

## RM-doc-comment-declarations: declarations only in class/method doc-comments

Per [§DISC-doc-comment-declarations](discussions/proposals/2026-05-21-doc-comment-declarations.md#disc-doc-comment-declarations-declarations-live-only-in-classmethod-doc-comments-never-inline). Tightens the [§AR-scanner.4](architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) recognizer so a code-resident declaration is seen only inside a doc-comment that documents the immediately-following definition (class, method, module, …), never a plain inline or trailing comment — with a default-on `[scan]` switch that restores today's any-comment behavior. Composes with [§FS-check.4.6](functional-spec/FS-check.md#46-declaration-near-miss): the gate drops the phantom declaration, the near-miss optionally surfaces "this looks like a declaration but is ignored." The marker/position classifier this milestone planned already exists — `comment_block.rs` decides doc comment or inline comment per block for the inline citation sites of [§FS-inline-citation-style.1.1](functional-spec/FS-inline-citation-style.md#11-doc-comments-are-not-sites) — so the declaration gate reuses it rather than building a second one.

### 1. What

The declaration recognizer splits the comment-prefix alternation in two: a *declaration-prefix* set holding only doc-comment markers (selected per file extension), and the existing any-comment set that citations keep using unchanged. For *marker* languages (Rust `///`/`//!`, Java/JS/TS `/** */`, C# `///`, …) a declaration is recognized only behind the doc marker; a bare `//`/`/* */` is a regular comment and never declares — closing the `//[/!]?` widening in `grammar.rs` that lets a plain `//` line declare today. For *position* languages where the regular marker is also the doc marker (Go `//`, Ruby `#`, …), the declaration is emitted only when its comment block is immediately followed by a line-anchored definition-starter (`func`, `class`, `def`, …) — recognition, not parsing ([§FS-non-goals.3](functional-spec/FS-non-goals.md#3-code-ast-parsing), [§AR-scanner](architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)). Python docstrings ([§AR-scanner.4](architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)) are unchanged. A new `[scan] declarations_in_doc_comments` key (default `true`, [§FS-config.3.5](functional-spec/FS-config.md#35-scan--what-gets-walked)) restores the legacy any-comment recognizer when set `false`; it is a recognizer toggle, not a severity knob ([§FS-non-goals.9](functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)), so the two-installs-agree contract ([§FS-non-goals.13](functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)) holds. Citations are untouched: a `§<ID>` in any comment still resolves and climbs.

### 2. Why now

Surfaced from a real LSP session: a citation note written in a plain `//` comment with the `§` marker dropped (`// FS-check.3.9 / …`) is silently read as a declaration of `FS-check` *inside a function body*, colliding with the real spec declaration and raising a `duplicate declaration` diagnostic. The recognizer is looser than [§AR-scanner.4](architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)'s own framing, which already says an inline declaration lives in "the class, method, module, or package doc-comment." This realigns the recognizer with the spec and removes a sharp edge that bites authors and agents who write inline citation notes — directly serving [§GOAL-friendliness-first](goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) and [§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree) (default-on, no config).

### 3. Measurable

E2E fixtures: a plain `//` ID note inside a function body is *not* a declaration (`grund list` does not show it, no `duplicate declaration`); the same file with the switch off restores the declaration; a Rust `///` declaration is still recognized; a Go `//` block immediately above `func`/`type` is recognized while a Go `//` note not above a definition is not. The recognizer holds its shape across all three bindings ([§GOAL-multi-language](goals.md#goal-multi-language-same-engine-three-platforms)). Run on this repo (after the two `//` Rust fixtures move to `///`), `grund check` stays clean.

## RM-positioning: the Lychee contrast and the instruction-count framing in README and landing copy

`grund` lives next to `lychee` in CI, not against it ([§FS-non-goals.1](functional-spec/FS-non-goals.md#1-markdown-link-validation), [§AR-ci.3](architecture/AR-ci.md#3-current-hooks)). The README says so in mechanism — a `§`-marked citation in a Rust file is invisible to a link checker — and not yet in product terms: the "Lychee is the link checker; `grund` is the intent checker" pair and the instruction-count-not-stopwatch framing beside the benchmark badge are still to be written, and the committed instruction-count baseline they attach to exists now ([§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands)). This milestone ships words, not code.

### 1. What

A short "vs. a link checker" block in the README:

- Lychee checks whether Markdown links still open; `grund` checks whether your code still knows why it exists.
- Lychee catches dead links; `grund` catches dead grounding.
- Lychee validates the web of pages; `grund` validates the web of intent.
- Lychee says "this URL broke"; `grund` says "this implementation lost its spec."
- Use Lychee for links out; use `grund` for reasons in.
- Lychee guards navigation; `grund` guards meaning.

…landing on the closing line: **Lychee is the link checker; `grund` is the intent checker. Both belong in CI; they guard different failure modes.**

And the benchmark framing next to the local throughput badge, naming the committed instruction-count baseline of [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands): **`grund` measures performance by instruction count, not stopwatch time — same binary, same repo, same number — which gives CI a stable regression meter instead of a noisy timing guess** ([§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds), [§AR-goal-measurement.2](architecture/AR-goal-measurement.md#2-goal-meters)).

### 2. Why now

The 0.1.0 product review found the README explained the *mechanism* well and the *pitch* thinly: a reader who already runs `lychee` could not tell in one line what `grund` adds beside it ([§GRUND-grund.1](grund.md#1-what-grund-does-about-it)). The framing is cheap to write and pays off on every landing. It pairs with [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) because the instruction-count line earns its full place now that there is a committed figure to attach it to.

### 3. Measurable

The README (and landing page, if any) carries a "vs. link checkers" block whose closing line is the "link checker / intent checker" pair. The benchmark section states the instruction-count-not-wall-clock framing alongside the committed baseline from [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands). `grund check` stays clean.

## RM-gap-report: orphan and uncovered ID reports

The inverse of [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file): same scan, but instead of "what does this file cite?" it answers "which declared IDs have nothing climbing into them?" Without it `grund` is a navigation tool; with it, `grund` is a traceability tool — the column every comparable requirements tool already has. The framing comparison lives in [§RM-positioning-trace-tools](roadmap.md#rm-positioning-trace-tools-position-grund-against-requirements-traceability-tools-in-readme).

The orphan half already exists as `grund list --unused` ([§FS-list](functional-spec/FS-list.md#fs-list-grund-lists-every-declared-id)), which lists the declarations nothing cites; what remains is the *unclimbed* view and the report shape below. GitHub: [#89](https://github.com/vjovanov/grund/issues/89) asks for the deliberately-uncited marker that lets that list be driven to zero.

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

## RM-obligation-no-unit: warn when a citation-direction obligation applies to nothing

An obligation attaches to a unit — a declaration for a citable kind, a citation-carrying scanned file for a non-citable home ([§FS-config.3.9.1](functional-spec/FS-config.md#391-levels), [§FS-check.3.11](functional-spec/FS-check.md#311-missing-required-citation)) — so a kind that yields no unit yields no finding, and `must` passes vacuously while the entrypoint keeps advertising the rule. [§DF-non-citable-kinds.2.5](decisions/functional/DF-non-citable-kinds.md#25-obligations-get-a-per-file-unit-and-grounding-follows-the-home) closed this for non-citable kinds by giving them a per-file unit, and [§FS-config.3.4.7](functional-spec/FS-config.md#347-scan--a-place-that-is-listed-not-walked) refuses a rule on an unwalked citing kind at load time; the case between them — a citable kind with a walked home and nothing declared in it — is still open. Seen on a real adoption: a `skills/` kind declared citable, eleven files, no `SKILL` ID, six files citing nothing, `grund check` green.

GitHub: [#149](https://github.com/vjovanov/grund/issues/149).

### 1. What

One CLI-level `warning:` on stderr ([§FS-check.2.1.1](functional-spec/FS-check.md#211-cli-level-messages)), exit code untouched — the class [§FS-check.2.2](functional-spec/FS-check.md#22-empty-scan) uses for a walk that read nothing — when a `[citations.<kind>]` table carries a `must` or `should` entry, the citing kind has a folder home holding at least one scanned file other than its entry file (the configured `index`, or `README.md` where there is none), and the scan produced zero units for it. Two message shapes: the citable kind that declares nothing, and the non-citable home in which no scanned file carries a citation while grounding is off. Single-file kinds and the homeless kind never warn.

### 2. Why now

It is the smallest change in this group and the only one that came from a verified failure rather than a preference: a green verdict over a rule the maintainer believed was enforced ([§GOAL-no-silent-breakage](goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)). The entry-file cut is what makes it safe to ship without a flag — `grund init --docs` leaves every home holding exactly its entry file and zero declarations, and that tree must stay silent.

### 3. Measurable

Four e2e cases: a citable folder kind with files beside its index and no declaration warns and exits as before; the non-citable mirror warns; the scaffold tree with the canonical `[citations]` ruleset does not; a single-file kind stub does not.

## RM-directions-one-source: one source for the citation-directions explanation

`[citations]` is specified in [§FS-config.3.9](functional-spec/FS-config.md#39-citations--citation-direction-rules) and rendered per [§FS-init.2.3.5](functional-spec/FS-init.md#235-citation-directions), and explained on none of the three surfaces a person or agent reads during setup: the `grund-init` skill walks every config section except this one, against the "pros and cons for every config option" [§FS-init.5](functional-spec/FS-init.md#5-agent-setup-instructions) asks for; the `grund.toml` template comment shows nine example rules and never states that entries in one array are all required while `|` inside an entry is any one of them; the README states two directions inside a table cell. Nowhere is a config shown beside the bullet it becomes.

GitHub: [#148](https://github.com/vjovanov/grund/issues/148).

### 1. What

One page, `docs/user-facing/citation-directions.md`: the levels, the grammar stated once, who may cite and be cited, the no-unit trap, and an example config beside its render. Every other face is a checked copy or a checked render of it, on the two precedents the repository already has: the skill carries it between markers and `test_asset_sync.py` compares the region byte for byte, so `grund agent-setup-instructions` prints it too; the example render is a unit test through the code `grund init` uses — the [§DF-citation-directions.2.7](decisions/functional/DF-citation-directions.md#27-generated-agent-entrypoint-section-with-a-drift-check) drift check turned on the documentation; the README links the page. The template comment and the no-config entrypoint sentence gain the grammar line and the path.

### 2. Why now

Reading `must = ["FS|AR|TCK|CI|METADATA|TESTS"]` in a real config, grund's own author asked whether it should be a plain list — and a plain list would have meant "cite all six". Three surfaces explaining this independently would drift the way the skill and its embedded copy would have without the sync test.

### 3. Measurable

The sync test fails when the skill's marked region and the page differ by a byte; the render test fails when the page's example block and the live render differ; `grund check --full` over this repository stays green.

## RM-workspace-absorbed-scan-error: flip the absorbed-scan warning to an error

[§FS-check.4.7](functional-spec/FS-check.md#47-a-workspace-member-swallows-the-blocks-own-scan) ships as a warning that names the release it becomes an error in, which is the deprecation path [§REQ-backwards-compatibility.2](requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) requires of a finding no command can fix ([§DF-absorbed-scan-warning.2.1](decisions/functional/DF-absorbed-scan-warning.md#21-a-warning-because-the-repair-is-a-judgement-rather-than-a-command)). The named release is half a contract until it happens: a deadline `grund` prints to every user and then lets slip is worse than one it never printed. This milestone is that release.

GitHub: follows [#78](https://github.com/vjovanov/grund/issues/78), which shipped the warning.

### 1. What

Raise the finding from a load-time `warning:` to the config error [§FS-config.4.3](functional-spec/FS-config.md#43-invalid-config-behavior) already defines for a bad `members` line — the same anchor, the same entry-as-written naming, exit `2` — trade the deadline clause for the past-tense report of the release the flip lands in ([§FS-distribution.4.2](functional-spec/FS-distribution.md#42-a-release-may-not-contradict-the-releases-the-trees-own-messages-name)), and delete the release constant with it. [§FS-workspace.2.1](functional-spec/FS-workspace.md#21-a-member-that-swallows-the-blocks-own-scan) keeps its rule and its four exclusions unchanged; only the severity sentence at its end moves, and [§FS-check.4.7](functional-spec/FS-check.md#47-a-workspace-member-swallows-the-blocks-own-scan) moves from §4 to §3. [§DF-absorbed-scan-warning](decisions/functional/DF-absorbed-scan-warning.md#df-absorbed-scan-warning-a-scan-its-own-members-swallowed-is-a-warning-with-a-named-release-not-an-error) gains a consequence line rather than a rewrite: what it decided — which ramp applied, and why the sibling containment case was an error from the start — stays true once the ramp completes.

The two-release window is the point of the ramp, so this is not a milestone to pull forward: a repository that upgrades on the day of the flip must have had a release in which the warning told it what was coming, and told it in a run rather than in release notes.

### 2. Why now

The e2e goldens carry the release as literal bytes and a unit test asserts it is still ahead of `CARGO_PKG_VERSION`, so the version bump that reaches the deadline fails CI — the same forcing function [§DF-index-compatibility-ramp.2.3](decisions/functional/DF-index-compatibility-ramp.md#23-both-findings-name-their-versions-and-a-test-keeps-the-names-honest) states, and the one that carried the `[[kinds]] prefix` removal ([§FS-config.3.4.6](functional-spec/FS-config.md#346-prefix-the-former-spelling-of-kind-removed-in-0130)) and the missing-index-entry flip ([§FS-check.3.18](functional-spec/FS-check.md#318-declaration-missing-from-its-kinds-index)) to the release each named, for the same reason: a deadline that can pass quietly is not a deadline. The release path asks the same question of every message it ships, in both directions ([§FS-distribution.4.2](functional-spec/FS-distribution.md#42-a-release-may-not-contradict-the-releases-the-trees-own-messages-name)).

### 3. Measurable

A `[workspace]` block whose members cover every one of its walk roots fails to load with a `members`-line error and exits `2` on every walking command, no message in the tree still promises a release the running binary has reached, and `grund check --full` over this repository stays green. The e2e cases that pin the warning today (`workspace-member-absorbs-scan-list`, `workspace-member-absorbs-scan-check`, `workspace-member-absorbs-scan-full`, `workspace-member-absorbs-scan-glob-symlink`, `workspace-member-absorbs-scan-nested`) move to `expected.exit` `2`, and the four that pin its boundaries stay green untouched.

## RM-unlisted-workspace-error: flip the unlisted-workspace warning to an error

[§FS-check.4.8](functional-spec/FS-check.md#48-unlisted-workspace-block) ships as a warning that names the release it becomes an error in, which is the deprecation path [§REQ-backwards-compatibility.2](requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) requires of a finding no command can fix — neither remedy is one `grund` writes, because choosing between them is deciding whether the subtree is one of ours ([§DF-unlisted-workspace-block.2.1](decisions/functional/DF-unlisted-workspace-block.md#21-a-warning-that-names-the-release-it-becomes-an-error-in)). A named release that does not happen is a deadline `grund` told every user and then let slip. This milestone is that release.

GitHub: follows [#72](https://github.com/vjovanov/grund/issues/72), which shipped the warning.

### 1. What

Move `unlisted-workspace-block` from the walk's warnings to `report.errors` in `check`, trade the deadline clause for the past-tense report of the release the flip lands in ([§FS-distribution.4.2](functional-spec/FS-distribution.md#42-a-release-may-not-contradict-the-releases-the-trees-own-messages-name)), and delete the ramp constant beside the rule. [§FS-check.4.8](functional-spec/FS-check.md#48-unlisted-workspace-block) loses its ramp paragraph and moves to [§FS-check.3](functional-spec/FS-check.md#3-errors-detected); [§DF-unlisted-workspace-block](decisions/functional/DF-unlisted-workspace-block.md#df-unlisted-workspace-block-an-unlisted-workspace-block-is-reported-by-the-walk-that-meets-it) gains a consequence line rather than being rewritten, because what it decides is which ramp applied and that stays true once the ramp completes.

The other five surfaces keep the warning shape they have: `list`, `refs`, `cover`, `fmt`, and the ID read have no error channel for a fact about the run, and the exit-code mapping [§FS-cli.5](functional-spec/FS-cli.md#5-exit-code-mapping-is-fixed) freezes is not theirs to move. The flip is `check`'s verdict, which is where the guarantee is gated.

The two-release window is the whole point of the ramp, so this is not a milestone to pull forward: a repository that upgrades on the day of the flip must have had a release in which the warning told it what was coming.

### 2. Why now

The deadline unit test asserts the release named in the warning is still ahead of `CARGO_PKG_VERSION`, so the version bump that reaches it fails CI — the same forcing function [§DF-index-compatibility-ramp.2.3](decisions/functional/DF-index-compatibility-ramp.md#23-both-findings-name-their-versions-and-a-test-keeps-the-names-honest) states, and the one that carried the `[[kinds]] prefix` removal ([§FS-config.3.4.6](functional-spec/FS-config.md#346-prefix-the-former-spelling-of-kind-removed-in-0130)) and the missing-index-entry flip ([§FS-check.3.18](functional-spec/FS-check.md#318-declaration-missing-from-its-kinds-index)) to the release each named, for the same reason: a deadline that can pass quietly is not a deadline. The release path stops on it rather than shipping a warning whose date has passed ([§FS-distribution.4.2](functional-spec/FS-distribution.md#42-a-release-may-not-contradict-the-releases-the-trees-own-messages-name)).

### 3. Measurable

`grund check` at the root of a tree holding a `[workspace]` block no enclosing block lists exits `1`, not `0`; the same tree with the block listed as a member, or excluded from `[scan]`, still exits `0`; and no message in the tree still promises a release the running binary has reached. The e2e cases that pin the warning today move to `expected.exit` `1` where the warning was their only finding, and `grund check --full` over this repository stays green.
