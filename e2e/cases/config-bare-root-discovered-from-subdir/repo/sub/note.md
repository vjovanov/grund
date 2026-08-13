# Note

Mentions FS-001-foo without a marker. The bare `grund.toml` one directory up
sets `strict = true`, so this token is not a citation and `check` must stay
silent — proving discovery probes the root-visible name on the way up, not
only `.agents/`.
