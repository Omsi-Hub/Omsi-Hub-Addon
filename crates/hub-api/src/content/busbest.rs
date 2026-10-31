//! `hub_more_bus_best`: the bus that fits the duty best. Of the buses whose depot files know
//! the duty's trips, the map's own fleet before the others, then the one that knows most; a
//! free drive (or a duty no depot file knows) the bus the map's depot runs most.
//!
//! Its arguments: `map`, `date`, `free`, the tour (`line`, `tour`, `first`) or the shift's
//! `legs`, and the buses to weigh (`buses`: `[[file, name]]`; `only` narrows them to a line's
//! own). It answers `{ file, fit, depot, free }`, or null when no bus is offered.

use super::{bus_depot_files, fold, name_cmp, slashed};
use crate::{arg, Answer};
use hub_omsi::Vfs;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A depot file as the fit sees it: its name, the routes its IBIS knows, the destinations it
/// shows.
struct Depot {
    name: String,
    trips: HashSet<String>,
    termini: HashSet<String>,
}

fn depot_of(h: &hub_omsi::hof::Hof) -> Depot {
    let name = if h.name.trim().is_empty() { h.path.file_stem().unwrap_or_default().to_string_lossy().into_owned() } else { h.name.trim().to_string() };
    let trips = h.info_trips.iter().map(|t| fold(&t.name)).filter(|s| !s.is_empty()).collect();
    let mut termini = HashSet::new();
    for t in &h.termini {
        termini.insert(fold(&t.texture_id));
        termini.insert(fold(&t.strings.join(" ")));
        termini.extend(t.strings.iter().map(|s| fold(s)));
    }
    termini.remove("");
    Depot { name, trips, termini }
}

/// How many of the trips (name, terminus) a depot file knows: by the route's name, or by a
/// destination it shows.
fn knows(d: &Depot, trips: &[(String, String)]) -> usize {
    trips.iter().filter(|(name, end)| d.trips.contains(&fold(name)) || (!end.is_empty() && d.termini.contains(&fold(end)))).count()
}

/// A bus to weigh: its file and its name.
type Bus = (String, String);

fn key(file: &str) -> String {
    slashed(file.trim()).to_lowercase()
}

/// How many of each bus file the map's own fleet has (its depot groups' buses and type
/// groups, those of the day `code`).
fn fleet_of(ai: &hub_omsi::map::AiLists, code: Option<i32>) -> HashMap<String, f32> {
    let mut out: HashMap<String, f32> = HashMap::new();
    for g in ai.groups.iter().filter(|g| g.is_depot || g.hof.is_some()) {
        for v in &g.vehicles {
            *out.entry(key(&v.file)).or_default() += v.weight.max(0.0);
        }
        for t in &g.typgroups {
            let n = t.entries.iter().filter(|e| code.is_none_or(|d| e.valid(d))).count();
            *out.entry(key(&t.file)).or_default() += n.max(1) as f32;
        }
    }
    out
}

/// A bus weighed for the duty.
struct Candidate {
    file: String,
    name: String,
    fit: f32,
    depot: String,
    wagons: f32,
}

/// The bus to offer.
fn rank(c: &[Candidate], free: bool) -> Option<Value> {
    let by_name = |a: &Candidate, b: &Candidate| name_cmp(&b.name, &a.name).then_with(|| b.file.cmp(&a.file));
    if !free {
        let fitting: Vec<&Candidate> = c.iter().filter(|x| x.fit > 0.0).collect();
        let own: Vec<&Candidate> = fitting.iter().copied().filter(|x| x.wagons > 0.0).collect();
        let pool = if own.is_empty() { fitting } else { own };
        if let Some(b) = pool.into_iter().max_by(|a, b| a.fit.total_cmp(&b.fit).then(a.wagons.total_cmp(&b.wagons)).then_with(|| by_name(a, b))) {
            return Some(json!({ "file": b.file, "fit": b.fit, "depot": b.depot, "free": false }));
        }
    }
    c.iter().filter(|x| x.wagons > 0.0).max_by(|a, b| a.wagons.total_cmp(&b.wagons).then_with(|| by_name(a, b))).map(|b| json!({ "file": b.file, "fit": null, "depot": "", "free": free }))
}

/// The duty's trips (name, terminus): a work shift's legs, or the tour's trips from the one
/// it starts with.
fn duty_trips(args: &Value, lines: &[hub_core::LineInfo]) -> Result<Vec<(String, String)>, String> {
    let mut trips = Vec::new();
    let legs = arg::<Option<Vec<String>>>(args, "legs")?.unwrap_or_default();
    if !legs.is_empty() {
        for leg in &legs {
            let Some((line, tour, first, count)) = hub_core::compose::Block::parse(leg) else { continue };
            if let Some(t) = lines.iter().find(|x| x.name == line).and_then(|x| x.tours.iter().find(|t| t.number == tour)) {
                trips.extend(t.trips.iter().filter(|x| x.index >= first && x.index < first + count).map(|x| (x.name.clone(), x.terminus.clone())));
            }
        }
    } else if let (Some(line), Some(tour)) = (arg::<Option<String>>(args, "line")?, arg::<Option<String>>(args, "tour")?) {
        let first = arg::<Option<f64>>(args, "first")?.unwrap_or(0.0).max(0.0) as usize;
        if let Some(t) = lines.iter().find(|x| x.name == line).and_then(|x| x.tours.iter().find(|t| t.number == tour)) {
            trips.extend(t.trips.iter().skip(first).map(|x| (x.name.clone(), x.terminus.clone())));
        }
    }
    Ok(trips)
}

pub fn bus_best(args: &Value) -> Answer {
    let Some(vfs) = super::vfs() else { return Ok(Value::Null) };
    let map = arg::<String>(args, "map")?;
    let date = super::date_arg(args)?;
    let free = arg::<Option<bool>>(args, "free")?.unwrap_or(false);
    let trips = if free { Vec::new() } else { duty_trips(args, &hub_core::list_lines(&map, &date).unwrap_or_default())? };
    let mut buses = arg::<Option<Vec<(String, String)>>>(args, "buses")?.unwrap_or_default();
    let only: HashSet<String> = arg::<Option<Vec<String>>>(args, "only")?.unwrap_or_default().iter().map(|f| key(f)).collect();
    if !only.is_empty() {
        buses.retain(|(f, _)| only.contains(&key(f)));
    }
    Ok(best_of(&vfs, &map, &date, free || trips.is_empty(), &trips, &buses))
}

/// The bus of `buses` that fits the trips best on the map `map` on `date`.
fn best_of(vfs: &Vfs, map: &str, date: &str, free: bool, trips: &[(String, String)], buses: &[(String, String)]) -> Value {
    let Some(map_dir) = super::map_dir(vfs, map) else { return Value::Null };
    let day = chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok();
    let chrono = day.map(|d| hub_omsi::map::active_chrono_dirs(&map_dir, d)).unwrap_or_default();
    let ai = hub_omsi::map::AiLists::with_chrono(&map_dir, &chrono);
    let fleet = fleet_of(&ai, day.map(hub_omsi::map::code_of));
    // (each depot file read once: the buses of a folder share them; and kept for the next duty)
    static DEPOTS: hub_omsi::vfs::Kept<Arc<Depot>> = hub_omsi::vfs::Kept::new();
    let mut read: HashMap<String, Vec<Arc<Depot>>> = HashMap::new();
    let mut weigh = |(file, name): &(String, String)| -> Candidate {
        let wagons = fleet.get(&key(file)).copied().unwrap_or(0.0);
        let mut c = Candidate { file: file.clone(), name: name.clone(), fit: 0.0, depot: String::new(), wagons };
        if !free {
            let depots = read.entry(key(file)).or_insert_with(|| bus_depot_files(vfs, file, false).iter().filter_map(|f| DEPOTS.get(f, |f| hub_omsi::hof::Hof::load(f).ok().map(|h| Arc::new(depot_of(&h))))).collect());
            for d in depots.iter() {
                let fit = knows(d, trips) as f32 / trips.len().max(1) as f32;
                if fit > c.fit {
                    (c.fit, c.depot) = (fit, d.name.clone());
                }
            }
        }
        c
    };
    // the map's own buses first: when one of them fits, the others need not be read
    let (own, rest): (Vec<&Bus>, Vec<&Bus>) = buses.iter().partition(|(f, _)| fleet.get(&key(f)).is_some_and(|w| *w > 0.0));
    let mut c: Vec<Candidate> = own.into_iter().map(&mut weigh).collect();
    if !free && !c.iter().any(|x| x.fit > 0.0) {
        c.extend(rest.into_iter().map(&mut weigh));
    }
    rank(&c, free).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bus_offered_is_the_maps_own_that_knows_most() {
        let cand = |file: &str, fit: f32, wagons: f32| Candidate { file: file.into(), name: file.into(), fit, depot: "D".into(), wagons };
        // of those that fit, the map's own before the others
        let c = [cand("a", 1.0, 0.0), cand("b", 0.5, 3.0), cand("c", 0.2, 9.0)];
        assert_eq!(rank(&c, false).unwrap()["file"], "b");
        // nothing fits: the map's own most common, said plainly
        let c = [cand("a", 0.0, 1.0), cand("b", 0.0, 4.0)];
        let p = rank(&c, false).unwrap();
        assert_eq!((p["file"].as_str(), p["fit"].is_null()), (Some("b"), true));
        // a free drive: the one the map's depot runs most
        assert_eq!(rank(&c, true).unwrap()["free"], true);
        assert_eq!(fold("Müller-Straße 3"), "mullerstrasse3");
    }

    #[test]
    fn the_test_map_s_own_bus_is_offered() {
        let v = crate::content::tests::fixture_vfs();
        let buses = vec![("Vehicles/Testbus/Testbus.bus".to_string(), "Testwerk TW 12".to_string()), ("Vehicles/Other/x.bus".to_string(), "Other".to_string())];
        let b = best_of(&v, "maps/Testdorf/global.cfg", "1989-05-30", true, &[], &buses);
        assert_eq!(b["file"], "Vehicles/Testbus/Testbus.bus", "{b}");
        assert!(best_of(&v, "maps/Gone/global.cfg", "1989-05-30", true, &[], &buses).is_null());
    }
}
