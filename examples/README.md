# Examples

Self-contained mini-repos demonstrating each ID scheme `grund` supports.
Each subfolder is shaped like an `tests/e2e/cases/<name>/` directory — `repo/`
holds the fixture, and `expected.exit`/`expected.stdout`/`expected.stderr`
record the contract — so each example doubles as a regression fixture.
The e2e test runner also runs `grund <repo>` against every example on `cargo
test`, so the snippets below cannot drift from what the tool actually does.

Examples are maintained as user-facing walkthroughs for canonical `grund`
workflows, per [FS-examples](../docs/functional-spec/FS-examples.md).
Each example README should name the use-case it teaches, show the command to
run, explain the expected output, and call out the practical trade-offs.
Runnable examples share the same golden-output runner as `tests/e2e/cases/`; the
examples tree should not grow a parallel test harness.

## ID schemes

| Folder                                                       | `[id] format`             | Example IDs                |
|--------------------------------------------------------------|---------------------------|----------------------------|
| [`scheme-numbered-slug/`](scheme-numbered-slug/)             | `{kind}-{number}-{slug}`  | `FS-001-login`             |
| [`scheme-numbered/`](scheme-numbered/)                       | `{kind}-{number}`         | `RFC-001`, `FS-002`        |
| [`scheme-slug/`](scheme-slug/)                               | `{kind}-{slug}`           | `FS-login`, `AR-event-bus` |

Each subfolder's `README.md` lists the trade-offs for that scheme. The
top-level project [README](../README.md#4-the-structure-that-gets-cited)
summarizes when to reach for each.

## Workflows

| Folder                                                       | Use-case                                                 |
|--------------------------------------------------------------|----------------------------------------------------------|
| [`workspace/`](workspace/)                                   | Cross-project citation in a monorepo ([§FS-workspace](../docs/functional-spec/FS-workspace.md)) |

## Run an example

From the repo root, with a built `grund` binary on `$PATH` (or invoked
via `cargo run --`):

```bash
grund examples/scheme-slug/repo
echo $?    # 0
```

A passing scheme prints `success` on stdout and exits 0.
