# FS-workspace: grund validates cross-project citations in a workspace

`grund` can treat a repository as a workspace of independent project namespaces.
Local citations stay unchanged, while cross-project citations name a stable alias
before the local ID. This keeps the zero-config single-project path intact
([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)) and gives larger repos an explicit resolver for sub-projects
without forcing project names into every ID ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)). The alias
syntax is chosen in [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos).

## 1. Citation syntax

A normal citation still resolves inside the current project:

```text
§FS-login
§FS-login.2.1
```

A cross-project citation writes the target project alias, a slash, then the
target ID:

```text
§api/FS-login
§root/GOAL-compatibility
§payments/FS-refunds.3.2
```

The grammar is:

```text
§[alias/]ID[.section]
```

`alias` is a lowercase slug: it starts with a letter and then uses lowercase
letters, digits, or `-`. The slash is part of the citation namespace, not part of
the ID. For an unqualified citation, the `ID[.section]` part uses the current
project's ID and section grammar. For a qualified citation, it uses the target
project's grammar: `§api/FS-001-session` is parsed with `api`'s `[id]` config,
even when the citing/root project uses a different ID format.

## 2. Workspace configuration

A workspace is declared in the root project's `.agents/grund.toml`:

```toml
project_name = "root"

[workspace]
members = ["apps/api", "packages/*"]
include_root = true
```

`members` is a list of paths or single-segment trailing globs, resolved relative
to the config root. Member paths must be relative, must not use `.` or `..`, must
not use platform-specific absolute forms or backslash separators, and must not
overlap after glob expansion; one member root cannot contain another member
root. Invalid member entries, missing member paths, and overlapping expanded
roots are config errors reported at the `members` line per
[§FS-config.4.3](FS-config.md#43-invalid-config-behavior). `packages/*` means
every direct child directory under `packages/`; recursive `**` globs are not
part of v1. `include_root` defaults to `true`; when false, `grund check` at the
workspace root checks only member projects.

Each member is a separate project namespace. If a member has its own
`.agents/grund.toml`, that file configures the member. If it does not, the
canonical defaults apply with the member directory as the config root.

## 3. Aliases

The root alias is the root config's `project_name`, or `root` when omitted.
A member alias is the member config's `project_name`, or the member directory's
basename when omitted.

Aliases must be unique across the workspace and must match the lowercase slug
grammar in §1. A duplicate or invalid alias is a launch-time error, because a
qualified citation would otherwise have two possible targets.

A project's optional one-line `project_description` ([§FS-config.3](FS-config.md#3-schema)) follows the
same residency rule as the alias: a member's description comes from the
member's own config, the root row's from the root config, and a member without
its own `.agents/grund.toml` has none ([§DF-workspace-member-descriptions](../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists)).
Unlike the alias it is presentation metadata only — generated workspace member
lists render it beside the alias ([§FS-init.2.3.4.15](FS-init.md#23415-workspace-members)), and it never
participates in alias derivation, citation resolution, or `check` semantics.

## 4. Resolution

During `grund check`:

- `<§>ID` resolves only against declarations in the current project.
- `<§>alias/ID` resolves only against declarations in the named workspace project.
- `<§>alias/ID.section` additionally requires that the target declaration contain
  the cited section.
- an unknown alias is an error at the citation site;
- a known alias with no matching declaration is an error at the citation site.
- diagnostics for a known alias render the `<ID>` and section separator with the
  target project's `[id]` config, not the citing project's config. The literal
  source token remains the scanner's evidence, but the diagnostic names the same
  target that resolution attempted.

Cross-project references are deliberately never resolved by path syntax such as
`../FS-login` or `packages/api/FS-login`; aliases are the stable handles.

## 5. Command scope

`grund check` run at a workspace root checks the root project and all configured
members, aggregates diagnostics, and prints the same `path:line: message` shape
as a normal check ([§FS-check.2.1](FS-check.md#21-report-format)). Paths are rendered relative to the workspace
root when `[output] relative_paths = true`.

`grund check <member>` (or `grund check` invoked from inside a member tree)
discovers the member's own config first and validates it as an independent
project. Qualified citations such as `§root/<ID>` or `§sibling/<ID>` cannot be
resolved without the workspace context; each such citation produces an
`unknown project alias <name>` error at the citation site
([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)). This matches [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6 — silent skipping
would let a passing member check ship a cross-project reference that no longer
resolves at the workspace root, which violates [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration). Run
`grund check` at the workspace root to validate cross-project citations.

The member-local scanner recognizes the `<§>alias/` prefix before applying the
member's local ID grammar. That way `§root/FS-root` is still an unknown-alias
citation inside a default `{kind}-{number}-{slug}` member, rather than plain
text that disappears from `check`.

Member-local recognition uses a fixed fallback ID shape — `KIND[-NUM]-SLUG`
with an uppercase-or-digit kind and a non-empty slug — because the workspace
catalogue (and therefore each target's `[id] format`) is unreachable at member
scope. A qualified citation whose tail does not match that shape (lowercase
kinds, slug-only ID grammars that don't split on `-`/`_`, kinds with
non-`[A-Z0-9]` characters) is not flagged at member scope; the workspace-root
run, which parses each qualified tail with the target project's grammar, is
the one place that catches every shape. Run `grund check` at the workspace
root for full coverage.

A relaxed standalone mode (downgrade `unknown project alias` from error to
warning on a member-only run) is deferred follow-up; see
[§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6.

## 6. Nested project boundary

Workspace members are namespace boundaries. When the root project is checked as
part of a workspace, the root scan does not scan member project directories or
their descendants, even if the root project's `[scan] include` names a path
inside a member. The member is scanned separately under its own config and alias.

This prevents a child project declaration from accidentally becoming a duplicate
or dependency of the root namespace.

A member config that itself carries a `[workspace]` block is a load-time
error in v1 — nested workspaces are not supported. The resolver semantics
they would require (alias priority across nesting levels, which
`current_alias` a checker run uses, parent-of-parent reachability) are not
pinned, and the safer default is to refuse the configuration loudly rather
than silently flatten it ([AR-workspace.6.1](../architecture/AR-workspace.md#61-nested-workspaces-are-rejected)).

## 7. Neighboring repos

Neighboring repositories in the same organization should use the same external
syntax, for example `<§>payments/FS-refunds`, but external repository resolution
is not part of this first implementation. It requires an explicit cache or
lockfile so ordinary `grund check` remains offline, deterministic, and fast
([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)). Until that cache layer exists, aliases are
workspace-local.

## 8. Other commands

The workspace surface composes through the same resolver `grund check` uses
([AR-workspace.4](../architecture/AR-workspace.md#4-the-resolver-one-function)), so qualified-ID behavior in query commands is a UX layer over
an already-built engine — not new resolution logic. Three shared rules apply to
every command in this section:

- **Discovery follows the same walk-up rule as `grund check`** ([§FS-config.1](FS-config.md#1-file-location-and-discovery),
  §5): from the CWD (or from an explicit `<path>` argument), walk up to the
  nearest `.agents/grund.toml`. If the nearest config is a member's own config,
  the command runs member-local — qualified `<alias>/<ID>` cannot resolve, the
  same way `check` errors at the member scope (§5). If the nearest config is
  the workspace root, the command runs workspace-wide. An explicit `<path>`
  argument (e.g. `grund list apps/api`, `grund refs FS-x apps/api`,
  `grund complete ids --path apps/api`) behaves as if the command were invoked
  from that path: a `<path>` inside a member is member-scoped, not
  workspace-aggregate, even when a workspace exists above it.
- **The "current project" is what an unqualified ID resolves against** (§4):
  the root project at the workspace root, the member project inside a member
  tree. When `include_root = false` and the command is invoked at the workspace
  root, there is no current project; commands that accept a single ID reject an
  unqualified `<ID>` and require `<alias>/<ID>` instead. Cross-project lookups
  always require the `<alias>/<ID>` form. There is no `--all-projects` flag; the
  alias *is* the scope handle.
- **`include_root = false`** (§2): when the root project is excluded from the
  workspace, it has no catalog entry and its alias is not known. `<§>root/<ID>`
  (or whatever name `project_name` would have assigned) is treated as any other
  unknown alias by every command in this section — the root alias is not
  silently reserved. Output paths still render from the workspace root, not from
  the first member. Completions, `show`, `refs`, and `list --project` all agree
  on this.

### 8.1 `grund <alias>/<ID>`

`grund <alias>/<ID>` reads a declaration in another workspace project. The
body is rendered exactly as `grund <ID>` renders it for a local declaration
([§FS-show.2](FS-show.md#2-behavior)) — same slice rules (`--brief`, default, `--toc`, `--full`), same
`text` vs `md` heading behavior, same section selection, same inline-code
extraction ([§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments)). The alias prefix is a routing instruction; it
changes which tree is scanned, not what is printed.

- `grund api/FS-login` — print the lead of `api`'s `FS-login`.
- `grund api/FS-login.3.1 --toc` — print the `3.1` section's lead plus its
  nested heading map, against `api`'s declaration.
- `grund FS-login` invoked from inside `apps/api/` — print the member's
  local `FS-login`. Unchanged from today; no workspace context is needed
  because the citation is local.

Outside a workspace context — including a `<path>` argument that resolves
member-local — `grund api/FS-login` exits `2` with two stderr lines:

```text
error: unknown project alias `api`
note: workspace aliases are defined in the root .agents/grund.toml under [workspace]
```

Ambiguity within a project is unchanged ([§FS-show.2.2.1](FS-show.md#221-ambiguous-id)). An ID that exists in
two *different* projects is not ambiguous — they are two declarations in two
namespaces, and the alias picks one.

### 8.2 `grund refs`

`grund refs <alias>/<ID>` lists every citation of that qualified declaration,
across the **whole workspace**, including:

- citations written `<§><alias>/<ID>` anywhere in the workspace (the
  cross-project form), and
- citations written `<§><ID>` **inside the named project's tree** (the
  project's own local form for the same declaration).

The two forms cite the same target (§4), so they appear in the same `refs`
result. This is what makes `refs` a blast-radius answer: an author about to
delete `api`'s `FS-login` learns about both `api`'s own files and every other
project that wrote `<§>api/FS-login`.

- `grund refs api/FS-login` — listed exactly as [§FS-refs.3.1](FS-refs.md#31---format-text-default) specifies; the
  `text` column shows each citation verbatim — `<§>api/FS-login` from sibling
  projects, `<§>FS-login` from inside `api`.
- `grund refs FS-login` invoked at the workspace root — citations of the
  *root's* `FS-login` only (root is the current project). To get cross-project
  occurrences, qualify: `grund refs root/FS-login` is the same query in this
  context and is the canonical form for scripts.
- `--summary` ([§FS-refs.3.3](FS-refs.md#33---summary)) aggregates per file regardless of which member the
  file lives in. Paths render relative to the workspace root when
  `[output] relative_paths = true`.
- `--format json` adds one new field on the per-citation object:
  `"project": "<alias>"` — the alias of the project that *contains* the
  citation site, not the target. The target project is the query argument
  itself: `<alias>` for a qualified lookup, the current project for an
  unqualified lookup. It is not repeated per row; per-row redundancy would
  balloon the wire size of a wide blast-radius scan without adding information
  the caller did not just hand to `refs`. The object's `"id"` field is rendered
  with that target project's `[id]` config; the `"text"` field remains the
  verbatim source citation.

The "neither declared nor cited" stderr note ([§FS-refs.2](FS-refs.md#2-behaviour)) becomes alias-aware:

```text
note: api/FS-login is neither declared nor cited — run `grund list --project api` to see api's declared IDs
```

### 8.3 `grund list`

`grund list` invoked at a workspace root prints the catalog of every project
the workspace covers (root plus members, subject to `include_root` per §8
intro), so the resulting catalog has one row per declaration with no collisions
even when two projects declare the same local ID.

Output changes:

- The ID column renders as `<alias>/<ID>` for every row **whenever workspace
  mode is loaded**, including the current project's and including the narrowed
  catalog under `--project`. Always-qualifying keeps the column self-labeled —
  `FS-login` next to `api/FS-login` in the same dump would read as a third
  project named `(local)` — and keeps script output dependent on what the
  command returned, not on how the user invoked it. The only way to get
  unqualified rows is to invoke `list` member-locally (no workspace context —
  including via a member-local `<path>` argument).
- A new optional filter, `--project <alias>[,<alias>...]`, narrows the catalog
  to one or more named projects. `--project api` returns only `api`'s
  declarations, still rendered with the `api/` prefix per the rule above. An
  unknown alias is a CLI-level error, exit `2`, same shape as `--kind`
  ([§FS-list.4](FS-list.md#4-exit-codes)).
- `--kind` composes with `--project` (intersection):
  `--kind FS --project api,payments` lists FS declarations in those two
  projects.

`grund list --summary` ([§FS-list.3.3](FS-list.md#33---summary)) gains a new variant when a workspace is
loaded: rows are emitted per `(project, kind)` pair, sorted by alias then by
configured kind order. `--project <alias>` narrows the summary to that
project's kinds.

`--format json` adds `"project": "<alias>"` to every object and renders `id` in
the qualified form (`api/FS-login`). The `refs` count is the count under that
project's qualified target, computed exactly as §8.2 defines.

### 8.4 Shell completions

The dynamic helper `grund complete ids` ([§FS-completions.2](FS-completions.md#2-internal-dynamic-helper)) gains workspace
awareness.

Completion grammar (the prefix the user has typed so far):

- No `/` in the prefix → bare-ID candidates from the current project (existing
  behavior, unchanged). When the helper is invoked at the workspace root,
  **also** emit one candidate per known alias, with a trailing `/` — `api/`,
  `payments/`, and `root/` only when `include_root = true` (§8 intro). The
  trailing slash is the continuation signal — see "Shell script adjustments"
  below.
- Prefix contains a `/` → split on the first `/`. The left side is the alias;
  emit IDs from that workspace project whose qualified form matches the prefix.
- `--sections` and the implicit section mode (prefix containing the configured
  `[id] section_separator`) compose with the alias prefix in the natural way:
  `api/FS-login.` triggers section candidates against `api`'s declaration of
  `FS-login`.

Helper errors stay quiet ([§FS-completions.2](FS-completions.md#2-internal-dynamic-helper)): a workspace that fails to load
drops back to single-project mode so a user typing in the shell never sees a
stack trace.

**Shell script adjustments.** Alias-as-completion only feels right if a Tab
from `api` advances to `api/` *without inserting a space* — otherwise the user
types `api`-Tab-Backspace-`/` instead of `api`-Tab-Tab. The generated scripts
therefore *do* change for this case (bare-ID completion remains byte-identical
for repos with no workspace):

- `bash` — the completion function calls `compopt -o nospace` for the current
  invocation whenever any candidate in the batch ends in `/`.
- `zsh` — the completion function partitions the candidate batch:
  slash-suffixed candidates are added with `compadd -S ''` (no auto-suffix),
  bare-ID candidates with the default suffix.
- `fish` — candidates are emitted via `complete -f -k`; fish appends no space
  when the candidate's last character is `/`.

These adjustments are testable per shell and must be covered before
`<alias>/`-completion is advertised. Until each shell's no-space behavior is
verified by a fixture, the helper may fall back to emitting full
`<alias>/<ID>` candidates in one shot (one Tab completes both the alias and an
ID), at the cost of needing to type a disambiguating letter first.

### 8.5 `grund fmt --cross-refs`

The link wrapper ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)) becomes workspace-aware when invoked at the
workspace root.

- A qualified citation `<§>api/FS-login` in `docs/index.md` wraps to
  `[<§>api/FS-login](../apps/api/docs/functional-spec/FS-login.md#fs-login-...)` —
  the relative path crosses the workspace into the member's home, the anchor
  is computed from the *member's* declaration heading under the member's
  configured anchor profile ([§FS-fmt.6.7](FS-fmt.md#67-configurability)). The cross-project resolution goes
  through `target_findings_for_citation` ([AR-workspace.4](../architecture/AR-workspace.md#4-the-resolver-one-function)) so the wrapped link
  and `check`'s resolution can never disagree.
- A member-local run (invoked inside `apps/api/`, or via a `<path>` that
  resolves member-local — see §8 intro) **leaves qualified citations
  untouched**: per §5, the member run cannot resolve qualified targets and
  `--cross-refs` does not paper over a citation that `check` would error on
  ([§FS-fmt.6.4](FS-fmt.md#64-what-is-never-wrapped)). A previously-emitted wrapper around a qualified citation —
  e.g. `[<§>root/FS-x](../../docs/FS-x.md#...)` left by an earlier
  workspace-root pass — is **preserved as written**: not stripped (which would
  destroy information the member-local run cannot recompute), not re-derived
  (the resolver has no canonical URL to compare against in this context). It
  will be re-derived the next time `fmt --cross-refs` runs at the workspace
  root. The single canonical way to create or refresh cross-project wrappers
  is the workspace-root run.
- Re-derive ([§FS-fmt.6.3](FS-fmt.md#63-idempotency-and-re-derive)): a heading rename or a file move in `api` triggers a
  one-line `fmt` diff in any *other* project that wrapped a citation of the
  renamed thing, exactly as it triggers a diff in `api`'s own files. The
  workspace-root run is what makes that single pass possible.

`[fmt.cross_refs]` config ([§FS-fmt.6.7](FS-fmt.md#67-configurability)) is read from each member's own config
when wrapping a citation that targets that member — the member's
`anchor_format` wins for its own declarations. Mixed-profile workspaces (rare)
render each project's anchors under its own configured profile, consistent
with the per-member config rule ([AR-workspace.5](../architecture/AR-workspace.md#5-the-config-one-parse-one-validation-pass)).

### 8.6 Output and exit codes

All five surfaces above keep the exit codes they had:

- `show` — `0` body printed, `1` ID/section not found or ambiguous, `2` CLI/
  scan error. An unknown alias is `2` (it is a CLI-shaped error, not a "found
  something else" error) and matches the standalone-mode shape in §8.1.
- `refs` — `0` always when the scan succeeds; `2` on scan/CLI error.
- `list` — `0` always when the scan succeeds; `2` on scan/CLI error (now
  including unknown `--project`).
- Completion helper — quiet failures, exit `0`, unchanged from [§FS-completions.2](FS-completions.md#2-internal-dynamic-helper).
- `fmt --cross-refs` — unchanged from [§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk).

Paths in every command respect `[output] relative_paths` as `check` already
does (§5).
