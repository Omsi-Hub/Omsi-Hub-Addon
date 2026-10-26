-- Omsi-Hub's plugin: signing on, as Omsi-Hub's own game had it in the navigator
-- (crates/omsi-app/src/nav_signon.rs) - the driver signs on with a personnel number and a code,
-- signs the duty order, and only then has the duty and its codes in the IBIS. The number and the
-- code are the driver's pass, which Omsi-Hub keeps (`~/.openomsi/personnel.json`) and gives in
-- the assignment; the company's panel (main.lua) shows the stage this module says.
--
-- The stages: "signon" (the number and the code asked), "order" (the duty order to accept),
-- "work" (signed on, the duty accepted). Without a pass in the assignment the driver is at work
-- at once, as with sign-on switched off in Omsi-Hub.

local M = {}

-- the trips the duty order lists; the rest are counted under them
local LISTED = 6
-- Omsi-Hub's colours (the launcher's dark theme), as the board has them
local PLATE, ON_PLATE = "#FFD23F", "#1A1400"
local TEXT, DIM, FAINT = "#E8EBF2", "#959DB0", "#6E7688"

local tr = function(t) return t end
local pass = nil
local typed = { number = "", code = "" }
local wrong = false

M.stage = "work"

function M.setup(translate, assignment)
  tr = translate
  local p = assignment and assignment.personnel
  if type(p) == "table" and (p.number or "") ~= "" and (p.code or "") ~= "" then
    pass = { number = tostring(p.number), code = tostring(p.code) }
    M.stage = "signon"
  end
end

local function hhmm(t)
  local m = math.floor((t or 0) / 60)
  return string.format("%02d:%02d", (m // 60) % 24, m % 60)
end

local function trips_of()
  return (omsi.duty and omsi.duty.trips) and omsi.duty.trips() or {}
end

-- (the line as the trip's displays show it; "5 Rathaus" is the line's terminus Rathaus)
local function terminus_of(line, terminus)
  terminus = terminus or ""
  if line ~= "" and terminus:sub(1, #line + 1) == line .. " " then terminus = terminus:sub(#line + 2) end
  return terminus
end

-- the lines the duty's trips show on their displays, each once, in the order they come
local function shown_lines(trips)
  local out, seen = {}, {}
  for _, t in ipairs(trips) do
    local l = t.line or ""
    if l ~= "" and not seen[l] then
      seen[l] = true
      out[#out + 1] = l
    end
  end
  return out
end

local function plate(line)
  return { type = "badge", text = line, color = PLATE, text_color = ON_PLATE, weight = "bold" }
end

-- the duty as the company's panel heads it: the plates of its lines, its tour, from when to when
-- (`duty` the assignment; without its trips the line as Omsi-Hub named it)
function M.heading(duty)
  local trips = trips_of()
  local lines = shown_lines(trips)
  if #lines == 0 and (duty.line or "") ~= "" then lines = { duty.line } end
  local row = { type = "row", gap = 6, children = {} }
  for k, l in ipairs(lines) do
    if k > 3 then break end
    row.children[#row.children + 1] = plate(l)
  end
  row.children[#row.children + 1] = { type = "text", text = tr("Tour %{tour}", { tour = duty.tour or "" }), size = 13, weight = "bold", color = TEXT, grow = true, wrap = false }
  if #trips > 0 then
    row.children[#row.children + 1] = { type = "text", text = hhmm(trips[1].departure) .. " – " .. hhmm(trips[#trips].arrival), size = 12, weight = "medium", color = DIM, wrap = false }
  end
  return row
end

-- the elements of the stage, for the company's panel; `duty` the assignment
function M.children(duty)
  if M.stage == "signon" then
    return {
      { type = "text", text = tr("Not signed on"), weight = "bold" },
      { type = "input", id = "signon_number", text = typed.number, placeholder = tr("Personnel number"), max = 10 },
      { type = "input", id = "signon_code", text = typed.code, placeholder = tr("Code"), max = 8 },
      { type = "text", text = tr("Not known here. Your number and code are on your pass below."), size = 12, color = "#FF5F57", visible = wrong },
      { type = "button", id = "signon", text = tr("Sign on"), icon = "badge" },
      { type = "text", text = tr("Your pass") .. ": " .. tr("Personnel no. %{number} · code %{code}", { number = pass.number, code = pass.code }), size = 12, color = "#9AA3AD" },
    }
  end
  if M.stage == "order" then
    -- (the first trips, each when it leaves and where it goes - the line's plate too when the
    -- duty drives more than one; the others counted under them)
    local trips = trips_of()
    local several = #shown_lines(trips) > 1
    local rows = {}
    for k, t in ipairs(trips) do
      if k > LISTED then break end
      local line = t.line or ""
      rows[#rows + 1] = several and { hhmm(t.departure), line, terminus_of(line, t.terminus) } or { hhmm(t.departure), terminus_of(line, t.terminus) }
    end
    return {
      { type = "row", gap = 6, children = {
        { type = "text", text = tr("Duty assignment"), size = 11, weight = "bold", color = DIM, grow = true, wrap = false },
        { type = "text", text = tr("%{n} trips", { n = #trips }), size = 11, weight = "medium", color = FAINT, wrap = false, visible = #trips > 0 },
      } },
      { type = "table", columns = {}, widths = several and { 0.2, 0.18, 0.62 } or { 0.2, 0.8 }, rows = rows, size = 12.5, visible = #rows > 0 },
      { type = "text", size = 11.5, weight = "medium", color = FAINT, visible = #trips > LISTED,
        text = tr("and %{n} more trips until %{time}", { n = #trips - LISTED, time = hhmm(#trips > 0 and trips[#trips].arrival or 0) }) },
      { type = "text", text = tr("The duty's codes go to the IBIS as soon as you accept it."), size = 11.5, color = DIM },
      { type = "button", id = "accept", text = tr("Accept duty"), icon = "task_alt" },
    }
  end
  return {}
end

-- what was typed into a field
function M.change(element, value)
  if element == "signon_number" then typed.number = tostring(value or "") return true end
  if element == "signon_code" then typed.code = tostring(value or "") return true end
  return false
end

-- a click (or Enter in a field) of the panel: `accepted` is called when the duty is accepted.
-- Whether it was this module's.
function M.click(element, accepted)
  if element == "signon" or element == "signon_number" or element == "signon_code" then
    if M.stage ~= "signon" then return true end
    if typed.number == pass.number and typed.code == pass.code then
      M.stage, wrong = "order", false
    elseif element == "signon" or (typed.number ~= "" and typed.code ~= "") then
      wrong = true
    end
    return true
  end
  if element == "accept" and M.stage == "order" then
    M.stage = "work"
    accepted()
    return true
  end
  if element == "signoff" then
    if pass then M.stage, typed = "signon", { number = "", code = "" } end
    return true
  end
  return false
end

-- signing on from the phone (companion.lua): the number and the code at once; whether the
-- driver is past them (signed on, or no sign-on asked)
function M.try(number, code)
  if M.stage ~= "signon" or not pass then return true end
  typed = { number = tostring(number or ""), code = tostring(code or "") }
  M.click("signon", function() end)
  return M.stage ~= "signon"
end

-- the last number and code were not the driver's
function M.wrong()
  return wrong
end

-- the driver drives without the duty (the phone's "Drive without duty"): at work, no order
function M.skip()
  M.stage, wrong = "work", false
end

-- the driver is at work (signed on, or no sign-on asked)
function M.at_work()
  return M.stage == "work"
end

function M.signed()
  return pass ~= nil
end

return M
