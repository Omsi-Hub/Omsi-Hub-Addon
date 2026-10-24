//! The line editor end to end (`hub_le`, the old app's headless one): a synthetic OMSI 2 folder
//! with a map of one tile - a road 600 m east, its two lanes from a spline file, and two bus
//! stops beside it (a `.sco` with `[busstop]`) - opened, read (its roads, its lanes, its stops),
//! and a new line clicked from the first stop to the second: the way between them found over
//! the road's lanes.

use hub_api::{call, Quiet};
use serde_json::{json, Value};
use std::path::Path;

/// A text as OMSI writes a map file: UTF-16 little-endian, its BOM first, `\r\n`.
fn utf16(path: &Path, text: &str) {
    let mut bytes = vec![0xFF, 0xFE];
    for u in text.replace('\n', "\r\n").encode_utf16() {
        bytes.extend(u.to_le_bytes());
    }
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn le(op: &str, args: Value) -> Value {
    let mut a = args;
    a["op"] = json!(op);
    a["lang"] = json!("en");
    call("hub_le", &a, &Quiet).unwrap_or_else(|e| panic!("{op}: {e}"))
}

#[test]
fn a_map_is_read_and_a_new_line_routed_between_two_of_its_stops() {
    let base = std::env::temp_dir().join(format!("hub-api-le-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("OMSI 2");
    for d in ["Vehicles", "Sceneryobjects/Test", "Splines/Test", "maps/Linedorf/TTData"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("Omsi.exe"), b"").unwrap();
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, base.join("data"));
    std::env::set_var(hub_core::paths::HOME_DIR_ENV, base.join("home"));
    hub_core::settings::Settings { omsi2_folder: root.to_string_lossy().into_owned(), ..Default::default() }.save().unwrap();
    let map = root.join("maps/Linedorf");
    utf16(&map.join("global.cfg"), "[name]\nLinedorf\n\n[version]\n14\n\n[map]\n0\n0\ntile_0_0.map\n");
    // (a road from (20, 100) 250 m east and on another 250 m; the stops 50 m and 450 m along it,
    // beside its right-hand lane)
    let spline = |id: i64, prev: i64, next: i64, x: f64| format!("[spline]\n0\nSplines\\Test\\road.sli\n{id}\n{prev}\n{next}\n{x}\n0\n100\n90\n250\n0\n0\n0\n0\n0\n0\n0\n0\n\n");
    let stop = |id: i64, x: f64, name: &str| format!("[object]\n0\nSceneryobjects\\Test\\stop.sco\n{id}\n{x}\n96\n0\n90\n0\n0\n1\n{name}\n\n");
    utf16(&map.join("tile_0_0.map"), &format!("[version]\n14\n\n{}{}{}{}", spline(1, -1, 2, 20.0), spline(2, 1, -1, 270.0), stop(900, 70.0, "Market"), stop(901, 470.0, "Church")));
    std::fs::write(root.join("Splines/Test/road.sli"), "[path]\r\n0\r\n-1.75\r\n0\r\n3.5\r\n1\r\n\r\n[path]\r\n0\r\n1.75\r\n0\r\n3.5\r\n0\r\n").unwrap();
    std::fs::write(root.join("Sceneryobjects/Test/stop.sco"), "[busstop]\r\n\r\n[groups]\r\n1\r\nStop\r\n").unwrap();

    let maps = json!([["Linedorf", "maps/Linedorf/global.cfg"]]);
    let mut v = le(
        "open",
        // (no folder, as the page sends it: the line editor finds the map in the OMSI 2 folder of
        // the settings)
        json!({ "maps": maps, "root": "", "date": "1989-05-30", "choice_map": "maps/Linedorf/global.cfg", "company": "", "line": null, "new_days": null, "map_line": "" }),
    );
    for _ in 0..300 {
        if v["ready"].as_bool() == Some(true) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        v = le("view", json!({}));
    }
    assert_eq!(v["ready"].as_bool(), Some(true), "the map read: {v}");
    let data = le("map_data", json!({}));
    let stops = data["stops"].as_array().unwrap();
    assert!(data["roads"].as_array().is_some_and(|r| !r.is_empty()), "{data}");
    let names: Vec<&str> = stops.iter().filter_map(|s| s["name"].as_str()).collect();
    assert!(names.contains(&"Market") && names.contains(&"Church"), "{names:?}");
    let at = |name: &str| names.iter().position(|n| *n == name).unwrap();
    le("new_line", json!({}));
    le("click_stop", json!({ "i": at("Market") }));
    let v = le("click_stop", json!({ "i": at("Church") }));
    let d = &v["line"]["directions"][0];
    assert_eq!(d["stops"].as_array().map(Vec::len), Some(2), "{v}");
    let leg = &d["legs"][0];
    assert_eq!(leg["ok"].as_bool(), Some(true), "the way over the road: {leg}");
    // (the road from the first stop to the second: about 400 m)
    let m = leg["length"].as_f64().unwrap();
    assert!((350.0..=450.0).contains(&m), "{m} m");
    // a leg or a way back the page asks for that is not there: nothing happens (no panic)
    let v = le("drag_new", json!({ "k": 7, "at": [0.0, 0.0] }));
    assert_eq!(v["line"]["directions"][0]["stops"].as_array().map(Vec::len), Some(2), "{v}");
    le("use_similar", json!({}));
    le("dir", json!({ "d": 1 }));
    le("use_similar", json!({}));
    le("dir", json!({ "d": 0 }));
    // the self-check's line left unsaved and forgotten: the map's lines read again on opening
    le("leave", json!({ "forget": true }));
    let v = le("open", json!({ "maps": maps, "root": "", "date": "1989-05-30", "choice_map": "maps/Linedorf/global.cfg", "company": "", "line": null, "new_days": null, "map_line": "" }));
    assert_eq!((v["rows"].as_array().map(Vec::len), v["dirty"].as_bool()), (Some(0), Some(false)), "{v}");
    le("leave", json!({}));
    let _ = std::fs::remove_dir_all(&base);
}
