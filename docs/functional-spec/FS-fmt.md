# FS-fmt: grund normalizes references in bulk

The `fmt` subcommand rewrites a tree to canonical form: trigger sequences become markers, and (optionally) bare citations become marker-prefixed. It is the batch counterpart to the optional LSP server's live trigger transform ([§FS-lsp.1.4](FS-lsp.md#14-live-trigger-transform)) and the always-available path: every install of `grund` ships `fmt`, while the LSP server is opt-in. Implements [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger).

## 1. Inputs

```
grund fmt [<path>] [--check] [--marker] [--cross-refs] [--write]
```

- `<path>` — directory or file. Defaults to the current directory.
- `--check` — explicit form of the default behavior: report exactly what `--write` would change, across all four rewrite classes (including cross-references when `[fmt.cross_refs] enabled = true` and the scope contains Markdown); exit non-zero if any change would be made; do not write. Provided as a flag for CI clarity (a script that says `grund fmt --check` is unambiguous about intent).
- `--marker` — also rewrite bare citations (`FS-check`) to marker-prefixed (`§FS-check`). Off by default to preserve existing repos that have not opted in.
- `--cross-refs` — in `.md` files only, also wrap each marker-prefixed citation in a clickable Markdown link to the declaration body. Per §6. This forces the cross-reference pass for one invocation; generated configs set `[fmt.cross_refs] enabled = true`, so every `fmt` scope containing Markdown runs it by default in both dry-run and write mode unless the repo opts out. Implements [§DF-md-link-emission](../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations).
- `--write` — write the transformed contents back to disk. Exit 0 even when changes were made (the changes were the requested operation, not a failure).

`--check` and `--write` are mutually exclusive. Without either, the default is `--check`.

## 2. Behavior

### 2.1 Trigger-to-marker

Wherever the configured trigger (default `$$`) is immediately followed by a token that matches the repo's `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) — `FS-007` under a numbered format, `FS-login` under the slug-only form `grund` itself uses — replace the trigger with the configured marker (default `§`). The trigger is only consumed when a real ID-shaped token follows it, so a bare `$$` (LaTeX display math, a shell variable) is left alone. Idempotent: running `grund fmt` twice produces no further change.

### 2.2 Bare-to-marker (with `--marker`)

When `--marker` is given, every recognized bare citation is also rewritten to its marker-prefixed form. This is how a repo migrates out of `[reference] strict = false` compatibility mode: run `grund fmt --marker --write` once, then remove the override or flip the strict flag back to `true`.

### 2.3 What is never rewritten

This list is the ownership boundary for the one command that edits files in place ([§REQ-no-data-loss.2](../requirements/REQ-no-data-loss.md#2-writers-touch-only-what-they-own)): everything outside it keeps its bytes.

- Declaration headings (the line that names the ID). The marker is for *citations*, not declarations.
- Citations inside string literals on a source line (where rewriting would change runtime behavior).
- Citations inside Markdown inline code spans (where rewriting would change a literal command, path, or example).
- ID-shaped text inside Markdown link destinations (where rewriting would change the URL rather than the visible citation) — and, off strict mode, such a **bare** token is not recognized as a citation there at all ([§FS-check.1.1](FS-check.md#11-recognized-citations)), the same never-rewrite zone keeping `check` from demanding an edit this command refuses to make.
- Files outside the configured scan set.
- Files reached through a symlink that leaves the config root — `--write` reads them and does not write through them (§2.3.2).
- Everything in a **suppressed scope**: a file the `[fmt] exclude` list names, and a region between a `grund:fmt off` directive and the `grund:fmt on` that closes it (§2.5). Unlike every other entry in this list, these two are asked for by the repository rather than forced by the text, and they are the only ones an index carve-out outranks (§2.5.3).

#### 2.3.1 String-literal exclusion rule

The string-literal exclusion is deterministic, not heuristic. For every candidate transform site on a source-file line:

1. Walk the line left-to-right from column 0 up to the candidate's start column.
2. Track an open-quote state per `'`, `"`, and `` ` ``. Toggling rules: an unescaped (no immediately preceding `\`) quote of a given kind toggles its state, but only when no other kind is currently open.
3. If any quote state is open at the candidate's start column, the candidate is inside a string literal and is **not** rewritten.

Markdown files (`.md`) are not subject to this rule — they have no string literals. The rule applies only to files matched by the `extensions` list excluding `md`.

**A Python docstring is documentation, so the walk runs over its content.** In a `.py` file scanned with `[scan] docstring_python` ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)), a `"""` / `'''` delimiter is doc-comment syntax rather than a quote — the same reading that makes a docstring a place a citation, or a whole declaration, may live ([§FS-check.1.1](FS-check.md#11-recognized-citations)). So the line this walk reads is the docstring's **content**, the delimiters stripped and the candidate's start column measured from there. A citation on the opening line, in a one-line docstring, or on the closing line is then judged exactly like one on an interior line, and exactly like one in a `#` comment: a quote written *inside* the content still opens and closes as it does anywhere else, and the delimiter never does. Without this one docstring gets three verdicts by line — the delimiter opens a literal on the opening line, closes nothing on an interior one, and precedes or follows the citation on the closing one — and the reason for the exclusion, that rewriting would change runtime behavior (§2.3), describes none of them. A docstring is documentation, which is what §2.4 exists to canonicalize.

Everything else keeps the raw-line rule, unchanged: a string literal on a **code** line — the runtime text the exclusion is actually about — every line of a `.py` file scanned with `docstring_python = false`, and every file of every other language. On a docstring's closing line the content ends at the delimiter, so a candidate in the code after it is judged on the raw line like any other code.

This gives two correctly-configured installs identical output on identical input ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).

#### 2.3.2 A link that leaves the config root is not written through

`--write` does not rewrite a file whose path is in the tree but whose bytes are not: one reached through a symlink — the file itself, or a directory above it — whose target resolves **outside the config root**. The walk still reads it and its citations are still checked, exactly as [§FS-config.3.5.1](FS-config.md#351-a-symlink-in-the-tree-is-followed) says; what stops at the boundary is the write. Putting this project's rewrites into a file the project does not own is the one thing the in-place editor must not do on its own initiative ([§REQ-no-data-loss.2](../requirements/REQ-no-data-loss.md#2-writers-touch-only-what-they-own)), and the file it would edit sits at a path this project's own report cannot render ([§FS-config.3.6](FS-config.md#36-output--report-format)). Each refused file is named once on stderr — `warning: <path>: not rewritten: the symlink target is outside the config root`, the CLI-level shape of [§FS-errors.2.2](FS-errors.md#22-cli-level-message) — and the exit code is unchanged, because the refusal is the intended behavior and not a failure of the run (§3).

**The dry run refuses the same file**, with the same `warning:` line, and does not list a rewrite for it. A dry run predicts what `--write` does (§3); a pending rewrite that `--write` will never perform is one no edit can clear, so `fmt --check` on a tree holding such a link would exit `1` forever and every CI gate and pre-commit hook built on it could never pass. Reporting what the tree contains is not worth a report nobody can act on — and the refusal, printed where the rewrite would have been, is the actionable half: it names the link to resolve.

A link whose target is **inside** the config root is written through, and one consequence of that is named here rather than fixed. One physical file reached under two spellings is read once, under the surviving spelling ([§FS-config.3.5.4](FS-config.md#354-one-physical-file-is-read-once)), so `--cross-refs` computes every relative target from *that* spelling: the link it writes is correct when the file is opened where grund read it and wrong when it is opened at its other name. No rewrite can be right both ways — a single relative path is resolved against whichever directory the reader opened the file from — so the fix is not a better anchor but not giving one file two names. Decided in [§DF-symlink-scan.2.5](../decisions/functional/DF-symlink-scan.md#25-fmt---write-refuses-a-link-that-leaves-the-config-root-and-nothing-else).

### 2.4 Shorthand-to-canonical

Where a number-only shorthand citation ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)) resolves to exactly one declaration, rewrite it to the canonical full ID: `§FS-042` becomes `§FS-042-user-login`, preserving any `.<section>` suffix and any `<alias>/` namespace. This is the bulk fix-it for the [§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation) error — `grund fmt --write` clears a tree of them in one pass, which is what lets that finding be an error ([§DF-number-only-citation-shorthand.2.2](../decisions/functional/DF-number-only-citation-shorthand.md#22-where-the-shorthand-is-accepted-and-where-it-is-an-error)).

A shorthand that matches no declaration, or more than one, is **left alone**: `fmt` normalizes, it does not guess, and `check` is where the ambiguity is reported. The same rule runs on the trigger form, so `$$FS-042` becomes `§FS-042-user-login` in one step rather than landing on a shorthand that immediately fails `check` (§2.1). The never-rewrite rules of §2.3 apply unchanged — and because they do, [§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation) withholds its error wherever they bite, so everything that rule reports is something this one can actually fix.

**Only a whole token is a shorthand.** The rewrite fires when the character after the match cannot continue an ID ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)); `§FS-042-User-Login`, `§FS-042_user_login`, and `§FS-042abc` are left byte-for-byte alone. The shorthand is a *prefix* of every longer ID-shaped token, so rewriting one that is not the whole token would splice the canonical slug into the middle of what the author wrote and leave the tail glued on — a silent edit to text that was never a citation.

**And a whole token is not always a citation** — the rewrite is withheld again wherever the token sits in a numeric run (§2.4.1).

The qualified form is matched, resolved, and rendered with the **aliased project's** grammar throughout — the same routing the scanner uses — so `<§>api/FS-042` becomes `<§>api/FS-042-session` even in a workspace that mixes formats (escaped here — this repository has no `api` member). Matching the tail with the citing project's shape instead would rewrite tokens `check` never saw and skip the ones it reported ([§FS-workspace.8.5](FS-workspace.md#85-grund-fmt---cross-refs)). A member-local run carries no workspace context and leaves qualified citations untouched, exactly as `--cross-refs` does there.

The pass needs the declaration set, and getting one costs a tree scan that `fmt --check` otherwise never performs. So it is not scheduled up front: the walk starts without declarations and scans the first time it actually meets a shorthand, then redoes that one file. A repo that writes none — including every repo whose `[id] format` lacks `{number}` or `{slug}` and therefore has no shorthand at all ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)) — pays nothing. The scan covers the whole project even under a narrowed path scope, because the declaration a shorthand names routinely lives outside the files being rewritten. Its rewrite label in the `--check` report is `shorthand → canonical`, and that label carries the replacement text (§3).

#### 2.4.1 A shorthand in a numeric run is not rewritten

A marked shorthand whose token is glued to a second number is being used as a numeral, not as a citation, and is left byte-for-byte alone. Decided in [§DF-shorthand-numeric-run](../decisions/functional/DF-shorthand-numeric-run.md#df-shorthand-numeric-run-a-marked-shorthand-glued-to-another-number-is-a-numeral-not-a-citation).

A changelog line recording an ID remapping writes the old numbers as runs — `§SPEC-001→SPEC-003`, `§SPEC-001/003`, `§COMP-047/046/049`. Every character after the first number is one that cannot continue an ID, so §2.4's boundary test passes and, without this clause, the rewrite fires and the sentence ends up naming a declaration it never meant. Nothing catches it afterwards: the output is a well-formed citation of a real declaration, so `grund check` passes and only a human reading the prose can see the damage. That is the one rewrite in this command that writes characters *into* the ID token rather than around it, and therefore the one whose mistakes are invisible to every later pass.

Reading forward from the end of the shorthand token, the site is a **numeric run** when both hold:

1. The maximal run of characters that can neither continue nor start an ID — anything not alphanumeric and not `_` — is non-empty, and carries no whitespace, no marker, and no bracket or quote (`(`, `)`, `[`, `]`, `{`, `}`, `"`, `'`, `` ` ``).
2. What follows that run begins with a `[id] number_pattern` match, or with the whole shorthand shape of the grammar that parsed the token ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) written **unqualified** — an `<alias>/` namespace prefix can only precede a citation ([§FS-workspace.1](FS-workspace.md#1-citation-syntax)) and is never the second number of a run, so a path that happens to end in an ID-shaped segment is not one.

The discriminator is the second number, not the punctuation, so the clause covers `→`, `/`, `..`, `…`, and everything else anyone writes between two numbers without enumerating a list. The three exclusions are what keep it from over-firing: whitespace, because the gluing is the evidence and `§FS-042 (2024)` is an ordinary citation under the default `number_pattern = "\d+"`; the marker, because `§FS-042, §FS-043` is two citations the author marked one at a time; and brackets and quotes, because those bound a construct rather than glue two numerals — without that exclusion the characters *closing* the citation's own construct and *opening* the next one read as one delimiter run, and `[§FS-042](FS-042-user-login.md)` — the Markdown link §6 itself writes — or the footnote reference `§FS-042[^1]` would be refused a rewrite they need.

The rule reads forward only. `SPEC-001→§SPEC-003` — a run whose tail carries the marker — is rewritten, because a glued number-shaped token to the *left* is also what `2026-08-19/§FS-042` looks like, and refusing those would withhold the rewrite from real citations on no evidence.

The site is reported rather than skipped in silence: [§FS-check.3.15](FS-check.md#315-shorthand-citation-in-a-numeric-run) names it, with both the canonical form and the `<§>` escape, so the author picks. The site is still a citation in every other respect — it resolves, it counts as an edge, it grounds its file. Only the rewrite is withheld, and only where the shorthand resolves: one matching zero or several declarations keeps its [§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation) message, which reports a resolution failure a run is no reason to say less about.

### 2.5 Suppressed scopes

A repository may take a file, or a region inside one, out of `fmt`'s reach without taking its citations out of grund's. Both scopes suppress **every** rewrite this command performs — trigger-to-marker (§2.1), bare-to-marker (§2.2), shorthand-to-canonical (§2.4), and cross-reference wrapping (§6) — and neither changes anything else. The file is still walked, its citations still resolve, still dangle, still count as edges, and still ground the file they sit in, so `grund check`, `grund refs`, `grund show`, and `grund cover` report exactly what they reported before. Only the edit stops. Decided in [§DF-fmt-suppression](../decisions/functional/DF-fmt-suppression.md#df-fmt-suppression-fmt-suppression-is-per-file-and-per-region-and-the-index-carve-out-outranks-both).

The case this exists for is a citation whose *position* carries meaning: an ASCII topology diagram in an HTML `<pre>` block or a four-space indented block, whose edge labels are citations. Those citations are deliberately live — a fenced block would kill them (§6.4) — but wrapping one into `[§ID](path#anchor)` destroys the alignment the diagram is made of, and in an HTML block it renders as raw link markup. Before these scopes the choice was backticks around every citation in the diagram (an inline code span is never rewritten, §2.3), at the cost of literal backticks in the rendered output, or `[fmt.cross_refs] enabled = false` across the whole project.

A dry run reports nothing for a suppressed scope and `--write` leaves its bytes byte-for-byte identical, so the two agree as §3 requires. Both scopes are idempotent by construction: a run that writes nothing has nothing left to write on the next pass.

**What this costs, and who pays it.** The never-rewrite zones above are intrinsic — `fmt` may not touch a string literal, so [§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation) withholds its error there rather than demanding an edit the tool refuses to make. A suppressed scope is the opposite: the repository asked for it, in its own config or in its own text, and every finding stays. A number-only shorthand written inside one is still a `check` error that `grund fmt --write` no longer clears — the fix is to write the canonical ID, or to lift the suppression around that line. Withholding the finding instead would make a suppressed region a place where citations quietly stop being checked, which is the one thing these scopes are specified not to be ([§DF-fmt-suppression.2.4](../decisions/functional/DF-fmt-suppression.md#24-check-is-unchanged-and-the-repository-pays-for-that)).

#### 2.5.1 `[fmt] exclude` — a file at a time

```toml
[fmt]
exclude = ["docs/architecture/AR-topology.md", "docs/diagrams"]
```

Each entry is a gitignore-style glob resolved against the config root ([§FS-config.3.10](FS-config.md#310-fmt--suppressing-the-rewrite)) — the same dialect the scanner already reads for `[scan] respect_gitignore` ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)), so an entry naming a directory takes every file under it and an entry with no `/` matches at any depth. A file the list matches is walked, read, and checked exactly as before, and no rewrite is performed in it. A malformed pattern is a config error at its own line, like any other bad value ([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)).

The key is additive and the default is the empty list, so `grund_config_version` is unchanged ([§FS-config.5](FS-config.md#5-schema-versioning)). In a workspace every project is rewritten under its own config, so the list is read from the project that owns the file and a member's entries never reach its siblings ([§FS-workspace.8.5](FS-workspace.md#85-grund-fmt---cross-refs)).

#### 2.5.2 `grund:fmt off` / `grund:fmt on` — a region at a time

A comment line whose entire content is `grund:fmt off` suppresses every rewrite from the **next** line onward; one whose content is `grund:fmt on` resumes it. In Markdown the comment is an HTML comment — `<!-- grund:fmt off -->` — and in a source file it is a comment line under the configured `[scan] comment_prefixes` ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)): `// grund:fmt off`, `# grund:fmt on`, `/* grund:fmt off */`, or a line of a Python docstring, which is read for its content like any other doc comment (§2.3.1).

The rules in full:

- The directive line itself is never rewritten, whichever state it leaves behind.
- A region opened with no `on` after it runs to the end of the file. Nothing carries across files: every file starts with the rewrite on.
- A redundant directive is a no-op — `on` where the rewrite is already on, `off` where it is already off — and a stray `on` in a file that never turned the rewrite off changes nothing.
- **In Markdown**, a directive inside a fenced code block is an illustration and toggles nothing, the same reading that makes a citation there dead (§6.4). A directive is inert exactly where a citation is, which is what scopes this to Markdown: a fence is dead text there and `fmt` carries the state to know it, while a fence drawn inside a source file's doc comment is live text — a citation written in one resolves, is found by `grund refs`, and grounds its file ([§FS-check.1.1](FS-check.md#11-recognized-citations)) — so a source-file directive is judged by its comment content alone and a `grund:fmt off` illustrated in a doc comment opens a region like any other. Tracking fences in source files instead would make the citation and the directive disagree about the same three backticks, which is a larger change than this feature and a worse reading of it.
- An inline code span holds a directive without using one, in either kind of file — `` `<!-- grund:fmt off -->` `` and `` // `grund:fmt off` `` are not exact content matches — which is how this document names them above, and how a source file names one it does not want to fire.
- Only an exact content match is a directive: `<!-- grund:fmt off please -->` and `// grund:fmt-off` are ordinary comments, and the text is fixed rather than configured so that a reader meeting one in an unfamiliar repository knows what it is.

The region form is chosen over a rule keyed by declaration section (`AR-topology.2` → do not wrap) because it sits beside the thing it protects and survives the sections around it being renumbered ([§DF-fmt-suppression.2.2](../decisions/functional/DF-fmt-suppression.md#22-an-in-text-region-not-a-rule-keyed-by-declaration-section)).

#### 2.5.3 A kind's index is still linkified

The always-linkify carve-out for a kind's index entries (§6.1, [§DF-index-always-linkified](../decisions/functional/DF-index-always-linkified.md#df-index-always-linkified-the-cross-reference-pass-always-runs-on-a-kinds-index-file)) outranks both scopes, exactly as it already outranks `[fmt.cross_refs] enabled = false`. In an excluded index file, and inside a suppressed region in one, the entries that index owes ([§FS-check.4.6](FS-check.md#46-declaration-missing-from-its-kinds-index)) are still wrapped; every other citation there is suppressed as it would be anywhere else. Without this precedence a suppression could put an index into a state [§FS-check.3.17](FS-check.md#317-index-entry-is-not-a-link) reports and `grund fmt --write` refuses to repair, which is the hole [§DF-index-always-linkified](../decisions/functional/DF-index-always-linkified.md#df-index-always-linkified-the-cross-reference-pass-always-runs-on-a-kinds-index-file) exists to close.

## 3. Outputs

- `0` — no changes needed, **or** `--write` succeeded (regardless of whether changes were made — they were the requested operation, not a failure).
- `1` — `--check` found at least one line that `--write` would change. Never returned by `--write`.
- `2` — I/O error, or a path in the walked tree that could not be read — a broken symlink, a symlink loop, an unreadable directory ([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)). `fmt` walks the tree `grund check` walks and says the same thing about the parts of it that could not be read: one `error: <path>: <reason>` line on stderr per unreadable path ([§FS-errors.2.2](FS-errors.md#22-cli-level-message)), printed after the report, exactly as [§FS-check.2](FS-check.md#2-outputs) does. The rewrite still ran on every file that *could* be read — with `--write` those changes are on disk and in the report — and the `2` says the view of the tree was incomplete, not that nothing happened. Staying silent here would mean the one command that edits files in place is also the one that will not say which files it never saw. The other case is a run that needs the **whole declaration set** before it can rewrite anything — `--cross-refs`, an automatically enabled cross-reference pass, or a shorthand to expand (§2.4) — where an unreadable path is fatal up front and nothing is rewritten at all: wrapping or expanding against a partial set can write a link or an ID that names the wrong declaration, which is the same reason the point queries treat one as fatal ([§FS-show.3](FS-show.md#3-outputs), [§FS-id.4](FS-id.md#4-next-number-derivation)). The completed strict scan still reports every unreadable path it found, once and in the normal deterministic order, even though the first one was already enough to refuse the rewrite; its stdout report stays empty because it rewrote nothing. For a workspace-root run, that completed preflight covers every in-scope project before any project is rewritten: failures are one root-then-members list in ordinary workspace scan order, and a failure in a later member leaves earlier projects byte-for-byte unchanged. It is not the exception it reads as: every `fmt` mode turns the cross-reference pass on by itself wherever the scope holds Markdown and `[fmt.cross_refs] enabled` is set, which generated configs and the built-in default both do (§6.6) — so this is the **ordinary** path in both dry-run and write mode, while the partial-scan path above is what a source-only scope or an explicit opt-out leaves. This fatal-up-front check binds to the **scope**, not to how a caller reached it: `grund fmt --write` with no `<path>` and `grund fmt --write .` name the same scope (§1) and must refuse alike, as must their dry-run counterparts. A run that already holds a scan of that scope — a workspace-root run holding each member's, or the CLI reusing a scan it made resolving the current directory — may skip re-scanning only after confirming that scan met no error; reusing it on the strength of already having a declaration set, without that check, is the same hazard under a different name.

Two exit `2`s that mean opposite things must not be spelled the same, so every strict-abort line says so: `error: nothing was rewritten: <path>: <reason>`, against the partial run's bare `error: <path>: <reason>`. One says the tree was edited and the view of it was short; the other says the tree was not touched. A reader deciding whether to re-run, revert, or fix the link needs to know which.

The report goes to **stdout** — it is `fmt`'s output ([§FS-errors.1](FS-errors.md#1-streams)), the same stream `grund check`'s findings use, so `grund fmt --check | …` and `grund fmt --check > pending.txt` work the way they do for `grund check`. (CLI-level `error:` lines — a bad flag, an I/O failure — go to stderr as everywhere, [§FS-errors.2.2](FS-errors.md#22-cli-level-message).) With `--check` (or no flag, the implicit dry run), the report lists one `path:line: <kind>` line per changed line ([§FS-errors.2.1](FS-errors.md#21-located-finding) shape), where `<kind>` names the rewrite: `trigger → marker` (a typed trigger sequence rewritten to the marker, §2.1), `bare → marker` (a bare citation marked, §2.2, with `--marker`), `shorthand → canonical` (a number-only shorthand expanded, §2.4), or `markdown link` (a citation wrapped or re-derived, §6, with `--cross-refs`).

**A line that expanded a shorthand names the text it will write**, appended to whichever label won: `shorthand → canonical: §FS-042 → §FS-042-user-login`, and `trigger → marker: §FS-042 → §FS-042-user-login` where a typed trigger was marked and expanded in the one pass (§2.4). Several expansions on one line are listed in source order, separated by `, `. The other three labels carry no such detail and need none: they move markup around a citation and leave the ID token byte-identical, so a wrong one dangles, duplicates, or fails to resolve and `grund check` names it. Expanding a shorthand is the one rewrite that writes characters *into* the ID, which is information the source did not carry — a wrong one is a well-formed citation of the wrong declaration and no later pass can see it, so the only review available is before the fact ([§DF-shorthand-numeric-run.2.7](../decisions/functional/DF-shorthand-numeric-run.md#27-invention-is-reported-in-full-whatever-the-rule-decides)). Naming the line and the rewrite class is enough to review the former and not the latter. With `--write`, the report names what changed on disk — on stdout, not the stderr transcript shape `grund init` uses ([§FS-errors.6](FS-errors.md#6-the-grund-init-transcript)): a `rewrote N line(s):` summary line, then one `  <path> (<count>)` line per file touched, in lexicographic path order, where both counts are changed-line counts (an empty change set prints `rewrote 0 lines` with no list). The file system carries the actual change; the summary is so a reviewer can see which files to re-inspect without diffing the whole tree.

## 4. Why this exists

Three reasons:

1. **Onboarding.** Adopting the marker scheme on an existing repo requires rewriting hundreds of citations. `grund fmt --marker --write` does it in seconds.
2. **CI safety net.** A contributor who bypasses the IDE plugin (e.g., edits via the GitHub web UI) leaves bare triggers in place. `grund fmt --check` in CI catches it.
3. **Pre-commit hook.** Run on staged files; transform locally before commit. Keeps the canonical form in version control.

## 5. Configurability

Marker, trigger, and the recognized `KIND` set are read from `grund.toml` per [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable). The defaults are `§` and `$$` as decided in [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger).

Which *files* the command may rewrite is configurable too: `[fmt] exclude` takes a file out of every rewrite while leaving it walked and checked (§2.5.1). The per-region counterpart is written in the file rather than in the config (§2.5.2).

## 6. Cross-reference emission

A free convenience layer on top of the ID system: render each citation as a clickable cross-reference to the declaration body — without giving up any of the polyglot, refactor-safe properties IDs already provide. Decided in [§DF-md-link-emission](../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations).

A "cross-reference" is whatever construct the surrounding markup uses to point at another location — a Markdown inline link `[text](url#anchor)`, an AsciiDoc `xref:`, a reStructuredText `:ref:`. **Today, `--cross-refs` emits exactly one form: the Markdown inline link, and only in `.md` files (§6.1).** The flag, and the `[fmt.cross_refs]` config block (§6.7), are named for the general concept on purpose: a later `grund` that learns a second markup family emits that family's cross-reference syntax in those files under the *same* flag, with its settings under the *same* config block — an additive change, no new flag and no `grund_config_version` bump. Language-specific cross-references are deliberately not in scope yet (getting each renderer's anchor algorithm exactly right is the same kind of fidelity work the Markdown profiles already needed — [§DF-github-anchor-fidelity](../decisions/functional/DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly)); the name just leaves the door open.

### 6.1 Scope

`--cross-refs` runs **only on files with the `.md` extension** in the configured scan set. Source files are never touched: their host languages do not render Markdown, and rewriting a comment in `src/bus.rs` to inject `[…](…)` syntax is at best noise and at worst a parse error. The polyglot citation grammar (`§GOAL-polyglot-citation`) is the universal form; cross-reference emission is the rendered view of it — Markdown today, with room for other markup families later (the introduction above).

**A kind's index entries are always linkified.** The index a `[[kinds]]` entry names ([§FS-config.3.4](FS-config.md#34-kinds--recognized-kinds)) is a region the cross-reference pass runs on regardless of `[fmt.cross_refs] enabled` (§6.7) — the mirror image of §2.3's never-rewrite zones, one region `fmt` always writes rather than one it never does. `grund check` requires an index entry to be a full link ([§FS-check.3.17](FS-check.md#317-index-entry-is-not-a-link)) and takes the [§REQ-backwards-compatibility.3](../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) licence to do so in the release it arrives, on the grounds that `grund fmt --write` is the fix; under `enabled = false` that would otherwise be false, and the fix would become `grund fmt --cross-refs --write`, which linkifies the whole tree to repair one file. The carve-out is what makes the requirement cost nothing in every configuration.

The carve-out is scoped to the **entries**, not to the page. Reached this way — that is, only where `enabled = false` would otherwise have skipped the file — the pass wraps the citations the index owes an entry for ([§FS-check.4.6](FS-check.md#46-declaration-missing-from-its-kinds-index)) and leaves every other citation in the file alone: a mention of a foreign ID in the prose around the list is an ordinary citation in an ordinary file, and a repository that turned generated links off asked for it to stay bare. An external inline declaration enters this owed set only while its canonical enrollment link already exists; the carve-out therefore preserves that form but does not infer membership from an ordinary external citation or repair a link whose noncanonical destination makes it ordinary. Under the default `enabled = true` nothing is scoped, because the pass was already running on that file for its own reasons; a marker-prefixed bare-ID citation that the ordinary pass wraps canonically acquires the enrollment meaning of its stored form.

It follows the pass's own gate, in both modes: the dry run previews the index-entry wraps that `--write` applies even when `[fmt.cross_refs] enabled = false`. It needs no `.md` test of its own: `index` must name a Markdown file ([§FS-config.3.4](FS-config.md#34-kinds--recognized-kinds)), which is a rule this scope clause is the reason for. Decided in [§DF-index-always-linkified](../decisions/functional/DF-index-always-linkified.md#df-index-always-linkified-the-cross-reference-pass-always-runs-on-a-kinds-index-file).

### 6.2 Form

Wrap the citation. A bare or marker-prefixed citation (illustrated as `§FS-<foo>.3.1`) becomes:

```
[§FS-<foo>.3.1](<relative-path>#<anchor>)
```

- `<relative-path>` — path from the file containing the citation to the file containing the declaration, in POSIX form (`../functional-spec/FS-<foo>.md`). When the declaration's home is in source code (a stub points at `src/foo.rs`), the link targets the source file directly with no anchor — the host renderer will not jump inside a doc-comment, but the link still leads to the right file.
- `#<anchor>` — a heading anchor, present whenever the declaration's home is a Markdown file (and the active profile is not `none`). For a `.<section>` citation it is the cited section's heading; for a bare-ID citation it is the declaration's own heading — `§GOAL-<x>` → `[§GOAL-<x>](goals.md#goal-x-the-title)` rather than a bare link to `goals.md` ([§DF-declaration-anchor](../decisions/functional/DF-declaration-anchor.md#df-declaration-anchor-a-bare-id-markdown-link-points-at-the-declarations-heading-anchor)). The anchor is the heading's **rendered text** slugified per the configured renderer profile (§6.7) — for the default `github` profile, `### 6.2 Form` produces `#62-form`. "Rendered text" matters when the heading itself contains an inline link (including a citation that `--cross-refs` has already wrapped, §6.4) or an HTML-tag-shaped span: `## 4. Refining [§FS-<x>.1](FS-x.md#1-y)` slugifies as if it read `## 4. Refining §FS-<x>.1` (the destination URL is not part of the text), and `## RM-read: grund <ID>` slugifies as `## RM-read: grund ` (the `<ID>` is dropped) — exactly as a Markdown renderer treats them. The `github` (and `gitlab`) profile then reproduces `github-slugger` byte-for-byte: disallowed characters are deleted in place and each remaining space becomes one `-`, with no run-collapsing and no trailing-`-` trim — `## A — B` → `#a--b`, `` ## 6. Watch mode (`--watch`) `` → `#6-watch-mode---watch` ([§DF-github-anchor-fidelity](../decisions/functional/DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly)). The full strategy and profile list is decided in [§DF-md-link-anchor-strategy](../decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass). When the home is a source file (a stub points at `src/x.rs`) the link is the bare file path with no anchor — a renderer will not jump inside a doc-comment; when the active profile is `none`, the anchor is omitted regardless.

The citation text inside the brackets is preserved verbatim, including the marker. A reader scanning the rendered Markdown sees the citation exactly as before; only now it is clickable.

### 6.3 Idempotency and re-derive

Per [§DF-md-link-anchor-strategy.2.2](../decisions/functional/DF-md-link-anchor-strategy.md#22-re-derive-on-every-pass-supersede-fs-fmt63), every `grund fmt --cross-refs` pass recomputes the canonical URL inside each existing wrap and rewrites if it differs. This makes `fmt` a normalizer, not a preserver: a heading rename or a file move that invalidates a wrap produces a one-line `fmt` diff on the next pass, instead of a silently-broken link.

Idempotency holds: a second run with no intervening edits is a no-op, because the URL on disk is now equal to the canonical URL.

Detection of an existing wrap, for both the rewrite and the no-double-wrap rules: the citation's immediately-preceding character is `[` and its immediately-following text begins `](`. When this matches, the wrapper computes the canonical URL and replaces the existing one if different. When it does not match, the citation is wrapped fresh.

### 6.4 What is never wrapped

In addition to the never-rewrite rules in §2.3:

- Citations inside fenced code blocks (the same skip used by §2.3 / `grund fmt`'s existing trigger pass). Code samples often illustrate citations as plain tokens; rewriting them changes what the docs claim.
- Citations inside inline code spans (between backticks).
- Citations on a declaration heading line. The marker is for citations, not declarations (§2.3).
- Citations whose declaration cannot be located by the scanner. A dangling citation is a `grund check` error; `fmt` does not paper over it by emitting a link to a nonexistent file. Report the unwrapped citation; let `check` flag the underlying problem.
- Citations in a suppressed scope — a file `[fmt] exclude` names, or a `grund:fmt off` region (§2.5) — with one exception: a kind's index entries are wrapped there anyway, because the always-linkify carve-out of §6.1 outranks the suppression (§2.5.3).

### 6.5 Interaction with `--marker`

`--cross-refs` operates on marker-prefixed citations. When run together with `--marker`, the marker pass runs first (bare → marker), then the link pass wraps the now-marker-prefixed citations. When run without `--marker`, bare citations are left bare and unwrapped — wrapping only the marked ones gives a consistent, predictable output instead of two mixed forms.

### 6.6 Why generated configs enable cross-references

Generated `grund.toml` files set `[fmt.cross_refs] enabled = true`, and the built-in default is the same. This makes rendered Markdown useful by default while keeping the ID citation as the source of truth. The default favors GitHub code review and external discovery over the cleaner editor-only source view, per [§DF-md-link-default-on](../decisions/functional/DF-md-link-default-on.md#df-md-link-default-on-markdown-cross-reference-links-default-on-for-github-review-and-discovery):

1. The source text still contains the exact citation, only wrapped as `[§ID](target)`; `grund check`, `grund show`, and `grund refs` continue to resolve the citation, not the Markdown URL.
2. The pass runs automatically for every `grund fmt` scope that contains Markdown files, in both dry-run and write mode. The dry run reports exactly the set of changes `--write` applies. Source-only scopes such as `grund fmt src/app.rs --write` stay on the lightweight marker/trigger path unless `--cross-refs` is passed.
3. Repos that do not want generated Markdown links can set `enabled = false`; the generated config writes the key explicitly so the opt-out is visible.
4. Projects with non-GitHub renderers keep the default link behavior but choose a matching `anchor_format` (§6.7), instead of disabling links entirely.

### 6.7 Configurability

```toml
[fmt.cross_refs]
enabled       = true       # default; false opts out of generated Markdown links
anchor_format = "github"   # default; named renderer profile per §DF-md-link-anchor-strategy.2.3
```

`[fmt.cross_refs]` is the home for cross-reference settings. Today it carries two keys — `enabled` (the default-on toggle for `fmt`) and `anchor_format` (which renderer's anchor-slug algorithm the Markdown link form uses). `anchor_format` accepts one of the named profiles defined in [§DF-md-link-anchor-strategy.2.3](../decisions/functional/DF-md-link-anchor-strategy.md#23-renderer-profiles):

- `github` (default) — GitHub's slugger; covers the most common host.
- `gitlab` — GitLab's slugger.
- `mkdocs` — MkDocs / Python-Markdown TOC extension's slugger.
- `pandoc` — Pandoc's `auto_identifiers` algorithm.
- `none` — emit no anchor; produce a file-level link with no fragment.

When `enabled = true`, the cross-reference pass runs on every `grund fmt` invocation whose rewrite scope contains at least one Markdown file, without requiring `--cross-refs`; dry-run and write mode make the same decision. Source-only scopes do not pay the full-project link-target scan because no file in that scope can be wrapped. When `enabled = false`, both modes skip link emission unless `--cross-refs` is passed for that invocation, apart from the index carve-out in §6.1. When a future `grund` adds a second markup family (the introduction to §6), that family's settings live under this same `[fmt.cross_refs]` block (a new key, or a sub-table such as `[fmt.cross_refs.asciidoc]`) — additive, so a v1 config that only set `anchor_format` keeps working and `grund_config_version` is unchanged ([§FS-config.5](FS-config.md#5-schema-versioning) bump rules).

What is **not** here is the per-file opt-out: `[fmt] exclude` (§2.5.1) sits in the sibling `[fmt]` table because it governs every rewrite the command performs, not just this pass ([§DF-fmt-suppression.2.1](../decisions/functional/DF-fmt-suppression.md#21-one-general-fmt-exclude-not-one-exclude-per-pass)). Its files, and any `grund:fmt off` region (§2.5.2), are skipped by this pass with them — apart from a kind's index entries, which the §6.1 carve-out wraps regardless (§2.5.3).

### 6.8 Measurable

E2E fixtures cover: wrap-on-first-run; dry-run/write parity when the default `enabled = true` causes both modes to run the cross-reference pass without the flag; source-only scopes skipping the default link-target scan in both modes; `enabled = false` preserving trigger-only behavior in both modes unless `--cross-refs` is passed, while the §6.1 index carve-out is still previewed and written; changed-line summary counts; no-op on second-run (idempotency); re-derive on heading rename (a wrap pointing at the old slug is rewritten to the new one in a single `fmt` pass); re-derive on file move; correct relative path across `docs/` subdirectories; a bare-ID citation linking to the declaration's own heading anchor ([§DF-declaration-anchor](../decisions/functional/DF-declaration-anchor.md#df-declaration-anchor-a-bare-id-markdown-link-points-at-the-declarations-heading-anchor)); source-file declaration link with no anchor; `anchor_format = "none"` produces file-only links; each named renderer profile (`github`, `gitlab`, `mkdocs`, `pandoc`) produces its expected slug for a curated heading set — for `github`, that set includes headings whose punctuation closes up into runs of `-` that GitHub keeps and a naive collapser would not (`## A — B` → `#a--b`; [§DF-github-anchor-fidelity](../decisions/functional/DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly)) and a section heading that itself carries a citation, with another citation pointing at that section (the anchor derives from the heading's rendered text, so it is identical before and after `--cross-refs` wraps the heading's own citation — i.e. the wrap is idempotent over a citation that lives in a section heading); fenced-block exemption; dangling-citation skipped; declaration-line skipped; and `--cross-refs` without `--marker` on a tree containing both forms.

The suppressed scopes of §2.5 carry their own fixtures: an excluded file silent in dry run and byte-identical under `--write` while its `check` and `refs` output is unchanged; a region protecting an HTML `<pre>` diagram while ordinary prose in the same file is still wrapped; all four rewrites suppressed in each scope, a typed `$$` trigger included; a directive inside a fenced block toggling nothing in Markdown; an `off` with no `on` running to the end of the file; a stray `on` changing nothing; the source-file comment form (`fmt-directive-in-source-comment`), where the same directive illustrated inside a doc comment's own fence does open a region (§2.5.2); idempotency on a second pass; and a kind's index entries still wrapped under both scopes (§2.5.3).
