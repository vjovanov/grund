# FS-integrations: grund prints and installs its rendering-layer integrations

Locally, a plain `§<ID>` citation is meant to be clickable and hoverable — resolution belongs to the rendering layer ([§DF-neural-link-generation](../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command)). That layer is user-side, one-time configuration in the spirit of the editor LSP snippets ([§FS-lsp.2.3](FS-lsp.md#23-editor-configuration-one-time-per-editor)): grund ships no first-party marketplace plugin. `grund integrations` is the one-stop shop that makes those integrations installable by humans and agents alike — the binary carries every artifact and prints it on demand, in the same dry-run-first ethos as `grund completions` ([§FS-completions](FS-completions.md#fs-completions-grund-completes-declared-ids-in-shells)) and `grund init` ([§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo)). It exists so that `cargo install grund` is enough to get clickable citations in a terminal or editor, serving [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) without asking the user to hand-assemble scripts. Why a one-time-setup command earns a slot on the frozen subcommand surface when a per-citation `link` command did not is recorded in [§DF-integrations-command](../decisions/functional/DF-integrations-command.md#df-integrations-command-integrations-earns-a-cli-slot-as-one-time-setup-where-a-per-citation-link-command-did-not).

## 1. User-facing command

```
grund integrations [<client>] [--write] [--conversation plain|link] [--format text|json]
```

- With no `<client>` and no write flags, grund detects the caller's environment (§2) and prints the integrations that apply, each with its one-line install command. `--format json` prints the same detection as a machine-shaped plan (§5).
- With a `<client>` — one of `kitty`, `tmux`, `vscode`, `wezterm` — grund prints that client's integration: the config snippet plus, for the terminal clients, the embedded `grund-open` resolver script (§3). Deterministic and environment-independent (§6).
- With an explicit client, `--write` applies the integration to the client's real configuration instead of printing it, as a managed, marked block (§4), and synchronizes the user's local-conversation preference into global agent instructions (§4.3).
- `--conversation plain|link` overrides the user preference while writing. It is accepted only with `--write`. When present without a client, the command updates only the user preference and global agent instructions; this preference-only form is unambiguous and does not install an arbitrary client. With no override, a client installation records `plain` on first use and preserves the stored preference thereafter. `plain` tells agents to write bare citations for the installed rendering layer, while `link` tells them to follow a citation with its declaration location as plain `path:line` text — never a Markdown link ([§DF-repo-conversation-opinion.2.1](../decisions/functional/DF-repo-conversation-opinion.md#21-the-link-form-is-plain-pathline-text)) — and fall back to the bare citation when uncertain. Bare `grund integrations --write`, with neither a client nor a conversation value, remains an error.

An unknown client is a CLI-level error: `error: unknown integration client \`<client>\`` plus `known clients: kitty, tmux, vscode, wezterm` on stderr, empty stdout, exit `2`. The client set is closed and frozen the same way the subcommand surface is ([§FS-cli.4](FS-cli.md#4-errors-with-no-source-location)), so adding a client is a deliberate, changelog-gated change.

## 2. Detection

With no `<client>`, grund inspects the environment to decide which integrations apply. Detection reads only these variables, in this fixed order, and never writes or scans:

- `WEZTERM_EXECUTABLE` → `wezterm`
- `KITTY_WINDOW_ID` → `kitty`
- `TERM_PROGRAM` — value `WezTerm` → `wezterm`, `tmux` → `tmux`, `vscode` → `vscode`
- `VSCODE_PID` or any `VSCODE_*` marker → `vscode`
- `TMUX` (non-empty) → `tmux`

Zero, one, or several clients may match at once (a VS Code integrated terminal running under tmux inside WezTerm matches three). Matches are reported in the frozen client order `kitty, tmux, vscode, wezterm`, deduplicated, never in environment-probe order, so a given environment always prints the same text. When nothing matches, grund prints the full catalog of clients with their one-line installs and a note that none was detected — a discoverable menu rather than an error. Detection succeeds with exit `0` in every case; an unrecognized terminal is not a failure.

Because detection depends on ambient environment variables, only the explicit-`<client>` form (§1) and `--help` are byte-stable across machines; the no-client form is stable only for a fixed environment. Golden coverage therefore pins explicit clients (§6).

## 3. Per-client artifacts

Every artifact is embedded in the binary, like the `init` templates and `completions` scripts; nothing is fetched and nothing is published to a marketplace, so the no-first-party-plugins stance ([§FS-lsp.2.3](FS-lsp.md#23-editor-configuration-one-time-per-editor)) holds.

### 3.1 Terminal clients: `wezterm`, `kitty`, `tmux`

Each prints two things on stdout, exit `0`:

1. A config snippet for that terminal that registers a click/hover handler for `§<ID>` citations, wired to run the resolver.
2. The embedded `grund-open` resolver script — a POSIX shell script that takes a citation or bare `<ID>`, resolves it with `grund` to a `path:line` declaration site, and opens it. The command it opens with is `GRUND_OPEN_CMD` when set, else `EDITOR`, else a platform default; the user's editor choice lives in that environment variable, never in shared repository text ([§DF-neural-link-generation](../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command)).

The snippet and the resolver are printed together so a human can read both before installing. `GRUND_OPEN_CMD` is an argv-style command prefix, not shell source: grund-open appends one `path:line` argument without evaluating repository-controlled paths as commands; users who need more elaborate argument placement use a wrapper executable. Any text the handler passes in — a tmux copy buffer, a kitty hint — reaches grund-open as a single quoted argument, never as shell source. The one-line install shown by detection (§2) is the `--write` form (§4).

The click matchers and the resolver key on a bare local `§<ID>` citation; a namespace-qualified `§<alias>/<ID>` is out of scope and is not made clickable. Resolution still succeeds inside a workspace: `grund list --format json` qualifies its `id` field as `<alias>/<ID>` in workspace mode ([§FS-workspace.8.3](FS-workspace.md#83-grund-list)), so the resolver matches the clicked local `<ID>` against either the bare or the `/`-suffixed qualified form.

### 3.2 Editor client: `vscode`

`grund integrations vscode` prints the terminal-citations extension source and the one-line `--write` install. `grund integrations vscode --write` materializes the unpacked extension into the editor's extensions directory (§4.2); the binary carries the artifact, so nothing is published to a marketplace and the [§FS-lsp.2.3](FS-lsp.md#23-editor-configuration-one-time-per-editor) stance is preserved. The extension is a `TerminalLinkProvider` that turns `§<ID>` occurrences in the integrated terminal into links resolved through `grund`.

## 4. Managed writes (`--write`)

`--write` never prints an artifact; with a client it installs that integration, while the clientless `--conversation` form updates only user guidance (§4.3). It reports what it did on stderr, exit `0`. Writes are idempotent and reversible by construction, mirroring the `init` managed-block contract ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)): a re-run of an up-to-date integration changes nothing and reports `exists`; an upgrade is a diff of the marked region; removal is deleting the marked region.

### 4.1 Marked blocks in dotfiles

For `kitty`, `tmux`, and `wezterm`, `--write` splices a comment-delimited managed block into the client's configuration file:

```
# >>> grund integrations (v1) >>>
<snippet>
# <<< grund integrations (v1) <<<
```

The begin and end markers carry a block version. On write grund finds one existing block at any supported version and replaces its complete marker-line span, preserving everything outside byte-for-byte; a file with no block gets the block appended after a blank-line separator. Indented marker lines are accepted and consumed as complete lines. A missing matching marker, multiple blocks, or a block whose version is newer than the binary understands is a hard error (`exit 2`), the same guard `init` applies to a newer AGENTS block ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)). The resolver script is written alongside, to a fixed resolver path, only when absent or when its embedded version differs, and a failure to make it executable is an I/O error rather than a successful install.

Outcomes map to stderr verbs `appended` / `updated` / `exists`, and to `would-append` / `would-update` / `would-write` under a `--dry-run` reserved for symmetry with `init`. The default (no `--write`) is already the dry read: it prints, it does not touch the disk.

### 4.2 The VS Code extension

`vscode --write` writes the unpacked extension directory (its `package.json`, the provider script, and a marker file recording the embedded version) into the editor extensions directory. A present, byte-current same-version extension is left untouched (`exists`); a different, missing, or damaged grund-owned file is repaired. grund writes only files it owns inside its own extension directory; it never edits the user's `settings.json`.

### 4.3 User preference and global agent instructions

Every client `--write`, and every clientless `--write --conversation <value>`, records the effective local-conversation preference in the user's Grund configuration at `$XDG_CONFIG_HOME/grund/config.toml`, or `~/.config/grund/config.toml` when `XDG_CONFIG_HOME` is unset. This is deliberately user-scoped: whether a TUI has the rendering integration is a property of the user's machine, not a repository. The first client write defaults to `plain`; a stored value survives later writes unless `--conversation` explicitly changes it. Only `plain` and `link` are accepted. The repository side may commit the `link`-only opinion `[reference] conversation` ([§FS-config.3.1](FS-config.md#31-reference--citation-form), [§DF-repo-conversation-opinion](../decisions/functional/DF-repo-conversation-opinion.md#df-repo-conversation-opinion-repositories-may-commit-a-link-only-conversation-rendering-opinion)); that opinion takes precedence inside its repository, while the user preference here governs everywhere else.

The same write synchronizes a marked `## Grund citation rendering` block into the global instruction files for every supported agent with a file-backed user scope: Codex (`~/.codex/AGENTS.md`), Claude (`~/.claude/CLAUDE.md`), Gemini (`~/.gemini/GEMINI.md`), GitHub Copilot (`~/.copilot/copilot-instructions.md`), Zed (`~/.config/zed/AGENTS.md`), and Pi (`~/.pi/agent/AGENTS.md`). This is the global-file subset of the fixed supported-agent superset in [§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints); Cursor and Windsurf expose user rules through their own settings rather than a portable file target, so their repository entrypoints still receive the same managed guidance through `grund init`. The block texts are self-scoping: they apply only inside grund repositories, so their entire footprint elsewhere is one inert sentence of session context. The `plain` block says `In repositories with a .agents/grund.toml: write plain §<ID> citations in local conversations; grund integrations makes them clickable. A repository whose agent instructions ask for linked citations takes precedence. Elsewhere, ignore this.` The `link` block says `In repositories with a .agents/grund.toml: follow §<ID> with its declaration location as plain path:line text in local conversations — never a Markdown link; fall back to the bare citation when unsure. Elsewhere, ignore this.` The precedence sentence appears only in the `plain` block because repository `link` against user `plain` is the only possible conflict ([§DF-repo-conversation-opinion.2.3](../decisions/functional/DF-repo-conversation-opinion.md#23-precedence)). Adopting these self-scoping texts is an agent-guidance block version bump (v1 → v2) under the marked-block contract below. These blocks are user-global and intentionally contain no repository-web rule; repository entrypoints own that fixed context-sensitive instruction ([§FS-init.2.3.6](FS-init.md#236-clickable-citations)).

The global instruction block uses versioned HTML-comment begin/end markers. The replacement, append, newer-version refusal, malformed/multiple-block rejection, preservation, idempotence, and stderr outcome rules are the same as the dotfile block in §4.1. Existing global instruction files keep all text outside the managed block byte-for-byte; missing parent directories and files are created because `--write` is the explicit installation action. A failure to update the preference or any instruction file is an error; an earlier integration artifact may already have been written, and re-running after correction safely completes the installation.

## 5. JSON format

`grund integrations --format json` (no client) emits the detection plan as one JSON object on stdout: the detected clients in frozen order, and for each the client name, whether it is detected, whether its grund-owned artifacts are installed and current, and the exact `--write` command an agent would run. Installation checks read only the four fixed integration targets; they never search the filesystem. `grund integrations <client> --format json` emits one deterministic object describing that client's artifact — its target path(s) and the `--write` command — without printing the artifact bytes. The stream split follows the rest of the surface ([§FS-errors.1](FS-errors.md#1-streams)): the plan is stdout, any CLI-level `error:` is stderr. This is the machine surface `grund agent-setup-instructions` points an agent at ([§FS-init.5](FS-init.md#5-agent-setup-instructions)): detect, show the diff, write only on user confirmation.

## 6. Determinism and exit codes

With an explicit `<client>`, output is a pure function of the binary version and the flags — no environment, no tree scan, no clock — so it is byte-identical across machines and safe to golden. The no-client detection form (§2) is a pure function of the binary version, the named environment variables, and, for JSON, the four fixed installation targets; it is byte-identical for a fixed environment and installation state. `--write` is idempotent: a second run against an up-to-date target is a no-op that reports `exists`.

Exit codes follow the frozen mapping ([§FS-cli.5](FS-cli.md#5-exit-code-mapping-is-fixed)): `0` printed, written, or already current; `2` an unknown client, bare `--write` with neither a client nor a conversation value, `--conversation` without `--write`, an invalid preference, an unknown flag, or a configuration file whose managed block is newer than this binary understands. There is no `1` outcome — `integrations` has no findings surface.
