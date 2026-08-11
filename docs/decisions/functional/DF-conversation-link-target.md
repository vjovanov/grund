# DF-conversation-link-target: the conversation link form is a Markdown link over an absolute URI, addressed per machine

## 1. Context

§DF-repo-conversation-opinion.2.1 defined the `link` form as the declaration location in plain
`path:line` text, *never* a Markdown link. That form rested on row 2 of the §DF-neural-link-generation
matrix — Claude Code linkifies `file_path:line_number` references itself — which was documented
behavior recorded without a click-test, the one row in the matrix carrying no observation of its own.

Click-testing on 2026-08-11 reversed it. Plain `path:line` is not linkified in Claude Code in either
the relative or the absolute form: the location travelled beside the citation, was correct, and did
nothing. The committed `link` opinion was therefore buying, on its primary target surface, exactly the
readable-but-dead text it already had.

The same session tested what row 1 was being read as excluding. Row 1's failure is a *relative*
Markdown target, whose click handler receives a path with no working directory to resolve it against —
a missing scheme, not a rejected Markdown link. An absolute URI has neither problem, and all three
schemes tested worked on the first attempt: `file:`, `vscodium:`, and `https:` (rows 12–14).

## 2. Decision

A citation in a local conversation renders as a **Markdown link whose label is the bare citation and
whose target is an absolute URI**. Which scheme addresses the declaration is the machine's choice,
recorded as `[reference] conversation_target` in the user configuration (§FS-config.3.1,
§FS-integrations.4.3), and the form is instructed only to agents whose renderers are verified to
honor it (§2.4). `conversation = "link"` remains the switch that decides *whether* a location travels
with the citation; this key decides only *how it is addressed*.

### 2.1 The label is the citation, the target is the location

`[§FS-refs.3.3](file:///repo/docs/functional-spec/FS-refs.md#L50)` keeps the property every earlier
form was chosen to protect: the visible text is exactly the citation, so the ID layer survives being
quoted back into the repository and `grund check` still validates it (§DF-neural-link-generation).
What changes is that the location stops competing with the citation for line width — it moves behind
the label instead of trailing it, which is also why the form can carry a full absolute URI without
costing the reader anything.

### 2.2 The scheme is chosen per machine, and `file` is the default

`conversation_target` is a closed enum. Each value names a fixed template that an agent fills from the
declaration's absolute path and line:

| Value | Target | Notes |
| --- | --- | --- |
| `file` | `file://<abs>#L<line>` | **default**; assumes only a desktop handler for `file:` |
| `path` | *(no URI)* — plain `path:line` beside the citation | the pre-2026-08-11 form, kept as the opt-out |
| `web` | the forge blob URL for the current ref | the §DF-neural-link-generation repository-web recipe, reused |
| `vscode`, `vscodium`, `cursor` | `<scheme>://file<abs>:<line>` | one shape across the VS Code family; `vscodium` verified (row 13) |

`file` is the default because it is the only local form that presumes nothing about the machine beyond
a handler every desktop already has, and because it names the real file: where the scheme fails to
dispatch, the reader still sees the path and the line. An editor scheme opens the declaration where the
work happens and is the better setting, but only a machine can know it applies — so it is a choice,
not a default. `web` is for transcripts that leave the machine; it costs a network round trip and a ref
that must already exist on the forge, so an unpushed commit resolves to nothing.

### 2.3 The target is user-scoped, but the default is committable

`vscodium:` opens only where that editor registered a scheme handler; that is machine state, and
committing it fails the §DF-repo-conversation-opinion.2.2 test the same way `plain` does. The key is
therefore a user-configuration key with no repository spelling.

The *default* is a different question, and it is committable. `file` embeds nothing machine-specific:
the agent composes the absolute path at write time from the repository root it already has, so two
installs render the same instruction byte-for-byte (§FS-non-goals.13). A repository that commits
`conversation = "link"` therefore teaches the `file` form through its generated entrypoint
(§FS-init.2.3.4.17), and a machine that names a target overrides it under the precedence rule that
already governs this pair (§DF-repo-conversation-opinion.2.3).

### 2.4 The form is gated per agent, and the fallback is `path`

Row 3 of the matrix is the constraint: the Codex TUI renders a local-path destination *in place of the
link label*, erasing the visible citation. Instructing the link form there would destroy the one thing
§2.1 exists to protect, on a surface where the failure is silent — the transcript reads as if the agent
never cited anything.

A target is therefore instructed to an agent only where the pair is verified, and every other agent's
block keeps `path`:

| Agent | Instructed form | Evidence |
| --- | --- | --- |
| Claude | every target | rows 12–14, click-tested 2026-08-11 |
| Codex | `web` only; local schemes fall back to `path` | rows 3 and 15, click-tested 2026-08-11; row 11 for the fallback |
| Pi | `file` and `web`; editor schemes fall back to `path` | row 16, click-tested 2026-08-11 — the floor, not the ceiling (below) |
| Gemini, Copilot, Zed, Cursor, Windsurf | `path` | unverified — no row claims either way |

Codex was click-tested against this table on 2026-08-11 and the table survived, with one qualification
worth recording: its TUI never hides a destination behind a label, so even the `web` form it *is* given
renders as the citation followed by a visible URL. `web` earns its place there because it is the only
natively clickable form on that surface, not because it renders the way it does in Claude. An editor
scheme, by contrast, measured as the worst of both — a long inline URL and no click at all — which is
the concrete reason the local schemes fall back instead of being passed through on the user's say-so.

Pi adds a third profile and, with it, the sharpest statement of what the gate actually measures. Its
labels always survive, and what it does with a destination depends on the *terminal* rather than on
Pi: with OSC 8 available it hides the URI behind the label and every scheme dispatches, editor
schemes included, while without it the renderer prints `text (url)` and only the schemes a terminal's
own URL matcher knows — `file:` and `https:` — can be clicked (row 16).

That splits Pi's entry along an axis the other agents do not have, and the table records the
guaranteed floor rather than the ceiling: `file` and `web` click under **both** outcomes, an editor
scheme only under one. A reader whose terminal grund never saw is the case the table has to be right
for, so Pi is listed at what works everywhere. The ceiling is reachable and worth reaching — it needs
grund to know the reader's terminal at the moment the instruction is written, which is a real
extension of §2.4 rather than a correction to it.

The gate can only hold a surface at the form it already had, never make one worse, and it is reported
rather than silent: `grund integrations --write` names the effective form per target (§FS-integrations.4.3),
so a machine that set `vscodium` can see which agents took it. An unverified agent leaves the table by
being click-tested and gaining a matrix row, which is the same evidence bar row 2 failed to meet.

### 2.5 A per-agent override is a preference, not evidence

One machine reads several agents and they demonstrably do not render alike, so the machine-wide
target is the wrong granularity for a machine that reads two of them. `[reference.agents.<agent>]`
is a **partial of the machine-wide settings, keyed by agent**, shallow-merged over them
(§FS-integrations.4.4): a key present under an agent replaces the base for that agent, an absent
key inherits, and there is no per-agent default to reason about.

The shape was chosen against a flatter alternative — a `conversation_target_by_agent` map of
scalars beside the key it overrides. That is smaller while exactly one key is overridable, and it
does not survive a second: the two would be parallel maps under the same agent names, tied
together by naming convention rather than by structure, and both would have to be read to answer
what one agent gets. A partial of the parent is the same idea expressed as data, so a second key
is one more accepted name inside it rather than a second map.

What the layer must *not* become is a way around §2.4. The override sets the **request**; the gate
sets the **verdict**, and it still runs last. `[reference.agents.codex] conversation_target =
"vscodium"` resolves to `vscodium` and is then gated to `path`, precisely as the machine-wide value
would be. The distinction is the whole reason the layer is safe to add: a user asserting a
preference about their own machine is ordinary configuration, while a user overriding a click-test
would be writing an instruction recorded as erasing citations on that surface, and no
configuration key should be able to buy that. The motivating case needs no such power anyway —
Claude on `vscodium` with Codex on `web` is two requests the gate already grants.

Both layers are therefore reported per target (§FS-integrations.4.4). Unreported, an override and a
gate downgrade and an unread key all look identical from the outside: a block that does not say
what the user set.

## 3. Consequences

- Matrix rows 1 and 2 are corrected and rows 12–14 added; the corrected row 2 is the reason this
  decision exists, and the reason an unverified row is now recorded as unverified rather than as
  behavior.
- The clause *never a Markdown link* leaves the three texts that carried it — the repository entrypoint
  (§FS-init.2.3.4.17), the `link` global block (§FS-integrations.4.3), and §DF-repo-conversation-opinion.2.1
  — and is replaced by the narrower true statement: never a *relative* target, and never a local target
  on a surface that renders the destination in place of the label.
- The managed entrypoint block bumps to v6 and the agent-guidance block to v3 (§GOAL-no-silent-breakage).
- The enum is widenable without a `grund_config_version` bump (§FS-config.5). Zed and the JetBrains
  family are deliberately absent: their URI shapes differ from the VS Code family's and neither is
  verified here, and shipping an unverified template is what this decision was written to stop.
- A machine that wants the old behavior sets `conversation_target = "path"`, so nothing that worked
  before this decision becomes unreachable.
- The user configuration gains its first nested table. It stays a closed allow-list of warnings
  rather than errors (§FS-integrations.4.3), with one new member: an override under an agent grund
  does not know names the closed agent set rather than the key, because the mistake is nearly
  always the agent's spelling.
