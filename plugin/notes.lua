-- Omsi-Hub's plugin: its notifications - the company's notices, service control's messages, the
-- drive watch's offences, the duty's parts and reliefs, the on-time alerts - in the navigator
-- (navigator.lua), the newest on top, each with a button that clicks it away; a few at a time,
-- the older ones behind them until those are gone. Each goes by itself after a while, longer
-- than the game's toast would stay (the driver may be looking at the road); one that must be
-- read (`sticky`: an urgent message, an alert) stays until it is clicked away.
--
-- An openOMSI without the navigator's board gets the game's toast, and one without panels a line
-- of text, as before.

local navigator = require("navigator")

local M = {}

-- (shown at a time; kept in all, the oldest going first)
local SHOWN, KEPT = 3, 8
local INK, DIM = "#F5E6C8", "#959DB0"

local list = {}
local seq = 0

local function put()
  navigator.put("notes", M.elements())
end

-- `text` told the driver; `opts`: title, icon, color, seconds (how long the toast would stay),
-- sticky
function M.say(text, opts)
  opts = opts or {}
  text = text or ""
  local title = opts.title or ""
  if navigator.docks() then
    seq = seq + 1
    table.insert(list, 1, {
      id = seq, text = text, title = title, icon = opts.icon or "notifications", color = opts.color or "#F58620",
      left = math.max((tonumber(opts.seconds) or 8) * 3, 20), sticky = opts.sticky == true,
    })
    while #list > KEPT do table.remove(list) end
    put()
    return
  end
  if omsi.ui and omsi.ui.toast then
    omsi.ui.toast(text, { title = title ~= "" and title or nil, icon = opts.icon, color = opts.color, seconds = opts.seconds })
  else
    omsi.message((title ~= "" and (title .. ": ") or "") .. text, math.min(tonumber(opts.seconds) or 8, 10))
  end
end

-- the notifications' elements in the navigator (none: nothing to tell)
function M.elements()
  local out = {}
  for k = 1, math.min(SHOWN, #list) do
    local n = list[k]
    local head = {
      { type = "icon", name = n.icon, size = 16, color = n.color },
      { type = "text", text = n.title ~= "" and n.title or n.text, size = 12.5, weight = "bold", color = INK, grow = true, wrap = false },
      { type = "button", id = "note_close_" .. n.id, icon = "close", text = "" },
    }
    local box = { { type = "row", gap = 8, children = head } }
    if n.title ~= "" and n.text ~= "" then box[2] = { type = "text", text = n.text, size = 12, color = INK } end
    out[#out + 1] = { type = "box", color = n.color, children = box }
  end
  if #list > SHOWN then
    out[#out + 1] = { type = "text", text = "+" .. (#list - SHOWN), size = 11.5, weight = "medium", color = DIM, align = "right" }
  end
  return out
end

-- every second: the ones whose time is up go
function M.tick(dt)
  local gone = false
  for k = #list, 1, -1 do
    local n = list[k]
    -- (only those the driver sees run their time; those behind wait their turn)
    if k <= SHOWN and not n.sticky then
      n.left = n.left - (dt or 1)
      if n.left <= 0 then
        table.remove(list, k)
        gone = true
      end
    end
  end
  if gone then put() end
end

-- a click in the navigator: whether it was a notification's
function M.click(element)
  local id = type(element) == "string" and tonumber(element:match("^note_close_(%d+)$"))
  if not id then return false end
  for k, n in ipairs(list) do
    if n.id == id then
      table.remove(list, k)
      break
    end
  end
  put()
  return true
end

-- the notifications now (for the tests and the phone)
function M.list()
  return list
end

return M
