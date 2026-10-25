-- Omsi-Hub's plugin: everything it shows while driving, in the navigator (Luc, 2026-10-25) - the
-- notifications, the control room's messages, the company's panel and signing on, the duty
-- board with the company's parts, the on-time card and the duty strip, one under the other in
-- the navigator's board under its map. The navigator is the one place the driver looks: it is
-- moved and sized as the navigator is (dragged by its map, sized at its corner, Shift+N), and
-- the panel takes its clicks without F10 (`grab`), so a notification is clicked away with the
-- mouse as it is; only typing into a field (the personnel number) still asks for F10.
--
-- Each part puts its elements here (`put`, nil: it has nothing to show) and the navigator's
-- panel is set again once a few times a second (`flush`). An openOMSI without the navigator's
-- board (`omsi.ui.version` before 3) has each part as a panel of its own, as before (`docks`).

local layout = require("layout")

local M = {}

M.PANEL = "omsihub_board"
-- the parts, top to bottom
local ORDER = { "notes", "messages", "company", "board", "ontime", "strip" }
-- (openOMSI takes 200 elements in a panel: a part that would go over is left out)
local MOST = 195

local parts = {}
local dirty = false
local shown = false

-- whether the parts go into the navigator (an openOMSI with its board)
function M.docks()
  return omsi.ui ~= nil and type(omsi.ui.set) == "function" and (tonumber(omsi.ui.version) or 0) >= 3
end

-- the part `name`'s elements now (nil or none: it has nothing to show)
function M.put(name, children)
  if children ~= nil and #children == 0 then children = nil end
  if parts[name] == children then return end
  parts[name] = children
  dirty = true
end

-- whether this panel's click or change is the navigator's (the parts' own ids tell them apart)
function M.is(panel)
  return panel == M.PANEL
end

local function count(list)
  local n = 0
  for _, e in ipairs(list or {}) do
    n = n + 1
    if type(e) == "table" and type(e.children) == "table" then n = n + count(e.children) end
  end
  return n
end

-- the navigator's panel set again, when a part changed
function M.flush(force)
  if not (dirty or force) or not M.docks() then return end
  dirty = false
  local children, n = {}, 0
  for _, name in ipairs(ORDER) do
    local list = parts[name]
    local k = list and count(list) or 0
    if k > 0 and n + k + 1 <= MOST then
      if #children > 0 then
        children[#children + 1] = { type = "divider" }
        n = n + 1
      end
      for _, e in ipairs(list) do children[#children + 1] = e end
      n = n + k
    end
  end
  if #children == 0 then
    if shown then layout.remove(M.PANEL) end
    shown = false
    return
  end
  layout.set(M.PANEL, { anchor = "navigator", padding = 11, gap = 5, grab = true, children = children })
  shown = true
end

function M.stop()
  for name in pairs(parts) do parts[name] = nil end
  if shown then layout.remove(M.PANEL) end
  shown, dirty = false, false
end

return M
