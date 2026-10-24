//! The plugin in the player's openOMSI (`addon_`), on `hub-game`: the Settings page's "Your
//! openOMSI", the assignment every drive gives the plugin, and what the plugin reports booked.
//!
//! * `addon_state` - the openOMSI in use (its folder, program and version: the one openOMSI's
//!   own launcher plays, else the chosen one, `hub_core::paths::openomsi_folder`; `followed`
//!   when they differ, the chosen one in `chosen`), how well Omsi-Hub
//!   works with it (estimated from the version, or checked with the plugin's handshake
//!   `data/hello.json` since the first drive: `compat`), the plugin there (missing, on, off,
//!   out of date) and the duty it was last given;
//! * `addon_set_folder {folder}` - another openOMSI folder, kept only when openOMSI is in it and
//!   Omsi-Hub works with it (else `refused` says why and the old one stays);
//! * `addon_version` - the folder's version and where it was read from;
//! * `addon_enable {on}` - the plugin installed (or brought up to date) and on, or parked off;
//!   `addon_remove` - taken out of openOMSI;
//! * `addon_texts` - the plugin's texts written in the player's language (`texts.lua`);
//! * `addon_assign {assignment, lang?}` - an assignment given to the plugin as it is;
//! * `addon_reports` - the plugin's reports since the last time, booked first ([`book`]);
//! * `addon_poll` - the bridge looked at once ([`crate::drive`]'s loop does it every 2 s);
//! * `addon_game_settings` - openOMSI's own settings Omsi-Hub changes
//!   (`hub_game::gamesettings`), by group (`controllers`: one of the game controllers'), with
//!   their values in `~/.openomsi/settings.cfg`;
//!   `addon_game_setting {key, value}` - one of them set (`value` null or "": openOMSI's
//!   default again), never while the game runs.
//!
//! **The drive's assignment** ([`assignment_for`], written by [`give`] before every start): the
//! driver, the map, the line, tour and trip, the duty's parts (`legs`: a shift's tours, a relief
//! on the way), a free route, the display font and the IBIS, the navigator's settings, and for
//! a company's duty its name and duty, its sheet, controls, fare, contract and crew (from
//! `company::plugin::assign_duty`, kept here until the drive starts: [`set_company`]).
//!
//! **Booking** ([`book`]): `data/reports.jsonl` read on from where it was read last (kept per
//! plugin folder in `addon.json` of the data folder; a folder seen the first time is read from
//! its end, so the old Omsi-Hub's reports already booked are not booked again):
//!
//! * a trip → the driver's record (`trips/<driver>.jsonl`: the careers and the company's day
//!   take it from there);
//! * a note, a defect, a fault of a company's duty → the company's queue of orders
//!   (`companies/<id>.orders.jsonl`, carried out with the company's rules: `remote::take`);
//! * a live event of the company (`live`) → its live file (`companies/<id>.live.jsonl`);
//! * the duty taken and ended → the log.
//!
//! The control room's replies (`data/replies.jsonl`) and the duty's status (`data/status.json`)
//! are taken by the open company itself (`company::messages::tick`), with this module's
//! [`duty_link`].

use crate::{arg, to_json, Answer, Platform};
use hub_core::company::remote;
use hub_core::company::store;
use hub_core::settings::Settings;
use hub_core::{Duty, TripRun};
use hub_game::bridge::{self, Assignment, Report, ReportReader};
use hub_game::install;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What this module keeps, in the data folder.
pub const FILE: &str = "addon.json";
/// The notices the plugin's file keeps (it shows each id once).
const NOTICES_KEPT: usize = 12;

/// What is kept between two starts of the app.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kept {
    /// How far each plugin folder's reports were read (by the folder's path).
    pub reports: BTreeMap<String, ReportReader>,
    /// How far the game's log was read for the reports of a plugin without file functions.
    pub log: u64,
    /// The last assignment given: its reports are booked for its driver and company.
    pub assigned: Option<Assigned>,
}

/// The duty the plugin was last given (the old `omsihub-addon.json`'s `assigned`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Assigned {
    pub issued: u64,
    pub driver: String,
    pub map: String,
    pub bus: String,
    /// A company's duty: the company's name, the duty's line (the timetable's name) and tour.
    /// Empty: a drive of the player's own.
    pub company: String,
    pub line: String,
    pub tour: String,
    /// The company whose duty it is, by its id ("": a drive of the player's own).
    pub company_id: String,
    /// The plugin's folder it was written to.
    pub plugin_dir: String,
}

/// The company's side of a duty about to be driven (`company::plugin::assign_duty`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompanyPart {
    /// The company's name, and the duty as the plugin shows it ("5 2 1": line, tour, duty).
    pub name: String,
    pub duty: String,
    pub parts: crate::company::plugin::Parts,
    pub crew: Option<bridge::Crew>,
    pub company_id: String,
    /// The drive it is for (its bus and map): another drive does not take it.
    pub bus: String,
    pub map: String,
}

/// The company's part waiting for its drive to start.
static COMPANY: Mutex<Option<CompanyPart>> = Mutex::new(None);
/// The last notices for the driver (`notify_game`).
static NOTICES: Mutex<Vec<bridge::Notice>> = Mutex::new(Vec::new());

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn kept_path(data: &Path) -> PathBuf {
    data.join(FILE)
}

pub fn load(data: &Path) -> Kept {
    std::fs::read_to_string(kept_path(data)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(data: &Path, k: &Kept) -> Result<(), String> {
    let path = kept_path(data);
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(k).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// The plugins folder of the openOMSI in use (`paths::openomsi_folder`).
pub fn plugins_dir() -> Result<PathBuf, String> {
    let folder = hub_core::paths::openomsi_folder().ok_or("Choose your openOMSI folder first")?;
    let c = hub_game::check_folder(&folder, &hub_core::paths::home_dir()).map_err(|p| p.text().to_string())?;
    Ok(c.install.plugins_dir)
}

/// Whether the folder `a` is the folder `b` (written alike, or the same folder on disk: another
/// slash, another case).
fn same_folder(a: &Path, b: &str) -> bool {
    let b = Path::new(b.trim());
    a == b || a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(x, y)| x == y)
}

/// The player's language, as the texts' tables name it (`en` when none is set).
pub fn player_language() -> String {
    let l = Settings::load().language.trim().to_ascii_lowercase();
    if l.is_empty() {
        #[cfg(feature = "native")]
        let page = crate::online::language();
        // (the website: the page says its language with every command)
        #[cfg(not(feature = "native"))]
        let page = String::new();
        if page.is_empty() {
            "en".into()
        } else {
            page
        }
    } else {
        l
    }
}

/// The plugin's texts in `lang` (English where a text has no translation).
pub fn plugin_texts(lang: &str) -> BTreeMap<String, String> {
    let table = if lang.is_empty() || lang == "en" { BTreeMap::new() } else { hub_locales::texts(lang) };
    install::TEXTS.iter().map(|t| (t.to_string(), table.get(*t).cloned().unwrap_or_else(|| t.to_string()))).collect()
}

/// The plugin's texts in the player's language beside it (`texts.lua`).
pub fn write_texts(plugin_dir: &Path) {
    if let Err(e) = bridge::write_texts(plugin_dir, &plugin_texts(&player_language())) {
        hub_core::log_line(&format!("plugin: its texts not written: {e}"));
    }
}

/// The plugin of `plugins` ready for a drive: put in or brought up to date where it is missing
/// or older (the drive checks it the first time), left alone where the player parked it off.
/// Its folder when it is on.
pub fn ready(plugins: &Path) -> Option<PathBuf> {
    let st = install::status(plugins);
    match st.state {
        install::State::On => {}
        install::State::Off => return None,
        install::State::Missing | install::State::Outdated => match install::install(plugins) {
            Ok(_) => hub_core::log_line(&format!("plugin: {} put into {}", install::VERSION, plugins.display())),
            Err(e) => {
                hub_core::log_line(&format!("plugin: not put into {}: {e}", plugins.display()));
                return None;
            }
        },
    }
    let dir = install::plugin_dir(plugins);
    write_texts(&dir);
    Some(dir)
}

/// The company's part of the drive about to start (`company::plugin::assign_duty`).
pub fn set_company(part: Option<CompanyPart>) {
    *COMPANY.lock().unwrap_or_else(|e| e.into_inner()) = part;
}

/// The company's part waiting for the drive `d` (taken: one drive has it once).
fn take_company(d: &Duty) -> Option<CompanyPart> {
    let mut c = COMPANY.lock().unwrap_or_else(|e| e.into_inner());
    let same = |a: &str, b: &str| a.replace('\\', "/").eq_ignore_ascii_case(&b.replace('\\', "/"));
    if c.as_ref().is_some_and(|p| same(&p.bus, &d.bus) && same(&p.map, &d.map)) {
        c.take()
    } else {
        None
    }
}

/// A part of a duty (`compose::Block`'s `line|tour|first[@stop]|trips[@stop]`, stops from 0) as
/// the plugin takes it (stops from 1).
pub fn leg_of(text: &str) -> Option<bridge::Leg> {
    let ((line, tour, first, trips), from, to) = hub_core::compose::Block::parse_between(text)?;
    Some(bridge::Leg { line, tour, first: first.max(1) as u32, trips: trips.max(1) as u32, from_stop: from.map_or(0, |s| s as u32 + 1), to_stop: to.map_or(0, |s| s as u32 + 1) })
}

/// The assignment of the drive `d` (`company`: a company's duty), with the navigator's
/// settings of `settings`, issued at `issued`.
pub fn assignment_for(d: &Duty, company: Option<&CompanyPart>, settings: &Settings, issued: u64) -> Assignment {
    let on = |k: &str, default: bool| settings.other.get(k).and_then(Value::as_bool).unwrap_or(default);
    let driver = d.profile.clone().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).unwrap_or_else(crate::launcher::driver);
    let personnel = company.filter(|_| on("nav_signon", false)).and_then(|_| personnel_of(&driver, true));
    let legs: Vec<bridge::Leg> = d.legs.iter().filter_map(|l| leg_of(l)).collect();
    let first = legs.first();
    let free = d.free_line.as_deref().map(str::trim).filter(|r| !r.is_empty()).map(str::to_string);
    let trip = if free.is_some() { 0 } else { first.map(|l| l.first).or_else(|| d.trip.as_deref().and_then(|t| t.trim().parse().ok())).unwrap_or(0) };
    let (name, duty) = company.map(|c| (c.name.clone(), c.duty.clone())).unwrap_or_default();
    let parts = company.map(|c| c.parts.clone()).unwrap_or_default();
    Assignment {
        issued,
        company: name,
        duty,
        driver,
        map: d.map.clone(),
        line: first.map(|l| l.line.clone()).or_else(|| d.line.clone()).unwrap_or_default(),
        tour: if free.is_some() { String::new() } else { first.map(|l| l.tour.clone()).or_else(|| d.tour.clone()).unwrap_or_default() },
        trip,
        stop: first.map_or(0, |l| l.from_stop),
        personnel,
        board: Some(on("nav_board", true)),
        ontime: Some(on("nav_ontime", true)),
        // (the IBIS typed by the game unless the player types it: off where the game cannot,
        // `compat` says so on the settings page)
        ibis: Some(on("ibis_auto", true)),
        bus: d.bus.clone(),
        display_font: d.display_font.clone().unwrap_or_default(),
        editor: d.editor,
        // (the duty strip unless the navigator's settings say not: `nav_strip`)
        crew: company.and_then(|c| c.crew.clone()).map(|x| bridge::Crew { strip: Some(on("nav_strip", true)), ..x }),
        sheet: parts.sheet,
        controls: parts.controls,
        fare: parts.fare,
        contract: parts.contract,
        legs: if free.is_some() { Vec::new() } else { legs },
        free_line: free.unwrap_or_default(),
        ..Assignment::default()
    }
}

/// The driver's personnel number and code for signing on (`personnel.json` of the data folder,
/// `{ "drivers": { "<driver in lower case>": { "number", "code" } } }`): made and written down
/// the first time (`make`), as the old launcher's welcome did. A file that is not one of these
/// is left alone (no sign-on then).
pub fn personnel_of(driver: &str, make: bool) -> Option<bridge::Personnel> {
    let path = hub_core::data_dir().join("personnel.json");
    let key = Some(driver.trim().to_lowercase()).filter(|k| !k.is_empty()).unwrap_or_else(|| "driver".into());
    let text = std::fs::read_to_string(&path).ok();
    let mut file: serde_json::Map<String, Value> = match text.as_deref().map(str::trim) {
        None | Some("") => serde_json::Map::new(),
        Some(t) => serde_json::from_str::<Value>(t).ok()?.get("drivers")?.as_object()?.clone(),
    };
    let digits = |s: &str, n: std::ops::RangeInclusive<usize>| n.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    if let Some(e) = file.get(&key) {
        let (number, code) = (e.get("number")?.as_str()?, e.get("code")?.as_str()?);
        return (digits(number, 3..=10) && digits(code, 3..=8)).then(|| bridge::Personnel { number: number.into(), code: code.into() });
    }
    if !make {
        return None;
    }
    // (a number of six digits and a code of four, the first of each 1-9)
    let mut seed = now() ^ (std::process::id() as u64) << 20 ^ key.bytes().fold(0u64, |h, b| h.rotate_left(5) ^ b as u64);
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut make = |n: usize| -> String { (0..n).map(|i| char::from(b'0' + if i == 0 { 1 + next() % 9 } else { next() % 10 } as u8)).collect() };
    let p = bridge::Personnel { number: make(6), code: make(4) };
    file.insert(key, json!({ "number": p.number, "code": p.code }));
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(&json!({ "drivers": file })).ok()?;
    std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, &path)).ok()?;
    Some(p)
}

/// Give the plugin in `plugin_dir` the drive `d` (with the company's part waiting for it), and
/// keep what its reports are booked with. The assignment written.
pub fn give(plugin_dir: &Path, d: &Duty, data: &Path) -> Result<Assignment, String> {
    let company = take_company(d);
    let a = assignment_for(d, company.as_ref(), &Settings::load(), now());
    bridge::write_assignment(plugin_dir, &a).map_err(|e| format!("the plugin's assignment: {e}"))?;
    // (the phone's duty card)
    #[cfg(feature = "native")]
    crate::companion::assigned(&a);
    let mut k = load(data);
    k.assigned = Some(Assigned {
        issued: a.issued,
        driver: a.driver.clone(),
        map: a.map.clone(),
        bus: a.bus.clone(),
        company: a.company.clone(),
        line: a.line.clone(),
        tour: a.tour.clone(),
        company_id: company.map(|c| c.company_id).unwrap_or_default(),
        plugin_dir: plugin_dir.to_string_lossy().into_owned(),
    });
    // (the reports from here on: a folder seen the first time is read from its end)
    reader_of(&mut k, plugin_dir);
    save(data, &k)?;
    hub_core::log_line(&format!(
        "plugin: assignment {} for {} ({})",
        a.issued,
        a.driver,
        if a.company.is_empty() { "a drive of the player's own".to_string() } else { format!("{} {}", a.company, a.duty) }
    ));
    Ok(a)
}

/// The reader of `plugin_dir`'s reports, made at the file's end when the folder is new.
fn reader_of<'a>(k: &'a mut Kept, plugin_dir: &Path) -> &'a mut ReportReader {
    let key = plugin_dir.to_string_lossy().into_owned();
    k.reports.entry(key).or_insert_with(|| {
        let len = std::fs::metadata(plugin_dir.join(bridge::DATA_DIR).join(bridge::REPORTS_FILE)).map_or(0, |m| m.len());
        ReportReader { offset: len }
    })
}

/// The plugin's folder and the company's duty it was given, while it has one (the control
/// room's messages go to it: `company::messages`).
pub fn duty_link() -> Option<(PathBuf, Assigned)> {
    let a = load(&hub_core::data_dir()).assigned.filter(|a| !a.company.is_empty() && !a.line.is_empty())?;
    let dir = PathBuf::from(&a.plugin_dir);
    (!a.plugin_dir.is_empty() && dir.is_dir()).then_some((dir, a))
}

/// The plugin's folder in the player's openOMSI, where it is installed (Play together's drives
/// read its files).
pub fn game_plugin_dir() -> Option<PathBuf> {
    plugins_dir().ok().map(|d| install::plugin_dir(&d))
}

/// Notices for the driver in the game, as cards (`data/notices.json`): where the plugin is on.
pub fn notify_game(notices: Vec<bridge::Notice>) {
    if notices.is_empty() {
        return;
    }
    let Some(dir) = plugins_dir().ok().filter(|d| install::status(d).state == install::State::On).map(|d| install::plugin_dir(&d)) else { return };
    let mut kept = NOTICES.lock().unwrap_or_else(|e| e.into_inner());
    kept.extend(notices);
    let over = kept.len().saturating_sub(NOTICES_KEPT);
    kept.drain(..over);
    if let Err(e) = bridge::write_notices(&dir, &kept) {
        hub_core::log_line(&format!("plugin: the notices not written: {e}"));
    }
}

// --- booking -------------------------------------------------------------------------------

/// What one look at the reports booked.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Booked {
    /// The reports read.
    pub reports: usize,
    /// Trips into the drivers' records, orders into the companies' queues, live events into
    /// their live files.
    pub trips: usize,
    pub orders: usize,
    pub live: usize,
    /// The drivers whose trips were booked, and the companies something was booked for.
    pub drivers: Vec<String>,
    pub companies: Vec<String>,
}

/// A trip the plugin reported, as the driver's record keeps it, booked for the assignment it
/// names (`a`; None: the app's driver).
pub fn trip_run(r: &Report, a: Option<&Assigned>) -> Option<TripRun> {
    if !r.is_trip() || r.trip == 0 {
        return None;
    }
    let driver = a.map(|a| a.driver.clone()).filter(|d| !d.is_empty()).unwrap_or_else(crate::launcher::driver);
    let pick = |ours: &str, theirs: Option<&str>| if ours.is_empty() { theirs.unwrap_or_default().to_string() } else { ours.to_string() };
    let time = if r.at > 0.0 { r.at as u64 } else { now() };
    let map = pick(&r.map, a.map(|a| a.map.as_str()));
    let bus = pick(&r.bus, a.map(|a| a.bus.as_str()));
    // (the plugin's own record of the trip, where it followed it from its start)
    if let Some(run) = r.record.clone().and_then(|v| serde_json::from_value::<TripRun>(v).ok()) {
        return Some(TripRun { time, driver, map, bus, ..run });
    }
    let line = if r.number.trim().is_empty() { r.line.trim() } else { r.number.trim() };
    // (as the game's own report writes it: "5 Rathaus" is the line's terminus Rathaus)
    let terminus = r.terminus.trim();
    let terminus = if line.is_empty() { terminus } else { terminus.strip_prefix(&format!("{line} ")).unwrap_or(terminus) };
    Some(TripRun {
        time,
        driver,
        map,
        bus,
        line: line.to_string(),
        tour: r.tour.trim().to_string(),
        terminus: terminus.trim().to_string(),
        trip: r.trip as usize,
        trips: r.trips as usize,
        departure: r.departure.unwrap_or_default(),
        arrival: r.arrival.unwrap_or_default(),
        completed: r.how == "arrived",
        worst: r.delay,
        average: r.delay,
        ..Default::default()
    })
}

/// What a crew report of the plugin orders the company of the assignment `a`: the company's id
/// and the order (None: not a company's duty, or not a report of a known shape - checked as
/// strictly as the phone's orders, `remote::parse`).
pub fn crew_order(r: &Report, a: &Assigned) -> Option<(String, remote::Order)> {
    use hub_core::company::crew;
    if !r.is_crew() || a.company_id.is_empty() || r.bus_id == 0 {
        return None;
    }
    let text = if r.kind == "fault" { crew::COLLISION.to_string() } else { r.text.clone() };
    let text: String = text.chars().take(crew::TEXT_MAX).collect();
    let v = match r.kind.as_str() {
        "note" => json!({ "do": "note", "bus": r.bus_id, "template": (!r.template.is_empty()).then_some(r.template.as_str()), "text": text }),
        _ => json!({ "do": "defect", "bus": r.bus_id, "kind": r.defect, "text": text, "fault": r.kind == "fault" }),
    };
    remote::parse(&v).map(|o| (a.company_id.clone(), o))
}

/// A live event of the company the plugin reported (`live`): the company's id and the event.
pub fn live_event(r: &Report, a: &Assigned) -> Option<(String, hub_core::company::day::LiveEvent)> {
    if !r.is_live() || a.company_id.is_empty() {
        return None;
    }
    let ev = serde_json::from_value(r.event.clone()?).ok()?;
    Some((a.company_id.clone(), ev))
}

/// Book reports into the data folder `data`, each with the assignment it names.
pub fn book_reports(data: &Path, reports: &[Report], assigned: Option<&Assigned>) -> Booked {
    let mut b = Booked { reports: reports.len(), ..Booked::default() };
    let note = |list: &mut Vec<String>, x: &str| {
        if !x.is_empty() && !list.iter().any(|y| y == x) {
            list.push(x.to_string());
        }
    };
    for r in reports {
        let a = assigned.filter(|a| a.issued == r.issued && r.issued != 0);
        match r.kind.as_str() {
            "duty_start" | "duty_end" => hub_core::log_line(&format!("plugin: {} line {} tour {}", r.kind, r.line, r.tour)),
            _ => {}
        }
        if let Some(t) = trip_run(r, a) {
            match hub_core::append_trip(data, &t) {
                Ok(_) => {
                    b.trips += 1;
                    note(&mut b.drivers, &t.driver);
                }
                Err(e) => hub_core::log_line(&format!("plugin: a trip of line {} not booked: {e:#}", t.line)),
            }
        }
        // (a handover note, a defect of the walk-around, a fault on the way: to the company's
        // queue, which the company carries out with its rules - `remote::take`)
        if let Some((company, o)) = a.and_then(|a| crew_order(r, a)) {
            match remote::queue(data, &company, &o) {
                Ok(()) => {
                    b.orders += 1;
                    note(&mut b.companies, &company);
                }
                Err(e) => hub_core::log_line(&format!("plugin: a {} of the driver not queued: {e:#}", r.kind)),
            }
        }
        if let Some((company, ev)) = a.and_then(|a| live_event(r, a)) {
            match store::append_live(data, &company, &ev) {
                Ok(()) => {
                    b.live += 1;
                    note(&mut b.companies, &company);
                }
                Err(e) => hub_core::log_line(&format!("plugin: a live event of {company} not booked: {e:#}")),
            }
        }
    }
    b
}

/// Book what the plugin in `plugin_dir` reported since the last look (into `data`).
pub fn book_from(data: &Path, plugin_dir: &Path) -> Booked {
    let mut k = load(data);
    let reader = reader_of(&mut k, plugin_dir);
    let before = reader.offset;
    let new = reader.read_new(plugin_dir);
    let moved = reader.offset != before;
    let b = book_reports(data, &new, k.assigned.as_ref());
    if moved || !k.reports.is_empty() {
        let _ = save(data, &k);
    }
    b
}

/// The reports a plugin without file functions put into the game's log (`~/.openomsi/game.log`,
/// openOMSI's own; only read), since the last look.
fn book_from_log(data: &Path) -> Booked {
    let log = hub_core::paths::home_dir().join(".openomsi").join("game.log");
    let Ok(text) = std::fs::read(&log) else { return Booked::default() };
    let mut k = load(data);
    if (text.len() as u64) < k.log {
        k.log = 0;
    }
    // (up to the last whole line: one the game is still writing is read whole next time)
    let start = k.log as usize;
    let Some(end) = text[start..].iter().rposition(|b| *b == b'\n') else { return Booked::default() };
    let new = String::from_utf8_lossy(&text[start..start + end]).into_owned();
    k.log = (start + end + 1) as u64;
    let (_, reports) = bridge::from_log(&new);
    let b = book_reports(data, &reports, k.assigned.as_ref());
    let _ = save(data, &k);
    b
}

/// Book what the plugin of the chosen openOMSI reported since the last look: from its files,
/// or from the game's log where its handshake says it cannot write files.
pub fn book() -> Booked {
    let data = hub_core::data_dir();
    let Ok(plugins) = plugins_dir() else { return Booked::default() };
    let dir = install::plugin_dir(&plugins);
    let mut b = book_from(&data, &dir);
    let no_files = bridge::read_hello(&dir).is_some_and(|h| h.functions.get("files.append") == Some(&false));
    if b.reports == 0 && no_files {
        b = book_from_log(&data);
    }
    if b.trips > 0 || b.orders > 0 || b.live > 0 {
        hub_core::log_line(&format!("plugin: booked {} trips, {} orders, {} live events", b.trips, b.orders, b.live));
    }
    b
}

// --- the commands --------------------------------------------------------------------------

fn state() -> Value {
    let s = Settings::load();
    // (the openOMSI in use: the one openOMSI's own launcher plays, else the chosen one)
    let folder = hub_core::paths::openomsi_folder().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let game = (!folder.is_empty()).then(|| crate::hub::check_openomsi(&folder));
    let plugins = plugins_dir().ok();
    let plugin = plugins.as_deref().map(install::status);
    let hello = plugins.as_deref().and_then(|d| bridge::read_hello(&install::plugin_dir(d)));
    let assigned = load(&hub_core::data_dir()).assigned;
    json!({
        "folder": folder,
        // (followed: openOMSI's launcher plays another openOMSI than the one chosen here)
        "chosen": s.openomsi_folder,
        "followed": hub_core::paths::launcher_openomsi(&hub_core::paths::home_dir()).is_some_and(|f| !same_folder(&f, &s.openomsi_folder)),
        "game": game,
        "plugin": plugin,
        "plugin_version": install::VERSION,
        // (the plugin's handshake: the game's own word on what it offers, since the first drive)
        "hello": hello,
        "checked": hello.is_some(),
        "assigned": assigned,
    })
}

fn set_folder(folder: &str) -> Answer {
    let checked = crate::hub::check_openomsi(folder);
    if checked["ok"] != true {
        let mut v = state();
        v["refused"] = json!({ "folder": folder.trim(), "problem": checked["text"], "say": checked["say"], "vars": checked["vars"] });
        return Ok(v);
    }
    let mut s = Settings::load();
    s.openomsi_folder = folder.trim().to_string();
    s.save().map_err(|e| format!("the settings could not be saved: {e}"))?;
    hub_core::log_line(&format!("addon: openOMSI {} in {}", checked["version"].as_str().unwrap_or("?"), s.openomsi_folder));
    Ok(state())
}

fn assign(a: Assignment, lang: Option<String>) -> Answer {
    let plugins = plugins_dir()?;
    let status = install::status(&plugins);
    if status.state != install::State::On {
        return Err("The Omsi-Hub plugin is not switched on in this openOMSI".into());
    }
    let lang = lang.filter(|l| !l.is_empty()).unwrap_or_else(player_language);
    let a = Assignment { issued: if a.issued == 0 { now() } else { a.issued }, texts: plugin_texts(&lang), ..a };
    bridge::write_assignment(&status.dir, &a).map_err(|e| e.to_string())?;
    Ok(json!({ "issued": a.issued }))
}

/// An `addon_` command.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    match cmd {
        "addon_state" => Ok(state()),
        "addon_set_folder" => set_folder(&arg::<String>(args, "folder")?),
        "addon_version" => {
            let folder = hub_core::paths::openomsi_folder().ok_or("Choose your openOMSI folder first")?;
            let c = crate::hub::check_openomsi(&folder.to_string_lossy());
            Ok(json!({ "version": c["version"], "version_from": c["version_from"], "min": hub_game::compat::FIRST_SUPPORTED.to_string(), "ok": c["ok"] }))
        }
        "addon_enable" => {
            let dir = plugins_dir()?;
            let on = arg::<bool>(args, "on")?;
            let st = if on { install::install(&dir) } else { install::disable(&dir) };
            let st = st.map_err(|e| format!("the plugin could not be changed: {e}"))?;
            if on {
                write_texts(&install::plugin_dir(&dir));
            }
            hub_core::log_line(&format!("plugin: {:?} in {}", st.state, dir.display()));
            to_json(st)
        }
        "addon_remove" => {
            let dir = plugins_dir()?;
            to_json(install::remove(&dir).map_err(|e| format!("the plugin could not be taken out: {e}"))?)
        }
        "addon_texts" => {
            let dir = plugins_dir()?;
            write_texts(&install::plugin_dir(&dir));
            Ok(json!({ "language": player_language() }))
        }
        "addon_assign" => assign(arg::<Assignment>(args, "assignment")?, arg::<Option<String>>(args, "lang")?),
        "addon_reports" => to_json(book()),
        "addon_poll" => Ok(crate::drive::poll_once()),
        "addon_game_settings" => Ok(game_settings(&hub_core::paths::home_dir(), game_running())),
        "addon_game_setting" => {
            if game_running() {
                return Err("openOMSI is running: change its settings once it has ended".into());
            }
            let home = hub_core::paths::home_dir();
            let value = arg::<Option<String>>(args, "value")?;
            hub_game::gamesettings::set(&hub_game::gamesettings::path(&home), &arg::<String>(args, "key")?, value.as_deref())?;
            Ok(game_settings(&home, false))
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// A game started from Omsi-Hub is running.
fn game_running() -> bool {
    crate::drive::state()["running"].as_bool().unwrap_or(false)
}

/// openOMSI's settings as the page shows them: by group, each with what it takes and its value
/// in the file of the home folder `home` (null: not in it, openOMSI's default holds).
pub fn game_settings(home: &Path, running: bool) -> Value {
    use hub_game::gamesettings::{self as gs, Kind};
    let path = gs::path(home);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let groups: Vec<Value> = gs::GROUPS
        .iter()
        .map(|g| {
            let fields: Vec<Value> = gs::FIELDS
                .iter()
                .filter(|f| f.group == *g)
                .map(|f| {
                    let mut v = json!({ "key": f.key, "label": f.label, "value": gs::value(&text, f.key) });
                    match f.kind {
                        Kind::Switch => v["kind"] = json!("switch"),
                        Kind::Choice(options) => {
                            v["kind"] = json!("choice");
                            v["options"] = json!(options);
                        }
                        Kind::Number { min, max, step, default } => {
                            v["kind"] = json!("number");
                            v["min"] = json!(min);
                            v["max"] = json!(max);
                            v["step"] = json!(step);
                            v["default"] = json!(default);
                        }
                        Kind::Text { hint } => {
                            v["kind"] = json!("text");
                            v["hint"] = json!(hint);
                        }
                    }
                    v
                })
                .collect();
            // (`controllers`: shown on the tab "Controllers", beside the devices)
            json!({ "title": g, "controllers": gs::CONTROLLER_GROUPS.contains(g), "fields": fields })
        })
        .collect();
    json!({ "path": path.to_string_lossy(), "exists": path.is_file(), "running": running, "groups": groups })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_core::company::crew;

    #[test]
    fn openomsi_s_settings_are_shown_by_group_with_the_file_s_values() {
        let home = temp("game-settings");
        let v = game_settings(&home, false);
        assert_eq!(v["exists"], false);
        let field = |v: &Value, key: &str| v["groups"].as_array().unwrap().iter().flat_map(|g| g["fields"].as_array().unwrap().clone()).find(|f| f["key"] == key).unwrap();
        assert_eq!(field(&v, "msaa")["value"], Value::Null, "not in the file: openOMSI's default");
        assert_eq!(field(&v, "msaa")["options"][2], json!(["4", "4×"]));
        assert_eq!((field(&v, "ui_scale")["kind"].as_str(), field(&v, "ui_scale")["default"].as_f64()), (Some("number"), Some(1.0)));
        hub_game::gamesettings::set(&hub_game::gamesettings::path(&home), "msaa", Some("8")).unwrap();
        let v = game_settings(&home, true);
        assert_eq!((field(&v, "msaa")["value"].as_str(), v["exists"].as_bool(), v["running"].as_bool()), (Some("8"), Some(true), Some(true)));
        assert_eq!(v["groups"][0]["title"], "Graphics");
        let group = |t: &str| v["groups"].as_array().unwrap().iter().find(|g| g["title"] == t).unwrap().clone();
        assert_eq!((group("Graphics")["controllers"].as_bool(), group("Force feedback")["controllers"].as_bool()), (Some(false), Some(true)));
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("omsihub-addon-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_driver_s_notes_and_defects_go_to_the_company_of_the_duty() {
        let a = Assigned { issued: 7, driver: "Luc".into(), company_id: "stadtbus".into(), ..Default::default() };
        let r = |kind: &str| Report { kind: kind.into(), issued: 7, bus_id: 4, ..Default::default() };
        let note = Report { template: "door_sticks".into(), text: "rear".into(), ..r("note") };
        let (company, o) = crew_order(&note, &a).unwrap();
        assert_eq!(company, "stadtbus");
        assert_eq!(o, remote::Order::Note { bus: 4, template: Some(crew::NoteTemplate::DoorSticks), text: "rear".into() });
        let defect = Report { defect: "tyres".into(), ..r("defect") };
        assert_eq!(crew_order(&defect, &a).map(|x| x.1), Some(remote::Order::Defect { bus: 4, kind: crew::DefectKind::Tyres, text: String::new(), fault: false }));
        // a collision the game saw: a fault, in the company's words
        let fault = Report { defect: "other".into(), minutes: Some(30.0), ..r("fault") };
        assert!(matches!(crew_order(&fault, &a).map(|x| x.1), Some(remote::Order::Defect { fault: true, ref text, .. }) if text == crew::COLLISION));
        // not of a company's duty, no bus, an unknown check, a trip: nothing
        assert!(crew_order(&note, &Assigned { company_id: String::new(), ..a.clone() }).is_none());
        assert!(crew_order(&Report { bus_id: 0, ..note.clone() }, &a).is_none());
        assert!(crew_order(&Report { defect: "engine".into(), ..r("defect") }, &a).is_none());
        assert!(crew_order(&r("trip"), &a).is_none());
        // (an older choice reads, without the company)
        let old: Assigned = serde_json::from_str(r#"{"issued":3,"driver":"Luc","map":"m","bus":"b"}"#).unwrap();
        assert_eq!((old.company.as_str(), old.company_id.as_str()), ("", ""));
    }

    #[test]
    fn a_trip_the_plugin_reported_goes_into_its_driver_s_record() {
        let a = Assigned { issued: 7, driver: "Luc".into(), map: "maps/Grundorf/global.cfg".into(), bus: "Vehicles/MAN_SD202/SD202.bus".into(), ..Default::default() };
        let r = Report {
            kind: "trip".into(),
            issued: 7,
            line: "Linie5".into(),
            tour: "2".into(),
            trip: 3,
            trips: 6,
            number: "5".into(),
            terminus: "5 Rathaus".into(),
            departure: Some(30_000.0),
            arrival: Some(31_800.0),
            how: "arrived".into(),
            delay: Some(65.0),
            at: 1_760_000_000.0,
            ..Default::default()
        };
        let t = trip_run(&r, Some(&a)).unwrap();
        assert_eq!((t.driver.as_str(), t.map.as_str(), t.bus.as_str()), ("Luc", "maps/Grundorf/global.cfg", "Vehicles/MAN_SD202/SD202.bus"));
        // (the line as its displays show it, the terminus without it - as the game's own report)
        assert_eq!((t.line.as_str(), t.tour.as_str(), t.terminus.as_str(), t.trip, t.trips), ("5", "2", "Rathaus", 3, 6));
        assert_eq!((t.time, t.departure, t.arrival, t.completed, t.worst), (1_760_000_000, 30_000.0, 31_800.0, true, Some(65.0)));
        // (the bus and map the plugin saw win; a trip given up is not completed)
        let given_up = Report { how: "given_up".into(), bus: "Vehicles/X/x.bus".into(), ..r.clone() };
        let t = trip_run(&given_up, Some(&a)).unwrap();
        assert_eq!((t.bus.as_str(), t.completed), ("Vehicles/X/x.bus", false));
        assert!(trip_run(&Report { kind: "duty_start".into(), ..r }, Some(&a)).is_none());
    }

    #[test]
    fn a_shift_s_parts_and_a_free_route_reach_the_plugin_without_fork_flags() {
        assert_eq!(leg_of("14|1|3|2"), Some(bridge::Leg { line: "14".into(), tour: "1".into(), first: 3, trips: 2, from_stop: 0, to_stop: 0 }));
        // (a relief on the way: the compose format counts the stops from 0, the plugin from 1)
        assert_eq!(leg_of("4 Liman|Mo|Fr 3|2@4|5@7").map(|l| (l.tour, l.from_stop, l.to_stop)), Some(("Mo|Fr 3".to_string(), 5, 8)));
        assert!(leg_of("rubbish").is_none());
        let d = Duty {
            map: "maps/G/global.cfg".into(),
            bus: "Vehicles/X/x.bus".into(),
            line: Some("14".into()),
            tour: Some("1".into()),
            trip: Some("3".into()),
            legs: vec!["14|1|3|2".into(), "9|4|1|1".into()],
            display_font: Some("Gelb".into()),
            profile: Some("Luc".into()),
            ..Duty::default()
        };
        let mut s = Settings::default();
        s.other.insert("ibis_auto".into(), json!(false));
        let a = assignment_for(&d, None, &s, 9);
        assert_eq!((a.issued, a.driver.as_str(), a.line.as_str(), a.tour.as_str(), a.trip), (9, "Luc", "14", "1", 3));
        assert_eq!(a.legs.len(), 2);
        assert_eq!((a.display_font.as_str(), a.ibis, a.board), ("Gelb", Some(false), Some(true)));
        assert!(a.company.is_empty() && a.crew.is_none() && a.free_line.is_empty());
        // a free drive along a route: the route, no parts, no tour
        let free = Duty { free_line: Some("14_Hbf-Zoo".into()), ..d.clone() };
        let a = assignment_for(&free, None, &s, 9);
        assert_eq!((a.free_line.as_str(), a.line.as_str(), a.tour.as_str(), a.legs.len(), a.trip), ("14_Hbf-Zoo", "14", "", 0, 0));
        // a company's duty: its parts and its crew
        let part = CompanyPart {
            name: "Stadtbus".into(),
            duty: "5 2 1".into(),
            company_id: "stadtbus".into(),
            crew: Some(bridge::Crew { bus: 4, ..Default::default() }),
            parts: crate::company::plugin::Parts { sheet: vec![bridge::SheetRow { kind: "sign_on".into(), ..Default::default() }], ..Default::default() },
            ..Default::default()
        };
        let a = assignment_for(&d, Some(&part), &s, 9);
        assert_eq!((a.company.as_str(), a.duty.as_str(), a.sheet.len()), ("Stadtbus", "5 2 1", 1));
        assert_eq!(a.crew.map(|c| (c.bus, c.strip)), Some((4, Some(true))));
    }

    #[test]
    fn reports_are_booked_once_and_a_new_folder_is_read_from_its_end() {
        let data = temp("book");
        let plugin = data.join("plugin");
        std::fs::create_dir_all(plugin.join(bridge::DATA_DIR)).unwrap();
        let file = plugin.join(bridge::DATA_DIR).join(bridge::REPORTS_FILE);
        // (the old Omsi-Hub's reports, booked by it long ago)
        std::fs::write(&file, "{\"kind\":\"trip\",\"issued\":1,\"trip\":1,\"line\":\"9\"}\n").unwrap();
        let mut k = load(&data);
        k.assigned = Some(Assigned { issued: 5, driver: "Luc".into(), company_id: "c1".into(), ..Default::default() });
        reader_of(&mut k, &plugin);
        save(&data, &k).unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&file).unwrap();
        std::io::Write::write_all(
            &mut f,
            b"{\"kind\":\"duty_start\",\"issued\":5}\n{\"kind\":\"trip\",\"issued\":5,\"trip\":2,\"line\":\"9\",\"how\":\"arrived\"}\n{\"kind\":\"defect\",\"issued\":5,\"bus_id\":3,\"defect\":\"tyres\"}\n{\"kind\":\"live\",\"issued\":5,\"event\":{\"kind\":\"breakdown\",\"vehicle\":3}}\n",
        )
        .unwrap();
        let b = book_from(&data, &plugin);
        assert_eq!((b.reports, b.trips, b.orders, b.live), (4, 1, 1, 1));
        assert_eq!((b.drivers.as_slice(), b.companies.as_slice()), (["Luc".to_string()].as_slice(), ["c1".to_string()].as_slice()));
        let trips = hub_core::trips_of(&data, "Luc");
        assert_eq!((trips.len(), trips[0].trip), (1, 2));
        assert_eq!(remote::pending(&data, "c1").len(), 1);
        assert!(std::fs::read_to_string(store::live_file(&data, "c1")).unwrap().contains("breakdown"));
        // (again: nothing new)
        assert_eq!(book_from(&data, &plugin).reports, 0);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn every_text_the_add_on_says_is_translated() {
        use hub_game::{compat, detect};
        let mut texts: Vec<&str> = install::TEXTS.to_vec();
        texts.extend(compat::NOTES);
        for f in compat::FEATURES {
            texts.push(f.label);
            if !f.fallback.is_empty() {
                texts.push(f.fallback);
            }
        }
        for p in [detect::Problem::Missing, detect::Problem::Omsi2Only, detect::Problem::NoGame] {
            texts.push(p.text());
        }
        let mut missing = Vec::new();
        for lang in ["nl", "de", "fr", "ru", "uk", "pl", "es"] {
            let table = hub_locales::texts(lang);
            for t in &texts {
                match table.get(*t) {
                    // (the holes stay, so the plugin and the page fill them)
                    Some(tr) if t.matches("%{").count() == tr.matches("%{").count() => {}
                    _ => missing.push(format!("{lang}: {t:?}")),
                }
            }
        }
        assert!(missing.is_empty(), "{} missing or with other holes (crates/hub-locales/locales/tauri-addon.yml):\n{}", missing.len(), missing.join("\n"));
        assert_eq!(plugin_texts("de").get("Start duty").map(String::as_str), hub_locales::texts("de").get("Start duty").map(String::as_str));
    }
}
