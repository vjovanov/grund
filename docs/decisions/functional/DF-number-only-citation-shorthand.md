# DF-number-only-citation-shorthand: the number-only shorthand is authoring sugar, and a persisted one is a check error

**Status:** Accepted
**Date:** 2026-08-14

## 1. Context

`grund init` writes `[id] format = "{kind}-{number}-{slug}"` by default ([§FS-config.3.2](../../functional-spec/FS-config.md#32-id--id-grammar)), so most repositories that adopt `grund` are on a format where **the number alone already identifies the declaration within its kind**: `grund id` issues numbers strictly above the maximum and never reuses one ([§FS-id.4](../../functional-spec/FS-id.md#4-next-number-derivation)), so `FS-042` names exactly one `FS-042-…`.

People will therefore write `§FS-042`. It is shorter, and it does not go stale when the topic drifts away from the slug — an ID is an immutable handle ([§FS-non-goals.4](../../functional-spec/FS-non-goals.md#4-cross-workspace-id-renaming)), so a drifted slug is never rewritten and the full citation keeps naming the old topic forever.

Before this record, `grund` had **no position on the shorthand at all, and the failure mode was silent**. Against a declaration `FS-042-user-login`:

```
$ grund check .
requirements.md:1: declared but never cited: FS-042-user-login

$ grund refs FS-042-user-login
(no output)

$ grund FS-042
invalid ID `FS-042`
```

A `§`-marked token that every human reads as a live citation was dropped on the floor: not resolved, not flagged, absent from `refs` and `cover` — and the declaration was *simultaneously* reported as uncited, which is the opposite of the truth and invites a reader to delete a live declaration. A dangling *full* ID (`§FS-999-nope`) errors correctly; only the shorthand shape vanished.

[§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) states the standard this violates in one line: *false negatives are bugs*. The rule below is the fix; which rule it is matters less than that it is the same rule on every surface.

## 2. Decision

### 2.1 The shorthand is authoring sugar; persisted citations stay canonical

`§FS-042` is **not** a second stored citation grammar. It is a shape `grund` recognizes so it can be resolved at an input boundary and corrected in a file — never a form that is allowed to persist.

This is the same principle [§DISC-declaration-local-shorthand](../../discussions/proposals/2026-05-24-declaration-local-shorthand.md#disc-declaration-local-shorthand-declaration-local-shorthand-for-citing-sections-of-the-same-declaration) already records for section-local shorthand — *persist canonical citations, keep resolution explicit* — and that note reaches the same verdict in advance: "Persisted marker shorthand such as `§2` should not be accepted by `check` as a citation. If supported at all, it should be reported or corrected to the full canonical form."

Two properties pay for it:

- **One greppable string per edge.** `grep -r 'FS-042-user-login'`, `grund refs`, and `grund cover` all see the same token. A resolvable alias would split every ID into two forms, and a reader grepping the canonical one would silently miss half the graph.
- **A citation is readable without a lookup.** `§FS-042-user-login` tells a reader what it points at; `§FS-042` makes them run a command. The slug is the whole reason the citation is worth reading inline, which is the [§GOAL-token-economy](../../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) case for the long form even though it costs more characters.

### 2.2 Where the shorthand is accepted, and where it is an error

| Surface | Shorthand | Why |
| --- | --- | --- |
| Typed after the trigger (`$$FS-042`) | **Expands** to `§FS-042-user-login` | Authoring. The sugar's whole point. |
| A CLI ID argument (`grund FS-042`, `grund refs FS-042`) | **Resolves** | Input boundary; nothing is persisted. Also what makes a clicked `§FS-042` open ([§FS-integrations.3.1](../../functional-spec/FS-integrations.md#31-terminal-clients-wezterm-kitty-tmux-iterm2)). |
| Persisted in a scanned file (`§FS-042`) | **`grund check` error**, with the canonical form named | The graph must have one form. |
| `grund fmt --write` | **Rewrites** to the canonical form | The bulk fix-it for the error above. |

The error carries the answer, so the fix is mechanical:

```
docs/notes.md:5: shorthand citation §FS-042; write §FS-042-user-login
```

### 2.3 It is an error, not a warning or a suggestion

An error, because the alternative is to keep a false negative. A warning does not change the exit code, so a repository could accumulate shorthand citations forever while CI stayed green — which is the state this record exists to end. The suggestions channel ([§FS-check.2.3](../../functional-spec/FS-check.md#23-suggestions-channel-opt-in)) is for advisory `should`-level citation directions a project may consciously deviate from; a citation form the project cannot grep is not a matter of taste.

This does mean a repository that already contains shorthand citations gains errors on upgrade. That is not the silent semantic change [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) guards against, and it needs no deprecation window: the shorthand was never a working feature to deprecate — it resolved nowhere and appeared in no report — so nothing that used to pass and mean something now means something else. What changes is that a token which used to mean *nothing* now means *fix me*, and it says so with the exact replacement text and a `grund fmt --write` that applies it.

### 2.4 The marker is required; a bare shorthand is text

`§FS-042` is a citation shape; a bare `FS-042` is not — not even under `[reference] strict = false` ([§FS-config.3.1](../../functional-spec/FS-config.md#31-reference--citation-form)), where a bare *full* ID does count.

The asymmetry is deliberate and is the reason the shorthand can exist at all. A full ID carries a slug, which makes an accidental match vanishingly rare; `KIND-NNN` is a shape that occurs constantly in the wild as issue keys, part numbers, standards references, and version strings. Recognizing it unmarked would turn compatibility mode into a false-positive machine — precisely the failure [§DF-reference-marker.2.1](DF-reference-marker.md#21-marker) introduced the marker to prevent. The marker is what supplies the missing intent, so the shorthand requires it.

### 2.5 The shorthand exists only where the format has something to drop

The shorthand shape is the configured `[id] format` with the `{slug}` placeholder and one adjacent literal separator removed — under the default `{kind}-{number}-{slug}` that is `{kind}-{number}`. It therefore exists **only when the format carries both `{number}` and `{slug}`**:

- `{kind}-{slug}` (the format `grund` itself uses) has no number to stand in for the ID. No shorthand; nothing in this record applies.
- `{kind}-{number}` has no slug to omit. The "shorthand" would be the full ID.

A repository on the number-less form is entirely unaffected by this decision, which is why `grund`'s own tree gains no new findings from it ([§FS-id.4.1](../../functional-spec/FS-id.md#41-number-less-id-formats)).

### 2.6 The full ID always wins

The full-ID pass claims a token first; the shorthand pass only considers what is left. A configured grammar exotic enough that some full ID is also shorthand-shaped therefore resolves as the full ID, and no repository can be pushed into ambiguity by adopting this rule.

### 2.7 Ambiguity is reported, never guessed

Nothing forbids two declarations sharing a kind and number — `grund check` catches duplicate *full* IDs ([§FS-check.3.3](../../functional-spec/FS-check.md#33-duplicate-declaration)), not duplicate numbers. When a shorthand matches more than one declaration, `grund` names every candidate and resolves nothing; when it matches none, it says so. Picking one would be a guess, and `check` reports facts about the tree ([§GOAL-agent-grounding.3](../../goals.md#3-what-this-rules-out)).

### 2.8 A resolved shorthand is a real edge

Once a shorthand resolves to exactly one declaration, it is a citation like any other for every graph question: `grund refs` lists it, `grund cover` groups it, the declaration is no longer "declared but never cited" ([§FS-check.4.1](../../functional-spec/FS-check.md#41-unused-declaration)), it grounds its file under `require_grounding` ([§FS-check.3.6](../../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), and it counts for citation directions ([§FS-config.3.9](../../functional-spec/FS-config.md#39-citations--citation-direction-rules)).

This is the half of the fix that is easy to skip and expensive to omit. The original report's most damaging symptom was not the missing dangling error — it was `declared but never cited` printed about a declaration cited twice. A rule that flagged the shorthand but still refused to count it would have left that lie in place.

## 3. Rejected alternative: a resolvable alias

Let `§FS-042` resolve everywhere and make the slug advisory.

It is cheaper to type, rename-proof, and needs no error class. It was rejected because it splits every ID into two greppable forms — `grund refs` would have to report both, every downstream `grep` becomes wrong, and the two forms drift in different files — and because it makes a citation opaque at the point of reading, which is the property the sectioned, slugged citation exists to provide ([§RM-positioning-trace-tools](../../roadmap.md#rm-positioning-trace-tools-position-grund-against-requirements-traceability-tools-in-readme) names it as the axis `grund` is different on).

The decisive point is that the two options are not symmetric in cost of error. Under §2, a project that wants the alias behaviour runs `grund fmt --write` and loses nothing but characters. Under the alias, a project that wants one canonical form has no way back — the tool has already blessed both.

## 4. Consequences

- A new `shorthand-citation` error class ([§FS-check.3.13](../../functional-spec/FS-check.md#313-number-only-shorthand-citation)) and a new recognized-citation clause ([§FS-check.1.2](../../functional-spec/FS-check.md#12-the-number-only-shorthand)). The dangling check (§3.1) is suppressed at a shorthand site, since the shorthand finding already reports it and `unknown reference FS-042` would name a token that is not a full ID.
- `Citation` gains a `shorthand` flag and the scanner gains a resolution post-pass; every existing consumer keeps reading a canonical `Id`.
- `grund fmt` gains a third rewrite label, `shorthand → canonical` ([§FS-fmt.2.4](../../functional-spec/FS-fmt.md#24-shorthand-to-canonical)), and the LSP's live transform expands the trigger form in one step ([§FS-lsp.1.4](../../functional-spec/FS-lsp.md#14-live-trigger-transform)).
- No `grund_config_version` bump and no `[id]` key: the shorthand is derived from `format`, not configured beside it. A second knob would let two installs disagree about what a citation *is* ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)).
- No managed-block version bump ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)). The block tells an agent to write canonical citations, which is still exactly right; the shorthand rule only fires when one is written anyway, and the finding names its own fix. Bumping the block would hand every repository on the previous version an `agents-init` error for a rule that changes nothing about what they should write.
- The terminal and editor clients need no new matcher. Their shared citation shape ([§FS-integrations.3.1](../../functional-spec/FS-integrations.md#31-terminal-clients-wezterm-kitty-tmux-iterm2)) already matches `§FS-042`, so a clicked shorthand resolves the moment `grund <ID>` accepts one — which is why §2.2 puts the shorthand at the CLI input boundary and not only in the editor.
