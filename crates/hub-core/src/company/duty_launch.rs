//! The company's duty launcher (Luc: "a launcher from the bus company that works ONLY with
//! the data given in the bus company"): a duty of the company's planning as it is driven,
//! fixed - its day, its trips in order with their breaks, deadheads and relief points, the
//! bus the planning gives its tour, the depot file, the map - and what keeps it from being
//! driven now (`Hitch`). Nothing of it is chosen: the game is started with exactly this duty
//! (`Start`), and what goes wrong is put right in the planning, not by picking another duty,
//! bus, map or day.

use super::dates;
use super::model::Company;
use super::network::PlannedTrip;
use super::plan::{BusOf, DayPlan, DayTour, Problem, Who};
use crate::{TripInfo, TripRun};
use serde::{Deserialize, Serialize};

/// Which duty the launcher shows: its day, its line and tour as the plan has them, and the
/// duty of that tour (0 the first). Kept by these, not by places in a plan, so that the plan
/// made again (the company changed) still finds it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct DutyRef {
    pub date: String,
    pub line: String,
    pub tour: String,
    pub duty: usize,
}

impl DutyRef {
    pub fn of(p: &DayPlan, ti: usize, k: usize) -> Option<DutyRef> {
        let t = p.tours.get(ti)?;
        t.duties.get(k)?;
        Some(DutyRef { date: p.date.clone(), line: t.tour.line.clone(), tour: t.tour.tour.clone(), duty: k })
    }

    /// Its tour and duty in `p` (None: the plan no longer has it).
    pub fn find(&self, p: &DayPlan) -> Option<(usize, usize)> {
        if p.date != self.date {
            return None;
        }
        let ti = p.tours.iter().position(|t| t.tour.line.eq_ignore_ascii_case(&self.line) && t.tour.tour.trim() == self.tour.trim())?;
        (self.duty < p.tours[ti].duties.len()).then_some((ti, self.duty))
    }
}

/// A row of the duty's sheet, in the order it is driven.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// Signing on (`relief`: taking the bus over from the driver before, at `at`).
    SignOn { at: String, time: i32, relief: bool },
    /// A trip: with passengers, or a deadhead (a depot or empty run, `empty`).
    Trip { dep: i32, arr: i32, line: String, from: String, to: String, empty: bool, name: String },
    /// Standing between two trips at `at`, from `from` to `to` (minutes of the day).
    Break { at: String, from: i32, to: i32 },
    /// Signing off (`relief`: handing the bus to the next driver at `at`).
    SignOff { at: String, time: i32, relief: bool },
}

/// The bus the planning gives the duty's tour.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SheetBus {
    pub id: u32,
    pub file: String,
    pub name: String,
    pub number: String,
    pub plate: String,
    /// The livery it runs in: the company's house livery, else its own.
    pub paint: String,
}

/// What keeps the duty from being driven now. Each is put right in the planning.
#[derive(Clone, Debug, PartialEq)]
pub enum Hitch {
    /// The plan no longer has the duty (the timetable or the planning changed).
    Gone,
    /// No bus planned for the tour (why, when the planning knows).
    NoBus(Option<Problem>),
    /// A rental bus runs the tour: the company has no bus of its own for it to start with.
    Rental,
    /// The planned bus cannot run that day (in the workshop, gone, broken down...).
    Bus(Problem),
    /// The planned bus has no bus file to start the game with.
    NoBusFile,
    /// The planned bus is not installed (its file).
    NotInstalled(String),
    /// Nobody is planned on the duty.
    NoDriver,
    /// Someone else is planned on the duty (their name).
    NotYours(String),
    /// The player drove it already: every trip of it the game reported.
    Driven,
    /// His game ended before the duty did, and the AI drove the rest of it (`own`).
    AiDone,
    /// Its day or its time is over on the company's clock.
    Over,
    /// The day's timetable no longer has its trips as the plan has them.
    Timetable,
}

impl Hitch {
    /// What is wrong, and what to do about it (English: the keys of the translations).
    pub fn words(&self) -> (&'static str, &'static str) {
        match self {
            Hitch::Gone => ("This duty is no longer in the planning", "The timetable or the planning changed since. Choose the duty again in RosterFlow."),
            Hitch::NoBus(_) => ("No bus is planned for this duty", "Give its tour a bus of the fleet in RosterFlow."),
            Hitch::Rental => ("A rental bus runs this tour", "The company has no bus of its own on it to start with: give the tour a bus of the fleet in RosterFlow."),
            Hitch::Bus(_) => ("The planned bus cannot run this day", "Give the tour another bus of the fleet in RosterFlow."),
            Hitch::NoBusFile => ("The planned bus has no bus file", "Give the tour another bus of the fleet in RosterFlow."),
            Hitch::NotInstalled(_) => ("The planned bus is not installed", "Install it again, or give the tour another bus of the fleet in RosterFlow."),
            Hitch::NoDriver => ("Nobody is planned on this duty", "Plan yourself on it in RosterFlow (\"You\") to drive it."),
            Hitch::NotYours(_) => ("Another driver is planned on this duty", "Plan yourself on it in RosterFlow (\"You\") to drive it."),
            Hitch::Driven => ("You drove this duty already", "Its trips are reported to the company. Choose another duty of yours in RosterFlow."),
            Hitch::AiDone => {
                ("The AI drove the rest of this duty", "Your game ended before the duty did: the trips you drove are yours, the AI drove the others. Choose another duty of yours in RosterFlow.")
            }
            Hitch::Over => ("The time of this duty has passed", "The company's clock is past its end. Choose a later duty in RosterFlow."),
            Hitch::Timetable => ("The timetable changed", "The day's timetable no longer has this duty's trips as planned. Plan the day again in RosterFlow."),
        }
    }
}

/// A duty of the company as the launcher shows it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Sheet {
    pub duty: DutyRef,
    pub company: String,
    pub company_name: String,
    pub map: String,
    pub map_name: String,
    /// The line's number shown, and the timetable's tour the game drives (that of its first
    /// trip with passengers - a block of the studio may hold trips of several).
    pub number: String,
    pub game_tour: String,
    /// The numbers of the lines its trips show (in the order they first come).
    pub lines: Vec<String>,
    /// The first departure, the last arrival (minutes of the day).
    pub from: i32,
    pub to: i32,
    pub rows: Vec<Row>,
    /// The trips with passengers, and the deadheads.
    pub trips: usize,
    pub deadheads: usize,
    pub bus: Option<SheetBus>,
    /// The depot file: the line's own, else the company's.
    pub hof: String,
    /// Who the plan has on it.
    pub who: Option<Who>,
    pub hitches: Vec<Hitch>,
    /// The duty's planned trips (for the game's legs, `start`).
    pub planned: Vec<PlannedTrip>,
    /// Part of it is driven already (his game ended before the duty did, the AI drives it
    /// since; or he takes over a bus its driver drives): where the start goes on from.
    pub resume: Option<Resume>,
}

/// Where a duty driven in part goes on from: its trip (an index into `Sheet::planned`), whether
/// that trip is under way at the company's minute `now` (the game starts then, and takes the
/// bus over where it is: `Start::take_over`), the trips with passengers the player drove of it,
/// and since when the AI drives it (the company's minute; None: its driver as planned).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Resume {
    pub from: usize,
    pub under_way: bool,
    pub now: i32,
    pub driven: usize,
    pub ai_since: Option<i32>,
}

impl Sheet {
    pub fn startable(&self) -> bool {
        self.hitches.is_empty()
    }

    /// The trip files of its trips, in order (the route map draws them).
    pub fn trip_names(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for t in &self.planned {
            if !t.name.trim().is_empty() && out.last() != Some(&t.name) {
                out.push(t.name.clone());
            }
        }
        out
    }
}

/// Shorter stands than this are not shown as a break of their own (minutes).
const BREAK_MIN: i32 = 3;

/// The sheet of duty `r` in `p` (the plan of its day), at minute `now` of the company's clock
/// (`clock::now`), with the player's trip reports `trips` (to see whether he drove it).
pub fn sheet(c: &Company, p: &DayPlan, r: &DutyRef, now: i64, trips: &[TripRun]) -> Sheet {
    let mut s = Sheet { duty: r.clone(), company: c.id.clone(), company_name: c.name.clone(), map: c.map.clone(), map_name: c.map_name.clone(), ..Default::default() };
    let Some((ti, k)) = r.find(p) else {
        s.hitches.push(Hitch::Gone);
        return s;
    };
    let t = &p.tours[ti];
    let d = &t.duties[k];
    let Some(planned) = t.tour.trips.get(d.start..d.end).filter(|x| !x.is_empty()) else {
        s.hitches.push(Hitch::Gone);
        return s;
    };
    s.planned = planned.to_vec();
    s.number = t.tour.number.clone();
    s.game_tour = planned.iter().find(|x| x.counts() && !x.origin.trim().is_empty()).map(|x| x.origin.trim().to_string()).unwrap_or_else(|| t.tour.tour.clone());
    s.from = d.from;
    s.to = d.to;
    s.who = d.who;
    for x in planned {
        if !x.empty && !x.line.trim().is_empty() && !s.lines.contains(&x.line) {
            s.lines.push(x.line.clone());
        }
    }
    if s.lines.is_empty() {
        s.lines.push(t.tour.number.clone());
    }
    // the rows: signing on, the trips with the stands between them, signing off
    let first = &planned[0];
    s.rows.push(Row::SignOn { at: first.from.clone(), time: first.dep, relief: k > 0 });
    let mut prev: Option<&PlannedTrip> = None;
    for x in planned {
        if let Some(b) = prev {
            if x.dep - b.arr >= BREAK_MIN {
                s.rows.push(Row::Break { at: b.end().to_string(), from: b.arr, to: x.dep });
            }
        }
        // (the first part of a trip cut at a relief point on the way: to the stop the next
        // driver takes over at)
        let to = if x.part.as_ref().is_some_and(|p| !p.second) { x.end().to_string() } else { x.to.clone() };
        s.rows.push(Row::Trip { dep: x.dep, arr: x.arr, line: x.line.clone(), from: x.from.clone(), to, empty: x.empty, name: x.name.clone() });
        if x.empty {
            s.deadheads += 1;
        } else {
            s.trips += 1;
        }
        prev = Some(x);
    }
    let last = planned.last().unwrap_or(first);
    s.rows.push(Row::SignOff { at: last.end().to_string(), time: d.to.max(last.arr), relief: k + 1 < t.duties.len() });
    // the depot file: the line's own, else the company's
    s.hof = c.lines.iter().find(|x| x.name.eq_ignore_ascii_case(&t.tour.line)).map(|x| x.hof_or(&c.depot).to_string()).unwrap_or_else(|| c.depot.clone());
    // the bus
    match t.bus {
        None => s.hitches.push(Hitch::NoBus(t.bus_problem)),
        Some(BusOf::Rental) => s.hitches.push(Hitch::Rental),
        Some(BusOf::Own(id)) => match c.vehicle(id) {
            None => s.hitches.push(Hitch::Bus(Problem::Gone)),
            Some(v) => {
                s.bus = Some(SheetBus {
                    id,
                    file: v.bus.clone(),
                    name: v.name.clone(),
                    number: v.number.clone(),
                    plate: v.plate.clone(),
                    paint: v.house_livery.clone().filter(|h| !h.trim().is_empty()).unwrap_or_else(|| v.livery.clone()),
                });
                if !v.held_on(&p.date) {
                    s.hitches.push(Hitch::Bus(Problem::Gone));
                } else if v.in_workshop(&p.date) {
                    s.hitches.push(Hitch::Bus(Problem::Workshop));
                } else if v.bus.trim().is_empty() {
                    s.hitches.push(Hitch::NoBusFile);
                }
            }
        },
    }
    // the driver
    match d.who {
        Some(Who::Player) => {}
        None => s.hitches.push(Hitch::NoDriver),
        Some(Who::Staff(id)) => s.hitches.push(Hitch::NotYours(c.employee(id).map(|e| e.name.clone()).unwrap_or_else(|| "?".into()))),
        Some(Who::Agency) => s.hitches.push(Hitch::NotYours("Agency driver".into())),
        Some(Who::Guest(g)) => s.hitches.push(Hitch::NotYours(super::together::guest_name(c, g))),
    }
    // driven already (every trip of it, as the game reported them), driven in part, or over
    let minute = (now - super::clock::moment(&p.date, 0)) as i32;
    if t.by_player {
        s.hitches.push(Hitch::Driven);
    } else if p.today {
        match progress(c, r, t, planned, trips, minute) {
            Progress::Fresh => {}
            Progress::All => s.hitches.push(Hitch::Driven),
            Progress::AiDone => s.hitches.push(Hitch::AiDone),
            Progress::From(x) => s.resume = Some(x),
        }
    }
    let end = super::clock::moment(&p.date, s.to as i64);
    if dates::between(&c.date, &p.date) < 0 || now >= end {
        s.hitches.push(Hitch::Over);
    }
    s
}

/// How much of a duty is driven.
enum Progress {
    /// None of it.
    Fresh,
    /// Every trip with passengers of it, by the player.
    All,
    /// What he left of it the AI drove to its end.
    AiDone,
    /// Part of it: the start goes on from there.
    From(Resume),
}

/// How much of duty `r` (`t`'s trips `planned`) is driven at the company's `minute`: the trips
/// with passengers the player reported since the company's last close (`trips_seen`: a trip of
/// its line and the game's tour leaving at the minute one of its trips leaves), and what the AI
/// drives of it since his game ended before it (`own`). One trip reported no longer makes all of
/// it driven (Luc, 2026-10-31: a game that crashed halfway had it "driven").
fn progress(c: &Company, r: &DutyRef, t: &DayTour, planned: &[PlannedTrip], trips: &[TripRun], minute: i32) -> Progress {
    let own = super::own::find(c, r);
    let reported = |x: &PlannedTrip| {
        trips
            .iter()
            .filter(|r| {
                r.time > c.trips_seen
                    && !r.free
                    && same_map(&r.map, &c.map)
                    && (r.line.trim().eq_ignore_ascii_case(t.tour.number.trim()) || (!x.line.trim().is_empty() && r.line.trim().eq_ignore_ascii_case(x.line.trim())))
            })
            .any(|r| (r.departure / 60.0).round() as i32 == x.whole_dep() && (if x.origin.trim().is_empty() { &t.tour.tour } else { &x.origin }).trim() == r.tour.trim())
    };
    let counted = |i: usize| planned[i].counts() && !planned[i].is_second();
    let drove = |i: usize| own.is_some_and(|o| o.drove(i)) || reported(&planned[i]);
    let all: Vec<usize> = (0..planned.len()).filter(|&i| counted(i)).collect();
    let driven = all.iter().filter(|&&i| drove(i)).count();
    if !all.is_empty() && driven == all.len() {
        return Progress::All;
    }
    if let Some(a) = own.and_then(|o| o.ai_now()) {
        return match super::own::back_at(c, r, minute) {
            Some((from, under_way)) => Progress::From(Resume { from, under_way, now: minute, driven, ai_since: Some(a.minute) }),
            None => Progress::AiDone,
        };
    }
    if driven == 0 {
        return Progress::Fresh;
    }
    // (driven in part, the AI not on it: from his first trip not driven that is still to come)
    match all.iter().copied().find(|&i| !drove(i) && planned[i].arr > minute) {
        Some(from) => Progress::From(Resume { from, under_way: planned[from].dep <= minute, now: minute, driven, ai_since: None }),
        None => Progress::Fresh,
    }
}

/// A sheet of a duty another drives now (one of the company's drivers, the agency's, the AI),
/// for the player to take its bus over where it is at the company's `minute`: from its trip
/// with passengers that has not arrived yet (None: none is left). Nobody's to drive besides.
pub fn take_over(mut s: Sheet, minute: i32) -> Option<Sheet> {
    s.hitches.retain(|h| !matches!(h, Hitch::NotYours(_) | Hitch::NoDriver));
    let from = (0..s.planned.len()).find(|&i| s.planned[i].counts() && !s.planned[i].is_second() && s.planned[i].arr > minute)?;
    let under_way = s.planned[from].dep <= minute;
    s.resume = Some(Resume { from, under_way, now: minute, driven: 0, ai_since: None });
    Some(s)
}

fn same_map(a: &str, b: &str) -> bool {
    let n = |s: &str| s.trim().replace('\\', "/").to_ascii_lowercase();
    a.trim().is_empty() || n(a) == n(b) || n(a).ends_with(&n(b)) || n(b).ends_with(&n(a))
}

/// A duty's trips (`planned`, as the plan has them) in its tour of the day's timetable
/// (`trips`): the first one's place as the game counts it (1 the first), the minute it leaves,
/// and how many there are. The plan has the trips in the order they leave, an own line's with
/// the depot runs its timetable lacks: each is found by its name and the minute it leaves, the
/// depot runs are not. None: none is found, or a trip with passengers is missing or out of
/// order (the timetable changed since the plan was made).
pub fn timetable_legs(planned: &[PlannedTrip], trips: &[TripInfo]) -> Option<(usize, i32, usize)> {
    timetable_part(planned, trips).map(|p| (p.first, p.time, p.trips))
}

/// A duty's trips in its tour of the day's timetable (`timetable_legs`), and where it begins
/// and ends on the way: a duty that takes a bus over at a relief point halfway along a trip
/// (RosterFlow's mid-trip relief, `network::Part`) drives that trip from that stop on, one that
/// hands the bus over halfway ends its last trip there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimetablePart {
    pub first: usize,
    /// The minute the duty's first trip leaves (from the relief point, where it begins there).
    pub time: i32,
    pub trips: usize,
    /// The stop of the first trip it begins at, of the last it ends at (0 the first).
    pub from_stop: Option<usize>,
    pub to_stop: Option<usize>,
}

pub fn timetable_part(planned: &[PlannedTrip], trips: &[TripInfo]) -> Option<TimetablePart> {
    let mut found: Vec<(usize, i32, &PlannedTrip)> = Vec::new();
    for x in planned {
        // (a part of a trip is found by the whole trip's minute)
        match trips.iter().find(|y| y.name == x.name && (y.departure / 60.0).round() as i32 == x.whole_dep()) {
            Some(y) => found.push((y.index, x.dep, x)),
            None if x.empty => {}
            None => return None,
        }
    }
    let (first, time, x0) = *found.first()?;
    if found.iter().enumerate().any(|(n, f)| f.0 != first + n) {
        return None;
    }
    let last = found.last().map(|f| f.2).unwrap_or(x0);
    let from_stop = x0.part.as_ref().filter(|p| p.second).map(|p| p.stop);
    let to_stop = last.part.as_ref().filter(|p| !p.second).map(|p| p.stop);
    Some(TimetablePart { first, time, trips: found.len(), from_stop, to_stop })
}

/// What the game is started with for a duty: everything fixed by the company.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Start {
    pub company: String,
    pub map: String,
    /// The day and the time the game starts at: the bus's lead before the first departure.
    pub date: String,
    pub time: i32,
    pub line: String,
    pub tour: String,
    /// The first trip (its place in the tour) and the duty's part as `--duty-leg` takes it.
    pub first: usize,
    pub legs: Vec<String>,
    /// The stop of its first trip it begins at, and of its last it ends at, where it does so on
    /// the way (a relief halfway along a trip: `TimetablePart`).
    pub from_stop: Option<usize>,
    pub to_stop: Option<usize>,
    pub bus: String,
    pub paint: String,
    pub number: String,
    pub plate: String,
    pub hof: String,
    /// A duty taken up halfway along a trip (`Resume::under_way`): the trip (its place in the
    /// tour, from 1) and the stop it is taken from (from 1 among those it calls at). The game's
    /// plugin takes over the AI's bus of the tour where it is then (`plugin/takeover.lua`), or
    /// puts the bus at that stop.
    pub take_over: Option<(usize, usize)>,
}

/// The start of a sheet's duty, with the day's timetable `lines` and the bus's `lead` before
/// the first departure (minutes): Err the hitch that keeps it.
pub fn start(s: &Sheet, lines: &[crate::LineInfo], lead: i64) -> Result<Start, Hitch> {
    if let Some(h) = s.hitches.first() {
        return Err(h.clone());
    }
    let Some(bus) = s.bus.as_ref() else { return Err(Hitch::NoBus(None)) };
    let line = s.duty.line.clone();
    // (driven in part: from the trip it goes on from)
    let planned = match s.resume {
        Some(x) => &s.planned[x.from.min(s.planned.len())..],
        None => &s.planned[..],
    };
    let tour = lines.iter().find(|x| x.name.eq_ignore_ascii_case(&line)).and_then(|l| l.tours.iter().find(|t| t.number.trim() == s.game_tour.trim()));
    let Some(part) = tour.and_then(|t| timetable_part(planned, &t.trips)) else { return Err(Hitch::Timetable) };
    let (first, dep, count) = (part.first, part.time, part.trips);
    let (mut date, mut time) = crate::lead_start(&s.duty.date, dep, lead);
    let mut from_stop = part.from_stop;
    let mut take_over = None;
    if let Some(x) = s.resume {
        let started = super::clock::moment(&date, time as i64) - super::clock::moment(&s.duty.date, 0);
        if x.under_way {
            // (under way: now, from the trip's next stop by the timetable - the plugin takes
            // the AI's bus over where it really is)
            (date, time) = crate::lead_start(&s.duty.date, x.now, 0);
            let trip = tour.and_then(|t| t.trips.iter().find(|y| y.index == first));
            let next = trip.and_then(|y| y.stops.iter().position(|st| st.dep > x.now as f64 * 60.0)).unwrap_or(0);
            from_stop = from_stop.max(Some(next)).filter(|&k| k > 0);
            take_over = Some((first, from_stop.map_or(1, |k| k + 1)));
        } else if started < x.now as i64 {
            // (not before the company's clock)
            (date, time) = crate::lead_start(&s.duty.date, x.now, 0);
        }
    }
    let leg = crate::compose::Block { line: line.clone(), tour: s.game_tour.clone(), first, trips: count, departure: 0.0 };
    Ok(Start {
        company: s.company.clone(),
        map: s.map.clone(),
        date,
        time,
        line: line.clone(),
        tour: s.game_tour.clone(),
        first,
        // (`line|tour|first|trips`, a relief on the way after an `@`)
        legs: vec![leg.arg_between(from_stop, part.to_stop)],
        from_stop,
        to_stop: part.to_stop,
        bus: bus.file.clone(),
        paint: bus.paint.clone(),
        number: bus.number.clone(),
        plate: bus.plate.clone(),
        hof: s.hof.clone(),
        take_over,
    })
}

impl Start {
    /// The duty as the backend takes it, made of `base` (the player's game options: traffic,
    /// the engine running, the profile) with everything of the duty the company's - and the
    /// weather chosen (empty: as the map has it).
    pub fn duty(&self, base: crate::Duty, weather: &str) -> crate::Duty {
        let mut d = base;
        d.map = self.map.clone();
        d.bus = self.bus.clone();
        d.paint = Some(self.paint.clone()).filter(|p| !p.trim().is_empty());
        d.plate = Some(self.plate.clone()).filter(|p| !p.trim().is_empty());
        d.number = Some(self.number.clone()).filter(|n| !n.trim().is_empty());
        d.hof = Some(self.hof.clone()).filter(|h| !h.trim().is_empty());
        // (the entry point nearest to the duty's first stop)
        d.entry = Some(-1);
        d.line = Some(self.line.clone());
        d.tour = Some(self.tour.clone());
        d.trip = Some(self.first.to_string());
        d.whole_tour = true;
        d.legs = self.legs.clone();
        d.free_line = None;
        d.time = format!("{:02}:{:02}", self.time / 60, self.time % 60);
        d.date = Some(self.date.clone());
        d.weather = Some(weather.to_string()).filter(|w| !w.trim().is_empty());
        // (the company's service: its passengers and its buses on the road, the season by
        // the date, alone - not in someone's session)
        d.passengers = Some(true);
        d.schedule = Some(true);
        d.on_foot = Some(false);
        d.lan = Some("off".into());
        d.lan_name = None;
        d.season = None;
        d.tutorial = None;
        d.situation = None;
        d.editor = false;
        d.company = Some(self.company.clone());
        d
    }
}

#[cfg(test)]
mod tests {
    use super::super::market::{self, MarketBus, Payment};
    use super::super::model::{CompanyLine, GameMode};
    use super::super::network::TourOfDay;
    use super::super::plan::{day_plan, set_bus, set_driver, weekday_of};
    use super::super::{found, Founding};
    use super::*;

    fn trip(name: &str, dep: i32, arr: i32, from: &str, to: &str, empty: bool) -> PlannedTrip {
        PlannedTrip {
            name: name.into(),
            line: if empty { String::new() } else { "5".into() },
            from: from.into(),
            to: to.into(),
            dep,
            arr,
            km: 8.0,
            stops: if empty { 0 } else { 10 },
            empty,
            origin: String::new(),
            last: String::new(),
            part: None,
            of_line: String::new(),
        }
    }

    /// A company with one bus and line 5, whose tour 2 is a depot run, three trips and the run
    /// back, the player planned on it with the bus.
    fn setup() -> (Company, Vec<TourOfDay>) {
        let mut c = found(&Founding { name: "Stadtbus".into(), game_mode: GameMode::Casual, date: "2024-03-04".into(), map: "maps/Grundorf/global.cfg".into(), ..Default::default() }, "Luc");
        c.depot = "Grundorf.hof".into();
        c.lines.push(CompanyLine { name: "Linie5".into(), number: "5".into(), numbers: vec!["5".into()], added: c.date.clone(), service_from: Some(0), ..Default::default() });
        c.cash += 5_000_000_00;
        market::buy_new(&mut c, &MarketBus { file: "Vehicles/Citaro/Citaro.bus".into(), name: "Citaro".into(), ..Default::default() }, Payment::Cash, "").unwrap();
        let tours = vec![TourOfDay {
            line: "Linie5".into(),
            number: "5".into(),
            tour: "2".into(),
            ai_group: String::new(),
            trips: vec![
                trip("Depot-A", 350, 358, "Depot", "A", true),
                trip("5 A-B", 360, 390, "A", "B", false),
                trip("5 B-A", 400, 430, "B", "A", false),
                trip("5 A-B", 440, 470, "A", "B", false),
                trip("A-Depot", 472, 480, "A", "Depot", true),
            ],
            unplanned: false,
            cuts: None,
            duty_max: None,
        }];
        let wd = weekday_of(&c.date);
        let id = c.fleet[0].id;
        set_bus(&mut c, wd, "Linie5", "2", Some(id));
        set_driver(&mut c, wd, "Linie5", "2", 0, Some(Who::Player));
        (c, tours)
    }

    fn info(name: &str, index: usize, dep: i32) -> TripInfo {
        TripInfo { name: name.into(), index, line: "5".into(), from: "A".into(), terminus: "B".into(), departure: dep as f64 * 60.0, arrival: (dep + 30) as f64 * 60.0, stops: Vec::new(), km: 8.0 }
    }

    fn timetable() -> Vec<crate::LineInfo> {
        vec![crate::LineInfo {
            name: "Linie5".into(),
            user_allowed: true,
            termini: Vec::new(),
            tours: vec![crate::TourInfo {
                number: "2".into(),
                ai_group: String::new(),
                first: 360.0 * 60.0,
                last: 470.0 * 60.0,
                days: "daily".into(),
                runs: true,
                next_run: None,
                trips: vec![info("5 A-B", 1, 360), info("5 B-A", 2, 400), info("5 A-B", 3, 440)],
            }],
        }]
    }

    #[test]
    fn the_launcher_shows_the_duty_as_planned() {
        let (c, tours) = setup();
        let p = day_plan(&c, &c.date, tours, &[], &[], false);
        let r = DutyRef::of(&p, 0, 0).unwrap();
        let s = sheet(&c, &p, &r, super::super::clock::moment(&c.date, 300), &[]);
        assert!(s.startable(), "{:?}", s.hitches);
        // the bus the planning gives the tour, in the company's livery
        let v = &c.fleet[0];
        let b = s.bus.as_ref().unwrap();
        assert_eq!((b.id, b.file.as_str(), b.number.as_str()), (v.id, v.bus.as_str(), v.number.as_str()));
        // its trips in order, with the stands, the deadheads and signing on and off
        assert_eq!((s.trips, s.deadheads, s.from, s.to), (3, 2, 350, 480));
        assert!(matches!(&s.rows[0], Row::SignOn { at, time: 350, relief: false } if at == "Depot"));
        assert!(matches!(&s.rows[1], Row::Trip { empty: true, .. }));
        assert!(matches!(&s.rows[3], Row::Break { from: 390, to: 400, .. }));
        assert!(matches!(s.rows.last(), Some(Row::SignOff { at, time: 480, relief: false }) if at == "Depot"));
        assert_eq!(s.hof, "Grundorf.hof");
        assert_eq!(s.lines, vec!["5".to_string()]);
        assert_eq!(s.trip_names(), vec!["Depot-A", "5 A-B", "5 B-A", "5 A-B", "A-Depot"]);
    }

    #[test]
    fn the_start_is_exactly_the_planned_duty() {
        let (c, tours) = setup();
        let p = day_plan(&c, &c.date, tours, &[], &[], false);
        let s = sheet(&c, &p, &DutyRef::of(&p, 0, 0).unwrap(), super::super::clock::moment(&c.date, 300), &[]);
        let st = start(&s, &timetable(), 10).unwrap();
        let v = &c.fleet[0];
        assert_eq!((st.bus.as_str(), st.number.as_str(), st.plate.as_str()), (v.bus.as_str(), v.number.as_str(), v.plate.as_str()));
        assert_eq!((st.line.as_str(), st.tour.as_str(), st.first), ("Linie5", "2", 1));
        assert_eq!(st.legs, vec!["Linie5|2|1|3".to_string()]);
        // the lead before the first trip with passengers, on the plan's day
        assert_eq!((st.date.as_str(), st.time), ("2024-03-04", 350));
        let base = crate::Duty {
            bus: "Vehicles/Other/other.bus".into(),
            map: "maps/Other/global.cfg".into(),
            traffic: Some(40),
            weather: Some("x.wtr".into()),
            passengers: Some(false),
            lan: Some("host".into()),
            ..Default::default()
        };
        let d = st.duty(base, "");
        assert_eq!((d.map.as_str(), d.bus.as_str()), ("maps/Grundorf/global.cfg", v.bus.as_str()));
        assert_eq!((d.line.as_deref(), d.tour.as_deref(), d.trip.as_deref(), d.time.as_str(), d.date.as_deref()), (Some("Linie5"), Some("2"), Some("1"), "05:50", Some("2024-03-04")));
        assert_eq!(d.legs, st.legs);
        assert_eq!(d.hof.as_deref(), Some("Grundorf.hof"));
        assert_eq!(d.company.as_deref(), Some(c.id.as_str()));
        // the player's game options stay; nothing of another drive does
        assert_eq!((d.traffic, d.weather, d.passengers, d.lan.as_deref(), d.entry), (Some(40), None, Some(true), Some("off"), Some(-1)));
    }

    #[test]
    fn a_problem_blocks_the_start_with_its_reason() {
        let (mut c, tours) = setup();
        let lines = timetable();
        let now = super::super::clock::moment(&c.date, 300);
        // the bus in the workshop
        c.fleet[0].workshop_until = Some(c.date.clone());
        let p = day_plan(&c, &c.date, tours.clone(), &[], &[], false);
        let s = sheet(&c, &p, &DutyRef::of(&p, 0, 0).unwrap(), now, &[]);
        assert!(!s.startable());
        assert!(start(&s, &lines, 10).is_err());
        c.fleet[0].workshop_until = None;
        // another driver on it
        let wd = weekday_of(&c.date);
        set_driver(&mut c, wd, "Linie5", "2", 0, None);
        let p = day_plan(&c, &c.date, tours.clone(), &[], &[], false);
        let s = sheet(&c, &p, &DutyRef::of(&p, 0, 0).unwrap(), now, &[]);
        assert!(s.hitches.iter().any(|h| matches!(h, Hitch::NoDriver | Hitch::NotYours(_))), "{:?}", s.hitches);
        set_driver(&mut c, wd, "Linie5", "2", 0, Some(Who::Player));
        // its time over on the company's clock
        let p = day_plan(&c, &c.date, tours.clone(), &[], &[], false);
        let r = DutyRef::of(&p, 0, 0).unwrap();
        let s = sheet(&c, &p, &r, super::super::clock::moment(&c.date, 481), &[]);
        assert_eq!(start(&s, &lines, 10), Err(Hitch::Over));
        // driven in part (a trip report of one of its trips, the game ended): not driven - it
        // goes on from its first trip not driven
        let run = |dep: f64| TripRun { time: c.trips_seen + 1, map: c.map.clone(), line: "5".into(), tour: "2".into(), departure: dep * 60.0, ..Default::default() };
        let s = sheet(&c, &p, &r, now, &[run(400.0)]);
        assert!(s.hitches.is_empty(), "{:?}", s.hitches);
        assert_eq!(s.resume, Some(Resume { from: 1, under_way: false, now: 300, driven: 1, ai_since: None }));
        // driven already: every trip of it reported
        let s = sheet(&c, &p, &r, now, &[run(360.0), run(400.0), run(440.0)]);
        assert_eq!(s.hitches, vec![Hitch::Driven]);
        // the timetable changed: a trip of the duty is no longer in it
        let s = sheet(&c, &p, &r, now, &[]);
        let mut changed = lines.clone();
        changed[0].tours[0].trips.remove(1);
        assert_eq!(start(&s, &changed, 10), Err(Hitch::Timetable));
        // the plan no longer has the duty
        let gone = DutyRef { tour: "9".into(), ..r.clone() };
        assert_eq!(sheet(&c, &p, &gone, now, &[]).hitches, vec![Hitch::Gone]);
        // every hitch says what is wrong and what to do
        for h in [
            Hitch::Gone,
            Hitch::NoBus(None),
            Hitch::Rental,
            Hitch::Bus(Problem::Workshop),
            Hitch::NoBusFile,
            Hitch::NotInstalled(String::new()),
            Hitch::NoDriver,
            Hitch::NotYours(String::new()),
            Hitch::Driven,
            Hitch::Over,
            Hitch::Timetable,
        ] {
            let (a, b) = h.words();
            assert!(!a.is_empty() && !b.is_empty());
        }
    }

    #[test]
    fn a_duty_begins_or_ends_halfway_along_a_trip_at_its_relief_point() {
        // tour 2's first trip cut at its stop 3 (5 A-B at 06:00, there at 06:15): the first
        // duty hands the bus over there, the second takes it over and goes on
        let (mut c, mut tours) = setup();
        let (a, b) = tours[0].trips[1].split_at(3, "Mitte", 375, 0.5).unwrap();
        tours[0].trips[1] = a;
        tours[0].trips.insert(2, b);
        tours[0].cuts = Some(vec![2]);
        let wd = weekday_of(&c.date);
        set_driver(&mut c, wd, "Linie5", "2", 1, Some(Who::Player));
        let now = super::super::clock::moment(&c.date, 300);
        let p = day_plan(&c, &c.date, tours.clone(), &[], &[], false);
        // the second duty: from Mitte at 06:15, the rest of that trip and the two after
        let s = sheet(&c, &p, &DutyRef::of(&p, 0, 1).unwrap(), now, &[]);
        assert!(s.startable(), "{:?}", s.hitches);
        assert!(matches!(&s.rows[0], Row::SignOn { at, time: 375, relief: true } if at == "Mitte"));
        let st = start(&s, &timetable(), 10).unwrap();
        assert_eq!((st.first, st.from_stop, st.to_stop), (1, Some(3), None));
        assert_eq!(st.legs, vec!["Linie5|2|1@3|3".to_string()]);
        assert_eq!(crate::compose::Block::parse(&st.legs[0]), Some(("Linie5".into(), "2".into(), 1, 3)));
        // (the game starts before the relief, not before the trip's first stop)
        assert_eq!((st.date.as_str(), st.time), ("2024-03-04", 365));
        // the first duty: out of the depot, to Mitte - its trip ends there
        set_driver(&mut c, wd, "Linie5", "2", 1, None);
        set_driver(&mut c, wd, "Linie5", "2", 0, Some(Who::Player));
        let p = day_plan(&c, &c.date, tours, &[], &[], false);
        let s = sheet(&c, &p, &DutyRef::of(&p, 0, 0).unwrap(), now, &[]);
        assert!(s.startable(), "{:?}", s.hitches);
        assert!(matches!(&s.rows[2], Row::Trip { to, .. } if to == "Mitte"));
        assert!(matches!(s.rows.last(), Some(Row::SignOff { at, time: 375, relief: true }) if at == "Mitte"));
        let st = start(&s, &timetable(), 10).unwrap();
        assert_eq!((st.first, st.from_stop, st.to_stop), (1, None, Some(3)));
        assert_eq!(st.legs, vec!["Linie5|2|1|1@3".to_string()]);
        assert_eq!(st.duty(crate::Duty::default(), "").legs, st.legs);
        // a trip report of the whole trip says the duty was driven
        let run = TripRun { time: c.trips_seen + 1, map: c.map.clone(), line: "5".into(), tour: "2".into(), departure: 360.0 * 60.0, ..Default::default() };
        assert_eq!(sheet(&c, &p, &DutyRef::of(&p, 0, 0).unwrap(), now, &[run]).hitches, vec![Hitch::Driven]);
    }

    /// His game crashed after the first trip of three (Luc, 2026-10-31): the duty is not
    /// driven; the AI drives it from its second trip on; the launcher takes it back from the AI
    /// halfway along the trip it drives then; and the close books his trip as measured and the
    /// AI's as run - none dropped, none his that he did not drive.
    #[test]
    fn a_duty_whose_game_crashed_halfway_is_the_ai_s_from_there() {
        use super::super::own::{self, Ended};
        use super::super::together::book::LeftWhy;
        let (mut c, tours) = setup();
        let p = day_plan(&c, &c.date, tours.clone(), &[], &[], false);
        let r = DutyRef::of(&p, 0, 0).unwrap();
        let s = sheet(&c, &p, &r, super::super::clock::moment(&c.date, 300), &[]);
        let since = c.trips_seen + 1;
        own::begin(&mut c, &r, &s.planned, &s.game_tour, 0, since);
        let run = TripRun {
            time: c.trips_seen + 2,
            map: c.map.clone(),
            line: "5".into(),
            tour: "2".into(),
            departure: 360.0 * 60.0,
            metres: 8000.0,
            seconds: 1800.0,
            stops: 10,
            completed: true,
            ..Default::default()
        };
        assert!(own::note_trips(&mut c, std::slice::from_ref(&run)));
        assert_eq!(own::ended(&mut c, &r, LeftWhy::Crashed, 0, 395), Some(Ended::Ai { from: 2, driven: 1 }));
        // the launcher: not driven - taken back where the AI's bus is, on its trip of 06:40
        let s = sheet(&c, &p, &r, super::super::clock::moment(&c.date, 410), std::slice::from_ref(&run));
        assert!(s.hitches.is_empty(), "{:?}", s.hitches);
        assert_eq!(s.resume.map(|x| (x.from, x.under_way, x.ai_since)), Some((2, true, Some(395))));
        let st = start(&s, &timetable(), 10).unwrap();
        assert_eq!((st.first, st.time, st.take_over), (2, 410, Some((2, 1))));
        // the close: his trip measured, the AI's two run
        let report = super::super::day::close_day(&mut c, tours, &[run]);
        assert_eq!((report.measured, report.dropped, report.by_player), (1, 0, 0), "{report:?}");
    }

    #[test]
    fn a_duty_is_driven_from_its_own_first_trip() {
        let planned = |name: &str, dep: i32, depot: bool| PlannedTrip {
            name: name.into(),
            line: if depot { String::new() } else { "7".into() },
            dep,
            arr: dep + 30,
            stops: if depot { 0 } else { 5 },
            empty: depot,
            ..Default::default()
        };
        let timetable = [info("7 A-B", 1, 360), info("7 B-A", 2, 400), info("7 A-B", 3, 440)];
        // an own line's tour as the plan has it: the depot runs added before and after
        let tour = [planned("Betriebsfahrt", 352, true), planned("7 A-B", 360, false), planned("7 B-A", 400, false), planned("7 A-B", 440, false), planned("Betriebsfahrt", 470, true)];
        assert_eq!(timetable_legs(&tour[0..3], &timetable), Some((1, 360, 2)));
        assert_eq!(timetable_legs(&tour[3..5], &timetable), Some((3, 440, 1)));
        assert_eq!(timetable_legs(&tour[1..4], &timetable), Some((1, 360, 3)));
        assert_eq!(timetable_legs(&tour[4..5], &timetable), None);
        // a trip of the duty the timetable no longer has, or one out of order
        assert_eq!(timetable_legs(&tour[1..4], &[info("7 A-B", 1, 360), info("7 A-B", 3, 440)]), None);
        assert_eq!(timetable_legs(&tour[1..3], &[info("7 A-B", 1, 360), info("7 B-A", 3, 400)]), None);
    }
}
