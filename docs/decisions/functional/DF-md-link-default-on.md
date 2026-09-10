# DF-md-link-default-on: Markdown cross-reference links default on for GitHub review and discovery

**Status:** Accepted
**Date:** 2026-05-19
**Authors:** kimeta, vjovanov

## 1. Context

`grund fmt` can wrap marker-prefixed Markdown citations as links: `§<ID>.1` becomes `[§<ID>.1](<path>#1-inputs)`. The wrapped form is noisier in a plain-text editor: lines are longer, the URL is visible in source, and editing prose around a citation has more syntax in the way. The bare citation is the cleaner authoring form.

But a large share of spec reading does not happen in the author's editor. It happens in the GitHub Web UI: code review, PR discussion, browsing from an issue, external contributors reading a linked spec, and reviewers who need to jump from a claim to the grounded requirement without cloning the repo or running `grund show`. In those contexts, a bare `§ID` is discoverable only to people who already know the tool and have a local checkout. A Markdown link is directly useful to every GitHub reader.

## 2. Decision

Default generated configs to:

```toml
[fmt.cross_refs]
enabled = true
anchor_format = "github"
```

`grund fmt --write` therefore emits and re-derives Markdown links in `.md` files by default. Repos can opt out with `enabled = false`, and `--cross-refs` remains a one-run override.

The default favors GitHub review and discovery over the cleaner editor-only source view. This is a deliberate trade: the source stays readable because the visible citation text is unchanged, and `grund fmt` owns the generated URL. The Web UI gains immediate navigation for reviewers and external readers, which is the higher-value default for a documentation and review surface.

## 3. Boundaries

- The Markdown URL is still derived presentation, not the source of truth. `grund check`, `grund show`, and `grund refs` resolve the citation text inside the brackets.
- Source files are not rewritten into Markdown links. The polyglot citation grammar remains `§ID` in comments and doc-comments.
- Source-only `fmt --write` scopes do not run the default link-target scan; no Markdown file in that scope can be wrapped.
- Repos with a non-GitHub renderer should keep links enabled and set the matching `anchor_format` when possible. Repos that intentionally optimize for plain Markdown source can set `enabled = false`.

## 4. Consequences

- The generated `grund.toml` is more opinionated but also more explicit: the opt-out key is visible where users configure the repo ([§DF-config-file-location.2.3](DF-config-file-location.md#23-grund-init-writes-the-bare-grundtoml)).
- PR diffs in Markdown files may include generated link wrappers, but those wrappers make review-time navigation substantially better.
- The editor experience sacrifices some source minimalism. That cost is bounded by idempotent formatting, and `grund show --format text` still flattens link wrappers when an agent or human wants token-cheap grounding text.

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Keep links opt-in | Preserves the cleanest editor source, but leaves the common GitHub review path non-clickable unless each repo discovers and enables the setting. |
| Default links on only for `grund` itself | Solves this repo's docs but fails the generated-config goal: new adopters should get the review-friendly default without copying a local convention. |
| Remove the opt-out | Too rigid for repos with unusually high Markdown churn or source-first authoring workflows. |
