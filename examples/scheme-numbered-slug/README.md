# Scheme: `{kind}-{number}-{slug}`

The default `grund` ID format. Numbers disambiguate; slugs describe.

This example teaches choosing between the supported ID schemes ([§FS-examples.2](../../docs/functional-spec/FS-examples.md#2-canonical-use-cases)); the `format` grammar and its three canonical shapes are [§FS-config.3.2](../../docs/functional-spec/FS-config.md#32-id--id-grammar).

```toml
# .agents/grund.toml — empty `[id]` block, defaults apply
```

Example IDs:

```
FS-001-login
FS-002-session
AR-014-event-bus
```

## Pros

- **Stable refs.** Renaming a spec's title rewrites the slug only — the number keeps every existing citation valid (with `grund fmt --marker` to refresh slugs in prose later).
- **Skimmable.** A reader sees both an identifier and a hint of what it's about (`§FS-014-event-bus` vs `§FS-014`).
- **No slug-uniqueness collisions.** Two specs with similar slugs are fine — different numbers separate them.

## Cons

- **Two facts to maintain.** When a title drifts, the slug grows stale until someone re-slugs it.
- **Longer.** Citations are wider in prose than the pure-numbered or pure-slug forms.
- **Cosmetic churn.** A title edit produces a slug change that reads like a semantic change in diffs even when the number (the real identity) is untouched.

## Verify

From the repo root:

```bash
grund examples/scheme-numbered-slug/repo
echo $?    # 0
```

Silent + exit 0 means every cross-citation resolved against the declared IDs.
