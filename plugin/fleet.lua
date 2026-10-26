-- Omsi-Hub's plugin: the company's buses in the game. For a company's duty Omsi-Hub writes the
-- trips of the map's timetable the company's own buses run today, each with how late the
-- company's day has it (data/fleet.json: crates/hub-game/src/bridge.rs `Fleet`, worked out by
-- crates/hub-api/src/company/fleetgame.rs). This gives them to the game
-- (`omsi.timetable.set_ai_trips`): its AI runs those trips and no others - no bus of a line the
-- company does not run, none of a tour it dropped - each as late as the fleet map has it.
--
-- An openOMSI without `set_ai_trips`: the other timetable buses are taken off the road as they
-- come (`traffic.remove`), and the company's run on time. No file, another map or a drive of the
-- player's own: the map's whole timetable, as ever.

local M = {}

-- the file is looked at this often (s); Omsi-Hub writes it when the day's delays moved on
local EVERY = 5
-- a departure is the one asked for this close either side (s)
local NEAR = 45
local DAY = 86400

local wait = 0
local has = {}
-- the file's text last given to the game, and whether the game runs a list of ours
local seen
local given = false
-- (an openOMSI without set_ai_trips) the trips the company runs: "line|tour" -> departures
-- (true: every trip of the tour)
local allowed

M.NEEDS = { "files.read", "json.decode", "timetable.set_ai_trips", "timetable.buses", "traffic.remove", "map.info" }

local function fn(name)
  local t = omsi
  for part in name:gmatch("[^%.]+") do
    if type(t) ~= "table" then return nil end
    t = t[part]
  end
  if type(t) == "function" then return t end
  return nil
end

function M.setup(functions)
  has = {}
  for k, v in pairs(functions or {}) do has[k] = v end
  for _, n in ipairs(M.NEEDS) do has[n] = fn(n) ~= nil end
end

-- whether this openOMSI can keep the company's buses alone on the road
function M.can()
  return has["files.read"] == true and has["json.decode"] == true
    and (has["timetable.set_ai_trips"] or (has["timetable.buses"] and has["traffic.remove"])) == true
end

local function key(line, tour)
  return tostring(line or ""):match("^%s*(.-)%s*$"):lower() .. "|" .. tostring(tour or ""):match("^%s*(.-)%s*$"):lower()
end

local function norm(path)
  return tostring(path or ""):gsub("\\", "/"):gsub("^%./", ""):match("^%s*(.-)%s*$"):lower()
end

-- the same map, however the game and the company name it (a global.cfg with or without the
-- game's folder before it)
local function same_map(game, company)
  local g, c = norm(game), norm(company)
  if g == "" or c == "" or g == c then return true end
  return g:sub(-#c - 1) == "/" .. c or c:sub(-#g - 1) == "/" .. g
end

-- the company's trips in the file's text (nil: none for this game)
local function read(text)
  if not text or text == "" then return nil end
  local ok, f = pcall(omsi.json.decode, text)
  if not ok or type(f) ~= "table" or type(f.trips) ~= "table" then return nil end
  if has["map.info"] then
    local m = omsi.map.info()
    if type(m) == "table" and not same_map(m.path, f.map) then return nil end
  end
  local trips = {}
  for _, t in ipairs(f.trips) do
    if type(t) == "table" and t.line and t.tour then
      trips[#trips + 1] = {
        line = tostring(t.line), tour = tostring(t.tour),
        departure = type(t.departure) == "number" and t.departure or nil,
        delay = type(t.delay) == "number" and t.delay or 0,
      }
    end
  end
  return trips
end

-- the game told which trips its AI runs (nil: the whole timetable again)
local function give(trips)
  if has["timetable.set_ai_trips"] then
    if trips or given then
      omsi.timetable.set_ai_trips(trips)
      given = trips ~= nil
    end
    return
  end
  allowed = nil
  if not trips then return end
  allowed = {}
  for _, t in ipairs(trips) do
    local k = key(t.line, t.tour)
    if t.departure == nil then
      allowed[k] = true
    elseif allowed[k] ~= true then
      allowed[k] = allowed[k] or {}
      table.insert(allowed[k], t.departure)
    end
  end
end

-- (an openOMSI without set_ai_trips) whether the timetable bus `b` runs a trip of the company's
local function runs(b)
  local a = allowed[key(b.line, b.tour)]
  if a == true then return true end
  if not a or type(b.departure) ~= "number" then return a ~= nil end
  for _, d in ipairs(a) do
    local dt = (b.departure - d) % DAY
    if dt < NEAR or dt > DAY - NEAR then return true end
  end
  return false
end

-- (an openOMSI without set_ai_trips) the other timetable buses off the road
local function sweep()
  for _, b in ipairs(omsi.timetable.buses() or {}) do
    if type(b) == "table" and b.id ~= nil and not runs(b) then omsi.traffic.remove(b.id) end
  end
end

-- every second: the file looked at every few seconds; `company`: the game drives a company's
-- duty of Omsi-Hub's (in any other drive the map's whole timetable)
function M.tick(seconds, company)
  if not M.can() then return end
  wait = wait - seconds
  if wait > 0 then return end
  wait = EVERY
  local text = company and omsi.files.read("fleet.json") or nil
  if text ~= seen then
    seen = text
    give(read(text))
  end
  if allowed then sweep() end
end

-- the plugin stops: the game runs its whole timetable again
function M.stop()
  if given and has["timetable.set_ai_trips"] then omsi.timetable.set_ai_trips(nil) end
  given, allowed, seen = false, nil, nil
end

return M
