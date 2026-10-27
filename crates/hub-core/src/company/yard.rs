//! The depot's yard laid out: where each building and block of spaces stands, as the player
//! puts them down (the 3D depot's build mode), on a grid of 2 m cells.
//!
//! The yard keeps the depot's levels as they are (`depot`): what is built is still a level an
//! area, and costs, upkeep and what the depot does still come from the levels. What the layout
//! adds is where: every block of twelve parking spaces, every workshop bay and every wash lane
//! is a module of its own, put down where the player wants it (a level more is a module more);
//! the diesel station and the offices are put down once and built up where they stand (a pump
//! more, a storey more); the charging points stand at the head of the spaces and are not put
//! anywhere.
//!
//! The rules (`fits`), so the buses can always get round (the moving buses of the yard to come):
//! a module stands on the company's own land, not on the lane from the gate, not on another
//! module, and keeps the apron in front of it free - the lane before the spaces, the forecourt
//! of the workshop's doors, the way into and out of the washing bay and the diesel station.
//! Aprons may share the ground (two rows of spaces facing one lane). The land grows by buying
//! the plots beside it (`buy_land`).
//!
//! A layout follows the levels (`reconcile`): a file of before has none and gets one, put down
//! as the depot is; a level built without a place, or set elsewhere, is put on the first free
//! spot; a level taken away takes its last module.

use super::depot::{self, Area, Site, Works};
use super::model::{BookingKind, Cents, Company};
use serde::{Deserialize, Serialize};

/// The grid: a cell is 2 m.
pub const CELL_M: f64 = 2.0;
/// The company's land at first: 96 m wide, 128 m deep (rows from the back to the street).
pub const LOT_COLS: i32 = 48;
pub const LOT_ROWS: i32 = 64;
/// A plot of land more, beside it (to the east): 40 m.
pub const LAND_COLS: i32 = 20;
pub const LAND_MAX: u32 = 2;
/// What the plots cost (Realistic, at founding): about €35 a m² of industrial land on the edge
/// of a town, the second one dearer.
const LAND_COST: [Cents; 2] = [180_000_00, 240_000_00];
/// The lane from the gate into the yard, kept free.
pub const GATE: Rect = Rect { x: 20, y: 54, w: 8, h: 10 };

/// A rectangle of cells.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.w > 0 && self.h > 0 && o.w > 0 && o.h > 0 && self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }

    pub fn inside(&self, o: &Rect) -> bool {
        self.x >= o.x && self.y >= o.y && self.x + self.w <= o.x + o.w && self.y + self.h <= o.y + o.h
    }
}

/// A module put down: its area, the cell of its corner (towards the back and the west) and how
/// it is turned (0: its front to the street, 1: to the east, 2: to the back, 3: to the west).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Placed {
    pub id: u32,
    pub area: Area,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub rot: u8,
}

/// An area's module: its size in cells turned to the street (across, deep), and its aprons in
/// cells (in front, behind). None: it is not put down (the charging points).
pub fn shape(area: Area) -> Option<(i32, i32, i32, i32)> {
    match area {
        // twelve spaces of 4 m by 20 m, a 10 m lane before them
        Area::Parking => Some((24, 10, 5, 0)),
        // a bay of 8 m by 28 m, its forecourt 12 m
        Area::Workshop => Some((4, 14, 6, 0)),
        // a lane through it, in and out
        Area::Wash => Some((6, 14, 4, 3)),
        Area::Fuel => Some((10, 10, 4, 2)),
        Area::Offices => Some((10, 12, 2, 0)),
        Area::Charging => None,
    }
}

/// The cells of a module at (x, y) turned `rot`.
pub fn rect_of(area: Area, x: i32, y: i32, rot: u8) -> Rect {
    let (w, h, _, _) = shape(area).unwrap_or((1, 1, 0, 0));
    if rot.is_multiple_of(2) {
        Rect { x, y, w, h }
    } else {
        Rect { x, y, w: h, h: w }
    }
}

/// Its aprons: in front of it and behind it, as it is turned.
pub fn aprons_of(area: Area, x: i32, y: i32, rot: u8) -> Vec<Rect> {
    let Some((_, _, front, back)) = shape(area) else { return Vec::new() };
    let r = rect_of(area, x, y, rot);
    let side = |dir: u8, d: i32| match dir % 4 {
        0 => Rect { x: r.x, y: r.y + r.h, w: r.w, h: d },
        1 => Rect { x: r.x + r.w, y: r.y, w: d, h: r.h },
        2 => Rect { x: r.x, y: r.y - d, w: r.w, h: d },
        _ => Rect { x: r.x - d, y: r.y, w: d, h: r.h },
    };
    [(rot, front), (rot + 2, back)].into_iter().filter(|(_, d)| *d > 0).map(|(dir, d)| side(dir, d)).collect()
}

impl Placed {
    pub fn rect(&self) -> Rect {
        rect_of(self.area, self.x, self.y, self.rot)
    }

    pub fn aprons(&self) -> Vec<Rect> {
        aprons_of(self.area, self.x, self.y, self.rot)
    }
}

/// The land the company has: its columns.
pub fn owned(s: &Site) -> Rect {
    Rect { x: 0, y: 0, w: LOT_COLS + LAND_COLS * s.land.min(LAND_MAX) as i32, h: LOT_ROWS }
}

/// The modules standing, and those being built where they will stand.
fn standing(s: &Site) -> impl Iterator<Item = &Placed> {
    s.layout.iter().chain(s.works.iter().filter_map(|w| w.at.as_ref()))
}

/// Whether a module of `area` fits at (x, y) turned `rot`, `ignore` the one being moved.
pub fn fits(s: &Site, area: Area, x: i32, y: i32, rot: u8, ignore: Option<u32>) -> Result<(), &'static str> {
    if shape(area).is_none() {
        return Err("This is not put down on the yard.");
    }
    let r = rect_of(area, x, y, rot);
    let aprons = aprons_of(area, x, y, rot);
    let land = owned(s);
    if !r.inside(&land) || aprons.iter().any(|a| !a.inside(&land)) {
        return Err("It does not fit on your land here: put it elsewhere or buy more land.");
    }
    if r.overlaps(&GATE) {
        return Err("It would block the way in from the gate.");
    }
    for o in standing(s).filter(|o| Some(o.id) != ignore) {
        if r.overlaps(&o.rect()) {
            return Err("Something stands there already.");
        }
        if o.aprons().iter().any(|a| a.overlaps(&r)) || aprons.iter().any(|a| a.overlaps(&o.rect())) {
            return Err("Keep the lanes in front of the buildings and spaces free.");
        }
    }
    Ok(())
}

/// The first free spot for a module of `area` (from the back, from the west): turned to the
/// street where it can be, else away from it, else across.
pub fn spot(s: &Site, area: Area) -> Option<(i32, i32, u8)> {
    let land = owned(s);
    for rot in [0, 2, 1, 3] {
        for y in 0..land.h {
            for x in 0..land.w {
                if fits(s, area, x, y, rot, None).is_ok() {
                    return Some((x, y, rot));
                }
            }
        }
    }
    None
}

/// How many modules of an area its level means.
fn wanted(s: &Site, a: Area) -> usize {
    match a {
        Area::Parking | Area::Workshop | Area::Wash => s.level(a) as usize,
        Area::Fuel | Area::Offices => (s.level(a) > 0) as usize,
        Area::Charging => 0,
    }
}

fn count(s: &Site, a: Area) -> usize {
    s.layout.iter().filter(|p| p.area == a).count()
}

fn next_id(s: &mut Site) -> u32 {
    s.placed_counter += 1;
    s.placed_counter
}

/// Bring the layout in line with the levels: what is too many goes (the last put down), what
/// is missing is put on the first free spot - the land grown for a depot of before that does not
/// fit on the land of now.
pub fn reconcile(s: &mut Site) {
    depot::migrate(s);
    for a in Area::ALL {
        while count(s, a) > wanted(s, a) {
            if let Some(k) = s.layout.iter().rposition(|p| p.area == a) {
                s.layout.remove(k);
            }
        }
    }
    for a in [Area::Offices, Area::Workshop, Area::Wash, Area::Fuel, Area::Parking] {
        while count(s, a) < wanted(s, a) {
            match spot(s, a) {
                Some((x, y, rot)) => {
                    let id = next_id(s);
                    s.layout.push(Placed { id, area: a, x, y, rot });
                }
                None if s.land < LAND_MAX => s.land += 1,
                None => break,
            }
        }
    }
}

/// Whether the next level of an area is put down somewhere (a module more) rather than built
/// where it stands.
pub fn needs_place(s: &Site, a: Area) -> bool {
    match a {
        Area::Parking | Area::Workshop | Area::Wash => true,
        Area::Fuel | Area::Offices => count(s, a) == 0 && !s.works.iter().any(|w| w.area == a && w.at.is_some()),
        Area::Charging => false,
    }
}

/// Building work done: the level, and the module where it was put down.
pub fn finish(s: &mut Site, w: &Works) {
    s.set_level(w.area, w.level);
    if let Some(at) = w.at {
        // (its id taken: the modules `reconcile` puts down get the next ones)
        s.placed_counter = s.placed_counter.max(at.id);
        if count(s, w.area) < wanted(s, w.area) && fits(s, at.area, at.x, at.y, at.rot, None).is_ok() {
            s.layout.push(at);
        }
    }
    reconcile(s);
}

/// Build the next level of an area as a module put down at (x, y) turned `rot`: paid, and
/// there at once (`depot::build` for one built where it stands).
pub fn build_at(c: &mut Company, area: Area, x: i32, y: i32, rot: u8) -> Result<(), &'static str> {
    reconcile(&mut c.site);
    if !needs_place(&c.site, area) {
        return depot::build(c, area);
    }
    fits(&c.site, area, x, y, rot % 4, None)?;
    let id = c.site.placed_counter + 1;
    depot::build_placed(c, area, Some(Placed { id, area, x, y, rot: rot % 4 }))?;
    c.site.placed_counter = id;
    Ok(())
}

/// Move a module (free, at once); not one being built.
pub fn move_module(c: &mut Company, id: u32, x: i32, y: i32, rot: u8) -> Result<(), &'static str> {
    reconcile(&mut c.site);
    let Some(k) = c.site.layout.iter().position(|p| p.id == id) else { return Err("That is not there any more.") };
    let area = c.site.layout[k].area;
    if c.site.works_on(area).is_some_and(|w| w.at.is_none()) {
        return Err("Building work is under way on it.");
    }
    fits(&c.site, area, x, y, rot % 4, Some(id))?;
    let p = &mut c.site.layout[k];
    (p.x, p.y, p.rot) = (x, y, rot % 4);
    Ok(())
}

/// Pull a module down: its level goes (the washing bay, the diesel station and the offices
/// wholly), nothing paid back; not while a bus is worked on in the bay or work is under way.
pub fn demolish(c: &mut Company, id: u32) -> Result<(), &'static str> {
    reconcile(&mut c.site);
    let Some(k) = c.site.layout.iter().position(|p| p.id == id) else { return Err("That is not there any more.") };
    let area = c.site.layout[k].area;
    if c.site.works_on(area).is_some() {
        return Err("Building work is under way on it.");
    }
    let level = c.site.level(area);
    // (one module of an area of several: a wash lane too - all of them went with one, `wanted`)
    let after = match area {
        Area::Parking | Area::Workshop | Area::Wash => level.saturating_sub(1),
        _ => 0,
    };
    if area == Area::Workshop && depot::bays_used(c, &c.date) > after as usize {
        return Err("A bus is being worked on in it.");
    }
    c.site.layout.remove(k);
    c.site.set_level(area, after);
    reconcile(&mut c.site);
    Ok(())
}

/// What the next plot of land costs the company now (None: it has all there is).
pub fn land_cost(c: &Company) -> Option<Cents> {
    let k = c.site.land as usize;
    let base = *LAND_COST.get(k)?;
    Some(((base as f64 * super::econ_uk::mode(c.game_mode).costs * c.price_index / 100.0).round() as Cents) * 100)
}

/// Buy the next plot of land: paid now, the company's at once.
pub fn buy_land(c: &mut Company) -> Result<(), &'static str> {
    let Some(cost) = land_cost(c) else { return Err("There is no more land to buy here.") };
    if !c.can_pay(cost) {
        return Err("Not enough cash.");
    }
    c.book(BookingKind::Construction, -cost, format!("Land for the depot {}", c.site.land + 1), false);
    c.site.land += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::model::GameMode;
    use super::super::{found, Founding};
    use super::*;

    fn company() -> Company {
        let mut c = found(&Founding { name: "Yard".into(), game_mode: GameMode::Normal, date: "2024-03-04".into(), ..Default::default() }, "Luc");
        c.cash += 10_000_000_00;
        c.progress.xp = super::super::levels::LEVEL_XP[1];
        c
    }

    fn clear(s: &Site) {
        let all: Vec<&Placed> = s.layout.iter().collect();
        for (i, a) in all.iter().enumerate() {
            assert!(a.rect().inside(&owned(s)), "{a:?} off the land");
            assert!(!a.rect().overlaps(&GATE), "{a:?} on the gate's lane");
            for b in &all[i + 1..] {
                assert!(!a.rect().overlaps(&b.rect()), "{a:?} on {b:?}");
                assert!(!a.aprons().iter().any(|x| x.overlaps(&b.rect())) && !b.aprons().iter().any(|x| x.overlaps(&a.rect())), "{a:?} and {b:?} block a lane");
            }
        }
    }

    #[test]
    fn a_depot_of_before_is_laid_out_as_it_is_built() {
        // the founding's rented yard: offices, a bay, the diesel station, a block of spaces
        let mut s: Site = serde_json::from_str("{\"parking\":1,\"workshop\":1,\"wash\":0,\"fuel\":1,\"charging\":0,\"offices\":1}").unwrap();
        assert!(s.layout.is_empty());
        reconcile(&mut s);
        let n = |a: Area| s.layout.iter().filter(|p| p.area == a).count();
        assert_eq!((n(Area::Parking), n(Area::Workshop), n(Area::Wash), n(Area::Fuel), n(Area::Offices)), (1, 1, 0, 1, 1));
        assert_eq!(s.land, 0);
        clear(&s);
        // (the same again: nothing moves)
        let before = s.layout.clone();
        reconcile(&mut s);
        assert_eq!(s.layout, before);
        // a depot at its top gets the land it needs
        let mut big = Site { parking: 8, workshop: 6, wash: 2, fuel: 3, charging: 10, offices: 3, ..Site::default() };
        reconcile(&mut big);
        assert_eq!(big.layout.iter().filter(|p| p.area == Area::Parking).count(), 8);
        assert_eq!(big.layout.iter().filter(|p| p.area == Area::Workshop).count(), 6);
        assert!(big.land >= 1);
        clear(&big);
        // a level taken away takes its module
        big.parking = 5;
        reconcile(&mut big);
        assert_eq!(big.layout.iter().filter(|p| p.area == Area::Parking).count(), 5);
    }

    #[test]
    fn a_module_is_put_down_where_the_player_wants_it_and_stands_there_at_once() {
        let mut c = company();
        reconcile(&mut c.site);
        let free = spot(&c.site, Area::Parking).unwrap();
        // not on another, not on the gate's lane, not off the land, not on a lane
        let first = c.site.layout[0];
        assert_eq!(build_at(&mut c, Area::Parking, first.x, first.y, 0), Err("Something stands there already."));
        assert_eq!(build_at(&mut c, Area::Parking, GATE.x - 4, GATE.y - 8, 0), Err("It would block the way in from the gate."));
        assert!(build_at(&mut c, Area::Parking, LOT_COLS - 4, 0, 0).is_err());
        let cash = c.cash;
        build_at(&mut c, Area::Parking, free.0, free.1, free.2).unwrap();
        assert!(c.cash < cash);
        assert_eq!(c.site.parking, 2, "built at once");
        assert_eq!(fits(&c.site, Area::Parking, free.0, free.1, free.2, None), Err("Something stands there already."));
        assert!(c.site.layout.iter().any(|p| p.area == Area::Parking && (p.x, p.y, p.rot) == free));
        clear(&c.site);
        // a wash lane is a module of its own, the next one too; the diesel station is put down
        // once, then built up where it stands (a pump more)
        assert!(needs_place(&c.site, Area::Wash));
        let (x, y, rot) = spot(&c.site, Area::Wash).unwrap();
        build_at(&mut c, Area::Wash, x, y, rot).unwrap();
        assert_eq!(c.site.wash, 1);
        assert!(needs_place(&c.site, Area::Wash));
        depot::build(&mut c, Area::Wash).unwrap();
        assert_eq!((c.site.wash, c.site.layout.iter().filter(|p| p.area == Area::Wash).count()), (2, 2));
        assert!(!needs_place(&c.site, Area::Fuel));
        depot::build(&mut c, Area::Fuel).unwrap();
        assert_eq!((c.site.fuel, c.site.layout.iter().filter(|p| p.area == Area::Fuel).count()), (2, 1));
    }

    #[test]
    fn a_level_built_without_a_place_finds_one() {
        let mut c = company();
        depot::build(&mut c, Area::Parking).unwrap();
        assert!(c.site.works.is_empty());
        assert_eq!(c.site.layout.iter().filter(|p| p.area == Area::Parking).count(), 2);
        clear(&c.site);
    }

    #[test]
    fn a_module_moves_free_and_comes_down_with_its_level() {
        let mut c = company();
        reconcile(&mut c.site);
        let block = *c.site.layout.iter().find(|p| p.area == Area::Parking).unwrap();
        let cash = c.cash;
        // turned away from the street, somewhere free
        let mut to = None;
        'find: for y in 0..LOT_ROWS {
            for x in 0..LOT_COLS {
                if fits(&c.site, Area::Parking, x, y, 2, Some(block.id)).is_ok() && (x, y) != (block.x, block.y) {
                    to = Some((x, y));
                    break 'find;
                }
            }
        }
        let (x, y) = to.unwrap();
        move_module(&mut c, block.id, x, y, 2).unwrap();
        assert_eq!(c.cash, cash);
        assert!(c.site.layout.iter().any(|p| p.id == block.id && (p.x, p.y, p.rot) == (x, y, 2)));
        clear(&c.site);
        demolish(&mut c, block.id).unwrap();
        assert_eq!(c.site.parking, 0);
        assert!(c.site.layout.iter().all(|p| p.area != Area::Parking));
        let offices = c.site.layout.iter().find(|p| p.area == Area::Offices).unwrap().id;
        demolish(&mut c, offices).unwrap();
        assert_eq!(c.site.offices, 0);
        assert!(needs_place(&c.site, Area::Offices));
    }

    #[test]
    fn more_land_is_bought_beside_the_yard() {
        let mut c = company();
        let w = owned(&c.site).w;
        let cost = land_cost(&c).unwrap();
        let cash = c.cash;
        buy_land(&mut c).unwrap();
        assert_eq!((owned(&c.site).w, c.cash), (w + LAND_COLS, cash - cost));
        buy_land(&mut c).unwrap();
        assert_eq!(buy_land(&mut c), Err("There is no more land to buy here."));
        assert!(land_cost(&c).is_none());
    }
}
