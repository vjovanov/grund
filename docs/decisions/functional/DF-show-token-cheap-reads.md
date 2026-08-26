# DF-show-token-cheap-reads: grund show keeps the full-body default; token-cheap slices are opt-in

**Status:** Superseded
**Date:** 2026-05-12
**Superseded by:** [§DF-show-default-token-cheap](DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in) — the four-flag surface (`--head` / `--outline` / `--brief` / `--full`) introduced here proved harder to learn than expected; the default flips to the cheap read and the slice flags are renamed around an incremental ladder. The "structural, not generated" and "`check` is not abridged" properties (§2.2, §2.3) carry over unchanged.

## 1. Context

`grund show` exists so an agent can pull one grounded fact into context without loading the whole file ([§FS-show](../../functional-spec/FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id), [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible)). It already has `--head` (the lead "what is this about" prose) and section selection (`grund show <ID>.<section>`), and `grund refs` / `grund list` round out the read surface. But on a real repo the *cheapest* path is easy for an agent to miss: when a citation is a bare `§<ID>` with no section, the obvious move is `grund show <ID>` — the full body, ~15 KB for a large spec — when what the agent actually needs is "is this the right declaration, and which section do I want?", which is a fraction of that. `grund refs <ID>` on a heavily-cited ID repeats the same file path dozens of times; `grund list` dumps the entire catalog when the agent wanted only `FS` and `AR`. The retrieval primitives are correct; the token economy isn't, and the generated `AGENTS.md` guidance ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)) still names `grund show <ID>` as the first read in places. This decision records the shape of the fix and what it deliberately does *not* change. Background and measurements: [§DISC-token-cheap-grounding](../../discussions/proposals/2026-05-12-token-cheap-grounding.md#disc-token-cheap-grounding-token-cheap-grounding-surfaces).

## 2. Decision

Add token-cheap read modes as **opt-in flags over the existing scanner**, and leave every default unchanged:

- `grund show <ID> --outline` — only the declaration's numbered section headings ([§FS-show.2.1.2](../../functional-spec/FS-show.md#212-section-map---toc)): the map an agent reads before choosing a section.
- `grund show <ID> --brief` — `--head` then `--outline` ([§FS-show.2.1.3](../../functional-spec/FS-show.md#213-full-body---full)): the recommended first read for a bare cited `§<ID>`.
- `grund refs <ID> --summary` — one line per citing file with a count and line list ([§FS-refs.3.3](../../functional-spec/FS-refs.md#33---summary)).
- `grund list --kind FS,AR` (multi-value `--kind`) and `grund list --summary` (one line per kind, with counts and homes) ([§FS-list.3.3](../../functional-spec/FS-list.md#33---summary)).
- The generated `AGENTS.md` block teaches the ladder — `show <ID> --brief` for a bare ID, `show <ID>.<section>` for a section citation, `show <ID>` only as the escalation — which is a managed-block version bump under [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)).

### 2.1 `grund show <ID>` default is not changed

A bare `grund show <ID>` keeps printing the full body, exactly as it does today. The slices are strictly additive flags; a caller that wants the whole declaration types nothing new. Changing the default would be a user-visible output change under [§GOAL-no-silent-breakage.1](../../goals.md#1-what-counts-as-user-visible), and the full body is the right answer for a human reading a spec on the terminal — the token pressure is an *agent* concern, met by giving the agent a cheaper opt-in, not by degrading the human path.

### 2.2 Slices are structural, never generated

`--outline`, `--brief`, `--summary` are verbatim subsets of bytes `grund` already parsed — heading lines, lead prose, per-file citation counts. No model summarization, no paraphrase. This keeps every output byte-deterministic on `(tree, config)` like the rest of `grund` ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree), [§FS-non-goals.14](../../functional-spec/FS-non-goals.md#14-generated-summaries-token-saving-inside-check)).

### 2.3 `check` is not abridged

The token economy applies to the read/query surface only. `grund check` diagnostics stay complete — every dangling reference, every warning, in full located form. A checker that hid findings to save tokens would defeat the point ([§FS-non-goals.14](../../functional-spec/FS-non-goals.md#14-generated-summaries-token-saving-inside-check)).

## 3. Why this fits grund's goals

- [§GOAL-token-economy](../../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) / [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — grounding should be *cheap*; a 1 KB first read that still names the section to fetch is friendlier to an agent's context budget than a 15 KB body, and the human path is untouched. This is the read ladder that goal asks for.
- [§GRUND-grund](../../grund.md#grund-grund-agents-stay-grounded-in-the-spec) — an agent more likely to ground itself before editing is one for which grounding costs less; the cheaper the brief, the fewer overfetches and skips.
- [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — additive flags break nothing; the `AGENTS.md` guidance change rides the existing managed-block version mechanism.
- [§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree) — structural slices stay byte-deterministic; a generative summarizer would not, so it is out of scope.

## 4. Consequences

- New [§FS-show.2.1.2](../../functional-spec/FS-show.md#212-section-map---toc) / [§FS-show.2.1.3](../../functional-spec/FS-show.md#213-full-body---full) and the `--head | --outline | --brief | --full` mutual-exclusion group in [§FS-show.1](../../functional-spec/FS-show.md#1-inputs); the format-variant bullets in [§FS-show.3.1](../../functional-spec/FS-show.md#31-format-variants) gain `sections` / `head` JSON fields.
- New [§FS-refs.3.3](../../functional-spec/FS-refs.md#33---summary) (`--summary`) and [§FS-list.3.3](../../functional-spec/FS-list.md#33---summary) (`--summary`, multi-value `--kind`).
- [§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints) and [§FS-init.2.3.3](../../functional-spec/FS-init.md#233-citation-form) describe the brief/section-first workflow; landing it bumps the `AGENTS.md` managed-block version and refreshes `templates/AGENTS.md` and the `init-agents-*` e2e fixtures.
- New [§FS-non-goals.14](../../functional-spec/FS-non-goals.md#14-generated-summaries-token-saving-inside-check) records the "no generated summaries, no abridged `check`" boundary.
- E2E fixtures cover `show --outline`, `show --brief`, `refs --summary`, `list --kind FS,AR`, `list --summary` in text and JSON, plus the bumped `init-agents-*` block.

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Change `grund show <ID>` to print `--brief` by default, `--full` for the body | A user-visible output change for everyone, including humans on the terminal who want the body; the friction is an agent-context concern, not a reason to degrade the default. The opt-in flag costs an agent two tokens. |
| One generated "summary" mode (an LLM-written précis of the declaration) | Defeats `grund`'s core property — same input, same answer everywhere. A précis is non-deterministic and a maintenance/attack surface; the heading tree the author already wrote is the structure worth exposing. |
| A separate `grund brief` / `grund outline` subcommand | These are slices of `show`'s output, not new queries; a flag keeps the surface small and the relationship obvious. Same for `refs --summary` / `list --summary` vs. new subcommands. |
| Also trim `grund check` output for agents | A check that hides findings to save tokens is worse than a verbose one — the whole value is that nothing dangles silently. Token-saving stays on the read/query side. |
