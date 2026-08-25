# Architectural decisions

Why `grund` is *built* the way it is. Each file here is one decision about structure, packaging, or measurement: the context, the call, the alternatives, and the cost. The H1 declares a `DA-<slug>` ID, and the architecture point it settles cites it.

Half of this folder is the record of one rename. That is deliberate: the superseded entries are kept for the evaluation they contain, not for their verdicts, and deleting them would leave the accepted decisions arguing with nobody.

## Shape of the shipped tool

- [§DA-lsp-optional](DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary) — the LSP server is a separate, optional binary
- [§DA-pgo-release](DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload) — distributed binaries are PGO-built, trained on the benchmark workload
- [§DA-benchmark-instruction-counting](DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds) — the performance harness counts instructions, not wall-clock seconds

## Name and packaging

- [§DA-rename-to-grund](DA-rename-to-grund.md#da-rename-to-grund-rename-gnd-to-grund-before-first-publish) — `gnd` becomes `grund` before the first publish
- [§DA-pypi-uses-grund-as-the-package-name](DA-pypi-uses-grund-as-the-package-name.md#da-pypi-uses-grund-as-the-package-name-pypi-uses-grund-as-the-package-name) — and PyPI ships it as `grund`
- [§DA-reference-checker-name](DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) — *(superseded)* the original name evaluation that chose the `gnd` working title
- [§DA-pypi-package-name](DA-pypi-package-name.md#da-pypi-package-name-pypi-uses-gnd-cli-as-the-package-name) — *(superseded)* `gnd-cli` on PyPI, voided by the rename above

This index is navigational — citations should target the decision ID directly, never this file.
