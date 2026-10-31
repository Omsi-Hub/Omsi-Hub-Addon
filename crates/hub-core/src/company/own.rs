//! The player's own drives of the company's duties, alone in a game of his own (Luc, 2026-10-31:
//! "when I drive a duty in the company mode and the game crashes, it registers that I drove the
//! duty. That must not be: the live game data decides whether it was driven, how much of it, and
//! when the AI takes over. The same for a player who wants to take over an AI bus").
//!
//! A duty counts as driven by the player only for the trips the game reported he drove
//! (`OwnDrive::driven`); one trip of it no longer makes all of it his. While his game runs, the
//! app writes down here what the game says of the drive (the trip under way, the company's
//! minute: `live`); when the game ends before the duty did - closed, or crashed - the AI drives
//! the rest of it from the trip after the last he drove, with the same model as the company's own
//! drivers (`ended`, the model's `day::close_day_with` and `lateness`), as "Play together" does
//! for a player who left (`together::book::hand_to_ai`). He may take it back from the AI where
//! its bus is then (`back_at`), and he may take over a bus the AI or one of the company's drivers
//! drives (`begin` with `from` the trip under way): the trips before are that driver's, the rest
//! his.

use super::duty_launch::DutyRef;
use super::model::Company;
use super::network::PlannedTrip;
use super::together::book::{AiPart, LeftWhy};
use crate::TripRun;
use serde::{Deserialize, Serialize};

/// A duty of the company the player drives (or drove) in his own game, as the game said.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct OwnDrive {
    pub duty: DutyRef,
    /// Its trips as when he began it (its tour's trips of the duty), and the timetable's tour
    /// the game knows them by.
    pub sheet: Vec<PlannedTrip>,
    pub game_tour: String,
    /// The first of its trips he drives (an index into `sheet`): 0, or the trip under way where
    /// he took the bus over - those before are its driver's, as planned.
    pub from: usize,
    /// When his first drive of it began (Unix seconds): his trips reported since count; and
    /// when his last began.
    pub since: u64,
    pub began: u64,
    /// The departures (whole minutes) of the trips of it the game reported he drove.
    pub driven: Vec<i32>,
    /// His game runs it now, and the company's minute the game was last seen at (the clock
    /// follows the game's while it runs).
    pub running: bool,
    pub seen: Option<i32>,
    /// The parts of it the AI drives because his game ended before the duty did.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ai: Vec<AiPart>,
}

/// A trip a player reports (with passengers, not a trip's second part).
fn reportable(x: &PlannedTrip) -> bool {
    x.counts() && !x.is_second()
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

impl OwnDrive {
    /// The AI drives it now (from where his game ended, not taken back yet).
    pub fn ai_now(&self) -> Option<&AiPart> {
        self.ai.last().filter(|a| a.until.is_none())
    }

    /// The AI drives (or drove) trip `i` of it.
    pub fn ai_drives(&self, i: usize) -> bool {
        self.ai.iter().any(|a| a.has(i))
    }

    /// Trip `i` is his to drive: from where he began, and not the AI's.
    pub fn his(&self, i: usize) -> bool {
        i >= self.from && !self.ai_drives(i)
    }

    /// The game reported he drove trip `i`.
    pub fn drove(&self, i: usize) -> bool {
        self.sheet.get(i).is_some_and(|x| reportable(x) && self.driven.contains(&x.whole_dep()))
    }

    /// A trip the game reported is one of this duty's: its line, the tour the game knows it by
    /// and the minute it leaves (the index into `sheet`).
    pub fn trip_of(&self, t: &TripRun) -> Option<usize> {
        if t.free || t.time < self.since {
            return None;
        }
        let dep = (t.departure / 60.0).round() as i32;
        self.sheet.iter().position(|x| {
            let tour = if x.origin.trim().is_empty() { self.game_tour.as_str() } else { x.origin.as_str() };
            reportable(x) && x.whole_dep() == dep && same(&x.line, &t.line) && same(tour, &t.tour)
        })
    }

    /// Where he got to: the trip after the last he drove (an index into `sheet`; `from` when he
    /// drove none), and after the AI's part he took back.
    pub fn reached(&self) -> usize {
        let last = self.sheet.iter().enumerate().rposition(|(i, x)| reportable(x) && self.drove(i)).map_or(0, |i| i + 1);
        last.max(self.from).max(self.ai.last().and_then(|a| a.until).unwrap_or(0))
    }

    /// Every trip of it with passengers from where he began is driven - by him, or by the AI.
    pub fn done(&self) -> bool {
        (self.from..self.sheet.len()).all(|i| !reportable(&self.sheet[i]) || self.drove(i) || self.ai_drives(i))
    }

    /// How many trips with passengers of it he drove.
    pub fn trips_driven(&self) -> usize {
        (0..self.sheet.len()).filter(|&i| self.drove(i)).count()
    }
}

/// The record of duty `r`.
pub fn find<'a>(c: &'a Company, r: &DutyRef) -> Option<&'a OwnDrive> {
    c.own.iter().find(|x| x.duty == *r)
}

fn find_mut<'a>(c: &'a mut Company, r: &DutyRef) -> Option<&'a mut OwnDrive> {
    c.own.iter_mut().find(|x| x.duty == *r)
}

/// The record of duty `k` of `line`/`tour` on `date`.
pub fn of<'a>(c: &'a Company, date: &str, line: &str, tour: &str, k: usize) -> Option<&'a OwnDrive> {
    c.own.iter().find(|x| x.duty.date == date && x.duty.duty == k && same(&x.duty.line, line) && x.duty.tour.trim() == tour.trim())
}

/// His game starts on duty `r` (its trips `sheet`, the game's tour `game_tour`) from its trip
/// `from` (`now_unix` the real time): a duty he drives for the first time, one he takes back from
/// the AI (its part ends there), or a bus he takes over from its driver at the trip under way.
pub fn begin(c: &mut Company, r: &DutyRef, sheet: &[PlannedTrip], game_tour: &str, from: usize, now_unix: u64) {
    match find_mut(c, r) {
        Some(x) => {
            if let Some(a) = x.ai.last_mut().filter(|a| a.until.is_none()) {
                // (the AI drove none of it yet: as if it never had it)
                if from <= a.from {
                    x.ai.pop();
                } else {
                    a.until = Some(from);
                }
            }
            x.from = x.from.min(from);
            x.running = true;
            x.began = now_unix;
        }
        None => c.own.push(OwnDrive { duty: r.clone(), sheet: sheet.to_vec(), game_tour: game_tour.trim().to_string(), from, since: now_unix, began: now_unix, running: true, ..Default::default() }),
    }
}

/// The trips the game reported (the driver's record) taken into the records of the company's
/// day: each one of a duty he drives. Whether any was new.
pub fn note_trips(c: &mut Company, trips: &[TripRun]) -> bool {
    let date = c.date.clone();
    let mut changed = false;
    for x in c.own.iter_mut().filter(|x| x.duty.date == date) {
        for t in trips {
            let Some(i) = x.trip_of(t) else { continue };
            let dep = x.sheet[i].whole_dep();
            if !x.driven.contains(&dep) {
                x.driven.push(dep);
                changed = true;
            }
        }
    }
    changed
}

/// The game runs duty `r` at the company's `minute`.
pub fn live(c: &mut Company, r: &DutyRef, minute: i32) {
    if let Some(x) = find_mut(c, r).filter(|x| x.running) {
        x.seen = Some(minute);
    }
}

/// What became of a duty when his game ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// He drove all of it (the AI what he left of it before).
    Done,
    /// The AI drives its trips from this one on (an index into its trips); he drove `driven`.
    Ai { from: usize, driven: usize },
}

/// His game on duty `r` ended (`why`: closed, or crashed), at the company's `minute` (the game's
/// last, where it said one) and `now_unix`: the AI drives the rest of it from the trip after the
/// last he drove. None: no drive of it ran.
pub fn ended(c: &mut Company, r: &DutyRef, why: LeftWhy, now_unix: u64, minute: i32) -> Option<Ended> {
    let x = find_mut(c, r).filter(|x| x.running)?;
    x.running = false;
    if x.done() {
        return Some(Ended::Done);
    }
    let from = x.reached();
    if !x.sheet.iter().skip(from).any(reportable) {
        return Some(Ended::Done);
    }
    let at = x.seen.unwrap_or(minute);
    x.ai.push(AiPart { from, until: None, why, at: now_unix, minute: at });
    Some(Ended::Ai { from, driven: x.trips_driven() })
}

/// The trip of duty `r` he takes back from the AI at the company's minute `now` (minutes of its
/// day): the first trip with passengers of the AI's part that has not arrived yet, and whether it
/// is under way then. None: the AI does not drive it, or it drove all of it.
pub fn back_at(c: &Company, r: &DutyRef, now: i32) -> Option<(usize, bool)> {
    let x = find(c, r)?;
    let a = x.ai_now()?;
    (a.from..x.sheet.len()).find(|&i| reportable(&x.sheet[i]) && x.sheet[i].arr > now).map(|i| (i, x.sheet[i].dep <= now))
}

/// The AI drives trip `i` (leaving at `dep`) of duty `k` of `line`/`tour` on `date`: in a part
/// it drives, and not one he reported.
pub fn ai_runs(c: &Company, date: &str, line: &str, tour: &str, k: usize, i: usize) -> bool {
    of(c, date, line, tour, k).is_some_and(|x| x.ai_drives(i) && !x.drove(i))
}

/// A trip the game reported is of a duty of the day he drives in his own game (the day's close
/// books it as measured, and his duty's other trips by who drives them - not his whole tour).
pub fn claims_trip(c: &Company, t: &TripRun) -> bool {
    c.own.iter().any(|x| x.duty.date == c.date && x.trip_of(t).is_some())
}

/// After the close of `closed`: its records go.
pub fn after_close(c: &mut Company, closed: &str) {
    c.own.retain(|x| x.duty.date.as_str() > closed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trip(dep: i32, empty: bool) -> PlannedTrip {
        PlannedTrip { name: "5 A-B".into(), line: if empty { String::new() } else { "5".into() }, dep, arr: dep + 30, stops: if empty { 0 } else { 10 }, empty, ..Default::default() }
    }

    fn run(dep: i32) -> TripRun {
        TripRun { time: 1_000, line: "5".into(), tour: "2".into(), departure: dep as f64 * 60.0, ..Default::default() }
    }

    fn company() -> (Company, DutyRef) {
        let mut c = super::super::found(&super::super::Founding { name: "Stadtbus".into(), date: "2024-03-04".into(), ..Default::default() }, "Luc");
        c.trips_seen = 0;
        let r = DutyRef { date: c.date.clone(), line: "Linie5".into(), tour: "2".into(), duty: 0 };
        let sheet = vec![trip(350, true), trip(360, false), trip(400, false), trip(440, false), trip(472, true)];
        begin(&mut c, &r, &sheet, "2", 0, 900);
        (c, r)
    }

    /// The game ends after two trips (it crashed): the duty is not driven, the AI drives it from
    /// its third trip on - and the trips he drove stay his.
    #[test]
    fn a_game_that_ends_halfway_leaves_the_rest_to_the_ai() {
        let (mut c, r) = company();
        assert!(note_trips(&mut c, &[run(360), run(400)]));
        live(&mut c, &r, 445);
        assert_eq!(ended(&mut c, &r, LeftWhy::Quit, 2_000, 450), Some(Ended::Ai { from: 3, driven: 2 }));
        let x = find(&c, &r).unwrap();
        assert!(!x.running && x.drove(1) && x.drove(2) && !x.drove(3));
        assert_eq!(x.ai[0].minute, 445, "the AI takes over when the game last said where he was");
        assert!(!ai_runs(&c, &r.date, "Linie5", "2", 0, 2) && ai_runs(&c, &r.date, "Linie5", "2", 0, 3));
        assert!(x.done(), "with the AI's part every trip is driven");
    }

    /// All of it driven: nothing for the AI.
    #[test]
    fn a_duty_driven_whole_is_done() {
        let (mut c, r) = company();
        note_trips(&mut c, &[run(360), run(400), run(440)]);
        assert_eq!(ended(&mut c, &r, LeftWhy::Quit, 2_000, 480), Some(Ended::Done));
        assert!(find(&c, &r).unwrap().ai.is_empty());
    }

    /// Taken back from the AI at the trip it drives then: the AI's part ends there, and a second
    /// end of the game hands it the rest again.
    #[test]
    fn taken_back_from_the_ai_and_left_again() {
        let (mut c, r) = company();
        note_trips(&mut c, &[run(360)]);
        ended(&mut c, &r, LeftWhy::Quit, 2_000, 380);
        // (at 07:25 the AI is on its trip of 07:20: under way)
        assert_eq!(back_at(&c, &r, 445), Some((3, true)));
        let sheet = find(&c, &r).unwrap().sheet.clone();
        begin(&mut c, &r, &sheet, "2", 3, 3_000);
        assert_eq!((find(&c, &r).unwrap().ai[0].from, find(&c, &r).unwrap().ai[0].until), (2, Some(3)));
        assert_eq!(ended(&mut c, &r, LeftWhy::Crashed, 4_000, 460), Some(Ended::Ai { from: 3, driven: 1 }));
        assert!(!ai_runs(&c, &r.date, "Linie5", "2", 0, 1) && ai_runs(&c, &r.date, "Linie5", "2", 0, 2) && ai_runs(&c, &r.date, "Linie5", "2", 0, 3));
        // (taken back at the AI's first trip: it drove none of it)
        let (mut c, r) = company();
        ended(&mut c, &r, LeftWhy::Quit, 2_000, 340);
        begin(&mut c, &r, &sheet, "2", 0, 3_000);
        assert!(find(&c, &r).unwrap().ai.is_empty());
    }

    /// A trip reported before the drive began, or of another tour, is not of it.
    #[test]
    fn only_the_duty_s_own_trips_count() {
        let (mut c, _) = company();
        let before = TripRun { time: 10, ..run(360) };
        let other = TripRun { tour: "3".into(), ..run(400) };
        assert!(!note_trips(&mut c, &[before, other]));
        assert!(claims_trip(&c, &run(440)) && !claims_trip(&c, &TripRun { line: "6".into(), ..run(440) }));
    }
}
