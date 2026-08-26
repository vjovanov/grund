# grund — agent instructions

<!-- BEGIN GRUND MANAGED BLOCK -->
## Grounding with grund (v7)

This project uses [`grund`](https://github.com/vjovanov/grund): every spec, goal, decision, and end-to-end test has a stable ID `<KIND>-<slug>[.<section>]` (`KIND ∈ {GRUND, GOAL, FS, REQ, AR, DF, DA, RM, DISC}`), cited with the marker `§` — e.g. `<§>FS-user-login.3.1` (the `FS-user-login` here is a shape illustration, not a real ID in this repo, hence the `<§>` escape). Type `$$` in a grund-aware editor and it becomes `§`. Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `grund.toml`, so only `§`-prefixed citations are checked.

### Grounding from a citation

A `§<ID>` is a pointer to a fact, not a file path. Resolve it with `grund` and climb only as far as needed:

- `grund <ID>` — the lead (heading-less, cut at the first child section). The cheap first read for a bare `§<ID>` citation.
- `grund <ID> --toc` — the lead plus the nested section map. Use to choose which subsection to fetch next.
- `grund <ID> --full` — the entire body. Escalate to this when narrower reads aren't enough.
- `grund <ID> --brief` — heading + first paragraph only.
- `grund refs <ID>` — every site that cites the ID; add `--summary` for one line per file. Run before renaming or moving a declaration.
- `grund list` / `grund list --kind FS,AR` — discover IDs if you get lost

### Project map

- [GRUND](docs/grund.md): Why: project motivation
- [GOAL](docs/goals.md): Where: project direction and outcomes
- [FS](docs/functional-spec): What: behavior, requirements, and constraints
- [REQ](docs/requirements): Hard requirements: what grund must never break
- [AR](docs/architecture): How: high-level implementation, structure, and design
- [DF](docs/decisions/functional): Product behavior decisions and tradeoffs
- [DA](docs/decisions/architectural): Architecture decisions and tradeoffs
- [tests/e2e/](tests/e2e): User scenarios: black-box proof of the spec
- [tests/integration/](tests/integration): Integration tests: proof that the parts fit as designed
- [RM](docs/roadmap.md): Planned milestones and sequencing
- [DISC](docs/discussions): Design discussions and proposals
- [skills/](skills): Agent review and automation skills
- [examples/](examples): Worked examples: user-facing walkthroughs that double as fixtures
- [.github/workflows/](.github/workflows): CI and release workflows: the gate on GitHub, and how a release ships
- [scripts/](scripts): Repository scripts: hook checks, release preparation, benchmarks, PGO build

### Project namespaces

A namespace is a project boundary, not a docs folder. The current project is the local namespace: cite its IDs as `§<ID>`.

Create or use a separate namespace when work introduces an independently checked app, package, service, or subproject. Give that project its own `grund.toml`, add it to the workspace root's `[workspace] members`, run `grund init` there, and set a stable `project_name`.

Do not create a namespace for a regular module or component that still belongs to this project. Cite across namespaces as `§alias/<ID>` and run `grund check` from the workspace root.

### Declarations and citations

Declarations are heading lines `# FS-user-login: …` in markdown. In a code doc-comment (Rustdoc, Javadoc, JSDoc, Python docstring, Go `//`, …) drop the `#` — write `/// FS-user-login: …` directly. Numbered headings inside a declaration are citable sections: use depth-matching headings (`## 1. …`, `### 1.1 …`, etc.) so `§<ID>.1` / `§<ID>.1.1` resolve; mismatched heading depth is a `grund check` error. Plain headings or bold labels are fine for non-citable local structure. One doc-comment may declare multiple IDs (e.g. an `AR-` and an `FS-` on the same class) — each gets its own body. An inline source declaration is reachable from the configured kind home via a one-line stub: `# <ID>: [<path>](<path>)`.

### Rules

- **Spec first.** For behavior or design changes, write or update the most-specific spec point before code.
- **Cite as you write.** Place `§<ID>` at the point a claim or behavior is made — on the doc-comment for a whole behavior, inline beside the clause it enforces.
- **Marker = live citation.** A `§`-prefixed token resolves and is checked wherever it appears — including inside Markdown backticks. To mention an ID without citing it, write `<§><ID>`, omit the marker, or use a fenced code block.
- **Inline citation style.** Inline notes: ≤ 1 line preferred, hard cap 25 lines; ≤ 180 columns.
- **Always cite the most-specific point.**

### Citation directions

- **GOAL** should cite GRUND or GOAL.
- **FS** should cite GOAL or FS; never cite AR.
- **REQ** should cite GRUND or GOAL; never cite AR.
- **AR** should cite FS or GOAL.
- **DF** should cite FS or GOAL.
- **DA** should cite AR or FS.
- **tests/e2e/** must cite FS; avoid citing AR.
- **tests/integration/** should cite AR.
- **skills/** must cite FS; never cite AR.
- **examples/** must cite FS; never cite AR.
- **.github/workflows/** should cite FS or AR.
- **scripts/** should cite FS or AR.
- **code** (any file outside a kind home) should cite FS or AR.
Unlisted kinds and pairs are fine.

### Clickable citations

On repository web surfaces, link `§<ID>` to the PR branch in PR bodies, the reviewed commit in reviews, an exact commit for permalinks, and the default branch otherwise; fall back to plain when unsure. In local conversations, follow `§<ID>` with its declaration location as plain `path:line` text; fall back to the bare citation when unsure. If a user-level grund block states a local-conversation rendering, follow that instead: that machine knows what its surface can open.
<!-- END GRUND MANAGED BLOCK -->

## Repository workflow

- Every PR for this repository needs a `docs/changelog.md` `## Unreleased` bullet that mentions its PR number (`PR #N`); the pre-push hook checks this once the branch has a PR ([§FS-distribution.4](docs/functional-spec/FS-distribution.md#4-release-process)).
- Edit `AGENTS.md`, never the `CLAUDE.md` symlink ([§REQ-agents-md.1](docs/requirements/REQ-agents-md.md#1-one-source-symlinked-companions)); this entrypoint's contract is [§REQ-agents-md](docs/requirements/REQ-agents-md.md#req-agents-md-the-agent-entrypoint-stays-managed-and-grounded), the README's is [§REQ-readme](docs/requirements/REQ-readme.md#req-readme-the-readme-is-the-grounded-shop-window).

<!-- BEGIN FISSILE MANAGED BLOCK -->
## Keeping Files Small With fissile (v3)

This repository caps file size with [`fissile`](https://github.com/vjovanov/fissile)
so that agents spend fewer tokens reading. Run `fissile check --staged` before
claiming work is done; its findings say what to split and how. Where the
pre-commit hook is installed it runs that same check — never get past it with
`--no-verify`.
<!-- END FISSILE MANAGED BLOCK -->
