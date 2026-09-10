FINDING: F-sol-high-2-1
OUTCOME: fixed
FILES: README.md, docs/user-facing/values.md
WHAT: Added `format = "{kind}-{slug}"` to both `CONST` examples, matching the per-kind override used by the slug-only IDs.

FINDING: F-sol-high-2-2
OUTCOME: fixed
FILES: docs/user-facing/values.md
WHAT: Added the sibling `## 2. Use` heading to close the source-comment value root before its binding prose.

FINDING: F-sol-high-2-3
OUTCOME: fixed
FILES: docs/user-facing/values.md
WHAT: Warned that an explicit kinds list replaces implicit defaults, directs readers to copy the effective defaults from `FS-config.3.4.4`, and explains that omitted kinds disappear while `check` stays green.

FINDING: F-sol-high-2-4
OUTCOME: fixed
FILES: docs/user-facing/external-facts.md
WHAT: Changed the fetcher to compute an outer backtick fence longer than every run in the provider body and explained its purpose; final `pre-commit run --all-files` passed fmt, build, Python, grund check/fmt/init, fissile, and attribution, while cargo-test failed only because `/tmp/.git` breaks `init_refused_targets` and lychee failed only on the pre-existing 404 at `docs/decisions/functional/DF-config-file-location.md`.

VERIFICATION:
`grund check` → `success`
`fissile check --staged` → `ok`
`pre-commit run --all-files` → exit `1`; `cargo fmt --check`, `cargo build`, `python tests`, `grund check`, `grund fmt`, `grund init --check`, `fissile`, and attribution passed; `cargo test` failed only because `/tmp/.git` covers the temp root required by `init_refused_targets`; `lychee` failed only on the pre-existing 404 for `https://github.com/vjovanov/fissile/issues/61` referenced by `docs/decisions/functional/DF-config-file-location.md`.
