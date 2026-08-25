# Note

Mentions FS-001-nowhere without a marker. Only the bare `grund.toml` is read,
and it sets `strict = true`, so the bare token is not a citation: a clean run
proves the tie went to the root-visible file. Under the ignored
`.agents/grund.toml`'s `strict = false` this line would be a dangling citation
instead.
