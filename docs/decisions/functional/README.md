# Functional decisions

Why `grund` behaves the way it does. Each file here is one product-behavior decision: the context that forced it, the call, the alternatives that lost, and what the call costs. The H1 declares a `DF-<slug>` ID, and the spec point a decision settles cites it — so a rule in `docs/functional-spec/` is always one hop from its argument.

Read a decision when the spec tells you *what* and you need *why*. Do not read them for the current behavior: a decision records the state of the argument on its date, and a superseded one is kept for its reasoning, not its verdict.

## The citation form

How a citation is written, and what counts as one.

- [§DF-reference-marker](DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger) — Use § as the reference marker, with $$ as the typing trigger
- [§DF-code-declarations-drop-hash](DF-code-declarations-drop-hash.md#df-code-declarations-drop-hash-code-resident-declarations-may-drop-the--prefix) — code-resident declarations may drop the `#` prefix
- [§DF-number-only-citation-shorthand](DF-number-only-citation-shorthand.md#df-number-only-citation-shorthand-the-number-only-shorthand-is-authoring-sugar-and-a-persisted-one-is-a-check-error) — the number-only shorthand is authoring sugar, and a persisted one is a check error
- [§DF-shorthand-numeric-run](DF-shorthand-numeric-run.md#df-shorthand-numeric-run-a-marked-shorthand-glued-to-another-number-is-a-numeral-not-a-citation) — a marked shorthand glued to another number is a numeral, not a citation
- [§DF-inline-note-layout](DF-inline-note-layout.md#df-inline-note-layout-inline-note-layout-is-a-configured-house-style-checked-per-line-and-never-normalized) — inline note layout is a configured house style, checked per line and never normalized
- [§DF-note-columns-are-characters](DF-note-columns-are-characters.md#df-note-columns-are-characters-a-note-column-is-one-character-not-one-byte-and-not-one-display-cell) — a note column is one character, not one byte and not one display cell
- [§DF-doc-comments-are-not-notes](DF-doc-comments-are-not-notes.md#df-doc-comments-are-not-notes-a-doc-comment-is-documentation-not-a-note-and-is-never-an-inline-citation-site) — a doc comment is documentation, not a note, and is never an inline citation site

## Cross-reference links

The rendered view of a citation — `[§ID](path#anchor)` — and who owns it.

- [§DF-md-link-emission](DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations) — grund fmt may emit clickable Markdown links alongside §-prefixed citations
- [§DF-md-link-default-on](DF-md-link-default-on.md#df-md-link-default-on-markdown-cross-reference-links-default-on-for-github-review-and-discovery) — Markdown cross-reference links default on for GitHub review and discovery
- [§DF-md-link-anchor-strategy](DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass) — heading-text slugs, re-derived on every fmt pass
- [§DF-github-anchor-fidelity](DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly) — the github anchor profile reproduces github-slugger exactly
- [§DF-declaration-anchor](DF-declaration-anchor.md#df-declaration-anchor-a-bare-id-markdown-link-points-at-the-declarations-heading-anchor) — a bare-ID Markdown link points at the declaration's heading anchor
- [§DF-show-cross-ref-flattening](DF-show-cross-ref-flattening.md#df-show-cross-ref-flattening-grund-show-flattens-cross-reference-link-wrappers) — grund show flattens cross-reference link wrappers

## The index a folder kind keeps

The rules that make this file, and the ones beside it, checked rather than hoped for.

- [§DF-index-entry-form](DF-index-entry-form.md#df-index-entry-form-an-index-entry-is-one-full-link-per-id-and-nothing-else-about-the-page) — an index entry is one full link per ID, and nothing else about the page
- [§DF-index-compatibility-ramp](DF-index-compatibility-ramp.md#df-index-compatibility-ramp-a-findings-ramp-follows-its-fix-command-not-the-size-of-the-offence) — a finding's ramp follows its fix command, not the size of the offence
- [§DF-index-not-an-inbound-citation](DF-index-not-an-inbound-citation.md#df-index-not-an-inbound-citation-an-index-entry-is-navigation-not-use) — an index entry is navigation, not use
- [§DF-index-always-linkified](DF-index-always-linkified.md#df-index-always-linkified-the-cross-reference-pass-always-runs-on-a-kinds-index-file) — the cross-reference pass always runs on a kind's index file

## What `check` reports, and how loudly

- [§DF-check-full-scope](DF-check-full-scope.md#df-check-full-scope-check---full-walks-past-scan-include-and-reports-only-unresolvable-references-out-there) — `check --full` walks past `[scan] include` and reports only unresolvable references out there
- [§DF-require-grounding](DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec) — an opt-in check that every source file cites a spec
- [§DF-nothing-recognized](DF-nothing-recognized.md#df-nothing-recognized-a-run-that-recognized-nothing-says-so-and-says-it-as-a-warning) — a run that recognized nothing says so, and says it as a warning
- [§DF-duplicate-section-path](DF-duplicate-section-path.md#df-duplicate-section-path-a-section-coordinate-names-one-heading-or-the-run-says-so) — a section coordinate names one heading, or the run says so
- [§DF-citation-directions](DF-citation-directions.md#df-citation-directions-encode-citation-directions-as-checked-config-with-rfc-2119-levels) — encode citation directions as checked config with RFC-2119 levels

## Config, discovery, and workspaces

- [§DF-config-file-location](DF-config-file-location.md#df-config-file-location-grundtoml-is-discovered-at-two-names-per-directory-and-init-writes-the-bare-one) — grund.toml is discovered at two names per directory, and init writes the bare one
- [§DF-non-citable-kinds](DF-non-citable-kinds.md#df-non-citable-kinds-a-kind-may-declare-no-ids-and-stays-one-kinds-table-when-it-does) — a kind may declare no IDs, and stays one `[[kinds]]` table when it does
- [§DF-unwalked-kind-home](DF-unwalked-kind-home.md#df-unwalked-kind-home-a-kind-may-be-a-place-that-is-listed-but-not-walked) — a kind may be a place that is listed but not walked
- [§DF-symlink-scan](DF-symlink-scan.md#df-symlink-scan-a-symlink-in-the-scanned-tree-is-followed-and-the-report-names-the-link) — a symlink in the scanned tree is followed, and the report names the link
- [§DF-subproject-namespaces](DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) — alias-namespace model for sub-projects and external repos
- [§DF-nested-workspaces](DF-nested-workspaces.md#df-nested-workspaces-a-nested-project-is-named-by-its-whole-alias-path) — a nested project is named by its whole alias path
- [§DF-workspace-member-descriptions](DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists) — member-side `project_description` for workspace member lists
- [§DF-cover-workspace-scope](DF-cover-workspace-scope.md#df-cover-workspace-scope-cover-indexes-the-whole-run-and-counts-cross-project-citations) — cover indexes the whole run and counts cross-project citations
- [§DF-cli-base-parent-paths](DF-cli-base-parent-paths.md#df-cli-base-parent-paths-relative_paths--false-keeps-one-cli-base-and-may-climb-within-the-loaded-root) — `relative_paths = false` keeps one CLI base and may climb within the loaded root

## The command surface

- [§DF-show-default-token-cheap](DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in) — grund show defaults to the cheap read; the full body is opt-in
- [§DF-show-token-cheap-reads](DF-show-token-cheap-reads.md#df-show-token-cheap-reads-grund-show-keeps-the-full-body-default-token-cheap-slices-are-opt-in) — *(superseded)* grund show keeps the full-body default; token-cheap slices are opt-in
- [§DF-show-keep-explicit-form](DF-show-keep-explicit-form.md#df-show-keep-explicit-form-grund-keeps-show-as-a-subcommand-alongside-the-bare-id-default) — grund keeps `show` as a subcommand alongside the bare-ID default
- [§DF-keep-id-for-pure-id-allocation-and-reserve-new-for-stub](DF-keep-id-for-pure-id-allocation-and-reserve-new-for-stub.md#df-keep-id-for-pure-id-allocation-and-reserve-new-for-stub-keep-id-for-pure-id-allocation-and-reserve-new-for-stub-creation) — Keep `id` for pure ID allocation and reserve `new` for stub creation
- [§DF-id-number-width](DF-id-number-width.md#df-id-number-width-grund-id-zero-pads-minted-numbers-to-a-default-width-of-3) — grund id zero-pads minted numbers to a default width of 3
- [§DF-integrations-command](DF-integrations-command.md#df-integrations-command-integrations-earns-a-cli-slot-as-one-time-setup-where-a-per-citation-link-command-did-not) — integrations earns a CLI slot as one-time setup, where a per-citation `link` command did not
- [§DF-neural-link-generation](DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command) — agents compose clickable citation links themselves; grund does not grow a `link` command

## Agent-facing surfaces

- [§DF-managed-block-delimiters](DF-managed-block-delimiters.md#df-managed-block-delimiters-standard-beginend-delimiters-for-the-managed-agent-instructions-block) — standard BEGIN/END delimiters for the managed agent-instructions block
- [§DF-skill-init-existing-specs](DF-skill-init-existing-specs.md#df-skill-init-existing-specs-grund-init-adopts-existing-specs-before-scaffolding) — `grund-init` adopts existing specs before scaffolding
- [§DF-repo-conversation-opinion](DF-repo-conversation-opinion.md#df-repo-conversation-opinion-repositories-may-commit-a-link-only-conversation-rendering-opinion) — repositories may commit a link-only conversation-rendering opinion
- [§DF-conversation-link-target](DF-conversation-link-target.md#df-conversation-link-target-the-conversation-link-form-is-a-markdown-link-over-an-absolute-uri-addressed-per-machine) — the conversation link form is a Markdown link over an absolute URI, addressed per machine
- [§DF-directions-render](DF-directions-render.md#df-directions-render-the-citation-directions-wording-is-chosen-once-against-a-canonical-config) — the citation-directions wording is chosen once, against a canonical config

This index is navigational — citations should target the decision ID directly, never this file.
