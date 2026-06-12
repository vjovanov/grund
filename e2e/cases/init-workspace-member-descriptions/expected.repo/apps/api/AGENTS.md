# api — agent instructions

## Grounding with grund (v1)

This project uses [`grund`](https://github.com/vjovanov/grund): every spec, goal, decision, and end-to-end test has a stable ID `<KIND>-<NNN>-<slug>[.<section>]` (`KIND ∈ {GRUND, GOAL, FS, AR, DF, DA, E2E, RM}`), cited with the marker `§` — e.g. `§FS-042-user-login.3.1` (the `FS-042-user-login` here is a shape illustration, not a real ID in this repo). Type `$$` in a grund-aware editor and it becomes `§`. Bare ID-shaped tokens are also recognized as citations for backward compatibility; set `[reference] strict = true` in `.agents/grund.toml` to require the `§` marker (run `grund fmt --marker` first to upgrade existing bare citations).

### Project map

- [GRUND](docs/grund.md): Why: project motivation
