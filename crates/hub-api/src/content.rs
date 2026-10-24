//! The drive's steps (`hub_`): what the page's start and steps read of the player's OMSI 2 -
//! the maps, the buses, the weather, the day's lines, the depot files, the shifts, the bus's
//! options and display fonts, the route map. Routed here by [`crate::hub`] for every `hub_`
//! command it does not answer itself. Everything is read through `hub_omsi::vfs` (the content
//! folder first, then the OMSI 2 folder, in any case) and nothing of OMSI 2 is written.
//!
//! * `hub_maps` - the installed maps (`[{ file, name, friendly, entry_points, hof … }]`);
//! * `hub_buses` - the buses a player can drive, with their paints and fleet numbers;
//! * `hub_weather` - the weather files (`[{ file, name }]`);
//! * `hub_lines {map, date}` - the map's lines on the day (`hub_core::LineInfo`);
//! * `hub_default_hof {map, bus, date, line}` - the depot file the drive gives the bus;
//! * `hub_ibis {bus, hof, line}` - what to type into the IBIS: the routes the depot file has
//!   for the line;
//! * `hub_situations {map}` - the situations to continue: none yet (`[]`);
//! * `hub_route_map {map, date, trip}` - the map's roads, the trip's route (`a|b`, or `a|` for
//!   one: each trip's by its name), its frame and entry points, as the game's city map has them
//!   (see [`mapview`]);
//! * `hub_map_picture {map}` (bytes) - the map folder's `picture.jpg`;
//! * `hub_settings_patch {patch}` - settings changed (the look's keys go to `Settings.other`);
//! * `hub_shifts`, `hub_duty_extras` - see [`shifts`];
//! * `hub_map_facts`, `hub_weather_extras`, `hub_own_lines` - see [`facts`];
//! * `hub_depots`, `hub_depot_add` - see [`depots`];
//! * `hub_more_bus_best` - see [`busbest`]; `hub_more_weather_read`, `hub_more_weather_write` -
//!   see [`weather`]; `hub_more_bus_options`, `hub_more_bus_option_set`, `hub_more_setvars` -
//!   see [`busoptions`]; `hub_more_font_*` and `hub_display_preview` - see [`fonts`];
//! * `b3d_*` - the bus in 3D, its photos and the livery studio's bus, read as the game reads it
//!   with openOMSI's readers (the old app's Bus3D): see [`bus3d`];
//! * `hub_le` - the line editor: not available yet (it needs the map's lane graph).

#[cfg(feature = "native")]
pub mod bus3d;
pub mod busbest;
#[cfg(feature = "native")]
pub mod busoptions;
pub mod depots;
pub mod facts;
pub mod fonts;
#[cfg(feature = "native")]
pub mod mapview;
// (the website: the maps as the desktop app exported them into the cloud save's content)
#[cfg(not(feature = "native"))]
#[path = "content/mapview_web.rs"]
pub mod mapview;
pub mod shifts;
pub mod weather;

use crate::launcher::MapInfo;
use crate::{arg, to_json, Answer, Platform};
use hub_core::settings::Settings;
use hub_core::VehicleInfo;
use hub_omsi::Vfs;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// A `hub_` command of the drive's steps.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    match cmd {
        "hub_maps" => to_json(vfs().map(|v| maps_in(&v)).unwrap_or_default()),
        "hub_buses" => to_json(vfs().map(|v| buses_in(&v)).unwrap_or_default()),
        "hub_weather" => to_json(vfs().map(|v| weather_in(&v)).unwrap_or_default()),
        "hub_lines" => to_json(hub_core::list_lines(&arg::<String>(args, "map")?, &date_arg(args)?)?),
        "hub_default_hof" => {
            let v = need_vfs()?;
            Ok(json!(default_hof(&v, &arg::<String>(args, "map")?, &str_arg(args, "bus"), &date_arg(args)?, arg::<Option<String>>(args, "line")?.as_deref())))
        }
        "hub_ibis" => Ok(ibis(&need_vfs()?, &str_arg(args, "bus"), &str_arg(args, "hof"), &str_arg(args, "line"))),
        "hub_situations" => Ok(json!([])),
        "hub_route_map" => route_map(&arg::<String>(args, "map")?, &str_arg(args, "date"), &str_arg(args, "trip")),
        "hub_settings_patch" => settings_patch(args.get("patch").unwrap_or(&Value::Null)),
        "hub_shifts" => shifts::shifts(args),
        "hub_duty_extras" => shifts::duty_extras(args),
        "hub_map_facts" => facts::map_facts(&arg::<Vec<String>>(args, "maps")?),
        "hub_weather_extras" => facts::weather_extras(&str_arg(args, "map")),
        "hub_own_lines" => Ok(facts::own_lines(&arg::<String>(args, "map")?)),
        "hub_depots" => depots::depots(args),
        "hub_depot_add" => depots::add(&arg::<String>(args, "bus")?, &arg::<String>(args, "want")?, &arg::<String>(args, "source")?),
        "hub_more_bus_best" => busbest::bus_best(args),
        "hub_more_weather_read" => weather::read(args),
        "hub_more_weather_write" => weather::write(args),
        #[cfg(feature = "native")]
        "hub_more_bus_options" => busoptions::view(args),
        #[cfg(feature = "native")]
        "hub_more_bus_option_set" => busoptions::set(args),
        #[cfg(feature = "native")]
        "hub_more_setvars" => busoptions::for_game(args),
        "hub_more_font_view" => fonts::view(args),
        "hub_more_font_set" => fonts::set(args),
        "hub_more_font_add" => fonts::add(args),
        "hub_display_preview" => Ok(fonts::display_preview(&str_arg(args, "bus"), &str_arg(args, "text"))),
        // (the line editor, the old app's headless one: every change a request, the view answered)
        #[cfg(feature = "native")]
        "hub_le" => Ok(crate::lineeditor::headless::serve(args)),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// A `hub_` command of the drive's steps that answers bytes.
pub fn call_bytes(cmd: &str, args: &Value, _p: &dyn Platform) -> Result<Vec<u8>, String> {
    match cmd {
        "hub_map_picture" => map_picture(&need_vfs()?, &arg::<String>(args, "map")?),
        "hub_more_font_pic" => fonts::picture(args),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

// --- helpers the parts share ---------------------------------------------------------------------

/// The OMSI 2 content as the game sees it (None: the OMSI 2 folder is not set).
pub(crate) fn vfs() -> Option<Vfs> {
    hub_core::paths::vfs()
}

pub(crate) fn need_vfs() -> Result<Vfs, String> {
    vfs().ok_or_else(|| "the OMSI 2 folder is not set".to_string())
}

/// A text argument ("" when it is missing or no text).
pub(crate) fn str_arg(args: &Value, name: &str) -> String {
    args.get(name).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The `date` argument (`YYYY-MM-DD`), the app's default date when there is none.
pub(crate) fn date_arg(args: &Value) -> Result<String, String> {
    Ok(arg::<Option<String>>(args, "date")?.filter(|d| !d.trim().is_empty()).unwrap_or_else(|| hub_core::DEFAULT_DATE.to_string()))
}

/// A path as it is kept: `/` between its parts.
pub(crate) fn slashed(p: &str) -> String {
    p.replace('\\', "/")
}

/// The folder of a map's `global.cfg` (`maps/Grundorf/global.cfg` → `maps/Grundorf`).
pub(crate) fn map_dir_rel(map: &str) -> String {
    let m = slashed(map);
    m.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default()
}

/// The map's folder on disk (the first content folder that has its `global.cfg`).
pub(crate) fn map_dir(vfs: &Vfs, map: &str) -> Option<PathBuf> {
    vfs.resolve(map).filter(|p| p.is_file()).and_then(|p| p.parent().map(Path::to_path_buf))
}

/// A name to compare: lower case, letters and digits only, the accents off.
pub(crate) fn fold(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().flat_map(char::to_lowercase) {
        match c {
            'ß' => out.push_str("ss"),
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => out.push('a'),
            'è' | 'é' | 'ê' | 'ë' => out.push('e'),
            'ì' | 'í' | 'î' | 'ï' => out.push('i'),
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => out.push('o'),
            'ù' | 'ú' | 'û' | 'ü' => out.push('u'),
            c if c.is_alphanumeric() => out.push(c),
            _ => {}
        }
    }
    out
}

/// Names in a natural order: case aside, a run of digits by its number ("Bus 9" before
/// "bus 10").
pub(crate) fn name_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let x: String = std::iter::from_fn(|| a.next_if(|c| c.is_ascii_digit())).collect();
                let y: String = std::iter::from_fn(|| b.next_if(|c| c.is_ascii_digit())).collect();
                let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let o = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                a.next();
                b.next();
            }
        }
    }
}

/// Words that say nothing of a place: what depot files and maps are called besides it.
const NOT_PLACES: &[&str] = &[
    "hof",
    "depot",
    "betriebshof",
    "linie",
    "line",
    "lines",
    "map",
    "karte",
    "bus",
    "busse",
    "busses",
    "the",
    "und",
    "and",
    "der",
    "die",
    "das",
    "von",
    "omsi",
    "addon",
    "version",
    "neu",
    "new",
    "alt",
    "old",
];

/// The words of a name that may name a place: folded, three letters at least, not a year or a
/// word every depot file has ("Hof", "Linie" …).
pub(crate) fn place_words(name: &str) -> Vec<String> {
    name.split(|c: char| !c.is_alphanumeric()).map(fold).filter(|w| w.chars().count() >= 3 && !w.chars().all(|c| c.is_ascii_digit()) && !NOT_PLACES.contains(&w.as_str())).collect()
}

/// Of `names`, the one that shares the most of the place `hints` name (by the letters of the
/// words they share); None when none shares a word.
pub(crate) fn closest_name(names: &[&str], hints: &[&str]) -> Option<usize> {
    let wanted: Vec<String> = hints.iter().flat_map(|h| place_words(h)).collect();
    let mut best: Option<(usize, usize)> = None;
    for (i, n) in names.iter().enumerate() {
        let mut words = place_words(n);
        words.sort();
        words.dedup();
        let score: usize = words.iter().filter(|w| wanted.contains(w)).map(|w| w.chars().count()).sum();
        if score > 0 && best.is_none_or(|(_, b)| score > b) {
            best = Some((i, score));
        }
    }
    best.map(|(i, _)| i)
}

/// The depot files a bus reads (`Vehicles/<pack>/x.bus`, as the lists name it): those of its
/// own folder in every content folder, else those of its pack's folder - each file name once,
/// the content folder's first. The player's own (`oo_…`) are left out unless `own` says so.
pub(crate) fn bus_depot_files(vfs: &Vfs, bus: &str, own: bool) -> Vec<PathBuf> {
    let f = slashed(bus);
    let dir = f.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
    let pack = f.split('/').take(2).collect::<Vec<_>>().join("/");
    let of = |rel: &str| -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        for b in vfs.bases() {
            let Some(d) = hub_omsi::vfs::resolve_in(b, rel).filter(|d| d.is_dir()) else { continue };
            for p in hub_omsi::hof::depot_files(&d) {
                let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                if !out.iter().any(|o| o.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase() == name)) && (own || !hub_core::owndepot::is_players(&p)) {
                    out.push(p);
                }
            }
        }
        out
    };
    let files = if dir.is_empty() { Vec::new() } else { of(&dir) };
    if files.is_empty() && !pack.is_empty() && pack != dir {
        of(&pack)
    } else {
        files
    }
}

/// What the game is given (`--hof`) for each depot file: its `[name]` (its file name without
/// `.hof` when it has none), unless another one has the same name - then its file name.
pub(crate) fn keys_for(names: &[String], stems: &[String]) -> Vec<String> {
    names.iter().zip(stems).map(|(n, s)| if names.iter().filter(|o| o.trim().eq_ignore_ascii_case(n.trim())).count() > 1 { s.clone() } else { n.clone() }).collect()
}

fn stem_of(p: &Path) -> String {
    p.file_stem().unwrap_or_default().to_string_lossy().trim().to_string()
}

// --- the lists -----------------------------------------------------------------------------------

/// Every map of `vfs` with a `global.cfg`, by its name.
pub fn maps_in(vfs: &Vfs) -> Vec<MapInfo> {
    let mut out = Vec::new();
    for (folder, is_dir) in vfs.list_dir("maps") {
        if !is_dir {
            continue;
        }
        let Some(path) = vfs.resolve(&format!("maps/{folder}/global.cfg")) else { continue };
        let Ok(g) = hub_omsi::map::GlobalCfg::load(&path) else { continue };
        let installed = vfs.bases().len() > 1 && path.starts_with(&vfs.bases()[0]);
        let mut m = MapInfo::of(&g, &folder, installed);
        // (`[standarddepot]` names the AI group: the depot file is the one that group gives)
        let lists = path.parent().and_then(|d| hub_omsi::vfs::find_child(d, "ailists.cfg")).and_then(|p| hub_omsi::map::AiLists::load(&p).ok());
        if let Some(hof) = lists.as_ref().and_then(|l| l.depot_group(g.standard_depot.as_deref())).and_then(|d| d.hof.clone()) {
            m.hof = hof;
        }
        out.push(m);
    }
    out.sort_by(|a, b| name_cmp(&a.friendly, &b.friendly));
    out
}

/// The buses of `vfs` a player can drive (not a car, not a rear section, not a bus for the AI
/// traffic only: `Bus::is_ai_bus`), by name, each with its paints (its `[CTC]` folder's `.cti`
/// items) and its fleet numbers.
pub fn buses_in(vfs: &Vfs) -> Vec<VehicleInfo> {
    let mut out: Vec<VehicleInfo> = Vec::new();
    for (folder, is_dir) in vfs.list_dir("Vehicles") {
        if !is_dir {
            continue;
        }
        let rel_dir = format!("Vehicles/{folder}");
        let files = vfs.list_dir(&rel_dir);
        let hofs: Vec<String> = files.iter().filter(|(n, d)| !d && n.to_lowercase().ends_with(".hof")).map(|(n, _)| n.clone()).collect();
        for (name, d) in &files {
            if *d || !name.to_lowercase().ends_with(".bus") {
                continue;
            }
            let rel = format!("{rel_dir}/{name}");
            let Some(path) = vfs.resolve(&rel) else { continue };
            let Ok(bus) = hub_omsi::vehicle::Bus::load(&path) else { continue };
            if bus.is_traffic() || bus.is_rear_section() || bus.is_ai_bus(&rel) {
                continue;
            }
            let installed = vfs.bases().len() > 1 && path.starts_with(&vfs.bases()[0]);
            let mut v = VehicleInfo::of_bus(&bus, &rel, &folder, installed, hofs.clone());
            // (its paints in every content folder, as openOMSI offers them)
            v.paints = bus.paints_in_vfs(vfs).into_iter().filter(|p| !p.trim().is_empty()).collect();
            out.push(v);
        }
    }
    out.sort_by(|a, b| name_cmp(&a.name, &b.name).then_with(|| a.file.cmp(&b.file)));
    out
}

/// The bus `file` (relative to the OMSI 2 folder) is one for the AI traffic only
/// (`Bus::is_ai_bus`: its names, else its `[ai_veh_type]`); a bus not found is none.
pub fn is_ai_bus(vfs: &Vfs, file: &str) -> bool {
    let rel = slashed(file);
    hub_omsi::vehicle::is_ai_name(&rel) || vfs.resolve(&rel).and_then(|p| hub_omsi::CfgFile::read(&p).ok()).is_some_and(|c| hub_omsi::vehicle::Bus::parse(&c).is_ai_bus(&rel))
}

/// A weather file as the day step lists it: its file and name, and what it holds - the
/// temperature (°C), what falls (`none`, `rain`, `snow`), the visibility (m), the clouds'
/// texture and snow on the ground - by which the step offers the presets that fit the season
/// and says what each is (none of them when the file cannot be read).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct WeatherRow {
    pub file: String,
    pub name: String,
    pub temp: Option<f32>,
    pub precip: Option<&'static str>,
    pub fog_m: Option<f32>,
    pub clouds: Option<String>,
    pub snow: Option<bool>,
}

/// The weather files of `vfs`'s `Weather` folder, by their file names, each with what it holds.
pub fn weather_in(vfs: &Vfs) -> Vec<WeatherRow> {
    vfs.list_dir("Weather")
        .into_iter()
        .filter(|(n, d)| !d && n.to_lowercase().ends_with(".owt"))
        .map(|(n, _)| {
            let (file, name) = (format!("Weather/{n}"), n[..n.len() - 4].replace('_', " "));
            let w = vfs.resolve(&file).and_then(|p| hub_omsi::weather::Weather::load(&p).ok());
            let precip = w.as_ref().map(|w| match w.precipitation {
                hub_omsi::weather::Precipitation::None => "none",
                hub_omsi::weather::Precipitation::Rain => "rain",
                hub_omsi::weather::Precipitation::Snow => "snow",
            });
            WeatherRow {
                file,
                name,
                temp: w.as_ref().map(|w| w.temperature),
                precip,
                fog_m: w.as_ref().map(|w| w.fog_m),
                clouds: w.as_ref().map(|w| w.clouds.clone()),
                snow: w.as_ref().map(|w| w.snow),
            }
        })
        .collect()
}

// --- the depot file and the IBIS -------------------------------------------------------------

/// The map's depot file on `date` (the `[name]` its depot group's buses get, a Chrono folder's
/// change applied), else the one its `[standarddepot]` names.
pub(crate) fn map_hof(vfs: &Vfs, map: &str, date: &str) -> String {
    let Some(dir) = map_dir(vfs, map) else { return String::new() };
    let on_date = chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok().and_then(|d| hub_omsi::map::depot_hof_on(&dir, d));
    on_date.unwrap_or_default()
}

/// The depot file the drive gives the bus (what `--hof` names): the player's own one the
/// driven line of his asks for, else the map's on the day, else the bus's first.
pub fn default_hof(vfs: &Vfs, map: &str, bus: &str, date: &str, line: Option<&str>) -> String {
    if let Some(line) = line.map(str::trim).filter(|l| !l.is_empty()) {
        let own = hub_core::lines::own_lines_of_map(map);
        if let Some(o) = own.iter().find(|o| o.file.eq_ignore_ascii_case(line)).filter(|o| !o.depot_file.trim().is_empty()) {
            return o.depot_file.trim().to_string();
        }
    }
    let want = map_hof(vfs, map, date);
    if !want.trim().is_empty() {
        return want;
    }
    let files = bus_depot_files(vfs, bus, false);
    let stems: Vec<String> = files.iter().map(|f| stem_of(f)).collect();
    let names: Vec<String> = files.iter().zip(&stems).map(|(f, s)| hub_omsi::hof::Hof::read_name(f).map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| s.clone())).collect();
    keys_for(&names, &stems).into_iter().next().unwrap_or_default()
}

/// What to type into the IBIS for `line` with the bus's depot file `hof` (by what the game is
/// given, its file name or its `[name]`; the bus's first when none answers): `{ hof, line,
/// line_code, routes: [{ code, name, terminus }] }` - the depot file's routes of the line.
pub fn ibis(vfs: &Vfs, bus: &str, hof: &str, line: &str) -> Value {
    let files = bus_depot_files(vfs, bus, true);
    let path = files.iter().find(|f| hub_core::depot::answers_to(f, hof)).or_else(|| files.first());
    let number = hub_core::linehof::line_number(line);
    let line_code = if number > 0 { number.to_string() } else { line.trim().to_string() };
    let Some(h) = path.and_then(|p| hub_omsi::hof::Hof::load(p).ok()) else {
        return json!({ "hof": hof, "line": line, "line_code": line_code, "routes": [] });
    };
    let same = |l: &str| l.trim().eq_ignore_ascii_case(line.trim()) || (number > 0 && hub_core::linehof::line_number(l) == number);
    let routes: Vec<Value> = h
        .info_trips
        .iter()
        .filter(|t| same(&t.line))
        .map(|t| {
            let terminus = h.terminus(hub_omsi::text::parse_i32(&t.route)).map(|x| x.strings.iter().find(|s| !s.trim().is_empty()).cloned().unwrap_or_else(|| x.texture_id.clone()));
            json!({ "code": t.code.trim(), "name": t.name.trim(), "terminus": terminus.unwrap_or_default().trim() })
        })
        .collect();
    let name = if h.name.trim().is_empty() { path.map(|p| stem_of(p)).unwrap_or_default() } else { h.name.trim().to_string() };
    json!({ "hof": name, "line": line, "line_code": line_code, "routes": routes, "destinations": h.termini.len() })
}

// --- the map -------------------------------------------------------------------------------------

/// The map as the route map draws it (`content::mapview::export`: `{ origin, lo, hi, roads,
/// route, stops, entries, colors }`, metres from `origin`, y north): the roads the game's city map
/// draws and the trip's route over its lanes; a duty's trips (`a|b`) each their route.
pub fn route_map(map: &str, date: &str, trip: &str) -> Answer {
    let global = hub_core::paths::resolve_content(map).filter(|p| p.is_file()).ok_or_else(|| format!("The map {map} is not there"))?;
    // (`a|b`: the trips of a duty, each its route - `a|` a duty of one trip, as the live
    // dashboard asks: each trip by its name, its route the lanes the game drives one after
    // the other)
    let trips: Vec<String> = trip.split('|').map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).collect();
    Ok(if trip.contains('|') { mapview::trips_page(map, &global, date, &trips) } else { mapview::export(map, &global, date, trips.first().map(String::as_str).unwrap_or(""), 0.6) })
}

/// The picture of the map `map` (`maps/<folder>/global.cfg`): its folder's `picture.jpg`, in
/// any case and from whichever content folder has it.
pub fn map_picture(vfs: &Vfs, map: &str) -> Result<Vec<u8>, String> {
    let dir = map_dir_rel(map);
    for name in ["picture.jpg", "picture.jpeg", "picture.png", "picture.bmp"] {
        let rel = if dir.is_empty() { name.to_string() } else { format!("{dir}/{name}") };
        if vfs.is_file(&rel) {
            return vfs.read(&rel).map_err(|e| e.to_string());
        }
    }
    Err(format!("no picture of {map}"))
}

// --- the settings --------------------------------------------------------------------------------

/// The settings a patch does not change: the folders and the setup go through the first-run
/// setup's own checks (`hub_setup_save`).
const NOT_PATCHED: &[&str] = &["openomsi_folder", "omsi2_folder", "setup_done"];

/// `patch` applied to `s`: a key of the file's own fields to it (`language` …), any other one
/// into `other` (the look: `dark_mode`, `accent`, `launcher_scale` …); `null` takes a key out
/// (a field back to its default). A value of the wrong kind for a field is refused, and then
/// nothing changes.
pub fn apply_patch(s: &mut Settings, patch: &Map<String, Value>) -> Result<(), String> {
    let mut v = serde_json::to_value(&*s).map_err(|e| e.to_string())?;
    let o = v.as_object_mut().ok_or("the settings are no object")?;
    for (k, x) in patch {
        if NOT_PATCHED.contains(&k.as_str()) {
            continue;
        }
        if x.is_null() {
            o.remove(k);
        } else {
            o.insert(k.clone(), x.clone());
        }
    }
    *s = serde_json::from_value(v).map_err(|e| format!("patch: {e}"))?;
    Ok(())
}

/// The settings with `patch` applied, saved: the whole settings.
fn settings_patch(patch: &Value) -> Answer {
    let patch = match patch {
        Value::Object(m) => m.clone(),
        Value::Null => Map::new(),
        _ => return Err("patch: an object of settings".into()),
    };
    let mut s = Settings::load();
    apply_patch(&mut s, &patch)?;
    if !patch.is_empty() {
        s.save().map_err(|e| format!("the settings could not be saved: {e}"))?;
    }
    to_json(&s)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// hub-omsi's synthetic OMSI 2 folder (read only).
    pub(crate) fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../hub-omsi/tests/fixtures/omsi2")
    }

    pub(crate) fn fixture_vfs() -> Vfs {
        Vfs::new(fixture(), None)
    }

    /// A fresh folder of the tests' own.
    pub(crate) fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("omsihub-content-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_maps_buses_and_weather_of_an_omsi_2_folder() {
        let v = fixture_vfs();
        let maps = maps_in(&v);
        assert_eq!(maps.len(), 1);
        let m = &maps[0];
        assert_eq!((m.file.as_str(), m.name.as_str(), m.friendly.as_str()), ("maps/Testdorf/global.cfg", "Testdorf", "Testdorf (test map)"));
        assert_eq!(m.entry_points.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Depot", "Grünau Markt"]);
        assert_eq!(m.hof, "Testdorf", "the depot file of the [standarddepot] group");
        let j = serde_json::to_value(m).unwrap();
        assert_eq!(j["entry_points"][1]["name"], "Grünau Markt");

        let buses = buses_in(&v);
        assert_eq!(buses.len(), 1, "the trailer is a rear section: {buses:?}");
        let b = &buses[0];
        assert_eq!(b.file, "Vehicles/Testbus/Testbus.bus");
        assert_eq!(b.folder, "Testbus");
        assert_eq!(b.manufacturer, "Testwerk");
        assert_eq!(b.paints, ["Blau", "Red"]);
        assert!(!b.numbers.is_empty());
        assert_eq!(b.hofs, ["Testdorf.hof"]);
        // (the page reads the paints as names, the numbers as pairs)
        let j = serde_json::to_value(b).unwrap();
        assert!(j["paints"][0].is_string() && j["numbers"][0].is_array());

        let w = weather_in(&v);
        assert_eq!(w.iter().map(|x| (x.file.as_str(), x.name.as_str())).collect::<Vec<_>>(), [("Weather/Grey.owt", "Grey"), ("Weather/Snow.owt", "Snow")]);
        // (what each holds, for the day step's seasons and its words: no NaN there)
        assert_eq!((w[0].temp, w[0].precip, w[0].clouds.as_deref()), (Some(11.0), Some("rain"), Some("Overcast 1")), "{:?}", w[0]);
        assert!(w[0].fog_m.is_some_and(|f| (f - 200.0).abs() < 0.01), "{:?}", w[0]);
        assert_eq!((w[1].precip, w[1].snow.is_some()), (Some("snow"), true), "{:?}", w[1]);
        let j = serde_json::to_value(&w[0]).unwrap();
        assert!(j["temp"].is_number() && j["fog_m"].is_number() && j["precip"].is_string(), "{j}");
    }

    #[test]
    fn a_bus_for_the_ai_traffic_is_offered_nowhere_to_drive_or_to_buy() {
        let root = temp("ai-buses");
        let bus = |rel: &str, maker: &str, type_name: &str, extra: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, format!("[friendlyname]\r\n{maker}\r\n{type_name}\r\nweiss\r\n[passengercabin]\r\nmodel\\cabin.cfg\r\n{extra}")).unwrap();
        };
        bus("Vehicles/MAN_SD202/SD202.bus", "MAN", "SD 202", "");
        bus("Vehicles/MAN_SD202_AI/SD202.bus", "MAN", "SD 202", "");
        bus("Vehicles/Citaro/Citaro.bus", "Mercedes-Benz", "Citaro KI", "");
        bus("Vehicles/Citaro/Citaro_typed.bus", "Mercedes-Benz", "Citaro", "[ai_veh_type]\r\n1\r\n");
        let v = Vfs::new(root.clone(), None);
        // (the drive's bus list: the bus to drive alone)
        let files = |l: &[VehicleInfo]| l.iter().map(|b| b.file.clone()).collect::<Vec<_>>();
        assert_eq!(files(&buses_in(&v)), ["Vehicles/MAN_SD202/SD202.bus"]);
        // (the company's: the AI buses by their names known, but neither on the market nor in
        // the line editor's choice)
        let all = hub_core::vehicles::vehicles_in(&v);
        assert_eq!(all.iter().filter(|b| b.ai).count(), 2, "{all:?}");
        assert!(all.iter().any(|b| b.file == "Vehicles/MAN_SD202/SD202.bus" && !b.ai));
        let market: Vec<String> = hub_core::company::market::market_of(&all).into_iter().map(|b| b.file).collect();
        assert_eq!(market, ["Vehicles/MAN_SD202/SD202.bus"]);
        let cat = crate::lineeditor::catalogue(&all);
        assert_eq!(cat.iter().flat_map(|m| m.1.iter().flat_map(|k| k.1.iter().map(|x| x.0.clone()))).collect::<Vec<_>>(), ["Vehicles/MAN_SD202/SD202.bus"]);
        // (nor started: a choice of one made before is refused)
        for ai in ["Vehicles/MAN_SD202_AI/SD202.bus", "vehicles\\citaro\\citaro.bus", "Vehicles/Citaro/Citaro_typed.bus"] {
            assert!(is_ai_bus(&v, ai), "{ai}");
        }
        assert!(!is_ai_bus(&v, "Vehicles/MAN_SD202/SD202.bus") && !is_ai_bus(&v, "Vehicles/Gone/Gone.bus"));
        // (a choice saved before the field was there reads as a bus to drive)
        let old: VehicleInfo = serde_json::from_str(r#"{"name":"X","file":"a.bus"}"#).unwrap();
        assert!(!old.ai);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_map_s_picture_is_found_in_any_case() {
        let root = temp("picture");
        std::fs::create_dir_all(root.join("Maps/Hilldorf")).unwrap();
        std::fs::write(root.join("Maps/Hilldorf/Picture.JPG"), [0xFF, 0xD8, 0xFF, 1, 2]).unwrap();
        let v = Vfs::new(root.clone(), None);
        assert_eq!(map_picture(&v, "maps\\hilldorf\\global.cfg").unwrap(), [0xFF, 0xD8, 0xFF, 1, 2]);
        assert!(map_picture(&v, "maps/Other/global.cfg").is_err());
        // (the content folder's copy first)
        let content = temp("picture-content");
        std::fs::create_dir_all(content.join("maps/hilldorf")).unwrap();
        std::fs::write(content.join("maps/hilldorf/picture.jpg"), [9]).unwrap();
        assert_eq!(map_picture(&Vfs::new(root.clone(), Some(content.clone())), "maps/Hilldorf/global.cfg").unwrap(), [9]);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(content);
    }

    #[test]
    fn a_patch_goes_to_the_settings_fields_and_the_rest_is_kept() {
        let mut s = Settings { omsi2_folder: "C:/OMSI 2".into(), language: "en".into(), ..Default::default() };
        let patch = json!({ "language": "nl", "dark_mode": true, "accent": "#ff8800", "launcher_scale": 1.25, "omsi2_folder": "D:/elsewhere" });
        apply_patch(&mut s, patch.as_object().unwrap()).unwrap();
        assert_eq!(s.language, "nl");
        assert_eq!(s.omsi2_folder, "C:/OMSI 2", "the folders go through the setup");
        assert_eq!((s.other["dark_mode"].clone(), s.other["accent"].clone()), (json!(true), json!("#ff8800")));
        // (null takes a key out; a field of the wrong kind changes nothing)
        apply_patch(&mut s, json!({ "accent": null }).as_object().unwrap()).unwrap();
        assert!(!s.other.contains_key("accent"));
        let before = s.clone();
        assert!(apply_patch(&mut s, json!({ "language": 5, "animations": false }).as_object().unwrap()).is_err());
        assert_eq!(s, before);
    }

    #[test]
    fn hub_settings_patch_saves_and_answers_the_whole_settings() {
        let data = crate::online::test_data_dir();
        let v = call("hub_settings_patch", &json!({ "patch": { "content_test_key": 7, "company_pins": ["a"] } }), &crate::Quiet).unwrap();
        assert_eq!(v["content_test_key"], 7);
        assert!(v.get("omsi2_folder").is_some(), "the whole settings");
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(data.join("settings.json")).unwrap()).unwrap();
        assert_eq!(saved["company_pins"], json!(["a"]));
        let v = call("hub_settings_patch", &json!({ "patch": { "content_test_key": null, "company_pins": null } }), &crate::Quiet).unwrap();
        assert!(v.get("content_test_key").is_none());
        assert!(call("hub_settings_patch", &json!({ "patch": 3 }), &crate::Quiet).is_err());
    }

    #[test]
    fn the_depot_file_and_the_ibis_of_a_bus() {
        let v = fixture_vfs();
        let hof = default_hof(&v, "maps/Testdorf/global.cfg", "Vehicles/Testbus/Testbus.bus", "1989-05-30", None);
        assert_eq!(hof, "Testdorf", "the map's depot group's file");
        let i = ibis(&v, "Vehicles/Testbus/Testbus.bus", &hof, "10");
        assert_eq!((i["hof"].as_str(), i["line_code"].as_str()), (Some("Testdorf"), Some("10")));
        assert_eq!(i["routes"][0], json!({ "code": "1001", "name": "DEPOT-KIRCHE", "terminus": "KIRCHE" }));
        assert_eq!(i["routes"].as_array().map(Vec::len), Some(2));
        assert_eq!(ibis(&v, "Vehicles/Testbus/Testbus.bus", &hof, "77")["routes"], json!([]));
    }

    #[test]
    fn the_route_map_has_the_map_s_entry_points() {
        let global = fixture_vfs().resolve("maps/Testdorf/global.cfg").unwrap();
        let m = mapview::export("maps/Testdorf/global.cfg", &global, "", "10_out", 0.6);
        // (the test map has no tile files: no roads, no route; the entry points on their tiles)
        assert_eq!((m["roads"].clone(), m["route"].clone()), (json!([]), json!([])));
        assert_eq!(m["entries"][0], json!({ "index": 0, "name": "Depot", "at": [10.0, 20.0] }));
        assert_eq!(m["entries"][1]["at"], json!([350.0, 60.0]));
    }

    #[test]
    fn names_compare_and_places_are_told() {
        assert_eq!(name_cmp("Bus 9", "bus 10"), Ordering::Less);
        assert_eq!(fold("Heerstraße"), "heerstrasse");
        let names = ["Grundorf", "Hamburg Linie 20 (2014)", "Spandau"];
        assert_eq!(closest_name(&names, &["Berlin-Spandau 1986"]), Some(2));
        assert_eq!(closest_name(&names, &["HH Linie"]), None);
        assert_eq!(keys_for(&["Hamburg".into(), "hamburg".into(), "Grundorf".into()], &["HH_2014".into(), "HH_2022".into(), "Grundorf".into()]), ["HH_2014", "HH_2022", "Grundorf"]);
    }

    #[test]
    fn the_line_editor_answers_its_view_and_unknown_commands_are_refused() {
        assert!(call("hub_le", &json!({ "op": "view" }), &crate::Quiet).is_ok_and(|v| v.is_object()));
        assert_eq!(call("hub_situations", &json!({ "map": "maps/x/global.cfg" }), &crate::Quiet), Ok(json!([])));
        assert_eq!(call("hub_nope", &json!({}), &crate::Quiet), Err("unknown command: hub_nope".into()));
        assert_eq!(call_bytes("hub_nope", &json!({}), &crate::Quiet), Err("unknown command: hub_nope".into()));
    }

    /// The lists of a real OMSI 2 folder, only read:
    /// `HUB_OMSI_REAL_ROOT=<OMSI 2 folder> cargo test -p hub-api content -- --ignored`.
    #[test]
    #[ignore = "needs a real OMSI 2 folder"]
    fn a_real_omsi_2_folder_is_listed() {
        let Some(root) = std::env::var_os("HUB_OMSI_REAL_ROOT").map(PathBuf::from) else { return };
        let v = Vfs::new(root, None);
        let maps = maps_in(&v);
        let buses = buses_in(&v);
        assert!(!maps.is_empty() && !buses.is_empty() && !weather_in(&v).is_empty());
        let files: Vec<String> = maps.iter().map(|m| m.file.clone()).collect();
        let facts = facts::facts_of(&v, &files);
        for m in &maps {
            let pic = map_picture(&v, &m.file).map(|b| b.len()).unwrap_or(0);
            let rm = v.resolve(&m.file).map(|g| mapview::export(&m.file, &g, "", "", 0.6)).unwrap_or_default();
            println!(
                "{}: hof {:?}, {} entries ({} placed), {} roads, picture {} bytes, {}",
                m.file,
                m.hof,
                m.entry_points.len(),
                rm["entries"].as_array().map_or(0, Vec::len),
                rm["roads"].as_array().map_or(0, Vec::len),
                pic,
                facts[&m.file]
            );
        }
        let with_paints = buses.iter().filter(|b| !b.paints.is_empty()).count();
        println!("{} buses, {with_paints} with paints, {} airports", buses.len(), facts::metar_airports(&v).len());
    }
}
