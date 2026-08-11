-- grund citation integration for WezTerm.
-- Makes a §<ID> citation in the terminal Ctrl/Cmd-clickable: the click resolves
-- the citation with `grund` and opens the declaration in your editor via the
-- installed `grund-open` resolver (§FS-integrations). The location an agent
-- prints beside a citation — plain `path:line` text — is clickable the same
-- way, opening the editor at that line.
--
-- WezTerm applies hyperlink rules only from the config object you return, so add
-- one line where you build your config:
--
--     local config = wezterm.config_builder()
--     -- ... your settings ...
--     grund_apply_hyperlink_rule(config)   -- defined below
--     return config
--
-- (The open-uri handler below is global and needs no wiring.)
--
-- Gestures, once that line is in place:
--
--     ctrl-click              open the citation under the mouse
--     ctrl+shift-click        peek at it in a split pane
--     ctrl+shift+g            label every citation on screen, then open one
--     ctrl+shift+i            the same, but peek
--
-- The two keyboard gestures exist because the mouse is the fragile path: a
-- full-screen program that has captured the mouse can swallow a click, and
-- there is nothing to click at all when your hands are on the keyboard.
--
-- One more requirement, and it is not WezTerm's fault: your shell must report
-- its directory with OSC 7, or WezTerm has no idea where the clicked pane is
-- and the citation cannot be resolved. Most shells do not do this by default —
-- the usual emitter, vte.sh, skips every terminal that is not VTE. For bash:
--
--     __osc7() { printf '\033]7;file://%s%s\033\\' "${HOSTNAME:-localhost}" "$PWD"; }
--     PROMPT_COMMAND="__osc7${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
--
-- and for zsh, the same printf in a `precmd` function. Without it, a click says
-- so in a toast rather than doing nothing.
local wezterm = require 'wezterm'

-- The citation shape, written once and shared by the hyperlink rule and the
-- keyboard selection below, so a click and a keypress can never disagree about
-- what counts as a citation. Every group is non-capturing: the whole citation
-- has to stay the $0 match, and WezTerm's quick-select hands back the first
-- capture group when one exists.
--
-- The leading [^\w\s]{1,3} matches the citation marker without naming it:
-- `[reference] marker` is per-repo while this file is user-global and installed
-- once, so hardcoding § would leave every repo with a custom marker silently
-- unclickable. grund-open strips whatever punctuation this sweeps in.
grund_citation_pattern =
  '[^\\w\\s]{1,3}(?:[a-z][a-z0-9-]*/)?[A-Z][A-Z0-9]*-[a-z0-9][a-z0-9-]*(?:\\.[0-9]+)*'

-- The *location* beside a citation (§FS-integrations.3.1): the `link`
-- conversation form prints `§<ID> path:line`, and WezTerm's default rules
-- match URLs only, so the location half would be inert text. A final path
-- segment with a dot-extension and a :line (optionally :col) suffix is a
-- location; grund-open tells the two shapes apart itself — an ID's section
-- suffix is dotted, never coloned — so both rules share one handler.
grund_location_pattern =
  '(?:[A-Za-z0-9_.~-]+/)*[A-Za-z0-9_.~-]+\\.[A-Za-z0-9]+:[0-9]+(?::[0-9]+)?'

-- Append grund's rules to config.hyperlink_rules, seeding WezTerm's
-- defaults first when the config carries none of its own.
function grund_apply_hyperlink_rule(config)
  config.hyperlink_rules = config.hyperlink_rules or wezterm.default_hyperlink_rules()
  -- Location before citation: rules are applied in order, and the citation
  -- matcher false-positives on ID-shaped path segments (§FS-integrations.3.1).
  -- Registered first, the location rule claims the whole `path:line` as one
  -- correct link before that fragment can form inside it.
  table.insert(config.hyperlink_rules, {
    regex = grund_location_pattern,
    format = 'grund:$0',
  })
  table.insert(config.hyperlink_rules, {
    regex = grund_citation_pattern,
    format = 'grund:$0',
  })
  -- Adding the bindings here rather than in the scaffold means an existing
  -- config that already calls this function gets them without re-wiring.
  -- WezTerm's own defaults still apply to anything not listed here.
  --
  -- Each gesture is registered twice: once for a plain shell, and once with
  -- mouse_reporting = true — WezTerm ignores user mouse bindings while the
  -- foreground program has captured the mouse, and the very programs that
  -- print citations (Claude Code, other agent TUIs, editors) are mouse-
  -- capturing full-screen apps (§FS-integrations.3.1).
  config.mouse_bindings = config.mouse_bindings or {}
  table.insert(config.mouse_bindings, grund_open_mouse_binding(false))
  table.insert(config.mouse_bindings, grund_open_mouse_binding(true))
  table.insert(config.mouse_bindings, grund_peek_mouse_binding(false))
  table.insert(config.mouse_bindings, grund_peek_mouse_binding(true))

  -- The keyboard path (§FS-integrations.3.3). ctrl+shift+p would mirror kitty's
  -- peek key, but WezTerm binds it to the command palette, so peek takes `i`
  -- for inspect; `g` is free and matches kitty's open key.
  config.keys = config.keys or {}
  table.insert(config.keys, { key = 'g', mods = 'CTRL|SHIFT', action = grund_quick_select(false) })
  table.insert(config.keys, { key = 'i', mods = 'CTRL|SHIFT', action = grund_quick_select(true) })
end

-- Label every citation on screen and act on the one you pick. This is the same
-- resolver in the same two modes as the mouse gestures, reached without a mouse:
-- a full-screen program that has captured the mouse can swallow a click, and
-- keyboard work should not have to reach for the pointer to read a citation.
--
-- Passing an `action` suppresses quick-select's normal copy, so the clipboard is
-- left alone; the picked text arrives as the pane's selection.
function grund_quick_select(peek)
  return wezterm.action.QuickSelectArgs {
    label = peek and 'peek citation or location' or 'open citation or location',
    patterns = { grund_location_pattern, grund_citation_pattern },
    action = wezterm.action_callback(function(window, pane)
      local citation = window:get_selection_text_for_pane(pane)
      if citation and citation ~= '' then
        grund_resolve(window, pane, peek, citation)
      end
    end),
  }
end

-- Ctrl-click opens the declaration in your editor; Ctrl-Shift-click peeks at it
-- in a split pane instead (§FS-integrations.3.3). WezTerm has no hover event and
-- no link tooltip (wezterm/wezterm#4, open since 2018), so peek is the read-without-
-- leaving path here.
--
-- Ctrl-click duplicates a WezTerm default on purpose: the default variant is
-- inert inside a mouse-capturing TUI, and registering our own pair keeps the
-- open gesture and the peek gesture governed by the same table.
function grund_open_mouse_binding(mouse_reporting)
  return {
    event = { Up = { streak = 1, button = 'Left' } },
    mods = 'CTRL',
    mouse_reporting = mouse_reporting,
    action = wezterm.action.OpenLinkAtMouseCursor,
  }
end

-- Lua cannot ask which link is under the mouse — only the built-in
-- OpenLinkAtMouseCursor knows that, and all it exposes is the resulting URI via
-- open-uri. window:current_event() does not carry modifiers either. So the
-- binding records the intent and delegates; the handler below reads it back.
-- This is safe because both run in one synchronous event, not across ticks.
grund_peek_requested = false

function grund_peek_mouse_binding(mouse_reporting)
  return {
    event = { Up = { streak = 1, button = 'Left' } },
    mods = 'CTRL|SHIFT',
    mouse_reporting = mouse_reporting,
    action = wezterm.action_callback(function(window, pane)
      grund_peek_requested = true
      window:perform_action(wezterm.action.OpenLinkAtMouseCursor, pane)
      -- Cleared here too: a Ctrl-Shift-click on non-link text never reaches
      -- open-uri, and a stale flag would turn the next plain click into a peek.
      grund_peek_requested = false
    end),
  }
end

-- grund-open finds the repository by walking up from its working directory
-- (§FS-integrations.3.1), and the WezTerm GUI process's own cwd is wherever the
-- desktop launched it — so every spawn below must run in the *clicked pane's*
-- directory instead.
--
-- There is no single way to ask WezTerm for it. Newer builds answer
-- `get_current_working_directory` with a Url object; older ones spell it
-- `get_current_working_dir` and answer with a "file://host/path" string. Both
-- are empty unless the shell emits OSC 7, which no shell does by default here:
-- the usual emitter, the distributions' vte.sh, returns early for any terminal
-- that is not VTE. And calling a method the running build does not export is a
-- Lua *error*, not a nil — it aborts this handler before anything is spawned,
-- which looks exactly like a broken install.
--
-- So each lookup is guarded and tried in turn, falling back to the foreground
-- process's own cwd, which needs no cooperation from the shell. That fallback is
-- skipped under Flatpak: there the pane's program is spawned on the *host*
-- through flatpak-spawn, so the process WezTerm can see is the sandbox-side
-- helper, and its cwd is a real directory that is simply not the shell's — a
-- wrong answer, which is worse than none. OSC 7 crosses that boundary; nothing
-- else does.
function grund_pane_cwd(pane)
  if not pane then
    return nil
  end

  local function path_of(cwd)
    if not cwd then
      return nil
    end
    if type(cwd) == 'string' then
      local path = cwd:gsub('^file://[^/]*', '')
      if path ~= '' then
        return path
      end
      return nil
    end
    -- A Url object; older builds hand back something without a file_path.
    local ok, file_path = pcall(function()
      return cwd.file_path
    end)
    if ok and file_path ~= '' then
      return file_path
    end
    return nil
  end

  -- Indexing an absent field can itself raise, so even the lookup is guarded.
  local function method(name)
    local ok, fn = pcall(function()
      return pane[name]
    end)
    if ok and type(fn) == 'function' then
      return fn
    end
    return nil
  end

  for _, name in ipairs { 'get_current_working_directory', 'get_current_working_dir' } do
    local getter = method(name)
    if getter then
      local ok, cwd = pcall(getter, pane)
      local path = ok and path_of(cwd) or nil
      if path then
        return path
      end
    end
  end

  if not os.getenv 'FLATPAK_ID' then
    local process_info = method 'get_foreground_process_info'
    if process_info then
      local ok, info = pcall(process_info, pane)
      if ok and info and info.cwd and info.cwd ~= '' then
        return info.cwd
      end
    end
  end

  return nil
end

-- Build the argv that runs the resolver. It goes through `sh` so the pane's
-- directory and the citation travel as arguments — never spliced into shell
-- source — and so ~/.local/bin (where --write installs grund-open) and
-- ~/.cargo/bin (where `cargo install grund` lands) are on PATH even when
-- WezTerm was launched from a desktop entry whose environment has neither.
--
-- A Flatpak-packaged WezTerm adds one more indirection: the sandbox's PATH is
-- /app/bin:/usr/bin, so neither grund-open nor grund exists inside it. Its
-- session-bus talk permission includes org.freedesktop.Flatpak, so the spawn
-- is handed back to the host through flatpak-spawn --host instead
-- (§FS-integrations.3.1).
--
-- But only when *we* are the one spawning, from inside the sandbox. A pane
-- command is spawned by WezTerm itself, and the Flatpak build already sends
-- those to the host — it is how your shell gets started, via
-- `flatpak-spawn --host --watch-bus --directory=...`. Wrapping such an argv a
-- second time runs flatpak-spawn on the host, where it has no portal to talk
-- to: it exits 127 with no output, the split pane closes before anything is
-- drawn, and a peek reads as a flicker. Hence from_sandbox.
function grund_resolver_argv(cwd, peek, citation, from_sandbox)
  local resolver = peek and 'grund-open --peek' or 'grund-open'
  local argv = {
    'sh',
    '-c',
    'PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; cd "$1" && exec ' .. resolver .. ' "$2"',
    'grund-open',
    cwd or '.',
    citation,
  }
  if from_sandbox and os.getenv 'FLATPAK_ID' then
    local wrapped = { 'flatpak-spawn', '--host' }
    for _, arg in ipairs(argv) do
      wrapped[#wrapped + 1] = arg
    end
    return wrapped
  end
  return argv
end

-- Hand one citation to grund-open, in whichever mode was asked for. Every
-- gesture — click, ctrl+shift-click, and both keyboard pickers — ends here, so
-- there is exactly one place where a citation becomes a resolver invocation and
-- the paths cannot drift apart.
function grund_resolve(window, pane, peek, citation)
  local cwd = grund_pane_cwd(pane)
  -- Say so rather than resolving from an arbitrary directory and failing into a
  -- stderr nobody reads: with no cwd the climb starts wherever this process
  -- happens to stand, and the gesture does nothing for no stated reason.
  if not cwd then
    local why = 'grund: this pane reports no working directory, so '
      .. citation
      .. ' cannot be resolved. Have your shell emit OSC 7 (WezTerm shell integration).'
    wezterm.log_error(why)
    pcall(function()
      window:toast_notification('grund', why, nil, 6000)
    end)
  end
  if peek then
    -- WezTerm spawns this one, the same way it spawns your shell.
    window:perform_action(
      wezterm.action.SplitPane {
        direction = 'Right',
        size = { Percent = 45 },
        command = { args = grund_resolver_argv(cwd, true, citation, false) },
      },
      pane
    )
  else
    -- This one we spawn ourselves, from wherever WezTerm is running.
    wezterm.background_child_process(grund_resolver_argv(cwd, false, citation, true))
  end
end

-- Resolve a grund: URI, which is what a click arrives as.
wezterm.on('open-uri', function(window, pane, uri)
  local citation = uri:match '^grund:(.+)$'
  if citation then
    local peek = grund_peek_requested
    grund_peek_requested = false
    grund_resolve(window, pane, peek, citation)
    return false -- handled; don't let WezTerm open it as a normal URL
  end
end)
