# Point sizes

`grund list --size` measures the text an agent can fetch through each declaration
and section coordinate. It reports both the lead returned by a default read and
the full body returned by `--full`, using the same body slicing as `grund show`
([§FS-list.3.4](../functional-spec/FS-list.md#34---size--per-point-lead-and-full-body-measurements)).

```console
$ grund list --size=words --top 3
FS-large.2  docs/functional-spec/FS-large.md:44  words=721/944
FS-large    docs/functional-spec/FS-large.md:1   words=510/1730
AR-wide.1   docs/architecture/AR-wide.md:19      words=405/405
```

Bare `--size` selects `lines,words,bytes`. An attached comma list selects and
orders columns explicitly; `--top N` sorts on the first selected unit after the
ordinary project, kind, and unused filters. JSON output is NDJSON with stable
metadata fields followed by each selected `lead_<unit>` / `full_<unit>` pair.

The counts are byte-defined and deterministic:

- `lines` counts LF-delimited segments containing at least one byte other than
  ASCII whitespace;
- `words` counts non-empty byte runs separated by ASCII whitespace;
- `bytes` is the UTF-8 byte length.

Section headings are part of section bodies; declaration headings are not part
of declaration bodies. Comment markers are stripped from inline/doc-comment
declarations. Duplicate declarations and duplicate section claimants retain one
site-local row each. A broken stub remains visible with `-/-` in text or `null`
measurements in JSON rather than borrowing another site's body.

## Opt into a warning

Add one closed inline table to the project whose point leads you want checked
([§FS-config.3.1](../functional-spec/FS-config.md#31-reference--citation-form)):

```toml
[reference]
lead_size_warning = { max = 600, unit = "words" }
```

The key accepts a non-negative `max` and one of `lines`, `words`, or `bytes`.
It is absent by default. A lead strictly over the maximum produces the fixed
`oversized-lead` warning; equality passes and warnings do not change the exit
status ([§FS-check.4.13](../functional-spec/FS-check.md#413-oversized-lead-opt-in)).

The remedy is structural, not deletion: move detailed prose into citable child
sections, which keeps the parent coordinate stable, or promote a child section
to its own declaration after running `grund refs <ID> --summary`. Each workspace
member uses its own config. Explicit-path checks judge only sites in that path,
and `--full` does not widen this project policy beyond its configured scan scope.
