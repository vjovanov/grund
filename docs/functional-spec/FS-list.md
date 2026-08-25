# FS-list: grund lists every declared ID

The `list` subcommand prints the repo's ID catalog: every declaration, where it lives, and its one-line title — or, narrowed with `--kind`/`--summary`, just the slice an agent asked for. It is the index that `grund <ID>` reads from and the broad counterpart of `grund refs` — `refs` answers "who cites *this* ID?", `list` answers "what IDs are there?". An agent that has been told to ground itself with `grund <ID>` needs a way to discover the `<ID>`s; a human auditing a spec tree needs the same. Serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (no `grep` for `^# [A-Z]+-` across the tree), [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file), and the agent-grounding loop in [§GRUND-grund](../grund.md#grund-grund-agents-stay-grounded-in-the-spec).

## 1. Inputs

```
grund list [<path>] [--kind <KIND>[,<KIND>…]]… [--unused] [--summary] [--format text|json]
```

- `<path>` — directory or file whose tree is scanned. Defaults to `.`. Discovery is the same as every other subcommand (walk up to a `grund.toml`, else defaults — [§FS-config.1](FS-config.md#1-file-location-and-discovery)).
- `--kind <KIND>[,<KIND>…]` — list only declarations whose ID has one of the named kind prefixes (each a configured `[[kinds]]` prefix — [§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)). Accepts a comma-separated list (`--kind FS,AR`) and may be repeated (`--kind FS --kind AR`); the selections union. So an agent that wants only the specs and the architecture runs `grund list --kind FS,AR` instead of dumping the whole catalog. An unknown kind *anywhere* in the selection is a CLI-level error (§4): a typo'd `--kind` must not silently produce an empty — or merely short — catalog.
- `--unused` — list only declarations that no recognised citation points at, **excluding `E2E` cases unless `E2E` is explicitly selected with `--kind`** — the same set `check` warns on ([§FS-check.4.1](FS-check.md#41-unused-declaration)). An e2e case is a proof artifact, exercised by being run, not a citation target, so it is uncited by construction and would only ever bury the actionable signal (uncited specs, decisions, goals) in a bare `--unused` query. A citation that is a kind's own index entry ([§FS-check.4.6](FS-check.md#46-declaration-missing-from-its-kinds-index)) is discounted here too, and for the same reason `check` discounts it ([§FS-check.4.1](FS-check.md#41-unused-declaration)): an index names every declaration in its folder by construction, so counting its entries would empty this query of everything an indexed folder holds. To inventory uncited e2e cases anyway, include `E2E` in the kind filter: `--unused --kind E2E` lists uncited cases only, while `--unused --kind FS,E2E` lists uncited `FS` declarations plus uncited `E2E` cases because `E2E` was explicitly requested.
- `--summary` — instead of one line per declaration, print one line per kind: the kind prefix, its declaration count, and its configured `[[kinds]]` home (`file` or `folder`, §3.3). The catalog's shape at a glance — how many IDs of each kind there are and where their declarations live — without the full list. Composes with `--kind` (summarise only those kinds) and `--unused` (count only the uncited declarations, with the same `E2E` suppression this section describes for the per-declaration form).
- `--format text|json` — output shape (§3). Default `text`.

`list` is a query, like `show` and `refs` — non-interactive, no prompts ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)).

## 2. Behaviour

`list` runs the same scan as `check` ([AR-scanner](../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)) and emits, for every declaration the scan found, one catalog line. The set of declarations is exactly the set `check` validates and `show` can resolve, so the three never disagree on what exists.

- **Order.** Declarations come out sorted by ID — kind, then number, then slug — the same stable order `check` reports diagnostics in ([§FS-errors.4](FS-errors.md#4-determinism)). The result is deterministic for a given tree.
- **Inline homes stay canonical.** When an ID's home is an inline declaration in source code with a one-line stub under `docs/architecture/` pointing at it (the [§FS-check.3.4](FS-check.md#34-broken-inline-spec-stub) / [§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments) arrangement), `list` shows **one** line for that ID, naming the source file where the body lives — not two lines, one for the stub and one for the inline declaration. An external inline declaration enrolled directly by its kind's index ([§FS-check.4.6](FS-check.md#46-declaration-missing-from-its-kinds-index)) likewise appears once at the source home: the index link creates no declaration to collapse. A *broken* stub (its target missing, or the target has no matching inline declaration) is not paired with anything, so it does appear, listed at the stub's own location with a `→ <target>` note; `check` reports the breakage in located form.
- **Duplicate declarations.** When an ID is declared in more than one independent home — the [§FS-check.3.3](FS-check.md#33-duplicate-declaration) error — `list` prints one line per home, each flagged so the duplication is visible at a glance. `list` does not pick a winner; it shows the situation and leaves the located error to `check`.
- **What it is not.** `list` does not print declaration *bodies* (that is `grund <ID>`), and it does not list *citations* (that is `grund refs <ID>`). It does not modify anything. An ID that is cited but never declared does **not** appear in `list` — it has no declaration to catalog; `grund refs <ID>` and `grund check` are where a dangling citation surfaces.

## 3. Outputs

### 3.1 `--format text` (default)

One line per catalog entry on **stdout** (this is a result a caller consumes and pipes, like `grund <ID>` / `grund id` / `grund config show`, not diagnostic output):

```
$ grund list
AR-event-bus    src/bus.rs:14                 In-process event broadcaster
FS-check        docs/functional-spec/FS-check.md:1    grund validates every reference in a repo
FS-login        docs/functional-spec/FS-login.md:1    A player can log in with email
G-no-dangling-refs  docs/goals.md:7     every cited ID resolves to a declaration
```

The columns are: the ID (rendered in the repo's `[id] format`, left-padded so the column aligns — capped so one very long ID does not blow out the table), then `<path>:<line>` of the home declaration (for a collapsed stub-and-inline pair, the source file the body is in), then the title — the heading text the author wrote after `<ID>:`. A declaration whose heading carries no `: <text>` tail has an empty title column. A broken stub shows `→ <target>` in place of a title. A duplicated ID's lines carry a `(duplicate declaration — grund check)` note. With `--kind`, only the selected kinds' lines appear; with `--unused`, only lines for declarations with zero inbound citations, with `E2E` cases excluded by default (the same suppression `check`'s unused-declaration warning applies, [§FS-check.4.1](FS-check.md#41-unused-declaration)) and re-included whenever `E2E` is one of the explicitly selected kinds. An empty catalog (or an empty filter result) prints nothing — that is not an error.

Stderr is empty on success.

### 3.2 `--format json`

NDJSON on stdout — one object per catalog entry, same order as the text form:

```json
{"id":"AR-event-bus","kind":"AR","path":"src/bus.rs","line":14,"title":"In-process event broadcaster","stub":false,"defines":null,"refs":3,"duplicate":false}
{"id":"FS-login","kind":"FS","path":"docs/functional-spec/FS-login.md","line":1,"title":"A player can log in with email","stub":false,"defines":null,"refs":7,"duplicate":false}
```

Fields: `id` (rendered ID), `kind`, `path` and `line` of the home declaration, `title` (`null` when the heading has no title tail or the home is a broken stub), `stub` (true when this entry's home is a stub heading — only ever true for a *broken* stub, since a healthy one collapses into its inline declaration), `defines` (the `<target>` of a stub heading, else `null`), `refs` (the count of recognised citations of this ID across the scanned tree — exactly the number `grund refs` would list, carried on every entry so a tool need not run `grund refs` per ID to learn it. It is a *count of citations*, not the `--unused` predicate: where a kind's index is checked ([§FS-check.4.6](FS-check.md#46-declaration-missing-from-its-kinds-index)) that count includes the ID's index entry, which `--unused` discounts (§1), so an entry selected by `--unused` may carry `refs: 1`. A tool filtering for uncited IDs should pass `--unused` and read the rows, not threshold on `refs`), and `duplicate` (true when the ID has more than one home). The wire form is stable per [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path).

### 3.3 `--summary`

`grund list --summary` prints one line per kind — in the configured `[[kinds]]` order, for the kinds that have at least one declaration in scope:

```
$ grund list --summary
AR    7    docs/architecture
DA    3    docs/decisions/architectural
DF    9    docs/decisions/functional
FS   18    requirements.md
…
```

Columns: the kind prefix, the count of declarations of that kind the scan found (after `--kind` / `--unused` narrowing, if any), and that kind's configured home (`file` for single-file kinds, otherwise `folder`; [§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)) — so one line tells an agent both how big each slice of the catalog is and where to look. A kind with zero declarations in scope is omitted; an empty result (every kind empty, or `--kind` narrowed to kinds with no declarations) prints nothing — not an error. With `--kind FS,AR --summary` only those rows appear; with `--unused --summary` the counts are of uncited declarations — the same set the per-declaration `--unused` lists (`E2E` excluded unless `E2E` is explicitly selected by `--kind`, including in a multi-kind selection such as `--kind FS,E2E`). `--format json` together with `--summary`: NDJSON, one object per kind, `{"kind":<prefix>,"title":<[[kinds]] title>,"home":<file-or-folder>,"count":<n>}`, same order. Exit codes (§4) are unchanged.

## 4. Exit codes

- `0` — the scan succeeded; the listed catalog (possibly empty) is the result.
- `2` — scan / I/O error ([§FS-check.2](FS-check.md#2-outputs) partial-scan semantics apply: an incomplete scan exits `2` and the catalog may be short), an unknown `--kind` (any value in a comma-separated or repeated `--kind`), an unsupported `--format`, or any other CLI-level error ([§FS-cli.4](FS-cli.md#4-errors-with-no-source-location)).

There is no `1`: `list` is a query that always returns *its* answer (a possibly-empty catalog), never "found something other than one body" — unlike `show`, it has no single-result expectation to violate.

## 5. Why this exists

`grep -RhoE '^#+ [A-Z]+-[a-z0-9-]+'` across `docs/` gives a contributor a rough list of declaration headings but cannot: reach inline declarations inside source-code doc-comments; collapse a stub onto the inline declaration it points at; honour the configured `[id]` grammar in a repo that customised it; tell which declarations are uncited; or produce a stable, machine-shaped result an agent can program against. `list` is the scheme's own answer, sharing the scanner with `check` so the catalog and the validator never disagree on what a declaration is. Multi-kind `--kind FS,AR` and `--summary` give an agent a scoped slice or a bird's-eye count of the catalog instead of the full dump — the discovery half of token-cheap grounding. With `show` and `refs` it completes the read surface: `list` enumerates the IDs, `show` reads the body one promises, `refs` enumerates who took the promise.
