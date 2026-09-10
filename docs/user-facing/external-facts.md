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
complete Markdown declaration to stdout. Save the following as `scripts/fetch-ticket`
and run `chmod +x scripts/fetch-ticket` before the first fetch:

````sh
#!/bin/sh
title=$(gh issue view "${1#TICKET-}" --json title --jq .title) || exit
body=$(gh issue view "${1#TICKET-}" --json body --jq .body) || exit
fence=$(printf '%s\n```\n' "$body" | grep -o '`\{3,\}' | sort | tail -n 1)
printf '## %s: %s\n\n%s`markdown\n%s\n%s`\n' "$1" "$title" "$fence" "$body" "$fence"
````

For a `file` home the declaration is H2; for a `folder` home it is H1, and body
subsections must be exactly one level deeper than that declaration. The example
computes an outer backtick fence one character longer than any run in the
provider body, so its own Markdown headings remain content but are not citable
child headings; raw issue Markdown is not a declaration body.
Grund validates the complete output before atomically replacing only that ID's
snapshot. It preserves accepted bytes verbatim and inserts a new file-home
declaration in ID order ([§FS-fetch.3](../functional-spec/FS-fetch.md#3-accepted-declaration),
[§FS-fetch.4](../functional-spec/FS-fetch.md#4-file-home-write)).

With `resolve = "must"`, a missing snapshot is the ordinary `dangling` error.
With `resolve = "should"`, it is the fixed `missing-snapshot` warning and a
warning-only check exits 0. Choose `must` when CI should enforce that every
external fact is materialized before merge; choose `should` when the repository
may carry a citation ahead of its snapshot and should only report the gap. These
mechanics are defined by [§FS-config.3.4.10](../functional-spec/FS-config.md#3410-format-resolve-and-fetch--external-snapshot-kinds).
Omitting `resolve` from a fetch-enabled kind means `must`; `resolve` without
`fetch` and `resolve = "may"` are invalid. No check or editor action fetches
implicitly ([§FS-check.4.12](../functional-spec/FS-check.md#412-missing-snapshot)).

In a workspace, qualify the ID to select the owning project:

```sh
grund fetch api/TICKET-1234
```

The member's executable receives only `TICKET-1234`, and the declaration is
written under that member's configured home ([§FS-fetch.1](../functional-spec/FS-fetch.md#1-input-and-project-selection)).
