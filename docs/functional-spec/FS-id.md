# FS-id: grund proposes IDs for new declarations

The `id` subcommand emits a single, conflict-free `<KIND>-<NNN>-<slug>` ID for a new declaration. The name is deliberate: `id` is the pure allocator, while `new` is reserved for a future command that would create a declaration stub ([§DF-keep-id-for-pure-id-allocation-and-reserve-new-for-stub](../decisions/functional/DF-keep-id-for-pure-id-allocation-and-reserve-new-for-stub.md#df-keep-id-for-pure-id-allocation-and-reserve-new-for-stub-keep-id-for-pure-id-allocation-and-reserve-new-for-stub-creation)). Authors writing a new spec, agents drafting a new doc, and IDE plugins offering a "new declaration" action all call the same primitive — so the next number for a kind, and the canonical slug for a title, are computed in exactly one place. Serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (no human picks the next number by reading a directory listing) and [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) (proposed IDs cannot collide with existing declarations).

## 1. Inputs

```
grund id <KIND> "<title>" [<path>] [--width <N>] [--explain] [--format text|json]
```

- `<KIND>` — required. One of the configured `[[kinds]]` prefixes ([§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)). An unknown kind is a CLI-level error: an `error:`-prefixed line naming the kind, then a `known kinds: …` line listing the configured prefixes, on stderr, exit `2` (§6) — the same shape `grund list --kind <unknown>` produces, so a typo is never mistaken for a clean run.
- `<title>` — required. Free-form human title for the new declaration; converted to a slug per §3.
- `<path>` — directory whose tree is scanned to determine "next free number." Defaults to the current directory. Discovery is the same as every other `grund` command (walks up to find `grund.toml`; falls back to defaults).
- `--width <N>` — minimum digit width for the number. Default `3`, matching the canonical form's `-NNN-` — see [§DF-id-number-width](../decisions/functional/DF-id-number-width.md#df-id-number-width-grund-id-zero-pads-minted-numbers-to-a-default-width-of-3) for why 3, and why it is a per-invocation flag rather than an `[id]` config key (for now). The number is left-padded with zeros to at least this width; a number that already has more digits is emitted as-is (`FS-1000`), so the default is a floor, not a cap.
- `--explain` — in `--format text`, also print a one-line `next:` hint to stderr telling the caller where to put the declaration (§2.3). No effect in `--format json`, which already carries the `folder`.
- `--format text|json` — output shape (§2). Default `text`.

Per [§FS-non-goals.10](FS-non-goals.md#10-interactive-mode), `id` is non-interactive: no prompt, no confirmation. `stdout` is always exactly the proposed ID (so `$(grund id …)` is safe); `--explain` adds a hint on `stderr` only.

## 2. Outputs

### 2.1 `--format text` (default)

A single line on stdout: the proposed ID, with no marker prefix, followed by a newline.

```
$ grund id FS "User can log in with email"        # default [id] format = {kind}-{number}-{slug}
FS-008-user-can-log-in-with-email
$ grund id FS "User can log in with email"        # a repo whose [id] format = {kind}-{slug}, like grund itself
FS-user-can-log-in-with-email
```

The shape of the emitted ID always follows the repo's configured `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) — see §4.1 for the number-less formats.

This is shaped for shell composition. A typical workflow:

```sh
ID=$(grund id FS "User can log in with email")
$EDITOR requirements.md
```

Stderr is empty on success unless `--explain` was passed (§2.3). The `path:line:` prefix from [§GOAL-friendliness-first.1](../goals.md#1-hard-requirements) does not apply — `id` synthesizes; it does not point at a source location.

### 2.3 `--explain` (text only)

With `--explain`, stdout is unchanged — still the bare ID — and stderr carries one extra line: where to put the declaration and how to start it. For a kind with a configured `file`:

```
$ grund id FS "User can log in with email" --explain
FS-008-user-can-log-in-with-email
next: add the declaration to requirements.md  (H2: `## FS-008-user-can-log-in-with-email: <one-line statement>`), then cite it as §FS-008-user-can-log-in-with-email
```

For a kind with a configured `folder`, the hint names the new declaration file under that folder and uses an H1. If the kind has neither `file` nor `folder`, the hint names the H1 and the citation but not a path. This is the human-facing complement to the script-facing default: the bare ID still composes in `$(…)`, while a person who ran `grund id` interactively gets the obvious next step instead of having to recall the layout. It does not create the file (§7).

### 2.2 `--format json`

```json
{"id":"FS-008-user-can-log-in-with-email","kind":"FS","number":8,"slug":"user-can-log-in-with-email","folder":"","file":"requirements.md"}
```

`folder` and `file` are the configured `[[kinds]]` home for the kind ([§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)) — exactly one is usually non-empty, included so editor "create new declaration" actions can place the declaration without a second lookup. Under a number-less `[id] format` the `number` field is `null` (§4.1).

## 3. Slug derivation

The title is converted to a slug deterministically. Two `grund id` calls with the same title and the same configured `slug_pattern` produce the same slug, on every platform, in every locale ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).

1. Unicode-normalize the title to NFKD, strip combining marks. (`Café log-in` → `Cafe log-in`.)
2. Lower-case. ASCII-only; non-ASCII letters that survive step 1 are passed through to step 3 unchanged and will be filtered there.
3. Replace every run of characters that does **not** match the configured `slug_pattern` character class ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) with a single `-`. The default pattern is `[a-z0-9][a-z0-9-]*`, so spaces, punctuation, and quotes all collapse to `-`.
4. Trim leading and trailing `-`.
5. Collapse runs of two or more `-` into a single `-`.
6. Truncate to 60 characters at the nearest preceding `-` boundary (so a slug never ends mid-word).

If the resulting slug is empty (every character in the title was non-slug), `id` exits with code 1 and a bare query-failure line on stderr (no `error:` prefix — that prefix is reserved for the exit-`2` CLI-level failures of §6):

```
title produces empty slug after normalization: "<original title>"
```

The author is expected to provide a title that contains at least one slug-character. `id` does not invent a fallback — a meaningless slug is worse than an error.

## 4. Next-number derivation

The scan from [§FS-check](FS-check.md#fs-check-grund-validates-every-reference-in-a-repo) runs across the tree (or the configured `[scan] include` paths from [§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)) and collects every declaration of the requested `<KIND>`. The proposed number is `max(existing numbers) + 1`, or `1` if the kind has no existing declarations.

Holes in the numbering (e.g., `FS-001`, `FS-002`, `FS-004` exists but `FS-003` does not) are **not** filled. Numbers are issued strictly above the maximum, never reused, never recycled. Reasoning: an ID that once existed and was removed may still be cited from external systems (PRs, chat, mirrored repos); reusing the number would silently change what those references point at. This is the same principle as [§FS-non-goals.4](FS-non-goals.md#4-cross-workspace-id-renaming) (no rename) applied to allocation.

If the scan fails (I/O, malformed file), `id` exits 2 with the underlying error — it does **not** fall back to a guess. Allocating a number against an incomplete view of the tree could produce a collision.

### 4.1 Number-less ID formats

When the repo's `[id] format` has no `{number}` placeholder — `{kind}-{slug}` (the form `grund` itself uses) — there is nothing to derive: the proposed ID is `format` with `{kind}` and `{slug}` substituted, e.g. `FS-<user-can-log-in-with-email>`. The `--width` flag is accepted but has no effect (it pads a number that does not exist), and the `--format json` `number` field is `null`. The collision check (§5) still runs, and it carries more weight here: with no number to disambiguate, two declarations sharing a kind and slug collide on the same ID, so a clash is far more likely than under a numbered format. Conversely, when `format` has no `{slug}` placeholder (`{kind}-{number}`), the title is still required — it is used only to render a helpful collision message and is otherwise discarded; the proposed ID is `{kind}-{number}` with the next number, and the `slug` field in JSON output is the derived slug even though it does not appear in the ID.

Either of these one-component formats also puts the repo outside the number-only citation shorthand ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)): that shape is the format with `{slug}` dropped, so it exists only where the format carries both `{number}` and `{slug}`. A `{kind}-{slug}` repo has no number standing in for the ID and a `{kind}-{number}` repo has no slug to omit, so neither can produce a [§FS-check.3.13](FS-check.md#313-number-only-shorthand-citation) finding.

## 5. Collision check

After deriving slug and number, `id` verifies the full proposed ID does not already appear as a declaration in the scanned tree. This is belt-and-suspenders against:

- A configured `slug_pattern` that admits ambiguity (e.g., a project that loosened the pattern after declarations were authored under the strict default).
- A `--width` change that re-zero-pads existing IDs into the candidate.

If a collision is detected, `id` exits 1 with a bare query-failure line on stderr (same family as the empty-slug line above — no `error:` prefix):

```
proposed ID `FS-user-login` already declared at docs/functional-spec/FS-user-login.md:1
```

Authors disambiguate by editing the title.

## 6. Exit codes

- `0` — proposed ID emitted.
- `1` — slug empty (§3) or collision detected (§5). These are query failures — `id` had a well-formed request but cannot return an ID — so they print a bare line on stderr, no `error:` prefix, the same convention ID queries use for `ID not found` ([§FS-errors.2.3](FS-errors.md#23-bare-query-failure)).
- `2` — scan / I/O error, an unknown kind, an unknown `--format`, or any other CLI-level error ([§FS-cli.4](FS-cli.md#4-errors-with-no-source-location)). These print `error: <message>` on stderr — the prefix CI scripts grep for to tell a launch-time failure from a clean run.

## 7. What `id` does **not** do

- It does not create the file. Mechanically writing a stub is the caller's job — for an author, an `$EDITOR` invocation; for an IDE plugin, the `New file` action; for an agent, a follow-up `Write`. `id` stays a pure function from `(kind, title, tree)` to `id`. (Reasoning: the same primitive serves three callers with three different file-creation behaviors; baking any one of them in shrinks the surface.)
- It does not modify `grund.toml`, the scanned tree, or any existing declaration.
- It does not allocate a number reservation. Two parallel `grund id FS …` calls against the same tree may both propose `FS-008-…`; the loser sees the collision when their declaration is committed and `grund check` runs. Reasoning: reservation is state, and `grund` is stateless ([§FS-non-goals.6](FS-non-goals.md#6-decision-database-audit-log-history-tracking)).

## 8. Why this exists

Three callers, one source of truth:

1. **Authors.** Picking the next free number means listing a directory and squinting; one typo creates a duplicate that `grund check` will catch hours later. `id` removes the typo class.
2. **Agents.** An LLM proposing a new declaration cannot reliably read a directory listing and increment the right number — and even if it can, the answer drifts with the next file added. Calling `grund id` is cheap, deterministic, and committed to the same regex grammar as the checker.
3. **The optional LSP server.** A "new declaration" code action in [§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server) would need the same number `grund id` would compute; sharing the engine through `grund-core` means there is exactly one allocator, not three subtly different ones.
