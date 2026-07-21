# DISC-conversation-rendering-layers: Layered ownership of local-conversation citation rendering

## Status

Concluded on 2026-07-21. Accepted as [§DF-repo-conversation-opinion](../../decisions/functional/DF-repo-conversation-opinion.md#df-repo-conversation-opinion-repositories-may-commit-a-link-only-conversation-rendering-opinion) and drafted into the specs
listed under "Spec changes this drafts into" below.

## Context

[§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions) installs the local-conversation rendering preference (`plain` | `link`)
exclusively into *global* agent instruction files (`~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md`, …),
on the grounds that rendering is "a property of the user's machine, not a repository". The
repository entrypoint owns only the repository-web rule ([§FS-init.2.3.4.17](../../functional-spec/FS-init.md#23417-clickable-citations)), and the repository
`.agents/grund.toml` schema deliberately has no rendering keys ([§FS-config](../../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents)).

Two frictions with the global-only design surfaced in practice:

1. **Leakage.** The global block applies in every repository the user's agent touches, including
   repositories that do not use grund. There the instruction is at best dead context and at worst a
   misfire when `§` carries another meaning (legal texts, other tooling).
2. **Zero-setup teammates get nothing.** In a grund-using repository, a contributor who has never
   run `grund integrations --write` has no conversation rendering guidance at all, even though
   declaration links would work out of the box in their agent surface.

A third idea — appending the guidance to a repo-local file excluded from version control — was
researched and turns out to be portable to exactly one agent (§4).

## Proposal

Split conversation-rendering guidance across two layers with a defined precedence, mirroring the
web-rule ownership split that already exists ([§FS-init.2.3.4.17](../../functional-spec/FS-init.md#23417-clickable-citations), [§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions)).

## 1. Repository layer: an opt-in committed opinion

In local conversations, `link` means **the declaration location as plain `path:line` text beside
the citation** — `§FS-check — docs/functional-spec/FS-check.md:1` — the row-2 form of the
[§DF-neural-link-generation](../../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command) recipe. It is *not* a Markdown `[§<ID>](path)` wrap: that decision's
matrix records the relative-path Markdown link (row 1) as non-working in terminal TUIs — Claude
Code and terminals linkify visible `path:line` text into an editor-open action, and do nothing
with a hidden Markdown target. Editor-scheme URLs (row 7) remain machine-local and unproven.

The key asymmetry making a committed opinion safe: **`link` degrades gracefully, `plain` does
not.** Plain `path:line` text opens the declaration in the editor wherever the surface linkifies
paths, and is still a correct, readable location everywhere else — there is no broken state. A
bare `§<ID>` is clickable only where a resolver is installed ([§FS-integrations.3](../../functional-spec/FS-integrations.md#3-per-client-artifacts)). Therefore
`link` is the only opinion a repository may impose on every clone; `plain` remains machine-scoped.

Concretely:

- A new optional key in `.agents/grund.toml`: `[reference] conversation = "link"`. The only
  accepted value is `link`; the key is absent by default (no opinion). The closed value set can be
  widened later without breaking the schema ([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)). This reverses the current "no
  rendering keys" stance and needs its own DF recording the reversal and the graceful-degradation
  argument.
- When the key is set, the managed agent-entrypoint block ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)) renders one additional
  config-derived sentence in its `### Clickable citations` section:
  `In local conversations, follow §<ID> with its declaration location as plain path:line text;
  fall back to the bare citation when unsure. Never use a Markdown link for this.`
  When the key is absent, the section is unchanged. New template content is a managed-block
  version bump to v5, carried under [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) exactly like the v4 bump
  ([§FS-init.2.3.6](../../functional-spec/FS-init.md#236-clickable-citations)).

For a grund-using team this yields clickable citations with zero per-user setup: clone, the agent
reads the committed entrypoint, done.

## 2. User layer: the global block becomes self-scoping

The [§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions) mechanism is kept unchanged — it is the only machine-local instruction
channel that exists for six of the eight supported agents (§4) — but the canonical block texts are
rewritten to gate themselves on grund presence:

- `plain`: `In repositories with a .agents/grund.toml: write plain §<ID> citations in local
  conversations; grund integrations makes them clickable. A repository whose agent instructions
  ask for linked citations takes precedence. Elsewhere, ignore this.`
- `link`: `In repositories with a .agents/grund.toml: follow §<ID> with its declaration location
  as plain path:line text in local conversations — never a Markdown link; fall back to the bare
  citation when unsure. Elsewhere, ignore this.`

In a non-grund repository the block is inert by its own words, and its entire footprint is one
sentence of session context. The text change is a global-instruction block version bump under the
existing marked-block contract ([§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions)).

## 3. Precedence

Repository opinion > user preference > default (`plain`). Rationale:

- Agents already weight project instructions over personal ones (Zed documents it; Codex's
  root-down concatenation implies it), so any other spec'd order would fight observed behavior.
- The only possible conflict — repo `link` vs. user `plain` — is harmless: linked citations still
  render correctly on a machine with terminal integrations installed, merely redundantly.

The precedence sentence lives in the `plain` global block (the only side with a conflict) and in
the DF; the repository block stays silent about user preference, keeping it deterministic and
config-derived only ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).

## 4. Researched alternative: a repo-local file excluded from version control

Appending the guidance to an uncommitted per-clone file (registered in `.git/info/exclude`, which
both git and the scanner already honor via `respect_gitignore`, [§FS-config](../../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents)) was researched across
the fixed supported-agent set ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)). Result: it is not a portable mechanism.

| Agent | Repo-local uncommitted instruction file | Mechanism |
|---|---|---|
| Codex | Yes | [`AGENTS.override.md`](https://learn.chatgpt.com/docs/agent-configuration/agents-md), per directory, wins over `AGENTS.md`; documented for local-only overrides |
| Claude Code | Marginal | [`CLAUDE.local.md`](https://github.com/anthropics/claude-code/issues/2394) still loads but is deprecating in favor of imports |
| Gemini CLI | No | machine scope is global [`~/.gemini/GEMINI.md`](https://google-gemini.github.io/gemini-cli/docs/cli/gemini-md.html) |
| GitHub Copilot | No | machine scope is global [`~/.copilot/copilot-instructions.md`](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions) |
| Zed | No | machine scope is global [`~/.config/zed/AGENTS.md`](https://zed.dev/docs/ai/instructions) |
| Pi | No | machine scope is global [`~/.pi/agent/AGENTS.md`](https://pi.dev/docs/latest/usage) |
| Cursor | No | `.cursor/rules/` is meant to be committed; only ignore lists have a `.local` form |
| Windsurf | Actively breaks | [gitignored `.windsurf/rules` is not loaded at all](https://github.com/Exafunction/codeium/issues/239) |

A Codex-only `--local` write (`AGENTS.override.md` + `.git/info/exclude`) remains possible as a
later additive flag on `grund integrations`, but the two layers above remove most of its
motivation: the repo layer covers opinionated repositories for every agent, and the self-scoping
global block no longer leaks. **Deferred, not designed.**

## Spec changes this drafts into (if accepted)

- New DF (`DF-repo-conversation-opinion`, name TBD): repositories may commit a `link`-only
  conversation-rendering opinion; records the graceful-degradation asymmetry, the precedence
  order, and the reversal of the "no rendering keys" stance. Cites [§DF-neural-link-generation](../../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command) and
  [§DF-integrations-command](../../decisions/functional/DF-integrations-command.md#df-integrations-command-integrations-earns-a-cli-slot-as-one-time-setup-where-a-per-citation-link-command-did-not) as the standing frame.
- [§FS-config.3](../../functional-spec/FS-config.md#3-schema): the `[reference] conversation` key (closed enum, `link` only, absent by default).
- [§FS-init.2.3.4.17](../../functional-spec/FS-init.md#23417-clickable-citations) / [§FS-init.2.3.6](../../functional-spec/FS-init.md#236-clickable-citations): config-derived local-conversation sentence; managed-block
  version bump v4 → v5.
- [§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions): new self-scoping canonical block texts with the precedence sentence;
  global-instruction block version bump; drop the "schema has no rendering keys" sentence in favor
  of a pointer to the new key.
- E2E cases: entrypoint render with/without the key; global block upgrade to the new text;
  precedence text presence; non-grund-repo inertness is textual and needs no case.

## Open questions

- Key placement: `[reference] conversation` (adjacent to marker configuration) versus a dedicated
  `[rendering]` table. `[reference]` is proposed because a second rendering key is not foreseen;
  `[fmt.cross_refs]` ([§DF-md-link-emission](../../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations)) shows precedent for feature-scoped tables if that
  changes.
- Whether the `link` global block also needs the precedence sentence for symmetry, at the cost of
  one line of context in every session ([§GOAL-token-economy](../../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) says no unless a real conflict exists).
- Whether `--conversation` later grows an `editor-link` value emitting row-7 editor-scheme URLs
  (`vscode://file/...`) — strictly user-scoped, and blocked on the pending click-through
  verification recorded in [§DF-neural-link-generation](../../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command).
