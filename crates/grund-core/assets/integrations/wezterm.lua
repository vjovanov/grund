-- grund citation integration for WezTerm.
-- Makes a §<ID> citation in the terminal Ctrl/Cmd-clickable: the click resolves
-- the citation with `grund` and opens the declaration in your editor via the
-- installed `grund-open` resolver (§FS-integrations). Paste this into your
-- wezterm.lua, or let `grund integrations wezterm --write` manage it for you.
local wezterm = require 'wezterm'

-- Turn a §<ID>[.<section>] token into a clickable grund: URI.
table.insert(wezterm.default_hyperlink_rules and wezterm.default_hyperlink_rules() or {}, {
  regex = '§[A-Z]+-[a-z0-9][a-z0-9-]*(\\.[0-9]+)*',
  format = 'grund:$0',
})

-- Resolve a grund: URI by handing the citation to grund-open.
wezterm.on('open-uri', function(_window, _pane, uri)
  local citation = uri:match '^grund:(.+)$'
  if citation then
    wezterm.background_child_process { 'grund-open', citation }
    return false -- handled; don't let WezTerm open it as a normal URL
  end
end)
