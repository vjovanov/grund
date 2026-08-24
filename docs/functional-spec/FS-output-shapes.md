# FS-output-shapes: machine-readable output shapes

This file is the verbose output-shape companion to [§FS-errors](FS-errors.md#fs-errors-grund-emits-messages-in-fixed-shapes). It collects the JSON/text envelopes that are spread across [§FS-check](FS-check.md#fs-check-grund-validates-every-reference-in-a-repo), [§FS-show](FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id), [§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id), [§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id), [§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file), [§FS-id](FS-id.md#fs-id-grund-proposes-ids-for-new-declarations), and [§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up). The examples here are normative for fields, stream split, and ordering.

## 1. Diagnostic object

Diagnostics use this shape:

```json
{"severity":"error","path":"docs/functional-spec/FS-001-alpha.md","line":3,"code":"unknown-reference","message":"unknown reference FS-999-missing","sites":null}
```

Fields:

- `severity` is `error` or `warning`.
- `path` is a relative path string, or `null` when there is no single source location.
- `line` is 1-indexed, or `null` when `path` is `null`.
- `code` is a stable kebab-case diagnostic code.
- `message` is the same lowercase, no-terminal-period text used in text mode.
- `sites` is `null` for single-site diagnostics, or a sorted array of `{ "path": <path>, "line": <line> }` for multi-site diagnostics.

`check --format=json` emits diagnostic objects as NDJSON on stdout for graph findings. Run-level warnings such as empty scans, and line-less mid-walk read failures, emit the same diagnostic object shape on stderr. Launch-time CLI failures stay raw `error:` text on stderr even when `--format=json` was requested.

## 2. Empty JSON check

Command:

```text
grund check <empty-repo> --format json
```

If the scan reads no scannable files, exit `0`, stdout empty, stderr contains one JSON warning object:

```json
{"severity":"warning","path":null,"line":null,"code":"empty-scan","message":"nothing to scan — no files under `<path>` matched grund's extensions (md, rs, go, java, kt, ts, tsx, js, py, c, cpp, swift, scala, rb, php, cs, lisp, scm, clj, sql, hs, lhs, lua, ada, adb, ads).","sites":null}
```

A clean non-empty JSON check emits nothing on stdout and nothing on stderr. There is no success object.

## 3. Text report ordering

Text diagnostics are globally sorted by `(path, line)` lexicographically across errors and warnings. Errors do not sort before warnings; source location does. Example stdout for a failing `check`:

```text
docs/functional-spec/FS-001-alpha.md:3: unknown reference FS-999-missing
docs/functional-spec/FS-002-beta.md:1: declared but never cited: FS-002-beta
```

stderr is empty for ordinary graph findings.

## 4. `show --format=json`

A successful `show --format=json` emits exactly one JSON object on stdout and nothing on stderr:

```json
{"id":"FS-001-alpha","section":"1","body":"## 1. First\n\nFirst body.\n","path":"docs/functional-spec/FS-001-alpha.md","line":5}
```

Fields:

- `id` is the resolved declaration ID.
- `section` is the requested section path as a string, or `null` for a whole declaration.
- `body` is exactly the text-mode body, including trailing newline when text mode would print one.
- `path` and `line` point at the declaration or selected section start.
- `sections`, present for `show --toc --format=json`, is the ordered section-map slice as objects with `path`, `title`, and `depth`.

`show --toc --format=json` example:

```json
{"id":"FS-001-alpha","section":null,"body":"Alpha overview.\n\n## 1. First\n### 1.1 Child\n","path":"docs/functional-spec/FS-001-alpha.md","line":1,"sections":[{"path":"1","title":"First","depth":1},{"path":"1.1","title":"Child","depth":2}]}
```

`show --brief --format=json` keeps the normal `show` object shape and narrows only `body`:

```json
{"id":"FS-001-alpha","section":null,"body":"# FS-001-alpha: Alpha\n\nAlpha overview.\n","path":"docs/functional-spec/FS-001-alpha.md","line":1}
```

For an E2E case, `show --format=json` uses the E2E manifest shape from [§FS-show.2.4](FS-show.md#24-e2e-cases):

```json
{"id":"E2E-login","kind":"E2E","path":"e2e/cases/login","args":[],"expected_exit":0,"fixtures":["expected.exit","expected.stdout","repo/docs/functional-spec/FS-001-login.md"]}
```

Failed queries emit one diagnostic object on stderr and leave stdout empty; launch-time errors stay raw `error:` text.

## 5. `list --format=json`

`list --format=json` emits one declaration object per line, sorted by `(id, path, line)`:

```json
{"id":"AR-001-auth","kind":"AR","path":"docs/architecture/AR-001-auth.md","line":1,"title":"The auth module","stub":false,"defines":null,"refs":0,"duplicate":false}
{"id":"FS-001-login","kind":"FS","path":"docs/functional-spec/FS-001-login.md","line":1,"title":"User can log in","stub":false,"defines":null,"refs":2,"duplicate":false}
```

Fields:

- `id`, `kind`, `path`, `line`, and `title` identify the declaration.
- `stub` is true when the declaration is a docs stub pointing at an inline declaration.
- `defines` is the target path for a stub, otherwise `null`.
- `refs` is the number of citations that resolve to this ID.
- `duplicate` is true when this ID has more than one independent declaration home.

`list --summary --format=json` emits one kind summary object per line, in configured kind order after `--kind` / `--unused` filtering:

```json
{"kind":"FS","title":"What: behavior, requirements, and constraints","home":"requirements.md","count":2}
{"kind":"AR","title":"How: high-level implementation, structure, and design","home":"docs/architecture","count":1}
```

### 5.1 `refs --format=json`

`refs --format=json` emits one citation object per line. With `--summary`, it emits one file summary object per line instead:

```json
{"path":"docs/functional-spec/FS-002-beta.md","count":3,"lines":[3,5]}
```

`count` is the number of citation sites in the file; `lines` is the sorted, de-duplicated set of 1-indexed source lines containing those sites.

## 6. CLI and config failures

CLI-level failures use raw text on stderr, not JSON, because the command did not reach its data-producing phase. Examples:

```text
error: invalid ID `FS-bar`
hint: this repo's [id] format is `{kind}-{number}-{slug}` (run `grund config show`); `grund list` shows the IDs that exist
```

```text
error: .agents/grund.toml:2: unknown config key `strcit`
```

The invalid-ID example exits `2` for list-like query commands such as `refs`; the config validation example exits `1` for `grund config validate` and `2` when the same invalid config blocks another subcommand.

## 7. Stream matrix

| Case | stdout | stderr | Exit |
|------|--------|--------|------|
| clean `check` text | `success\n` | empty | `0` |
| clean `check --format=json` | empty | empty | `0` |
| empty scan JSON | empty | one warning diagnostic object | `0` |
| graph findings text | located finding lines | empty unless run-level diagnostics exist | `1` |
| graph findings JSON | diagnostic NDJSON | line-less run diagnostics only | `1` |
| successful `show --format=json` | one result object | empty | `0` |
| failed `show --format=json` query | empty | one diagnostic object | `1` |
| bad flag / malformed CLI | empty | raw `error:` text | `2` |
| invalid config during `config validate` | empty | raw `error: <path>:<line>:` text | `1` |
| invalid config blocking another command | empty | raw `error: <path>:<line>:` text | `2` |
