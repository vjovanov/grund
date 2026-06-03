# api — agent instructions

## Grounding with grund (v1)

This project uses [`grund`](https://github.com/vjovanov/grund): every spec, goal, decision, and end-to-end test has a stable ID `<KIND>-<NNN>-<slug>[.<section>]` (`KIND ∈ {GRUND, GOAL, FS, AR, DF, DA, E2E, RM}`), cited with the marker `§` — e.g. `§FS-042-user-login.3.1` (the `FS-042-user-login` here is a shape illustration, not a real ID in this repo). Type `$$` in a grund-aware editor and it becomes `§`. Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `.agents/grund.toml`, so only `§`-prefixed citations are checked.

### Project map

- [GRUND](docs/grund.md): Why: project motivation
