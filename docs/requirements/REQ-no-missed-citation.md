# REQ-no-missed-citation: every citation the run reads is checked

A citation the scanner does not see cannot dangle, so a missed site turns `success` into a lie. Every `§`-marked citation in the text a run reads is recognised and validated — Markdown prose and each supported doc-comment form alike (§GOAL-polyglot-citation) — and false negatives are bugs (§GOAL-no-dangling-refs).

## 1. No silent skips

A file the scan cannot read is reported at its path and the run exits `2` — the report may be true as far as it got, but it is not complete, and it never passes as if it were (§FS-check.2). A walk that matches nothing is reported too, as a warning naming what was searched: an empty tree is not a failure, but it must never look like a clean one.

## 2. Every blind spot is declared and bounded

Regions the scanner deliberately does not read are legitimate; regions nobody wrote down are not. Each one must be stated in the spec, chosen by the repository or by a rule the repository can see, and answerable on demand.

- **Out of scope.** A citation outside `[scan] include` is invisible rather than checked — it neither resolves nor dangles (§FS-config.3.5). `grund check --full` widens the walk to the whole config root and reports what points at nothing out there (§FS-check.3.14). A blind spot nobody can look into would be a false negative; this one has a light switch.
- **Not text the scanner treats as prose.** Fenced code blocks are skipped, which is what makes an illustration in documentation safe to write (§FS-check.1.1). Inline code and string literals stay live for unqualified citations and are skipped only for the namespace-qualified form (§FS-check.1.1).
- **Not walked at all.** Hidden paths, `[scan] exclude` names, ignore-file matches, non-scanned extensions, and E2E fixture trees are outside every walk, `--full` included (§FS-check.1.3).

The obligation this creates is on the spec, not the scanner: a skip that no section names is the bug, because it is the one a reader cannot plan around.

## 3. Proven per host language

Every doc-comment form the scanner claims to support is exercised by an executable case in which a **dangling** citation planted in that form fails the check. The dangling direction is what proves the citation was *found* — a passing fixture cannot, since a missed citation also yields `success`. A form the default configuration cannot reach is not supported: advertising a comment prefix whose file extension `[scan] extensions` omits is a claim with no path to being true.
