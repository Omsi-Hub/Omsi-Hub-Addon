//! Omsi-Hub's plugin (`plugin/*.lua`) run without openOMSI: a Lua 5.4 state (mlua) with a mock
//! of the official plugin API's functions the plugin uses (INVENTORY §4), and a fake game
//! behind them ([`GameState`]) that a test sets up and looks at.
//!
//! The mock is written from what the plugin calls and what the API's documentation says of
//! each function; it is not openOMSI's runtime. Its rules:
//!
//! * `omsi.on(event, fn)` and `omsi.every(seconds, fn)` keep the handlers; [`Harness::frame`]
//!   is one second of game: it fires `vehicle` and `duty` when they changed, then `frame`,
//!   then the timers that are due. [`Harness::fire`] fires any other event (`trip_done` …);
//! * `omsi.files.*` read and write the plugin's own `data/` folder;
//! * `omsi.json.*` is serde_json; `omsi.log` lines go to [`GameState::log`] as
//!   `[lua omsihub] <text>`, as the game's log has them;
//! * `omsi.duty.get()` is [`GameState::duty`] (its `trip` counted from 1, as the plugin
//!   counts); `omsi.duty.start` records the start and makes it the duty;
//! * `omsi.position()` is [`GameState::position`], `omsi.traffic.list()`,
//!   `omsi.timetable.buses()` and `omsi.others()` are [`GameState::traffic`],
//!   [`GameState::timetable_buses`] and [`GameState::others`] as JSON; `omsi.map.tile_at`
//!   puts the map's coordinates on tiles of [`GameState::tile`] metres from the origin;
//!   `omsi.timetable.set_ai_trips` and `omsi.traffic.remove` are kept in
//!   [`GameState::ai_trips`] and [`GameState::removed`] (a bus removed leaves the timetable's);
//! * the bus's triggers, script variables and strings, tickets and sales are
//!   [`GameState::triggers`], [`GameState::vars`], [`GameState::strs`], [`GameState::tickets`]
//!   and [`GameState::sales`]; `omsi.trigger`, `omsi.bus.action` and `omsi.bus.set_line` are
//!   kept in [`GameState::pressed`], [`GameState::bus_actions`] and [`GameState::lines_typed`],
//!   `omsi.press` and `omsi.release` in [`GameState::held`]; `omsi.bus.get_vars` and
//!   `omsi.bus.get_strings` give only the names the bus has, as the API does.
//!
//! A test can take functions away before the plugin runs ([`Harness::load_with`]), as an older
//! openOMSI would not have them.

use mlua::{Function, IntoLuaMulti, Lua, LuaSerdeExt, Table, Value as LuaValue};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The duty the game has.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Duty {
    pub line: String,
    pub tour: String,
    /// The trip under way, 1 = the first.
    pub trip: i64,
    pub trips: i64,
    /// Seconds late (negative: early).
    pub delay: f64,
}

/// A stop of a duty's trip.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stop {
    pub id: i64,
    pub name: String,
    pub arr: f64,
    pub dep: f64,
}

/// A trip of the duty.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trip {
    pub name: String,
    pub line: String,
    pub terminus: String,
    pub departure: f64,
    pub arrival: f64,
    pub stops: Vec<Stop>,
}

/// A card shown (`omsi.ui.toast`).
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub text: String,
    pub title: String,
}

/// The fake game.
#[derive(Debug, Clone)]
pub struct GameState {
    pub version: String,
    pub api: i64,
    pub platform: String,
    pub language: String,
    /// The clock, seconds of the day; and the Unix time `omsi.util.now` says.
    pub clock: f64,
    pub now: f64,
    /// The bus the player is in (its name), and its file.
    pub vehicle: Option<String>,
    pub bus_file: String,
    pub map: String,
    /// km/h; the passengers on board (where the bus is: `position`).
    pub speed: f64,
    pub passengers: i64,
    pub duty: Option<Duty>,
    pub trips: Vec<Trip>,
    /// What the plugin did: duties started (line, tour, trip, stop), lines of text, log lines,
    /// cards, panels (by id, as JSON), game actions, display fonts, UDP sends.
    pub started: Vec<(String, String, i64, i64)>,
    pub messages: Vec<String>,
    pub log: Vec<String>,
    pub toasts: Vec<Toast>,
    pub panels: BTreeMap<String, Value>,
    /// How far the player dragged each panel (`omsi.ui.moved`); a panel set again keeps it,
    /// one removed loses it.
    pub moved: BTreeMap<String, (f64, f64)>,
    pub actions: Vec<String>,
    pub fonts: Vec<String>,
    pub sent: Vec<(i64, String)>,
    pub ibis: Vec<String>,
    /// Where the player's bus is (x, y, heading; None on foot), the AI vehicles, the
    /// timetable's buses and the other vehicles round the bus, as the API gives them; a tile's
    /// edge.
    pub position: Option<(f64, f64, f64)>,
    pub traffic: Vec<Value>,
    pub timetable_buses: Vec<Value>,
    pub others: Vec<Value>,
    pub tile: f64,
    /// The trips given to the timetable's AI (`omsi.timetable.set_ai_trips`, each call; `null`:
    /// the whole timetable again) and the AI vehicles taken off the road (`omsi.traffic.remove`).
    pub ai_trips: Vec<Value>,
    pub removed: Vec<i64>,
    /// The bus's script triggers (`omsi.bus.triggers`) and the ones fired (`omsi.trigger`),
    /// its key actions done (`omsi.bus.action`) and the lines typed (`omsi.bus.set_line`).
    pub triggers: Vec<String>,
    pub pressed: Vec<String>,
    /// The keys held down (`omsi.press`: true) and let go (`omsi.release`: false), in order.
    pub held: Vec<(String, bool)>,
    pub bus_actions: Vec<String>,
    pub lines_typed: Vec<String>,
    /// Its script variables and strings (`omsi.bus.get_vars`, `omsi.vars`, `omsi.str`).
    pub vars: BTreeMap<String, f64>,
    pub strs: BTreeMap<String, String>,
    /// The map's tickets (name, price), the one asked for at the desk, the sales so far.
    pub tickets: Vec<(String, f64)>,
    pub ticket_request: Option<(String, f64)>,
    pub sales: (i64, f64),
    /// The map's lines as `omsi.timetable.lines` gives them, and the name of every lane
    /// (`omsi.map.lane`).
    pub lines: Value,
    pub lane: String,
}

impl Default for GameState {
    fn default() -> GameState {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as f64).unwrap_or(0.0);
        GameState {
            version: "0.2.1500".into(),
            api: 1,
            platform: "windows".into(),
            language: "en".into(),
            clock: 28_800.0,
            now,
            vehicle: Some("MAN SD202".into()),
            bus_file: "Vehicles/MAN_SD202/SD202.bus".into(),
            map: "maps/Grundorf/global.cfg".into(),
            speed: 0.0,
            passengers: 0,
            duty: None,
            trips: Vec::new(),
            started: Vec::new(),
            messages: Vec::new(),
            log: Vec::new(),
            toasts: Vec::new(),
            panels: BTreeMap::new(),
            moved: BTreeMap::new(),
            actions: Vec::new(),
            fonts: Vec::new(),
            sent: Vec::new(),
            ibis: Vec::new(),
            position: None,
            traffic: Vec::new(),
            timetable_buses: Vec::new(),
            others: Vec::new(),
            tile: 300.0,
            ai_trips: Vec::new(),
            removed: Vec::new(),
            triggers: Vec::new(),
            pressed: Vec::new(),
            held: Vec::new(),
            bus_actions: Vec::new(),
            lines_typed: Vec::new(),
            vars: BTreeMap::new(),
            strs: BTreeMap::new(),
            tickets: Vec::new(),
            ticket_request: None,
            sales: (0, 0.0),
            lines: json!([]),
            lane: String::new(),
        }
    }
}

/// Every function the mock offers, as `omsi.game.has` names them.
pub const FUNCTIONS: &[&str] = &[
    "on",
    "every",
    "log",
    "message",
    "info",
    "has_vehicle",
    "speed",
    "send",
    "util.now",
    "files.read",
    "files.write",
    "files.append",
    "json.encode",
    "json.decode",
    "ui.set",
    "ui.toast",
    "ui.remove",
    "ui.update",
    "ui.focus",
    "ui.focused",
    "ui.moved",
    "duty.get",
    "duty.trips",
    "duty.stops",
    "duty.trip_stops",
    "duty.start",
    "duty.finish",
    "duty.skip_to",
    "duty.set_auto_ibis",
    "duty.type_ibis",
    "duty.set_closed_stops",
    "bus.file",
    "bus.passengers",
    "bus.set_display_font",
    "game.version",
    "game.api",
    "game.platform",
    "game.settings",
    "game.action",
    "game.has",
    "map.info",
    "map.tile_at",
    "audio.play",
    "position",
    "others",
    "traffic.list",
    "traffic.remove",
    "timetable.buses",
    "timetable.lines",
    "timetable.set_ai_trips",
    "trigger",
    "press",
    "release",
    "vars",
    "str",
    "vehicle",
    "bus.triggers",
    "bus.tickets",
    "bus.ticket_request",
    "bus.sales",
    "bus.get_vars",
    "bus.get_strings",
    "bus.action",
    "bus.set_line",
    "map.lane",
];

/// The handlers and timers, in Lua (the harness's own).
const PRELUDE: &str = r#"
local handlers, timers = {}, {}
function omsi.on(event, fn)
  handlers[event] = handlers[event] or {}
  table.insert(handlers[event], fn)
end
function omsi.every(seconds, fn)
  table.insert(timers, { every = seconds, left = seconds, fn = fn })
end
function omsi.after(seconds, fn)
  table.insert(timers, { left = seconds, fn = fn })
end
function omsi._fire(event, ...)
  for _, fn in ipairs(handlers[event] or {}) do fn(...) end
end
function omsi._tick(dt)
  local due = {}
  for _, t in ipairs(timers) do
    t.left = t.left - dt
    if t.left <= 1e-9 then
      t.left = t.left + (t.every or 0)
      due[#due + 1] = t
    end
  end
  local keep = {}
  for _, t in ipairs(timers) do
    if t.every then keep[#keep + 1] = t end
  end
  timers = keep
  for _, t in ipairs(due) do t.fn() end
end
"#;

/// The plugin running on the mock.
pub struct Harness {
    pub lua: Lua,
    pub state: Rc<RefCell<GameState>>,
    pub plugin_dir: PathBuf,
    seen_vehicle: Option<String>,
    seen_duty: Option<(String, String)>,
}

fn to_lua(lua: &Lua, v: &Value) -> mlua::Result<LuaValue> {
    lua.to_value_with(v, mlua::serde::SerializeOptions::new().serialize_none_to_null(false).serialize_unit_to_null(false))
}

fn from_lua(lua: &Lua, v: LuaValue) -> mlua::Result<Value> {
    lua.from_value_with(v, mlua::serde::DeserializeOptions::new().deny_unsupported_types(false))
}

fn stops_json(stops: &[Stop]) -> Value {
    Value::Array(stops.iter().enumerate().map(|(k, s)| json!({ "id": s.id, "number": k + 1, "name": s.name, "arr": s.arr, "dep": s.dep, "stops": true })).collect())
}

impl Harness {
    /// The plugin in `plugin_dir` (its `main.lua` and modules) started on `state`.
    pub fn load(plugin_dir: &Path, state: GameState) -> mlua::Result<Harness> {
        Harness::load_with(plugin_dir, state, "")
    }

    /// The same, with `before` (Lua) run ahead of the plugin's code: `omsi.files = nil` takes
    /// the file functions away, as an older openOMSI.
    pub fn load_with(plugin_dir: &Path, state: GameState, before: &str) -> mlua::Result<Harness> {
        let lua = Lua::new();
        let state = Rc::new(RefCell::new(state));
        let package: Table = lua.globals().get("package")?;
        package.set("path", format!("{}/?.lua", plugin_dir.to_string_lossy().replace('\\', "/")))?;
        let omsi = api(&lua, &state, plugin_dir)?;
        lua.globals().set("omsi", omsi)?;
        lua.load(PRELUDE).set_name("harness").exec()?;
        if !before.is_empty() {
            lua.load(before).set_name("before").exec()?;
        }
        let main = std::fs::read_to_string(plugin_dir.join("main.lua")).map_err(mlua::Error::external)?;
        lua.load(main).set_name("main.lua").exec()?;
        Ok(Harness { lua, state, plugin_dir: plugin_dir.to_path_buf(), seen_vehicle: None, seen_duty: None })
    }

    /// An event to the plugin's handlers, its arguments as JSON (`null` is `nil`).
    pub fn fire(&self, event: &str, args: &[Value]) -> mlua::Result<()> {
        let omsi: Table = self.lua.globals().get("omsi")?;
        let fire: Function = omsi.get("_fire")?;
        let mut all = vec![LuaValue::String(self.lua.create_string(event)?)];
        for a in args {
            all.push(to_lua(&self.lua, a)?);
        }
        fire.call::<()>(mlua::MultiValue::from_vec(all))
    }

    /// One second of game.
    pub fn frame(&mut self) -> mlua::Result<()> {
        let vehicle = self.state.borrow().vehicle.clone();
        if vehicle != self.seen_vehicle {
            self.seen_vehicle = vehicle.clone();
            self.fire("vehicle", &[vehicle.map_or(Value::Null, Value::String)])?;
        }
        self.check_duty()?;
        self.fire("frame", &[json!(1.0)])?;
        self.check_duty()?;
        let omsi: Table = self.lua.globals().get("omsi")?;
        omsi.get::<Function>("_tick")?.call::<()>(1.0)?;
        self.check_duty()
    }

    fn check_duty(&mut self) -> mlua::Result<()> {
        let duty = self.state.borrow().duty.as_ref().map(|d| (d.line.clone(), d.tour.clone()));
        if duty != self.seen_duty {
            self.seen_duty = duty.clone();
            match duty {
                Some((l, t)) => self.fire("duty", &[json!(l), json!(t)])?,
                None => self.fire("duty", &[Value::Null, Value::Null])?,
            }
        }
        Ok(())
    }

    /// The game's log, as one text.
    pub fn log_text(&self) -> String {
        self.state.borrow().log.join("\n")
    }
}

/// The `omsi` table: the mock API on `state`.
fn api(lua: &Lua, state: &Rc<RefCell<GameState>>, plugin_dir: &Path) -> mlua::Result<Table> {
    let omsi = lua.create_table()?;
    omsi.set("data", lua.create_table()?)?;
    let data_dir = plugin_dir.join("data");

    macro_rules! f {
        ($t:expr, $name:expr, |$lua:ident, $s:ident, $args:ident : $ty:ty| $body:expr) => {{
            let st = Rc::clone(state);
            $t.set(
                $name,
                lua.create_function(move |$lua, $args: $ty| {
                    #[allow(unused_variables)]
                    let $s = &st;
                    $body
                })?,
            )?;
        }};
    }

    // the top level
    f!(omsi, "log", |_l, s, text: String| {
        s.borrow_mut().log.push(format!("[lua omsihub] {text}"));
        Ok(())
    });
    f!(omsi, "message", |_l, s, a: (String, Option<f64>)| {
        s.borrow_mut().messages.push(a.0);
        Ok(())
    });
    f!(omsi, "info", |l, s, _a: ()| {
        let g = s.borrow();
        let mut v = json!({ "version": g.version, "clock": g.clock });
        if let Some(d) = &g.duty {
            v["line"] = json!(d.line);
            v["tour"] = json!(d.tour);
            v["delay"] = json!(d.delay);
        }
        to_lua(l, &v)
    });
    f!(omsi, "has_vehicle", |_l, s, _a: ()| Ok(s.borrow().vehicle.is_some()));
    f!(omsi, "position", |l, s, _a: ()| match s.borrow().position {
        Some((x, y, h)) => (x, y, 0.0, h).into_lua_multi(l),
        None => Ok(mlua::MultiValue::new()),
    });
    f!(omsi, "others", |l, s, _a: Option<f64>| to_lua(l, &Value::from(s.borrow().others.clone())));
    let traffic = lua.create_table()?;
    f!(traffic, "list", |l, s, _a: Option<f64>| to_lua(l, &Value::from(s.borrow().traffic.clone())));
    f!(traffic, "remove", |_l, s, id: i64| {
        let mut g = s.borrow_mut();
        let before = g.timetable_buses.len();
        g.timetable_buses.retain(|b| b.get("id").and_then(Value::as_i64) != Some(id));
        g.removed.push(id);
        Ok(g.timetable_buses.len() < before)
    });
    omsi.set("traffic", traffic)?;
    let timetable = lua.create_table()?;
    f!(timetable, "buses", |l, s, _a: ()| to_lua(l, &Value::from(s.borrow().timetable_buses.clone())));
    f!(timetable, "lines", |l, s, _a: ()| to_lua(l, &s.borrow().lines.clone()));
    f!(timetable, "set_ai_trips", |l, s, trips: LuaValue| {
        let v = if trips.is_nil() { Value::Null } else { from_lua(l, trips)? };
        let n = v.as_array().map_or(0, Vec::len) as i64;
        s.borrow_mut().ai_trips.push(v);
        Ok(n)
    });
    omsi.set("timetable", timetable)?;
    f!(omsi, "speed", |_l, s, _a: ()| Ok(s.borrow().speed));
    f!(omsi, "vehicle", |_l, s, _a: ()| Ok(s.borrow().vehicle.clone()));
    f!(omsi, "trigger", |_l, s, name: String| {
        s.borrow_mut().pressed.push(name);
        Ok(())
    });
    f!(omsi, "press", |_l, s, name: String| {
        s.borrow_mut().held.push((name, true));
        Ok(())
    });
    f!(omsi, "release", |_l, s, name: String| {
        s.borrow_mut().held.push((name, false));
        Ok(())
    });
    f!(omsi, "vars", |l, s, kind: Option<String>| {
        let g = s.borrow();
        let names: Vec<String> = if kind.as_deref() == Some("str") { g.strs.keys().cloned().collect() } else { g.vars.keys().cloned().collect() };
        to_lua(l, &json!(names))
    });
    f!(omsi, "str", |_l, s, name: String| Ok(s.borrow().strs.get(&name).cloned()));
    f!(omsi, "send", |_l, s, a: (i64, String)| {
        s.borrow_mut().sent.push(a);
        Ok(true)
    });

    let util = lua.create_table()?;
    f!(util, "now", |_l, s, _a: ()| Ok(s.borrow().now));
    omsi.set("util", util)?;

    // the plugin's own files
    let files = lua.create_table()?;
    {
        let dir = data_dir.clone();
        files.set("read", lua.create_function(move |_, name: String| Ok(std::fs::read_to_string(dir.join(name)).ok()))?)?;
        let dir = data_dir.clone();
        files.set(
            "write",
            lua.create_function(move |_, (name, text): (String, String)| {
                let _ = std::fs::create_dir_all(&dir);
                Ok(std::fs::write(dir.join(name), text).is_ok())
            })?,
        )?;
        let dir = data_dir.clone();
        files.set(
            "append",
            lua.create_function(move |_, (name, text): (String, String)| {
                use std::io::Write;
                let _ = std::fs::create_dir_all(&dir);
                let ok = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(name)).and_then(|mut f| f.write_all(text.as_bytes())).is_ok();
                Ok(ok)
            })?,
        )?;
    }
    omsi.set("files", files)?;

    let jsn = lua.create_table()?;
    jsn.set("encode", lua.create_function(|l, v: LuaValue| Ok(serde_json::to_string(&from_lua(l, v)?).unwrap_or_default()))?)?;
    jsn.set("decode", lua.create_function(|l, text: String| serde_json::from_str::<Value>(&text).map_or(Ok(LuaValue::Nil), |v| to_lua(l, &v)))?)?;
    omsi.set("json", jsn)?;

    // the plugin's panels
    let ui = lua.create_table()?;
    ui.set("version", 2)?;
    f!(ui, "set", |l, s, a: (String, LuaValue)| {
        let v = from_lua(l, a.1)?;
        s.borrow_mut().panels.insert(a.0, v);
        Ok(true)
    });
    f!(ui, "toast", |l, s, a: (String, Option<LuaValue>)| {
        let opts = a.1.map(|o| from_lua(l, o)).transpose()?.unwrap_or(Value::Null);
        let title = opts.get("title").and_then(Value::as_str).unwrap_or("").to_string();
        s.borrow_mut().toasts.push(Toast { text: a.0, title });
        Ok(true)
    });
    f!(ui, "remove", |_l, s, id: String| {
        let mut g = s.borrow_mut();
        g.moved.remove(&id);
        Ok(g.panels.remove(&id).is_some())
    });
    f!(ui, "moved", |_l, s, id: String| {
        let g = s.borrow();
        // (no panel: nothing, as the game answers)
        let m = g.panels.contains_key(&id).then(|| g.moved.get(&id).copied().unwrap_or((0.0, 0.0)));
        Ok((m.map(|m| m.0), m.map(|m| m.1)))
    });
    f!(ui, "update", |_l, _s, _a: mlua::MultiValue| Ok(true));
    {
        let focused = Rc::new(RefCell::new(false));
        let f2 = Rc::clone(&focused);
        ui.set(
            "focus",
            lua.create_function(move |_, on: bool| {
                *f2.borrow_mut() = on;
                Ok(())
            })?,
        )?;
        ui.set("focused", lua.create_function(move |_, ()| Ok(*focused.borrow()))?)?;
    }
    omsi.set("ui", ui)?;

    // the duty
    let duty = lua.create_table()?;
    f!(duty, "get", |l, s, _a: ()| match &s.borrow().duty {
        Some(d) => to_lua(l, &json!({ "line": d.line, "tour": d.tour, "trip": d.trip, "trips": d.trips, "delay": d.delay })),
        None => Ok(LuaValue::Nil),
    });
    f!(duty, "trips", |l, s, _a: ()| {
        let trips: Vec<Value> =
            s.borrow().trips.iter().map(|t| json!({ "name": t.name, "line": t.line, "terminus": t.terminus, "departure": t.departure, "arrival": t.arrival, "stops": stops_json(&t.stops) })).collect();
        to_lua(l, &Value::Array(trips))
    });
    f!(duty, "stops", |l, s, _a: ()| {
        let g = s.borrow();
        let k = g.duty.as_ref().map_or(0, |d| d.trip.max(1) as usize - 1);
        to_lua(l, &g.trips.get(k).map_or(json!([]), |t| stops_json(&t.stops)))
    });
    f!(duty, "trip_stops", |l, s, n: i64| {
        let g = s.borrow();
        let t = usize::try_from(n - 1).ok().and_then(|k| g.trips.get(k));
        to_lua(l, &t.map_or(json!([]), |t| stops_json(&t.stops)))
    });
    f!(duty, "start", |_l, s, a: (String, String, Option<i64>, Option<i64>)| {
        let mut g = s.borrow_mut();
        let (trip, stop) = (a.2.unwrap_or(0), a.3.unwrap_or(0));
        g.started.push((a.0.clone(), a.1.clone(), trip, stop));
        let trips = g.trips.len() as i64;
        // (trip 0: the tour's first, or the one under way)
        g.duty = Some(Duty { line: a.0, tour: a.1, trip: trip.max(1), trips, delay: 42.0 });
        Ok(true)
    });
    f!(duty, "finish", |_l, s, _a: ()| {
        s.borrow_mut().duty = None;
        Ok(true)
    });
    f!(duty, "skip_to", |_l, s, n: i64| {
        if let Some(d) = s.borrow_mut().duty.as_mut() {
            d.trip = n;
        }
        Ok(true)
    });
    f!(duty, "set_auto_ibis", |_l, s, on: bool| {
        s.borrow_mut().ibis.push(if on { "auto on" } else { "auto off" }.into());
        Ok(true)
    });
    f!(duty, "type_ibis", |_l, s, _a: ()| {
        s.borrow_mut().ibis.push("typed".into());
        Ok(true)
    });
    f!(duty, "set_closed_stops", |_l, _s, _a: mlua::MultiValue| Ok(true));
    omsi.set("duty", duty)?;

    // the bus
    let bus = lua.create_table()?;
    f!(bus, "file", |_l, s, _a: ()| Ok(s.borrow().bus_file.clone()));
    f!(bus, "passengers", |_l, s, _a: ()| Ok(s.borrow().passengers));
    // (in lower case, as openOMSI 0.3.37 lists them; its trigger functions know no case)
    f!(bus, "triggers", |l, s, _a: ()| to_lua(l, &json!(s.borrow().triggers.iter().map(|t| t.to_lowercase()).collect::<Vec<_>>())));
    f!(bus, "tickets", |l, s, _a: ()| {
        let list: Vec<Value> = s.borrow().tickets.iter().map(|(n, p)| json!({ "name": n, "price": p, "day_ticket": false })).collect();
        to_lua(l, &Value::Array(list))
    });
    f!(bus, "ticket_request", |l, s, _a: ()| match &s.borrow().ticket_request {
        Some((n, p)) => (n.clone(), *p).into_lua_multi(l),
        None => Ok(mlua::MultiValue::new()),
    });
    f!(bus, "sales", |_l, s, _a: ()| Ok(s.borrow().sales));
    f!(bus, "get_vars", |l, s, names: Option<Vec<String>>| {
        // (as Lua numbers: a NaN stays one, as the game gives it)
        let out = l.create_table()?;
        for (k, v) in s.borrow().vars.iter().filter(|(k, _)| names.as_ref().is_none_or(|n| n.contains(k))) {
            out.set(k.as_str(), *v)?;
        }
        Ok(out)
    });
    f!(bus, "get_strings", |l, s, names: Option<Vec<String>>| {
        let g = s.borrow();
        let out: serde_json::Map<String, Value> = g.strs.iter().filter(|(k, _)| names.as_ref().is_none_or(|n| n.contains(k))).map(|(k, v)| (k.clone(), json!(v))).collect();
        to_lua(l, &Value::Object(out))
    });
    f!(bus, "action", |_l, s, a: (String, Option<bool>)| {
        s.borrow_mut().bus_actions.push(a.0);
        Ok(true)
    });
    f!(bus, "set_line", |_l, s, line: String| {
        s.borrow_mut().lines_typed.push(line);
        Ok(true)
    });
    f!(bus, "set_display_font", |_l, s, font: String| {
        s.borrow_mut().fonts.push(font);
        Ok(1)
    });
    omsi.set("bus", bus)?;

    // the game
    let game = lua.create_table()?;
    f!(game, "version", |_l, s, _a: ()| Ok(s.borrow().version.clone()));
    f!(game, "api", |_l, s, _a: ()| Ok(s.borrow().api));
    f!(game, "platform", |_l, s, _a: ()| Ok(s.borrow().platform.clone()));
    f!(game, "settings", |l, s, _a: ()| to_lua(l, &json!({ "language": s.borrow().language, "units": "metric" })));
    f!(game, "action", |_l, s, name: String| {
        s.borrow_mut().actions.push(name);
        Ok(true)
    });
    {
        // (whether a function is there: looked up in the `omsi` table as it is now)
        game.set(
            "has",
            lua.create_function(|l, name: String| {
                let mut t: LuaValue = l.globals().get("omsi")?;
                for part in name.split('.') {
                    t = match t {
                        LuaValue::Table(tb) => tb.get(part)?,
                        _ => return Ok(false),
                    };
                }
                Ok(matches!(t, LuaValue::Function(_)))
            })?,
        )?;
    }
    omsi.set("game", game)?;

    let map = lua.create_table()?;
    f!(map, "info", |l, s, _a: ()| to_lua(l, &json!({ "path": s.borrow().map })));
    f!(map, "lane", |l, s, _a: (f64, f64)| to_lua(l, &json!({ "index": 1, "distance": 0.5, "name": s.borrow().lane })));
    f!(map, "tile_at", |_l, s, a: (f64, f64)| {
        let t = s.borrow().tile;
        let (tx, ty) = ((a.0 / t).floor(), (a.1 / t).floor());
        Ok((tx as i64, ty as i64, a.0 - tx * t, a.1 - ty * t))
    });
    omsi.set("map", map)?;

    let audio = lua.create_table()?;
    f!(audio, "play", |_l, _s, _a: mlua::MultiValue| Ok(true));
    omsi.set("audio", audio)?;

    Ok(omsi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mock_offers_every_function_it_lists() {
        let dir = std::env::temp_dir().join(format!("hub-harness-self-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.lua"), "seen = 0\nomsi.every(2, function() seen = seen + 1 end)\nomsi.on('duty', function(l) last = l end)").unwrap();
        let mut h = Harness::load(&dir, GameState::default()).unwrap();
        let has: Function = h.lua.load("return omsi.game.has").eval().unwrap();
        for f in FUNCTIONS {
            assert!(has.call::<bool>(*f).unwrap(), "omsi.{f}");
        }
        for _ in 0..4 {
            h.frame().unwrap();
        }
        assert_eq!(h.lua.globals().get::<i64>("seen").unwrap(), 2);
        h.state.borrow_mut().duty = Some(Duty { line: "10".into(), ..Duty::default() });
        h.frame().unwrap();
        assert_eq!(h.lua.globals().get::<String>("last").unwrap(), "10");
        h.state.borrow_mut().duty = None;
        h.frame().unwrap();
        assert_eq!(h.lua.globals().get::<Option<String>>("last").unwrap(), None);
    }
}
