//! The company's duty launcher (`crates/omsi-launcher-ui/src/company/planning/duty_launch.rs`):
//! one duty of the planning, fixed - the company, the day and its times, the trips in order with
//! the breaks, the deadheads and the relief points, the bus the planning gives the tour, the depot
//! file and the map - what keeps it from being driven, and its start. Only the weather is chosen
//! here. "Start duty" starts the game with exactly the duty (`co::duty_launch::start`), with the
//! command line the game-drawn launcher builds: the page's duty with the company's fixed in it.
//! While the company is shared and players drive at the same time, the boss chooses as they do:
//! alone, or together in the company's world (`together::boss_place`).

use super::kit::Popup;
use super::words::{day_label, tr, trf};
use super::{lines_of, plan_of, week_work, Session};
use crate::launcher as core;
use hub_core::company::duty_launch::{self as dl, DutyRef, Hitch, Row, Sheet};
use hub_core::company::{self as co, Company};
use serde_json::{json, Value};
use std::time::Instant;

/// The launcher open: the duty, the page it was opened from, the weather chosen, and the player's
/// trip reports (read again now and then).
pub struct DutyLaunch {
    pub duty: DutyRef,
    pub from_tab: usize,
    pub weather: String,
    trips: Vec<core::TripRun>,
    trips_read: Option<Instant>,
}

impl DutyLaunch {
    pub fn new(duty: DutyRef, from_tab: usize) -> DutyLaunch {
        DutyLaunch { duty, from_tab, weather: "cycle".into(), trips: Vec::new(), trips_read: None }
    }
}

/// Alone or together (the company shared, players driving at the same time as the duty, or its
/// world open on its day): the start card's two starts instead of one. None: alone is all there
/// is.
fn together_card(s: &mut Session, sheet: &Sheet) -> Option<Value> {
    let (along, world) = crate::together::boss_along(s, &sheet.duty, sheet.from, sheet.to)?;
    let world = world.filter(|w| w.date == sheet.duty.date);
    if along.is_empty() && world.is_none() {
        return None;
    }
    let with_tip = trf(
        "Together: in the company's world, on its clock. You can join it from %{early} min before your duty's start until %{late} min after it; the world closes with the game of whoever opened it.",
        &[("early", &co::together::world::EARLY.to_string()), ("late", &co::together::world::LATE.to_string())],
    );
    // (who drives at the same time, the company's world)
    let said: Vec<String> = [(!along.is_empty()).then(|| crate::together::along_words(&along)), world.as_ref().map(crate::together::world_words)].into_iter().flatten().collect();
    Some(json!({
        "alone": tr("Drive alone"), "with": tr("Drive together"),
        "alone_tip": tr("Single player: the duty in a game of your own, at your own pace."), "with_tip": with_tip,
        "along": said.join("  ·  "),
    }))
}

fn hhmm(m: i32) -> String {
    co::clock::hhmm(m as i64)
}

fn length(minutes: i32) -> String {
    format!("{} h {:02}", minutes / 60, minutes % 60)
}

/// The sheet as it is now (None: the planning is being read), with the hitches the launcher sees
/// besides the company's (the bus not installed).
fn sheet_now(s: &mut Session, c: &Company) -> Option<Sheet> {
    let r = s.launch.as_ref()?.duty.clone();
    let stale = s.launch.as_ref().is_some_and(|x| x.trips_read.is_none_or(|t| t.elapsed().as_secs_f32() > 5.0));
    if stale {
        let trips = core::trips_of(&super::data(), &c.profile);
        if let Some(x) = s.launch.as_mut() {
            x.trips = trips;
            x.trips_read = Some(Instant::now());
        }
    }
    let now = co::clock::now(c);
    let mut sh = if co::dates::between(&c.date, &r.date) < 0 {
        Sheet { duty: r.clone(), company: c.id.clone(), company_name: c.name.clone(), map: c.map.clone(), hitches: vec![Hitch::Over], ..Default::default() }
    } else {
        week_work(s);
        match plan_of(s, &r.date)? {
            Ok(p) => {
                let trips = s.launch.as_ref().map(|x| x.trips.clone()).unwrap_or_default();
                dl::sheet(c, &p, &r, now, &trips)
            }
            Err(_) => Sheet { duty: r.clone(), company: c.id.clone(), company_name: c.name.clone(), map: c.map.clone(), hitches: vec![Hitch::Timetable], ..Default::default() },
        }
    };
    if let Some(b) = sh.bus.as_ref().filter(|b| !b.file.trim().is_empty()) {
        let same = |f: &str| f.replace('\\', "/").eq_ignore_ascii_case(&b.file.replace('\\', "/"));
        let read = s.vehicles_read();
        let file = b.file.clone();
        if read && !s.vehicles().iter().any(|v| same(&v.file)) {
            sh.hitches.push(Hitch::NotInstalled(file));
        }
    }
    Some(sh)
}

/// What a hitch is about in particular.
fn detail(h: &Hitch) -> Option<String> {
    match h {
        Hitch::NoBus(Some(p)) | Hitch::Bus(p) => Some(tr(p.label())),
        Hitch::NotYours(name) => Some(trf("Planned for %{name}", &[("name", &tr(name))])),
        Hitch::NotInstalled(file) => Some(file.clone()),
        _ => None,
    }
}

/// Where the duty is put right: RosterFlow, the planning, or in Casual its shifts.
fn to_planning_label(c: &Company) -> &'static str {
    let shown = super::pages_of(c.game_mode).contains(&5);
    if shown && co::autopilot::rosterflow(c) {
        "Open in RosterFlow"
    } else if shown {
        "Open the planning"
    } else {
        "Back to the shifts"
    }
}

/// The duty launcher's view.
pub fn view(s: &mut Session, _args: &Value) -> Value {
    let Some(c) = s.company.clone() else { return Value::Null };
    let Some(sheet) = sheet_now(s, &c) else {
        return json!({ "reading": tr("Reading the planning…"), "name": c.name, "mark": super::mark_json(&c), "caps": tr("Company duty · fixed by the planning"), "back": tr("Back"), "to": tr(to_planning_label(&c)) });
    };
    let lines = sheet.lines.join(", ");
    let none = sheet.rows.is_empty();
    let tiles = json!([
        [tr("Day").to_uppercase(), day_label(&sheet.duty.date)],
        [tr("Sign on").to_uppercase(), if none { "-".into() } else { hhmm(sheet.from) }],
        [tr("Sign off").to_uppercase(), if none { "-".into() } else { hhmm(sheet.to) }],
        [tr("Length").to_uppercase(), if none { "-".into() } else { length(sheet.to - sheet.from) }],
        [tr("Trips").to_uppercase(), trf("%{n} with passengers · %{m} deadheads", &[("n", &sheet.trips.to_string()), ("m", &sheet.deadheads.to_string())])],
        [tr("Lines").to_uppercase(), if lines.is_empty() { "-".into() } else { lines }],
    ]);
    let colour = co::rosterflow::line_colour(&c, &sheet.duty.line);
    let rows: Vec<Value> = sheet
        .rows
        .iter()
        .map(|row| match row {
            Row::SignOn { at, time, relief } => json!({ "kind": "on", "time": hhmm(*time), "title": tr(if *relief { "Take the bus over (relief)" } else { "Sign on" }), "at": at }),
            Row::SignOff { at, time, relief } => json!({ "kind": "off", "time": hhmm(*time), "title": tr(if *relief { "Hand the bus over (relief)" } else { "Sign off" }), "at": at }),
            Row::Break { at, from, to } => {
                let mins = to - from;
                json!({ "kind": "break", "text": format!("{} · {} · {}", tr(if mins >= 15 { "Break" } else { "Layover" }), trf("%{n} min", &[("n", &mins.to_string())]), at) })
            }
            Row::Trip { dep, arr, line, from, to, empty, .. } => json!({
                "kind": "trip", "dep": hhmm(*dep), "arr": hhmm(*arr), "empty": empty, "line": if line.trim().is_empty() { "?".to_string() } else { line.trim().to_string() },
                "route": format!("{from}  →  {to}"), "deadhead": tr("Deadhead"), "length": trf("%{n} min", &[("n", &(arr - dep).to_string())]),
            }),
        })
        .collect();
    let map = if sheet.map_name.trim().is_empty() { super::words::short_map(&sheet.map) } else { sheet.map_name.clone() };
    let bus_rows: Vec<Value> = match sheet.bus.as_ref() {
        Some(b) => vec![
            json!([tr("Bus"), if b.number.trim().is_empty() { b.name.clone() } else { format!("{} · {}", b.number.trim(), b.name) }]),
            json!([tr("Livery"), if b.paint.trim().is_empty() { tr("The company's") } else { b.paint.clone() }]),
            json!([tr("Number plate"), b.plate.clone()]),
            json!([tr("Depot file"), sheet.hof.clone()]),
            json!([tr("Map"), map]),
        ],
        None => vec![json!([tr("Bus"), tr("Not planned")]), json!([tr("Depot file"), sheet.hof.clone()]), json!([tr("Map"), map])],
    };
    let start = match sheet.hitches.first() {
        Some(h) => {
            let (title, what) = h.words();
            json!({
                "blocked": true, "caps": tr("Cannot start").to_uppercase(), "title": tr(title), "detail": detail(h), "what": tr(what),
                "more": sheet.hitches.iter().skip(1).map(|x| tr(x.words().0)).collect::<Vec<_>>(),
                "fix": tr(to_planning_label(&c)), "off": tr("Start duty").to_uppercase(), "why": trf("Not possible yet: %{reason}", &[("reason", &tr(title))]),
            })
        }
        None => {
            let settings = core::get_settings().unwrap_or_else(|_| core::settings_from_text(None));
            let lead = core::bus_lead(settings.get("bus_lead").and_then(|x| x.as_i64()));
            let first = sheet.planned.iter().find(|x| !x.empty).map(|x| x.dep).unwrap_or(sheet.from);
            let (date, time) = core::lead_start(&sheet.duty.date, first, lead);
            // (the current weather at the map's airport, METAR: not in the add-on)
            let mut weathers = vec![json!(["cycle", tr("As forecast (weather cycle)")]), json!(["", tr("Map default")])];
            weathers.extend(core::list_weather().unwrap_or_default().into_iter().map(|w| json!([w.file, w.name])));
            json!({
                "blocked": false, "caps": tr("Before you start").to_uppercase(), "weather_label": tr("Weather"), "weathers": weathers,
                "weather": s.launch.as_ref().map(|x| x.weather.clone()).unwrap_or_else(|| "cycle".into()),
                "when": trf("The game starts on %{date} at %{time}, %{lead} min before the first departure: the company's clock, not the computer's.", &[("date", &day_label(&date)), ("time", &hhmm(time)), ("lead", &lead.to_string())]),
                "fixed": tr("Nothing of the duty can be changed here: the planning fixes it. Only the weather is yours to choose; sound, camera and traffic are as in the Settings."),
                "go": tr("Start duty"),
                // (alone or together with the players driving at the same time)
                "together": together_card(s, &sheet),
            })
        }
    };
    json!({
        "name": c.name, "mark": super::mark_json(&c), "caps": tr("Company duty · fixed by the planning"),
        "title": (!sheet.number.is_empty()).then(|| format!("{} {}  ·  {} {}  ·  {} {}", tr("Line"), sheet.number, tr("Tour"), sheet.duty.tour, tr("Duty"), sheet.duty.duty + 1)),
        "back": tr("Back"), "to": tr(to_planning_label(&c)), "tiles": tiles,
        // (the duty card of it, printed: `dutycard`)
        "duty": sheet.duty, "print": tr("Print duty card"),
        "duty_caps": tr("The duty as planned").to_uppercase(), "rows": rows, "no_rows": tr("Nothing to show: the planning no longer has this duty."),
        "colour": format!("#{:02x}{:02x}{:02x}", colour[0], colour[1], colour[2]),
        "bus_caps": tr("Bus and depot").to_uppercase(), "bus": bus_rows, "bus_note": tr("The planning gives the duty its bus, in the company's livery: it is not chosen here."),
        "map_caps": tr("Route of the duty").to_uppercase(), "map": c.map, "date": sheet.duty.date, "trips": sheet.trip_names(),
        "start": start,
    })
}

/// "Start duty" (`duty_launch::start`): the game started with exactly the duty - the page's
/// duty (`base`: the player's game options) with everything of the company's fixed in it - or,
/// a dry run (`dry`), the command line it becomes.
fn start(s: &mut Session, args: &Value, p: &dyn crate::Platform) -> Result<Value, String> {
    let Some(c) = s.company.clone() else { return Ok(json!({})) };
    let dry = args.get("dry").and_then(Value::as_bool).unwrap_or(false);
    if !dry && s.in_game() {
        return Ok(
            json!({ "popup": super::popup(s, Popup::new("sports_esports", "A game is running", tr("The duty waits until the game has ended."), tr("Close the game, then start the duty again."), None)) }),
        );
    }
    let root = core::load_config().root;
    if !dry && !hub_omsi::vfs::missing_omsi2_essentials(std::path::Path::new(&root)).is_empty() {
        return Ok(
            json!({ "popup": super::popup(s, Popup::new("folder_open", "The original OMSI 2 is missing", tr("A session needs the original OMSI 2."), tr("Choose its folder under Setup first."), None)) }),
        );
    }
    let Some(sheet) = sheet_now(s, &c) else {
        return Ok(json!({ "popup": super::popup(s, Popup::new("timer", "Not yet", tr("The timetable of the day is still being read."), tr("Try again in a moment."), None)) }));
    };
    let Some(lines) = lines_of(s, &sheet.duty.date) else {
        return Ok(json!({ "popup": super::popup(s, Popup::new("timer", "Not yet", tr("The timetable of the day is still being read."), tr("Try again in a moment."), None)) }));
    };
    let settings = core::get_settings().unwrap_or_else(|_| core::settings_from_text(None));
    let lead = core::bus_lead(settings.get("bus_lead").and_then(|x| x.as_i64()));
    let st = match dl::start(&sheet, &lines, lead) {
        Ok(st) => st,
        Err(h) => return Ok(json!({ "popup": super::popup(s, Popup::new("error", h.words().0, tr(h.words().1), String::new(), None)) })),
    };
    let base: core::Duty = serde_json::from_value(args.get("base").cloned().unwrap_or(Value::Null)).map_err(|e| format!("base: {e}"))?;
    let weather = s.launch.as_ref().map(|x| x.weather.clone()).unwrap_or_else(|| "cycle".into());
    let mut d = st.duty(base, &weather);
    // (together: in the company's world - opened by his game, or joined on its clock; alone: out
    // of it)
    let together = args.get("together").and_then(Value::as_bool).unwrap_or(false) && together_card(s, &sheet).is_some();
    let place = if together && !dry {
        match crate::together::boss_place(&st.date, st.time) {
            Ok(pl) => Some(pl),
            Err(e) => return Ok(json!({ "popup": super::popup(s, Popup::new("groups", "Not together yet", e, tr("Wait a moment, or drive the duty alone."), None)) })),
        }
    } else {
        None
    };
    if let Some(pl) = &place {
        d.lan = Some(pl.lan());
        d.lan_name = Some(c.profile.clone()).filter(|n| !n.trim().is_empty());
        if let co::together::world::Place::Join { minute, .. } = pl {
            d.time = hhmm(*minute);
        }
    } else if !dry {
        crate::together::boss_out_of_world();
    }
    d.display_font = core::busfonts::BusFonts::load().font_for(&st.bus);
    // (the bus options chosen for the bus, as `duty_launch::start` passes them: Omsi-Hub 6/6)
    // (as they were chosen: the livery's own values the bus 3D port's catalogue would leave out)
    d.set_vars = core::busoptions::BusOptions::load().of(&st.bus).into_iter().collect();
    if dry {
        return Ok(json!({ "args": core::duty_args(&d)? }));
    }
    // (a later day's duty: that day's plan for the game too)
    if sheet.duty.date != c.date {
        if let Some(Ok(p)) = plan_of(s, &sheet.duty.date) {
            if let Err(e) = co::plan::save_drive_plan(&super::data(), &co::plan::live_plan(&c, &p)) {
                core::log_to_file(&format!("company: the plan of {} for the game: {e:#}", p.date));
            }
        }
    }
    core::log_to_file(&format!("company duty launcher: {} {} duty {} on {} with {}", st.line, st.tour, sheet.duty.duty + 1, st.date, st.bus));
    // (the add-on's plugin told of the company's duty: `addon.rs`)
    let crew = crew_of(s, &c, &sheet);
    super::plugin::assign_duty(&d, Some((&c.name, &format!("{} {} {}", st.line, st.tour, sheet.duty.duty + 1), plugin_parts(&c, &sheet))), Some(crew), Some(&c.id));
    // (the company's buses for the game's AI: only they are on the road, where the fleet map has them)
    super::fleetgame::for_drive(s, &sheet.duty.date, (&st.line, &st.tour));
    #[cfg(feature = "native")]
    if place == Some(co::together::world::Place::Open) {
        std::thread::spawn(omsi_net::tunnel::ensure_cloudflared);
    }
    let launched = match core::launch(&d, p) {
        Ok(l) => l,
        Err(e) => {
            if place.is_some() {
                crate::together::boss_out_of_world();
            }
            return Err(e);
        }
    };
    core::log_to_file(&format!("game {}: {}", launched.pid, launched.command));
    // (the world his game opens: its code for the players as it comes; out of it when the game ends)
    #[cfg(feature = "native")]
    if let Some(pl) = &place {
        let opens = *pl == co::together::world::Place::Open;
        crate::together::watch_world(
            launched.pid,
            move |code| {
                if opens {
                    crate::together::boss_world_code(code);
                }
            },
            crate::together::boss_out_of_world,
        );
    }
    let t = tr("Starting your company's duty…");
    s.say(t, false);
    Ok(json!({ "launched": serde_json::to_value(launched).unwrap_or(Value::Null) }))
}

/// The driver's side of the duty for the game's plugin (`bridge::Crew`, `lua/duty_strip.lua`): its
/// pieces as the sheet has them, what changed on it today (`crew::changes`), the bus's handover
/// notes of today, and the forms' choices - its words in the launcher's language.
pub fn crew_of(s: &mut Session, c: &Company, sheet: &Sheet) -> hub_game::bridge::Crew {
    use hub_game::bridge::{Crew, StripRow};
    let rows = sheet
        .rows
        .iter()
        .map(|r| match r {
            Row::SignOn { at, time, relief } => StripRow { kind: "sign_on".into(), at: at.clone(), from: *time, to: *time, relief: *relief, ..Default::default() },
            Row::Trip { dep, arr, line, from, to, empty, .. } => {
                StripRow { kind: "trip".into(), at: from.clone(), to_place: to.clone(), from: *dep, to: *arr, line: line.clone(), empty: *empty, ..Default::default() }
            }
            Row::Break { at, from, to } => StripRow { kind: "break".into(), at: at.clone(), from: *from, to: *to, ..Default::default() },
            Row::SignOff { at, time, relief } => StripRow { kind: "sign_off".into(), at: at.clone(), from: *time, to: *time, relief: *relief, ..Default::default() },
        })
        .collect();
    let changes = match plan_of(s, &sheet.duty.date) {
        Some(Ok(p)) => sheet.duty.find(&p).map(|(ti, k)| co::crew::changes(c, &p, ti, k)).unwrap_or_default(),
        _ => Vec::new(),
    };
    let changes = changes
        .iter()
        .map(|ch| {
            let (text, args) = ch.words();
            let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
            trf(text, &args)
        })
        .collect();
    let bus = sheet.bus.as_ref();
    let notes = bus.map(|b| co::crew::notes_for(c, b.id).iter().map(|n| trf(co::crew::NOTE_LEFT, &[("bus", &b.number), ("note", &note_words(n))])).collect()).unwrap_or_default();
    Crew {
        strip: None,
        rows,
        changes,
        bus: bus.map_or(0, |b| b.id),
        bus_number: bus.map(|b| b.number.clone()).unwrap_or_default(),
        notes,
        templates: co::crew::NoteTemplate::ALL.iter().map(|t| (t.key().to_string(), tr(t.text()))).collect(),
        checks: co::crew::DefectKind::ALL.iter().map(|k| (k.key().to_string(), tr(k.label()))).collect(),
        text_max: co::crew::TEXT_MAX as u32,
        low_fuel: 0.15,
    }
}

/// A note's words in the launcher's language (its template translated, the typed text as it is).
pub fn note_words(n: &co::crew::Note) -> String {
    match (n.template, n.text.is_empty()) {
        (Some(t), true) => tr(t.text()),
        (Some(t), false) => format!("{}: {}", tr(t.text()), n.text),
        (None, _) => n.text.clone(),
    }
}

/// The company's side of the duty for the plugin's board in the game: its sheet (signing on,
/// the trips, the breaks, the reliefs, signing off), what service control decided today for its
/// tour within its time, what a boarding brings (where the company keeps the fares) and the
/// line's contract with the authority.
pub(crate) fn plugin_parts(c: &Company, sheet: &Sheet) -> super::plugin::Parts {
    use hub_game::bridge::{ContractNote, ControlNote, Fare, SheetRow};
    let rows = sheet
        .rows
        .iter()
        .map(|r| match r {
            Row::SignOn { at, time, relief } => SheetRow { kind: "sign_on".into(), at: at.clone(), from: *time, to: *time, relief: *relief, ..SheetRow::default() },
            Row::Trip { dep, arr, line, to, empty, .. } => SheetRow { kind: "trip".into(), at: to.clone(), from: *dep, to: *arr, line: line.clone(), empty: *empty, ..SheetRow::default() },
            Row::Break { at, from, to } => SheetRow { kind: "break".into(), at: at.clone(), from: *from, to: *to, ..SheetRow::default() },
            Row::SignOff { at, time, relief } => SheetRow { kind: "sign_off".into(), at: at.clone(), from: *time, to: *time, relief: *relief, ..SheetRow::default() },
        })
        .collect();
    let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
    let controls = if sheet.duty.date == c.date {
        co::control::of_day(c, &sheet.duty.date)
            .into_iter()
            .filter(|x| same(&x.line, &sheet.duty.line) && same(&x.tour, &sheet.duty.tour) && x.dep >= sheet.from && x.dep <= sheet.to)
            .filter_map(|x| {
                let note = |minutes: i32| ControlNote { kind: x.act.key().into(), at: x.at, dep: x.dep, minutes, bus: String::new() };
                Some(match x.act {
                    co::control::Act::ShortTurn { saved, .. } | co::control::Act::Express { saved, .. } | co::control::Act::Layover { saved } => note(saved),
                    co::control::Act::Swap { vehicle, .. } => ControlNote { bus: c.vehicle(vehicle).map(|v| v.number.clone()).filter(|n| !n.trim().is_empty()).unwrap_or_default(), ..note(0) },
                    // (a cover is another driver's: it says nothing to this one)
                    co::control::Act::Cover { .. } => return None,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let line = c.lines.iter().find(|l| same(&l.name, &sheet.duty.line));
    let fare = line.map(|l| co::income::Rates::of(c, l, &sheet.duty.date)).filter(|r| !r.to_authority).map(|r| {
        let look = c.currency.look();
        Fare { per_boarding: c.currency.units(r.per_boarding()), symbol: look.symbol.to_string(), after: matches!(look.at, co::currency::SymbolAt::After), places: look.places as u32 }
    });
    let contract = co::bus_contracts::on_line(c, &sheet.duty.line, &sheet.duty.date).map(|k| ContractNote {
        kind: match k.kind {
            co::bus_contracts::Kind::Gross => "gross",
            co::bus_contracts::Kind::Net => "net",
            co::bus_contracts::Kind::Hire => "hire",
        }
        .into(),
        rag: match k.rag(&sheet.duty.date) {
            co::bus_contracts::Rag::Green => "green",
            co::bus_contracts::Rag::Orange => "orange",
            co::bus_contracts::Rag::Red => "red",
        }
        .into(),
        early: co::econ_uk::ON_TIME_EARLY,
        late: co::econ_uk::ON_TIME_LATE,
    });
    super::plugin::Parts { sheet: rows, controls, fare, contract }
}

/// What the duty launcher does.
pub fn act(s: &mut Session, what: &str, args: &Value, p: &dyn crate::Platform) -> Option<Result<Value, String>> {
    Some(match what {
        "launch_weather" => {
            let w = args.get("weather").and_then(Value::as_str).unwrap_or("cycle").to_string();
            if let Some(x) = s.launch.as_mut() {
                x.weather = w;
            }
            s.generation += 1;
            Ok(json!({}))
        }
        "launch_close" => {
            let from = s.launch.take().map(|x| x.from_tab);
            s.generation += 1;
            Ok(json!({ "tab": from }))
        }
        "launch_fix" => {
            let Some(c) = s.company.clone() else { return Some(Ok(json!({}))) };
            let x = s.launch.take();
            s.generation += 1;
            // (the planning is the game-drawn launcher's until its phase; Casual's are its shifts)
            if super::pages_of(c.game_mode).contains(&5) {
                // Omsi-Hub 4/6: the duty with it, for RosterFlow's Roster to open on it
                // (`duty_launch::to_planning`).
                let duty = x.as_ref().map(|x| json!({ "date": x.duty.date, "line": x.duty.line, "tour": x.duty.tour, "duty": x.duty.duty }));
                Ok(json!({ "hand_over": "5", "date": x.map(|x| x.duty.date), "duty": duty }))
            } else {
                Ok(json!({ "tab": super::casual::SHIFTS }))
            }
        }
        "launch_start" => start(s, args, p),
        _ => return None,
    })
}
