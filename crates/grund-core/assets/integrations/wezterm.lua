-- grund citation integration for WezTerm.
-- Makes a §<ID> citation in the terminal Ctrl/Cmd-clickable: the click resolves
-- the citation with `grund` and opens the declaration in your editor via the
-- installed `grund-open` resolver (§FS-integrations).
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
local wezterm = require 'wezterm'

-- Append grund's citation rule to config.hyperlink_rules, seeding WezTerm's
-- defaults first when the config carries none of its own. Every group is
-- non-capturing so the whole citation stays the $0 match.
--
-- The leading [^\w\s]{1,3} matches the citation marker without naming it:
-- `[reference] marker` is per-repo while this file is user-global and installed
-- once, so hardcoding § would leave every repo with a custom marker silently
-- unclickable. grund-open strips whatever punctuation this sweeps in.
function grund_apply_hyperlink_rule(config)
  config.hyperlink_rules = config.hyperlink_rules or wezterm.default_hyperlink_rules()
  table.insert(config.hyperlink_rules, {
    regex = '[^\\w\\s]{1,3}(?:[a-z][a-z0-9-]*/)?[A-Z][A-Z0-9]*-[a-z0-9][a-z0-9-]*(?:\\.[0-9]+)*',
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
end

-- Ctrl-click opens the declaration in your editor; Ctrl-Shift-click peeks at it
-- in a split pane instead (§FS-integrations.3.3). WezTerm has no hover event and
-- no link tooltip (wez/wezterm#4, open since 2018), so peek is the read-without-
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
-- directory instead. Newer WezTerm returns a Url object with a file_path;
-- older versions return a plain "file://host/path" string.
function grund_pane_cwd(pane)
  local cwd = pane:get_current_working_directory()
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
  return cwd.file_path
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
function grund_resolver_argv(cwd, peek, citation)
  local resolver = peek and 'grund-open --peek' or 'grund-open'
  local argv = {
    'sh',
    '-c',
    'PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; cd "$1" && exec ' .. resolver .. ' "$2"',
    'grund-open',
    cwd or '.',
    citation,
  }
  if os.getenv 'FLATPAK_ID' then
    local wrapped = { 'flatpak-spawn', '--host' }
    for _, arg in ipairs(argv) do
      wrapped[#wrapped + 1] = arg
    end
    return wrapped
  end
  return argv
end

-- Resolve a grund: URI by handing the citation to grund-open.
wezterm.on('open-uri', function(window, pane, uri)
  local citation = uri:match '^grund:(.+)$'
  if citation then
    local cwd = grund_pane_cwd(pane)
    if grund_peek_requested then
      grund_peek_requested = false
      window:perform_action(
        wezterm.action.SplitPane {
          direction = 'Right',
          size = { Percent = 45 },
          command = { args = grund_resolver_argv(cwd, true, citation) },
        },
        pane
      )
    else
      wezterm.background_child_process(grund_resolver_argv(cwd, false, citation))
    end
    return false -- handled; don't let WezTerm open it as a normal URL
  end
end)
