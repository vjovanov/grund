# FS-config: grund reads a TOML config file found by walking up

`grund` is zero-config out of the box ([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)) and fully configurable when a project's conventions diverge ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)). This spec defines the contract: where the config lives, what it contains, what it overrides, and how malformed configs are reported.

## 1. File location and discovery

The config file is named **`grund.toml`** and is discovered at **two locations per directory**: the bare `grund.toml` beside the project's own metadata files, then `.agents/grund.toml`. Discovery walks upward from the working directory, probing both names in that order at every level, and stops at the first directory where either exists — mirroring how `cargo` finds `Cargo.toml`. That directory is the **config root**; relative paths inside the config are resolved against it, never against `.agents/`. One uniform rule at every level: a repository root and a workspace member each pick the form that suits them, and a workspace may mix the two ([§FS-workspace.2](FS-workspace.md#2-workspace-configuration)). Per [§DF-config-file-location](../decisions/functional/DF-config-file-location.md#df-config-file-location-grundtoml-is-discovered-at-two-names-per-directory-and-init-writes-the-bare-one).

`.agents/` is a single-purpose directory: it holds agent-facing tooling configuration that does not belong at the repo root next to the project's own metadata files. Other agent tools may colocate their configuration here; `grund` only owns `.agents/grund.toml`. The bare `grund.toml` is the form `grund init` generates ([§FS-init.2.4](FS-init.md#24-generated-grundtoml)) and the default this spec recommends, because it is the form that makes a project's grounding **visible from its root listing**: `.agents/` is a dot-directory, hidden by `ls`, by editor file trees, and by shell globs, so under that form the question "is this a grund workspace?" has no answer a reader can see. That matters most where several grund workspaces are used together — a workspace root with members, or sibling checkouts side by side — where the cost is paid once per project and the one property a reader needs at a glance is the one the layout hides. A root `grund.toml` answers it the way `Cargo.toml` answers "is this a Rust crate". See [§DF-config-file-location.2.4](../decisions/functional/DF-config-file-location.md#24-a-projects-grounding-must-be-visible-from-its-root-listing).

### 1.1 When one directory carries both

The bare `grund.toml` wins — the form `grund init` generates is the form that governs ([§FS-init.2.4](FS-init.md#24-generated-grundtoml)), so a project never has to hold one rule for the file grund writes and a contradicting one for the file grund reads. It is also what a user reaching for a root `grund.toml` means: a repository acquires the pair only when someone deliberately puts a bare file beside an existing `.agents/` one, and the reason to do that is to move to the recommended form. The `.agents/grund.toml` is then read by nothing at all.

Because a config `grund` ignores is still a config a user edits, `grund check` reports the pair as a warning naming both files ([§FS-check.4.3](FS-check.md#43-redundant-config-pair)). It is a warning and not an error because the pair is the ordinary transient state of a move in either direction, and warnings never affect the exit code ([§FS-check.2](FS-check.md#2-outputs)) — a repository mid-migration stays green while the diagnostic stays visible. The warning is what makes this order safe to state: the losing file is never silently ignored, so a config quietly replaced is reported at the first `check`. See [§DF-config-file-location.2.2](../decisions/functional/DF-config-file-location.md#22-the-bare-grundtoml-wins-a-tie-and-check-warns-about-the-pair). That warning is the *only* one the pair earns: the run read the bare `grund.toml`, which is the location §1.2 deprecates the other one in favour of, so there is nothing about the config in force left to deprecate.

If neither name is found anywhere up the walk, `grund` runs with the built-in defaults defined in this spec. The defaults are the canonical `grund` grammar — they are not stored in any file.

### 1.2 The `.agents/` location is deprecated

Both names keep working (§1), and the bare `grund.toml` is the one a project should carry. The reason is [§DF-config-file-location.2.4](../decisions/functional/DF-config-file-location.md#24-a-projects-grounding-must-be-visible-from-its-root-listing)'s: a dot-directory is hidden from `ls`, from editor file trees and from shell globs, so under the `.agents/` form the question *"is this a grund project?"* has no answer a reader can see. A recommendation only this specification states is one a repository never hears, so a run whose config resolved to `.agents/grund.toml` says so — once, naming the file it read and the bare `grund.toml` it should move to ([§FS-check.4.11](FS-check.md#411-config-read-from-the-deprecated-agents-location)). Nothing else about that run changes: the file is read exactly as before, every key means what it meant, and the exit code is untouched. Moving is a `git mv` with no other edit ([§DF-config-file-location.2.3](../decisions/functional/DF-config-file-location.md#23-grund-init-writes-the-bare-grundtoml)), and the config root does not move with it — relative paths already resolve against the directory, never against `.agents/` (§1).

**Deprecated here means the tool asks you to move, not that it is going to stop reading.** There is **no release in which `.agents/grund.toml` stops being a config location**, and the message therefore names none. That is a deliberate departure from the default deprecation path ([§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)), which ships the new form beside the old with a warning naming the release the old one dies in, and it is stated here rather than left for a reader to notice the omission. The departure follows from what that path is *for*: a named release buys a repository the time to move before something breaks, and it is owed only where something will break. `.agents/` was `grund`'s sole config location for its whole life before dual discovery, so every repository grounded under the old rule is on it — and every one of those is a **correct** configuration rather than a broken one, because §1 reads the two names as equals and [§DF-config-file-location.2.1](../decisions/functional/DF-config-file-location.md#21-symmetric-dual-discovery)'s one rule at every level depends on a project being free to pick the form that suits it. Naming a release would promise to break configurations nothing is wrong with, to buy a uniformity this spec does not ask for. What the warning is actually for is narrower and needs no deadline: it stops a *new* project landing on the old path by copying an old one, at the moment the tools around `grund` are moving their own agent-facing files ([§DF-config-file-location.2.5](../decisions/functional/DF-config-file-location.md#25-the-agents-form-is-deprecated-and-never-removed)). A nudge that never expires is still a nudge; a deadline it cannot keep would be a lie.

**A directory carrying both names is §1.1's case and not this one.** The bare file won, so the config in force is already on the home path, and the `.agents/` file beside it is read by nothing at all — which is the redundant pair's warning to report ([§FS-check.4.3](FS-check.md#43-redundant-config-pair)) and not this one's. The two never fire on one directory: a run says either *the file you edited is ignored* or *the file you read should move*, and a repository mid-migration only ever needs one of them.

## 2. Precedence

CLI flags > `grund.toml` > built-in defaults. Layering is shallow: a value present in `grund.toml` overrides the entire corresponding default; CLI flags override individual leaf values.

Compatibility note: a pre-existing `grund.toml` that omits `[[kinds]]` keeps the pre-`requirements.md` implicit FS home (`folder = "docs/functional-spec"`) until the project writes an explicit `[[kinds]]` table. New zero-config projects and freshly generated configs use the canonical defaults in §3.4, where `FS` is `file = "requirements.md"`. This preserves existing configs without adding a new schema version.

## 3. Schema

The config file is TOML. Every key is optional; omitted keys take the default value. Unknown keys are an **error**, not a warning, per [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — typos in config files are bugs and grund surfaces them loudly.

The recognized surface is the line-oriented subset that the schema below uses: one `key = value` per line, basic (double-quoted) strings, booleans, integers, and single-line `["…", "…"]` arrays of basic strings; `#` comments; `[table]` and `[[array.of.tables]]` headers. Multi-line arrays, inline `{ … }` tables, and other TOML constructs are not parsed — keep each value on one line. A line that does not fit this shape is reported as an error pointing at the offending line, per §4.3.

Top-level keys:

```toml
grund_config_version = 1
project_name = "Example" # optional metadata written by `grund init`
project_description = "One line describing what this project is for" # optional
```

`project_name` is free-form metadata. When the project participates in a workspace (its own config sets `[workspace]`, or its directory is listed as a member by a parent), `project_name` is also the project's workspace alias — but only when it matches the alias grammar in [§FS-workspace.1](FS-workspace.md#1-citation-syntax). A `project_name` that is not a valid alias is not a load-time error; it errors loudly at workspace expansion with `invalid workspace project alias <name>`. Outside any workspace context `project_name` is purely metadata: no checker, scanner, formatter, or query behavior depends on it.

`project_description` is a free-form one-line description of the project, chosen in [§DF-workspace-member-descriptions](../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists). It is presentation metadata only: generated workspace member lists render it next to the project's alias ([§FS-init.2.3.4.15](FS-init.md#23415-workspace-members), [§FS-workspace.3](FS-workspace.md#3-aliases)), and no checker, scanner, formatter, or query behavior depends on it. A value containing a line break (a `\n` or `\r` escape in the TOML string) is a config error at the `project_description` line, reported per §4.3 — the key exists to feed single-line list bullets, so a multi-line value is a bug surfaced loudly.

### 3.1 `[reference]` — citation form

```toml
[reference]
marker            = "§"      # default; rare character that prefixes a citation in prose
trigger           = "$$"     # default; typed sequence rewritten to marker by IDE plugin and `grund fmt`
strict            = true     # default; if false, bare citations are also recognized
require_grounding = false    # default; if true, `check` flags source files that cite no declared ID
#                            # …and the default for every [[kinds]] row (§3.4.8)
# grounding_level = 1        # default; 1 = the file — the unit inside each governed file (§3.4.8)
# conversation    = "link"   # optional; committed conversation-rendering opinion — see below

# Inline citation style — see [§FS-inline-citation-style](FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations)
inline_style                 = "citation-with-note"   # default; alt: "citation-only"
inline_note_suggested_lines  = 1                       # soft cap; advisory unless warn_on_suggested = true
inline_note_max_lines        = 3                       # hard cap (error)
inline_note_max_columns      = 100                     # hard cap (error)
inline_note_layout           = "any"                   # default; alt: "citation-first-colon"
inline_note_layout_check     = "off"                   # off | warn | error — how `check` reports a layout deviation
warn_on_suggested            = false                   # if true, soft-cap overruns surface as `check` warnings
```

Per [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger). `strict = true` requires a non-empty `marker`; `strict = false` is the compatibility mode for repositories that still rely on bare citations.

`conversation` selects how agents render citations in **local conversations** — the answers, reviews, and transcripts an agent writes, not the citations on disk ([§DF-repo-conversation-opinion](../decisions/functional/DF-repo-conversation-opinion.md#df-repo-conversation-opinion-repositories-may-commit-a-link-only-conversation-rendering-opinion)). It is absent by default (no opinion), and it does not affect scanning, checking, or formatting — it only selects entrypoint guidance.

The key has **one name and two scopes**, and the scope decides both who is instructed and which values are legal:

| Where | File | Accepted | Instructs |
| --- | --- | --- | --- |
| Repository *opinion* | the project's `grund.toml` (§1) | `link` only | every agent that clones the repo, through the generated entrypoint ([§FS-init.2.3.6](FS-init.md#236-clickable-citations)) |
| User *preference* | `$XDG_CONFIG_HOME/grund/config.toml` | `plain` \| `link` | every agent on this machine, through its global instruction file ([§FS-integrations.4.3](FS-integrations.md#43-user-preference-and-global-agent-instructions)) |

A second key, **`conversation_target`**, selects how a linked citation addresses its declaration. It is
**user-scope only** — there is no repository spelling, and setting it in the project's `grund.toml` is the
same unknown-key error as any other (§4.3). Its accepted values are `file` (default), `path`, `web`,
`vscode`, `vscodium`, and `cursor`; the templates each one fills, and the per-agent gate that decides
where the linked form is instructed at all, are specified in [§FS-integrations.4.3](FS-integrations.md#43-user-preference-and-global-agent-instructions) and decided in
[§DF-conversation-link-target](../decisions/functional/DF-conversation-link-target.md#df-conversation-link-target-the-conversation-link-form-is-a-markdown-link-over-an-absolute-uri-addressed-per-machine). The key is inert unless the effective `conversation` is
`link`; it is still parsed and reported either way, like the `inline_note_*` keys below. One machine
may read several agents that do not render alike, so the same key is also accepted per agent under
`[reference.agents.<agent>]`, a partial merged over the machine-wide value ([§FS-integrations.4.4](FS-integrations.md#44-per-agent-overrides)).

The same spelling in both files is deliberate: one setting the user already knows by name, read at two scopes, rather than a second vocabulary for the same idea. Only the *values* narrow, and only in the direction a repository can actually justify.

**Set the repository key to `link`** when the citations your agents write should carry their declaration location for readers whose machines grund never touched — teammates on a fresh clone, cloud agent sessions, CI reviewers, and Cursor or Windsurf users, who have no user-level file grund can write. It costs installed users nothing: their recorded `plain` still wins (§3.1, [§DF-repo-conversation-opinion.2.3](../decisions/functional/DF-repo-conversation-opinion.md#23-precedence)). **Leave it absent** when local-conversation rendering is each contributor's own business — then the user preference governs alone, and a machine that never stated one gets today's bare citations.

**`plain` is deliberately not a repository value.** It presumes an installed rendering layer, which is machine state a repository cannot know; committing it would break exactly the clones the key exists to serve ([§DF-repo-conversation-opinion.2.2](../decisions/functional/DF-repo-conversation-opinion.md#22-only-link-is-committable)). The repository value set is therefore a closed enum with the single member `link`, widenable later without a `grund_config_version` bump (§5); any other value — including `plain` — is a load-time error (§4.3) raised by every command that renders or checks the entrypoint, `grund init` included. `link` makes the declaration's location travel with the citation; the committed form is a Markdown link over the machine-independent `file` target, which the reader's own `conversation_target` may override ([§FS-init.2.3.4.17](FS-init.md#23417-clickable-citations), [§DF-conversation-link-target.2.3](../decisions/functional/DF-conversation-link-target.md#23-the-target-is-user-scoped-but-the-default-is-committable)).

`require_grounding = true` adds the ungrounded-source-file error ([§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in)): every scanned non-Markdown file must carry at least one resolving citation, or declare an ID inline. `grund check --require-grounding` forces it on for one run. Per [§DF-require-grounding](../decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec); off by default so adopting the discipline is a deliberate step, like `strict`.

`require_grounding` and `grounding_level` are the two keys of this section that are **defaults for the `[[kinds]]` table** rather than settings of their own: each may be written on a row, and the row wins (§3.4.8). Written here they say what every place does; written on a row they say what one place does. `grounding_level` names the unit inside each governed file — `1`, the default, is the file, which is the unit every config had before the key existed. It is inert, and a config error, where nothing turns grounding on (§3.4.8).

`inline_style`, the three budget keys (`inline_note_suggested_lines`, `inline_note_max_lines`, `inline_note_max_columns`), and `warn_on_suggested` govern the shape of inline citations in code comments — whether a `§<ID>` token may be accompanied by a short rationale, and how long that rationale may run. The budgets and the style bound *inline* comments only; a doc comment is documentation and lies outside all of them, so a citation inside one is checked for everything except its shape ([§FS-inline-citation-style.1.1](FS-inline-citation-style.md#11-doc-comments-are-not-sites)). The full contract — modes, enforcement, agent-facing rendering — lives in [§FS-inline-citation-style](FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations). Load-time invariant: `inline_note_suggested_lines ≤ inline_note_max_lines`. Under `inline_style = "citation-only"` the three budget keys are inert (no note is ever permitted), but they are still parsed and printed by `grund config show` — the file is the canonical machine-readable form.

`inline_note_layout` adds the third axis of that shape — where the `§<ID>` tokens sit inside the note — and `inline_note_layout_check` selects whether `grund check` reports a deviation and through which channel. Both are closed enums: `any` (default, no constraint) or `citation-first-colon` for the layout, and `off` (default), `warn`, or `error` for the check; an unrecognized value is a load-time error (§4.3), and either set may be widened later without a `grund_config_version` bump (§5). The layout key is the house style and the check key is the gate, so a project can publish the style to its agents ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)) before it starts failing on it. `inline_note_layout_check` is inert under `inline_note_layout = "any"` and both are inert under `inline_style = "citation-only"` — still parsed, still printed, like the budgets above. The canonical form and the per-line rule live in [§FS-inline-citation-style.3.3](FS-inline-citation-style.md#33-inline_note_layout--where-the-citations-sit).

### 3.2 `[id]` — ID grammar

```toml
[id]
format             = "{kind}-{number}-{slug}"
section_separator  = "."
section_heading_levels = "strict"
number_pattern     = "\\d+"
slug_pattern       = "[a-z0-9][a-z0-9-]*"
```

`format` is a template: `{kind}`, `{number}`, `{slug}` are placeholders; everything else is literal. `{kind}` is required. `{number}` and `{slug}` are individually optional — but **at least one** of them must appear, because a bare kind would not identify a declaration. The literal characters between placeholders may be anything — `-`, `_`, `.`, `:`, etc.

The three canonical shapes:

| `format`                       | Example ID            | Disambiguator                |
|--------------------------------|-----------------------|------------------------------|
| `"{kind}-{number}-{slug}"`     | `FS-NNN-<slug>`       | number; slug is descriptive  |
| `"{kind}-{number}"`            | `FS-NNN`              | number                       |
| `"{kind}-{slug}"`              | `FS-<slug>`           | slug must be unique per kind |

When `{number}` is omitted, slugs must be unique within each kind — two declarations sharing a kind and slug collide on the same ID and are reported as duplicate declarations (per [§FS-check.3](FS-check.md#3-errors-detected)). When `{number}` is present, slugs are descriptive only and may repeat across declarations with different numbers.

`section_separator` must not collide lexically with any literal in `format` or with `slug_pattern`. grund validates this on load and refuses ambiguous configs. It must not be — or contain — a `/` either, which is the invariant below seen from the other side: a citation is `[<alias path>/]<ID>[<sep><section>]` and its alias-path boundary is the **last** `/`, so a `/` separator makes the two boundaries the same character. With `section_separator = "/"`, `<§>root/fs-x/1` — section 1 of `fs-x` in project `root` — reads as alias path `root/fs-x` and ID `1`: a citation that resolved before alias *paths* existed stops resolving, and a `[citations]` obligation (§3.9) resting on it turns red with no config change. Rejected at that key's line.

No ID the grammar can build may contain a `/`, and what that forbids depends on how the key reaches an ID. `format` and a `[[kinds]]` prefix (§3.4) contribute literal text — a prefix is the leading component of every ID in its kind — so neither may carry the character: a `/` in the key is a `/` in the ID. `number_pattern` and `slug_pattern` are regexes, and the rule asks what they **match**, not what they spell. A pattern with no `/` in its text may produce one freely (`[^.[:space:]]+`, `.+`, `[^[:space:]]+`) and is rejected; a pattern that names the character to *exclude* it (`[^/.]+`) can never produce one and loads. A `/` belongs to the citation namespace and never to an ID — a qualified citation splits on its **last** `/`, and every command that takes an `<alias>/<ID>` argument splits it the same way ([§FS-workspace.1](FS-workspace.md#1-citation-syntax)). So a grammar permitting `FS-a/b` would declare and resolve an ID that grund cannot accept back as a query, and would make the alias-path boundary depend on which project's grammar the reader had in mind. Rejected on load at the offending key's line, like the regex check below; the message says *must not contain* for a literal key and *must not match* for a pattern, since only one of those is a question about the key's text.

`number_pattern` and `slug_pattern` must each be a valid regex **on their own**, not merely valid once spliced into the ID pattern. Two that balance only against each other — `number_pattern = "("` with `slug_pattern = "a)"` — would compile as one ID pattern and then fall apart the moment grund derives a narrower pattern from a subset of the format's components, which is what the number-only shorthand does ([§FS-check.1.2](FS-check.md#12-the-number-only-shorthand)). Such a config is rejected on load with the underlying regex error, rather than accepted and failed later.

The chosen format is repo-wide. Mixing styles in one tree (some IDs numbered, others slug-only) is not supported — citations would look identical but resolve differently. Pick one shape per repo and keep it stable.

### 3.3 Section paths — arbitrary nesting depth

Section coordinates are **dotted paths of arbitrary depth**. There is no maximum nesting level. All of the following are valid section references when the corresponding heading exists in the declaration:

```
§FS-check.3
§FS-check.3.1
§FS-check.3.1.2
§FS-check.3.1.2.7.4
```

Section depth in the citation must match a heading at that depth in the declaration. The scanner records every numbered heading inside a declaration body and validates citations against the recorded set, so a project that wants four-deep nesting (`## 1.`, `### 1.1`, `#### 1.1.1`, `##### 1.1.1.1`) is supported with no config changes — the dotted path simply grows.

`section_heading_levels` controls how the Markdown heading depth must line up with the dotted section path. The default, `"strict"`, requires the heading level to equal the declaration heading level plus the number of dotted path components: under an H1 declaration, `## 1. …`, `### 1.1 …`, and `#### 1.1.1 …` are valid, while `## 1.1 …` is a `section heading level mismatch` error in `grund check` ([§FS-check.3.9](FS-check.md#39-section-heading-level-mismatch)). `"warn"` reports the same mismatch as a warning, so CI can stay green while a repo migrates. `"loose"` preserves the historical behavior: any heading deeper than the declaration heading is recorded as a section, and the dotted number alone determines the tree. Plain, unnumbered headings and bold labels are always allowed prose structure; they are not grund section targets. Unknown values are invalid config.

The default `section_separator` is `.`. Projects that prefer `:` (`§FS-check:3.1.2`) or `#` (`RFC-42#3.1.2`) override it; the dotted **components** stay separated by `.` regardless of the outer separator. Example with `section_separator = "#"`:

```
§FS-check#3.1.2     ← outer separator is `#`, intra-section separator is `.`
```

This split keeps the section grammar regular at any depth.

### 3.4 `[[kinds]]` — recognized kinds

One `[[kinds]]` table per kind. `kind` is its name — mandatory, and the handle everything else keys on: `[citations.<kind>]` (§3.9), `grund list --kind`, and, for a kind that declares IDs, the literal prefix of every ID in it.

A kind is either *multi-file* (`folder = "<dir>"`) — each declaration is the H1 of its own file under `<dir>` — or *single-file* (`file = "<path>"`) — every declaration of the kind is an H2 inside that one document. Setting both `folder` and `file` on the same kind is invalid; setting neither leaves the kind with no configured home (`grund id` will print no folder, and the misplaced-declaration check in [§FS-check.3.7](FS-check.md#37-misplaced-declaration-configured-kind-home) only applies when a declaration sits inside some other kind's unique configured home).

`folder` is used by `grund id` ([§FS-id.2.2](FS-id.md#22---format-json) emits it as the `folder` field) and by editor "create new declaration" / "go to home folder" actions; it is also a checker boundary: a declaration inside exactly one configured `folder` home must declare that folder's kind ([§FS-check.3.7](FS-check.md#37-misplaced-declaration-configured-kind-home)). `file` is stricter: declarations of a single-file kind found outside the configured path are reported under [§FS-check.3.7](FS-check.md#37-misplaced-declaration-configured-kind-home), and a different-kind declaration inside that exact file is likewise a unique-home conflict. Declarations are still recognized outside configured homes — including inline source declarations — and declarations in files covered by zero or multiple configured homes are not rejected by the home-kind rule because there is no single expected kind.

Every configured home is also **in the scan scope by construction**, whether or not `[scan] include` names it (§3.5).

#### 3.4.1 `citable` — kinds that declare no IDs

A kind has two independent properties: it *has a home*, and it *declares IDs*. `citable = false` (default `true`) is the second one turned off — a kind that is a **place** and nothing more.

```toml
[[kinds]]
kind = "skill"
folder = "skills"
title = "Agent review and automation skills"
citable = false

[citations.skill]
must = ["FS"]
must-not = ["AR"]
```

Some directories hold agent-facing content rather than specification — skills, prompt libraries, runbooks, test suites. An agent has to be told they exist and what they are for, and the citations inside them should be checked and directed like citations anywhere else; but their files are not declarations and carry no IDs. *Citable* is already this spec's word for "can be the target of a `§` citation" — citable sections, citable IDs — so a citable kind is one whose IDs you can cite, and this key is the one that says so about a whole kind.

What a non-citable kind **keeps**:

- **A home** — `folder` or `file` — wherever the kind is a *place*. Leaving both out is not an omission but a different thing: the entry becomes the **homeless kind**, the complement of every home, whose default name is `code` (§3.9.2). Everything below is written about a non-citable kind with a home; §3.9.2 says where the homeless one differs.
- **A row in the generated Project map and in the generated citation directions** ([§FS-init.2.3.4.4](FS-init.md#2344-project-map), [§FS-init.2.3.5](FS-init.md#235-citation-directions)) — rendered by **place**, never by name, because the name is a config handle and the place is the thing a reader can open.
- **Citation-direction rules.** The citing-side classification already reaches it: a citation inside a kind's home is classified as that kind whether or not a declaration encloses it ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)). Obligations attach per file rather than per declaration ([§FS-check.3.11](FS-check.md#311-missing-required-citation)), since there is no declaration to attach them to.
- **Grounding**, over every scanned file in its home, `.md` included ([§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in)) — asked of this home alone with `require_grounding` on the row, or of every place at once with the `[reference]` default the row inherits (§3.4.8).

What it **loses**:

- **The ID grammar.** Its name is not a recognized prefix, so `<name>-<slug>` is not an ID and never tokenizes as a citation. It is left out of the `KIND ∈ {…}` vocabulary line, out of `grund list --kind`, and out of `grund id` — both selectors refuse it by name, saying that it declares no IDs rather than that it is unknown ([§FS-list.1](FS-list.md#1-inputs), [§FS-id.1](FS-id.md#1-inputs)).
- **Declarations.** Its home admits none: any declaration inside it is a misplaced declaration ([§FS-check.3.7](FS-check.md#37-misplaced-declaration-configured-kind-home)).
- **An index.** `index` lists a folder's declarations ([§FS-check.3.18](FS-check.md#318-declaration-missing-from-its-kinds-index)) and this kind has none, so setting both keys is a config error rather than a no-op — a statement about a set that can never be non-empty.
- **Being cited.** A `[citations.<kind>]` rule may not *name it as a target*; there is no ID to point at (§3.9.5).

`citable` is an additive optional key and does not move `grund_config_version` (§5): a config that sets it is only ever written for a binary that understands it, and an older binary meets it through the unknown-key rejection in §4.3. A non-citable kind whose files are not this repository's to read — content that ships verbatim — sets `scan = false` as well (§3.4.7).

#### 3.4.2 `index` — the kind's index file

`index` names the kind's **index file** — the document under `folder` that must list every declaration in it ([§FS-check.3.18](FS-check.md#318-declaration-missing-from-its-kinds-index)). It is resolved relative to `folder`, defaults to `README.md`, and takes either a file name or `false`:

```toml
[[kinds]]
kind = "DF"
folder = "docs/decisions/functional"
# index = "README.md"   # the default, relative to `folder`
# index = false         # opt out — the folder is not navigated
# index = "INDEX.md"    # or name a different file
```

The default follows from `folder` rather than from a second key restating it: a kind whose declarations live in a directory has a directory a reader arrives at, and `grund init --docs` already scaffolds that README and writes the convention into it ([§FS-init.2.1](FS-init.md#21-files-written-updated-or-left-in-place)). `index = false` is for a kind whose declarations are *exercised* rather than navigated — the canonical case is a repository's own `E2E` kind, whose home holds case directories and no README, and whose `e2e/README.md` one level up documents the case layout in English instead of naming `E2E-` IDs. That is why the opt-out spells a file name or `false` rather than being inferred from a README's absence: "any folder README is an index" is not true of every tree.

`index` requires `folder`, and requires a citable kind. On a single-file kind (`file = "<path>"`), on a kind with no configured home, or on a `citable = false` kind (§3.4.1) there is nothing to index, and the key is a config error reported per §4.3. `index = true` is an error for the same reason a bare `true` names no file — write the name, or leave the key out for the default.

A named `index` must be **a relative path inside `folder`, naming a Markdown file**; anything else is a config error per §4.3. Both halves close a state the rules built on the key cannot describe. The value is joined onto `folder`, so an absolute path or one that climbs out with `..` does not name a file *in* the folder — it silently replaces the folder, and `grund check` would read a file outside the tree the config describes — the same boundary [§FS-fmt.2.3.2](FS-fmt.md#232-a-link-that-leaves-the-config-root-is-not-written-through) holds a rewrite to, for the reason [§REQ-no-data-loss.2](../requirements/REQ-no-data-loss.md#2-writers-touch-only-what-they-own) gives; `.` is refused with them, because it names the same file by a path no message should have to print. And an entry has to be a Markdown link that `grund fmt --write` can write ([§FS-check.3.17](FS-check.md#317-index-entry-is-not-a-link)), while the cross-reference pass runs on `.md` files only ([§FS-fmt.6.1](FS-fmt.md#61-scope)) — so an index named `INDEX.rst` would carry an error class whose one documented fix declines to act on it.

**The default is per kind name, and it is the same default for a declared kind and a built-in one.** `E2E` defaults to `index = false` and every other citable folder kind to `README.md`, whether the name comes from the built-in list or from a `[[kinds]]` block that omits the key. A `[[kinds]]` block replaces the built-in list rather than merging into it (§3.4.4), so without this the generated config would mean one thing when it spells `index = false` out and another when it does not — and every config written before this key existed, which is every config on disk, would inherit an obligation the built-in default deliberately declines. `E2E` keeps its entry in that table after leaving the default kind set (§3.4.4) for exactly the same reason: the configs that name it are the ones written before it left. A project that names its cases folder `E2E` *and* wants an index writes `index = "README.md"`, which is the ordinary way to override a default.

The key is purely additive and does not move `grund_config_version` (§5): a config that sets it is only ever written for a binary that understands it, and an older binary meets it through the unknown-key rejection in §4.3. The name-keyed default is additive in the same sense — it can only *remove* an obligation that no released `grund` has ever imposed.

#### 3.4.3 `title`

`title` is human-readable metadata: it surfaces in `grund <ID> --format=json`, `grund refs --format=json`, and IDE hover previews, and is **not** injected into `grund <ID> --format=md` text (which is the declaration verbatim — [§FS-show.3](FS-show.md#3-outputs)). It is also the text of the kind's Project map row ([§FS-init.2.3.4.4](FS-init.md#2344-project-map)), which for a non-citable kind is the only thing that says what the place is for.

#### 3.4.4 The default kinds

The defaults declare these nine, in this order:

```toml
[[kinds]]
kind   = "GRUND"
file   = "docs/grund.md"
title  = "Why: project motivation"

[[kinds]]
kind   = "GOAL"
file   = "docs/goals.md"
title  = "Where: project direction and outcomes"

[[kinds]]
kind   = "FS"
file   = "requirements.md"
title  = "What: behavior, requirements, and constraints"

[[kinds]]
kind   = "AR"
folder = "docs/architecture"
title  = "How: high-level implementation, structure, and design"

[[kinds]]
kind   = "DF"
folder = "docs/decisions/functional"
title  = "Product behavior decisions and tradeoffs"

[[kinds]]
kind   = "DA"
folder = "docs/decisions/architectural"
title  = "Architecture decisions and tradeoffs"

[[kinds]]
kind    = "e2e"
folder  = "tests/e2e"
citable = false
title   = "User scenarios: black-box proof of the spec"

[[kinds]]
kind    = "integration"
folder  = "tests/integration"
citable = false
title   = "Integration tests: proof that the parts fit as designed"

[[kinds]]
kind   = "RM"
file   = "docs/roadmap.md"
title  = "Planned milestones and sequencing"
```

`GRUND` is the H1 of the single file `docs/grund.md` (the project's reason for being — one declaration, all of it inline); `GOAL` declarations are H2 headings inside the single file `docs/goals.md` (one file, all goals inline); `FS` declarations are H2 headings inside the single file `requirements.md` (one obvious requirements entry for new projects); `RM` declarations are likewise H2 headings inside the single file `docs/roadmap.md` (one file, all milestones inline) — those four are single-file kinds (`file = "<path>"`); `AR`, `DF`, and `DA` declarations are the H1 of a file in their `folder` (an `AR` declaration may instead live inline in a source doc-comment with an optional stub in `folder` — [AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments)). A single-file kind can always be broken up later by swapping `file = "<path>"` for `folder = "<dir>"` and moving the document into that folder — the schema models the transition as exchanging one key for the other, not setting both.

**The two test kinds are non-citable, and lowercase.** A test cites the document whose claim it proves, and is never cited back: an `e2e` scenario proves the What as a user sees it (`must` cite `FS`, and `should-not` cite `AR` — a black-box scenario that reads the design is not black-box), an `integration` test proves the How — that the parts fit as designed (`should` cite `AR`). Unit tests live with the code and follow `code`'s rule, so there is no third kind for them. Lowercase because these names never appear in an ID, so a reader should not mistake one for a prefix; the `KIND ∈ {…}` vocabulary line lists the citable seven.

`E2E` is **not** in this list, and is still a fully supported kind: a repository whose e2e suite is a corpus of case directories declares it (`kind = "E2E"`, `folder = "e2e/cases"`, `index = false`) and gets the case-declaration machinery of [AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations) — `E2E-<case>` IDs, `grund <ID>` over the case manifest, per-case obligations, and the fixture-tree pruning that keeps a nested case repo out of the host scan. That machinery follows the configured `E2E` home, so a config that wants it names it. Decided in [§DF-non-citable-kinds.3](../decisions/functional/DF-non-citable-kinds.md#3-consequences), which also records what a default-config repository with an `e2e/cases` tree sees on upgrade.

A project that overrides this list replaces the defaults entirely — there is no merge. To extend rather than replace, copy the defaults and add to them.

#### 3.4.5 Name rules

**Names are unique across the whole table.** `[citations.<kind>]` and `grund list --kind` key on a name, so two rows wearing one name is a config with no answer to "which".

**Citable names must also be prefix-free**: no citable kind's name may be a prefix of another citable kind's name. `kind = "DA"` and `kind = "DAT"` together are invalid because a token starting with `DAT-` would parse as either kind. The rule is about *tokenization*, so it stops where tokenization does: a non-citable kind's name never appears in an ID, so `skill` beside a citable `SKI` is fine and a config that spells it loads. grund validates this on load and refuses ambiguous configs with a single error pointing at the offending pair (per §4.3).

**`code` is reserved to the homeless kind** (§3.9.2). It is the default name of the complement of every configured home, so a row may take it only by *being* that complement — `citable = false`, no `folder`, no `file`. Any other row wearing it would collide with the kind every citation outside a home resolves to.

#### 3.4.6 `prefix`, the former spelling of `kind` *(removed in 0.13.0)*

`prefix` was this key's name while every kind declared IDs and its name really was one. It stopped loading in grund **0.13.0**, at the end of the deprecation window [§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) asks of a renamed config key: 0.12.0 shipped `kind` beside it and warned every config that still spelled it, naming this release. A config that still spells it is **refused**, not read with the key ignored — an ignored name leaves a `[[kinds]]` row with no kind, which changes what the configuration means without saying so. The refusal is an ordinary config error (§4.3) that names `kind` as the key to write instead, anchored at the line `prefix` is written on. An entry that sets both `kind` and `prefix` earns that same error at that same line: with one of the two names gone there is nothing left to disambiguate, so the pair is no longer a rule of its own.

```text
error: grund.toml:4: [[kinds]] `prefix` was removed in grund 0.13.0 — rename it to `kind`
```

The migration is that rename, and the error names the line to make it on. A grund before 0.13.0 did it in one command — `config show` printed every entry under the canonical spelling whichever the file used, so `grund config show > grund.toml` rewrote the file — but from 0.13.0 nothing that has to load the config can help.

The rename is what `citable = false` forces. `prefix` was accurate for every row of the table and stopped being accurate for half of it; *kind* is what the rest of grund already calls this value — the `{kind}` placeholder of `[id] format` (§3.2), the `--kind <KIND>` selector of [§FS-list.1](FS-list.md#1-inputs), and the `[citations.<kind>]` table key (§3.9). Under the new name, prefix-ness is a *derived* property of citable kinds (§3.4.5) rather than the schema's word for the whole concept. Decided in [§DF-non-citable-kinds.2.4](../decisions/functional/DF-non-citable-kinds.md#24-the-field-is-a-kind-not-a-prefix).

#### 3.4.7 `scan` — a place that is listed, not walked

`scan = false` (default `true`) keeps a non-citable kind's home out of the walk. The kind is still a **place**: it gets its Project map row ([§FS-init.2.3.4.4](FS-init.md#2344-project-map)) with its title, so an agent is told the directory exists and what it is for — and nothing in it is read, so nothing in it is checked.

```toml
[[kinds]]
kind = "template"
folder = "templates"
citable = false
scan = false
title = "Init scaffold templates: what grund init writes, verbatim"
```

The case it exists for is content that ships verbatim somewhere else: scaffold templates, embedded assets, example configs. Such files cannot be grounded — a `§` citation in one lands in every tree it is copied into as a dangling reference to a declaration that tree does not have — and leaving the kind unconfigured would leave the directory out of the map. §3.5's rule that a home is a walk root `exclude` cannot prune is about a config that says both "this directory matters" and "skip its descendants"; this key is the config saying one thing: listed, not walked.

**Not walked means not walked, however the walk arrives.** The home is left out of the walk roots of §3.5, *and* pruned when a walk meets it on the way down — under the config root, or under an `include` entry it sits inside, as `docs/templates` sits inside `docs`. An `include` entry that names the home itself does not walk it either: the narrower key, the one written on the kind, is the config's answer where the two disagree. A `file` home (§3.4) is unwalked on the same terms as a `folder` one, and it is the case that shows why the rule cannot be a rule about directories: `docs/template.md` is never a directory to skip, and skipping its parent is not on offer, `docs` being an ordinary scanned home. Anything less would make `scan = false` a silent no-op for every repository that keeps such a file or directory under a scanned one, which is where a scaffold usually is.

An explicit path argument still reads it — `grund check docs/templates` scans the directory it names, the same way it reads past `[scan] include` (§3.5). The key describes the *default* scope, which is what a run with no argument reads and what [§FS-check.3.14](FS-check.md#314-out-of-scope-unresolvable-citation---full-only) tiers against; a path a user typed is that user narrowing the run to a directory they are asking about.

What an unwalked kind keeps: its home, its title, and its Project map row. What it loses, beyond what `citable = false` already takes (§3.4.1), is every rule that reaches a file — citation checking, the directions bullet and `[citations.<kind>]` rules, and the grounding clause of §3.4.1 — because no file in it is scanned. That is why `require_grounding = true` on this row is a config error rather than a no-op (§3.4.8). Under `grund check --full` ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full)) the whole config root is walked and its files are reached like any directory nobody configured: resolution failures only, never a convention it did not adopt. They are reached from *outside* the configured scope even when a walk root encloses them ([§FS-check.3.14](FS-check.md#314-out-of-scope-unresolvable-citation---full-only)), because the scope is what a run without the flag reads, and that run does not read them.

Three combinations are config errors, reported per §4.3, each closing a state the key cannot describe:

- `scan = false` on a **citable** kind. Its declarations would be invisible rather than declared — the trap §3.5 closes — so a kind that declares IDs is always walked. Set `citable = false`, or drop the key.
- `scan = false` with **no home**. The homeless kind (§3.9.2) is the complement of every home, and what of that complement is walked is `[scan] include`'s to say.
- a `[citations.<kind>]` table naming an unwalked kind as the **citing** kind. No file in the home is scanned, so the rule could never fire — the vacuous pass [§DF-non-citable-kinds.2.5](../decisions/functional/DF-non-citable-kinds.md#25-obligations-get-a-per-file-unit-and-grounding-follows-the-home) refused for a kind with no declarations, one level up.

`grund config show` (§4.2) prints `scan = false` where it is set and nothing where it is not, as it does for `citable`. The key is additive and does not move `grund_config_version` (§5). Decided in [§DF-unwalked-kind-home](../decisions/functional/DF-unwalked-kind-home.md#df-unwalked-kind-home-a-kind-may-be-a-place-that-is-listed-but-not-walked).

#### 3.4.8 `require_grounding` and `grounding_level` — grounding per place and per level

Two keys say, per kind, **whether** the files of a place must cite a declared ID and **how finely** that is asked. Each has a `[reference]` twin (§3.1) that is the default for every row not setting it — the shape `index` already has (§3.4.2): a global default, the row wins.

```toml
[reference]
require_grounding = false      # whether — the default for every row below
grounding_level   = 1          # how fine — 1 = the file; the default for every row below

[[kinds]]
kind = "skill"
folder = "skills"
citable = false
require_grounding = true       # every scanned file here must cite a declared ID
grounding_level = 2            # …and so must every `##` section of it

[[kinds]]
kind = "code"                  # the homeless row (§3.9.2)
citable = false
require_grounding = true       # …must cite one, or declare one inline
```

The keys exist because *whether* a file must cite is already reasoned about per place. Direction rules constrain how you ground and never whether ([§DISC-citation-directions](../discussions/proposals/2026-06-13-citation-directions.md#disc-citation-directions-encode-citation-directions-as-checked-config)), and for a non-citable kind grounding follows the home rather than the file extension (§3.4.1). One global boolean cannot say "every skill must cite" without also saying it of every workflow and build script in the scan, so the repository that wants the first declines the second and leaves the hole open ([§FS-check.2.2.1](FS-check.md#221-citation-direction-obligation-applies-to-nothing) can then only warn about it).

**Which files a row governs** is [§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in)'s own predicate, asked per row: every scanned file in a non-citable home — a `folder` home's files, or a `file` home's one document; scanned *source* files — extension not `.md` — in a citable folder home and in the homeless kind's complement (§3.9.2). A file that no single home claims falls to the homeless row, the way its citing side already does ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)).

**`grounding_level` picks the unit inside each governed file.** It is an integer in Markdown heading levels, `1..=6`: `1` is the file — the H1's own subtree, so one citation anywhere under it, which is exactly the unit every config had before this key existed — `2` adds every `##` subtree, `3` every `###` as well, and `6` every heading Markdown can have. Authors already think in `##`, and `[id] section_heading_levels` uses *level* for the same count, so there is no second numbering to learn. A source file has no headings, so it gets the two ranks grund can see without parsing code ([§FS-non-goals.3](FS-non-goals.md#3-code-ast-parsing)): by indentation, not by syntax. The units and the findings they produce are specified in [§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in); the same unit is what `[citations]` obligations are asked of ([§FS-check.3.11](FS-check.md#311-missing-required-citation)), so *whether* and *what* are asked of the same thing.

**Precedence is row > global**, for both keys. `grund check --require-grounding` ([§FS-check.1](FS-check.md#1-inputs)) is the run-level spelling of the global boolean and sets the same default, so an explicit `require_grounding = false` on a row still wins over the flag: the flag and the key are one knob, and the row's word is the more specific one. The level comes from config only — there is no flag for it.

**The homeless kind takes both keys like any row** (§3.9.2). A config that never declared it writes the row to set them, the same way it writes one to take a `title`.

Five combinations are config errors, reported per §4.3 at the offending line, each closing a state the keys cannot describe:

- `require_grounding = true` on a `scan = false` row (§3.4.7). No file in the home is read, so the rule could never fire — the reasoning §3.4.7 already gives for a `[citations.<kind>]` rule on an unwalked kind.
- Either key on a **citable** `file = "<path>"` row. Such a kind's document is where its declarations live, and [§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in) leaves Markdown alone outside a non-citable home, so there is nothing the key could mean — as `index` means nothing on a file kind (§3.4.2). A **non-citable** `file` row is not rejected: its document is governed like every other file of a non-citable home ([§FS-check.3.6.1](FS-check.md#361-which-files-a-row-governs)), so the row is exactly where that one document's grounding is said, and `grounding_level` cuts it into heading subtrees like any other Markdown file ([§FS-check.3.6.2](FS-check.md#362-the-unit)).
- `grounding_level` outside `1..=6`, on a row or in `[reference]`. There is no heading it could name.
- `grounding_level` on a row whose **effective** `require_grounding` is off — written `false` on the row, or inherited off from `[reference]`. The level could never fire, and a level nothing reads would still switch on the scanner's per-file structure pass ([AR-scanner.2.7](../architecture/AR-scanner.md#27-grounding-units-per-file)) for a tree that grounds nothing.
- `[reference] grounding_level` where the global boolean is off and no row turns grounding on. The same reason, one scope up.

`grund config show` (§4.2) prints each key on a row only where it differs from the effective global, as it does for `citable` and `scan`, so the printed config loads back as itself. Both keys are additive and do not move `grund_config_version` (§5): a config that sets one is only ever written for a binary that understands it, and an older binary meets it through the unknown-key rejection in §4.3. The global keys are kept rather than deprecated — every existing config keeps its exact meaning with no edit, and `--require-grounding` needs a global meaning regardless. Decided in [§DF-require-grounding.4](../decisions/functional/DF-require-grounding.md#4-grounding-per-place-and-per-level).

### 3.5 `[scan]` — what gets walked

```toml
[scan]
include            = ["requirements.md", "docs", "e2e", "src"]
exclude            = ["target", "node_modules", ".git", "dist", "build", ".venv"]
extensions         = ["md", "rs", "go", "java", "kt", "ts", "tsx", "js", "py", "c", "cpp", "swift", "scala", "rb", "php", "cs", "lisp", "scm", "clj", "sql", "hs", "lhs", "lua", "ada", "adb", "ads"]
comment_prefixes   = ["//", "#", ";", "--", "*", "/*"]
docstring_python   = true
respect_gitignore  = true
```

`include` is the set of paths walked **from the config root** — the directory the discovered `grund.toml` was found at (§1), or, when no config was discovered, the current working directory (never a subdirectory that merely happened to be passed as `grund`'s path argument). So in a config-less repo `grund` (no path) and `grund check .` both walk `requirements.md`, `docs/`, `e2e/`, `src/` relative to the cwd, while `grund check src/foo` or `grund check lib/` scans exactly the file or directory it is handed — an explicit path argument overrides `include` rather than being filtered by it. A walk that ends up reading no files at all is reported, not silently passed ([§FS-check.2.2](FS-check.md#22-empty-scan)). `exclude` is the set of directory names skipped at any depth. `extensions` filters which files are read. `comment_prefixes` are the markers recognized when looking for inline declarations and citations in source files. The two lists compose: adding `sql` without `--`, or `--` without `sql` (or another extension using that marker), does not enable SQL doc-comments. `docstring_python` enables Python triple-quoted-string scanning in addition to `#` comments.

**A hidden *file* is not read either, and that rule is not about descent.** The walk skips a hidden directory by not descending into it; it skips a file whose own name begins with `.` by the name that file wears, before `extensions` is consulted at all ([AR-scanner.1](../architecture/AR-scanner.md#1-tree-walk)). So `docs/.notes.md` is not scanned though `md` is listed, and a citation inside it neither resolves nor dangles — it is invisible the way one outside `include` is, and `grund check --full` does not reach it either, because that flag cancels `include` and nothing else ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full)). Being a rule about a name rather than about a descent, it also reaches a walk **root**, which is the one exception to the walk-root rule the kind-home paragraph below states. It is a blind spot the repository can see and plan around ([§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded)): what decides is the file's own name, so a document that must be checked is one not named as a dotfile, and a run handed such a file whose extension this list *does* allow says so rather than blaming the list ([§FS-check.2.2](FS-check.md#22-empty-scan)).

Listing an extension makes a file *readable*, not declarable. A prose markup format other than Markdown — AsciiDoc, reStructuredText, LaTeX — has its citations checked as soon as its extension appears here, because the citation grammar is format-agnostic; its native heading syntax still declares nothing, since a declaration is a `#`-prefixed heading or a comment-prefixed line. Whether those formats should be declaration homes of their own is an open discussion ([§DISC-markup-format-declarations](../discussions/proposals/2026-05-25-markup-format-declarations.md#disc-markup-format-declarations-declarations-in-asciidoc-restructuredtext-latex-and-similar-markup-document-formats)), not a configured behavior.

Every default comment prefix has a path through the default extension list: `;` pairs with Lisp, Scheme, and Clojure extensions; `--` pairs with SQL, Haskell, Lua, and Ada extensions; and `*` / `/*` are block-comment continuation and opener forms in the C-family extensions. Any line whose first non-whitespace run is a configured prefix is eligible to host a declaration heading or a citation. Each claimed form has a strict-mode executable case that plants a marked dangling citation in that form ([§REQ-no-missed-citation.3](../requirements/REQ-no-missed-citation.md#3-proven-per-host-language)).

**Every configured kind home is walked, whether or not `include` names it** (§3.4). A home is the repository saying "declarations and citations live here", so `include` names the *extra* roots — `src`, `crates`, a `README.md` — rather than having to repeat the homes the `[[kinds]]` table already spelled. A home that does not exist walks as nothing and earns no finding, so a fresh repository whose default homes are not scaffolded yet stays silent. A home is a **walk root**, and no walk root is pruned by `exclude`, an ignore file, or the hidden-directory rule ([AR-scanner.1](../architecture/AR-scanner.md#1-tree-walk)) — so a home the repository also excludes is read, while everything *below* it is filtered as usual. That is the honest reading of a config that says both: the `[[kinds]]` entry names the directory, and `exclude` was written about descendants. All three are rules about a descent, which is why a root outruns them; the hidden-**file** rule above is not, so it is the one skip a root does not outrun — a `file` home whose name is hidden, like an `include` entry naming one, is not read as a root either. The one home that is not a walk root is the one the config says so about: `scan = false` (§3.4.7) lists a place without walking it.

This closes a trap that had nothing to do with non-citable kinds and everything to do with why one would be configured: a `folder` or `file` outside `include` was never walked, so its declarations did not exist and its citations were **invisible rather than dangling** — no resolution, no finding, nothing to notice. A kind whose entire content is "this directory matters" would have fallen into it on its first line of config. **Upgrade note:** a repository that had a home outside `include` starts seeing that home's findings; they were always true of the tree, and the run was simply not reading it.

`include` is a **scan scope, not a fence**. A citation in a file outside it is invisible rather than merely unchecked — it does not resolve and it does not dangle — so `grund check --full` ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full)) walks the whole config root past this key and reports the references that resolve to nothing out there ([§FS-check.3.14](FS-check.md#314-out-of-scope-unresolvable-citation---full-only)), which is how a forgotten directory is found without first guessing which one to add here. The flag cancels `include` alone: `exclude`, the ignore files, and `extensions` below apply to that walk unchanged.

`respect_gitignore` (default `true`) makes the scanner honor every form of ignore file the `ignore` crate recognizes — `.gitignore` at any depth, `.git/info/exclude`, the global `core.excludesFile`, and `.ignore` files. Set to `false` only when you genuinely need to scan ignored paths. The directory-level `exclude` list above is applied **in addition** to ignore-file rules, never instead of them. See [AR-scanner.1.1](../architecture/AR-scanner.md#11-respecting-gitignore-and-friends).

**Symlinks (§3.5.1–§3.5.6).** Decided in [§DF-symlink-scan](../decisions/functional/DF-symlink-scan.md#df-symlink-scan-a-symlink-in-the-scanned-tree-is-followed-and-the-report-names-the-link).

#### 3.5.1 A symlink in the tree is followed

A **symlink inside a walked tree is followed**, file and directory alike: the link is part of the tree by the path you wrote, so what it points at is read there and its citations are checked — including when the target resolves outside the config root, because the tree is what the walk was handed and the file is in it.

#### 3.5.2 A finding names the in-tree link path

Every finding from a link met **inside a walked tree** is reported at the in-tree link path, never the target's: that is the path a reader can act on, and it is what keeps `relative_paths` output (§3.6) and the additivity rule of [§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full) meaningful. An explicit path argument follows the same reporting rule: resolving `grund check docs/beta.md` identifies and reads the target, but every text and JSON report names `docs/beta.md` as reached through the configured CLI base. This remains true when the target resolves outside the config root — the in-tree link is the bounded, actionable spelling and the external physical path never becomes report output.

The same rule reaches the **walk root itself**: a repository whose own path is reached through a link — a symlinked `~/work`, macOS resolving `/var` to `/private/var` — is walked and reported under the path the run was handed, never the physical one it resolves to. Resolving a root is how a run recognizes that a scope *is* the config root; it is not a decision about what the report calls it, and a finding spelled physically is one [`relative_paths`](#36-output--report-format) cannot render and no reader of that repository ever wrote.

#### 3.5.3 The directory rules apply under the link name

The directory rules above still apply to a followed directory under its **link** name, so `docs/node_modules -> ../../node_modules` is excluded exactly as a real directory of that name would be. The one boundary that is *not* a name rule is another project's root, which a link may not carry a walk across in any direction ([§FS-workspace.6](FS-workspace.md#6-nested-project-boundary)).

#### 3.5.4 One physical file is read once

One **physical** file is read once however many spellings reach it; when two do, the surviving spelling is the earlier root's, and within a single root the lexicographically first one ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full), [§FS-errors.4](FS-errors.md#4-determinism)).

#### 3.5.5 A link the walk cannot resolve is reported, and not walked into

A link the walk cannot resolve is not a silent skip: a broken target, or a loop such as `docs/self -> .`, is the per-file scan failure of [§FS-check.2](FS-check.md#2-outputs) — reported at the link's own path, the walk continuing past it, the run exiting `2` ([§REQ-no-missed-citation.1](../requirements/REQ-no-missed-citation.md#1-no-silent-skips)). Past it, and never *into* it: a loop is pruned where it is met, including the kind whose target reaches back over the walk root itself (`docs/up -> ..`), so the run never reports findings out of a tree it has just called unreadable and `include` still bounds what was read.

#### 3.5.6 Which unresolvable links are owed a report

That report is owed only where the walk would otherwise have read through the link, judged by the same rules as any other entry: the ignore files for both kinds, and `extensions` as well for a broken link, which names a file where a loop names a directory. So a dangling `docs/logo.png -> nowhere`, and a link of either kind that `.gitignore` covers, stay silent — the walk was never going to read them.

A broken link with **no extension at all** is silent for the same reason and is worth naming, because it is the one case where that answer can be wrong: `docs/shared -> ../nonexistent-dir` would have been a directory to descend into had it resolved. Nothing on disk distinguishes it from `bin/tool -> nowhere`, since the target does not exist and only the target could have said which it was, and reporting every extensionless dangling link is the noise this gate exists to prevent. That is a declared, bounded blind spot ([§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded)) and not a silent one: a link you need scanned is one you need to resolve.

### 3.6 `[output]` — report format

```toml
[output]
format         = "text"   # text | json
color          = "auto"   # auto | always | never
relative_paths = true     # show paths relative to config root in reports
```

`relative_paths = true` (default) renders every `<path>` in a report relative to the config root (§1). `relative_paths = false` renders them relative to the path argument passed on the command line — or to the current working directory when no path is given — i.e. the same base `grund` uses when no config is discovered. A target elsewhere inside the loaded project or workspace uses the minimum `..` components needed to reach it from that base; those parent components are allowed only while the resolved target remains inside the loaded root. Either way `grund` **never** emits an absolute path or a path that escapes the loaded root; this is what keeps the report deterministic per [§FS-errors.4](FS-errors.md#4-determinism). The CLI-base choice and the rejected workspace-root alternative are recorded in [§DF-cli-base-parent-paths](../decisions/functional/DF-cli-base-parent-paths.md#df-cli-base-parent-paths-relative_paths--false-keeps-one-cli-base-and-may-climb-within-the-loaded-root). `color` controls ANSI styling once the colored-output feature lands ([§FS-errors.3](FS-errors.md#3-message-text)); until then output is plain bytes regardless of this value, and a change to that default goes through the [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) path.

### 3.7 `[fmt.cross_refs]` — cross-reference emission

```toml
[fmt.cross_refs]
enabled       = true       # default; false opts out of generated Markdown links
anchor_format = "github"   # default; one of github | gitlab | mkdocs | pandoc | none
```

The full contract for this block — what `enabled` does, the named `anchor_format` profiles, and when the cross-reference pass runs — lives in [§FS-fmt.6.7](FS-fmt.md#67-configurability), [§DF-md-link-default-on](../decisions/functional/DF-md-link-default-on.md#df-md-link-default-on-markdown-cross-reference-links-default-on-for-github-review-and-discovery), and [§DF-md-link-anchor-strategy](../decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass). It is part of the schema here because the generated `grund.toml` ([§FS-init.2.4](FS-init.md#24-generated-grundtoml)) writes every key in this section explicitly, including `enabled = true`, so the default generated file teaches that `grund fmt --write` emits Markdown inline links in `.md` files. `[fmt.cross_refs]` is the home for cross-reference settings; today `grund fmt --cross-refs` only emits the Markdown inline-link form ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)), so `anchor_format` is the only knob — a future markup family adds its settings under this same block ([§FS-fmt.6.7](FS-fmt.md#67-configurability)), additively, with no `grund_config_version` bump (§5). The sibling `[fmt]` table (§3.10) is a different thing and is documented apart from this one: it governs every rewrite `grund fmt` performs, not just this pass.

### 3.8 `[workspace]` — sub-project namespaces

```toml
[workspace]
members          = ["apps/api", "packages/*"]
optional_members = ["vendored"]
include_root     = true
```

`members`, `optional_members` and `include_root` are specified by [§FS-workspace](FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace). The table is optional; without it the repository is a single project exactly as before. Unknown keys under `[workspace]` are errors like any other config typo.

`optional_members` is purely additive — a config that omits it behaves exactly as it did before the key existed — so `grund_config_version` stays `1` (§5), and a binary older than the key refuses it through the unknown-key rule above rather than ignoring it, which is the loud failure §5 asks of a config a binary cannot honour.

### 3.9 `[citations]` — citation direction rules

```toml
[citations]               # absent section = no direction checks (backward compatible)
default = "may"           # global default level for unlisted (citing → cited) pairs

[citations.FS]
should = ["GOAL|FS"]      # an FS declaration should cite a GOAL or a parent FS
must-not = ["AR"]         # an FS citation site may never cite an AR

[citations.E2E]
must = ["FS"]             # every E2E case must cite the FS it tests

[citations.code]          # reserved pseudo-kind: citing sites outside any kind home
should = ["FS|AR"]
```

Each `[citations.<kind>]` subsection names the **citing** kind, by the `kind` name of §3.4; its arrays name the **cited** kinds. The citing side may be any configured kind — citable or not (§3.4.1) — or `code` (§3.9.2); the cited side must be a **citable** kind, because a kind with no IDs has nothing a citation could point at. The section is decided in [§DF-citation-directions](../decisions/functional/DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels) and proposed in [§DISC-citation-directions](../discussions/proposals/2026-06-13-citation-directions.md#disc-citation-directions-encode-citation-directions-as-checked-config). It is optional; without it no direction check runs and `grund check` behaves exactly as before. Its complete user-facing explanation is [Citation directions](../user-facing/citation-directions.md); the skill carries that page as a checked copy, while the generated entrypoint carries a checked render of configured rules.

#### 3.9.1 Levels

Five keys form an RFC-2119 ladder, split into two rule classes and two enforcement surfaces:

| Level | Rule class | Checked per | Surface |
|---|---|---|---|
| `must` | obligation | declaration | `grund check` error — `missing-citation` ([§FS-check.3.11](FS-check.md#311-missing-required-citation)) |
| `should` | obligation | declaration | suggestion — `suggested-citation` ([§FS-check.2.3](FS-check.md#23-suggestions-channel-opt-in)) |
| `may` | permission | — | never checked; punches a hole in a stricter `default` |
| `should-not` | prohibition | citation site | suggestion — `discouraged-citation` ([§FS-check.2.3](FS-check.md#23-suggestions-channel-opt-in)) |
| `must-not` | prohibition | citation site | `grund check` error — `forbidden-citation` ([§FS-check.3.12](FS-check.md#312-forbidden-citation)) |

An **obligation** asks: does each top-level declaration of the citing kind contain at least one citation to the target kind, anywhere in its body? Multiple array entries are **conjunctive** — `must = ["GOAL", "GRUND"]` requires a citation to each — while a `|` disjunction inside one entry is satisfied by any one alternative — `must = ["GOAL|GRUND"]` requires a citation to either. `E2E` obligations are per case declaration: they evaluate the case's scanned files plus the case manifest's `spec.refs` entries, but an otherwise empty evidence set still fails a `must` entry rather than satisfying it vacuously. A `spec.refs` entry is kind-shaped evidence — it may name an idealized ID that does not resolve locally, so it is not subject to the dangling check that governs ordinary citations. A **prohibition** fires once per offending citation site, anchored at its exact `file:line`.

`must` and `must-not` gate (`grund check` errors); `should` and `should-not` are machine-checked suggestions that never appear in `grund check`'s standing output and are surfaced only at write time (the generated entrypoint, [§FS-init.2.3.5](FS-init.md#235-citation-directions)) and on demand (`grund check --suggestions`, [§FS-check.2.3](FS-check.md#23-suggestions-channel-opt-in)). The level→surface mapping is fixed, never a project knob, so two installs reading one config agree on what gates and what is suggested ([§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)).

#### 3.9.2 The homeless kind

Every citation site that falls outside every configured kind home resolves to one citing kind ([AR-scanner.2.4](../architecture/AR-scanner.md#24-citing-side-classification)). That kind is the **complement** of the whole `[[kinds]]` table: it is the one kind that is not a place, which is why it has no `folder` and no `file` and why there is exactly one of it.

Its name is `code` by default, and a project may name it something truer by declaring it:

```toml
[[kinds]]
kind = "src"                              # or `modules`, `implementation`, `code`…
citable = false                           # required: the complement declares no IDs
title = "Terraform modules and shell"     # optional: what it covers, for the generated block

[citations.src]
should = ["FS|AR"]
```

An entry is the homeless kind exactly when it sets `citable = false` and neither `folder` nor `file` — that shape is the declaration, not a separate key. Declaring two is a config error (§4.3): a complement is one place, and two rows claiming it leave the fallback with no single answer.

The row takes `require_grounding` and `grounding_level` like any other (§3.4.8), and that is how a project asks for grounding of its source tree and of nothing else: `kind = "code"`, `citable = false`, `require_grounding = true`. Written on this row the keys govern the complement alone; written in `[reference]` they are the default this row inherits with every other.

`code` is the default rather than a fixed name because it is the right word for most repositories and the wrong one for some — a Terraform tree, a SQL tree, a prose tree. It is still **reserved**: a `[[kinds]]` entry may take the name `code` only by *being* the homeless kind, because any other row wearing it would collide with the fallback every citation outside a home resolves to. Declaring `code` with a `title` is therefore how a project keeps the name and says what it covers.

Naming the kind moves the rules with it: `[citations.src]` governs those sites, and `[citations.code]` in that config names an unknown kind (§3.9.5) rather than sitting inert.

**Obligations apply per file**, only to files that contain at least one citation, and only to **source files** under the exact predicate `require_grounding` uses — a scanned file whose extension is not `.md` ([§DF-require-grounding.2.2](../decisions/functional/DF-require-grounding.md#22-grounded-is-defined-syntactically)). Markdown outside a kind home (a README, the changelog) is therefore prohibition-checked but obligation-exempt. A configured non-citable kind *with* a home (§3.4.1) is the same species and differs on exactly that point: its unit is every scanned file in its home, `.md` included ([§FS-check.3.11](FS-check.md#311-missing-required-citation)).

**It gets no Project map row** ([§FS-init.2.3.4.4](FS-init.md#2344-project-map)) — every row there links a place, and this is the kind that has none. Its citation-directions row renders **last**, wherever in the table it was declared, and is the one row whose subject names the kind rather than a place — *each source file outside the Project map*, qualified by its `title` where the project wrote one ([§FS-init.2.3.5](FS-init.md#235-citation-directions)).

#### 3.9.3 Namespace matching

Rule entries reuse the citation grammar of [§FS-workspace.1](FS-workspace.md#1-citation-syntax): a bare `AR` matches the **local** namespace only; `alias/AR` pins one workspace member, spelled with the same whole alias path a citation uses (`group/api/AR`, [§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)); `*/AR` matches the kind in **any** namespace including the local one. `*/` is new syntax valid in rule entries only — it is never a citation. Each entry parses as `[alias-path-or-*/]KIND`, split at the last `/` exactly as a citation is. A malformed qualifier is rejected with a citation-target diagnostic that names the target's kind and the first invalid qualifier segment; an empty qualifier or segment is named explicitly. A `*` segment is invalid unless it is the whole qualifier, and the diagnostic says so. The match is textual on the qualifier and prefix; resolution failures are separate errors ([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)), so the direction check never loads a foreign config. Each member's own `[citations]` governs the citation sites in that member's tree — like `strict`, `require_grounding`, and `[id]`, no section inherits from the workspace root.

#### 3.9.4 Defaults and precedence

`default` (top-level, or per-kind inside a `[citations.<KIND>]` table) sets the level for unlisted target kinds; the global default is `may`, so adoption is incremental. Precedence is **explicit target list > per-kind `default` > global `default`**. Obligations come only from explicit `must` / `should` entries — a `default` of `must` or `should` never invents an obligation toward every unlisted kind; `default` governs only how a citation to an otherwise-unlisted target is leveled for the prohibition pass.

#### 3.9.5 Validation

Config validation rejects: a `[citations.<kind>]` table whose kind is neither a configured `[[kinds]]` name nor the homeless kind's name (§3.9.2) — so `[citations.code]` is rejected in a config that named its complement something else; a target naming a kind that is not a configured *citable* kind — reported as *an unknown target kind* for a name the table does not hold and *a non-citable target kind* for one it does, since those are different mistakes; `code` used as a `[[kinds]]` name by anything but the homeless kind (§3.4.5); two entries declaring the homeless kind (§3.9.2); an unknown level key; and two targets of the same cited kind at different levels whose namespace matchers can match the same citation. The last rule is on namespace **overlap**, not textual equality — `*/AR` (any namespace) overlaps a bare `AR` (local), so listing one at `should` and the other at `must-not` is rejected, while a local `AR` permitted alongside a pinned `alias/AR` forbidden is allowed because those matchers are disjoint (§3.9.3). The section composes unchanged with project-defined `[[kinds]]` — a new kind is one more `[citations.<KIND>]` table.

Adding `[citations]` does **not** bump `grund_config_version` (§5): it is additive surface, like `[workspace]` and `require_grounding`. An older binary meeting it fails loudly with `unknown config section`.

### 3.10 `[fmt]` — suppressing the rewrite

```toml
[fmt]
exclude = ["docs/architecture/AR-topology.md", "docs/diagrams"]
```

`exclude` is the set of files `grund fmt` performs **no** rewrite in. The full contract — that it covers all four rewrite classes, that a suppressed file is still walked and still checked, and that a kind's index entries are wrapped there anyway — is [§FS-fmt.2.5.1](FS-fmt.md#251-fmt-exclude--a-file-at-a-time). Nothing else reads the key: no scanner, checker, or query behavior depends on it, and `grund check` says exactly the same thing about a tree with the key and without it.

Each entry is a gitignore-style glob resolved against the config root (§1) — the same dialect `respect_gitignore` already brings to the walk (§3.5) — so `docs/diagrams` takes every file under that directory, `AR-*.md` matches at any depth, and `docs/architecture/AR-topology.md` names one file. A pattern the glob parser rejects is a config error at its own line, per §4.3.

The table is optional and defaults to the empty list, which is what every config written before the key existed means. It is additive, so `grund_config_version` stays 1 (§5), and an older binary meeting it fails loudly through the unknown-section rejection (§4.3) rather than silently ignoring it. `grund config show` (§4.2) prints the table only where the list is non-empty, so a shown config still loads back as itself.

`[fmt]` is the home for settings about the `fmt` command as a whole; `[fmt.cross_refs]` (§3.7) remains the home for cross-reference settings specifically. The per-region counterpart to this key is not configured here at all — it is written in the file it governs ([§FS-fmt.2.5.2](FS-fmt.md#252-grundfmt-off--grundfmt-on--a-region-at-a-time)), and the reasoning for both is in [§DF-fmt-suppression](../decisions/functional/DF-fmt-suppression.md#df-fmt-suppression-fmt-suppression-is-per-file-and-per-region-and-the-index-carve-out-outranks-both).

## 4. Validation and inspection

### 4.1 `grund config validate [path]`

Loads the config discovered by walking up from `path` (or `.` when omitted), checks the schema, and reports problems. Exits 0 on success, 1 on validation errors — the error in the same `error: <path>:<line>: <message>` shape §4.3 defines. No tree scan is performed. A redundant config pair at the config root is reported as a `warning:` here too (§1.1, [§FS-check.4.3](FS-check.md#43-redundant-config-pair)); it is a warning, so it does not change the exit code.

#### 4.1.1 At a workspace root

When the discovered config declares `[workspace]` ([§3.8](#38-workspace--sub-project-namespaces), [§FS-workspace.2](FS-workspace.md#2-workspace-configuration)), `config validate` also expands `members` and loads every member config the run would load — nested workspaces included ([§FS-workspace.6.1](FS-workspace.md#61-nested-workspaces)) — the same launch-time pass `grund check` runs before it scans anything. The first problem, whether a member config that does not load or a `members` entry that cannot be resolved, is reported once in the same `error: <path>:<line>: <message>` line `grund check` prints for it, paths rendered from the workspace root ([§FS-workspace.5](FS-workspace.md#5-command-scope)), exit 1 (§4.3). No tree scan is performed. A path inside a member validates that member alone, as `grund check <member>` does ([§FS-workspace.5](FS-workspace.md#5-command-scope)). `config show` is unchanged: it prints the discovered project's effective config (§4.2).

### 4.2 `grund config show [path]`

Prints the **effective** configuration — defaults merged with the config discovered by walking up from `path` (or `.` when omitted), plus CLI flags — as TOML. Every `[[kinds]]` entry is printed under the canonical `kind` key, and `citable` is printed **only where it is `false`**: absence *is* `citable = true`, and the printed config has to load back as itself. `require_grounding` and `grounding_level` follow the same rule for the same reason, one scope down: a row prints either key only where its effective value differs from the effective global, which is printed under `[reference]` (§3.4.8). A row that inherits both prints neither, and the config that comes out loads back to the same effective values it went in with. Useful for debugging "why did grund recognize this citation" or "what does my config actually evaluate to." A redundant config pair at the config root is reported as a `warning:` on stderr before the TOML (§1.1, [§FS-check.4.3](FS-check.md#43-redundant-config-pair)), so the answer to "why is this key not taking effect" is on screen next to the effective value.

### 4.3 Invalid config behavior

A `grund.toml` that fails validation causes every `grund` subcommand to exit with code 2 (code 1 for `grund config validate` itself, §4.1) and a single error message pointing at the first problem, in the form `error: <path>:<line>: <message>` on stderr ([§FS-errors.2.2](FS-errors.md#22-cli-level-message), [§FS-check.2.1.1](FS-check.md#211-cli-level-messages)) — the `error:` prefix marks it a CLI-level failure, the `<path>:<line>:` inside the text points at the offending key or line. Subsequent problems are not reported until the first is fixed — this avoids cascading errors that obscure the root cause.

For concrete stderr examples and the distinction between `config validate` exit `1` and config-blocked command exit `2`, see [§FS-output-shapes.6](FS-output-shapes.md#6-cli-and-config-failures).

## 5. Schema versioning

The TOML file may include a top-level `grund_config_version = N`. The current version is **1**. Future incompatible schema changes increment this; grund refuses to load a config whose version is greater than the grund binary's known maximum, with an error suggesting an upgrade. Configs with no version key are interpreted as version 1.

The version tracks **incompatible** changes to the meaning of existing keys, not the arrival of new ones. Adding an optional table or key — `[workspace]`, `[citations]`, a future `anchor_format` profile — is additive and does not bump the version, because a config that uses it is only ever written for a binary that understands it, and an older binary meeting it fails loudly and locatably through the unknown-section / unknown-key rejection (§4.3) rather than silently misreading it. The safety net for the forward direction is the closed section and key allow-list, not the version integer. In the other direction the gate is what [§REQ-backwards-compatibility.1](../requirements/REQ-backwards-compatibility.md#1-what-is-covered) rests on: a binary that supports version `N` keeps interpreting every version `≤ N` under the semantics that version shipped with, so upgrading the binary never re-reads a config it already understood.

## 6. What is NOT configured here

Per [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible), the following are deliberately **not** configurable, to avoid the trap of every grund repo behaving differently in surprising ways:

- The set of severity levels (only `error` and `warning` exist). The `should`-level citation-direction findings ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)) do **not** add a third severity: they are carried on a separate non-severity advisory channel ([§FS-check.2.3](FS-check.md#23-suggestions-channel-opt-in)), so this frozen `{error, warning}` set stays exactly two.
- The exit code mapping (`0`/`1`/`2` per [§FS-check.2](FS-check.md#2-outputs)).
- The ordering of the report (always deterministic).
- Anything that would let two correctly-configured grund installs disagree on whether a given repo is well-formed.
- The local conversation citation *preference*: it follows the user's TUI setup and is installed through `grund integrations --write` ([§FS-integrations.4.3](FS-integrations.md#43-user-preference-and-global-agent-instructions)). A repository may commit the `link`-only *opinion* via `[reference] conversation` (§3.1, [§DF-repo-conversation-opinion](../decisions/functional/DF-repo-conversation-opinion.md#df-repo-conversation-opinion-repositories-may-commit-a-link-only-conversation-rendering-opinion)), the fallback for machines that never stated a preference; an explicitly recorded user preference wins over it ([§DF-repo-conversation-opinion.2.3](../decisions/functional/DF-repo-conversation-opinion.md#23-precedence)). Repository-web guidance stays fixed in the generated agent entrypoint ([§FS-init.2.3.6](FS-init.md#236-clickable-citations)).
