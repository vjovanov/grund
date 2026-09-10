# DA-rename-to-grund: Rename gnd to grund before first publish

**Status:** Accepted
**Date:** 2026-05-11
**Supersedes:** [§DA-reference-checker-name](DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) (the name decision) and [§DA-pypi-package-name](DA-pypi-package-name.md#da-pypi-package-name-pypi-uses-gnd-cli-as-the-package-name) (whose PyPI-collision premise the rename voids).

## 1. Context

The working title carried through pre-release development was `gnd` — chosen in [§DA-reference-checker-name](DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) as a terse, cargo-clean abbreviation for "ground." Two costs accumulated against it:

- **Registry friction.** The unscoped `gnd` is a dormant squat on npm, and PyPI serves releases under that occupied package name — so [§DA-pypi-package-name](DA-pypi-package-name.md#da-pypi-package-name-pypi-uses-gnd-cli-as-the-package-name) had already had to fall back to `gnd-cli` on PyPI (and npm), splitting the install name from the binary name. Each registry needed its own caveat.
- **Searchability.** `gnd` is the universal electrical-engineering abbreviation for *ground*; "gnd" alone is unsearchable, and the name carried no meaning a reader could land on.

No `gnd` binary or package was ever published — `0.1.0` is the first release — so the name is still free to change at zero cost to any consumer. The pre-release review asked for a name that is (a) short and CLI-friendly, (b) clean on crates.io, npm, and PyPI without a `-cli` workaround if possible, and (c) *says* what the tool does.

`grund` — German for *reason*, *ground*, *basis*, and the stem of `Grundlage* ("foundation") — is exactly that concept: every claim in a conformant tree climbs, via a `§<ID>` citation, toward the reason that grounds it. It is `gnd` with its vowels restored, so it reads as the honest expansion of the name the project was already using. It is five ASCII letters, no shift key. crates.io and PyPI are free for `grund`; the unscoped `grund` on npm is a low-use dormant package, the same dormant-squat situation [§DA-reference-checker-name](DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) judged ignorable for `gnd`.

## 2. Decision

**Rename the project from `gnd` to `grund`.** This is a single pre-publish rename, applied in one pass:

- **cargo:** crate and binary `grund`; library `grund-core`; optional server `grund-lsp`.
- **npm / PyPI:** the CLI publishes as `grund-cli`, the server as `grund-lsp` — one name that reads identically on both registries and as "the package that installs the `grund` command." `grund` itself appears free on PyPI; the later PyPI decision records why the package collapsed to the bare `grund` after the live registry check ([§DA-pypi-uses-grund-as-the-package-name](DA-pypi-uses-grund-as-the-package-name.md#da-pypi-uses-grund-as-the-package-name-pypi-uses-grund-as-the-package-name)).
- **GitHub repository:** `github.com/vjovanov/gnd` → `github.com/agent-grounds/grund`.
- **CLI:** the installed command is `grund`; every subcommand and flag is otherwise unchanged.
- **Config:** the discovered file is `.agents/grund.toml`; the schema-version key is `grund_config_version` (value still `1`).
- **Agent surface:** the `AGENTS.md` managed block and `grund agent-setup-instructions` use `grund`; the embedded skill source moves to `skills/grund-init/SKILL.md` (block version still v1).
- **Docs, specs, e2e corpus, code:** every `gnd` reference becomes `grund`. The e2e fixture configs (`.agents/grund.toml`) and the one fixture named for the file (`init-grund-toml-conflict-no-force`) move with it.

**What does not change:** the `§` reference marker, the `$$` typing trigger ([§DF-reference-marker](../functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger)), the `<KIND>-<slug>` ID grammar, every declared ID, and the `grund_config_version` / `AGENTS.md`-block version *numbers*. The config-key *name* changed, but since no released `gnd` exists there is no migration: a fresh clone uses `.agents/grund.toml` with `grund_config_version` from the start.

## 3. Consequences

- [§FS-distribution](../../functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) records the `grund` package names; the release process re-checks them before publish ([§FS-distribution.4](../../functional-spec/FS-distribution.md#4-release-process)).
- `scripts/check-registry-names.sh` queries the claimed names — `grund` and `grund-lsp` on crates.io, `grund-cli` and `grund-lsp` on npm and PyPI — and reports the unscoped `grund` on npm (dormant squat) and PyPI (currently free) as notices.
- The `gnd` → `grund` change is itself a changelog entry under §2.5 *Renamed* in the `0.1.0` release.
- [§DA-reference-checker-name](DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) and [§DA-pypi-package-name](DA-pypi-package-name.md#da-pypi-package-name-pypi-uses-gnd-cli-as-the-package-name) are marked **Superseded** and link here. Their bodies are kept verbatim: the name-evaluation table in the first still records *how* names were judged, and the second records the PyPI collision that pushed toward an explicit alternate — both inputs to this decision.

## 4. Alternatives considered

| Option | Why not |
|---|---|
| Keep `gnd`, keep the `gnd-cli` workaround | Carries the EE-abbreviation searchability problem and a per-registry caveat forever; the cost of fixing it only ever rises after the first publish. |
| `fiducial` (the runner-up in [§DA-reference-checker-name](DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool)) | Free everywhere and a sharp metaphor, but nine letters and esoteric — a CLI name people look up once. `grund` is shorter, ASCII, and reads as the obvious expansion of the name already in use. |
| `beleg` (German *citation / supporting reference*) | Arguably the tightest semantic fit for "a citation that resolves," but opaque to an English-speaking user typing it daily; a guessable CLI name is worth more than the marginally better meaning. |
| Scoped `@grund/cli` on npm to dodge the squat | A scope is extra ceremony for users and still does not match the crates.io/PyPI names; `grund-cli` unscoped is consistent and the squat is dormant. |
