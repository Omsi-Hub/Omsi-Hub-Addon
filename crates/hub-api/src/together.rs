//! "Play together" in the bus company, the launcher's side (Beta 58, built out in Beta 61; the
//! company's side is `crate::launcher::company::together`): the boss shares his open company
//! (`host`: the company server, its code, who is online with his role and permissions and what
//! each drives, the duties asked for, the activity - the company's "Multiplayer" page), and
//! friends join it with the code (`join`: the boss's company in a menu of its own - their role,
//! their duties with Drive, the open duties to ask for, their figures, the players, the news -
//! and, as their role allows, the company's own pages relayed to the boss's launcher: `relay`).
//! Nobody opens a world: each drives his duties in his own game, at his own pace - unless players
//! whose duties run at the same time drive them together, in the company's world (`world`: the
//! first one's game opens it, the others' join it on its clock, each bound to his duty's times).
//!
//! The commands are `tg_…`; the texts come in the page's language (`lang`), as the company's.

#[cfg(feature = "native")]
mod host;
#[cfg(feature = "native")]
mod join;
// (the website: nobody hosts, nobody joins)
#[cfg(not(feature = "native"))]
#[path = "together/off.rs"]
mod host;
#[cfg(not(feature = "native"))]
use host::join;
pub mod relay;
/// RosterFlow in company multiplayer, over the loopback.
#[cfg(all(test, feature = "native"))]
mod rf_tests;

use crate::company::words::{self, tr, trf};
use crate::launcher::company::duty_launch::DutyRef;
use crate::launcher::company::together::wire::{Notice, SnapWorld};
use crate::launcher::company::together::{Perm, Role};
use crate::{Answer, Platform};
use serde_json::{json, Value};

pub use host::{boss_along, boss_out_of_world, boss_place, boss_world_code, closing_company, give_to_guest, guest_buses, online_guests, quit, rail_badge, tile_line};

/// The code of the company session hosted now (None: none, or "Play together" switched off).
pub fn session_code() -> Option<String> {
    host::session_code().filter(|_| enabled())
}

/// The colour a player of "Play together" has on the rosters.
pub const GUEST_COLOUR: &str = "#e58fd8";

/// The kill switch (Beta 61): "Play together" is on for the bus company unless this is set, or
/// the launcher runs with `OMSI_TOGETHER_OFF`. `OMSI_TOGETHER` forces it on (for testing).
pub const KILLED: bool = false;

/// "Play together" is open (Luc 2026-10-15: on for the company mode, with a kill switch).
pub fn enabled() -> bool {
    #[cfg(feature = "native")]
    return omsi_cfg::flags::OMSI_TOGETHER.is_set() || (!KILLED && !omsi_cfg::flags::OMSI_TOGETHER_OFF.is_set());
    // (the website: it needs the boss's computer)
    #[cfg(not(feature = "native"))]
    false
}

/// What the pages say while it is blocked.
pub const BLOCKED: &str = "Multiplayer - coming soon (beta)";

/// The gate's answer to a command: whether it is open, and what is said while it is not.
pub fn gate() -> Value {
    json!({ "on": enabled(), "label": tr(BLOCKED) })
}

/// A command of "Play together", or None when `cmd` is not one.
pub fn call(cmd: &str, args: &Value, p: &dyn Platform) -> Option<Answer> {
    words::set_language(args.get("lang").and_then(Value::as_str).unwrap_or(""));
    if cmd == "tg_gate" {
        return Some(Ok(gate()));
    }
    // (blocked: nothing runs - no server, no joining - and the badge is off)
    if !enabled() {
        return Some(Ok(json!({ "off": true, "on": false, "tip": tr(BLOCKED) })));
    }
    if cmd.starts_with("tg_host") {
        host::remember_language();
        return host::call(cmd, args);
    }
    join::call(cmd, args, p)
}

/// A notice as the page says it.
pub fn notice_text(n: &Notice) -> String {
    let args: Vec<(String, String)> = n.args.iter().map(|(k, v)| (k.clone(), if k == "name" || k == "text" || k == "tour" { v.clone() } else { tr(v) })).collect();
    let refs: Vec<(&str, &str)> = args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    trf(&n.text, &refs)
}

/// A notice's time of day (the computer's).
fn clock_of(at: u64) -> String {
    let local = at as i64 + crate::launcher::utc_offset();
    let m = local.rem_euclid(86400) / 60;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// A notice for the page: its time and its words.
pub fn notice_json(n: &Notice) -> Value {
    json!({ "n": n.n, "at": clock_of(n.at), "text": notice_text(n) })
}

/// A refusal of the boss's launcher in words: its text, its arguments filled in
/// (`host::refusal_with`).
pub fn refusal_text(v: &Value) -> String {
    let args: Vec<(String, String)> = v.get("args").and_then(|a| serde_json::from_value(a.clone()).ok()).unwrap_or_default();
    let refs: Vec<(&str, &str)> = args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    trf(v.get("error").and_then(Value::as_str).unwrap_or(""), &refs)
}

/// Who drives at the same time as a duty: "At the same time: Jan, Piet".
pub fn along_words(names: &[String]) -> String {
    trf("At the same time: %{names}", &[("names", &names.join(", "))])
}

/// The company's world in words: who opened it, its clock, who drives in it.
pub fn world_words(w: &SnapWorld) -> String {
    let clock = format!("{:02}:{:02}", w.minute / 60, w.minute % 60);
    if !w.ready {
        return trf("%{name} is opening the company's world…", &[("name", &w.host)]);
    }
    trf("The company's world is open (%{name}), its clock at %{time}: %{drivers} drive in it.", &[("name", &w.host), ("time", &clock), ("drivers", &w.drivers.join(", "))])
}

/// Keep watch over the game `pid` that opens the company's world (`--lan-host`): its game code,
/// once the game's status file says it (`lan::local_hosts`), to `told`; `ended` once the game has
/// ended (the world closed with it). A game that has not said its code in `world::OPENING_FOR` is
/// given up on (`ended` still comes when it ends).
#[cfg(feature = "native")]
pub fn watch_world(pid: u32, told: impl Fn(&str) + Send + 'static, ended: impl FnOnce() + Send + 'static) {
    use crate::launcher::company::together::world;
    let watch = move || {
        let since = std::time::Instant::now();
        let mut said = false;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !crate::launcher::list_instances().iter().any(|i| i.pid == pid && i.running) {
                ended();
                return;
            }
            if said || since.elapsed().as_secs() > world::OPENING_FOR {
                continue;
            }
            // (the status file of this game; one game at a time, so a single one is it)
            let hosts = crate::lan::local_hosts();
            let mine = hosts.iter().find(|h| h.get("pid").and_then(Value::as_u64) == Some(pid as u64)).or(if hosts.len() == 1 { hosts.first() } else { None });
            if let Some(code) = mine.and_then(|h| h.get("code")).and_then(Value::as_str).map(str::trim).filter(|c| !c.is_empty()) {
                crate::launcher::log_to_file(&format!("company world: game {pid} opened it ({code})"));
                told(code);
                said = true;
            }
        }
    };
    if let Err(e) = std::thread::Builder::new().name("company world".into()).spawn(watch) {
        crate::launcher::log_to_file(&format!("company world: not watched: {e}"));
    }
}

/// Who drives a duty of a player who left it: "AI (Jan left)" (Beta 63).
pub fn ai_label(name: &str) -> String {
    trf("AI (%{name} left)", &[("name", name)])
}

/// A player of "Play together" as the company's pages name him on his duties: his name, or "AI
/// (his name left)" while the AI drives a duty he left (`book::guest_left`).
pub fn guest_label(c: &crate::launcher::company::Company, g: u32) -> String {
    let name = crate::launcher::company::together::guest_name(c, g);
    if crate::launcher::company::together::book::guest_left(c, g) {
        ai_label(&name)
    } else {
        name
    }
}

/// A duty in short: "Line 5 · Tour 2 · Duty 1".
pub fn duty_words(number: &str, r: &DutyRef) -> String {
    format!("{} {}  ·  {} {}  ·  {} {}", tr("Line"), number, tr("Tour"), r.tour, tr("Duty"), r.duty + 1)
}

/// A role as the pages show it: its key, name, icon and what it is for.
pub fn role_json(r: Role) -> Value {
    json!({ "key": r.key(), "label": tr(r.label()), "icon": r.icon(), "about": tr(r.about()) })
}

/// A permission as the pages show it.
pub fn perm_json(p: Perm) -> Value {
    json!({ "key": p.key(), "label": tr(p.label()), "icon": p.icon() })
}

/// Every role and permission, for the pages' editors.
pub fn roles_json() -> Value {
    json!({
        "roles": Role::ALL.iter().map(|r| { let mut v = role_json(*r); v["perms"] = json!(r.preset().iter().map(|p| p.key()).collect::<Vec<_>>()); v }).collect::<Vec<_>>(),
        "perms": Perm::ALL.iter().map(|p| perm_json(*p)).collect::<Vec<_>>(),
    })
}

/// Permissions as the pages send them (their keys).
#[cfg(feature = "native")]
fn perms_arg(args: &Value) -> Vec<Perm> {
    args.get("perms").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().and_then(Perm::of)).collect()).unwrap_or_default()
}

/// A duty as a page sends it back.
#[cfg(feature = "native")]
fn duty_arg(args: &Value) -> Option<DutyRef> {
    serde_json::from_value(args.get("duty").cloned().unwrap_or(Value::Null)).ok()
}

#[cfg(feature = "native")]
fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use crate::launcher::company::together::{book, client, code, host, roles, world};

    /// Every text of "Play together" - the company's side, the launcher's and the page's
    /// (`together.js`, `multiplayer.js`) - in the seven languages.
    #[test]
    fn every_together_text_is_translated() {
        let mut texts: Vec<String> = book::texts()
            .into_iter()
            .chain(host::texts())
            .chain(roles::texts())
            .chain(world::texts())
            .chain([code::NOT_A_CODE, code::GAME_CODE, code::TYPO, code::NEWER, client::UNREACHED])
            .map(String::from)
            .collect();
        let js = include_str!("../../../ui/js/company/together.js");
        let mp = include_str!("../../../ui/js/company/multiplayer.js");
        // (this file without its tests)
        let own = include_str!("together.rs").split("#[cfg(test)]").next().unwrap_or("");
        let relay = include_str!("together/relay.rs").split("#[cfg(test)]").next().unwrap_or("");
        let sources =
            [(own, "tr(\"", '"'), (include_str!("together/host.rs"), "tr(\"", '"'), (include_str!("together/join.rs"), "tr(\"", '"'), (relay, "tr(\"", '"'), (js, "tr('", '\''), (mp, "tr('", '\'')];
        for (src, lead, end) in sources {
            for lead in [lead.to_string(), lead.replacen("tr(", "trf(", 1)] {
                let mut rest = src;
                while let Some(k) = rest.find(lead.as_str()) {
                    let after = &rest[k + lead.len()..];
                    rest = after;
                    let mut t = String::new();
                    let mut chars = after.chars();
                    while let Some(c) = chars.next() {
                        match c {
                            '\\' => t.extend(chars.next()),
                            c if c == end => break,
                            c => t.push(c),
                        }
                    }
                    if t.chars().filter(|c| c.is_alphabetic()).count() >= 2 {
                        texts.push(t);
                    }
                }
            }
        }
        assert!(texts.len() > 100, "only {} texts found", texts.len());
        let mut missing = Vec::new();
        for lang in ["nl", "de", "fr", "ru", "uk", "pl", "es"] {
            let table = crate::launcher::i18n::table(lang);
            for t in &texts {
                if !table.contains_key(t) {
                    missing.push(format!("{lang}: {t:?}"));
                }
            }
        }
        assert!(missing.is_empty(), "{} missing (crates/omsihub-locales/locales/company-together.yml):\n{}", missing.len(), missing.join("\n"));
    }

    struct NoPlatform;
    impl crate::Platform for NoPlatform {}

    /// On for the company mode, unless the kill switch is set; switched off, every command is
    /// refused - no server starts, nobody joins - and the gate says so.
    #[test]
    fn playing_together_is_on_unless_switched_off() {
        let p = NoPlatform;
        let gate = super::call("tg_gate", &serde_json::json!({}), &p).unwrap().unwrap();
        assert_eq!(gate["on"], super::enabled());
        if !super::KILLED && !omsi_cfg::flags::OMSI_TOGETHER_OFF.is_set() {
            assert!(super::enabled(), "on without a switch");
        }
        if super::enabled() {
            return;
        }
        let p = NoPlatform;
        let gate = super::call("tg_gate", &serde_json::json!({}), &p).unwrap().unwrap();
        assert_eq!(gate["on"], false);
        for cmd in ["tg_host_start", "tg_host_badge", "tg_join", "tg_view"] {
            let v = super::call(cmd, &serde_json::json!({ "code": "ABC" }), &p).unwrap().unwrap();
            assert_eq!(v["off"], true, "{cmd}");
        }
        assert!(super::host::hosting_none(), "no session was started");
    }
}
