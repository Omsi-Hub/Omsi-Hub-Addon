//! The maps the pages draw, end to end (`hub_route_map`, `co_duty_map`: `content::mapview`, the
//! old app's map read as the game's city map has it): a synthetic OMSI 2 folder with a map of
//! one tile - a road of two splines with its two lanes from a spline file, two stops beside it -
//! and a timetable whose trip drives the road (its track): the roads drawn, the trip's route on
//! the lane it drives, its stops where the map has them.

use hub_api::{call, Quiet};
use hub_omsi::timetable::{write_text, BusStopEntry, Track, TrackEntry, Trip, TripProfile, TripStop};
use hub_omsi::Codepage;
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

fn points(v: &Value) -> Vec<[f64; 2]> {
    v.as_array().unwrap().chunks(2).map(|c| [c[0].as_f64().unwrap(), c[1].as_f64().unwrap()]).collect()
}

#[test]
fn the_maps_draw_the_city_map_s_roads_and_the_trip_on_its_lane() {
    let base = std::env::temp_dir().join(format!("hub-api-mapview-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("OMSI 2");
    for d in ["Vehicles", "Sceneryobjects/Test", "Splines/Test", "maps/Roadtown/TTData"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("Omsi.exe"), b"").unwrap();
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, base.join("data"));
    std::env::set_var(hub_core::paths::HOME_DIR_ENV, base.join("home"));
    hub_core::settings::Settings { omsi2_folder: root.to_string_lossy().into_owned(), ..Default::default() }.save().unwrap();
    let map = root.join("maps/Roadtown");
    utf16(&map.join("global.cfg"), "[name]\nRoadtown\n\n[version]\n14\n\n[map]\n0\n0\ntile_0_0.map\n");
    // (a road from (20, 100) 500 m east in two splines; the stops 50 m and 450 m along it)
    let spline = |id: i64, prev: i64, next: i64, x: f64| format!("[spline]\n0\nSplines\\Test\\road.sli\n{id}\n{prev}\n{next}\n{x}\n0\n100\n90\n250\n0\n0\n0\n0\n0\n0\n0\n0\n\n");
    let stop = |id: i64, x: f64, name: &str| format!("[object]\n0\nSceneryobjects\\Test\\stop.sco\n{id}\n{x}\n96\n0\n90\n0\n0\n1\n{name}\n\n");
    utf16(&map.join("tile_0_0.map"), &format!("[version]\n14\n\n{}{}{}{}", spline(1, -1, 2, 20.0), spline(2, 1, -1, 270.0), stop(900, 70.0, "Market"), stop(901, 470.0, "Church")));
    std::fs::write(root.join("Splines/Test/road.sli"), "[path]\r\n0\r\n-1.75\r\n0\r\n3.5\r\n1\r\n\r\n[path]\r\n0\r\n1.75\r\n0\r\n3.5\r\n0\r\n").unwrap();
    std::fs::write(root.join("Sceneryobjects/Test/stop.sco"), "[busstop]\r\n\r\n[groups]\r\n1\r\nStop\r\n").unwrap();
    // the trip east on the road's right-hand lane (path 1), from Market to Church
    let tt = map.join("TTData");
    let trip = Trip {
        name: "1_out".into(),
        terminus: "Church".into(),
        line: "1".into(),
        stops: [900, 901].iter().map(|id| TripStop { id: *id, ..TripStop::default() }).collect(),
        profiles: vec![TripProfile { name: "standard".into(), minutes: 3.0, ..TripProfile::default() }],
        ..Trip::default()
    };
    write_text(&tt.join("1_out.ttp"), &trip.to_text(), Codepage::Windows1252).unwrap();
    let entry = |spline: i64| TrackEntry { spline, path: 1, tile: 0, length: 250.0, ..TrackEntry::default() };
    write_text(&tt.join("1_out.ttr"), &Track { name: "1_out".into(), entries: vec![entry(1), entry(2)], ..Track::default() }.to_text(), Codepage::Windows1252).unwrap();
    let busstops = [(900, "Market"), (901, "Church")].iter().map(|(id, n)| BusStopEntry { name: n.to_string(), tile: 0, object_id: *id, ..BusStopEntry::default() }.to_text()).collect::<String>();
    write_text(&tt.join("Busstops.cfg"), &busstops, Codepage::Windows1252).unwrap();
    const MAP: &str = "maps/Roadtown/global.cfg";

    let m = call("hub_route_map", &json!({ "map": MAP, "date": "1989-05-30", "trip": "1_out" }), &Quiet).unwrap();
    assert!(m["roads"].as_array().is_some_and(|r| !r.is_empty()), "{m}");
    let o = [m["origin"][0].as_f64().unwrap(), m["origin"][1].as_f64().unwrap()];
    // (the route: the right-hand lane east, 1.75 m south of the road's line, 500 m)
    let route: Vec<[f64; 2]> = m["route"].as_array().unwrap().iter().flat_map(points).map(|p| [p[0] + o[0], p[1] + o[1]]).collect();
    assert!(route.len() >= 2, "{m}");
    let (first, last) = (route[0], *route.last().unwrap());
    assert!((first[0] - 20.0).abs() < 1.0 && (last[0] - 520.0).abs() < 1.0, "{first:?} - {last:?}");
    assert!(route.iter().all(|p| (p[1] - 98.25).abs() < 0.2), "on the right-hand lane: {route:?}");
    assert_eq!(m["stops"].as_array().map(Vec::len), Some(2), "{m}");

    // the duty map: the same roads, the trip's route
    let d = call("co_duty_map", &json!({ "map": MAP, "date": "1989-05-30", "trips": ["1_out", "none"] }), &Quiet).unwrap();
    assert_eq!(d["route"].as_array().map(Vec::len), Some(1), "a trip the timetable lacks draws nothing: {d}");
    assert!(call("hub_route_map", &json!({ "map": "maps/Nowhere/global.cfg" }), &Quiet).is_err());
    let _ = std::fs::remove_dir_all(&base);
}
