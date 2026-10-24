//! A bus: the header of its `.bus` file ([`Bus`]: names, model, cabin, coupling, numbers) and
//! its `passengercabin.cfg` ([`PassengerCabin`]). Not its scripts' contents or its model: Omsi-Hub
//! needs what the dealer, the fleet and the drive steps show.
//!
//! The `.bus` keywords read:
//! * `[friendlyname]` (manufacturer, type, default paint; OMSI offers only a bus that has it),
//!   `[description]` … `[end]`;
//! * `[model]`, `[sound]`, `[sound_ai]`, `[passengercabin]`: files relative to the bus's folder;
//! * `[script]`, `[constfile]`, `[varnamelist]`, `[stringvarnamelist]`: a count, then the files;
//! * `[boundingbox]` (size x, y, z, centre x, y, z), `[couple_back]`/`[couple_front]` (the
//!   coupled section's `.bus` and whether it is turned round), `[coupling_front]` (a section
//!   that something couples to: a rear section);
//! * `[add_camera_driver]`: the first one's x tells the side the driver sits on (below 0: on
//!   the left, a bus for right-hand traffic);
//! * `[number]` (the file of fleet numbers), `[registration_list]` (the file of the plates, one
//!   a number), `[registration_automatic]` (the plate's prefix and suffix around the number),
//!   `[registration_free]`;
//! * `[ai_veh_type]` (a vehicle for the AI traffic only), `[type]` 2 or `[rail_body_osc]` (on
//!   rails).
//!
//! [`Bus::load`] reads, besides the file, its paints (the `.cti` files of the model's `[CTC]`
//! folder) and its numbers with their plates.

use crate::text::{parse_f32, parse_i32, CfgFile, Reader};
use crate::vfs::resolve_in;
use crate::ReadError;
use std::path::{Path, PathBuf};

/// The scripts a bus runs, as its `.bus` lists them (paths relative to its folder, with `/`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BusScripts {
    pub scripts: Vec<PathBuf>,
    pub constfiles: Vec<PathBuf>,
    pub varlists: Vec<PathBuf>,
    pub string_varlists: Vec<PathBuf>,
}

/// A bus's `.bus` file, its header.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bus {
    pub path: PathBuf,
    /// `[friendlyname]`: the manufacturer, the type's name, the default livery.
    pub manufacturer: String,
    pub type_name: String,
    pub default_paint: String,
    /// `[description]`.
    pub description: String,
    /// The model's `model.cfg`, relative to the bus's folder.
    pub model: Option<String>,
    /// The sound's and the passenger cabin's files, relative to the bus's folder.
    pub sound: Option<String>,
    pub passenger_cabin: Option<String>,
    /// Only for the AI (`[ai_veh_type]`).
    pub ai_only: bool,
    /// A section coupled behind (its `.bus`, and whether it is turned round) and before.
    pub couple_back: Option<(String, bool)>,
    pub couple_front: Option<(String, bool)>,
    /// The bounding box: size x, y, z, then centre x, y, z (m).
    pub bounding_box: Option<[f32; 6]>,
    /// The fleet numbers of its `[number]` list with the plate each gives (empty: none; read
    /// by [`Bus::load`]).
    pub numbers: Vec<(String, String)>,
    /// Its registration list (`[registration_list]`), when it has one.
    pub registration_list: Option<String>,
    /// It runs on rails (a tram).
    pub rail: bool,
    /// It has a `[friendlyname]` (OMSI offers only such a bus).
    pub has_friendly_name: bool,
    /// The paints it can wear (the names of its `.cti` files' items; read by [`Bus::load`]).
    pub paints: Vec<String>,
    /// The model's paint folder (`[CTC]` in `model.cfg`), relative to the bus's folder (read by
    /// [`Bus::load`]): the first of `ctc_dirs`, every `[CTC]` the model has.
    pub ctc_dir: Option<String>,
    pub ctc_dirs: Vec<String>,
    pub scripts: BusScripts,
    /// `[sound_ai]`.
    pub sound_ai: Option<String>,
    /// The first driver's camera's x (m): below 0 the driver sits on the left.
    pub driver_camera_x: Option<f32>,
    /// It has a `[coupling_front]`: another section couples to its front.
    pub coupling_front: bool,
    /// `[number]`: the file of its fleet numbers.
    pub number_list: Option<String>,
    /// `[registration_automatic]`: the plate's prefix and suffix around the number.
    pub registration_automatic: Option<(String, String)>,
    /// `[registration_free]`.
    pub registration_free: bool,
}

/// A path as the file has it, with `/`.
fn rel(s: &str) -> String {
    crate::vfs::normalize(s)
}

fn opt_rel(s: &str) -> Option<String> {
    Some(rel(s)).filter(|s| !s.is_empty())
}

fn flag(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes")
}

/// A name that marks a vehicle for the AI traffic (a modder's simpler copy of a bus that only
/// the timetable drives): a word "AI" or "KI" (the German "künstliche Intelligenz"), alone, with
/// a number (`AI2`) or run into what it is (`AIBus`, `KIBusse`, `AI_Traffic`). A word that only
/// holds those letters is not one (`Main`, `Kiel`, `Ikarus`, `Taiwan`).
pub fn is_ai_name(s: &str) -> bool {
    const KINDS: [&str; 10] = ["bus", "busse", "buses", "busses", "traffic", "verkehr", "vehicle", "vehicles", "fahrzeug", "fahrzeuge"];
    s.split(|c: char| !c.is_alphanumeric()).any(|w| {
        let w = w.to_lowercase();
        let Some(rest) = w.strip_prefix("ai").or_else(|| w.strip_prefix("ki")) else { return false };
        rest.chars().all(|c| c.is_ascii_digit()) || KINDS.contains(&rest)
    })
}

/// A count, then as many files.
fn file_list(r: &mut Reader) -> Vec<PathBuf> {
    let n = parse_i32(r.value()).max(0) as usize;
    r.values(n).into_iter().map(rel).filter(|s| !s.is_empty()).map(PathBuf::from).collect()
}

impl Bus {
    pub fn parse(cfg: &CfgFile) -> Bus {
        let mut b = Bus { path: cfg.path.clone(), ..Bus::default() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k.to_ascii_lowercase().as_str() {
                "friendlyname" => {
                    let v = r.values(3);
                    let at = |i: usize| v.get(i).map_or("", |s| s.trim()).to_string();
                    (b.manufacturer, b.type_name, b.default_paint) = (at(0), at(1), at(2));
                    b.has_friendly_name = true;
                }
                "description" => b.description = r.until("end").join("\n").trim().to_string(),
                "model" => b.model = opt_rel(r.value()),
                "sound" => b.sound = opt_rel(r.value()),
                "sound_ai" => b.sound_ai = opt_rel(r.value()),
                "passengercabin" => b.passenger_cabin = opt_rel(r.value()),
                "ai_veh_type" => b.ai_only = true,
                "rail_body_osc" => b.rail = true,
                "type" => b.rail |= parse_i32(r.value()) == 2,
                "coupling_front" => b.coupling_front = true,
                kw @ ("couple_back" | "couple_front") => {
                    let v = r.values(2);
                    let file = v.first().map_or(String::new(), |s| rel(s));
                    if !file.is_empty() {
                        let c = Some((file, v.get(1).is_some_and(|s| flag(s))));
                        if kw == "couple_back" {
                            b.couple_back = c;
                        } else {
                            b.couple_front = c;
                        }
                    }
                }
                "boundingbox" => {
                    let v = r.values(6);
                    if v.len() == 6 {
                        let mut bb = [0.0; 6];
                        for (slot, s) in bb.iter_mut().zip(&v) {
                            *slot = parse_f32(s);
                        }
                        b.bounding_box = Some(bb);
                    }
                }
                "add_camera_driver" => {
                    let x = r.value();
                    if b.driver_camera_x.is_none() && !x.trim().is_empty() {
                        b.driver_camera_x = Some(parse_f32(x));
                    }
                }
                "script" => b.scripts.scripts.extend(file_list(&mut r)),
                "constfile" => b.scripts.constfiles.extend(file_list(&mut r)),
                "varnamelist" => b.scripts.varlists.extend(file_list(&mut r)),
                "stringvarnamelist" => b.scripts.string_varlists.extend(file_list(&mut r)),
                "number" => b.number_list = opt_rel(r.value()),
                "registration_list" => b.registration_list = opt_rel(r.value()),
                "registration_automatic" => {
                    let v = r.values(2);
                    // (the prefix's spaces are the plate's: "B-V " gives "B-V 2530")
                    let at = |i: usize| v.get(i).map_or(String::new(), |s| s.trim_end_matches(['\t', '\r']).to_string());
                    b.registration_automatic = Some((at(0), at(1)));
                }
                "registration_free" => b.registration_free = true,
                _ => {}
            }
        }
        b
    }

    /// The file, and what it points to: its paints and its numbers with their plates.
    pub fn load(path: &Path) -> Result<Bus, ReadError> {
        let mut b = Bus::parse(&CfgFile::read(path)?);
        b.path = path.to_path_buf();
        let dir = b.dir().to_path_buf();
        b.ctc_dirs = b.model.as_deref().and_then(|m| resolve_in(&dir, m)).and_then(|m| CfgFile::read(m).ok()).map(|m| ctc_dirs_of(&m)).unwrap_or_default();
        b.ctc_dir = b.ctc_dirs.first().cloned();
        b.paints = paints_in_dirs(&b.ctc_dirs.iter().filter_map(|c| resolve_in(&dir, c)).collect::<Vec<_>>());
        b.numbers = b.read_numbers();
        Ok(b)
    }

    /// The fleet numbers its `[number]` file lists, with the plate each gets: the registration
    /// list's line of the same place, else the automatic prefix and suffix around the number,
    /// else none.
    pub fn read_numbers(&self) -> Vec<(String, String)> {
        let lines = |f: &Option<String>| -> Vec<String> {
            f.as_deref()
                .and_then(|f| resolve_in(self.dir(), f))
                .and_then(|p| CfgFile::read(p).ok())
                .map(|c| c.lines.iter().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
                .unwrap_or_default()
        };
        let numbers = lines(&self.number_list);
        let plates = lines(&self.registration_list);
        numbers
            .into_iter()
            .enumerate()
            .map(|(i, n)| {
                let plate = match (plates.get(i), &self.registration_automatic) {
                    (Some(p), _) => p.clone(),
                    (None, Some((pre, post))) => format!("{pre}{n}{post}"),
                    _ => String::new(),
                };
                (n, plate)
            })
            .collect()
    }

    /// The file's header alone (without its paints and numbers).
    fn parse_file(path: &Path) -> Option<Bus> {
        let mut b = Bus::parse(&CfgFile::read(path).ok()?);
        b.path = path.to_path_buf();
        Some(b)
    }

    /// The bus's folder (its files' paths are relative to it).
    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new(""))
    }

    /// It is a rear section of an articulated or coupled bus (another section couples to its
    /// front).
    pub fn is_rear_section(&self) -> bool {
        self.couple_front.is_some() || self.coupling_front
    }

    /// A car rather than a bus (a drivable car of a traffic pack, a taxi): no passenger cabin
    /// for passengers to board, or shorter than a bus (its bounding box under 6 m long; the
    /// shortest minibuses are about 7). Omsi-Hub does not offer it to drive a duty with.
    pub fn is_car(&self) -> bool {
        !self.rail && (self.passenger_cabin.is_none() || self.bounding_box.is_some_and(|b| b[1] > 0.0 && b[1] < 6.0))
    }

    /// Made for the AI traffic or a car: not a bus to drive a duty with.
    pub fn is_traffic(&self) -> bool {
        self.ai_only || self.is_car()
    }

    /// A bus for the AI traffic only, read from `file` (its `.bus` relative to the OMSI 2
    /// folder, `Vehicles/<folder>/<name>.bus`): `[ai_veh_type]`, or "AI" or "KI" in its folder,
    /// its file name or its `[friendlyname]` ([`is_ai_name`]). Omsi-Hub offers it nowhere to
    /// drive.
    pub fn is_ai_bus(&self, file: &str) -> bool {
        self.ai_only || is_ai_name(file) || is_ai_name(&self.manufacturer) || is_ai_name(&self.type_name)
    }

    /// Its name as OMSI's bus list shows it: the manufacturer and the type.
    pub fn name(&self) -> String {
        format!("{} {}", self.manufacturer.trim(), self.type_name.trim()).trim().to_string()
    }

    /// The section coupled behind it, found in its folder in any case.
    pub fn couple_back_path(&self) -> Option<PathBuf> {
        self.couple_back.as_ref().and_then(|(f, _)| resolve_in(self.dir(), f))
    }

    /// Its passenger cabin's file, found in its folder in any case.
    pub fn passenger_cabin_path(&self) -> Option<PathBuf> {
        self.passenger_cabin.as_deref().and_then(|f| resolve_in(self.dir(), f))
    }

    /// The driver sits on the left (a bus for right-hand traffic), as its first camera says.
    pub fn left_hand_drive(&self) -> Option<bool> {
        self.driver_camera_x.map(|x| x < 0.0)
    }
}

/// Every `[CTC]` paint folder of a `model.cfg` (relative to the bus's folder), each once.
fn ctc_dirs_of(model: &CfgFile) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut r = model.reader();
    while let Some(k) = r.next_keyword() {
        if k.eq_ignore_ascii_case("CTC") {
            let v = r.values(2);
            if let Some(d) = v.get(1).and_then(|s| opt_rel(s)).filter(|d| !out.iter().any(|o| crate::vfs::same_name(o, d))) {
                out.push(d);
            }
        }
    }
    out
}

/// `rel` (with `..` parts) under `dir`, both relative to a content folder: `None` above it.
fn join_rel(dir: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for part in rel.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        if part == ".." {
            parts.pop()?;
        } else {
            parts.push(part);
        }
    }
    Some(parts.join("/"))
}

impl Bus {
    /// Its paints as openOMSI offers them: the `.cti` files of every `[CTC]` folder of its model,
    /// in every content folder of `vfs` (openOMSI's content folder first, where its mods and the
    /// livery studio put theirs: a file there hides one of the same name in the OMSI 2 folder),
    /// the model found in any of them too. [`Bus::load`] reads only the bus's own folder.
    pub fn paints_in_vfs(&self, vfs: &crate::Vfs) -> Vec<String> {
        let Some(dir) = vfs.relative(self.dir()) else { return self.paints.clone() };
        let model = self.model.as_deref().and_then(|m| join_rel(&dir, m)).and_then(|m| vfs.read_cfg(&m).ok());
        let ctc = model.map(|m| ctc_dirs_of(&m)).unwrap_or_else(|| self.ctc_dirs.clone());
        let mut dirs: Vec<PathBuf> = Vec::new();
        for c in ctc.iter().filter_map(|c| join_rel(&dir, c)) {
            dirs.extend(vfs.bases().iter().filter_map(|b| resolve_in(b, &c)).filter(|d| d.is_dir()));
        }
        paints_in_dirs(&dirs)
    }
}

/// The paints of a `[CTC]` folder: every `[item]` name of its `.cti` files (a file without one:
/// the file's name), each once, in the order of the files' names.
pub fn paints_in(dir: &Path) -> Vec<String> {
    paints_in_dirs(&[dir.to_path_buf()])
}

/// The same over several folders, as the game reads them: a `.cti` file of a name an earlier
/// folder had is not read again.
pub fn paints_in_dirs(dirs: &[PathBuf]) -> Vec<String> {
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut here: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cti"))).collect();
        here.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
        for f in here {
            let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !files.iter().any(|o| o.file_name().is_some_and(|n| crate::vfs::same_name(&n.to_string_lossy(), &name))) {
                files.push(f);
            }
        }
    }
    let mut out: Vec<String> = Vec::new();
    for f in files {
        // (a .cti may hold several liveries, an [item] each: the game offers every one)
        let mut items: Vec<String> = CfgFile::read(&f).map(|c| c.blocks("item").iter().filter_map(|b| b.first()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
        if items.is_empty() {
            items.push(f.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
        }
        for name in items {
            if !out.iter().any(|o| crate::vfs::same_name(o, &name)) {
                out.push(name);
            }
        }
    }
    out
}

/// A place for a passenger.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PassengerPlace {
    pub pos: [f32; 3],
    /// The seat's height (0: a standing place).
    pub height: f32,
    pub rotation: f32,
}

/// A bus's `passengercabin.cfg`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PassengerCabin {
    pub places: Vec<PassengerPlace>,
    /// The doors passengers get in and out by.
    pub entries: usize,
    pub exits: usize,
}

impl PassengerCabin {
    /// `[passpos]` (x, y, z, the seat's height, its rotation), `[entry]` and `[exit]`.
    pub fn parse(cfg: &CfgFile) -> PassengerCabin {
        let mut c = PassengerCabin::default();
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k.to_ascii_lowercase().as_str() {
                "passpos" => {
                    let v = r.values(5);
                    let f = |i: usize| v.get(i).map_or(0.0, |s| parse_f32(s));
                    c.places.push(PassengerPlace { pos: [f(0), f(1), f(2)], height: f(3), rotation: f(4) });
                }
                "entry" => c.entries += 1,
                "exit" => c.exits += 1,
                _ => {}
            }
        }
        c
    }

    pub fn load(path: &Path) -> Result<PassengerCabin, ReadError> {
        Ok(PassengerCabin::parse(&CfgFile::read(path)?))
    }

    pub fn seats(&self) -> usize {
        self.places.iter().filter(|p| p.height > 0.0).count()
    }

    pub fn standing(&self) -> usize {
        self.places.len() - self.seats()
    }
}

/// The seats and standing places of a bus and the sections coupled behind it (None: it has no
/// passenger cabin).
pub fn capacity(bus: &Bus) -> Option<(usize, usize)> {
    let mut total: Option<(usize, usize)> = None;
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut next = Some(bus.clone());
    while let Some(b) = next.take() {
        if let Some(c) = b.passenger_cabin_path().and_then(|p| PassengerCabin::load(&p).ok()) {
            let t = total.get_or_insert((0, 0));
            t.0 += c.seats();
            t.1 += c.standing();
        }
        // (each section once: a pair coupled to each other ends here)
        seen.push(b.path.clone());
        next = b.couple_back_path().filter(|p| !seen.contains(p)).and_then(|p| Bus::parse_file(&p));
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cabin_counts_seats_and_standing_places() {
        let seat = PassengerPlace { height: 0.45, ..PassengerPlace::default() };
        let c = PassengerCabin { places: vec![seat.clone(), seat, PassengerPlace::default()], ..PassengerCabin::default() };
        assert_eq!((c.seats(), c.standing()), (2, 1));
        let b = Bus { path: PathBuf::from("Vehicles/X/a.bus"), ..Bus::default() };
        assert_eq!(b.dir(), Path::new("Vehicles/X"));
    }

    #[test]
    fn a_bus_header_is_read() {
        let text = "\t[registration_free]\r\n[friendlyname]\r\nMAN\r\nSL 200\r\nBeige\r\n[description]\r\nA city bus.\r\n[end]\r\n[number]\r\nRegs.org\r\n[registration_automatic]\r\nTD-V \r\n\r\n[model]\r\nmodel\\model.cfg\r\n[sound]\r\nsound\\sound.cfg\r\n[passengercabin]\r\nmodel\\cabin.cfg\r\n[script]\r\n2\r\nscript\\main.osc\r\nscript\\engine.osc\r\n[constfile]\r\n1\r\nscript\\engine_constfile.txt\r\n[varnamelist]\r\n1\r\nscript\\main_varlist.txt\r\n[add_camera_driver]\r\n-0.9\r\n4.8\r\n1.7\r\n[add_camera_driver]\r\n0.5\r\n[boundingbox]\r\n2.5\r\n11.4\r\n2.9\r\n0\r\n0.9\r\n1.6\r\n[couple_back]\r\ntrail.bus\r\nfalse\r\n";
        let b = Bus::parse(&CfgFile::from_str("a.bus", text));
        assert_eq!((b.manufacturer.as_str(), b.type_name.as_str(), b.default_paint.as_str(), b.name()), ("MAN", "SL 200", "Beige", "MAN SL 200".to_string()));
        assert!(b.has_friendly_name && !b.ai_only && !b.rail && !b.registration_free && !b.is_rear_section());
        assert_eq!((b.model.as_deref(), b.passenger_cabin.as_deref()), (Some("model/model.cfg"), Some("model/cabin.cfg")));
        assert_eq!(b.scripts.scripts, [PathBuf::from("script/main.osc"), PathBuf::from("script/engine.osc")]);
        assert_eq!(b.scripts.scripts[1].file_name().unwrap(), "engine.osc");
        assert_eq!((b.scripts.constfiles.len(), b.scripts.varlists.len()), (1, 1));
        assert_eq!((b.driver_camera_x, b.left_hand_drive()), (Some(-0.9), Some(true)));
        assert_eq!(b.bounding_box, Some([2.5, 11.4, 2.9, 0.0, 0.9, 1.6]));
        assert_eq!(b.couple_back, Some(("trail.bus".to_string(), false)));
        assert_eq!(b.registration_automatic, Some(("TD-V ".to_string(), String::new())));
        assert!(!b.is_car() && !b.is_traffic());
        // (a drivable car: four metres long, no passenger cabin)
        let car = Bus::parse(&CfgFile::from_str("w123.bus", "[friendlyname]\r\nMB\r\nW123 230E\r\nWhite\r\n[model]\r\nmodel\\model.cfg\r\n[boundingbox]\r\n1.8\r\n4.7\r\n1.4\r\n0\r\n0\r\n0.7\r\n"));
        assert!(car.is_car() && car.is_traffic() && !car.ai_only);
        let short = Bus::parse(&CfgFile::from_str("taxi.bus", "[passengercabin]\r\nmodel\\cabin.cfg\r\n[boundingbox]\r\n1.8\r\n4.9\r\n1.5\r\n0\r\n0\r\n0.7\r\n"));
        assert!(short.is_car());
        let trail = Bus::parse(&CfgFile::from_str("trail.bus", "[coupling_front]\r\n0\r\n4.1\r\n0.3\r\n[ai_veh_type]\r\n0\r\n[type]\r\n2\r\n"));
        assert!(trail.is_rear_section() && trail.ai_only && trail.rail && !trail.has_friendly_name);
    }

    #[test]
    fn an_ai_bus_is_known_by_its_names_or_its_ai_veh_type() {
        for ai in [
            "Vehicles/MAN_SD202_AI/SD202.bus",
            "Vehicles/KI-Busse/O530.bus",
            "Vehicles/MAN_NL/NL202_ki2.bus",
            "Vehicles/AIBus Pack/a.bus",
            "Vehicles/Busse/MAN SD200 (KI).bus",
            "Vehicles/x/AI_Traffic.bus",
        ] {
            assert!(is_ai_name(ai), "{ai}");
        }
        for not in ["Vehicles/MAN_SD202/SD202.bus", "Vehicles/Kiel_Citaro/Main.bus", "Vehicles/Ikarus_280/Taiwan.bus", "Vehicles/Kia/Aixam.bus", "Vehicles/Haikou/Kirchberg.bus", ""] {
            assert!(!is_ai_name(not), "{not}");
        }
        let bus = Bus { manufacturer: "MAN".into(), type_name: "SD 202".into(), ..Bus::default() };
        assert!(!bus.is_ai_bus("Vehicles/MAN_SD202/SD202.bus"));
        assert!(bus.is_ai_bus("Vehicles/MAN_SD202/SD202_AI.bus"));
        assert!(Bus { type_name: "SD 202 KI".into(), ..bus.clone() }.is_ai_bus("Vehicles/MAN_SD202/SD202.bus"));
        assert!(Bus { ai_only: true, ..bus }.is_ai_bus("Vehicles/MAN_SD202/SD202.bus"));
    }
}
