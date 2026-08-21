# REQ-no-missed-citation: every citation is seen

A citation the scanner does not see cannot dangle, so a missed site turns `success` into a lie. `grund` must find every `§`-marked citation in every scanned file — Markdown prose and each supported doc-comment form alike (§GOAL-polyglot-citation) — and validate all of them: false negatives are bugs (§GOAL-no-dangling-refs).

## 1. No silent skips

A file the scan cannot read is reported at its path and the run exits `2` — the report may be true as far as it got, but it is not complete, and it never passes as if it were (§FS-check.2). A walk that matches nothing is reported too, as a warning naming what was searched: an empty tree is not a failure, but it must never look like a clean one.

## 2. Scope is a stated boundary, not a silent one

A citation outside `[scan] include` is invisible rather than checked — it neither resolves nor dangles (§FS-check.1.3). That is the one missed-citation class `grund` accepts, and it is accepted only because it is bounded by configuration the repository chose and answerable on demand: `grund check --full` widens the walk to the whole config root and reports what points at nothing out there (§FS-check.3.14). A blind spot nobody can look into would be a false negative; this one has a light switch.

## 3. Proven per host language

Every doc-comment form the scanner claims to support is exercised by an executable case: a citation planted in that form must be found, and a dangling one planted in it must fail the check.
