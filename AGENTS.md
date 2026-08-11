# grund — agent instructions

<!-- BEGIN GRUND MANAGED BLOCK -->
## Grounding with grund (v6)

This project uses [`grund`](https://github.com/vjovanov/grund): every spec, goal, decision, and end-to-end test has a stable ID `<KIND>-<slug>[.<section>]` (`KIND ∈ {GRUND, GOAL, FS, AR, DF, DA, E2E, RM, DISC}`), cited with the marker `§` — e.g. `<§>FS-user-login.3.1` (the `FS-user-login` here is a shape illustration, not a real ID in this repo, hence the `<§>` escape). Type `$$` in a grund-aware editor and it becomes `§`. Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `.agents/grund.toml`, so only `§`-prefixed citations are checked.

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
- [AR](docs/architecture): How: high-level implementation, structure, and design
- [DF](docs/decisions/functional): Product behavior decisions and tradeoffs
- [DA](docs/decisions/architectural): Architecture decisions and tradeoffs
- [E2E](e2e/cases): Executable user scenarios
- [RM](docs/roadmap.md): Planned milestones and sequencing
- [DISC](docs/discussions): Design discussions and proposals

### Project namespaces

A namespace is a project boundary, not a docs folder. The current project is the local namespace: cite its IDs as `§<ID>`.

Create or use a separate namespace when work introduces an independently checked app, package, service, or subproject. Give that project its own `.agents/grund.toml`, add it to the workspace root's `[workspace] members`, run `grund init` there, and set a stable `project_name`.

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
- **AR** should cite FS or GOAL.
- **DF** should cite FS or GOAL.
- **DA** should cite AR or FS.
- **E2E** must cite FS.
- **code** (any file outside a kind home) should cite FS or AR.
Unlisted kinds and pairs are fine.

### Clickable citations

On repository web surfaces, link `§<ID>` to the PR branch in PR bodies, the reviewed commit in reviews, an exact commit for permalinks, and the default branch otherwise; fall back to plain when unsure. In local conversations, follow `§<ID>` with its declaration location as plain `path:line` text; fall back to the bare citation when unsure. If a user-level grund block states a local-conversation rendering, follow that instead: that machine knows what its surface can open.
<!-- END GRUND MANAGED BLOCK -->

## Repository workflow

- Every PR for this repository needs a `docs/changelog.md` `## Unreleased` bullet that mentions its PR number (`PR #N`); the pre-push hook checks this once the branch has a PR (§FS-distribution.4).

## Keeping Files Small With fissile (v1)

This repository uses [`fissile`](https://github.com/vjovanov/fissile) to keep
files small so agents spend fewer tokens reading them, while respecting the
architecture. It is a simple guard, not a style police.

- Run `fissile check --staged` before claiming work is done.
- Findings are grouped: the block header names the severity, rule, and limit,
  and the guidance under it is this repository's configured remediation for
  every file listed below it.
- A **soft** overflow means *should split*: if you changed the file, split it
  the way the guidance says — along a seam that already exists, never at the
  line count, never breaking apart code that belongs together.
- A **hard** overflow means *must split*: stop the line. Do not commit unless a
  structured exception already accepts the file.
- Never damage the design to fit a budget. If no split leaves the code better,
  record a soft overflow with `fissile exception add --severity soft`; for a
  hard overflow, ask a human — `--severity hard` is theirs to add, not yours.
- Run `fissile audit --stale-exceptions` before removing or moving large files.

