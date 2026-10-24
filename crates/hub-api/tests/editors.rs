//! The timetable and depot editors (`tt_`, `de_`) end to end, on a copy of hub-omsi's synthetic
//! OMSI 2 folder and a fake openOMSI (its content folder beside its program): a line's tours
//! changed and saved into the content folder's copy of the map's `TTData`, the map's own
//! timetable back with the reset, and a depot file of the player's own made from the map's,
//! changed, saved and deleted - and nothing of the OMSI 2 folder written.

use hub_api::{call, Quiet};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MAP: &str = "maps/Testdorf/global.cfg";

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// Every file of `dir` with its bytes, by its path.
fn files(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push((p.clone(), std::fs::read(&p).unwrap()));
        }
    }
    out.sort();
}

fn ok(cmd: &str, args: Value) -> Value {
    call(cmd, &args, &Quiet).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

fn line<'a>(tt: &'a Value, name: &str) -> &'a Value {
    tt["lines"].as_array().unwrap().iter().find(|l| l["name"] == name).unwrap_or_else(|| panic!("no line {name} in {tt}"))
}

#[test]
fn the_editors_write_into_the_content_folder_and_the_data_folder_only() {
    let base = std::env::temp_dir().join(format!("hub-api-editors-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let omsi2 = base.join("OMSI 2");
    copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../hub-omsi/tests/fixtures/omsi2"), &omsi2);
    let openomsi = base.join("openOMSI");
    std::fs::create_dir_all(openomsi.join("Plugins")).unwrap();
    std::fs::write(openomsi.join("openomsi.exe"), b"").unwrap();
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, base.join("data"));
    std::env::set_var(hub_core::paths::HOME_DIR_ENV, base.join("home"));
    let settings = hub_core::settings::Settings { omsi2_folder: omsi2.to_string_lossy().into_owned(), openomsi_folder: openomsi.to_string_lossy().into_owned(), ..Default::default() };
    settings.save().unwrap();
    let mut before = Vec::new();
    files(&omsi2, &mut before);

    // --- the timetable ---
    let tt = ok("tt_load", json!({ "map": MAP }));
    assert_eq!(tt["folder"], "Testdorf");
    assert_eq!(PathBuf::from(tt["dir"].as_str().unwrap()), omsi2.join("maps/Testdorf"));
    let ten = line(&tt, "10").clone();
    assert_eq!((ten["user_allowed"].as_bool(), ten["own"].as_bool(), ten["tours"].as_array().unwrap().len()), (Some(true), Some(false), 2));
    assert_eq!(ten["tours"][0]["trips"][1], json!({ "trip": "10_back", "profile": 0, "departure": 375.0 }));
    let out = tt["trips"].as_array().unwrap().iter().find(|t| t["name"] == "10_out").expect("the trip 10_out");
    assert_eq!((out["line"].as_str(), out["profiles"][0].as_str()), (Some("10"), Some("standard")));

    // a copy of tour 1 twenty minutes later, a trip added to it after the last has arrived
    let e = ok("tt_edit", json!({ "line": ten, "tour": 0, "op": "copy", "offset": 20 }));
    assert_eq!((e["tour"].as_u64(), e["line"]["tours"][2]["number"].as_str(), e["line"]["tours"][2]["trips"][0]["departure"].as_f64()), (Some(2), Some("3"), Some(380.0)));
    let e = ok("tt_edit", json!({ "line": e["line"], "tour": 1, "op": "add_trip", "first": "10_back" }));
    // (tour 2 runs 10_out at 10:00, which takes its profile's 10 minutes)
    assert_eq!(e["line"]["tours"][1]["trips"][1], json!({ "trip": "10_back", "profile": 0, "departure": 610.0 }));
    let ten = e["line"].clone();
    let new = json!({ "path": format!("{}/TTData/N1.ttl", tt["dir"].as_str().unwrap()), "name": "N1", "user_allowed": true, "priority": 0, "own": false, "tours": [] });
    let new = ok("tt_edit", json!({ "line": new, "tour": 0, "op": "new_tour", "first": "10_out" }))["line"].clone();
    assert_eq!(new["tours"][0]["ai_group"], "Busses", "a new line's tour takes the AI group of the lines beside it");

    let r = ok("tt_save", json!({ "map": MAP, "lines": [ten, new] }));
    assert_eq!((r["saved"].as_u64(), r["error"].is_null()), (Some(2), true), "{r}");
    let copy = openomsi.join("maps/Testdorf/TTData");
    assert_eq!(PathBuf::from(r["lines"][0]["path"].as_str().unwrap()), copy.join("10.ttl"));
    assert!(copy.join(hub_core::ttstore::COPY_MARK).is_file() && copy.join("10_out.ttp").is_file() && copy.join("N1.ttl").is_file());

    // read again: the content folder's copy, as the game reads it
    let tt = ok("tt_load", json!({ "map": MAP }));
    assert_eq!(PathBuf::from(tt["dir"].as_str().unwrap()), openomsi.join("maps/Testdorf"));
    assert_eq!(line(&tt, "10")["tours"].as_array().unwrap().len(), 3);
    assert_eq!(line(&tt, "N1")["tours"][0]["trips"][0]["trip"], "10_out");
    // (saved again in place, from where it was read)
    let r = ok("tt_save", json!({ "map": MAP, "lines": [line(&tt, "10")] }));
    assert_eq!(PathBuf::from(r["lines"][0]["path"].as_str().unwrap()), copy.join("10.ttl"));
    let bad = ok("tt_save", json!({ "map": MAP, "lines": [{ "path": "", "name": "a/b", "tours": [] }] }));
    assert_eq!(bad["saved"], 0);
    assert!(bad["error"].as_str().unwrap().contains("file name"), "{bad}");

    // the reset: the map's own timetable back
    let r = ok("tt_reset", json!({ "map": MAP }));
    // (a text to translate, the map's name beside it)
    assert!(r["done"].as_str().unwrap().contains("%{map}") && r["done_args"]["map"] == "Testdorf", "{r}");
    assert_eq!((r["own"].as_u64(), r["own_error"].is_null()), (Some(0), true));
    assert!(!copy.exists());
    assert_eq!(line(&ok("tt_load", json!({ "map": MAP })), "10")["tours"].as_array().unwrap().len(), 2);
    assert!(call("tt_reset", &json!({ "map": MAP }), &Quiet).unwrap_err().contains("not been changed"));

    // --- the depot files ---
    let shown = ok("de_show", json!({ "key": null, "maps": [["Testdorf", "Testdorf.hof"]] }));
    assert_eq!((shown["entries"].as_array().unwrap().len(), shown["view"].is_null()), (0, true));
    let sources = ok("de_sources", json!({ "hof": "Testdorf.hof", "map": MAP, "bus": "Vehicles/Testbus/Testbus.bus", "bus_name": "Testbus" }));
    let kinds: Vec<&str> = sources.as_array().unwrap().iter().map(|s| s["source"]["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["empty", "map"], "the bus's one file is the map's: offered once");
    assert_eq!(sources[1]["args"]["hof"], "Testdorf.hof");

    let made = ok("de_make", json!({ "name": "Testdorf mine", "map": MAP, "source": sources[1]["source"] }));
    let v = &made["view"];
    assert_eq!((v["name"].as_str(), v["map"].as_str(), v["dirty"].as_bool()), (Some("Testdorf mine"), Some("Testdorf"), Some(false)));
    let key = v["key"].as_str().unwrap().to_string();
    assert!(key.starts_with("oo_"), "{key}");
    assert!(v["termini"].as_array().unwrap().iter().any(|t| t["name"] == "Kirche"), "the map's destinations are in it");
    assert_eq!(made["entries"][0]["key"], key.as_str());

    let i = ok("de_edit", json!({ "op": "t_add", "value": "Zoo" }));
    let at = i["sel"].as_u64().unwrap();
    assert_eq!((i["view"]["dirty"].as_bool(), i["view"]["termini"][at as usize]["name"].as_str()), (Some(true), Some("Zoo")));
    // (another file waits until the changes are saved)
    let refused = call("de_make", &json!({ "name": "Spare", "map": MAP, "source": { "kind": "empty" } }), &Quiet).unwrap_err();
    assert_eq!(refused, "%{name} has changes that are not saved: save it first|Testdorf mine");
    assert!(ok("de_show", json!({ "key": "oo_other" }))["error"].as_str().unwrap().ends_with("|Testdorf mine"));
    // (the file open clicked again in the list: its changes stay, not read over from the disk)
    let again = ok("de_open", json!({ "key": key }));
    assert_eq!(again["dirty"].as_bool(), Some(true));
    assert!(again["termini"].as_array().unwrap().iter().any(|t| t["name"] == "Zoo"), "Zoo kept");
    let saved = ok("de_save", json!({}));
    assert_eq!((saved["name"].as_str(), saved["copies"].as_u64(), saved["view"]["dirty"].as_bool()), (Some("Testdorf mine"), Some(0), Some(false)));
    let file = base.join("data/depots").join(format!("{key}.hof"));
    assert!(hub_core::owndepot::load(&file).unwrap().doc.destination("Zoo").is_some(), "Zoo written");

    // a second file, empty, then the first opened again from the list and deleted (kept aside)
    let other = ok("de_make", json!({ "name": "Spare", "map": MAP, "source": { "kind": "empty" } }));
    assert_eq!((other["view"]["name"].as_str(), other["view"]["termini"].as_array().unwrap().len(), other["entries"].as_array().unwrap().len()), (Some("Spare"), 0, 2));
    let shown = ok("de_show", json!({ "key": key, "maps": [["Testdorf", "Testdorf.hof"]] }));
    assert_eq!((shown["view"]["key"].as_str(), shown["error"].is_null()), (Some(key.as_str()), true));
    assert!(ok("de_show", json!({ "key": "oo_nothing" }))["error"].as_str().unwrap().ends_with("|oo_nothing"));
    let gone = ok("de_delete", json!({}));
    assert_eq!((gone["name"].as_str(), gone["entries"].as_array().unwrap().len()), (Some("Testdorf mine"), 1));
    assert!(!file.exists() && base.join("data/depots/deleted").is_dir());
    assert!(call("de_edit", &json!({ "op": "name", "value": "x" }), &Quiet).is_err(), "nothing open");

    // nothing of the OMSI 2 folder was written
    let mut after = Vec::new();
    files(&omsi2, &mut after);
    assert!(before == after, "the OMSI 2 folder changed");
    let _ = std::fs::remove_dir_all(&base);
}
