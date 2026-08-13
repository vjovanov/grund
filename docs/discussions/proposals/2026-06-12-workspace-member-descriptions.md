# DISC-workspace-member-descriptions: Describe workspace members in generated lists

## Status

Concluded. Tracks [issue #36](https://github.com/vjovanov/grund/issues/36);
accepted as [§DF-workspace-member-descriptions](../../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists) and drafted into the specs
listed under "Spec changes this drafts into" below.

## Context

[§DISC-init-workspace-members](2026-05-17-init-workspace-members.md#disc-init-workspace-members-have-init-mention-workspace-members) gave the root `AGENTS.md` a "Workspace members"
section ([§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members)) that maps each alias to its entrypoint:

```markdown
- `common` → [common/AGENTS.md](common/AGENTS.md)
- `gradle` → [native-gradle-plugin/AGENTS.md](native-gradle-plugin/AGENTS.md)
- `maven` → [native-maven-plugin/AGENTS.md](native-maven-plugin/AGENTS.md)
- `root` → [AGENTS.md](AGENTS.md)
```

This is mechanically correct but semantically mute: nothing tells a reading
agent what each namespace is *for*. The agent must open each member's
`AGENTS.md` or guess from path names before it can pick the right alias for a
cross-project citation. The list is high-traffic grounding context, so a
one-line hint per member buys cheaper, less error-prone alias selection — the
same token-cheap-grounding argument as [§DISC-token-cheap-grounding](2026-05-12-token-cheap-grounding.md#disc-token-cheap-grounding-token-cheap-grounding-surfaces).

## Proposed shape

Add one optional top-level config key, a sibling of `project_name`, declared in
each project's own `.agents/grund.toml`:

```toml
project_name = "gradle"
project_description = "Gradle plugin that builds native images from JVM projects"
```

Rules:

- **Member-side, like the alias.** A member's description comes from the
  member's own config; the root row's description comes from the root config.
  This mirrors alias derivation in [§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases) and keeps [§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration)'s
  contract — "if a member has its own `.agents/grund.toml`, that file
  configures the member" — intact. A member without its own config simply has
  no description.
- **Single line.** A value containing a line break is a config error at the
  `project_description` line, reported per [§FS-config.4.3](../../functional-spec/FS-config.md#43-invalid-config-behavior). No hard length cap
  in v1; the inline-note column discipline is a style concern, not a parse
  concern.
- **Presentation metadata only.** The key never participates in alias
  derivation, citation resolution, or `check` semantics. Omitting it changes
  nothing but the rendered bullet.

The [§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members) renderer appends the description after the existing
link, em-dash separated; bullets without a description keep today's exact
form, and the *(not yet initialized)* suffix stays last:

```markdown
- `common` → [common/AGENTS.md](common/AGENTS.md) — Shared model and utilities
- `gradle` → [native-gradle-plugin/AGENTS.md](native-gradle-plugin/AGENTS.md) — Gradle plugin that builds native images from JVM projects
- `maven` → [native-maven-plugin/](native-maven-plugin/) *(not yet initialized)*
- `root` → [AGENTS.md](AGENTS.md) — Workspace root: shared specs and release tooling
```

Two companion touches:

- The generated `.agents/grund.toml` template ([§FS-init.2.4](../../functional-spec/FS-init.md#24-generated-grundtoml)) gains a commented
  `# project_description = "<one line shown next to this project in workspace member lists>"`
  teaching line under `project_name`, since the template writes its surface
  explicitly.
- `grund init --description "<text>"` joins `--name` so a member can be
  bootstrapped with both, and the [§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members) self-exception renders the
  pending description the same way it already renders the pending alias.

## Alternatives considered

- **Structured member entries at the root**
  (`members = [{ path = "apps/api", description = "…" }]`). Rejected: it kills
  the `packages/*` glob ergonomics of [§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration), forces a second
  `members` parse shape, and moves a member fact to the root.
- **Alias-keyed root table** (`[workspace.descriptions] api = "…"`). Rejected:
  it breaks silently on alias rename, invites dangling keys that need new
  diagnostics, and again edits the root to describe a member.
- **Derive from the member's `GRUND` lead.** Attractive — the motivation text
  already exists — but the lead is multi-sentence prose written for a
  different audience, so v1 would need truncation heuristics and would give
  authors no control over the one line agents actually see. Kept as a
  possible later fallback (configured string wins, `GRUND` lead otherwise),
  not as the v1 mechanism.

## Spec changes this drafts into

- [§FS-config](../../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-found-by-walking-up): the new `project_description` key next to `project_name`, with
  single-line validation under [§FS-config.4.3](../../functional-spec/FS-config.md#43-invalid-config-behavior).
- [§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases): description resolution alongside alias resolution (member
  config, else none; root config for the root row).
- [§FS-init.2.3.4.15](../../functional-spec/FS-init.md#23415-workspace-members): the extended bullet rendering, the template teaching
  line, and the `--description` flag with its pending-config self-exception.

## Boundaries

- No prompts, no inference: `init` never invents a description, and a missing
  one renders today's bullet unchanged.
- Root-side `init` still writes nothing under member directories.
- The key is ignored everywhere outside generated-entrypoint rendering; in
  particular it cannot influence which project a citation resolves to.

## Open questions

- Key name: `project_description` (proposed, parallels `project_name`) vs. a
  bare `description` vs. `project_title` (parallels `[[kinds]] title`).
- Should `grund list` (or a future `grund members`) print the description too,
  so the fact is reachable without opening `AGENTS.md`?
- Is a soft length lint (warning above ~120 characters) worth having, or does
  review pressure on the generated block suffice?
- When the `GRUND`-lead fallback arrives, does it need a marker distinguishing
  derived text from authored text, or is silent fallback fine?
