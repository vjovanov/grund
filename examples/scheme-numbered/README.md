# Scheme: `{kind}-{number}`

Pure-numeric IDs. Familiar to anyone who has read an RFC or a JEP.

This example teaches choosing between the supported ID schemes ([§FS-examples.2](../../docs/functional-spec/FS-examples.md#2-canonical-use-cases)); the `format` grammar and its three canonical shapes are [§FS-config.3.2](../../docs/functional-spec/FS-config.md#32-id--id-grammar).

```toml
[id]
format = "{kind}-{number}"
```

Example IDs:

```
RFC-001
FS-002
AR-014
```

## Pros

- **Shortest possible ID.** Citations stay tight in prose: `§RFC-001`, `§FS-002.1`.
- **Title-edit safe.** A spec's heading can be reworded indefinitely without disturbing any existing citation — the ID has no descriptive payload to drift.
- **Easy to allocate.** `grund id FS "..."` just emits the next free number; no slug derivation, no collision check on the descriptive part.
- **Familiar.** Reviewers already trained on RFC-/JEP-/PEP-style identifiers feel at home.

## Cons

- **Opaque in prose.** A reader skimming code or a PR sees `§FS-042` and has no idea what the claim is about until they resolve it. This punishes drive-by review.
- **Memory load.** Maintainers learn the catalog by number; new contributors don't have that map.
- **Search friction.** Grepping `§FS-` finds every cite uniformly, with no descriptive hint to triage.

## Verify

```bash
grund examples/scheme-numbered/repo
echo $?    # 0
```
