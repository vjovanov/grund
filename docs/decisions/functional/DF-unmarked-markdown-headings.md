# DF-unmarked-markdown-headings: in-body Markdown ATX headings participate in the knowledge graph

**Status:** Accepted
**Date:** 2026-09-10

Making omitted section coordinates visible serves
[§GOAL-agent-grounding.1](../../goals.md#1-the-three-layers) and
[§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible).

## 1. Context

Grund already validates declarations, section coordinates, and the depth of a
heading that writes a coordinate. It nevertheless accepted an ordinary ATX
heading inside a Markdown declaration body without saying that the heading was
not addressable. A forgotten number and deliberately non-citable structure
therefore produced the same clean result.

The first report also exposed a separate scanner defect: a section-like heading
beyond a declaration body could remain in the prior declaration's section map.
Issue #225 and PR #226 corrected that invariant under
[§FS-check.3.23](../../functional-spec/FS-check.md#323-section-outside-a-declaration).
This decision begins at the body-local map that predecessor established and does
not reopen its `show --full` boundary.

## 2. Decision

Every non-declaration ATX heading deeper than a Markdown declaration heading and
still inside its body must carry a recognized numeric or enabled named section
coordinate. Before grund 0.15.0, [§FS-check.4.14](../../functional-spec/FS-check.md#414-unmarked-markdown-heading)
reports an unmarked heading as a fixed warning at the heading, names the nearest
enclosing declaration, and suggests a deterministic unused coordinate. In
0.15.0 the same project-wide finding becomes an error.

The body span is the policy boundary. Pre-declaration titles and
same-or-shallower headings that close a body remain legal. Fenced headings are
examples, and source doc-comments, setext text, and bold labels are not Markdown
ATX structure governed by this rule. A nested declaration owns headings in its
own overlapping body because the nearest enclosing declaration is the fact an
author is editing.

Severity remains fixed under [§GOAL-configurable.2](../../goals.md#2-what-is-not-configurable):
there is no `allow | warn | error` selector and no permanent opt-out. The
warning window follows
[§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path)
because grund ships no command that can choose an author's intended hierarchy.

## 3. Rejected alternatives

**Reject every ordinary heading in a grounded file.** This would make file
titles and body-closing chapter structure part of a declaration they do not
belong to. **Apply the rule to source doc-comments.** Language-native headings
such as Rustdoc `# Examples`, `# Errors`, and `# Panics` are documentation
structure rather than Markdown-file section declarations. **Make severity a
repository setting.** That would turn one project-wide graph invariant into a
per-repository verdict choice and preserve an indefinite escape from the
endpoint. **Number headings in `grund fmt`.** Choosing the next unused number is
mechanical; deciding the intended parent and whether the heading should instead
be a declaration or bold label is not.

## 4. Consequences

Repositories receive one release window to number or reclassify affected
headings before the 0.15.0 error. A warning suppresses the bare `success` line
but leaves exit `0`; text, JSON, selection, and LSP carry the same core finding.
The suggestion is stable guidance and never a write path. `show`, `list`,
`refs`, `cover`, formatting, section resolution, and source scanning keep their
existing behavior.

Agent guidance must teach the narrowed rule. Because the managed block is byte
compared, its existing v9 moves to v10 and `grund init` remains the one-command
block repair required by
[§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations).
No configuration key changes meaning, so `grund_config_version` stays 1. The
scheduled verdict change is tracked by
[§RM-unmarked-heading-error](../../roadmap.md#rm-unmarked-heading-error-make-unmarked-markdown-headings-errors-in-0150).
