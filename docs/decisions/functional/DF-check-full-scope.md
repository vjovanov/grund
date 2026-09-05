# DF-check-full-scope: `check --full` walks past `[scan] include` and reports only unresolvable references out there

**Status:** Accepted
**Date:** 2026-08-15

## 1. Context

`[scan] include` ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)) names the roots `grund check` walks. Everything else in the repository is not merely unchecked — it is **invisible**. A `§`-citation there does not resolve, does not dangle, appears in no report, and is counted by nothing: the edge is missing from the graph in both directions, so the declaration it points at is reported `declared but never cited` ([§FS-check.4.1](../../functional-spec/FS-check.md#41-unused-declaration)) as if nothing referred to it.

The failure mode is invisible by construction. `include` is written once, early, when the tree is small; code then moves, a simulation layer lands in `sim/`, a prompt set in `render/`, and nothing in the tool ever says that half the citations in the repository stopped being read. `check` prints `success` — which means "clean *within* `include`" and is read as "clean". A field report on this ([issue #55](https://github.com/vjovanov/grund/issues/55)) found three genuine defects sitting unreported behind exactly that gap: a citation a formatter had wrapped across a line break, a cited section that did not exist, and a citation missing its workspace namespace so it resolved against the wrong project.

That is the class [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) names in one line — *false negatives are bugs* — and it is worse than an ordinary miss, because the citations that most need checking are precisely the ones somebody forgot to bring into scope. Widening `include` is the durable fix, but a user cannot widen it toward a problem they cannot see, and pointing `check` at each candidate directory by hand is a search, not a check.

The same report showed why "just scan everything" is not the answer either: scanning one unconfigured directory produced **160 findings, of which 3 were real**. The other 157 were inline-note budget violations ([§FS-check.3.10](../../functional-spec/FS-check.md#310-inline-citation-style-violation)) on module docstrings that were never written to that convention. A mode that fails CI on style in directories that never opted into the style is a mode nobody runs twice.

## 2. Decision

### 2.1 A flag that cancels `include` and nothing else

Add `grund check --full` ([§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full)): the walk covers the whole config root. `[scan] exclude`, the ignore files, hidden directories, workspace member boundaries, and `[scan] extensions` apply unchanged. One flag, one meaning — "ignore `include`" — so a reader can answer "what did this run read?" from the flag alone.

"Cancels `include`" is a statement about *rules*, not about walk roots, and the difference is load-bearing: none of `exclude`, the ignore files, or the hidden-directory rule can prune the directory a walk *starts* at, only its descendants. A walk that started at the config root alone would therefore lose a gitignored, excluded, or hidden `include` root directory that the ordinary run reads as a root of its own — `--full` would read *fewer* files than `grund check` and could turn a red run green, which §2.4 forbids in the strongest terms available. So the `--full` walk starts at the config root **and** at every `include` root, and reads each file once.

`[scan] extensions` deliberately stays out of it. Which file *types* carry citations is a project fact that belongs in the config; which *directories* were forgotten is the accident this flag exists to expose. Folding both into one flag would make `--full` mean "read more kinds of file too", and the set of files a run touched would stop being derivable from the config.

### 2.2 Out-of-scope findings are limited to reference resolution

Outside the configured scope, only the resolution failures of [§FS-check.3.14](../../functional-spec/FS-check.md#314-out-of-scope-unresolvable-citation---full-only) are reported: unknown ID, missing section, unknown namespace alias, and a shorthand matching zero or several declarations. Style, grounding, placement, direction, duplicate, and unused rules are not.

The line is not "cheap rules versus expensive rules"; it is *what the repository agreed to*. "This citation points at nothing" is true of any tree, in any project, under any convention — it is the invariant `grund` exists to hold. "This inline note is more than three lines" is true only of a tree that adopted that convention, and `[scan] include` is where a project says which files it adopted it for. A directory nobody put in scope has agreed to none of it.

### 2.3 They are errors, and they move the exit code

A `--full` finding is an error like any other reference failure. A warning would leave the mode exit-code-neutral, and a finding no CI run can fail on is one a repository accumulates behind indefinitely — the argument [§FS-check.3.13](../../functional-spec/FS-check.md#313-number-only-shorthand-citation) already makes for the shorthand. Nothing turns red without being asked: `--full` is opt-in, and no existing invocation changes behavior ([§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)).

### 2.4 The in-scope report is untouched

`--full` is purely additive: the findings inside the configured scope are exactly the ones `grund check` reports on the same tree. Adding the flag can only turn a green run red, never the reverse, so a team can add it to CI beside the ordinary check without auditing what the wider walk did to the verdict it already trusted.

That is also why the wider walk does **not** feed the in-scope rules: an inline declaration living outside `include` would otherwise make an in-scope dangling citation resolve under `--full` and dangle without it — a citation that `grund <ID>` still cannot open, reported green by the mode meant to find more, not less.

The price is paid in one place and is worth naming: an in-scope declaration cited *only* from outside `include` still gets its [§FS-check.4.1](../../functional-spec/FS-check.md#41-unused-declaration) `declared but never cited` warning under `--full`, even though the wider walk has just read the citation and resolved it. Counting that edge would *retire* an in-scope finding — the one direction additivity does not allow — and would make a warning mean different things under different flags. `--full` reports what points at nothing; it does not re-score the governed graph. The remedy is the one every finding in the tier already names: widen `include`, and the citing file joins the graph with the whole rule set behind it.

### 2.5 No `grund.toml` key

`--full` is a flag and never a config setting. A project that wants its whole tree governed widens `include` and gets the whole rule set; the flag is for the tree whose config has drifted from where its code went. A key that made `--full` the default would be a second, weaker `include` — two knobs describing one scope, which is how two correctly-configured installs come to disagree ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).

### 2.6 ID-shaped tokens used as data are **not** reported

The same report asked for a second, advisory tier: grep every text file for ID-shaped tokens and report the ones that resolve to nothing, catching an ID that appears only inside a Python string literal (`sources=("SRC-077-…", …)`) or in a `.csv` header comment whose extension is not scanned. Declined, and the two supported answers are better ones:

- **Mark the token.** A marker-prefixed citation is recognized *everywhere*, string literal or not ([§FS-check.1.1](../../functional-spec/FS-check.md#11-recognized-citations)) — the marker is the signal of intent, and that is the whole of `[reference] strict = true`. Writing `§SRC-077-…` in the data table makes it a checked citation today, with no new mode.
- **Add the extension.** `extensions = […, "csv"]` puts the file in the walk; citations are matched on every line of a scanned file, not only in comments.

Reporting *unmarked* tokens instead would undo three decisions at once. It re-admits the false-positive class `strict = true` was created to end ([§DF-reference-marker.2.4](DF-reference-marker.md#24-strict-vs-optional)): `KIND-NNN`-shaped strings are ordinary text in the wild. It contradicts a rule the specs state explicitly — a bare ID-shaped token inside a string literal is *not* a citation ([§FS-check.1.1](../../functional-spec/FS-check.md#11-recognized-citations), [§FS-fmt.2.3.1](../../functional-spec/FS-fmt.md#231-string-literal-exclusion-rule)) — so one pass would call a site a citation while every other pass calls it text. And telling "a data table of source keys" from "a string that happens to look like an ID" is exactly the judgment [§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing) refuses to make.

An advisory tier that never touched the exit code would still be wrong here, for the reason §2.2 gives: a finding a project cannot act on and cannot silence is noise, and noise is what teaches a reader to skip the finding that mattered.

## 3. Consequences

- `grund check` gains `--full`; `CheckOpts` gains `full: bool` ([§FS-distribution.3.1](../../functional-spec/FS-distribution.md#31-rust-grund-core-crate)) — a documented, additive break in the library surface, like `include_suggestions` before it.
- Four new diagnostic codes — `out-of-scope-dangling`, `out-of-scope-missing-section`, `out-of-scope-unknown-project`, `out-of-scope-shorthand-citation` — one per rule in the tier, each the in-scope code under an `out-of-scope-` prefix. The `{ severity, path, line, code, message, sites }` JSON shape ([§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format)) is unchanged: the tier rides on the `code` field the shape already carries, filterable by prefix, while the rule stays exact-matchable. One code for all four would have made a consumer regex the prose to learn which rule fired.
- No `grund_config_version` bump and no new config key (§2.5).
- `grund fmt`, `list`, `refs`, `cover`, and `show` keep the configured scope. That asymmetry is deliberate — `check` answers "is anything broken?", which is a question about the repository; the others answer questions about the governed graph — and it is why [§FS-check.3.14](../../functional-spec/FS-check.md#314-out-of-scope-unresolvable-citation---full-only) withholds the one finding whose fix is a `fmt` rewrite.
- The durable fix stays `include`. Every out-of-scope finding leads with the key, so the tier reads as "put this directory in scope", not as a place to live.

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| Make the full walk the default and drop `include` | `include` is what keeps vendored trees, fixture repos, and generated code out of a check; scanning everything by default would make `grund check` unusable in exactly the large repos [§GOAL-small-and-large](../../goals.md#goal-small-and-large-start-small-configure-for-big) targets, and would flip every existing repo's verdict on upgrade. |
| Report the out-of-scope findings as warnings | Exit-code-neutral, so CI cannot hold the line and the dangling citations keep accumulating — the state this mode exists to end (§2.3). |
| Report every rule out of scope, in a separate section | The 157-to-3 noise ratio from the field report. Separating the sections does not make style findings in an unconfigured directory actionable; the project never agreed to that convention there (§2.2). |
| A `[scan] full = true` config key | A second, weaker `include` (§2.5). |
| A separate subcommand (`grund audit`) | The question is the same one `check` answers, over a wider scope. A second command would duplicate every `check` flag and drift from it. |
| Widen `[scan] extensions` under `--full` too | Makes "what did this run read?" underivable from the config, and pulls unknown file types into the scan for a mode whose job is to find *directories* nobody configured (§2.1). |
| Grep ID-shaped tokens in any text file as an advisory tier | Undoes `strict = true`, contradicts the string-literal rule, and needs the AST judgment [§FS-non-goals.3](../../functional-spec/FS-non-goals.md#3-code-ast-parsing) rules out (§2.6). The marker and `[scan] extensions` already cover the real cases, deterministically. |
