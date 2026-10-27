//! The depot page (`crates/omsi-launcher-ui/src/company/depot.rs`): its figures, the yard from
//! above - the offices, the workshop hall with its bays, the washing bay, the diesel station,
//! the parking's blocks with their chargers, the buses where they stand tonight -, the areas to
//! build (a bigger depot, level by level, and what can be added to each area), the building
//! credit a new company has, its look, the staff the 3D depot shows at work, the depot as a
//! dashboard (`dashboard`: what each area has against what is asked of it), and the workshop's
//! jobs. The rules are `company::depot`'s.

use super::super::kit;
use super::super::words::{day_label, eur, grade, tr, trf};
use super::super::{act as apply, Session};
use super::{popup_answer, str_arg, u32_arg};
use crate::Answer;
use hub_core::company::depot::{self as dp, Area, Finish, JobKind, Look, Upgrade};
use hub_core::company::model::{Drive, Role};
use hub_core::company::yard;
use hub_core::company::{self as co, Company};
use serde_json::{json, Value};

pub fn view(s: &mut Session, args: &Value) -> Value {
    let Some(c) = s.company.clone() else { return Value::Null };
    let selected = args.get("selected").and_then(Value::as_u64).map(|x| x as u32);
    json!({ "figures": figures(&c), "plan": plan(&c), "build": buildings(&c), "dashboard": dashboard(&c), "workshop": workshop(&c, selected) })
}

/// What is asked of an area against what it has, as the dashboard's meter: how full it is
/// (`frac`), its tone (room to spare, nearly full, short) and its line.
fn load(need: usize, has: usize, line: String, none: &str) -> Value {
    if has == 0 {
        let tone = if need > 0 { "danger" } else { "dim" };
        return json!({ "frac": if need > 0 { 1.0 } else { 0.0 }, "tone": tone, "line": tr(none) });
    }
    let f = need as f64 / has as f64;
    let tone = if f > 1.0 {
        "danger"
    } else if f > 0.85 {
        "warn"
    } else {
        "ok"
    };
    json!({ "frac": f.min(1.0), "tone": tone, "line": line, "pct": format!("{:.0} %", f * 100.0) })
}

/// The depot as a dashboard: the fleet where it stands tonight and the staff, then each area
/// with its level, what is asked of it against what it has (`load`), the work on it and what
/// was added to it.
fn dashboard(c: &Company) -> Value {
    let s = &c.site;
    let today = &c.date;
    let held: Vec<&co::Vehicle> = c.fleet.iter().filter(|v| v.held_on(today)).collect();
    let in_ws = held.iter().filter(|v| v.in_workshop(today)).count();
    let out = dp::outside(c);
    let parked = held.len() - in_ws;
    let drive = |d: Drive| held.iter().filter(|v| v.kind.drive == d).count();
    let employed = c.staff.iter().filter(|e| e.employed_on(today)).count();
    let waiting = s.jobs.iter().filter(|j| j.started.is_none()).count();
    let busy = dp::bays_used(c, today);
    let n = |x: usize| x.to_string();
    let fleet = json!([
        { "icon": "local_parking", "label": tr("On the yard"), "value": n(parked - out.min(parked)), "tone": "text" },
        { "icon": "construction", "label": tr("In the workshop"), "value": n(in_ws), "tone": "text" },
        { "icon": "warning", "label": tr("In the street"), "value": n(out), "tone": if out > 0 { "warn" } else { "text" } },
    ]);
    let st = staff(c);
    let people = json!([
        { "icon": "directions_bus", "label": tr("Drivers"), "value": st["drivers"].to_string() },
        { "icon": "build", "label": tr("Mechanics"), "value": st["mechanics"].to_string() },
        { "icon": "local_car_wash", "label": tr("Cleaners"), "value": st["cleaners"].to_string() },
        { "icon": "groups", "label": tr("Office"), "value": st["office"].to_string() },
    ]);
    let areas: Vec<Value> = Area::ALL
        .iter()
        .map(|a| {
            let meter = match a {
                Area::Parking => load(parked, dp::places(c), trf("%{need} buses for %{has} spaces", &[("need", &n(parked)), ("has", &n(dp::places(c)))]), "No spaces: the buses stand in the street"),
                Area::Workshop if s.bays() == 0 => json!({ "frac": 0.0, "tone": "dim", "line": tr("None: a garage outside does the work, dearer") }),
                Area::Workshop => {
                    load(busy + waiting, s.bays(), trf("%{busy} of %{has} bays busy, %{waiting} jobs waiting", &[("busy", &n(busy)), ("has", &n(s.bays())), ("waiting", &n(waiting))]), "")
                }
                Area::Wash => load(held.len(), s.washes(), trf("%{need} buses, washes %{has} a night", &[("need", &n(held.len())), ("has", &n(s.washes()))]), "None: the fleet is cleaned by hand"),
                Area::Fuel => load(
                    drive(Drive::Diesel),
                    s.diesel_served(),
                    trf("%{need} diesel buses, fills %{has} a night", &[("need", &n(drive(Drive::Diesel))), ("has", &n(s.diesel_served()))]),
                    "None: diesel is bought outside, dearer",
                ),
                Area::Charging => load(
                    drive(Drive::Electric),
                    s.electric_served(),
                    trf("%{need} electric buses, charges %{has} a night", &[("need", &n(drive(Drive::Electric))), ("has", &n(s.electric_served()))]),
                    "None: electric buses charge outside, dearer",
                ),
                Area::Offices => load(employed, s.staff_room(), trf("%{need} people, room for %{has}", &[("need", &n(employed)), ("has", &n(s.staff_room()))]), "None: the staff have no rooms"),
            };
            let added: Vec<Value> = Upgrade::ALL.iter().filter(|u| u.area() == *a && s.has(**u)).map(|u| json!({ "icon": u.icon(), "label": tr(u.label()) })).collect();
            let works = s.works_on(*a).map(|w| {
                let what = w.upgrade.map(|u| tr(u.label())).unwrap_or_else(|| trf("level %{n}", &[("n", &w.level.to_string())]));
                trf("%{what} ready %{date}", &[("what", &what), ("date", &day_label(&w.until))])
            });
            json!({
                "key": a.key(), "icon": a.icon(), "label": tr(a.label()), "lv": s.level(*a), "max": a.max(),
                "level": trf("level %{n} of %{max}", &[("n", &s.level(*a).to_string()), ("max", &a.max().to_string())]),
                "meter": meter, "added": added, "works": works,
            })
        })
        .collect();
    let money = json!([
        { "label": tr("Upkeep a month"), "value": eur(dp::upkeep_month(c)) },
        { "label": tr("Cleanliness"), "value": format!("{:.0} %", s.clean), "tone": grade(s.clean) },
        { "label": tr("Building credit left"), "value": eur(s.credit.max(0)) },
    ]);
    json!({ "fleet": fleet, "people": people, "areas": areas, "money": money, "fleet_title": tr("Buses tonight"), "people_title": tr("Staff") })
}

fn figure(label: &str, value: String, under: String, tone: &str) -> Value {
    json!({ "label": tr(label), "value": value, "under": under, "tone": tone })
}

/// Parking, the workshop's bays, the cleanliness and the buildings' upkeep (`depot::draw`).
fn figures(c: &Company) -> Value {
    let held = c.fleet.iter().filter(|v| v.held_on(&c.date)).count();
    let spaces = dp::places(c);
    let out = dp::outside(c);
    let under = if out > 0 { trf("%{n} in the street", &[("n", &out.to_string())]) } else { tr("spaces in use") };
    let busy = dp::bays_used(c, &c.date);
    let waiting = c.site.jobs.iter().filter(|j| j.started.is_none()).count();
    let under_ws = if waiting > 0 { trf("Jobs waiting: %{n}", &[("n", &waiting.to_string())]) } else { tr("bays in use") };
    json!([
        figure("Parking", format!("{} / {}", held.min(spaces), spaces), under, if out > 0 { "warn" } else { "text" }),
        figure("Workshop", format!("{} / {}", busy, c.site.bays()), under_ws, if busy > c.site.bays() { "warn" } else { "text" }),
        figure("Cleanliness", format!("{:.0} %", c.site.clean), tr("of the fleet"), grade(c.site.clean)),
        figure("Upkeep", eur(dp::upkeep_month(c)), tr("a month for the buildings"), "text"),
    ])
}

/// A building of the plan: built or a free plot, its head in capitals, the building work on it.
fn building(c: &Company, area: Area, title: &str) -> Value {
    let built = c.site.level(area) > 0;
    let head = if built { tr(title) } else { tr("Free plot") }.to_uppercase();
    let works = c.site.works_on(area).map(|w| trf("Ready %{date}", &[("date", &day_label(&w.until))]));
    json!({ "built": built, "head": head, "works": works })
}

/// A bus where it stands: its number, its size (the 3D yard draws it as long), its bus file and
/// livery (its depot model), its card in the tooltip, and whether it is out on its tours now
/// (`out`: its space kept, the 3D yard has it drive out and back).
fn bus_at(c: &Company, v: &co::Vehicle) -> Value {
    let mut tip = format!("{} {}  ·  {} {:.0} %", v.number, v.name, tr("condition"), v.condition);
    if v.in_workshop(&c.date) {
        tip.push_str(&format!("  ·  {}", trf("in the workshop until %{date}", &[("date", &day_label(v.workshop_until.as_deref().unwrap_or("")))])));
    }
    // (the livery it wears: the house livery once the studio made one, as the game gets it)
    let paint = v.house_livery.clone().filter(|h| !h.trim().is_empty()).unwrap_or_else(|| v.livery.clone());
    let out = c.clock.today.as_ref().filter(|s| s.date == c.date).is_some_and(|s| {
        let now = c.clock.minute as i32;
        co::control::busy(s).iter().any(|&(id, from, to)| id == v.id && from <= now && now < to)
    });
    json!({ "id": v.id, "number": v.number, "size": v.kind.size, "file": v.bus, "paint": paint, "tip": tip, "out": out })
}

/// The parking's spaces, a bus each (null: free), then the buses in the street. Each bus keeps
/// its own space - the fleet's order -, out on its tours or not: one going to the workshop
/// leaves its space free for a bus of the street meanwhile, and the others stay where they are
/// (the 3D yard has a bus drive where it goes, not the whole yard move up).
fn spaces(c: &Company, held: &[&co::Vehicle]) -> Vec<Value> {
    let today = &c.date;
    let n = c.site.spaces();
    let mut slots: Vec<Option<&co::Vehicle>> = vec![None; n.min(held.len())];
    let mut over = Vec::new();
    for (k, v) in held.iter().enumerate() {
        if v.in_workshop(today) {
            continue;
        }
        if k < n {
            slots[k] = Some(v);
        } else {
            over.push(*v);
        }
    }
    let mut street = Vec::new();
    for v in over {
        match slots.iter().position(Option::is_none) {
            Some(k) => slots[k] = Some(v),
            None => street.push(v),
        }
    }
    let mut out: Vec<Value> = slots.iter().map(|v| v.map_or(Value::Null, |v| bus_at(c, v))).collect();
    // (the street after every space the yard draws)
    if !street.is_empty() {
        out.resize(n, Value::Null);
    }
    out.extend(street.iter().map(|v| bus_at(c, v)));
    out
}

/// The yard from above (`depot::plan`): what is built, being built, where the buses stand, and
/// the staff at work today (the 3D yard has them walk about it, each as their role's figure).
fn plan(c: &Company) -> Value {
    let today = c.date.clone();
    let held: Vec<&co::Vehicle> = c.fleet.iter().filter(|v| v.held_on(&today)).collect();
    let mut in_ws: Vec<&co::Vehicle> = held.iter().copied().filter(|v| v.in_workshop(&today)).collect();
    in_ws.sort_by(|a, b| a.number.cmp(&b.number));
    let mut fleet = held.clone();
    fleet.sort_by_key(|v| (v.number.len(), v.number.clone()));
    let mut offices = building(c, Area::Offices, "Offices");
    offices["floors"] = json!((c.site.offices as usize).min(3));
    offices["room"] = json!(trf("room for %{n}", &[("n", &c.site.staff_room().to_string())]));
    let mut hall = building(c, Area::Workshop, "Workshop");
    hall["bays"] = json!(c.site.bays());
    hall["max"] = json!(Area::Workshop.max());
    let mut wash = building(c, Area::Wash, "Washing bay");
    wash["lanes"] = json!(c.site.wash);
    wash["gates"] = json!(c.site.has(Upgrade::Gantry));
    let mut fuel = building(c, Area::Fuel, "Diesel station");
    fuel["pumps"] = json!(c.site.fuel);
    let parking = json!({
        "label": tr(Area::Parking.label()).to_uppercase(),
        "works": c.site.works_on(Area::Parking).map(|w| trf("More spaces ready %{date}", &[("date", &day_label(&w.until))])),
        "blocks": Area::Parking.max(),
        "built": c.site.parking,
        "building": c.site.works_on(Area::Parking).is_some(),
        "per": dp::SPACES_PER_LEVEL,
        "chargers": c.site.electric_served(),
    });
    json!({
        "title": tr("The depot from above"),
        "colour": c.colours[0],
        "offices": offices, "hall": hall, "wash": wash, "fuel": fuel, "parking": parking,
        "in_ws": in_ws.iter().map(|v| bus_at(c, v)).collect::<Vec<_>>(),
        "parked": spaces(c, &fleet),
        "layout": layout(c),
        "upgrades": c.site.upgrades.iter().map(|u| u.key()).collect::<Vec<_>>(),
        "look": look(&c.site.look),
        "staff": staff(c),
    })
}

/// The depot's look as the page has it: its own colours (null: the depot's) and its walls.
fn look(l: &Look) -> Value {
    json!({ "wall": l.wall, "roof": l.roof, "trim": l.trim, "door": l.door, "finish": l.finish.key() })
}

/// The people at work at the depot today, as the 3D depot shows them: the drivers (those out on
/// a duty now among them), the mechanics, the cleaners and the others (the offices': service,
/// inspectors, research - each also counted alone, for its own figure).
fn staff(c: &Company) -> Value {
    let today = &c.date;
    let n = |r: &[Role]| c.staff.iter().filter(|e| e.employed_on(today) && r.contains(&e.role)).count();
    json!({
        "drivers": n(&[Role::Driver]),
        "mechanics": n(&[Role::Mechanic]),
        "cleaners": n(&[Role::Cleaner]),
        "office": n(&[Role::Service, Role::Inspector, Role::Researcher]),
        "service": n(&[Role::Service]),
        "inspectors": n(&[Role::Inspector]),
        "researchers": n(&[Role::Researcher]),
    })
}

/// The yard laid out (`yard`), as the 3D depot draws it and its build mode puts modules down:
/// the land (in cells of `cell` metres), the gate's lane, each module with its cells, its aprons
/// and what it is, the modules being built where they will stand, the shapes of the modules to
/// put down, and the land to buy.
fn layout(c: &Company) -> Value {
    let mut site = c.site.clone();
    yard::reconcile(&mut site);
    let land = yard::owned(&site);
    let module = |p: &yard::Placed, building: bool, works: Option<String>| {
        let r = p.rect();
        json!({
            "id": p.id, "area": p.area.key(), "x": r.x, "y": r.y, "w": r.w, "h": r.h, "rot": p.rot, "aprons": p.aprons(),
            "building": building, "works": works,
            "title": tr(p.area.label()), "what": effect(c, p.area),
        })
    };
    let mut modules: Vec<Value> = site
        .layout
        .iter()
        .map(|p| module(p, false, site.works.iter().find(|w| w.area == p.area && w.at.is_none()).map(|w| trf("Building until %{date}", &[("date", &day_label(&w.until))]))))
        .collect();
    for w in &site.works {
        if let Some(at) = &w.at {
            modules.push(module(at, true, Some(trf("Building until %{date}", &[("date", &day_label(&w.until))]))));
        }
    }
    let shapes: serde_json::Map<String, Value> =
        Area::ALL.iter().filter_map(|a| yard::shape(*a).map(|(w, h, front, back)| (a.key().to_string(), json!({ "w": w, "h": h, "front": front, "back": back })))).collect();
    let more = yard::land_cost(c).map(|cost| {
        json!({ "cost": eur(cost), "can": c.can_pay(cost), "tip": trf("%{m} m more land beside the yard, yours at once", &[("m", &((yard::LAND_COLS as f64 * yard::CELL_M) as i64).to_string())]) })
    });
    json!({
        "cell": yard::CELL_M, "cols": land.w, "rows": land.h, "max_cols": yard::LOT_COLS + yard::LAND_COLS * yard::LAND_MAX as i32,
        "gate": yard::GATE, "modules": modules, "shapes": shapes, "land": more,
    })
}

/// What a level of an area does, in words (`depot::effect`).
fn effect(c: &Company, a: Area) -> String {
    let s = &c.site;
    match a {
        Area::Parking => trf("%{n} spaces", &[("n", &s.spaces().to_string())]),
        Area::Workshop => trf("%{n} bays: buses worked on at once", &[("n", &s.bays().to_string())]),
        Area::Wash if s.wash == 0 => tr("None: the fleet is cleaned by hand"),
        Area::Wash => trf("%{lanes} lanes: washes %{n} buses a night", &[("lanes", &s.wash.to_string()), ("n", &s.washes().to_string())]),
        Area::Fuel => trf("Fills %{n} diesel buses a night", &[("n", &s.diesel_served().to_string())]),
        Area::Charging if s.charging == 0 => tr("None: electric buses charge outside, dearer"),
        Area::Charging => trf("Charges %{n} electric buses a night", &[("n", &s.electric_served().to_string())]),
        Area::Offices => trf("Room for %{n} people", &[("n", &s.staff_room().to_string())]),
    }
}

/// The areas to build (`depot::buildings`): each with its level, what it does, and the next
/// level's price, the building work under way, or complete.
fn buildings(c: &Company) -> Value {
    let mut site = c.site.clone();
    yard::reconcile(&mut site);
    let rows: Vec<Value> = Area::ALL
        .iter()
        .map(|a| {
            let lv = c.site.level(*a);
            let mut row = json!({
                "key": a.key(), "icon": a.icon(), "label": tr(a.label()),
                "level": trf("level %{n} of %{max}", &[("n", &lv.to_string()), ("max", &a.max().to_string())]),
                "lv": lv, "max": a.max(),
                "effect": effect(c, *a),
            });
            row["upgrades"] = json!(upgrades(c, *a));
            if let Some(w) = c.site.works_on(*a).filter(|w| w.upgrade.is_none()) {
                row["works"] = json!(trf("Building until %{date}", &[("date", &day_label(&w.until))]));
            } else if let Some((cost, _)) = dp::next_cost(c, *a) {
                let up = dp::step(*a, lv + 1).upkeep as f64 * c.price_index;
                row["cost"] = json!(eur(cost));
                row["locked"] = json!(!dp::area_allowed(c, *a));
                // (a module more, put down where the player wants it: the build mode)
                row["place"] = json!(yard::needs_place(&site, *a));
                row["tip"] = json!(trf("Build the next level: ready at once, then %{amount} a month", &[("amount", &eur(up.round() as i64))]));
            } else {
                row["complete"] = json!(tr("Complete"));
            }
            row
        })
        .collect();
    // (a new company's building credit, while some is left)
    let credit = (c.site.credit > 0).then(|| json!({ "amount": eur(c.site.credit), "tip": tr("Building credit: what your company was given to build its first depot with, spent before your cash") }));
    let empty = Area::ALL.iter().all(|a| c.site.level(*a) == 0) && c.site.works.is_empty();
    let hint = empty.then(|| tr("Your land is empty: build parking spaces, a workshop, a diesel station and offices to start with."));
    json!({ "title": tr("Build"), "rows": rows, "credit": credit, "hint": hint })
}

/// What can be added to an area (`depot::Upgrade`): each with what it does, and its price, the
/// work under way or that it is there.
fn upgrades(c: &Company, a: Area) -> Vec<Value> {
    Upgrade::ALL
        .iter()
        .filter(|u| u.area() == a)
        .map(|u| {
            let mut row = json!({ "key": u.key(), "icon": u.icon(), "label": tr(u.label()), "what": tr(u.what()), "built": c.site.has(*u) });
            if let Some(w) = c.site.works_for(*u) {
                row["works"] = json!(trf("Building until %{date}", &[("date", &day_label(&w.until))]));
            } else if c.site.level(a) == 0 && !c.site.has(*u) {
                row["needs"] = json!(trf("Build the %{area} first", &[("area", &tr(a.label()).to_lowercase())]));
            } else if let Some((cost, _)) = dp::upgrade_cost(c, *u) {
                row["cost"] = json!(eur(cost));
                row["busy"] = json!(c.site.works_on(a).is_some());
                row["tip"] = json!(tr("Ready at once"));
            }
            row
        })
        .collect()
}

/// The workshop's jobs, then every other bus by its condition, the one picked first
/// (`depot::workshop`).
fn workshop(c: &Company, selected: Option<u32>) -> Value {
    let today = c.date.clone();
    let mut buses: Vec<&co::Vehicle> = c.fleet.iter().filter(|v| v.held_on(&today) && c.site.job_of(v.id).is_none()).collect();
    buses.sort_by(|a, b| (Some(b.id) == selected).cmp(&(Some(a.id) == selected)).then(a.condition.total_cmp(&b.condition)));
    let jobs: Vec<Value> = c
        .site
        .jobs
        .iter()
        .filter_map(|j| {
            let bus = c.vehicle(j.vehicle)?;
            let (what, tone) = match (&j.started, &j.until) {
                (Some(_), Some(u)) => (trf("In a bay until %{date}", &[("date", &day_label(u))]), "accent2"),
                _ => (tr("Waiting for a free bay"), "warn"),
            };
            Some(json!({
                "id": j.id, "number": bus.number, "title": format!("{}  ·  {}", tr(j.kind.label()), bus.name),
                "what": what, "tone": tone, "cost": (j.cost > 0).then(|| eur(j.cost)), "cancel": j.started.is_none(),
            }))
        })
        .collect();
    let rows: Vec<Value> = buses
        .iter()
        .map(|b| {
            let due = b.km >= b.next_service_km - 1_000.0;
            let (sub, tone) = if b.in_workshop(&today) {
                (tr("in the workshop"), "dim")
            } else if due {
                (tr("service due"), "warn")
            } else {
                (format!("{:.0} %", b.condition), "dim")
            };
            let kinds: Vec<Value> = JobKind::ALL
                .iter()
                .map(|k| {
                    let cost = dp::job_cost(c, b.id, *k);
                    let tip = if cost > 0 {
                        trf("%{days} days in a bay, %{amount}", &[("days", &k.days().to_string()), ("amount", &eur(cost))])
                    } else {
                        trf("%{days} days in a bay; paid with the maintenance per kilometre", &[("days", &k.days().to_string())])
                    };
                    json!({ "kind": k.key(), "label": tr(k.label()), "tip": tip })
                })
                .collect();
            json!({
                "id": b.id, "number": b.number, "name": b.name, "sub": sub, "tone": tone,
                "cond": { "frac": b.condition / 100.0, "tone": grade(b.condition), "tip": trf("Condition: %{n} of 100", &[("n", &format!("{:.0}", b.condition))]) },
                "jobs": kinds,
            })
        })
        .collect();
    let empty = (jobs.is_empty() && rows.is_empty()).then(|| tr("No bus yet: the workshop services, repairs and overhauls the fleet's buses, one a bay."));
    json!({ "title": tr("Workshop"), "jobs": jobs, "buses": rows, "empty": empty, "cancel_tip": tr("Take the job back") })
}

/// What stands in the way of building an area before its rules are asked: the level that
/// opens it, or the money.
fn first_refusal(c: &Company, a: Area) -> Option<Answer> {
    let (cost, _) = dp::next_cost(c, a)?;
    refusal(c, a, cost)
}

/// The level that opens an area, or the money (the building credit counted), when they stand in
/// the way of building work of `cost` on it.
fn refusal(c: &Company, a: Area, cost: co::Cents) -> Option<Answer> {
    let casual = c.game_mode.casual();
    if !dp::area_allowed(c, a) {
        let f = if a == Area::Workshop { co::levels::Feature::Workshop } else { co::levels::Feature::ElectricBuses };
        return Some(popup_answer(kit::locked(c, f).json(casual)));
    }
    let need = dp::cash_needed(c, cost);
    if need > 0 && !c.can_pay(need) {
        return Some(popup_answer(kit::no_cash(c, need).json(casual)));
    }
    None
}

fn i32_arg(args: &Value, name: &str) -> i32 {
    args.get(name).and_then(Value::as_i64).unwrap_or(0) as i32
}

pub fn act(s: &mut Session, what: &str, args: &Value) -> Option<Answer> {
    Some(match what {
        "depot_build_at" => {
            let Some(a) = Area::from_key(&str_arg(args, "area")) else { return Some(Ok(json!({}))) };
            let Some(c) = s.company.clone() else { return Some(Ok(json!({}))) };
            if let Some(no) = first_refusal(&c, a) {
                return Some(no);
            }
            let (x, y, rot) = (i32_arg(args, "x"), i32_arg(args, "y"), u32_arg(args, "rot") as u8);
            match apply(s, |c| yard::build_at(c, a, x, y, rot)) {
                Ok(()) => {
                    s.say(trf("Building work has begun: %{what}.", &[("what", &tr(a.label()))]), false);
                    Ok(json!({ "ok": true }))
                }
                Err(p) => popup_answer(p),
            }
        }
        "depot_move" => {
            let (id, x, y, rot) = (u32_arg(args, "id"), i32_arg(args, "x"), i32_arg(args, "y"), u32_arg(args, "rot") as u8);
            match apply(s, |c| yard::move_module(c, id, x, y, rot)) {
                Ok(()) => Ok(json!({ "ok": true })),
                Err(p) => popup_answer(p),
            }
        }
        "depot_demolish" => {
            let id = u32_arg(args, "id");
            match apply(s, |c| yard::demolish(c, id)) {
                Ok(()) => {
                    s.say(tr("It has been pulled down."), false);
                    Ok(json!({ "ok": true }))
                }
                Err(p) => popup_answer(p),
            }
        }
        "depot_land" => {
            let Some(c) = s.company.clone() else { return Some(Ok(json!({}))) };
            if let Some(cost) = yard::land_cost(&c).filter(|cost| !c.can_pay(*cost)) {
                return Some(popup_answer(kit::no_cash(&c, cost).json(c.game_mode.casual())));
            }
            match apply(s, yard::buy_land) {
                Ok(()) => {
                    s.say(tr("The land is yours: the yard is bigger."), false);
                    Ok(json!({ "ok": true }))
                }
                Err(p) => popup_answer(p),
            }
        }
        "depot_build" => {
            let Some(a) = Area::from_key(&str_arg(args, "area")) else { return Some(Ok(json!({}))) };
            let Some(c) = s.company.clone() else { return Some(Ok(json!({}))) };
            if dp::next_cost(&c, a).is_none() {
                return Some(Ok(json!({})));
            }
            // (what is in the way is said first: the level that opens it, or the money)
            if let Some(no) = first_refusal(&c, a) {
                return Some(no);
            }
            match apply(s, |c| dp::build(c, a)) {
                Ok(()) => {
                    let t = trf("Building work has begun: %{what}.", &[("what", &tr(a.label()))]);
                    s.say(t, false);
                    Ok(json!({}))
                }
                Err(p) => popup_answer(p),
            }
        }
        "depot_upgrade" => {
            let Some(u) = Upgrade::from_key(&str_arg(args, "key")) else { return Some(Ok(json!({}))) };
            let Some(c) = s.company.clone() else { return Some(Ok(json!({}))) };
            let Some((cost, _)) = dp::upgrade_cost(&c, u) else { return Some(Ok(json!({}))) };
            if let Some(no) = refusal(&c, u.area(), cost) {
                return Some(no);
            }
            match apply(s, |c| dp::build_upgrade(c, u)) {
                Ok(()) => {
                    s.say(trf("Building work has begun: %{what}.", &[("what", &tr(u.label()))]), false);
                    Ok(json!({ "ok": true }))
                }
                Err(p) => popup_answer(p),
            }
        }
        "depot_look" => {
            let own = |k: &str| args.get(k).and_then(Value::as_str).and_then(dp::colour);
            let finish = Finish::from_key(&str_arg(args, "finish")).unwrap_or_default();
            let look = Look { wall: own("wall"), roof: own("roof"), trim: own("trim"), door: own("door"), finish };
            match apply(s, |c| {
                dp::set_look(c, look);
                Ok::<(), &'static str>(())
            }) {
                Ok(()) => Ok(json!({ "ok": true })),
                Err(p) => popup_answer(p),
            }
        }
        "depot_cancel" => {
            let id = u32_arg(args, "id");
            Ok(apply(s, |c| dp::cancel(c, id)).err().map(|p| json!({ "popup": p })).unwrap_or(json!({})))
        }
        "depot_order" => {
            let id = u32_arg(args, "id");
            let Some(kind) = JobKind::from_key(&str_arg(args, "kind")) else { return Some(Ok(json!({}))) };
            match apply(s, |c| dp::order(c, id, kind)) {
                Ok(_) => {
                    // (an order starts at the night: tomorrow when a bay is free for it then)
                    let started = s.company.as_ref().is_some_and(|c| c.site.job_of(id).is_some_and(|j| j.started.is_some() || dp::starts_tomorrow(c, j.id)));
                    let t = tr(if started { "The bus is in the workshop tomorrow." } else { "The job waits for a free bay." });
                    s.say(t, false);
                    Ok(json!({}))
                }
                Err(p) => popup_answer(p),
            }
        }
        _ => return None,
    })
}
