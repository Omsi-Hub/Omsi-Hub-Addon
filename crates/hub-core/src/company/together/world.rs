//! "Drive together" (Luc, 2026-10-20: "samen rijden in dezelfde wereld die de company maakt"):
//! players of a company session whose duties run at the same time may drive them in one world of
//! the company's instead of each alone in his own game. The first of them to start his duty
//! together opens it (his game hosts the session: `--lan-host`, its game code sent to the boss's
//! launcher as it comes: `Request::what` `world`); whoever starts a duty together after him joins
//! it (`--lan-join` with the code), on the world's clock.
//!
//! The players are bound to their duties' times: the world's clock runs on from the duty its
//! opener started (the company's clock when it opened, plus the time gone since), and a player
//! joins it for his duty from [`EARLY`] minutes before his duty's start until its end. Joining
//! more than [`LATE`] minutes after its start, he drives it from its next trip the world's clock
//! leaves him time for, and the AI drove the trips before (`host`; complaints, 2026-10-26: "the
//! players don't see each other" - a duty that began in the world could only be driven alone).
//! The world closes with its opener's game: what the others drive on of their duties the AI
//! finishes, as when a player leaves (`host::left_duty`).
//!
//! Driving alone is always there ("Solo"): a duty's start without `Request::world` is the
//! player's own game, as before.

use super::wire::DriveStart;
use serde::{Deserialize, Serialize};

/// How long before his duty's start a player may join the world for it (minutes on its clock).
pub const EARLY: i64 = 180;
/// How long after his duty's start he still joins it from its start (minutes on its clock); later
/// he joins it from its next trip (`host`).
pub const LATE: i64 = 5;
/// A world whose game code has not come this long after it was opened is given up (s): its game
/// did not start, or does not host.
pub const OPENING_FOR: u64 = 180;

/// Why a player cannot drive his duty in the world now (English: the keys of the translations).
pub const OPENING: &str = "%{name} is opening the company's world: try again in a moment.";
pub const TOO_EARLY: &str = "The company's world is at %{now}: you can join it for this duty from %{from}.";
pub const TOO_LATE: &str = "The company's world is at %{now}, past this duty's start (%{start}): drive it on your own.";
/// No trip of the duty is left to drive on the world's clock.
pub const PAST: &str = "The company's world is at %{now}: no trip of this duty is left to drive in it.";
pub const OTHER_DAY: &str = "The company's world drives %{date}: drive this duty on your own.";
pub const NOT_OPENER: &str = "Only the player who opened the company's world can say its code.";
/// What is told to everyone.
pub const OPENED: &str = "%{name} opened the company's world: drive along from your duty's start.";
pub const DRIVES_ALONG: &str = "%{name} drives along in the company's world.";
pub const CLOSED: &str = "The company's world closed: %{name}'s game ended.";
pub const JOINED_LATE: &str = "%{name} drives along in the company's world from %{time}: the AI drove the trips of his duty before.";

/// Every English text of this file (for the translations' check).
pub fn texts() -> Vec<&'static str> {
    vec![OPENING, TOO_EARLY, TOO_LATE, PAST, OTHER_DAY, NOT_OPENER, OPENED, DRIVES_ALONG, CLOSED, JOINED_LATE]
}

/// The company's world, while one is open.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct World {
    /// The game's session code (`OMSI-…`; empty: its game is starting, the code not here yet).
    pub code: String,
    /// Who opened it (a player's number; 0: the boss) and his name.
    pub host: u32,
    pub host_name: String,
    /// The company's day it drives.
    pub date: String,
    /// The moment of the company's clock it opened on (`clock::moment`: minutes), and when that
    /// was (Unix seconds).
    pub at: i64,
    pub opened: u64,
    /// Who drives in it (the opener first): a player's number (0: the boss) and his name.
    pub drivers: Vec<(u32, String)>,
}

impl World {
    /// The world's clock now (`clock::moment`'s minutes): from where it opened, at the game's
    /// pace (the real one).
    pub fn clock(&self, now: u64) -> i64 {
        self.at + (now.saturating_sub(self.opened) / 60) as i64
    }

    /// Its game has said its code: it can be joined.
    pub fn ready(&self) -> bool {
        !self.code.trim().is_empty()
    }

    /// Its game did not say its code in time (`OPENING_FOR`).
    pub fn stale(&self, now: u64) -> bool {
        !self.ready() && now.saturating_sub(self.opened) > OPENING_FOR
    }

    /// Player `who` drives in it.
    pub fn has(&self, who: u32) -> bool {
        self.drivers.iter().any(|d| d.0 == who)
    }

    /// Others drive in it besides its opener.
    fn has_others(&self) -> bool {
        self.drivers.iter().any(|d| d.0 != self.host)
    }
}

/// How a player's game starts on a duty driven together.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// It opens the world (hosts the game's session) at the duty's own start.
    Open,
    /// It joins the world by its code, at the world's clock (a minute of the day).
    Join { code: String, minute: i32 },
}

impl Place {
    /// As the command line's `lan` says it (`hub_core::Duty::lan`).
    pub fn lan(&self) -> String {
        match self {
            Place::Open => "host".into(),
            Place::Join { code, .. } => format!("join:{code}"),
        }
    }
}

/// A player's start on his duty as it is driven together: in the world (`lan`), on its clock
/// when it joins it.
pub fn apply(st: &mut DriveStart, place: &Place) {
    st.lan = place.lan();
    if let Place::Join { minute, .. } = place {
        st.time = *minute;
    }
}

/// Why not, with the words' arguments.
pub type Refused = (&'static str, Vec<(String, String)>);

fn hhmm(moment: i64) -> String {
    super::super::clock::hhmm(moment)
}

/// Where player `who` (`name`) drives the duty that starts at `start` and ends at `end`
/// (`clock::moment`'s minutes) on `date` together: he opens the world (none is open), or joins
/// it - when its clock is within his duty's window ([`EARLY`] before its start to its end; past
/// its start the host has him drive on from a later trip: `late`). The world is kept as it
/// becomes (`world`).
pub fn place(world: &mut Option<World>, who: u32, name: &str, date: &str, start: i64, end: i64, now: u64) -> Result<Place, Refused> {
    if world.as_ref().is_some_and(|w| w.stale(now) || (w.host == who && !w.has_others())) {
        *world = None;
    }
    let Some(w) = world.as_mut() else {
        *world = Some(World { host: who, host_name: name.to_string(), date: date.to_string(), at: start, opened: now, drivers: vec![(who, name.to_string())], ..Default::default() });
        return Ok(Place::Open);
    };
    if !w.ready() {
        return Err((OPENING, vec![("name".into(), w.host_name.clone())]));
    }
    if w.date != date {
        return Err((OTHER_DAY, vec![("date".into(), w.date.clone())]));
    }
    let clock = w.clock(now);
    if clock < start - EARLY {
        return Err((TOO_EARLY, vec![("now".into(), hhmm(clock)), ("from".into(), hhmm(start - EARLY))]));
    }
    if clock >= end {
        return Err((PAST, vec![("now".into(), hhmm(clock))]));
    }
    if !w.has(who) {
        w.drivers.push((who, name.to_string()));
    }
    Ok(Place::Join { code: w.code.clone(), minute: clock.rem_euclid(super::super::clock::DAY) as i32 })
}

/// The opener's game said its code (`code`): the world can be joined. Whether it was his to say.
pub fn opened(world: &mut Option<World>, who: u32, code: &str) -> Result<bool, &'static str> {
    let Some(w) = world.as_mut().filter(|w| w.host == who) else { return Err(NOT_OPENER) };
    let first = !w.ready();
    w.code = code.trim().to_string();
    Ok(first && w.ready())
}

/// A player joining the world on its clock `minute` (`Place::Join`) for a duty that started at
/// `start` (`clock::moment`'s minutes of `date`) joins it past its start: he drives on from a
/// later trip.
pub fn late(date: &str, start: i64, minute: i32) -> bool {
    super::super::clock::moment(date, minute as i64) > start + LATE
}

/// Player `who` drives no longer in the world: gone from it; its opener gone, the world is closed
/// (its name: who opened it). Whether it closed.
pub fn leave(world: &mut Option<World>, who: u32) -> Option<String> {
    let w = world.as_mut()?;
    if w.host == who {
        let name = w.host_name.clone();
        *world = None;
        return Some(name);
    }
    w.drivers.retain(|d| d.0 != who);
    None
}

/// Two duties of the day run at the same time (minutes of the day, `from` to `to`).
pub fn overlap(a: (i32, i32), b: (i32, i32)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: &str = "2026-10-20";

    fn m(h: i64, min: i64) -> i64 {
        super::super::super::clock::moment(DAY, h * 60 + min)
    }

    #[test]
    fn the_first_to_drive_together_opens_the_world_and_the_next_join_it_on_its_clock() {
        let mut w = None;
        let t0 = 1_000_000;
        assert_eq!(place(&mut w, 1, "Jan", DAY, m(6, 0), m(9, 0), t0), Ok(Place::Open));
        // (its code not here yet: the next waits)
        assert_eq!(place(&mut w, 2, "Piet", DAY, m(6, 10), m(9, 0), t0 + 30).unwrap_err().0, OPENING);
        assert_eq!(opened(&mut w, 2, "OMSI-X"), Err(NOT_OPENER));
        assert_eq!(opened(&mut w, 1, "OMSI-ABCD"), Ok(true));
        // (5 minutes on: the world at 06:05, Piet's duty at 06:10 - within its window)
        let p = place(&mut w, 2, "Piet", DAY, m(6, 10), m(9, 0), t0 + 300).unwrap();
        assert_eq!(p, Place::Join { code: "OMSI-ABCD".into(), minute: 6 * 60 + 5 });
        assert_eq!(p.lan(), "join:OMSI-ABCD");
        assert!(w.as_ref().unwrap().has(2));
    }

    #[test]
    fn a_player_is_bound_to_his_duty_s_times_in_the_world() {
        let mut w = None;
        let t0 = 2_000_000;
        place(&mut w, 0, "Boss", DAY, m(6, 0), m(14, 0), t0).unwrap();
        opened(&mut w, 0, "OMSI-B").unwrap();
        // (his duty at 10:00, the world at 06:00: too early - from 07:00)
        let (why, args) = place(&mut w, 3, "Ans", DAY, m(10, 0), m(15, 0), t0).unwrap_err();
        assert_eq!(why, TOO_EARLY);
        assert_eq!(args[1].1, "07:00");
        // (an hour on it may, and waits in the world for his duty)
        assert!(matches!(place(&mut w, 3, "Ans", DAY, m(10, 0), m(15, 0), t0 + 60 * 60), Ok(Place::Join { .. })));
        // (a duty that began at 05:50 joins on the world's clock, past its start: from a later trip)
        let Ok(Place::Join { minute, .. }) = place(&mut w, 4, "Kim", DAY, m(5, 50), m(9, 0), t0 + 40 * 60) else { panic!("Kim joins") };
        assert_eq!(minute, 6 * 60 + 40);
        assert!(late(DAY, m(5, 50), minute) && !late(DAY, m(6, 40), minute));
        // (one over by the world's clock: none of it is left to drive in it)
        assert_eq!(place(&mut w, 5, "Lou", DAY, m(5, 0), m(6, 30), t0 + 40 * 60).unwrap_err().0, PAST);
        // (another day's duty: alone)
        assert_eq!(place(&mut w, 4, "Kim", "2026-10-21", m(6, 0), m(9, 0), t0).unwrap_err().0, OTHER_DAY);
    }

    #[test]
    fn the_world_closes_with_its_opener_and_one_that_never_opened_is_given_up() {
        let mut w = None;
        place(&mut w, 1, "Jan", DAY, m(6, 0), m(9, 0), 10).unwrap();
        opened(&mut w, 1, "OMSI-J").unwrap();
        place(&mut w, 2, "Piet", DAY, m(6, 0), m(9, 0), 20).unwrap();
        assert_eq!(leave(&mut w, 2), None);
        assert!(w.is_some());
        assert_eq!(leave(&mut w, 1).as_deref(), Some("Jan"));
        assert!(w.is_none());
        // (a game that never said its code: the next opens a world of his own)
        place(&mut w, 1, "Jan", DAY, m(6, 0), m(9, 0), 10).unwrap();
        assert_eq!(place(&mut w, 2, "Piet", DAY, m(6, 0), m(9, 0), 10 + OPENING_FOR + 1), Ok(Place::Open));
        assert_eq!(w.as_ref().unwrap().host, 2);
    }

    #[test]
    fn duties_overlap_when_they_run_at_the_same_time() {
        assert!(overlap((360, 600), (540, 800)));
        assert!(!overlap((360, 540), (540, 800)));
    }
}
