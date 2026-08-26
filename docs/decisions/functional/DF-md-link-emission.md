# DF-md-link-emission: grund fmt may emit clickable Markdown links alongside §-prefixed citations

**Status:** Accepted
**Date:** 2026-05-09

## 1. Context

The grund reference scheme cites IDs in the form `§<KIND>-<slug>.<section>`. In rendered Markdown (GitHub, MkDocs, an IDE preview), these citations are not clickable — they are plain text. A reader scanning a doc has to mentally resolve the citation to a path, navigate there, and scroll to the section. `grund show <ID>` answers the question on the command line, but a reader already inside the rendered doc wants the link there too.

Off-the-shelf Markdown links solve the click problem but lose every other property an ID has — they are path-coupled (renames break them), anchor-coupled (heading rewrites break them), and they do not work in source comments (`§GOAL-polyglot-citation`). The whole point of `grund` is that none of those losses are necessary.

We want to keep IDs as the source of truth and *also* deliver clickable links in rendered Markdown — without the losses, and without the user maintaining two forms by hand.

## 2. Decision

Extend `grund fmt` (per [§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission)) with Markdown cross-reference emission that, in `.md` files only, wraps each marker-prefixed citation in a Markdown link to the declaration body. The unwrapped citation remains the canonical, source-of-truth form; the wrap is a derived presentation layer that `fmt` regenerates idempotently.

### 2.1 Form

Wrap-the-citation. Given an illustrative citation, the rewrite is:

```text
before:  §FS-foo.3.1
after:   [§FS-foo.3.1](../functional-spec/FS-foo.md#3-1)
```

The citation text inside the brackets is preserved verbatim — a reader sees the same characters as before, only now they form a link.

Why wrap, and not the alternatives we considered:

| Form | Why rejected |
|---|---|
| Parenthetical link — citation text plus a sidecar arrow link | Two artifacts per citation; visually noisy at scale; the arrow becomes meaningless to readers who do not learn the convention. |
| Reference-style — `[citation][label]` with footnote definitions collected at file bottom | Splits the citation across the file; harder to grep; reference-definition collection at file bottom drifts as citations move. |
| Auto-link only without text — the bare URL form `<path#anchor>` | Loses the human-readable citation form; readers can no longer skim a paragraph and identify the citation by its ID. |

Wrap-the-citation keeps one artifact per citation, keeps the cite human-readable, and produces clean diffs (only the brackets and parenthetical change).

### 2.2 Anchor format

**Superseded by [§DF-md-link-anchor-strategy](DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass).** This DR's first draft proposed a section-coordinate anchor (`.3.1` → `#3-1`) on a "stability under heading edits" argument. On review the proposed format proved factually wrong about renderer behavior — GitHub's slugger strips punctuation rather than converting it, so `### 3.1 Inputs` produces `#31-inputs`, not `#3-1`. [§DF-md-link-anchor-strategy](DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass) picks the actual strategy: heading-text slugs per a configurable renderer profile, re-derived on every `grund fmt` pass. The "stability" framing is retracted there in favor of a normalize-on-each-run rule that matches the existing trigger→marker pass.

### 2.3 Source-file links

When the citation's declaration lives in source code (a stub of the form `# <ID>: [<src/path>](<src/path>)` points the ID at a source file), the wrapped link targets the source file with no anchor — e.g., `[§<ID>](../../src/path.rs)`. The host renderer will not jump inside a Rustdoc, but the reader lands on the right file. This is the best available answer until renderers learn doc-comment fragments — the alternative (skip emission entirely) would punish the very polyglot case the project is built around.

### 2.4 Default-on for generated configs

`[fmt.cross_refs] enabled = true` is the default and is emitted by generated `.agents/grund.toml` files. That means `grund fmt --write` emits Markdown links in `.md` files without requiring `--cross-refs`; `enabled = false` opts a repo out, and `--cross-refs` still forces the pass for a single invocation. The default changed because rendered Markdown is the common reading surface, especially for GitHub code review and external discovery, and the wrap is now a normal derived artifact kept fresh by `fmt`, not something users should maintain by hand ([§FS-fmt.6.6](../../functional-spec/FS-fmt.md#66-why-generated-configs-enable-cross-references), [§DF-md-link-default-on](DF-md-link-default-on.md#df-md-link-default-on-markdown-cross-reference-links-default-on-for-github-review-and-discovery)).

This does not make Markdown links canonical. The citation text inside the brackets is still the source of truth; `grund check` validates the citation, `grund show` resolves the ID, and `grund fmt` re-derives the URL. Projects with path-churn or a non-rendered Markdown workflow can set `enabled = false`; projects with a non-GitHub renderer choose the matching `anchor_format`.

## 3. Reconciliation with non-goals

This decision sits close to two [§FS-non-goals](../../functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) entries; both stand and this is consistent with them.

### 3.1 [§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation) — grund does not validate Markdown links

`fmt --cross-refs` *emits* a link; it does not *validate* a link. Once the link is on disk it is a normal Markdown link and `lychee` is the right tool to validate it. `grund check` continues to validate only the underlying citation — the part inside the brackets — same as before. If a contributor hand-edits the URL inside `(...)` to point somewhere wrong, `grund` will not catch it; that is `lychee`'s job, exactly as [§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation) says.

### 3.2 [§FS-non-goals.5](../../functional-spec/FS-non-goals.md#5-documentation-generation) — grund does not generate rendered documentation

The link emission is a transformation on the source `.md` file (a sibling of the existing trigger→marker rewrite in [§FS-fmt.2.1](../../functional-spec/FS-fmt.md#21-trigger-to-marker)), not a render to a different format. The output is still `.md`; the file tree is unchanged in shape; no HTML, PDF, or static site is produced. A reader who never opens a renderer sees the same citation as before, now wrapped in link syntax. [§FS-non-goals.5](../../functional-spec/FS-non-goals.md#5-documentation-generation) prevents `grund` from owning the publishing pipeline; this decision keeps `grund` upstream of any such pipeline.

## 4. Consequences

- A new `--cross-refs` flag on `grund fmt` and a `[fmt.cross_refs]` config block in `grund.toml` ([§FS-fmt.6.7](../../functional-spec/FS-fmt.md#67-configurability)).
- The implementation shipped as [§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission).
- The [§GOAL-polyglot-citation](../../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) goal explicitly states that the polyglot grammar is the canonical form; this decision is the sanctioned exception that adds a presentation-layer view in `.md` only.
- Repos that keep the default should run `grund fmt --write` as a pre-commit hook so the generated links stay in sync with file moves and citation edits. CI that wants to flag link drift without writing should run `grund fmt --cross-refs --check`, because `enabled = true` auto-runs the link pass only for `--write`.
- The [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) path: generated configs write `enabled = true` explicitly so new repos see and can edit the default. Existing repos that do not want Markdown links can pin `enabled = false`; the flag remains an explicit one-shot opt-in.

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Markdown links as the source of truth (delete the ID grammar) | Loses the polyglot property entirely — the reason `grund` exists per [§GOAL-polyglot-citation](../../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) and [§GRUND-grund](../../grund.md#grund-grund-agents-stay-grounded-in-the-spec). Source comments cannot host clickable Markdown. |
| Render IDs to links at publish time, in a downstream tool | Pushes the work to every consumer — MkDocs plugin, GitHub Action, IDE preview — and gives each one a chance to disagree on the link target. Doing it once in `grund fmt` keeps two installs in agreement ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). |
| Always emit links with no config knob | Leaves path-churn-heavy repos no escape hatch. The chosen default keeps links on for generated/new configs but preserves `enabled = false` as a visible opt-out. |
| Heading-text slugs for anchors | Brittle under heading edits; would force `fmt` to rewrite anchors whenever prose changes; runs counter to "IDs survive refactors" — the property this project exists to defend. |
