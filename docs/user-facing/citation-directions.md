# Citation directions

`[citations]` describes which kinds of documents cite which other kinds. The
five levels use two rule classes and two enforcement surfaces, as specified by
`§FS-config.3.9.1`:

| Level | Rule class | Checked per | Surface |
| --- | --- | --- | --- |
| `must` | obligation | declaration | `grund check` error |
| `should` | obligation | declaration | `--suggestions` and the generated entrypoint |
| `may` | permission | — | never checked |
| `should-not` | prohibition | citation site | `--suggestions` and the generated entrypoint |
| `must-not` | prohibition | citation site | `grund check` error |

The grammar is: entries in one array are all required, while `|` inside one
entry means any one of its alternatives. Therefore `must = ["FS|GOAL"]`
requires a citation to either `FS` or `GOAL`, whereas `must = ["FS", "GOAL"]`
requires citations to both. Nearly every real rule is one entry with `|`.

The citing side may be any configured kind, including a non-citable kind, or
the homeless `code` kind. The cited side must be citable, because a citation
needs an ID to point at (`§FS-config.3.9.5`). A non-citable kind may cite and is
labelled by its home, such as `skills/`; the homeless kind covers any file
outside a configured kind home and renders last (`§FS-config.3.9.2`).

An obligation on a citable kind that declares no IDs has no unit and never
fires. A directory without declarations is `citable = false`, not a citable
kind with an empty rule; this is the no-unit trap described by
`§DF-non-citable-kinds.2.5`.

The config and the generated section below are the same example. The TOML is
shown beside the Markdown it renders:

```toml
[citations]
default = "may"

[citations.FS]
should = ["GOAL|FS"]        # either
must-not = ["AR"]

[citations.DA]
must = ["AR", "FS"]         # both

[citations.skill]           # citable = false: cites, is never cited
must = ["FS"]
must-not = ["AR"]

[citations.code]
should = ["FS|AR"]
```

```markdown
### Citation directions

- **FS** should cite GOAL or FS; never cite AR.
- **DA** must cite AR and FS.
- **skills/** must cite FS; never cite AR.
- **code** (any file outside a kind home) should cite FS or AR.
Unlisted kinds and pairs are fine.
```
