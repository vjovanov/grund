# FS-workspace: grund validates cross-project citations in a workspace

`grund` can treat a repository as a workspace of independent project namespaces.
Local citations stay unchanged, while cross-project citations name a stable alias
before the local ID. This keeps the zero-config single-project path intact
([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)) and gives larger repos an explicit resolver for sub-projects
without forcing project names into every ID ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)). The alias
syntax is chosen in [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos).

## 1. Citation syntax

A normal citation still resolves inside the current project:

```text
§FS-login
§FS-login.2.1
```

A cross-project citation writes the target project's alias path, a slash, then
the target ID:

```text
§api/FS-login
§root/GOAL-compatibility
§payments/FS-refunds.3.2
§hardware/sprayer/FS-nozzle
```

The grammar is:

```text
§[alias/…/alias/]ID[.section]
```

Each `alias` is a lowercase slug: it starts with a letter and then uses lowercase
letters, digits, or `-`. The alias path carries **one segment per workspace
level** (§6.1); in a workspace with no nesting it is always a single segment, so
the common form is `<§>alias/<ID>`. The slashes are part of the citation
namespace, not part of the ID — and since an ID never contains `/` (the ID
grammar rejects one on load, [§FS-config.3.2](FS-config.md#32-id--id-grammar)),
the last slash in the token is always the boundary between the two.

For an unqualified citation, the `ID[.section]` part uses the current project's
ID and section grammar. For a qualified citation, it uses the target project's
grammar: `<§>api/FS-001-session` is parsed with `api`'s `[id]` config, even when
the citing/root project uses a different ID format.

The shape is read the same way in **every** repository, including one with no
`[workspace]` at all: what makes a token a citation is the marker
([§FS-check.1.1](FS-check.md#11-recognized-citations)), and a marked token of this shape is a citation whose alias
path resolves against nothing, so it is reported as an unknown project alias at
its site rather than skipped (§5, [§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)) — the same rule that
stops a member-local run from shipping a cross-project citation the workspace
root would reject. The multi-segment path therefore makes one *file path*
readable as a citation: `<§>docs/functional-spec/FS-login.md` is a two-segment
alias path plus an ID, and marking it is what says "resolve this". A path meant
as a path is written without the marker, or outside the citation.

## 2. Workspace configuration

A workspace is declared in the root project's `grund.toml` ([§FS-config.1](FS-config.md#1-file-location-and-discovery)):

```toml
project_name = "root"

[workspace]
members = ["apps/api", "packages/*"]
include_root = true
```

`members` is a list of paths or single-segment trailing globs, resolved relative
to the config root. Member paths must be relative, must not use `.` or `..`, must
not use platform-specific absolute forms or backslash separators, and must not
overlap after glob expansion; one member root cannot contain another member
root. Each must also resolve — through symlinks, which is the only way left to
escape — to a directory *strictly inside* the config root that lists it: not that
root itself, not an ancestor of it, not another tree. Invalid member entries,
missing member paths, escaping member roots, and overlapping expanded
roots are config errors reported at the `members` line per
[§FS-config.4.3](FS-config.md#43-invalid-config-behavior), and they name the entry as the config wrote
it rather than the root it resolved to — a resolved root renders as nothing when
it is the block's own and as an absolute path once it leaves the tree. `packages/*` means
every direct child directory under `packages/`, including a child symlink whose
target is a directory; the same canonical containment rule applies to that target.
Recursive `**` globs are not part of v1. If the glob parent exists but cannot
be read, that is likewise a config error at the `members` line: “cannot read workspace member glob
`packages/*`: `<I/O reason>`”, naming the whole glob as written and preserving
the filesystem reason. `include_root` defaults to `true`; when false, a
workspace-root `grund check` checks only member projects.

Each member is a separate project namespace. If a member has its own config —
either discovery form, `.agents/grund.toml` or a bare `grund.toml`
([§FS-config.1](FS-config.md#1-file-location-and-discovery)) — that file configures the member. If it has neither, the
canonical defaults apply with the member directory as the config root. Root and
members choose independently, so a workspace may mix the two forms.

### 2.1 A member that swallows the block's own scan

A member root may sit anywhere strictly inside the block that lists it, and that
includes on top of the paths the block itself scans. When it covers **all** of
them the block's own project reads nothing: every one of its walk roots lies
under a member boundary (§6), so its declarations reach no catalog and its
dangling citations pass [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration). That is the same consequence
§6.1 gives as the reason a member root may not be an *ancestor* of its own
block, one step weaker — here the root is strictly inside the block and still
covers everything the block had to read.

So `grund` says so. Take the block's **default scope** — the roots `[scan]
include` and the walked `[[kinds]]` homes give it (§FS-config.3.5), which is the
set §6's boundary prunes — and keep the ones that exist on disk, since a root
that is not there is read by nobody and rescues nothing. When at least one such
root remains and **every** one of them is at or inside an expanded member root,
the block earns one warning at its `members` line, naming each covered root and
the member entry it is inside ([§FS-check.4.8](FS-check.md#48-a-workspace-member-swallows-the-blocks-own-scan)). Roots and member entries are
compared as canonical paths, the way the walk's own prune compares them, so a
member reached through a symlink or a glob covers what it actually lands on.

Three neighbouring shapes are deliberately *not* this finding:

- **A partly covered scope is specified behaviour.** §6 already says the root
  scan stops at a member "even if the root project's `[scan] include` names a
  path inside a member", so one covered entry beside a surviving one is the
  boundary working as designed and stays silent.
- **`include_root = false` has nothing to lose.** That block is not a project
  (§6.1), so it has no scan of its own to be covered. What its files cost is
  §6.1's own subject.
- **`--full` does not silence it** ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full)). The flag adds the config
  root as a walk root, but the member boundary still prunes, so the absorbed
  tree is no more readable with it than without. The question is therefore asked
  of the default scope whatever the flag says: this is a property of the
  configuration, not of one walk.

**An absent `[scan] include` key is not one of them.** The key carries a
materialized default — `requirements.md`, `docs`, `e2e`, `src`
([§FS-config.3.5](FS-config.md#35-scan--what-gets-walked)) — so a block that omits it has *those* roots rather than
only the block root, and the rule above is asked of them as it is of any other
block's: when the ones that exist on disk are all inside members, that block
reads nothing and is told so. The remedy the warning names is still the one to take, since adding
the key pointed somewhere that is not a member is exactly the repair.

The repair is a judgement rather than a command `grund` can run — point `[scan]
include` at a directory that is not also a member, or say `include_root = false`
and mean it — which is why the finding arrives as a warning on the deprecation
path of [§REQ-backwards-compatibility.2](../requirements/REQ-backwards-compatibility.md#2-the-deprecation-path) rather than as an error
([§DF-absorbed-scan-warning](../decisions/functional/DF-absorbed-scan-warning.md#df-absorbed-scan-warning-a-scan-its-own-members-swallowed-is-a-warning-with-a-named-release-not-an-error)).

### 2.2 A member that may be legitimately absent

§2 makes a missing member path a config error, and for a `members` entry that is
right: a member the repository says it has and does not have is a broken
workspace, and the alias every citation into it depends on is a promise this
checkout cannot keep.

It is the wrong answer for a member the repository *knows* may be missing — a
private submodule CI never fetches, a sparse checkout, a sibling repository
vendored in only for release builds. Such a repository had no run at all. Leaving
the entry in `members` is the config error above; taking it out unregisters the
alias, so every citation into that namespace becomes an unknown-alias error at
its own site (§4, [§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)) — thousands of lines in a tree that cites the
namespace widely. One path refuses to start and the other calls the whole tree
broken, and neither is a run.

`optional_members` is the third path. It is a sibling of `members` under
`[workspace]`, with the same grammar:

```toml
[workspace]
members          = ["apps/api", "packages/*"]
optional_members = ["vendored"]
```

An entry there is a member the repository has declared **may be legitimately
absent**. Present, it is an ordinary member: every rule in §2 applies to it
unchanged — relative path, no `.` or `..`, resolving strictly inside the block
that lists it, overlapping no other member root — and it is scanned under its own
config and citable at its own alias like any other. Absent (§2.2.1), the block
loads without it and the run continues; the namespace it would have contributed
is **unverified**, a third state beside resolved and unknown (§4), and the run
names it ([§FS-check.4.10](FS-check.md#410-a-workspace-member-declared-optional-is-absent)).

This is an opt-out, not a softer default. A member listed in `members` and
missing still fails, at the same line and with the same verdict; the message
gains one clause naming the key that would have made the absence legal, because a
CI author meeting the refusal should not have to guess that an escape hatch exists
([§FS-config.4.3](FS-config.md#43-invalid-config-behavior)). What `optional_members` buys is a recorded intent, in the
config, next to the entry it describes — which is what a per-run flag could not
be, since a flag records nothing for the next reader and excuses *every* missing
member rather than the one that was meant ([§DF-optional-workspace-members](../decisions/functional/DF-optional-workspace-members.md#df-optional-workspace-members-an-absent-member-is-declared-in-a-sibling-list-and-the-run-announces-the-namespace-it-did-not-check)).

What it costs is a blind spot, and [§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) is what makes that
affordable rather than free: a region nobody wrote down is a hole, a region the
repository declared and every run announces is a bounded skip. The declaration is
this key, the announcement is [§FS-check.4.10](FS-check.md#410-a-workspace-member-declared-optional-is-absent), and the bound is that nothing else
moves — an alias that is not an optional member is unknown exactly as before, and
a namespace that *is* present is checked to the letter.

**A trailing `/*` glob may not be optional.** A glob names namespaces by reading
its parent directory, so an absent parent names none: `hardware/*` in a checkout
without the submodule expands to nothing, contributes no alias, and leaves every
`§hardware/…` citation as unknown as deleting the entry would: the key would
appear to work and do nothing, which is worse than refusing it. An entry ending
in `/*` is therefore a config error at the `optional_members` line, and the
message names the shape to write instead — one concrete entry per namespace,
`hardware/sprayer` and `hardware/pod` rather than `hardware/*` — because a user
who has just been refused needs the form that works, not only the form that does
not. That is the whole cost of the rule: a tree that spells its members with a
glob expands it by hand for the ones that may go missing.

**One entry belongs to one list.** An entry naming a root that a `members` entry
of the same block also names is a config error at the `optional_members` line: the
two lists state opposite intents about one directory, and grund resolving the
contradiction in either direction would silently discard half of what the author
wrote. Roots are compared the way §6.1 compares them, canonically, so a glob in
`members` that expands onto an optional entry is the same collision and is
reported the same way.

**One entry written twice is one member, whether or not it is there.** §6.1
compares members as canonical paths and makes two entries of one list that
resolve to one root the same member — deduped rather than rejected — and an
absent entry is deduped by that same comparison, taken from the entry text before
anything is announced. A repeat is redundant rather than ambiguous: it names one
directory, one alias, and one namespace, and there is nothing for grund to
discard by folding it. Refusing it only in the checkout that lacks the directory
would be worse than either answer, because the same config would then be rejected
by CI and accepted by the developer holding the member — the checkout-dependent
verdict this section exists to remove.

**Every `[workspace]` block reads the key, at every depth.** A nested block's
`optional_members` are paths under *that* block's config root, expanded and
validated by the rules here exactly as its `members` are (§6.1), and its absent
entries are announced at its own `optional_members` line, rendered against the
root this run was launched at like every other diagnostic from a block the run
did not start in ([§FS-errors.4](FS-errors.md#4-determinism)). There is no outermost-block privilege in either
direction: a nested block may declare an optional member whose parent block knows
nothing about it, and an absent one below the run's root costs the same one line
as an absent one at the top. §6.1's ancestor climb reads `optional_members` beside
`members`, from the entry text by the same rule and for the same reason — an
optional entry claims the directory below it, so a run started *inside* a present
one reads its alias path out of that claim and spells itself the way the
workspace root does. An absent entry claims a directory no run can be started
inside.

**A block whose last project goes missing is not an empty block.** §6.1 requires
every block to put at least one project in scope, so `include_root = false` with
no members is a config error at that block's line. That test is read from the
config text, before any path is looked at: a non-empty `optional_members` list
names members, so the block is not empty, and whether they are present is a fact
about the checkout rather than about the config. A block that loses its last
project to an absence therefore does not fail — failing on a checkout is the
verdict this section exists to remove — it contributes no project, its absent
members are announced ([§FS-check.4.10](FS-check.md#410-a-workspace-member-declared-optional-is-absent)), and a run left with nothing to
read still earns the empty-scan caution beside them ([§FS-check.2.2](FS-check.md#22-empty-scan)). §6.1's
glob rule is not the precedent and reads the other way for a reason: a glob that
matches no directories is a mistake in every checkout, while an absent optional
member is a state one checkout has and another does not.

#### 2.2.1 What "absent" means

Absent means **the path is not a directory**, which is the test §2 already
applies to every member and nothing more. A path that does not exist is absent;
so is one that exists as a file, and so is a symlink that does not resolve to
one.

The temptation is to widen it, and the case that tempts is the motivating one.
Git materializes an uninitialized submodule as an **empty directory**, so a
repository whose member *is* the submodule (`members = ["hardware"]`, `hardware`
the gitlink) has a member that exists: it loads under the canonical defaults
(§2), contributes zero declarations, and turns citations into it into "declaration
not found" errors ([§FS-check.3.1](FS-check.md#31-dangling-citation)) rather than unknown aliases. That is a
different symptom from the one this section fixes, and the repository meeting it
has the ordinary repair — name the namespaces under the submodule rather than the
submodule directory, `optional_members = ["hardware/sprayer"]`, which *is* absent
when the submodule is not initialized.

Widening "absent" to "exists but is empty" is refused for two reasons. It would
put a typo'd or half-created directory on the unverified path, which is the
failure class [§FS-check.4.10](FS-check.md#410-a-workspace-member-declared-optional-is-absent) and [§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) exist to prevent: a
directory empty by accident and one empty by a submodule's design are identical
on disk, and grund would be guessing which it had. And it would put the boundary
of the blind spot somewhere a reader cannot see: `optional_members = ["hardware"]`
would mean "skipped" or "checked and found empty" depending on a fact no line of
the repository records. The simple rule is the one an author can plan around, and
the announcement is what keeps even its odd cases honest — a stray file where a
member was expected is skipped, but it is never skipped quietly. Whether the
empty-directory case deserves an answer of its own is a separate question, and
it is not decided here.

#### 2.2.2 The alias of an optional member

An absent member has no config to read, so §3's rule — the alias is the member's
`project_name`, or the directory basename when omitted — has nothing to read it
from. The only name recoverable from the block that lists it is the entry's own
text. So **the alias of an optional member is the entry's last path segment**:
`vendored` for `vendored`, `sprayer` for `hardware/sprayer`. It is that segment
whether the member is present or not.

That last clause is the load-bearing half. A citation's text has to mean the same
thing in a full checkout and a partial one; if it did not, `<§>hardware/sprayer/FS-nozzle`
would resolve in the tree that has the submodule and quietly stop naming anything
in the tree that does not — a trap strictly worse than the one this section
removes, because it appears only in the checkout least equipped to notice. So a
**present** optional member whose `project_name` disagrees with its entry's last
segment is a config error at the `optional_members` line, naming both the
configured name and the segment every citation would otherwise have to use. It is
not resolved in grund's favour in either direction, because either name may be
the one the existing citations already write; the repository picks, by renaming
the directory or by setting `project_name` to match.

The segment has to be a valid alias in its own right (§3): an entry whose last
segment is not a lowercase slug can never name a namespace in either checkout, so
it is the same config error at the same line, said before any directory is looked
for.

**An absent optional namespace absorbs the alias paths beneath it.** A member may
itself declare `[workspace]` (§6.1), and an absent one may have declared
anything; the run cannot know how many levels it had or what they were called. So
a citation whose alias path *begins* with an absent optional member's segment, at
that member's level, is unverified whatever follows it — `<§>hardware/AR-bus` and
`<§>hardware/sprayer/FS-nozzle` alike when `hardware` is the absent entry. That is
what lets one entry stand for a submodule contributing a whole subtree of
namespaces, and it is why listing one entry per namespace (the shape the glob
refusal above asks for) is a choice rather than an obligation: name the subtree's
root when the submodule directory is itself a member, name the namespaces when it
is not.

Recognizing such a citation uses the fixed `KIND[-NUM]-SLUG` fallback shape of
§5, not the target's `[id] format`, for exactly the reason §5 gives: the target's
grammar is unreachable. A qualified tail that does not match the fallback is no
more a citation here than at member scope, and the run over a checkout that
*has* the member stays the place every shape is caught.

## 3. Aliases

The root alias is the root config's `project_name`, or `root` when omitted.
A member alias is the member config's `project_name`, or the member directory's
basename when omitted. A member listed in `optional_members` is the one exception,
and it is an exception in both checkouts: its alias is the entry's last path
segment, and a `project_name` that disagrees with that segment is a config error
rather than a second name (§2.2.2).

Aliases must match the lowercase slug grammar in §1 and must be unique **among
siblings** — the root project and the top-level members share one level, and
each nested `[workspace]` block's members share another (§6.1). A duplicate or
invalid alias is a launch-time error, because a qualified citation would
otherwise have two possible targets. Two projects under different parents may
carry the same alias: their alias paths still differ, so nothing is ambiguous.

A project's optional one-line `project_description` ([§FS-config.3](FS-config.md#3-schema)) follows the
same residency rule as the alias: a member's description comes from the
member's own config, the root row's from the root config, and a member without
its own config has none ([§DF-workspace-member-descriptions](../decisions/functional/DF-workspace-member-descriptions.md#df-workspace-member-descriptions-member-side-project_description-for-workspace-member-lists)).
Unlike the alias it is presentation metadata only — generated workspace member
lists render it beside the alias ([§FS-init.2.3.4.15](FS-init.md#23415-workspace-members)), and it never
participates in alias derivation, citation resolution, or `check` semantics.

## 4. Resolution

During `grund check`:

- `<§>ID` resolves only against declarations in the current project.
- `<§>alias/ID` resolves only against declarations in the named workspace project.
- `<§>alias/ID.section` additionally requires that the target declaration contain
  the cited section.
- an unknown alias is an error at the citation site;
- a known alias with no matching declaration is an error at the citation site;
- an alias path that names, or descends into, an **absent optional member**
  (§2.2) is neither: the citation is *unverified*, and nothing is reported at
  its site.
- diagnostics for a known alias render the `<ID>` and section separator with the
  target project's `[id]` config, not the citing project's config. The literal
  source token remains the scanner's evidence, but the diagnostic names the same
  target that resolution attempted.

Unverified is the third state, and it is reported once per namespace rather than
once per site. The run names the namespace at the `optional_members` entry that
made the skip legal ([§FS-check.4.10](FS-check.md#410-a-workspace-member-declared-optional-is-absent)) and says nothing where the citations are,
because there is nothing true to say there: the citation may be perfect and the
checkout merely partial, and a tree that cites an absent namespace widely would
pay thousands of lines to be told one fact it can be told once. What must not
happen is the third possibility — that the run says nothing anywhere. That is the
trade [§REQ-no-missed-citation.2](../requirements/REQ-no-missed-citation.md#2-every-blind-spot-is-declared-and-bounded) licenses and [§FS-check.4.10](FS-check.md#410-a-workspace-member-declared-optional-is-absent) is the price of.

Cross-project references are deliberately never resolved by path syntax such as
`../FS-login` or `packages/api/FS-login`; aliases are the stable handles.

## 5. Command scope

`grund check` run at a workspace root checks the root project and all configured
members, aggregates diagnostics, and prints the same `path:line: message` shape
as a normal check ([§FS-check.2.1](FS-check.md#21-report-format)). Paths are rendered relative to the workspace
root when `[output] relative_paths = true`.

`grund check <member>` (or `grund check` invoked from inside a member tree)
discovers the member's own config first and validates it as an independent
project. Qualified citations such as `§root/<ID>` or `§sibling/<ID>` cannot be
resolved without the workspace context; each such citation produces an
`unknown project alias <name>` error at the citation site
([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)). This matches [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6 — silent skipping
would let a passing member check ship a cross-project reference that no longer
resolves at the workspace root, which violates [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration). Run
`grund check` at the workspace root to validate cross-project citations.

The member-local scanner recognizes the `<§>alias/` prefix before applying the
member's local ID grammar. That way `<§>root/FS-root` is still an unknown-alias
citation inside a default `{kind}-{number}-{slug}` member, rather than plain
text that disappears from `check`.

Member-local recognition uses a fixed fallback ID shape — `KIND[-NUM]-SLUG`
with an uppercase-or-digit kind and a non-empty slug — because the workspace
catalogue (and therefore each target's `[id] format`) is unreachable at member
scope. A qualified citation whose tail does not match that shape (lowercase
kinds, slug-only ID grammars that don't split on `-`/`_`, kinds with
non-`[A-Z0-9]` characters) is not flagged at member scope; the workspace-root
run, which parses each qualified tail with the target project's grammar, is
the one place that catches every shape. Run `grund check` at the workspace
root for full coverage.

A relaxed standalone mode (downgrade `unknown project alias` from error to
warning on a member-only run) is deferred follow-up; see
[§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6.

## 6. Nested project boundary

Workspace members are namespace boundaries. When the root project is checked as
part of a workspace, the root scan does not scan member project directories or
their descendants, even if the root project's `[scan] include` names a path
inside a member. The member is scanned separately under its own config and alias.

This prevents a child project declaration from accidentally becoming a duplicate
or dependency of the root namespace. It also has a limit: a `members` list that
prunes *every* one of the block's own walk roots leaves that block reading
nothing at all, which is a misconfiguration rather than a boundary, and §2.1 is
where the run says so.

**The boundary is mutual, and it belongs to the directory rather than to the path
that reaches it.** A member's own scan stops at every *other* project in the
workspace exactly as the root scan stops at the members — at a sibling member's
files, and at the root project's. Ordinary descent cannot cross the line, since
no project root contains another except along the workspace tree itself, but a
symlink can, and a symlink is followed ([§FS-config.3.5.1](FS-config.md#351-a-symlink-in-the-tree-is-followed)).
So the directory a link resolves to is asked which project owns it — the
innermost project root that contains it — and a directory owned by another
project is not descended into, whichever project's walk met it and under
whatever name. The harm is the same in every direction: `packages/a/docs/b ->
../../b` files `b`'s declarations under `a`'s namespace and reports them as
duplicates of themselves, which is what this section forbids of the root scan. A
directory that belongs to **no** project in the workspace is not a boundary: it
is outside content the repository deliberately linked into the tree, and it is
read there like any other ([§FS-config.3.5.1](FS-config.md#351-a-symlink-in-the-tree-is-followed)).

A member checked on its own is an independent project (§5) and does not load the
workspace, so it cannot know where the other projects are; this boundary is a
property of a run that loaded them, not of the member's own config.

### 6.1 Nested workspaces

A member may itself declare `[workspace]`. Its `members` are paths under *that*
member's config root, resolved and validated by the rules in §2 exactly as the
outermost root's are. The tree may nest to any depth.

A project is named by its **whole alias path**: one segment per level, read from
the outermost workspace down (§1). The outermost root contributes no segment, so
a workspace with no nesting is spelled exactly as it is today. Given a root with
`members = ["hardware", "final"]` and `hardware` declaring
`members = ["sprayer", "pod"]`:

```text
§root/GOAL-x            the root project        §final/FS-x     a top-level member
§hardware/AR-bus        the node's own decl     §hardware/sprayer/FS-x   a leaf
```

Paths, not globally unique names, are what let two `pod` projects under
different parents coexist ([§DF-nested-workspaces](../decisions/functional/DF-nested-workspaces.md#df-nested-workspaces-a-nested-project-is-named-by-its-whole-alias-path)) — hence the per-level
uniqueness rule in §3. The cost is that a short leaf name no longer resolves on
its own, and `grund check` answers that mistake by naming the projects the
written path could have meant ([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)).

`include_root` is read per `[workspace]` block and governs that block's own
project. Under the default an intermediate node is a project like any other: it
derives an alias (§3), is scanned under its own config minus its members'
subtrees (§6), and is citable at its own path. Under `include_root = false` it
is pure grouping — no project, but still a segment in every path below it, so
`<§>hardware/sprayer/<ID>` is unaffected either way. What it costs is that the
node's own files are then scanned by **nobody**: the enclosing scan stops at the
member boundary (§6) and no other scan covers that directory, so a declaration
there is in no catalog and a citation there is never checked — not even under
`--full` ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full)), which
widens a project's scope and has no project to widen here. `grund check` stays
green over content nothing reads, which is precisely why the default is a project
([§DF-nested-workspaces.3.5](../decisions/functional/DF-nested-workspaces.md#35-the-intermediate-node-reuses-include_root-and-defaults-to-a-project)).
Every block must put at
least one project in scope, so `include_root = false` with no members is a
config error at that block's `members` line — or at its `[workspace]` line when
there is no `members` key to point at, since a tree may hold many blocks and the
error has to say which one is empty. An explicitly empty `members = []` is the
same no-members case. A non-empty list whose glob entries all match no
directories is different: at the `members` line it says “the glob
`packages/*` matched no directories”, naming the first unmatched glob in list
order when there is more than one. This diagnostic belongs to the empty block,
not to each empty glob independently: an included root or another member that
does put a project in scope keeps the block valid.

Members are compared as canonical paths. Two entries in *one* `members` list that
resolve to the same root are the same member — deduped, not rejected, so a glob
may legitimately name a directory an explicit entry also names (`members =
["packages/*", "packages/api"]`); what §2 rejects there is one expanded root
*containing* another. An entry that resolves to a project root **another** block
already holds is a config error at the `members` line that introduced it. What
makes expansion terminate is containment: a member root resolves strictly inside
the block that lists it and no member of one block contains another, so every step
goes strictly downward into a finite tree and no two roots in it can be equal —
which is also what makes that duplicate error a backstop rather than a rule a
config can trip ([AR-workspace.6.1](../architecture/AR-workspace.md#61-nested-workspaces-are-one-recursion-not-a-second-namespace-model)). So is one that escapes
its own block (§2): no lexical ancestor lists such a root, so nothing gives it a
stable alias path, and a root *above* its own block scans nothing at all — every
scan root lies under its own member boundary, so the project's declarations
vanish and its dangling citations pass.

Scope follows §5 unchanged: discovery stops at the *nearest* config
([§FS-config.1](FS-config.md#1-file-location-and-discovery)), so a command invoked at an intermediate node runs that
node's subtree. **Alias paths do not change with scope** for every scope *in* the
**claimed chain** — the `[workspace]` blocks from the outermost root down, each
listing the directory below it among its `members` — so a run started at any of
those blocks resolves a *subset* of the same paths rather than a re-spelled set of
its own. The guarantee is quantified over the **scope**, not over the project: a
block the chain never lists is not one of those scopes even when the projects
*under* it are reached by the chain, which a multi-segment entry (`grp/inner`)
makes routine, and a run started at such a block names every path from itself. A
citation can therefore pass that block's check and fail the run CI does — the
disagreement the third rule below reports, not a property of a chained scope.
Inside `hardware/`, `<§>hardware/sprayer/<ID>` still names what it names at the
repository root and `<§>final/<ID>` is simply unknown — unknown *here*, which is
what the narrowed run's diagnostic says instead of proposing a project it does
hold ([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)). The alternative — naming a
subtree's projects from the subtree — would make that disagreement the rule at
*every* scope rather than the recorded exception at one: a citation passing a
subtree check and failing the run CI does, which is
[§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) failing in the one place it has to hold.

Three rules keep one chain readable from every scope. **A path is read from the outermost block that claims a directory:** a multi-segment `members` entry (`grp/inner`) hops a directory that may itself declare `[workspace]` and list the same child, and the outer claim is the one the walk down from the outermost root follows — ordinary nesting has one claim per directory, where the two agree.
**A block that claims a directory and cannot answer** — a missing member, overlapping roots, an invalid alias for the project below it, or a config that does not load at all — fails the run with *its own error*, from its own `members` or `project_name` line — rendered against the root **this run** was launched at, so a block above that root renders with `..` (`../grund.toml:16`) and the reader lands on the file that holds the line rather than on a same-named one inside the subtree ([§FS-errors.4](FS-errors.md#4-determinism)); dropping its segment would let the subtree invent a namespace, and [§FS-check.3.8](FS-check.md#38-cross-project-citation-failure) would then hint the one spelling that fails at the root. The obligation is a *claim's* and a **workspace run's**, and only theirs — a claim the climb that spells *this run's own* alias path had to ask: that climb happens because a run has a path to read, so a run at a project that declares no `[workspace]` block of its own — a leaf member, or any single-project repository — reads no path out of the chain. Such a run resolves one project (§5): it has no alias path to get wrong, every qualified citation is an unknown alias whatever an ancestor lists, and no claim above it, answered or not, can fail it. The claim rule is about the scopes that *do* read a path, which are the blocks. The chain is still *asked* about such a run, by a second climb that reads no path out of it: [§FS-check.4.9](FS-check.md#49-unlisted-workspace-block)'s rule walks these same ancestors, with this same `members`-only read, about a `[workspace]` block the run's own walk met rather than about the run's own name — so what an ancestor lists decides whether that block is reported. That question carries none of the obligations here, because it spells nothing: it fails no run, and an ancestor it cannot read leaves the claim unanswered and the block unreported rather than costing the reader a line. A block that does not name this directory is not asked, so neither a `members` list it could not expand nor a config that would not load at all is an error in a run below it. Otherwise one broken config anywhere above a repository — at any depth up to `/`, in a workspace that never mentions it — would answer every command inside it. The claim is therefore read from the **`members` entries alone**, never from a loaded config: the `members` value is parsed on its own — no other key read, no shape rule applied — so a config that fails to load is still asked whether it claims this directory. Deciding it from a loaded config instead made *every* load failure above a repository silently equal to "claims nothing", which is the collapsed prefix this rule exists to prevent, and two mistakes on one `members` line then behaved oppositely: a member that does not exist failed the subtree run, while an entry the shape rule rejects ([§2](#2-workspace-configuration)) let it re-spell itself. A config whose `members` text cannot be obtained at all — the file cannot be read, or its `members` value is not a list — leaves the claim undecidable in both directions. The run continues, because a stray unreadable `grund.toml` above a repository is not that repository's problem, and whether it says so is the asking climb's: the one spelling an alias path never continues silently — it prints a run-level `warning:` naming that config and saying alias paths below it may be missing a segment ([§FS-errors.2.2](FS-errors.md#22-cli-level-message)) — while the quiet climb has no path below it to warn about, so it says nothing and leaves the block it was asking about unreported.
**A `[workspace]` block that no enclosing block lists is outside the chain:** at the outer scope it is ignored, so its tree belongs to the enclosing project's namespace when that project's scan reaches it and to nobody when it does not, while a run started **at** it names every path from itself — a run started at a block *below* it that the chain does list is back inside the guarantee. A run whose own tree walk meets such a block reports it, naming the block's `[workspace]` line and saying that the projects under it are absorbed into the enclosing namespace instead of named under their own alias path ([§FS-check.4.9](FS-check.md#49-unlisted-workspace-block)). Two shapes stay unreported, for different reasons. A block the walk never reaches — behind `[scan] exclude`, an ignore file, a member boundary, or a narrowed scope — is the known limitation, because a run that cannot see something does not judge it. A block an enclosing config *names* and then cannot answer for is the undecidable claim of the rule above, left alone because no answer is not the answer that nothing claims it.

## 7. Neighboring repos

Neighboring repositories in the same organization should use the same external
syntax, for example `<§>payments/FS-refunds`, but external repository resolution
is not part of this first implementation. It requires an explicit cache or
lockfile so ordinary `grund check` remains offline, deterministic, and fast
([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)). Until that cache layer exists, aliases are
workspace-local.

## 8. Other commands

The workspace surface composes through the same resolver `grund check` uses
([AR-workspace.4](../architecture/AR-workspace.md#4-the-resolver-one-function)), so qualified-ID behavior in query commands is a UX layer over
an already-built engine — not new resolution logic. Four shared rules apply to
every command in this section:

- **Discovery follows the same walk-up rule as `grund check`** ([§FS-config.1](FS-config.md#1-file-location-and-discovery),
  §5): from the CWD (or from an explicit `<path>` argument), walk up to the
  nearest `grund.toml` in either discovery form. If the nearest config is a member's own config,
  the command runs member-local — qualified `<alias>/<ID>` cannot resolve, the
  same way `check` errors at the member scope (§5). If the nearest config is
  the workspace root, the command runs workspace-wide. An explicit `<path>`
  argument (e.g. `grund list apps/api`, `grund refs FS-x apps/api`,
  `grund complete ids --path apps/api`) behaves as if the command were invoked
  from that path: a `<path>` inside a member is member-scoped, not
  workspace-aggregate, even when a workspace exists above it.
- **The "current project" is what an unqualified ID resolves against** (§4):
  the root project at the workspace root, the member project inside a member
  tree. When `include_root = false` and the command is invoked at the workspace
  root, there is no current project; commands that accept a single ID reject an
  unqualified `<ID>` and require `<alias>/<ID>` instead. Cross-project lookups
  always require the `<alias>/<ID>` form. There is no `--all-projects` flag; the
  alias *is* the scope handle.
- **`include_root = false`** (§2): when the root project is excluded from the
  workspace, it has no catalog entry and its alias is not known. `<§>root/<ID>`
  (or whatever name `project_name` would have assigned) is treated as any other
  unknown alias by every command in this section — the root alias is not
  silently reserved. Output paths still render from the workspace root, not from
  the first member. Completions, `show`, `refs`, `cover`, and `list --project`
  all agree on this.
- **A malformed alias path is rejected before the scan, and the diagnostic names
  the offending segment** — `Sprayer` for `grund hardware/Sprayer/FS-x`, not the
  whole `hardware/Sprayer`. The path is one slug per level (§1), so the mistake
  is always in a segment; naming the path against a pattern that forbids `/`
  would read as "a namespace may not contain `/`", which is the opposite of the
  rule. An empty segment (`hardware//FS-x`) has nothing to quote, so the
  diagnostic says a segment is empty; a leading `/` (`/FS-x`) says the path is
  empty — never empty backticks. An alias path that is *well formed* but names no
  project is the unknown-alias error of §8.1, not this one.

### 8.1 `grund <alias>/<ID>`

`grund <alias>/<ID>` reads a declaration in another workspace project. The
body is rendered exactly as `grund <ID>` renders it for a local declaration
([§FS-show.2](FS-show.md#2-behavior)) — same slice rules (`--brief`, default, `--toc`, `--full`), same
`text` vs `md` heading behavior, same section selection, same inline-code
extraction ([§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments)). The alias prefix is a routing instruction; it
changes which tree is scanned, not what is printed.

- `grund api/FS-login` — print the lead of `api`'s `FS-login`.
- `grund api/FS-login.3.1 --toc` — print the `3.1` section's lead plus its
  nested heading map, against `api`'s declaration.
- `grund FS-login` invoked from inside `apps/api/` — print the member's
  local `FS-login`. Unchanged from today; no workspace context is needed
  because the citation is local.

Under `--format json` the reported `path` is relative to the config root the
command resolved against, matching `grund list` ([§FS-config.3.6](FS-config.md#36-output--report-format)) — so
`grund api/FS-login --format json` from the workspace root reports
`apps/api/…`, not the member-relative `…`. Consumers that join this path
against the directory they invoked `grund` in, such as the `grund-open`
resolver ([§FS-integrations.3.1](FS-integrations.md#31-terminal-clients-wezterm-kitty-tmux-iterm2)), depend on that base.

Outside a workspace context — including a `<path>` argument that resolves
member-local — `grund api/FS-login` exits `2` with two stderr lines:

```text
error: unknown project alias `api`
note: workspace aliases are defined in the root grund.toml under [workspace]
```

Ambiguity within a project is unchanged ([§FS-show.2.2.1](FS-show.md#221-ambiguous-id)). An ID that exists in
two *different* projects is not ambiguous — they are two declarations in two
namespaces, and the alias picks one.

#### 8.1.1 An unqualified ID another project declares

`grund <ID>` resolves against the current project alone (§4), so an ID that only
a *member* declares comes back not found — while `grund list`, in the same
invocation, can say exactly which project has it. In a workspace run the refusal
therefore names the projects that do declare it, before it gives up:

```text
ID not found: SPEC-007-shipping; did you mean vendored/SPEC-007-shipping?
```

**It still refuses**: exit `1`, stdout empty, no body printed. The unqualified
form is not resolved into the member, because §4 is what `grund check` enforces
and an unqualified cross-namespace citation is an error at its site — a query
command that printed the body would teach the reader to write a citation CI
rejects, and one resolver across `check` and the query commands is the direction
[§DF-subproject-namespaces.3.7](../decisions/functional/DF-subproject-namespaces.md#37-check-comes-first-query-commands-follow) states. Naming the qualified form gets the reader
the same fact in one step *and* the spelling that resolves. `grund show <ID>`
and the bare `grund <ID>` that defaults to it ([§FS-cli.1](FS-cli.md#1-the-default-subcommand)) print the one line.

The shape is [§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)'s `unknown project alias`, which settles these same
questions one level up, for alias paths rather than for IDs:

- **The candidate is appended**, after the ID, so the line still opens
  `ID not found: <ID>`. That prefix is the diagnostic's identity — it is what
  selects the `not-found` code (§8.7, [§FS-errors.5](FS-errors.md#5-json-format)) — so a candidate written
  ahead of it would trade that class away for a few characters of prominence.
- **Several candidates are listed, never chosen.** Two projects declaring one ID
  is not an `ambiguous ID` error — §8.1: they are two declarations in two
  namespaces — and picking one would be a guess
  ([§REQ-no-wrong-citation.1](../requirements/REQ-no-wrong-citation.md#1-no-wrong-resolution)). They are joined as [§FS-check.3.8](FS-check.md#38-cross-project-citation-failure) joins
  its own — `did you mean left/FS-shipping or right/FS-shipping?` — sorted, and
  cut at three: `grund list` is the catalogue, a diagnostic is not.
- **Each project is asked in its own grammar.** The written ID text is re-parsed
  with the candidate project's `[id]` config and rendered back with it, the way
  a qualified citation already is (§1, §4). In a mixed-format workspace one text
  is a different ID in each project, so carrying the current project's parse
  across the boundary would find nothing and report no candidate at all.
- **A narrowed run offers none.** A member-local run, a standalone repo, or a
  `<path>` that resolves inside a member holds one project (§5, §8): there is no
  second project to name, and the bare line is printed unchanged.
- **The section is not carried.** `grund SPEC-007-shipping.2` could not find the
  ID, so the candidate names the ID — `did you mean vendored/SPEC-007-shipping?`
  — and the reader asks that project for the section. Suggesting a coordinate
  this run never looked for would be the one part of the line that is a guess.
- **The `grund list` hint gives way.** Where a candidate is named, the
  `ID not found` hint line of [§FS-show.3](FS-show.md#3-outputs) is not printed: it sends the reader to
  the catalogue this line has already searched, and its other half — propose a
  new ID — is advice for an ID that does not exist. Where there is no candidate
  the hint prints exactly as before.
- **`--format json` keeps its shape.** The diagnostic still carries
  `"code":"not-found"`, with the candidate inside `message` ([§FS-errors.5](FS-errors.md#5-json-format)). The
  hint line has no JSON form under either branch.

### 8.2 `grund refs`

`grund refs <alias>/<ID>` lists every citation of that qualified declaration,
across the **whole workspace**, including:

- citations written `<§><alias>/<ID>` anywhere in the workspace (the
  cross-project form), and
- citations written `<§><ID>` **inside the named project's tree** (the
  project's own local form for the same declaration).

The two forms cite the same target (§4), so they appear in the same `refs`
result. This is what makes `refs` a blast-radius answer: an author about to
delete `api`'s `FS-login` learns about both `api`'s own files and every other
project that wrote `<§>api/FS-login`.

- `grund refs api/FS-login` — listed exactly as [§FS-refs.3.1](FS-refs.md#31---format-text-default) specifies; the
  `text` column shows each citation verbatim — `<§>api/FS-login` from sibling
  projects, `<§>FS-login` from inside `api`.
- `grund refs FS-login` invoked at the workspace root — citations of the
  *root's* `FS-login` only (root is the current project). To get cross-project
  occurrences, qualify: `grund refs root/FS-login` is the same query in this
  context and is the canonical form for scripts.
- `--summary` ([§FS-refs.3.3](FS-refs.md#33---summary)) aggregates per file regardless of which member the
  file lives in. Paths render relative to the workspace root when
  `[output] relative_paths = true`.
- `--format json` adds one new field on the per-citation object:
  `"project": "<alias>"` — the alias of the project that *contains* the
  citation site, not the target. The target project is the query argument
  itself: `<alias>` for a qualified lookup, the current project for an
  unqualified lookup. It is not repeated per row; per-row redundancy would
  balloon the wire size of a wide blast-radius scan without adding information
  the caller did not just hand to `refs`. The object's `"id"` field is rendered
  with that target project's `[id]` config; the `"text"` field remains the
  verbatim source citation.

The "neither declared nor cited" stderr note ([§FS-refs.2](FS-refs.md#2-behaviour)) becomes alias-aware:

```text
note: api/FS-login is neither declared nor cited — run `grund list --project api` to see api's declared IDs
```

### 8.3 `grund list`

`grund list` invoked at a workspace root prints the catalog of every project
the workspace covers (root plus members, subject to `include_root` per §8
intro), so the resulting catalog has one row per declaration with no collisions
even when two projects declare the same local ID.

Output changes:

- The ID column renders as `<alias>/<ID>` for every row **whenever workspace
  mode is loaded**, including the current project's and including the narrowed
  catalog under `--project`. Always-qualifying keeps the column self-labeled —
  `FS-login` next to `api/FS-login` in the same dump would read as a third
  project named `(local)` — and keeps script output dependent on what the
  command returned, not on how the user invoked it. The only way to get
  unqualified rows is to invoke `list` member-locally (no workspace context —
  including via a member-local `<path>` argument).
- A new optional filter, `--project <alias>[,<alias>...]`, narrows the catalog
  to one or more named projects. `--project api` returns only `api`'s
  declarations, still rendered with the `api/` prefix per the rule above. An
  unknown alias is a CLI-level error, exit `2`, same shape as `--kind`
  ([§FS-list.4](FS-list.md#4-exit-codes)).
- `--kind` composes with `--project` (intersection):
  `--kind FS --project api,payments` lists FS declarations in those two
  projects.

`grund list --summary` ([§FS-list.3.3](FS-list.md#33---summary)) gains a new variant when a workspace is
loaded: rows are emitted per `(project, kind)` pair, sorted by the whole alias
path — the same order the catalog's `<alias>/<ID>` rows above use — then by
that project's configured kind order; `--format json` emits in the same
order. `--project <alias>` narrows the summary to that project's kinds, and
the alias column is sized to the widest alias among the rows emitted after
that narrowing, capped the way the ID column is ([§FS-list.3.1](FS-list.md#31---format-text-default)).

`--format json` adds `"project": "<alias>"` to every object and renders `id` in
the qualified form (`api/FS-login`). The `refs` count is the count under that
project's qualified target, computed exactly as §8.2 defines.

### 8.4 Shell completions

The dynamic helper `grund complete ids` ([§FS-completions.2](FS-completions.md#2-internal-dynamic-helper)) gains workspace
awareness.

Completion grammar (the prefix the user has typed so far):

- No `/` in the prefix → bare-ID candidates from the current project (existing
  behavior, unchanged). When the helper is invoked at the workspace root,
  **also** emit one candidate per known alias path, with a trailing `/` —
  `api/`, `payments/`, `group/alpha/`, and `root/` only when
  `include_root = true` (§8 intro). The trailing slash is the continuation
  signal — see "Shell script adjustments" below.
- Prefix contains a `/` → split on the **last** `/`, the same boundary a
  citation uses (§1). The left side names a project; emit its IDs whose
  qualified form matches the prefix, **and** every known alias path that
  continues past the prefix, again with a trailing `/`. So `group/` offers
  `group/FS-x` alongside `group/alpha/` and `group/beta/`, and one more Tab
  after `group/alpha/` reaches that project's IDs. The prefix itself is never
  re-offered as a candidate: it would stall the shell rather than advance it.
- `--sections` and the implicit section mode (prefix containing the configured
  `[id] section_separator`) compose with the alias prefix in the natural way:
  `api/FS-login.` triggers section candidates against `api`'s declaration of
  `FS-login`.

Helper errors stay quiet ([§FS-completions.2](FS-completions.md#2-internal-dynamic-helper)): a workspace that fails to load
drops back to single-project mode so a user typing in the shell never sees a
stack trace.

**Shell script adjustments.** Alias-as-completion only feels right if a Tab
from `api` advances to `api/` *without inserting a space* — otherwise the user
types `api`-Tab-Backspace-`/` instead of `api`-Tab-Tab. The generated scripts
therefore *do* change for this case (bare-ID completion remains byte-identical
for repos with no workspace):

- `bash` — the completion function calls `compopt -o nospace` for the current
  invocation whenever any candidate in the batch ends in `/`.
- `zsh` — the completion function partitions the candidate batch:
  slash-suffixed candidates are added with `compadd -S ''` (no auto-suffix),
  bare-ID candidates with the default suffix.
- `fish` — candidates are emitted via `complete -f -k`; fish appends no space
  when the candidate's last character is `/`.

These adjustments are testable per shell and must be covered before
`<alias>/`-completion is advertised. Until each shell's no-space behavior is
verified by a fixture, the helper may fall back to emitting full
`<alias>/<ID>` candidates in one shot (one Tab completes both the alias and an
ID), at the cost of needing to type a disambiguating letter first.

### 8.5 `grund fmt --cross-refs`

The link wrapper ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission)) becomes workspace-aware when invoked at the
workspace root.

- A qualified citation `<§>api/FS-login` in `docs/index.md` wraps to
  `[<§>api/FS-login](../apps/api/docs/functional-spec/FS-login.md#fs-login-...)` —
  the relative path crosses the workspace into the member's home, the anchor
  is computed from the *member's* declaration heading under the member's
  configured anchor profile ([§FS-fmt.6.7](FS-fmt.md#67-configurability)). The cross-project resolution goes
  through `target_findings_for_citation` ([AR-workspace.4](../architecture/AR-workspace.md#4-the-resolver-one-function)) so the wrapped link
  and `check`'s resolution can never disagree.
- A member-local run (invoked inside `apps/api/`, or via a `<path>` that
  resolves member-local — see §8 intro) **leaves qualified citations
  untouched**: per §5, the member run cannot resolve qualified targets and
  `--cross-refs` does not paper over a citation that `check` would error on
  ([§FS-fmt.6.4](FS-fmt.md#64-what-is-never-wrapped)). A previously-emitted wrapper around a qualified citation —
  e.g. `[<§>root/FS-x](../../docs/FS-x.md#...)` left by an earlier
  workspace-root pass — is **preserved as written**: not stripped (which would
  destroy information the member-local run cannot recompute), not re-derived
  (the resolver has no canonical URL to compare against in this context). It
  will be re-derived the next time `fmt --cross-refs` runs at the workspace
  root. The single canonical way to create or refresh cross-project wrappers
  is the workspace-root run.
- Re-derive ([§FS-fmt.6.3](FS-fmt.md#63-idempotency-and-re-derive)): a heading rename or a file move in `api` triggers a
  one-line `fmt` diff in any *other* project that wrapped a citation of the
  renamed thing, exactly as it triggers a diff in `api`'s own files. The
  workspace-root run is what makes that single pass possible.

`[fmt.cross_refs]` config ([§FS-fmt.6.7](FS-fmt.md#67-configurability)) is read from each member's own config
when wrapping a citation that targets that member — the member's
`anchor_format` wins for its own declarations. Mixed-profile workspaces (rare)
render each project's anchors under its own configured profile, consistent
with the per-member config rule ([AR-workspace.5](../architecture/AR-workspace.md#5-the-config-one-parse-one-validation-pass)).

### 8.6 `grund cover`

`grund cover` invoked at a workspace root indexes **every project the workspace
covers** — root plus members, subject to `include_root` per the §8 intro — with
one entry per scanned file, exactly as §8.3 defines the catalog for `list`. A
member-local invocation (or a `<path>` that resolves member-local) indexes that
member alone, unchanged from a standalone run.

`cover` is the one command in this section whose `<path>` bounds a **walk**
rather than choosing which config answers ([§FS-cover.1](FS-cover.md#1-inputs)), so it draws the
aggregate/narrow line where `grund check` draws it and not where `list` does: a
scope narrower than the config root is one narrowed scan of the enclosing
project, no workspace loaded and no `project` field, the way `grund check <dir>`
already behaves ([§FS-check.1.3](FS-check.md#13-the-full-tree-scope---full)). Widening `grund cover src/` back to every
project would answer a question the caller did not ask, and an explicit path
deliberately bypasses `[scan] include`, so the narrowing is the only thing that
put those files in scope at all.

`cover`'s question is "which IDs does this file lean on?" ([§FS-cover.5](FS-cover.md#5-why-this-exists)), and the
answer for a file is the same fact whichever scope the run was launched at. A
per-project index would make the co-change recipe ([§RM-cochange-gate](../roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)) read a
changed member file as uncovered, and a coverage index that omits whole
projects while exiting `0` is the silent skip [§REQ-no-missed-citation.1](../requirements/REQ-no-missed-citation.md#1-no-silent-skips) forbids.
Rationale and the discarded project-local alternative: [§DF-cover-workspace-scope](../decisions/functional/DF-cover-workspace-scope.md#df-cover-workspace-scope-cover-indexes-the-whole-run-and-counts-cross-project-citations).

- **Qualified citations count toward the citing file.** A `<§><alias>/<ID>`
  written in `docs/index.md` is one of that file's citations, listed at its
  `(line, column)` like any other. It is what the file leans on; dropping it
  reports a fully grounded file as citing nothing. This holds outside a
  workspace too — a qualified citation in a standalone project is still a
  citation the file carries, and it is `check`'s job, not `cover`'s, to call
  the alias unknown (§8.1).
- **The rendered `id` says what the token says**, canonically: `api/FS-login`
  for a citation written qualified, the bare ID for a local one, rendered under
  the **target** project's `[id]` config exactly as `refs` renders it (§8.2).
  Qualifying a local `<§>FS-login` would report something other than what the
  file wrote, and reporting what the file wrote is the whole job. The target a
  row names is therefore `id` when it carries a `/`, and `<project>/<id>`
  otherwise — one join against the field on the same object. `text` stays the
  verbatim source token either way.
- **Paths render from the workspace root** when a workspace is loaded and
  `[output] relative_paths` is left at its default, so a member's file is
  spelled the way `[workspace] members` spells it and the recipe can join it
  against the same base `git diff` reports. Under `relative_paths = false` the
  base is the command's path argument, as it is for every other command
  ([§FS-config.3.6](FS-config.md#36-output--report-format)); a member outside
  that base is reached with the minimum `..` components while remaining inside
  the loaded workspace, never by falling back to its absolute path. Scan errors
  from any project render against whichever base the rows did.
- **`--format json` adds `"project": "<alias>"`** to the per-file object and to
  each nested citation object whenever workspace mode is loaded — the alias of
  the project that *contains* the file, which is also the citing project. The
  nested objects keep `refs --format json`'s **field** shape — same names, same
  order, this field included ([§FS-cover.3.2](FS-cover.md#32---format-json)) — not every value: `id` differs by
  the rule above, because `refs` was handed the alias in its query argument and
  `cover` was not. Outside workspace mode no field is added.
- **`include_root = false`** removes the root project's files from the index
  along with its catalog entry, per the §8 intro. Nothing else scans them
  (§6), which is the hole that rule already documents.

### 8.7 Output and exit codes

All six surfaces above keep the exit codes they had:

- `show` — `0` body printed, `1` ID/section not found or ambiguous, `2` CLI/
  scan error. An unknown alias is `2` (it is a CLI-shaped error, not a "found
  something else" error) and matches the standalone-mode shape in §8.1.
- `refs` — `0` always when the scan succeeds; `2` on scan/CLI error.
- `list` — `0` always when the scan succeeds; `2` on scan/CLI error (now
  including unknown `--project`).
- `cover` — `0` always when the scan succeeds; `2` on a scan error in **any**
  loaded project, since the index is then incomplete for the tree the run
  claimed ([§FS-cover.4](FS-cover.md#4-exit-codes)).
- Completion helper — quiet failures, exit `0`, unchanged from [§FS-completions.2](FS-completions.md#2-internal-dynamic-helper).
- `fmt --cross-refs` — unchanged from [§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk).

Paths in every command respect `[output] relative_paths` as `check` already
does (§5). That includes the `error: <path>: <reason>` line a per-file scan
failure earns ([§FS-check.2](FS-check.md#2-outputs)): it is spelled from the
run's configured base — the workspace root by default, or the path argument/cwd
under `relative_paths = false`, with in-workspace parent components as specified
by [§FS-config.3.6](FS-config.md#36-output--report-format) — the same line
`check` prints for the same tree, whichever command met the failure first.
