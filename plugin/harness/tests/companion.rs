//! The phone and tablet companion's side in the game (`plugin/companion.lua`), on the harness's
//! mock of openOMSI's plugin API: what it sends Omsi-Hub's companion server, and the phone's
//! commands it carries out - only the keys of the bus's IBIS and ticket printer, each command
//! once.

use hub_game::bridge::{self, Assignment, Inbox, InboxMsg, Personnel};
use hub_game::install;
use hub_plugin_harness::{GameState, Harness, Trip};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const PORT: i64 = 47812;

fn installed(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-companion-plugin-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let plugins = d.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    install::plugin_dir(&plugins)
}

fn issued() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 3600 * 3600
}

/// A bus with the stock IBIS, a ticket printer and a door; a duty of two trips.
fn game() -> GameState {
    let trip = |n: &str, t: &str| Trip { name: n.into(), line: "29".into(), terminus: t.into(), departure: 28_800.0, arrival: 30_000.0, ..Trip::default() };
    let mut triggers: Vec<String> = (0..10).map(|d| format!("IBIS_{d}")).collect();
    triggers.extend(["IBIS_eingabe", "IBIS_eingabe_off", "ticketprinter_button_ticket_1", "door_1", "wechselgeld_1"].map(String::from));
    let mut g = GameState { trips: vec![trip("Zoo - Hbf", "Hbf"), trip("Hbf - Zoo", "Zoo")], triggers, position: Some((100.0, 200.0, 90.0)), ..GameState::default() };
    g.vars.insert("IBIS_LinieKurs".into(), 29.0);
    g.strs.insert("IBIS_terminus_name".into(), "Hauptbahnhof".into());
    g.tickets = vec![("Single".into(), 2.9), ("Day".into(), 7.0)];
    g.ticket_request = Some(("Single".into(), 2.9));
    g.sales = (3, 8.7);
    g.lines = json!([{ "name": "29", "user_allowed": true, "tours": [{ "number": 1, "today": true }, { "number": 2, "today": false }] }]);
    g
}

/// What the plugin sent the companion server, as JSON, of kind `t`.
fn sent(h: &Harness, t: &str) -> Vec<Value> {
    h.state.borrow().sent.iter().filter(|(p, _)| *p == PORT).filter_map(|(_, text)| serde_json::from_str::<Value>(text).ok()).filter(|v| v["t"] == t).collect()
}

/// Omsi-Hub's queue of commands in the plugin's data folder.
fn queue(dir: &Path, sid: u64, cmds: Value) {
    let seq = cmds.as_array().and_then(|c| c.last()).and_then(|c| c["seq"].as_u64()).unwrap_or(0);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/companion.json"), json!({ "sid": sid, "port": PORT, "seq": seq, "cmds": cmds }).to_string()).unwrap();
}

fn acks(h: &Harness) -> Vec<(u64, bool, String)> {
    sent(h, "ack").iter().map(|a| (a["seq"].as_u64().unwrap(), a["ok"].as_bool().unwrap(), a["error"].as_str().unwrap_or("").to_string())).collect()
}

/// The keys held down, in order.
fn downs(h: &Harness) -> Vec<String> {
    h.state.borrow().held.iter().filter(|k| k.1).map(|k| k.0.clone()).collect()
}

/// The keys let go, in order.
fn ups(h: &Harness) -> Vec<String> {
    h.state.borrow().held.iter().filter(|k| !k.1).map(|k| k.0.clone()).collect()
}

#[test]
fn without_omsi_hubs_companion_nothing_is_sent() {
    let dir = installed("off");
    let mut h = Harness::load(&dir, game()).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    assert!(h.state.borrow().sent.iter().all(|(p, _)| *p != PORT));
}

#[test]
fn the_game_goes_to_the_companion_and_only_the_devices_keys_are_pressed() {
    let dir = installed("keys");
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "29".into(), tour: "1".into(), trip: 1, trips: 2, delay: 60.0 });
    for _ in 0..2 {
        h.frame().unwrap();
    }
    let live = sent(&h, "live");
    let last = live.last().expect("the live datagram");
    assert_eq!((last["duty"]["line"].as_str(), last["duty"]["number"].as_str(), last["duty"]["terminus"].as_str()), (Some("29"), Some("29"), Some("Hbf")));
    assert_eq!((last["bus"]["x"].as_f64(), last["ref"]["tile_x"].as_i64()), (Some(100.0), Some(0)));
    assert_eq!(last["signon"], "work");
    let keys = sent(&h, "keys");
    let k = keys.last().unwrap();
    assert_eq!(k["ibis"].as_array().unwrap().len(), 11, "the ten digits and the entry key, not its _off");
    assert_eq!(k["printer"], json!(["ticketprinter_button_ticket_1"]));
    assert_eq!(k["change"], json!(["wechselgeld_1"]));
    assert_eq!(k["tickets"][1]["name"], "Day");
    let dev = sent(&h, "dev");
    let d = dev.last().unwrap();
    assert_eq!((d["ibis"]["IBIS_LinieKurs"].as_f64(), d["strs"]["IBIS_terminus_name"].as_str()), (Some(29.0), Some("Hauptbahnhof")));
    assert_eq!((d["request"]["name"].as_str(), d["sales"]["count"].as_i64()), (Some("Single"), Some(3)));
    let lines = sent(&h, "lines");
    assert_eq!(lines.last().unwrap()["lines"][0]["tours"][1]["number"], "2");
    assert_eq!(sent(&h, "trips").last().unwrap()["trips"].as_array().unwrap().len(), 2);

    // the phone's commands: each once, a key not the devices' refused
    queue(
        &dir,
        5,
        json!([
            { "seq": 1, "do": "key", "key": "IBIS_7" },
            { "seq": 2, "do": "key", "key": "door_1" },
            { "seq": 3, "do": "ticket", "i": 1 },
            { "seq": 4, "do": "ticket", "i": 2 },
            { "seq": 5, "do": "ibis_line", "line": "29" },
            { "seq": 6, "do": "format_disk" },
        ]),
    );
    h.frame().unwrap();
    h.frame().unwrap();
    // (each a tap: held down and let go a moment later, as a hand presses it)
    assert_eq!(downs(&h), ["IBIS_7", "ticketprinter_button_ticket_1"]);
    assert_eq!(ups(&h), ["IBIS_7", "ticketprinter_button_ticket_1"]);
    assert!(h.state.borrow().pressed.is_empty(), "never pressed and let go in one frame");
    assert_eq!(h.state.borrow().bus_actions, ["ticket_give"], "a ticket without its own key: the ticket key");
    assert_eq!(h.state.borrow().lines_typed, ["29"]);
    let a = acks(&h);
    assert_eq!(a.len(), 6, "{a:?}");
    assert_eq!(a[1], (2, false, "unknown_key".into()));
    assert_eq!(a[5], (6, false, "unknown_command".into()));
    assert!(a[0].1 && a[2].1 && a[3].1 && a[4].1);
}

#[test]
fn commands_in_the_file_when_the_game_starts_are_old_and_a_new_session_starts_again() {
    let dir = installed("old");
    queue(&dir, 5, json!([{ "seq": 1, "do": "key", "key": "IBIS_7" }]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    assert!(h.state.borrow().held.is_empty(), "written before the game started: not done");
    // Omsi-Hub started again: its own numbers from 1
    queue(&dir, 6, json!([{ "seq": 1, "do": "key", "key": "IBIS_1" }]));
    h.frame().unwrap();
    h.frame().unwrap();
    assert_eq!(downs(&h), ["IBIS_1"]);
}

#[test]
#[ignore = "signing on is out for now (main.lua's SIGN_ON)"]
fn the_driver_signs_on_and_accepts_the_duty_on_the_phone() {
    let dir = installed("signon");
    let a = Assignment {
        issued: issued(),
        company: "Stadtbus".into(),
        duty: "14".into(),
        line: "29".into(),
        tour: "1".into(),
        auto_start: true,
        personnel: Some(Personnel { number: "482913".into(), code: "5821".into() }),
        ..Assignment::default()
    };
    bridge::write_assignment(&dir, &a).unwrap();
    queue(&dir, 9, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    assert_eq!(sent(&h, "live").last().unwrap()["signon"], "signon");
    queue(&dir, 9, json!([{ "seq": 1, "do": "sign_on", "number": "482913", "code": "0000" }]));
    h.frame().unwrap();
    h.frame().unwrap();
    assert_eq!(acks(&h), [(1, false, "wrong".to_string())]);
    assert_eq!(sent(&h, "live").last().unwrap()["signon_wrong"], true);
    assert!(h.state.borrow().started.is_empty(), "not signed on: no duty");
    queue(&dir, 9, json!([{ "seq": 2, "do": "sign_on", "number": "482913", "code": "5821" }, { "seq": 3, "do": "accept_order" }]));
    h.frame().unwrap();
    h.frame().unwrap();
    let a = acks(&h);
    assert!(a[1].1 && a[2].1, "{a:?}");
    assert_eq!(sent(&h, "live").last().unwrap()["signon"], "work");
    assert_eq!(h.state.borrow().started.len(), 1, "the duty taken as the order is signed");
    assert!(h.state.borrow().ibis.contains(&"typed".to_string()));
}

#[test]
fn a_message_is_answered_from_the_phone_as_from_the_panel() {
    let dir = installed("msgs");
    let a = Assignment { issued: issued(), company: "Stadtbus".into(), duty: "14".into(), line: "29".into(), tour: "1".into(), ..Assignment::default() };
    bridge::write_assignment(&dir, &a).unwrap();
    let inbox = Inbox {
        line: "29".into(),
        tour: "1".into(),
        messages: vec![InboxMsg { id: 77, text: "Diversion".into(), state: "delivered".into(), priority: "urgent".into(), ..InboxMsg::default() }],
        ..Inbox::default()
    };
    bridge::write_inbox(&dir, &inbox).unwrap();
    queue(&dir, 1, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    queue(&dir, 1, json!([{ "seq": 1, "do": "msg", "kind": "read", "id": 77 }, { "seq": 2, "do": "msg", "kind": "accept", "id": 77 }, { "seq": 3, "do": "msg", "kind": "read", "id": 5 }]));
    h.frame().unwrap();
    h.frame().unwrap();
    let a = acks(&h);
    assert_eq!(a.iter().map(|x| x.1).collect::<Vec<_>>(), [true, false, false], "read; no instruction to accept; no such message: {a:?}");
    let replies = bridge::ReplyReader::default().read_new(&dir);
    assert_eq!(replies.iter().map(|r| (r.kind.as_str(), r.id)).collect::<Vec<_>>(), [("read", 77)]);
}

#[test]
fn the_devices_screens_are_read_ten_times_a_second_and_only_their_triggers_pressed() {
    let dir = installed("screens");
    let mut g = game();
    g.bus_file = "Vehicles\\TestBus\\TestBus.bus".into();
    g.triggers.extend(["IBIS_menu", "IBIS_menu_off"].map(String::from));
    g.vars.insert("IBIS_mode".into(), 1.0);
    g.vars.insert("IBIS_broken".into(), f64::NAN);
    g.strs.insert("IBIS_line_text".into(), "  29/03".into());
    let write = |cmds: Value| {
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let seq = cmds.as_array().and_then(|c| c.last()).and_then(|c| c["seq"].as_u64()).unwrap_or(0);
        let q = json!({
            "sid": 9, "port": PORT, "seq": seq, "cmds": cmds,
            "want": { "nums": ["IBIS_mode", "IBIS_stop", "IBIS_broken"], "strs": ["IBIS_line_text", "IBIS_dest_text"] },
            // (`door_1` is no device's: Omsi-Hub would not list it, and `IBIS_ghost` the bus lacks)
            "press": ["IBIS_menu", "IBIS_7", "IBIS_ghost"],
        });
        std::fs::write(dir.join("data/companion.json"), q.to_string()).unwrap();
    };
    write(json!([]));
    let mut h = Harness::load(&dir, g).unwrap();
    h.frame().unwrap();
    let live = sent(&h, "live");
    assert_eq!(live.last().unwrap()["bus_file"], "Vehicles\\TestBus\\TestBus.bus");
    let scr = sent(&h, "scr");
    assert!(!scr.is_empty(), "the screens' values");
    let items = &scr.last().unwrap()["items"];
    assert_eq!(items[0], json!(["n", "IBIS_mode", 1]));
    assert_eq!(items[1], json!(["o", "IBIS_stop"]), "a number the bus does not have");
    assert_eq!(items[2], json!(["n", "IBIS_broken"]), "no number: none");
    assert_eq!(items[3], json!(["s", "IBIS_line_text", "  29/03"]), "as the bus has it, not trimmed");
    assert_eq!(items[4], json!(["s", "IBIS_dest_text", ""]));
    assert_eq!((scr.last().unwrap()["page"].as_i64(), scr.last().unwrap()["pages"].as_i64()), (Some(1), Some(1)));
    // the same values: the same version; a change: the next
    let v = scr.last().unwrap()["v"].as_i64().unwrap();
    h.frame().unwrap();
    assert_eq!(sent(&h, "scr").last().unwrap()["v"].as_i64(), Some(v));
    h.state.borrow_mut().strs.insert("IBIS_line_text".into(), "  29/04".into());
    h.frame().unwrap();
    assert_eq!(sent(&h, "scr").last().unwrap()["v"].as_i64(), Some(v + 1));

    // the taps: pressed and let go, held, let go; nothing the devices do not have
    write(json!([
        { "seq": 1, "do": "press", "key": "IBIS_menu" },
        { "seq": 2, "do": "press", "key": "IBIS_7", "down": true },
        { "seq": 3, "do": "press", "key": "IBIS_7", "down": false },
        { "seq": 4, "do": "press", "key": "door_1" },
        { "seq": 5, "do": "press", "key": "IBIS_ghost" },
    ]));
    h.frame().unwrap();
    h.frame().unwrap();
    let a = acks(&h);
    assert_eq!(a.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(), [(1, true), (2, true), (3, true), (4, false), (5, false)]);
    assert_eq!(a[3].2, "unknown_key");
    let st = h.state.borrow();
    assert!(st.pressed.is_empty(), "a tap is held a moment, never pressed and let go in one frame");
    // (the game names its triggers in lower case, the model files as written: the same keys)
    let held = |n: &str, d: bool| (n.to_string(), d);
    assert_eq!(st.held, [held("IBIS_menu", true), held("IBIS_7", true), held("IBIS_7", false), held("IBIS_menu", false)]);
}
