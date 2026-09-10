# DF-off-grammar-declaration-compatibility: persisted declarations remain readable without relaxing the authoring grammar

**Status:** Accepted
**Date:** 2026-09-07

Exact read compatibility serves [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) when a repository's configured ID format and its persisted declarations drift apart.

## 1. Context

A repository can change `[id].format`, or add a per-kind override, while a
declaration and its citations keep an older spelling. Rejecting that spelling
at the query boundary makes a fact visibly present in the tree unreadable and
leaves `show`, `check`, `list`, `refs`, completion, formatting, coverage, and
the LSP disagreeing about whether it exists. Silently accepting every
kind-prefixed token instead would replace the configured grammar with an
unbounded second grammar and turn malformed prose into citations.

## 2. Decision

The effective format remains the grammar for authoring and conformance. The
shared scanner retains a declaration-position, colon-terminated token that
unambiguously begins with a configured citable kind as a catalog declaration,
including its raw spelling, body, sections, and source site. Exact
marker-prefixed candidates rejected by normal grammar are promoted only after
catalog merge, and only when an exact declaration in the selected project
backs them. Every reader consumes that shared result ([§FS-config.3.2](../../functional-spec/FS-config.md#32-id--id-grammar)).

Configured full IDs retain precedence. All remaining shorthand, duplicate,
section, and workspace interpretations are considered without guessing: one
target resolves, zero preserves the current invalid or dangling outcome, and
multiple targets fail with sorted candidates or sites. A whole-token exact
declaration wins before the token is split as an inline section.

The format mismatch stays located and visible as `declaration-near-miss`. It is
a warning through 0.14.x and becomes an error in 0.15.0 under
[§REQ-backwards-compatibility.2](../../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path),
without changing lookup or citation resolution at that boundary.

## 3. Rejected alternatives

**A `show`-only fallback** was rejected because retrieval would claim the ID
exists while graph and editor surfaces omit it. **Relaxing the configured
grammar globally** was rejected because unmarked candidates, unbacked marked
candidates, and malformed input would acquire meaning without repository
evidence. **Automatic rename** was rejected because the tool cannot choose
between changing the declaration and changing the configured format, nor can
it know every external citation.

## 4. Consequences

Repositories regain consistent reads immediately and receive one migration
diagnostic per mismatched declaration. Newly authored mismatches are readable
but diagnosed just like old ones; no history database distinguishes them. The
scanner/catalog is the compatibility boundary, so downstream commands and the
LSP do not grow parallel parsers. Existing configuration, authoring commands,
and JSON schemas remain unchanged. The scheduled severity change is tracked by
[§RM-off-grammar-declaration-error](../../roadmap.md#rm-off-grammar-declaration-error-make-off-grammar-declarations-a-check-error-in-0150).
