# FS-check: grund validates every reference in a repo

The `check` command walks a repo and reports every violation of the grund reference scheme. Validation is explicit as `grund check [<path>]`; the bare `grund <ID>` default belongs to [§FS-show.1](FS-show.md#1-inputs). Serves [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) and [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible).

## 1. Inputs

- Optional path argument; defaults to the current directory. May be a directory or a single file (`grund check crates/grund-core/src/scanner.rs` scopes the scan to one file but still discovers the `grund.toml` by walking up — [§FS-config.1](FS-config.md#1-file-location-and-discovery)).
- The walked tree may contain markdown (`.md`) and source files (Rust, Go, Java, TS, Python, etc.).
- Optional `grund.toml` configuring marker, trigger, kinds, and skip lists per [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) ([§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up)).
- Optional `[workspace]` config; when present and `check` is run at the workspace root, `check` validates alias-qualified cross-project citations per [§FS-workspace](FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace).
- `--watch` is reserved for the planned resident checker (§6) and is not accepted by the current CLI.
- `--require-grounding` — turn the grounding check (§3.6) on for this run regardless of `[reference] require_grounding` in `grund.toml` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)). It only ever *adds* the check; it cannot switch off a config that already sets it. The flag and the key are **one knob**: the flag sets the same global default, so a `[[kinds]]` row that says `require_grounding = false` is still exempt under it ([§FS-config.3.4.8](FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level)). The row's word is the more specific one, and a run-level flag that overrode it would make the flag mean something the key cannot say. There is no flag for `grounding_level`; the unit comes from config only.
- `--suggestions` — emit the `should`-level citation-direction findings ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) on the suggestions channel (§2.3) for this run. The same config-plus-flag pattern as `--require-grounding`: the flag never adds an error or changes the exit code, it only surfaces the advisory `suggested-citation` / `discouraged-citation` records that the default run withholds.
- `--full` — walk the whole config root, past `[scan] include`, and report the references that resolve to nothing out there on their own tier (§1.3, §3.14). It only ever *adds* findings; the in-scope report is unchanged.
- `--format text|json` — output shape, per [§FS-errors.5](FS-errors.md#5-json-format). The global flags `--version` and `--help` are handled before any scan ([§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions)).

### 1.1 Recognized citations

Per [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger), a citation is the marker followed by an ID, e.g. `§FS-check.3.1`. The default marker is `§`; configurable via `grund.toml`.

In default mode (`[reference] strict = true`), only marker-prefixed citations are recognized — bare tokens are treated as plain text and do not trigger dangling-ref errors. Repositories that still rely on bare citations may set `[reference] strict = false` as a compatibility mode after checking the migration surface with `grund fmt --marker` ([§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk)).

Citations may appear in markdown prose, in source-file line/block comments, and in language doc-comments (Javadoc, JSDoc, Rustdoc, Python docstrings, etc.) — see [AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection) and [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) for the exact contexts. In source files, a **bare** ID-shaped token whose start column falls inside a string literal is not treated as a citation (the same deterministic quote-tracking rule `grund fmt` uses — [§FS-fmt.2.3.1](FS-fmt.md#231-string-literal-exclusion-rule), [AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection)), so an ID-shaped substring inside a runtime string does not raise a false dangling-ref. A marker-prefixed citation is recognized everywhere, string or not — the marker is the signal of intent. Markdown files have no string literals and the carve-out does not apply there.

In a Markdown file, the parallel carve-out is the link destination: a **bare** ID-shaped token whose start column falls inside an inline link's `(…)` half — `[text](…)` — is likewise not treated as a citation, because `grund fmt` never rewrites a link destination ([§FS-fmt.2.3](FS-fmt.md#23-what-is-never-rewritten)) and a finding whose only named fix the tool refuses to perform is one a repository can never clear ([§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation)). The exclusion is total, not just a withheld error: such a token is not a citation for `refs`, for unused-declaration counting (§4.1), or for grounding (§3.6) either. A marker-prefixed citation inside a link destination is unaffected — the marker is the signal of intent there too, exactly as it is inside a source-file string literal.

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

- **The walk covers the whole config root.** `[scan] exclude`, `.gitignore` and every other ignore file, hidden names — directory and file alike ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)) — workspace member boundaries ([§FS-workspace.6](FS-workspace.md#6-nested-project-boundary)), and `[scan] extensions` all still apply exactly as they do without the flag. A file type `grund` does not scan stays unscanned; widening `extensions` is a config decision, and one flag that widened both would make "what did this run read" unanswerable. A boundary is a *declared* member, not any nested `grund.toml`: a project directory the workspace never declared is ordinary tree to both walks, so `--full` reads it and judges its citations under *this* project's grammar and kinds. That is what the plain walk does with it too, but the flag is what makes it reachable by default — a vendored, generated, or example project belongs in `[scan] exclude`, or in `[workspace] members` if it is one of ours, before a run adds `--full`.
- **The wider walk reads a superset of the narrow one, each file once.** Every root `[scan] include` names is walked under `--full` too, whether or not `exclude`, an ignore file, or the hidden-directory rule would otherwise prune it: those three rules prune *descendants*, never the directory a walk starts at, so a gitignored, excluded, or hidden `include` root is read by the plain run and must be read here. Without that, `--full` could read *fewer* files than `grund check` and hide a finding instead of adding one. The hidden-name test on a **file** is not one of those three and does not follow this rule: it is asked of every entry rather than of every descent, so it reaches a root as readily as a descendant, and an `include` entry naming a hidden file — `docs/.notes.md` — is read by neither run ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)). Additivity survives it because the file is missing from both walks, not from one; what a reader must not conclude from the sentence above is that naming a hidden file in `include` brings it into scope. Overlapping roots — an `include` entry inside another, or inside the config root the flag adds — name one file once; a file read twice would be a declaration duplicated with itself (§3.3). "Once" is per *file*, not per path: an `include` root that is a symlink to a directory inside the config root, or a case alias of one on a case-insensitive filesystem, reaches its files under a spelling the config-root walk never produces, so a byte-identical compare cannot see the reread. The walk therefore starts at the `include` roots *before* the config root and keeps the first spelling of each file — the one `grund check` prints without the flag. Every in-scope line is the plain run's, character for character; `--full` only ever appends `outside [scan] include:` lines.
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

Lines that are about the run rather than a finding at a site in the repo — unknown subcommand, malformed flag, invalid `grund.toml` schema (when the config itself parses but a value is wrong), a per-file read failure mid-walk (§2), the empty-scan caution (§2.2), the citation-obligation caution (§2.2.1), the nothing-recognized caution (§4.5) — are emitted on **stderr**, never on stdout, as:

```
error: <message>
warning: <message>
```

These never carry the bare `<path>:<line>:` prefix a per-finding line wears (the one with no `error:`); the `error:` / `warning:` prefix is what distinguishes them from per-finding lines on stdout. A `grund.toml` schema error is the one CLI-level message that still points at a line — it is reported `error: <path>:<line>: <message>` ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)): the `error:` prefix keeps it CLI-level (stderr, exit `2`), but the `<path>:<line>:` inside the message text is the breadcrumb to the offending key, since a config file has one and a bad flag does not. CI scripts grep for the leading `error:` to detect launch-time failures. An `error:` always accompanies a non-zero exit; a `warning:` does not affect the exit code. In `--format=json`, a launch-time `error:` (bad flag, unreadable config) stays as raw text; a mid-walk per-file failure is one of the report's diagnostics and is rendered as JSON like the rest (on stderr, since it is `line`-less and not a graph finding — [§FS-errors.5](FS-errors.md#5-json-format)).

### 2.2 Empty scan

A walk that read **no scannable files** at all, and turned up no findings (no errors, no warnings — including the agent-entrypoint check of §3.5, which still runs and still reports even when nothing is scanned), is almost always a misconfigured scope rather than a clean repo. Rather than print nothing and exit `0` — which reads as "all clear" — `check` emits one CLI-level `warning:` line ([§FS-errors.2.2](FS-errors.md#22-cli-level-message)) to **stderr** — it is a caution about the run, not a finding about the repo, so it does not belong on stdout with the findings:

- when the scope is the repo root (no path argument, or `grund check .`) and `[scan] include` is set: the message names the `include` list and points at `grund.toml` / `grund init`, since the usual cause is a project whose sources live outside the default `docs/`, `e2e/`, `src/`;
- when an explicit path was given: the message names that path and the recognized extensions, since the usual cause is pointing `grund` at a tree with no `.md`/source files;
- when the explicit path is a **file whose own name begins with `.`** and whose extension is one `[scan] extensions` lists: the message names the hidden-name rule instead ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)). The extension list is the one rule that did *not* skip that file, so naming it sends the reader to edit config that was never the cause — the misdirection [§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) is about, since a blind spot a reader is pointed away from is one they cannot plan around. A hidden file whose extension is *also* unlisted keeps the message above: there the list is a true reason, and naming one of two causes would be its own misdirection. This arm answers for a handed **file** only — a handed *directory* whose only listed-extension content is hidden keeps the message above too, though the hidden-name rule is the sole reason it read nothing, because the walk does not record that it met candidates and rejected every one of them by name;

  ```
  warning: nothing to scan — `docs/.notes.md` is a hidden file. grund reads no file whose own name begins with `.`, whatever `[scan] extensions` says. Rename it, or move what needs checking into a file that is not hidden.
  ```
- when a `[workspace]` block put no project in scope at all — `include_root = false`, and every member it has is an optional one this checkout does not have ([§FS-workspace.2.2](FS-workspace.md#22-a-member-that-may-be-legitimately-absent)): the message says exactly that. Both of the messages above would be false here, because the walk never looked under `[scan] include` and the tree `grund init --docs` scaffolds is not what is missing. This one names no remedy either, for the reason §4.9's announcement names none: nothing is misconfigured, and the only thing that changes the answer is a fuller checkout.

This is a warning, not an error: the exit code stays `0` (a genuinely empty tree is not a failure), `--format=json` emits the warning as one diagnostic JSON object on stderr (the same stream as the text `warning:` line — it is not part of the findings on stdout), and a repo that *does* have a stale `AGENTS.md` block or any other finding **about the configured scope** gets that finding (on stdout) and **no** empty-scan notice. Three findings are not about that scope and do not suppress it: the redundant-config pair (§4.3), which is about which file the run read rather than what it walked — a repository mid-migration must not lose the scope diagnostic because it also has a config pair — the out-of-scope tier (§3.14), which is about the tree *outside* the scope, and the absent-member announcement (§4.9), which is about the namespaces the run skipped rather than the scope it walked: the block whose last project went missing is exactly the run that has nothing to read, and it must not lose the line saying so to the line saying why. The tier is the case the caution is worth most: it says where the citations actually are, and the caution says the config has not been told. This is the friendliness-first counterpart to the explicit success marker ([§GOAL-friendliness-first.1](../goals.md#1-hard-requirements)): the run that scanned nothing is the one case where `success` would be the wrong answer.

#### 2.2.1 Citation-direction obligation applies to nothing

When `[citations.<kind>]` contains at least one `must` or `should` obligation, but the citing kind has no unit for the obligation to evaluate, `check` emits one CLI-level `warning:` on **stderr**. This is a run-level fact, not a finding at a repository site: the warning has no path, line, or sites, and it does not change the exit code or emit `success` in text mode. In `--format=json`, it is one standard warning diagnostic on **stderr** with `path`, `line`, and `sites` all `null` ([§FS-errors.5](FS-errors.md#5-json-format)). Its stable diagnostic code is `empty-citation-obligation`.

The warning is emitted once per configured kind when all of these conditions hold:

1. The table has a non-empty `must` or `should` list. A table containing only `must-not` or `should-not` entries has no obligation unit and does not warn. A kind with both levels is named by `must`; a `should`-only table is named by `should`.
2. The citing kind has a `folder` home, and that kind is walked. File homes, the homeless kind, and `scan = false` kinds do not warn.
3. The run successfully scanned at least one file that belongs to that folder, excluding the folder's entry file. For a citable kind, the entry is its effective `index` (`README.md` when the key is omitted); for a non-citable kind, the entry is the literal `README.md`. `index = false` excludes no file. Files not successfully scanned, files outside the home, and files hidden or excluded by the walk do not count.
4. The ordinary obligation-unit derivation produced zero units: no declaration unit for a citable kind, or no citation-carrying scanned-file unit for a non-citable kind.

The messages distinguish the two kinds of missing unit:

```text
warning: [citations.SKILL] must applies to nothing — skills/ declares no SKILL ID; did you mean `citable = false`?
warning: [citations.skill] must applies to nothing — no scanned file in skills/ carries a citation; set require_grounding = true on the skills/ row to make that an error
```

The non-citable message keeps its `require_grounding` half only where that row's **effective** `require_grounding` is off ([§FS-config.3.4.8](FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level)) — it is advice, and where the row already grounds, the setting it asks for is made and this run is already reporting what it caught (§3.6). There the message stops at the fact:

```text
warning: [citations.skill] must applies to nothing — no scanned file in skills/ carries a citation
```

The folder membership and entry-file comparisons use the same normalized home matching as citation-source classification. The warning is independent of other findings: another warning or error does not suppress it. Workspace checking asks the question separately for each member, against that member's config and scanned files ([§FS-workspace.5](FS-workspace.md#5-command-scope)). An explicit path still evaluates the files it scans, so a path such as `grund check skills` can earn this warning; it does not broaden the path to unrelated homes.

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

Off by default. Two config keys decide it, each written in `[reference]` as the default for every `[[kinds]]` row and settable on the row itself ([§FS-config.3.4.8](FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level)): `require_grounding` says **whether** a place's files must be grounded, and `grounding_level` says **what the unit is** inside each of them. `grund check --require-grounding` (§1) sets the same global default, and an explicit `require_grounding = false` on a row wins over it.

A unit is **grounded** when it contains at least one recognized citation (§1.1) whose ID resolves to a declaration — **or**, in a source file outside every non-citable home, when it declares an ID inline (a spec home is grounded in the spec it *is*, [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)). A unit that is neither is an error:

```
src/foo.rs:1: ungrounded source file: no § citation to a declared ID
```

The marker in the message is the configured one ([§FS-config.3.1](FS-config.md#31-reference--citation-form)). A unit whose only citation is dangling (§3.1) is *not* grounded — it gets both findings; fixing the citation clears both.

This is a pure function of `(tree, config)` like every other `check` rule ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)): it reads no git history ([§FS-non-goals.6](FS-non-goals.md#6-decision-database-audit-log-history-tracking)) and parses no code ([§FS-non-goals.3](FS-non-goals.md#3-code-ast-parsing)) — "source file" is decided by extension, a unit by heading level or comment indentation, and "grounded" by the citations the scanner already collected. It is the floor of the grounding discipline — the verification-at-rest layer of [§GOAL-agent-grounding.1](../goals.md#1-the-three-layers), on top of which `grund cover` exposes the citation graph ([§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)) and [§RM-cochange-gate](../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test) tracks the diff-aware co-change gate. Decided in [§DF-require-grounding](../decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec).

#### 3.6.1 Which files a row governs

Every scanned file resolves to exactly one `[[kinds]]` row, and that row's effective `require_grounding` decides whether the file is checked at all:

- **A non-citable kind's home** ([§FS-config.3.4.1](FS-config.md#341-citable--kinds-that-declare-no-ids)) governs **every** scanned file in it, `.md` included — a `folder` home's files, or the one document of a `file` home ([§FS-config.3.4](FS-config.md#34-kinds--recognized-kinds)), which is a place a maintainer declared like any other and takes both keys on its row ([§FS-config.3.4.8](FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level)).
- **A citable kind's folder home** governs the **source files** in it — a file the walk reads whose extension is not `.md` ([AR-scanner.1](../architecture/AR-scanner.md#1-tree-walk)).
- **The homeless kind** ([§FS-config.3.9.2](FS-config.md#392-the-homeless-kind)) governs the source files no home claims, and a file claimed by two overlapping homes falls to it as well, the way its citing side already does ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)).

So Markdown is exempt except inside a non-citable home, and that exception is a home rather than an extension. The Markdown exemption reasons about implementation versus document; a non-citable home is neither guess — it is a directory the maintainer declared matters, and it is usually *all* Markdown, a skill, a runbook, a prompt library. Inheriting the exemption there would switch the rule off exactly where it was turned on. An unwalked home ([§FS-config.3.4.7](FS-config.md#347-scan--a-place-that-is-listed-not-walked)) has no scanned files, so the rule never reaches it — which is why `require_grounding = true` on such a row is a config error.

A repository that sets only `[reference] require_grounding = true` and configures no non-citable kind sees this rule exactly as it did before these keys existed: every row inherits the global `true`, every level is `1`, and the unit is the file.

#### 3.6.2 The unit

`grounding_level` is an integer in Markdown heading levels ([§FS-config.3.4.8](FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level)). Each level **contains the one below it**, so the file itself is always a unit and nothing passes vacuously for lacking structure.

**In a Markdown file**, level `L` makes a unit of the whole file and of every heading subtree whose level is between `2` and `L`. A subtree runs from its heading to the line before the next heading at the same or a higher level, so a parent is satisfied by any descendant and a leaf must cite directly; text before the first heading belongs to the file rather than to a section. At level `1` there are no section units and the file is the only one, which is the unit every config had before the key existed. A file with no heading at the level is one unit — the file — for the same reason.

**In a source file** there are two ranks, and they are read by indentation rather than by syntax ([§FS-non-goals.3](FS-non-goals.md#3-code-ast-parsing)): at level `2` every **unindented** doc-comment block is a unit — a parse-free stand-in for a top-level item, which holds across Rust, Python, Java, Go, and Kotlin — and at any higher level every doc-comment block is. What counts as a doc comment is the per-language rule of [§FS-inline-citation-style.1.1](FS-inline-citation-style.md#11-doc-comments-are-not-sites), already read once per file by the scanner. The file is a unit at every level, as in Markdown.

The **inline-declaration escape** applies per unit: a doc-comment block that declares an ID is grounded by that declaration, and so is the file it sits in. It has no effect inside a non-citable home, where a declaration is a misplaced declaration to begin with (§3.7) — there the only way to ground a unit is to cite one.

#### 3.6.3 Findings

A finding is anchored at its unit — line 1 for a file, the heading line for a section, the block's first line for a doc comment — and names the unit and, when the unit sits in a non-citable home, the home:

```
src/foo.rs:1: ungrounded source file: no § citation to a declared ID
skills/triage/SKILL.md:1: ungrounded file in kind home skills/: no § citation to a declared ID
skills/review/SKILL.md:14: ungrounded section `## Steps` in kind home skills/: no § citation to a declared ID
src/walk.rs:41: ungrounded doc-comment: no § citation to a declared ID
```

Section units arise only inside a non-citable home, since that is the only place Markdown is governed, so a section finding always names one. Every failing unit is reported: a file that cites nothing at level `2` earns the file finding *and* one per section, which is what "each level contains the one below it" means on the reporting side — the file is genuinely ungrounded, and so is each of its sections.

### 3.7 Misplaced declaration (configured kind home)

A kind configured with `file = "<path>"` in [[kinds]] ([§FS-config.3.4](FS-config.md#34-kinds--recognized-kinds)) is a *single-file kind* — every declaration of that kind must live in that exact document. A declaration whose H1/H2 is found in any other scanned file is reported as a misplaced-declaration error, anchored at the declaration line:

```
docs/notes.md:42: GOAL-foo must be declared in docs/goals.md (single-file kind)
```

Stubs (`# <ID>: [<text>](<path>)`) are exempt from this exact-file requirement — they are pointers from a kind's home folder to an inline declaration elsewhere, which is a multi-file-kind feature; a single-file kind has no stubs because there is no folder to redirect from. This is the canonical mechanism that keeps `GRUND`, `GOAL`, and `RM` declarations consolidated in their respective documents, and what makes "one file, all goals inline" a checked invariant rather than a convention.

Every configured `file` and `folder` also acts as a declaration-home boundary. If a declaration line appears in a file that belongs to exactly one configured kind home, the declaration's kind must match that home kind. A `file` home matches only that exact path; a `folder` home matches files below that directory. A different-kind declaration is reported as a misplaced-declaration error, anchored at the declaration line and naming the declared kind, the expected home kind, and the configured home:

```
docs/functional-spec/FS-lsp.md:42: AR-router declares kind AR inside FS home docs/functional-spec
```

A **non-citable home** ([§FS-config.3.4.1](FS-config.md#341-citable--kinds-that-declare-no-ids)) admits no declaration of any kind. It has no kind an author could have declared instead, so the message names the place and says why rather than pointing at a kind that does not exist:

```
skills/review/SKILL.md:1: FS-review must not be declared in skills/ (not a citable home)
```

That is this rule working as designed, not a gap in it: `citable = false` says the directory is a place, and a place with a declaration in it is one of the two facts in conflict.

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

A citation site in a code comment that violates the configured inline citation style — `inline_style = "citation-only"` with prose present, an inline note that exceeds `inline_note_max_lines`, or one that exceeds `inline_note_max_columns`. A site is an *inline* comment block; a doc-comment block is never one, so nothing in this rule reaches a citation written inside a `///`, a `/** … */`, a docstring, or a comment documenting the definition below it ([§FS-inline-citation-style.1.1](FS-inline-citation-style.md#11-doc-comments-are-not-sites)). The full mode and budget contract, and how multi-cap violations split into multiple findings, lives in [§FS-inline-citation-style.4.1](FS-inline-citation-style.md#41-errors--hard-caps). The schema for the controlling keys is in [§FS-config.3.1](FS-config.md#31-reference--citation-form).

One further form is opt-in: with `[reference] inline_note_layout` set to a layout and `inline_note_layout_check = "error"`, each line of a citation site that carries a citation and does not match the configured form is an error anchored at that line ([§FS-inline-citation-style.3.3](FS-inline-citation-style.md#33-inline_note_layout--where-the-citations-sit), [§FS-inline-citation-style.4.4](FS-inline-citation-style.md#44-warnings-and-errors--opt-in-layout-deviations)). The same deviation is a warning under `inline_note_layout_check = "warn"` (§4.4) and silent at the default `off`. It is the one member of this rule that anchors per line rather than at the site's first line, because a layout deviation is a property of the line an author has to edit.

### 3.11 Missing required citation

When `[citations]` ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) sets a `must` obligation for a citing kind, every top-level declaration of that kind must carry at least one citation satisfying each `must` entry, anywhere in its body. A declaration that does not is an error anchored at the declaration line, naming the unmet target:

```
docs/architecture/AR-router.md:1: AR-router must cite FS or GOAL (citation direction)
```

The body extent and the citing-side classification come from the scanner ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)); the obligation pass is [AR-checker.2.9](../../crates/grund-core/src/checker.rs). A **homeless-kind** obligation ([§FS-config.3.9.2](FS-config.md#392-the-homeless-kind)) — `code`, or whatever the project named it — is per file rather than per declaration — a source file that contains at least one citation but none satisfying the obligation is the error, anchored at line 1.

**A non-citable kind's obligation is per file too** ([§FS-config.3.4.1](FS-config.md#341-citable--kinds-that-declare-no-ids)), and its unit is every scanned file in the kind's home that carries at least one citation — **`.md` included**, unlike `code`. Obligations attach to declarations, and a kind that declares nothing would otherwise yield no units at all and let `must` pass vacuously; inheriting `code`'s Markdown exemption would do the same thing a second time, since such a home is usually all Markdown. The finding names the **home**, because the unit has no ID to print:

```
skills/review/SKILL.md:1: skills/ must cite FS (citation direction)
```

**Both per-file units follow the row's `grounding_level`** ([§FS-config.3.4.8](FS-config.md#348-require_grounding-and-grounding_level--grounding-per-place-and-per-level)): *whether* a place's files must cite and *what* they must cite are asked of the same thing (§3.6.2). At level `2` the unit of a non-citable Markdown home is every `##` subtree that carries a citation, and of a source file every unindented doc-comment block that does; the file stays a unit at every level, satisfied by any citation under it. A row at level `1` — which is every configuration written before the key existed — sees no change, and a citable kind's unit stays its declaration at every level, a declaration already being a unit inside a file.

**Every failing unit is reported**, as it is for grounding (§3.6.3): at level `2` or above a file whose citations satisfy no `must` entry earns the finding on the file unit *and* one on each section unit that satisfies none either. The reason is the same — the file genuinely cites no such target, and neither does the section — and the two lines differ only in the anchor, which is what tells the reader whether the miss is local to one section.

Units are still built from citations, so a file carrying none produces no unit and `must` cannot fire on it — except that a walked folder with real non-entry content now earns the run-level warning of [§FS-check.2.2.1](FS-check.md#221-citation-direction-obligation-applies-to-nothing). The same zero-unit boundary [§FS-config.3.9.2](FS-config.md#392-the-homeless-kind) states for the homeless kind remains intentionally unwarned. In a non-citable home `require_grounding` closes the per-file grounding hole (§3.6): there the grounding rule follows the home rather than the file extension, so "cite something" and "cite an `FS`" are two keys that compose, while the warning of §2.2.1 points at the row's key when grounding is off. An `E2E`-kind obligation ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) is per case declaration, can be satisfied by the case's `spec.refs` manifest entries, and remains an error when the case has no scanned citations or matching manifest reference. The parallel `should` obligation is not an error; it is a suggestion (§2.3).

### 3.12 Forbidden citation

When `[citations]` ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) sets a `must-not` prohibition for a citing kind, every citation site of that kind to a prohibited target is an error anchored at the citation site:

```
docs/functional-spec/FS-login.md:42: FS must not cite AR (citation direction)
```

The citing kind is the site's resolved `source_kind` ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)), named the way §3.11 names it — by kind for a citable kind, by **home** for a non-citable one (`skills/ must not cite AR`), and by name for the homeless kind, which has no home to name it by ([§FS-config.3.9.2](FS-config.md#392-the-homeless-kind)). The cited kind and namespace come from the citation token, matched against the rule's namespace grammar ([§FS-config.3.9.3](FS-config.md#393-namespace-matching)). The prohibition pass is [AR-checker.2.10](../../crates/grund-core/src/checker.rs). The parallel `should-not` prohibition is not an error; it is a suggestion (§2.3). The sanctioned way to keep a discouraged downward pointer is a plain Markdown link, which is not a citation under `strict = true` and so is exempt from this rule.

### 3.13 Number-only shorthand citation

A recognized shorthand citation (§1.2) persisted in a scanned file. The shorthand is authoring sugar, not a stored citation grammar, so the site is reported and the report carries the replacement text — the fix is mechanical, and `grund fmt --write` applies it in bulk ([§FS-fmt.2.4](FS-fmt.md#24-shorthand-to-canonical)).

**Where `fmt` may not rewrite, this rule does not fire.** A shorthand inside inline code, a Markdown link destination, or a source string literal is exempt from the resolving form of this error, because [§FS-fmt.2.3](FS-fmt.md#23-what-is-never-rewritten) forbids the rewrite there and an error whose only named fix the tool declines to perform is one a repository can never clear. The citation is untouched in every other respect — it resolves, `refs` lists it, and it keeps its declaration from being reported unused (§1.2). The exemption is for the *mechanical* form only: a shorthand matching zero or several declarations is still reported in those contexts, because that is a dangling reference rather than a formatting nit.

**A Python docstring is not a string literal for this exemption.** Its delimiters are doc-comment syntax, so the question is asked of the docstring's content ([§FS-fmt.2.3.1](FS-fmt.md#231-string-literal-exclusion-rule)) and a shorthand anywhere inside one — the opening line, a one-line docstring, an interior line, the closing line — is reported and rewritten exactly like one in a `#` comment. A shorthand inside a `"…"` or `'…'` literal on a **code** line is what stays exempt.

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

### 3.17 Index entry is not a link

An index entry (§3.18) that is present as a bare citation rather than a full Markdown link, reported at **the citation's line in the index**:

```
docs/discussions/README.md:12: index entry §DISC-external-ticket-resolvers is not a link; unchecked in grund 0.11.0, an error in 0.12.0 — run `grund fmt --write`
```

The index is a *file* index: its job is to get a reader from the folder to the declaration, and a bare `§<ID>` in it is a promise the reader cannot follow. The required form is exactly the link `grund fmt --cross-refs` writes ([§FS-fmt.6.2](FS-fmt.md#62-form)) — the relative path to the declaration's home, plus the heading anchor under the active `anchor_format`. That is the canonical target, not "has an anchor": a declaration whose home is a source file links to the bare file path with no anchor, and `anchor_format = "none"` drops anchors everywhere. `docs/architecture/README.md` already carries that case, and it is correct as written.

`check` requires the **shape** — the citation wrapped as `[§<ID>…](<target>)` — and never the target. A wrap's URL is re-derived on every `grund fmt --cross-refs` pass ([§FS-fmt.6.3](FS-fmt.md#63-idempotency-and-re-derive)), so a heading rename that rots an anchor is a one-line `fmt` diff rather than a second finding here, and re-deriving it in `check` would put the anchor algorithm in a second command for no new coverage.

This half is an **error on arrival**, under [§REQ-backwards-compatibility.3](../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations): the message names the versions the verdict moved between, the fix is one documented command the tool ships, `grund fmt --write`, and the release notes it. That licence is only honest while the named command is one that would actually act here, which is what the next paragraph is for.

**Only a citation `fmt` would wrap reaches this rule.** The bare form this reports is, exactly, an occurrence the next `grund fmt --write` turns into the link above:

- in the index file, which is a Markdown file by construction — `index` must name one ([§FS-config.3.4](FS-config.md#34-kinds--recognized-kinds)) because `--cross-refs` runs on `.md` files only ([§FS-fmt.6.1](FS-fmt.md#61-scope));
- **marker-prefixed**, because without `--marker` the link pass leaves a bare token bare ([§FS-fmt.6.5](FS-fmt.md#65-interaction-with---marker)). `grund fmt --write --marker` *would* reach an unmarked token — but only by marking every bare citation in the tree, which is the repository-wide style choice a project on `[reference] strict = false` has already declined ([§FS-config.3.1](FS-config.md#31-reference--citation-form)). A finding may name a command that repairs it, not one that changes something else on the way; the same objection [§DF-index-always-linkified](../decisions/functional/DF-index-always-linkified.md#df-index-always-linkified-the-cross-reference-pass-always-runs-on-a-kinds-index-file) raises against `--cross-refs --write` as a fix;
- outside every zone `fmt` never writes in ([§FS-fmt.2.3](FS-fmt.md#23-what-is-never-rewritten), [§FS-fmt.6.4](FS-fmt.md#64-what-is-never-wrapped)): an inline-code span, a Markdown link destination — a bare ID-shaped token inside `](…)` is a URL, not an entry — a fenced block, and a declaration heading line;
- and **naming a section that exists**, when it names one at all. The pass has to compute a link target ([§FS-fmt.6.2](FS-fmt.md#62-form)), and a citation whose section no declaration declares has none, so `fmt` passes over the line. Such a citation is already reported by §3.2, and adding a second finding whose named command answers `rewrote 0 lines` would be the trap this paragraph exists to avoid.

Anything else in the index is **not an entry**: it neither satisfies §3.18 nor is reported here, and the ID falls to §3.18, whose fix is a human edit rather than a command. The alternative is an error whose named fix the tool declines to perform, which is an error a repository can never clear — the same trap §3.13 stays out of, and by the same predicate ([§DF-index-entry-form.2.3](../decisions/functional/DF-index-entry-form.md#23-one-link-per-id-not-every-mention)). The condition runs one way only: an entry that already *is* a link satisfies §3.18 whatever `fmt` would do with it. Off strict mode, where an unmarked token is a citation ([§FS-config.3.1](FS-config.md#31-reference--citation-form)), that means a hand-written `[FS-x](…)` around one is a correct entry and is left alone; under `strict = true` the same line carries no citation at all, so the ID has no entry and is §3.18's.

For one release the two halves of the entry contract disagreed about severity — the greater offence warned while the lesser errored — and what decided that was having a fix command rather than the size of the offence ([§DF-index-compatibility-ramp](../decisions/functional/DF-index-compatibility-ramp.md#df-index-compatibility-ramp-a-findings-ramp-follows-its-fix-command-not-the-size-of-the-offence)). The inversion closed when §3.18's ramp did, and one verdict now covers both halves.

Only an ID that already has an entry reaches this rule; an ID with none is §3.18's, and one cause never yields both findings. Where several citations of one ID sit in the index and none is a link, the finding anchors at the first of them in file order.

- **Code:** `unlinked-index-entry` ([§FS-errors.5](FS-errors.md#5-json-format)).

### 3.18 Declaration missing from its kind's index

A kind configured with a `folder` and an index file ([§FS-config.3.4](FS-config.md#34-kinds--recognized-kinds)) promises that the index lists that folder's declarations; nothing verified it before. Every covered declaration the index does not name is one error, anchored at the **declaration's heading** and naming the index file:

```
docs/decisions/functional/DF-md-link-emission.md:1: DF-md-link-emission is not listed in docs/decisions/functional/README.md — became an error in grund 0.13.0
```

**Which kinds are covered.** Folder kinds that declare IDs. A `citable = false` kind ([§FS-config.3.4.1](FS-config.md#341-citable--kinds-that-declare-no-ids)) has no declarations, so it has no index and this rule never reaches it — which is why setting `index` on one is a config error rather than a silent no-op ([§FS-config.3.4.2](FS-config.md#342-index--the-kinds-index-file)).

**Which declarations are covered.** Every ID of that kind with at least one declaration site anywhere under `folder` — the whole subtree, not its top level, because a kind's folder routinely holds a directory per topic or per year (`DISC`'s proposals all live in `docs/discussions/proposals/`). A stub-and-inline pair collapses the way [§FS-list.2](FS-list.md#2-behaviour) collapses it: the stub under `folder` is what puts the ID in the folder, and **one** entry for the ID satisfies the rule — pointing at wherever the body lives, which for an inline home is the source file. A declaration of some *other* kind sitting inside the folder is a misplaced declaration (§3.7) and is not additionally demanded here.

An index may also **enroll one external inline declaration directly**, with no stub under `folder`. The enrollment is deliberately a stricter form than an ordinary entry: an unqualified, marker-prefixed citation of the bare ID (no section) whose declaration is in a non-Markdown source file outside `folder`, wrapped as a Markdown link whose destination is exactly the one `grund fmt --cross-refs` derives from that index to the source home ([§FS-fmt.6.2](FS-fmt.md#62-form)). The same canonical link is both the act of membership and the satisfying entry, so `check` requires no third artifact. Where two kinds share one `folder` and one `index`, each enrolls its own: the link is matched to the kind its ID names, so configuration order never lets one kind hide another's external entry. `grund show` and `grund list` still see the source declaration as the only home; enrollment creates no declaration record and no synthetic stub ([§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments), [§FS-list.2](FS-list.md#2-behaviour)).

Every condition distinguishes enrollment from surrounding prose. A foreign-kind or qualified citation, a citation of a section, a **number-only shorthand** (§1.2) — enrollment is the *persisted* whole ID, and a shorthand stays authoring sugar until `grund fmt --write` expands it ([§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation)) — an unlinked mention, a link **nested inside another link's destination** or any other zone `fmt` never writes in (§3.17), a link with a different destination, and a link to a Markdown declaration outside `folder` are ordinary references. They neither enroll the ID nor become navigational for §4.1. A marker-prefixed bare-ID mention that `grund fmt --cross-refs` turns into the exact canonical link becomes an enrollment when that form is written — the stored link is the unambiguous signal. Removing it removes the external membership, so there is no missing-entry finding for an external declaration that the index no longer claims. Decided in [§DF-index-entry-form.2.7](../decisions/functional/DF-index-entry-form.md#27-a-canonical-bare-id-link-enrolls-an-external-inline-declaration).

**What an entry is.** For an ID covered by a declaration under `folder`, one recognized citation (§1.1) of the ID in the index file, written as a full Markdown link. The two conditions are the entry's contract and either one unmet is a finding: this rule is the first, and §3.17 is the second. For an external inline declaration, the canonical link above establishes coverage and satisfies the entry simultaneously.

Between them sits a third case, and it lands here. A citation `grund fmt --write` would not wrap — inside an inline-code span or a Markdown link destination, in a fenced block, on a declaration heading line, written without the marker, or naming a section no declaration declares ([§FS-fmt.2.3](FS-fmt.md#23-what-is-never-rewritten), [§FS-fmt.6.4](FS-fmt.md#64-what-is-never-wrapped), [§FS-fmt.6.5](FS-fmt.md#65-interaction-with---marker), [§FS-fmt.6.2](FS-fmt.md#62-form)) — is not an entry at all. An index that mentions the ID only that way is reported *here*, where the fix is to write an entry, and never under §3.17, where the command the message names would decline to act. §3.17 lists the conditions in full.

**And nothing more.** Layout is free: table or list, grouped or flat, in any order, with any prose around it. `docs/functional-spec/README.md` groups its 21 entries under six curated headings, and a rule that dictated a table would break the best index in the tree. One link per ID is enough — every other occurrence of the ID in the index is untouched and is never a finding.

**A missing index file** is this same finding class, once per declaration in the folder: a folder whose index nobody wrote is the strongest form of the same fact, not a different one. It is also why the finding is anchored at the declaration and not at the index — an index file that does not exist has no line to point at, and every declaration has one. The message says which way it failed, in a parenthesis after the file name: `(the index file does not exist)`, `(the index file is a directory)` for a path that is one, and `(the index file could not be read)` for a file that is there and would not open. Three phrasings rather than one because "does not exist", said about a directory that plainly does, is a diagnosis the reader has to argue with before they can act on it.

**A run that cannot see the index does not judge it.** Which IDs the index names comes from the scan, while the form of each entry comes from re-reading the file, and the two can disagree about whether the index was read at all: a narrowed `grund check <one-file>` (§1.3), or an index the `[scan]` set excludes, leaves the index unscanned while the declarations under the folder are still in view. Reporting every one of them as unlisted would be a finding about the scope, not about the tree, so this rule is skipped for an index file the run did not scan. An index file that is *not there* — missing, or a directory wearing the name — is a fact about the tree and is still reported.

**An error, because the deadline the warning named has arrived.** No `grund` command writes a missing entry — rendering the index is not a pass `fmt` has — so this rule never had [§REQ-backwards-compatibility.3](../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations)'s licence for a same-release verdict flip, and took [§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)'s deprecation path instead. It arrived in `0.12.0` as a warning whose own text named the release it would become an error in, `0.13.0`; that release is this one, and the ramp ends here. So the finding is an error like every other in this section — it contributes to the exit code (§3) and it stands in place of the `success` marker (§2.1) — and the deadline clause is spent: a release still ahead is a date a reader can act on, and one that has arrived is not. What replaces it reports rather than promises, `— became an error in grund 0.13.0`, which is the past-tense half of the closed vocabulary [§FS-distribution.4.2](FS-distribution.md#42-a-release-may-not-contradict-the-releases-the-trees-own-messages-name) defines. A landed ramp names its release for the same reason a pending one names its deadline, and for one more: that clause is the only record of the flip a user reads without the changelog, and it is what makes the release path able to refuse a version this error would contradict. What the ramp bought is what it was for: a repository that never listed its declarations was told, by the tool, in the release before this one, exactly which run would start failing.

Decided in [§DF-index-entry-form](../decisions/functional/DF-index-entry-form.md#df-index-entry-form-an-index-entry-is-one-full-link-per-id-and-nothing-else-about-the-page), [§DF-index-compatibility-ramp](../decisions/functional/DF-index-compatibility-ramp.md#df-index-compatibility-ramp-a-findings-ramp-follows-its-fix-command-not-the-size-of-the-offence), and [§DF-index-not-an-inbound-citation](../decisions/functional/DF-index-not-an-inbound-citation.md#df-index-not-an-inbound-citation-an-index-entry-is-navigation-not-use).

- **Code:** `missing-index-entry` ([§FS-errors.5](FS-errors.md#5-json-format)).

## 4. Warnings

### 4.1 Unused declaration

An ID that is declared but never cited. Reported as a warning, not an error — newly declared IDs may not yet have citations. Warnings never affect the exit code (§2).

A number-only shorthand citation that resolves counts here like any other citation (§1.2): a declaration abbreviated as `§FS-042` everywhere is cited, and reporting it as unused would state the opposite of the truth.

A citation that is a kind's own **index entry** (§3.18) does not count here. An index names every declaration in its folder by construction, so counting its entries would leave every ID in an indexed folder permanently cited and delete the signal this warning exists to give ([§DF-index-not-an-inbound-citation](../decisions/functional/DF-index-not-an-inbound-citation.md#df-index-not-an-inbound-citation-an-index-entry-is-navigation-not-use)). The exclusion is exactly the entry: a citation in an index file of an ID whose home lies *outside* that folder is an ordinary citation and counts like any other **unless that exact site is the canonical link that enrolls an external inline declaration** (§3.18). Other citations of the enrolled ID on the same page still count. `grund refs` is unaffected and still lists every index entry — they are real citations, and a reader asking who points at an ID wants to be told that its index does.

`E2E` declarations ([AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations)) are exempt: an end-to-end case is exercised by being run, not by being cited, so a `§E2E-<name>` that nothing references is not a warning. Every other kind is subject to this rule. `grund list --unused` ([§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id)) uses the same default signal and suppresses uncited `E2E` cases unless `E2E` is explicitly selected with `--kind` (including a multi-kind filter such as `--kind FS,E2E`).

### 4.2 Inline note soft-cap overrun *(opt-in)*

Off by default. When `[reference] warn_on_suggested = true` is set in the project's `grund.toml` ([§FS-config.3.1](FS-config.md#31-reference--citation-form)), an inline citation site whose line count exceeds `inline_note_suggested_lines` but stays within `inline_note_max_lines` is reported as a warning. The full contract — what counts as a soft-cap overrun and how it interacts with the hard-cap error in §3.10 — lives in [§FS-inline-citation-style.4.2](FS-inline-citation-style.md#42-warnings--opt-in-soft-cap). Off by default because the soft cap is primarily agent-facing guidance ([§FS-inline-citation-style.5](FS-inline-citation-style.md#5-agent-facing-rendering)); flipping the toggle escalates it to a `check`-time signal.

### 4.3 Redundant config pair

A directory that carries both a bare `grund.toml` and `.agents/grund.toml` ([§FS-config.1.1](FS-config.md#11-when-one-directory-carries-both)). The bare file is the config; the `.agents/` one is read by nothing, so a user who edits it changes nothing and is told so:

```
warning: .agents/grund.toml is ignored — grund.toml takes precedence; delete one
```

It is a CLI-level `warning:` on **stderr** (§2.1.1), not a per-finding line: it is about which file the run read, not a finding at a site in the citation graph, and there is no offending line to point at — the whole file is ignored. Both paths are rendered relative to the config root ([§FS-config.3.6](FS-config.md#36-output--report-format)), so the message names the two files a user has to choose between. Like every warning it leaves the exit code alone (§2), because the pair is the ordinary transient state of a migration between the two forms ([§DF-config-file-location.2.2](../decisions/functional/DF-config-file-location.md#22-the-bare-grundtoml-wins-a-tie-and-check-warns-about-the-pair)).

The same warning is emitted by `grund config validate` and `grund config show` ([§FS-config.4.1](FS-config.md#41-grund-config-validate-path), [§FS-config.4.2](FS-config.md#42-grund-config-show-path)) — those are the surfaces a user reaches for when the answer to "why is my config not taking effect" is that `grund` is reading the other file. No other command reports it: a redundant pair is a fact about the repository's configuration, and `show`, `list`, `refs`, `cover`, and `fmt` answer questions about its content. §4.11 is this finding's sibling and inherits every sentence of this section: it fires where the run *read* the `.agents/` file rather than ignored it, which is the case this one cannot be in.

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

The per-heading half — naming each heading that looks like a declaration and does not match — is §4.6, a different rule asking a different question: this one is arithmetic over what the scan recorded, that one is about what a single line came close to being. Where both could speak, §4.6 does and this one is withheld under the rule above, because "these two headings, at these lines" is the same fact said usefully.

- **Code:** `nothing-recognized` ([§FS-errors.5](FS-errors.md#5-json-format)), with `path` and `line` null like every CLI-level diagnostic.

### 4.6 Declaration near miss

A heading that opens the way a declaration does and does not parse as one is, today, simply not a declaration: invisible to `check`, to `grund list`, and to citation resolution, with nothing said about it. The classic stumble is `# FS-login: …` under the default `{kind}-{number}-{slug}` — the `-NNN-` left out. `check` emits one **warning** per such heading, at the line a contributor has to edit:

```
docs/spec.md:1: `FS-login` is heading-shaped and declares nothing — [id] format = "{kind}-{number}-{slug}" reads `# <KIND>-<NNN>-<slug>: <title>`
```

**What counts.** A line in declaration position — a Markdown heading, or a comment-prefixed line in a source file under the rules of [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) — whose first token is a configured kind name ([§FS-config.3.4](FS-config.md#34-kinds--recognized-kinds)) followed by the literal `[id] format` puts after `{kind}`, which the ID grammar then rejects, and which is **followed by the declaration colon**.

That colon is the discriminator, and it earns its place: a line opening with an ID-shaped token and no colon is prose far more often than it is a declaration attempt — a comment wrapped across lines whose continuation begins with one is the case that proved it, in this repository's own source. So the rule reads exactly the shape a declaration attempt has, `<KIND>-…: <title>`, and says nothing about the rest. A near miss written without a title is not reported; that is the cost, and it buys a rule that stays quiet on prose. The token also stops at a backtick, so an inline-code mention is not one either. The position rules are the declaration rules exactly, so a near miss is only ever read where a declaration would have been: a bare `FS-login: …` in Markdown prose is not one ([§DF-code-declarations-drop-hash](../decisions/functional/DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix)), and neither is anything inside a fenced block.

**Three facts, and no fourth.** The message names the token as written, the configured template, and the shape that template reads. It does **not** propose the corrected ID. `check` reports facts about the tree and the config (§3 vs §4), and an ID assembled from `number_pattern` and `slug_pattern` would be a guess at what the author meant — the same line [§FS-check.4.5](FS-check.md#45-nothing-recognized) holds for the same reason. What the reader gets is the mismatch; what to do about it is theirs.

**A format with no literal after `{kind}` is not judged.** Where `[id] format` runs `{kind}` straight into what follows, "looks like a declaration" cannot be told from prose beginning with a kind name, so the rule declines rather than guess ([§FS-config.3.2](FS-config.md#32-id--id-grammar)). Every format that separates them — which is every default and every generated config — is covered.

**It is a warning, so the exit code is unchanged** (§4), and `grund list` is unchanged with it: a near-miss heading is still not a declaration, and this rule reports that rather than repairing it. Where every heading in a project misses this way, these findings are what the run says and [§FS-check.4.5](FS-check.md#45-nothing-recognized) is withheld under its own rule — the specific fact displaces the general one, which is the outcome that rule's "or the headings are written to a different shape than that" was standing in for.

The line-oriented opt-out held in reserve while the warning was new was never needed and is not implemented: the position rules are the declaration rules, so the rule reads only the lines where a declaration would have been.

- **Code:** `declaration-near-miss` ([§FS-errors.5](FS-errors.md#5-json-format)).

### 4.7 A workspace member swallows the block's own scan

A `[workspace]` block every one of whose walk roots lies inside one of its own members ([§FS-workspace.2.1](FS-workspace.md#21-a-member-that-swallows-the-blocks-own-scan)) reads nothing at all, and said nothing about it: `grund list` completed silently and exited `0`, and `check` offered only the empty-scan caution of §2.2 — which names `[scan] include`, the key that is usually already correct, so the one message the run did produce pointed away from the entry that caused it.

`grund` emits one CLI-level `warning:` line (§2.1.1) on **stderr**, carrying the block's `members` line as its breadcrumb the way a config error does ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)), then each covered root and the member entry it is inside, in config order. The member entry is named **as the config wrote it**; the covered root is named by its path **under the block root**, which is that spelling normalized rather than the spelling itself — an `include = ["./docs/"]` entry is named `docs`. Neither is the resolved path: that renders as nothing when it equals the render base and as an absolute path when it does not, and an author can edit neither ([§FS-errors.4](FS-errors.md#4-determinism)):

```
warning: grund.toml:16: [workspace] members swallows this project's whole scan — every scan root is inside a member: `docs` in `docs` — so its declarations are unreachable and its citations are never checked. Point [scan] include at a directory that is not a member, or set include_root = false. This becomes an error in grund 0.14.0.
```

**Every command that walks says it, not just `check`.** The question is asked where a run populates a block's member boundary, so `grund check`, `list`, `refs`, `cover`, `fmt`, and every other command that resolves that boundary carry it. A silent-scan defect only `check` reports is half-reported: the other surfaces are exactly where the repository looks fine. It is emitted **once per block per run**, ahead of any finding, and asked of every block in a nested tree against that block's own `members` line — rendered, like every diagnostic from a block above the run's root, against the root this run was launched at ([§FS-errors.4](FS-errors.md#4-determinism)).

**One run spells the whole tree from one place.** That base is the run's own root, for every block the run reaches — the ones above it, the one it is rooted at, and the ones below it alike. Most commands never have to think about it: a run narrowed into a member is re-rooted onto that member before it walks, so the top of the tree it expands *is* where it was launched. `grund init` is the exception, because it expands the outermost workspace above its target in order to teach the alias set. Its blocks below the target are still named from the target: run inside a member of a three-deep absorbed tree, the lines read `../grund.toml`, `grund.toml`, `sub/grund.toml`, and a reader resolves every one of them against the directory they are standing in. Re-basing them onto the workspace root instead would print a path that exists from there and is the wrong file, which is the defect [§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces) already forbids for an ancestor's `members` line.

**It stands beside the empty-scan caution rather than in place of it.** §2.2 is about a walk that read nothing; this is about a configuration that can read nothing. A `check` over an absorbed block prints both, this one first, and §2.2's text is unchanged — it is stable phrasing ([§FS-errors.3](FS-errors.md#3-message-text)) and a repository grepping for it keeps what it had.

**It is a launch-time diagnostic, so it keeps its text under `--format json`** ([§FS-errors.5](FS-errors.md#5-json-format)), like the undecidable-ancestor warning of [§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces) and every other message emitted before a report exists. It therefore carries no JSON `code`: it is never one of the report's diagnostics. Like every warning it leaves the exit code alone (§2), and like every warning it stands in place of the `success` marker (§2.1).

**A warning in this release, an error in the next.** No `grund` command repairs it — the fix is a choice between repointing `[scan] include` and declaring the block no project — so [§REQ-backwards-compatibility.3](../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations)'s single-release licence does not apply and [§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)'s deprecation path does: the message names the release the finding becomes an error in. That release is [§RM-workspace-absorbed-scan-error](../roadmap.md#rm-workspace-absorbed-scan-error-flip-the-absorbed-scan-warning-to-an-error), and a test holds it ahead of the running version so the deadline cannot pass unnoticed — the same guard §3.18 carried until its own release arrived, for the same reason. Decided in [§DF-absorbed-scan-warning](../decisions/functional/DF-absorbed-scan-warning.md#df-absorbed-scan-warning-a-scan-its-own-members-swallowed-is-a-warning-with-a-named-release-not-an-error).
### 4.8 Unlisted `[workspace]` block

A directory that declares `[workspace]` and that **no enclosing `[workspace]` block lists among its `members`** is claimed by nobody ([§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)). At the outer scope the block is ignored and its whole subtree is read into the enclosing project's namespace; a run started *at* it names every project from itself. The two scopes then spell the same projects differently — `c/FS-c` inside the block, `root/FS-c` at the repository root — so a citation passes the inner check and fails the run CI does, which is [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing in the one place the alias-path model exists to hold it. Every run whose tree walk meets such a block says so, in one CLI-level `warning:` on **stderr** (§2.1.1, [§FS-errors.2.2](FS-errors.md#22-cli-level-message)). Decided in [§DF-unlisted-workspace-block](../decisions/functional/DF-unlisted-workspace-block.md#df-unlisted-workspace-block-an-unlisted-workspace-block-is-reported-by-the-walk-that-meets-it).

**What counts.** A directory the run's own walk reached, carrying a config under either discovery name ([§FS-config.1](FS-config.md#1-file-location-and-discovery)), whose config declares a `[workspace]` table, and whose canonical root no `[workspace]` block above it names among its **expanded** members. The claim question is the ancestor climb the claimed chain already runs ([§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)) — asked of a directory the walk found rather than of the run's own root — so it is read from `members` entries alone and it climbs past the run's root exactly as that climb does. Both discovery names are probed at each walked *directory*, which is what finds the `.agents/grund.toml` form: the walk never descends into a hidden directory ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)), so watching instead for walked *files* named `grund.toml` would find half the blocks and call the other half claimed. A tree with no enclosing `[workspace]` block anywhere above it is the same case rather than a milder one — nothing claims the block, so nothing gives the projects under it a stable alias path, and the enclosing scan absorbs them just the same. An ancestor that *names* the candidate among its `members` and then cannot answer it — a member list that will not expand, a config that will not parse — leaves the claim undecidable in both directions ([§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)) and the block unreported, because no answer is not the answer that nothing claims it. That silence has a floor: the claim is read off the entry text before anything is expanded, so a block no ancestor *names* is never silenced by an ancestor's breakage, however broken that ancestor is. And the question is asked **quietly** — [§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)'s undecidable-claim warning belongs to the climb that spells an alias path out of the chain, which this rule does not do, so a run that would otherwise never ask the chain anything gains no line from having asked.

Three neighbouring shapes are deliberately **not** this finding. A nested directory carrying a plain `grund.toml` with no `[workspace]` table declares no projects to absorb: it is ordinary tree to the enclosing walk (§1.3) and nothing reports it. A block that *is* listed is inside the claimed chain at every depth, whatever the nesting. And **a project root of this run is never a candidate** — the run's own root, and each member root the walk stops at (§6) — because those are the scopes the run names everything else from, and a block absorbs nothing into a namespace it is the namespace of. Without that exemption the rule would fire on the run's own root the moment `--full` made it a walk root (§1.3), which is every workspace repository that sits under no enclosing one — that is to say, almost all of them.

**Only the outermost block of a chain.** A `[workspace]` block below an unlisted one *is* claimed — by the unlisted block — so the claim test answers it on its own, and listing the outer block puts the whole chain back in the claimed chain. One finding for one edit. One block the walk reached twice — under its own path and under a directory symlink to it — is one finding for the same reason: the claim test resolves both spellings to one root, one edit clears both, and the spelling reported is the first the walk met. Two unlisted blocks neither of which lists the other are two findings, because they are two edits.

**`include_root = false` changes nothing.** The block still contributes a segment to every alias path below it ([§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)), so the two scopes still disagree about how the projects under it are spelled. Same finding, same message. What that key costs the block's *own* files is §4.10, a separate finding on a separate condition: the two can fire on one block, because being claimed by nobody and being read by nobody are different holes with different repairs.

**The message** carries the block, what the absorption costs, the two config edits that clear it, and the release it becomes an error in:

```
warning: b/grund.toml:3: this [workspace] is listed by no enclosing workspace — the projects under it are absorbed into `root` instead of named under their own alias path; add "b" to [workspace] members in grund.toml, or keep it out of that project's [scan] — an unlisted [workspace] becomes an error in grund 0.14.0
```

The location sits *inside* the message text rather than in a bare `<path>:<line>:` prefix, because this is a fact about the run's configuration and not a finding at a site in the citation graph — the shape §4.3 and [§FS-config.4.3](FS-config.md#43-invalid-config-behavior) already use, and the shape the undecidable-claim warning of [§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces) already prints for a neighbouring fact about a `[workspace]` block. `<line>` is the block's `[workspace]` line: the reader has two files to open and this is the one that is wrong. Every path in the line is rendered against the run's report base ([§FS-errors.3](FS-errors.md#3-message-text)), and `"b"` is the block's directory relative to the enclosing project's root, which is where both remedies are written. The second remedy is stated as an outcome rather than as a key because which key carries it depends on the tree: `[scan] exclude` prunes descendants and never the directory a walk starts at (§1.3), so a block that is itself an `include` root leaves `include` as the edit, and a block below one takes `exclude`. Naming a key that clears the finding in one shape and not the other would be a remedy the reader has to argue with. The absorbing project is named by the alias path this run spells it with, so the message can be matched against what [§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id) printed.

**Which commands report it, and why not `check` alone.** Every command whose run walks a project tree: `check`, [§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id), [§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id), [§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file), [§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk), and the ID read of [§FS-show](FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id). That is a choice, and the reader is owed it in writing rather than left to infer it, because §4.3 draws the opposite line one page up for the redundant config pair. Three reasons it falls this way. The absorbed spelling is what `list` prints, so a `list` that shows `root/FS-c` where the block shows `c/FS-c` and says nothing is the same silence the finding exists to break. `refs` and `fmt --cross-refs` resolve qualified citations against the same project map, so they are equally wrong under an absorbed block. And §4.3's line does not reach this fact: a redundant config pair is about *which file the run read*, while this is about *how every command in the tree spells its projects*, which is not a question only `check` asks.

The honest difference from the neighbouring `[workspace]` cautions, and the reason it is stated rather than left implicit: a fact knowable at workspace-boundary population is knowable before any walk, so every command that merely *loads* the workspace can carry it. This one is knowable only from the walk that meets the nested config, so it is a property of **what the run walked** — carried by every command that walks, and silent wherever the walk does not reach the block. That is the earliest point at which the fact exists, not a second rule picked for convenience.

**The scope is the walk the run already makes.** The `[scan] include` roots and their subtrees, minus `[scan] exclude`, the ignore files, hidden directories, and member boundaries ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked), §6). No second walk is made for config files: the entries are already being enumerated, and the added work is one config probe per walked directory, which is what keeps [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) affordable here. It is also exactly the tree that gets absorbed — a block the scan never reaches absorbs nothing into the enclosing namespace.

What survives is a real limitation and is recorded rather than papered over: **an unlisted block outside the run's walk is still unreported.** A narrowed `grund check <path>` (§1.3), a directory named in `[scan] exclude`, a gitignored one, and a subtree behind a member boundary all leave a block unmet, and a run that cannot see something does not judge it — the same stance §3.18 takes for an index the run did not scan. `grund check --full` widens the walk, so it reaches blocks the plain run does not and may report one more; that is the flag being additive (§1.3) about a caution rather than about a located finding, and it is the same edit §1.3 already recommends for a vendored or example project sitting inside the config root.

**In `check` it is one of the report's warnings.** Like every warning it leaves the exit code alone (§2), and like every warning it stands in place of the `success` marker (§2.1) — which is what keeps it a verdict a repository can catch, and what makes the deprecation path below the right one. Under `--format=json` it is one warning diagnostic on stderr with `path`, `line`, and `sites` all `null` ([§FS-errors.5](FS-errors.md#5-json-format)), the location being in the message text; that is the cost of the CLI-level shape and it is paid once, on every surface, rather than by giving `check` a located finding and the other commands a different line for the same fact.

**A warning in this release, an error in [§RM-unlisted-workspace-error](../roadmap.md#rm-unlisted-workspace-error-flip-the-unlisted-workspace-warning-to-an-error).** No `grund` command writes either remedy — both are config edits and a judgement about which one the repository wants — so [§REQ-backwards-compatibility.3](../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations)'s licence for a same-release verdict flip does not apply and [§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)'s deprecation path does: the message names the release the finding becomes an error in, and a unit test holds that release ahead of the running version so the deadline cannot pass unnoticed. The same ramp, for the same reason, as [§FS-check.3.18](FS-check.md#318-declaration-missing-from-its-kinds-index) — argued in [§DF-index-compatibility-ramp](../decisions/functional/DF-index-compatibility-ramp.md#df-index-compatibility-ramp-a-findings-ramp-follows-its-fix-command-not-the-size-of-the-offence).

- **Code:** `unlisted-workspace-block` ([§FS-errors.5](FS-errors.md#5-json-format)).

### 4.9 A workspace member declared optional is absent

A member listed in `[workspace] optional_members` whose path is not a directory
in this checkout ([§FS-workspace.2.2](FS-workspace.md#22-a-member-that-may-be-legitimately-absent)). The namespace it would have contributed
was not read: no declaration in it reached a catalog, and no citation into it was
resolved or reported ([§FS-workspace.4](FS-workspace.md#4-resolution)). The run is a report about less of the
repository than it looks like, and saying so is this finding's whole job.

One warning per absent entry, in the order the list writes them, anchored at the
`optional_members` line of the block that holds it:

```
grund.toml:5: optional workspace member `vendored` is absent — citations into namespace `vendored` were not checked, so this run does not cover it
```

The entry is named **as the config wrote it** and the namespace by the **whole
alias path this run spells it with** — `vendored` and `sub/vendored` for one
entry inside a nested block — which is the pair §4.7 and §4.8 already use, for
the same two reasons: the entry is what an author can edit, and the alias path is
what a citation has to write ([§FS-errors.4](FS-errors.md#4-determinism)).

**It names no remedy, because nothing here is broken.** Most warnings in §4 end
in an edit, because they report a configuration that says something its author
did not mean. This one reports a state the author declared in advance and
a checkout that happens to be partial; the only thing that would "fix" it is a
checkout with the member in it, which is not grund's to ask for and is often not
available where the run happens. So the message stops at the fact, the way
[§FS-check.4.6](FS-check.md#46-declaration-near-miss) stops at the mismatch.

**It is a located finding on stdout, not a CLI-level caution.** That departs from
its two nearest neighbours — §4.7 and §4.8 both point at a `[workspace]` line and
both print as a CLI-level `warning:` on stderr (§2.1.1) — and the departure is
deliberate.

Those two report a **misconfiguration**, and the reason they are CLI-level is
that every command in the tree is wrong under them: a block that reads nothing
makes `grund list` silent, an unlisted block makes every command spell the same
project two ways. So they are emitted where the workspace loads, and carried by
every command that walks. This finding is not that. Nothing is misconfigured —
the repository said this may happen, and it happened — and no other command's
output is wrong, because a namespace that is not there has nothing to list and
nothing to point at. What is at stake is only the verdict `check` renders, and a
statement about the coverage of a report belongs in the report.

The exit code is what forces the shape. §2 gives grund one way to say "do not
trust this report as complete", and it is exit `2`. This is the one case where a
run is deliberately incomplete and still exits `0` ([§FS-workspace.2.2](FS-workspace.md#22-a-member-that-may-be-legitimately-absent)), so the
exit code carries nothing here and stdout has to. A CLI-level line would leave
stdout saying only that the `success` marker was withheld — a signal made of an
absence, which under `2>/dev/null` or a folded CI log is indistinguishable from
silence, and which is exactly the warning that scrolls past. The
`<path>:<line>:` prefix is also honest here in a way it is not for §4.3: there is
one line, in a file the repository wrote, and it is the line a reader has to open
to understand the run. `success` is withheld all the same, because it is withheld
for every warning (§2.1) — but that is now a consequence of the finding rather
than the whole of it.

**It names no release.** §4.7 and §4.8 are warnings on the way to being errors
([§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)). This one is permanent. An unverified namespace
is not a state to be migrated off; it is the standing price of the opt-out, paid
on every run of every checkout that takes it, and a repository that stops wanting
to pay it deletes the entry.

Under `--format=json` it is one warning diagnostic on **stdout** with `path` and
`line` set and `sites` null, like every other located finding ([§FS-errors.5](FS-errors.md#5-json-format)) —
which is the other half of what the CLI-level shape would have cost, since a
consumer filtering the report for coverage facts would have had to parse text on
a second stream to find this one.

- **Code:** `optional-member-absent` ([§FS-errors.5](FS-errors.md#5-json-format)).

### 4.10 `include_root = false` leaves the block's own files unread

A `[workspace]` block that sets `include_root = false` is not a project: it contributes a segment to every alias path below it and nothing else ([§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)). Its own files are then read by nobody — the enclosing scan stops at the member boundary ([§FS-workspace.6](FS-workspace.md#6-nested-project-boundary)), and `--full` (§1.3) widens a project's scope and has no project to widen here — so a declaration there reaches no catalog and a citation there is never checked, which is [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing through one config key. No run said so: over a grouping directory holding two dangling citations, `check`, `check --full` and `list` were all silent and exited `0` ([grund#71](https://github.com/vjovanov/grund/issues/71)). `grund` emits one CLI-level `warning:` (§2.1.1) on **stderr** for such a block **whose own tree actually holds a file a scan would have read**. Decided in [§DF-unread-opted-out-block](../decisions/functional/DF-unread-opted-out-block.md#df-unread-opted-out-block-the-unread-files-of-an-opted-out-block-are-a-conditional-warning-that-never-ramps).

**What counts is one question: would this block have read something, had it been a project?** That is the mirror of [§FS-workspace.2.1](FS-workspace.md#21-a-member-that-swallows-the-blocks-own-scan), which asks whether a block that *is* a project reads nothing — so one notion of "this block's own scope" answers both, and a `[[kinds]]` home or an unwalked home moves the two rules together. Take the block's **default scope** — the roots `[scan] include` and the walked `[[kinds]]` homes give it ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)), the set §6's boundary prunes, asked of the default scope whatever `--full` says, because this is a property of the configuration and not of one walk. Drop a root that is not on disk: the walk skips it before it prunes, so it is read by nobody and costs nobody anything. Drop a root at or inside one of the block's own expanded member roots: those files *are* read, by the member — compared as canonical paths, the way the walk's own prune compares them. On what is left, probe for one file the scan would have read, under the block's own `[scan] extensions`, `exclude`, ignore files and hidden-name rules applied exactly as the scanner applies them — including that a walk root is never pruned by `exclude`, an ignore file, or the hidden-directory rule, while a hidden *file* is skipped even as a root ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)). The probe **stops at the first hit**: the cost is "is there one file here", not the size of the tree, and only a block that opted out ever pays it ([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)).

**A block with no member in scope is not this finding.** `include_root = false` with no members — an absent `members` key, an empty list, or a non-empty list of globs that match no directories — is already a config error at that block's own line ([§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)). A configuration the run refuses is not one it also cautions about, and the caution's two remedies are not the repair that block needs.

**What stays silent, and why the silence is the point.** Four shapes are deliberately not this finding, and each is a correct configuration. A grouping directory that only groups — `grund.toml` and its members and nothing else — has no root on disk to read. A block whose own roots are all inside its members is [§FS-workspace.2.1](FS-workspace.md#21-a-member-that-swallows-the-blocks-own-scan)'s shape read from the other side, and those files are read, by the member. A tree whose only files are of unscanned types, or excluded, gitignored, or hidden, is a tree the block would not have read as a project either. And a root the config names that is not on disk rescues nothing. Firing on any of them would be a warning with no edit that clears it, which is permanent output tools learn to filter — the outcome [§DF-absorbed-scan-warning](../decisions/functional/DF-absorbed-scan-warning.md#df-absorbed-scan-warning-a-scan-its-own-members-swallowed-is-a-warning-with-a-named-release-not-an-error) already rejected once for a neighbouring finding.

**The message** carries the config line the reader should open, the tree that is unread, what that costs, and the two remedies:

```
warning: group/grund.toml:17: no project scans `docs`, so its citations are never checked. Set include_root = true, or point another project's [scan] include at it.
```

The breadcrumb is the block's own `include_root` line — the key that decided is the line to open — falling back to its `[workspace]` line if the key is absent, which the default `true` makes unreachable. It renders against the root this run was launched at, like every diagnostic about a block above or below it ([§FS-errors.4](FS-errors.md#4-determinism)). The unread tree is named by its path **under the block root**, that spelling normalized rather than the spelling itself, exactly as §4.7 names a covered root and never the resolved path, which renders as nothing when it equals the render base and as an absolute path when it does not.

**One root, not every root.** §4.7 lists every covered root because its claim is universal — *every* one is inside a member — and the list is the evidence for it. This claim is existential: one unread root is the whole finding, one edit clears all of them, and probing the rest would buy nothing the answer depends on. The one named is the first in scope order, `[scan] include` in config order and then the `[[kinds]]` homes, which is fixed; the first *file* under it is whatever the filesystem handed back, which is not, and is therefore never named ([§FS-errors.4](FS-errors.md#4-determinism)).

**Every command that walks says it, and each block says it once.** The question is asked where a run populates a block's member boundary — the block the run is rooted at, and each block below it — so `check`, [§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id), [§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id), [§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file), [§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk) and every other command that resolves that boundary carry it, on exactly the surfaces §4.7 is carried on. The reproduction in the ticket used `check` and `list`, which is the whole argument: a silent-scope defect only `check` reports is half-reported. It is **once per block per run** — a workspace-wide run that expands the same block a second time still says it once — and a tree with two opted-out blocks earns one line each, the block the run is rooted at first and the blocks below it after, in the order the run reaches them. A run narrowed inside a member never populates the enclosing block's boundary, so it stays silent about a block it is not reading through. One further silence is declared and bounded ([§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded)) rather than a gap: a run whose **workspace expansion fails** does not carry this finding for the blocks below its root — the answer depends on where the run's other projects are, and a failed expansion is exactly the case where that list was never produced, so the alternative is not an earlier warning but a wrong one. The run is exiting `2` on a config error the reader has to repair before anything else it says is worth reading, and the finding returns on the next run. §4.7 survives such a run, because what it asks is answered by the block's own members alone.

**It is a launch-time diagnostic, so it keeps its text under `--format json`** ([§FS-errors.5](FS-errors.md#5-json-format)) and carries no JSON `code`: it is never one of the report's diagnostics. That is §4.7's shape rather than §4.8's, and the difference between them is *when the fact exists*. §4.8's is knowable only from the walk that meets a nested config, so it is one of `check`'s report warnings and renders as a JSON diagnostic with `path`, `line` and `sites` null. This one is settled at boundary population, before a walk and before a report exists, and is carried by commands that have no report at all — so the report shape is not available to it on five of the six surfaces, and giving `check` a different one would make the text a consumer greps for depend on which command produced it.

**It stands in place of the `success` marker, and never moves the exit code.** §2.1 is unchanged: a run with a warning prints the warning rather than `success`. This is the first `[workspace]` caution that has to say so out loud, because it is the first that can fire on an otherwise clean run — §4.7's block always earns the empty-scan caution beside it (§2.2), and §4.8's is a report warning already. The exit code stays where it was (§2): opting out is a legitimate choice, and [§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization) fixes what a warning means.

**A warning permanently, with no release it becomes an error in.** This is the one of the three `[workspace]` findings that does not ramp, and the difference is not how wrong the repository is. A finding is eligible to become an error only when **every repository in that state is wrong** *and* **the configuration has a way to say "I meant it"** — the rule for all three, argued in [§DF-unread-opted-out-block.2.3](../decisions/functional/DF-unread-opted-out-block.md#23-what-makes-a-workspace-finding-ramp-and-what-makes-one-permanent). §4.7 and §4.8 pass both: a block that claims to be a project and reads nothing, and a block whose projects are spelled two ways, are wrong on every reading of them, and each has an edit that records the intent instead — `include_root = false` for one, listing the block for the other — so an error is a verdict their author can act on before it lands, and [§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)'s deprecation path gets them there. This finding passes neither. A grouping directory holding a README nobody needs checked is doing exactly what its author meant, and neither remedy records that: making the block a project and pointing another project's `[scan] include` at it both change what the repository *is*. That the finding is a property of the configuration **and** the tree — the same `grund.toml` silent on Monday and reportable on Tuesday because somebody added `group/docs/notes.md` — is the symptom that makes it recognizable, not the reason: §3.18 depends on the tree in the same way and ramped anyway. There is therefore no version constant, no roadmap milestone, and no clause in the message naming a release.

### 4.11 Config read from the deprecated `.agents/` location

The config this run read is an `.agents/grund.toml` ([§FS-config.1.2](FS-config.md#12-the-agents-location-is-deprecated)). The file is still read and still governs the project — the location is deprecated, not withdrawn — and the run says so, naming the file it read and the bare `grund.toml` beside it that should hold it instead:

```
warning: .agents/grund.toml is a deprecated config location — move it to grund.toml
```

This is §4.3's finding in everything but its trigger, and for §4.3's own reason: it is a fact about *which file the run read*, not about that file's content and not about a site in the citation graph. So it is a CLI-level `warning:` on **stderr** (§2.1.1) and never a per-finding line — there is no offending line to point at, the whole file is the subject. Both paths render relative to the config root ([§FS-config.3.6](FS-config.md#36-output--report-format)), so the message names the move as the `git mv` a reader can type. Like every warning it leaves the exit code alone (§2), because the location it names is a supported one and [§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization) fixes what a warning means.

**The same three surfaces as §4.3, and no more.** `grund check`, `grund config validate` and `grund config show` ([§FS-config.4.1](FS-config.md#41-grund-config-validate-path), [§FS-config.4.2](FS-config.md#42-grund-config-show-path)) carry it, byte-identically. `list`, `refs`, `cover`, `fmt` and a bare ID read stay silent, on §4.3's argument unchanged: where the config came from is a question about the repository's configuration, and those commands answer questions about its content. The silence is not an oversight to be widened later — a fact that never changes between runs, printed by every command, is permanent output tools learn to filter, and the surface a user reaches for to ask *where is my config* is `config show`.

**Once per config, not once per scope.** A workspace names the root's config and every member's, each at the path that project's config was loaded under ([§FS-errors.4](FS-errors.md#4-determinism)) — `packages/beta/.agents/grund.toml`, moving to `packages/beta/grund.toml`. Root and member are separate configs and a workspace may mix the two forms ([§FS-workspace.2](FS-workspace.md#2-workspace-configuration)), so a member on the old path earns its own line while a member on the bare form earns none, and a root and a member both on the old path earn one line each.

**A directory carrying both names earns §4.3 and not this.** The bare `grund.toml` won the tie ([§FS-config.1.1](FS-config.md#11-when-one-directory-carries-both)), so the config in force is on the home path already and nothing about it is deprecated; the `.agents/` file beside it is the one read by nothing, which is exactly what §4.3 reports. Emitting both would name one move twice and disagree about which of the two files is the problem.

**Under `--format=json` it keeps its text** and carries the `code` `deprecated-config-location`, with `path`, `line` and `sites` null ([§FS-errors.5](FS-errors.md#5-json-format)) — §4.3's shape, because it arrives the same way: one of the report's warnings, knowable from the config the run loaded rather than from the walk.

**It stands in place of the `success` marker** (§2.1), as every warning does, and that is the whole cost of the finding rather than a detail of it: a clean repository on the old path now prints this line where it printed `success`. [§REQ-backwards-compatibility.1](../requirements/REQ-backwards-compatibility.md#1-what-is-covered) governs that as a verdict change and permits it, since the exit code does not move. This repository pays it in its own e2e corpus, which is why the fixtures that had no reason to be on `.agents/` are on the bare form and the ones that remain are the ones whose subject is discovery itself.

**A warning permanently, with no release it becomes an error in.** The promise lives in [§FS-config.1.2](FS-config.md#12-the-agents-location-is-deprecated), because it is a fact about the location rather than about this message: the fallback is never removed, so there is no version constant, no roadmap milestone, and no clause in the text naming a release.

## 5. What grund does not check

See [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) — in particular [§FS-non-goals.1](FS-non-goals.md#1-markdown-link-validation) (markdown links / URLs), [§FS-non-goals.2](FS-non-goals.md#2-spelling-grammar-prose-quality) (spelling/grammar), and the convention that ID numbers are stable handles, not ordinal positions.

The declaration-side near miss is **no longer** in this section: a heading shaped like `# <KIND>-…: <title>` whose ID does not match the configured `[id] format` is reported per heading by §4.6, and a tree in which every heading misses that way says so twice over — once per line, and once as the run that recognized nothing (§4.5). Neither guesses the corrected ID. The citation-side near miss — a `§`-marked token in the shorthand shape — left this section earlier, when §1.2 and §3.13 began recognizing and reporting it.

## 6. Watch mode (`--watch`)

Status: planned — implementation tracked under [§RM-watch](../roadmap.md#rm-watch-implement-grund-check---watch).

When implemented, `grund check --watch [<path>]` will run the check once, then stay resident and re-run it whenever a file under the scanned tree (or the discovered `grund.toml`) changes. It is the editor-less counterpart to the optional LSP server ([§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)): the LSP integrates `grund` into an editor's diagnostics; `--watch` is the plain-terminal "every save" loop that [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) exists for. Until [§RM-watch](../roadmap.md#rm-watch-implement-grund-check---watch) lands, `grund check --watch` is a CLI error (`error: unknown flag \`--watch\``, exit 2).

- **Change detection.** Filesystem notifications where the OS provides them; a debounce window coalesces a burst of writes into one re-check. No polling loop is required, and there is no configurable interval — the watcher reacts, it does not sample.
- **Each run is a plain `grund check`.** Output and exit-status semantics of an individual run are exactly §2/§2.1 on the tree's state at that moment — byte-identical to what a non-`--watch` invocation would print ([§FS-errors.4](FS-errors.md#4-determinism)). Before each run the previous run's output is cleared so the terminal always shows the current report; with `--format=json` each run emits the same diagnostic NDJSON as non-watch mode, scoped to that run.
- **Lifecycle.** The process runs until interrupted (Ctrl-C / SIGINT). On interrupt it exits with the exit code of the most recently completed run (`0`/`1`/`2`), so `grund check --watch &` followed by a later signal is still a meaningful CI-ish probe. There is no TUI, no key bindings, no prompt — it is non-interactive per [§FS-non-goals.10](FS-non-goals.md#10-interactive-mode), just a re-printing checker. No network I/O ([§FS-non-goals.11](FS-non-goals.md#11-network-access-during-a-check)); the only files touched are the ones the walk already reads.
- **Scope.** `--watch` will be a `check` flag spelled as `grund check --watch [<path>]` ([§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions)). Other subcommands will not take it; a one-shot `grund fmt` or ID query has nothing to keep watching.
