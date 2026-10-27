//! Omsi-Hub online the way the page drives it (`ol_…`, `cs_…`), end to end against the stand-in
//! on this computer (never the real project), with a data folder of its own and the session in
//! memory only. A debug build only (`online.rs`' `standin`):
//!
//!   node online/standin/standin.mjs
//!   OMSIHUB_STANDIN=http://127.0.0.1:54321 cargo run -p hub-api --example online_standin
//!
//! An account made on "the website", signed in through `ol_verify` without the privacy statement
//! agreed to (the account and the cloud wait), then agreed to (`ol_privacy_accept`); the display
//! name; the leaderboard switched on and a service record reported; three companies, two put in
//! the cloud and the third meeting the limit, then replacing one; a change going up; another
//! computer's newer version offered and taken; changes on both sides asked and both kept; a slot
//! downloaded; one removed; the account deleted with its cloud saves. Then Google through the
//! loopback (the stand-in's "browser" answers at once), its privacy statement accepted before,
//! and that account deleted without a password.

use hub_online::store::MemoryStore;
use hub_online::{Client, Endpoints};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

fn call(cmd: &str, args: Value) -> Value {
    hub_api::call(cmd, &args, &hub_api::Quiet).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

fn step(what: &str) {
    println!("  ok  {what}");
}

/// One plain HTTP request to the stand-in (the website's part, or the browser's): the answer's
/// head and its JSON body.
fn http_raw(base: &str, method: &str, path: &str, key: &str, body: Option<Value>) -> (String, Value) {
    let host = base.trim_start_matches("http://").split('/').next().unwrap_or("");
    let mut s = std::net::TcpStream::connect(host).expect("the stand-in runs");
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    // (HTTP/1.0: the answer comes whole, not chunked)
    write!(s, "{method} {path} HTTP/1.0\r\nHost: {host}\r\napikey: {key}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let (head, rest) = out.split_once("\r\n\r\n").unwrap_or((out.as_str(), ""));
    (head.to_string(), serde_json::from_str(rest).unwrap_or(Value::Null))
}

fn http(base: &str, method: &str, path: &str, key: &str, body: Option<Value>) -> Value {
    http_raw(base, method, path, key, body).1
}

/// The `Location` an address sends the browser on to.
fn location(head: &str) -> Option<String> {
    head.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case("location")).map(|(_, v)| v.trim().to_string()))
}

/// An address's origin and path (`http://127.0.0.1:9/a?b` → ("http://127.0.0.1:9", "/a?b")).
fn split_url(url: &str) -> (String, String) {
    let rest = url.trim_start_matches("http://");
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    (format!("http://{host}"), format!("/{path}"))
}

/// A company founded here (as the company port writes one: these fields and more).
fn company(data: &Path, name: &str) -> String {
    let id = hub_api::cloud::files::unused_id(data, name);
    // (the account's driver's: signed in, the companies listed are the driver's chosen)
    let driver = hub_core::settings::Settings::load().other.get("driver").and_then(Value::as_str).unwrap_or("").to_string();
    hub_api::cloud::files::save(data, &json!({ "id": id, "name": name, "short": "", "profile": driver, "cash": 10_000, "map": "maps/Grundorf/global.cfg" })).unwrap();
    id
}

fn change(data: &Path, id: &str, cash: i64) {
    let mut c = hub_api::cloud::files::load(data, id).unwrap();
    c["cash"] = json!(c["cash"].as_i64().unwrap() + cash);
    // (another time than the file's: the app reads a file again by its size and time)
    std::thread::sleep(Duration::from_millis(20));
    hub_api::cloud::files::save(data, &c).unwrap();
}

fn cash(data: &Path, id: &str) -> i64 {
    hub_api::cloud::files::load(data, id).unwrap()["cash"].as_i64().unwrap()
}

/// The chooser's badge of a company (`cs_state`) once it is `want`, or as it is after 20 seconds
/// (the worker uploads, and lists the cloud again on its next round, within 5 seconds).
fn badge(id: &str, want: &str) -> Value {
    let t = Instant::now();
    loop {
        let s = call("cs_state", json!({}));
        let b = s["companies"].get(id).cloned().unwrap_or(json!({ "state": "local" }));
        if b["state"] == want || t.elapsed() > Duration::from_secs(20) {
            return b;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn main() {
    let base = std::env::var("OMSIHUB_STANDIN").expect("OMSIHUB_STANDIN=http://127.0.0.1:54321 (a debug build)");
    let key = std::env::var("OMSIHUB_STANDIN_KEY").unwrap_or_else(|_| "sb_publishable_standin".into());
    let data = std::env::temp_dir().join(format!("omsihub-online-standin-{}", std::process::id()));
    std::fs::create_dir_all(&data).unwrap();
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, &data);
    assert_eq!(hub_core::data_dir(), data, "a data folder of its own");
    println!("Omsi-Hub online's commands against {base} (data {})", data.display());

    // --- an account made on the website, signed in through the page's panel ---------------------
    let run = std::process::id();
    let (email, password, name) = (format!("app{run}@standin.example"), "app password 1", format!("App {}", run % 10000));
    http(&base, "POST", "/auth/v1/signup", &key, Some(json!({ "email": email, "password": password, "data": { "display_name": name } })));
    let r = call("ol_sign_in", json!({ "email": email, "password": password, "lang": "en" }));
    assert_eq!(r["step"], "verify", "{r}");
    let code = http(&base, "GET", &format!("/__mail?email={email}"), &key, None)["code"].as_str().unwrap().to_string();
    let r = call("ol_verify", json!({ "email": email, "code": code, "password": password }));
    assert!(r.get("error").is_none(), "{r}");
    let st = call("ol_state", json!({}));
    assert!(st["sign_up_url"].as_str().is_some_and(|u| u.ends_with("/account.html#sign-up")), "{st}");
    step("signed in through ol_verify; the website's sign-up page is in the state");

    // --- the gate: an account without the privacy statement agreed to waits --------------------------
    assert_eq!((st["mode"].as_str(), st["privacy"]["needed"].as_bool(), st["privacy"]["amended"].as_bool()), (Some("in"), Some(true), Some(false)), "{st}");
    assert_eq!(call("cs_state", json!({}))["companies"], json!({}));
    assert_eq!(call("ol_profile", json!({ "name": "Not yet" }))["code"], "privacy");
    assert_eq!(call("ol_leaderboard", json!({ "on": true }))["code"], "privacy");
    assert_eq!(call("cs_state", json!({}))["signed_in"], false);
    let doc = call("ol_privacy", json!({}));
    assert!(doc["version"].as_u64().is_some_and(|v| v >= 1) && doc["blocks"].as_array().is_some_and(|b| b.len() > 10), "{doc}");
    let r = call("ol_privacy_accept", json!({}));
    assert!(r.get("error").is_none(), "{r}");
    let st = call("ol_state", json!({}));
    assert_eq!(st["privacy"]["needed"], false);
    assert_eq!(st["me"]["name"], name.as_str(), "the website's display name became the profile: {st}");
    step("the account and the cloud waited for the privacy statement; agreed to, they work");

    // --- the profile and the leaderboard ------------------------------------------------------------
    let renamed = format!("Driver {}", run % 10000);
    assert_eq!(call("ol_profile", json!({ "name": renamed }))["ok"], true);
    assert_eq!(call("ol_state", json!({}))["me"]["board"], false, "off until switched on");
    let r = call("ol_leaderboard", json!({ "on": true }));
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(call("ol_state", json!({}))["me"]["board"], true);
    let r = call("ol_report_stats", json!({ "points": 4321, "level": 3, "duties": 12, "minutes": 600, "km": 250 }));
    assert_eq!((r["ok"].as_bool(), r["code"].as_str()), (Some(true), Some("ok")), "{r}");
    assert_eq!(call("ol_report_stats", json!({ "points": 4400, "level": 3 }))["code"], "too_soon");
    let board = http(&base, "POST", "/rest/v1/rpc/hub_leaderboard", &key, Some(json!({ "p_limit": 100 })));
    let me = board.as_array().and_then(|b| b.iter().find(|p| p["display_name"] == renamed.as_str())).cloned().unwrap_or(Value::Null);
    assert_eq!((me["points"].as_u64(), me["km"].as_u64()), (Some(4321), Some(250)), "{board}");
    step("the display name changed; on the leaderboard, the record reported once in 30 seconds, read by the website");

    // --- two companies into the cloud, the third meets the limit --------------------------------
    let (a, b, c) = (company(&data, "Stadtbus Nord"), company(&data, "Regio Süd"), company(&data, "Dritter Betrieb"));
    let companies = call("cs_companies", json!({}));
    assert_eq!(companies.as_array().map(Vec::len), Some(3), "{companies}");
    for (id, slot) in [(&a, 1), (&b, 2)] {
        let r = call("cs_put", json!({ "id": id }));
        assert_eq!(r["slot"], slot, "{r}");
    }
    let r = call("cs_put", json!({ "id": c }));
    assert_eq!((r["limit"].as_bool(), r["slots"].as_array().map(Vec::len)), (Some(true), Some(2)), "{r}");
    assert_eq!(badge(&a, "synced")["state"], "synced");
    step("two companies in the cloud; the third is offered keep-local or replace");
    let r = call("cs_put", json!({ "id": c, "replace": 2 }));
    assert_eq!(r["slot"], 2, "{r}");
    assert_eq!(badge(&b, "local")["state"], "local");
    let slots = call("cs_state", json!({}))["slots"].clone();
    assert_eq!((slots[1]["name"].as_str(), slots[1]["linked"].as_str()), (Some("Dritter Betrieb"), Some(c.as_str())), "{slots}");
    step("the third replaced the second, which stays here as a local save");

    // --- a change goes up --------------------------------------------------------------------------
    change(&data, &a, 1000);
    call("cs_sync", json!({ "id": a }));
    assert_eq!(badge(&a, "synced")["state"], "synced");
    let mut other = Client::new(Endpoints::local(&base, &key).unwrap(), Box::new(MemoryStore::new()));
    other.sign_in(&email, password).unwrap();
    assert_eq!(other.check_privacy(), Ok(hub_online::privacy::VERSION), "agreed once, for the account");
    let t = Instant::now();
    while other.saves_list().unwrap().slot(1).unwrap().version < 2 && t.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(300));
    }
    let cloud = other.saves_list().unwrap();
    assert_eq!(cloud.slot(1).unwrap().version, 2, "{cloud:?}");
    step("a changed cloud company goes up when its pages are left");

    // --- another computer's newer version, offered and taken ------------------------------------------
    let (_, packed) = other.download_save(1, None).unwrap();
    other.upload_save(Some(1), "Stadtbus Nord", "Laptop", 2, &packed).unwrap();
    let r = call("cs_open", json!({ "id": a }));
    assert_eq!((r["ask"].as_str(), r["cloud"]["device"].as_str(), r["cloud"]["version"].as_u64()), (Some("newer"), Some("Laptop"), Some(3)), "{r}");
    let r = call("cs_resolve", json!({ "id": a, "keep": "cloud" }));
    assert!(r["ok"].as_bool() == Some(true), "{r}");
    assert_eq!(badge(&a, "synced")["state"], "synced");
    step("the laptop's newer version is offered on opening, and comes down");

    // --- changed on both sides: both kept --------------------------------------------------------------
    let before = cash(&data, &a);
    let (_, packed) = other.download_save(1, None).unwrap();
    other.upload_save(Some(1), "Stadtbus Nord", "Laptop", 3, &packed).unwrap();
    change(&data, &a, 5);
    let r = call("cs_open", json!({ "id": a }));
    assert_eq!(r["ask"], "conflict", "{r}");
    let r = call("cs_resolve", json!({ "id": a, "keep": "both" }));
    let copy = r["copy"].as_str().unwrap_or_else(|| panic!("{r}")).to_string();
    assert_eq!((cash(&data, &a), cash(&data, &copy)), (before, before + 5), "the cloud's here, this PC's as a copy");
    assert_eq!(badge(&copy, "local")["state"], "local");
    step("changes on both sides are asked; both kept, this PC's as a local copy");

    // --- a slot downloaded, one removed --------------------------------------------------------------
    std::fs::remove_file(hub_api::cloud::files::path_of(&data, &c)).unwrap();
    let r = call("cs_download", json!({ "slot": 2 }));
    let got = r["reread"].as_str().unwrap_or_else(|| panic!("{r}")).to_string();
    assert_eq!(hub_api::cloud::files::load(&data, &got).unwrap()["name"], "Dritter Betrieb");
    let r = call("cs_remove", json!({ "slot": 2 }));
    assert!(r["ok"].as_bool() == Some(true), "{r}");
    assert!(other.saves_list().unwrap().slots[1].empty);
    assert!(hub_api::cloud::files::path_of(&data, &got).exists(), "the company here stays, local");
    step("a cloud slot downloaded to this PC; one removed from the cloud, kept here");

    // --- the account deleted with its cloud saves ---------------------------------------------------
    let r = call("ol_delete", json!({ "password": "not it" }));
    assert!(r.get("error").is_some(), "{r}");
    assert!(!other.saves_list().unwrap().slots[0].empty, "nothing deleted on a wrong password");
    let r = call("ol_delete", json!({ "password": password }));
    assert!(r.get("error").is_none(), "{r}");
    assert_eq!(call("cs_state", json!({}))["companies"], json!({}), "every company here is local");
    assert_eq!(call("ol_state", json!({}))["mode"], "out");
    assert!(other.sign_in(&email, password).is_err());
    step("the account deleted with its cloud saves; every company here is local");

    // --- Google, through the loopback ------------------------------------------------------------------
    assert_eq!(call("ol_oauth", json!({}))["code"], "privacy", "only after the statement's question");
    let r = call("ol_oauth", json!({ "agree": true }));
    let (id, url) = (r["id"].as_u64().unwrap(), r["url"].as_str().unwrap().to_string());
    let waiting = std::thread::spawn(move || call("ol_oauth_wait", json!({ "id": id, "agree": true })));
    // (the browser: the stand-in "signs in" at once and sends it on to the app's listener)
    let (origin, path) = split_url(&url);
    let (head, _) = http_raw(&origin, "GET", &path, &key, None);
    let back = location(&head).unwrap_or_else(|| panic!("no redirect: {head}"));
    assert!(back.starts_with("http://127.0.0.1:") && back.contains("/callback"), "{back}");
    let (origin, path) = split_url(&back);
    http_raw(&origin, "GET", &path, &key, None);
    let r = waiting.join().unwrap();
    assert!(r.get("error").is_none(), "{r}");
    let st = call("ol_state", json!({}));
    assert_eq!((st["mode"].as_str(), st["privacy"]["needed"].as_bool(), st["no_password"].as_bool()), (Some("in"), Some(false), Some(true)), "accepted before, so no question after: {st}");
    if st["need_profile"] == true {
        assert_eq!(call("ol_profile", json!({ "name": format!("Google {}", run % 10000) }))["ok"], true);
    }
    let r = call("ol_delete", json!({ "password": "" }));
    assert!(r.get("error").is_none(), "{r}");
    assert_eq!(call("ol_state", json!({}))["mode"], "out");
    step("signed in with Google through the browser, the statement accepted before; deleted without a password");

    let _ = std::fs::remove_dir_all(&data);
    println!("Omsi-Hub online's commands work against the stand-in.");
}
