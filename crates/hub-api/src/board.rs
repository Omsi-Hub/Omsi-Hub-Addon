//! The company board, the app's side (docs/ONLINE.md, "The company board"; the website's "Top
//! companies"): a player's company on the public board, its figures (`publish_company`) kept up to
//! date, or taken off it again (`unpublish_company`). Off until the player switches it on, in the
//! company's settings or in the account panel beside the cloud saves; at most 5 companies of an
//! account at a time.
//!
//! The choice is kept in the app's own data folder (`omsihub-board.json`: the company's id and the
//! account it was put on the board with), not in the company's file: a company signed in as another
//! account is off here. A company's key on the board is made from its id ([`key_of`]), so the same
//! company - a cloud save played on another PC too - is one row of the board.
//!
//! The figures (`entry_of`): the name, the short name, the main colour, the country, the map, the
//! game mode and Career or Sandbox, the currency, the company's days, its cash, this month's
//! result and passengers, its buses, lines and staff, its trips on time and its reputation. The
//! online worker sends them again whenever the company's file changed and they with it
//! ([`report_due`], at most every five minutes); the board shows companies told within 30 days.
//!
//! The commands (through `online`): `ol_companies` (the driver's companies, each on or off) and
//! `ol_company_board {id, on}`; the company's settings call [`settings_part`] and [`set`].

use crate::online::{self, tr, trf};
use hub_core::company::{self as co, Company};
use hub_online::saves::sha256_hex;
use hub_online::{BoardCompany, Error};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Instant, SystemTime};

/// The companies of an account the board takes at a time (the database's limit).
pub const MAX: usize = 5;
/// How often a company's changed figures are sent again, at most.
const EVERY_SECS: u64 = 5 * 60;

const FILE: &str = "omsihub-board.json";

fn data() -> PathBuf {
    hub_core::data_dir()
}

// --- the choice ---------------------------------------------------------------------------------

/// The companies on the board: the company's id → the account (user id) it was put there with.
fn read_book(data: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(data.join(FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_book(data: &Path, book: &BTreeMap<String, String>) {
    let _ = std::fs::write(data.join(FILE), serde_json::to_string_pretty(book).unwrap_or_default());
}

/// The ids of the companies `user` has on the board.
fn shown(data: &Path, user: &str) -> Vec<String> {
    read_book(data).into_iter().filter(|(_, u)| !user.is_empty() && u == user).map(|(id, _)| id).collect()
}

/// The account signed in (empty: none), as the online worker last saw it.
fn user() -> String {
    online::account_user()
}

/// Company `id`'s key on the board: the same for the same company on every PC, and nothing of its
/// name in it.
pub fn key_of(id: &str) -> String {
    sha256_hex(format!("omsihub-board:{id}").as_bytes())[..32].to_string()
}

// --- the figures ------------------------------------------------------------------------------

/// `text` at most `n` characters long, trimmed (None: nothing left).
fn cut(text: &str, n: usize) -> Option<String> {
    let t: String = text.trim().chars().take(n).collect();
    Some(t.trim().to_string()).filter(|t| !t.is_empty())
}

/// What the board is told of company `c`.
pub fn entry_of(c: &Company) -> BoardCompany {
    let month = c.date.get(..7).unwrap_or("");
    let colour = c.colours[0].trim();
    let colour = (colour.len() == 7 && colour.starts_with('#') && colour[1..].chars().all(|x| x.is_ascii_hexdigit())).then(|| colour.to_string());
    let country = c.country.trim().to_ascii_uppercase();
    let country = (country.len() == 2 && country.chars().all(|x| x.is_ascii_uppercase())).then_some(country);
    let passengers: u64 = c.history.iter().filter(|d| !month.is_empty() && d.date.starts_with(month)).map(|d| u64::from(d.passengers)).sum();
    let money = |cents: i64| cents.clamp(-1_000_000_000_000, 1_000_000_000_000);
    let count = |n: usize| (n as u32).min(10_000);
    let pct = |x: f64| x.is_finite().then(|| x.clamp(0.0, 100.0) as f32);
    BoardCompany {
        key: key_of(&c.id),
        name: cut(&c.name, 40).unwrap_or_default(),
        short: cut(&c.short, 6),
        colour,
        country,
        map_name: cut(&c.map_name, 80),
        mode: crate::company::mode_key(c.game_mode).to_string(),
        sandbox: c.company_mode == co::sandbox::CompanyMode::Sandbox,
        currency: Some(c.currency.code().to_string()),
        days: co::dates::between(&c.founded, &c.date).clamp(0, 1_000_000) as u32,
        cash: money(c.cash),
        profit_month: money(c.month(month).result()),
        buses: count(c.fleet.iter().filter(|v| v.held_on(&c.date)).count()),
        lines: count(c.lines.len()),
        staff: count(c.staff.iter().filter(|e| e.employed_on(&c.date)).count()),
        passengers_month: passengers.min(10_000_000_000),
        on_time: pct(c.punctuality),
        reputation: pct(c.reputation),
    }
}

/// What was last sent of each company on the board: its file's time then, when, and the figures.
struct Sent {
    modified: Option<SystemTime>,
    at: Instant,
    entry: BoardCompany,
}

static SENT: LazyLock<Mutex<HashMap<String, Sent>>> = LazyLock::new(Default::default);

fn sent() -> std::sync::MutexGuard<'static, HashMap<String, Sent>> {
    SENT.lock().unwrap_or_else(|e| e.into_inner())
}

fn modified(data: &Path, id: &str) -> Option<SystemTime> {
    std::fs::metadata(co::store::path_of(data, id)).and_then(|m| m.modified()).ok()
}

/// Company `id` told to the board now.
fn publish(data: &Path, id: &str) -> Result<String, Error> {
    let c = co::store::load(data, id).map_err(|_| Error::Code("not_found"))?;
    let entry = entry_of(&c);
    if entry.name.chars().count() < 2 {
        return Err(Error::Code("bad_input"));
    }
    online::with(|cl| cl.publish_company(&entry))?;
    sent().insert(id.to_string(), Sent { modified: modified(data, id), at: Instant::now(), entry });
    Ok(c.name)
}

/// The online worker's beat: the account's companies on the board whose figures changed since they
/// were last sent, sent again. Signed out, waiting for the privacy statement, or another driver
/// than the account's driving, nothing is sent.
pub(crate) fn report_due() {
    if !online::account_ready() || !crate::drivers::plays_online() {
        return;
    }
    let d = data();
    for id in shown(&d, &user()) {
        let now = modified(&d, &id);
        {
            let s = sent();
            if let Some(last) = s.get(&id) {
                if last.modified == now || last.at.elapsed().as_secs() < EVERY_SECS {
                    continue;
                }
            }
        }
        // (the same figures: only the file's time is new)
        let same = co::store::load(&d, &id).ok().map(|c| entry_of(&c)).is_some_and(|e| sent().get(&id).is_some_and(|s| s.entry == e));
        if same {
            if let Some(s) = sent().get_mut(&id) {
                s.modified = now;
            }
            continue;
        }
        match publish(&d, &id) {
            Ok(_) => {}
            Err(Error::Offline | Error::SignedOut) => return,
            // (refused: tried again in five minutes, not at every beat)
            Err(_) => {
                sent().insert(id, Sent { modified: None, at: Instant::now(), entry: BoardCompany::default() });
            }
        }
    }
}

// --- switched on and off ------------------------------------------------------------------------

/// Why the switch cannot be used now (None: it can): signed out (or the account waiting for the
/// privacy statement or its display name), or another driver's account.
fn blocked() -> Option<String> {
    if !crate::drivers::plays_online() || !online::account_ready() || user().is_empty() {
        return Some(tr("Sign in to your Omsi-Hub account to show this company on the leaderboard."));
    }
    None
}

/// Company `id` on the board (`on`) or off it: `{ok, message}` or `{error, code}`.
pub fn set(id: &str, on: bool) -> Value {
    if blocked().is_some() {
        return online::fail(Error::Code("not_signed_in"));
    }
    let d = data();
    let me = user();
    let mut book = read_book(&d);
    if on {
        if shown(&d, &me).iter().filter(|x| *x != id).count() >= MAX {
            return online::fail(Error::Code("board_full"));
        }
        match publish(&d, id) {
            Ok(name) => {
                book.insert(id.to_string(), me);
                write_book(&d, &book);
                json!({ "ok": true, "message": trf("%{name} is on the company board now.", &[("name", &name)]) })
            }
            Err(e) => online::fail(e),
        }
    } else {
        match online::with(|c| c.unpublish_company(&key_of(id))) {
            Ok(()) => {
                book.remove(id);
                write_book(&d, &book);
                sent().remove(id);
                let name = co::store::load(&d, id).map(|c| c.name).unwrap_or_default();
                json!({ "ok": true, "message": trf("%{name} is off the company board.", &[("name", &name)]) })
            }
            Err(e) => online::fail(e),
        }
    }
}

/// Whether company `id` is on the board for the account signed in.
fn is_on(data: &Path, id: &str) -> bool {
    let me = user();
    !me.is_empty() && read_book(data).get(id).is_some_and(|u| *u == me)
}

/// The company deleted: off the board too (in the background; offline it stays there until the
/// board's 30 days are over).
pub fn deleted(id: &str) {
    let d = data();
    let mut book = read_book(&d);
    if book.remove(id).is_none() {
        return;
    }
    write_book(&d, &book);
    sent().remove(id);
    let key = key_of(id);
    let _ = std::thread::Builder::new().name("omsihub board".into()).spawn(move || {
        let _ = online::with(|c| if c.signed_in() { c.unpublish_company(&key) } else { Ok(()) });
    });
}

/// The account deleted (its rows went with it): every company of it is off here.
pub fn forget_account(user: Option<&str>) {
    let d = data();
    let mut book = read_book(&d);
    book.retain(|_, u| user.is_some_and(|me| u != me));
    write_book(&d, &book);
    sent().clear();
}

// --- what the pages show ----------------------------------------------------------------------

/// The company settings' part: the switch for company `id`.
pub fn settings_part(id: &str) -> Value {
    let on = is_on(&data(), id);
    let off = blocked();
    json!({
        "caps": tr("Online leaderboard").to_uppercase(), "on": on && off.is_none(), "disabled": off.is_some(),
        "label": tr("Show this company on the online leaderboard"),
        "say": off.unwrap_or_else(|| tr(if on {
            "Its name, colour, map and figures (this month's profit and passengers, cash, buses, lines, staff, trips on time and reputation) are on omsihub.com, with your display name."
        } else {
            "Off: nothing of this company leaves this PC. At most 5 of your companies can be on the board."
        })),
        "url": format!("{}/leaderboard.html", hub_online::SITE), "open": tr("Open the leaderboard"),
    })
}

/// The account panel's list (`ol_companies`): the driver's companies, each on or off.
pub fn companies() -> Value {
    let d = data();
    let me = user();
    let list = crate::cloud::companies();
    let on: Vec<String> = shown(&d, &me);
    let rows: Vec<Value> = list
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| {
            let id = c.get("id").and_then(Value::as_str).unwrap_or("");
            json!({ "id": id, "label": c.get("label").cloned().unwrap_or(Value::Null), "on": on.iter().any(|x| x == id) })
        })
        .collect();
    let count = rows.iter().filter(|r| r["on"] == true).count();
    json!({ "companies": rows, "count": count, "max": MAX, "url": format!("{}/leaderboard.html", hub_online::SITE), "blocked": blocked() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A company's key is the same for the same id, and fits the board's (8 to 64 of
    /// `[A-Za-z0-9_-]`) whatever the id.
    #[test]
    fn a_companys_key_is_its_own_and_fits_the_board() {
        assert_eq!(key_of("stadtbus-nord"), key_of("stadtbus-nord"));
        assert_ne!(key_of("stadtbus-nord"), key_of("stadtbus-nord-2"));
        for id in ["a", "ünterföhring", "stadtbus-nord"] {
            let k = key_of(id);
            assert_eq!(k.len(), 32);
            assert!(k.chars().all(|c| c.is_ascii_alphanumeric()), "{k}");
        }
    }

    /// The figures of a company as the board takes them: this month's result and passengers, the
    /// buses still the company's, the staff employed, the texts cut to the board's lengths.
    #[test]
    fn a_companys_figures_fit_the_board() {
        let f = co::Founding {
            name: "Verkehrsbetriebe Grundorf und Umgebung mit einem langen Namen".into(),
            short: "VBG".into(),
            map: "maps/Grundorf/global.cfg".into(),
            map_name: "Grundorf".into(),
            date: "2026-05-04".into(),
            country: "de".into(),
            ..Default::default()
        };
        let mut c = co::found(&f, "Luc");
        c.date = "2026-05-20".into();
        c.history.push(co::DayRecord { date: "2026-05-19".into(), passengers: 300, ..Default::default() });
        c.history.push(co::DayRecord { date: "2026-04-30".into(), passengers: 999, ..Default::default() });
        let e = entry_of(&c);
        assert_eq!(e.name.chars().count(), 40);
        assert_eq!(e.short.as_deref(), Some("VBG"));
        assert_eq!(e.colour.as_deref(), Some("#f28c28"));
        assert_eq!(e.country.as_deref(), Some("DE"));
        assert_eq!(e.map_name.as_deref(), Some("Grundorf"));
        assert_eq!(e.mode, "normal");
        assert!(!e.sandbox);
        assert_eq!(e.currency.as_deref(), Some("EUR"));
        assert_eq!(e.days, 16);
        assert_eq!(e.passengers_month, 300);
        assert_eq!(e.cash, c.cash);
        assert_eq!(e.profit_month, c.month("2026-05").result());
        assert_eq!((e.buses, e.lines, e.staff), (0, 0, 0));
        assert_eq!(e.reputation, Some(50.0));
        assert_eq!(e.key, key_of(&c.id));
    }

    /// Signed out, the switch says why and nothing is sent.
    #[test]
    fn signed_out_the_switch_says_to_sign_in() {
        online::test_data_dir();
        let p = settings_part("stadtbus-nord");
        assert_eq!(p["disabled"], true);
        assert_eq!(p["on"], false);
        assert_eq!(set("stadtbus-nord", true)["code"], "not_signed_in");
        assert_eq!(companies()["max"], MAX);
    }
}
