# FS-integrations: grund prints and installs its rendering-layer integrations

Locally, a plain `§<ID>` citation is meant to be clickable and hoverable — resolution belongs to the rendering layer ([§DF-neural-link-generation](../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command)). That layer is user-side, one-time configuration in the spirit of the editor LSP snippets ([§FS-lsp.2.3](FS-lsp.md#23-editor-configuration-one-time-per-editor)): grund ships no first-party marketplace plugin. `grund integrations` is the one-stop shop that makes those integrations installable by humans and agents alike — the binary carries every artifact and prints it on demand, in the same dry-run-first ethos as `grund completions` ([§FS-completions](FS-completions.md#fs-completions-grund-completes-declared-ids-in-shells)) and `grund init` ([§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo)). It exists so that `cargo install grund` is enough to get clickable citations in a terminal or editor, serving [§GOAL-agent-grounding](../goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) without asking the user to hand-assemble scripts. Why a one-time-setup command earns a slot on the frozen subcommand surface when a per-citation `link` command did not is recorded in [§DF-integrations-command](../decisions/functional/DF-integrations-command.md#df-integrations-command-integrations-earns-a-cli-slot-as-one-time-setup-where-a-per-citation-link-command-did-not).

## 1. User-facing command

```
grund integrations [<client>] [--write] [--conversation plain|link] [--format text|json]
```

- With no `<client>` and no write flags, grund detects the caller's environment (§2) and prints the integrations that apply, each with its one-line install command. `--format json` prints the same detection as a machine-shaped plan (§5).
- With a `<client>` — one of `codium`, `iterm2`, `kitty`, `tmux`, `vscode`, `wezterm` — grund prints that client's integration: the config snippet plus, for the terminal clients, the embedded `grund-open` resolver script (§3). Deterministic and environment-independent (§6).
- With an explicit client, `--write` applies the integration to the client's real configuration instead of printing it, as a managed, marked block (§4), and synchronizes the user's local-conversation preference into global agent instructions (§4.3). A client with no writable configuration installs what it can and prints the rest for the user to apply (§3.4).
- `--conversation plain|link` overrides the user preference while writing. It is accepted only with `--write`. When present without a client, the command updates only the user preference and global agent instructions; this preference-only form is unambiguous and does not install an arbitrary client. With no override, a client installation records `plain` on first use and preserves the stored preference thereafter. `plain` tells agents to write bare citations for the installed rendering layer, while `link` tells them to follow a citation with its declaration location as plain `path:line` text — never a Markdown link ([§DF-repo-conversation-opinion.2.1](../decisions/functional/DF-repo-conversation-opinion.md#21-the-link-form-is-plain-pathline-text)) — and fall back to the bare citation when uncertain. Bare `grund integrations --write`, with neither a client nor a conversation value, remains an error.

An unknown client is a CLI-level error: `error: unknown integration client \`<client>\`` plus `known clients: codium, iterm2, kitty, tmux, vscode, wezterm` on stderr, empty stdout, exit `2`. The client set is closed and frozen the same way the subcommand surface is ([§FS-cli.4](FS-cli.md#4-errors-with-no-source-location)), so adding a client is a deliberate, changelog-gated change.

## 2. Detection

With no `<client>`, grund inspects the environment to decide which integrations apply. Detection reads only these variables, in this fixed order, and never writes or scans:

- `WEZTERM_EXECUTABLE` → `wezterm`
- `KITTY_WINDOW_ID` → `kitty`
- `TERM_PROGRAM` — value `WezTerm` → `wezterm`, `iTerm.app` → `iterm2`, `tmux` → `tmux`, `vscode` → `vscode`
- `VSCODE_PID` or any `VSCODE_*` marker → `vscode`; additionally → `codium` when any of those values names VSCodium's application directory
- `TMUX` (non-empty) → `tmux`

Zero, one, or several clients may match at once (a VS Code integrated terminal running under tmux inside WezTerm matches three). Matches are reported in the frozen client order `codium, iterm2, kitty, tmux, vscode, wezterm`, deduplicated, never in environment-probe order, so a given environment always prints the same text. When nothing matches, grund prints the full catalog of clients with their one-line installs and a note that none was detected — a discoverable menu rather than an error. Detection succeeds with exit `0` in every case; an unrecognized terminal is not a failure.

Because detection depends on ambient environment variables, only the explicit-`<client>` form (§1) and `--help` are byte-stable across machines; the no-client form is stable only for a fixed environment. Golden coverage therefore pins explicit clients (§6).

## 3. Per-client artifacts

Every artifact is embedded in the binary, like the `init` templates and `completions` scripts; nothing is fetched and nothing is published to a marketplace, so the no-first-party-plugins stance ([§FS-lsp.2.3](FS-lsp.md#23-editor-configuration-one-time-per-editor)) holds.

### 3.1 Terminal clients: `wezterm`, `kitty`, `tmux`, `iterm2`

Each prints two things on stdout, exit `0`:

1. A config snippet for that terminal that registers a click/hover handler for `§<ID>` citations, wired to run the resolver.
2. The embedded `grund-open` resolver script — a POSIX shell script that takes a citation or bare `<ID>`, resolves it with `grund` to a `path:line` site, and opens it. The command it opens with is `GRUND_OPEN_CMD` when set, else `EDITOR`, else a platform default; the user's editor choice lives in that environment variable, never in shared repository text ([§DF-neural-link-generation](../decisions/functional/DF-neural-link-generation.md#df-neural-link-generation-agents-compose-clickable-citation-links-themselves-grund-does-not-grow-a-link-command)).

The snippet and the resolver are printed together so a human can read both before installing. `GRUND_OPEN_CMD` is an argv-style command prefix, not shell source: grund-open appends one `path:line` argument without evaluating repository-controlled paths as commands; users who need more elaborate argument placement use a wrapper executable. Any text the handler passes in — a tmux copy buffer, a kitty hint — reaches grund-open as a single quoted argument, never as shell source. The one-line install shown by detection (§2) is the `--write` form (§4).

Resolution proceeds in three steps, and each exists because the click carries less context than a command line does:

- **Find the root.** The handler passes a token, never a directory, so the click may arrive with the shell anywhere in the tree. The resolver walks up from `$PWD` to the nearest ancestor holding `.agents/grund.toml` and runs `grund` there. With no such ancestor it reports `grund-open: no .agents/grund.toml at or above <dir>` and exits `1` — naming the real problem rather than blaming the ID.
- **Strip the marker, keep the section.** `[reference] marker` is per-repo (§FS-config.3.1) while the installed artifacts are user-global and written once, so neither the click matchers nor the resolver may hardcode `§`. The matchers require a 1–3 character run of non-word, non-space characters before the ID shape, and the resolver drops any leading run of non-alphanumeric characters. This admits every configured marker, tolerates punctuation the matcher swept in (`(<§>FS-x`), and leaves a namespace-qualified `<alias>/<ID>` intact because an alias begins alphanumeric. The `.<section>` suffix is **preserved**, not truncated.
- **Resolve and join.** `grund <ID>[.<section>] --format json` reports the cited section's own line — truncating the suffix would send every subsection click to the declaration heading — and a `path` relative to the config root ([§FS-config.3.6](FS-config.md#36-output)). The resolver joins that path onto the root it found and hands the editor an absolute path. `grund` itself still never emits one ([§FS-errors.4](FS-errors.md#4-determinism)); composing it at the point of use is what makes a click work from a subdirectory. Field extraction is anchored to the end of the JSON object, because `body` carries arbitrary declaration prose that may itself contain `"path":` or `"line":`.

Both the click matchers and the resolver therefore accept a bare local `§<ID>` and a namespace-qualified `§<alias>/<ID>`; in a workspace the qualified form resolves through `grund`'s own alias routing ([§FS-workspace.8.1](FS-workspace.md#81-grund-show)) rather than through string matching over the ID catalog.

### 3.2 Editor clients: `vscode`, `codium`

`grund integrations vscode` prints the terminal-citations extension source and the one-line `--write` install. `grund integrations vscode --write` materializes the unpacked extension into the editor's extensions directory (§4.2); the binary carries the artifact, so nothing is published to a marketplace and the [§FS-lsp.2.3](FS-lsp.md#23-editor-configuration-one-time-per-editor) stance is preserved. The extension is a `TerminalLinkProvider` that turns `§<ID>` occurrences in the integrated terminal into links resolved through `grund`.

It matches and strips the marker exactly as the terminal clients do (§3.1) and likewise keeps the `.<section>` suffix, resolving through `grund <ID>[.<section>] --format json`. It needs no root walk: the workspace folder is already the directory it runs `grund` in and the base it joins the reported path against.

`codium` installs the identical extension into VSCodium's own extensions root, `~/.vscode-oss/extensions`. It is a separate client rather than a flag because it is a separate application: writing to `~/.vscode` for a VSCodium user drops the extension where nothing ever loads it, and the failure is silent — the install reports success and no link is ever produced. Detection cannot separate the two by presence, since both export an identical `VSCODE_*` environment; what differs is where those variables *point*, so `codium` is marked in addition to `vscode` when one of those values names VSCodium's application directory. Marking both is the honest outcome of an ambiguous environment: the catalog then offers both one-line installs and the user picks the application they are actually running.

### 3.3 Reading a declaration without leaving

Opening a citation answers *where*; often the question is only *what does it say*, and switching to an editor to find out costs more than the answer is worth. Every client therefore also offers a read-in-place path, in whichever form that client can express.

Only VS Code can offer true hover. Its `TerminalLinkProvider` attaches a tooltip to each link, so the declaration's `--brief` slice — heading plus first paragraph — plus the resolved `path:line` is shown on hover, with no click. Because tooltips must carry their text when the link is *provided*, and a provider runs per rendered line, resolutions are cached per citation; the same citation scrolling past repeatedly costs one resolution, not one per line.

The terminal clients cannot hover: no terminal exposes a hover event or a link tooltip to configuration ([wezterm#4](https://github.com/wez/wezterm/issues/4) has requested one since 2018). They offer **peek** instead — a second binding that renders the declaration into a disposable surface next to the work, rather than opening an editor:

| Client | Peek | Surface |
| --- | --- | --- |
| kitty | `ctrl+shift+p`, then the hint label | overlay window |
| tmux | `prefix` + `G` | popup (requires tmux 3.2+; inert on older tmux, where `prefix` + `g` still opens) |
| wezterm | `ctrl+shift+`-click | split pane |

Peek is the same resolver in a different mode — `grund-open --peek <citation>` (§3.1) — so a peek and a click can never disagree about where a citation points. It prints the resolved `path:line` followed by the declaration lead, through a pager, because each of those surfaces closes when the process exits.

WezTerm needs one indirection worth recording, because it looks like an accident otherwise: Lua cannot ask which link is under the mouse — only the built-in `OpenLinkAtMouseCursor` knows, and it exposes only the resulting URI through `open-uri`, while `window:current_event()` does not carry modifiers. The peek binding therefore records the intent, delegates to that built-in action, and the `open-uri` handler reads the intent back. Both run inside one synchronous event, so the flag cannot leak across clicks.

### 3.4 Manual client: `iterm2`

iTerm2 stores its configuration in a binary property list, not a text file. There is no comment syntax to carry markers and no safe place to put them, so the managed-block contract (§4.1) simply does not apply — and rewriting a live profile blob under the user is not a trade grund makes for a convenience.

`iterm2` is therefore the one **manual** client. `grund integrations iterm2` prints the Smart Selection rule to add: the citation regex from §3.1, precision `Very High` so it does not lose to iTerm2's built-in filesystem-path rule, and a `Run Command…` action of `grund-open \0` — the first action on a rule is what fires on cmd-click. A second action, `grund-open --peek \0`, is offered for the right-click menu, which is where actions after the first appear; that is peek (§3.3) in the form iTerm2 can express. `\0` is the whole match, and needs no capture group because the resolver strips the marker itself.

`grund integrations iterm2 --write` installs everything grund *can* install — the `grund-open` resolver and the user preference and global agent instructions (§4.3) — then prints the steps it cannot apply, reporting `manual <client> (<where>)` on stderr. The exit code is `0`: nothing failed, and the remaining work is the user's. `integration_is_current` (§5) always reports `installed: false` for a manual client, because grund never reads that plist and reporting a guess would be worse than reporting nothing; the detection plan carries `"install_kind":"manual"` so a caller can tell the difference between *not installed* and *not knowable*.

A Mac user may not need the rule at all. iTerm2's Semantic History already makes a plain `path:line` cmd-clickable, so `--conversation link` (§4.3) — which has agents write the declaration location beside each citation — produces clickable citations there with no integration installed. That is the same reason `link` exists for TUIs: it degrades to something the host already understands.

### 3.5 Clients grund cannot support, and why

The client set is closed (§1), but absence from it is not always a judgement about the client — sometimes there is simply no hook. Recording why keeps the question from being re-litigated each time someone asks:

- **Ghostty** — documents exactly the option needed, `link`, which matches a regex against terminal text and runs an arbitrary binding action. The reference marks it `TODO: This can't currently be set!`; only the built-in `link-url` matcher works, and it takes no regex of its own. Nor does the `--conversation link` fallback help, since Ghostty has no `path:line` opener either. When `link` ships, the artifact is the §3.1 regex plus `grund-open` and nothing else.
- **Apple Terminal.app** — no regex-to-action mechanism of any kind. Nothing to build against.
- **Windows Terminal** — same: URL detection only, with no configurable matcher.

The common shape is that a terminal must let configuration bind *arbitrary matched text* to *an arbitrary command*. Every supported terminal client offers that; these do not.

## 4. Managed writes (`--write`)

`--write` never prints an artifact; with a client it installs that integration, while the clientless `--conversation` form updates only user guidance (§4.3). It reports what it did on stderr, exit `0`. Writes are idempotent and reversible by construction, mirroring the `init` managed-block contract ([§FS-init.2.3](FS-init.md#23-generated-agent-entrypoints)): a re-run of an up-to-date integration changes nothing and reports `exists`; an upgrade is a diff of the marked region; removal is deleting the marked region.

### 4.1 Marked blocks in dotfiles

For `kitty`, `tmux`, and `wezterm`, `--write` splices a comment-delimited managed block into the client's configuration file:

```
# >>> grund integrations (v1) >>>
<snippet>
# <<< grund integrations (v1) <<<
```

The marker's comment token is the one the **host file's own language** uses, not a fixed `#`: `#` for `kitty.conf` and `.tmux.conf`, `--` for `wezterm.lua`. A config file is source code in someone else's language, and a marker in the wrong dialect is not an inert stray line — in Lua `#` is the length operator, so a `#` marker is a syntax error that costs the user their entire WezTerm configuration, not just the grund block. Block lookup is scoped to the client's dialect for the same reason: a block found under the wrong comment token would be spliced with the wrong markers on the next write.

When `--write` creates the config file from scratch, a client whose configuration is a *program* rather than a list of settings also receives a starter scaffold below the block, so a fresh install is usable without hand-editing. WezTerm is the case that needs it: it applies hyperlink rules only from the config object the file returns, so a file containing the block alone parses and registers nothing. The scaffold is unmanaged — it sits outside the markers, later writes rewrite only the block, and the user owns it from then on. An existing config never receives a scaffold; there, calling `grund_apply_hyperlink_rule(config)` on the config being returned is the user's one wiring step (§3.1).

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

With an explicit `<client>`, output is a pure function of the binary version and the flags — no environment, no tree scan, no clock — so it is byte-identical across machines and safe to golden. The no-client detection form (§2) is a pure function of the binary version, the named environment variables, and, for JSON, the four fixed installation targets; it is byte-identical for a fixed environment and installation state. `--write` is idempotent: a second run against an up-to-date target is a no-op that reports `exists`. A manual client (§3.4) has no target to compare, so its `--write` is idempotent in the weaker sense that it re-installs the resolver only when stale and re-prints the same steps.

Exit codes follow the frozen mapping ([§FS-cli.5](FS-cli.md#5-exit-code-mapping-is-fixed)): `0` printed, written, or already current; `2` an unknown client, bare `--write` with neither a client nor a conversation value, `--conversation` without `--write`, an invalid preference, an unknown flag, or a configuration file whose managed block is newer than this binary understands. There is no `1` outcome — `integrations` has no findings surface.
