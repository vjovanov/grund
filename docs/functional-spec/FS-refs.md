# FS-refs: grund lists every citation of an ID

The `refs` subcommand answers the reverse of `grund <ID>`: not "what does this ID say?" but "who points at it?". An agent about to change a declaration — or delete one — needs to know what leans on it; `grund refs FS-check` is that lookup, scheme-aware in the ways a `grep` cannot be, with a compact `--summary` for a quick blast-radius read. Serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible), [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file), and the agent-grounding loop in [§GRUND-grund](../grund.md#grund-grund-agents-stay-grounded-in-the-spec) (an agent verifying a change reads the cited bodies *and* the back-references).

## 1. Inputs

```
grund refs <ID> [<path>] [--section <s>] [--summary] [--format text|json]
```

- `<ID>` — the ID to look up, without the marker. May carry an inline section (`FS-check.3.1`, or `FS-plan.goals.performance` when named sections are enabled) using the configured `[id] section_separator`; equivalently pass `--section 3.1` or `--section goals.performance`. An `<ID>` that does not match the repo's `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) is rejected before the scan with `error: invalid ID \`<arg>\`` followed by `hint: this repo's [id] format is \`<format>\` (run \`grund config show\`); \`grund list\` shows the IDs that exist` on stderr, exit `2` (§4) — the same hint the ID query gives for the same stumble ([§FS-show.3](FS-show.md#3-outputs)), the common surprise in a repo whose format differs from the `{kind}-{slug}` `grund` itself uses. The number-only shorthand is accepted where the repo's format has one — `grund refs FS-042` lists the citations of `FS-042-user-login`, including any written in the shorthand themselves ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)) — and a shorthand matching more than one declaration is rejected with the candidate list rather than resolved to a guess.
- `<path>` — directory or file whose tree is scanned. Defaults to `.`. Discovery is the same as every other subcommand (walk up to a `grund.toml`, else defaults — [§FS-config.1](FS-config.md#1-file-location-and-discovery)).
- `--section <s>` — restrict to citations that reference exactly that numeric or named section path. Without it, every citation of `<ID>` is listed regardless of section (including bare-ID citations with no section). Mutually exclusive with the dotted inline form. The result comes from the scanner's shared citation record, so a named filter cannot disagree with `check`, `show`, completion, or the LSP about the coordinate.
- `--summary` — collapse the per-citation lines into one line per citing **file**: the file path, the count of citations in it, and their line numbers (§3.3). The compact form for a blast-radius scan — how many files lean on `<ID>`, and where — where the full per-citation list would repeat the same path many times.
- `--format text|json` — output shape (§3). Default `text`.

`refs` is a query, like `show` — non-interactive, no prompts ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)).

## 2. Behaviour

`refs` runs the same scan as `check` ([AR-scanner](../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)) and emits, for the requested `<ID>`, every recognised citation site — the same set of citations `check` would validate, so it honours `[reference] strict` (bare tokens are listed only in non-strict mode), the string-literal carve-out in source files ([AR-scanner.2.3](../architecture/AR-scanner.md#23-citation-detection)), and citations inside doc-comments. It does **not** list the *declaration* of `<ID>` — that is `grund <ID> --format=json` (the README documents that one-liner). A `refs` lookup of an ID that has no declaration still works: the citations are listed (they are exactly the ones `check` flags as dangling), so `refs` is also the "what would break if I never create this ID" tool.

Output is sorted by `(path, line, column)` — deterministic per [§FS-errors.4](FS-errors.md#4-determinism). The citation list is the command's *result*, so it goes to **stdout** — text lines and `--format json` NDJSON alike, the same stream `grund list` and `grund cover` use ([§FS-errors.1](FS-errors.md#1-streams)). A `refs` line shares the `path:line: <text>` located-finding shape ([§FS-errors.2.1](FS-errors.md#21-located-finding)) so an editor can jump to it, but it is an *answer*, not a diagnostic; stderr is left for errors and the typo hint below. An ID with zero citations produces empty output and exit `0` (not an error: an as-yet-uncited declaration is normal, and `check` already warns about it — [§FS-check.4.1](FS-check.md#41-unused-declaration)). If the requested ID is *also* not declared anywhere in the scanned tree, the likeliest cause is a typo, so `refs` prints one `note:` line to **stderr** — `note: <ID> is neither declared nor cited — run \`grund list\` to see every declared ID` — and still exits `0`. The note is a hint, not part of the result: the empty stdout (no text lines, no NDJSON) is unchanged, so machine consumers that only read stdout never see it. This mirrors the `ID not found` hint the ID query gives for the same mistake ([§FS-show.3](FS-show.md#3-outputs)) without that query's exit `1` — `refs` has no single-result expectation to violate (§4).

## 3. Outputs

### 3.1 `--format text` (default)

One line per citation site on **stdout**, in the located-finding shape ([§FS-errors.2.1](FS-errors.md#21-located-finding)):

```
$ grund refs FS-check.1
docs/functional-spec/FS-show.md:11: §FS-check.1
crates/grund-core/src/scanner.rs:142: FS-check.1
```

`<message>` is the citation token exactly as it appears in the source — marker-prefixed or bare, with its section suffix — so the reader sees the form on disk. The lines *are* the result, so `grund refs <ID> | …` and `grund refs <ID> > out.txt` work the way they do for `grund list` — no `2>&1` needed (§2). Exit `0` always when the scan succeeds, regardless of how many citations were found.

### 3.2 `--format json`

NDJSON on stdout — one object per citation, matching the `Citation` shape ([AR-scanner.3](../architecture/AR-scanner.md#3-output)) plus the verbatim token:

```json
{"path":"docs/functional-spec/FS-show.md","line":11,"column":42,"id":"FS-check","section":"1","marker":true,"text":"§FS-check.1"}
{"path":"crates/grund-core/src/scanner.rs","line":142,"column":12,"id":"FS-check","section":"1","marker":false,"text":"FS-check.1"}
```

`section` is `null` for a bare-ID citation with no section coordinate.

### 3.3 `--summary`

`grund refs <ID> --summary` emits one line per citing **file** instead of one per citation site, sorted by path:

```
$ grund refs FS-check --summary
docs/functional-spec/FS-show.md: 3 (lines 11, 142, 200)
crates/grund-core/src/scanner.rs: 1 (line 142)
```

The shape is `<path>: <count> (lines <l1>, <l2>, …)` — the count is the number of citation sites from exactly the citation set §3.1 lists (so `--summary` honours `[reference] strict`, the string-literal carve-out, and doc-comment citations the same way), while the line list is the sorted, de-duplicated set of source lines that contain those citations. If two citations appear on line 10, the count includes both but the line list contains `10` once: `path: 2 (line 10)`. This makes `grund refs <ID> --summary | wc -l` the number of files that lean on `<ID>` while the line list still points an editor at every line that contains at least one site. With `--section`, the aggregate is over citations of that section only. An ID with no citations prints nothing and exits `0` — same as §3.1, and the "neither declared nor cited" `note:` on stderr (§2) is unaffected. `--format json` together with `--summary`: NDJSON, one object per file, `{"path":<path>,"count":<n>,"lines":[<unique l1>,<unique l2>,…]}`, same order; the per-citation object form (§3.2) is what you get *without* `--summary`. Exit codes (§4) are unchanged — `--summary` is a rendering of the same scan result, not a different query.

## 4. Exit codes

- `0` — scan succeeded; the listed citations (possibly none) are the result.
- `2` — scan / I/O error ([§FS-check.2](FS-check.md#2-outputs) partial-scan semantics apply: an incomplete scan exits `2` and the lookup is not trustworthy as complete), an `<ID>` argument that does not match the configured `[id] format` (§1), a number-only shorthand naming more than one declaration ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)), an unsupported `--format`, or any other CLI-level error ([§FS-cli.4](FS-cli.md#4-errors-with-no-source-location)). The `[id] format` hint accompanies only the first of those: an ambiguous shorthand *did* match the format and already named every candidate, so repeating the format there would send the reader to `grund config show` for a config that is not the problem.

There is no `1`: `refs` is a query that always returns *its* answer (a possibly-empty list), never "found something other than one body" — unlike `show`, it has no single-result expectation to violate.

## 5. Why this exists

`grep -oE '§…'` gives a contributor a rough back-reference list but cannot: distinguish a real citation from an ID-shaped substring in a string literal; respect `strict` mode; reach citations inside block doc-comments without language-specific regex; or produce a stable, machine-shaped result for an agent to program against. `refs` is the scheme's own answer, sharing the scanner with `check` so the two never disagree on what counts as a citation. `--summary` folds a wide back-reference set to one line per file, so the blast radius before changing a declaration is legible at a glance — token-cheap for an agent that needs the count and the file list, not every column. Together with `grund <ID>` it closes the loop: the ID query reads the body an ID promises, `refs` enumerates the code and docs that took the promise.
