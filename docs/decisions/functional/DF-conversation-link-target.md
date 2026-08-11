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
| Claude | every target | rows 12–14, verified 2026-08-11 |
| Codex | `web` only; local schemes fall back to `path` | row 3, verified 2026-08-10; row 11 for the fallback |
| Gemini, Copilot, Zed, Pi, Cursor, Windsurf | `path` | unverified — no row claims either way |

The gate can only hold a surface at the form it already had, never make one worse, and it is reported
rather than silent: `grund integrations --write` names the effective form per target (§FS-integrations.4.3),
so a machine that set `vscodium` can see which agents took it. An unverified agent leaves the table by
being click-tested and gaining a matrix row, which is the same evidence bar row 2 failed to meet.

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
