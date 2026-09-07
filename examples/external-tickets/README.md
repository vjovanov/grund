# External ticket snapshots

This example teaches the fetch-backed external-fact workflow specified by
[§FS-fetch](../../docs/functional-spec/FS-fetch.md#fs-fetch-grund-materializes-one-external-fact-snapshot). The repository uses ordinary slug IDs by default and a numeric
`TICKET` override. `repo/docs/guide.md` cites a ticket before its snapshot
exists; `repo/scripts/fetch-ticket` is a deterministic local stand-in for a
GitHub, Jira, or Linear integration.

From inside `repo/`, the complete workflow is:

```sh
grund check
grund fetch TICKET-1234
grund check
grund show TICKET-1234
grund refs TICKET-1234
```

The first check exits 0 with the fixed `missing-snapshot` warning. Fetch is the
only command that executes the repository integration; it is silent on success
and writes the complete declaration to `docs/tickets.md`. The remaining
commands resolve and query only that committed Markdown snapshot, so they keep
working offline and never execute the integration.

The e2e runner performs the mutating fetch from a copied repository and compares
the entire result with `expected.repo`. No network or credentials are involved.
The practical cost is deliberate: snapshot freshness is an explicit repository
diff rather than an implicit property of every check.
