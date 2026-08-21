# REQ-no-missed-citation: every citation is seen

A citation the scanner does not see cannot dangle, so a missed site turns `success` into a lie. `grund` must find every `§`-marked citation in every scanned file — Markdown prose and each supported doc-comment form alike (§GOAL-polyglot-citation) — and validate all of them: false negatives are bugs (§GOAL-no-dangling-refs).

## 1. No silent skips

A file the scan cannot read, or a walk that matches nothing, is reported and fails the run (§FS-check.2) — never a quiet pass over whatever part was reached.

## 2. Proven per host language

Every doc-comment form the scanner claims to support is exercised by an executable case: a citation planted in that form must be found, and a dangling one planted in it must fail the check.
