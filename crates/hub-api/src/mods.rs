//! Mods installed into the player's openOMSI (`hub_mods`), with openOMSI's own installer
//! (`omsi-launcher-core`'s `install` and `mods`): the Settings page's tab "Mods".
//!
//! A mod is a `.zip` or `.7z`, or a folder - a `.rar` only where the installer was built with its
//! feature `rar`, which Omsi-Hub's is not (unrar-rs is GPL-3.0): it is refused with what to do
//! instead. openOMSI's installer works out what it is
//! (a bus, a map, scenery, a whole OMSI folder), checks that it fits on the disk, unpacks it
//! beside the content folder and moves it into place - or, a `.zip` too big to unpack, puts it
//! into `Archives` to be read as it is - and notes it in `Mods/.mods.json`. So a mod installed
//! here is on openOMSI's own Mods page as well, and the other way round; the original OMSI 2
//! folder is never written to. The content folder is the one of the openOMSI in use
//! (`hub_core::paths::content_dir`), which is where Omsi-Hub's lists read the mods from.
//!
//! * `hub_mods` - the content folder, the installs of this run (newest first: `jobs`, each
//!   with its `state` - queued, planning, checking, unpacking, copying, moving, done, failed,
//!   cancelled - its files and bytes, its `message` and `report`) and the mods in the content
//!   folder (`mods`: id, name, kind - bus, map, archive, other - paths, on or off, bytes), and
//!   whether a `.rar` can be installed (`rar`);
//! * `hub_mods_inspect {path}` - how big the mod is unpacked, what is free, and whether it fits;
//! * `hub_mods_install {path, mode?}` - an install started (`mode`: auto - the default -,
//!   extract or inplace); it runs on its own thread, followed with `hub_mods`;
//! * `hub_mods_cancel {id}`; `hub_mods_clear` - the finished installs forgotten;
//! * `hub_mods_enable {id, on}` - a mod switched off (its folders to `Mods/disabled`) or on;
//!   `hub_mods_remove {id}` - deleted. Neither while the game runs (its files are open).

use crate::{arg, Answer};
use omsi_launcher_lib::{install, mods};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The content folder of the openOMSI in use.
fn content() -> Result<PathBuf, String> {
    hub_core::paths::content_dir().ok_or_else(|| "Choose your openOMSI folder first".to_string())
}

/// What the tab shows.
pub fn state(content: &Path) -> Value {
    json!({
        "content": content,
        "jobs": install::jobs(),
        "mods": mods::list(content),
        "running": game_running(),
        "rar": install::RAR,
    })
}

/// A game started from Omsi-Hub is running.
fn game_running() -> bool {
    crate::drive::state()["running"].as_bool().unwrap_or(false)
}

/// What a `.rar` gets: Omsi-Hub's installer does not read one.
const NO_RAR: &str = "A .rar cannot be installed here: unpack it first (with 7-Zip or WinRAR) and install the folder, or use a .zip or .7z";

/// A mod to install: a folder, or a `.zip` or `.7z` that is there (a `.rar` only when the
/// installer reads one, `install::RAR`).
fn source(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path.trim());
    let ext = |e: &str| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(e));
    if p.is_file() && ext("rar") && !install::RAR {
        return Err(NO_RAR.into());
    }
    let archive = ext("zip") || ext("7z") || ext("rar");
    if p.is_dir() || (p.is_file() && archive) {
        Ok(p)
    } else if p.exists() {
        Err("A mod is a folder, or a .zip or .7z file".into())
    } else {
        Err(format!("{} is not there", p.display()))
    }
}

/// Install the mod at `src` into `content` (on its own thread): the install as it starts.
pub fn start(content: &Path, src: &Path, mode: &str) -> install::Progress {
    // (one in the inbox, `Mods/`: openOMSI's launcher may install it too - its files are linked, not copied)
    let from_inbox = src.starts_with(content.join("Mods"));
    let job = install::start(content.to_path_buf(), hub_core::paths::root(), src.to_path_buf(), install::InstallMode::parse(mode), from_inbox);
    hub_core::log_line(&format!("mods: installing {} ({mode}) into {}", src.display(), content.display()));
    job.snapshot()
}

/// A command of the tab.
pub fn call(cmd: &str, args: &Value) -> Answer {
    let content = content()?;
    match cmd {
        "hub_mods" => Ok(state(&content)),
        "hub_mods_inspect" => {
            let src = source(&arg::<String>(args, "path")?)?;
            crate::to_json(install::inspect(&content, hub_core::paths::root().as_deref(), &src).map_err(|e| format!("{e:#}"))?)
        }
        "hub_mods_install" => {
            let src = source(&arg::<String>(args, "path")?)?;
            if install::is_busy(&src) {
                return Err("This mod is being installed already".into());
            }
            let mode = arg::<Option<String>>(args, "mode")?.filter(|m| !m.trim().is_empty()).unwrap_or_else(|| "auto".into());
            let job = start(&content, &src, &mode);
            Ok(json!({ "job": job, "state": state(&content) }))
        }
        "hub_mods_cancel" => {
            let id = arg::<u64>(args, "id")?;
            Ok(json!({ "cancelled": install::cancel(id), "state": state(&content) }))
        }
        "hub_mods_clear" => {
            install::clear_finished();
            Ok(state(&content))
        }
        "hub_mods_enable" | "hub_mods_remove" => {
            if game_running() {
                return Err("Close openOMSI first: the game has the mods' files open".into());
            }
            let id = arg::<String>(args, "id")?;
            let name = if cmd == "hub_mods_remove" {
                mods::remove(&content, &id)
            } else {
                mods::set_enabled(&content, &id, arg::<bool>(args, "on")?)
            }
            .map_err(|e| format!("{e:#}"))?;
            hub_core::log_line(&format!("mods: {cmd} {name}"));
            Ok(json!({ "name": name, "state": state(&content) }))
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

#[cfg(test)]
mod tests {
    /// Every text of the tab "Mods" and what the commands answer to it, in the seven languages
    /// (crates/hub-locales/locales/mods.yml).
    #[test]
    fn every_mods_text_is_translated() {
        let js = include_str!("../../../ui/js/hub/mods.js").replace("\r\n", "\n");
        let mut texts = crate::controllers::tests::page_texts(&js);
        texts.extend(["Mods", "Mods in openOMSI", "A mod is a folder, or a .zip or .7z file", super::NO_RAR, "This mod is being installed already", "Close openOMSI first: the game has the mods' files open"].map(String::from));
        texts.sort();
        texts.dedup();
        assert!(texts.len() > 30, "only {} texts found", texts.len());
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
        assert!(missing.is_empty(), "{} missing or with other holes (crates/hub-locales/locales/mods.yml):\n{}", missing.len(), missing.join("\n"));
    }
}
