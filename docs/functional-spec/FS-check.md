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
- `--full` — walk the whole config root, past `[scan] include`, and report the references that resolve to nothing out there on their own tier (§1.3, §3.14). It only ever *adds* findings; the in-scope report is unchanged.
- `--format text|json` — output shape, per [§FS-errors.5](FS-errors.md#5-json-format). The global flags `--version` and `--help` are handled before any scan ([§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions)).

### 1.1 Recognized citations

Per [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger), a citation is the marker followed by an ID, e.g. `§FS-check.3.1`. The default marker is `§`; configurable via `grund.toml`.

In default mode (`[reference] strict = true`), only marker-prefixed citations are recognized — bare tokens are treated as plain text and do not trigger dangling-ref errors. Repositories that still rely on bare citations may set `[reference] strict = false` as a compatibility mode after checking the migration surface with `grund fmt --marker` ([§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk)).

Citations may appear in markdown prose, in source-file line/block comments, and in language doc-comments (Javadoc, JSDoc, Rustdoc, Python docstrings, etc.) — see [AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection) and [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) for the exact contexts. In source files, a **bare** ID-shaped token whose start column falls inside a string literal is not treated as a citation (the same deterministic quote-tracking rule `grund fmt` uses — [§FS-fmt.2.3.1](FS-fmt.md#231-string-literal-exclusion-rule), [AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection)), so an ID-shaped substring inside a runtime string does not raise a false dangling-ref. A marker-prefixed citation is recognized everywhere, string or not — the marker is the signal of intent. Markdown files have no string literals and the carve-out does not apply there.

Two contexts are read as neither prose nor code, so nothing inside them is a citation. A **fenced code block** in Markdown is skipped entirely: this is what makes an example ID safe to write in documentation without the `<§>` escape, and it is why the illustrations throughout these specs resolve to nothing. A fence opens with at most three leading spaces followed by a run of at least three backticks or tildes; it closes only on a run of the **same character** at least as long as the opener, again with at most three leading spaces and only whitespace after the run. A backtick opener cannot carry a backtick in its info string. An unclosed fence runs to end of file. In **source files only**, the namespace-qualified form `§<alias>/<ID>` is additionally skipped inside an inline-code span or a string literal, because `alias/ID` is shaped like a path, module reference, or URL ([AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection)); the unqualified form stays live there, and neither skip applies in Markdown. Every other skip is a property of the *walk* rather than of the text — hidden paths, `[scan] exclude`, ignore-file matches, and unlisted `[scan] extensions` mean a file is never read at all ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full), [§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded)). `E2E` citations (`§E2E-<name>`) resolve against case directories under `e2e/cases/` per [AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations).

### 1.2 The number-only shorthand

When `[id] format` carries **both** `{number}` and `{slug}` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) — the default `{kind}-{number}-{slug}` that `grund init` writes — the number alone already identifies a declaration within its kind, so `§FS-042` is an abbreviation of `§FS-042-user-login` rather than a different ID. `check` **recognizes** that shape, resolves it, and reports it as an error to be rewritten (§3.13). It is never silently ignored, which is what [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) means by "false negatives are bugs".

The shorthand shape is the configured `format` with the `{slug}` placeholder and one adjacent literal separator removed. A repo whose format has no `{number}` (`{kind}-{slug}`, the form `grund` itself uses) or no `{slug}` (`{kind}-{number}`) has no shorthand and is untouched by this clause ([§FS-id.4.1](FS-id.md#41-number-less-id-formats)).

Three rules bound the recognition, decided in [§DF-number-only-citation-shorthand](../decisions/functional/DF-number-only-citation-shorthand.md#df-number-only-citation-shorthand-the-number-only-shorthand-is-authoring-sugar-and-a-persisted-one-is-a-check-error):

- **The marker is required.** A bare `FS-042` is plain text even under `strict = false`, where a bare *full* ID would count (§1.1). `KIND-NNN` occurs constantly in the wild as issue keys, part numbers, and standards references, and unlike a full ID it carries no slug to make an accidental match unlikely — so the marker is what supplies the intent ([§DF-number-only-citation-shorthand.2.4](../decisions/functional/DF-number-only-citation-shorthand.md#24-the-marker-is-required-a-bare-shorthand-is-text)).
- **The full ID always wins,** and the token must end where the shorthand does. The full-ID pass claims its tokens first; the shorthand pass only sees what is left, and it claims a token only when the character after the match cannot continue an ID — an alphanumeric, `_`, or a literal from `format` that itself has a component after it. Without that trailing boundary the shorthand is a *prefix* of every longer ID-shaped token, so a full ID whose slug the grammar rejects (`§FS-042-User-Login`, `§FS-042_user_login`) would be read as `§FS-042` with a tail hanging off it. Such a token is not a citation at all: it is reported by nothing here and rewritten by nothing in [§FS-fmt.2.4](FS-fmt.md#24-shorthand-to-canonical). The separator has to be *followed* by a component to count, or a citation ending a sentence would be lost in any repo whose `format` uses `.` as a literal. And `/` never counts: it can only precede a kind, so `§FS-042/x` is a citation of `FS-042` exactly as `§FS-042-user-login/x` is one of the full ID — the shorthand and the canonical form must never disagree about the same boundary.
- **A resolved shorthand is a real edge.** When it matches exactly one declaration, the citation participates in the graph like any other: [§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id) lists it, [§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) groups it, the declaration stops being reported as unused (§4.1), it grounds its file under `require_grounding` (§3.6), and it counts for citation directions ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)). The `.<section>` suffix works as it does on any citation, which means the section check (§3.2) applies to it independently and on its own terms: a shorthand carrying a section that does not exist earns *both* findings, because the canonical form §3.13 names is the right ID and still the wrong section.

Recognition is where those three rules stop. A recognized shorthand is still not always *being used* as a citation: one glued to a second number — `§SPEC-001→SPEC-003` — is a numeral in a run, and while it resolves and counts like any other edge, `grund fmt` will not rewrite it and §3.15 reports it instead of §3.13 ([§FS-fmt.2.4.1](FS-fmt.md#241-a-shorthand-in-a-numeric-run-is-not-rewritten)).

The same shape is accepted as a **CLI ID argument** — `grund FS-042`, `grund FS-042.1`, `grund refs FS-042` — where nothing is persisted and the caller gets the declaration ([§FS-show.1](FS-show.md#1-inputs), [§FS-refs.1](FS-refs.md#1-inputs)). That is also what makes a clicked `§FS-042` open in a terminal or editor, since those clients hand the token straight to `grund` ([§FS-integrations.3.1](FS-integrations.md#31-terminal-clients-wezterm-kitty-tmux-iterm2)).

### 1.3 The full-tree scope (`--full`)

`[scan] include` ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)) is the list of roots the walk starts from, so a citation in a file outside it is not merely unchecked — it is invisible. It does not resolve, it does not dangle, it appears in no report, and dangling IDs accumulate there for as long as nobody notices. `check` returning clean means "clean *within* `include`" and reads as "clean": a false negative in the one command the workflow trusts, and one that is invisible by construction, because the citations that most need checking are the ones somebody forgot to bring into scope. That is the class [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) calls a bug, so it needs a way to be *seen* without first editing the config to guess where to look — the bounded blind spot [§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) accepts only because this flag exists to look into it.

`grund check --full` is that way. It cancels `[scan] include` for the walk — and nothing else:

- **The walk covers the whole config root.** `[scan] exclude`, `.gitignore` and every other ignore file, hidden directories, workspace member boundaries ([§FS-workspace.6](FS-workspace.md#6-nested-project-boundary)), and `[scan] extensions` all still apply exactly as they do without the flag. A file type `grund` does not scan stays unscanned; widening `extensions` is a config decision, and one flag that widened both would make "what did this run read" unanswerable. A boundary is a *declared* member, not any nested `grund.toml`: a project directory the workspace never declared is ordinary tree to both walks, so `--full` reads it and judges its citations under *this* project's grammar and kinds. That is what the plain walk does with it too, but the flag is what makes it reachable by default — a vendored, generated, or example project belongs in `[scan] exclude`, or in `[workspace] members` if it is one of ours, before a run adds `--full`.
- **The wider walk reads a superset of the narrow one, each file once.** Every root `[scan] include` names is walked under `--full` too, whether or not `exclude`, an ignore file, or the hidden-directory rule would otherwise prune it: those three rules prune *descendants*, never the directory a walk starts at, so a gitignored, excluded, or hidden `include` root is read by the plain run and must be read here. Without that, `--full` could read *fewer* files than `grund check` and hide a finding instead of adding one. Overlapping roots — an `include` entry inside another, or inside the config root the flag adds — name one file once; a file read twice would be a declaration duplicated with itself (§3.3). "Once" is per *file*, not per path: an `include` root that is a symlink to a directory inside the config root, or a case alias of one on a case-insensitive filesystem, reaches its files under a spelling the config-root walk never produces, so a byte-identical compare cannot see the reread. The walk therefore starts at the `include` roots *before* the config root and keeps the first spelling of each file — the one `grund check` prints without the flag. Every in-scope line is the plain run's, character for character; `--full` only ever appends `outside [scan] include:` lines.
- **Two scopes, two rule sets.** Inside the configured scope, the report is the ordinary one. Outside it, the only findings are the reference-resolution errors of §3.14 — no style, no grounding, no unused declarations, no citation directions. A `--full` that failed on inline-note budgets in directories that never opted into them would be run once and never again.
- **Purely additive.** The findings inside the configured scope are exactly the ones `grund check` reports on the same tree, so `--full` can only ever turn a green run red, never the reverse. It is the ordinary check plus a wider search for references that point at nothing.
- **The unused-declaration warning is unchanged out there.** A declaration inside `include` cited *only* from outside it keeps its `declared but never cited` warning (§4.1) under `--full`, and the citation that would have retired it resolves and is reported by nothing. That is what additivity costs, and it is the right side of the trade: counting the wider walk's citations toward §4.1 would *remove* an in-scope finding, the one direction this flag must never move, and it would make the warning mean something different depending on a flag. The remedy is the one the tier already names — widen `include` so the citing file is governed, and the edge counts everywhere.
- **An explicit path still narrows.** `--full` cancels `include`, never a path the caller typed: `grund check <path> --full` scans exactly `<path>` ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)), which is already outside `include`'s reach, and has no out-of-scope tier. A path that *resolves to the config root* is the root scope and does widen — `grund check .`, `grund check ./`, and `grund check <abs-root>` are the bare `grund check --full`, tier included. Any other path leaves the flag nothing to cancel.
- **A path the flag cannot widen earns a caution, not a refusal.** When `--full` is passed with an explicit path that is not the config root, the run emits one CLI-level `warning:` (§2.1.1) on **stderr** and reports the same findings, on the same streams, with the same exit code as the run without the flag:

  ```
  warning: --full has no effect with an explicit PATH — it cancels [scan] include, and sim/ already bypasses it
  ```

  Silently accepting the flag is the failure this mode exists to end in miniature: the caller asked for the wider search and got the ordinary run, with no signal. Rejecting it would be worse — the run is a valid one, and a script that passes `--full` uniformly would start failing on the invocation where it happens to be redundant. It is a warning like any other: the exit code is untouched, it stands in place of the `success` marker on an otherwise clean run (§2.1), and under `--format json` it is one diagnostic object on stderr, so a clean run's **stdout** stays empty either way.
- **Workspaces widen per project.** Run at a workspace root, `--full` applies to the root project and to every member ([§FS-workspace.5](FS-workspace.md#5-command-scope)): each walks its own tree past its own `[scan] include` and tiers its findings against its own configured scope, because `include` is a per-project statement. It widens the projects a run already has and never invents one, so under `[workspace] include_root = false` ([§FS-workspace.2](FS-workspace.md#2-workspace-configuration)) a file at the workspace root outside every member is read by nothing, with or without the flag — there is no root project whose `include` there would be to cancel.
- **An empty configured scope is still reported.** A `--full` run whose *configured* scope read no files gets the §2.2 caution as well as its out-of-scope findings: the tier says where the citations actually are, the caution says the config has not been told.

It is a flag, never a `grund.toml` key. A project that wants its whole tree governed widens `include` and gets the whole rule set; `--full` exists for the tree whose config has drifted from where the code moved, and a config key for it would be a second, weaker `include` that two installs could read differently ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). Decided in [§DF-check-full-scope](../decisions/functional/DF-check-full-scope.md#df-check-full-scope-check---full-walks-past-scan-include-and-reports-only-unresolvable-references-out-there).

## 2. Outputs

A report on **stdout** — `check` is a linter and its findings are its output ([§FS-errors.1](FS-errors.md#1-streams)) — plus an exit code:

- `0` — no errors. Warnings allowed (they do not affect the exit code).
- `1` — at least one error.
- `2` — scan failure (I/O, malformed file, invalid `grund.toml`).

For verbose text and JSON report examples, including empty JSON scans and global diagnostic ordering, see [§FS-output-shapes](FS-output-shapes.md#fs-output-shapes-machine-readable-output-shapes).

An invalid `grund.toml` aborts before any file is read ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)): exit `2`, a single `error:` line on stderr, nothing on stdout. A per-file failure encountered *during* the walk (a file that cannot be read or decoded) is different: the offending file is reported as `error: <path>: <reason>` on stderr (the CLI-level shape, [§FS-errors.2.2](FS-errors.md#22-cli-level-message) — the file has no line to point at, and "I could not read this" is about the run, not a finding about the graph), the walk continues over the remaining files, every finding collected from the readable files is still printed to stdout in the normal `<path>:<line>:` form, and the run exits `2` because the view of the tree was incomplete. A `2` therefore always means "do not trust this report as complete"; the printed findings are still real. Malformed input is answered with a located diagnostic and a truthful code, never an abort ([§REQ-never-crashes](../requirements/REQ-never-crashes.md#req-never-crashes-garbage-in-diagnostic-out)).

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

Lines that are about the run rather than a finding at a site in the repo — unknown subcommand, malformed flag, invalid `grund.toml` schema (when the config itself parses but a value is wrong), a per-file read failure mid-walk (§2), the empty-scan caution (§2.2), the nothing-recognized caution (§4.5) — are emitted on **stderr**, never on stdout, as:

```
error: <message>
warning: <message>
```

These never carry the bare `<path>:<line>:` prefix a per-finding line wears (the one with no `error:`); the `error:` / `warning:` prefix is what distinguishes them from per-finding lines on stdout. A `grund.toml` schema error is the one CLI-level message that still points at a line — it is reported `error: <path>:<line>: <message>` ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)): the `error:` prefix keeps it CLI-level (stderr, exit `2`), but the `<path>:<line>:` inside the message text is the breadcrumb to the offending key, since a config file has one and a bad flag does not. CI scripts grep for the leading `error:` to detect launch-time failures. An `error:` always accompanies a non-zero exit; a `warning:` does not affect the exit code. In `--format=json`, a launch-time `error:` (bad flag, unreadable config) stays as raw text; a mid-walk per-file failure is one of the report's diagnostics and is rendered as JSON like the rest (on stderr, since it is `line`-less and not a graph finding — [§FS-errors.5](FS-errors.md#5-json-format)).

### 2.2 Empty scan

A walk that read **no scannable files** at all, and turned up no findings (no errors, no warnings — including the agent-entrypoint check of §3.5, which still runs and still reports even when nothing is scanned), is almost always a misconfigured scope rather than a clean repo. Rather than print nothing and exit `0` — which reads as "all clear" — `check` emits one CLI-level `warning:` line ([§FS-errors.2.2](FS-errors.md#22-cli-level-message)) to **stderr** — it is a caution about the run, not a finding about the repo, so it does not belong on stdout with the findings:

- when the scope is the repo root (no path argument, or `grund check .`) and `[scan] include` is set: the message names the `include` list and points at `grund.toml` / `grund init`, since the usual cause is a project whose sources live outside the default `docs/`, `e2e/`, `src/`;
- when an explicit path was given: the message names that path and the recognized extensions, since the usual cause is pointing `grund` at a tree with no `.md`/source files.

This is a warning, not an error: the exit code stays `0` (a genuinely empty tree is not a failure), `--format=json` emits the warning as one diagnostic JSON object on stderr (the same stream as the text `warning:` line — it is not part of the findings on stdout), and a repo that *does* have a stale `AGENTS.md` block or any other finding **about the configured scope** gets that finding (on stdout) and **no** empty-scan notice. Two findings are not about that scope and do not suppress it: the redundant-config pair (§4.3), which is about which file the run read rather than what it walked — a repository mid-migration must not lose the scope diagnostic because it also has a config pair — and the out-of-scope tier (§3.14), which is about the tree *outside* the scope. The second is the case the caution is worth most: the tier says where the citations actually are, and the caution says the config has not been told. This is the friendliness-first counterpart to the explicit success marker ([§GOAL-friendliness-first.1](../goals.md#1-hard-requirements)): the run that scanned nothing is the one case where `success` would be the wrong answer.

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

A number-only shorthand citation (§1.2) is exempt from this rule and reported by
§3.13 instead — never both, because `unknown reference FS-042` would name a token
that is not a full ID under the repo's own grammar.

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

In a workspace run, an alias-qualified citation whose alias path is unknown, whose target declaration is missing, or whose target section is missing is reported at the citation site. The namespace and resolution rules live in [§FS-workspace.4](FS-workspace.md#4-resolution).

An unknown alias path names the projects it could have meant, so the fix is in the diagnostic rather than in the config. At the outermost workspace root — the scope CI runs, and the only one that can see every project a path could name — candidates are taken from the projects in scope in one tier only, best first: a project whose path **ends with** what was written (a dropped prefix — the mistake full alias paths invite, [§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)), else one whose **last segment** matches (a wrong prefix), else one a **typo** away under the same near-match rule §3.1 uses. Up to three are listed — `grund list` is the catalogue, a finding is not — and a path with no candidate reports on its own, unchanged.

```text
docs/FS-root.md:3: unknown project alias sprayer; did you mean hardware/sprayer?
docs/FS-root.md:4: unknown project alias api; did you mean left/api or right/api?
```

A run narrowed to a subtree ([§FS-workspace.5](FS-workspace.md#5-command-scope), [§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)) holds only part of the tree, so a path naming a project outside it is unknown *here* while being exactly right at the workspace root. Such a run therefore offers **no candidate at all**. It cannot tell a dropped prefix from a path that correctly names a project outside its subtree, and every tier reads it as the first: the dropped-prefix tier included, because a *shorter* written path is itself a complete alias path whenever a top-level project carries that name, so re-pointing it at a deeper namesake rewrites a citation CI accepts into a different project's — green before and green after, so nothing catches it. One rule for the whole narrowed run, and no residual misdirection: it names the subtree it covers — as a *subtree*, since that scope's own alias path is one project among the several it holds — rather than reporting the path bare, which is neither "delete this" nor "re-prefix this" but "check this from the root":

```text
docs/AR-bus.md:3: unknown project alias final/pod; only the hardware subtree is in scope here — check from the workspace root for a path outside it
```

### 3.9 Section heading level mismatch

When `[id] section_heading_levels = "strict"` (the default), every numbered section heading must sit at the Markdown depth implied by its dotted path: expected level is the declaration heading level plus the number of path components ([§FS-config.3.3](FS-config.md#33-section-paths--arbitrary-nesting-depth), [AR-scanner.2.2](../architecture/AR-scanner.md#22-section-detection)). A heading `## 1.1 Details` under an H1 declaration is therefore an error at the heading line: it must be `### 1.1 Details`. With `"warn"`, the same mismatch is reported as a warning; with `"loose"`, the checker does not report it and retains the historical rule that any deeper heading can declare any dotted section path. Plain, unnumbered headings and bold labels are not checked by this rule because they are not grund section targets.

### 3.10 Inline citation style violation

A citation site in a code comment that violates the configured inline citation style — `inline_style = "citation-only"` with prose present, an inline note that exceeds `inline_note_max_lines`, or one that exceeds `inline_note_max_columns`. The full mode and budget contract, and how multi-cap violations split into multiple findings, lives in [§FS-inline-citation-style.4.1](FS-inline-citation-style.md#41-errors--hard-caps). The schema for the controlling keys is in [§FS-config.3.1](FS-config.md#31-reference--citation-form).

One further form is opt-in: with `[reference] inline_note_layout` set to a layout and `inline_note_layout_check = "error"`, each line of a citation site that carries a citation and does not match the configured form is an error anchored at that line ([§FS-inline-citation-style.3.3](FS-inline-citation-style.md#33-inline_note_layout--where-the-citations-sit), [§FS-inline-citation-style.4.4](FS-inline-citation-style.md#44-warnings-and-errors--opt-in-layout-deviations)). The same deviation is a warning under `inline_note_layout_check = "warn"` (§4.4) and silent at the default `off`. It is the one member of this rule that anchors per line rather than at the site's first line, because a layout deviation is a property of the line an author has to edit.

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

### 3.13 Number-only shorthand citation

A recognized shorthand citation (§1.2) persisted in a scanned file. The shorthand is authoring sugar, not a stored citation grammar, so the site is reported and the report carries the replacement text — the fix is mechanical, and `grund fmt --write` applies it in bulk ([§FS-fmt.2.4](FS-fmt.md#24-shorthand-to-canonical)).

**Where `fmt` may not rewrite, this rule does not fire.** A shorthand inside inline code, a Markdown link destination, or a source string literal is exempt from the resolving form of this error, because [§FS-fmt.2.3](FS-fmt.md#23-what-is-never-rewritten) forbids the rewrite there and an error whose only named fix the tool declines to perform is one a repository can never clear. The citation is untouched in every other respect — it resolves, `refs` lists it, and it keeps its declaration from being reported unused (§1.2). The exemption is for the *mechanical* form only: a shorthand matching zero or several declarations is still reported in those contexts, because that is a dangling reference rather than a formatting nit.

At most one *shorthand* finding per site, in one of three forms. Other rules judge the site on their own terms — a bad section (§3.2) or a forbidden direction (§3.12) is a separate fact about the same citation and is reported separately:

```
docs/notes.md:5: shorthand citation §FS-042; write §FS-042-user-login
docs/notes.md:6: shorthand citation §FS-999 matches no declaration
docs/notes.md:7: shorthand citation §FS-042 is ambiguous: FS-042-user-login, FS-042-user-logout
```

The candidate list in the ambiguous form is sorted and complete — `grund` names every match and resolves none, because choosing one would be a guess and `check` reports facts about the tree (§5, [§GOAL-agent-grounding.3](../goals.md#3-what-this-rules-out), [§REQ-no-wrong-citation.1](../requirements/REQ-no-wrong-citation.md#1-no-wrong-resolution)). Duplicate *numbers* are not otherwise an error: §3.3 catches duplicate full IDs, and a repo may legitimately hold `FS-042-user-login` alongside `FS-042-user-logout` as long as nothing abbreviates them. The marker rendered in the message is the configured one ([§FS-config.3.1](FS-config.md#31-reference--citation-form)), and the qualified form names its namespace (`<§>api/FS-042`, escaped here because this repo has no `api` member) so the replacement can be pasted as written.

An error rather than a warning or a suggestion: a warning leaves the exit code alone, so a repo could accumulate shorthand citations forever while CI stayed green, which is the state this rule exists to end ([§DF-number-only-citation-shorthand.2.3](../decisions/functional/DF-number-only-citation-shorthand.md#23-it-is-an-error-not-a-warning-or-a-suggestion)). Repos whose `[id] format` has no `{number}` or no `{slug}` never see this finding (§1.2).

### 3.14 Out-of-scope unresolvable citation *(`--full` only)*

Under `--full` (§1.3), a citation in a file outside the configured scope whose reference resolves to nothing: the ID is declared nowhere (§3.1), the declaration exists but the cited section does not (§3.2), the namespace alias is unknown (§3.8), or a number-only shorthand matches zero or several declarations (§3.13). The site is reported in the ordinary located-finding shape, with the tier named first and the rule's own message after it:

```
sim/world.py:12: outside [scan] include: unknown reference RES-061-world-arable-basin-screen
render/prompts.md:4: outside [scan] include: missing section DA-002-general-field-service-scope.1.4
```

- **An error, not a warning.** It moves the exit code to `1` like every other reference failure. A warning would leave `--full` exit-code-neutral, and a finding no CI run can fail on is one a repository accumulates behind forever — the argument §3.13 already makes. Nothing turns red without being asked: the flag is opt-in.
- **Only resolution is judged.** The style, placement, grounding, direction, and unused rules say how a project organizes the files it has chosen to govern, and `[scan] include` is exactly that choice; a directory nobody configured has agreed to none of them, so reporting them there would bury the findings that matter under a pile that does not.
- **Resolution sees the whole walk.** An out-of-scope citation whose declaration is also out of scope resolves normally. The tier reports references that point at *nothing*, not references that point outside the configured scope.
- **The mechanical shorthand rewrite is withheld** (§3.13). Its named fix is `grund fmt --write`, and `fmt` scopes by `[scan] include`, so out there the error would name a fix the formatter declines to apply — the same reason §3.13 withholds it at an unrewritable site. A shorthand that matches zero or several declarations is a resolution failure, not a formatting nit, and is reported.
- **A compound code per rule.** A finding here carries `out-of-scope-` followed by the code its in-scope equivalent carries: `out-of-scope-dangling`, `out-of-scope-missing-section`, `out-of-scope-unknown-project`, `out-of-scope-shorthand-citation`. A `--format=json` consumer ([§FS-errors.5](FS-errors.md#5-json-format)) then filters the tier by prefix and the rule by exact match, both on the `code` field the shape already carries; one code for all four would have left the rule readable only by parsing the message prose. The JSON shape gains no field.
- **The tier leads the message.** `outside [scan] include: ` comes first, before the rule's own text, because it is the fact that changes what to do: out there the usual fix is to widen the key, not to edit the citation, and a rule's own fix-it hint — `did you mean …?`, `or write <§>… if this is an illustration` — is likelier to be the wrong advice and would otherwise be read first. Naming the key is the whole remedy the message carries; it does not also spell out "widen `[scan] include`", because every finding in the tier would repeat the same sentence and §1.3 states the remedy once.
- **A wider walk can fail wider.** These findings are errors and move the exit code to `1`, but the flag also puts files the configured scope never touched into the walk, so one that cannot be read or decoded out there is reported as the §2 `error: <path>: <reason>` on stderr and the run exits `2` — "I could not read this" is a fact about the run, not about the tier, and it holds for a file in either scope. A tree whose plain `check` exits `0` can therefore exit `2` under `--full`; that is the wider walk reporting what it found, not a regression.

### 3.15 Shorthand citation in a numeric run

A number-only shorthand (§1.2) that resolves to exactly one declaration but sits glued to a second number, so `grund fmt` will not rewrite it ([§FS-fmt.2.4.1](FS-fmt.md#241-a-shorthand-in-a-numeric-run-is-not-rewritten)). Decided in [§DF-shorthand-numeric-run](../decisions/functional/DF-shorthand-numeric-run.md#df-shorthand-numeric-run-a-marked-shorthand-glued-to-another-number-is-a-numeral-not-a-citation).

```
docs/changelog.md:3: shorthand §SPEC-001 sits in a numeric run and was not rewritten; write §SPEC-001-checkout, or <§>SPEC-001 if these are old numbers
```

This is §3.13's site with a different verdict, so it takes §3.13's place there rather than adding a second finding — at most one *shorthand* finding per site still holds, and rules judging a different fact about the same citation still report alongside it.

- **Both exits are named.** `grund` cannot know which was meant and the author knows at a glance. If it was a citation, the canonical text is there to paste; if the numbers were a mapping, `<§>` is the escape for writing an ID without citing it (§2.3.1), offered in the same shape §3.1 uses for a dangling citation that might be an illustration.
- **An error, like §3.13.** A warning leaves the exit code alone, and a finding no CI run fails on is one a repository accumulates behind forever. Nothing turns green-to-red on upgrade: these sites are already §3.13 errors today, and what changes is that the message stops naming a fix that would corrupt the line ([§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)).
- **The rule this one is an exception to.** §3.13 withholds itself wherever `fmt` may not rewrite, because an error whose only named fix the tool declines to perform can never be cleared. That reasoning covers the [§FS-fmt.2.3](FS-fmt.md#23-what-is-never-rewritten) contexts, where the text is legitimate as it stands and no edit is wanted at all — and this finding is withheld there too, for that reason. It does not cover a run, where the line does need an edit and a person can make it in one keystroke. What has to hold is that every finding names a fix, not that every fix is `fmt`'s.
- **Only the resolving form.** A shorthand in a run matching zero or several declarations keeps its §3.13 message. That is a resolution failure, reported on its own terms, and a run is no reason to say less about it.
- **Withheld out of scope.** Under `--full` (§1.3) the site is outside `[scan] include`, where §3.14 withholds the mechanical shorthand rewrite for the same reason: `fmt` scopes by `include` too, so the finding would name an edit no run in that scope is asking for.
- **Code:** `shorthand-numeric-run` ([§FS-errors.5](FS-errors.md#5-json-format)).

### 3.16 Duplicate section path

Two or more numbered section headings inside one declaration claiming the same dotted path ([AR-scanner.2.2](../architecture/AR-scanner.md#22-section-detection)) — `## 1. Inputs` and `## 1. Outputs` under one `# FS-001-login`. Reported per §2.1 in §3.3's shape: one error anchored at the first heading in file order, with every other heading line named in the message.

```
docs/functional-spec/FS-001-login.md:5: duplicate section FS-001-login.1 (also declared at docs/functional-spec/FS-001-login.md:9)
```

This is §3.3 one level down. A section path is a citation target, so two headings claiming it give `§FS-001-login.1` two destinations, and picking one silently is the guess [§REQ-no-wrong-citation.1](../requirements/REQ-no-wrong-citation.md#1-no-wrong-resolution) forbids by name. Decided in [§DF-duplicate-section-path](../decisions/functional/DF-duplicate-section-path.md#df-duplicate-section-path-a-section-coordinate-names-one-heading-or-the-run-says-so).

- **Scoped to one declaration.** Section paths are addressed as `<ID>.<path>`, so the same `1.` under two different declarations is two distinct coordinates and not a finding. Only headings sharing a declaration collide.
- **Scoped to that declaration's body.** The headings judged are the ones inside the body [§FS-show.2.1](FS-show.md#21-whole-declaration-default) and [§FS-show.2.3.1](FS-show.md#231-what-counts-as-the-comment-block) delimit — in Markdown down to the next same-or-shallower heading, in a source file to the end of the comment block the declaration line opens. A `## 1.` further down the file — in the *next* item's doc-comment, or under a later unrelated heading — is not one of this declaration's sections: `grund <ID>.1` never reaches it, and reporting it would ask for a renumbering that changes what nothing points at. A stub ([§3.4](#34-broken-inline-spec-stub)) is one link line whose tail is a path rather than a body, so it declares no sections at all and is never reported here; the headings that count are the inline home's, which is also the file `grund <ID>.<path>` reads.
- **Independent of `[id] section_heading_levels`.** The mode ([§FS-config.3.3](FS-config.md#33-section-paths--arbitrary-nesting-depth)) governs how deep a heading must sit for the path it writes, which is a different fact; `## 1.` and `### 1.` under an H1 declaration both claim path `1` and are a duplicate in every mode, `"loose"` included. A heading that is both misplaced and duplicated yields §3.9's finding and this one — two facts, two findings.
- **The same record `show` reads.** This rule and [§FS-show.2.2.2](FS-show.md#222-ambiguous-section) answer from one recorded section set, so `grund <ID>.<path>` refuses exactly when this rule reports `<ID>.<path>` and returns a body exactly when it does not. Two readers that each decided for themselves would disagree — a fenced example, a heading past the end of the body — and a coordinate `check` calls clean but `show` will not resolve is [§REQ-no-wrong-citation](../requirements/REQ-no-wrong-citation.md#req-no-wrong-citation-a-citation-never-resolves-to-a-guess) failing quietly in the other direction.
- **Code:** `duplicate-section` ([§FS-errors.5](FS-errors.md#5-json-format)), carrying the same multi-site `sites` list §3.3 carries.

## 4. Warnings

### 4.1 Unused declaration

An ID that is declared but never cited. Reported as a warning, not an error — newly declared IDs may not yet have citations. Warnings never affect the exit code (§2).

A number-only shorthand citation that resolves counts here like any other citation (§1.2): a declaration abbreviated as `§FS-042` everywhere is cited, and reporting it as unused would state the opposite of the truth.

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

### 4.4 Inline note layout deviation *(opt-in)*

Off by default. When `[reference] inline_note_layout` names a layout and `[reference] inline_note_layout_check = "warn"` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)), every line of an inline citation site that carries a citation and does not match the configured form is reported as a warning, anchored at that line. Setting the key to `"error"` reports the identical message as an error instead (§3.10); `"off"`, or a `inline_note_layout` left at `any`, reports nothing. The form itself, the per-line rule, and the exemption for sites that carry no note live in [§FS-inline-citation-style.3.3](FS-inline-citation-style.md#33-inline_note_layout--where-the-citations-sit), and the channel table in [§FS-inline-citation-style.4.4](FS-inline-citation-style.md#44-warnings-and-errors--opt-in-layout-deviations).

Off by default because a layout is a house style rather than a correctness property, and the two levels exist so a repository can migrate on `warn` before it gates on `error` — the same ladder §4.2 gives the soft cap.

### 4.5 Nothing recognized

A walk that read at least one file and recognized **nothing in it** — no declaration and no citation — is §2.2's empty scan one step further in: the scope was right and the files were read, and the grammar matched none of their content. The usual cause is a docs tree whose headings are written for a different `[id] format` than the one configured ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) — `# FS-login: …` under the default `{kind}-{number}-{slug}` — which leaves every heading in the tree a non-declaration and the run's verdict `success` over a repository where nothing is grounded. Decided in [§DF-nothing-recognized](../decisions/functional/DF-nothing-recognized.md#df-nothing-recognized-a-run-that-recognized-nothing-says-so-and-says-it-as-a-warning).

`check` emits one CLI-level `warning:` line (§2.1.1) on **stderr**, naming how many files were read, the shape a declaration heading and a citation take under the configured format, and the configured `[[kinds]]` prefixes:

```
warning: nothing recognized — grund read 3 files and found no declaration and no citation in them. A declaration heading reads `# <KIND>-<NNN>-<slug>: <title>` and a citation `<marker><KIND>-<NNN>-<slug>`, under [id] format = "{kind}-{number}-{slug}" with <KIND> one of {AR, FS}. Either nothing is declared yet, or the headings are written to a different shape than that.
```

The shapes are rendered from the `[id] format` template, the same substitution [§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints) makes for the managed entrypoint block, and the citation shape carries the configured marker ([§FS-config.3.1](FS-config.md#31-reference--citation-form)). The closing sentence offers both readings of the fact, because the run cannot tell them apart without judging a line: a tree written to another format and a `grund init` scaffold nobody has declared in yet produce the identical report, and naming only the first would send a fresh adopter looking for a bug in a config that is fine. No example ID is built from `[id] number_pattern` and `[id] slug_pattern` and no corrected ID is proposed for any heading: `check` reports facts about the tree and the config (§3 vs §4), and an ID assembled from those patterns would be a guess at what they accept.

The question is asked **per project**, like §2.2: in a workspace ([§FS-workspace.5](FS-workspace.md#5-command-scope)) each member is judged against its own config, since one member's grammar mismatch says nothing about another's. It asks *recognized*, not *declared* — a member that only cites another member's specs declares nothing and is working as intended, so a citation anywhere in the project answers the question.

It is asked only of a run whose scope **is** that project's root (no path argument, or a path that resolves to it). A narrowed `grund check <dir>` is a slice the caller chose, and a slice holding no declaration and no citation is an answer rather than a misconfiguration — the claim this caution makes is about a whole project, and a run that read part of one cannot make it.

Like §2.2 it is a warning, and like §2.2 it is withheld from a run that has any other finding about the configured scope: the exit code stays `0` (a tree with nothing in it yet is the ordinary first day of a repository), and a report that already says something about that scope is not the silent verdict this rule exists to break. It inherits §2.2's two exceptions unchanged, and for the same reasons. A redundant-config pair (§4.3) is a fact about which file was read — and a repository mid-migration between the two config names is exactly where a mismatched `[id] format` hides, in the file that is no longer read. The out-of-scope tier (§3.14) is a fact about the tree beyond the scope, and a `--full` run that reports every citation out there while the configured scope holds nothing is the strongest form of this diagnosis, not a reason to withhold half of it. What it buys is the `success` marker — a warning stands in its place (§2.1), so the run that recognized nothing stops printing the same word as the run that checked everything.

The per-heading half — naming each heading that looks like a declaration and does not match — is [§RM-declaration-near-miss](../roadmap.md#rm-declaration-near-miss-warn-on-a-heading-that-looks-like-a-declaration-but-does-not-match-id-format) and §5, a different rule that needs a judgement about what any one line meant; this one is arithmetic over what the scan already recorded.

- **Code:** `nothing-recognized` ([§FS-errors.5](FS-errors.md#5-json-format)), with `path` and `line` null like every CLI-level diagnostic.

## 5. What grund does not check

See [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) — in particular [§FS-non-goals.1](FS-non-goals.md#1-markdown-link-validation) (markdown links / URLs), [§FS-non-goals.2](FS-non-goals.md#2-spelling-grammar-prose-quality) (spelling/grammar), and the convention that ID numbers are stable handles, not ordinal positions.

One near-miss `check` does not flag *today*: a heading shaped like `# <KIND>-…: <title>` whose ID does not match the configured `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) is simply not a declaration — invisible to `check`, to `grund list`, and to citation resolution, with no warning that something heading-shaped was ignored (the classic stumble: `# FS-login: …` under the default `{kind}-{number}-{slug}`). A tree in which *every* heading misses that way no longer passes silently — the run recognized nothing and says so (§4.5) — but the per-heading half, a non-heuristic "looks like a declaration" warning naming each one, is tracked under [§RM-declaration-near-miss](../roadmap.md#rm-declaration-near-miss-warn-on-a-heading-that-looks-like-a-declaration-but-does-not-match-id-format) — it would surface the mismatch, never guess the corrected ID (`check` reports facts about the tree, §3 vs §4). That is the *declaration*-side near miss; the citation-side one — a `§`-marked token in the shorthand shape — is no longer in this section, because §1.2 and §3.13 now recognize and report it.

## 6. Watch mode (`--watch`)

Status: planned — implementation tracked under [§RM-watch](../roadmap.md#rm-watch-implement-grund-check---watch).

When implemented, `grund check --watch [<path>]` will run the check once, then stay resident and re-run it whenever a file under the scanned tree (or the discovered `grund.toml`) changes. It is the editor-less counterpart to the optional LSP server ([§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)): the LSP integrates `grund` into an editor's diagnostics; `--watch` is the plain-terminal "every save" loop that [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) exists for. Until [§RM-watch](../roadmap.md#rm-watch-implement-grund-check---watch) lands, `grund check --watch` is a CLI error (`error: unknown flag \`--watch\``, exit 2).

- **Change detection.** Filesystem notifications where the OS provides them; a debounce window coalesces a burst of writes into one re-check. No polling loop is required, and there is no configurable interval — the watcher reacts, it does not sample.
- **Each run is a plain `grund check`.** Output and exit-status semantics of an individual run are exactly §2/§2.1 on the tree's state at that moment — byte-identical to what a non-`--watch` invocation would print ([§FS-errors.4](FS-errors.md#4-determinism)). Before each run the previous run's output is cleared so the terminal always shows the current report; with `--format=json` each run emits the same diagnostic NDJSON as non-watch mode, scoped to that run.
- **Lifecycle.** The process runs until interrupted (Ctrl-C / SIGINT). On interrupt it exits with the exit code of the most recently completed run (`0`/`1`/`2`), so `grund check --watch &` followed by a later signal is still a meaningful CI-ish probe. There is no TUI, no key bindings, no prompt — it is non-interactive per [§FS-non-goals.10](FS-non-goals.md#10-interactive-mode), just a re-printing checker. No network I/O ([§FS-non-goals.11](FS-non-goals.md#11-network-access-during-a-check)); the only files touched are the ones the walk already reads.
- **Scope.** `--watch` will be a `check` flag spelled as `grund check --watch [<path>]` ([§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions)). Other subcommands will not take it; a one-shot `grund fmt` or ID query has nothing to keep watching.
