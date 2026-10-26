//! RosterFlow's prints (`crates/omsi-launcher-ui/src/company/planning/rf_print.rs`): the bus
//! graph (a row per vehicle: its trips as bars in the line's colour, the pull-outs and pull-ins
//! black, its duties as a band under it) and the duty graph (a row per duty: its signing, trips,
//! travel and custom work, its vehicles under it), laid out as Optibus's "Bus Graph"
//! (`co::rosterpdf`), and the duty cards (every duty of the day as a British depot prints its
//! cards: every stop with its time, the timing points marked, the reliefs and the breaks, in the
//! company's colours and with its logo), saved where the player chooses (a folder given in
//! `OMSI_ROSTERFLOW_PDF` instead: the screenshot runs check the print so); the page shows its
//! folder then.

use super::super::company::dutycard::{cards_of_day, day_type};
use super::super::company::words::{day_label, tr, trf};
use super::super::company::Session;
use super::{block_label, date_of, ensure, outbound, with_rf, Rf, DAY_NAMES};
use crate::launcher as core;
use crate::Answer;
use hub_core::company::dutycard::{Card, Item};
use hub_core::company::network::TourOfDay;
use hub_core::company::rosterflow::{self as rf, Board, EvKind};
use hub_core::company::rosterpdf::{self as pdf, CardLine, GraphItem, GraphRow, ItemKind};
use hub_core::company::{self as co, Company};
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

/// What is printed: the bus graph, the duty graph, the duty cards (`rf_print`'s `bus` and
/// `cards`).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Print {
    Buses,
    Duties,
    Cards,
}

impl Print {
    fn of(args: &Value) -> Print {
        if args.get("cards").and_then(Value::as_bool) == Some(true) {
            Print::Cards
        } else if args.get("bus").and_then(Value::as_bool).unwrap_or(true) {
            Print::Buses
        } else {
            Print::Duties
        }
    }
}

/// The number of the duty that drives piece `k` of block `bt` (RosterFlow's: "1003").
fn duty_number(board: &Board, bt: usize, k: usize) -> Option<String> {
    co::studio::duty_of(&board.duties, bt, k).and_then(|di| board.shaped.iter().position(|s| s.duty == Some(di))).map(|row| board.shaped[row].label(row))
}

/// A block's trips as a graph's items.
fn block_items(c: &Company, board: &Board, t: &TourOfDay, range: std::ops::Range<usize>) -> Vec<GraphItem> {
    let colour = rf::line_colour(c, &t.line);
    let n = t.trips.len();
    let mut out = Vec::new();
    for j in range {
        let x = &t.trips[j];
        let kind = if x.empty && (j == 0 || j + 1 == n) {
            ItemKind::Pull
        } else if x.empty || !x.counts() {
            ItemKind::Deadhead
        } else {
            ItemKind::Trip { inbound: !outbound(&board.tours, &t.line, x) }
        };
        let label = if x.line.trim().is_empty() { t.number.trim().to_string() } else { x.line.trim().to_string() };
        out.push(GraphItem { from: x.dep, to: x.arr, kind, label, colour, start_stop: pdf::stop_letters(&x.from), end_stop: pdf::stop_letters(x.end()) });
    }
    out
}

/// The bus graph's rows: a vehicle each, its duties under it.
pub(crate) fn bus_rows(c: &Company, board: &Board) -> Vec<GraphRow> {
    let line_at = |l: &str| c.lines.iter().position(|x| x.name.eq_ignore_ascii_case(l)).unwrap_or(usize::MAX);
    let mut idx: Vec<usize> = (0..board.tours.len()).collect();
    idx.sort_by_key(|&i| (line_at(&board.tours[i].line), board.tours[i].from(), i));
    idx.into_iter()
        .map(|i| {
            let t = &board.tours[i];
            let under = co::network::duties_of(t)
                .iter()
                .enumerate()
                .map(|(k, r)| {
                    let (a, b) = rf::span_of(&c.planning.flow.prefs, t, r);
                    (a, b, duty_number(board, i, k).unwrap_or_default())
                })
                .collect();
            GraphRow { name: block_label(t), sub: tr("Line %{n}").replace("%{n}", &t.number), sub2: format!("({:.1} km)", t.km()), items: block_items(c, board, t, 0..t.trips.len()), under }
        })
        .collect()
}

/// The duty graph's rows: a duty each, its vehicles under it.
pub(crate) fn duty_rows(c: &Company, board: &Board) -> Vec<GraphRow> {
    let p = &c.planning.flow.prefs;
    board
        .shaped
        .iter()
        .enumerate()
        .map(|(row, s)| {
            let mut items: Vec<GraphItem> = Vec::new();
            for e in &s.events {
                let (kind, label) = match e.kind {
                    EvKind::SignOn | EvKind::SignOff => (ItemKind::Sign, String::new()),
                    EvKind::Travel(m) => (ItemKind::Travel, m.letter().to_string()),
                    EvKind::Custom(k) => (ItemKind::Custom, s.custom.get(k).map(|x| tr(&x.kind)).unwrap_or_default()),
                };
                items.push(GraphItem { from: e.from, to: e.to, kind, label, colour: [255, 255, 255], start_stop: String::new(), end_stop: String::new() });
            }
            let mut under = Vec::new();
            for sp in &s.spans {
                let t = &board.tours[sp.tour];
                let r = co::network::duties_of(t).get(sp.duty).cloned().unwrap_or(0..0);
                items.extend(block_items(c, board, t, r));
                under.push((sp.from, sp.to, block_label(t)));
            }
            items.sort_by_key(|x| x.from);
            GraphRow { name: s.label(row), sub: tr(s.kind_name(p)), sub2: format!("({} {})", tr("paid"), rf::hm(s.paid)), items, under }
        })
        .collect()
}

/// The PDF of the bus graph (`bus`) or the duty graph of `date`.
pub(crate) fn pdf_of(c: &Company, board: &Board, date: &str, bus: bool) -> Vec<u8> {
    let rows = if bus { bus_rows(c, board) } else { duty_rows(c, board) };
    let title = tr(if bus { "Bus Graph" } else { "Duty Graph" });
    let info = vec![
        tr("Depot: %{d}").replace("%{d}", c.depot.trim_end_matches(".hof")),
        // (the date's weekday: the board's is its plan's slot, a date's own plan counted from 7 on)
        tr("Service: %{s}").replace("%{s}", &tr(DAY_NAMES[co::plan::weekday_of(date) as usize % 7])),
        tr("Date: %{d}").replace("%{d}", date),
    ];
    let printed = format!("{}: {}  ·  {}", tr("Printed"), c.date, c.name);
    pdf::graph(&title, &info, &printed, &tr("Page"), &rows)
}

/// Minutes of the day as a card writes a time: "0525".
fn hhmm(m: i32) -> String {
    let m = m.rem_euclid(24 * 60);
    format!("{:02}{:02}", m / 60, m % 60)
}

/// Minutes as a card writes a length: "06:26".
fn length(m: i32) -> String {
    let m = m.max(0);
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// The number of a trip with passengers of block `t` leaving at `dep` (1 on), as the block
/// runs them.
fn trip_number(t: &TourOfDay, dep: i32) -> Option<usize> {
    t.trips.iter().filter(|x| x.counts()).position(|x| x.dep <= dep && dep < x.arr.max(x.dep + 1)).map(|k| k + 1)
}

/// A duty's card as the duty cards print it (`co::rosterpdf::duty_cards`): its head, its
/// figures, and its lines - the running number of every piece, every trip with its stops and
/// their times (the timing points marked), the reliefs with the duties before and after, the
/// breaks and the driver's travel in them.
pub(crate) fn card_print(c: &Company, board: &Board, card: &Card) -> pdf::DutyCard {
    let colon = |w: &str| format!("{}:", tr(w));
    let stop = |name: &str, time: i32, timing: bool| CardLine::Stop { name: name.trim().to_string(), time: hhmm(time), timing };
    let signing = |word: &str, at: &str| if at.trim().is_empty() { tr(word) } else { format!("{}: {}", tr(word), at.trim()) };
    let brk = |a: i32, b: i32| CardLine::Centre(format!("{} {} - {}", tr("Break").to_uppercase(), hhmm(a), hhmm(b)));
    let mut lines = Vec::new();
    let n = card.pieces.len();
    for (i, x) in card.pieces.iter().enumerate() {
        let bt = board.tours.iter().position(|t| t.line.eq_ignore_ascii_case(&x.line) && t.tour.trim() == x.tour.trim());
        if i == 0 {
            lines.push(CardLine::Centre(tr("Duty start")));
        }
        let running = bt.map(|bt| block_label(&board.tours[bt])).unwrap_or_else(|| format!("{} {}", x.number.trim(), x.tour.trim()));
        lines.push(CardLine::Head(tr("Running no."), running));
        if let Some(b) = x.bus.as_ref() {
            let bus = [b.number.trim(), b.plate.trim()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · ");
            if !bus.is_empty() {
                lines.push(CardLine::Stop { name: format!("{} {bus}", tr("Bus")), time: String::new(), timing: false });
            }
        }
        for it in &x.items {
            match it {
                Item::SignOn { at, time, relief } => {
                    let before = bt.zip(x.duty.checked_sub(1)).and_then(|(bt, k)| duty_number(board, bt, k));
                    if let Some(d) = before.filter(|_| *relief) {
                        lines.push(CardLine::Note(trf("Take over from duty %{d}", &[("d", &d)])));
                    }
                    // (the duty's own signing on, as its head has it: the checks before the bus in)
                    let (at, time) = if i == 0 && !relief { (&card.sign_on_at, card.sign_on) } else { (at, *time) };
                    lines.push(stop(&signing("Sign on", at), time, false));
                }
                Item::Trip { dep, arr, line, from, to, empty, stops, .. } => {
                    if *empty {
                        lines.push(CardLine::Sub(tr("Deadhead").to_uppercase()));
                        lines.push(stop(from, *dep, false));
                        lines.push(stop(to, *arr, false));
                        continue;
                    }
                    let head = match bt.and_then(|bt| trip_number(&board.tours[bt], *dep)) {
                        Some(no) => trf("Line %{l} - Trip %{n}", &[("l", line), ("n", &no.to_string())]),
                        None => tr("Line %{n}").replace("%{n}", line),
                    };
                    lines.push(CardLine::Sub(head));
                    if stops.is_empty() {
                        lines.push(stop(from, *dep, true));
                        lines.push(stop(to, *arr, true));
                    }
                    for (j, s) in stops.iter().enumerate() {
                        lines.push(stop(&s.name, if j + 1 == stops.len() { s.arr } else { s.dep }, s.timing));
                    }
                }
                // (a layover is no line of its own: the next trip's heading follows)
                Item::Break { from, to, .. } if to - from >= 15 => lines.push(brk(*from, *to)),
                Item::Break { .. } => {}
                Item::SignOff { at, time, relief } => {
                    if *relief {
                        if let Some(next) = bt.and_then(|bt| board.tours[bt].trips.iter().find(|t| t.dep >= *time)) {
                            lines.push(CardLine::Note(trf("Bus continues to %{to}", &[("to", next.to.trim())])));
                        }
                        if let Some(d) = bt.and_then(|bt| duty_number(board, bt, x.duty + 1)) {
                            lines.push(CardLine::Note(trf("Handover to duty %{d}", &[("d", &d)])));
                        }
                    }
                    let (at, time) = if i + 1 == n && !relief { (&card.sign_off_at, card.sign_off) } else { (at, *time) };
                    lines.push(stop(&signing("Sign off", at), time, false));
                }
            }
        }
        if let Some(g) = &x.after {
            lines.push(brk(g.from, g.to));
            for (what, a, b) in &g.travel {
                lines.push(CardLine::Stop { name: tr(what), time: format!("{}-{}", hhmm(*a), hhmm(*b)), timing: false });
            }
        }
        if i + 1 == n {
            lines.push(CardLine::Centre(tr("Duty end")));
        }
    }
    // (the longest break: between the pieces, or one of 15 minutes or more on the bus)
    let gaps = card.pieces.iter().filter_map(|x| x.after.as_ref().map(|g| (g.from, g.to)));
    let on_bus = card.pieces.iter().flat_map(|x| &x.items).filter_map(|i| match i {
        Item::Break { from, to, .. } if to - from >= 15 => Some((*from, *to)),
        _ => None,
    });
    let meal = gaps.chain(on_bus).max_by_key(|(a, b)| b - a).map(|(a, b)| format!("{}-{}", hhmm(a), hhmm(b))).unwrap_or_else(|| "-".into());
    let buses: Vec<String> = card.pieces.iter().filter_map(|x| x.bus.as_ref()).map(|b| b.number.trim().to_string()).filter(|b| !b.is_empty()).fold(Vec::new(), |mut v, b| {
        if !v.contains(&b) {
            v.push(b);
        }
        v
    });
    let more = [
        (colon("Length"), length(card.length())),
        (colon("Duty type"), card.kind.as_deref().map(tr).unwrap_or_default()),
        (colon("Lines"), card.lines.join(", ")),
        (colon("Bus"), buses.join(", ")),
    ];
    let depot = card.depots().first().cloned().unwrap_or_else(|| c.depot.clone());
    pdf::DutyCard {
        depot: depot.strip_suffix(".hof").unwrap_or(&depot).to_string(),
        day: (colon("Day type"), day_type(card.day_group)),
        date: (colon("Date"), day_label(&card.date)),
        number: (colon("Duty no."), card.number.clone().unwrap_or_else(|| (card.duty.duty + 1).to_string())),
        figures: vec![
            (colon("Sign on"), hhmm(card.sign_on)),
            (colon("Sign off"), hhmm(card.sign_off)),
            (colon("Break"), meal),
            (colon("Driving time"), length(card.driving)),
            (colon("Paid"), length(card.paid.unwrap_or(card.length() - card.breaks))),
        ],
        more: more.into_iter().filter(|(_, v)| !v.trim().is_empty()).collect(),
        lines,
    }
}

/// Bytes packed as a PDF's FlateDecode takes them.
fn deflated(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(bytes).ok()?;
    z.finish().ok()
}

/// The company's logo picture as a card shows it: a JPEG as it is, a PNG made at most 600
/// pixels across; another kind none (the card shows the monogram then).
fn logo_image(path: &str) -> Option<pdf::Image> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.starts_with(&[0xFF, 0xD8]) {
        // (its size and colours from its frame's header: a marker SOF0 to SOF15 but DHT, JPG and DAC)
        let mut at = 2;
        while at + 9 < bytes.len() && bytes[at] == 0xFF {
            let marker = bytes[at + 1];
            let len = u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]) as usize;
            if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
                let h = u16::from_be_bytes([bytes[at + 5], bytes[at + 6]]) as u32;
                let w = u16::from_be_bytes([bytes[at + 7], bytes[at + 8]]) as u32;
                let components = bytes[at + 9];
                return (w > 0 && h > 0 && matches!(components, 1 | 3)).then(|| pdf::Image { w, h, components, filter: "DCTDecode", data: bytes.clone(), alpha: None });
            }
            at += 2 + len;
        }
        return None;
    }
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let scale = (600.0 / w.max(h).max(1) as f32).min(1.0);
    let img = if scale < 1.0 {
        let (nw, nh) = (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1));
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let (w, h) = img.dimensions();
    let rgb: Vec<u8> = img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let alpha: Vec<u8> = img.pixels().map(|p| p[3]).collect();
    let alpha = if alpha.iter().all(|&a| a == 255) { None } else { Some(deflated(&alpha)?) };
    Some(pdf::Image { w, h, components: 3, filter: "FlateDecode", data: deflated(&rgb)?, alpha })
}

/// The company's look on the cards: its name, its monogram, its colours, its logo picture.
fn brand_of(c: &Company) -> pdf::Brand {
    let colour = |k: usize, or: [u8; 3]| rf::parse_hex(&c.colours[k]).unwrap_or(or);
    pdf::Brand { name: c.name.clone(), short: c.short.clone(), colours: [colour(0, [0, 61, 130]), colour(1, [255, 214, 0])], logo: c.logo.as_deref().and_then(logo_image) }
}

/// The duty cards of `cards` as a PDF.
pub(crate) fn cards_pdf(c: &Company, board: &Board, cards: &[Card]) -> Vec<u8> {
    let printed: Vec<pdf::DutyCard> = cards.iter().map(|x| card_print(c, board, x)).collect();
    let words = pdf::CardWords { legend: tr("Timing point: do not leave the stop before the time shown"), page: tr("Page"), more: tr("The duty goes on on the next page") };
    pdf::duty_cards(&printed, &brand_of(c), &words)
}

/// The day shown as a PDF file: its file name and its bytes, or what to say instead.
fn made(s: &mut Session, r: &mut Rf, date: &str, what: Print) -> Result<(String, Vec<u8>), Value> {
    let reading = || json!({ "say": { "text": tr("Reading the timetable…"), "error": false } });
    let k = match ensure(s, r, date) {
        Some(Ok(k)) => k,
        Some(Err(e)) => return Err(json!({ "say": { "text": e, "error": true } })),
        None => return Err(reading()),
    };
    let name = |kind: &str| format!("{kind}-{date}.pdf");
    match what {
        Print::Buses => Ok((name("bus-graph"), pdf_of(&k.view, &k.board, date, true))),
        Print::Duties => Ok((name("duty-graph"), pdf_of(&k.view, &k.board, date, false))),
        Print::Cards => {
            let c = s.company.clone().ok_or_else(reading)?;
            let cards = cards_of_day(s, &c, &k.board, date).ok_or_else(reading)?;
            if cards.is_empty() {
                return Err(json!({ "say": { "text": tr("This day has no duties to print"), "error": true } }));
            }
            Ok((name("duty-cards"), cards_pdf(&c, &k.board, &cards)))
        }
    }
}

/// Print the day shown (`rf_print`, `day`, and `bus` or `cards`): saved where the player
/// chooses, as the game-drawn launcher saves it (a folder given in `OMSI_ROSTERFLOW_PDF` takes
/// the file without asking: the screenshot runs and the checks print so). The dialog is shown
/// with the company let go, so that its clock and the page go on meanwhile; the page shows the
/// folder then.
pub(crate) fn print(args: &Value) -> Answer {
    let what = Print::of(args);
    let made = with_rf(|s, r| {
        let date = date_of(s, args).ok_or("no day")?;
        Ok::<_, String>(made(s, r, &date, what))
    })?;
    let title = tr(if what == Print::Cards { "Save the duty cards as a PDF file" } else { "Save the graph as a PDF file" });
    match made {
        Ok((name, bytes)) => Ok(save_as(&title, &name, &bytes)),
        Err(said) => Ok(said),
    }
}

/// The day shown as a PDF file for a player of the company's session (`rf_print` relayed): its
/// file name and its bytes (base64), saved on his own launcher (`save`).
pub(super) fn for_afar(s: &mut Session, r: &mut Rf, args: &Value) -> Answer {
    let date = date_of(s, args).ok_or("no day")?;
    Ok(match made(s, r, &date, Print::of(args)) {
        Ok((name, bytes)) => json!({ "pdf": crate::launcher::base64_encode(&bytes), "name": name }),
        Err(said) => said,
    })
}

/// A graph's PDF file saved where the player chooses (or in `OMSI_ROSTERFLOW_PDF`): what the
/// page shows then.
pub(crate) fn save(name: &str, bytes: &[u8]) -> Value {
    save_as(&tr("Save the graph as a PDF file"), name, bytes)
}

/// A PDF file saved where the player chooses, the dialog titled `title` (or in
/// `OMSI_ROSTERFLOW_PDF`): what the page shows then.
fn save_as(title: &str, name: &str, bytes: &[u8]) -> Value {
    let given = std::env::var("OMSI_ROSTERFLOW_PDF").ok().filter(|d| !d.trim().is_empty());
    let path = match &given {
        Some(dir) => PathBuf::from(dir).join(name),
        None => match core::save_file_as(title, name, "PDF", &["pdf"]) {
            Some(p) => p,
            None => return json!({}),
        },
    };
    let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, bytes));
    match written {
        Ok(()) => json!({
            "printed": path.to_string_lossy(),
            "folder": path.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default(),
            "reveal": given.is_none(),
            "say": { "text": tr("Saved: %{file}").replace("%{file}", &path.display().to_string()), "error": false },
        }),
        Err(e) => json!({ "say": { "text": format!("{}: {e}", tr("Could not save the file")), "error": true } }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_core::company::network::PlannedTrip;

    #[test]
    fn a_graph_is_a_pdf_with_a_row_for_every_bus() {
        let trip = |a: &str, b: &str, dep: i32| PlannedTrip { line: "4".into(), from: a.into(), to: b.into(), stops: 6, dep, arr: dep + 30, origin: "1".into(), ..Default::default() };
        let t = TourOfDay { line: "Linie4".into(), number: "4".into(), tour: "1".into(), trips: vec![trip("Nord", "Süd", 360), trip("Süd", "Nord", 400)], ..Default::default() };
        let c = hub_core::company::found(&hub_core::company::Founding { name: "Ziel".into(), date: "2024-05-06".into(), ..Default::default() }, "Luc");
        let b = rf::board_of(&c, 0, vec![t], Vec::new());
        assert_eq!(bus_rows(&c, &b).len(), 1);
        assert_eq!(bus_rows(&c, &b)[0].name, "4/01");
        let bytes = pdf_of(&c, &b, "2024-05-06", true);
        assert!(bytes.starts_with(b"%PDF"));
        assert!(!duty_rows(&c, &b).is_empty());
    }

    /// A duty's card as the duty cards print it: the depot, the duty's number, its running
    /// number, its trips numbered as the block runs them with every stop and its time, the
    /// timing points marked; the company's logo picture on it.
    #[test]
    fn a_duty_card_prints_every_stop_the_timing_points_and_the_running_number() {
        use hub_core::company::duty_launch::DutyRef;
        use hub_core::company::dutycard::{self as dc, CardStop, Lookups};
        use hub_core::company::model::CompanyLine;
        use hub_core::company::plan::day_plan;
        super::super::super::company::words::set_language("en");
        let trip = |name: &str, dep: i32, arr: i32, from: &str, to: &str, empty: bool| PlannedTrip {
            name: name.into(),
            line: if empty { String::new() } else { "130".into() },
            from: from.into(),
            to: to.into(),
            dep,
            arr,
            km: 8.0,
            stops: if empty { 0 } else { 3 },
            empty,
            ..Default::default()
        };
        let mut c = hub_core::company::found(&hub_core::company::Founding { name: "Stadtbus Grundorf".into(), date: "2024-03-04".into(), ..Default::default() }, "Luc");
        c.depot = "Grundorf.hof".into();
        c.colours = ["#d4002a".into(), "#ffffff".into()];
        c.lines.push(CompanyLine { name: "Linie130".into(), number: "130".into(), numbers: vec!["130".into()], added: c.date.clone(), service_from: Some(0), ..Default::default() });
        let t = TourOfDay {
            line: "Linie130".into(),
            number: "130".into(),
            tour: "1".into(),
            trips: vec![trip("Depot-A", 350, 358, "Depot", "Station", true), trip("130 A-B", 360, 390, "Station", "Zentrum", false), trip("130 B-A", 400, 430, "Zentrum", "Station", false), trip("A-Depot", 435, 445, "Station", "Depot", true)],
            ..Default::default()
        };
        let p = day_plan(&c, &c.date, vec![t.clone()], &[], &[], false);
        let board = rf::board_of(&c, 0, vec![t], Vec::new());
        let stops = |x: &PlannedTrip, _: &str, _: &str| {
            Some(vec![
                CardStop { id: 1, name: x.from.clone(), arr: x.dep, dep: x.dep, hold: true, ..Default::default() },
                CardStop { id: 2, name: "Markt".into(), arr: x.dep + 12, dep: x.dep + 12, ..Default::default() },
                CardStop { id: 3, name: x.to.clone(), arr: x.arr, dep: x.arr, hold: true, ..Default::default() },
            ])
        };
        let look = Lookups { stops: &stops, ibis: &|_, _, _, _| None, design: &|_| None };
        let card = dc::card(&c, &p, Some(&board), &DutyRef::of(&p, 0, 0).unwrap(), 0, "Luc", &look).unwrap();
        let printed = card_print(&c, &board, &card);
        assert_eq!((printed.depot.as_str(), printed.number.1.as_str()), ("Grundorf", "1001"));
        assert_eq!(printed.figures[0], ("Sign on:".to_string(), hhmm(card.sign_on)));
        let l = &printed.lines;
        assert_eq!(l[0], CardLine::Centre("Duty start".into()));
        assert_eq!(l[1], CardLine::Head("Running no.".into(), "130/01".into()));
        // signing on as the head has it (RosterFlow's: the check before the bus in)
        assert_eq!(l[2], CardLine::Stop { name: format!("Sign on: {}", card.sign_on_at), time: hhmm(card.sign_on), timing: false });
        assert!(l.contains(&CardLine::Sub("DEADHEAD".into())));
        // the trips numbered as the block runs them, every stop with its time, the ends timing points
        let at = l.iter().position(|x| *x == CardLine::Sub("Line 130 - Trip 2".into())).unwrap();
        assert_eq!(l[at + 1], CardLine::Stop { name: "Zentrum".into(), time: "0640".into(), timing: true });
        assert_eq!(l[at + 2], CardLine::Stop { name: "Markt".into(), time: "0652".into(), timing: false });
        assert_eq!(l[at + 3], CardLine::Stop { name: "Station".into(), time: "0710".into(), timing: true });
        assert_eq!(l.last(), Some(&CardLine::Centre("Duty end".into())));
        // the logo picture (a PNG, part of it see-through) packed into the file
        let logo = std::env::temp_dir().join(format!("omsihub-logo-{}.png", std::process::id()));
        image::RgbaImage::from_fn(800, 240, |x, y| if (x / 40 + y / 40) % 2 == 0 { image::Rgba([212, 0, 42, 255]) } else { image::Rgba([0, 0, 0, 0]) }).save(&logo).unwrap();
        c.logo = Some(logo.to_string_lossy().to_string());
        let img = brand_of(&c).logo.unwrap();
        assert_eq!((img.w, img.h, img.alpha.is_some()), (600, 180, true));
        let bytes = cards_pdf(&c, &board, &[card.clone(), card]);
        let _ = std::fs::remove_file(&logo);
        assert!(bytes.starts_with(b"%PDF"));
        if let Ok(p) = std::env::var("DUTY_CARDS_PDF_OUT") {
            std::fs::write(p, &bytes).unwrap();
        }
    }
}
