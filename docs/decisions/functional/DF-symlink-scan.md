# DF-symlink-scan: a symlink in the scanned tree is followed, and the report names the link

**Status:** Accepted
**Date:** 2026-08-21

## 1. Context

The walker was built without `follow_links`, so a symlink entry was neither a file nor a directory to it and fell out of the walk with nothing said. One skip produced two wrong answers at once: the citations in the linked file were never read, so a dangling one passed ([§REQ-no-missed-citation.1](../../requirements/REQ-no-missed-citation.md#1-no-silent-skips)), and every declaration that file cited was reported `declared but never cited` ([§FS-check.4.1](../../functional-spec/FS-check.md#41-unused-declaration)) because the edge retiring the warning had been dropped with it ([§REQ-no-wrong-citation.2](../../requirements/REQ-no-wrong-citation.md#2-no-false-alarms)). Replacing the link with a copy of the same bytes changed the verdict from green to red, which is the shape of a false negative that also lies about the tree it read. Reported as [issue #96](https://github.com/vjovanov/grund/issues/96).

The spec had discussed symlinks only as `[scan] include` **roots** ([§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full)). A symlinked *descendant* was unaddressed, so the behavior was not a bounded blind spot in the sense [§REQ-no-missed-citation.2](../../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) allows — it was one nobody had written down, and therefore one no reader could plan around.

## 2. Decision

### 2.1 Follow the link, do not report and skip it

A symlink is followed, file and directory alike ([§FS-config.3.5](../../functional-spec/FS-config.md#35-scan--what-gets-walked)). The alternative — keep the walk as it is and emit a finding naming the skipped link — is the smaller change and it does close the *silent* half of the bug, but it leaves the other half standing: the citations in the linked file are still unread, so the declaration it cites is still falsely reported unused, and the user's only remedy is to stop using a symlink. A tool whose answer to "this file is a link" is "then I will not check it" is a tool that a repository has to be laid out around.

Following also makes the two spellings of one file agree. `CLAUDE.md -> AGENTS.md` is a link precisely so that one set of bytes has two names; a checker that reads one name and refuses the other reports on half a repository.

### 2.2 A target outside the config root is followed too, not refused

A link whose target resolves outside the config root is followed like any other. The file is in the tree by the path the repository wrote, and that path is inside the walk; refusing it would be the report-and-skip of §2.1 wearing a safety argument, and it would leave the false `declared but never cited` warning in place for a citation that is plainly there.

The cost is real and worth naming: content the project does not own can be pulled into the scan, and its citations are then judged under *this* project's grammar and kinds. That is the same trade `--full` already makes for an undeclared project directory inside the root ([§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full)), and it has the same remedy — `[scan] exclude`, or `[workspace] members` when the tree is one of ours.

### 2.3 The report names the link, never the target

Everything a followed link produces is reported at the in-tree link path. The target path may be absolute, may sit outside the config root, and may not be reachable at all from where the reader is standing, so a finding wearing it is a finding nobody can jump to ([§FS-errors.2.1](../../functional-spec/FS-errors.md#21-located-finding)) and one that `relative_paths` cannot render ([§FS-config.3.6](../../functional-spec/FS-config.md#36-output--report-format)). Naming the link is also what keeps `--full` purely additive ([§FS-check.1.3](../../functional-spec/FS-check.md#13-the-full-tree-scope---full)): the in-scope lines of a `--full` run are the plain run's, spelling included, which only holds while the spelling is a property of the walk rather than of the filesystem underneath it.

Because one physical file can now be reached under two spellings within a single walk, the walk keeps the first and the rule for "first" is written down rather than inherited from readdir order: the earlier root wins, and within one root the lexicographically first path does ([§FS-errors.4](../../functional-spec/FS-errors.md#4-determinism)).

### 2.4 A link the walk cannot resolve is reported and the walk continues

A broken link and a symlink loop are files the scan cannot read, and [§FS-check.2](../../functional-spec/FS-check.md#2-outputs) already says what happens to one of those: reported at its path, the walk continuing past it, exit `2`. Aborting the whole scan on the first one would be worse than the bug being fixed — a single dangling link would take the entire report with it.

The report is owed only where the walk would otherwise have read through the link. A loop is a directory the walk was descending into, so it is always owed one; a broken link is judged by `[scan] extensions`, so a dangling `docs/logo.png -> nowhere` stays silent. Without that gate a repository full of links to build outputs — none of which were ever going to be scanned — turns red for reasons that have nothing to do with citations, and a mode that cries wolf about links is one whose scan errors get ignored.

## 3. Consequences

- A repository whose specs are reached through a symlink is checked for the first time. That can turn a green run red, which is the correct direction: the findings were always there and were being dropped.
- Files reached through a link may live outside the config root, so a run can now read bytes the project does not own (§2.2). `[scan] exclude` is the fence.
- A broken or looping link with a scannable name is a new source of exit `2` on a tree that used to exit `0` silently.
- The identity pass that `--full` used for aliased roots now also runs whenever the walk met a symlink — over the linked files alone. A tree with no symlink pays nothing, and a tree with one symlink pays one `realpath` rather than one per file, which is what keeps [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) intact.

## 4. Alternatives considered

| Option | Why rejected |
|---|---|
| Report the skipped link and keep not reading it | Closes the silent half of the bug and leaves the false-alarm half: the linked file's citations stay unread, so what it cites is still reported unused (§2.1). |
| Skip silently and say so in the spec | Makes the blind spot declared, which [§REQ-no-missed-citation.2](../../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) asks for, and stops there — the false `declared but never cited` warning is the part that actually misleads, and it survives. |
| Follow links inside the config root, refuse those that escape it | The link is in the tree by the path the repository wrote; refusing it by where its target lands re-creates the false alarm for the one case a user is most likely to have written deliberately (§2.2). |
| Report findings at the resolved target path | Unjumpable, unrenderable under `relative_paths`, and it breaks the `--full` additivity rule, which requires the in-scope lines to be identical spelling included (§2.3). |
| Canonicalize every walked file so aliases always collapse | A `realpath` per file on every run of every repository, to serve the trees that have a symlink in them. The walk is where [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) is set (§3). |
| Arm the identity pass with a flag the first symlink raises | The same `realpath` per file as the row above, bought by one link anywhere in the tree — and real repositories have one, `CLAUDE.md -> AGENTS.md` among them. Measured at ~2.7x on a 20 000-file tree with a single link. What the walk records instead is the *list* of files that can wear a second name, so the cost tracks the links rather than the repository (§3).
| Abort the scan on the first unresolvable link | One dangling link takes the whole report with it, which is the opposite of what [§FS-check.2](../../functional-spec/FS-check.md#2-outputs) promises about a file the walk cannot read (§2.4). |
