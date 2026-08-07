# DF-managed-block-delimiters: standard BEGIN/END delimiters for the managed agent-instructions block

**Status:** Accepted
**Date:** 2026-08-07

## 1. Context

`grund init` writes managed content into shared agent-instruction files (`AGENTS.md`, `CLAUDE.md`, …). Through block schema v3 the managed region had no explicit delimiters: the `## Grounding with grund (vN)` heading was the implicit begin marker, and the region ran to the next H1/H2 heading or end of file ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)).

That implicit boundary caused real damage. A generated grund section looks like ordinary editable prose, so humans and agents edit inside it and lose the edit on the next `grund init`. Worse, the *end* of the region depends on document structure the user controls: any non-heading content placed after the block — an HTML comment, a tool's own managed marker, a horizontal rule — silently becomes part of grund's region and is eaten on the next update. This is not hypothetical: the implicit region swallowed a downstream repo's `<!-- rhei:begin -->` marker because that marker is not an H1/H2 heading.

`<!-- BEGIN … -->` / `<!-- END … -->` HTML-comment pairs are the established convention for managed regions in Markdown (GitHub Actions-managed README sections, terraform-docs, doctoc, …). A reader who has never heard of grund still recognizes the ownership boundary.

## 2. Decision

The managed block is bounded by explicit, unversioned delimiter lines, with the version-carrying H2 heading just inside:

```markdown
<!-- BEGIN GRUND MANAGED BLOCK -->
## Grounding with grund (vN)
…
<!-- END GRUND MANAGED BLOCK -->
```

- The delimiters are part of the managed region: `init` owns every byte from the `BEGIN` line through the `END` line, and nothing outside them.
- The schema version stays in the H2 heading, not in the delimiters — one version surface, unchanged tooling for reading it.
- The block schema version is bumped (v3 → v4) to carry the structural change under [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path): a pre-delimiter `grund` binary that met a delimited block would end the region at the next heading, splice its own block over the `END` line, and leave a dangling `BEGIN` — the version bump makes it refuse ("unsupported newer block") instead of corrupting the file.
- Legacy H2-bounded blocks (v3 and earlier) remain recognized by both `init` and `check`. `grund check` reports them as an outdated block version whose fix is `run \`grund init\`` ([§FS-check.3.5](../../functional-spec/FS-check.md#35-invalid-agent-entrypoint-init-block)), and that `init` run migrates the block to the delimited form in place — same position, every byte outside the block preserved.
- Broken delimiters (a `BEGIN` without an `END`, an `END` without a `BEGIN`, duplicate `BEGIN`s, a delimited region without a version heading) are diagnosed by `check` and refused by `init` without rewriting the file — splicing against broken delimiters risks eating user content ([§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)).

## 3. Why this fits grund's goals

- [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — the ownership boundary is visible in the file itself, in a convention users already know, instead of being an implicit rule only grund's spec states.
- [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — the structural change ships as a block-version bump with an explicit migration step, and old binaries fail loudly rather than corrupting new files.

## 4. Consequences

- Fresh `grund init` output uses the delimiters; re-runs stay idempotent (`exists `).
- Existing repos see one `outdated grund init block v3 (run \`grund init\` to update to v4)` finding per entrypoint, and one `updated ` write migrates each block in place.
- The end of the managed region no longer depends on what the user puts after the block, so adjacent third-party managed markers are safe.

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Keep the implicit H2-bounded region and document it harder | The damage mode (region end depends on user content) is structural; documentation does not remove it. |
| Version the delimiters themselves (`<!-- BEGIN GRUND MANAGED BLOCK v4 -->`) | Two version surfaces to keep in sync; the H2 already carries the version and every existing reader keys off it. |
| Grund-specific arrow markers (`<!-- >>> grund >>> -->`) | Non-standard shape defeats the point — the value of the convention is that readers recognize it without knowing the tool. |
| Silently repair broken delimiter pairs on the next write | Any guess about where the region "really" ends risks deleting user content; refusing with a named defect is recoverable, a wrong splice is not. |
