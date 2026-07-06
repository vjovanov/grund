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
-- defaults first when the config carries none of its own. A non-capturing group
-- keeps the whole citation as the $0 match.
function grund_apply_hyperlink_rule(config)
  config.hyperlink_rules = config.hyperlink_rules or wezterm.default_hyperlink_rules()
  table.insert(config.hyperlink_rules, {
    regex = '§[A-Z]+-[a-z0-9][a-z0-9-]*(?:\\.[0-9]+)*',
    format = 'grund:$0',
  })
end

-- Resolve a grund: URI by handing the citation to grund-open.
wezterm.on('open-uri', function(_window, _pane, uri)
  local citation = uri:match '^grund:(.+)$'
  if citation then
    wezterm.background_child_process { 'grund-open', citation }
    return false -- handled; don't let WezTerm open it as a normal URL
  end
end)
