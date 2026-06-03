# FS-fmt: grund normalizes references in bulk

The `fmt` subcommand rewrites a tree to canonical form: trigger sequences become markers, and (optionally) bare citations become marker-prefixed. It is the batch counterpart to the optional LSP server's live trigger transform ([§FS-lsp.1.4](FS-lsp.md#14-live-trigger-transform)) and the always-available path: every install of `grund` ships `fmt`, while the LSP server is opt-in. Implements [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger).

## 1. Inputs

```
grund fmt [<path>] [--check] [--marker] [--cross-refs] [--write]
```

- `<path>` — directory or file. Defaults to the current directory.
- `--check` — explicit form of the default behavior: report what would change; exit non-zero if any change would be made; do not write. Provided as a flag for CI clarity (a script that says `grund fmt --check` is unambiguous about intent).
- `--marker` — also rewrite bare citations (`FS-check`) to marker-prefixed (`§FS-check`). Off by default to preserve existing repos that have not opted in.
- `--cross-refs` — in `.md` files only, also wrap each marker-prefixed citation in a clickable Markdown link to the declaration body. Per §6. This forces the cross-reference pass for one invocation; generated configs set `[fmt.cross_refs] enabled = true`, so `grund fmt --write` also runs it by default unless the repo opts out. Implements [§DF-md-link-emission](../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations).
- `--write` — write the transformed contents back to disk. Exit 0 even when changes were made (the changes were the requested operation, not a failure).

`--check` and `--write` are mutually exclusive. Without either, the default is `--check`.

## 2. Behavior

### 2.1 Trigger-to-marker

Wherever the configured trigger (default `$$`) is immediately followed by a token that matches the repo's `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) — `FS-007` under a numbered format, `FS-login` under the slug-only form `grund` itself uses — replace the trigger with the configured marker (default `§`). The trigger is only consumed when a real ID-shaped token follows it, so a bare `$$` (LaTeX display math, a shell variable) is left alone. Idempotent: running `grund fmt` twice produces no further change.

### 2.2 Bare-to-marker (with `--marker`)

When `--marker` is given, every recognized bare citation is also rewritten to its marker-prefixed form. This is how a repo migrates out of `[reference] strict = false` compatibility mode: run `grund fmt --marker --write` once, then remove the override or flip the strict flag back to `true`.

### 2.3 What is never rewritten

- Declaration headings (the line that names the ID). The marker is for *citations*, not declarations.
- Citations inside string literals on a source line (where rewriting would change runtime behavior).
- Citations inside Markdown inline code spans (where rewriting would change a literal command, path, or example).
- ID-shaped text inside Markdown link destinations (where rewriting would change the URL rather than the visible citation).
- Files outside the configured scan set.

#### 2.3.1 String-literal exclusion rule

The string-literal exclusion is deterministic, not heuristic. For every candidate transform site on a source-file line:

1. Walk the line left-to-right from column 0 up to the candidate's start column.
2. Track an open-quote state per `'`, `"`, and `` ` ``. Toggling rules: an unescaped (no immediately preceding `\`) quote of a given kind toggles its state, but only when no other kind is currently open.
3. If any quote state is open at the candidate's start column, the candidate is inside a string literal and is **not** rewritten.

Markdown files (`.md`) are not subject to this rule — they have no string literals. The rule applies only to files matched by the `extensions` list excluding `md`.

This gives two correctly-configured installs identical output on identical input ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).

## 3. Outputs

- `0` — no changes needed, **or** `--write` succeeded (regardless of whether changes were made — they were the requested operation, not a failure).
- `1` — `--check` found at least one citation that would be rewritten. Never returned by `--write`.
- `2` — I/O error.

The report goes to **stdout** — it is `fmt`'s output ([§FS-errors.1](FS-errors.md#1-streams)), the same stream `grund check`'s findings use, so `grund fmt --check | …` and `grund fmt --check > pending.txt` work the way they do for `grund check`. (CLI-level `error:` lines — a bad flag, an I/O failure — go to stderr as everywhere, [§FS-errors.2.2](FS-errors.md#22-cli-level-message).) With `--check` (or no flag, the implicit dry run), the report lists one `path:line: <kind>` line per changed line ([§FS-errors.2.1](FS-errors.md#21-located-finding) shape), where `<kind>` names the rewrite: `trigger → marker` (a typed trigger sequence rewritten to the marker, §2.1), `bare → marker` (a bare citation marked, §2.2, with `--marker`), or `markdown link` (a citation wrapped or re-derived, §6, with `--cross-refs`). With `--write`, the report names what changed on disk — on stdout, not the stderr transcript shape `grund init` uses ([§FS-errors.6](FS-errors.md#6-the-grund-init-transcript)): a `rewrote N reference(s):` summary line, then one `  <path> (<count>)` line per file touched, in lexicographic path order (an empty change set prints `rewrote 0 references` with no list). The file system carries the actual change; the summary is so a reviewer can see which files to re-inspect without diffing the whole tree.

## 4. Why this exists

Three reasons:

1. **Onboarding.** Adopting the marker scheme on an existing repo requires rewriting hundreds of citations. `grund fmt --marker --write` does it in seconds.
2. **CI safety net.** A contributor who bypasses the IDE plugin (e.g., edits via the GitHub web UI) leaves bare triggers in place. `grund fmt --check` in CI catches it.
3. **Pre-commit hook.** Run on staged files; transform locally before commit. Keeps the canonical form in version control.

## 5. Configurability

Marker, trigger, and the recognized `KIND` set are read from `grund.toml` per [§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable). The defaults are `§` and `$$` as decided in [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger).

## 6. Cross-reference emission

A free convenience layer on top of the ID system: render each citation as a clickable cross-reference to the declaration body — without giving up any of the polyglot, refactor-safe properties IDs already provide. Decided in [§DF-md-link-emission](../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations).

A "cross-reference" is whatever construct the surrounding markup uses to point at another location — a Markdown inline link `[text](url#anchor)`, an AsciiDoc `xref:`, a reStructuredText `:ref:`. **Today, `--cross-refs` emits exactly one form: the Markdown inline link, and only in `.md` files (§6.1).** The flag, and the `[fmt.cross_refs]` config block (§6.7), are named for the general concept on purpose: a later `grund` that learns a second markup family emits that family's cross-reference syntax in those files under the *same* flag, with its settings under the *same* config block — an additive change, no new flag and no `grund_config_version` bump. Language-specific cross-references are deliberately not in scope yet (getting each renderer's anchor algorithm exactly right is the same kind of fidelity work the Markdown profiles already needed — [§DF-github-anchor-fidelity](../decisions/functional/DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly)); the name just leaves the door open.

### 6.1 Scope

`--cross-refs` runs **only on files with the `.md` extension** in the configured scan set. Source files are never touched: their host languages do not render Markdown, and rewriting a comment in `src/bus.rs` to inject `[…](…)` syntax is at best noise and at worst a parse error. The polyglot citation grammar (`§GOAL-polyglot-citation`) is the universal form; cross-reference emission is the rendered view of it — Markdown today, with room for other markup families later (the introduction above).

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

### 6.5 Interaction with `--marker`

`--cross-refs` operates on marker-prefixed citations. When run together with `--marker`, the marker pass runs first (bare → marker), then the link pass wraps the now-marker-prefixed citations. When run without `--marker`, bare citations are left bare and unwrapped — wrapping only the marked ones gives a consistent, predictable output instead of two mixed forms.

### 6.6 Why generated configs enable cross-references

Generated `.agents/grund.toml` files set `[fmt.cross_refs] enabled = true`, and the built-in default is the same. This makes rendered Markdown useful by default while keeping the ID citation as the source of truth. The default favors GitHub code review and external discovery over the cleaner editor-only source view, per [§DF-md-link-default-on](../decisions/functional/DF-md-link-default-on.md#df-md-link-default-on-markdown-cross-reference-links-default-on-for-github-review-and-discovery):

1. The source text still contains the exact citation, only wrapped as `[§ID](target)`; `grund check`, `grund show`, and `grund refs` continue to resolve the citation, not the Markdown URL.
2. The pass runs automatically only for `grund fmt --write` scopes that contain Markdown files; source-only scopes such as `grund fmt src/app.rs --write` stay on the lightweight marker/trigger path unless `--cross-refs` is passed.
3. Repos that do not want generated Markdown links can set `enabled = false`; the generated config writes the key explicitly so the opt-out is visible.
4. Projects with non-GitHub renderers keep the default link behavior but choose a matching `anchor_format` (§6.7), instead of disabling links entirely.

### 6.7 Configurability

```toml
[fmt.cross_refs]
enabled       = true       # default; false opts out of generated Markdown links
anchor_format = "github"   # default; named renderer profile per §DF-md-link-anchor-strategy.2.3
```

`[fmt.cross_refs]` is the home for cross-reference settings. Today it carries two keys — `enabled` (the default-on toggle for `fmt --write`) and `anchor_format` (which renderer's anchor-slug algorithm the Markdown link form uses). `anchor_format` accepts one of the named profiles defined in [§DF-md-link-anchor-strategy.2.3](../decisions/functional/DF-md-link-anchor-strategy.md#23-renderer-profiles):

- `github` (default) — GitHub's slugger; covers the most common host.
- `gitlab` — GitLab's slugger.
- `mkdocs` — MkDocs / Python-Markdown TOC extension's slugger.
- `pandoc` — Pandoc's `auto_identifiers` algorithm.
- `none` — emit no anchor; produce a file-level link with no fragment.

When `enabled = true`, the cross-reference pass runs on every `grund fmt --write` invocation whose rewrite scope contains at least one Markdown file, without requiring `--cross-refs`. Source-only scopes do not pay the full-project link-target scan because no file in that scope can be wrapped. When `enabled = false`, `grund fmt --write` skips link emission unless `--cross-refs` is passed for that invocation. When a future `grund` adds a second markup family (the introduction to §6), that family's settings live under this same `[fmt.cross_refs]` block (a new key, or a sub-table such as `[fmt.cross_refs.asciidoc]`) — additive, so a v1 config that only set `anchor_format` keeps working and `grund_config_version` is unchanged ([§FS-config.5](FS-config.md#5-schema-versioning) bump rules).

### 6.8 Measurable

E2E fixtures cover: wrap-on-first-run, the default `enabled = true` causing `grund fmt --write` to run the cross-reference pass without the flag, source-only `fmt --write` skipping the default link-target scan, `enabled = false` preserving trigger-only `fmt --write` unless `--cross-refs` is passed, no-op on second-run (idempotency), re-derive on heading rename (a wrap pointing at the old slug is rewritten to the new one in a single `fmt` pass), re-derive on file move, correct relative path across `docs/` subdirectories, a bare-ID citation linking to the declaration's own heading anchor ([§DF-declaration-anchor](../decisions/functional/DF-declaration-anchor.md#df-declaration-anchor-a-bare-id-markdown-link-points-at-the-declarations-heading-anchor)), source-file declaration link with no anchor, `anchor_format = "none"` produces file-only links, each named renderer profile (`github`, `gitlab`, `mkdocs`, `pandoc`) produces its expected slug for a curated heading set — for `github`, that set includes headings whose punctuation closes up into runs of `-` that GitHub keeps and a naive collapser would not (`## A — B` → `#a--b`; [§DF-github-anchor-fidelity](../decisions/functional/DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly)) and a section heading that itself carries a citation, with another citation pointing at that section (the anchor derives from the heading's rendered text, so it is identical before and after `--cross-refs` wraps the heading's own citation — i.e. the wrap is idempotent over a citation that lives in a section heading) — fenced-block exemption, dangling-citation skipped, declaration-line skipped, `--cross-refs` without `--marker` on a tree containing both forms.
