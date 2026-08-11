# DF-repo-conversation-opinion: repositories may commit a link-only conversation-rendering opinion

## 1. Context

[§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions) made local-conversation citation rendering purely user-scoped: the
`plain` | `link` preference lives in the user's Grund configuration and is synchronized into
*global* agent instruction files, and the repository `.agents/grund.toml` schema deliberately
carried no rendering keys. [§DF-integrations-command](DF-integrations-command.md#df-integrations-command-integrations-earns-a-cli-slot-as-one-time-setup-where-a-per-citation-link-command-did-not) recorded the rationale: whether a rendering
layer is installed is a property of the user's machine, and repository instructions stay
deterministic, carrying only the fixed repository-web rule ([§FS-init.2.3.4.17](../../functional-spec/FS-init.md#23417-clickable-citations)).

Two frictions surfaced in practice, examined in [§DISC-conversation-rendering-layers](../../discussions/proposals/2026-07-21-conversation-rendering-layers.md#disc-conversation-rendering-layers-layered-ownership-of-local-conversation-citation-rendering): the global
block leaks into repositories that do not use grund, and a contributor to a grund-using
repository who never ran `grund integrations --write` gets no conversation rendering guidance at
all — even though the declaration's location, travelling with the citation, would open in their
editor with zero setup.

## 2. Decision

A repository may commit a conversation-rendering opinion, and `link` is the only opinion it may
commit. The optional key `[reference] conversation = "link"` in `.agents/grund.toml` ([§FS-config.3](../../functional-spec/FS-config.md#3-schema))
makes the generated agent entrypoint ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)) teach linked local-conversation citations;
absence of the key keeps today's behavior. The user-scoped mechanism of [§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions) is
unchanged, and its canonical block texts become self-scoping — inert outside grund repositories.

### 2.1 The link form is a Markdown link over an absolute URI

In a local conversation, a linked citation is a Markdown link whose label is the bare citation and
whose target is the declaration's absolute URI — `[§FS-check](file:///<repo>/docs/functional-spec/FS-check.md#L1)`.
The addressing scheme is a per-machine choice and the form is instructed only to agents verified to
render it; both live in §DF-conversation-link-target.

This section originally specified plain `path:line` text and forbade a Markdown link outright. That
rested on row 2 of the §DF-neural-link-generation matrix — Claude Code linkifying visible `path:line`
text into an editor-open action — which was click-tested on 2026-08-11 and failed, taking the original
form's whole justification with it. What was true and stayed true is the shape of the hazard: never a
*relative* Markdown target (row 1), and never a local target on a surface that renders the destination
in place of the citation (row 3, Codex). The gate in §DF-conversation-link-target.2.4 is where that
constraint now lives, and plain `path:line` (row 11) is what it falls back to.

### 2.2 Only `link` is committable

`link` degrades gracefully: its committed form is the machine-independent `file` target
(§DF-conversation-link-target.2.3), which opens the declaration wherever the scheme dispatches, falls
back to plain `path:line` on the surfaces that cannot render it (§DF-conversation-link-target.2.4),
and is a correct, readable location in either form — a clone with no grund tooling installed has no
broken state. `plain` does not degrade: a bare `§<ID>` is clickable only
where a resolver from [§FS-integrations.3](../../functional-spec/FS-integrations.md#3-per-client-artifacts) is installed, so `plain` encodes a machine assumption
and stays user-scoped. The key's value set is a closed enum with the single member `link`,
widenable later under [§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning).

### 2.3 Precedence

Explicit user preference > repository opinion > default (`plain`). The committed opinion is the
*no-knowledge fallback*: it is the right form on every machine that never stated a preference —
fresh clones, cloud and CI sessions, agents whose only instruction channel is the committed
entrypoint ([§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions)). A recorded `plain` is machine knowledge the repository cannot
have: it is only ever written by a `grund integrations --write` that installed a rendering layer,
so bare citations are known to resolve there, and appending a location would be redundant on
exactly the machine that does not need it. Agents weight project instructions over personal ones,
so the deference is written into the project text itself — the repository sentence names the
user-level `plain` block as the exception and the `plain` block asserts itself; both sides state
the same order, and there is nothing for an agent to reconcile. The remaining risk is a stale
`plain` on a machine whose integration was since removed; that costs a bare-but-correct citation,
the state every repository had before the opinion existed. The repository block stays
deterministic and config-derived only ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).

## 3. Consequences

- The `.agents/grund.toml` schema gains its first rendering key, reversing the "no rendering
  keys" stance of [§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions). The repository block remains deterministic: the sentence
  is config-derived, like the citation-directions section ([§FS-init.2.3.5](../../functional-spec/FS-init.md#235-citation-directions)), so two installs still
  agree byte-for-byte.
- **One name, two scopes.** The user-level preference is spelled `[reference] conversation` too,
  in `$XDG_CONFIG_HOME/grund/config.toml` ([§FS-config.3.1](../../functional-spec/FS-config.md#31-reference--citation-form)). Having two names for one setting was the
  worse outcome of adding this layer, not a neutral one: the file is hand-editable, so the
  repository spelling is the spelling a user will reach for, and a scan that failed to see it
  would silently record the *opposite* preference and write it back beside what the user wrote.
  Only the accepted values differ by scope — `plain | link` for a machine, `link` for a
  repository — which is exactly the asymmetry §2.2 argues for, expressed in values rather than
  in vocabulary.
- The managed agent-entrypoint block bumps to v5 ([§FS-init.2.3.6](../../functional-spec/FS-init.md#236-clickable-citations)): setting the key renders one
  additional sentence in the `### Clickable citations` section.
- The global instruction block texts of [§FS-integrations.4.3](../../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions) are rewritten to gate themselves on
  the presence of a `.agents/grund.toml`, a block version bump under the existing marked-block
  contract; in a non-grund repository their entire footprint is one inert sentence.
- A grund-using team gets clickable conversation citations with zero per-user setup: clone, the
  agent reads the committed entrypoint, done. `grund integrations` becomes purely personal tuning
  — and a machine that recorded `plain` keeps bare citations even in an opinionated repository,
  because its rendering layer already resolves them.
- The written location is agent-composed and therefore approximate: measured over 15 headless
  trials (2026-08-11), the opinion bound a location to the citation in every trial where it was
  set and inconsistently where it was absent, but a third of those locations named the prose line
  inside the cited section rather than the section's own heading line that `grund` resolves. It is
  the same anchor-fidelity trade [§DF-neural-link-generation](DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command) already accepts for web links: off by a
  couple of lines in the right file, never a wrong file, and a click on the citation itself — the
  half a rendering layer resolves — stays exact ([§FS-integrations.3.1](../../functional-spec/FS-integrations.md#31-terminal-clients-wezterm-kitty-tmux-iterm2)).

## 4. Alternatives considered

- **Status quo: global-only.** Rejected: it cannot serve the zero-setup contributor, and its
  block text leaked into non-grund repositories. The self-scoping rewrite fixes the leak, but
  only the repository layer fixes the contributor gap.
- **A repo-local file excluded from version control.** Researched in
  [§DISC-conversation-rendering-layers.4](../../discussions/proposals/2026-07-21-conversation-rendering-layers.md#4-researched-alternative-a-repo-local-file-excluded-from-version-control) across the fixed supported-agent set: portable to Codex
  alone (`AGENTS.override.md`), marginal for Claude Code, unsupported elsewhere, and actively
  broken for Windsurf (a gitignored rules file is not loaded). Deferred, not designed.
- **Allowing `plain` as a committable value.** Rejected: `plain` presumes an installed resolver,
  which is machine state a repository cannot know (§2.2); committing it would break exactly the
  clones the repository layer exists to serve.
- **A Markdown-link form for conversations.** Rejected here on the §DF-neural-link-generation matrix
  as recorded non-working in terminal TUIs — then **reversed** by §DF-conversation-link-target after
  click-testing showed the matrix had generalized from a *relative*-target failure (row 1) to Markdown
  links as such, while the row that carried the replacement form (row 2) had never been tested at all.
  An absolute-URI target works in Claude Code (rows 12–14). The rejection was correct for the evidence
  as recorded and wrong about the evidence as gathered, which is why the reversal moved the constraint
  onto the surfaces that genuinely break (row 3) rather than onto the form.
