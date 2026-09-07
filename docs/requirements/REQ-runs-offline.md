# REQ-runs-offline: verification never depends on an external service

The same committed tree and configuration must remain verifiable on a disconnected
machine. A citation is grounded by repository bytes, never by a service response that
can change, disappear, rate-limit, or require credentials. This preserves
[§REQ-deterministic-output](REQ-deterministic-output.md#req-deterministic-output-same-input-same-bytes) and the fast local loop of [§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible).

## 1. Read and verification paths execute nothing

Scanning, `check`, ID queries, `refs`, `list`, `cover`, `config show` and `validate`,
`fmt`, shell completion and its dynamic helper, and every LSP request perform no network
I/O and execute no repository-configured process. This remains true for a kind that
configures a fetcher and for a missing snapshot. Those paths read only the tree and
configuration they already own.

## 2. Materialization is explicit

Only a deliberate `grund fetch <ID>` invocation may execute the selected kind's
configured integration, under [§FS-fetch](../functional-spec/FS-fetch.md#fs-fetch-grund-materializes-one-external-fact-snapshot). It does not make verification online: the integration's output must first become a local Markdown declaration, and subsequent resolution uses that committed snapshot through the ordinary scanner.

## 3. No implicit freshness

Grund does not check remote freshness, re-fetch on a timer, fetch during editor events,
or silently materialize a missing citation. Integrations decide how to contact their
service; grund decides only whether their complete declaration output is safe to place
in the configured home.
