# FS-init: grund bootstraps a new grund-conformant repo

The `init` subcommand writes the minimum set of files a project needs to start using `grund` — an agent entrypoint and a `.agents/grund.toml` config — so that a fresh repo (or an existing repo adopting the scheme) becomes scannable in one command. `init` is designed to be the minimum-effort, non-intrusive on-ramp to `grund`: every default is the most-common case, every existing entrypoint the repo already has is preserved, nothing the user already authored is rewritten without `--force`, and a single `--dry-run` flag previews any run before it touches the working tree. Every behavior below should be read as serving that principle — the consolidated guarantees are in §3. The config lives under `.agents/` per [§FS-config.1](FS-config.md#1-file-location-and-discovery), so the repo root stays free of `grund`-specific files. In a repo that already has known agent entrypoint files, `init` preserves the existing choice and appends or updates a versioned `grund` block there by default; for example, a repo with only `CLAUDE.md` gets `CLAUDE.md` updated, not a new `AGENTS.md`. A repo with no existing agent entrypoint falls back to the canonical `AGENTS.md`. Serves [§GOAL-agent-grounding.1](../goals.md#1-the-three-layers) (the managed `grund` block is the instruction layer — an agent reading its entry-point file at session start arrives already taught), [§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree) (the emitted defaults are the canonical grammar), and [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (no prompts, no surprises).

`init` is the only `grund` subcommand that creates adoption scaffolding in the working tree. It may also update the managed `grund` block inside existing agent entrypoint files; it must not rewrite unrelated user-authored content in those files unless the file is the canonical `AGENTS.md` and `--force` is passed.

For verbose implementer fixtures — exact stderr transcripts, final tree expectations, and common existing-file cases — see [§FS-init-fixtures](FS-init-fixtures.md#fs-init-fixtures-concrete-init-fixtures). Those fixtures are examples of this spec, not a separate feature.

## 1. Inputs

```
grund init [<path>] [--name <name>] [--description <text>] [--docs] [--force] [--dry-run] [--agents-md] [--claude] [--gemini] [--copilot] [--cursor] [--windsurf] [--zed]
```

- `<path>` — directory in which to scaffold. Defaults to `.` (the current directory). Applies to every form of `init` — including `--docs` — and prefixes every emitted path in §2.1. Must exist; `init` does not create the target directory itself (a missing target is a user error, not something to silently paper over).
- `--name <name>` — human-readable project name baked into the generated `AGENTS.md` heading when canonical `AGENTS.md` is selected, and into the `.agents/grund.toml` `project_name` key. Defaults to the basename of `<path>` resolved to an absolute path.
- `--description <text>` — one-line project description baked into the generated `.agents/grund.toml` `project_description` key ([§FS-config.3](FS-config.md#3-schema)), replacing the commented teaching line §2.4 writes by default. No default: when omitted, `init` never invents a description. A `<text>` containing a line break is a CLI error, mirroring the config-side single-line rule. Like `--name`, the value only lands in a freshly written config — an existing `.agents/grund.toml` is never modified (§2.4) — but it still feeds the Workspace Members self row (§2.3.4.15).
- `--docs` — also scaffold the docs referenced by the effective config: by default this means stub `requirements.md`, `docs/grund.md`, `docs/goals.md`, `docs/roadmap.md`, `docs/changelog.md`, `docs/architecture/`, `docs/decisions/architectural/`, `docs/decisions/functional/`, and an empty `e2e/` directory with a stub `README.md`. The root-level `requirements.md` stub is scaffolded because the generated `FS` kind uses it as the default requirements/spec home (§2.4); an existing config that omits `[[kinds]]` instead keeps the compatibility FS home from [§FS-config.2](FS-config.md#2-precedence), so `--docs` scaffolds `docs/functional-spec/README.md` and points next-step guidance at `docs/functional-spec`. `roadmap.md` and `changelog.md` are scaffolded because the generated managed block's `docs/` table links to them (§2.3). Off by default — most adopters already have a `docs/` of some shape and want only the entry point and config. Composes with `<path>`: every scaffolded file lands under `<path>/`.
- `--force` — overwrite files that already exist at the target paths. Off by default. The default existing-entrypoint behavior is already append-or-update (§2.3), so `--force` is only needed to reset a generated `AGENTS.md` or a `--docs` scaffold to its canonical bytes.
- `--dry-run` — preview the run without writing or modifying any file. Every line that would have been emitted as `wrote `, `appended `, or `updated ` is reported with the `would-write `, `would-append `, or `would-update ` prefix instead; `exists ` lines and the `next:` block are unchanged. Composes with every other flag, including `--force`. Off by default.
- `--agents-md` — explicitly create or update the canonical `<path>/AGENTS.md` entrypoint even when another agent entrypoint already exists.
- `--claude` — explicitly create or update the Claude entrypoints: `<path>/CLAUDE.md` and `<path>/.claude/CLAUDE.md`.
- `--gemini` — explicitly create or update `<path>/GEMINI.md`.
- `--copilot` — explicitly create or update `<path>/.github/copilot-instructions.md`.
- `--cursor` — explicitly create or update `<path>/.cursor/rules/grund.mdc`. A legacy `<path>/.cursorrules` is also updated if it already exists, but `--cursor` does not create one.
- `--windsurf` — explicitly create or update `<path>/.windsurfrules`.
- `--zed` — explicitly create or update `<path>/.rules`.

When no explicit agent-entrypoint flag is passed, `init` runs in automatic mode: update existing known agent entrypoint files if any are present; otherwise create workspace-triggered aliases for `.claude/`, `.gemini/`, `.cursor/`, or `.zed/` if those agent-specific directories already exist; otherwise create canonical `AGENTS.md`. When one or more explicit agent-entrypoint flags are passed, `init` writes exactly those requested entrypoint families and does not add automatic fallbacks.

Per [§FS-non-goals.10](FS-non-goals.md#10-interactive-mode), `init` is non-interactive: it never prompts. Every choice is a flag.

### 1.1 Usage examples

```
grund init                       # cwd, no docs tree
grund init path/to/repo          # auto-detect existing entrypoint, else AGENTS.md
grund init --docs                # cwd, with docs/ + e2e/ scaffolds
grund init --docs path/to/repo   # explicit target, with docs/ + e2e/ scaffolds
grund init --dry-run path/to/repo # preview without writing anything
grund init --claude --gemini path/to/repo # create/update both agent entrypoints
grund init --docs --name acme path/to/repo  # full form
```

Argument order is flexible: positional `<path>` may appear before or after the flags.

## 2. Outputs

### 2.1 Files written, updated, or left in place

In the default form (no `--docs`):

- Agent entrypoints — see §2.3. In automatic mode, existing known entrypoints are appended or updated in place: `<path>/AGENTS.md`, `<path>/AGENTS.override.md`, `<path>/CLAUDE.md`, `<path>/.claude/CLAUDE.md`, `<path>/GEMINI.md`, `<path>/.github/copilot-instructions.md`, `<path>/.cursor/rules/grund.mdc`, `<path>/.cursorrules`, and `<path>/.windsurfrules`, excluding companion symlinks to `AGENTS.md`. A companion symlink to `AGENTS.md` selects the canonical `AGENTS.md` target instead, including when the symlink is dangling because `AGENTS.md` has not been created yet. If none exist, missing aliases are created only when their owning agent-specific workspace already exists: `.claude/` creates `CLAUDE.md` and `.claude/CLAUDE.md`, `.gemini/` creates `GEMINI.md`, `.cursor/` creates `.cursor/rules/grund.mdc`, and `.zed/` creates `.rules`. If there are still no entrypoints to update or create, canonical `AGENTS.md` is written. `AGENTS.override.md`, `.github/copilot-instructions.md`, `.cursorrules`, and `.windsurfrules` are automatic existing-file-only — `AGENTS.override.md` is an override channel; `.github/` is generic GitHub metadata; `.cursorrules` is Cursor's legacy single-file form (the modern `.cursor/rules/` directory is preferred when creating new); and `.windsurfrules` is a root file with no companion directory to key off, so creating one requires the explicit `--windsurf` flag. `.rules` is workspace-gated *only* (never detected by file existence alone) because the filename is too generic to attribute to Zed by itself. Explicit flags (`--agents-md`, `--claude`, `--gemini`, `--copilot`, `--cursor`, `--windsurf`, `--zed`) create or update their requested entrypoints regardless of automatic detection.
- `<path>/.agents/grund.toml` — see §2.4. The `.agents/` directory is created if missing.

With `--docs`, additionally:

- `<path>/requirements.md`
- `<path>/docs/grund.md`
- `<path>/docs/goals.md`
- `<path>/docs/roadmap.md`
- `<path>/docs/changelog.md`
- `<path>/docs/architecture/README.md`
- `<path>/docs/decisions/architectural/.gitkeep`
- `<path>/docs/decisions/functional/.gitkeep`
- `<path>/e2e/README.md`
- `<path>/e2e/cases/.gitkeep`

Each scaffolded markdown file is a minimal starter — enough structure to teach the layout, no real content:

- `grund.md` — the H1 plus a one-line note on how the project's reason for being is declared inline (`# GRUND-NNN-slug: …`), then the three H2 sections (`## 1. The problem`, `## 2. What this project does about it`, `## 3. Who it is for`), each with a one-line italic prompt to be replaced.
- `goals.md` — the H1 plus a one-line note on how goals are declared inline (`# GOAL-NNN-slug: …`).
- `requirements.md` — the H1 plus a one-line note on how `FS-` requirements/spec IDs are declared inline as H2 headings in that file.
- `roadmap.md`, `changelog.md` — the H1 plus a single `<!-- placeholder - replace with real content -->` line.
- `architecture/README.md` — the H1, the navigational note about how `AR-` IDs declare into the directory and the convention that the index lists every architecture point, and an empty `| ID | Subject |` table to fill in.
- `e2e/README.md` — the H1 (`# e2e`) plus a one-line note that every behaviour described in the effective FS home has at least one case.

The exact bytes for a given `grund` version are embedded in the binary; reference copies live under `templates/` in the `grund` source tree, and two `grund init --docs` runs at the same version with the same `--name` produce byte-identical scaffolds ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). `grund check` is clean against the freshly-scaffolded tree. The `.gitkeep` files exist solely so the empty directories survive a `git add`; their content is a single line: `# placeholder — replace this directory's contents with real declarations`.

### 2.2 Stdout / stderr

On success, stderr lists every path touched, one per line:

```
wrote AGENTS.md
wrote .agents/grund.toml
```

The prefix is `wrote ` for a newly written or overwritten file, including a workspace-triggered companion alias; `appended ` when the versioned `grund` block was added to an existing agent entrypoint; `updated ` when an existing managed block's bytes changed (an older block upgraded, or a same-version block re-rendered against a changed template or config — §2.3); and `exists ` when an existing file was left unchanged — including an agent entrypoint whose managed block already matches the current rendered block byte-for-byte, in which case `init` rewrites nothing, so re-running `grund init` on an up-to-date repo touches no files. Under `--dry-run`, those three write prefixes are replaced with `would-write `, `would-append `, and `would-update ` and nothing is written to disk; `exists ` lines are identical to a non-dry run. A companion file that is a symlink to `AGENTS.md` is omitted from the output; when the canonical file is selected, that update already covers the symlink target.

After the file list, stderr prints a short `next:` block — a blank line, then numbered first steps, then `see <entrypoint> for the full workflow.` — so the user is not left at a bare list of paths wondering what to do. `<entrypoint>` is the first agent entrypoint `init` wrote, updated, or found already current. The steps are: run `grund check` (a fresh tree is clean); allocate an ID with `ID=$(grund id FS "…")` and add it to the effective FS home (`requirements.md` with the `## <ID>: …` H2 for generated defaults, or under the configured FS folder with `# <ID>: …` when compatibility or explicit config makes FS a folder); cite it as `§<ID>` from the docs and e2e tests that depend on it. When `--docs` was *not* passed, step 2 instead points at re-running with `--docs` (or creating the effective FS home, `docs/`, and `e2e/` by hand). The `next:` block is suppressed entirely when every reported path is `exists ` (or, under `--dry-run`, every reported path is `exists ` and no `would-…` lines were emitted) — the user already has a complete, current grund setup, so there is no next step to teach. This block is guidance, not a finding; when it is printed, it is part of the success output and does not change the exit code.

Paths are relative to `<path>`. Stdout is always empty (consistent with [§GOAL-friendliness-first.1](../goals.md#1-hard-requirements) — output that other tools might pipe stays clean).

### 2.3 Generated agent entrypoints

The emitted agent guidance is a canonical managed block: it teaches the session-start workflow and rules listed in §2.3.4, rendered against the target repo's effective configuration. The block stays concise under [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file): it teaches only the rules an agent needs before work begins, leaving detail to cited specs and `grund <ID>` output. The canonical text for a given block version `vN` is embedded in the `grund` binary; the reference copy lives at `templates/AGENTS.md` in the `grund` source tree, and the `vN` marker (§2.3) is what versions it under [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — so changing the taught workflow is itself a block-version bump, carried by that mechanism, not a silent rewrite.

Several things in the block are *substituted in* rather than fixed for that `vN`, so the file describes the repo it is in: the project name from `--name` (interpolated into the scaffolding H1 emitted above the block for a fresh `AGENTS.md`), and the **effective ID grammar and artifact map** — taken from the `.agents/grund.toml` `init` leaves governing the target (an existing config in the target, or the defaults `init` is about to write, never an ancestor's). From that config the block fills in the ID shape (`<KIND>-<NNN>-<slug>`, `<KIND>-<slug>`, …, derived from `[id].format`), one worked example ID and citation, the `[id].section_separator`, the marker and `$$`-trigger from `[reference]`, the `KIND ∈ {…}` set from `[[kinds]]`, a raw-readable link list of each kind's configured declaration home and title, a sentence on whether bare ID-shaped tokens count as citations (driven by `[reference].strict`), and the inline citation style sentence defined by [§FS-inline-citation-style.5](FS-inline-citation-style.md#5-agent-facing-rendering). The worked example citation is written in the escaped illustration form — `<§>` around the configured marker, per [§FS-workspace.1](FS-workspace.md#1-citation-syntax) — never as a live citation: the example ID is deliberately not a real declaration in the host repo, so a live marker would make the freshly generated block fail the host repo's own `grund check` as a dangling reference wherever the entrypoint falls inside the scan scope. Generated `init` output must pass `grund check` unmodified — the generator and the checker are never allowed to disagree about the block's own contents. The contract this spec makes is the *determinism and versioning*, not a literal transcript: two `grund init` runs at the same `grund` version against trees with the same `--name` and the same effective config produce byte-identical managed blocks ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)), and `grund check`'s agent-entrypoint validation ([§FS-check.3.5](FS-check.md#35-invalid-agent-entrypoint-init-block)) checks the marker line and the version, not a byte-diff against the canonical text.

The managed block is bounded by explicit standard `BEGIN` / `END` HTML-comment delimiters, with an H2 heading just inside the opening delimiter carrying the schema version:

```markdown
<!-- BEGIN GRUND MANAGED BLOCK -->
## Grounding with grund (vN)
…
<!-- END GRUND MANAGED BLOCK -->
```

The integer after `v` is the agent guidance block schema version. The `<!-- BEGIN GRUND MANAGED BLOCK -->` line is the block's begin marker and `<!-- END GRUND MANAGED BLOCK -->` is its end marker; both delimiter lines belong to the managed region, so everything between them — delimiters included — is `init`'s to rewrite and nothing outside them is. The delimiters are deliberately the conventional managed-region shape rather than a grund-specific arrow syntax, so humans and agents skimming the file recognize the ownership boundary without knowing grund ([§DF-managed-block-delimiters](../decisions/functional/DF-managed-block-delimiters.md#df-managed-block-delimiters-standard-beginend-delimiters-for-the-managed-agent-instructions-block)). Blocks at v3 and earlier predate the delimiters (**legacy blocks**): there the H2 heading itself is the begin marker and the block runs until the next H1 or H2 heading, or end of file. Both `init` and `check` continue to recognize the legacy form; on its next write `init` migrates a recognized legacy block to the delimited form in place (reported `updated `), preserving the block's position and every byte outside the block.

A fresh `AGENTS.md` consists of this block preceded by a one-line scaffolding H1 (`# {NAME} — agent instructions`); the H1 is *unmanaged*, so `init --force` rewrites the block but leaves the title alone. A freshly-created companion entrypoint consists of just the managed block, with no extra H1, so the agent-specific file remains a thin alias for the canonical workflow. If an agent entrypoint already exists and contains no managed block, `init` appends the current block after the existing content (no H1 is inserted — the host file owns its title). If the file already contains any supported block version, including the current one, `init` re-renders the block from the current effective `.agents/grund.toml`, compares it to the bytes of the managed region on disk (delimiter-bounded, or marker-line-to-next-H1/H2 for a legacy block), and — when they differ — replaces only those bytes, leaving all content before and after the block untouched (reported `updated `). This means same-version template/config changes propagate on the next `grund init` without requiring `--force`. When the re-render is byte-identical to what is on disk, `init` writes nothing and reports the file with `exists ` (§2.2) — re-running `grund init` on an already-current repo is a no-op on every file. If the file contains a newer block version than the running binary supports, `init` exits 2 and leaves the file unchanged.

A file whose delimiters are present but broken is **malformed**: a `BEGIN` delimiter with no `END` after it, an `END` with no `BEGIN` before it, more than one `BEGIN`, or a delimited region that contains no `## Grounding with grund (vN)` version heading. `init` refuses to touch a malformed file — it exits 2 with a message naming the file and the specific defect, and rewrites nothing — because any splice against broken delimiters risks eating user content; `grund check` reports the same defect as an error ([§FS-check.3.5](FS-check.md#35-invalid-agent-entrypoint-init-block)). Fixing the delimiter pair by hand (or deleting the remnants of the block) and re-running `grund init` is the recovery path.

`AGENTS.md` is the canonical fallback entrypoint. Companion entrypoints are mostly discovery-based: `init` updates them if the repo already has them, and creates the neutral aliases for Claude, Gemini, Cursor, or Zed only when the matching agent-specific workspace directory already shows that tool is in use. The built-in companion set covers common root or repository instruction files used by agent-specific tooling: Codex override instructions (`AGENTS.override.md`), Claude Code (`CLAUDE.md`, `.claude/CLAUDE.md`), Gemini (`GEMINI.md`), GitHub Copilot (`.github/copilot-instructions.md`), Cursor (`.cursor/rules/grund.mdc`, plus the legacy `.cursorrules`), Windsurf (`.windsurfrules`), and Zed (`.rules`). Since `.github/` is also used for Actions, issue templates, and other non-Copilot metadata, automatic mode does not create `.github/copilot-instructions.md` from directory existence alone; use `--copilot` to create it explicitly. `.windsurfrules` is similarly a root file with no workspace directory to key off, so first-time creation requires the explicit `--windsurf` flag. `.rules` (Zed) is created only when `.zed/` already exists or `--zed` is passed — file existence alone is not enough, because `.rules` is too generic a filename to attribute to Zed. When a companion path is a symlink to `AGENTS.md`, `init` does not touch it separately. If that companion is explicitly requested, the request selects canonical `AGENTS.md` as the update target; when `AGENTS.md` is part of the selected entrypoint set, `grund check` treats the canonical `AGENTS.md` block as sufficient for that symlink.

#### 2.3.1 Position invariants

The managed block's **position within an existing agent entrypoint is preserved on update**. `init` does not move the block: a supported block sandwiched between user-authored sections is replaced in place with the current rendered block, with the surrounding sections — both before and after — left byte-identical. When the block is already byte-identical to the current render, nothing is rewritten at all (`exists `, §2.2) — the strongest form of "position preserved."

#### 2.3.2 Line endings

`init` does not normalize line endings. When updating or reading an existing agent entrypoint:

- The bytes outside the managed block (everything before the `<!-- BEGIN GRUND MANAGED BLOCK -->` line and everything after the `<!-- END GRUND MANAGED BLOCK -->` line — or, for a legacy block, everything before the `## Grounding with grund (vN)` heading line and everything after the next H1 or H2) are preserved byte-for-byte, including CRLF (`\r\n`) and lone-CR endings.
- The delimiter and H2 marker matching tolerates an optional trailing `\r` before the line end, so a CRLF-encoded file is detected correctly.
- The freshly-written block uses LF endings (the bytes embedded in the binary). On a CRLF-encoded host file the result is mixed line endings inside the managed region and CRLF outside; this is intentional. Normalizing the rest of the file would violate the "leave content alone" guarantee.

#### 2.3.3 Citation form

The managed block teaches a single citation form — `§<ID>`, bare, with an optional `.<section>` path. `[fmt.cross_refs].enabled` per [§DF-md-link-emission.2.4](../decisions/functional/DF-md-link-emission.md#24-default-on-for-generated-configs) governs whether `grund fmt --write` wraps on-disk citations in Markdown links; the managed block's guidance is the same in either mode — cite specs by ID; any Markdown wrap is generated by `grund fmt`, not hand-authored.

The generated file uses the canonical `grund` reference grammar even before any IDs exist in the repo — citations of `§GOAL-no-dangling-refs`, `§FS-check`, etc. inside the boilerplate point at the **grund project's own** documentation, not the new repo's. This is intentional: the boilerplate is a teaching surface, and the IDs anchor the teaching to a stable source. The generated `.agents/grund.toml` (§2.4) sets `[scan] include = ["requirements.md", "docs", "e2e", "src"]` so the generated single-file FS home is scanned while the boilerplate's pedagogical citations in the agent entrypoint are not themselves scanned (known entrypoints live outside `include`) — the citations remain inert text in the host repo and never flow into its findings.

#### 2.3.4 Managed-block content points

The generated managed block is not a literal transcript in this spec, but its canonical template must preserve the following separately citeable content points. Each point is phrased compactly in the template, because the entrypoint is read at session start and serves [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file).

##### 2.3.4.1 Entrypoint

The block identifies itself as the entrypoint for agents working in the project and instructs them to read it, then read the declared artifacts it points to, before making changes.

##### 2.3.4.2 Reference Scheme

The block teaches the configured reference scheme: IDs have the configured shape, citations use the configured marker and optional section path, `grund check` validates citations, `grund list` discovers IDs, `grund <ID>` reads cited declarations or sections, and `grund refs` reports what leans on a declaration.

##### 2.3.4.3 Cheap Grounding

The block teaches the cheap grounding ladder: use `grund <ID>` as the first read for a bare citation, `grund <ID> --toc` when section navigation is needed, `grund <ID>.<section>` for section citations, `grund <ID> --full` only when a narrower read is insufficient, `grund list --kind FS,AR` for scoped discovery, and `grund refs <ID> --summary` before a full back-reference listing.

##### 2.3.4.4 Project Map

The block describes the configured declaration homes from the effective `.agents/grund.toml` as a raw-readable Markdown link list (`- [KIND](home): Title`), so the generated instructions name and link the host repo's actual artifact layout instead of hard-coding the default layout or relying on table rendering. The scan scope (`[scan].include` / `[scan].exclude`) is *not* surfaced here — it is configuration an agent never needs to read inline, since `grund <ID>`, `grund list`, and `grund refs` apply it transparently. When the effective config declares `[workspace]`, the sibling §2.3.4.15 block names the workspace projects; the Project Map itself describes the *current* project's declaration homes only and is unchanged in workspace mode.

##### 2.3.4.5 Declaration Forms

The block teaches that declarations are heading lines in markdown or supported language doc-comments, and that an inline source declaration can be represented by a one-line markdown stub in the configured kind home. It also teaches, rendered according to `[id] section_heading_levels`, that numbered headings inside a declaration are citable sections; depth-matching headings (`## 1. …`, `### 1.1 …`) are required in strict mode, warned on in warn mode, and recommended in loose mode. Plain headings and bold labels are allowed for non-citable local structure ([§FS-config.3.3](FS-config.md#33-section-paths--arbitrary-nesting-depth), [§FS-check.3.9](FS-check.md#39-section-heading-level-mismatch)).

##### 2.3.4.6 Spec First

The rules tell agents to write or update the most-specific functional or architectural spec point before writing behavior or design code that implements it.

##### 2.3.4.7 Most-Specific Citations

The code back-reference guidance tells agents to cite the most-specific spec point the code or prose realizes: whole behavior on the function, class, or block doc-comment; narrower clauses or decisions inline where they are enforced.

##### 2.3.4.8 Refresh Before Editing

The rules tell agents to refresh the cited spec with `grund <ID>` before editing code that already carries a `§<ID>` or `§<ID>.<section>` citation.

##### 2.3.4.9 Declaration Blast Radius

The rules tell agents to run `grund refs <ID> --summary` before changing, moving, or renaming a declaration, and to use the full `grund refs <ID>` output when exact citation sites are needed.

##### 2.3.4.10 Citation Direction

The rules teach the expected citation direction: specs cite goals, architecture cites specs, code cites the specs it implements, and executable tests or cases cite the behavior they verify. When the effective `.agents/grund.toml` declares `[citations]` ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)), this guidance is the generated, config-derived Citation directions section (§2.3.5) rather than the static sentence above — so the rule an agent reads is the rule `grund check` enforces. A config without `[citations]` keeps the static sentence.

##### 2.3.4.11 Decisions

The rules tell agents that decisions must be cited from the spec or architecture point they shaped, and that decision history is append-only: reversals are new decisions that supersede older ones rather than rewrites.

##### 2.3.4.12 Cross-Linking

The block tells agents to cross-link specs only via IDs — a single short sentence in the citations narrative, not phrased as a rule. The `[fmt.cross_refs].enabled` mode (§2.3.3) does not change this wording.

##### 2.3.4.13 Executable Proof

The rules tell agents that behavior is proven by executable tests or cases, and that disagreements between the spec and executable proof require fixing both in the same change.

##### 2.3.4.14 Final Check

The rules tell agents to run `grund check` before committing, because dangling references are stop-the-line bugs whose diagnostics name the file and line.

##### 2.3.4.15 Workspace Members

When — and only when — the effective `.agents/grund.toml` at the init target declares `[workspace]` ([§FS-workspace.2](FS-workspace.md#2-workspace-configuration)), the block emits a `### Workspace members` section as a sibling to the Project Map (§2.3.4.4). Effective config here follows the same walk-up rule [§FS-config.1](FS-config.md#1-file-location-and-discovery) defines, but does not stop at the target's own `.agents/grund.toml` if that file has no `[workspace]` block — the search continues up to the nearest ancestor that does. The section is therefore emitted whether `init` runs at the workspace root or inside a member directory. The alias list, ordering, and discoverability line are identical across runs against the same workspace; what differs by run is purely local-perspective rendering (paths in link targets, and which row is "self" — see below).

The section contains exactly two things:

1. One discoverability line, verbatim except for the configured citation marker: `Cross-project citations use <marker>alias/<ID>.` This is the briefest surface that teaches the workspace-citation grammar to an agent landing at the entrypoint cold. The marker is the target project's effective `[reference].marker`, the same value used by the citation example in §2.3.4.1; the full grammar lives in [§FS-workspace.1](FS-workspace.md#1-citation-syntax), and the block does not duplicate it.
2. One Markdown bullet per resolved workspace project, sorted by alias. Members in `[workspace] members = [...]` are expanded the same way `grund check` expands them (single-segment trailing `*` globs, hidden directories skipped, [§FS-workspace.2](FS-workspace.md#2-workspace-configuration)). Aliases are derived per [§FS-workspace.3](FS-workspace.md#3-aliases) — `project_name` when set, otherwise the member directory's basename for a member or the literal `root` for the workspace root. The root project appears as a row in the list when `include_root = true` and is omitted when `include_root = false`; the root is rendered with the same `alias → AGENTS.md` shape as a member, not specially.

Each bullet renders the backticked alias as the *label* of a Markdown link, so the destination path appears exactly once — `` - [`api`](apps/api/AGENTS.md) `` — the same raw-readable `- [x](y): …` list grammar the Project Map (§2.3.4.4) uses; a raw-bytes reader still sees the path in the link destination, and token cost stays minimal ([§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file)). When the project has a `project_description` in its own config ([§FS-config.3](FS-config.md#3-schema), [§FS-workspace.3](FS-workspace.md#3-aliases)), the bullet appends `: <description>` (colon, space) after the link, before any trailing marker; a project without one keeps the link-only bullet byte-identical, and `init` never derives a description it was not given ([§DF-workspace-member-descriptions](../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists)). When the project's `AGENTS.md` exists on disk at the time `init` runs, the link target is `<member-root>/AGENTS.md`. When it does not, the link target is the member root directory (with a trailing `/`) and the bullet ends with the literal trailing marker `*(not yet initialized)*` — so an agent reading the block never follows a 404 link, and the missing entrypoint is surfaced as actionable information (running `grund init` inside that directory is the next step). Paths in link targets are emitted relative to the directory where the entrypoint being written lives, so a workspace-root AGENTS.md points at `apps/api/AGENTS.md` while the same section emitted inside `apps/api/AGENTS.md` points at `../../apps/api/AGENTS.md`. Existence is checked after path canonicalization; symlinks count. One exception: when the current `init` invocation is actually writing the canonical `AGENTS.md` for the self project, that self row is rendered as initialized, since the file is about to exist when the run finishes. Companion-only init runs do **not** take this exception: if they write `CLAUDE.md`, `.claude/CLAUDE.md`, `GEMINI.md`, or another non-canonical entrypoint without writing `AGENTS.md`, and the project's `AGENTS.md` is absent, the self row still links to the project directory and carries `*(not yet initialized)*`. If that self project has no `.agents/grund.toml` yet, its alias is derived from the config `init` is about to write, including `project_name = "<name>"` from `--name`, rather than from the directory basename; the same pending config supplies `project_description = "<text>"` from `--description`, so the self row's description appears in the same run that writes it. So in a member-side `init` that writes a previously-absent `apps/api/AGENTS.md` with `--name service`, the `service` row is initialized and any other uninitialized projects (including the workspace root, if its `AGENTS.md` is missing) still carry the marker.

If workspace expansion, alias validation, alias uniqueness, or nested-workspace validation would fail under the same rules as `grund check`, `init` suppresses the Workspace members section instead of emitting partial or ambiguous guidance. `grund check` remains the command that reports the configuration error.

`init` does **not** prompt for, infer, or configure workspace topology. It does not add a `[workspace]` block to a config that lacks one. It does not write or modify any file under a member directory other than the member it was invoked in — even when invoked at the workspace root, the Workspace Members section is the only artifact `init` produces about sibling members; bootstrapping a member's own `AGENTS.md` remains a separate `init` invocation inside that member. Out of scope for v1: a reciprocal member→root pointer, and any collapse-to-alias-only rendering for very large workspaces. This serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) by making the cross-project citation scope visible at the entrypoint an agent reads first, without breaking the no-prompts-no-surprises contract in §3.

##### 2.3.4.16 Project Namespaces

The block always emits a `### Project namespaces` section that teaches agents the workspace namespace concept before they start creating or citing declarations. It distinguishes project namespaces from documentation folders; names the current project as the local namespace; tells agents to create or use a separate namespace only for independently checked apps, packages, services, or subprojects; and gives the operational steps for that split: create the member's `.agents/grund.toml`, add it to the workspace root's `[workspace] members`, run `grund init` in the member, and set a stable `project_name`. It also teaches the cross-namespace citation form `<marker>alias/<ID>` and says full cross-namespace validation runs from the workspace root with `grund check` ([§FS-workspace.1](FS-workspace.md#1-citation-syntax), [§FS-workspace.2](FS-workspace.md#2-workspace-configuration), [§FS-workspace.5](FS-workspace.md#5-command-scope)).

##### 2.3.4.17 Clickable Citations

The block teaches how a `§<ID>` citation renders in conversation as a rendering of the effective `[render.links].conversation` policy ([§FS-config.3.10](FS-config.md#310-renderlinks--clickable-citation-rendering)): `plain` tells agents to write the bare citation and names `grund integrations` as its rendering layer; `link` tells them to wrap it in a context-valid declaration link, falling back to plain when uncertain ([§DF-neural-link-generation](../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command)). The full recipe, rationale, and test matrix stay in the cited decision; the managed block carries only the compact convention.

#### 2.3.5 Citation directions

When the effective `.agents/grund.toml` declares `[citations]` ([§FS-config.3.9](FS-config.md#39-citations--citation-direction-rules)), the managed block renders a `### Citation directions` section generated from those rules, replacing the static citation-direction sentence (§2.3.4.10). This is the strongest enforcement point for the `should` levels, which never appear in `grund check`'s standing output ([§FS-check.2.3](FS-check.md#23-suggestions-channel-opt-in)): the agent reads the rule before writing the declaration, so most `should` obligations are met at write time. Decided in [§DF-citation-directions.2.7](../decisions/functional/DF-citation-directions.md#27-generated-agent-entrypoint-section-with-a-drift-check).

The section renders a top-level default sentence when the global default changes the open-world behavior, then one bullet per citing kind that has any rule, in `[[kinds]]` order with the `code` pseudo-kind last. Rendering is deterministic: explicit levels render as the fixed phrases *must cite* / *should cite* / *may cite* / *avoid citing* / *never cite*, a `|` disjunction renders as "or", and conjunctive entries render with "and". Per-kind defaults render in the same bullet as an unlisted-citation clause, so closed-world configs such as `default = "must-not"` plus explicit `may` entries tell agents which targets are allowed. When no rendered default changes the open-world behavior, the section ends with the verbatim line `Unlisted kinds and pairs are fine.` — load-bearing, so an agent does not over-infer prohibitions from silence. Otherwise it ends with `Unlisted kinds and pairs follow their configured defaults.`

Because this content derives from config rather than the template alone, the version marker (§2.3) is no longer sufficient to detect staleness: editing `[citations]` without re-running `grund init` would leave guidance that disagrees with the live rules under a current version number. `grund check` therefore re-renders this section from the live config and byte-compares it against the section in the block; a mismatch is an `agents-init` finding ([§FS-check.3.5](FS-check.md#35-invalid-agent-entrypoint-init-block)) telling the author to re-run `grund init`. Rendering determinism is what makes the comparison sound — the render *is* the hash. The managed-block version was bumped to carry this content change under [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path).

#### 2.3.6 Clickable citations

The managed block renders a `### Clickable citations` section from the effective `[render.links].conversation` policy ([§FS-config.3.10](FS-config.md#310-renderlinks--clickable-citation-rendering)), carrying the content point in §2.3.4.17. Rendering is deterministic: the value selects one fixed plain or link sentence, so the same effective config produces byte-identical text ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). A config with no `[render.links]` table renders the plain default, so every repo carries the convention.

Like §2.3.5, this content derives from config, so the version marker alone cannot detect drift: editing `[render.links]` without re-running `grund init` would leave stale guidance under a current version number. `grund check` re-renders this section from the live config and byte-compares it against the block; a mismatch is the same `agents-init` finding ([§FS-check.3.5](FS-check.md#35-invalid-agent-entrypoint-init-block)) telling the author to re-run `grund init`. Adding this config-derived section is the content change that bumps the managed-block version to **v4**, carried under [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path); a v3 block is reported outdated by `grund check` until `grund init` re-renders it.

### 2.4 Generated `.agents/grund.toml`

`init` writes this file **only when it is absent**. An existing `.agents/grund.toml` is the repo's configuration — the one surface a project customizes ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)) — and `init` never overwrites it, not even with `--force`: an existing config is reported as `exists` and left byte-for-byte unchanged (§3). `--force` resets the things `init` owns end to end — the managed `AGENTS.md` block and the `--docs` scaffold stubs — not the user's settings; a customized config that `init --force` clobbered would be a footgun. (When the file is absent and `init` does write it, every key written matches the built-in default for that key — including `FS` as `file = "requirements.md"` — so the file is a teaching surface, not an override surface, and a new user can see the schema they will be editing. Top of file is `grund_config_version = 1` per [§FS-config.5](FS-config.md#5-schema-versioning); the schema written is exactly the one in [§FS-config.3](FS-config.md#3-schema), no extra keys, no missing keys.)

Because the generated config is also a usability surface, non-boolean keys with a finite accepted value set carry inline comments listing that set. Boolean keys are left uncommented: spelling out `true | false` is not useful guidance and should not be emitted. Free-form keys instead describe their grammar or omit the comment when a finite list would be misleading. For example, `[reference].inline_style` names `citation-with-note | citation-only`, `[id].section_heading_levels` names `strict | warn | loose`, `[output].format` names `text | json`, and `[fmt.cross_refs].anchor_format` names `github | gitlab | mkdocs | pandoc | none`. These comments are explanatory only; they are not schema keys and do not change parsing under [§FS-config.3](FS-config.md#3-schema).

A single `project_name = "<name>"` key appears at the top above the section tables. This key is metadata only — it is not consumed by any other `grund` subcommand and exists so downstream tooling (IDE status bars, CI dashboards) can read it without re-deriving the name. Directly below it the file teaches the optional `project_description` key ([§FS-config.3](FS-config.md#3-schema)): a commented `# project_description = "<one line shown next to this project in workspace member lists>"` line by default, or the real `project_description = "<text>"` key when `--description` was given (§1). The commented form is a teaching comment like the finite-value-set comments below, not a schema key, so the "no extra keys, no missing keys" guarantee above is unchanged.

## 3. Non-intrusive guarantees

A reader skimming for "what won't `init` touch?" gets the consolidated answer here; the detail lives at the section cited beside each guarantee.

- **Automatic mode never adds a competing entrypoint.** An existing `CLAUDE.md`, `GEMINI.md`, or other known entrypoint is updated in place; no canonical `AGENTS.md` is invented alongside it (§1, §2.1).
- **Ambiguous companions need a workspace signal.** `.github/copilot-instructions.md` is never created from `.github/` alone (it is generic GitHub metadata); `.rules` is never created from file existence alone (its filename is too generic) — both require either an existing workspace directory or an explicit flag (§2.1, §2.3).
- **User-authored content in agent entrypoints is preserved.** Only the delimiter-bounded managed `## Grounding with grund (vN)` block is touched. Everything before and after the block is byte-for-byte preserved, including the block's position within the file (§2.3, §2.3.1).
- **Line endings outside the managed block are preserved.** CRLF, lone-CR, and mixed-encoding host files keep their endings; `init` never normalizes the surrounding bytes (§2.3.2).
- **`.agents/grund.toml` is never overwritten.** Once the config exists it is the project's, not `init`'s; `--force` does not touch it. The file is only ever written when it is absent (§2.4, below).
- **No prompts, ever.** Every choice is a flag; there is no interactive mode that can surprise a human or break a script ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)).
- **`--dry-run` previews any run.** The user can see exactly which lines `init` would emit — `would-write`, `would-append`, `would-update`, `exists` — before letting it touch a single file (§1, §2.2).
- **Re-running is a true no-op when the repo is already current.** Every reported path is `exists `, no bytes change on disk, and the trailing `next:` guidance block is suppressed because there is nothing left to teach (§2.2, §2.3).

The remainder of this section gives the detailed `--force` semantics that back the `.agents/grund.toml` and "refuse to clobber" guarantees.

Without `--force`, `init` never overwrites an existing file. Existing agent entrypoints are handled by the append/update rules in §2.3. Every other existing target file is left unchanged and reported with `exists `:

```
exists .agents/grund.toml
```

This makes repeated `grund init` runs idempotent and safe for existing repos. With `--force`, a selected canonical `AGENTS.md` and the `--docs` scaffold files are overwritten in place; their previous contents are not preserved (the user has git for that, per [§FS-non-goals.6](FS-non-goals.md#6-decision-database-audit-log-history-tracking) — `grund` does not maintain its own history). **`.agents/grund.toml` is the exception: `--force` never overwrites it.** Once the config exists it is the project's, not `init`'s — `init --force` still reports it as `exists ` and leaves it untouched (§2.4); the file is only ever written when it is absent. Existing agent entrypoints are likewise not full-file overwritten by `--force`; if present and not symlinked to `AGENTS.md`, only their managed block is appended or updated so unrelated agent-specific instructions remain intact.

The `--docs` mode applies the same rule across the docs tree: existing scaffold files are reported as `exists ` and left unchanged; missing scaffold files are written.

## 4. Exit codes

- `0` — every requested file was written, appended, updated, or already current.
- `2` — I/O error (target path does not exist, permission denied, disk full, etc.); a CLI-level error such as an unknown flag; or an existing `AGENTS.md` contains a managed block whose schema version is newer than the running binary supports (§2.3), in which case the file is left unchanged.

Exit-code mapping is fixed per [§GOAL-friendliness-first.2](../goals.md#2-what-this-rules-out) and [§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization).

## 5. Agent setup instructions

`grund agent-setup-instructions` prints the AI-agent-facing guided setup instructions for adopting `grund` in an arbitrary repo. This is a read-only discovery command: stdout is Markdown, stderr is empty, exit `0`; passing any positional argument or flag is a CLI-level error (`error: agent-setup-instructions takes no arguments`, exit `2`). The command exists so a user can tell an agent only "set up grund" plus provide an installed `grund` binary, and the agent can still discover the recommended setup workflow without browsing the source repository.

The output reads like an agent skill rather than a human tutorial. It instructs the agent to inspect the target repo first, identify the existing specs, artifact types, roadmaps, changelogs, decisions, plans, tests, and agent instruction files, recommend suitable `grund init` and `.agents/grund.toml` choices with evidence, show pros and cons for every config option, ask the user to confirm or override the recommendations, write `.agents/grund.toml`, run `grund init`, then validate with `grund config validate` and `grund check`. The config must be written before `grund init` so the generated managed block reflects the selected ID grammar, marker, strict mode, kinds, and existing artifact layout.

For an existing docs-heavy repo, the instructions make the adoption choice explicit before any write: show the canonical `grund` artifact types, including `GRUND` for the grounding doc, beside the detected project-specific sections, tags, or document classes, then ask whether to use canonical `grund`, canonical core plus project-specific extras, or the existing structure with `grund` citations. The recommended `grund init` form omits `--docs` unless the user selects a canonical-layout migration; existing specs are represented in `[[kinds]]` and `[scan]` settings rather than replaced by generic scaffold folders. This follows [§DF-skill-init-existing-specs](../decisions/functional/DF-skill-init-existing-specs.md#df-skill-init-existing-specs-grund-init-adopts-existing-specs-before-scaffolding).

The core instructions must not fork from the distributable skill. The repository keeps the distributable skill at `skills/grund-init/SKILL.md` and the binary-embedded copy at `crates/grund-core/assets/skills/grund-init/SKILL.md`; they must be byte-identical, and `grund agent-setup-instructions` prints that Markdown source byte-for-byte. The source package therefore exposes the instructions in two ways with one body: agents that can read the repository may load `skills/grund-init/SKILL.md` as a skill; agents that only have the installed CLI may run `grund agent-setup-instructions`. A release that edits one surface without the other is invalid.

## 6. Why this exists

`init` is the smallest on-ramp `grund` offers: one command, every default the most-common case, nothing the user already authored rewritten without `--force` (§3). The shape follows from three goals taken together — an agent that reads its entrypoint file at session start should arrive already taught ([§GOAL-agent-grounding.1](../goals.md#1-the-three-layers)); a conformant tree should work without configuration so the emitted defaults *are* the canonical grammar ([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)); and the on-ramp should not surprise either humans or scripts, so there are no prompts and every choice is a flag ([§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible)).

Without `init`, the on-ramp to `grund` would be a copy-paste of someone else's agent instructions, with the resulting drift between projects. `init` collapses that on-ramp to one command and freezes the selected agent entrypoint at the `grund` version that wrote it — when the canonical form evolves, `grund init` re-emits the managed block in place ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) still applies for downstream edits).

`init` is also the only safe place to demonstrate the ID grammar before any IDs exist: a repo that has not yet authored its first `FS-001-…` declaration still gets a literate agent entrypoint from `init` that teaches the grammar by example.
