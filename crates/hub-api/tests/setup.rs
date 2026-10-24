//! The first-run setup against the fake folders of `fixtures/`: an openOMSI found and its
//! version read, one too old refused, the OMSI 2 folder checked, and the choice kept in
//! Omsi-Hub's own data folder (a temporary one here, `OMSIHUB_DATA_DIR`).

use hub_api::{call, Quiet};
use serde_json::json;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(name).canonicalize().unwrap().to_string_lossy().into_owned()
}

#[test]
fn the_setup_checks_and_keeps_the_folders() {
    let data = std::env::temp_dir().join(format!("hub-api-setup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, &data);
    std::env::set_var(hub_core::paths::HOME_DIR_ENV, data.join("home"));

    let boot = call("hub_boot", &json!({}), &Quiet).unwrap();
    assert_eq!(boot["setup_needed"], true);

    let good = fixture("openOMSI-0.2.30-windows-x64");
    let old = fixture("openOMSI-0.2.10-windows-x64");
    let omsi2 = fixture("OMSI 2");

    let o = call("hub_setup_openomsi", &json!({ "folder": good }), &Quiet).unwrap();
    assert_eq!((o["ok"].clone(), o["version"].clone(), o["version_from"].clone()), (json!(true), json!("0.2.30"), json!("folder_name")), "{o}");
    assert_eq!(o["compat"]["level"], "supported");

    let too_old = call("hub_setup_openomsi", &json!({ "folder": old }), &Quiet).unwrap();
    assert_eq!((too_old["ok"].clone(), too_old["too_old"].clone()), (json!(false), json!(true)));
    assert!(too_old["text"].as_str().unwrap().contains("0.2.22"), "{too_old}");

    // (the OMSI 2 folder is not openOMSI's, and openOMSI's is not OMSI 2's)
    let wrong = call("hub_setup_openomsi", &json!({ "folder": omsi2 }), &Quiet).unwrap();
    assert_eq!(wrong["problem"], "omsi2_only");
    assert_eq!(call("hub_setup_omsi2", &json!({ "folder": good }), &Quiet).unwrap()["ok"], false);
    assert_eq!(call("hub_setup_omsi2", &json!({ "folder": omsi2 }), &Quiet).unwrap()["ok"], true);

    // refused with an old openOMSI; kept with a good one
    assert!(call("hub_setup_save", &json!({ "openomsi": old, "omsi2": omsi2 }), &Quiet).unwrap_err().contains("0.2.22"));
    let saved = call("hub_setup_save", &json!({ "openomsi": good, "omsi2": omsi2, "language": "nl" }), &Quiet).unwrap();
    assert_eq!(saved["ok"], true);
    let file = std::fs::read_to_string(data.join("settings.json")).unwrap();
    assert!(file.contains("openOMSI-0.2.30-windows-x64") && file.contains("\"language\": \"nl\""), "{file}");

    let boot = call("hub_boot", &json!({}), &Quiet).unwrap();
    assert_eq!(boot["setup_needed"], false);
    let state = call("addon_state", &json!({}), &Quiet).unwrap();
    assert_eq!(state["game"]["version"], "0.2.30");
    assert_eq!(state["plugin"]["state"], "missing");

    // the drive's command line, official flags only
    let duty = json!({ "map": "maps/Grundorf/global.cfg", "bus": "Vehicles/X/a.bus", "time": "08:15" });
    let args = call("hub_drive_args", &json!({ "duty": duty }), &Quiet).unwrap();
    assert_eq!(args["args"][0], "--root");
    assert_eq!(args["args"][1], omsi2);

    let _ = std::fs::remove_dir_all(&data);
}
