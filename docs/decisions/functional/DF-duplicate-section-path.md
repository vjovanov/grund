# DF-duplicate-section-path: a section coordinate names one heading, or the run says so

**Status:** Accepted
**Date:** 2026-08-22

## 1. Context

A declaration's numbered headings are addressable: `## 1. Inputs` under `# FS-001-login` makes `§FS-001-login.1` a pointer to that heading. Two headings claiming the same dotted path were accepted in silence:

```markdown
# FS-001-login: User login

Lead.

## 1. First

First body.

## 1. Second

Second body.
```

`grund check .` reported `success`. Reported from a real tree ([issue #97](https://github.com/vjovanov/grund/issues/97)).

**The citation had two targets and the tool picked without saying so.** [§FS-check.3.3](../../functional-spec/FS-check.md#33-duplicate-declaration) makes a duplicate *declaration* an error for exactly this reason: a citation must not have two possible homes. A duplicate section path is the same hazard one level down and nothing was watching it — [§FS-check.3.2](../../functional-spec/FS-check.md#32-missing-section) reports a section that is absent, [§FS-check.3.9](../../functional-spec/FS-check.md#39-section-heading-level-mismatch) reports one written at the wrong depth, and neither notices one written twice. [§REQ-no-wrong-citation.1](../../requirements/REQ-no-wrong-citation.md#1-no-wrong-resolution) names the case in as many words: *"Silently preferring the first of two identical section paths is the shape of a guess even when it is deterministic."*

**And the two readers disagreed about which one it picked.** The scanner's section map was a last-wins insert, so the recorded line was the *second* heading's. `show` did not use that line: it re-scanned the file and matched the path itself, and its "found the target" branch had no guard against firing twice — so the second heading re-entered it, and `grund FS-001-login.1` printed **both** headings and **both** bodies as one slice. That is not a pick between two defensible answers. It is a body no heading in the file spans, assembled by the query, handed to a reader as the content of section 1 — a wrong resolution, not an ambiguous one, and the failure class [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) exists to forbid: not a citation resolving to nothing, but one resolving to the wrong thing while the run reports green.

The `--toc` map, meanwhile, listed both headings as section `1`, so the one surface that showed the collision showed it as a list a reader had to notice for themselves.

## 2. Decision

### 2.1 The collision is an error, in §FS-check.3.3's shape

Two headings claiming one path inside one declaration is [§FS-check.3.16](../../functional-spec/FS-check.md#316-duplicate-section-path), a `duplicate-section` error anchored at the first heading and naming the rest.

The alternative on the table was to write first-wins down and leave the tree legal (§3). It was rejected because writing a pick down does not stop it from being a pick. [§REQ-no-wrong-citation.1](../../requirements/REQ-no-wrong-citation.md#1-no-wrong-resolution) asks for one of two things at every fork — the rule that picks, *or* the report that refuses — and for a fork that a repository can simply not have, the report is the answer that leaves nothing to remember. `grund` already made that call one level up: [§FS-check.3.3](../../functional-spec/FS-check.md#33-duplicate-declaration) does not rank two homes by path order, it reports them. There is no argument for ranking that survives the move from IDs to section paths, and a rule that treats the same ambiguity two ways at two levels is one more thing for a reader to hold.

The **error**, not a warning, on the argument [§FS-check.3.3](../../functional-spec/FS-check.md#33-duplicate-declaration) is an error by: a finding no CI run fails on is one a repository accumulates behind forever, and the cost of the accumulation here is that every existing `§<ID>.<path>` citation silently changes meaning the day someone moves a paragraph between the two sections.

Renumbering one heading clears it, and the finding names both lines, so the fix is visible from the message without opening the file.

### 2.2 Scoped to one declaration, and independent of the heading-level mode

A section path is addressed as `<ID>.<path>`, so `1.` under `FS-001-login` and `1.` under `FS-002-session` are two coordinates that never met. Only headings sharing a declaration collide.

The `[id] section_heading_levels` mode ([§FS-config.3.3](../../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)) does not gate the rule. That mode governs how deep a heading must sit for the path it writes — a different fact, judged by [§FS-check.3.9](../../functional-spec/FS-check.md#39-section-heading-level-mismatch). Under `"loose"`, where `## 1.1` and `### 1.1` both declare `1.1`, the collision is if anything easier to write by accident, so exempting the mode that most needs the rule would be backwards.

### 2.3 The map keeps the first heading, and that is now written down

Resolution has to answer while the tree is red — `check` still has to say whether every *other* citation resolves, `list` and completions still have to run — so the recorded section map still holds exactly one heading per path. It is now the **first** one.

Last-wins was not a decision anyone made; it was `insert` overwriting. First-wins is the answer that agrees with everything around it: the first heading is the one a reader scrolling to section `1` meets, the one `show`'s re-scan already reached, and the one [§FS-check.2.1](../../functional-spec/FS-check.md#21-report-format) anchors a multi-site finding at. One heading recorded, one heading anchored, one heading a human sees first.

The later headings are kept in a parallel list so the error can name their lines, and that list is inert to every lookup.

### 2.4 The heading-level rule judges only the heading the path resolves to

[§FS-check.3.9](../../functional-spec/FS-check.md#39-section-heading-level-mismatch) reads the recorded map, so under §2.3 it now judges the first heading where it used to judge the last. The duplicates are not additionally level-checked.

They are not section targets. Nothing resolves to them, and the run has already said they should not exist; measuring the depth of a heading against a path the tool is reporting it does not own is a finding about a line that is about to be renumbered or deleted, and whose correct depth depends on which of those the author chooses. One cause, one finding — the rule [§FS-check.3.13](../../functional-spec/FS-check.md#313-number-only-shorthand-citation) already follows for a site two rules could both claim.

No tree loses a finding to this: wherever §3.9 stops reporting the second heading, §3.16 reports the pair, at the same lines, in the same run.

### 2.5 `show` refuses the ambiguous section instead of printing a pick

`grund <ID>.<path>` where the path is duplicated exits `1` with the bare query-failure line of [§FS-show.2.2.2](../../functional-spec/FS-show.md#222-ambiguous-section), naming both sites — the section-level twin of the ambiguous-ID refusal at [§FS-show.2.2.1](../../functional-spec/FS-show.md#221-ambiguous-id), which already says `show` does not pick between two homes and the repo must be fixed first.

Printing the first heading's body would be defensible if the query were the only thing at stake. It is not: the caller of `show` is usually an agent pulling a fact into context to act on, and a slice that is *silently one of two* is a fact it cannot audit. The refusal costs a `grund check` run and returns the same body afterwards.

`--toc` is deliberately exempt and still lists both heading lines. It is a map of what the file contains, not a resolution of a coordinate, and a reader who ran `--toc` on a declaration they are about to cite should see the collision rather than a refusal that hides which headings caused it.

This also retires the merged-body answer of §1, which no option here preserves and none should.

### 2.6 Nothing that worked stops working

Adding an error moves verdicts, so [§REQ-backwards-compatibility](../../requirements/REQ-backwards-compatibility.md#req-backwards-compatibility-an-upgrade-never-changes-a-verdict-quietly) has to be answered rather than assumed. This lands under its §4: a construct that **had no defined meaning** was not a working feature.

No spec described a duplicate section path; no rule accepted it deliberately; the two readers of the section map disagreed about which heading it named, and the one a user actually sees returned a body assembled out of both. There was no behavior here to preserve — only an unspecified region that produced a wrong answer. That is the [§DF-number-only-citation-shorthand](DF-number-only-citation-shorthand.md#df-number-only-citation-shorthand-the-number-only-shorthand-is-authoring-sugar-and-a-persisted-one-is-a-check-error) precedent §4 was written from.

The finding is also mechanical in the sense [§REQ-backwards-compatibility.3](../../requirements/REQ-backwards-compatibility.md#3-loud-mechanical-migrations) cares about — it names every colliding line, and the fix is renumbering one heading — so a repository that upgrades into it can clear it without reading a changelog. No `grund_config_version` bump and no `AGENTS.md` block bump: the rule adds no key and changes nothing an agent is told to write.

## 3. Rejected alternative: specify first-wins and leave it legal

Write "the first heading claiming a path wins" into [§FS-check](../../functional-spec/FS-check.md#fs-check-grund-validates-every-reference-in-a-repo) and [AR-scanner.2.2](../../architecture/AR-scanner.md#22-section-detection), fix `show` to stop merging, and report nothing.

It is cheaper, it satisfies the letter of [§REQ-no-wrong-citation.1](../../requirements/REQ-no-wrong-citation.md#1-no-wrong-resolution) — the rule that picks would be written down — and it was rejected for what it makes permanent. A repository could then carry two "section 1"s indefinitely, legally, with `§<ID>.1` pointing at whichever the author wrote first. The hazard is not that the first read is wrong; it is that **moving a paragraph** between the two sections, or reordering them, changes what every existing citation to that coordinate means, with nothing reported in the run that did it. That is [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) arriving inside a single repository's own edit history, and no amount of specification makes it visible at the moment it happens.

The rule this project actually holds is that ambiguity is reported rather than ranked, and §2.1 keeps it at both levels.

## 4. Rejected alternative: a warning

Report the collision but leave the exit code alone, so an upgrade cannot turn a tree red.

The verdict-change worry is real and §2.6 answers it on its own terms. Paying for it with a warning would buy silence in CI, which is where the finding has to land: the collision is invisible in review — two headings numbered `1.` are several screens apart in the file that has them — and a finding that never fails a build is one nobody is required to clear. It would also split the treatment of one ambiguity across two severities, with the ID case an error and its section twin a warning, for no reason a reader could reconstruct.

## 5. Consequences

- A new `duplicate-section` error code ([§FS-check.3.16](../../functional-spec/FS-check.md#316-duplicate-section-path)), carrying the multi-site `sites` list [§FS-errors.5](../../functional-spec/FS-errors.md#5-json-format) defines for a duplicate declaration.
- `Declaration` gains a `duplicate_sections` list beside `sections`, and the scanner's section insert becomes first-wins ([AR-scanner.2.2](../../architecture/AR-scanner.md#22-section-detection)).
- `show` gains the ambiguous-section refusal ([§FS-show.2.2.2](../../functional-spec/FS-show.md#222-ambiguous-section)) and loses the merged-body answer; `--toc` is unchanged.
- Every other reader of the section map — resolution, completions, the heading-level rule, `fmt`'s anchor emission — is unchanged in shape and now reads the first heading rather than the last.
