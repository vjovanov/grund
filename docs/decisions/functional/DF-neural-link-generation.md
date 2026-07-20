# DF-neural-link-generation: agents compose clickable citation links themselves; grund does not grow a `link` command

## Decision

Clickable citation rendering for ephemeral, user-facing text — agent TUI messages, PR
descriptions, issue and ticket bodies, review comments — is the **writing agent's job**,
specified through two instruction scopes. `grund integrations --write` records the user's local
conversation preference and synchronizes it into supported user-level agent instructions, while
repository `AGENTS.md` files carry one fixed rule for context-valid repository-web links whose
visible text is exactly the citation; plain is always the fallback. Repository-web links do not
carry Markdown title attributes for hover text. Rendering-layer integrations make the plain form
clickable locally.
`grund` itself ships no `link` subcommand and no linkify filter;
`grund fmt --cross-refs` ([§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission)) stays the only link emitter, and it emits only into
repository Markdown. This repository is the convention's testbed; the experiment that decided
this is recorded below.

## Why

1. **Tool calls are the wrong cost model for prose.** A command invocation per citation is a
   round trip plus output tokens for pure presentation, against [§GOAL-token-economy](../../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file). The
   writing agent generally already holds the declaration's path and heading from the read it
   just made — and repository Markdown is already wrapped by `grund fmt --cross-refs`, so the
   canonical `[§<ID>](<path>#<anchor>)` form usually sits in the agent's context ready to be
   copied and re-targeted.
2. **The CLI surface stays small.** The subcommand set is user-visible and frozen
   ([§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible), [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)): a shipped command is a permanent
   liability that only leaves through a deprecation path, which a presentation convenience
   does not earn while the neural path is untested at scale.
3. **A wrong link degrades to a right citation.** The visible text is the citation itself, so
   the ID layer stays intact wherever the text is quoted back into the repo — `grund check`
   validates the citation, never the URL. The worst failure of a hand-composed link (a stale
   or misspelled fragment) costs one click, not a broken grounding chain.
4. **Instructions are testable here.** Every PR description, issue, and agent session in this
   repository exercises the `AGENTS.md` recipe; observations land in the test matrix below.
   If neural anchors prove unreliable in practice, the recorded fallback is to revisit a
   *data* surface first (e.g. the anchor as a field in the ID query's JSON) before any command.
5. **Hover titles cost more than they return.** GitHub preserves an optional Markdown link title
   and desktop browsers can expose it as a delayed native tooltip, but it is not a declaration
   preview, has no touch equivalent, and has inconsistent accessibility. Repeating the declaration
   heading inside every linked citation also spends output tokens on a secondary presentation hint.
   That tradeoff fails [§GOAL-token-economy](../../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file), so agents emit no link title.

## The experiment

[§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission) already makes citations clickable in *rendered repository Markdown* by rewriting
`.md` files ([§DISC-link-support](../../discussions/proposals/2026-05-09-link-support.md#disc-link-support-link-support-as-a-derived-presentation-layer) resolved that layer). This experiment covered the other
surface: ephemeral, user-facing text, where no file is rewritten and the visible text must
stay exactly the citation. Approaches considered:

1. **Markdown links composed by the agent** — costs no tool calls; **chosen**, together with 4.
2. **OSC 8 hyperlinks for terminal output** — rejected: an agent has no business emitting raw
   terminal escapes from prose, and no command ships to produce them.
3. **A `grund link` helper command** (`--format=md|osc8|json`, plus a stdin filter) —
   prototyped on this decision's PR branch, then **reverted** for the reasons above.
4. **Agent instructions** — **chosen**: the recipe is short because the canonical wrapped form
   already exists throughout the repo's Markdown for the agent to copy.
5. **Declaration heading as a Markdown link title** — verified to survive GitHub rendering as an
   HTML `title` attribute, then **rejected**: it adds heading-sized output to every citation for a
   limited native tooltip rather than a rich, portable declaration preview.

## Test matrix

Target forms: **rel** = repo-relative path, **abs** = absolute path, **file** = `file://` URL,
**web** = repository blob URL, **editor** = `vscodium://file/<abs>:<line>`.

| # | Surface | Input form | Target | Expected | Observed |
|---|---------|-----------|--------|----------|----------|
| 1 | Claude Code (assistant message) | Markdown link `[§FS-check](…)` | rel / abs | rendered as clickable link | **fails** for rel targets (verified 2026-07-02): the terminal's click handler receives a bare relative path, not a URI, and has no working directory to resolve it against |
| 2 | Claude Code (assistant message) | plain `path:line` text | rel / abs | clickable file reference | documented Claude Code behavior (`file_path:line_number` references are clickable) |
| 3 | Codex TUI (assistant message) | Markdown link | rel / abs / web | rendered as clickable link | pending manual run |
| 4 | Plain terminal: kitty / WezTerm / iTerm2 / ghostty / VTE ≥ 0.50 | OSC 8 escape | file | Ctrl/Cmd-click opens the file | degrades to visible text where unsupported, by protocol design; no producer ships |
| 5 | GitHub PR description / issue / review comment | Markdown link | web | clickable, anchor jumps to the heading | standard GitHub Markdown rendering; web links posted on issue #33 and PR #45 (2026-07-02) |
| 6 | GitHub PR description / issue | Markdown link | rel / abs / file | **not** clickable or wrong host | known GitHub behavior: local paths do not resolve in issue/PR bodies |
| 7 | Terminal TUI (assistant message) | Markdown link over an editor URL | editor | click opens the file in the editor | viable only for ephemeral messages on the user's own machine; specimens emitted 2026-07-02, pending click-through |
| 8 | GitHub (hover) | Markdown link with a `"title"` attribute carrying the declaration heading | web | browser tooltip shows the title on hover | GitHub's Markdown API preserved the `title` attribute (verified 2026-07-20); **rejected** because the native tooltip is limited and inaccessible on touch while repeating the heading costs output tokens |
| 9 | LSP editor (hover) | plain `§<ID>` citation | — | declaration preview on hover | shipped behavior — `grund-lsp` hover ([§FS-lsp](../../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)); no markup needed |
| 10 | Terminal TUI (hover) | any link form | — | declaration content on hover | not achievable from the text side: terminals only preview the target URL on hover; content hover needs client-side integration — a VSCodium `TerminalLinkProvider` prototype validated this; productized as `grund integrations vscode` (issue #46, PR #45) |

## Recipe

The long form behind the two-sentence `AGENTS.md` instruction:

- **Shape**: `[§<ID>](<target>)` / `[§<ID>.<sec>](<target>#<anchor>)` — the same wrap
  `grund fmt --cross-refs` writes ([§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)); the cheapest correct move is to copy an
  existing wrap's target from a file already read and re-base it.
- **TUI messages**: follow the user-level instruction installed by `grund integrations --write`.
  Its default says to write the plain citation because the rendering layer resolves it (row 10);
  users without that layer may select the link override. Where a location must be explicit, use
  plain `path:line` text (row 2) or an editor-scheme link on the user's own machine (row 7), never
  a relative-path Markdown link (row 1).
- **Repository web surfaces**: use the forge's file URL without a Markdown link title (row 8).
  The writing context selects the ref: PR branch in PR bodies, reviewed commit in reviews,
  explicit commit for permalinks, and the default branch otherwise. When unsure, keep the plain
  citation.
- **Anchor** (this repo's `github` profile, [§FS-fmt.6.7](../../functional-spec/FS-fmt.md#67-configurability)): slugify the heading's rendered text —
  lowercase, delete every character that is not a letter, digit, `_`, or `-`, each space
  becomes one `-`, no run-collapsing and no trimming. A bare-ID citation anchors on the
  declaration heading, a section citation on the section heading; a source-home declaration
  has no heading anchor (GitHub accepts `#L<line>`).
- **Fallback ladder**: heading text not at hand → file link with no fragment; path uncertain
  or ID unresolved → the plain `§<ID>` citation. Never a guessed fragment.

## Consequences

- Anchor fidelity is on the agent; the visible text can stay exactly the citation on every
  surface, so it remains greppable and `grund check`-able wherever it is quoted back.
- Repository-web links carry no declaration-heading title: navigation earns its token cost;
  limited native hover text does not.
- No stdin filter: PR and ticket bodies get their links at writing time.
- Repository files are unchanged: plain citations wrapped by `grund fmt --cross-refs`, source
  files plain, exactly as before.
- Rendering-layer integrations (the local-clickability half of the convention) are user-side,
  one-time configuration in the [§FS-lsp.2.3](../../functional-spec/FS-lsp.md#23-editor-configuration-one-time-per-editor) spirit; their survey, reference implementation,
  and productization shipped as `grund integrations`; the local convention is rendered from a
  user preference into global agent instructions, while the repository-web convention is fixed
  in each repository's generated instructions (issue #46, PR #45).
