# DF-integrations-command: integrations earns a CLI slot as one-time setup, where a per-citation `link` command did not

## 1. Context

Two halves of the clickable-citations convention were separated by [§DF-neural-link-generation](DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command). The first half — composing a link for one citation in ephemeral prose — was refused a command: a tool call per citation is the wrong cost model for presentation ([§GOAL-token-economy](../../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file)), and the writing agent already holds the wrapped form to copy. A prototype `grund link` was built on that branch and reverted.

The second half is the rendering layer that makes a *plain* `§<ID>` clickable in a terminal or editor. That layer is real code — terminal config snippets, a resolver script, an editor extension — and today it lives nowhere a user of `cargo install grund` can reach. [§FS-integrations](../../functional-spec/FS-integrations.md#fs-integrations-grund-prints-and-installs-its-rendering-layer-integrations) specifies a `grund integrations` command that prints and installs it. This decision records why that command earns a slot on a surface that is user-visible and frozen ([§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible), [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)) when `link` did not.

## 2. Decision

Ship `grund integrations` as a subcommand. It is the counterpart to `grund completions`: an embedded artifact, printed on demand, installed on request. The two commands sit at opposite ends of a frequency axis, and that axis is the whole justification.

### 2.1 Frequency is the test

`link` would run once per citation, inside the hot authoring loop, on a surface an agent regenerates constantly — so it must be free of tool calls, and it is (the agent copies the wrapped form). `integrations` runs once per machine, out of band, to configure the terminal or editor. A one-time setup command that a human or an agent invokes deliberately does not compete with the token economy of the authoring loop; it competes with the alternative of a hand-written setup guide, and it wins by carrying the artifacts in the binary so there is nothing to copy-paste wrong. This is exactly the precedent `completions` and `init` set, and `integrations` is spec'd to match their dry-run-first, idempotent-write shape ([§FS-integrations.1](../../functional-spec/FS-integrations.md#1-user-facing-command), [§FS-integrations.4](../../functional-spec/FS-integrations.md#4-managed-writes---write)).

### 2.2 Printing is the default; writing is opt-in

The command prints by default and only touches disk under `--write`, as managed marked blocks ([§FS-integrations.4](../../functional-spec/FS-integrations.md#4-managed-writes---write)). This keeps the same safety contract as `init`: re-runs are idempotent, upgrades are diffs, removal is clean. An agent can therefore propose the change, show the diff from `--format json` detection, and write only on confirmation ([§FS-init.5](../../functional-spec/FS-init.md#5-agent-setup-instructions)) — the command is safe to hand to automation because its default is inert.

### 2.3 The client set stays closed

The clients are a frozen, enumerated set ([§FS-integrations.1](../../functional-spec/FS-integrations.md#1-user-facing-command)). A new client is a deliberate, changelog-gated addition, not an open plugin surface — so the command does not become a maintenance sink, and the no-first-party-marketplace-plugin stance ([§FS-lsp.2.3](../../functional-spec/FS-lsp.md#23-editor-configuration-one-time-per-editor)) is preserved by shipping every artifact embedded in the binary rather than published.

## 3. Consequences

- The frozen subcommand surface grows by one verb — a change gated by [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path), justified by one-time setup value rather than per-use convenience — the line `link` failed to clear.
- `cargo install grund` now delivers the rendering layer, not just the checker, so the plain-citation-is-clickable half of [§DF-neural-link-generation](DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command) is reachable without this repository as a testbed.
- The local-conversation sentence becomes user configuration installed into detected global agent
  instruction files, so it follows the user's TUI setup without making repositories encode a
  machine-local preference. Repository instructions remain deterministic and carry only the
  fixed repository-web rule ([§FS-init.2.3.6](../../functional-spec/FS-init.md#236-clickable-citations)).

## 4. Alternatives considered

- **A setup guide in docs, no command.** Rejected: a guide cannot be `--write`-installed idempotently, cannot emit a machine plan for an agent, and drifts from the artifacts it describes. The `completions`/`init` precedent is that setup which ships bytes belongs in the binary.
- **A general `grund link`/linkify surface covering both halves.** Rejected in [§DF-neural-link-generation](DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command) and not reopened here: the per-citation half stays an `AGENTS.md` convention; only the one-time-setup half earns a command.
- **A published marketplace extension instead of embedded `vscode --write`.** Deferred, not chosen: shipping the extension embedded keeps the [§FS-lsp.2.3](../../functional-spec/FS-lsp.md#23-editor-configuration-one-time-per-editor) stance and avoids a marketplace release train; revisit with a new decision only if `--write` friction proves real.
