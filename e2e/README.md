# e2e

End-to-end tests for `grund`. Each case is a tiny repository plus golden command results. The Rust integration test in `tests/e2e.rs` discovers every directory under `e2e/cases/` and runs the built `grund` binary.

## Case layout

```
e2e/cases/<case-name>/
├── repo/
│   └── ... files scanned by grund ...
├── spec.refs
├── expected.exit
├── expected.stdout
└── expected.stderr
```

`spec.refs` is required. Every non-empty line must cite a functional spec ID such as `FS-001-check.3.1`; the harness rejects cases that do not cite the behavior they exercise.

### The `symlinks` manifest

An optional `symlinks` file adds links the fixture cannot carry in git — git on Windows checks a committed symlink out as a text file holding its target unless developer mode is on, so the fixture would be a different tree and the golden would fail for a reason the case is not about. The links are created in the copied tree at run time, which makes the file a contract the harness enforces line by line; every rejection names the case and the manifest line.

- One link per line, `<link> -> <target>`, with `->` appearing **exactly once**. Blank lines and lines starting with `#` are ignored.
- Both paths are relative to the fixture repo, `/`-separated. The **link** path must stay inside the copy: no absolute form, no `..`, and no `\`. The **target** is free to leave it — `link -> ..` is exactly what one case tests.
- The **kind** of link is not written down: the harness resolves the target against the link's own directory and creates a directory link where it lands on a directory, a file link otherwise (a target that does not exist is a file link). Unix has one kind and ignores this; Windows stores the kind in the link, and one made with the wrong kind does not resolve, so a fixture's file links looked like unreadable paths to `grund` and the case exited `2` where its golden said `1`. A link whose target existed and still does not resolve fails the case at creation, naming the fixture rather than the golden.
- The manifest must declare at least one link. An empty one used to yield no links, no diagnostic, and no skip, so a case could be green with a dead manifest.
- The case must run against `{repo_copy}`. Only that branch copies the fixture and creates the links, so a manifest case written against `{repo}` tested the committed tree while claiming to test a symlinked one.

A case that does not run is **not** a case that passed. The harness probes the directory it actually creates the links in (`target/e2e-work/`, not the system temp directory), and at the end of each pass it prints every case it skipped with the reason and the count. On a platform that can create a directory symlink a skip is a hard failure, because there it means the harness lost the coverage rather than the platform refusing it.

`expected.exit` contains `0`, `1`, or `2`. `expected.stdout` and `expected.stderr` are compared byte-for-byte, except that a file containing only one newline is treated as empty so empty golden files can be represented cleanly in patches.

Most cases run `grund check <repo>`. A case may override the command with `command.args`; use `{repo}` for the fixture repo path. For write-mode tests, use `{repo_copy}` so the harness copies the fixture under `target/e2e-work/` before running the command.

Error output is part of the contract. Non-zero cases should keep `expected.stderr` concise: one actionable diagnostic per line, no aggregate footer, and no long explanatory prose that makes editor and agent consumption harder.

## Current coverage

- basic Markdown valid references
- dangling Markdown citation
- missing Markdown section
- duplicate Markdown declaration
- two headings claiming one dotted section path: `check` naming both lines, `show` refusing the coordinate rather than merging the two bodies, and `--toc` over the whole declaration still mapping both while `--toc` on the ambiguous coordinate itself refuses
- the same two headings written inside a fenced Markdown example: `check` silent and `show` returning the section whole, fence included — the shape every document in this repository is made of
- the ambiguous-section refusal in JSON, under its own `ambiguous-section` code rather than the ambiguous-ID `ambiguous`
- two headings claiming one path inside one Rust doc-comment, beside a heading in the *next* item's doc-comment and a stub whose prose repeats one: only the collision inside the declaration's own body is reported
- fenced Markdown examples ignored under strict mode, for matched backtick and tilde fences
- marker-prefixed citations
- optional-mode bare citations
- strict-mode bare tokens ignored
- strict-mode marker citations accepted
- config unknown-key failure
- config unsupported-version failure (newer `grund_config_version` refused, with upgrade hint)
- config custom marker in strict mode
- config discovered as a bare root `grund.toml` from a subdirectory — which is also the case that pins the nothing-recognized caution to whole-project runs, since its one file holds neither a declaration nor a citation and its narrowed run must stay silent about that
- config redundant pair (the bare `grund.toml` wins, the `.agents/` file is warned about) — and, beside it, the nothing-recognized caution the same run earns, the two cautions being independent facts about it
- a docs tree written for a different `[id] format` than the one configured: every heading heading-shaped, nothing declared, nothing cited, and the run naming the shapes the grammar wanted instead of printing `success`
- a qualified shorthand under an `[id] format` the member-local fallback parser cannot read (`{kind}{number}-{slug}`): the shorthand pass is the only producer there, so the token stays a citation and the run still reports the unknown alias rather than reading the file as recognizing nothing
- workspace mixing both config discovery forms across its members
- a `[workspace]` member that escapes the block listing it, both ways a symlink can do it: pointing back at the block's own root (`self -> .`) and out of its tree (`link -> ..`)
- nested workspaces: whole-alias-path naming, per-level alias uniqueness, the grouping node as a project, subtree scope, the short-leaf-name hint, an enclosing workspace whose own member list fails to expand, one whose config does not load at all and still owes the subtree its own error, one cross-branch citation checked at both scopes, and an empty nested block with no `members` key at all
- `include_root = false` leaving the excluded root's own files scanned by nobody: a dangling citation there passes even `check --full`
- `include_root = false` at the outermost root of a nested tree: no catalog row for the root, member paths still rendered from the workspace root, the root alias unknown to `show` and to `list --project`, and completions offering no `root/`
- `fmt --cross-refs --write` wrapping citations that cross two workspace levels in both directions, and a `--check` run over that result reporting only a third, deliberately bare citation — silent about the two wrapped ones, so "nothing left to do" cannot be mistaken for "does nothing"
- a nested qualified ID as a CLI argument: `grund group/alpha/FS-x`, `refs group/alpha/FS-x`, and `list --project` for both a leaf and the grouping node it sits under (an exact alias match, never a prefix one)
- a `[citations]` rule entry qualified by a whole alias path (`must = ["group/alpha/FS"]`), satisfied by a nested-path line in an `spec.refs` manifest and unsatisfied by the leaf name alone
- a malformed nested alias path as a CLI argument, naming the segment that failed rather than the whole path
- a number-only shorthand glued to a second number: `fmt --write` leaves every glued shape byte-for-byte while the ordinary citations around them still expand, `check` reports the site with both the canonical form and the `<§>` escape, and the escape clears it
- the boundary of that rule: a construct closing and another opening is not one delimiter run, so a Markdown link, a footnote reference, a quoted or bracketed citation, and a path glued to the token all still expand — while a real run inside brackets still does not
- `fmt --check` naming the text each shorthand expansion will write, including two on one line and a typed trigger marked and expanded in one pass
- the unknown-alias hint at the workspace root (the two `FS-check.3.8` worked examples, byte-exactly): a dropped prefix naming one project, and a leaf name two projects share naming both, joined as `a or b`
- a narrowed run offering no candidate where the workspace root would have offered one: a citation naming the top-level `api` is green at the root and, from inside `left` (which holds its own `api`), reports the scope it covers rather than pointing at `left/api`
- config include/exclude/extensions
- explicit `check` subcommand
- default `show` shorthand and mistyped-path failure with explicit-check hint
- top-level help output
- per-subcommand help (`grund help check`, `grund help show`, `grund help list`)
- `grund help <unknown>` failure
- nested-workspace shell completions: the alias-path candidates a nested tree offers with no prefix, a mid-path prefix offering the grouping node's own ID beside its members' deeper paths, and one more Tab reaching a leaf's IDs — the typed prefix never re-offered
- `list` ID catalog (text), comma and repeated multi-kind `--kind`, `--unused`, `--summary`, summary composition with `--kind` / `--unused`, `--format json`
- JSON report output
- `fmt --check` trigger-to-marker report
- `fmt` custom trigger and marker from config
- `fmt --write` trigger-to-marker mutation path
- `fmt --marker --check` bare-to-marker report
- `fmt` idempotence
- `fmt` skips declaration headings and fenced Markdown
- `show` full Markdown declaration
- `show` Markdown section extraction
- `show` lead default
- `show --toc` / `show --brief` in text, Markdown, and JSON, including empty lead handling, empty output, E2E manifests, and mode mutex errors
- `show` missing ID failure (with recovery hint)
- `show` missing section failure (with recovery hint)
- `refs --summary` in text and JSON, including duplicate citations on one line and section-filtered summaries
- `name --explain` next-step hint
- `show` Rust inline declaration extraction
- Markdown stub to Rust inline declaration
- broken Markdown-to-Rust inline stub
- Rust source comment to Markdown citation
- Rust `///` doc-comment declaration and marked citation under strict mode
- Rust block doc-comment declaration and marked citation under strict mode
- Go line doc-comment declaration and marked citation under strict mode
- Python `"""` / `'''` docstring declarations and marked citations under strict mode
- Clojure `;` and SQL `--` declarations and marked citations under strict mode
- missing stub-link target
- stub-link target is a directory
- stub-link target has an unsupported extension
- skipped output/hidden directories
- a symlinked Markdown file whose target sits outside `[scan] include`: its dangling citation reported at the link path, and the in-tree declaration it cites no longer reported unused
- a symlinked directory, with the dangling citation inside it reported under the link's name
- a workspace where a link inside one member reaches a sibling project and another reaches the root project's docs: neither crosses, while a link to content no project owns is still followed
- one file reached under two names read once: no duplicate declaration, and the lexicographically first spelling is the one reported
- a symlink loop (`docs/self -> .`) reported at its own path while the walk carries on to the findings past it
- a loop whose target is above the walk root (`docs/up -> ..`) reported at the link and not descended into: no finding out of the second copy of the tree, and `[scan] include` still bounds the scan
- a broken symlink with a scanned extension reported and exiting 2, beside one without that stays silent
- `fmt --write` reading a file reached through a link that leaves the config root and refusing to write it: the target keeps its bytes, the in-root file is rewritten, and the refusal is one `warning:` line
- `fmt --check` refusing the same link `--write` refuses instead of listing a rewrite nobody can perform: the one pending edit in the tree is inside it, so the dry run is green with a `warning:` and not red forever
- `fmt` in a workspace naming a member's unreadable path and its refused write from the **workspace root**, the way `check` names them, not from the member's own root where the same spelling is a different file
- `fmt --write` aborting on a path it could not read, in the shape that says so — `nothing was rewritten:` — rather than the bare line the partial run prints after rewriting everything readable
- `fmt --check` naming the broken symlink it walked past, in the same shape and with the same exit `2` `check` uses for it
- nested e2e fixture repos ignored during ordinary scans
- unsupported extension ignored
- deterministic multiple-error output
- `check --full` reporting a dangling citation outside `[scan] include`, the same tree staying silent without the flag, and style / grounding findings withheld out there
- an explicitly included `.github/workflows` YAML root staying scanned despite its hidden parent, both ordinarily and under `check --full`
- `check --full` keeping an `[scan] include` root that `[scan] exclude` names, and one whose name is hidden, inside the ordinary scope
- `check --full` resolving an out-of-scope citation against an out-of-scope declaration
- `check --full` compound out-of-scope diagnostic codes in `--format json`
- `check --full` cautioning on stderr when an explicit path leaves it nothing to widen
- `check --full` reporting a cross-member number-only shorthand once in a workspace
- inline citation style: a citation-only site carrying prose, and a soft-cap overrun surfacing as a warning
- inline note layout (`citation-first-colon`): one error per nonconforming line under `inline_note_layout_check = "error"`, the same lines as warnings under `warn`, silence at the default `off`, and silence under `inline_note_layout = "any"` whatever the check level
- config invalid-value failures for `inline_note_layout` and `inline_note_layout_check`, and for a soft cap above the hard cap
- `inline_note_max_columns` counted in characters, not bytes: at a cap of 40, a 40-character ASCII note and a 40-character accented one both pass though each is over 40 bytes — the `§` marker alone puts the ASCII line there — while a 47-character note is the only one reported
- config ID-grammar failure for a `slug_pattern` that admits the alias separator `/`
- the index a folder kind keeps ([§FS-check.4.6](../docs/functional-spec/FS-check.md#46-declaration-missing-from-its-kinds-index), [§FS-check.3.17](../docs/functional-spec/FS-check.md#317-index-entry-is-not-a-link)): a declaration the index does not name, a folder with no index file at all, an entry present but bare, `index = false` opting a kind out, `index = "<name>"` selecting another file, the recursive subtree, a stub-and-inline pair collapsing to one entry — which is also the anchorless link to a source home, accepted because "full link" means the link `fmt` writes here — and an inline-code mention counting as neither an entry nor a finding
- the three states in which `check` must **not** demand a link, because `grund fmt --write` would not write one: an unmarked token off strict mode ([§FS-fmt.6.5](../docs/functional-spec/FS-fmt.md#65-interaction-with---marker)), an ID-shaped file name inside a Markdown link destination ([§FS-fmt.2.3](../docs/functional-spec/FS-fmt.md#23-what-is-never-rewritten)), and — closed at config time instead — an `index` naming a file the cross-reference pass never runs on. The link-destination fixture is also why `basic-markdown-duplicate` spells its two homes `login-a.md` / `login-b.md`: an ID-shaped file name in a destination is scanned as a citation off strict mode, which is [grund#131](https://github.com/vjovanov/grund/issues/131) and not this rule's to fix
- `[[kinds]] index` rejected when it is not a relative path inside `folder`, and the per-prefix default applying to a *declared* `[[kinds]]` block exactly as to the built-in list, so a config written before the key existed does not acquire an obligation on upgrade ([§FS-config.3.4](../docs/functional-spec/FS-config.md#34-kinds--recognized-prefixes))
- `fmt --write` linkifying an index under `[fmt.cross_refs] enabled = false`, and doing it to the index's own entries only — a citation of a foreign ID in the prose beside them stays bare ([§FS-fmt.6.1](../docs/functional-spec/FS-fmt.md#61-scope))
- an index entry not suppressing the unused-declaration warning: a declaration whose only inbound citation is its own index entry is still reported uncited ([§DF-index-not-an-inbound-citation](../docs/decisions/functional/DF-index-not-an-inbound-citation.md#df-index-not-an-inbound-citation-an-index-entry-is-navigation-not-use))

Warning coverage is partial. The inline-citation-style family pins its warning channel here — the soft-cap overrun and the `inline_note_layout_check = "warn"` case both assert the warning text and the exit code it must not move. Other warning tiers are not covered yet; they are lower priority than the error, retrieval, formatting, and configuration contracts.
