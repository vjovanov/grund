# DISC-token-cheap-grounding: Token-cheap grounding surfaces

## Status

Concluded. Implementation shipped as [§DF-show-default-token-cheap](../../decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in).

## Context

`grund show` already prevents the worst failure mode: an agent can fetch one declaration by ID instead of opening a whole file ([§FS-show](../../functional-spec/FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id)). But the full-declaration default is still expensive for large specs when an agent only needs to decide which section to read.

Measured on this repo:

| Query | Output |
|---|---:|
| `grund show FS-check` | 15.5 KB / 2,375 words |
| `grund show FS-check --head` | 235 B / 32 words |
| `grund show FS-check.3.6` | 1.6 KB / 235 words |
| `grund list` | 23.3 KB / 1,269 words |
| `grund list --kind FS` | 1.6 KB / 130 words |
| `grund refs FS-check` | 6.1 KB / 298 words |

The pattern is clear: precise section reads are cheap; discovery and broad reads are where tokens leak. This matters because the generated `AGENTS.md` guidance currently teaches the right primitives but still names `grund show <ID>` as the first read in several places. The cheaper first move should be: head or outline first, then a targeted section.

## Proposed shape

Add a small "token-cheap read" layer over the existing scanner and section table:

- `grund show <ID> --outline` prints only the declaration's numbered section headings, one per line. This is the map an agent needs before choosing `ID.section`.
- `grund show <ID> --brief` prints the existing `--head` output plus the outline. This becomes the agent-first read for a bare cited ID.
- `grund refs <ID> --summary` groups references by file, with either counts or line lists, so a change-impact scan does not repeat the same path dozens of times. This builds on [§FS-refs](../../functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id).
- `grund list` gets a compact discovery path for agents, either `--summary`, multi-kind filtering, or both. This builds on [§FS-list](../../functional-spec/FS-list.md#fs-list-grund-lists-every-declared-id).
- The generated `AGENTS.md` block from [§FS-init](../../functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) should prefer `grund show <ID> --brief` for bare IDs, `grund show <ID>.<section>` for cited sections, and `grund show <ID> --full` only when the narrower output is insufficient.

These outputs should remain deterministic, line-oriented, and easy to pipe, following [§FS-errors.4](../../functional-spec/FS-errors.md#4-determinism).

## Boundaries

- Do not change `grund show <ID>` default behavior in the first pass. Full-body output is already specified and useful for humans; changing it is a compatibility decision under [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path).
- Do not add summarization by language model. These are structural slices from the declaration headings and citation graph, not generated prose.
- Do not hide information in `check`. Diagnostics stay complete; token-saving applies to read/query commands.

## Open questions

- Should `--brief` include the declaration heading in text format, or stay consistent with `show` text output and omit the H1?
- Should `refs --summary` show counts only, line lists, or both behind separate flags?
- Should `list --summary` group by kind with counts only, or also include each kind's configured home?
- Should generated `AGENTS.md` recommend `--brief` only for bare IDs, or make it the universal first step before any full-body read?
