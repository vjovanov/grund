# DISC-external-facts: External facts are committed declarations materialized explicitly

## 1. Status

Concluded. This discussion replaces [§DISC-external-ticket-resolvers](2026-05-09-external-ticket-resolvers.md#disc-external-ticket-resolvers-external-ticket-resolvers) and is realized by [§FS-fetch](../../functional-spec/FS-fetch.md#fs-fetch-grund-materializes-one-external-fact-snapshot).

## 2. Context

The fact that motivates work often begins outside the repository: an issue, incident,
RFC, or wiki page. A URL-only resolver would make that fact visible only while the
external service and credentials were available. An index-only record would prove that
an identifier once existed but would not preserve the fact that authors cited. Both
would create a second resolution path beside grund's declaration scanner.

The chosen boundary is: **grund verifies snapshots; integrations own freshness**. The
repository keeps the complete Markdown declaration it relies on. All ordinary reads
remain functions of committed bytes; a user deliberately runs one command to ask a
repository-configured integration for newer bytes.

## 3. Decision

An external fact is an ordinary marked citation to an ordinary configured kind. Its
snapshot is a full Markdown declaration in that kind's `file` or `folder` home. It is
scanned, checked, listed, shown, formatted, completed, navigated, and counted exactly
like an authored declaration. Marked citations in its fetched body remain live.

The repository-wide `[id].format` is the default grammar, and a citable
`[[kinds]].format` may override it for that kind. This lets a repository combine local
slug IDs with numeric tickets without admitting markerless ticket-shaped prose.

A fetch-enabled kind has a target-side resolution obligation. `must` selects the fixed
`dangling` error; `should` selects the distinct fixed `missing-snapshot` warning. This
is not configurable severity: the setting chooses between two finding classes whose
severity and exit effects are invariant. There is no `may`. An explicit `resolve`
requires `fetch`; `fetch` without `resolve` means `must`, so every configured resolution
remedy can actually be attempted.

Only deliberate `grund fetch <ID>` may execute the configured program. It resolves the
program relative to the selected project's config root, invokes it directly without a
shell, appends the local unqualified ID as its sole argument, and inherits the caller's
environment. Repository configuration is trusted because the user deliberately invoked
the mutating command. No check, query, formatter, completion, or editor event executes
it. [§REQ-runs-offline](../../requirements/REQ-runs-offline.md#req-runs-offline-verification-never-depends-on-an-external-service) fixes that trust boundary.

The integration returns exactly one complete declaration at the native depth of its
home: H2 in a single-file home, H1 in a folder home. Grund validates all output before
mutation, preserves accepted output verbatim, and atomically replaces or stably inserts
only the requested declaration. Refusal leaves the tree byte-identical. This makes a
committed snapshot reviewable, deterministic, and useful after the source ticket closes.

## 4. Consequences

- Existing repositories opt into nothing and retain their current schema-v1 behavior.
- Freshness is visible as a normal repository diff and is never part of `check`.
- Snapshot declarations keep ordinary unused-declaration warnings; grund does not prune.
- Workspaces keep their current project ownership. A qualified fetch selects that
  member, while its integration receives the local ID.
- The first delivery has one-ID fetch only: no `--refresh`, generated timestamp,
  unused-snapshot exemption, or workspace-wide snapshot owner.
- LSP diagnostics, hover, and navigation reuse the ordinary engine result. A fetch code
  action and `workspace/executeCommand` remain future work and are not implied here.

## 5. Rejected alternatives

- **Live or implicit resolution:** makes verdicts depend on network, credentials, rate
  limits, and mutable remote state.
- **URL expansion or an index without bodies:** creates an external-reference subsystem
  and fails to preserve the cited fact.
- **Markerless ticket recognition:** manufactures citations from ticket-shaped prose,
  changelogs, and URLs, contrary to [§REQ-no-wrong-citation](../../requirements/REQ-no-wrong-citation.md#req-no-wrong-citation-a-citation-never-resolves-to-a-guess).
- **`resolve = "may"` or a fetchless `should`:** either hides a missing cited fact or
  becomes a severity knob without an actionable remedy.
- **Shell command strings:** make quoting and shell interpretation part of the fetcher
  contract. Direct execution gives one stable argv on every platform.
- **Implicit editor fetch:** crosses the deliberate-execution boundary. The editor may
  report the offline finding, but this change adds no action that runs repository code.
