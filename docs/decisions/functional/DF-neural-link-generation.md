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
| 1 | Claude Code (assistant message) | Markdown link `[§FS-check](…)` | rel | rendered as clickable link | **fails** (verified 2026-07-02): the terminal's click handler receives a bare relative path, not a URI, and has no working directory to resolve it against. The failure is the missing *scheme*, not the Markdown link — an absolute URI target works (rows 12–14) |
| 2 | Claude Code (assistant message) | plain `path:line` text | rel / abs | clickable file reference | **fails** (verified 2026-08-11): neither form linkifies. This row previously recorded *documented* Claude Code behavior that was never click-tested, and it was wrong; rows 12–14 are what replaced it |
| 3 | Codex TUI (assistant message) | Markdown link | rel / abs / web | rendered as clickable link | **fails** for rel / abs (verified 2026-08-10 against the Codex TUI renderer source): a local-path destination is rendered *in place of the link label*, erasing the visible citation, and only `http(s)` URLs receive OSC 8 hyperlinks — web targets alone render clickable. Both halves were confirmed by click-test on 2026-08-11 (row 15), which also found what the source-read missed: even the web target shows its URL beside the label rather than behind it |
| 4 | Plain terminal: kitty / WezTerm / iTerm2 / ghostty / VTE ≥ 0.50 | OSC 8 escape | file | Ctrl/Cmd-click opens the file | degrades to visible text where unsupported, by protocol design; no producer ships |
| 5 | GitHub PR description / issue / review comment | Markdown link | web | clickable, anchor jumps to the heading | standard GitHub Markdown rendering; web links posted on issue #33 and PR #45 (2026-07-02) |
| 6 | GitHub PR description / issue | Markdown link | rel / abs / file | **not** clickable or wrong host | known GitHub behavior: local paths do not resolve in issue/PR bodies |
| 7 | Terminal TUI (assistant message) | Markdown link over an editor URL | editor | click opens the file in the editor | specimens emitted 2026-07-02, click-through **resolved 2026-08-11** and it is surface-specific: works in Claude Code (row 13), not clickable in the Codex TUI (row 15). Still machine-local by nature — the scheme handler is the user's — which is why the target is a user-configuration key and never committable ([§DF-conversation-link-target.2.3](DF-conversation-link-target.md#23-the-target-is-user-scoped-but-the-default-is-committable)) |
| 8 | GitHub (hover) | Markdown link with a `"title"` attribute carrying the declaration heading | web | browser tooltip shows the title on hover | GitHub's Markdown API preserved the `title` attribute (verified 2026-07-20); **rejected** because the native tooltip is limited and inaccessible on touch while repeating the heading costs output tokens |
| 9 | LSP editor (hover) | plain `§<ID>` citation | — | declaration preview on hover | shipped behavior — `grund-lsp` hover ([§FS-lsp](../../functional-spec/FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)); no markup needed |
| 10 | Terminal TUI (hover) | any link form | — | declaration content on hover | not achievable from the text side: terminals only preview the target URL on hover; content hover needs client-side integration — a VSCodium `TerminalLinkProvider` prototype validated this; productized as `grund integrations vscode` (issue #46, PR #45) |
| 11 | Codex TUI (assistant message) | plain `path:line` text | rel / abs | clickable file reference | not linkified by Codex itself — its renderer hyperlinks web URLs only (verified 2026-08-10) — but the text survives verbatim for the rendering layer: `grund integrations` clients, iTerm2 Semantic History, and the VS Code terminal's own path links make it clickable |
| 12 | Claude Code (assistant message) | Markdown link `[§FS-refs.3.3](…)` | file | label stays the citation, click opens the file | **works** (verified 2026-08-11): `file:///<abs>#L<line>` |
| 13 | Claude Code (assistant message) | Markdown link | editor | click opens the editor at the line | **works** (verified 2026-08-11): `vscodium://file/<abs>:<line>`, dispatched through the scheme handler the editor's own desktop entry registers |
| 14 | Claude Code (assistant message) | Markdown link | web | click opens the browser | **works** (verified 2026-08-11) — the control that separates "the renderer honors Markdown links" from "this scheme dispatches" |
| 15 | Codex TUI (assistant message) | Markdown link | file / editor / web | label stays the citation, destination hidden behind it | **the destination is never hidden** (observed 2026-08-11, the same three-target probe that produced rows 12–14): a `file:` target is surfaced as a cwd-relative `path:line` *in place of the label*, erasing the citation; `vscodium:` and `https:` keep the label and append the full URL in parentheses. Only the `https:` line is clickable, confirming row 3's OSC-8-for-web rule by observation rather than source-read. The general finding is the new part — **no target hides its destination behind the citation here** — so the property the link form exists to provide is unavailable on this surface, and an editor scheme buys a long inline URL with no click at all. `web` remains Codex's one natively clickable form (label plus visible URL); everything else is strictly worse than plain `path:line` (row 11), which is what the [§DF-conversation-link-target.2.4](DF-conversation-link-target.md#24-the-form-is-gated-per-agent-and-the-fallback-is-path) gate falls back to. No Codex configuration changes this: `desktop.custom_file_handlers` only selects the "Open in" target for files Codex already recognizes, and nothing exposed there enables arbitrary local Markdown links — so this row is a property of the surface, not a setup gap to be closed later |
| 16 | Pi TUI (assistant message) | Markdown link | file / editor / web | label stays the citation, destination hidden behind it | **depends on the terminal, not on Pi** (observed 2026-08-11). `pi-tui`'s Markdown renderer emits OSC 8 — hiding the URL behind the label — only when `detectCapabilities()` positively identifies a hyperlink-capable terminal, and otherwise falls back to printing `text (url)`. In the fallback all three labels survive and clicks come from the *terminal's own URL matcher*, so `file:` and `https:` work and no editor scheme does. With OSC 8 enabled, **all three work, `vscodium:` included** — the terminal hands the URI to the desktop handler and the scheme stops mattering. Measured in Ptyxis (VTE 0.78), which supports OSC 8 but sets no `TERM_PROGRAM`, so `detectCapabilities()` had no branch to match it and took the conservative default; adding one flipped the same session from fallback to OSC 8. `file` is therefore the one target that clicks under **both** outcomes, and an editor scheme is a bet on the reader's terminal |

## Recipe

The long form behind the two-sentence `AGENTS.md` instruction:

- **Shape**: `[§<ID>](<target>)` / `[§<ID>.<sec>](<target>#<anchor>)` — the same wrap
  `grund fmt --cross-refs` writes ([§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)); the cheapest correct move is to copy an
  existing wrap's target from a file already read and re-base it.
- **TUI messages**: follow the user-level instruction installed by `grund integrations --write`.
  Its default says to write the plain citation because the rendering layer resolves it (row 10);
  users without that layer may select the link override. Where a location must be explicit, the
  form is a Markdown link whose label is the bare citation and whose target is an **absolute URI**
  — `file:`, an editor scheme, or the forge URL, selected per machine
  ([§DF-conversation-link-target](DF-conversation-link-target.md#df-conversation-link-target-the-conversation-link-form-is-a-markdown-link-over-an-absolute-uri-addressed-per-machine)).
  Never a relative-path target (row 1), and never a local target on a surface that renders the
  destination in place of the label (row 3): the citation itself is the one part that must survive.
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
