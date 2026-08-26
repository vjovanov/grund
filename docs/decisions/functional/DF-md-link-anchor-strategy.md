# DF-md-link-anchor-strategy: heading-text slugs, re-derived on every fmt pass

**Status:** Accepted
**Date:** 2026-05-09

## 1. Context

[§DF-md-link-emission](DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations) decided that `grund fmt --cross-refs` wraps citations in clickable Markdown links inside `.md` files. That decision left [§DF-md-link-emission.2.2](DF-md-link-emission.md#22-anchor-format) with a placeholder anchor format — section-coordinate slugs of the shape `#3-1` derived from `.3.1` — and an "idempotency" rule ([§FS-fmt.6.3](../../functional-spec/FS-fmt.md#63-idempotency-and-re-derive)) that told `fmt` to leave existing wrap URLs alone once written.

Both placeholders are wrong on review. The section-coordinate slug does not match what standard Markdown renderers actually produce: GitHub's slugger strips punctuation rather than converting it (`### 3.1 Inputs` → `#31-inputs`, not `#3-1`); Pandoc's `auto_identifiers` algorithm differs further; MkDocs' TOC extension differs again. A repo emitting `#3-1` would render dead anchors in every renderer in common use. The "leave URLs alone" rule, in turn, lets the wrap go stale silently — a heading edit or a file move would invalidate every wrap pointing at it, and `fmt` would never repair them.

This DR picks the anchor strategy and tightens the idempotency rule.

## 2. Decision

### 2.1 Strategy

**Heading-text slugs**, derived per a configurable renderer profile, **re-derived on every `grund fmt --cross-refs` pass**.

`grund fmt --cross-refs` uses the target section heading text when deriving anchor fragments ([§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)). When it emits a wrap, it looks up the heading text for the target section and slugifies it using the configured renderer profile. With the default `github` profile, a citation `§FS-fmt.6.2` (heading `### 6.2 Form`) becomes:

```text
[§FS-fmt.6.2](../functional-spec/FS-fmt.md#62-form)
```

The slug `#62-form` matches what GitHub actually produces — the anchor is clickable in the rendered doc.

### 2.2 Re-derive on every pass, supersede [§FS-fmt.6.3](../../functional-spec/FS-fmt.md#63-idempotency-and-re-derive)

Every `grund fmt --cross-refs` invocation recomputes the canonical URL inside each existing wrap and rewrites if it differs. The pass remains idempotent — a second run with no intervening edits is a no-op, because the URL on disk is now equal to the canonical URL — but the rule shifts from "preserve what is there" to "make what is there canonical." This is the same property the existing trigger→marker pass ([§FS-fmt.2.1](../../functional-spec/FS-fmt.md#21-trigger-to-marker)) already has: `fmt` is a normalizer, and normalizers do not preserve drift.

The consequence: heading edits and file moves that invalidate a wrap produce a one-line `fmt` diff on the next pass, instead of a silently-broken link. With `fmt --check` in CI and a pre-commit hook ([§FS-fmt.4](../../functional-spec/FS-fmt.md#4-why-this-exists)), the window between drift and re-derive is bounded by one commit.

### 2.3 Renderer profiles

`[fmt.cross_refs] anchor_format` ships with named profiles from day one:

- `github` (default) — GitHub's slugger: lowercase, strip punctuation, replace whitespace runs with `-`, collapse consecutive `-`. Covers GitHub, the most common host.
- `gitlab` — GitLab's slugger (similar to GitHub with minor Unicode-handling differences).
- `mkdocs` — MkDocs / Python-Markdown TOC extension's slugger.
- `pandoc` — Pandoc's `auto_identifiers` algorithm.
- `none` — no anchor; emit a file-level link with no fragment. Reader lands at the top of the target file. This is the same behavior [§DF-md-link-emission.2.3](DF-md-link-emission.md#23-source-file-links) already specifies for source-file declarations.

A repo using a renderer with no matching profile selects `none` until a profile is added. Adding a new profile is a small contribution behind a focused e2e fixture.

**Corrected by [§DF-github-anchor-fidelity](DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly).** The `github` (and `gitlab`) bullet above says "replace whitespace runs with `-`, collapse consecutive `-`" — that is wrong about `github-slugger`. GitHub deletes each disallowed character *in place* (so its neighbours close up) and turns *each* remaining space into one `-`; it does not collapse runs or trim trailing `-` (`## A — B` → `#a--b`). The decision to ship named profiles stands; [§DF-github-anchor-fidelity](DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly) fixes the algorithm those two profiles use to match `github-slugger` exactly.

## 3. Why this fits grund's goals

[§GRUND-grund.1](../../grund.md#1-what-grund-does-about-it) names three pillars — verify, refactor-safe, extract — and explicitly positions Markdown links as *not* a pillar: *"Markdown links cover navigation in rendered docs. The three above are the load-bearing ones."* `--cross-refs` is a peripheral convenience layer over the canonical citation grammar, not a load-bearing feature. That positioning is the test this decision is graded against.

- **[§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration).** Untouched. Wraps are emitted from validated citations; the citation form §grund checks is unchanged.
- **[§GOAL-polyglot-citation](../../goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful).** Untouched. The `§<KIND>-<slug>.<section>` grammar remains the canonical, source-of-truth form across `.md` and every supported source-comment host. Wrap is a presentation layer over `.md` only.
- **[§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible).** `fmt` is not on the keystroke path (`check` is). The added scanner work is one extra string per declaration; the slugifier is a per-emission pure function.
- **[§GOAL-zero-config](../../goals.md#goal-zero-config-works-on-any-conformant-tree).** Cross-reference links are default-on for generated configs ([§DF-md-link-emission.2.4](DF-md-link-emission.md#24-default-on-for-generated-configs)); the `github` default fits the most common hosting case, so the default works out-of-the-box for the majority.
- **[§GOAL-friendliness-first.1](../../goals.md#1-hard-requirements)'s "no surprises" bullet (no surprises).** Same input + same config → same output bytes. No mutation of headings, no HTML injected into source `.md`. A reader scanning the source sees the same characters they wrote, only wrapped in `[…](…)`.
- **[§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable).** Renderer profiles are first-class and named.
- **[§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path).** Holds *conditional on running `fmt`* — the same condition that already governs every other `fmt`-managed normalization in the project. Heading edits and file moves produce a one-line diff on the next `fmt` pass, not silent breakage.
- **[§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation) (no link validation).** `fmt` emits; it does not validate. The emitted URL is correct-by-construction (grund computed it), but `grund check` continues to validate only the citation inside the brackets, exactly as [§DF-md-link-emission.3.1](DF-md-link-emission.md#31-fs-non-goals1--grund-does-not-validate-markdown-links) says.
- **[§FS-non-goals.5](../../functional-spec/FS-non-goals.md#5-documentation-generation) (no rendered docs).** Output remains `.md`; no HTML is injected anywhere; the file tree is unchanged in shape.
- **[§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree) (two installs agree).** Same input + same config → byte-identical output. A renderer-side mismatch between `github` and `pandoc` profiles is not a disagreement between two correctly-configured installs — it is the project asking the user which profile is correct.

## 4. Consequences

- [§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)'s "Anchor format" bullet is rewritten to reference this DR and the heading-text strategy.
- [§FS-fmt.6.3](../../functional-spec/FS-fmt.md#63-idempotency-and-re-derive)'s idempotency rule changes from "leave wrapped URLs alone" to "recompute and rewrite if different." Idempotency itself holds — second-run-with-no-edits is a no-op.
- [§FS-fmt.6.7](../../functional-spec/FS-fmt.md#67-configurability)'s `anchor_format` config gains the named-profile shape from §2.3 above.
- [§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)'s anchor form depends on recording heading text per section, in addition to the existing section path.
- [§DF-md-link-emission.2.2](DF-md-link-emission.md#22-anchor-format) is superseded by this DR. The section-coord stability framing in that section is retracted.
- The implementation ([§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission)) grows by one item: implement the renderer-profile slugifiers and the heading-text storage in `Findings`.

## 5. Alternatives considered

The four anchor strategies surveyed before this decision:

| Approach | Why rejected (or how folded in) |
|---|---|
| **(b) Anchor injection.** `grund fmt` rewrites every section heading to embed an explicit `<a id="6-2"></a>` tag, then wraps cite as `#6-2`. Renderer-portable; immune to heading-text edits. | Two costs the project will not absorb for a peripheral convenience feature. (1) grund would write literal HTML into source `.md` headings, which sits uncomfortably close to [§FS-non-goals.5](../../functional-spec/FS-non-goals.md#5-documentation-generation) ("does not generate rendered documentation") even on the strict reading; the optics of "grund is editing my headings to add HTML" undermine the trust relationship. (2) [§GOAL-friendliness-first.1](../../goals.md#1-hard-requirements)'s "no surprises" bullet — opting in rewrites every heading in `docs/`, surprising the user with their own diff. The technically strongest answer; too invasive for what `--cross-refs` is. |
| **(c) Per-renderer format with no default.** User must pick a profile before `--cross-refs` works. | Not actually a separate option — it is how (a) gets shipped configurably. Without specifying the underlying slug strategy, this is a deferral, not a decision. Folded into (a) as the renderer-profile config in §2.3. |
| **(d) No anchors, file-only links.** Every wrap targets the file with no fragment; reader scrolls to the section. Renderer-universal, trivial implementation, contractually cleanest. | Delivers minimal value over what `grund show <ID>.<section>` already provides at the CLI: the link takes you to the file, not the section. If the peripheral convenience adds nothing beyond `show`, the spec, the flag, the config block, and the e2e fixtures are not justified. Retained as the `none` profile in (a)'s config — a sane fallback when no renderer profile fits. |

The clinching argument for (a) over (b): [§GRUND-grund](../../grund.md#grund-grund-agents-stay-grounded-in-the-spec) frames `--cross-refs` as a "free convenience layer" — peripheral, not load-bearing. Peripheral features should not push on [§FS-non-goals](../../functional-spec/FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) or surprise users in source markdown. (a) keeps grund's machinery invisible in the source form (no HTML injection), accepts a brittleness that the project's own contract (run `fmt`) makes a non-event, and delivers section-level navigation in the renderer. (d) is contractually safer but the link's value collapses to "open the file," which is too thin to ship.
