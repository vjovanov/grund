# DISC-link-support: Link support as a derived presentation layer

## Status

Resolved by [§DF-md-link-emission](../../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations) and [§DF-md-link-anchor-strategy](../../decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass). Implementation shipped as [§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission). The "Open questions" section below is preserved as-is for historical context; the answers live in the two DFs above.

## Context

Readers often want clickable navigation in rendered Markdown, while `grund`'s core
model is based on stable ID citations. A Markdown link is useful for a human in a
browser or IDE preview, but it is path-coupled and anchor-coupled. An ID citation
is location-independent and works across Markdown and source doc-comments.

The current direction in [§DF-md-link-emission](../../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations) and [§FS-fmt.6](../../functional-spec/FS-fmt.md#6-cross-reference-emission) is to keep IDs as the
source of truth and let `grund fmt --cross-refs` generate Markdown links around
marker-prefixed citations in `.md` files.

## Proposed shape

Keep this form canonical:

```text
§FS-fmt.6
```

Allow `fmt` to derive this rendered-Markdown convenience form:

```markdown
[§FS-fmt.6](../functional-spec/FS-fmt.md#...)
```

The link target should be regenerated from the ID graph, not edited by hand as
the authoritative reference. If a file moves or a heading changes, a later
`grund fmt --cross-refs --write` pass updates the generated URL.

## Boundaries

- `grund check` should continue to validate the underlying ID citation, not general
  Markdown links.
- General Markdown link validation remains out of scope for `grund` per
  [§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation); tools such as lychee remain better suited to `[text](url)`
  and HTTP validation.
- Source files should not be rewritten with Markdown link syntax. The universal
  form in source comments remains the marker-prefixed ID citation.

## Open questions

- Should repositories be able to opt into `--cross-refs` globally through config,
  or should it stay invocation-only until the formatter behavior is mature?
- Should generated links to source-hosted declarations point only at the file, or
  should `grund` eventually support best-effort line anchors where hosts support
  them?
- Should CI recommend both `grund fmt --cross-refs --check` and a separate lychee
  pass, or leave that composition entirely to the project?
