# Scheme: `{kind}-{slug}`

Pure-slug IDs. The `grund` repo itself uses this scheme.

This example teaches choosing between the supported ID schemes ([§FS-examples.2](../../docs/functional-spec/FS-examples.md#2-canonical-use-cases)); the `format` grammar and its three canonical shapes are [§FS-config.3.2](../../docs/functional-spec/FS-config.md#32-id--id-grammar).

```toml
[id]
format = "{kind}-{slug}"
slug_pattern = "[a-z][a-z0-9-]*"
```

Example IDs:

```
FS-login
FS-session
AR-event-bus
```

## Pros

- **Self-describing.** A reader sees a citation — the `§` marker followed by an ID like `FS-event-bus` — and immediately knows the topic, no lookup round-trip to triage a PR comment. (Illustrative IDs in this README carry no `§` marker on purpose, so `grund check examples/scheme-slug` stays clean — only a marker-prefixed token is a checked citation.)
- **No number bookkeeping.** Authors and agents pick a title; the ID falls out. There is no "next free number" to allocate, so two contributors creating specs in parallel never race for a number.
- **Greppable by topic.** `grep -r 'FS-event' src/` finds every citation in the area without a separate ID-to-topic lookup.

## Cons

- **Slug uniqueness is enforced.** Two specs about adjacent topics need distinct slugs (`FS-login`, `FS-login-mfa`) — there is no number to fall back on.
- **Renaming is destructive.** Reslugging a spec changes its ID, which breaks every existing citation. Either preserve the original slug after a title change, or deliberately rewrite cites.
- **Prefix discipline.** With `slug_pattern = "[a-z][a-z0-9-]*"`, slugs must start with a letter so illustrative tokens like `FS-001` in prose aren't accidentally picked up as citations.

## Verify

```bash
grund examples/scheme-slug/repo
echo $?    # 0
```
