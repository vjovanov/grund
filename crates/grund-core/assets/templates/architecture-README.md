# Architectural spec

Internals — *how* this project is built. One file per spec; each H1 declares an `AR-NNN-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§AR-NNN-<slug>.<section>`) resolve into these files.

An architectural spec may live inline in the class- or module-level doc-comment of the file it describes. Link its bare ID canonically from this index to enroll it here without a stub; `grund fmt --cross-refs --write` derives that link. A one-line stub whose H1 is `# AR-NNN-<slug>: [<path>](<path>)` remains valid when a separate Markdown pointer is useful. `grund <ID>` resolves the source declaration either way.

By convention every declaration under this directory is linked from this README, and `grund check` verifies it: each ID appears here once, as a full Markdown link that `grund fmt --write` writes and keeps current. A canonical bare-ID link to an inline source declaration outside this directory additionally enrolls that declaration here, with no stub file. Extra prose, recommended reading order, and conceptual groupings are welcome around the link set.

| ID | Subject |
|---|---|

This index is navigational — citations should target the spec ID directly, never this file.
