# Clickable citations

Make a `§<ID>` citation in your terminal jump straight to the line it cites.

Agents write citations constantly — in answers, in commit messages, in code
comments. Without an integration a citation is a string you have to go look up.
With one, `§FS-integrations.3.1` is a thing you click, and your editor opens at
that section's own line.

This is a one-time setup per machine. It changes nothing in any repository.

---

## Quick start

Three commands, if the three checks below already pass:

```bash
grund integrations                  # what applies in this environment
grund integrations kitty            # read the snippet and the resolver first
grund integrations kitty --write    # install it
```

Supported clients: `codium`, `iterm2`, `kitty`, `tmux`, `vscode`, `wezterm`.
Then reload the client and click:

| Client | Reload with | Click with |
| --- | --- | --- |
| **kitty** | `ctrl+shift+F5` | `ctrl+shift+p` `g`, then the hint label |
| **wezterm** | automatic — but **wire it up first** ([§1](#1-install)) | ctrl-click, or `ctrl+shift+g` then the label |
| **tmux** | `tmux source-file ~/.tmux.conf` | select in copy mode, then `prefix` + `g` |
| **vscode**, **codium** | *Developer: Reload Window* | click the link in the integrated terminal |
| **iterm2** | — **apply the rule by hand** ([§1](#1-install)) | cmd-click the citation |

**The three checks.** Each has a quiet failure mode, and on a machine where one
of them is false every click silently does nothing:

```bash
grund --version                                     # 1. grund is on your PATH
case ":$PATH:" in *":$HOME/.local/bin:"*) echo ok ;; *) echo "add it" ;; esac
echo "${GRUND_OPEN_CMD:-${EDITOR:-<none — falls back to code/codium>}}"
```

The second is where the resolver script is installed, and on **macOS it is not
on the default `PATH`** — add it to your shell profile before going further. The
third is the editor the resolver opens files with. Both are explained in
[§4](#4-when-a-click-does-nothing), which is also where to look when a click
does nothing.

**Also in this guide:** [install](#1-install) · [check it
works](#2-check-it-works) · [peek without opening an
editor](#3-peek-without-leaving-the-terminal) · [when a click does
nothing](#4-when-a-click-does-nothing) · [citations in
conversations](#5-citations-in-conversations) · [terminals that are not
supported](#6-terminals-that-are-not-supported) · [remove
it](#7-remove-it)

## 1. Install

`grund integrations` reads your environment and prints the integrations that
match, each with its one-line install, and closes by naming this guide. Nothing
is written and nothing is scanned. If you are in a VS Code terminal inside tmux
inside WezTerm, all three match — that is normal, and you can install all three.

Preview any client before committing to it:

```bash
grund integrations kitty
```

That prints the exact config snippet and the full resolver script, so you can
read both before anything touches your dotfiles. `--write` then installs it:

```bash
grund integrations kitty --write
```

The write is idempotent and lands as a marked block, so re-running is safe and
removing it later is a matter of deleting the block ([§7](#7-remove-it)).

Three clients need a step grund cannot take for you.

**Use `codium`, not `vscode`, if you run VSCodium.** They are separate
applications with separate extension roots, and installing into the wrong one
fails silently — the install reports success and no link ever appears. If you
are unsure, `grund integrations` will name whichever it can detect.

**iTerm2 is applied by hand.** It keeps its settings in a binary property list,
not a text file, so there is no config to manage a block in and grund will not
rewrite a live profile under you. `grund integrations iterm2 --write` installs
the resolver and then prints the rule to add under
*Settings → Profiles → Advanced → Smart Selection*: the citation regex, and a
`Run Command…` action of `grund-open \0`. Add a second action of
`grund-open --peek \0` to get peek on the right-click menu.

Before you do any of that, though, see the note at the end of
[§3](#3-peek-without-leaving-the-terminal) — on iTerm2 you may not need a rule
at all.

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
`--write` says so rather than leaving you to find out by clicking:

```
$ grund integrations wezterm --write
updated /home/you/.config/wezterm/wezterm.lua
note: /home/you/.config/wezterm/wezterm.lua does not call `grund_apply_hyperlink_rule(config)` on the config it returns, so WezTerm reads the block and registers nothing; add that line where you build your config
```

The note repeats on every write until the call is there, and stops on its own
once it is.

If you had no `wezterm.lua`, `--write` creates one that already calls it, and
you can skip this. That starter config sits below the managed block and is
yours: later writes rewrite only the block.

**WezTerm also needs your shell to report its directory.** A click carries a
citation, not a location, so the resolver has to run in the clicked pane's
directory — and WezTerm only knows that directory if the shell tells it, with
`OSC 7`. Most shells never do: the usual emitter, `/etc/profile.d/vte.sh`,
returns early for any terminal that is not VTE, and WezTerm is not. Add it to
your shell profile:

```bash
# bash — in ~/.bashrc
__osc7() { printf '\033]7;file://%s%s\033\\' "${HOSTNAME:-localhost}" "$PWD"; }
PROMPT_COMMAND="__osc7${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
```

```zsh
# zsh — in ~/.zshrc
__osc7() { printf '\033]7;file://%s%s\033\\' "${HOST:-localhost}" "$PWD"; }
precmd_functions+=(__osc7)
```

Check it with `wezterm cli list`: the `cwd` column is empty until this works,
and a click in a pane with no `cwd` now says so in a notification rather than
doing nothing. This matters most under **Flatpak**, where WezTerm runs your
shell on the host through `flatpak-spawn` and `OSC 7` is the only way the
directory can reach it at all.

## 2. Check it works

Use grund's own repository — every ID below is real, so you can compare against
what you see.

```bash
cd /path/to/grund
grund-open '§FS-integrations.3.1'
```

Your editor should open `docs/functional-spec/FS-integrations.md` at the
`### 3.1` heading itself — **not line 1**. That distinction is the whole point
of the section suffix, so it is worth checking all three shapes:

| Run this | Opens |
| --- | --- |
| `grund-open '§FS-integrations'` | `FS-integrations.md:1` — the declaration |
| `grund-open '§FS-integrations.3.1'` | the same file at its `### 3.1` heading |
| `grund-open '§FS-integrations.4.3'` | the same file at its `### 4.3` heading |
| `grund-open 'README.md:1'` | `README.md` at line 1 — a *location*, the `path:line` an agent prints beside a citation |

`grund` is the authority on which line a click should land on, so ask it rather
than counting:

```bash
grund FS-integrations.4.3 --format json
```

The last row is the other clickable shape: a location needs no `grund` and no
grund repository at all — the resolver climbs to the nearest ancestor holding
the file and opens it at that line — so the text agents write beside citations
in the `link` conversation form ([§5](#5-citations-in-conversations)) is a link
in its own right.

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

Now trigger your client — `ctrl+shift+p` then `g` in kitty, ctrl-click in
WezTerm — and pick `§FS-integrations.4.3`. It should open `FS-integrations.md`
at the `### 4.3` heading.

## 3. Peek without leaving the terminal

Opening a citation answers *where it is*. Often you only want to know *what it
says* — and a trip to the editor costs more than the answer is worth.

Every client has a second binding for that. It renders the declaration into
something disposable right next to your work, and closes when you quit the
pager:

| Client | Peek | Where it appears |
| --- | --- | --- |
| **kitty** | `ctrl+shift+p` `i`, then the hint label | overlay window |
| **tmux** | `prefix` + `G` | popup (needs tmux 3.2+) |
| **wezterm** | `ctrl+shift+`-click, or `ctrl+shift+i` then the label | split pane to the right |

**On WezTerm, prefer the keyboard.** `ctrl+shift+i` labels every citation on the
screen and peeks at the one whose label you type; `ctrl+shift+g` opens instead.
No pointer is involved, so nothing can swallow the gesture the way a
mouse-capturing full-screen program swallows a click — and citations tend to
appear while you are typing anyway.

**Both terminals use the same two letters:** `g` to *go* to a citation, `i` to
*inspect* it. WezTerm takes them plainly, as `ctrl+shift+g` and `ctrl+shift+i`.
kitty puts them behind `ctrl+shift+p`, the prefix its own hint kittens already
live under (`p` `f` for a path, `p` `y` for a hyperlink) — both plain keys are
kitty defaults, and taking either would delete a binding you already have.

You get the resolved `path:line` on the first line, then the declaration's lead.
It is the same resolver in a different mode, so a peek and a click can never
point at different places.

On **iTerm2**, peek is the second action on the Smart Selection rule, which puts
it on the citation's right-click menu — iTerm2 fires only the first action on
cmd-click.

**VS Code and VSCodium get real hover instead.** The extension attaches the declaration's
heading and first paragraph to the link as a tooltip, so you just point at a
citation — no click, no binding. No terminal can do this: none of them expose a
hover event or a link tooltip to configuration, and
[WezTerm has had an open request for one since 2018](https://github.com/wezterm/wezterm/issues/4).
Peek is the terminal's answer to the same question.

**On iTerm2 you may not need any of this.** Its Semantic History already makes a
plain `path:line` cmd-clickable, line number included. So:

```bash
grund integrations --write --conversation link
```

is enough on its own — agents then write the location beside each citation, and
iTerm2 makes it clickable with no rule installed. Set up the Smart Selection rule
only if you want the bare `§<ID>` itself to be the clickable thing.

## 4. When a click does nothing

Work from the inside out — this separates a matcher problem from a resolver
problem in one step:

```bash
grund-open '§FS-integrations.3.1'
```

**If that opens the file**, the resolver is fine and the terminal is not
matching or not wired. Reload the client's config. For WezTerm, check that
`grund_apply_hyperlink_rule(config)` is actually called on the config you
return ([§1](#1-install)).

**If the command is not found**, `~/.local/bin` — where `--write` installs the
resolver — is not on your `PATH`. On **macOS this is usually the one that
bites**: it is not on the default `PATH` there, so the resolver is never found
and every click silently does nothing.

**If it prints `grund-open: set GRUND_OPEN_CMD or EDITOR to open …`**, no editor
was found. The resolver opens files with `GRUND_OPEN_CMD` if set, otherwise
`EDITOR`, otherwise the first of `code`, `codium`, `code-insiders` on your
`PATH`. Set one of the variables somewhere the **terminal itself** sees, not
only in your shell rc: a GUI terminal spawns the resolver from its own process,
so a variable exported by `.zshrc` or `.bashrc` — which only the shell *inside*
a window ever ran — is not there; kitty says as much about its own `--copy-env`.
`~/.profile`, your desktop session's environment, or the terminal's config
(kitty's `env` directive) all work.

`EDITOR` may carry flags, and the line number is passed in the syntax your
editor understands — `+LINE FILE` for the vi and emacs families, `--goto
FILE:LINE` for VS Code and Sublime, `FILE:LINE` for helix and micro. Set
`GRUND_OPEN_CMD` instead when you want an argv prefix that ignores all of that;
for anything more elaborate, point it at a wrapper script.

**If it prints `no grund.toml (root or .agents/) at or above …`**, you are outside a grund
repository. The resolver looks upward from the current directory for the
project root; it needs to be inside one.

**If it prints `unknown id …`**, the citation does not resolve in this
repository. Check with `grund list`.

One case of that is expected: a printed spec *path* contains ID-shaped
segments, so `docs/functional-spec/FS-integrations.md` is also matched at
`/functional-spec/FS-integrations` — the matcher cannot tell a path segment
from a citation with a custom marker. Clicking that fragment reports
`unknown id`; the real citation on the same line still works. Click the
`§<ID>`, not the path beside it.

A path with a `:line` suffix — the location agents write beside citations in
the `link` form — does not fall into this: the location matcher claims the
whole `path:line` first and clicking it opens the file at that line. What
remains is the bare path with no line suffix, where the ID-shaped fragment can
still match; there, click the `§<ID>`, not the path — a misclicked *open*
reports its `unknown id` only in the terminal's own log, while a *peek* shows
the error in its surface.

**If nothing on screen is clickable at all**, check whether the repository uses
a custom `marker`. The matchers accept any short punctuation marker, not just
`§`, but they do require one — a bare ID-shaped token is deliberately not
clickable, because that would make every `FS-`-prefixed word in your terminal a
link.

## 5. Citations in conversations

Everything so far makes citations clickable in your *terminal*. This section
is about the citations agents write in *conversation* — answers, reviews,
session transcripts. Two independent things can make those navigable:

1. **A rendering layer on your machine** ([§1](#1-install)) turns a bare `§<ID>`
   into a click.
2. **The agent carrying the declaration's location with each citation** — as a
   Markdown link over an absolute URI, so the visible text stays the citation
   and the click opens the file. Which scheme it uses is yours to pick
   ([§5.2](#52-which-scheme-the-link-uses)).

Pick your situation:

**Your terminal is supported (wezterm, kitty, tmux, iterm2, vscode).**
Install the integration ([§1](#1-install)). That records the `plain`
preference: agents write bare `§<ID>` citations and your terminal makes them
clickable — no location noise beside every citation.

**Your surface cannot click bare citations** (a stock terminal, ssh, a TUI
with no matcher). Tell agents to write the location beside each citation:

```bash
grund integrations --write --conversation link
```

This preference-only form touches no terminal config; it updates your grund
config and the global instruction files. By default agents then write
`[§FS-check](file:///abs/path/docs/functional-spec/FS-check.md#L1)` — the
citation as the visible text, the file behind it. Pick a different scheme with
`--conversation-target` ([§5.2](#52-which-scheme-the-link-uses));
`--conversation-target path` gets you the older plain `path:line` form, which
iTerm2 and the VS Code terminal click natively and the terminal integrations
match as a *location*.

**You want everyone served, not just you** — teammates who will never run a
setup command, cloud agent sessions, CI reviewers, and Cursor or Windsurf,
which have no user-level file grund can write. Commit the opinion in the
repository's `grund.toml`
([§FS-config.3.1](../functional-spec/FS-config.md#31-reference--citation-form)):

```toml
[reference]
conversation = "link"
```

and re-run `grund init`. The generated agent entrypoint then teaches the
linked form to every agent that clones the repository — it is the only channel
that reaches readers whose machines grund never touched.

### 5.1 The key, and where to put it

The setting has one name, `[reference] conversation`, and lives in two files.
Which file you put it in is the whole decision — it decides *who* is
instructed, and that in turn decides which values are legal:

| File | Scope | Values | Instructs |
|---|---|---|---|
| `~/.config/grund/config.toml` (or `$XDG_CONFIG_HOME/grund/config.toml`) | this machine, every repository | `plain` \| `link` | every agent you run here |
| `<repo>/grund.toml`, committed | this repository, every machine | `link` only | every agent that clones it |

```toml
# ~/.config/grund/config.toml — your machine
[reference]
conversation = "plain"   # or "link"
```

**Use the machine file** to describe *your* setup: `plain` when you installed
an integration from [§1](#1-install) and bare citations are already clickable
for you, `link` when they are not. `grund integrations --write` writes it for
you, but editing it by hand is equivalent — run `grund integrations --write
--conversation <value>` afterwards to push the change into the global
instruction files.

`reference.conversation` and `reference.conversation_target`
([§5.2](#52-which-scheme-the-link-uses)) are the **only** keys grund reads from
this file, and nothing in it is ever fatal. Anything grund did not act on is
reported at its line and then ignored:

```
$ grund integrations --write --conversation link
warning: /home/you/.config/grund/config.toml:2: unused key `reference.marker`; grund reads only `reference.conversation` and `reference.conversation_target` from this file
warning: /home/you/.config/grund/config.toml:5: ignoring `reference.conversation = "bare"`: must be one of plain | link
```

That covers a typo, a key that only belongs in a repository's
`grund.toml`, a spelling from an older grund, and a value grund cannot
read — the last simply leaves it with no preference from this file, exactly
like a machine that never wrote one. A repeated key resolves to the first.
This is the opposite of a repository config, where an unknown key or a bad
value is refused outright
([§FS-config.4.3](../functional-spec/FS-config.md#43-invalid-config-behavior)):
that file decides whether a tree checks clean, while this one only picks
between two renderings of the same true citation, so a stale line here should
never stop you installing a terminal integration.

**Use the repository key** to describe what *readers of this project* need
when their machine has said nothing — teammates on a fresh clone, cloud agent
sessions, CI reviewers, Cursor and Windsurf users. Leave it absent if
conversation rendering is each contributor's own business.

`plain` is not accepted in the repository file, and this is the point of the
split rather than an arbitrary restriction: `plain` claims a resolver is
installed, which is true of a machine and never of a repository. Committing it
would silently break the fresh clones the key exists to serve, so
`grund init` and `grund check` both reject it with exit `2`.

**How the layers combine.** Your machine's recorded preference wins over the
repository opinion; the repository opinion covers every machine that never
stated one; the default is `plain`
([§DF-repo-conversation-opinion.2.3](../decisions/functional/DF-repo-conversation-opinion.md#23-precedence)).
So committing `link` costs installed users nothing — their agents still write
bare clickable citations — while everyone else gets locations. Concretely:

| Reader | Where the guidance comes from | What they get |
|---|---|---|
| You, integration installed | your global instruction files (`plain`) | bare `§<ID>`, clickable in the terminal |
| You, no rendering layer | your global instruction files (`link`) | `[§<ID>](<uri>)` in Claude, in the scheme you picked ([§5.2](#52-which-scheme-the-link-uses)) |
| A teammate, fresh clone | the committed entrypoint | the same link over `file:`, zero setup |
| Cloud / CI agent session | the committed entrypoint | a citation carrying its location in the transcript |
| Cursor / Windsurf | the committed entrypoint (their only grund channel) | `§<ID>` + `path:line` in the panel |

The global instruction files are written for Codex, Claude, Gemini, GitHub
Copilot, Zed, and Pi
([§FS-integrations.4.3](../functional-spec/FS-integrations.md#43-user-preference-and-global-agent-instructions));
the blocks scope themselves to grund repositories, so they are inert
everywhere else. Only agents you actually use are touched — a target whose
directory (`~/.claude`, `~/.codex`, …) does not exist is reported `skipped`
and nothing is created for it:

```
$ grund integrations --write --conversation link
appended /home/you/.config/grund/config.toml
appended /home/you/.codex/AGENTS.md
appended /home/you/.claude/CLAUDE.md
skipped /home/you/.gemini/GEMINI.md (no ~/.gemini)
skipped /home/you/.copilot/copilot-instructions.md (no ~/.copilot)
skipped /home/you/.config/zed/AGENTS.md (no ~/.config/zed)
skipped /home/you/.pi/agent/AGENTS.md (no ~/.pi)
```

Install one of those agents later and re-run the same command; it is
idempotent, so the only thing that changes is the target that just appeared.

### 5.2 Which scheme the link uses

`[reference] conversation_target` picks how a linked citation addresses its
declaration. It is a machine key only — there is no repository spelling,
because the scheme depends on what *your* desktop can open:

```bash
grund integrations --write --conversation-target vscodium
```

| Value | What agents write | Use it when |
|---|---|---|
| `file` (default) | `file://<abs>#L<line>` | you want no assumptions — every desktop opens `file:` somehow |
| `vscode`, `vscodium`, `cursor` | `<scheme>://file<abs>:<line>` | you want the declaration to open in your editor, at the line |
| `web` | the forge URL at the current commit | the transcript will be read by people without the repository |
| `path` | the location as plain `path:line` text | you prefer no link at all |

The flag records the value and re-renders the instruction blocks; editing the
key by hand and re-running `grund integrations --write` is equivalent. It is
accepted (and inert) while your preference is still `plain`, so the target you
picked is waiting if you later switch to `link`.

**Not every agent gets your choice, deliberately.** grund writes the linked
form only into the instruction files of agents whose renderers are verified to
honor it — Claude today, plus `web` for Codex, whose TUI replaces a local
Markdown destination with the URL and erases the citation itself. Every other
agent's block keeps plain `path:line`, the form it already had
([§DF-conversation-link-target.2.4](../decisions/functional/DF-conversation-link-target.md#24-the-form-is-gated-per-agent-and-the-fallback-is-path)).
The same gate applies to the repository entrypoints: `CLAUDE.md` teaches the
link form, `AGENTS.md` does not — so if your `CLAUDE.md` is a symlink to
`AGENTS.md`, it is one file and it keeps the plain form. Run
`grund init --claude` to write real Claude entrypoints instead.

### 5.3 Codex, specifically

grund can instruct Codex to emit citation links; it cannot change Codex's
renderer, and that renderer is the whole constraint. Click-tested 2026-08-11:

- **HTTPS links work**, and they keep `§AR-checker` as the clickable label.
- **Local paths, `file:` URLs, and editor-scheme URLs are not clickable here.**
  A `file:` target is worse than not clickable: the citation is replaced by the
  destination, so the reference disappears from the transcript.
- **Codex exposes no configuration that would enable arbitrary local Markdown
  links.** `desktop.custom_file_handlers` is not that switch — it only chooses
  the "Open in" target for files Codex already recognizes.

So for Codex: **use `web` if you want clickable citations, and otherwise leave
it on plain `§<ID> path:line`.** That is what the gate does for you already —
`web` passes through to Codex, every local scheme falls back to `path` — so the
only decision left is whether the transcripts you read there are worth pointing
at the forge instead of at your disk.

If you read both Claude and Codex, you do not have to pick — override the one
that differs ([§5.4](#54-overriding-one-agent)).

### 5.4 Overriding one agent

`conversation_target` is machine-wide, but agents do not render alike, so a
single value is the wrong granularity for a machine that reads two of them.
`--agent` scopes the choice to one:

```bash
grund integrations --write --conversation-target vscodium   # the machine
grund integrations --write --agent codex --conversation-target web
```

which records a partial under that agent, merged over the base:

```toml
[reference]
conversation        = "link"
conversation_target = "vscodium"   # everyone inherits this

[reference.agents.codex]
conversation_target = "web"        # except Codex
```

The rule is one line: **a key under an agent replaces the base for that agent;
an absent key inherits it.** There is no per-agent default and nothing to
unset — delete the entry and that agent goes back to inheriting. Accepted agent
names are `codex`, `claude`, `gemini`, `copilot`, `zed`, and `pi`; anything else
is an error listing the six.

**An override is a preference, not evidence.** It sets what you *ask* for; the
gate in [§5.3](#53-codex-specifically) still decides what gets written. Asking
for `vscodium` under `codex` resolves to `vscodium` and is then held at `path`,
exactly as the machine-wide value would be — because the click-test says the
citation would be worse there, and no key should be able to buy that
([§DF-conversation-link-target.2.5](../decisions/functional/DF-conversation-link-target.md#25-a-per-agent-override-is-a-preference-not-evidence)).
The pairing above needs no such power: `web` is a request the gate already
grants Codex.

Both layers show up in the report, so you never have to guess which one acted:

```
$ grund integrations --write --conversation-target vscodium
updated /home/you/.claude/CLAUDE.md (link → vscodium)
updated /home/you/.codex/AGENTS.md (link → web; agent override)
updated /home/you/.gemini/GEMINI.md (link → path; vscodium unverified here)
skipped /home/you/.pi/agent/AGENTS.md (no ~/.pi)
```

## 6. Terminals that are not supported

Some terminals have no way to make arbitrary text clickable, so there is nothing
to install:

- **Ghostty** documents the exact option needed — `link`, matching a regex and
  running an action — but its reference currently reads
  *"TODO: This can't currently be set!"*. Only built-in URL matching works, and
  Ghostty has no `path:line` opener either, so the `link` conversation mode does
  not help as a fallback. When upstream ships `link`, support is a few lines.
- **Apple Terminal.app** and **Windows Terminal** have no regex-to-action hook at
  all.

On Windows, use **WSL** — it is Linux, so kitty, tmux, and wezterm all work
unchanged. Native Windows is not supported: the resolver is a POSIX shell
script. The exception is VS Code, which never uses the resolver and works there
today.

## 7. Remove it

`--write` touches four things, and each is removed independently:

1. **The client integration.** For kitty, tmux, and wezterm, delete the marked
   block from the client's config — it is bracketed by `>>> grund integrations`
   and `<<< grund integrations` comments. For `vscode` and `codium`, delete the
   extension directory instead
   (`~/.vscode/extensions/grund.grund-terminal-citations`, or
   `~/.vscode-oss/extensions/…` for VSCodium). For iTerm2, remove the Smart
   Selection rule you added by hand.
2. **The resolver**, at `~/.local/bin/grund-open`.
3. **The conversation preference**, in `~/.config/grund/config.toml` — the
   `[reference]` keys of [§5.1](#51-the-key-and-where-to-put-it). Delete the
   file if grund wrote it for you.
4. **The global agent instruction blocks**, one per agent you have installed
   (`~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md`, `~/.gemini/GEMINI.md`,
   `~/.copilot/copilot-instructions.md`, `~/.config/zed/AGENTS.md`,
   `~/.pi/agent/AGENTS.md`). Each is a marked block bracketed by
   `>>> grund integrations citation rendering` HTML comments; everything
   outside it is yours and was never touched.

No repository is involved in any of this. If you also committed the
`[reference] conversation` opinion of [§5.1](#51-the-key-and-where-to-put-it)
in a repository's `grund.toml`, that is a separate, checked-in setting —
remove it there and re-run `grund init`.

---

The full behavioral contract, including the resolver's exact resolution steps
and the managed-block rules, is
[§FS-integrations](../functional-spec/FS-integrations.md#fs-integrations-grund-prints-and-installs-its-rendering-layer-integrations).
