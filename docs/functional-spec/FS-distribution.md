# FS-distribution: grund distribution targets

`grund` is written in Rust; the target distribution is **all three** major language ecosystems — cargo, npm, and PyPI — with idiomatic API bindings on each. The check engine stays a single shared library; only the surfaces differ. Today the implemented release path publishes the Rust crates: `grund-core`, the Cargo CLI package `grund`, and the optional Cargo LSP package `grund-lsp`. The npm and PyPI bindings, including their future `grund-lsp` packages, are tracked in `docs/roadmap.md`. Serves [§GOAL-multi-language](../goals.md#goal-multi-language-same-engine-three-platforms) and [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible).

## 1. Targets

| Registry | Package name        | Status      | Contents                                                                  |
|----------|---------------------|-------------|---------------------------------------------------------------------------|
| cargo    | `grund-core`          | implemented | Shared engine library used by the CLI, LSP, and future bindings.            |
| cargo    | `grund`               | implemented | CLI crate depending on `grund-core`; installs the `grund` binary.              |
| cargo    | `grund-lsp`           | implemented | Optional LSP server binary ([§FS-lsp](FS-lsp.md#fs-lsp-grund-ships-an-optional-lsp-server)). Depends on `grund-core`.              |
| npm      | `grund-cli`           | planned     | Prebuilt CLI binary + thin Node API surface (via `napi-rs`).              |
| npm      | `grund-lsp`           | planned     | Optional LSP server binary, prebuilt per platform.                        |
| PyPI     | `grund`               | planned     | Prebuilt CLI wheel + Python API surface (via `PyO3` / `maturin`).         |
| PyPI     | `grund-lsp`           | planned     | Optional LSP server, distributed via wheel (`pipx install grund-lsp`).      |

On crates.io the shared engine crate is `grund-core`; the installable CLI crate is `grund`, which depends on `grund-core` and produces the `grund` binary; and the optional LSP crate is `grund-lsp`, which depends on `grund-core` and produces the `grund-lsp` binary. On PyPI the planned CLI package is also `grund`: the wheel installs the `grund` command and exposes the Python import module `grund`. On npm the planned CLI package is `grund-cli` because the unscoped `grund` package is externally occupied; it still installs the `grund` command. `grund-lsp` is the optional server slot on each registry, but only the Cargo package is implemented today. The tool was renamed from its pre-release working title `gnd` to `grund` ([§DA-rename-to-grund](../decisions/architectural/DA-rename-to-grund.md#da-rename-to-grund-rename-gnd-to-grund-before-first-publish)); the final PyPI name is set by [§DA-pypi-uses-grund-as-the-package-name](../decisions/architectural/DA-pypi-uses-grund-as-the-package-name.md#da-pypi-uses-grund-as-the-package-name-pypi-uses-grund-as-the-package-name), which records why PyPI uses the bare name while npm keeps `grund-cli`. Every one of these names — and the remaining unreserved npm/PyPI LSP slots — is re-verified against the live registries before publish by the release process ([§FS-distribution.4](FS-distribution.md#4-release-process)).

Support packages that are not the primary user-facing install, such as `grund-core`, publish registry README content that links users to the `grund` CLI and names sibling packages such as `grund-lsp` once they exist.

The CLI install on each registry does **not** transitively pull in `grund-lsp` — they are independent packages, per [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). A user who only runs `grund check` in CI installs the CLI alone; a user who wants editor integration installs `grund-lsp` separately and configures their editor to launch it ([§FS-lsp.2](FS-lsp.md#2-installation-and-lifecycle)).

## 2. CLI parity

The `grund` binary behaves identically regardless of how it was installed: the same flags, the same exit codes, the same byte-for-byte report format ([§REQ-deterministic-output](../requirements/REQ-deterministic-output.md#req-deterministic-output-same-input-same-bytes)). Users on Linux, macOS, and Windows who run `grund .` against the same repo get the same answer.

CLI reports use repo-relative logical paths with `/` as the separator, even on Windows. This applies to text reports, JSON fields, `sites`, ID-query e2e fixture lists, stub-link targets, and generated cross-reference URLs; native platform paths may appear only in launch-time errors about paths outside the scanned repo, where there is no repo-relative path to print. The CI build/test matrix is the proof for this contract: every normal e2e case must pass on Linux, macOS, and Windows.

## 3. API surfaces

Each binding exposes the same conceptual operations as the CLI subcommands, plus a programmatic check-and-iterate path so the engine can be embedded inside test runners and editor servers.

### 3.0 Language-neutral data shapes

Every binding returns the same data, only spelled idiomatically. The conceptual shapes are:

```
Report {
  errors:   [Finding]
  warnings: [Finding]
}

Finding {
  severity: "error" | "warning"
  code:     // a `check` finding — "dangling" | "missing-section" | "duplicate" | "broken-stub"
            //                   | "unused" | "ungrounded" | "agents-init" | "empty-scan" | "io"
            // — or, on a failed ID query (FS-show.3, rendered with this same shape on stderr,
            //   path/line null) — "not-found" | "missing-section" | "broken-stub" | "ambiguous"
            //                   | "invalid-id" | "query-failed"
  path:     string?        // relative to config root (FS-config.3.6); null for a CLI-level error
  line:     u32?           // 1-indexed; null for a file-level finding with no line (e.g. an unreadable file, FS-check.2)
  message:  string         // the human-readable text
  sites:    [{ path, line }]?  // null for a single-site diagnostic; a list naming every site for a multi-site
                               // finding (a duplicate declaration) or an ambiguous-ID query failure
}

ShowOpts {
  section: string?    // dotted section path, e.g. "3.1.2"
  mode:    "lead" | "brief" | "toc" | "full"
                      // the same mutually exclusive show ladder as §FS-show.1;
                      // "lead" is the CLI's no-flag default
                      // default: "lead"
  format:  "text" | "md" | "json"
}
```

These fields are normative. The byte-for-byte JSON form emitted by `grund --format=json` and consumed by IDE/agent integrations follows the same shape and is the cross-binding equivalence test for [§GOAL-multi-language](../goals.md#goal-multi-language-same-engine-three-platforms), tracked in [AR-goal-measurement.2](../architecture/AR-goal-measurement.md#2-goal-meters).

### 3.1 Rust (`grund-core` crate)

```rust
let report = grund_core::check(&path)?;
let body = grund_core::show("FS-check", ShowOpts::default())?;
```

`Report` and the underlying `Findings` are exposed as plain data structures so callers can iterate, filter, or render their own output.

### 3.2 Node (`grund-cli` npm package)

```js
import { check, show } from 'grund-cli';

const report = await check('./repo');
const body = await show('FS-check', { mode: 'brief' });
```

The Node binding is built with `napi-rs`. Native binaries are prebuilt for the platforms covered by `napi-rs` (macOS arm64/x64, Linux x64/arm64, Windows x64). Source builds are supported as a fallback.

### 3.3 Python (`grund` PyPI package)

```python
from grund import check, show

report = check("./repo")
body = show("FS-check", mode="brief")
```

The Python binding is built with `PyO3` and packaged with `maturin`. Wheels are built for CPython 3.10+ across the platforms covered by `cibuildwheel`. The distribution package and import module are both named `grund` ([§DA-pypi-uses-grund-as-the-package-name](../decisions/architectural/DA-pypi-uses-grund-as-the-package-name.md#da-pypi-uses-grund-as-the-package-name-pypi-uses-grund-as-the-package-name)).

## 4. Release process

The implemented release workflow publishes the Cargo CLI today and builds downloadable PGO binaries for every supported desktop CI platform. A `vX.Y.Z` tag triggers `.github/workflows/release.yml` directly. The same workflow can also be run manually from the release commit: the operator enters the version, crate publishing is enabled by default, and the workflow creates `vX.Y.Z` if that tag does not already exist. If the requested tag already exists, that tag becomes the release source ref after the workflow verifies the tagged Cargo package versions match the requested version; this lets a failed release be recovered from a newer workflow commit without moving the release tag. In either entry path, the workflow verifies the tag/version matches both Cargo package versions, runs a fail-fast preflight on the crates.io token when crate publishing is enabled, re-runs `scripts/check-registry-names.sh` so claimed package names must still be available or owned by this project, then builds and self-checks profile-guided-optimized binaries on six targets — `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, and `aarch64-pc-windows-msvc`. `release.yml` does not bump versions: the selected commit already carries the version being released.

Version bumping lives in separate helper workflows. `.github/workflows/release-minor.yml` is a manual helper that starts from the latest `vX.Y.Z` tag, computes `vX.(Y+1).0`, verifies that `main` has commits since the previous tag and that CI is green on the current `main` tip, commits the workspace version bump to a temporary release-candidate branch, runs `release.yml` as a non-publishing dry run on that candidate, and only then fast-forwards `main` and dispatches `release.yml` for the real publish. `.github/workflows/auto-bump.yml` is the scheduled/manual patch helper with the same shape: when `main` has substantive non-doc/CI changes since the latest `vX.Y.Z` tag and CI is green on that exact tip, it computes `vX.Y.(Z+1)`, validates the version bump on a temporary release-candidate branch, and publishes the bump to `main` only after the dry run succeeds. In both helpers, "the version bump" includes the Cargo manifests, the lockfile, any checked fixture whose expected output embeds `grund --version`, and the deterministic changelog rotation performed by `scripts/prepare_changelog_release.py`: the curated `## Unreleased` bullets become the new inline release section, the former inline latest release is archived under `docs/changelog/<version>.md`, and the older-release index gains the archive link. The helper fails rather than inventing release notes when `## Unreleased` has no bullet entries, so the release candidate is already e2e-clean and changelog-clean before `release.yml` runs. Both helpers use the release push credential configured for branch-protection bypass; neither publishes directly.

Pull-request CI protects those curated notes before release time: `.github/workflows/ci.yml` runs `scripts/check_changelog_pr_entry.py` on `pull_request` events, and that script requires `docs/changelog.md`'s `## Unreleased` body to mention the current pull request as `PR #N`, `pull request #N`, or a `/pull/N` URL. The local pre-push hook runs the same check once the current branch has a resolvable GitHub PR number. This keeps the release section mappable back to the PRs it contains; push CI and release-candidate branches skip the PR-number gate because they do not have a current pull request context.

`release.yml` treats the changelog as the source of GitHub release notes. Before creating or updating a GitHub release, it extracts the requested `vX.Y.Z` section from `docs/changelog.md` at the selected release ref and passes that body to `gh release create` or `gh release edit`; if the section is missing or empty, the release fails before artifacts are published to the GitHub release.

Both Linux binaries are built on GitHub inside a `manylinux2014_<arch>` container (pinned by digest, not by tag) so the release artifact targets an old glibc baseline instead of inheriting whatever glibc happens to ship on `ubuntu-latest`. Every job — host-runner or in-container — installs the same pinned Rust toolchain so the six binaries are produced by the same compiler version.

The distributed `grund` binaries are profile-guided-optimized: each platform build runs `scripts/pgo-build.sh`, which builds an instrumented binary, runs the [AR-benchmarks](../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) self-repo hot command list (the commands agents and CI invoke most) against `grund`'s own conformant tree to record a profile, then rebuilds against it. If a hosted runner's PGO training is platform-broken while the ordinary release build still self-checks cleanly, that platform may publish a self-checked LTO-only fallback binary instead of blocking the whole release. The rationale, and why the self-repo benchmark workload is also the PGO training corpus, is [§DA-pgo-release](../decisions/architectural/DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload). PGO is not part of development builds or push/PR CI; it is a release-packaging step, and an explicit benchmarking step when comparing the optimized release artifact. A `cargo install grund` from source is LTO-optimized but not PGO'd — `cargo install` runs no custom build step — and is byte-for-byte behavior-identical to the distributed binary; only its performance differs.

After every platform binary passes its self-check, the workflow publishes `grund-core`, `grund`, and `grund-lsp` to crates.io when crate publishing is enabled. The dependency order is fixed: `grund-core` publishes first, and any run that still needs to publish `grund` or `grund-lsp` waits up to 30 minutes for Cargo to resolve the matching `grund-core` version before publishing the dependent crate. GitHub release artifacts are uploaded only after the platform PGO builds pass and, when enabled, crates.io publishing succeeds.

The future full-ecosystem release keeps the same shape but adds the remaining packages after their frontends exist:

1. Build per-platform Node binaries and publish `grund-cli` and `grund-lsp` to npm.
2. Build per-platform Python wheels and publish `grund` and `grund-lsp` to PyPI.

All artifacts must succeed for a full ecosystem release to be considered complete. Versions across the CLI and the LSP move together within a release; each `grund-lsp` package pins or bundles the same `grund-core` version the matching CLI release ships, so a CLI/LSP version mismatch from the official packages is structurally avoided.

## 5. What we do not promise

- 100% identical APIs across languages. Each binding is idiomatic to its host (camelCase for Node, snake_case for Python, `Result<T,E>` for Rust). The *behavior* is identical; the surface fits each ecosystem.
- Stable ABI for the C-level FFI. Bindings link against the Rust core at compile time; we do not ship a separate C library.
