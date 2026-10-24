//! The player's game controllers (`addon_controllers`), on `hub_game::controllers`: the Settings
//! page's tab "Controllers" - the wheel, the pedals, joysticks, shifters and button boxes as
//! openOMSI keeps them in `Inputs/gamectrler.cfg`.
//!
//! * `addon_controllers` - the devices of the file openOMSI reads (`path`, and where it came
//!   from: `from` is `openomsi`, `content` or `omsi2`, null when there is none yet), the file
//!   Omsi-Hub writes (`save_to`), the key actions a button can press (`actions`, from the
//!   `keyboard.cfg` in use) and OMSI 2's names of them in the player's language (`names`,
//!   the actions in lower case), and whether the game runs;
//! * `addon_controllers_save {devices}` - the devices written (all of them, as the page has
//!   them: a device left out is taken out of the file); `written` says where.
//!
//! What a device's axes and buttons do now is the page's own (the webview's game controllers:
//! `ui/js/hub/controllers.js`); the settings round them (dead zone, pedals, force feedback,
//! gamepad) are openOMSI's `settings.cfg`, the groups `CONTROLLER_GROUPS` of `addon_game_settings`.

use crate::{arg, Answer};
use hub_core::settings::Settings;
use hub_game::controllers::{self as ctl, Device, Places};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A command of the tab.
pub fn call(cmd: &str, args: &Value) -> Answer {
    let s = Settings::load();
    let places = places(&hub_core::paths::home_dir(), hub_core::paths::content_dir(), &s.omsi2_folder);
    match cmd {
        "addon_controllers" => Ok(state(&places, &crate::addon::player_language(), running())),
        "addon_controllers_save" => {
            let devices = arg::<Vec<Device>>(args, "devices")?;
            let written = ctl::save(&places, &devices)?;
            hub_core::log_line(&format!("controllers: {} devices written to {}", devices.len(), written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")));
            Ok(json!({ "written": written, "path": places.in_use(ctl::FILE) }))
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// A game started from Omsi-Hub is running.
fn running() -> bool {
    crate::drive::state()["running"].as_bool().unwrap_or(false)
}

fn places(home: &Path, content: Option<PathBuf>, omsi2: &str) -> Places {
    let omsi2 = Some(omsi2.trim()).filter(|o| !o.is_empty()).map(PathBuf::from);
    Places::new(home, content.as_deref(), omsi2.as_deref())
}

/// The tab's state (see the module's text).
pub fn state(places: &Places, lang: &str, running: bool) -> Value {
    let (devices, path, error) = match ctl::load(places) {
        Ok((d, p)) => (d, p, None),
        Err(e) => (Vec::new(), places.in_use(ctl::FILE), Some(e)),
    };
    let from = path.as_deref().map(|p| {
        if p.starts_with(&places.own) {
            "openomsi"
        } else if places.content.as_deref().is_some_and(|c| p.starts_with(c)) {
            "content"
        } else {
            "omsi2"
        }
    });
    let names = places.omsi2.as_deref().map(|o| ctl::action_names(o, ctl::names_language(lang))).unwrap_or_default();
    json!({
        "path": path,
        "from": from,
        "save_to": places.targets(ctl::FILE).first(),
        "error": error,
        "devices": devices,
        "actions": ctl::key_actions(places),
        "names": names,
        "running": running,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("omsihub-controllers-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_tab_shows_the_file_in_use_its_actions_and_their_names() {
        let root = temp("state");
        let omsi2 = root.join("OMSI 2");
        std::fs::create_dir_all(omsi2.join("Inputs")).unwrap();
        std::fs::create_dir_all(omsi2.join("Languages")).unwrap();
        std::fs::write(omsi2.join("Inputs").join("keyboard.cfg"), "[vehicles]\r\n\r\n[entry]\r\nhorn\r\n35\r\n0\r\n").unwrap();
        std::fs::write(omsi2.join("Languages").join("ENG_key_veh_gen.olf"), "ENG\t\r\nKY_horn\tHorn\r\n").unwrap();
        let p = places(&root.join("home"), None, &omsi2.to_string_lossy());
        let v = state(&p, "nl", false);
        assert_eq!((v["from"].clone(), v["devices"].clone()), (Value::Null, json!([])), "no file yet");
        assert_eq!((v["actions"].clone(), v["names"]["horn"].clone()), (json!(["horn"]), json!("Horn")));
        assert!(v["save_to"].as_str().unwrap().replace('\\', "/").ends_with(".openomsi/Inputs/gamectrler.cfg"));
        // (a wheel set up: saved where openOMSI reads it, and read back from there)
        let mut wheel = Device::new("MOZA R5 Base");
        wheel.axes[0] = [0, 0];
        wheel.buttons = vec![("horn".into(), "1".into())];
        ctl::save(&p, &[wheel.clone()]).unwrap();
        let v = state(&p, "nl", true);
        assert_eq!((v["from"].as_str(), v["running"].as_bool()), (Some("openomsi"), Some(true)));
        assert_eq!(v["devices"][0]["name"], "MOZA R5 Base");
        assert_eq!(v["devices"][0]["axes"][0], json!([0, 0]));
        assert_eq!(v["devices"][0]["buttons"][0], json!(["horn", "1"]));
        // (the page's devices as JSON, back into the file)
        let back: Vec<Device> = serde_json::from_value(v["devices"].clone()).unwrap();
        assert_eq!(back[0].buttons, wheel.buttons);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The texts of a page's source: every string literal that reads as an English text (a
    /// capital, then small letters) - the tables of names, the labels, what `tr` is given.
    pub(crate) fn page_texts(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut chars = src.chars().peekable();
        let mut last = ' ';
        while let Some(c) = chars.next() {
            // (comments skipped: their words are no texts, and their quotes would pair wrongly)
            if c == '/' && last != ':' && chars.peek() == Some(&'/') {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
                last = '\n';
                continue;
            }
            if c == '/' && chars.peek() == Some(&'*') {
                let mut prev = ' ';
                for c in chars.by_ref() {
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
                continue;
            }
            last = c;
            if c != '\'' && c != '"' {
                continue;
            }
            let mut t = String::new();
            while let Some(x) = chars.next() {
                match x {
                    '\\' => t.extend(chars.next()),
                    x if x == c => break,
                    x => t.push(x),
                }
            }
            if t.chars().next().is_some_and(|f| f.is_ascii_uppercase()) && t.chars().any(|x| x.is_ascii_lowercase()) {
                out.push(t);
            }
        }
        out
    }

    /// Every text of the tab "Controllers" (and of the openOMSI tab beside it), and openOMSI's
    /// settings round the controllers, in the seven languages (crates/hub-locales/locales/
    /// controllers.yml).
    #[test]
    fn every_controllers_text_is_translated() {
        let js = include_str!("../../../ui/js/hub/controllers.js").replace("\r\n", "\n");
        let addon = include_str!("../../../ui/js/hub/addon.js").replace("\r\n", "\n");
        let mut texts: Vec<String> = page_texts(&js).into_iter().chain(page_texts(&addon)).filter(|t| t != "Escape").collect();
        // (a hat's directions, in "Hat 1 up")
        texts.extend(["up", "right", "down", "left"].map(String::from));
        for f in hub_game::gamesettings::FIELDS.iter().filter(|f| hub_game::gamesettings::CONTROLLER_GROUPS.contains(&f.group)) {
            texts.push(f.group.to_string());
            texts.push(f.label.to_string());
        }
        texts.sort();
        texts.dedup();
        assert!(texts.len() > 150, "only {} texts found", texts.len());
        // (the names of things are the same in every language)
        let names = ["Xbox", "PlayStation 4", "PlayStation 5", "Steam, Steam with Proton, or retail"];
        let texts: Vec<String> = texts.into_iter().filter(|t| !names.contains(&t.as_str())).collect();
        let mut missing = Vec::new();
        for lang in ["nl", "de", "fr", "ru", "uk", "pl", "es"] {
            let table = hub_locales::texts(lang);
            for t in &texts {
                match table.get(t) {
                    Some(tr) if t.matches("%{").count() == tr.matches("%{").count() => {}
                    _ => missing.push(format!("{lang}: {t:?}")),
                }
            }
        }
        assert!(missing.is_empty(), "{} missing or with other holes (crates/hub-locales/locales/controllers.yml):\n{}", missing.len(), missing.join("\n"));
    }
}
