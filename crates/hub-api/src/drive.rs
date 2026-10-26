//! A drive started in the player's openOMSI: the page's [`Duty`] made into the official command
//! line ([`launch_spec`]), the plugin given its assignment, the game started from the openOMSI
//! folder of the settings, the bridge looked at while it runs, and its end told to the page.
//!
//! **The drive's flow** ([`start`]): a company's duty (`co_act` `launch_start`, which gives the
//! plugin the company's part first: `company::plugin::assign_duty`), a shift, a tour and a free
//! drive (`hub_drive`) all come here:
//!
//! 1. the openOMSI of the settings checked (its version; one Omsi-Hub does not work with is
//!    refused);
//! 2. the plugin made ready (`addon::ready`: put in or brought up to date, unless the player
//!    parked it off) and given the drive's assignment (`addon::give`): what the command line
//!    cannot say - the company and its duty, the duty's parts (legs), a free route, the sheet,
//!    the controls, the fare, the contract, the crew, the display font and the IBIS;
//! 3. the game started with official flags only (`hub_game::launch::LaunchSpec`), and the page
//!    told (`game-started`, with the way it is driven: the live dashboard, `live.rs`);
//! 4. **the bridge's loop** on a thread of its own, every [`EVERY`] while the game runs: the
//!    plugin's reports booked (`addon::book`), the company told (`company::after_booking`), its
//!    own work done (the replies, the status, the inbox for the plugin: `messages::tick`), the
//!    on-time brief written (`company::ontime::refresh`), the company's buses for the game's AI
//!    (`company::fleetgame::refresh`), the live dashboard sampled
//!    (`live::sample`); the notices go to the plugin as they come (`addon::notify_game`);
//! 5. the game ended: the reports booked once more, then the page told (`game-ended`, with what
//!    was booked).

use crate::{to_json, Answer, Platform};
use hub_core::Duty;
use hub_game::launch::{Lan, LaunchSpec};
use hub_game::process::{self, Game};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The game Omsi-Hub started (one at a time).
static GAME: Mutex<Option<Game>> = Mutex::new(None);
/// How often the bridge's loop looks while the game runs.
pub const EVERY: Duration = Duration::from_secs(2);

fn some(s: &Option<String>) -> Option<String> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

/// The command line of a drive: official flags only.
pub fn launch_spec(d: &Duty, root: &Path) -> LaunchSpec {
    let mut s = LaunchSpec { root: Some(root.to_path_buf()), ..LaunchSpec::default() };
    // (a tutorial or a situation is all the game needs)
    if let Some(t) = d.tutorial {
        s.tutorial = Some(t);
        return s;
    }
    // (the driver whose personnel file the game writes, as the old launcher started it)
    #[cfg(feature = "native")]
    let driver = crate::drivers::launch_driver(d.profile.as_deref());
    #[cfg(not(feature = "native"))]
    let driver: Option<String> = None;
    s.driver = driver.as_ref().map(|n| format!("Drivers/{n}.odr"));
    if let Some(sit) = some(&d.situation) {
        s.situation = Some(sit);
        return s;
    }
    s.map = Some(d.map.clone()).filter(|m| !m.trim().is_empty());
    s.bus = Some(d.bus.clone()).filter(|b| !b.trim().is_empty());
    s.paint = some(&d.paint);
    s.plate = some(&d.plate);
    s.number = some(&d.number);
    s.hof = some(&d.hof);
    s.set_vars = d.set_vars.clone();
    // (-1: the entry point nearest to the duty's first stop, the game's own choice)
    match d.entry {
        Some(e) if e < 0 => s.auto_entry = true,
        Some(e) => s.entry = usize::try_from(e).ok(),
        None => {}
    }
    s.date = some(&d.date);
    s.time = Some(d.time.trim().to_string()).filter(|t| !t.is_empty());
    s.weather = some(&d.weather);
    s.season = some(&d.season);
    s.traffic = d.traffic;
    s.passengers = d.passengers == Some(true);
    s.schedule = d.schedule == Some(true);
    s.autostart = d.autostart == Some(true);
    s.on_foot = d.on_foot == Some(true);
    // (a free drive along a line has no tour: the plugin follows the route)
    if d.free_line.is_none() {
        s.line = some(&d.line);
        s.tour = some(&d.tour).filter(|_| s.line.is_some());
        s.trip = some(&d.trip).filter(|_| s.tour.is_some());
        s.whole_tour = d.whole_tour && s.trip.is_some();
        // (a duty of several parts: the game starts the first part's tour, the plugin chains
        // the rest with `duty.start` and ends the duty after the last)
        if let Some(first) = d.legs.first().and_then(|l| crate::addon::leg_of(l)) {
            s.line = Some(first.line);
            s.tour = Some(first.tour);
            s.trip = Some(first.first.to_string());
            s.whole_tour = true;
        }
        if s.line.is_some() {
            s.schedule = true;
        }
    }
    s.lan = d.lan.as_deref().map(str::trim).and_then(|l| {
        if l == "host" {
            Some(Lan::Host(0))
        } else {
            l.strip_prefix("join").map(|r| Lan::Join(Some(r.trim_start_matches(':').trim()).filter(|r| !r.is_empty()).unwrap_or("auto").to_string()))
        }
    });
    // (the name the others see: the driver's, as the old launcher gave it)
    s.lan_name = some(&d.lan_name).or_else(|| s.lan.as_ref().and(driver));
    s.no_menu = s.map.is_some() && s.bus.is_some();
    s
}

/// The command line `hub_drive` would start (the page shows it; the tests check it).
pub fn args(d: &Duty) -> Answer {
    let s = hub_core::settings::Settings::load();
    let spec = launch_spec(d, Path::new(&s.omsi2_folder));
    let a = spec.args().map_err(|e| e.to_string())?;
    Ok(json!({ "args": a }))
}

/// The drive started.
pub fn start(d: &Duty, p: &dyn Platform) -> Answer {
    let s = hub_core::settings::Settings::load();
    if !s.has_folders() {
        return Err("Choose your openOMSI and OMSI 2 folders first".into());
    }
    // (a bus for the AI traffic only is never driven, whatever chose it before)
    if !d.bus.trim().is_empty() && crate::content::vfs().is_some_and(|v| crate::content::is_ai_bus(&v, &d.bus)) {
        return Err("This bus is for the AI traffic only and cannot be driven: choose another bus".into());
    }
    let mut game = GAME.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(g) = game.as_mut() {
        if g.poll().is_none() {
            return Err("openOMSI is running already".into());
        }
    }
    // (the openOMSI in use: the one openOMSI's own launcher plays, else the chosen one)
    let folder = hub_core::paths::openomsi_folder().unwrap_or_else(|| s.openomsi_folder.clone().into());
    let checked = hub_game::check_folder(&folder, &hub_core::paths::home_dir()).map_err(|e| e.text().to_string())?;
    if checked.compat.level == hub_game::compat::Level::Unsupported {
        return Err(checked.compat.notes.first().cloned().unwrap_or_else(|| "This openOMSI is not supported".into()));
    }
    // the plugin given the drive (a drive without it still starts: the command line alone)
    let data = hub_core::data_dir();
    let plugin = crate::addon::ready(&checked.install.plugins_dir);
    let assignment = match &plugin {
        Some(dir) => match crate::addon::give(dir, d, &data) {
            Ok(a) => Some(a),
            Err(e) => {
                hub_core::log_line(&format!("drive: {e}"));
                None
            }
        },
        None => {
            hub_core::log_line("drive: the Omsi-Hub plugin is off in this openOMSI: the drive has no assignment");
            None
        }
    };
    let issued = assignment.as_ref().map(|a| a.issued);
    // (the company's buses for the game's AI only in a company's duty, written for it just now:
    // `company::fleetgame`; a drive of the player's own runs the map's whole timetable)
    if let Some(dir) = plugin.as_deref().filter(|_| assignment.as_ref().is_none_or(|a| a.company.trim().is_empty())) {
        if let Err(e) = hub_game::bridge::clear_fleet(dir) {
            hub_core::log_line(&format!("drive: the company's buses for the game: {e}"));
        }
    }
    let spec = launch_spec(d, Path::new(&s.omsi2_folder));
    let g = process::start(&checked.install.exe, &spec).map_err(|e| match e {
        #[cfg(feature = "native")]
        process::StartError::Blocked => crate::online::tr(process::BLOCKED),
        // (the website starts no game)
        #[cfg(not(feature = "native"))]
        process::StartError::Blocked => crate::company::words::tr(process::BLOCKED),
        e => e.to_string(),
    })?;
    let pid = g.pid();
    hub_core::log_line(&format!("drive: openOMSI started (pid {pid}): {}", spec.args().unwrap_or_default().join(" ")));
    *game = Some(g);
    drop(game);
    // (the live dashboard of the drive: `live.rs`)
    crate::live::began(d, assignment.as_ref(), plugin.as_deref());
    let mode = crate::live::mode_of(d, assignment.as_ref());
    p.emit("game-started", json!({ "pid": pid, "mode": mode }));
    watch(p.emitter());
    Ok(json!({ "started": true, "pid": pid, "issued": issued, "mode": mode }))
}

/// One look of the bridge: the reports booked, the company told, the on-time brief written.
/// What was booked.
pub fn look() -> crate::addon::Booked {
    let b = crate::addon::book();
    crate::company::after_booking(&b);
    crate::company::ontime::refresh();
    crate::company::fleetgame::refresh();
    crate::live::sample();
    b
}

/// The bridge looked at once (`addon_poll`).
pub fn poll_once() -> Value {
    to_json(look()).unwrap_or(Value::Null)
}

/// Watch the game on a thread of its own until it ends: the bridge every [`EVERY`], and once
/// more at the end; then tell the page.
fn watch(emit: Option<crate::Emitter>) {
    std::thread::spawn(move || {
        // (the company's words on this thread in the player's language)
        crate::company::words::set_language(&crate::addon::player_language());
        let mut last = Instant::now();
        let mut booked = crate::addon::Booked::default();
        let mut add = |b: crate::addon::Booked| {
            booked.reports += b.reports;
            booked.trips += b.trips;
            booked.orders += b.orders;
            booked.live += b.live;
            for d in b.drivers {
                if !booked.drivers.contains(&d) {
                    booked.drivers.push(d);
                }
            }
            for c in b.companies {
                if !booked.companies.contains(&c) {
                    booked.companies.push(c);
                }
            }
        };
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let ended = {
                let mut game = GAME.lock().unwrap_or_else(|e| e.into_inner());
                let Some(g) = game.as_mut() else { return };
                let ended = g.poll();
                if ended.is_some() {
                    *game = None;
                }
                ended
            };
            if let Some(ended) = ended {
                add(look());
                hub_core::log_line(&format!("drive: openOMSI ended ({:?}) after {:.0} s; {} reports, {} trips booked", ended.code, ended.seconds, booked.reports, booked.trips));
                if let Some(e) = &emit {
                    e("game-ended", json!({ "code": ended.code, "seconds": ended.seconds, "booked": booked }));
                }
                return;
            }
            if last.elapsed() >= EVERY {
                last = Instant::now();
                add(look());
            }
        }
    });
}

/// Whether a game Omsi-Hub started runs.
pub fn state() -> Value {
    let mut game = GAME.lock().unwrap_or_else(|e| e.into_inner());
    let running = match game.as_mut() {
        Some(g) => g.poll().is_none().then(|| g.pid()),
        None => None,
    };
    match running {
        Some(pid) => json!({ "running": true, "pid": pid }),
        None => json!({ "running": false }),
    }
}

/// The game ended by the app (the bridge's loop books the rest and tells the page).
pub fn stop(_p: &dyn Platform) -> Value {
    let mut game = GAME.lock().unwrap_or_else(|e| e.into_inner());
    match game.as_mut() {
        Some(g) => to_json(g.stop()).unwrap_or(Value::Null),
        None => json!({ "running": false }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duty() -> Duty {
        // (the tests' own data folder: never the settings of the driver on this computer)
        crate::online::test_data_dir();
        Duty { map: "maps/Grundorf/global.cfg".into(), bus: "Vehicles/X/a.bus".into(), time: "08:15".into(), line: Some("10".into()), tour: Some("2".into()), autostart: Some(true), ..Duty::default() }
    }

    #[test]
    fn a_duty_becomes_official_flags() {
        let s = launch_spec(&duty(), Path::new("C:/OMSI 2"));
        let mut a = s.args().unwrap();
        // (a driver another test chose is the game's driver too: not what this one looks at)
        if let Some(k) = a.iter().position(|x| x == "--driver") {
            a.drain(k..k + 2);
        }
        assert_eq!(
            a,
            ["--root", "C:/OMSI 2", "--map", "maps/Grundorf/global.cfg", "--bus", "Vehicles/X/a.bus", "--time", "08:15", "--schedule", "--line", "10", "--tour", "2", "--autostart", "--no-menu"]
        );
    }

    #[test]
    fn what_only_the_plugin_can_do_stays_off_the_command_line() {
        let free = Duty { free_line: Some("10_a".into()), company: Some("c1".into()), legs: vec!["11|3|1|2".into()], editor: true, display_font: Some("x".into()), ..duty() };
        let a = launch_spec(&free, Path::new("C:/OMSI 2")).args().unwrap();
        assert!(!a.iter().any(|x| x == "--line" || x == "--tour" || x == "--company" || x == "--free-line" || x == "--duty-leg"), "{a:?}");
        let host = Duty { lan: Some("host".into()), ..duty() };
        assert!(launch_spec(&host, Path::new("r")).args().unwrap().windows(2).any(|w| w == ["--lan-host", "0"]));
        let join = Duty { lan: Some("join:".into()), ..duty() };
        assert!(launch_spec(&join, Path::new("r")).args().unwrap().windows(2).any(|w| w == ["--lan-join", "auto"]));
        let tut = Duty { tutorial: Some(2), ..duty() };
        assert_eq!(launch_spec(&tut, Path::new("r")).args().unwrap(), ["--root", "r", "--tutorial", "2"]);
    }

    #[test]
    fn the_driver_drives_and_is_named_in_a_lan_game() {
        let host = Duty { lan: Some("host".into()), profile: Some("Luc".into()), ..duty() };
        let a = launch_spec(&host, Path::new("r")).args().unwrap();
        let has = |f: &str, v: &str| a.windows(2).any(|w| w[0] == f && w[1] == v);
        assert!(has("--driver", "Drivers/Luc.odr") && has("--lan-name", "Luc"), "{a:?}");
        // (alone: the driver, no name for others)
        let alone = launch_spec(&Duty { profile: Some("Luc".into()), ..duty() }, Path::new("r")).args().unwrap();
        assert!(alone.windows(2).any(|w| w == ["--driver", "Drivers/Luc.odr"]) && !alone.iter().any(|x| x == "--lan-name"), "{alone:?}");
    }

    #[test]
    fn a_shift_starts_its_first_part_and_leaves_the_rest_to_the_plugin() {
        let shift = Duty { line: None, tour: None, legs: vec!["14|1|3|2".into(), "9|4|1|1".into()], entry: Some(-1), ..duty() };
        let a = launch_spec(&shift, Path::new("r")).args().unwrap();
        let has = |f: &str, v: &str| a.windows(2).any(|w| w[0] == f && w[1] == v);
        assert!(has("--line", "14") && has("--tour", "1") && has("--trip", "3"), "{a:?}");
        assert!(a.iter().any(|x| x == "--whole-tour") && a.iter().any(|x| x == "--auto-entry") && a.iter().any(|x| x == "--schedule"), "{a:?}");
        // (the second part is not on the command line: the plugin takes it)
        assert!(!a.iter().any(|x| x == "9" || x == "--duty-leg"), "{a:?}");
        for x in a.iter().filter(|x| x.starts_with("--")) {
            assert!(hub_game::launch::OFFICIAL_FLAGS.contains(&x.as_str()), "{x} is not an official flag");
        }
    }
}
