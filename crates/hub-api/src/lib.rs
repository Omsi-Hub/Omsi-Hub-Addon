//! Omsi-Hub's commands: what the page asks for (`ui/js/api.js` `call(cmd, args)`), answered as
//! JSON, or as bytes for pictures (`call_bytes`, the app's `hub://localhost/bytes/<cmd>`).
//!
//! A command's prefix says whose it is, and each prefix is one module (one port):
//!
//! | prefix | module | what |
//! |---|---|---|
//! | `hub_` | [`hub`] | the app: boot, texts, first-run setup, migration, drives |
//! | `hub_update` | [`update`] | the app's own updates (this repository's releases) |
//! | `hub_companion` | [`companion`] | the phone and tablet companion's server |
//! | `hub_mods` | [`mods`] | mods installed into openOMSI with its own installer |
//! | `addon_` | [`addon`] | the plugin in the player's openOMSI, the bridge |
//! | `addon_controllers` | [`controllers`] | the player's wheel, pedals and button boxes |
//! | `co_` | [`company`] | the bus company |
//! | `rf_` | [`rosterflow`] | RosterFlow |
//! | `tg_` | [`together`] | playing together |
//! | `ol_` | [`online`] | the account, friends, the leaderboard |
//! | `cs_` | [`cloud`] | the cloud saves |
//! | `tt_` | [`timetable`] | the timetable editor |
//! | `de_` | [`depot`] | the depot editor |
//!
//! A command no module knows answers `unknown command`; one whose module is not ported yet
//! answers [`not_ported`]. Arguments are one JSON object; [`arg`] reads one of them.

// (Play together's page words are one `json!` too deep for the default)
#![recursion_limit = "512"]

pub mod addon;
#[cfg(feature = "native")]
pub mod cloud;
#[cfg(feature = "native")]
pub mod cloud_content;
#[cfg(feature = "native")]
pub mod companion;
pub mod company;
pub mod content;
pub mod controllers;
pub mod depot;
pub mod drive;
#[cfg(feature = "native")]
pub mod drivers;
pub mod hub;
#[cfg(feature = "native")]
pub mod lak;
#[cfg(feature = "native")]
pub mod lan;
pub mod launcher;
#[cfg(feature = "native")]
pub mod lineeditor;
pub mod live;
#[cfg(feature = "native")]
pub mod mods;
#[cfg(feature = "native")]
pub mod online;
#[cfg(feature = "native")]
pub mod privacy;
pub mod rosterflow;
pub mod timetable;
pub mod together;
#[cfg(feature = "native")]
pub mod update;

use serde_json::Value;

/// A command's answer.
pub type Answer = Result<Value, String>;

/// Work done beside the command (a list read, a day simulated): on its own thread on the desktop;
/// on the website, where the browser gives one thread, after the command, once its locks are let
/// go ([`run_later`]) - the page sees it on its next look, as it sees a thread's.
pub fn spawn<F: FnOnce() + Send + 'static>(f: F) {
    #[cfg(feature = "native")]
    std::thread::spawn(f);
    #[cfg(not(feature = "native"))]
    later().lock().unwrap_or_else(|e| e.into_inner()).push_back(Box::new(f));
}

#[cfg(not(feature = "native"))]
type Later = std::sync::Mutex<std::collections::VecDeque<Box<dyn FnOnce() + Send>>>;

#[cfg(not(feature = "native"))]
fn later() -> &'static Later {
    static LATER: std::sync::OnceLock<Later> = std::sync::OnceLock::new();
    LATER.get_or_init(Default::default)
}

/// The website: the work [`spawn`] was given, done now (and what it gave in turn), in order.
#[cfg(not(feature = "native"))]
pub fn run_later() {
    loop {
        let next = later().lock().unwrap_or_else(|e| e.into_inner()).pop_front();
        match next {
            Some(f) => f(),
            None => break,
        }
    }
}

/// A way to tell the page something that outlives the command (a thread watching the game).
pub type Emitter = std::sync::Arc<dyn Fn(&str, Value) + Send + Sync>;

/// What the commands need of the app around them (the Tauri shell, or a test).
pub trait Platform: Send + Sync {
    /// Tell the page something (`game-ended`, `progress` …), with a JSON payload.
    fn emit(&self, _event: &str, _payload: Value) {}
    /// End the app.
    fn quit(&self) {}
    /// [`Platform::emit`] for a thread that outlives the command (None: nothing to tell).
    fn emitter(&self) -> Option<Emitter> {
        None
    }
}

/// A platform that does nothing (tests, the command line).
pub struct Quiet;

impl Platform for Quiet {}

/// The answer of a command whose module is not ported yet.
pub fn not_ported(cmd: &str) -> String {
    format!("not ported yet: {cmd}")
}

/// One argument of `args` (`args.name`), read as `T`; a missing one reads as `null` (so an
/// `Option` is None).
pub fn arg<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null)).map_err(|e| format!("{name}: {e}"))
}

/// Any value as an answer.
pub fn to_json<T: serde::Serialize>(v: T) -> Answer {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// A command of the page.
pub fn call(cmd: &str, args: &Value, p: &dyn Platform) -> Answer {
    match cmd {
        #[cfg(feature = "native")]
        c if c.starts_with("hub_update") => update::call(c, args, p),
        #[cfg(feature = "native")]
        c if c.starts_with("hub_companion") => companion::call(c, args, p),
        #[cfg(feature = "native")]
        c if c.starts_with("hub_lan") => lan::call(c, args).unwrap_or_else(|| Err(format!("unknown command: {cmd}"))),
        #[cfg(feature = "native")]
        c if c.starts_with("hub_profile") => drivers::call(c, args).unwrap_or_else(|| Err(format!("unknown command: {cmd}"))),
        #[cfg(feature = "native")]
        c if c.starts_with("hub_mods") => mods::call(c, args),
        c if c.starts_with("hub_") => hub::call(c, args, p),
        c if c.starts_with("addon_controllers") => controllers::call(c, args),
        c if c.starts_with("addon_") => addon::call(c, args, p),
        c if c.starts_with("co_") => company::call(c, args, p),
        c if c.starts_with("rf_") => rosterflow::call(c, args, p),
        c if c.starts_with("tg_") => together::call(c, args, p).unwrap_or_else(|| Err(format!("unknown command: {cmd}"))),
        #[cfg(feature = "native")]
        c if c.starts_with("ol_") => online::call(c, args, p),
        #[cfg(feature = "native")]
        c if c.starts_with("cs_") => cloud::call(c, args, p),
        c if c.starts_with("tt_") => timetable::call(c, args, p),
        c if c.starts_with("de_") => depot::call(c, args, p),
        #[cfg(feature = "native")]
        c if c.starts_with("b3d_") => content::bus3d::call(c, args).unwrap_or_else(|| Err(format!("unknown command: {cmd}"))),
        #[cfg(feature = "native")]
        c if c.starts_with("lak_") => lak::call(c, args).unwrap_or_else(|| Err(format!("unknown command: {cmd}"))),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// A command that answers bytes (a picture, a file).
pub fn call_bytes(cmd: &str, args: &Value, p: &dyn Platform) -> Result<Vec<u8>, String> {
    match cmd {
        c if c.starts_with("hub_") => hub::call_bytes(c, args, p),
        c if c.starts_with("co_") => company::call_bytes(c, args, p),
        c if c.starts_with("rf_") => rosterflow::call_bytes(c, args, p),
        #[cfg(feature = "native")]
        c if c.starts_with("b3d_") => content::bus3d::call_bytes(c, args).unwrap_or_else(|| Err(format!("unknown command: {cmd}"))),
        // (the website: the photos the desktop sent with the cloud save, `cloud_content`)
        #[cfg(not(feature = "native"))]
        "b3d_photo" => {
            let s = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            std::fs::read(hub_core::paths::data_dir().join(hub_core::webpack::photo_name(&s("bus"), &s("paint")))).map_err(|_| "no photo".to_string())
        }
        #[cfg(feature = "native")]
        c if c.starts_with("lak_") => lak::call_bytes(c, args).unwrap_or_else(|| Err(format!("unknown command: {cmd}"))),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn commands_go_by_their_prefix() {
        assert_eq!(call("nope", &json!({}), &Quiet), Err("unknown command: nope".into()));
        // (Play together answers: its gate open, a command it does not know refused)
        assert!(call("tg_gate", &json!({}), &Quiet).is_ok_and(|v| v.get("on").is_some()));
        // (the ported modules say so of a command they do not know)
        for cmd in ["co_nope", "rf_nope", "tt_nope", "de_nope"] {
            assert_eq!(call(cmd, &json!({}), &Quiet), Err(format!("unknown command: {cmd}")), "{cmd}");
        }
        assert_eq!(call_bytes("co_portrait", &json!({ "n": 0 }), &Quiet), Err("no portrait 0".into()));
        assert!(call_bytes("co_portrait", &json!({ "n": 1 }), &Quiet).is_ok_and(|b| b.starts_with(&[0xFF, 0xD8])));
        assert_eq!(call_bytes("xx", &json!({}), &Quiet), Err("unknown command: xx".into()));
        let v = call("hub_version", &json!({}), &Quiet).unwrap();
        assert_eq!(v["app"], env!("CARGO_PKG_VERSION"));
        assert_eq!(v["min_openomsi"], "0.2.22");
    }

    #[test]
    fn arguments_are_read_by_name() {
        let a = json!({"n": 3, "s": "x"});
        assert_eq!(arg::<i64>(&a, "n"), Ok(3));
        assert_eq!(arg::<Option<String>>(&a, "missing"), Ok(None));
        assert!(arg::<i64>(&a, "s").unwrap_err().starts_with("s: "));
    }
}
