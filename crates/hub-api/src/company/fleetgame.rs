//! The company's buses in the game (Luc, 2026-10-26: "the player starts a company, and when he
//! drives himself he sees only his own buses, where the company says they are"). The trips of
//! the map's timetable the company runs with its own buses today, each with how late the day's
//! one delay model has it (`lateness::model_with`, as the fleet map moves them), go to the
//! plugin (`data/fleet.json`, `hub_game::bridge::Fleet`), which gives them to the game
//! (`lua/fleet.lua`, `omsi.timetable.set_ai_trips`): the game's AI runs those trips and no
//! others - no bus of a line the company does not run, none of a tour dropped today - and puts
//! each on the road that late, where the fleet map has it. While the game drives them, the fleet
//! map shows them where the game says (`fleetmap`), so the two stay one.
//!
//! The list is written for a company's duty before the game starts ([`for_drive`]) and again
//! while it runs ([`refresh`], from the bridge's loop) as the day's delays move on; a drive of
//! the player's own has none (`drive::start` takes it away), and the game runs the map's whole
//! timetable as ever. The player's own block is not in it: the player drives it.
//!
//! Nothing is booked here: the list changes nothing of the company.

use super::{with, Session};
use crate::launcher as core;
use hub_core::company::clock as ck;
use hub_core::company::control as ctl;
use hub_core::company::day::Plan;
use hub_core::company::lateness::{self as lt, Model};
use hub_core::company::plan::{self as cp, PLAYER};
use hub_core::company::Company;
use hub_game::bridge::{self, Fleet, FleetTrip};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The list is worked out again at most this often while the game runs.
const EVERY: Duration = Duration::from_secs(10);

/// The day's delays and what they were worked out for (date, the clock's minute, the plan's
/// generation, the company's).
type Kept = Option<((String, u32, u64, u64), Model)>;

/// The timetable's trips running today by their name and the minute they leave: the tours that
/// run them (line, tour) and the second they leave.
type Runs<'a> = HashMap<(String, i32), Vec<(&'a str, &'a str, f64)>>;

#[derive(Default)]
struct State {
    last: Option<Instant>,
    model: Kept,
}

fn state() -> &'static Mutex<State> {
    static S: Mutex<State> = Mutex::new(State { last: None, model: None });
    &S
}

fn real_now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// The trips of the map's timetable (`lines`, read for the plan's day) that the buses of `plan`
/// run: each block with a bus but the player's (and `skip`, the block of the duty driven: its
/// timetable line and tour), each of its trips the timetable has, by the tour the game knows it
/// by and the time it leaves, `model`'s minutes late as it leaves (on time where the model has
/// none). A depot run the company added itself is not the timetable's: the game has no bus for it.
/// `keep`: the drive takes `skip`'s bus over halfway along its trip (`bridge::TakeOver`) - the
/// game's AI drives it until the plugin took it, the player's own block too.
pub fn trips_of(plan: &Plan, lines: &[core::LineInfo], model: &Model, skip: Option<(&str, &str)>, keep: bool) -> Vec<FleetTrip> {
    let mut runs: Runs = HashMap::new();
    for l in lines {
        for t in l.tours.iter().filter(|t| t.runs) {
            for x in &t.trips {
                runs.entry((x.name.trim().to_lowercase(), (x.departure / 60.0).round() as i32)).or_default().push((&l.name, &t.number, x.departure));
            }
        }
    }
    let mut out = Vec::new();
    let mut seen: HashSet<(String, String, i64)> = HashSet::new();
    for (ti, tp) in plan.tours.iter().enumerate() {
        // (a block dropped today has no bus; the player's own the player drives - but the one
        // he takes over the AI's bus of)
        let taken = skip.is_some_and(|(l, t)| same(l, &tp.tour.line) && same(t, &tp.tour.tour));
        if tp.tour.unplanned || tp.bus.is_none() || tp.by_player || (tp.duties.iter().any(|d| d.driver == Some(PLAYER)) && !(keep && taken)) {
            continue;
        }
        if taken && !keep {
            continue;
        }
        for (j, x) in tp.tour.trips.iter().enumerate() {
            let Some(tours) = runs.get(&(x.name.trim().to_lowercase(), x.whole_dep())) else { continue };
            // (the tour the game knows the trip by: the one it came from, else the block's own)
            let tour = if x.origin.trim().is_empty() { tp.tour.tour.trim() } else { x.origin.trim() };
            let line = tp.tour.line_of(x);
            let Some(&(line, tour, departure)) = tours.iter().find(|r| same(r.0, line) && same(r.1, tour)).or_else(|| tours.iter().find(|r| same(r.1, tour))).or(tours.first()) else { continue };
            if !seen.insert((line.to_lowercase(), tour.to_lowercase(), departure.round() as i64)) {
                continue;
            }
            let late = model.get(ti).and_then(|r| r.get(j)).and_then(|t| t.as_ref()).map_or(0.0, |t| t.dep * 60.0);
            out.push(FleetTrip { line: line.to_string(), tour: tour.to_string(), departure: Some(departure), delay: late.round() });
        }
    }
    out
}

/// The day's delays at the company clock's minute (kept in `model` until the minute, the plan or
/// the company changed).
fn model_of<'a>(s: &Session, c: &Company, plan: &Plan, model: &'a mut Kept) -> &'a Model {
    let key = (c.date.clone(), c.clock.minute, s.plan_gen, s.generation);
    if model.as_ref().is_none_or(|m| m.0 != key) {
        let cut = ck::breakdowns(c, plan, &c.date, &[]);
        let m = lt::model_with(c, plan, &c.date, &cut, &ctl::of_day(c, &c.date), Some(c.clock.minute as i32), None);
        *model = Some((key, m));
    }
    &model.as_ref().expect("made above").1
}

/// The company's buses today, as the game is to run them (`skip`: the duty's block).
fn today(s: &Session, model: &mut Kept, skip: Option<(&str, &str)>, keep: bool) -> Option<Fleet> {
    let c = s.company.as_ref()?;
    let (plan, lines) = (s.plan.as_ref()?, s.map_lines()?);
    let trips = trips_of(plan, lines, model_of(s, c, plan, model), skip, keep);
    Some(Fleet { at: real_now(), company: c.id.clone(), map: c.map.clone(), date: c.date.clone(), trips })
}

/// The company's buses for the game of a company's duty about to start (`date` the duty's day,
/// `skip` its block: its timetable line and tour), written for the plugin before the game reads
/// its timetable. A later day's duty: that day's plan, its tours as the game knows them, on time.
/// `keep`: the drive takes the bus of `skip` over from the game's AI.
pub fn for_drive(s: &mut Session, date: &str, skip: (&str, &str), keep: bool) {
    let Some(dir) = crate::addon::game_plugin_dir() else { return };
    let fleet = match s.company.as_ref() {
        Some(c) if c.date == date => {
            let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
            st.last = Some(Instant::now());
            today(s, &mut st.model, Some(skip), keep)
        }
        Some(_) => later(s, date, skip, keep),
        None => None,
    };
    // (written afresh: the drive's own, however like the last)
    let _ = bridge::clear_fleet(&dir);
    let written = fleet.map(|f| bridge::write_fleet(&dir, &f).map(|_| f.trips.len()));
    match written {
        Some(Ok(n)) => core::log_to_file(&format!("company: {n} trips of the company's buses for the game's AI ({})", dir.display())),
        Some(Err(e)) => core::log_to_file(&format!("company: the company's buses for the game: {e}")),
        None => {}
    }
}

/// A later day's plan as the game is to run it: the tours of its blocks with a bus, whole and on
/// time (the day has no delays yet).
fn later(s: &mut Session, date: &str, skip: (&str, &str), keep: bool) -> Option<Fleet> {
    let dp = match super::plan_of(s, date)? {
        Ok(dp) => dp,
        Err(_) => return None,
    };
    let c = s.company.as_ref()?;
    let live = cp::live_plan(c, &dp);
    let trips = live
        .tours
        .iter()
        .filter(|t| !t.dropped && (keep || !(same(&t.line, skip.0) && same(&t.tour, skip.1))))
        .map(|t| FleetTrip { line: t.line.clone(), tour: t.tour.clone(), departure: None, delay: 0.0 })
        .collect();
    Some(Fleet { at: real_now(), company: c.id.clone(), map: c.map.clone(), date: date.to_string(), trips })
}

/// The bridge's loop while the game runs (`drive::look`): the company's buses of today worked
/// out again as the day's delays move on, for the game of a company's duty of today, and
/// written when they changed.
pub fn refresh() {
    // (the model taken out while the session is asked for: `for_drive` holds the session and
    // then asks for this)
    let mut model = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        if st.last.is_some_and(|t| t.elapsed() < EVERY) {
            return;
        }
        st.last = Some(Instant::now());
        st.model.take()
    };
    let link = crate::addon::duty_link();
    // (only the list this drive was given: a later day's stays as it is)
    let was = link.as_ref().and_then(|(dir, _)| bridge::read_fleet(dir));
    let fleet = match (&link, &was) {
        (Some((_, a)), Some(was)) => with(|s| {
            let c = s.company.as_ref()?;
            (c.id == was.company && c.id == a.company_id && c.date == was.date).then_some(())?;
            today(s, &mut model, Some((&a.line, &a.tour)), a.take_over)
        }),
        _ => None,
    };
    state().lock().unwrap_or_else(|e| e.into_inner()).model = model;
    if let (Some(f), Some((dir, _))) = (fleet, &link) {
        if let Err(e) = bridge::write_fleet(dir, &f) {
            core::log_to_file(&format!("company: the company's buses for the game: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_core::company::day::{DutyPlan, TourPlan};
    use hub_core::company::lateness::Timed;
    use hub_core::company::network::{PlannedTrip, TourOfDay};
    use hub_core::timetable::{TourInfo, TripInfo};

    fn trip(name: &str, dep: i32, origin: &str) -> PlannedTrip {
        PlannedTrip { name: name.into(), dep, arr: dep + 20, origin: origin.into(), ..Default::default() }
    }

    fn block(tour: &str, trips: Vec<PlannedTrip>, bus: Option<u32>) -> TourPlan {
        TourPlan { tour: TourOfDay { line: "5".into(), number: "5".into(), tour: tour.into(), trips, ..Default::default() }, bus, duties: vec![DutyPlan::default()], ..Default::default() }
    }

    fn lines() -> Vec<core::LineInfo> {
        let t = |name: &str, dep: f64| TripInfo { name: name.into(), departure: dep, arrival: dep + 1200.0, ..Default::default() };
        vec![core::LineInfo {
            name: "5".into(),
            tours: vec![
                TourInfo { number: "1".into(), runs: true, trips: vec![t("5 Bf-Markt", 21_600.0), t("5 Markt-Bf", 23_400.0)], ..Default::default() },
                TourInfo { number: "2".into(), runs: true, trips: vec![t("5 Bf-Markt", 22_500.0)], ..Default::default() },
                TourInfo { number: "3".into(), runs: true, trips: vec![t("5 Bf-Markt", 25_200.0)], ..Default::default() },
            ],
            ..Default::default()
        }]
    }

    /// Only the company's blocks with a bus, but the player's, come to the game: each trip by
    /// the tour the game knows it by, as late as the day's model has it; a depot run the company
    /// added and a block dropped today do not.
    #[test]
    fn the_game_runs_the_company_buses_as_late_as_the_day_has_them() {
        let mut mine = block("9", vec![], Some(2));
        mine.duties[0].driver = Some(PLAYER);
        let plan = Plan {
            tours: vec![
                // a block of tour 1's first trip and tour 2's (the studio laid it out), and a depot run
                block("1", vec![trip("5 Bf-Markt", 360, "1"), trip("5 Bf-Markt", 375, "2"), trip("Depot-Bf", 400, "")], Some(1)),
                // dropped today: no bus
                block("3", vec![trip("5 Bf-Markt", 420, "3")], None),
                mine,
            ],
        };
        let late = |m: f64| Some(Timed { dep: m, arr: m, bus: 1, causes: vec![], earlier: vec![], known: false, lost: 0.0, recovered: 0, swapped: None, roots: vec![] });
        let model: Model = vec![vec![late(2.0), late(3.5), None], vec![None], vec![]];
        let trips = trips_of(&plan, &lines(), &model, None, false);
        let got: Vec<(&str, &str, Option<f64>, f64)> = trips.iter().map(|t| (t.line.as_str(), t.tour.as_str(), t.departure, t.delay)).collect();
        assert_eq!(got, [("5", "1", Some(21_600.0), 120.0), ("5", "2", Some(22_500.0), 210.0)]);
        // (the block of the duty driven is the player's)
        assert!(trips_of(&plan, &lines(), &model, Some(("5", "1")), false).is_empty());
    }
}
