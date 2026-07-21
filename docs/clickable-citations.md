# Clickable citations

Make a `§<ID>` citation in your terminal jump straight to the line it cites.

Agents write citations constantly — in answers, in commit messages, in code
comments. Without an integration a citation is a string you have to go look up.
With one, `§FS-integrations.3.1` is a thing you click, and your editor opens at
that section's own line.

This is a one-time setup per machine. It changes nothing in any repository.

---

## 1. Before you start

Three things have to be true, and each has a quiet failure mode if it isn't.

**`grund` is on your `PATH`.**

```bash
grund --version
```

**`~/.local/bin` is on your `PATH`.** The resolver script is installed there.

```bash
case ":$PATH:" in *":$HOME/.local/bin:"*) echo ok ;; *) echo "add it to your shell profile" ;; esac
```

**Your editor is set.** The resolver opens files with `GRUND_OPEN_CMD` if set,
otherwise `EDITOR`, otherwise `code`. If none of those resolve, clicks appear to
do nothing.

```bash
echo "${GRUND_OPEN_CMD:-${EDITOR:-<neither set>}}"
```

`EDITOR` may carry flags, and the line number is passed in the syntax your
editor understands — `+LINE FILE` for the vi and emacs families, `--goto
FILE:LINE` for VS Code and Sublime, `FILE:LINE` for helix and micro. Set
`GRUND_OPEN_CMD` instead when you want an argv prefix that ignores all of that;
for anything more elaborate, point it at a wrapper script.

## 2. See what applies to you

```bash
grund integrations
```

This reads your environment and prints the integrations that match, each with
its one-line install. Nothing is written and nothing is scanned. If you are in a
VS Code terminal inside tmux inside WezTerm, all three match — that is normal,
and you can install all three.

Preview any client before committing to it:

```bash
grund integrations kitty
```

That prints the exact config snippet and the full resolver script, so you can
read both before anything touches your dotfiles.

## 3. Install it

```bash
grund integrations kitty --write
```

The supported clients are `kitty`, `tmux`, `vscode`, and `wezterm`. The write is
idempotent and lands as a marked block, so re-running is safe and removing it
later is a matter of deleting the block.

Each client then needs one more step:

| Client | How you click | Afterwards |
| --- | --- | --- |
| **kitty** | `ctrl+shift+g`, then the hint label | reload config with `ctrl+shift+F5` |
| **wezterm** | ctrl/cmd-click the citation | **wire it up — see below** |
| **tmux** | select the citation in copy mode, then `prefix` + `g` | `tmux source-file ~/.tmux.conf` |
| **vscode** | click the link in the integrated terminal | *Developer: Reload Window* |

**WezTerm needs one manual edit — but only if you already have a config.** It
applies hyperlink rules only from the config object your Lua returns, and no
installer can safely rewrite the function that builds yours. Add the call where
you build it:

```lua
local config = wezterm.config_builder()
-- ... your settings ...
grund_apply_hyperlink_rule(config)   -- defined by the block above
return config
```

Until that line exists, WezTerm reads the installed block and does nothing with
it — the most confusing possible failure, because everything looks installed.

If you had no `wezterm.lua`, `--write` creates one that already calls it, and
you can skip this. That starter config sits below the managed block and is
yours: later writes rewrite only the block.

## 4. Check it works

Use grund's own repository — every ID below is real, so you can compare against
what you see.

```bash
cd /path/to/grund
grund-open '§FS-integrations.3.1'
```

Your editor should open `docs/functional-spec/FS-integrations.md` at **line 36**,
which is the `### 3.1` heading itself — not line 1.

That distinction is the whole point of the section suffix, so it is worth
checking all three shapes:

| Run this | Opens |
| --- | --- |
| `grund-open '§FS-integrations'` | `FS-integrations.md:1` — the declaration |
| `grund-open '§FS-integrations.3.1'` | `FS-integrations.md:36` — that section |
| `grund-open '§FS-integrations.4.3'` | `FS-integrations.md:85` — that section |

Those line numbers move whenever the spec is edited, so treat `grund` as the
authority rather than this table — it prints the line the click should land on:

```bash
grund FS-integrations.4.3 --format json
```

Then confirm it works from anywhere in the tree, because a click carries no
directory with it:

```bash
cd crates/grund-core/src && grund-open '§FS-integrations.3.1'
```

Same file, same line. Finally, do it for real — click one.

The matchers only see **what is currently on screen**, so put citations there and
leave them there. Do not `cat` a long file: the citations scroll past and you are
left looking at a tail that may contain none, which reports no matches and reads
exactly like a broken install.

```bash
grep -n '§FS-\|§DF-' crates/grund-core/src/integrations.rs | head -20
```

Now trigger your client — `ctrl+shift+g` in kitty, ctrl/cmd-click in WezTerm —
and pick `§FS-integrations.4.3`. It should open `FS-integrations.md` at line 85,
the `### 4.3` heading.

## 5. Peek without leaving the terminal

Opening a citation answers *where it is*. Often you only want to know *what it
says* — and a trip to the editor costs more than the answer is worth.

Every client has a second binding for that. It renders the declaration into
something disposable right next to your work, and closes when you quit the
pager:

| Client | Peek | Where it appears |
| --- | --- | --- |
| **kitty** | `ctrl+shift+p`, then the hint label | overlay window |
| **tmux** | `prefix` + `G` | popup (needs tmux 3.2+) |
| **wezterm** | `ctrl+shift+`-click | split pane to the right |

You get the resolved `path:line` on the first line, then the declaration's lead.
It is the same resolver in a different mode, so a peek and a click can never
point at different places.

**VS Code gets real hover instead.** Its extension attaches the declaration's
heading and first paragraph to the link as a tooltip, so you just point at a
citation — no click, no binding. No terminal can do this: none of them expose a
hover event or a link tooltip to configuration, and
[WezTerm has had an open request for one since 2018](https://github.com/wez/wezterm/issues/4).
Peek is the terminal's answer to the same question.

## 6. Tell your agent how to write citations

A clickable citation is only useful if your agent writes citations you can
click. The same `--write` records that preference and syncs it into the global
instruction files for Codex, Claude, Gemini, GitHub Copilot, Zed, and Pi
([§FS-integrations.4.3](functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions)).

**`plain`** is the default, and the right choice once you have an integration
installed: the agent writes a bare `§<ID>` and your terminal makes it clickable.

**`link`** is for a TUI that cannot render clicks. The agent follows each
citation with its location as plain `path:line` text, which you can copy or
`ctrl+click` in most terminals.

```bash
grund integrations --write --conversation link
```

That preference-only form updates your config and global instructions without
installing a client you do not want.

A repository can also commit an opinion. When its `.agents/grund.toml` sets
`conversation = "link"` under `[reference]`
([§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form)),
agents working in that repository use the linked form regardless of your
personal preference — useful when a project's readers are mostly on surfaces
without an integration. Precedence is: repository opinion, then your
preference, then `plain`.

## 7. When a click does nothing

Work from the inside out — this separates a matcher problem from a resolver
problem in one step:

```bash
grund-open '§FS-integrations.3.1'
```

**If that opens the file**, the resolver is fine and the terminal is not
matching or not wired. Reload the client's config. For WezTerm, check that
`grund_apply_hyperlink_rule(config)` is actually called on the config you
return.

**If it prints `no .agents/grund.toml at or above …`**, you are outside a grund
repository. The resolver looks upward from the current directory for the
project root; it needs to be inside one.

**If it prints `unknown id …`**, the citation does not resolve in this
repository. Check with `grund list`.

**If it prints nothing and exits**, no editor was found. See §1.

**If nothing on screen is clickable at all**, check whether the repository uses
a custom `marker`. The matchers accept any short punctuation marker, not just
`§`, but they do require one — a bare ID-shaped token is deliberately not
clickable, because that would make every `FS-`-prefixed word in your terminal a
link.

## 8. Remove it

Delete the marked block from the client's config — it is bracketed by
`>>> grund integrations` and `<<< grund integrations` comments — and delete
`~/.local/bin/grund-open`. Nothing else was touched.

---

The full behavioral contract, including the resolver's exact resolution steps
and the managed-block rules, is
[§FS-integrations](functional-spec/FS-integrations.md#fs-integrations-grund-prints-and-installs-its-rendering-layer-integrations).
