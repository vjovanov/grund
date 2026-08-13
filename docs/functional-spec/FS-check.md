# FS-check: grund validates every reference in a repo

The `check` command walks a repo and reports every violation of the grund reference scheme. Validation is explicit as `grund check [<path>]`; the bare `grund <ID>` default belongs to [§FS-show.1](FS-show.md#1-inputs). Serves [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) and [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible).

## 1. Inputs

- Optional path argument; defaults to the current directory. May be a directory or a single file (`grund check crates/grund-core/src/scanner.rs` scopes the scan to one file but still discovers the `grund.toml` by walking up — [§FS-config.1](FS-config.md#1-file-location-and-discovery)).
- The walked tree may contain markdown (`.md`) and source files (Rust, Go, Java, TS, Python, etc.).
- Optional `grund.toml` configuring marker, trigger, kinds, and skip lists per [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) ([§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up)).
- Optional `[workspace]` config; when present and `check` is run at the workspace root, `check` validates alias-qualified cross-project citations per [§FS-workspace](FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace).
- `--watch` is reserved for the planned resident checker (§6) and is not accepted by the current CLI.
- `--require-grounding` — turn the grounding check (§3.6) on for this run regardless of `[reference] require_grounding` in `grund.toml` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)). It only ever *adds* the check; it cannot switch off a config that already sets it.
- `--suggestions` — emit the `should`-level citation-direction findings ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) on the suggestions channel (§2.3) for this run. The same config-plus-flag pattern as `--require-grounding`: the flag never adds an error or changes the exit code, it only surfaces the advisory `suggested-citation` / `discouraged-citation` records that the default run withholds.
- `--format text|json` — output shape, per [§FS-errors.5](FS-errors.md#5-json-format). The global flags `--version` and `--help` are handled before any scan ([§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions)).

### 1.1 Recognized citations

Per [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger), a citation is the marker followed by an ID, e.g. `§FS-check.3.1`. The default marker is `§`; configurable via `grund.toml`.

In default mode (`[reference] strict = true`), only marker-prefixed citations are recognized — bare tokens are treated as plain text and do not trigger dangling-ref errors. Repositories that still rely on bare citations may set `[reference] strict = false` as a compatibility mode after checking the migration surface with `grund fmt --marker` ([§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk)).

Citations may appear in markdown prose, in source-file line/block comments, and in language doc-comments (Javadoc, JSDoc, Rustdoc, Python docstrings, etc.) — see [AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection) and [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) for the exact contexts. In source files, a **bare** ID-shaped token whose start column falls inside a string literal is not treated as a citation (the same deterministic quote-tracking rule `grund fmt` uses — [§FS-fmt.2.3.1](FS-fmt.md#231-string-literal-exclusion-rule), [AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection)), so an ID-shaped substring inside a runtime string does not raise a false dangling-ref. A marker-prefixed citation is recognized everywhere, string or not — the marker is the signal of intent. Markdown files have no string literals and the carve-out does not apply there. `E2E` citations (`§E2E-<name>`) resolve against case directories under `e2e/cases/` per [AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations).

## 2. Outputs

A report on **stdout** — `check` is a linter and its findings are its output ([§FS-errors.1](FS-errors.md#1-streams)) — plus an exit code:

- `0` — no errors. Warnings allowed (they do not affect the exit code).
- `1` — at least one error.
- `2` — scan failure (I/O, malformed file, invalid `grund.toml`).

For verbose text and JSON report examples, including empty JSON scans and global diagnostic ordering, see [§FS-output-shapes](FS-output-shapes.md#fs-output-shapes-machine-readable-output-shapes).

An invalid `grund.toml` aborts before any file is read ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)): exit `2`, a single `error:` line on stderr, nothing on stdout. A per-file failure encountered *during* the walk (a file that cannot be read or decoded) is different: the offending file is reported as `error: <path>: <reason>` on stderr (the CLI-level shape, [§FS-errors.2.2](FS-errors.md#22-cli-level-message) — the file has no line to point at, and "I could not read this" is about the run, not a finding about the graph), the walk continues over the remaining files, every finding collected from the readable files is still printed to stdout in the normal `<path>:<line>:` form, and the run exits `2` because the view of the tree was incomplete. A `2` therefore always means "do not trust this report as complete"; the printed findings are still real.

### 2.1 Report format

Findings are written to **stdout**, one per line, in the form:

```
<path>:<line>: <message>
```

`<path>` is relative to the config root ([§FS-config.3.6](FS-config.md#36-output--report-format)) when a `grund.toml` was discovered, otherwise relative to the path passed on the command line. `<line>` is 1-indexed. The `<path>:<line>:` prefix is mandatory on every finding so editors and agents can jump unmodified — this is the contract from [§GOAL-friendliness-first.1](../goals.md#1-hard-requirements).

Severity is implicit. Per-finding lines carry no `error:`/`warning:` prefix because the severity of a rule is fixed ([§FS-check.3](FS-check.md#3-errors-detected) vs §4) and the message text is what humans read. Consumers that need machine-distinguishable severity use `--format=json`.

When a finding inherently spans multiple sites (e.g., duplicate declarations, [§FS-check.3.3](FS-check.md#33-duplicate-declaration)), the message is anchored at the lexicographically-first site (sort by `path`, then `line`) and the other sites are listed parenthetically inside the message.

When there are zero errors and zero warnings, the default text form writes exactly `success` plus a trailing newline to stdout. The explicit success marker is only emitted for a diagnostic-free run; a run that has warnings prints the warning lines instead. There is no summary footer — the exit code is still the machine-readable verdict, and the per-finding lines are the human-readable detail.

With `--format=json`, the findings are emitted as NDJSON on stdout instead — same stream, machine shape per [§FS-errors.5](FS-errors.md#5-json-format). JSON remains diagnostics-only: stdout is empty when there are zero errors and zero warnings, so `grund check --format=json | jq …` sees only diagnostic objects. (CLI-level `error:` / `warning:` lines, when there are any, go to stderr — §2.1.1 — so a clean JSON run is empty on *both* streams and a `2` always means something on stderr.)

#### 2.1.1 CLI-level messages

Lines that are about the run rather than a finding at a site in the repo — unknown subcommand, malformed flag, invalid `grund.toml` schema (when the config itself parses but a value is wrong), a per-file read failure mid-walk (§2), the empty-scan caution (§2.2) — are emitted on **stderr**, never on stdout, as:

```
error: <message>
warning: <message>
```

These never carry the bare `<path>:<line>:` prefix a per-finding line wears (the one with no `error:`); the `error:` / `warning:` prefix is what distinguishes them from per-finding lines on stdout. A `grund.toml` schema error is the one CLI-level message that still points at a line — it is reported `error: <path>:<line>: <message>` ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)): the `error:` prefix keeps it CLI-level (stderr, exit `2`), but the `<path>:<line>:` inside the message text is the breadcrumb to the offending key, since a config file has one and a bad flag does not. CI scripts grep for the leading `error:` to detect launch-time failures. An `error:` always accompanies a non-zero exit; a `warning:` does not affect the exit code. In `--format=json`, a launch-time `error:` (bad flag, unreadable config) stays as raw text; a mid-walk per-file failure is one of the report's diagnostics and is rendered as JSON like the rest (on stderr, since it is `line`-less and not a graph finding — [§FS-errors.5](FS-errors.md#5-json-format)).

### 2.2 Empty scan

A walk that read **no scannable files** at all, and turned up no findings (no errors, no warnings — including the agent-entrypoint check of §3.5, which still runs and still reports even when nothing is scanned), is almost always a misconfigured scope rather than a clean repo. Rather than print nothing and exit `0` — which reads as "all clear" — `check` emits one CLI-level `warning:` line ([§FS-errors.2.2](FS-errors.md#22-cli-level-message)) to **stderr** — it is a caution about the run, not a finding about the repo, so it does not belong on stdout with the findings:

- when the scope is the repo root (no path argument, or `grund check .`) and `[scan] include` is set: the message names the `include` list and points at `grund.toml` / `grund init`, since the usual cause is a project whose sources live outside the default `docs/`, `e2e/`, `src/`;
- when an explicit path was given: the message names that path and the recognized extensions, since the usual cause is pointing `grund` at a tree with no `.md`/source files.

This is a warning, not an error: the exit code stays `0` (a genuinely empty tree is not a failure), `--format=json` emits the warning as one diagnostic JSON object on stderr (the same stream as the text `warning:` line — it is not part of the findings on stdout), and a repo that *does* have a stale `AGENTS.md` block or any other finding gets that finding (on stdout) and **no** empty-scan notice. This is the friendliness-first counterpart to the explicit success marker ([§GOAL-friendliness-first.1](../goals.md#1-hard-requirements)): the run that scanned nothing is the one case where `success` would be the wrong answer.

### 2.3 Suggestions channel *(opt-in)*

The `should` / `should-not` levels of `[citations]` ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) produce **suggestions**, not findings: they are advisory by RFC-2119 definition and grund has no per-site suppression mechanism, so surfacing them in the default run would replace the `success` marker (§2.1) on a repo that has consciously accepted a deviation, and it would never recover. They are therefore withheld from the default run and live on a separate channel, decided in [§DF-citation-directions](../decisions/functional/DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels).

`grund check --suggestions` (§1) emits them. A suggestion is a third report channel, **not** a third severity: [§FS-config.6](FS-config.md#6-what-is-not-configured-here) freezes the severity set at `{error, warning}`, so a suggestion carries `"channel": "suggestion"` rather than a `severity` ([§FS-errors.5](FS-errors.md#5-json-format)). The codes are `suggested-citation` (a `should` obligation a declaration does not meet), `discouraged-citation` (a `should-not` citation site), and `escaped-citation-resolves` (§2.3.1).

- **Text** — `--suggestions` prints each suggestion in the located-finding shape `path:line: message` (§2.1), interleaved with errors and warnings in the same deterministic order ([§FS-errors.4](FS-errors.md#4-determinism)). Without the flag, suggestions are not printed, and the `success` marker still appears for a run with zero errors and zero warnings even if suggestions exist — a suggestion is not a finding about well-formedness.
- **Exit code** — suggestions never affect it (`0`/`1`/`2` unchanged), exactly like the empty-scan caution.
- **JSON** — under `--suggestions`, suggestion objects are emitted on stdout alongside the findings with `"channel": "suggestion"`; a consumer filtering on `severity ∈ {error, warning}` is unaffected. Without the flag none are emitted.

`grund gap` ([§RM-gap-report](../roadmap.md#rm-gap-report-orphan-and-uncovered-id-reports)) is the standing home for these records once it ships — a should-level miss is precisely "the graph is thinner than recommended," and gap is exit-code-neutral by design.

#### 2.3.1 Escaped citation resolves

A citation whose marker is bracketed — the schematic `<§>alias/ID` shape — is deliberately inert: the `§` is not immediately followed by the ID, so no pass treats it as a citation ([§FS-workspace.1](FS-workspace.md#1-citation-syntax)). That is how a citation's *shape* is written in prose without `grund check` resolving it. It also makes an escape of an ID that *does* exist ambiguous: usually a deliberate illustration, but also exactly what a live citation looks like once the marker is bracketed by accident — and that slip is invisible, since the escaped form raises no dangling error (§3.1) and navigates nowhere. So when an escaped citation's ID resolves to a real declaration, grund emits an `escaped-citation-resolves` suggestion at the escape site, naming the live `§`-form to switch to. It is a suggestion, never a warning or error: illustrating a real ID is legitimate, so it must never replace `success` (§2.1) or change the exit code. It is the mirror of the §3.1 dangling check — that flags a live citation whose ID does not resolve; this flags an escaped one whose ID does. Unqualified `<§>ID` and qualified `<§>alias/ID` escapes are both covered; the ID is parsed with the citing project's grammar, so a cross-namespace target under an unusual grammar may be skipped, which only ever withholds a suggestion.

## 3. Errors detected

Each of the following is an error and contributes to a non-zero exit code.

### 3.1 Dangling citation

A recognized citation (per §1.1) for which no declaration is found. If the
target namespace contains a declared ID of the same kind that is close by
deterministic edit distance, the diagnostic appends one hint:
`unknown reference FS-chek; did you mean FS-check?`. If no same-kind candidate
is close enough, the message stays `unknown reference <ID>` so unrelated missing
IDs do not produce noisy guesses.

When the dangling citation sits inside a Markdown inline-code span — where a
`§`-citation is as often an illustration as a live reference — the diagnostic
also offers the `<§>` escape ([§FS-workspace.1](FS-workspace.md#1-citation-syntax)):
`unknown reference api/FS-zzz; write <§>api/FS-zzz if this is an illustration`.
The two hints combine when a near-ID match and an inline-code context apply at
once: `unknown reference api/FS-login; did you mean api/FS-logout? (or write
<§>api/FS-login if this is an illustration)`. Outside inline code the escape hint
is withheld, so an ordinary prose typo is nudged toward the near ID, not toward
escaping. This is the live-citation counterpart to §2.3.1's escaped-citation
suggestion: there an escape resolves and might be live; here a live citation
dangles and might be an escape.

### 3.2 Missing section

A citation with a section suffix (`§FS-<user-login>.3.1`) where the declaration exists but the requested section heading does not.

### 3.3 Duplicate declaration

The same `<KIND>-<NNN>-<slug>` declared as a heading in more than one file. Reported per §2.1: one error anchored at the lexicographically-first site, with the remaining sites listed in the message.

### 3.4 Broken inline-spec stub

A `docs/` file whose H1 has the stub shape `# <ID>: [<text>](<path>)` where either the path does not exist, or the file at that path contains no inline declaration of the same ID. Relative stub links resolve as normal Markdown links first — relative to the stub file's directory — so `lychee` and rendered docs see the same target. If that path does not exist, `grund` falls back to resolving the path relative to the config root for compatibility with older stubs that wrote repo-root paths.

### 3.5 Invalid agent entrypoint init block

If `<path>/AGENTS.md` exists, `check` verifies the versioned `grund init` block defined by [§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints). It also verifies known companion agent entrypoints whenever they exist and are not symlinks to `AGENTS.md`; for example, existing standalone `AGENTS.override.md`, `CLAUDE.md`, `.claude/CLAUDE.md`, `GEMINI.md`, and `.github/copilot-instructions.md` files must carry the same managed block, while `CLAUDE.md -> AGENTS.md` is already covered by the canonical file. `check` does not require absent workspace-triggered aliases from [§FS-init.2.1](FS-init.md#21-files-written-updated-or-left-in-place); once `grund init` creates one, it is validated because it exists. If `AGENTS.md` does not exist, existing companion agent files without a managed block are treated as project-owned instructions and are not validated by `grund check`; this keeps config-only adoption from modifying or policing an existing agent setup. A companion that already contains a managed `grund init` block is still version-checked even without `AGENTS.md`, so repos initialized directly into `CLAUDE.md`, `GEMINI.md`, or another explicit entrypoint still get drift detection. A missing managed block when one is required, an older block version, or a newer unsupported block version is an error in scaffolded-entrypoint mode. A legacy H2-bounded block from v3 or earlier ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)) is still recognized and reported as an *older block version* — `run \`grund init\`` is its transition path to the delimited form, so existing repositories are told how to migrate rather than treated as malformed. Broken delimiters are a distinct error: a `<!-- BEGIN GRUND MANAGED BLOCK -->` with no `<!-- END GRUND MANAGED BLOCK -->` after it, an `END` with no `BEGIN` before it, more than one `BEGIN`, or a delimited region without a `## Grounding with grund (vN)` version heading is reported as a malformed managed block, anchored at the offending delimiter line and naming the defect; `check` never rewrites the file, and `grund init` refuses to splice against broken delimiters for the same reason ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)). This lets CI catch repos whose managed agent entry points were initialized and later drifted or need to be refreshed with `grund init`.

### 3.6 Ungrounded source file *(opt-in)*

Off by default. When `[reference] require_grounding = true` is set in `grund.toml` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)) — or `grund check --require-grounding` is passed (§1) — every scanned **source file** (a file the walk reads whose extension is not `.md`, [AR-scanner.1](../architecture/AR-scanner.md#1-tree-walk)) must be *grounded*: it must contain at least one recognized citation (§1.1) whose ID resolves to a declaration, **or** it must itself declare an ID inline (a spec home is grounded in the spec it *is*, [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)). A source file that is neither is an error, anchored at line 1:

```
src/foo.rs:1: ungrounded source file: no § citation to a declared ID
```

The marker in the message is the configured one ([§FS-config.3.1](FS-config.md#31-reference--citation-form)). A file whose only citation is dangling (§3.1) is *not* grounded — it gets both findings; fixing the citation clears both. Markdown files are never subject to this rule (they are documents, not implementation); use the unused-declaration warning (§4.1) and dangling/section errors for those.

This is a pure function of `(tree, config)` like every other `check` rule ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)): it reads no git history ([§FS-non-goals.6](FS-non-goals.md#6-decision-database-audit-log-history-tracking)) and parses no code ([§FS-non-goals.3](FS-non-goals.md#3-code-ast-parsing)) — "source file" is decided by extension, "grounded" by the citations the scanner already collected. It is the floor of the grounding discipline — the verification-at-rest layer of [§GOAL-agent-grounding.1](../goals.md#1-the-three-layers), on top of which `grund cover` exposes the citation graph ([§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)) and [§RM-cochange-gate](../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test) tracks the diff-aware co-change gate. Decided in [§DF-require-grounding](../decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec).

### 3.7 Misplaced declaration (configured kind home)

A kind configured with `file = "<path>"` in [[kinds]] ([§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)) is a *single-file kind* — every declaration of that kind must live in that exact document. A declaration whose H1/H2 is found in any other scanned file is reported as a misplaced-declaration error, anchored at the declaration line:

```
docs/notes.md:42: GOAL-foo must be declared in docs/goals.md (single-file kind)
```

Stubs (`# <ID>: [<text>](<path>)`) are exempt from this exact-file requirement — they are pointers from a kind's home folder to an inline declaration elsewhere, which is a multi-file-kind feature; a single-file kind has no stubs because there is no folder to redirect from. This is the canonical mechanism that keeps `GRUND`, `GOAL`, and `RM` declarations consolidated in their respective documents, and what makes "one file, all goals inline" a checked invariant rather than a convention.

Every configured `file` and `folder` also acts as a declaration-home boundary. If a declaration line appears in a file that belongs to exactly one configured kind home, the declaration's kind must match that home kind. A `file` home matches only that exact path; a `folder` home matches files below that directory. A different-kind declaration is reported as a misplaced-declaration error, anchored at the declaration line and naming the declared kind, the expected home kind, and the configured home:

```
docs/functional-spec/FS-lsp.md:42: AR-router declares kind AR inside FS home docs/functional-spec
```

The home-kind rule applies to declaration lines and stub lines, not citations or prose mentions. Files that belong to no configured home, or that match multiple homes because configured homes overlap or nest, are not checked by this rule because the expected kind is ambiguous.

### 3.8 Cross-project citation failure

In a workspace run, an alias-qualified citation whose alias is unknown, whose target declaration is missing, or whose target section is missing is reported at the citation site. The namespace and resolution rules live in [§FS-workspace.4](FS-workspace.md#4-resolution).

### 3.9 Section heading level mismatch

When `[id] section_heading_levels = "strict"` (the default), every numbered section heading must sit at the Markdown depth implied by its dotted path: expected level is the declaration heading level plus the number of path components ([§FS-config.3.3](FS-config.md#33-section-paths--arbitrary-nesting-depth), [AR-scanner.2.2](../architecture/AR-scanner.md#22-section-detection)). A heading `## 1.1 Details` under an H1 declaration is therefore an error at the heading line: it must be `### 1.1 Details`. With `"warn"`, the same mismatch is reported as a warning; with `"loose"`, the checker does not report it and retains the historical rule that any deeper heading can declare any dotted section path. Plain, unnumbered headings and bold labels are not checked by this rule because they are not grund section targets.

### 3.10 Inline citation style violation

A citation site in a code comment that violates the configured inline citation style — `inline_style = "citation-only"` with prose present, an inline note that exceeds `inline_note_max_lines`, or one that exceeds `inline_note_max_columns`. The full mode and budget contract, and how multi-cap violations split into multiple findings, lives in [§FS-inline-citation-style.4.1](FS-inline-citation-style.md#41-errors--hard-caps). The schema for the controlling keys is in [§FS-config.3.1](FS-config.md#31-reference--citation-form).

### 3.11 Missing required citation

When `[citations]` ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) sets a `must` obligation for a citing kind, every top-level declaration of that kind must carry at least one citation satisfying each `must` entry, anywhere in its body. A declaration that does not is an error anchored at the declaration line, naming the unmet target:

```
docs/architecture/AR-router.md:1: AR-router must cite FS or GOAL (citation direction)
```

The body extent and the citing-side classification come from the scanner ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)); the obligation pass is [AR-checker.2.9](../../crates/grund-core/src/checker.rs). A `code`-kind obligation ([§FS-config.3.9.2](FS-config.md#392-the-code-pseudo-kind)) is per file rather than per declaration — a source file that contains at least one citation but none satisfying the obligation is the error, anchored at line 1. An `E2E`-kind obligation ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) is per case declaration, can be satisfied by the case's `spec.refs` manifest entries, and remains an error when the case has no scanned citations or matching manifest reference. The parallel `should` obligation is not an error; it is a suggestion (§2.3).

### 3.12 Forbidden citation

When `[citations]` ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) sets a `must-not` prohibition for a citing kind, every citation site of that kind to a prohibited target is an error anchored at the citation site:

```
docs/functional-spec/FS-login.md:42: FS must not cite AR (citation direction)
```

The citing kind is the site's resolved `source_kind` ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)); the cited kind and namespace come from the citation token, matched against the rule's namespace grammar ([§FS-config.3.9.3](FS-config.md#393-namespace-matching)). The prohibition pass is [AR-checker.2.10](../../crates/grund-core/src/checker.rs). The parallel `should-not` prohibition is not an error; it is a suggestion (§2.3). The sanctioned way to keep a discouraged downward pointer is a plain Markdown link, which is not a citation under `strict = true` and so is exempt from this rule.

## 4. Warnings

### 4.1 Unused declaration

An ID that is declared but never cited. Reported as a warning, not an error — newly declared IDs may not yet have citations. Warnings never affect the exit code (§2).

`E2E` declarations ([AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations)) are exempt: an end-to-end case is exercised by being run, not by being cited, so a `§E2E-<name>` that nothing references is not a warning. Every other kind is subject to this rule. `grund list --unused` ([§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id)) uses the same default signal and suppresses uncited `E2E` cases unless `E2E` is explicitly selected with `--kind` (including a multi-kind filter such as `--kind FS,E2E`).

### 4.2 Inline note soft-cap overrun *(opt-in)*

Off by default. When `[reference] warn_on_suggested = true` is set in the project's `grund.toml` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)), an inline citation site whose line count exceeds `inline_note_suggested_lines` but stays within `inline_note_max_lines` is reported as a warning. The full contract — what counts as a soft-cap overrun and how it interacts with the hard-cap error in §3.10 — lives in [§FS-inline-citation-style.4.2](FS-inline-citation-style.md#42-warnings--opt-in-soft-cap). Off by default because the soft cap is primarily agent-facing guidance ([§FS-inline-citation-style.5](FS-inline-citation-style.md#5-agent-facing-rendering)); flipping the toggle escalates it to a `check`-time signal.

### 4.3 Redundant config pair

A directory that carries both a bare `grund.toml` and `.agents/grund.toml` ([§FS-config.1.1](FS-config.md#11-when-one-directory-carries-both)). The bare file is the config; the `.agents/` one is read by nothing, so a user who edits it changes nothing and is told so:

```
warning: .agents/grund.toml is ignored — grund.toml takes precedence; delete one
```

It is a CLI-level `warning:` on **stderr** (§2.1.1), not a per-finding line: it is about which file the run read, not a finding at a site in the citation graph, and there is no offending line to point at — the whole file is ignored. Both paths are rendered relative to the config root ([§FS-config.3.6](FS-config.md#36-output--report-format)), so the message names the two files a user has to choose between. Like every warning it leaves the exit code alone (§2), because the pair is the ordinary transient state of a migration between the two forms ([§DF-config-file-location.2.2](../decisions/functional/DF-config-file-location.md#22-the-bare-grundtoml-wins-a-tie-and-check-warns-about-the-pair)).

The same warning is emitted by `grund config validate` and `grund config show` ([§FS-config.4.1](FS-config.md#41-grund-config-validate-path), [§FS-config.4.2](FS-config.md#42-grund-config-show-path)) — those are the surfaces a user reaches for when the answer to "why is my config not taking effect" is that `grund` is reading the other file. No other command reports it: a redundant pair is a fact about the repository's configuration, and `show`, `list`, `refs`, `cover`, and `fmt` answer questions about its content.

## 5. What grund does not check

See [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) — in particular [§FS-non-goals.1](FS-non-goals.md#1-markdown-link-validation) (markdown links / URLs), [§FS-non-goals.2](FS-non-goals.md#2-spelling-grammar-prose-quality) (spelling/grammar), and the convention that ID numbers are stable handles, not ordinal positions.

One near-miss `check` does not flag *today*: a heading shaped like `# <KIND>-…: <title>` whose ID does not match the configured `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) is simply not a declaration — invisible to `check`, to `grund list`, and to citation resolution, with no warning that something heading-shaped was ignored (the classic stumble: `# FS-login: …` under the default `{kind}-{number}-{slug}`). A non-heuristic "looks like a declaration" warning for it is tracked under [§RM-declaration-near-miss](../roadmap.md#rm-declaration-near-miss-warn-on-a-heading-that-looks-like-a-declaration-but-does-not-match-id-format) — it would surface the mismatch, never guess the corrected ID (`check` reports facts about the tree, §3 vs §4).

## 6. Watch mode (`--watch`)

Status: planned — implementation tracked under [§RM-watch](../roadmap.md#rm-watch-implement-grund-check---watch).

When implemented, `grund check --watch [<path>]` will run the check once, then stay resident and re-run it whenever a file under the scanned tree (or the discovered `grund.toml`) changes. It is the editor-less counterpart to the optional LSP server ([§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)): the LSP integrates `grund` into an editor's diagnostics; `--watch` is the plain-terminal "every save" loop that [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) exists for. Until [§RM-watch](../roadmap.md#rm-watch-implement-grund-check---watch) lands, `grund check --watch` is a CLI error (`error: unknown flag \`--watch\``, exit 2).

- **Change detection.** Filesystem notifications where the OS provides them; a debounce window coalesces a burst of writes into one re-check. No polling loop is required, and there is no configurable interval — the watcher reacts, it does not sample.
- **Each run is a plain `grund check`.** Output and exit-status semantics of an individual run are exactly §2/§2.1 on the tree's state at that moment — byte-identical to what a non-`--watch` invocation would print ([§FS-errors.4](FS-errors.md#4-determinism)). Before each run the previous run's output is cleared so the terminal always shows the current report; with `--format=json` each run emits the same diagnostic NDJSON as non-watch mode, scoped to that run.
- **Lifecycle.** The process runs until interrupted (Ctrl-C / SIGINT). On interrupt it exits with the exit code of the most recently completed run (`0`/`1`/`2`), so `grund check --watch &` followed by a later signal is still a meaningful CI-ish probe. There is no TUI, no key bindings, no prompt — it is non-interactive per [§FS-non-goals.10](FS-non-goals.md#10-interactive-mode), just a re-printing checker. No network I/O ([§FS-non-goals.11](FS-non-goals.md#11-network-access-during-a-check)); the only files touched are the ones the walk already reads.
- **Scope.** `--watch` will be a `check` flag spelled as `grund check --watch [<path>]` ([§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions)). Other subcommands will not take it; a one-shot `grund fmt` or ID query has nothing to keep watching.
