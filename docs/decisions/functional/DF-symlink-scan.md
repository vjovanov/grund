# DF-symlink-scan: a symlink in the scanned tree is followed, and the report names the link

**Status:** Accepted
**Date:** 2026-08-21

## 1. Context

The walker was built without `follow_links`, so a symlink entry was neither a file nor a directory to it and fell out of the walk with nothing said. One skip produced two wrong answers at once: the citations in the linked file were never read, so a dangling one passed ([§REQ-no-missed-citation.1](../../requirements/REQ-no-missed-citation.md#1-no-silent-skips)), and every declaration that file cited was reported `declared but never cited` ([§FS-check.4.1](../../functional-spec/FS-check.md#41-unused-declaration)) because the edge retiring the warning had been dropped with it ([§REQ-no-wrong-citation.2](../../requirements/REQ-no-wrong-citation.md#2-no-false-alarms)). Replacing the link with a copy of the same bytes changed the verdict from green to red, which is the shape of a false negative that also lies about the tree it read. Reported as [issue #96](https://github.com/vjovanov/grund/issues/96).

The spec had discussed symlinks only as `[scan] include` **roots** ([§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full)). A symlinked *descendant* was unaddressed, so the behavior was not a bounded blind spot in the sense [§REQ-no-missed-citation.2](../../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) allows — it was one nobody had written down, and therefore one no reader could plan around.

Following every external directory target later exposed the opposite namespace
error: a member checked independently followed `docs/blink -> ../../sibling`
and absorbed the sibling's declarations, reporting them as duplicates of its
own. A workspace-root run pruned the same link because it had loaded project
ownership, leaving the same tree green at the root and red in the member.
Reported as [issue #102](https://github.com/vjovanov/grund/issues/102).

## 2. Decision

### 2.1 Follow the link, do not report and skip it

A file symlink is followed, and a directory symlink is followed while it stays
inside the independently checked project's canonical root
([§FS-config.3.5.1](../../functional-spec/FS-config.md#351-a-symlink-in-the-tree-is-followed)).
The alternative — refuse every link — does close the *silent* half of the
original bug, but it leaves the other half standing: citations in a linked file
stay unread, so the declaration it cites is falsely reported unused. The root
fence therefore distinguishes a directory traversal from a linked file rather
than making a repository stop using links.

Following also makes the two spellings of one file agree. `CLAUDE.md -> AGENTS.md` is a link precisely so that one set of bytes has two names; a checker that reads one name and refuses the other reports on half a repository.

### 2.2 The canonical project root fences directory links, not file links or scan paths

An external file symlink is followed as before. An external directory symlink
is not: traversal stops as soon as its canonical target is outside the
independently checked project's canonical root. The rule also gates a scan root
that is itself a directory link. It does not mistake a repository reached
through a symlink for an outward link; the canonical location of the discovered
config root is the fence, so that repository remains readable.

This reverses the original decision for external **directory** targets only.
In-root directory links remain followed, and a non-symlink parent-relative
`[scan] include` still names external content intentionally. Loaded workspace
ownership remains stronger: another loaded project is pruned even when its root
is physically inside this project's root
([§FS-workspace.6](../../functional-spec/FS-workspace.md#6-nested-project-boundary)).
The distinction prevents a path alias from importing a foreign tree while
preserving the explicit configuration and file-link uses whose intent is
available without discovering an ancestor workspace.

### 2.3 The report names the link, never the target

Everything a link met **inside a walked tree** produces is reported at the in-tree link path. The target path may be absolute, may sit outside the config root, and may not be reachable at all from where the reader is standing, so a finding wearing it is a finding nobody can jump to ([§FS-errors.2.1](../../functional-spec/FS-errors.md#21-located-finding)) and one that `relative_paths` cannot render ([§FS-config.3.6](../../functional-spec/FS-config.md#36-output--report-format)). Naming the link is also what keeps `--full` purely additive ([§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full)): the in-scope lines of a `--full` run are the plain run's, spelling included, which only holds while the spelling is a property of the walk rather than of the filesystem underneath it.

An explicit path argument is not that case and is left as it was: `grund check docs/FS-beta.md` resolves the scope before any walk begins, so a finding there wears the target's path — absolute, when the target is outside the root. That predates this decision, it is the same resolution that makes `grund check <path>` work at all, and narrowing it belongs to whoever revisits explicit scopes.

Because one physical file can now be reached under two spellings within a single walk, the walk keeps the first and the rule for "first" is written down rather than inherited from readdir order: the earlier root wins, and within one root the lexicographically first path does ([§FS-errors.4](../../functional-spec/FS-errors.md#4-determinism)).

### 2.4 A link the walk cannot resolve is reported and the walk continues

A broken link and a symlink loop are files the scan cannot read, and [§FS-check.2](../../functional-spec/FS-check.md#2-outputs) already says what happens to one of those: reported at its path, the walk continuing past it, exit `2`. Aborting the whole scan on the first one would be worse than the bug being fixed — a single dangling link would take the entire report with it.

The report is owed only where the walk would otherwise have read through the link, and that question is asked of the link the same way it is asked of any other entry. An ignore file that covers the link answers it for both kinds: a `.gitignore`d `docs/self -> .` is a path the ordinary walk was never going to descend into, so a loop there is not a hole in what was scanned. A broken link is judged by `[scan] extensions` as well, since a dangling `docs/logo.png -> nowhere` was never going to be read either; a loop is a directory and has no extension to judge, so the ignore rules are the whole of its gate.

Without that gate a repository full of links to build outputs — none of which were ever going to be scanned — turns red for reasons that have nothing to do with citations, and a mode that cries wolf about links is one whose scan errors get ignored. An earlier draft of this decision said a loop "is always owed one" and made exactly that trade against its own rationale: a repository whose `.gitignore` covers a looping link exited `2` for a directory no scan would have entered.

### 2.5 `fmt --write` refuses an external file link; external directory links never reach it

Following a link means `grund fmt --write` can now reach a file the project does not own, and in the first cut it wrote through one: a `docs/FS-beta.md -> ../../outside/FS-beta.md` was rewritten on disk at the target, outside the root, with a relative cross-reference that is broken where the file actually lives. Reading foreign bytes and *editing* them are different acts — the first is what §2.2 decided and is recoverable by reading, the second is not ([§REQ-no-data-loss.2](../../requirements/REQ-no-data-loss.md#2-writers-touch-only-what-they-own)).

So an external file link is read and the write stops at the config root:
`--write` skips it and says so
([§FS-fmt.2.3.2](../../functional-spec/FS-fmt.md#232-a-link-that-leaves-the-config-root-is-not-written-through)).
An external directory link contributes no files to `fmt`, because §2.2 prunes
it in the shared scan; it therefore produces neither rewrites nor per-file
refusal warnings. Refusing every symlink would take `CLAUDE.md -> AGENTS.md` — a
link into the project's own root — with it.

The dry run reports the refusal too, rather than the rewrite it would have made there. The first cut had it list the rewrite — "it reports what the tree contains" — which made `fmt --check` exit `1` on such a tree permanently: it named a pending edit that `--write` was never going to perform and no author could clear. A check mode whose job is to predict the write has to predict this part of it as well, or it stops being a gate anyone can put in CI.

A link whose target is inside the root is written through, which leaves one residue that is accepted rather than fixed. The file is read once, under the surviving spelling (§2.3), so `--cross-refs` anchors its relative links to that spelling: the link is right where grund read the file and wrong at its other name. There is no anchor that is right at both — a relative path resolves against the directory the reader opened, and the file has two — so the only real fix is one name per file. Naming it beats a rule that pretends to solve it.

### 2.6 The pre-1.0 boundary change is explicit and has three migrations

Some projects deliberately scan an external directory through a symlink. They
will stop seeing its findings; a formerly green project can instead turn red
when an in-root citation depended on a declaration found only through that
link. The migration is necessarily a choice among meanings: move the target
under the project root, check it as its own project, or replace the link with an
intentional non-symlink parent-relative `[scan] include`.

No one command can choose among those meanings, and retaining the former rule
would require a legacy catalog or compatibility setting larger than the
boundary itself. `grund` is pre-1.0, so this uses the explicit exception of
[§REQ-backwards-compatibility.4](../../requirements/REQ-backwards-compatibility.md#4-what-was-never-a-promise):
the release notes must name the possible verdict movement, the unchanged file
links, and all three migrations. The exception is accepted here rather than
assumed, and expires at 1.0.

## 3. Consequences

- A repository whose specs are reached through a symlink is checked for the first time. That can turn a green run red, which is the correct direction: the findings were always there and were being dropped.
- External file links and plain parent-relative includes can still read bytes outside the project root; external directory links cannot (§2.2).
- A standalone project and an independently checked workspace member both prune an outward directory link without loading an ancestor workspace. This removes the member-local false duplicate while leaving unknown-alias behavior unchanged.
- A broken or looping link with a scannable name is a new source of exit `2` on a tree that used to exit `0` silently.
- A **plain** run now collapses an aliased *root*, where only `--full` did. `include = ["docs", "docslink"]` with `docslink -> docs` reported `duplicate declaration` and exited `1`; it exits `0`. The duplicate was always false — one file under two names — and the `--full` restriction was an artifact of `--full` being the only place two roots were known to overlap, not a judgment that a plain run should keep a finding it can prove wrong. It is a verdict moving from red to green, so it is written down here and in the changelog rather than discovered ([§REQ-backwards-compatibility.1](../../requirements/REQ-backwards-compatibility.md#1-what-is-covered)).
- `grund fmt --write` reads an external file link and does not write it, naming it on stderr instead (§2.5). Files below an external directory link never reach it; inside the root it writes through, with the cross-reference anchoring residue §2.5 records.
- The identity pass that `--full` used for aliased roots now also runs whenever the walk met a symlink — over the linked files alone. A tree with no symlink pays nothing, and a tree with one symlink pays one `realpath` rather than one per file, which is what keeps [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) intact.

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| Report the skipped link and keep not reading it | Closes the silent half of the bug and leaves the false-alarm half: the linked file's citations stay unread, so what it cites is still reported unused (§2.1). |
| Skip silently and say so in the spec | Makes the blind spot declared, which [§REQ-no-missed-citation.2](../../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) asks for, and stops there — the false `declared but never cited` warning is the part that actually misleads, and it survives. |
| Keep following every external directory link | Preserves the old reading but lets an independent workspace member absorb sibling declarations and produce a scope-dependent false duplicate. |
| Make member-local runs discover and load an ancestor workspace | Imports workspace expansion and its full error surface into every independent run to answer a boundary question the project's own canonical root already answers (§2.2). |
| Refuse external file links too | Re-creates the original false alarm for a citation plainly present in a linked file and breaks common in-tree entrypoint aliases (§2.1). |
| Report findings at the resolved target path | Unjumpable, unrenderable under `relative_paths`, and it breaks the `--full` additivity rule, which requires the in-scope lines to be identical spelling included (§2.3). |
| Canonicalize every walked file so aliases always collapse | A `realpath` per file on every run of every repository, to serve the trees that have a symlink in them. The walk is where [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) is set (§3). |
| Arm the identity pass with a flag the first symlink raises | The same `realpath` per file as the row above, bought by one link anywhere in the tree — and real repositories have one, `CLAUDE.md -> AGENTS.md` among them. Measured at ~2.7x on a 20 000-file tree with a single link. What the walk records instead is the *list* of files that can wear a second name, so the cost tracks the links rather than the repository (§3).
| Abort the scan on the first unresolvable link | One dangling link takes the whole report with it, which is the opposite of what [§FS-check.2](../../functional-spec/FS-check.md#2-outputs) promises about a file the walk cannot read (§2.4). |
