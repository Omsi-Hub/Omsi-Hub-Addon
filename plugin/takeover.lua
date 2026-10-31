-- Omsi-Hub's plugin: a bus of the timetable taken over halfway along its trip - one of the
-- company's buses another driver or the AI drives (the fleet map's "Take over this bus"), or the
-- player's own duty taken back from the AI after his game ended before it was done. The
-- assignment's `take_over` (crates/hub-game/src/bridge.rs `TakeOver`) names the line, the tour,
-- the trip and the stop it goes on from, and where Omsi-Hub's fleet map has the bus now (`at`:
-- x, y and heading in the game's coordinates).
--
-- Once the world is there and the game has the duty, the bus the game's AI drives on that tour is
-- looked for among the timetable's buses for a few seconds: found, it is taken off the road and
-- the player's bus put exactly where it was, facing its way, its trip's next stop the duty's next
-- one. Else the player's bus is put where the fleet map has it, else at the stop the duty goes on
-- from. Everything of the duty is typed in already: he drives on. Each step is a call of its own
-- (openOMSI stops a plugin when one call runs longer than 50 ms).

local M = {}

-- (the AI's bus is looked for this long, s, before the fleet map's place is taken)
local LOOK_FOR = 8

local want
-- "off", "wait" (for the world and the duty), "look" (for the AI's bus), "done"
local state = "off"
local looked = 0
local say = function() end

local function fn(name)
  local t = omsi
  for part in name:gmatch("[^%.]+") do
    if type(t) ~= "table" then return nil end
    t = t[part]
  end
  if type(t) == "function" then return t end
  return nil
end

local function same(a, b)
  return tostring(a or ""):match("^%s*(.-)%s*$"):lower() == tostring(b or ""):match("^%s*(.-)%s*$"):lower()
end

-- `assignment` the drive's; `notify(text, title, icon)` tells the driver; `tr` the texts
function M.setup(assignment, notify)
  want, state, looked = nil, "off", 0
  say = notify or function() end
  if type(assignment) == "table" and type(assignment.take_over) == "table" and (assignment.take_over.line or "") ~= "" then
    want = assignment.take_over
    state = "wait"
  end
end

-- the line and tour taken over, once the player's bus is there (fleet.lua keeps the AI off it)
function M.taken()
  if state ~= "done" then return nil end
  return want.line, want.tour
end

function M.active()
  return state ~= "off"
end

-- run `f` in a call of its own
local function later(f)
  if omsi.after then omsi.after(0, f) else f() end
end

-- the duty's stop of the map's object `id` (from 1), and the stops
local function stop_of(id)
  if id == nil or not fn("duty.stops") then return nil end
  for k, s in ipairs(omsi.duty.stops() or {}) do
    if s.id ~= nil and s.id == id then return k end
  end
  return nil
end

-- the player's bus put at x, y facing `heading`, its next stop `k` (from 1; nil: as it is)
local function put(x, y, heading, k)
  if not (fn("map.teleport") and x and y) then return false end
  local ok = omsi.map.teleport(x, y, nil, heading or 0)
  if ok and k and k > 1 and fn("duty.skip_to") then
    later(function() omsi.duty.skip_to(k) end)
  end
  return ok == true
end

-- the bearing from one point to another, as the game's headings go (0 north, clockwise)
local function bearing(ax, ay, bx, by)
  return (math.deg(math.atan(bx - ax, by - ay)) + 360) % 360
end

-- the stop the duty goes on from, and the one after it (to face its way)
local function at_stop()
  if not fn("duty.stops") then return false end
  local stops = omsi.duty.stops() or {}
  local k = math.max(1, math.floor(tonumber(want.stop) or 1))
  local s, n = stops[k], stops[k + 1]
  if not (s and s.x and s.y) then return false end
  local heading = (n and n.x and n.y) and bearing(s.x, s.y, n.x, n.y) or 0
  return put(s.x, s.y, heading, k)
end

local function finish(how)
  state = "done"
  if how == "bus" then
    say("You took the bus over where it was: drive on, the duty is typed in", "Bus taken over", "sports_esports")
  elseif how == "place" then
    say("Your bus is where the company's bus was: drive on, the duty is typed in", "Bus taken over", "sports_esports")
  elseif how == "stop" then
    say("Your bus waits at the stop the duty goes on from: drive on, the duty is typed in", "Bus taken over", "sports_esports")
  else
    say("The bus could not be put on its way: drive to the duty's next stop", "Bus taken over", "warning")
  end
end

-- the AI's bus of the tour taken over: off the road, the player's bus where it was
local function take(b)
  local heading = b.heading
  if fn("traffic.get") and b.id ~= nil then
    local v = omsi.traffic.get(b.id)
    if type(v) == "table" and v.heading then heading = v.heading end
  end
  local x, y, next_id = b.x, b.y, b.next_stop_id
  later(function()
    if fn("traffic.remove") and b.id ~= nil then omsi.traffic.remove(b.id) end
    later(function()
      local ok = put(x, y, heading, stop_of(next_id) or tonumber(want.stop))
      finish(ok and "bus" or (at_stop() and "stop" or nil))
    end)
  end)
end

-- every frame: `dt` seconds since the last
function M.frame(dt)
  if state == "off" or state == "done" or state == "busy" then return end
  if fn("has_vehicle") and not omsi.has_vehicle() then return end
  if state == "wait" then
    local d = fn("duty.get") and omsi.duty.get() or nil
    if type(d) == "table" and same(d.line, want.line) and same(d.tour, want.tour) then
      state, looked = "look", 0
    end
    return
  end
  looked = looked + (dt or 0)
  if fn("timetable.buses") then
    for _, b in ipairs(omsi.timetable.buses() or {}) do
      if type(b) == "table" and b.x and b.y and same(b.line, want.line) and same(b.tour, want.tour) then
        state = "busy"
        take(b)
        return
      end
    end
  end
  if looked < LOOK_FOR then return end
  state = "busy"
  later(function()
    local at = type(want.at) == "table" and want.at or nil
    if at and put(tonumber(at[1]), tonumber(at[2]), tonumber(at[3]), tonumber(want.stop)) then
      finish("place")
    else
      finish(at_stop() and "stop" or nil)
    end
  end)
end

return M
