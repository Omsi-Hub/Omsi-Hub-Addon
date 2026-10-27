//! The depot: the company's yard and its buildings, and the workshop's jobs.
//!
//! As the Busbetrieb-Simulator's depot, it is built up area by area: parking spaces (they set
//! how many buses the company can keep), workshop bays (how many buses can be worked on at
//! once), wash lanes (how clean the fleet is, which the passengers and the authority see),
//! a diesel station's pumps and charging points (what is not filled or charged at the depot is
//! bought dearer outside), and offices with rest rooms (the staff's satisfaction). Every level
//! of an area costs money and days to build and something every month to keep. Beyond its
//! levels an area can have things added (`Upgrade`): lights and carports or a hall over the
//! parking, a paint booth in the workshop, a gantry over the wash lanes, a bulk tank at the
//! diesel station, a canteen in the offices, solar panels for the chargers - each with what it
//! costs, keeps and does.
//!
//! A company founded now starts with its land only (`Founding::empty_depot`) and builds all of
//! it, with a building credit for a first depot (`start_empty`: what a block of spaces, a
//! workshop bay, a diesel pump and the offices cost at its game mode) spent before its cash.
//! Until it has a workshop its jobs go to a garage outside, dearer and a day longer
//! (`OUTSIDE_GARAGE`); until it has spaces its buses stand in the street, and so on. A company
//! of before keeps the yard it rented at founding (`Site::rented`: those first levels cost it no
//! upkeep, the overhead per bus already pays for them, `economy::bus_month`).
//!
//! The workshop takes jobs - a service, a repair, an overhaul - one bus a bay; a job waits
//! until a bay is free, and the bus is out of service only while it is worked on. The nightly
//! part (`after_day`) is called after the day's close: building work finished, jobs started,
//! the fleet's cleanliness, the fuel and power bought outside, the staff rooms.
//!
//! Figures (Realistic, net, mid-2020s Germany, rounded): a parking space on an asphalt yard
//! with lights and fencing about €7,500 (twelve a level); a workshop bay with a pit and lifting
//! gear in an existing hall €250,000; a manual wash bay €120,000, a wash gantry €350,000; a
//! diesel station with a tank and a pump €150,000, another pump €40,000; a 150 kW depot charger
//! with its share of the grid connection €60,000; a staff building €180,000-320,000. Diesel at a
//! public station costs operators about 12 % more than their own tank; public fast charging
//! about 45 % more than depot charging.

use super::dates;
use super::day::{DayReport, Note};
use super::economy;
use super::market;
use super::model::{BookingKind, BusSize, Cents, Company, Drive, GameMode};
use serde::{Deserialize, Serialize};

/// The parts of the depot that are built.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Parking,
    Workshop,
    Wash,
    Fuel,
    Charging,
    Offices,
}

impl Area {
    pub const ALL: [Area; 6] = [Area::Parking, Area::Workshop, Area::Wash, Area::Fuel, Area::Charging, Area::Offices];

    pub fn label(self) -> &'static str {
        match self {
            Area::Parking => "Parking spaces",
            Area::Workshop => "Workshop bays",
            Area::Wash => "Washing bay",
            Area::Fuel => "Diesel station",
            Area::Charging => "Charging points",
            Area::Offices => "Offices and rest rooms",
        }
    }

    /// Its name in the phone's JSON and in a saved order.
    pub fn key(self) -> &'static str {
        match self {
            Area::Parking => "parking",
            Area::Workshop => "workshop",
            Area::Wash => "wash",
            Area::Fuel => "fuel",
            Area::Charging => "charging",
            Area::Offices => "offices",
        }
    }

    pub fn from_key(s: &str) -> Option<Area> {
        Area::ALL.into_iter().find(|a| a.key() == s)
    }

    /// The launcher's icon for it.
    pub fn icon(self) -> &'static str {
        match self {
            Area::Parking => "local_parking",
            Area::Workshop => "construction",
            Area::Wash => "water_drop",
            Area::Fuel => "inventory_2",
            Area::Charging => "bolt",
            Area::Offices => "groups",
        }
    }

    /// The highest level it can be built to.
    pub fn max(self) -> u32 {
        match self {
            Area::Parking => 8,
            Area::Workshop => 6,
            Area::Wash => 4,
            Area::Fuel => 6,
            Area::Charging => 10,
            Area::Offices => 3,
        }
    }

    /// The level the yard rented at founding had (a company of before: `Site::rented`; one
    /// founded now has its land only, `Site::empty`).
    pub fn start(self) -> u32 {
        match self {
            Area::Parking | Area::Workshop | Area::Fuel | Area::Offices => 1,
            Area::Wash | Area::Charging => 0,
        }
    }
}

/// Parking spaces a level of the yard has.
pub const SPACES_PER_LEVEL: u32 = 12;
/// Buses that can stand on rented spaces in the street when the yard is full, and what one
/// such space costs a day.
pub const OUTSIDE_MAX: usize = 6;
pub const OUTSIDE_PER_DAY: Cents = 30_00;
/// Buses a diesel pump fills a night, and a charging point charges.
pub const BUSES_PER_PUMP: usize = 15;
pub const BUSES_PER_CHARGER: usize = 2;
/// Buses a wash lane washes a night: by hand, and with a gantry's brushes.
pub const WASHES_BY_HAND: usize = 10;
pub const WASHES_GANTRY: usize = 40;
/// A garage outside, while the company has no workshop of its own: dearer by this, a day longer.
pub const OUTSIDE_GARAGE: f64 = 1.3;
/// What fuel and power cost more when bought outside the depot.
pub const DIESEL_OUTSIDE: f64 = 0.12;
pub const POWER_OUTSIDE: f64 = 0.45;

/// One level of an area: what building it costs (Realistic, at founding), what it costs a
/// month to keep, and how many days the work takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub cost: Cents,
    pub upkeep: Cents,
    pub days: i64,
}

/// The step that brings an area to `level` (1 = its first): the UK's build costs
/// (`econ_uk::DEPOT_*`, to the hundred euros) and a month's upkeep of them (`DEPOT_UPKEEP`, a
/// workshop's twice that).
pub fn step(area: Area, level: u32) -> Step {
    use super::econ_uk as uk;
    let (pence, upkeep, days) = match (area, level) {
        (Area::Parking, _) => (uk::DEPOT_PARKING, 1.0, 14),
        (Area::Workshop, _) => (uk::DEPOT_WORKSHOP, 2.0, 30),
        // (a lane with its bay; the gantry is an upgrade)
        (Area::Wash, _) => (uk::DEPOT_WASH.0, 1.0, 21),
        (Area::Fuel, 1) => (uk::DEPOT_FUEL.0, 1.0, 21),
        (Area::Fuel, _) => (uk::DEPOT_FUEL.1, 1.0, 7),
        (Area::Charging, _) => (uk::DEPOT_CHARGER, 1.0, 10),
        (Area::Offices, 1) => (uk::DEPOT_OFFICES.0, 1.0, 45),
        (Area::Offices, 2) => (uk::DEPOT_OFFICES.1, 1.0, 45),
        (Area::Offices, _) => (uk::DEPOT_OFFICES.2, 1.0, 50),
    };
    let cents = uk::gbp(pence);
    let to = |x: f64, step: f64| ((x / step).round() * step) as Cents;
    Step { cost: to(cents, 100_00.0), upkeep: to(cents * uk::DEPOT_UPKEEP * upkeep, 10_00.0), days }
}

/// Building costs against the UK's, by game mode: its cost factor (Realistic 1.0, Normal 0.9,
/// Casual 0.75).
fn cost_factor(d: GameMode) -> f64 {
    super::econ_uk::mode(d).costs
}

/// What can be added to an area beyond its levels, once the area has its first.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Upgrade {
    /// Floodlights and cameras over the parking: the staff feel safer, less is daubed on.
    Lights,
    /// Roofs over the spaces: the buses stand dry.
    Carports,
    /// A hall over the spaces (a parking garage of concrete): cleaner still.
    Hall,
    /// The workshop's paint booth: a livery painted in house, cheaper.
    PaintBooth,
    /// An inspection lane with its brake tester: repairs cheaper.
    InspectionLane,
    /// A store of tyres: repairs and overhauls a little cheaper.
    TyreStore,
    /// Brushes on a gantry over every wash lane: four times the buses a night.
    Gantry,
    /// Interior cleaning: a wash does more.
    Interior,
    /// An underbody wash: the buses stay clean longer.
    Underbody,
    /// The wash water recycled: the lanes cost less to keep.
    Recycling,
    /// A bulk tank: diesel bought in bulk, cheaper at the depot.
    BulkTank,
    /// An AdBlue dispenser at the pumps: no detours to fill it.
    AdBlue,
    /// A canteen: the staff's satisfaction.
    Canteen,
    /// A control room: the service watched, the passengers see it.
    ControlRoom,
    /// A training room: the staff's satisfaction.
    TrainingRoom,
    /// Solar panels on the roofs: the chargers' power cheaper.
    Solar,
}

impl Upgrade {
    pub const ALL: [Upgrade; 16] = [
        Upgrade::Lights,
        Upgrade::Carports,
        Upgrade::Hall,
        Upgrade::PaintBooth,
        Upgrade::InspectionLane,
        Upgrade::TyreStore,
        Upgrade::Gantry,
        Upgrade::Interior,
        Upgrade::Underbody,
        Upgrade::Recycling,
        Upgrade::BulkTank,
        Upgrade::AdBlue,
        Upgrade::Canteen,
        Upgrade::ControlRoom,
        Upgrade::TrainingRoom,
        Upgrade::Solar,
    ];

    /// The area it is added to.
    pub fn area(self) -> Area {
        use Upgrade::*;
        match self {
            Lights | Carports | Hall => Area::Parking,
            PaintBooth | InspectionLane | TyreStore => Area::Workshop,
            Gantry | Interior | Underbody | Recycling => Area::Wash,
            BulkTank | AdBlue => Area::Fuel,
            Canteen | ControlRoom | TrainingRoom => Area::Offices,
            Solar => Area::Charging,
        }
    }

    pub fn key(self) -> &'static str {
        use Upgrade::*;
        match self {
            Lights => "lights",
            Carports => "carports",
            Hall => "hall",
            PaintBooth => "paint_booth",
            InspectionLane => "inspection_lane",
            TyreStore => "tyre_store",
            Gantry => "gantry",
            Interior => "interior",
            Underbody => "underbody",
            // (its name, as the yard draws it - its icon's name, the tanks were never drawn)
            Recycling => "recycling",
            BulkTank => "bulk_tank",
            AdBlue => "adblue",
            Canteen => "canteen",
            ControlRoom => "control_room",
            TrainingRoom => "training_room",
            Solar => "solar",
        }
    }

    pub fn from_key(s: &str) -> Option<Upgrade> {
        Upgrade::ALL.into_iter().find(|u| u.key() == s)
    }

    pub fn label(self) -> &'static str {
        use Upgrade::*;
        match self {
            Lights => "Lights and cameras",
            Carports => "Carports",
            Hall => "Parking hall",
            PaintBooth => "Paint booth",
            InspectionLane => "Inspection lane",
            TyreStore => "Tyre store",
            Gantry => "Wash gantries",
            Interior => "Interior cleaning",
            Underbody => "Underbody wash",
            Recycling => "Water recycling",
            BulkTank => "Bulk tank",
            AdBlue => "AdBlue dispenser",
            Canteen => "Canteen",
            ControlRoom => "Control room",
            TrainingRoom => "Training room",
            Solar => "Solar roof",
        }
    }

    /// What it does, in words.
    pub fn what(self) -> &'static str {
        use Upgrade::*;
        match self {
            Lights => "The staff feel safer, the buses are daubed on less",
            Carports => "The buses stand dry: they stay clean longer",
            Hall => "The buses stand indoors: they stay clean much longer",
            PaintBooth => "Liveries painted in house: 30 % cheaper",
            InspectionLane => "Repairs found sooner: 15 % cheaper",
            TyreStore => "Repairs and overhauls 7 % cheaper",
            Gantry => "Every lane washes four times the buses a night",
            Interior => "A wash cleans inside too: it does more",
            Underbody => "The buses stay clean longer",
            Recycling => "The wash lanes cost 40 % less to keep",
            BulkTank => "Diesel bought in bulk: 3 % cheaper at the depot",
            AdBlue => "No detours for AdBlue: 1 % cheaper at the depot",
            Canteen => "The staff are more satisfied",
            ControlRoom => "The service watched: the passengers notice",
            TrainingRoom => "The staff are a little more satisfied",
            Solar => "Power for the chargers 8 % cheaper",
        }
    }

    /// The launcher's icon for it.
    pub fn icon(self) -> &'static str {
        use Upgrade::*;
        match self {
            Lights => "videocam",
            Carports => "home",
            Hall => "garage",
            PaintBooth => "livery_brush",
            InspectionLane => "speed",
            TyreStore => "build",
            Gantry => "local_car_wash",
            Interior => "airline_seat_recline_normal",
            Underbody => "rainy",
            Recycling => "autorenew",
            BulkTank => "local_gas_station",
            AdBlue => "water_drop",
            Canteen => "group",
            ControlRoom => "monitor",
            TrainingRoom => "school",
            Solar => "wb_sunny",
        }
    }

    /// What building it costs (the UK's, in pence, at founding), its upkeep as a level's is
    /// reckoned (`DEPOT_UPKEEP` of it), and its days of work. A parking's carports and hall and a
    /// wash's gantries cost this a block or a lane (`upgrade_cost`).
    fn price(self) -> (f64, i64) {
        use Upgrade::*;
        match self {
            Lights => (2_500_000.0, 10),
            Carports => (9_000_000.0, 20),
            Hall => (45_000_000.0, 60),
            PaintBooth => (15_000_000.0, 30),
            InspectionLane => (8_000_000.0, 21),
            TyreStore => (4_000_000.0, 10),
            Gantry => (19_000_000.0, 21),
            Interior => (4_500_000.0, 14),
            Underbody => (3_500_000.0, 14),
            Recycling => (6_000_000.0, 21),
            BulkTank => (7_000_000.0, 21),
            AdBlue => (1_200_000.0, 5),
            Canteen => (9_000_000.0, 30),
            ControlRoom => (12_000_000.0, 30),
            TrainingRoom => (6_000_000.0, 21),
            Solar => (14_000_000.0, 21),
        }
    }

    /// Whether it is priced a block of spaces (or a wash lane).
    fn per_unit(self) -> bool {
        matches!(self, Upgrade::Carports | Upgrade::Hall | Upgrade::Gantry)
    }
}

/// How the depot's buildings look (the 3D depot's): the walls', roofs', trims' and doors' own
/// colours (`#rrggbb`; None: the depot's own) and what the walls are made of.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Look {
    #[serde(default)]
    pub wall: Option<String>,
    #[serde(default)]
    pub roof: Option<String>,
    #[serde(default)]
    pub trim: Option<String>,
    #[serde(default)]
    pub door: Option<String>,
    #[serde(default)]
    pub finish: Finish,
}

/// What the depot's walls are made of.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Finish {
    #[default]
    Cladding,
    Brick,
    Plaster,
}

impl Finish {
    pub const ALL: [Finish; 3] = [Finish::Cladding, Finish::Brick, Finish::Plaster];

    pub fn key(self) -> &'static str {
        match self {
            Finish::Cladding => "cladding",
            Finish::Brick => "brick",
            Finish::Plaster => "plaster",
        }
    }

    pub fn from_key(s: &str) -> Option<Finish> {
        Finish::ALL.into_iter().find(|f| f.key() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Finish::Cladding => "Steel cladding",
            Finish::Brick => "Brick",
            Finish::Plaster => "Plaster",
        }
    }
}

/// A colour as the look keeps it: `#rrggbb` in lower case, else None.
pub fn colour(s: &str) -> Option<String> {
    let h = s.trim().trim_start_matches('#');
    (h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit())).then(|| format!("#{}", h.to_ascii_lowercase()))
}

/// The depot's look, as the player chose it (free, at once).
pub fn set_look(c: &mut Company, look: Look) {
    let own = |x: Option<String>| x.as_deref().and_then(colour);
    c.site.look = Look { wall: own(look.wall), roof: own(look.roof), trim: own(look.trim), door: own(look.door), finish: look.finish };
}

/// Building work under way: the level it brings and its last day; a module more of the yard
/// where it is put down (`yard`; None: built up where it stands); or what is added to the area
/// (`upgrade`: its level stays as it is).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Works {
    pub area: Area,
    pub level: u32,
    pub until: String,
    #[serde(default)]
    pub at: Option<super::yard::Placed>,
    #[serde(default)]
    pub upgrade: Option<Upgrade>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    /// The regular service (its work is paid with the maintenance per kilometre).
    Service,
    /// Body and running gear put right: the condition a service brings.
    Repair,
    /// A general overhaul: nearly as new.
    Overhaul,
    /// Painted in one of the company's liveries (`livery::paint`; not one of `ALL`, the jobs
    /// ordered as they are).
    Paint,
}

impl JobKind {
    pub const ALL: [JobKind; 3] = [JobKind::Service, JobKind::Repair, JobKind::Overhaul];

    pub fn label(self) -> &'static str {
        match self {
            JobKind::Service => "Service",
            JobKind::Repair => "Repair",
            JobKind::Overhaul => "Overhaul",
            JobKind::Paint => "Painting",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            JobKind::Service => "service",
            JobKind::Repair => "repair",
            JobKind::Overhaul => "overhaul",
            JobKind::Paint => "paint",
        }
    }

    pub fn from_key(s: &str) -> Option<JobKind> {
        JobKind::ALL.into_iter().find(|k| k.key() == s)
    }

    /// Workshop days it takes.
    pub fn days(self) -> i64 {
        match self {
            JobKind::Service | JobKind::Paint => 1,
            JobKind::Repair => 2,
            JobKind::Overhaul => 5,
        }
    }
}

/// A job of the workshop: waiting for a bay (`started` None) or in hand until `until`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Job {
    pub id: u32,
    pub vehicle: u32,
    pub kind: JobKind,
    pub ordered: String,
    #[serde(default)]
    pub started: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
    /// What it costs (booked when it starts).
    pub cost: Cents,
    /// The company's livery a painting puts on the bus.
    #[serde(default)]
    pub livery: Option<String>,
}

/// The depot as it is built.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Site {
    pub parking: u32,
    pub workshop: u32,
    pub wash: u32,
    pub fuel: u32,
    pub charging: u32,
    pub offices: u32,
    #[serde(default)]
    pub works: Vec<Works>,
    #[serde(default)]
    pub jobs: Vec<Job>,
    /// How clean the fleet is, 0 to 100.
    #[serde(default = "clean_start")]
    pub clean: f64,
    #[serde(default)]
    pub job_counter: u32,
    /// Where the modules stand (`yard`; empty in a file of before: laid out as it is built).
    #[serde(default)]
    pub layout: Vec<super::yard::Placed>,
    /// The plots of land bought beside the yard.
    #[serde(default)]
    pub land: u32,
    #[serde(default)]
    pub placed_counter: u32,
    /// What has been added beyond the levels.
    #[serde(default)]
    pub upgrades: Vec<Upgrade>,
    /// What is left of the building credit a company founded with its land only was given
    /// (`start_empty`), spent on building before its cash.
    #[serde(default)]
    pub credit: Cents,
    /// Whether its first levels are the yard rented at founding (a company of before: they cost
    /// no upkeep); false for one that built them all.
    #[serde(default = "yes")]
    pub rented: bool,
    /// How its buildings look.
    #[serde(default)]
    pub look: Look,
    /// The model the file was written with: 0 before the wash lanes (one washing bay, its second
    /// level the gantry), 1 now (`migrate`).
    #[serde(default)]
    pub generation: u32,
}

fn clean_start() -> f64 {
    80.0
}

fn yes() -> bool {
    true
}

/// The depot model of now (`Site::generation`).
pub const GENERATION: u32 = 1;

impl Default for Site {
    /// The yard rented at founding (a company of before, and the tests'); a company founded now
    /// has its land only (`Site::empty`).
    fn default() -> Self {
        Site {
            parking: Area::Parking.start(),
            workshop: Area::Workshop.start(),
            wash: Area::Wash.start(),
            fuel: Area::Fuel.start(),
            charging: Area::Charging.start(),
            offices: Area::Offices.start(),
            works: Vec::new(),
            jobs: Vec::new(),
            clean: clean_start(),
            job_counter: 0,
            layout: Vec::new(),
            land: 0,
            placed_counter: 0,
            upgrades: Vec::new(),
            credit: 0,
            rented: true,
            look: Look::default(),
            generation: GENERATION,
        }
    }
}

/// A file of before brought up to the depot of now: its washing bay's second level, the
/// gantry, is a lane with the gantries added.
pub fn migrate(s: &mut Site) {
    if s.generation < 1 {
        if s.wash >= 2 {
            s.wash = 1;
            if !s.upgrades.contains(&Upgrade::Gantry) {
                s.upgrades.push(Upgrade::Gantry);
            }
        }
        for w in s.works.iter_mut().filter(|w| w.area == Area::Wash && w.level >= 2 && w.upgrade.is_none()) {
            w.level = 1;
            w.upgrade = Some(Upgrade::Gantry);
        }
    }
    s.generation = GENERATION;
    // (building is done at once now: work a file of before still had under way is done)
    for w in std::mem::take(&mut s.works) {
        complete(s, &w);
    }
}

/// Building work done: what was added to the area, or its level and the module put down.
pub(super) fn complete(s: &mut Site, w: &Works) {
    match w.upgrade {
        Some(u) => {
            if !s.has(u) {
                s.upgrades.push(u);
            }
        }
        None => super::yard::finish(s, w),
    }
}

impl Site {
    /// The land only: nothing built, the company builds it all.
    pub fn empty() -> Self {
        Site { parking: 0, workshop: 0, wash: 0, fuel: 0, charging: 0, offices: 0, rented: false, ..Site::default() }
    }

    /// The level an area's upkeep is reckoned from (the rented yard's are paid for already).
    pub fn base(&self, a: Area) -> u32 {
        if self.rented {
            a.start()
        } else {
            0
        }
    }

    pub fn has(&self, u: Upgrade) -> bool {
        self.upgrades.contains(&u)
    }

    pub fn level(&self, a: Area) -> u32 {
        match a {
            Area::Parking => self.parking,
            Area::Workshop => self.workshop,
            Area::Wash => self.wash,
            Area::Fuel => self.fuel,
            Area::Charging => self.charging,
            Area::Offices => self.offices,
        }
    }

    pub fn set_level(&mut self, a: Area, v: u32) {
        let v = v.min(a.max());
        match a {
            Area::Parking => self.parking = v,
            Area::Workshop => self.workshop = v,
            Area::Wash => self.wash = v,
            Area::Fuel => self.fuel = v,
            Area::Charging => self.charging = v,
            Area::Offices => self.offices = v,
        }
    }

    pub fn spaces(&self) -> usize {
        (self.parking * SPACES_PER_LEVEL) as usize
    }

    pub fn bays(&self) -> usize {
        self.workshop as usize
    }

    /// Buses the wash lanes wash a night: ten a lane by hand, forty with the gantries.
    pub fn washes(&self) -> usize {
        self.wash as usize * if self.has(Upgrade::Gantry) { WASHES_GANTRY } else { WASHES_BY_HAND }
    }

    pub fn diesel_served(&self) -> usize {
        self.fuel as usize * BUSES_PER_PUMP
    }

    pub fn electric_served(&self) -> usize {
        self.charging as usize * BUSES_PER_CHARGER
    }

    /// The staff the offices and rest rooms have room for.
    pub fn staff_room(&self) -> usize {
        match self.offices {
            0 => 0,
            1 => 20,
            2 => 50,
            _ => 120,
        }
    }

    /// The building work on an area, if any (a level or an upgrade: one at a time).
    pub fn works_on(&self, a: Area) -> Option<&Works> {
        self.works.iter().find(|w| w.area == a)
    }

    /// The building work adding `u`, if any.
    pub fn works_for(&self, u: Upgrade) -> Option<&Works> {
        self.works.iter().find(|w| w.upgrade == Some(u))
    }

    /// The job of a bus, waiting or in hand.
    pub fn job_of(&self, vehicle: u32) -> Option<&Job> {
        self.jobs.iter().find(|j| j.vehicle == vehicle)
    }
}

/// Whether the company may build an area now: the workshop and the charging points open with
/// the company's levels (`levels::Feature`), the rest at once.
pub fn area_allowed(c: &Company, area: Area) -> bool {
    use super::levels::{unlocked, Feature};
    match area {
        Area::Workshop => unlocked(c, Feature::Workshop),
        Area::Charging => unlocked(c, Feature::ElectricBuses),
        _ => true,
    }
}

/// What building the next level of an area costs the company now (None: it is at its top).
pub fn next_cost(c: &Company, area: Area) -> Option<(Cents, i64)> {
    let next = c.site.level(area) + 1;
    if next > area.max() {
        return None;
    }
    let s = step(area, next);
    // (with what the area's upgrades priced by the lane or block need for one more: a wash lane
    // built after the gantries got its gantry for nothing, a parking block its carport)
    let more: Cents = c.site.upgrades.iter().filter(|u| u.per_unit() && u.area() == area).map(|u| unit_cost(*u)).sum();
    let cost = (((s.cost + more) as f64 * cost_factor(c.game_mode) * c.price_index / 100.0).round() as Cents) * 100;
    let days = if c.game_mode == GameMode::Casual { (s.days + 1) / 2 } else { s.days };
    Some((cost, days))
}

/// What the depot's buildings cost a month beyond the rented yard (the levels built since, and
/// what was added: the water recycled, the wash lanes' a little less).
pub fn upkeep_month(c: &Company) -> Cents {
    let mut sum = 0;
    for a in Area::ALL {
        let mut area = 0;
        for lv in (c.site.base(a) + 1)..=c.site.level(a) {
            area += step(a, lv).upkeep;
        }
        if a == Area::Wash && c.site.has(Upgrade::Recycling) {
            area = (area as f64 * 0.6).round() as Cents;
        }
        sum += area;
    }
    for u in &c.site.upgrades {
        sum += upgrade_step(&c.site, *u).upkeep;
    }
    (sum as f64 * c.price_index).round() as Cents
}

/// What an upgrade priced by the lane or block costs for one of them.
fn unit_cost(u: Upgrade) -> Cents {
    let cents = super::econ_uk::gbp(u.price().0);
    ((cents / 100_00.0).round() * 100_00.0) as Cents
}

/// An upgrade's price as a level's (`step`): a block's or a lane's for those priced by them, as
/// many as the area has.
fn upgrade_step(s: &Site, u: Upgrade) -> Step {
    use super::econ_uk as uk;
    let (pence, days) = u.price();
    let n = if u.per_unit() { s.level(u.area()).max(1) as f64 } else { 1.0 };
    let cents = uk::gbp(pence * n);
    let to = |x: f64, step: f64| ((x / step).round() * step) as Cents;
    Step { cost: to(cents, 100_00.0), upkeep: to(cents * uk::DEPOT_UPKEEP, 10_00.0), days }
}

/// What adding `u` costs the company now, and its days (None: added already, or its area is
/// not built yet).
pub fn upgrade_cost(c: &Company, u: Upgrade) -> Option<(Cents, i64)> {
    if c.site.has(u) || c.site.level(u.area()) == 0 {
        return None;
    }
    let s = upgrade_step(&c.site, u);
    let cost = ((s.cost as f64 * cost_factor(c.game_mode) * c.price_index / 100.0).round() as Cents) * 100;
    let days = if c.game_mode == GameMode::Casual { (s.days + 1) / 2 } else { s.days };
    Some((cost, days))
}

/// Pay for building work: from what is left of the building credit first, then the cash.
fn pay_building(c: &mut Company, cost: Cents, what: String) -> Result<(), &'static str> {
    let credit = c.site.credit.clamp(0, cost);
    if cost > credit && !c.can_pay(cost - credit) {
        return Err("Not enough cash.");
    }
    c.site.credit -= credit;
    if cost > credit {
        c.book(BookingKind::Construction, -(cost - credit), what, false);
    }
    Ok(())
}

/// What the cash must hold for building work of `cost` (the building credit taken off).
pub fn cash_needed(c: &Company, cost: Cents) -> Cents {
    (cost - c.site.credit.max(0)).max(0)
}

/// Add `u` to its area: paid, and there at once.
pub fn build_upgrade(c: &mut Company, u: Upgrade) -> Result<(), &'static str> {
    migrate(&mut c.site);
    if c.site.has(u) {
        return Err("This is there already.");
    }
    if c.site.level(u.area()) == 0 {
        return Err("Build the area first.");
    }
    if !area_allowed(c, u.area()) {
        return Err("Your company cannot build this yet.");
    }
    if c.site.works_on(u.area()).is_some() {
        return Err("This is being built already.");
    }
    let Some((cost, _)) = upgrade_cost(c, u) else { return Err("This is there already.") };
    pay_building(c, cost, u.label().to_string())?;
    let level = c.site.level(u.area());
    complete(&mut c.site, &Works { area: u.area(), level, until: c.date.clone(), at: None, upgrade: Some(u) });
    Ok(())
}

/// A company founded with its land only (`Founding::empty_depot`): nothing built, and a
/// building credit for its first depot - a block of spaces, a workshop bay, a diesel pump and
/// the offices, at its game mode's prices.
pub fn start_empty(c: &mut Company) {
    c.site = Site::empty();
    c.site.credit = [Area::Parking, Area::Workshop, Area::Fuel, Area::Offices].iter().filter_map(|a| next_cost(c, *a)).map(|(cost, _)| cost).sum();
}

/// Build the next level of an area: paid, and in use at once. A module more of
/// the yard goes on its first free spot (`yard::build_at` puts it where the player wants it).
pub fn build(c: &mut Company, area: Area) -> Result<(), &'static str> {
    super::yard::reconcile(&mut c.site);
    if !super::yard::needs_place(&c.site, area) {
        return build_placed(c, area, None);
    }
    let Some((x, y, rot)) = super::yard::spot(&c.site, area) else { return Err("There is no room for it on the yard: buy more land.") };
    let id = c.site.placed_counter + 1;
    build_placed(c, area, Some(super::yard::Placed { id, area, x, y, rot }))?;
    c.site.placed_counter = id;
    Ok(())
}

/// `build`, the module put down at `at` (None: built up where it stands).
pub(super) fn build_placed(c: &mut Company, area: Area, at: Option<super::yard::Placed>) -> Result<(), &'static str> {
    if !area_allowed(c, area) {
        return Err("Your company cannot build this yet.");
    }
    if c.site.works_on(area).is_some() {
        return Err("This is being built already.");
    }
    let Some((cost, _)) = next_cost(c, area) else { return Err("This is as big as it gets.") };
    let level = c.site.level(area) + 1;
    pay_building(c, cost, format!("{} {}", area.label(), level))?;
    complete(&mut c.site, &Works { area, level, until: c.date.clone(), at, upgrade: None });
    Ok(())
}

/// The buses the company holds today.
fn held(c: &Company) -> usize {
    c.fleet.iter().filter(|v| v.held_on(&c.date)).count()
}

/// The places the depot has for buses: the yard's spaces, those of the halls the
/// company's levels open (`levels::extra_places`) and those its research added
/// (`research::extra_places`: the premises, the external depots).
pub fn places(c: &Company) -> usize {
    c.site.spaces() + super::levels::extra_places(c) as usize + super::research::extra_places(c) as usize
}

/// The buses ordered from the dealers that come by `until` (None: all of them).
fn ordered(c: &Company, until: Option<&str>) -> usize {
    c.dealer.orders.iter().filter(|o| until.is_none_or(|u| dates::between(&super::dealer::day_of(&o.delivery), u) >= 0)).map(|o| o.contract.count as usize).sum()
}

/// Whether there is room for another bus: the depot's places and a few rented ones in the
/// street (the market asks before a bus is bought, leased or rented), the buses ordered from
/// the dealers counted as `dealer::room_for` counts them.
pub fn room(c: &Company) -> Result<(), &'static str> {
    room_until(c, None)
}

/// `room` for a bus the company has until `until` (a rental; None: one it keeps): only the
/// buses ordered that come before it goes back take its place.
pub fn room_until(c: &Company, until: Option<&str>) -> Result<(), &'static str> {
    if held(c) + ordered(c, until) >= places(c) + OUTSIDE_MAX {
        return Err("The depot has no room for another bus: build more parking spaces.");
    }
    Ok(())
}

/// Buses that stand outside the yard tonight (over its places).
pub fn outside(c: &Company) -> usize {
    outside_on(c, &c.date)
}

/// Buses that stand outside the yard on `day`: those held and not in the workshop (a bus in
/// a bay needs no space) over the depot's places, the halls' too.
pub fn outside_on(c: &Company, day: &str) -> usize {
    let parked = c.fleet.iter().filter(|v| v.held_on(day) && !v.in_workshop(day)).count();
    parked.saturating_sub(places(c))
}

/// What a job on a bus costs now.
pub fn job_cost(c: &Company, vehicle: u32, kind: JobKind) -> Cents {
    let Some(v) = c.vehicle(vehicle) else { return 0 };
    let size = match v.kind.size {
        BusSize::Midi => 0.8,
        BusSize::Solo => 1.0,
        BusSize::Articulated | BusSize::Double => 1.35,
    };
    let age = dates::years_between(&v.built, &c.date);
    let euros = match kind {
        JobKind::Service => 0.0,
        JobKind::Repair => (market::serviced_condition(age) - v.condition).max(5.0) * 120.0 * size,
        JobKind::Overhaul => economy::reference_price(v.kind) as f64 / 100.0 * 0.06,
        JobKind::Paint => {
            let cost = super::livery::paint_cost(c, v) as f64 * shop_factor(c, kind);
            return (cost / 100.0).round() as Cents * 100;
        }
    };
    ((euros * c.price_index * shop_factor(c, kind)).round() as Cents) * 100
}

/// What the workshop does to a job's cost: a garage outside dearer while there is none of the
/// company's own, its paint booth, inspection lane and tyre store cheaper.
pub fn shop_factor(c: &Company, kind: JobKind) -> f64 {
    let s = &c.site;
    if s.bays() == 0 {
        return OUTSIDE_GARAGE;
    }
    let mut f = 1.0;
    if kind == JobKind::Paint && s.has(Upgrade::PaintBooth) {
        f *= 0.7;
    }
    if kind == JobKind::Repair && s.has(Upgrade::InspectionLane) {
        f *= 0.85;
    }
    if matches!(kind, JobKind::Repair | JobKind::Overhaul) && s.has(Upgrade::TyreStore) {
        f *= 0.93;
    }
    f
}

/// What the parking's roofs and lights and the underbody wash do to the dirt a day.
pub fn dirt_factor(s: &Site) -> f64 {
    let mut f = 1.0;
    if s.has(Upgrade::Hall) {
        f *= 0.75;
    } else if s.has(Upgrade::Carports) {
        f *= 0.88;
    }
    if s.has(Upgrade::Lights) {
        f *= 0.97;
    }
    if s.has(Upgrade::Underbody) {
        f *= 0.95;
    }
    f
}

/// Bays free for jobs on `day`: the company's own not in use, or as many as are asked for at
/// a garage outside while it has no workshop.
fn bays_free(c: &Company, day: &str) -> usize {
    if c.site.bays() == 0 {
        return usize::MAX;
    }
    c.site.bays().saturating_sub(bays_used(c, day))
}

/// Order a job for a bus: it starts tomorrow when a bay is free, else when one is. The night
/// starts it (`after_day`): until then the bus runs its tours and the job can be taken back.
pub fn order(c: &mut Company, vehicle: u32, kind: JobKind) -> Result<u32, &'static str> {
    let Some(v) = c.vehicle(vehicle) else { return Err("This bus is not in the fleet.") };
    if !v.held_on(&c.date) {
        return Err("This bus is not in the fleet.");
    }
    if c.site.job_of(vehicle).is_some() {
        return Err("The workshop has a job for this bus already.");
    }
    let cost = job_cost(c, vehicle, kind);
    if cost > 0 && !c.can_pay(cost) {
        return Err("Not enough cash.");
    }
    c.site.job_counter += 1;
    let id = c.site.job_counter;
    c.site.jobs.push(Job { id, vehicle, kind, ordered: c.date.clone(), started: None, until: None, cost, livery: None });
    Ok(id)
}

/// Order the painting of a bus in a livery of the company's (`livery::paint` checks it): as
/// any job, from tomorrow when a bay is free, paid when it starts.
pub fn order_paint(c: &mut Company, vehicle: u32, livery: &str, cost: Cents) -> Result<u32, &'static str> {
    let Some(v) = c.vehicle(vehicle) else { return Err("This bus is not in the fleet.") };
    if !v.held_on(&c.date) {
        return Err("This bus is not in the fleet.");
    }
    if c.site.job_of(vehicle).is_some() {
        return Err("The workshop has a job for this bus already.");
    }
    if !c.can_pay(cost) {
        return Err("Not enough cash.");
    }
    c.site.job_counter += 1;
    let id = c.site.job_counter;
    c.site.jobs.push(Job { id, vehicle, kind: JobKind::Paint, ordered: c.date.clone(), started: None, until: None, cost, livery: Some(livery.to_string()) });
    Ok(id)
}

/// Take back a job that has not started.
pub fn cancel(c: &mut Company, id: u32) -> Result<(), &'static str> {
    match c.site.jobs.iter().position(|j| j.id == id) {
        Some(i) if c.site.jobs[i].started.is_none() => {
            c.site.jobs.remove(i);
            Ok(())
        }
        Some(_) => Err("The work has started."),
        None => Err("There is no such job."),
    }
}

/// The bays in use on `day`: buses in the workshop (a breakdown, the service the night sends
/// a bus to, a job of ours).
pub fn bays_used(c: &Company, day: &str) -> usize {
    c.fleet.iter().filter(|v| v.held_on(day) && v.in_workshop(day)).count()
}

/// Whether a waiting job starts tomorrow as things stand (the night starts it): a bay is free
/// for it then, after the jobs ordered before it.
pub fn starts_tomorrow(c: &Company, id: u32) -> bool {
    let tomorrow = dates::add(&c.date, 1);
    let mut free = bays_free(c, &tomorrow);
    for j in c.site.jobs.iter().filter(|j| j.started.is_none()) {
        let Some(v) = c.vehicle(j.vehicle) else { continue };
        let there = v.in_workshop(&tomorrow);
        let starts = if there { j.kind == JobKind::Service } else { free > 0 };
        if j.id == id {
            return starts;
        }
        if starts && !there {
            free = free.saturating_sub(1);
        }
    }
    false
}

/// Start the waiting jobs that find a free bay on `day`, in the order they were ordered.
fn start_jobs(c: &mut Company, day: &str) {
    let mut free = bays_free(c, day);
    let outside = c.site.bays() == 0;
    let waiting: Vec<u32> = c.site.jobs.iter().filter(|j| j.started.is_none()).map(|j| j.id).collect();
    for id in waiting {
        let Some(j) = c.site.jobs.iter().find(|j| j.id == id).cloned() else { continue };
        // (fewer days with the research into repairs: `parts::job_days`)
        let days = super::parts::job_days(c, j.kind) + outside as i64;
        let Some(v) = c.fleet.iter_mut().find(|v| v.id == j.vehicle) else {
            c.site.jobs.retain(|x| x.id != id);
            continue;
        };
        // (in the workshop already - a breakdown, the night's own service -: a service is done
        // while it is there, in the bay it has; any other job waits for it to be out)
        let there = v.in_workshop(day);
        if (there && j.kind != JobKind::Service) || (!there && free == 0) {
            continue;
        }
        let until = if there { day.to_string() } else { dates::add(day, days - 1) };
        if !there {
            v.workshop_until = Some(until.clone());
            free = free.saturating_sub(1);
        }
        let age = dates::years_between(&v.built, day);
        match j.kind {
            JobKind::Service => {
                v.condition = v.condition.max(market::serviced_condition(age));
                v.next_service_km = ((v.km / market::SERVICE_KM).floor() + 1.0) * market::SERVICE_KM;
            }
            JobKind::Repair => v.condition = v.condition.max(market::serviced_condition(age)),
            JobKind::Overhaul => {
                v.condition = v.condition.max(96.0);
                v.next_service_km = ((v.km / market::SERVICE_KM).floor() + 1.0) * market::SERVICE_KM;
            }
            JobKind::Paint => {
                if let Some(name) = j.livery.clone().filter(|n| !n.trim().is_empty()) {
                    v.livery = name.clone();
                    v.house_livery = Some(name);
                }
            }
        }
        let text = format!("{} {} ({})", v.number, v.name, j.kind.label());
        if let Some(x) = c.site.jobs.iter_mut().find(|x| x.id == id) {
            x.started = Some(day.to_string());
            x.until = Some(until);
        }
        // (the dealer's free first service is his: what it is worth comes back; repairs under
        // his warranty cost nothing)
        if j.kind == JobKind::Service {
            super::dealer::credit_free_service(c, j.vehicle);
        }
        let free_of_charge = j.kind == JobKind::Repair && super::dealer::under_warranty(c, j.vehicle, day);
        let kind = if j.kind == JobKind::Paint { BookingKind::Livery } else { BookingKind::Repair };
        // (the parts it takes from the workshop's stores, their worth off its cost: `parts`)
        super::parts::job_started(c, j.vehicle, j.kind, if free_of_charge { 0 } else { j.cost });
        c.book(kind, if free_of_charge { 0 } else { -j.cost }, text, false);
    }
}

/// What the night did at the depot, for the day's report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Night {
    /// Areas whose building work was finished (their labels).
    pub built: Vec<Area>,
    /// What was added to them.
    pub added: Vec<Upgrade>,
    /// What the depot's own bulk tank, AdBlue and solar roof saved.
    pub energy_saved: Cents,
    /// Buses that wait for a free bay (their numbers).
    pub waiting: Vec<String>,
    /// Bought outside the depot: fuel and power, street parking.
    pub energy_outside: Cents,
    pub parking_outside: Cents,
    pub upkeep: Cents,
}

/// The night at the depot after the day `date` was closed (the company's date is the next
/// day already; what is booked here is booked on `date`).
pub fn after_day(c: &mut Company, date: &str) -> Night {
    let mut night = Night::default();
    let tomorrow = c.date.clone();
    let keep = std::mem::replace(&mut c.date, date.to_string());

    // building work finished
    let done: Vec<Works> = c.site.works.iter().filter(|w| dates::between(&w.until, date) >= 0).cloned().collect();
    c.site.works.retain(|w| dates::between(&w.until, date) < 0);
    migrate(&mut c.site);
    for w in done {
        match w.upgrade {
            Some(u) => {
                if !c.site.has(u) {
                    c.site.upgrades.push(u);
                }
                night.added.push(u);
            }
            None => {
                super::yard::finish(&mut c.site, &w);
                night.built.push(w.area);
            }
        }
    }

    // fuel and power bought outside, for the share of the fleet the depot cannot serve
    let fleet: Vec<_> = c.fleet.iter().filter(|v| v.held_on(date)).cloned().collect();
    let energy_today: Cents = -c.ledger.iter().filter(|b| b.date == date && b.kind == BookingKind::Energy).map(|b| b.amount).sum::<Cents>();
    if energy_today > 0 && !fleet.is_empty() {
        let weight = |d: Drive| fleet.iter().filter(|v| v.kind.drive == d).map(|v| economy::energy_per_km(v.kind, &super::econ_uk::Level::uk(1.0))).sum::<f64>();
        let (wd, we) = (weight(Drive::Diesel), weight(Drive::Electric));
        let all = (wd + we).max(1e-9);
        let n = |d: Drive| fleet.iter().filter(|v| v.kind.drive == d).count();
        let (nd, ne) = (n(Drive::Diesel), n(Drive::Electric));
        let short = |n: usize, served: usize| if n == 0 { 0.0 } else { n.saturating_sub(served) as f64 / n as f64 };
        // (the automatic refuelling researched serves more a night)
        let refuel = super::research::refuel_factor(c);
        let served = |n: usize| (n as f64 * refuel).floor() as usize;
        let extra = energy_today as f64 * (wd / all * short(nd, served(c.site.diesel_served())) * DIESEL_OUTSIDE + we / all * short(ne, served(c.site.electric_served())) * POWER_OUTSIDE);
        let extra = extra.round() as Cents;
        if extra > 0 {
            c.book(BookingKind::Energy, -extra, "Fuel and power bought outside the depot", false);
            night.energy_outside = extra;
        }
        // what the depot's own fuel and power save: the bulk tank and AdBlue on the diesel
        // filled there, the solar roof on the power charged there
        let s = &c.site;
        let diesel = (s.has(Upgrade::BulkTank) as u8 as f64) * 0.03 + (s.has(Upgrade::AdBlue) as u8 as f64) * 0.01;
        let power = (s.has(Upgrade::Solar) as u8 as f64) * 0.08;
        let saved = energy_today as f64 * (wd / all * (1.0 - short(nd, served(s.diesel_served()))) * diesel + we / all * (1.0 - short(ne, served(s.electric_served()))) * power);
        let saved = saved.round() as Cents;
        if saved > 0 {
            c.book(BookingKind::Energy, saved, "Fuel and power the depot saved", false);
            night.energy_saved = saved;
        }
    }

    // street parking for the buses the yard has no space for
    let out = outside_on(c, &tomorrow);
    if out > 0 {
        let cost = (OUTSIDE_PER_DAY as f64 * out as f64 * c.price_index).round() as Cents;
        c.book(BookingKind::Depot, -cost, format!("Street parking ({out})"), false);
        night.parking_outside = cost;
    }

    // the fleet's cleanliness: every day takes some, the washing bay gives back
    if !fleet.is_empty() {
        // (what research does: the automatic washing, less dirt, the cleaners' rounds)
        let interior = if c.site.has(Upgrade::Interior) { 1.15 } else { 1.0 };
        let w = (c.site.washes() as f64 * super::research::wash_factor(c) * interior / fleet.len() as f64).min(1.0);
        let dirt = if out > 0 { 5.0 + 3.0 * out as f64 / fleet.len() as f64 } else { 5.0 } * super::research::dirt_factor(c) * dirt_factor(&c.site);
        // (without a washing bay the cleaners' buckets keep it from going below this)
        let floor = super::research::clean_floor(c);
        c.site.clean = ((c.site.clean - dirt).max(floor) * (1.0 - w) + 100.0 * w).clamp(0.0, 100.0);
        super::reputation::change(c, date, super::reputation::Cause::Depot, (c.site.clean - 60.0) / 40.0 * 0.08, 0);
    }

    // the staff's rooms: room enough lifts them a little, too few is felt
    let staff_n = c.staff.iter().filter(|e| e.employed_on(date)).count();
    if staff_n > 0 {
        let room = c.site.staff_room();
        let s = &c.site;
        let extras = 0.02 * s.has(Upgrade::Canteen) as u8 as f64 + 0.01 * s.has(Upgrade::TrainingRoom) as u8 as f64 + 0.01 * s.has(Upgrade::Lights) as u8 as f64;
        let lift = if staff_n <= room { 0.03 * c.site.offices as f64 + extras } else { -0.05 + extras };
        for e in c.staff.iter_mut().filter(|e| e.employed_on(date)) {
            e.satisfaction = (e.satisfaction + lift).clamp(0.0, 100.0);
        }
    }

    // the control room: the service watched, the passengers notice
    if c.site.has(Upgrade::ControlRoom) && !fleet.is_empty() {
        super::reputation::change(c, date, super::reputation::Cause::Depot, 0.01, 0);
    }

    // the month's upkeep of what was built
    if dates::last_of_month(date) {
        let up = upkeep_month(c);
        if up > 0 {
            c.book(BookingKind::Depot, -up, "Upkeep of the depot's buildings", false);
            night.upkeep = up;
        }
    }

    // the workshop: jobs done, buses over the bays wait, waiting jobs start
    c.site.jobs.retain(|j| j.until.as_deref().is_none_or(|u| dates::between(u, date) < 0));
    c.site.jobs.retain(|j| c.fleet.iter().any(|v| v.id == j.vehicle));
    let ours: Vec<u32> = c.site.jobs.iter().filter(|j| j.started.is_some()).map(|j| j.vehicle).collect();
    let used = bays_used(c, &tomorrow);
    // (none of its own: the garage outside takes them all)
    if c.site.bays() > 0 && used > c.site.bays() {
        let mut over = used - c.site.bays();
        // (the bays our own jobs hold are not free for the others)
        let ours_in = c.fleet.iter().filter(|v| v.held_on(&tomorrow) && v.in_workshop(&tomorrow) && ours.contains(&v.id)).count();
        let bays = c.site.bays().saturating_sub(ours_in);
        for v in c.fleet.iter_mut().filter(|v| v.held_on(&tomorrow) && v.in_workshop(&tomorrow) && !ours.contains(&v.id)).skip(bays) {
            if over == 0 {
                break;
            }
            if let Some(u) = v.workshop_until.clone() {
                v.workshop_until = Some(dates::add(&u, 1));
                night.waiting.push(v.number.clone());
                over -= 1;
            }
        }
    }
    start_jobs(c, &tomorrow);

    c.date = keep;
    night
}

/// After the day's close (`day::close_day`): the night at the depot and of the concessions
/// (`lines`: the timetable's lines of the day closed), what they booked counted into the
/// day's report and its record, and what they did told in its notes.
pub fn after_close(c: &mut Company, mut report: DayReport, lines: &[crate::LineInfo]) -> DayReport {
    let date = report.date.clone();
    let month = dates::month_of(&date);
    let before = c.month(&month);
    let night = after_day(c, &date);
    let km: Vec<(String, f64)> = report.lines.iter().map(|l| (l.line.clone(), l.km)).collect();
    let events = super::concessions::after_day(c, &date, &km, lines);
    let after = c.month(&month);
    for k in BookingKind::ALL.iter().filter(|k| !k.is_capital()) {
        let d = after.get(*k) - before.get(*k);
        if d > 0 {
            report.income += d;
        } else {
            report.expenses -= d;
        }
    }
    report.result = report.income - report.expenses;
    report.cash = c.cash;
    for a in night.built {
        report.notes.push(Note::Built { area: a.label().to_string() });
    }
    for u in night.added {
        report.notes.push(Note::Built { area: u.label().to_string() });
    }
    for number in night.waiting {
        report.notes.push(Note::BayWait { number });
    }
    for e in events {
        report.notes.push(match e {
            super::concessions::Event::Won { number, until } => Note::Won { number, until },
            super::concessions::Event::Lost { number, winner } => Note::Lost { number, winner },
            super::concessions::Event::Ended { number } => Note::Ended { number },
        });
    }
    if let Some(h) = c.history.last_mut().filter(|h| h.date == date) {
        h.cash = report.cash;
        h.income = report.income;
        h.expenses = report.expenses;
        h.result = report.result;
    }
    c.last_report = Some(report.clone());
    report
}

#[cfg(test)]
mod tests {
    use super::super::market::{self, MarketBus, Payment};
    use super::super::model::BusKind;
    use super::super::{found, Founding};
    use super::*;

    fn company(d: GameMode, buses: usize) -> Company {
        let mut c = found(&Founding { name: "Depot".into(), game_mode: d, date: "2024-03-04".into(), ..Default::default() }, "Luc");
        c.cash += 10_000_000_00;
        // (level 2: the workshop may be built - the levels' gates are tested in `market`)
        c.progress.xp = super::super::levels::LEVEL_XP[1];
        let bus = MarketBus { file: "Vehicles/Citaro/Citaro.bus".into(), name: "Citaro".into(), ..Default::default() };
        for _ in 0..buses {
            market::buy_new(&mut c, &bus, Payment::Cash, "").unwrap();
        }
        c
    }

    /// Close `n` days of the depot only (the company's day moves on, as the day's close does).
    fn nights(c: &mut Company, n: usize) {
        for _ in 0..n {
            let date = c.date.clone();
            c.date = dates::add(&date, 1);
            after_day(c, &date);
        }
    }

    #[test]
    fn a_company_starts_with_the_rented_yard_and_its_upkeep_is_nothing() {
        let c = company(GameMode::Normal, 0);
        assert_eq!((c.site.spaces(), c.site.bays(), c.site.washes(), c.site.staff_room()), (12, 1, 0, 20));
        assert_eq!(upkeep_month(&c), 0);
        // (an old file without a site reads the same yard, brought up to now)
        let mut s: Site = serde_json::from_str("{\"parking\":1,\"workshop\":1,\"wash\":0,\"fuel\":1,\"charging\":0,\"offices\":1}").unwrap();
        assert!(s.rented && s.generation == 0);
        migrate(&mut s);
        assert_eq!(s, Site::default());
    }

    #[test]
    fn a_company_founded_now_has_its_land_only_and_a_building_credit_for_a_first_depot() {
        let mut c = found(&Founding { name: "Leer".into(), game_mode: GameMode::Normal, date: "2024-03-04".into(), empty_depot: true, ..Default::default() }, "Luc");
        c.progress.xp = super::super::levels::LEVEL_XP[1];
        assert_eq!((c.site.spaces(), c.site.bays(), c.site.washes(), c.site.staff_room(), c.site.fuel), (0, 0, 0, 0, 0));
        assert!(!c.site.rented && upkeep_month(&c) == 0);
        // the credit: a block of spaces, a workshop bay, a diesel pump and the offices
        let first = |c: &Company, a| next_cost(c, a).unwrap().0;
        let set = first(&c, Area::Parking) + first(&c, Area::Workshop) + first(&c, Area::Fuel) + first(&c, Area::Offices);
        assert_eq!(c.site.credit, set);
        // building spends the credit first, the cash not at all while it lasts
        let (cash, ledger) = (c.cash, c.ledger.len());
        build(&mut c, Area::Parking).unwrap();
        assert_eq!((c.cash, c.ledger.len()), (cash, ledger));
        assert_eq!(c.site.credit, set - first_parking(&c));
        nights(&mut c, 14);
        assert_eq!(c.site.spaces(), SPACES_PER_LEVEL as usize);
        // what was built from the land up costs its upkeep from its first level
        assert_eq!(upkeep_month(&c), step(Area::Parking, 1).upkeep);
        // the rest of the credit and then the cash: a dear second hall needs some of both
        c.site.credit = 1_000_00;
        let (cost, _) = next_cost(&c, Area::Wash).unwrap();
        assert_eq!(cash_needed(&c, cost), cost - 1_000_00);
        let cash = c.cash;
        build(&mut c, Area::Wash).unwrap();
        assert_eq!((c.site.credit, c.cash), (0, cash - (cost - 1_000_00)));
    }

    /// What the first block of spaces cost (the one being built: its works know its level).
    fn first_parking(c: &Company) -> Cents {
        let mut before = c.clone();
        before.site.works.clear();
        next_cost(&before, Area::Parking).unwrap().0
    }

    #[test]
    fn without_a_workshop_jobs_go_to_a_garage_outside_dearer_and_a_day_longer() {
        let mut c = company(GameMode::Normal, 3);
        for v in c.fleet.iter_mut() {
            v.condition = 50.0;
        }
        let ids: Vec<u32> = c.fleet.iter().map(|v| v.id).collect();
        let own = job_cost(&c, ids[0], JobKind::Repair);
        c.site.workshop = 0;
        let outside = job_cost(&c, ids[0], JobKind::Repair);
        assert!(outside > own, "{outside} {own}");
        // all three at once: the garage outside has room for them
        for &id in &ids {
            order(&mut c, id, JobKind::Repair).unwrap();
        }
        assert!(ids.iter().all(|&id| starts_tomorrow(&c, c.site.job_of(id).unwrap().id)));
        nights(&mut c, 1);
        assert!(c.site.jobs.iter().all(|j| j.started.is_some()));
        // (a repair's two days and the day of the way there)
        let until = c.site.jobs[0].until.clone().unwrap();
        assert_eq!(dates::between(&c.date, &until), 2);
    }

    #[test]
    fn what_is_added_to_an_area_is_there_at_once_and_does_what_it_says() {
        let mut c = company(GameMode::Normal, 10);
        // (nothing before its area: the wash lanes first)
        assert_eq!(build_upgrade(&mut c, Upgrade::Gantry), Err("Build the area first."));
        assert_eq!(upgrade_cost(&c, Upgrade::Gantry), None);
        c.site.wash = 2;
        let (cost, _) = upgrade_cost(&c, Upgrade::Gantry).unwrap();
        // (priced a lane: two lanes cost twice one)
        c.site.wash = 1;
        assert_eq!(upgrade_cost(&c, Upgrade::Gantry).unwrap().0 * 2, cost);
        c.site.wash = 2;
        assert_eq!(c.site.washes(), 2 * WASHES_BY_HAND);
        let cash = c.cash;
        build_upgrade(&mut c, Upgrade::Gantry).unwrap();
        assert_eq!(c.cash, cash - cost);
        // (no waiting: there at once, and the next one may follow straight away)
        assert!(c.site.has(Upgrade::Gantry) && c.site.level(Area::Wash) == 2 && c.site.works.is_empty());
        assert_eq!(c.site.washes(), 2 * WASHES_GANTRY);
        assert_eq!(build_upgrade(&mut c, Upgrade::Gantry), Err("This is there already."));
        // a lane built since comes with its gantry, paid for with it
        let with = next_cost(&c, Area::Wash).unwrap().0;
        c.site.upgrades.retain(|u| *u != Upgrade::Gantry);
        let without = next_cost(&c, Area::Wash).unwrap().0;
        c.site.upgrades.push(Upgrade::Gantry);
        assert!((with - without - cost / 2).abs() <= 100, "{with} {without} {cost}");
        // its upkeep is the depot's; the water recycled makes the lanes' less
        let up = upkeep_month(&c);
        c.site.upgrades.push(Upgrade::Recycling);
        let wash_lanes = step(Area::Wash, 1).upkeep * 2;
        let recycling = upgrade_step(&c.site, Upgrade::Recycling).upkeep;
        assert_eq!(upkeep_month(&c), up - wash_lanes + (wash_lanes as f64 * 0.6).round() as Cents + recycling);
        // the workshop's: a livery and a repair cheaper
        let id = c.fleet[0].id;
        c.fleet[0].condition = 50.0;
        let (paint, repair) = (job_cost(&c, id, JobKind::Paint), job_cost(&c, id, JobKind::Repair));
        c.site.upgrades.extend([Upgrade::PaintBooth, Upgrade::InspectionLane]);
        assert!(job_cost(&c, id, JobKind::Paint) < paint && job_cost(&c, id, JobKind::Repair) < repair);
        // the roofs over the spaces: less dirt a day
        assert!(dirt_factor(&c.site) == 1.0);
        c.site.upgrades.push(Upgrade::Carports);
        let carports = dirt_factor(&c.site);
        c.site.upgrades.push(Upgrade::Hall);
        assert!(dirt_factor(&c.site) < carports && carports < 1.0);
        // the bulk tank: the diesel filled at the depot a little cheaper, booked back the night
        c.site.upgrades.push(Upgrade::BulkTank);
        c.book(BookingKind::Energy, -1_000_00, "line", false);
        let date = c.date.clone();
        c.date = dates::add(&date, 1);
        assert_eq!(after_day(&mut c, &date).energy_saved, 30_00);
    }

    #[test]
    fn a_washing_bay_of_before_is_a_lane_with_its_gantries() {
        let mut s = Site { wash: 2, generation: 0, ..Site::default() };
        migrate(&mut s);
        assert_eq!((s.wash, s.has(Upgrade::Gantry), s.washes(), s.generation), (1, true, WASHES_GANTRY, GENERATION));
        // (once only: a second lane built since stays)
        s.wash = 2;
        migrate(&mut s);
        assert_eq!(s.wash, 2);
    }

    #[test]
    fn the_look_keeps_colours_only() {
        let mut c = company(GameMode::Normal, 0);
        set_look(&mut c, Look { wall: Some("#AA3311".into()), roof: Some("blue".into()), trim: Some("12ab3".into()), door: None, finish: Finish::Brick });
        assert_eq!(c.site.look, Look { wall: Some("#aa3311".into()), roof: None, trim: None, door: None, finish: Finish::Brick });
        let back: Site = serde_json::from_str(&serde_json::to_string(&c.site).unwrap()).unwrap();
        assert_eq!(back.look, c.site.look);
    }

    #[test]
    fn building_costs_by_difficulty_is_done_at_once_and_then_costs_upkeep() {
        let easy = company(GameMode::Casual, 0);
        let hard = company(GameMode::Realistic, 0);
        let mut c = company(GameMode::Normal, 0);
        // (the UK's build cost at the mode's cost factor: Realistic 1, Normal 0.9, Casual 0.75)
        let uk = step(Area::Wash, 1).cost as f64;
        assert!((crate::company::currency::Currency::Gbp.units(uk) - 90_000.0).abs() < 100.0);
        let at = |f: f64| ((uk * f / 100.0).round() as Cents) * 100;
        assert_eq!(next_cost(&c, Area::Wash), Some((at(0.9), 21)));
        assert_eq!(next_cost(&easy, Area::Wash), Some((at(0.75), 11)));
        assert_eq!(next_cost(&hard, Area::Wash), Some((at(1.0), 21)));
        let cash = c.cash;
        build(&mut c, Area::Wash).unwrap();
        assert_eq!(c.cash, cash - at(0.9));
        assert!(c.ledger.last().is_some_and(|b| b.kind == BookingKind::Construction && b.kind.is_capital()));
        assert_eq!(c.site.wash, 1, "built at once");
        assert!(c.site.works.is_empty());
        let upkeep = step(Area::Wash, 1).upkeep;
        assert!(upkeep > 0);
        assert_eq!(upkeep_month(&c), upkeep);
        // the month's end books it
        while !dates::last_of_month(&c.date) {
            nights(&mut c, 1);
        }
        let month = dates::month_of(&c.date);
        nights(&mut c, 1);
        assert_eq!(c.month(&month).get(BookingKind::Depot), -upkeep);
        // a top level is the top
        c.site.wash = Area::Wash.max();
        assert_eq!(build(&mut c, Area::Wash), Err("This is as big as it gets."));
        let mut poor = company(GameMode::Realistic, 0);
        poor.cash = 10;
        assert_eq!(build(&mut poor, Area::Workshop), Err("Not enough cash."));
    }

    #[test]
    fn the_yard_limits_the_fleet_and_buses_outside_cost_street_parking() {
        let mut c = company(GameMode::Normal, 12);
        assert_eq!(outside(&c), 0);
        let bus = MarketBus { file: "Vehicles/Citaro/Citaro.bus".into(), name: "Citaro".into(), ..Default::default() };
        for _ in 0..OUTSIDE_MAX {
            market::buy_new(&mut c, &bus, Payment::Cash, "").unwrap();
        }
        assert_eq!(outside(&c), OUTSIDE_MAX);
        assert!(room(&c).is_err());
        assert!(market::buy_new(&mut c, &bus, Payment::Cash, "").is_err());
        assert!(market::rent(&mut c, &bus, 2, "").is_err());
        let cash = c.cash;
        nights(&mut c, 1);
        assert_eq!(cash - c.cash, OUTSIDE_PER_DAY * OUTSIDE_MAX as Cents);
        // more spaces, more room
        c.site.parking = 2;
        assert!(room(&c).is_ok());
        // the second hall the levels open: six places under a roof, not in the street
        let mut hall = company(GameMode::Normal, 0);
        hall.progress.xp = super::super::levels::LEVEL_XP[2];
        for _ in 0..18 {
            market::buy_new(&mut hall, &bus, Payment::Cash, "").unwrap();
        }
        assert_eq!((places(&hall), outside(&hall)), (18, 0));
        let cash = hall.cash;
        nights(&mut hall, 1);
        assert_eq!(cash, hall.cash, "no street parking");
        // buses ordered from a dealer have their places kept: a bus kept is refused, a rental
        // that goes back before they come is not
        let delivery = super::super::dealer::at(&dates::add(&hall.date, 14), 9 * 60);
        let contract = super::super::dealer::Contract { count: OUTSIDE_MAX as u32, ..Default::default() };
        hall.dealer.orders.push(super::super::dealer::Order { contract, delivery });
        assert!(room(&hall).is_err());
        assert!(market::buy_new(&mut hall, &bus, Payment::Cash, "").is_err());
        assert!(market::rent(&mut hall, &bus, 2, "").is_ok());
        assert!(market::rent(&mut hall, &bus, 20, "").is_err());
    }

    #[test]
    fn jobs_wait_for_a_free_bay_and_the_bus_is_out_only_while_worked_on() {
        let mut c = company(GameMode::Normal, 3);
        let ids: Vec<u32> = c.fleet.iter().map(|v| v.id).collect();
        for &id in &ids {
            c.fleet.iter_mut().find(|v| v.id == id).unwrap().condition = 50.0;
        }
        let tomorrow = dates::add(&c.date, 1);
        let a = order(&mut c, ids[0], JobKind::Repair).unwrap();
        let b = order(&mut c, ids[1], JobKind::Service).unwrap();
        assert_eq!(order(&mut c, ids[1], JobKind::Repair), Err("The workshop has a job for this bus already."));
        // ordered today: nothing starts before the night, the buses run today's tours
        assert!(c.site.jobs.iter().all(|j| j.started.is_none()));
        assert!(!c.vehicle(ids[0]).unwrap().in_workshop(&c.date) && c.vehicle(ids[0]).unwrap().condition == 50.0);
        assert!(starts_tomorrow(&c, a) && !starts_tomorrow(&c, b));
        nights(&mut c, 1);
        // one bay: the first starts tomorrow, the second waits and its bus still runs
        let ja = c.site.jobs.iter().find(|j| j.id == a).unwrap().clone();
        assert_eq!(ja.started.as_deref(), Some(tomorrow.as_str()));
        assert!(c.site.jobs.iter().find(|j| j.id == b).unwrap().started.is_none());
        assert!(c.vehicle(ids[0]).unwrap().in_workshop(&tomorrow));
        assert!(!c.vehicle(ids[1]).unwrap().in_workshop(&tomorrow));
        assert!(c.vehicle(ids[0]).unwrap().condition > 80.0);
        assert!(c.ledger.last().is_some_and(|x| x.kind == BookingKind::Repair && x.amount < 0));
        // the repair takes two days (tomorrow and the day after); then the service gets the bay
        nights(&mut c, 1);
        assert!(c.site.jobs.iter().find(|j| j.id == b).unwrap().started.is_none());
        assert!(c.vehicle(ids[0]).unwrap().in_workshop(&c.date));
        nights(&mut c, 1);
        assert!(c.site.jobs.iter().all(|j| j.id != a), "the repair is done");
        assert!(!c.vehicle(ids[0]).unwrap().in_workshop(&c.date));
        let jb = c.site.jobs.iter().find(|j| j.id == b).unwrap();
        assert_eq!(jb.started.as_deref(), Some(c.date.as_str()));
        // with two bays both start at once; until the night a job can be taken back
        let mut two = company(GameMode::Normal, 3);
        two.site.workshop = 2;
        let v: Vec<u32> = two.fleet.iter().map(|v| v.id).collect();
        let s = order(&mut two, v[0], JobKind::Service).unwrap();
        order(&mut two, v[1], JobKind::Overhaul).unwrap();
        let r = order(&mut two, v[2], JobKind::Repair).unwrap();
        cancel(&mut two, r).unwrap();
        nights(&mut two, 1);
        assert!(two.site.jobs.len() == 2 && two.site.jobs.iter().all(|j| j.started.is_some()));
        assert_eq!(cancel(&mut two, s), Err("The work has started."));
    }

    #[test]
    fn a_service_ordered_for_a_bus_in_the_workshop_anyway_is_done_there() {
        let mut c = company(GameMode::Normal, 1);
        let id = c.fleet[0].id;
        c.fleet[0].condition = 60.0;
        let job = order(&mut c, id, JobKind::Service).unwrap();
        // (the night's own service, or a breakdown, has it in the workshop tomorrow)
        let tomorrow = dates::add(&c.date, 1);
        c.fleet[0].workshop_until = Some(tomorrow.clone());
        assert!(starts_tomorrow(&c, job));
        nights(&mut c, 1);
        assert!(c.fleet[0].condition > 90.0);
        assert!(c.site.job_of(id).is_some_and(|j| j.started.as_deref() == Some(tomorrow.as_str())));
        // not a second day in the workshop for it
        nights(&mut c, 1);
        assert!(c.site.jobs.is_empty() && !c.fleet[0].in_workshop(&c.date));
    }

    #[test]
    fn buses_over_the_bays_wait_when_our_own_jobs_hold_them() {
        let mut c = company(GameMode::Normal, 2);
        let (a, b) = (c.fleet[0].id, c.fleet[1].id);
        order(&mut c, a, JobKind::Overhaul).unwrap();
        nights(&mut c, 1);
        assert!(c.vehicle(a).unwrap().in_workshop(&c.date));
        // a breakdown keeps the other bus in tomorrow: the one bay is the overhaul's
        let tomorrow = dates::add(&c.date, 1);
        c.fleet.iter_mut().find(|v| v.id == b).unwrap().workshop_until = Some(tomorrow.clone());
        let date = c.date.clone();
        c.date = tomorrow.clone();
        let n = after_day(&mut c, &date);
        assert_eq!(n.waiting, vec![c.vehicle(b).unwrap().number.clone()]);
        assert_eq!(c.vehicle(b).unwrap().workshop_until, Some(dates::add(&tomorrow, 1)));
    }

    #[test]
    fn the_dealers_free_first_service_is_credited_once() {
        let mut c = company(GameMode::Normal, 2);
        let (a, b) = (c.fleet[0].id, c.fleet[1].id);
        c.dealer.free_services.push(a);
        c.site.workshop = 2;
        // (no parts in the stores: what a service takes from them is credited too, `parts`)
        c.stores.stock.clear();
        order(&mut c, a, JobKind::Service).unwrap();
        order(&mut c, b, JobKind::Service).unwrap();
        let cash = c.cash;
        nights(&mut c, 1);
        let worth = super::super::dealer::extra_value(&c, super::super::dealer::Extra::FreeService, 0);
        assert!(worth > 0);
        assert_eq!(c.cash - cash, worth, "only the bus that had it");
        assert!(c.dealer.free_services.is_empty());
    }

    #[test]
    fn the_washing_bay_keeps_the_fleet_clean_and_the_tank_saves_on_fuel() {
        let mut dirty = company(GameMode::Normal, 10);
        let mut clean = dirty.clone();
        clean.site.wash = 1;
        nights(&mut dirty, 30);
        nights(&mut clean, 30);
        assert!(dirty.site.clean <= 40.0 && clean.site.clean > 90.0, "{} {}", dirty.site.clean, clean.site.clean);
        assert!(clean.reputation > dirty.reputation);
        // fuel: twenty diesel buses, one pump for fifteen of them - a quarter of it outside
        let mut c = company(GameMode::Normal, 0);
        let bus = MarketBus { file: "Vehicles/Citaro/Citaro.bus".into(), name: "Citaro".into(), ..Default::default() };
        c.site.parking = 2;
        for _ in 0..20 {
            market::buy_new(&mut c, &bus, Payment::Cash, "").unwrap();
        }
        c.book(BookingKind::Energy, -1_000_00, "line", false);
        let date = c.date.clone();
        c.date = dates::add(&date, 1);
        let n = after_day(&mut c, &date);
        assert_eq!(n.energy_outside, 30_00);
        // and an electric bus without a charger charges outside, dearer still
        let mut e = company(GameMode::Normal, 0);
        // (electric buses open at level 3)
        e.progress.xp = super::super::levels::LEVEL_XP[2];
        let ebus = MarketBus { file: "Vehicles/E/E.bus".into(), name: "eCitaro".into(), kind: BusKind { size: BusSize::Solo, drive: Drive::Electric }, ..Default::default() };
        market::buy_new(&mut e, &ebus, Payment::Cash, "").unwrap();
        e.book(BookingKind::Energy, -1_000_00, "line", false);
        let date = e.date.clone();
        e.date = dates::add(&date, 1);
        assert_eq!(after_day(&mut e, &date).energy_outside, 450_00);
        let mut f = e.clone();
        f.site.charging = 1;
        f.book(BookingKind::Energy, -1_000_00, "line", false);
        let date = f.date.clone();
        f.date = dates::add(&date, 1);
        assert_eq!(after_day(&mut f, &date).energy_outside, 0);
    }
}
