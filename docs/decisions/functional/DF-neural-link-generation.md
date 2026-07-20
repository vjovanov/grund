# DF-neural-link-generation: agents compose clickable citation links themselves; grund does not grow a `link` command

## Decision

Clickable citation rendering for ephemeral, user-facing text — agent TUI messages, PR
descriptions, issue and ticket bodies, review comments — is the **writing agent's job**,
specified as a generated `AGENTS.md` convention: `[render.links].conversation` chooses plain
citations or context-valid links whose visible text is exactly the citation, and plain is
always the fallback. Rendering-layer integrations make the plain form clickable locally.
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
| 8 | GitHub (hover) | Markdown link with a `"title"` attribute carrying the declaration heading | web | browser tooltip shows the title on hover | specimen posted on PR #45 (2026-07-02); pending hover check |
| 9 | LSP editor (hover) | plain `§<ID>` citation | — | declaration preview on hover | shipped behavior — `grund-lsp` hover ([§FS-lsp](../../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)); no markup needed |
| 10 | Terminal TUI (hover) | any link form | — | declaration content on hover | not achievable from the text side: terminals only preview the target URL on hover; content hover needs client-side integration — a VSCodium `TerminalLinkProvider` prototype validated this; productization is issue #46 |

## Recipe

The long form behind the two-sentence `AGENTS.md` instruction:

- **Shape**: `[§<ID>](<target>)` / `[§<ID>.<sec>](<target>#<anchor>)` — the same wrap
  `grund fmt --cross-refs` writes ([§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)); the cheapest correct move is to copy an
  existing wrap's target from a file already read and re-base it.
- **TUI messages**: write the plain citation — the rendering layer resolves it (row 10;
  issue #46). Where a location must be explicit, plain `path:line` text (row 2), or an
  editor-scheme link on the user's own machine (row 7); never a relative-path Markdown link
  (row 1).
- **GitHub surfaces**: a blob URL, `https://github.com/<owner>/<repo>/blob/<ref>/<path>#<anchor>`,
  with the declaration heading as the link's Markdown title so hover shows the fact (row 8).
  The writing context selects the ref: PR branch in PR bodies, reviewed commit in reviews,
  default branch in issues, explicit commit for permalinks. When unsure, keep the plain citation.
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
- No stdin filter: PR and ticket bodies get their links at writing time.
- Repository files are unchanged: plain citations wrapped by `grund fmt --cross-refs`, source
  files plain, exactly as before.
- Rendering-layer integrations (the local-clickability half of the convention) are user-side,
  one-time configuration in the [§FS-lsp.2.3](../../functional-spec/FS-lsp.md#23-editor-configuration-one-time-per-editor) spirit; their survey, reference implementation,
  and productization (`grund integrations`, config-rendered convention sentences) are tracked
  in issue #46.
