# DF-index-always-linkified: the cross-reference pass always runs on a kind's index file

**Status:** Accepted
**Date:** 2026-08-25

## 1. Context

[§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link) makes a bare index entry an error in the release it arrives, under the [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) licence for a migration one shipped command completes. The command is `grund fmt --write`, which runs the cross-reference pass by itself on any scope holding Markdown ([§FS-fmt.6.6](../../functional-spec/FS-fmt.md#66-why-generated-configs-enable-cross-references)).

That claim has one hole. `[fmt.cross_refs] enabled = false` ([§FS-fmt.6.7](../../functional-spec/FS-fmt.md#67-configurability)) turns the pass off, and in a repository that set it the fix becomes `grund fmt --cross-refs --write` — a command that linkifies the entire tree to repair one file, in a repository whose maintainers said in their config that they do not want generated links. The [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) licence weakens with it: "one documented command the tool ships" is not the same promise when the command also rewrites every other document.

## 2. Decision

### 2.1 A kind's index file is linkified regardless of `[fmt.cross_refs] enabled`

The index a `[[kinds]]` entry names ([§FS-config.3.4](../../functional-spec/FS-config.md#34-kinds--recognized-prefixes)) is a region the cross-reference pass always runs on ([§FS-fmt.6.1](../../functional-spec/FS-fmt.md#61-scope)). It is the mirror image of [§FS-fmt.2.3](../../functional-spec/FS-fmt.md#23-what-is-never-rewritten)'s never-rewrite zones — one region `fmt` always writes, rather than one it never does — and it is what makes "the entry is a full link, unconditionally" cost nothing anywhere: the fix for [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link) is plain `grund fmt --write` in every configuration.

### 2.2 It follows the pass's own gate, and reaches the entries rather than the page

The carve-out decides *which citations* the pass touches when it runs, never *whether* it runs. A scope that would run no cross-reference pass at all — `grund fmt --check` without `--cross-refs`, which does not predict the automatic pass today — still runs none.

Within the index file it is narrower still: the citations wrapped are the ones the index owes an entry for ([§FS-check.4.6](../../functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index)), and a citation of a foreign ID in the prose around the list is left bare. The size of the carve-out is argued from what [§FS-check.3.17](../../functional-spec/FS-check.md#317-index-entry-is-not-a-link) demands, and it demands exactly one link per covered ID; a repository that set `enabled = false` gets the smallest write that clears the error and nothing beyond it. Writing the whole page would still have been defensible — the file is the unit `fmt` reasons in — but it would answer a question nobody asked, in the one configuration whose maintainers were explicit about not wanting generated links.

The narrowing has one visible consequence: an *existing* wrap around a non-entry citation in an index file is no longer re-derived under `enabled = false` ([§FS-fmt.6.3](../../functional-spec/FS-fmt.md#63-idempotency-and-re-derive)). That is the same answer the rest of that repository already gets for a wrap it hand-wrote, and the entries — the links the rule is about — are re-derived as always.

### 2.3 The two carve-outs are the same shape

The unused-declaration accounting needs the index excluded ([§DF-index-not-an-inbound-citation](DF-index-not-an-inbound-citation.md#df-index-not-an-inbound-citation-an-index-entry-is-navigation-not-use)) and the formatter needs it included; both are the checker and the formatter being told that this one file has a job, and both are stated against the same `[[kinds]] index` key rather than against a path convention. A repository that opts out with `index = false` opts out of both at once, which is the property that makes the pair reviewable.

## 3. Alternatives considered

**Make the requirement conditional on `enabled`.** An index whose entries are links only where the config happens to allow generated links is an index whose contract a reader cannot state. The point of the entry is that a reader can follow it; a renderer preference is not a reason to withdraw that.

**Name `grund fmt --cross-refs --write` as the fix and accept the blast radius.** It repairs one file by rewriting the tree, and in the one configuration where it is needed it is the tree the maintainers deliberately left alone. That is not a migration a release should ask for.

**Leave the link half a warning under `enabled = false`.** A finding whose severity depends on an unrelated config key is a second severity model in disguise, and [§FS-config.6](../../functional-spec/FS-config.md#6-what-is-not-configured-here) keeps the severity set frozen at two for exactly that reason.
