# Architectural spec

Internals — *how* this project is built. One file per spec; each H1 declares an `AR-NNN-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§AR-NNN-<slug>.<section>`) resolve into these files.

An architectural spec may live inline in the class- or module-level doc-comment of the file it describes. A one-line stub here whose H1 is `# AR-NNN-<slug>: [<path>](<path>)` is **optional** — add it when you want the inline spec to appear in the index below; omit it when the doc-comment alone is enough. `grund <ID>` resolves the ID either way.

By convention every file under this directory — each file-form spec and each stub you chose to write — is linked from this README, and `grund check` verifies it: each ID appears here once, as a full Markdown link that `grund fmt --write` writes and keeps current. Inline specs without a stub are not listed here and that is fine. Extra prose, recommended reading order, and conceptual groupings are welcome around the link set.

| ID | Subject |
|---|---|

This index is navigational — citations should target the spec ID directly, never this file.
