# FS-cover: grund groups citations by scanned file

The `cover` subcommand exposes the citation graph as data: for each scanned file, which spec IDs does it cite, and where? This is the plumbing surface for the diff-aware co-change recipe ([§RM-cochange-gate](../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)): git decides what changed, `cover` says which IDs the changed files lean on. Serves [§RM-cover](../roadmap.md#rm-cover-grund-cover) and keeps the policy layer out of `grund-core`.

## 1. Inputs

```
grund cover [<path>] [--format text|json]
```

- `<path>` — directory or file whose tree is scanned. Defaults to `.`. Discovery is the same as every other subcommand (walk up to `.agents/grund.toml`, else defaults — [§FS-config.1](FS-config.md#1-file-location-and-discovery)).
- `--format text|json` — output shape (§3). Default `text`.

`cover` is a query, like `list` and `refs` — non-interactive, no prompts ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)). It reads no git history ([§FS-non-goals.6](FS-non-goals.md#6-decision-database-audit-log-history-tracking)) and parses no AST ([§FS-non-goals.3](FS-non-goals.md#3-code-ast-parsing)).

## 2. Behaviour

`cover` runs the same scan as `check`, `list`, and `refs` ([AR-scanner](../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations)). It does not decide whether a file is sufficiently covered, whether a hunk is behavioral, or whether a spec/test co-change is required; those are recipe concerns ([§RM-cochange-gate](../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)). The command only renders the `Findings` the scanner already collected.

Output is grouped by scanned file, sorted by path. Within a file, citations are sorted by `(line, column)`. Files with no recognised citations are still included, so a caller can distinguish "the file was scanned and cites nothing" from "the file was outside the scan scope." A citation object is the same shape `grund refs --format=json` emits: path, line, column, rendered ID, optional section, marker boolean, and the verbatim token text.

## 3. Outputs

### 3.1 `--format text` (default)

Text output goes to stdout. It prints a heading line per scanned file, followed by either the citation line/column and token, or `(no citations)`:

```
$ grund cover src/
src/login.rs:
  14:5 §FS-login.2
  28:9 §DF-password-policy
src/untouched.rs:
  (no citations)
```

### 3.2 `--format json`

NDJSON on stdout — one object per scanned file:

```json
{"path":"src/login.rs","citations":[{"path":"src/login.rs","line":14,"column":5,"id":"FS-login","section":"2","marker":true,"text":"§FS-login.2"},{"path":"src/login.rs","line":28,"column":9,"id":"DF-password-policy","section":null,"marker":true,"text":"§DF-password-policy"}]}
{"path":"src/untouched.rs","citations":[]}
```

The nested citation objects intentionally carry `path` too, matching `refs` JSON byte shape so a caller can compare `cover` and `refs` without a field mapping layer.

## 4. Exit codes

- `0` — scan succeeded; the emitted file records are the result.
- `2` — scan / I/O error ([§FS-check.2](FS-check.md#2-outputs) partial-scan semantics apply: records found before or after the unreadable file may print, but the result is not trustworthy as complete).

There is no `1`: `cover` is a query over the current tree and has no finding class of its own.

## 5. Why this exists

`grund refs <ID>` answers "who cites this ID?" and `grund list` answers "what IDs exist?". The co-change gate needs the inverse grouping: "for this changed file, what IDs does it cite?" A shell script could run `grund refs` once per ID and regroup the output, but that is slower, loses files with zero citations, and makes every recipe reconstruct scanner state. `cover` provides that view directly while preserving the no-git, no-policy boundary: git diff is an input to the recipe, not to `grund cover`.
