# DF-config-file-location: grund.toml is discovered at two names per directory, and init writes the bare one

## 1. Context

Config discovery probed exactly one location: `.agents/grund.toml`, found by walking up from the
working directory ([§FS-config.1](../../functional-spec/FS-config.md#1-file-location-and-discovery)). The `.agents/` wrapper was chosen for a repository root, where it
keeps agent-facing tooling configuration off a root already carrying the project's own metadata —
`Cargo.toml`, `package.json`, `pyproject.toml` — and leaves room for other agent tools to colocate
their files beside `grund.toml`.

Workspaces ([§FS-workspace](../../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace)) stretched that rationale past where it holds. A workspace with four
members means four `member/.agents/` directories, each containing exactly one file one level down.
Nothing else ever lands in them: the sibling agent-tooling files a repository root accumulates live
at the root, not per member. And the layout was not merely conventional — a member root was partly
*identified* by carrying `.agents/grund.toml`, so the nesting was load-bearing in
`grund init`'s workspace-member rendering ([§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members)) as well as in discovery.

Two facts about the single-probe rule made it worth revisiting rather than living with. It is the
first thing every adopter meets, and a config file at a path no other tool in the ecosystem uses is
a worse first meeting than one that sits where `Cargo.toml` and `package.json` sit
([§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible)). And it is the one part of the config contract a user cannot override
from within the config, because it is what has to be found before any key can be read
([§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable)).

## 2. Decision

### 2.1 Symmetric dual discovery

At **every** directory of the upward walk, `grund` probes two names in order — `.agents/grund.toml`
first, then a bare `grund.toml` — and the first that exists is the config. The directory holding it
is the config root, exactly as before: relative paths inside the config resolve against it, never
against `.agents/`.

```
repo/
  grund.toml              ← root, bare
  packages/app/
    grund.toml            ← member, bare — no nested .agents/
  packages/lib/
    .agents/grund.toml    ← member, still fine
```

One rule at every level, with no root-versus-member special case. A project picks the form that
suits it, and a workspace may mix the two, because the choice is per directory and nothing outside
discovery depends on which was chosen.

The walk stops at the *directory* that has either name, not at the first `.agents/grund.toml` above
the cwd. That matters for a bare member under a `.agents/`-style root: the member's own bare
`grund.toml` must shadow the root config the same way a nested `.agents/grund.toml` always did, or
the two forms would not be interchangeable.

### 2.2 The bare `grund.toml` wins a tie, and `check` warns about the pair

When one directory carries both names, the bare `grund.toml` is the config. **The form the tool
generates is the form that wins.** §2.3 makes `grund init` write the bare file and argues it is the
better default; a tie-break that then handed the decision to the other file would say the opposite,
and a user would have to hold two rules — "grund writes this one" and "grund reads that one" —
whose only relationship is that they disagree.

It is also what a user reaching for a root `grund.toml` means. A repository acquires the pair one
way: someone deliberately puts a bare file next to an existing `.agents/` one, and the reason to do
that is to move to the form the tool recommends. Winning the tie is that move working; losing it is
the move silently doing nothing.

The ignored file is reported: `grund check` emits a warning naming the `.agents/grund.toml` and the
bare `grund.toml` that outranks it ([§FS-check.4.3](../../functional-spec/FS-check.md#43-redundant-config-pair)). A file a user edits and `grund` ignores is
precisely the confusion dual discovery could otherwise introduce, and a warning says so without
blocking the run. That warning is what makes the tie-break safe to state this way: the losing file
is never silently ignored, so the failure mode the opposite order was reaching for — a config
quietly replaced — is reported at the first `check` either way.

Warning rather than error, because the pair is the natural transient state of a migration in either
direction: a repository moving its config out of `.agents/` writes the new file, runs `check`, and
deletes the old one. An error would make the intermediate step of a supported migration a hard
failure, which is the opposite of what a migration aid should do. Warnings never affect the exit
code ([§FS-check.2](../../functional-spec/FS-check.md#2-outputs)), so a repository mid-move stays green while the diagnostic stays visible.

### 2.3 `grund init` writes the bare `grund.toml`

`grund init` generates `<target>/grund.toml`, not `<target>/.agents/grund.toml`
([§FS-init.2.4](../../functional-spec/FS-init.md#24-generated-grundtoml)). The generated config is a teaching surface — every default written out explicitly —
and what it teaches first is where the file lives. A tool that supports both forms and generates
the one that needs a directory created for it is recommending the other one by omission.

The bare form is what the rest of the ecosystem does. `Cargo.toml`, `package.json`,
`pyproject.toml`, `deno.json`, `.editorconfig` are all root-visible: a contributor who opens the
repository sees that the project is grund-grounded without knowing to look inside a dot-directory,
and every "where do I change the marker" question answers itself from a directory listing. That
discoverability is the same argument [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) makes for loud config errors, applied
one step earlier — to finding the file at all.

Existing repositories are untouched. `init` never overwrites a config it finds ([§FS-init.3](../../functional-spec/FS-init.md#3-non-intrusive-guarantees)), and
that rule now reads across both names: a repository with `.agents/grund.toml` gets
`exists .agents/grund.toml` and no second file, so re-running `init` after this change cannot
produce the redundant pair §2.2 warns about. Moving an existing config to the root is a `git mv`
with no other edit, and never required.

## 3. Consequences

- **No `grund_config_version` bump.** Discovery gains a second probe; no existing key changes
  meaning and no existing file becomes invalid ([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)). A config written for the old rule is
  read identically by the new one.
- **Member-root identity is a two-name test.** Everything that asked "does this directory have
  `.agents/grund.toml`" — the workspace-member self-exception in `init` ([§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members)), the
  nested-workspace boundary ([§FS-workspace.6](../../functional-spec/FS-workspace.md#6-nested-project-boundary)), the member-config residency rule for `project_name`
  and `project_description` ([§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration), [§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases)) — now asks whether the directory has
  *either* name. Where the answer feeds a diagnostic, the diagnostic names the file that was
  actually read.
- **Generated agent instructions stop naming a layout.** The entrypoint and global-instruction texts
  that scoped themselves to "repositories with a `.agents/grund.toml`" would exclude a bare-config
  repository on a literal reading, so they name `grund.toml` and let the reader's tree answer where
  it sits ([§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions)). That is a managed-block text change and carries the block version
  bump the marked-block contract requires.
- **Two names to search for.** Anyone grepping a machine for grund configs, and any future tool that
  wants to read one, has two paths to consider instead of one. This is the standing cost of the
  decision, paid once per such consumer, and the reason §2.1 fixes the probe order rather than
  leaving it to a search.

## 4. Alternatives considered

- **Bare `grund.toml` for members only.** Rejected: it makes the root and its members structurally
  different projects for no reason a user could predict, and it leaves the one-file-in-a-directory
  awkwardness in place at exactly the level — the root — where a contributor first meets it. The
  ticket's own framing of "one uniform rule" is the whole value of the change.
- **Move to a bare `grund.toml` outright and drop `.agents/`.** Rejected: every repository that
  adopted `grund` under the old rule would break at once, with a config silently ignored rather than
  reported — the failure mode [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) exists to prevent. Dual discovery reaches the
  same destination for new projects while costing existing ones nothing.
- **`.agents/grund.toml` wins the tie.** Rejected in favor of §2.2. The argument for it is that a
  bare file dropped beside an existing `.agents/` one cannot then take over the grammar a
  repository's citations were written against, and this repository supplied evidence for exactly
  that shape: six e2e fixtures carried dead root copies orphaned when their configs moved into
  `.agents/`, one of them stale enough to disagree with the file actually read. But those files
  predate dual discovery — under the old rule a bare `grund.toml` was neither generated nor read, so
  every one of them is an artifact of a form that was never live, not a case of a user writing the
  file on purpose. Going forward the bare file is the deliberate one, `check` reports the pair
  either way (§2.2), and the cost of this order is a rule that contradicts the tool's own default.
- **Erroring on the redundant pair.** Rejected in favor of warning, for the migration reason in
  §2.2. Revisitable if the pair turns out to arise from anything other than a move in progress.
- **Keeping `init` on `.agents/` while supporting both.** Rejected: it makes the supported bare form
  a thing users must discover from the specification rather than from the tool, and leaves the
  default at the option §2.3 argues is the worse first meeting. `init` is where a default is
  actually expressed.
