# FS-fetch: grund materializes one external fact snapshot

`grund fetch <ID>` deliberately asks the selected kind's configured integration for one
complete Markdown declaration and stores it in that kind's configured home. It is the
only external-materialization surface; all later scanning and resolution use the saved
declaration under [§REQ-runs-offline](../requirements/REQ-runs-offline.md#req-runs-offline-verification-never-depends-on-an-external-service).

## 1. Input and project selection

The command accepts exactly one local or workspace-qualified ID and no section. It uses
the same per-kind grammar and target-project selection as the ID query
([§FS-config.3.2](FS-config.md#32-id--id-grammar), [§FS-workspace.5](FS-workspace.md#5-command-scope)). `grund fetch alias/TICKET-1234` loads that member's config and home, renders the qualified form in user messages, and passes only `TICKET-1234` to the integration.

An ID that cannot be parsed follows the existing bare query-failure shape and exits 1.
A parsed ID whose kind has no `fetch`, or whose selected project cannot supply exactly
one configured file or folder home, is an operational refusal on stderr and exits 2.

## 2. Integration invocation

`[[kinds]].fetch` names one executable. A relative value is resolved from the selected
project's config root. Grund invokes that path directly, never through a shell, appends
the local unqualified ID as the integration's sole argument, inherits the user's
environment, and provides no implicit input on stdin. Deliberately running `fetch` is
sufficient authorization to execute repository-controlled configuration.

Fetcher stdout is snapshot data and is never copied to grund's stdout. Exit 0 advances
to validation. A spawn failure, signal, or non-zero integration exit is an operational
error on stderr and exit 2. The message identifies the configured integration and ID;
the tree remains byte-identical.

## 3. Accepted declaration

The complete stdout must be UTF-8 and contain exactly one Markdown declaration for the
requested local ID, with no sibling declaration:

- a `file` home accepts one H2 declaration (`## TICKET-1234: Title`);
- a `folder` home accepts one H1 declaration (`# TICKET-1234: Title`).

The declaration may contain ordinary prose, fenced blocks, and subsections exactly one
level deeper than its native declaration depth. Its title must be non-empty. A malformed
heading, wrong ID or kind, wrong native depth, duplicate declaration, sibling
declaration, invalid subsection depth, invalid UTF-8, or trailing content outside the
one declaration is refused on stderr with exit 2.

The whole output is validated before any filesystem mutation. Accepted bytes are
preserved verbatim; grund does not add timestamps, normalize whitespace, run `fmt`, or
sanitize marked citations. A marked citation in the body is therefore a live citation
on the next ordinary scan ([§FS-check.1.1](FS-check.md#11-recognized-citations)).

## 4. File-home write

For `file = "<home>"`, grund replaces only the H2 declaration whose ID exactly matches
the requested ID. If it is absent, grund inserts the declaration among sibling H2
declarations in ID order. The file's preamble, other declarations, line endings, and
all bytes outside the owned declaration remain unchanged.

The final file is installed atomically only after the complete result is available. If
the home cannot be read, parsed unambiguously, or atomically replaced, the command emits
an operational error, exits 2, and leaves it byte-identical.

## 5. Folder-home write

For `folder = "<home>"`, grund replaces the file that currently declares the requested
ID, or creates `<home>/<ID>.md` when no declaration exists. It never replaces a
different-ID file. Multiple existing declarations for the requested ID are ambiguous
and refused. The declaring file or new file contains the accepted stdout verbatim.

The replacement is atomic. A missing folder may be created only as part of the
successful final installation; any discovery, validation, or write failure leaves the
tree byte-identical and exits 2.

## 6. Stability and ownership

Fetching unchanged output is idempotent: it leaves the same file set and bytes. Stable
ID-order insertion and declaration-local replacement make repeated and independent
fetches produce reviewable diffs. `fetch` owns only the requested declaration in the
configured snapshot home, extending [§REQ-no-data-loss.2](../requirements/REQ-no-data-loss.md#2-writers-touch-only-what-they-own); it never prunes another snapshot or rewrites a citation.

One invocation fetches one ID. There is no `--refresh`, generated timestamp,
unused-snapshot exemption, or workspace-wide snapshot ownership behavior.

## 7. Output and exits

Success writes nothing to stdout or stderr and exits 0. An unparseable ID is a query
failure and exits 1. Missing fetch configuration, integration failure, rejected output,
ambiguous existing content, and filesystem failure are CLI-level operational errors on
stderr and exit 2, following [§FS-errors.2.2](FS-errors.md#22-cli-level-message).
