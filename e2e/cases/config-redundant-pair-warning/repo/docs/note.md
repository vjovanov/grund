# Note

Mentions FS-001-nowhere without a marker. Only the `.agents/` config is read,
and it sets `strict = true`, so the bare token is not a citation: a clean run
proves the tie went to `.agents/grund.toml`. Under the ignored file's
`strict = false` this line would be a dangling citation instead.
