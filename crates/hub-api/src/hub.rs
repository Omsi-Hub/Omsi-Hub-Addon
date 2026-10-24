//! The app's own commands (`hub_`): the boot, the texts, the first-run setup, the old saves,
//! and the drives (`drive`).
//!
//! * `hub_version` - the app's, the plugin's and the bridge's versions, and the oldest openOMSI;
//! * `hub_boot` - what the page needs first: the settings, whether the setup is needed;
//! * `hub_texts {lang}` - every text in `lang` (`hub-locales`), for `ui/js/i18n.js`;
//! * `hub_setup_state` - the folders chosen and what is in them, and the old saves found;
//! * `hub_setup_suggest` - folders found on this computer to offer (read only);
//! * `hub_setup_openomsi {folder}`, `hub_setup_omsi2 {folder}` - one folder checked, not kept;
//! * `hub_setup_save {openomsi, omsi2, language?}` - both checked again and kept;
//! * `hub_migrate_scan`, `hub_migrate_import`, `hub_migrate_skip` - the old saves (`~/.openomsi`);
//! * `hub_drive {duty}`, `hub_drive_args {duty}`, `hub_game_state`, `hub_game_stop` - see
//!   [`crate::drive`]; `hub_live` - the live dashboard of the drive that runs ([`crate::live`]),
//!   `hub_live_pos` where its bus is now;
//! * `hub_log {line}` - a line into Omsi-Hub's log; `hub_quit` - end the app.

use crate::{arg, drive, to_json, Answer, Platform};
use hub_core::migrate;
use hub_core::settings::Settings;
use hub_game::compat::Level;
use serde_json::{json, Value};
use std::path::Path;

/// A `hub_` command.
pub fn call(cmd: &str, args: &Value, p: &dyn Platform) -> Answer {
    match cmd {
        "hub_version" => Ok(version()),
        "hub_boot" => Ok(boot()),
        "hub_texts" => {
            let lang = arg::<Option<String>>(args, "lang")?.unwrap_or_default();
            to_json(hub_locales::texts(&lang))
        }
        "hub_setup_state" => Ok(setup_state()),
        "hub_setup_suggest" => Ok(suggest()),
        "hub_setup_openomsi" => Ok(check_openomsi(&arg::<String>(args, "folder")?)),
        "hub_setup_omsi2" => Ok(check_omsi2(&arg::<String>(args, "folder")?)),
        "hub_setup_save" => setup_save(&arg::<String>(args, "openomsi")?, &arg::<String>(args, "omsi2")?, arg::<Option<String>>(args, "language")?),
        "hub_migrate_scan" => {
            let found = migrate::scan(&migrate::old_folder());
            Ok(json!({ "folder": found.folder, "items": found.items, "companies": found.companies, "careers": found.careers, "preview": migrate::preview(&found) }))
        }
        "hub_migrate_import" => {
            let found = migrate::scan(&migrate::old_folder());
            let n = migrate::import(&found)?;
            Ok(json!({ "imported": n, "preview": migrate::preview(&found), "settings": Settings::load() }))
        }
        "hub_migrate_skip" => {
            let mut s = Settings::load();
            s.migrated = true;
            s.save().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "hub_drive" => drive::start(&arg::<hub_core::Duty>(args, "duty")?, p),
        "hub_drive_args" => drive::args(&arg::<hub_core::Duty>(args, "duty")?),
        "hub_game_state" => Ok(drive::state()),
        "hub_game_stop" => Ok(drive::stop(p)),
        "hub_live" => Ok(crate::live::view()),
        "hub_live_pos" => Ok(crate::live::pos()),
        "hub_log" => {
            hub_core::log_line(&format!("page: {}", arg::<String>(args, "line")?));
            Ok(Value::Null)
        }
        "hub_quit" => {
            p.quit();
            Ok(Value::Null)
        }
        // (the drive's steps: the maps, buses, lines, shifts, weather, depots … `content`)
        _ => crate::content::call(cmd, args, p),
    }
}

/// A `hub_` command that answers bytes (`content`: the map's picture, a font's).
pub fn call_bytes(cmd: &str, args: &Value, p: &dyn Platform) -> Result<Vec<u8>, String> {
    crate::content::call_bytes(cmd, args, p)
}

fn version() -> Value {
    json!({
        "app": env!("CARGO_PKG_VERSION"),
        "min_openomsi": hub_game::compat::FIRST_SUPPORTED.to_string(),
        "plugin": hub_game::install::VERSION,
        "bridge": hub_game::bridge::BRIDGE,
    })
}

fn boot() -> Value {
    let s = Settings::load();
    json!({
        "version": version(),
        "data_dir": hub_core::data_dir(),
        "settings": s,
        "setup_needed": !s.setup_done || !s.has_folders(),
        "languages": hub_locales::languages(),
    })
}

/// The openOMSI in `folder`: found or why not, its version, and whether Omsi-Hub works with it.
pub fn check_openomsi(folder: &str) -> Value {
    let folder = folder.trim();
    if folder.is_empty() {
        return json!({ "ok": false, "problem": "missing", "text": "Choose the folder openOMSI is in" });
    }
    // (the version asked of the program once where no file says it: Linux's usual case)
    let cache = hub_core::data_dir().join("openomsi-versions.json");
    match hub_game::check_folder_probing(Path::new(folder), &hub_core::paths::home_dir(), &cache) {
        Err(problem) => json!({ "ok": false, "problem": problem, "text": problem.text() }),
        Ok(c) => {
            let too_old = c.install.version.as_ref().is_some_and(|v| *v < hub_game::compat::FIRST_SUPPORTED);
            let ok = c.compat.level != Level::Unsupported;
            let (say, vars) = if too_old {
                let version = c.install.version.as_ref().map(|v| v.to_string()).unwrap_or_default();
                ("This openOMSI is %{version}: Omsi-Hub needs %{needs} or newer. Update openOMSI.".to_string(), json!({ "version": version, "needs": hub_game::compat::FIRST_SUPPORTED.to_string() }))
            } else if c.install.omsihub_build {
                ("This is the old Omsi-Hub's own game: choose the folder of the official openOMSI".to_string(), json!({}))
            } else {
                (c.compat.notes.first().cloned().unwrap_or_default(), json!({}))
            };
            json!({
                "ok": ok && !c.install.omsihub_build,
                "too_old": too_old,
                "text": filled(&say, &vars),
                "say": say,
                "vars": vars,
                "version": c.install.version,
                "version_from": c.install.version_from,
                "exe": c.install.exe,
                "plugins_dir": c.install.plugins_dir,
                "compat": c.compat,
            })
        }
    }
}

/// The OMSI 2 in `folder`: whether it is a complete original install.
pub fn check_omsi2(folder: &str) -> Value {
    let folder = folder.trim();
    if folder.is_empty() || !Path::new(folder).is_dir() {
        return json!({ "ok": false, "missing": [], "text": "This folder does not exist" });
    }
    let missing = hub_omsi::vfs::missing_omsi2_essentials(Path::new(folder));
    let (say, vars) = if missing.is_empty() { (String::new(), json!({})) } else { ("This is not the OMSI 2 folder: %{missing} not found".to_string(), json!({ "missing": missing.join(", ") })) };
    json!({ "ok": missing.is_empty(), "missing": missing, "text": filled(&say, &vars), "say": say, "vars": vars })
}

/// A check's text in English, its `%{name}`s filled from `vars` (the page says it translated:
/// `say` and `vars`).
fn filled(say: &str, vars: &Value) -> String {
    let mut t = say.to_string();
    for (k, v) in vars.as_object().into_iter().flatten() {
        t = t.replace(&format!("%{{{k}}}"), v.as_str().unwrap_or_default());
    }
    t
}

fn setup_state() -> Value {
    let s = Settings::load();
    let found = migrate::scan(&migrate::old_folder());
    json!({
        "settings": s,
        "openomsi": (!s.openomsi_folder.is_empty()).then(|| check_openomsi(&s.openomsi_folder)),
        "omsi2": (!s.omsi2_folder.is_empty()).then(|| check_omsi2(&s.omsi2_folder)),
        "migrate": { "offer": !s.migrated && !found.is_empty(), "folder": found.folder, "companies": found.companies, "careers": found.careers, "files": found.items.len(), "preview": migrate::preview(&found) },
    })
}

fn suggest() -> Value {
    let omsi2: Vec<String> = hub_game::find::omsi2_candidates().into_iter().filter(|p| hub_omsi::vfs::missing_omsi2_essentials(p).is_empty()).map(|p| p.to_string_lossy().into_owned()).collect();
    let home = hub_core::paths::home_dir();
    // (the openOMSI openOMSI's own launcher plays first: its mods are the player's)
    let played = hub_core::paths::launcher_openomsi(&home);
    let others = hub_game::find::openomsi_candidates(&home, omsi2.first().map(Path::new)).into_iter().filter(|p| Some(p) != played.as_ref());
    let openomsi: Vec<String> = played.clone().into_iter().chain(others).map(|p| p.to_string_lossy().into_owned()).collect();
    json!({ "openomsi": openomsi, "omsi2": omsi2 })
}

fn setup_save(openomsi: &str, omsi2: &str, language: Option<String>) -> Answer {
    let o = check_openomsi(openomsi);
    if o["ok"] != true {
        return Err(o["text"].as_str().unwrap_or("openOMSI was not found").to_string());
    }
    let r = check_omsi2(omsi2);
    if r["ok"] != true {
        return Err(r["text"].as_str().unwrap_or("OMSI 2 was not found").to_string());
    }
    let mut s = Settings::load();
    s.openomsi_folder = openomsi.trim().to_string();
    s.omsi2_folder = omsi2.trim().to_string();
    if let Some(l) = language.filter(|l| !l.is_empty()) {
        s.language = l;
    }
    s.setup_done = true;
    s.save().map_err(|e| format!("the settings could not be saved: {e}"))?;
    hub_core::log_line(&format!("setup: openOMSI {} in {}, OMSI 2 in {}", o["version"].as_str().unwrap_or("?"), s.openomsi_folder, s.omsi2_folder));
    Ok(json!({ "ok": true, "settings": s, "openomsi": o }))
}
