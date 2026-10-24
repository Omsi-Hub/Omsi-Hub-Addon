//! The chosen map as the game's own city map has it, for the pages that draw it: the route map
//! of the drive's steps (`hub_route_map`), the company's duty map (`co_duty_map`) and its fleet
//! map (`co_more_map`) - the old app's `omsi_launcher_model::mapview` and `hub_fleet`, which its
//! game program answered (the launcher service's "map"). Every spline's and object's `[path]`,
//! linked to its neighbours and grouped into carriageways with their real widths
//! (`lineeditor::navmap`, read as the game reads it), where the entry points stand and the
//! objects are - and the road pieces the chosen trip drives over it (`trip_path`, from the
//! timetable's track or its station links).

use glam::{DVec2, DVec3, Vec3};
use hashbrown::HashMap;
use omsi_sim::traffic::Network;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The game's city map colours (casing, road, main road, route: `omsi-app`'s navigator).
const CITY_MAP_COLOURS: [[f32; 4]; 4] = [
    [30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 0.9],
    [92.0 / 255.0, 92.0 / 255.0, 92.0 / 255.0, 1.0],
    [112.0 / 255.0, 112.0 / 255.0, 112.0 / 255.0, 1.0],
    [214.0 / 255.0, 48.0 / 255.0, 40.0 / 255.0, 1.0],
];

/// The route of a trip as the game drives it: its road pieces and its stops.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TripPath {
    /// The road pieces it drives, in order (the shared piece of two links appears once).
    pub route: Vec<RoadPiece>,
    /// The stop objects the trip calls at, in order.
    pub stops: Vec<i64>,
}

/// One spline of the map a trip drives over: its id in its tile, which `[path]` of it the trip
/// uses, and the tile's place in `global.cfg`'s `[map]` list - the game's own `LaneKey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoadPiece {
    pub tile_x: i32,
    pub tile_y: i32,
    pub spline: i64,
    pub path: u16,
}

/// The route of trip `trip` of the map `map` on `date` (the old launcher's `trip_path`): the
/// trip's own track (`.ttr`) when it has one, else the station links between its stops, as the
/// game walks them.
pub fn trip_path(map: &str, date: &str, trip: &str) -> Result<TripPath, String> {
    Ok(Trips::read(map, date)?.path(trip))
}

/// The timetable of a map on a day and its tiles, read once for the routes of many trips (the
/// fleet map's: `fleet_export`).
pub struct Trips {
    data: hub_omsi::timetable::Timetable,
    /// The tiles of `global.cfg`'s `[map]` list, in its order.
    tiles: Vec<(i32, i32)>,
}

impl Trips {
    pub fn read(map: &str, date: &str) -> Result<Trips, String> {
        let vfs = hub_core::paths::vfs().ok_or("no OMSI 2 folder")?;
        let global = vfs.resolve(map).ok_or_else(|| format!("the map {map} is not there"))?;
        let date = if date.trim().is_empty() { "1989-05-30" } else { date.trim() };
        let day = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| format!("'{date}' is not a date (YYYY-MM-DD)"))?;
        // (the TTData openOMSI reads: the content folder's - where the line editor's lines are -
        // before the one beside global.cfg)
        let data = hub_omsi::timetable::timetable_for(&vfs, map, day).map_err(|e| e.to_string())?.unwrap_or_default();
        // (the tile of a step is the index of its entry in `[map]`, as the game reads it)
        let tiles = hub_omsi::map::GlobalCfg::load(&global).map(|g| g.tiles).unwrap_or_default();
        Ok(Trips { data, tiles })
    }

    /// The route of trip `trip` (`trip_path`).
    pub fn path(&self, trip: &str) -> TripPath {
        let data = &self.data;
        let mut out = TripPath::default();
        let Some(t) = data.trip(trip) else { return out };
        out.stops = t.stops.iter().map(|s| s.id).collect();
        let at = |index: usize| self.tiles.get(index).copied();
        if let Some(track) = data.track_of(t) {
            for e in &track.entries {
                if let Some(tile) = at(e.tile) {
                    out.route.push(RoadPiece { tile_x: tile.0, tile_y: tile.1, spline: e.spline, path: e.path.max(0) as u16 });
                }
            }
            return out;
        }
        for w in out.stops.clone().windows(2) {
            let Some(link) = data.stn_links.iter().find(|l| l.from_id == w[0] && l.to_id == w[1]) else { continue };
            for e in &link.entries {
                let Some(tile) = at(e[2].max(0.0) as usize) else { continue };
                let p = RoadPiece { tile_x: tile.0, tile_y: tile.1, spline: e[0] as i64, path: e[1].max(0.0) as u16 };
                // (consecutive links repeat the piece they share)
                if out.route.last() != Some(&p) {
                    out.route.push(p);
                }
            }
        }
        out
    }
}

/// What the map should show. Changing `map` reads the tiles again; changing `trip` only walks
/// the timetable; changing `entry` only moves the ring.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Look {
    /// The map as the launcher names it (`maps/<name>/global.cfg`), and where that is.
    pub map: String,
    pub global: PathBuf,
    /// The date the duty starts on (the chrono folders change the map and the timetable).
    pub date: String,
    /// The trip whose route is drawn (empty: the map alone).
    pub trip: String,
    /// The entry point the player picked, -1 = automatic (nothing is ringed).
    pub entry: i32,
}

/// An entry point as the map draws it: where its marker stands, its name, and its own place
/// in `global.cfg`'s list. That place - not its place among the drawn markers - is what the
/// choice and the game count (`--entry` is the position in that list), so an entry point
/// whose position cannot be resolved leaves a gap that does not shift every marker after it
/// onto the wrong spawn.
struct Entry {
    /// Where the marker stands, in world metres.
    at: DVec2,
    /// Its name, as `global.cfg` writes it.
    name: String,
    /// Its position in `global.entry_points`.
    index: usize,
}

/// A map as the map needs it: the roads the game's own city map draws, the lanes they were
/// grouped from (the chosen trip's route runs on those), where its entry points stand and
/// where its objects do - all in world metres.
#[derive(Default)]
struct Roads {
    /// Every carriageway the map has, as the game groups them: points in world metres.
    roads: Vec<crate::lineeditor::navmap::MapRoad>,
    /// The linked network behind them (a trip's road pieces are lanes of it).
    net: Arc<Network>,
    /// (tile x, tile y, spline id, path) → the lane the timetable names.
    lanes: HashMap<(i32, i32, i64, u16), usize>,
    /// (tile x, tile y, spline id) → every lane of that spline (for a piece whose exact path
    /// is not there any more).
    splines: HashMap<(i32, i32, i64), Vec<usize>>,
    /// The world point the drawn lines are measured from: an f32 cannot hold a map's own
    /// millions of metres to the centimetre, and roads are drawn to the centimetre.
    origin: DVec2,
    /// The world rectangle the roads cover (the 1 - 99 % box: see `read_map`).
    lo: DVec2,
    hi: DVec2,
    /// The entry points that could be placed, in `global.cfg`'s order - each carrying its own
    /// place in that list (see `Entry`).
    entries: Vec<Entry>,
    /// Every placed object's world place: the trip's stops are found here.
    objects: HashMap<i64, DVec2>,
}

impl Roads {
    /// The lanes a trip's road piece runs on: the very `[path]` the timetable names, else -
    /// a map whose timetable still names a spline it no longer has - every lane of that
    /// spline, so a hole in the map's own data is not a hole in the picture.
    fn routes_of(&self, piece: &RoadPiece) -> Vec<usize> {
        if let Some(&i) = self.lanes.get(&(piece.tile_x, piece.tile_y, piece.spline, piece.path)) {
            return vec![i];
        }
        self.splines.get(&(piece.tile_x, piece.tile_y, piece.spline)).cloned().unwrap_or_default()
    }
}

/// A polyline with the points the eye cannot tell apart taken out (the game's own map
/// simplification, in metres: 12 cm there, half a pixel here).
fn simplify(pts: &[Vec3], tol: f32) -> Vec<Vec3> {
    crate::lineeditor::navmap::simplify(pts, tol)
}

/// The map for another front end (the desktop launcher draws it itself): the roads the
/// game's city map draws, thinned to `tolerance` metres, the entry points with their own place
/// in `global.cfg`'s list, the chosen trip's route as the lanes the game drives and where its
/// stops stand - points in metres from `origin`, rounded to the decimetre, as flat
/// `[x0, y0, x1, y1, ...]` lists. `colors` are the game's city map colours (casing, road, main
/// road, route) as CSS.
pub fn export(map: &str, global: &Path, date: &str, trip: &str, tolerance: f32) -> serde_json::Value {
    use serde_json::json;
    let look = Look { map: map.to_string(), global: global.to_path_buf(), date: date.to_string(), trip: trip.to_string(), entry: -1 };
    let roads = roads_of(&look);
    let path = trip_of(&look);
    let o = roads.origin;
    let flat = |pts: &[DVec3]| -> Vec<f32> {
        let v: Vec<Vec3> = pts.iter().map(|q| Vec3::new((q.x - o.x) as f32, (q.y - o.y) as f32, 0.0)).collect();
        simplify(&v, tolerance).iter().flat_map(|q| [(q.x * 10.0).round() / 10.0, (q.y * 10.0).round() / 10.0]).collect()
    };
    let list: Vec<serde_json::Value> = roads.roads.iter().map(|r| json!({ "w": (r.width * 10.0).round() / 10.0, "main": r.main, "p": flat(&r.points) })).collect();
    let mut route = Vec::new();
    for piece in &path.route {
        for i in roads.routes_of(piece) {
            if let Some(l) = roads.net.lanes.get(i) {
                route.push(flat(&l.points));
            }
        }
    }
    let rel = |q: DVec2| [((q.x - o.x) * 10.0).round() / 10.0, ((q.y - o.y) * 10.0).round() / 10.0];
    let stops: Vec<serde_json::Value> = path.stops.iter().enumerate().filter_map(|(k, id)| roads.objects.get(id).map(|q| json!({ "k": k, "at": rel(*q) }))).collect();
    let entries: Vec<serde_json::Value> = roads.entries.iter().map(|e| json!({ "index": e.index, "name": e.name, "at": rel(e.at) })).collect();
    let css = |c: [f32; 4]| format!("rgba({},{},{},{:.3})", (c[0] * 255.0).round(), (c[1] * 255.0).round(), (c[2] * 255.0).round(), c[3]);
    let [casing, road, main, red] = CITY_MAP_COLOURS;
    json!({
        "origin": [o.x, o.y],
        "lo": rel(roads.lo), "hi": rel(roads.hi),
        "roads": list, "route": route, "stops": stops, "entries": entries,
        "colors": [css(casing), css(road), css(main), css(red)],
    })
}

/// The route of the chosen trip (nothing when no trip is chosen or the map has a timetable
/// that does not know it).
fn trip_of(look: &Look) -> TripPath {
    if look.trip.trim().is_empty() {
        return Default::default();
    }
    match trip_path(&look.map, &look.date, &look.trip) {
        Ok(t) => t,
        Err(e) => {
            log::debug!("trip path for {}: {e}", look.trip);
            Default::default()
        }
    }
}

/// The map's roads of `look` (its map and date), read once and kept for the next page that
/// draws the same map (a few are kept: a big map's tiles take a while).
fn roads_of(look: &Look) -> Arc<Roads> {
    static KEPT: std::sync::Mutex<Vec<(PathBuf, String, Arc<Roads>)>> = std::sync::Mutex::new(Vec::new());
    if let Some(r) = KEPT.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(g, d, _)| *g == look.global && *d == look.date).map(|k| k.2.clone()) {
        return r;
    }
    let _ = crate::content::bus3d::roots();
    let r = Arc::new(read_map(look));
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    kept.push((look.global.clone(), look.date.clone(), r.clone()));
    if kept.len() > 3 {
        kept.remove(0);
    }
    r
}

/// The map's own data, read the way the game reads it for its city map: every tile's paths
/// (splines and the crossings' objects), linked, grouped into carriageways with their real
/// widths. `look.global` is the map's `global.cfg`; the tiles stand beside it and the assets
/// the map names under the installation it belongs to.
fn read_map(look: &Look) -> Roads {
    let Ok(global) = omsi_map::GlobalCfg::load(&look.global) else { return Roads::default() };
    // everything that turns a tile index and a place in the tile into world metres needs
    // this first (`[worldcoordinates]` maps use another grid)
    omsi_map::configure_grid(&global);
    let Some(map_dir) = look.global.parent().map(Path::to_path_buf) else { return Roads::default() };
    let chrono = omsi_map::date_code(&look.date).map(|c| omsi_map::active_chrono_dirs(&map_dir, c)).unwrap_or_default();
    let size = omsi_map::tile_size();
    // the installation the map belongs to: what its `.sli` and `.sco` files are named from.
    // A map is a folder under a content root's `maps` (that is where the launcher's list came
    // from), so the root that holds it is the one its assets are named from; a map outside
    // every root falls back to the folder two steps up, as `<root>/maps/<name>`.
    let root = omsi_cfg::content_roots().into_iter().find(|r| map_dir.starts_with(r)).or_else(|| map_dir.parent().and_then(|p| p.parent()).map(Path::to_path_buf)).unwrap_or_else(|| map_dir.clone());
    let tiles: Vec<(i32, i32, PathBuf)> = global.tiles.iter().map(|t| (t.x, t.y, omsi_cfg::resolve_path(&map_dir, &t.file))).filter(|t| omsi_cfg::vfs::is_file(&t.2)).collect();
    let t0 = std::time::Instant::now();
    let crate::lineeditor::navmap::CityMap { roads, net, places, .. } = crate::lineeditor::navmap::city_map(&root, &tiles, &chrono);
    // the lanes the timetable can name, and the lanes of every spline behind them
    let mut by_path = HashMap::new();
    let mut by_spline: HashMap<(i32, i32, i64), Vec<usize>> = HashMap::new();
    for (i, l) in net.lanes.iter().enumerate() {
        let Some(key) = l.key else { continue };
        by_path.insert((key.tile.0, key.tile.1, key.id, key.path), i);
        by_spline.entry((key.tile.0, key.tile.1, key.id)).or_default().push(i);
    }
    // the entry points: on the tile the record names, else at their object (`World::
    // entry_point_place` reads them the same way)
    let mut entries = Vec::new();
    for (index, ep) in global.entry_points.iter().enumerate() {
        let p = usize::try_from(ep.group)
            .ok()
            .and_then(|i| global.raw_tiles.get(i))
            .map(|t| DVec2::new(t.0 as f64 * size + ep.pos[0], t.1 as f64 * size + ep.pos[1]))
            .or_else(|| places.get(&ep.object_id).map(|q| q.truncate()));
        // the marker carries where it stands in `global.cfg`'s list, so one left out here
        // (no tile, no object) does not shift the ones after it
        if let Some(p) = p {
            entries.push(Entry { at: p, name: ep.name.clone(), index });
        }
    }
    let objects: HashMap<i64, DVec2> = places.iter().map(|(k, v)| (*k, v.truncate())).collect();
    // where the roads are, and where the picture is measured from
    let (mut min, mut max) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
    for r in &roads {
        for q in &r.points {
            min = min.min(q.truncate());
            max = max.max(q.truncate());
        }
    }
    if min.x == f64::MAX {
        min = DVec2::ZERO;
        max = DVec2::ZERO;
    }
    let origin = (min + max) * 0.5;
    // Where the roads really are: one spline left in a far corner (an editor's marker, a
    // piece of the next town) would shrink everything else to a dot, so the map opens on the
    // 1 - 99 % box of what the roads cover instead of the extremes (the far ones are still
    // there to be dragged to).
    let mut xs: Vec<f32> = roads.iter().flat_map(|r| r.points.iter()).map(|q| q.x as f32).collect();
    let mut ys: Vec<f32> = roads.iter().flat_map(|r| r.points.iter()).map(|q| q.y as f32).collect();
    xs.sort_by(f32::total_cmp);
    ys.sort_by(f32::total_cmp);
    let (lo, hi) = if xs.is_empty() {
        (min, max)
    } else {
        let at = |v: &Vec<f32>, t: f64| v[((v.len() - 1) as f64 * t) as usize] as f64;
        (DVec2::new(at(&xs, 0.01), at(&ys, 0.01)), DVec2::new(at(&xs, 0.99), at(&ys, 0.99)))
    };
    log::info!(
        "map: {} roads from {} lanes in {} tiles, {} objects, {} entry points, read in {:.2} s; roads cover ({:.0}, {:.0}) - ({:.0}, {:.0}), opened at the 1 - 99 % box ({:.0}, {:.0}) - ({:.0}, {:.0})",
        roads.len(),
        net.lanes.len(),
        tiles.len(),
        objects.len(),
        entries.len(),
        t0.elapsed().as_secs_f64(),
        min.x, min.y, max.x, max.y,
        lo.x, lo.y, hi.x, hi.y
    );
    Roads { roads, net: Arc::new(net), lanes: by_path, splines: by_spline, origin, lo, hi, entries, objects }
}

/// The duty map (`co_duty_map`) and a duty's route map: [`fleet_export`]'s roads, the routes
/// of `trips` drawn as one route each and their stops, as the route map has a trip's.
pub fn trips_page(map: &str, global: &Path, date: &str, trips: &[String]) -> serde_json::Value {
    use serde_json::{json, Value};
    let mut v = fleet_export(map, global, date, trips, 0.6);
    let tracks = v.get("tracks").and_then(Value::as_array).cloned().unwrap_or_default();
    let route: Vec<Value> = tracks.iter().filter_map(|t| t.get("p").filter(|p| p.as_array().is_some_and(|a| !a.is_empty())).cloned()).collect();
    let stops: Vec<Value> = tracks.iter().flat_map(|t| t.get("stops").and_then(Value::as_array).cloned().unwrap_or_default()).filter(|s| s.is_array()).map(|at| json!({ "at": at, "k": -1 })).collect();
    // (each trip by its name: its route's place in `route` and its stops, null where the map
    // lacks one - the live dashboard's navigator follows the trip under way on it)
    let mut at = 0;
    let named: Vec<Value> = tracks
        .iter()
        .map(|t| {
            let has = t.get("p").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
            let k = has.then(|| {
                at += 1;
                at - 1
            });
            json!({ "name": t.get("name"), "route": k, "stops": t.get("stops") })
        })
        .collect();
    v["route"] = json!(route);
    v["stops"] = json!(stops);
    v["trips"] = json!(named);
    v["entries"] = json!([]);
    if let Some(o) = v.as_object_mut() {
        o.remove("tracks");
    }
    v
}

/// The map and the routes of `trips` on `date`, for the desktop launcher's fleet map: `export`'s
/// `roads`, `lo`, `hi`, `origin` and `colors`, and `tracks` - for each trip its `name`, its route
/// `p` (flat `[x0, y0, ...]` in metres from `origin`, to the decimetre, a little thinned) and
/// its `stops` (`[x, y]`, or null where the map lacks the stop). The map is read once.
pub fn fleet_export(map: &str, global: &Path, date: &str, trips: &[String], tolerance: f32) -> serde_json::Value {
    use serde_json::json;
    let look = Look { map: map.to_string(), global: global.to_path_buf(), date: date.to_string(), trip: String::new(), entry: -1 };
    let roads = roads_of(&look);
    let o = roads.origin;
    let r1 = |v: f64| (v * 10.0).round() / 10.0;
    let flat = |pts: &[DVec3], tol: f32| -> Vec<f64> {
        let v: Vec<Vec3> = pts.iter().map(|q| Vec3::new((q.x - o.x) as f32, (q.y - o.y) as f32, 0.0)).collect();
        simplify(&v, tol).iter().flat_map(|q| [r1(q.x as f64), r1(q.y as f64)]).collect()
    };
    // the roads, as `export` has them
    let list: Vec<serde_json::Value> = roads.roads.iter().map(|r| json!({ "w": (r.width * 10.0).round() / 10.0, "main": r.main, "p": flat(&r.points, tolerance) })).collect();
    let rel = |q: DVec2| [r1(q.x - o.x), r1(q.y - o.y)];
    // the trips' routes and stops (the timetable read once for all of them)
    let mut tracks = Vec::new();
    let mut read: Option<Result<Trips, String>> = None;
    for name in trips {
        if name.trim().is_empty() {
            continue;
        }
        let path = match read.get_or_insert_with(|| Trips::read(map, date)) {
            Ok(t) => t.path(name),
            Err(e) => {
                log::debug!("trip path for {name}: {e}");
                tracks.push(json!({ "name": name, "p": [], "stops": [] }));
                continue;
            }
        };
        // (the first lane of each piece, end to end: `trip_tracks`)
        let mut pts: Vec<DVec3> = Vec::new();
        for piece in &path.route {
            let Some(lane) = roads.routes_of(piece).first().and_then(|i| roads.net.lanes.get(*i)) else { continue };
            for q in &lane.points {
                let p = DVec3::new(q.x, q.y, 0.0);
                if pts.last().is_none_or(|x| (x.truncate() - p.truncate()).length() > 0.05) {
                    pts.push(p);
                }
            }
        }
        let stops: Vec<serde_json::Value> = path.stops.iter().map(|id| roads.objects.get(id).map(|q| json!(rel(*q))).unwrap_or(serde_json::Value::Null)).collect();
        // (a hair thinner than the roads: the buses ride on it, measured along it)
        tracks.push(json!({ "name": name, "p": flat(&pts, tolerance * 0.5), "stops": stops }));
    }
    let css = |c: [f32; 4]| format!("rgba({},{},{},{:.3})", (c[0] * 255.0).round(), (c[1] * 255.0).round(), (c[2] * 255.0).round(), c[3]);
    let [casing, road, main, red] = CITY_MAP_COLOURS;
    json!({
        "origin": [o.x, o.y],
        "lo": rel(roads.lo), "hi": rel(roads.hi),
        "roads": list, "tracks": tracks,
        "colors": [css(casing), css(road), css(main), css(red)],
    })
}

/// The route of `trip` as one line through the lanes it drives, in map metres (the first lane
/// of each piece, end to end, as [`fleet_export`] has it): the companion's next turn
/// (`crate::companion`).
pub fn route_line(map: &str, global: &Path, date: &str, trip: &str) -> Vec<[f64; 2]> {
    let look = Look { map: map.to_string(), global: global.to_path_buf(), date: date.to_string(), trip: String::new(), entry: -1 };
    let roads = roads_of(&look);
    let Ok(path) = trip_path(map, date, trip) else { return Vec::new() };
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for piece in &path.route {
        let Some(lane) = roads.routes_of(piece).first().and_then(|i| roads.net.lanes.get(*i)) else { continue };
        for q in &lane.points {
            if pts.last().is_none_or(|x| (x[0] - q.x).hypot(x[1] - q.y) > 0.05) {
                pts.push([q.x, q.y]);
            }
        }
    }
    pts
}

/// The map's roads with a point within `r` metres of `at` (map metres), thinned to half a metre:
/// their width and their line, for the companion's map.
pub fn roads_near(map: &str, global: &Path, date: &str, at: [f64; 2], r: f64) -> Vec<(f64, Vec<[f64; 2]>)> {
    let look = Look { map: map.to_string(), global: global.to_path_buf(), date: date.to_string(), trip: String::new(), entry: -1 };
    let roads = roads_of(&look);
    let o = roads.origin;
    roads
        .roads
        .iter()
        .filter(|road| road.points.iter().any(|q| (q.x - at[0]).abs() <= r && (q.y - at[1]).abs() <= r))
        .take(4000)
        .map(|road| {
            let v: Vec<Vec3> = road.points.iter().map(|q| Vec3::new((q.x - o.x) as f32, (q.y - o.y) as f32, 0.0)).collect();
            (road.width as f64, simplify(&v, 0.5).iter().map(|q| [q.x as f64 + o.x, q.y as f64 + o.y]).collect())
        })
        .collect()
}

/// The map metres of a place given by its tile (the grid's x and y) and the metres in it.
pub fn world_of(map: &str, global: &Path, date: &str, tile: [i64; 2], local: [f64; 2]) -> [f64; 2] {
    // (the map read first: that sets the grid its tiles are measured in)
    let _ = roads_of(&Look { map: map.to_string(), global: global.to_path_buf(), date: date.to_string(), trip: String::new(), entry: -1 });
    let size = omsi_map::tile_size();
    [tile[0] as f64 * size + local[0], tile[1] as f64 * size + local[1]]
}
