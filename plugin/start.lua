-- Omsi-Hub's plugin: the duty taken (`omsi.duty.start`) in a call of the plugin's own.
--
-- openOMSI stops a plugin and switches it off for the session when one call into it runs longer
-- than 50 ms ("ran longer than 50 ms and was stopped"), and the time the game spends inside the
-- API counts too: `duty.start` makes the duty's route on the map and the driver's timetable
-- paper, which on a large map takes about that long by itself. So the duty is taken in a timer
-- of its own with nothing else in it, and what follows (the answer, the panel, a card) runs in
-- the next one, with a budget of its own.

local M = {}

-- (a duty asked for and not yet taken: a second one waits for it)
local busy = false

function M.busy()
  return busy
end

-- take line `line`, tour `tour` at trip `trip` and stop `stop` as `omsi.duty.start` does, in
-- the next frame; then `done(ok, why)`, in the frame after
function M.duty(line, tour, trip, stop, done)
  if not omsi.after then
    -- (an openOMSI without timers: at once, as before)
    local ok, why = omsi.duty.start(line, tour, trip, stop)
    if done then done(ok, why) end
    return
  end
  busy = true
  omsi.after(0, function()
    busy = false
    local ok, why = omsi.duty.start(line, tour, trip, stop)
    if done then omsi.after(0, function() done(ok, why) end) end
  end)
end

return M
