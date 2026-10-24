//! A company's duty driven end to end, against a FAKE openOMSI (`tests/support/fake_openomsi.rs`,
//! built by Cargo as the `fake-openomsi` program and copied into a folder as `openomsi(.exe)`):
//! the folders set up, the company's part given to the plugin, the drive started through
//! `hub_drive`, the fake game writing the plugin's files as the real plugin would, the bridge
//! looking while it runs, `game-ended` told - and then what was booked where: the trips in the
//! driver's record, the walk-around's defect in the company's queue and, once the company is
//! open, on its bus, the live event in its live file. Nothing of the player's own folders is
//! touched: the data folder, openOMSI and the plugin are in a temporary folder
//! (`OMSIHUB_DATA_DIR`), OMSI 2 is the repository's fixture.

use hub_api::{call, Emitter, Platform};
use hub_core::company::{self as co, remote, store};
use hub_game::bridge;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A platform that keeps what it is told.
#[derive(Default)]
struct Recorder {
    events: Arc<Mutex<Vec<(String, Value)>>>,
}

impl Platform for Recorder {
    fn emit(&self, event: &str, payload: Value) {
        self.events.lock().unwrap().push((event.to_string(), payload));
    }
    fn emitter(&self) -> Option<Emitter> {
        let events = self.events.clone();
        Some(Arc::new(move |e: &str, v: Value| events.lock().unwrap().push((e.to_string(), v))))
    }
}

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(name).canonicalize().unwrap().to_string_lossy().into_owned()
}

/// The fake openOMSI put into `dir` under openOMSI's own program name.
fn fake_openomsi(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let exe = dir.join(if cfg!(windows) { "openomsi.exe" } else { "openomsi" });
    std::fs::copy(env!("CARGO_BIN_EXE_fake-openomsi"), &exe).unwrap();
    // (a release unpacked by some tools loses the executable bit: the app sets it again)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    exe
}

#[test]
fn a_company_duty_is_driven_reported_and_booked() {
    let base = std::env::temp_dir().join(format!("hub-api-drive-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let data = base.join("data");
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, &data);
    std::env::set_var(hub_core::paths::HOME_DIR_ENV, base.join("home"));
    std::env::set_var("FAKE_OPENOMSI_SECONDS", "5");

    // the folders, as the first-run setup keeps them
    let game = base.join("openOMSI-0.2.30-linux-x64");
    fake_openomsi(&game);
    let omsi2 = fixture("OMSI 2");
    let saved = call("hub_setup_save", &json!({ "openomsi": game.to_string_lossy(), "omsi2": omsi2 }), &Recorder::default()).unwrap();
    assert_eq!(saved["ok"], true, "{saved}");
    let state = call("addon_state", &json!({}), &Recorder::default()).unwrap();
    assert_eq!((state["game"]["version"].as_str(), state["checked"].as_bool()), (Some("0.2.30"), Some(false)), "{state}");
    // (not yet checked with the game itself: estimated from the version until the first drive)
    assert_eq!(state["game"]["compat"]["confirmed"], false, "{state}");

    // the player's company, with a bus of its own
    let driver = hub_api::launcher::driver();
    let mut c = co::found(&co::Founding { name: "Stadtbus Test".into(), date: "2024-03-04".into(), map: "maps/Grundorf/global.cfg".into(), ..Default::default() }, &driver);
    c.cash += 5_000_000_00;
    let bus = co::market::MarketBus { file: "Vehicles/MAN_SD200/SD202.bus".into(), name: "SD202".into(), ..Default::default() };
    let bus_id = co::market::buy_new(&mut c, &bus, co::market::Payment::Cash, "").unwrap();
    store::save(&data, &c).unwrap();

    // a company's duty (what `co_act` `launch_start` does): its part given to the plugin, then
    // the drive - line 5, tour 2 from its third trip, a relief on the way after two trips
    let duty = hub_core::Duty {
        map: c.map.clone(),
        bus: bus.file.clone(),
        time: "07:55".into(),
        date: Some(c.date.clone()),
        line: Some("Linie5".into()),
        tour: Some("2".into()),
        trip: Some("3".into()),
        whole_tour: true,
        legs: vec!["Linie5|2|3|2@4".into()],
        entry: Some(-1),
        company: Some(c.id.clone()),
        display_font: Some("Gelb Retro".into()),
        ..Default::default()
    };
    let crew = bridge::Crew { bus: bus_id, bus_number: "17".into(), ..Default::default() };
    let parts = hub_api::company::plugin::Parts { sheet: vec![bridge::SheetRow { kind: "sign_on".into(), at: "Depot".into(), from: 470, to: 470, ..Default::default() }], ..Default::default() };
    hub_api::company::plugin::assign_duty(&duty, Some((&c.name, "5 2 1", parts)), Some(crew), Some(&c.id));
    let p = Recorder::default();
    let started = call("hub_drive", &json!({ "duty": duty }), &p).unwrap();
    assert_eq!(started["started"], true, "{started}");
    let issued = started["issued"].as_u64().expect("an assignment was given");

    // the plugin was put in and given the duty: the company, its parts, the legs, the font
    let plugin = game.join("Plugins").join("omsihub");
    let lua = std::fs::read_to_string(plugin.join(bridge::ASSIGNMENT_FILE)).unwrap();
    for want in ["Stadtbus Test", "5 2 1", "Gelb Retro", "[\"legs\"] = {", "[\"to_stop\"] = 5", "[\"sign_on\"]", &format!("[\"issued\"] = {issued}")] {
        assert!(lua.contains(want) || (want == "[\"sign_on\"]" && lua.contains("\"sign_on\"")), "{want} not in the assignment:\n{lua}");
    }
    assert!(plugin.join(bridge::TEXTS_FILE).is_file() && plugin.join("main.lua").is_file() && plugin.join("legs.lua").is_file());
    assert!(!lua.contains("--company") && !lua.contains("--duty-leg"));

    // the game runs; the bridge looks; the game ends
    let end = Instant::now() + Duration::from_secs(90);
    let ended = loop {
        if let Some(e) = p.events.lock().unwrap().iter().find(|(e, _)| e == "game-ended").cloned() {
            break e.1;
        }
        assert!(Instant::now() < end, "the game never ended");
        std::thread::sleep(Duration::from_millis(200));
    };
    assert_eq!(ended["code"], 0, "{ended}");
    assert_eq!((ended["booked"]["trips"].as_u64(), ended["booked"]["orders"].as_u64(), ended["booked"]["live"].as_u64()), (Some(2), Some(1), Some(1)), "{ended}");
    assert_eq!(call("hub_game_state", &json!({}), &p).unwrap()["running"], false);

    // official flags only, the duty's first part on them
    let args = std::fs::read_to_string(plugin.join("data").join("fake-args.txt")).unwrap();
    let args: Vec<&str> = args.lines().collect();
    for a in args.iter().filter(|a| a.starts_with("--")) {
        assert!(hub_game::launch::OFFICIAL_FLAGS.contains(a), "{a} is not an official flag: {args:?}");
    }
    let has = |f: &str, v: &str| args.windows(2).any(|w| w[0] == f && w[1] == v);
    assert!(has("--line", "Linie5") && has("--tour", "2") && has("--trip", "3") && args.contains(&"--auto-entry"), "{args:?}");

    // the trips in the driver's record (the careers read it), for the duty's line and tour
    let trips = hub_core::trips_of(&data, &driver);
    assert_eq!(trips.len(), 2, "{trips:?}");
    assert!(trips.iter().all(|t| t.line == "Linie5" && t.tour == "2" && t.completed && t.bus == bus.file && t.map == c.map), "{trips:?}");
    let mut numbers: Vec<usize> = trips.iter().map(|t| t.trip).collect();
    numbers.sort();
    assert_eq!(numbers, [3, 4]);
    // the company's live event in its live file, its order waiting in its queue
    let live = store::peek_live(&data, &c.id, 0);
    assert!(matches!(live.as_slice(), [co::day::LiveEvent::Breakdown { vehicle }] if *vehicle == bus_id), "{live:?}");
    assert_eq!(remote::pending(&data, &c.id).len(), 1);
    // the handshake: this openOMSI is now checked with the game itself
    let state = call("addon_state", &json!({}), &p).unwrap();
    assert_eq!((state["checked"].as_bool(), state["game"]["compat"]["confirmed"].as_bool(), state["game"]["compat"]["level"].as_str()), (Some(true), Some(true), Some("supported")), "{state}");
    assert_eq!(state["assigned"]["company_id"], c.id.as_str());
    // (nothing booked twice)
    assert_eq!(call("addon_reports", &json!({}), &p).unwrap()["reports"], 0);

    // the company opened: the driver's defect on its bus, the queue empty
    call("co_boot", &json!({}), &p).unwrap();
    call("co_open", &json!({ "id": c.id }), &p).unwrap();
    call("co_tick", &json!({}), &p).unwrap();
    assert!(remote::pending(&data, &c.id).is_empty(), "the order was carried out");
    let back = store::load(&data, &c.id).unwrap();
    let d = back.crew.defects.iter().find(|d| d.vehicle == bus_id).expect("the defect on the company's bus");
    assert_eq!((d.kind, d.text.as_str()), (co::crew::DefectKind::Tyres, "front left"));

    call("co_leave", &json!({}), &p).unwrap();
    let _ = std::fs::remove_dir_all(&base);
}
