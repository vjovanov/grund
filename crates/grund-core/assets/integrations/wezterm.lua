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
    regex = '[^\\w\\s]{1,3}(?:[a-z][a-z0-9-]*/)?[A-Z]+-[a-z0-9][a-z0-9-]*(?:\\.[0-9]+)*',
    format = 'grund:$0',
  })
  -- Adding the peek binding here rather than in the scaffold means an existing
  -- config that already calls this function gets peek without re-wiring.
  -- WezTerm's own defaults still apply to anything not listed here.
  config.mouse_bindings = config.mouse_bindings or {}
  table.insert(config.mouse_bindings, grund_peek_mouse_binding())
end

-- Ctrl-click opens the declaration in your editor; Ctrl-Shift-click peeks at it
-- in a split pane instead (§FS-integrations.3.3). WezTerm has no hover event and
-- no link tooltip (wez/wezterm#4, open since 2018), so peek is the read-without-
-- leaving path here.
--
-- Lua cannot ask which link is under the mouse — only the built-in
-- OpenLinkAtMouseCursor knows that, and all it exposes is the resulting URI via
-- open-uri. window:current_event() does not carry modifiers either. So the
-- binding records the intent and delegates; the handler below reads it back.
-- This is safe because both run in one synchronous event, not across ticks.
grund_peek_requested = false

function grund_peek_mouse_binding()
  return {
    event = { Up = { streak = 1, button = 'Left' } },
    mods = 'CTRL|SHIFT',
    action = wezterm.action_callback(function(window, pane)
      grund_peek_requested = true
      window:perform_action(wezterm.action.OpenLinkAtMouseCursor, pane)
      -- Cleared here too: a Ctrl-Shift-click on non-link text never reaches
      -- open-uri, and a stale flag would turn the next plain click into a peek.
      grund_peek_requested = false
    end),
  }
end

-- Resolve a grund: URI by handing the citation to grund-open.
wezterm.on('open-uri', function(window, pane, uri)
  local citation = uri:match '^grund:(.+)$'
  if citation then
    if grund_peek_requested then
      grund_peek_requested = false
      window:perform_action(
        wezterm.action.SplitPane {
          direction = 'Right',
          size = { Percent = 45 },
          -- argv, never shell source: the citation is arbitrary screen text.
          command = { args = { 'grund-open', '--peek', citation } },
        },
        pane
      )
    else
      wezterm.background_child_process { 'grund-open', citation }
    end
    return false -- handled; don't let WezTerm open it as a normal URL
  end
end)
