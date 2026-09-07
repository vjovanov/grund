# External facts

External facts are ordinary declarations saved in the repository. Grund reads
those committed bytes offline; only an explicit `grund fetch <ID>` runs the
integration configured by the repository ([§FS-fetch](../functional-spec/FS-fetch.md#fs-fetch-grund-materializes-one-external-fact-snapshot)).

Configure a kind with its provider's ID shape, one snapshot home, and one direct
executable:

```toml
[[kinds]]
kind = "TICKET"
file = "docs/tickets.md"
title = "External tickets (generated snapshots)"
format = "{kind}-{number}"
resolve = "should"
fetch = "scripts/fetch-ticket"
```

The executable receives exactly one argument, the local ID, and prints one
complete Markdown declaration to stdout:

```sh
#!/bin/sh
title=$(gh issue view "${1#TICKET-}" --json title --jq .title) || exit
body=$(gh issue view "${1#TICKET-}" --json body --jq .body) || exit
printf '## %s: %s\n\n%s\n' "$1" "$title" "$body"
```

For a `file` home the declaration is H2; for a `folder` home it is H1. Grund
validates the complete output before atomically replacing only that ID's
snapshot. It preserves accepted bytes verbatim and inserts a new file-home
declaration in ID order ([§FS-fetch.3](../functional-spec/FS-fetch.md#3-accepted-declaration),
[§FS-fetch.4](../functional-spec/FS-fetch.md#4-file-home-write)).

With `resolve = "must"`, a missing snapshot is the ordinary `dangling` error.
With `resolve = "should"`, it is the fixed `missing-snapshot` warning and a
warning-only check exits 0. Omitting `resolve` from a fetch-enabled kind means
`must`; `resolve` without `fetch` and `resolve = "may"` are invalid. No check or
editor action fetches implicitly ([§FS-check.4.12](../functional-spec/FS-check.md#412-missing-snapshot)).

In a workspace, qualify the ID to select the owning project:

```sh
grund fetch api/TICKET-1234
```

The member's executable receives only `TICKET-1234`, and the declaration is
written under that member's configured home ([§FS-fetch.1](../functional-spec/FS-fetch.md#1-input-and-project-selection)).
