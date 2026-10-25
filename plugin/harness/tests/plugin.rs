//! The plugin run on the harness's mock of openOMSI's plugin API: a fake bus with a duty, a
//! few seconds of game, and what reaches Omsi-Hub through the bridge. Ported from the old
//! `omsihub-plugin/tests/plugin.rs`, which ran the plugin inside openOMSI's own runtime.

use hub_game::bridge::{self, Assignment, Notice, ReportReader};
use hub_game::{compat, install};
use hub_plugin_harness::{GameState, Harness, Trip, FUNCTIONS};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A plugins folder of its own for a test, the plugin installed in it as Omsi-Hub installs it.
fn installed(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-plugin-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let plugins = d.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    install::plugin_dir(&plugins)
}

/// When the assignment was given: today (one of yesterday is not believed).
fn issued() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 3600 * 3600
}

fn assignment() -> Assignment {
    let mut texts = BTreeMap::new();
    texts.insert("Start duty".to_string(), "Dienst starten".to_string());
    Assignment {
        issued: issued(),
        company: "Stadtbus Berlin".into(),
        duty: "D-12".into(),
        line: "10".into(),
        tour: "2".into(),
        auto_start: true,
        notices: vec![Notice { id: "n1".into(), title: "Depot".into(), text: "Bus 4711 is in the workshop".into(), kind: "info".into() }],
        texts,
        ..Assignment::default()
    }
}

/// The game: the duty's two trips without stops.
fn game() -> GameState {
    let trip = |n: &str| Trip { name: n.into(), line: "10".into(), ..Trip::default() };
    GameState { trips: vec![trip("Zoo - Hbf"), trip("Hbf - Zoo")], ..GameState::default() }
}

#[test]
fn the_duty_is_taken_its_trips_reported_and_the_notices_shown() {
    let dir = installed("full");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();

    // the handshake: everything the add-on uses is there
    let hello = bridge::read_hello(&dir).expect("data/hello.json");
    assert_eq!((hello.bridge, hello.plugin.as_str(), hello.api, hello.issued), (bridge::BRIDGE, install::VERSION, 1, issued()));
    let c = compat::negotiate(&hello);
    assert_eq!(c.level, compat::Level::Supported, "{c:?}");

    h.frame().unwrap();
    // (auto_start: the duty is taken as soon as the player is in the bus)
    assert_eq!(h.state.borrow().started, [("10".to_string(), "2".to_string(), 0, 0)]);
    h.frame().unwrap();
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(95.0), json!(88.0), json!(100.0)]).unwrap();
    h.frame().unwrap();
    h.state.borrow_mut().duty = None;
    h.frame().unwrap();

    let reports = ReportReader::default().read_new(&dir);
    let kinds: Vec<&str> = reports.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["duty_start", "trip", "duty_end"]);
    let trip = &reports[1];
    assert_eq!((trip.issued, trip.duty.as_str(), trip.line.as_str(), trip.tour.as_str()), (issued(), "D-12", "10", "2"));
    assert_eq!((trip.trip, trip.trip_name.as_str(), trip.how.as_str()), (1, "Zoo - Hbf", "arrived"));
    assert_eq!((trip.trips, trip.number.as_str()), (2, "10"));
    assert_eq!((trip.driving, trip.comfort, trip.tickets, trip.delay), (Some(95.0), Some(88.0), Some(100.0), Some(42.0)));
    assert_eq!(trip.clock, Some(28_800.0));

    // the notice as a card, once; the company's panel
    let toasts: Vec<String> = h.state.borrow().toasts.iter().map(|t| t.text.clone()).collect();
    assert_eq!(toasts, ["Bus 4711 is in the workshop"]);
    let panel = h.state.borrow().panels.get("omsihub").cloned().expect("the company's panel");
    assert!(panel.to_string().contains("Stadtbus Berlin"), "{panel}");

    // a notice that comes while the game runs
    bridge::write_notices(&dir, &[Notice { id: "n2".into(), text: "Diversion on line 10".into(), ..Notice::default() }]).unwrap();
    for _ in 0..6 {
        h.frame().unwrap();
    }
    assert!(h.state.borrow().toasts.iter().any(|t| t.text == "Diversion on line 10"));
}

#[test]
fn an_older_openomsi_without_files_panels_or_game_functions_still_works() {
    let dir = installed("old");
    bridge::write_assignment(&dir, &Assignment { auto_start: false, ..assignment() }).unwrap();
    // (an openOMSI before the file functions, the panels and `game.*`)
    let mut h = Harness::load_with(&dir, game(), "omsi.files = nil; omsi.ui = nil; omsi.game = nil; omsi.json = nil").unwrap();
    // the player takes the duty from the game's menu; trips are watched one by one
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 42.0 });
    for trip in [1, 1, 2, 2, 2] {
        h.state.borrow_mut().duty.as_mut().unwrap().trip = trip;
        h.frame().unwrap();
    }
    // nothing could be written: the handshake and the reports are in the log
    assert!(!dir.join(bridge::DATA_DIR).join(bridge::REPORTS_FILE).exists());
    let (hello, reports) = bridge::from_log(&h.log_text());
    let hello = hello.expect("the handshake in the log");
    assert_eq!(hello.functions.get("files.append"), Some(&false));
    let c = compat::negotiate(&hello);
    assert_eq!(c.level, compat::Level::Limited);
    assert!(c.features.iter().any(|f| f.id == "reports" && !f.on));
    let kinds: Vec<&str> = reports.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["duty_start", "trip"]);
    assert_eq!((reports[1].trip, reports[1].trip_name.as_str()), (1, "Zoo - Hbf"));
    // the notice as a plain line of text
    let messages = h.state.borrow().messages.clone();
    assert!(messages.iter().any(|m| m == "Depot: Bus 4711 is in the workshop"), "{messages:?}");
}

#[test]
fn the_plugin_checks_every_function_the_add_on_negotiates() {
    let main = install::FILES[0].1;
    let needs = &main[main.find("local NEEDS = {").unwrap()..];
    let needs = &needs[..needs.find('}').unwrap()];
    let mut in_lua: Vec<&str> = needs.split('"').skip(1).step_by(2).collect();
    let mut in_rust: Vec<&str> = compat::FEATURES.iter().flat_map(|f| f.needs.iter().copied()).collect();
    in_lua.sort();
    in_rust.sort();
    // (a function two features need is looked up once)
    in_rust.dedup();
    assert_eq!(in_lua, in_rust);
    // (and the harness's mock of the API has each of them, so every feature is exercised)
    for n in in_rust {
        assert!(FUNCTIONS.contains(&n), "the mock lacks omsi.{n}");
    }
}

#[test]
fn an_assignment_of_an_earlier_day_is_not_taken() {
    let dir = installed("stale");
    bridge::write_assignment(&dir, &Assignment { issued: issued() - 2 * 86_400, ..assignment() }).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    h.frame().unwrap();
    // (no duty taken, no company's panel)
    assert!(h.state.borrow().started.is_empty());
    assert!(!h.state.borrow().panels.contains_key("omsihub"));
    assert_eq!(bridge::read_hello(&dir).map(|h| h.issued), Some(0));
}

/// A duty of parts as Omsi-Hub gives it (`bridge::Leg`).
fn leg(line: &str, tour: &str, first: u32, trips: u32, from_stop: u32, to_stop: u32) -> bridge::Leg {
    bridge::Leg { line: line.into(), tour: tour.into(), first, trips, from_stop, to_stop }
}

fn toasted(h: &Harness, part: &str) -> bool {
    h.state.borrow().toasts.iter().any(|t| t.text.contains(part))
}

#[test]
fn a_shift_s_parts_are_chained_and_the_duty_ends_after_the_last() {
    let dir = installed("legs");
    bridge::write_assignment(&dir, &Assignment { auto_start: false, legs: vec![leg("10", "2", 1, 1, 0, 0), leg("11", "3", 2, 1, 0, 0)], ..assignment() }).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();
    // (the game's command line started the first part's tour)
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 0.0 });
    h.frame().unwrap();
    assert!(h.state.borrow().started.is_empty(), "the first part is the game's");
    // the first part's last trip done: the second part taken with duty.start
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    // (in a call of its own, the next frame's: openOMSI stops a plugin whose one call runs over
    // 50 ms, and duty.start can take that long by itself; its card in the frame after)
    assert!(h.state.borrow().started.is_empty());
    h.frame().unwrap();
    assert_eq!(h.state.borrow().started, [("11".to_string(), "3".to_string(), 2, 0)]);
    h.frame().unwrap();
    assert!(toasted(&h, "Part 2 of 2: line 11, tour 3"));
    // its last trip done: the duty ended
    h.fire("trip_done", &[json!(2), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    assert!(h.state.borrow().duty.is_none());
    assert!(toasted(&h, "Your duty is done"));
    h.frame().unwrap();
    let reports = ReportReader::default().read_new(&dir);
    let kinds: Vec<(&str, &str)> = reports.iter().map(|r| (r.kind.as_str(), r.line.as_str())).collect();
    assert_eq!(kinds, [("duty_start", "10"), ("trip", "10"), ("duty_end", "10"), ("duty_start", "11"), ("trip", "11"), ("duty_end", "11")]);
}

#[test]
fn a_duty_with_reliefs_begins_and_ends_on_the_way() {
    let dir = installed("relief");
    // (taken over at the third stop of trip 1, handed over at the fourth stop of trip 2)
    bridge::write_assignment(&dir, &Assignment { auto_start: false, legs: vec![leg("10", "2", 1, 2, 3, 4)], ..assignment() }).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 0.0 });
    h.frame().unwrap();
    assert_eq!(h.state.borrow().started, [("10".to_string(), "2".to_string(), 1, 3)], "taken again from the relief stop");
    h.frame().unwrap();
    // trip 1 done: the part goes on (it ends at a relief, not at a trip's end)
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    assert!(h.state.borrow().duty.is_some());
    h.state.borrow_mut().duty.as_mut().unwrap().trip = 2;
    h.frame().unwrap();
    h.fire("stop_arrive", &[json!(null), json!(null), json!(3), json!(0.0)]).unwrap();
    assert!(h.state.borrow().duty.is_some(), "not yet at the relief stop");
    h.fire("stop_arrive", &[json!(null), json!(null), json!(4), json!(0.0)]).unwrap();
    assert!(h.state.borrow().duty.is_none(), "the relief takes over");
    assert!(toasted(&h, "Your relief takes over here"));
    let reports = ReportReader::default().read_new(&dir);
    assert!(reports.iter().any(|r| r.kind == "trip" && r.trip == 2 && r.how == "relieved"), "{reports:?}");
}

#[test]
fn a_free_drive_says_its_route() {
    let dir = installed("free");
    bridge::write_assignment(&dir, &Assignment { auto_start: false, duty: String::new(), line: "14".into(), tour: String::new(), free_line: "14_Hbf-Zoo".into(), ..assignment() }).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    assert!(toasted(&h, "Free drive on line 14: 14_Hbf-Zoo"));
    assert!(h.state.borrow().started.is_empty());
}

#[test]
fn the_game_s_traffic_reaches_the_fleet_map() {
    let dir = installed("traffic");
    let mut g = game();
    g.position = Some((650.5, 1210.25, 92.0));
    g.duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 95.0 });
    g.speed = 31.0;
    g.traffic = vec![
        json!({ "id": 7, "kind": "car", "x": 700.0, "y": 1200.0, "heading": 270.0 }),
        json!({ "id": 9, "kind": "timetable_bus", "x": 100.0, "y": 50.0, "heading": 45.0 }),
        json!({ "id": 11, "kind": "tram", "x": 820.04, "y": 1190.0 }),
    ];
    g.timetable_buses = vec![json!({ "id": 9, "line": 5, "tour": "3", "trip": 2, "terminus": "Zoo", "delay": 125.0, "number": 4711, "x": 100.0, "y": 50.0, "at_stop": true })];
    g.others = vec![json!({ "kind": "ai", "x": 1.0, "y": 2.0 }), json!({ "kind": "player", "name": "Sam", "x": 905.0, "y": 610.0, "heading": 180.0 })];
    let mut h = Harness::load(&dir, g).unwrap();
    h.frame().unwrap();
    let t = bridge::read_traffic(&dir).expect("data/traffic.json");
    assert_eq!(t.map, "maps/Grundorf/global.cfg");
    let p = t.player.expect("the player's bus");
    assert_eq!((p.x, p.y, p.heading, p.speed), (650.5, 1210.3, 92.0, 31.0));
    assert_eq!((p.line.as_str(), p.tour.as_str(), p.delay), ("10", "2", Some(95.0)));
    // (the timetable's bus only in its own list, with its heading from the traffic's)
    let v: Vec<_> = t.vehicles.iter().map(|v| (v.id, v.x, v.y, v.heading, v.kind.as_str())).collect();
    assert_eq!(v, [(Some(7), 700.0, 1200.0, 270.0, "car"), (Some(11), 820.0, 1190.0, 0.0, "tram")]);
    let b = &t.buses[0];
    assert_eq!((b.line.as_str(), b.tour.as_str(), b.number.as_str(), b.delay, b.heading, b.at_stop), ("5", "3", "4711", Some(125.0), 45.0, true));
    assert_eq!((t.players.len(), t.players[0].name.as_str()), (1, "Sam"));
    // (where the tiles lie: the player's place on its tile of 300 m)
    let r = t.reference.expect("a place by its tile");
    assert_eq!((r.tile_x, r.tile_y, r.local_x, r.x), (2, 4, 50.5, 650.5));

    // on foot, without traffic: empty lists, as `{}` from Lua, read as empty
    h.state.borrow_mut().position = None;
    h.state.borrow_mut().traffic.clear();
    h.state.borrow_mut().timetable_buses.clear();
    h.state.borrow_mut().others.clear();
    for _ in 0..2 {
        h.frame().unwrap();
    }
    let t = bridge::read_traffic(&dir).expect("written again");
    assert!(t.player.is_none() && t.vehicles.is_empty() && t.buses.is_empty() && t.reference.is_none(), "{t:?}");
}

#[test]
fn an_openomsi_without_the_traffic_functions_writes_none() {
    let dir = installed("no-traffic");
    let mut h = Harness::load_with(&dir, game(), "omsi.traffic = nil; omsi.timetable = nil; omsi.position = nil").unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    assert!(bridge::read_traffic(&dir).is_none());
}

#[test]
fn every_drive_writes_the_live_dashboard() {
    // a drive of the player's own, no assignment: the bus alone
    let dir = installed("dash-free");
    let mut h = Harness::load(&dir, GameState { speed: 37.5, position: Some((1200.0, -340.0, 0.0)), passengers: 14, ..game() }).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    assert_eq!((d.issued, d.bus.speed, d.bus.x, d.bus.y, d.bus.passengers), (0, 37.5, Some(1200.0), Some(-340.0), Some(14.0)));
    assert_eq!(d.bus.file, "Vehicles/MAN_SD202/SD202.bus");
    assert_eq!(d.clock, Some(28_800.0));
    assert!(d.duty.is_none() && d.stops.is_empty(), "{d:?}");
    assert!(d.drive.seconds >= 2.0 && d.drive.metres > 0.0, "{:?}", d.drive);
    // (where the bus is, alone, for the navigator: every frame here, a frame being a second)
    let p = bridge::read_pos(&dir).expect("data/pos.json");
    assert_eq!((p.n, p.t, p.x, p.y, p.heading, p.speed), (3, 3.0, 1200.0, -340.0, Some(0.0), 37.5));

    // a company's duty: the duty, its trip and its stops
    let dir = installed("dash-duty");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    let stop = |id: i64, name: &str, at: f64| hub_plugin_harness::Stop { id, name: name.into(), arr: at, dep: at };
    let mut g = game();
    g.trips[0].stops = vec![stop(1, "Zoo", 28_800.0), stop(2, "Markt", 29_100.0), stop(3, "Hbf", 29_400.0)];
    g.trips[0].terminus = "Hbf".into();
    let mut h = Harness::load(&dir, g).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    assert_eq!(d.issued, issued());
    let duty = d.duty.expect("the duty under way");
    assert_eq!((duty.line.as_str(), duty.tour.as_str(), duty.trip, duty.trips, duty.delay), ("10", "2", 1, 2, 42.0));
    assert_eq!((duty.trip_name.as_str(), duty.number.as_str(), duty.terminus.as_str()), ("Zoo - Hbf", "10", "Hbf"));
    let names: Vec<&str> = d.stops.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Zoo", "Markt", "Hbf"]);
    assert_eq!((d.stops[1].arrival, d.stops[1].stops), (29_100.0, true));
}

/// What the bus's IBIS is set to (its scripts' variables) and the way it faces go into the live
/// dashboard; a game that answers a variable it does not have with an error still gets the file
/// written, and the rest of the plugin's second goes on.
#[test]
fn the_live_dashboard_has_the_bus_s_ibis_and_heading_and_survives_a_variable_s_error() {
    let dir = installed("dash-ibis");
    let vars = "local v = { IBIS_LinieKurs = 2301, IBIS_RouteIndex = 0, IBIS_TerminusCode = 105 }
        omsi.var = function(n) if v[n] == nil then error('no variable ' .. n) end return v[n] end
        omsi.str = function(n) error('no string variable ' .. n) end";
    let mut h = Harness::load_with(&dir, GameState { position: Some((1200.0, -340.0, 90.0)), ..game() }, vars).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    let i = d.ibis.expect("the IBIS");
    assert_eq!((i.line, i.route_index, i.terminus, i.terminus_name), (Some(2301.0), Some(0.0), Some(105.0), None));
    assert_eq!(d.bus.heading, Some(90.0));

    // (no variable at all: no IBIS, the file still written every time)
    let dir = installed("dash-ibis-none");
    let mut h = Harness::load_with(&dir, game(), "omsi.var = function(n) error('no variable ' .. n) end").unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    assert!(d.ibis.is_none(), "{d:?}");
}

/// The panel `id` as the game has it now.
fn panel(h: &Harness, id: &str) -> serde_json::Value {
    h.state.borrow().panels.get(id).cloned().unwrap_or_else(|| panic!("no panel {id}"))
}

/// A Lua expression of the plugin's state, as JSON.
fn eval(h: &Harness, lua: &str) -> serde_json::Value {
    let v: mlua::Value = h.lua.load(lua).eval().unwrap();
    serde_json::to_value(&v).unwrap()
}

#[test]
fn the_driver_sizes_and_moves_the_panels_and_they_stay_so() {
    let dir = installed("layout");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    let before = panel(&h, "omsihub");
    assert_eq!((before["width"].as_f64(), before["x"].as_f64(), before["y"].as_f64()), (Some(300.0), Some(16.0), Some(0.0)));
    assert!(!before.to_string().contains("hub_size_up"), "no size row without the mouse");

    // F10: the size row under the panel; two clicks on "larger"
    h.fire("key", &[json!("F10"), json!(true)]).unwrap();
    assert!(panel(&h, "omsihub").to_string().contains("hub_size_up"));
    h.fire("ui_click", &[json!("omsihub"), json!("hub_size_up")]).unwrap();
    h.fire("ui_click", &[json!("omsihub"), json!("hub_size_up")]).unwrap();
    let big = panel(&h, "omsihub");
    assert_eq!(big["width"].as_f64().map(f64::round), Some(360.0));
    assert!(big.to_string().contains("120 %"), "{big}");
    // (its texts grow with it; the padding the game would give it, too)
    assert_eq!(big["children"][1]["size"].as_f64().map(|s| (s * 10.0).round() / 10.0), Some(16.8));
    assert_eq!(big["padding"].as_f64().map(|s| (s * 10.0).round() / 10.0), Some(14.4));

    // dragged 40 to the left and 25 down (the panel sits on the right: x grows leftwards)
    h.state.borrow_mut().moved.insert("omsihub".into(), (-40.0, 25.0));
    h.frame().unwrap();
    h.frame().unwrap();
    assert_eq!(eval(&h, "{ omsi.data.layout.omsihub.dx, omsi.data.layout.omsihub.dy, omsi.data.layout.omsihub.scale }"), json!([-40.0, 25.0, 1.2]));
    // (the panels give the mouse back: the size row goes)
    h.fire("key", &[json!("F10"), json!(true)]).unwrap();
    assert!(!panel(&h, "omsihub").to_string().contains("hub_size_up"));

    // the next drive: the panel comes where it was left, at its size
    let kept = eval(&h, "omsi.data.layout");
    let mut h = Harness::load_with(&dir, game(), &format!("omsi.data.layout = omsi.json.decode('{kept}')")).unwrap();
    h.frame().unwrap();
    let again = panel(&h, "omsihub");
    assert_eq!((again["x"].as_f64(), again["y"].as_f64()), (Some(56.0), Some(25.0)));
    assert_eq!(again["width"].as_f64().map(f64::round), Some(360.0));

    // back to how it came
    h.fire("key", &[json!("F10"), json!(true)]).unwrap();
    h.fire("ui_click", &[json!("omsihub"), json!("hub_size_reset")]).unwrap();
    let reset = panel(&h, "omsihub");
    assert_eq!((reset["x"].as_f64(), reset["y"].as_f64(), reset["width"].as_f64()), (Some(16.0), Some(0.0), Some(300.0)));
}

/// The ids in a panel that start with `prefix`.
fn ids(v: &serde_json::Value, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    match v {
        serde_json::Value::Object(m) => {
            if let Some(id) = m.get("id").and_then(|i| i.as_str()).filter(|i| i.starts_with(prefix)) {
                out.push(id.to_string());
            }
            for x in m.values() {
                out.extend(ids(x, prefix));
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| out.extend(ids(x, prefix))),
        _ => {}
    }
    out
}

#[test]
fn in_an_openomsi_with_the_navigator_s_board_everything_is_in_the_navigator_and_clicked_away_there() {
    let dir = installed("navigator");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    let navigator = "omsi.ui.version = 5; omsi.ui.dock = function() return 320, 900, 400, true end";
    let mut h = Harness::load_with(&dir, game(), navigator).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    // one panel, the navigator's, that takes its clicks without F10; no toasts, no panels of their own
    let nav = panel(&h, "omsihub_board");
    assert_eq!((nav["anchor"].as_str(), nav["grab"].as_bool()), (Some("navigator"), Some(true)));
    assert!(h.state.borrow().toasts.is_empty(), "{:?}", h.state.borrow().toasts.iter().map(|t| t.text.clone()).collect::<Vec<_>>());
    for own in ["omsihub", "omsihub_ontime", "omsihub_msgs", "omsihub_strip"] {
        assert!(!h.state.borrow().panels.contains_key(own), "{own} is a panel of its own");
    }
    let text = nav.to_string();
    for part in ["Bus 4711 is in the workshop", "Stadtbus Berlin", "trip 1 of 2", "On time"] {
        assert!(text.contains(part), "{part} in {text}");
    }
    // (the notification first, under the navigator's map)
    let at = |part: &str| text.find(part).unwrap_or_else(|| panic!("{part} in {text}"));
    assert!(at("Bus 4711 is in the workshop") < at("Stadtbus Berlin") && at("Stadtbus Berlin") < at("trip 1 of 2"));

    // the notice clicked away with the mouse, as it is
    let close = ids(&nav, "note_close_");
    assert_eq!(close.len(), 1, "{nav}");
    h.fire("ui_click", &[json!("omsihub_board"), json!(close[0])]).unwrap();
    assert!(!panel(&h, "omsihub_board").to_string().contains("Bus 4711 is in the workshop"));

    // a notice while the game runs comes into the navigator too; one not clicked away goes by itself
    bridge::write_notices(&dir, &[Notice { id: "n2".into(), text: "Diversion on line 10".into(), ..Notice::default() }]).unwrap();
    for _ in 0..6 {
        h.frame().unwrap();
    }
    assert!(panel(&h, "omsihub_board").to_string().contains("Diversion on line 10"));
    for _ in 0..40 {
        h.frame().unwrap();
    }
    assert!(!panel(&h, "omsihub_board").to_string().contains("Diversion on line 10"));

    // the on-time card in the navigator, and closed there for the drive
    assert_eq!(ids(&panel(&h, "omsihub_board"), "ontime_close").len(), 1);
    h.fire("ui_click", &[json!("omsihub_board"), json!("ontime_close")]).unwrap();
    h.frame().unwrap();
    assert!(ids(&panel(&h, "omsihub_board"), "ontime_close").is_empty());
}
