# grund-core

`grund-core` is the shared Rust engine behind [`grund`](https://crates.io/crates/grund).
It contains the scanner, resolver, checker, formatter, config loader, and
structured APIs for the CLI surfaces (`check`, `show`, `refs`, `list`, `cover`,
`fmt`, `fetch`, `id`, `init`, dynamic completions, and config inspection). The published
CLI owns argument parsing, text/JSON rendering, and exit-code mapping;
`grund-core` returns data for embedders and frontends. This package role follows
[§FS-distribution.1](../../docs/functional-spec/FS-distribution.md#1-targets).

Most users should install the CLI crate instead:

```bash
cargo install grund
```

Use `grund-core` directly when embedding the engine in Rust code. The public API
is still pre-1.0 and may change between minor releases.

`KindConfig` now exposes `format`, `resolve`, and `fetch`. This is additive for
field access but source-incompatible for downstream exhaustive struct literals;
embedders constructing the public struct must initialize the three new fields.
The executable integration still runs only through an explicit
`fetch_snapshot` call ([§FS-fetch](../../docs/functional-spec/FS-fetch.md#fs-fetch-grund-materializes-one-external-fact-snapshot)).

Project home: <https://github.com/vjovanov/grund>
