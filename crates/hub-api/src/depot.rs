//! The depot editor (the old launcher's depot editor): the player's own depot files (`.hof`),
//! on `hub_core::owndepot` - kept in the data folder's `depots`, never in the OMSI 2 folder. One
//! file is open at a time and its changes are kept here until it is saved; the page shows what
//! [`view`] answers.
//!
//! * `de_show {key, maps}` - the page opened: the files, and the one asked for (`key`) open, or
//!   the one open before - `{ entries, view, changed_elsewhere, error }`. `maps` is `[[folder,
//!   hof]]`, each map's folder with its own depot file (the problems name a file of its name);
//! * `de_open {key}` - another file open: its view;
//! * `de_edit {op, i, k, to, value, on}` - a change of the file open: `{ view, sel }` (`sel`:
//!   the row the change made, to choose);
//! * `de_save` - the file open written, its copies beside buses with it: `{ view, entries,
//!   copies, name }`;
//! * `de_delete` - the file open put aside (`deleted` in the depots folder): `{ entries, name }`;
//! * `de_sources {hof, map, bus, bus_name}` - what a new file can start from: empty, a copy of
//!   the map's depot file, of one of the bus's, or of one of the player's - `[{ source: { kind,
//!   path }, text, args }]`;
//! * `de_make {name, map, source}` - a new file made from one of them and opened: `{ entries,
//!   view }`.
//!
//! An error that names something is `"text|name"`: the page translates the text and puts the
//! name in.

use crate::content::{bus_depot_files, str_arg, vfs};
use crate::{arg, to_json, Answer, Platform};
use hub_core::linehof::{Coding, Role};
use hub_core::owndepot::{self, Doc, Place};
use hub_omsi::hof::InfoTrip;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// The file open in the editor.
struct Open {
    key: String,
    path: PathBuf,
    doc: Doc,
    coding: Coding,
    /// The line editor's blocks at its end (shown, never changed here).
    blocks: Vec<(String, String)>,
    dirty: bool,
    /// When the file was last written, as it was read or saved here.
    stamp: Option<SystemTime>,
}

/// The editor's state: the file open, and each map's own depot file (`[[folder, hof]]`).
struct State {
    open: Option<Open>,
    maps: Vec<(String, String)>,
}

static STATE: Mutex<State> = Mutex::new(State { open: None, maps: Vec::new() });

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Where the player's depot files lie, and openOMSI's content folder (their copies beside buses).
fn places() -> (PathBuf, Option<PathBuf>) {
    (owndepot::dir(), hub_core::paths::content_dir())
}

/// A `de_` command.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    let (dir, content) = places();
    let mut s = state();
    match cmd {
        "de_show" => {
            if let Some(maps) = arg::<Option<Vec<(String, String)>>>(args, "maps")? {
                s.maps = maps;
            }
            show(&mut s, &dir, &str_arg(args, "key"))
        }
        "de_open" => {
            let key = str_arg(args, "key");
            let same = s.open.as_ref().is_some_and(|o| o.key.eq_ignore_ascii_case(key.trim()));
            // (the file open clicked again: its changes stay, not read over from the disk)
            if same && s.open.as_ref().is_some_and(|o| o.dirty) {
                return Ok(view_of(&s));
            }
            if !same {
                unsaved(&s)?;
            }
            s.open = Some(open(&dir, &key)?);
            Ok(view_of(&s))
        }
        "de_edit" => {
            let o = s.open.as_mut().ok_or("No depot file is open")?;
            let sel = edit(&mut o.doc, &str_arg(args, "op"), args)?;
            o.dirty = true;
            Ok(json!({ "view": view_of(&s), "sel": sel }))
        }
        "de_save" => save(&mut s, &dir, content.as_deref()),
        "de_delete" => {
            let o = s.open.take().ok_or("No depot file is open")?;
            if let Err(e) = owndepot::delete(&o.path, content.as_deref()) {
                s.open = Some(o);
                return Err(e);
            }
            Ok(json!({ "entries": entries(&dir), "name": name_of(&o.doc, &o.key) }))
        }
        "de_sources" => to_json(sources(&dir, &str_arg(args, "hof"), &str_arg(args, "map"), &str_arg(args, "bus"), &str_arg(args, "bus_name"))),
        "de_make" => {
            unsaved(&s)?;
            let made = make(&dir, &str_arg(args, "name"), &str_arg(args, "map"), args.get("source").unwrap_or(&Value::Null))?;
            s.open = Some(made);
            Ok(json!({ "entries": entries(&dir), "view": view_of(&s) }))
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

// --- the files -------------------------------------------------------------------------------

/// The player's depot files as the list shows them.
fn entries(dir: &Path) -> Value {
    owndepot::list(dir).iter().map(|e| json!({ "key": e.key, "name": e.name, "map": e.map, "termini": e.termini.len() })).collect()
}

fn stamp(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn name_of(doc: &Doc, key: &str) -> String {
    let n = doc.hof.name.trim();
    if n.is_empty() {
        key.to_string()
    } else {
        n.to_string()
    }
}

/// The player's depot file `key` read.
fn open(dir: &Path, key: &str) -> Result<Open, String> {
    let path = owndepot::path_of(dir, key).ok_or_else(|| format!("The depot file %{{name}} is not there any more|{}", key.trim()))?;
    let l = owndepot::load(&path)?;
    let key = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    Ok(Open { key, stamp: stamp(&path), path, doc: l.doc, coding: l.coding, blocks: l.blocks, dirty: false })
}

/// The refusal to open another file while the one open has changes that are not saved.
fn unsaved(s: &State) -> Result<(), String> {
    match &s.open {
        Some(o) if o.dirty => Err(format!("%{{name}} has changes that are not saved: save it first|{}", name_of(&o.doc, &o.key))),
        _ => Ok(()),
    }
}

/// `de_show`: the file asked for open (one open with changes stays as it is, said when it was
/// written elsewhere since; another asked for then waits until it is saved), else the one open
/// before while it is there.
fn show(s: &mut State, dir: &Path, key: &str) -> Answer {
    let mut error = None;
    let mut changed = None;
    let same = s.open.as_ref().is_some_and(|o| !key.trim().is_empty() && o.key.eq_ignore_ascii_case(key.trim()));
    // (a file gone elsewhere closes - but not with changes not saved: saving writes it again)
    if same {
        let o = s.open.as_ref().expect("open");
        if !o.path.is_file() && !o.dirty {
            s.open = None;
        } else if o.dirty {
            if stamp(&o.path) != o.stamp {
                changed = Some(name_of(&o.doc, &o.key));
            }
        } else {
            s.open = open(dir, key).ok();
        }
    } else if !key.trim().is_empty() {
        match unsaved(s).and_then(|()| open(dir, key)) {
            Ok(o) => s.open = Some(o),
            Err(e) => error = Some(e),
        }
    } else if let Some(o) = &s.open {
        if !o.path.is_file() && !o.dirty {
            s.open = None;
        } else if !o.dirty {
            s.open = open(dir, &o.key.clone()).ok();
        }
    }
    Ok(json!({ "entries": entries(dir), "view": view_of(s), "changed_elsewhere": changed, "error": error }))
}

/// `de_save`: the file open written (the line editor's blocks it has now kept after it), and
/// its copies beside buses in the content folder made the same again.
fn save(s: &mut State, dir: &Path, content: Option<&Path>) -> Answer {
    let o = s.open.as_mut().ok_or("No depot file is open")?;
    o.doc.hof.name = o.doc.hof.name.trim().to_string();
    o.doc.map = o.doc.map.trim().to_string();
    o.doc.tidy();
    let bytes = owndepot::save(&o.path, &o.doc, o.coding)?;
    let file = o.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let copies = content.map_or(0, |c| owndepot::refresh(c, &file, &bytes));
    o.blocks = owndepot::read(&bytes).blocks;
    o.dirty = false;
    o.stamp = stamp(&o.path);
    let name = name_of(&o.doc, &o.key);
    Ok(json!({ "view": view_of(s), "entries": entries(dir), "copies": copies, "name": name }))
}

/// What a new file can start from.
fn sources(dir: &Path, hof: &str, map: &str, bus: &str, bus_name: &str) -> Vec<Value> {
    let item = |kind: &str, path: Option<&Path>, text: &str, args: Value| json!({ "source": { "kind": kind, "path": path.map(|p| p.to_string_lossy()) }, "text": text, "args": args });
    let file_name = |p: &Path| p.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let mut out = vec![item("empty", None, "Empty, laid out as OMSI's own", json!({}))];
    let mut map_path = None;
    let wanted = hof_stem(hof);
    if !wanted.is_empty() {
        let found = hub_core::depot::find_sources(&wanted, map);
        if let Some(src) = hub_core::depot::best(&found).and_then(|i| found.get(i)) {
            out.push(item("map", Some(&src.path), "A copy of the map's: %{hof}", json!({ "hof": src.file })));
            map_path = Some(src.path.clone());
        }
    }
    if let (Some(v), false) = (vfs(), bus.trim().is_empty()) {
        let shown = if bus_name.trim().is_empty() { bus.trim() } else { bus_name.trim() };
        for p in bus_depot_files(&v, bus, false) {
            if map_path.as_ref().is_some_and(|m| m == &p) {
                continue;
            }
            out.push(item("bus", Some(&p), "A copy of %{bus}'s: %{file}", json!({ "bus": shown, "file": file_name(&p) })));
        }
    }
    for e in owndepot::list(dir) {
        out.push(item("own", Some(&e.path), "A copy of mine: %{name}", json!({ "name": e.name })));
    }
    out
}

/// `de_make`: a new file called `name` for the map `map` (its `global.cfg`), from `source`
/// (`{ kind, path }` of [`sources`]), opened.
fn make(dir: &Path, name: &str, map: &str, source: &Value) -> Result<Open, String> {
    if name.trim().is_empty() {
        return Err("The depot file has no name".to_string());
    }
    let kind = source.get("kind").and_then(Value::as_str).unwrap_or("empty");
    let base = match kind {
        "empty" => None,
        _ => {
            let p = PathBuf::from(source.get("path").and_then(Value::as_str).unwrap_or_default());
            if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("hof")) {
                return Err(format!("no depot file: {}", p.display()));
            }
            Some(std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?)
        }
    };
    let path = owndepot::create_from(dir, name, &hub_core::lines::map_folder(map), base.as_deref())?;
    open(dir, &path.file_stem().unwrap_or_default().to_string_lossy())
}

// --- the view --------------------------------------------------------------------------------

fn role_name(r: Role) -> &'static str {
    match r {
        Role::IbisDisplay => "ibis",
        Role::FrontTop => "front_top",
        Role::FrontBottom => "front_bottom",
        Role::Side => "side",
        Role::RollerBlind => "roller_blind",
        Role::ClearName => "clear_name",
        Role::ExtraSign => "extra_sign",
        Role::Bitmap => "bitmap",
        Role::Special => "special",
        Role::Rear => "rear",
        Role::Fallback(_) => "fallback",
        Role::Other => "other",
    }
}

/// A depot file's name as the game asks for it: without `.hof` (`Grundorf.hof` → `Grundorf`).
fn hof_stem(hof: &str) -> String {
    let hof = hof.trim();
    let n = hof.len();
    if n > 4 && hof.is_char_boundary(n - 4) && hof[n - 4..].eq_ignore_ascii_case(".hof") {
        hof[..n - 4].trim().to_string()
    } else {
        hof.to_string()
    }
}

/// The name of the own depot file of the map whose folder is `folder`, "" when it is not known.
fn maps_depot(maps: &[(String, String)], folder: &str) -> String {
    maps.iter().find(|(f, _)| f.trim().eq_ignore_ascii_case(folder.trim())).map(|(_, h)| hof_stem(h)).unwrap_or_default()
}

fn view_of(s: &State) -> Value {
    s.open.as_ref().map_or(Value::Null, |o| view(o, &s.maps))
}

/// The file open as the page shows it.
fn view(o: &Open, maps: &[(String, String)]) -> Value {
    let d = &o.doc;
    let h = &d.hof;
    let termini: Vec<Value> = h
        .termini
        .iter()
        .enumerate()
        .map(|(i, t)| json!({ "code": t.code, "name": t.texture_id, "all_exit": t.all_exit, "strings": t.strings, "special": d.is_special(i), "marked": d.specials.contains(&t.code) }))
        .collect();
    let stops: Vec<Value> = h.bus_stops.iter().map(|b| json!({ "ident": b.ident, "strings": b.strings })).collect();
    let trips: Vec<Value> = h
        .info_trips
        .iter()
        .enumerate()
        .map(|(i, t)| json!({ "code": t.code, "line": t.line, "name": t.name, "route": t.route, "stops": h.info_busstop_lists.get(i).cloned().unwrap_or_default() }))
        .collect();
    let labels: Vec<Value> = d
        .labels()
        .into_iter()
        .enumerate()
        .map(|(k, (role, note))| json!({ "role": role_name(role), "note": note, "limit": owndepot::limit(role, d.notes.get(k).map(String::as_str).unwrap_or("")) }))
        .collect();
    let blocks: Vec<Value> = o
        .blocks
        .iter()
        .map(|(map, body)| {
            let e = owndepot::block_entries(d, body);
            json!({ "map": map, "d": e.termini.len(), "s": e.bus_stops.len(), "routes": e.info_trips.iter().map(|t| t.code.trim().to_string()).collect::<Vec<_>>() })
        })
        .collect();
    let issues: Vec<Value> = d
        .issues(&maps_depot(maps, &d.map), o.coding)
        .into_iter()
        .map(|i| {
            let at = match i.at {
                Place::File => json!(["file", 0]),
                Place::Terminus(k) => json!(["terminus", k]),
                Place::Stop(k) => json!(["stop", k]),
                Place::Trip(k) => json!(["trip", k]),
            };
            json!({ "at": at, "serious": i.serious, "text": i.text, "args": i.args })
        })
        .collect();
    json!({
        "key": o.key,
        "file": o.path.file_name().unwrap_or_default().to_string_lossy(),
        "name": h.name,
        "map": d.map,
        "dirty": o.dirty,
        "termini": termini,
        "stops": stops,
        "trips": trips,
        "service_trip": h.service_trip,
        "notes": d.notes,
        "stop_notes": d.stop_notes,
        "global_strings": h.global_strings,
        "string_count": h.string_count_terminus,
        "labels": labels,
        "blocks": blocks,
        "issues": issues,
    })
}

// --- the changes -----------------------------------------------------------------------------

/// A place argument.
fn index(args: &Value, name: &str) -> Result<usize, String> {
    args.get(name).and_then(Value::as_u64).map(|n| n as usize).ok_or_else(|| format!("{name}: no place"))
}

fn text(args: &Value) -> String {
    str_arg(args, "value")
}

fn flag(args: &Value) -> bool {
    args.get("on").and_then(Value::as_bool).unwrap_or(false)
}

/// The slot `k` of `v`, made when it is not there yet.
fn slot(v: &mut Vec<String>, k: usize) -> &mut String {
    if v.len() <= k {
        v.resize(k + 1, String::new());
    }
    &mut v[k]
}

/// A change `op` of `doc`. Returns the row it made or chose, if any. Texts are kept as typed
/// (the page's fields hold them); `tidy` trims names when the file is saved.
pub fn edit(doc: &mut Doc, op: &str, args: &Value) -> Result<Option<usize>, String> {
    let h = &mut doc.hof;
    let nt = h.termini.len();
    let ns = h.bus_stops.len();
    let nr = h.info_trips.len();
    let at = |n: usize, k: usize| if k < n { Ok(k) } else { Err(format!("no row {k}")) };
    match op {
        "name" => h.name = text(args),
        "map" => doc.map = text(args).trim().to_string(),
        "service" => h.service_trip = text(args),
        // the destinations
        "t_add" => return Ok(Some(doc.add_destination(&text(args), false, false))),
        "usual" => return Ok(Some(doc.add_destination(&text(args), flag(args), true))),
        "t_name" => {
            let t = &mut h.termini[at(nt, index(args, "i")?)?];
            t.texture_id = text(args);
            t.terminus_stop = (!t.all_exit).then(|| t.texture_id.clone());
        }
        "t_code" => {
            let i = at(nt, index(args, "i")?)?;
            // (what is no number yet stays as it was: the page keeps what is typed)
            if let Ok(code) = text(args).trim().parse::<i32>() {
                let old = h.termini[i].code;
                h.termini[i].code = code;
                if doc.specials.contains(&old) {
                    doc.specials.retain(|c| *c != old);
                    doc.specials.push(code);
                }
            }
        }
        "t_all_exit" => {
            let t = &mut h.termini[at(nt, index(args, "i")?)?];
            t.all_exit = flag(args);
            t.terminus_stop = (!t.all_exit).then(|| t.texture_id.clone());
        }
        "t_special" => doc.set_special(at(nt, index(args, "i")?)?, flag(args)),
        "t_string" => {
            let i = at(nt, index(args, "i")?)?;
            *slot(&mut h.termini[i].strings, index(args, "k")?) = text(args);
        }
        "t_dup" => {
            let i = at(nt, index(args, "i")?)?;
            let mut t = h.termini[i].clone();
            let marked = doc.specials.contains(&t.code);
            t.code = doc.free_code(t.code.saturating_add(1));
            t.texture_id = format!("{} 2", t.texture_id.trim());
            t.terminus_stop = (!t.all_exit).then(|| t.texture_id.clone());
            if marked {
                doc.specials.push(t.code);
            }
            doc.hof.termini.insert(i + 1, t);
            return Ok(Some(i + 1));
        }
        "t_del" => {
            let i = at(nt, index(args, "i")?)?;
            let code = h.termini.remove(i).code;
            if !h.termini.iter().any(|t| t.code == code) {
                doc.specials.retain(|c| *c != code);
            }
            return Ok(i.checked_sub(usize::from(i >= nt - 1)));
        }
        // the IBIS's stops
        "s_add" => return Ok(Some(doc.add_stop(&text(args)))),
        "s_ident" => {
            let i = at(ns, index(args, "i")?)?;
            let new = text(args);
            let old = std::mem::replace(&mut h.bus_stops[i].ident, new.clone());
            // (the routes that call at it follow it)
            let old = old.trim().to_lowercase();
            if !old.is_empty() && !h.bus_stops.iter().enumerate().any(|(k, b)| k != i && b.ident.trim().to_lowercase() == old) {
                for s in h.info_busstop_lists.iter_mut().flatten().filter(|s| s.trim().to_lowercase() == old) {
                    *s = new.trim().to_string();
                }
            }
        }
        "s_string" => {
            let i = at(ns, index(args, "i")?)?;
            *slot(&mut h.bus_stops[i].strings, index(args, "k")?) = text(args);
        }
        "s_del" => {
            let i = at(ns, index(args, "i")?)?;
            h.bus_stops.remove(i);
            return Ok(i.checked_sub(usize::from(i >= ns - 1)));
        }
        // the routes
        "r_add" => {
            let code = doc.free_route(0);
            let route = doc.hof.termini.first().map(|t| t.code.to_string()).unwrap_or_default();
            doc.hof.info_trips.push(InfoTrip { code: code.to_string(), name: text(args), route, ..Default::default() });
            doc.tidy();
            return Ok(Some(doc.hof.info_trips.len() - 1));
        }
        "r_code" => h.info_trips[at(nr, index(args, "i")?)?].code = text(args),
        "r_line" => h.info_trips[at(nr, index(args, "i")?)?].line = text(args),
        "r_name" => h.info_trips[at(nr, index(args, "i")?)?].name = text(args),
        "r_route" => h.info_trips[at(nr, index(args, "i")?)?].route = text(args).trim().to_string(),
        "r_del" => {
            let i = at(nr, index(args, "i")?)?;
            h.info_trips.remove(i);
            if i < h.info_busstop_lists.len() {
                h.info_busstop_lists.remove(i);
            }
            return Ok(i.checked_sub(usize::from(i >= nr - 1)));
        }
        "r_stop_add" | "r_stop_del" | "r_stop_move" => {
            let i = at(nr, index(args, "i")?)?;
            doc.tidy();
            let list = &mut doc.hof.info_busstop_lists[i];
            match op {
                "r_stop_add" => list.push(text(args).trim().to_string()),
                "r_stop_del" => {
                    let k = at(list.len(), index(args, "k")?)?;
                    list.remove(k);
                }
                _ => {
                    let k = at(list.len(), index(args, "k")?)?;
                    let to = index(args, "to")?.min(list.len() - 1);
                    let s = list.remove(k);
                    list.insert(to, s);
                }
            }
            return Ok(Some(i));
        }
        // the file
        "count" => doc.set_string_count(index(args, "k")?),
        "note" => {
            let k = index(args, "k")?;
            if k < h.string_count_terminus {
                *slot(&mut doc.notes, k) = text(args);
            }
        }
        "global" => *slot(&mut h.global_strings, index(args, "k")?) = text(args),
        "global_add" => h.global_strings.push(String::new()),
        _ => return Err(format!("unknown change of the depot file: {op}")),
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Doc {
        let mut d = Doc::blank("Testdorf mine", "Testdorf");
        d.add_destination("Kirche", false, false);
        d.add_destination("Bahnhof", false, false);
        d.add_stop("Kirche");
        d.add_stop("Markt");
        d
    }

    fn ed(d: &mut Doc, op: &str, args: Value) -> Option<usize> {
        edit(d, op, &args).unwrap_or_else(|e| panic!("{op}: {e}"))
    }

    #[test]
    fn destinations_are_added_named_coded_marked_and_deleted() {
        let mut d = doc();
        assert_eq!(ed(&mut d, "t_add", json!({ "value": "Zoo" })), Some(2));
        ed(&mut d, "t_name", json!({ "i": 2, "value": "Zoo " }));
        assert_eq!(d.hof.termini[2].texture_id, "Zoo ", "kept as typed");
        ed(&mut d, "t_code", json!({ "i": 2, "value": "40" }));
        ed(&mut d, "t_code", json!({ "i": 2, "value": "4x" }));
        assert_eq!(d.hof.termini[2].code, 40, "what is no number leaves the code as it was");
        ed(&mut d, "t_special", json!({ "i": 2, "on": true }));
        ed(&mut d, "t_code", json!({ "i": 2, "value": "41" }));
        assert_eq!(d.specials, [41], "the mark follows the code");
        ed(&mut d, "t_string", json!({ "i": 2, "k": 1, "value": "ZOO" }));
        assert_eq!(d.hof.termini[2].strings[1], "ZOO");

        assert_eq!(ed(&mut d, "t_dup", json!({ "i": 2 })), Some(3));
        let copy = &d.hof.termini[3];
        assert_eq!((copy.texture_id.as_str(), copy.strings[1].as_str()), ("Zoo 2", "ZOO"));
        assert_ne!(copy.code, 41);
        assert!(d.specials.contains(&copy.code), "a marked one's copy is marked too");

        assert_eq!(ed(&mut d, "t_del", json!({ "i": 3 })), Some(2), "the last gone: the one before chosen");
        assert_eq!(ed(&mut d, "t_del", json!({ "i": 0 })), Some(0));
        assert_eq!(d.hof.termini.len(), 2);
        assert!(edit(&mut d, "t_name", &json!({ "i": 9, "value": "x" })).is_err());
    }

    #[test]
    fn the_usual_service_trip_is_one_where_nobody_boards() {
        let mut d = doc();
        let i = ed(&mut d, "usual", json!({ "value": "Betriebsfahrt", "on": true })).unwrap();
        assert!(d.hof.termini[i].all_exit && d.is_special(i));
        ed(&mut d, "service", json!({ "value": "Betriebsfahrt" }));
        assert_eq!(d.hof.service_trip, "Betriebsfahrt");
        assert!(d.issues("", Coding::Utf16).iter().all(|x| !x.text.starts_with("The service trip names")));
    }

    #[test]
    fn routes_get_their_stops_and_follow_a_renamed_stop() {
        let mut d = doc();
        let r = ed(&mut d, "r_add", json!({ "value": "Kirche - Markt" })).unwrap();
        assert_eq!(d.hof.info_trips[r].route, d.hof.termini[0].code.to_string(), "to the first destination");
        for s in ["Kirche", "Markt"] {
            ed(&mut d, "r_stop_add", json!({ "i": r, "value": s }));
        }
        ed(&mut d, "r_stop_move", json!({ "i": r, "k": 1, "to": 0 }));
        assert_eq!(d.hof.info_busstop_lists[r], ["Markt", "Kirche"]);
        ed(&mut d, "s_ident", json!({ "i": 1, "value": "Marktplatz" }));
        assert_eq!(d.hof.info_busstop_lists[r], ["Marktplatz", "Kirche"]);
        ed(&mut d, "r_stop_del", json!({ "i": r, "k": 0 }));
        assert_eq!(d.hof.info_busstop_lists[r], ["Kirche"]);
        ed(&mut d, "r_code", json!({ "i": r, "value": "abc" }));
        assert!(d.issues("", Coding::Utf16).iter().any(|x| x.serious && x.text == "Route %{code} is no number: the IBIS cannot type it"));
        assert_eq!(ed(&mut d, "r_del", json!({ "i": r })), None, "none left");
        assert!(d.hof.info_trips.is_empty() && d.hof.info_busstop_lists.is_empty());
    }

    #[test]
    fn the_file_s_texts_are_counted_noted_and_kept() {
        let mut d = doc();
        ed(&mut d, "count", json!({ "k": 9 }));
        assert!(d.hof.termini.iter().all(|t| t.strings.len() == 9) && d.notes.len() == 9);
        ed(&mut d, "note", json!({ "k": 8, "value": "Rear sign" }));
        assert_eq!(d.notes[8], "Rear sign");
        ed(&mut d, "global_add", json!({}));
        ed(&mut d, "global", json!({ "k": 0, "value": "Testdorf" }));
        assert_eq!(d.hof.global_strings, ["Testdorf"]);
        ed(&mut d, "name", json!({ "value": "Mine" }));
        ed(&mut d, "map", json!({ "value": " Grundorf " }));
        assert_eq!((d.hof.name.as_str(), d.map.as_str()), ("Mine", "Grundorf"));
        assert!(edit(&mut d, "nope", &json!({})).unwrap_err().contains("nope"));
    }

    const SPECIALS_OF_OMSI: [&str; 6] = ["Betriebsfahrt", "Leerfahrt", "Dienstfahrt", "Nicht einsteigen", "Sonderfahrt", "Schulbus"];

    /// Every text of the depot editor, the bus's depot files, the Timetable page and the editor's
    /// tiles - the pages' and the depot file's problems - in the seven languages.
    #[test]
    fn every_depot_and_timetable_editor_text_is_translated() {
        use crate::online::{texts_in, untranslated};
        let pages = [
            include_str!("../../../ui/js/editors/depots.js"),
            include_str!("../../../ui/js/editors/hof.js"),
            include_str!("../../../ui/js/editors/timetable.js"),
            include_str!("../../../ui/js/editors/hub.js"),
            include_str!("../../../ui/js/editors/index.js"),
        ];
        let mut texts: Vec<String> = Vec::new();
        for js in pages {
            for lead in ["tr('", "tr(\"", "label=\"", "title=\"", "placeholder=\"", "status('", "status(\""] {
                let end = if lead.ends_with('\'') { '\'' } else { '"' };
                texts.extend(texts_in(js, lead, end));
            }
            // (the words chosen in a template: `label=${on ? 'Press again' : 'Delete'}`)
            for lead in ["? '", ": '", "['"] {
                texts.extend(texts_in(js, lead, '\'').into_iter().filter(|t| t.starts_with(|c: char| c.is_uppercase()) && !t.contains("${")));
            }
        }
        // (what resetting a map's timetable says, `ttstore::RESET_*`)
        texts.extend([hub_core::ttstore::RESET_COPY, hub_core::ttstore::RESET_BACK, hub_core::ttstore::RESET_NONE].map(String::from));
        texts.extend(["one tour", "%{n} tours", "Empty, laid out as OMSI's own", "A copy of the map's: %{hof}", "A copy of %{bus}'s: %{file}", "A copy of mine: %{name}"].map(String::from));
        let own = include_str!("depot.rs").replace("\r\n", "\n");
        let own = own.split("#[cfg(test)]").next().unwrap_or("").to_string();
        let said = texts_in(&own, "Err(\"", '"').into_iter().chain(texts_in(&own, "format!(\"", '"'));
        texts.extend(said.filter(|t| t.contains("%{{") || !t.contains('{')).map(|t| t.split('|').next().unwrap_or("").replace("{{", "{").replace("}}", "}")));
        // (what the Timetable page's changes say when they cannot be made)
        let tt = include_str!("timetable.rs").replace("\r\n", "\n");
        let tt = tt.split("#[cfg(test)]").next().unwrap_or("").to_string();
        texts.extend(["ok_or(\"", "Err(\""].into_iter().flat_map(|lead| texts_in(&tt, lead, '"')).filter(|t| !t.contains('{')));
        texts.retain(|t| !t.contains("${") && !t.starts_with("unknown") && !t.starts_with("no ") && !t.starts_with("a line's name"));
        // (OMSI's own names of the usual special trips, and the editors' names in the page list)
        texts.retain(|t| !SPECIALS_OF_OMSI.contains(&t.as_str()) && t != "Livery");
        texts.sort();
        texts.dedup();
        let missing = untranslated(&texts);
        assert!(missing.is_empty(), "{} missing:\n{}", missing.len(), missing.join("\n"));
    }

    #[test]
    fn the_view_has_what_the_page_reads() {
        let d = doc();
        let o = Open { key: "oo_mine".into(), path: PathBuf::from("/x/oo_mine.hof"), doc: d, coding: Coding::Utf16, blocks: Vec::new(), dirty: true, stamp: None };
        let v = view(&o, &[("Testdorf".into(), "Testdorf mine.hof".into())]);
        assert_eq!((v["key"].as_str(), v["file"].as_str(), v["dirty"].as_bool()), (Some("oo_mine"), Some("oo_mine.hof"), Some(true)));
        assert_eq!(v["termini"][0]["name"], "Kirche");
        assert_eq!(v["labels"][0]["role"], "ibis");
        assert_eq!(v["labels"][0]["limit"], 16);
        assert_eq!(v["string_count"], 8);
        // (it has the map's own depot file's name: said, and where)
        let named = v["issues"].as_array().unwrap().iter().find(|i| i["text"].as_str().unwrap().starts_with("It has the name of the map's own")).expect("named like the map's");
        assert_eq!(named["at"], json!(["file", 0]));
        assert_eq!(maps_depot(&[("testdorf".into(), "Testdorf.HOF".into())], "Testdorf"), "Testdorf");
    }
}
