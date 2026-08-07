<!-- BEGIN GRUND MANAGED BLOCK -->
## Grounding with grund (v4)

This project uses [`grund`](https://github.com/vjovanov/grund): every spec, goal, decision, and end-to-end test has a stable ID `{ID_SHAPE_SEC}` (`KIND ∈ {KINDS_SET}`), cited with the marker `{MARKER}` — e.g. `{CITE_EXAMPLE}` (the `{ID_EXAMPLE}` here is a shape illustration, not a real ID in this repo, hence the `<{MARKER}>` escape). Type `{TRIGGER}` in a grund-aware editor and it becomes `{MARKER}`. {BARE_TOKEN_NOTE}

### Grounding from a citation

A `{MARKER}<ID>` is a pointer to a fact, not a file path. Resolve it with `grund` and climb only as far as needed:

- `grund <ID>` — the lead (heading-less, cut at the first child section). The cheap first read for a bare `{MARKER}<ID>` citation.
- `grund <ID> --toc` — the lead plus the nested section map. Use to choose which subsection to fetch next.
- `grund <ID> --full` — the entire body. Escalate to this when narrower reads aren't enough.
- `grund <ID> --brief` — heading + first paragraph only.
- `grund refs <ID>` — every site that cites the ID; add `--summary` for one line per file. Run before renaming or moving a declaration.
- `grund list` / `grund list --kind FS,AR` — discover IDs if you get lost

### Project map

{DECLARATION_MAP}

### Project namespaces

A namespace is a project boundary, not a docs folder. The current project is the local namespace: cite its IDs as `{MARKER}<ID>`.

Create or use a separate namespace when work introduces an independently checked app, package, service, or subproject. Give that project its own `.agents/grund.toml`, add it to the workspace root's `[workspace] members`, run `grund init` there, and set a stable `project_name`.

Do not create a namespace for a regular module or component that still belongs to this project. Cite across namespaces as `{MARKER}alias/<ID>` and run `grund check` from the workspace root.{WORKSPACE_MEMBERS}

### Declarations and citations

Declarations are heading lines `# {ID_EXAMPLE}: …` in markdown. In a code doc-comment (Rustdoc, Javadoc, JSDoc, Python docstring, Go `//`, …) drop the `#` — write `/// {ID_EXAMPLE}: …` directly. {SECTION_HEADING_NOTE} One doc-comment may declare multiple IDs (e.g. an `AR-` and an `FS-` on the same class) — each gets its own body. An inline source declaration is reachable from the configured kind home via a one-line stub: `# <ID>: [<path>](<path>)`.

### Rules

- **Spec first.** For behavior or design changes, write or update the most-specific spec point before code.
- **Cite as you write.** Place `{MARKER}<ID>` at the point a claim or behavior is made — on the doc-comment for a whole behavior, inline beside the clause it enforces.
- **Marker = live citation.** A `{MARKER}`-prefixed token resolves and is checked wherever it appears — including inside Markdown backticks. To mention an ID without citing it, write `<{MARKER}><ID>`, omit the marker, or use a fenced code block.
- **Inline citation style.** {INLINE_CITATION_STYLE}
- **Always cite the most-specific point.**

{CITATION_DIRECTIONS}
<!-- END GRUND MANAGED BLOCK -->
