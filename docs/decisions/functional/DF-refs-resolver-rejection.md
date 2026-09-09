# DF-refs-resolver-rejection: an ID rejected by a selected grammar is a failed query

**Status:** Accepted
**Date:** 2026-09-09

## 1. Context

`show` and `refs` sent the same two resolver outcomes to different public exit
codes: an ID rejected by the configured grammar and an ambiguous number-only
shorthand exited `1` from `show`, but `2` from `refs`
([§FS-refs.4](../../functional-spec/FS-refs.md#4-exit-codes)). Both readings had
a coherent local rule. `show` treated them as queries with no result; `refs`
treated every failure outside its possibly-empty list as a run error. The split
made `2` mean either “the scan is untrustworthy” or “repair this operand”
depending on which query command a script invoked.

The status is a frozen user-visible scalar
([§FS-cli.5](../../functional-spec/FS-cli.md#5-exit-code-mapping-is-fixed)), so
choosing a common meaning also has to apply the deprecation path in
[§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path).

## 2. Decision

### 2.1 Grammar selection is the boundary

Exit `2` means the run could not establish or complete the query context, or
could not complete a trustworthy scan. Once context has selected a project's ID
grammar, rejection of the operand by that resolver is a failed query at exit
`1` ([§FS-errors.2.3](../../functional-spec/FS-errors.md#23-bare-query-failure)).
An unknown alias therefore remains `2`: without its project the ID tail has no
selected grammar. A resolved target with no citations remains the successful
empty answer at `0`.

The rule covers both configured-format rejection and ambiguous number-only
shorthand, including after `--summary` or `--section` and after a known workspace
alias selects its target grammar. It does not change `show`, full-ID ambiguity
across two homes, or configuration-validation failures.

### 2.2 A scalar gets a hold-and-warn release

Grund 0.14.0 retains `refs`' exit `2` and existing diagnostic and hint bytes,
then appends one exact warning naming the 0.15.0 exit-`1` change. Grund 0.15.0
removes the warning and uses the shared failed-query text and JSON shapes. This
is the two-release path required by
[§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path), applied to a scalar for which no command can migrate a caller.

The warning itself holds the pending release promise. At the flip, the ordinary
failed-query diagnostic remains free of a historical suffix; a version-gated
wire contract activates the new mapping and warning retirement, while the
release gate reads the warning-phase scalar clause
([§FS-distribution.4.2](../../functional-spec/FS-distribution.md#42-a-release-may-not-contradict-the-releases-the-trees-own-messages-name)).

## 3. Alternatives considered

Making every malformed operand a CLI error would have moved `show` to `2` and
made a selected grammar's answer indistinguishable from setup or scan failure.
Keeping `refs` permanently at `2` would have preserved its list-only model but
left scripts without one cross-command meaning for the same resolver outcome.
A configurable mapping was rejected because exit mappings are not policy knobs
([§FS-non-goals.9](../../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)).

## 4. Consequences

Exact-stderr consumers see an appended warning throughout 0.14.0. At 0.15.0,
callers repair these operands on exit `1` and reserve `2` for context, run, I/O,
and incomplete-scan failures. JSON callers receive one failed-query diagnostic
object instead of raw CLI text; successful and empty citation lists do not move.
