//! Omsi-Hub online, the app's side (docs/ONLINE.md; the client is the crate `hub-online`): the
//! optional account, the friends, their online state, the invites to a company session and the
//! leaderboard. Without an account - or without a network - the app is as it was: nothing here
//! blocks it, and the page is told "offline" quietly. Ported from the old `omsihub/online.rs`.
//!
//! * `ol_state` - what the page shows, at once from what the worker last fetched (no network);
//!   the first call starts the worker: the heartbeat about once a minute and whenever the state
//!   changes (online, away, hosting a session), the friends and the invites with it;
//! * `ol_sign_in {email, password}`, `ol_verify {email, code, password}`, `ol_resend {email}`,
//!   `ol_sign_out`; an account is made on the website (`sign_up_url` in the state, which the page
//!   opens), and its display name becomes the profile at the first sign-in here;
//! * `ol_forgot {email}`, `ol_reset {email, code, password}` - the password reset by a code;
//! * `ol_local {on}` - "Play without an account" (only that choice is kept, in the data
//!   folder's `omsihub-online.json`);
//! * `ol_profile {name}` - the display name chosen or changed;
//! * the account's personnel pass (`my_personnel`, the state's `me.personnel`) is fetched once a
//!   session and written into the account's driver's pass (`addon::set_personnel`): the
//!   number and the code they sign on with are the account's, the same on every computer;
//! * `ol_leaderboard {on}` - standing on the public leaderboard, or not (off until chosen);
//! * `ol_report_stats {points, level, duties, minutes, km}` - the service record for the
//!   leaderboard; the worker tells it itself, the account's driver's whenever it changed
//!   (`report_due`, from `drivers::board_stats`);
//! * `ol_delete {password}` - the account and everything of it deleted, its cloud saves first
//!   (`cloud.rs` then makes every company here local);
//! * `ol_add {code}`, `ol_respond {id, accept}`, `ol_remove {id}`, `ol_block {id}`;
//! * `ol_invite {id}` - the company session hosted now, to a friend; `ol_dismiss {id}`;
//! * `ol_presence {away}`, `ol_seen`, `ol_refresh`;
//! * `ol_privacy` - the privacy statement (`privacy.rs`); `ol_privacy_accept` - agreed to;
//! * `ol_oauth {agree, provider}` - signing in with Google or Discord (`provider`: `google`, the
//!   default, or `discord`): the listener started (`hub_online::oauth`), the address the page
//!   opens in the system's browser answered with a number; `ol_oauth_wait {id, agree}` - waits
//!   for the browser (a few minutes at most), then signs in as `ol_sign_in` does, and a Discord
//!   sign-in links the account to that Discord account as the website does (`discord_claim`);
//!   `ol_oauth_cancel` - the wait given up.
//!
//! The privacy statement (docs/PRIVACY.md): the page asks it before "Create an account" and
//! before Google or Discord (`agree`: accepted there), which is sent as `accept_privacy` once signed in; the
//! email sign-in sends no `agree`, and its account is asked afterwards. At every sign-in and every
//! restored session `my_privacy` is asked; below the current version the state says
//! `privacy.needed`, the page asks (agree, or sign out), and the account's features - the
//! heartbeat, the friends, the invites, the profile, the leaderboard, the cloud saves - do
//! nothing meanwhile (the client itself refuses them). Playing without an account asks nothing.
//!
//! Every answer is JSON with the texts in the page's language (`lang`): `{ok, message}` or
//! `{error, code}`, the error one of `hub_online::codes`', never the server's own words. The email
//! address and the password pass through and are kept nowhere; the refresh token is in the OS's
//! credential store only (`hub_online::store`). Nothing here is logged.
//!
//! A debug build may talk to the stand-in on this computer instead of the real project
//! (`OMSIHUB_STANDIN=http://127.0.0.1:54321`, `online/standin/standin.mjs`); a release build never
//! reads it.

use crate::{Answer, Platform};
use hub_online::oauth::{self, Provider, Texts};
use hub_online::{codes, privacy, store, Client, Endpoints, Error, Flow, Friend, Invite, Personnel, Profile, Stats};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Duration;

// --- the texts, in the page's language ------------------------------------------------------------

/// The page's language, as its last command said (`lang`); empty: English.
static LANGUAGE: Mutex<String> = Mutex::new(String::new());
/// A language's texts: the English text → its translation.
pub(crate) type Table = Arc<BTreeMap<String, String>>;
/// The tables read so far (`hub_locales::texts` reads every file: once per language).
static TABLES: LazyLock<Mutex<HashMap<String, Table>>> = LazyLock::new(Default::default);

/// The page's language for the texts that follow (the commands' `lang`; an empty one keeps it).
pub(crate) fn set_language(lang: &str) {
    let lang = lang.trim().to_ascii_lowercase();
    if !lang.is_empty() {
        *LANGUAGE.lock().unwrap_or_else(|e| e.into_inner()) = lang;
    }
}

pub(crate) fn language() -> String {
    LANGUAGE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The texts of `lang`.
pub(crate) fn table(lang: &str) -> Table {
    let mut t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    t.entry(lang.to_string()).or_insert_with(|| Arc::new(if lang.is_empty() || lang == "en" { BTreeMap::new() } else { hub_locales::texts(lang) })).clone()
}

/// `text` in the page's language (English without a translation).
pub(crate) fn tr(text: &str) -> String {
    table(&language()).get(text).cloned().unwrap_or_else(|| text.to_string())
}

/// `text` translated, its `%{name}`s filled.
pub(crate) fn trf(text: &str, vars: &[(&str, &str)]) -> String {
    let mut s = tr(text);
    for (k, v) in vars {
        s = s.replace(&format!("%{{{k}}}"), v);
    }
    s
}

// --- playing together ------------------------------------------------------------------------------

/// The company session this app hosts now: its code, and whether the boss's friends may join it
/// without an invite (the company's `together.friends_join`). Playing together is not ported yet
/// (the `tg_` commands need a relay): until `tg_host_start` calls [`host`], none, so no invite can
/// be sent and no friend's session joined.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Hosted {
    pub code: String,
    pub friends_join: bool,
}

static HOSTED: Mutex<Option<Hosted>> = Mutex::new(None);

fn hosted() -> Option<Hosted> {
    HOSTED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The session hosted from now on (none: sharing stopped); the worker tells the friends at once.
#[allow(dead_code)]
pub(crate) fn host(h: Option<Hosted>) {
    *HOSTED.lock().unwrap_or_else(|e| e.into_inner()) = h;
}

/// The company's "friends can join" changed: the session hosted now (if any) says so from now on.
pub(crate) fn friends_join_changed(on: bool) {
    if let Some(h) = HOSTED.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        h.friends_join = on;
    }
}

fn session_code() -> Option<String> {
    hosted().map(|h| h.code)
}

/// Whether playing together is there to join a friend's session.
fn together_enabled() -> bool {
    false
}

// --- the client --------------------------------------------------------------------------------------

/// What the page is shown, as the worker and the commands last saw it.
#[derive(Default)]
struct View {
    worker: bool,
    signed_in: bool,
    /// The session outlives the app (kept in the OS's store).
    lasting: bool,
    offline: bool,
    /// Signed in without a profile yet: the display name to choose.
    need_profile: bool,
    /// The account has no password (made with Google): deleting it asks for none.
    no_password: bool,
    /// Signed in, and the current privacy statement not agreed to: the account waits.
    need_privacy: bool,
    /// The version accepted before (0: none), while `need_privacy`: an amended statement, or none
    /// accepted yet.
    privacy_before: u32,
    profile: Option<Profile>,
    /// The account's personnel pass, fetched once a session (`my_personnel`): the account's
    /// driver signs on with it (`addon::set_personnel`).
    personnel: Option<Personnel>,
    friends: Vec<Friend>,
    invites: Vec<Invite>,
    /// The requests and invites the page has shown (the badge counts the others).
    seen: HashSet<String>,
    /// The window in the background a while (the page says so).
    away: bool,
    /// The state and the session last told the server, and when.
    told: Option<(&'static str, Option<String>)>,
    beat_at: u64,
    refreshed_at: u64,
    /// The service record the leaderboard was last told (`report_due`).
    reported: Option<Stats>,
}

static VIEW: LazyLock<Mutex<View>> = LazyLock::new(Default::default);
/// The client (made at the first need; its calls run one at a time, under this lock, never
/// under `VIEW`'s).
static CLIENT: Mutex<Option<Client>> = Mutex::new(None);

fn view() -> MutexGuard<'static, View> {
    VIEW.lock().unwrap_or_else(|e| e.into_inner())
}

fn client() -> MutexGuard<'static, Option<Client>> {
    CLIENT.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn endpoints() -> Endpoints {
    // (the tests reach nothing: an HTTPS address nobody answers on)
    #[cfg(test)]
    return Endpoints::new("https://127.0.0.1:9", "sb_publishable_test").unwrap();
    #[cfg(not(test))]
    return standin().unwrap_or_else(Endpoints::supabase);
}

/// A debug build's stand-in on this computer (`OMSIHUB_STANDIN=http://127.0.0.1:54321`, the key
/// `OMSIHUB_STANDIN_KEY`; online/standin/standin.mjs): the flows tried without the real project.
/// A release build never reads it.
#[cfg(not(test))]
fn standin() -> Option<Endpoints> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let base = std::env::var("OMSIHUB_STANDIN").ok().filter(|b| !b.is_empty())?;
    Endpoints::local(&base, &std::env::var("OMSIHUB_STANDIN_KEY").unwrap_or_else(|_| "sb_publishable_standin".into())).ok()
}

fn new_store() -> Box<dyn store::SecretStore> {
    #[cfg(test)]
    return Box::new(store::MemoryStore::new());
    // (a stand-in's session is not the player's: it stays out of the OS's store)
    #[cfg(not(test))]
    return if standin().is_some() { Box::new(store::MemoryStore::new()) } else { store::os_store() };
}

/// `f` with the client, made (and the stored session read) on the first call.
pub(crate) fn with<T>(f: impl FnOnce(&mut Client) -> T) -> T {
    let mut g = client();
    let c = g.get_or_insert_with(|| Client::new(endpoints(), new_store()));
    f(c)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

// --- the choice of playing without an account --------------------------------------------------

const FILE: &str = "omsihub-online.json";

fn local() -> bool {
    std::fs::read_to_string(hub_core::data_dir().join(FILE)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.get("local").and_then(Value::as_bool)).unwrap_or(false)
}

fn set_local(on: bool) {
    let _ = std::fs::write(hub_core::data_dir().join(FILE), json!({ "local": on }).to_string());
}

// --- the answers ------------------------------------------------------------------------------

/// An error as the page shows it; signed out or offline, the view says so too.
fn fail(e: Error) -> Value {
    {
        let mut v = view();
        match e {
            Error::Offline => v.offline = true,
            Error::SignedOut => signed_out(&mut v),
            _ => v.offline = false,
        }
    }
    json!({ "error": tr(e.text()), "code": e.code() })
}

fn done(message: &str) -> Value {
    view().offline = false;
    json!({ "ok": true, "message": message })
}

fn answer(r: Result<Value, Error>) -> Value {
    r.unwrap_or_else(fail)
}

fn signed_out(v: &mut View) {
    v.signed_in = false;
    v.need_profile = false;
    v.no_password = false;
    v.need_privacy = false;
    v.privacy_before = 0;
    v.profile = None;
    v.personnel = None;
    v.friends.clear();
    v.invites.clear();
    v.told = None;
    v.reported = None;
}

fn str_arg<'a>(args: &'a Value, name: &str) -> &'a str {
    args.get(name).and_then(Value::as_str).unwrap_or("")
}

// --- what is fetched -------------------------------------------------------------------------

/// The state to tell: in a session while hosting one, else online or away. The session's code
/// goes with it only when the friends may join without an invite (the server shows it to friends
/// only); else they join by an invite, which carries the code itself.
fn presence(away: bool) -> (&'static str, Option<String>) {
    presence_of(hosted(), away)
}

fn presence_of(hosted: Option<Hosted>, away: bool) -> (&'static str, Option<String>) {
    match hosted {
        Some(h) => ("busy", h.friends_join.then_some(h.code)),
        None if away => ("away", None),
        None => ("online", None),
    }
}

/// What a refresh fetched: (lasting, no password), then the account, or the version accepted
/// before while the privacy statement waits.
type Fetched = ((bool, bool), Result<(Option<Profile>, Option<Personnel>, Vec<Friend>, Vec<Invite>), u32>);

/// The heartbeat, then the profile, the personnel pass (once), the friends and the invites, into
/// the view; first, while the privacy statement is not agreed to, `my_privacy` (and nothing else).
fn refresh(beat: bool) -> Result<(), Error> {
    let (away, has_pass) = {
        let v = view();
        (v.away, v.personnel.is_some())
    };
    let now_state = presence(away);
    // (the account's user id beside what was fetched: the driver linked is this account's)
    let mut user = String::new();
    let got = with(|c| -> Result<Option<Fetched>, Error> {
        user = c.user_id().unwrap_or("").to_string();
        if !c.signed_in() {
            return Ok(None);
        }
        if c.privacy_needed() && c.check_privacy()? < privacy::VERSION {
            // (held: the version accepted before, if any)
            return Ok(Some(((c.lasting(), !c.has_password()), Err(c.privacy_agreed().unwrap_or(0)))));
        }
        if beat {
            // (no profile yet: the heartbeat has nothing to tell)
            if let Err(e @ (Error::Offline | Error::SignedOut)) = c.heartbeat(now_state.0, now_state.1.as_deref()) {
                return Err(e);
            }
        }
        let profile = c.profile()?;
        // (a server without `my_personnel` yet, or a pass not fetched now: the driver keeps the
        // one of this computer, and it is asked again at the next beat)
        let personnel = if profile.is_some() && !has_pass { c.personnel().ok() } else { None };
        let (friends, invites) = if profile.is_some() { (c.friends()?, c.invites()?) } else { (Vec::new(), Vec::new()) };
        Ok(Some(((c.lasting(), !c.has_password()), Ok((profile, personnel, friends, invites)))))
    });
    let mut v = view();
    match got {
        Ok(Some(((lasting, no_password), Err(agreed)))) => {
            // (the account waits for the player's agreement; asked again at the next beat)
            v.signed_in = true;
            v.lasting = lasting;
            v.no_password = no_password;
            v.offline = false;
            v.need_privacy = true;
            v.privacy_before = agreed;
            v.need_profile = false;
            v.profile = None;
            v.friends.clear();
            v.invites.clear();
            v.refreshed_at = unix_now();
            if beat {
                v.told = Some(now_state);
                v.beat_at = unix_now();
            }
            Ok(())
        }
        Ok(Some(((lasting, no_password), Ok((profile, personnel, friends, invites))))) => {
            let link = profile.as_ref().map(|p| p.display_name.clone());
            if personnel.is_some() {
                v.personnel = personnel;
            }
            let pass = v.personnel.clone();
            v.signed_in = true;
            v.lasting = lasting;
            v.no_password = no_password;
            v.offline = false;
            v.need_privacy = false;
            v.need_profile = profile.is_none();
            v.profile = profile;
            v.friends = friends;
            v.invites = invites;
            v.refreshed_at = unix_now();
            if beat {
                v.told = Some(now_state);
                v.beat_at = unix_now();
            }
            // (the account's driver: the display name's, the first time - `drivers::link_account`;
            // its file work done after the view is let go, which `ol_state` waits for)
            drop(v);
            if let Some(name) = link {
                crate::drivers::link_account(&name, &user);
                // (the account's pass is its driver's: the plugin's sign-on, the Companion and
                // the welcome's pass read it there)
                if let (Some(p), Some(driver)) = (pass, crate::drivers::account_driver()) {
                    crate::addon::set_personnel(&driver, &hub_game::bridge::Personnel { number: p.number, code: p.code });
                }
            }
            Ok(())
        }
        Ok(None) => {
            signed_out(&mut v);
            Ok(())
        }
        Err(e) => {
            match e {
                Error::Offline => v.offline = true,
                Error::SignedOut => signed_out(&mut v),
                _ => {}
            }
            // (tried again at the next beat, not at the next look)
            if beat {
                v.beat_at = unix_now();
                v.told = Some(now_state);
            }
            Err(e)
        }
    }
}

/// The worker: a look every few seconds, the heartbeat once a minute or when the state to tell
/// changed. It runs while the app does; it never holds the view during a request.
fn worker() {
    let mut first = true;
    loop {
        let due = {
            let v = view();
            let state = presence(v.away);
            first || (v.signed_in && (unix_now().saturating_sub(v.beat_at) >= 60 || v.told.as_ref() != Some(&state)))
        };
        // (another driver than the account's drives: they play locally, nobody is told)
        let due = due && (first || crate::drivers::plays_online());
        if due {
            let signed = with(|c| c.signed_in());
            if signed {
                // (a session the store kept is signed in, offline too)
                view().signed_in = true;
                if refresh(true).is_ok() {
                    report_due();
                }
            }
            first = false;
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// The account's driver's service record to the leaderboard (`drivers::board_stats`: the
/// points, the level, the duties, the minutes and the kilometres), whenever it changed since it
/// was last told - at the first beat, after a drive. Signed out, waiting for the privacy
/// statement or without a display name nothing is told, nor for another driver than the
/// account's; the server keeps one report in 30 seconds ("too_soon": told at the next beat).
fn report_due() {
    let ready = {
        let v = view();
        v.signed_in && !v.need_privacy && v.profile.is_some()
    };
    if !ready || !crate::drivers::plays_online() {
        return;
    }
    let Some(stats) = crate::drivers::account_driver().and_then(|d| crate::drivers::board_stats(&d)) else { return };
    if view().reported == Some(stats) {
        return;
    }
    if let Ok("ok") = report_stats(&stats) {
        view().reported = Some(stats);
    }
}

fn start_worker() {
    let mut v = view();
    if v.worker {
        return;
    }
    v.worker = true;
    let _ = std::thread::Builder::new().name("omsihub online".into()).spawn(worker);
}

/// "Offline" told as the app closes (best effort, a few seconds at most; skipped while another
/// request runs).
pub fn goodbye() {
    if let Ok(mut g) = CLIENT.try_lock() {
        if let Some(c) = g.as_mut() {
            c.goodbye();
        }
    }
}

// --- the page's view ----------------------------------------------------------------------------

/// Seconds since the Unix epoch of an RFC 3339 time (`2026-10-16T10:00:00.123+02:00`).
pub(crate) fn unix_of(ts: &str) -> Option<i64> {
    let ts = ts.trim();
    let n = |a: usize, b: usize| ts.get(a..b).and_then(|s| s.parse::<i64>().ok());
    let (y, mo, d, h, mi, s) = (n(0, 4)?, n(5, 7)?, n(8, 10)?, n(11, 13)?, n(14, 16)?, n(17, 19)?);
    let rest = &ts[19..];
    let rest = rest.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match rest.chars().next() {
        Some(sign @ ('+' | '-')) => {
            let z: String = rest[1..].chars().filter(char::is_ascii_digit).collect();
            let (oh, om) = (z.get(0..2)?.parse::<i64>().ok()?, z.get(2..4).and_then(|m| m.parse::<i64>().ok()).unwrap_or(0));
            (oh * 3600 + om * 60) * if sign == '-' { -1 } else { 1 }
        }
        _ => 0,
    };
    // (days from the civil date, Howard Hinnant's)
    let (y, m) = if mo <= 2 { (y - 1, mo + 9) } else { (y, mo - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146097 + doe - 719468) * 86400 + h * 3600 + mi * 60 + s - offset)
}

/// "Last seen 5 min ago".
fn seen_words(last: Option<&str>) -> String {
    let Some(t) = last.and_then(unix_of) else { return String::new() };
    let ago = (unix_now() as i64 - t).max(0);
    match ago {
        0..=119 => tr("Last seen just now"),
        120..=3599 => trf("Last seen %{n} min ago", &[("n", &(ago / 60).to_string())]),
        3600..=86399 => trf("Last seen %{n} h ago", &[("n", &(ago / 3600).to_string())]),
        86400..=2591999 => trf("Last seen %{n} days ago", &[("n", &(ago / 86400).to_string())]),
        _ => tr("Last seen a long time ago"),
    }
}

/// A friend's state: `online`, `away`, `busy` (in a session) or `offline`.
fn state_of(f: &Friend) -> &'static str {
    match (f.online, f.status.as_str()) {
        (false, _) | (_, "offline") => "offline",
        (true, "busy") => "busy",
        (true, "away") => "away",
        _ => "online",
    }
}

fn state_label(s: &str) -> String {
    tr(match s {
        "online" => "Online",
        "away" => "Away",
        "busy" => "In a session",
        _ => "Offline",
    })
}

/// A photo's address, from its name in the bucket.
fn avatar_of(name: Option<&String>) -> Option<String> {
    name.map(|n| endpoints().avatar_url(n))
}

fn friend_json(f: &Friend) -> Value {
    let st = state_of(f);
    let session = f.session_code.clone().filter(|_| st != "offline");
    json!({
        "id": f.user_id, "name": f.display_name, "state": st, "state_label": state_label(st),
        "seen": if st == "offline" { seen_words(f.last_seen.as_deref()) } else { String::new() },
        "session": session, "join": session.is_some() && together_enabled(),
        "avatar": avatar_of(f.avatar.as_ref()),
    })
}

fn state() -> Value {
    start_worker();
    let hosting = session_code().is_some();
    let v = view();
    // (another driver than the account's plays locally: `account_of` says whose the account is)
    let mine = crate::drivers::plays_online();
    let mode = if v.signed_in && mine {
        "in"
    } else if local() || v.signed_in {
        "local"
    } else {
        "out"
    };
    let friends: Vec<&Friend> = v.friends.iter().filter(|f| f.relation == "accepted").collect();
    let requests: Vec<&Friend> = v.friends.iter().filter(|f| f.relation == "pending").collect();
    let name_of = |id: &str| v.friends.iter().find(|f| f.user_id == id).map(|f| f.display_name.clone()).unwrap_or_else(|| tr("A friend"));
    let new_requests = requests.iter().filter(|f| f.incoming && !v.seen.contains(&format!("r:{}", f.user_id))).count();
    // (an invite holds for 15 minutes)
    let invites: Vec<&Invite> = v.invites.iter().filter(|i| together_enabled() && i.created_at.as_deref().and_then(unix_of).is_none_or(|t| unix_now() as i64 - t < 15 * 60)).collect();
    let new_invites = invites.iter().filter(|i| !v.seen.contains(&format!("i:{}", i.key()))).count();
    let mut order: Vec<&&Friend> = friends.iter().collect();
    let rank = |f: &Friend| match state_of(f) {
        "busy" => 0,
        "online" => 1,
        "away" => 2,
        _ => 3,
    };
    order.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase())));
    json!({
        "mode": mode,
        "account_of": if v.signed_in && !mine { crate::drivers::account_driver() } else { None },
        "offline": v.offline,
        "lasting": v.lasting,
        "need_profile": v.need_profile,
        "no_password": v.signed_in && v.no_password,
        "me": v.profile.as_ref().map(|p| json!({ "name": p.display_name, "code": p.friend_code, "board": p.on_board, "avatar": avatar_of(p.avatar.as_ref()),
            "personnel": v.personnel.as_ref().map(|p| json!({ "number": p.number, "code": p.code })) })),
        "friends": order.iter().map(|f| friend_json(f)).collect::<Vec<_>>(),
        "online_count": friends.iter().filter(|f| state_of(f) != "offline").count(),
        "requests": requests.iter().map(|f| json!({ "id": f.user_id, "name": f.display_name, "incoming": f.incoming, "new": f.incoming && !v.seen.contains(&format!("r:{}", f.user_id)), "avatar": avatar_of(f.avatar.as_ref()) })).collect::<Vec<_>>(),
        "invites": invites.iter().map(|i| {
            let from = name_of(&i.from);
            json!({ "id": i.key(), "from": from, "session": i.session_code, "text": trf("%{name} invites you to their company", &[("name", &from)]), "new": !v.seen.contains(&format!("i:{}", i.key())) })
        }).collect::<Vec<_>>(),
        "badge": new_requests + new_invites,
        "hosting": hosting,
        "together": together_enabled(),
        "sign_up_url": hub_online::sign_up_page(),
        "board_url": format!("{}/leaderboard.html", hub_online::SITE),
        "privacy": { "version": privacy::VERSION, "needed": v.signed_in && v.need_privacy, "amended": v.privacy_before > 0 },
    })
}

// --- the account ------------------------------------------------------------------------------

/// The privacy statement accepted on the page just before (Google's sign-in asks it first).
fn agree(args: &Value) -> bool {
    args.get("agree").and_then(Value::as_bool).unwrap_or(false)
}

/// Signed in (`agree`: the statement accepted just before): `my_privacy` asked, and
/// `accept_privacy` sent when it is below the current version and the player agreed. Then the
/// account, or, not agreed, the page asks for the agreement first and the account waits.
fn after_sign_in(agree: bool) -> Value {
    set_local(false);
    {
        let mut v = view();
        v.signed_in = true;
        v.seen.clear();
        // (another account's pass is not this one's)
        v.personnel = None;
    }
    // (Ok(None): accepted; Ok(Some(n)): held, version n accepted before)
    let agreed = with(|c| -> Result<Option<u32>, Error> {
        let before = c.check_privacy()?;
        if before >= privacy::VERSION || (agree && c.accept_privacy()? >= privacy::VERSION) {
            return Ok(None);
        }
        Ok(Some(before))
    });
    match agreed {
        Ok(None) => start_account(),
        Ok(Some(before)) => {
            {
                let mut v = view();
                v.need_privacy = true;
                v.privacy_before = before;
            }
            done(&tr("Signed in. Please accept the Privacy Statement to use your account."))
        }
        Err(e) => fail(e),
    }
}

/// The privacy statement agreed to (the page's question): then the account.
fn privacy_accept() -> Value {
    match with(|c| c.accept_privacy()) {
        Ok(n) if n >= privacy::VERSION => {
            view().need_privacy = false;
            let mut a = start_account();
            if a.get("ok").is_some() {
                a["message"] = json!(tr("The Privacy Statement has been accepted."));
            }
            a
        }
        Ok(_) => fail(Error::Code("server")),
        Err(e) => fail(e),
    }
}

/// The account in use: the profile (made, when there is none, with the display name chosen on
/// the website's sign-up), the friends, the heartbeat.
fn start_account() -> Value {
    let made = with(|c| -> Result<(), Error> {
        if let Some(n) = c.signed_up_name().map(str::to_string) {
            if c.profile()?.is_none() {
                // (a name the profiles table will not take, or taken since, is chosen again here)
                if let Err(e @ (Error::Offline | Error::SignedOut)) = c.create_profile(&n) {
                    return Err(e);
                }
            }
        }
        Ok(())
    });
    if let Err(e) = made.and_then(|_| refresh(true)) {
        return fail(e);
    }
    let v = view();
    let message = match &v.profile {
        Some(p) => trf("Signed in as %{name}.", &[("name", &p.display_name)]),
        None => tr("Signed in. Choose your display name."),
    };
    drop(v);
    done(&message)
}

fn verify(args: &Value) -> Value {
    let (email, password) = (str_arg(args, "email"), str_arg(args, "password"));
    let signed = with(|c| c.verify_email(email, str_arg(args, "code")).and_then(|in_| if in_ || password.is_empty() { Ok(in_) } else { c.sign_in(email, password).map(|_| true) }));
    match signed {
        Ok(true) => after_sign_in(agree(args)),
        Ok(false) => {
            let mut a = done(&tr("Email address confirmed. Sign in now."));
            a["step"] = json!("sign_in");
            a
        }
        Err(e) => fail(e),
    }
}

fn sign_in(args: &Value) -> Value {
    match with(|c| c.sign_in(str_arg(args, "email"), str_arg(args, "password"))) {
        Ok(()) => after_sign_in(agree(args)),
        Err(e) => {
            let mut a = fail(e);
            if e == Error::Code("email_not_verified") {
                a["step"] = json!("verify");
            }
            a
        }
    }
}

fn reset(args: &Value) -> Value {
    let (email, password) = (str_arg(args, "email"), str_arg(args, "password"));
    // (the code signs in, and the new password is set with it)
    match with(|c| c.reset_password(email, str_arg(args, "code"), password)) {
        Ok(()) => {
            let mut a = after_sign_in(agree(args));
            if a.get("ok").is_some() {
                a["message"] = json!(tr("Password changed. You are signed in."));
            }
            a
        }
        Err(e) => fail(e),
    }
}

// --- Google and Discord --------------------------------------------------------------------------

/// The sign-in waiting in the browser (one at a time): its number, the flow until `ol_oauth_wait`
/// takes it, and its cancel.
struct Pending {
    id: u64,
    flow: Option<Flow>,
    cancel: Arc<AtomicBool>,
}

static OAUTH: Mutex<Option<Pending>> = Mutex::new(None);
static OAUTH_ID: AtomicU64 = AtomicU64::new(1);

fn pending() -> MutexGuard<'static, Option<Pending>> {
    OAUTH.lock().unwrap_or_else(|e| e.into_inner())
}

/// The listener's page, in the page's language.
fn oauth_texts() -> Texts {
    let lang = language();
    Texts {
        lang: if lang.is_empty() { "en".into() } else { lang },
        done_title: tr("You are signed in to Omsi-Hub"),
        done_text: tr("You may close this tab and return to Omsi-Hub."),
        failed_title: tr("The sign-in could not be completed"),
        failed_text: tr("Please return to Omsi-Hub and try again."),
        cancelled_text: tr("The sign-in was cancelled. Please return to Omsi-Hub."),
    }
}

/// The sign-in started: the listener, and the address for the page to open (an earlier one still
/// waiting is given up).
fn oauth_start(args: &Value) -> Value {
    // (only after the privacy statement's question, accepted)
    if !agree(args) {
        return fail(Error::Code("privacy"));
    }
    let Some(provider) = Provider::of(args.get("provider").and_then(Value::as_str).unwrap_or("google")) else { return fail(Error::Code("oauth_failed")) };
    let flow = match Flow::start(&endpoints(), provider, oauth_texts()) {
        Ok(f) => f,
        Err(e) => return fail(e),
    };
    let id = OAUTH_ID.fetch_add(1, Ordering::Relaxed);
    let url = flow.url().to_string();
    let mut p = pending();
    if let Some(old) = p.take() {
        old.cancel.store(true, Ordering::Relaxed);
    }
    *p = Some(Pending { id, flow: Some(flow), cancel: Arc::new(AtomicBool::new(false)) });
    json!({ "ok": true, "id": id, "url": url })
}

/// The browser waited for (this command's own thread, a few minutes at most), then signed in with
/// what it handed back, as with a password.
fn oauth_wait(args: &Value) -> Value {
    let id = args.get("id").and_then(Value::as_u64).unwrap_or(0);
    let taken = {
        let mut p = pending();
        p.as_mut().filter(|p| p.id == id).and_then(|p| p.flow.take().map(|f| (f, p.cancel.clone())))
    };
    let Some((flow, cancel)) = taken else { return fail(Error::Code("oauth_cancelled")) };
    let got = flow.wait(oauth::TIMEOUT, &cancel);
    {
        let mut p = pending();
        if p.as_ref().is_some_and(|p| p.id == id) {
            *p = None;
        }
    }
    match got.and_then(|h| with(|c| c.sign_in_with(&h))) {
        Ok(discord) => {
            let a = after_sign_in(agree(args));
            // (signed in with Discord: that Discord account linked, as the website does - quietly:
            // a link that fails is made on the website's account page)
            if let Some(t) = discord.filter(|_| a.get("ok").is_some() && !view().need_privacy) {
                let _ = with(|c| c.discord_claim(&t));
            }
            a
        }
        Err(e) => fail(e),
    }
}

fn oauth_cancel() -> Value {
    if let Some(p) = pending().as_ref() {
        p.cancel.store(true, Ordering::Relaxed);
    }
    done("")
}

fn sign_out() -> Value {
    with(|c| {
        c.goodbye();
        c.sign_out();
    });
    signed_out(&mut view());
    crate::drivers::unlink_account();
    done(&tr("Signed out. Omsi-Hub plays on locally."))
}

fn delete(args: &Value) -> Value {
    // (whose account it was: only its companies are made local)
    let me = with(|c| c.user_id().map(str::to_string));
    match with(|c| c.delete_account(str_arg(args, "password"))) {
        Ok(()) => {
            signed_out(&mut view());
            crate::drivers::unlink_account();
            // (its cloud saves went with it: the companies here are local saves now)
            crate::cloud::forget_account(me.as_deref());
            done(&tr("Your account, your cloud saves and everything of it are deleted."))
        }
        Err(e) => fail(e),
    }
}

fn profile(args: &Value) -> Value {
    let name = str_arg(args, "name");
    let need = view().need_profile;
    answer(with(|c| if need { c.create_profile(name) } else { c.set_display_name(name) }).and_then(|_| refresh(false)).map(|_| done(&tr("Display name saved."))))
}

/// The profile photo: the page's picture (base64, made small and square already), or none
/// (`clear`); the website shows the same one.
fn avatar(args: &Value) -> Value {
    if args.get("clear").and_then(Value::as_bool).unwrap_or(false) {
        return answer(with(Client::clear_avatar).and_then(|_| refresh(false)).map(|_| done(&tr("Profile photo removed."))));
    }
    let Ok(bytes) = crate::launcher::base64(str_arg(args, "data")) else {
        return fail(Error::Code("bad_photo"));
    };
    answer(with(|c| c.set_avatar(&bytes)).and_then(|_| refresh(false)).map(|_| done(&tr("Profile photo saved."))))
}

// --- the leaderboard ------------------------------------------------------------------------------

/// Standing on the public leaderboard switched on or off, then the profile fetched again.
fn leaderboard(args: &Value) -> Value {
    let on = args.get("on").and_then(Value::as_bool).unwrap_or(false);
    answer(with(|c| c.set_leaderboard(on)).and_then(|_| refresh(false)).map(|_| done(&if on { tr("You are on the leaderboard now.") } else { tr("You are off the leaderboard.") })))
}

/// The service record sent for the leaderboard (`ok`, or `too_soon`: one report in 30 seconds
/// is kept). Signed out, waiting for the privacy statement or offline it is not sent. For the
/// company, after a day.
pub fn report_stats(s: &Stats) -> Result<&'static str, Error> {
    if !crate::drivers::plays_online() {
        return Err(Error::Code("not_signed_in"));
    }
    with(|c| if c.signed_in() { c.report_stats(s) } else { Err(Error::Code("not_signed_in")) })
}

fn stats_of(args: &Value) -> Stats {
    let n = |k: &str| args.get(k).and_then(Value::as_u64).unwrap_or(0);
    let small = |k: &str| n(k).min(u32::MAX as u64) as u32;
    Stats { points: n("points"), level: small("level"), duties: small("duties"), minutes_driven: small("minutes"), km: small("km") }
}

// --- the friends ----------------------------------------------------------------------------

fn name_of(id: &str) -> String {
    view().friends.iter().find(|f| f.user_id == id).map(|f| f.display_name.clone()).unwrap_or_else(|| tr("A friend"))
}

/// A change to the friends, then the list fetched again.
fn friends_act(f: impl FnOnce(&mut Client) -> Result<&'static str, Error>, message: impl FnOnce(&'static str) -> String) -> Value {
    match with(f) {
        Ok(code) => {
            let _ = refresh(false);
            done(&message(code))
        }
        Err(e) => fail(e),
    }
}

fn invite(args: &Value) -> Value {
    let id = str_arg(args, "id").to_string();
    let Some(code) = session_code() else { return fail(Error::Code("no_session")) };
    let name = name_of(&id);
    match with(|c| c.send_invite(&id, &code)) {
        Ok(()) => done(&trf("Invite sent to %{name}.", &[("name", &name)])),
        Err(e) => fail(e),
    }
}

fn dismiss(args: &Value) -> Value {
    let key = str_arg(args, "id");
    let Some(id) = view().invites.iter().find(|i| i.key() == key).map(|i| i.id.clone()) else { return done("") };
    match with(|c| c.dismiss_invite(&id)) {
        // (gone from the view at once; the server's list follows at the next beat)
        Ok(()) | Err(Error::Code("not_found")) => {
            view().invites.retain(|i| i.key() != key);
            done("")
        }
        Err(e) => fail(e),
    }
}

/// An `ol_` command.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    set_language(str_arg(args, "lang"));
    let id = || str_arg(args, "id").to_string();
    Ok(match cmd {
        "ol_state" => state(),
        "ol_local" => {
            set_local(args.get("on").and_then(Value::as_bool).unwrap_or(true));
            state()
        }
        "ol_verify" => verify(args),
        "ol_resend" => answer(with(|c| c.send_verification(str_arg(args, "email"))).map(|_| done(&tr("A new email is on its way.")))),
        "ol_sign_in" => sign_in(args),
        "ol_forgot" => answer(with(|c| c.request_reset(str_arg(args, "email"))).map(|_| done(&tr("If an account has this address, an email is on its way to it.")))),
        "ol_reset" => reset(args),
        "ol_sign_out" => sign_out(),
        "ol_oauth" => oauth_start(args),
        "ol_oauth_wait" => oauth_wait(args),
        "ol_oauth_cancel" => oauth_cancel(),
        "ol_privacy" => crate::privacy::doc(),
        "ol_privacy_accept" => privacy_accept(),
        "ol_delete" => delete(args),
        "ol_profile" => profile(args),
        "ol_avatar" => avatar(args),
        "ol_leaderboard" => leaderboard(args),
        "ol_report_stats" => match report_stats(&stats_of(args)) {
            Ok(code) => json!({ "ok": true, "code": code }),
            Err(e) => fail(e),
        },
        "ol_add" => friends_act(|c| c.send_friend_request(str_arg(args, "code")), |code| tr(codes::text(code))),
        "ol_respond" => {
            let accept = args.get("accept").and_then(Value::as_bool).unwrap_or(false);
            let name = name_of(&id());
            friends_act(|c| c.respond_friend_request(&id(), accept).map(|_| ""), |_| if accept { trf("You and %{name} are friends now.", &[("name", &name)]) } else { tr("Request declined.") })
        }
        "ol_remove" => {
            let name = name_of(&id());
            friends_act(|c| c.remove_friend(&id()).map(|_| ""), |_| trf("%{name} is no longer your friend.", &[("name", &name)]))
        }
        "ol_block" => {
            let name = name_of(&id());
            friends_act(|c| c.block_player(&id()).map(|_| ""), |_| trf("%{name} is blocked.", &[("name", &name)]))
        }
        "ol_invite" => invite(args),
        "ol_dismiss" => dismiss(args),
        "ol_presence" => {
            view().away = args.get("away").and_then(Value::as_bool).unwrap_or(false);
            json!({})
        }
        "ol_seen" => {
            let mut v = view();
            let keys: Vec<String> = v.friends.iter().filter(|f| f.relation == "pending").map(|f| format!("r:{}", f.user_id)).chain(v.invites.iter().map(|i| format!("i:{}", i.key()))).collect();
            v.seen.extend(keys);
            json!({})
        }
        "ol_refresh" => {
            // (the panel opened: at most every ten seconds)
            let recent = unix_now().saturating_sub(view().refreshed_at) < 10;
            if !recent && with(|c| c.signed_in()) {
                let _ = refresh(false);
            }
            state()
        }
        _ => return Err(format!("unknown command: {cmd}")),
    })
}

/// The texts between `lead` and `end` in `src` (escapes read), with two letters at least: what a
/// source file gives `tr` to translate.
#[cfg(test)]
pub(crate) fn texts_in(src: &str, lead: &str, end: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(k) = rest.find(lead) {
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
            out.push(t);
        }
    }
    out
}

/// Every text of `texts` missing in one of the seven languages.
#[cfg(test)]
pub(crate) fn untranslated(texts: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    for lang in ["nl", "de", "fr", "ru", "uk", "pl", "es"] {
        let table = table(lang);
        for t in texts {
            if !table.contains_key(t) {
                missing.push(format!("{lang}: {t:?}"));
            }
        }
    }
    missing
}

/// The tests' own data folder (one for the whole test binary: the commands read it).
#[cfg(test)]
pub(crate) fn test_data_dir() -> std::path::PathBuf {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!("omsihub-api-online-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::env::set_var(hub_core::paths::DATA_DIR_ENV, &d);
        d
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Quiet;

    fn run(cmd: &str, args: Value) -> Value {
        test_data_dir();
        call(cmd, &args, &Quiet).unwrap()
    }

    /// Every text of Omsi-Hub online - the codes, this file's, the page's (`online/*.js`, and the
    /// privacy statement's `privacy.js`) - in the seven languages.
    #[test]
    fn every_online_text_is_translated() {
        let mut texts: Vec<String> = codes::TEXTS.iter().map(|(_, t)| t.to_string()).collect();
        let own = include_str!("online.rs").replace("\r\n", "\n");
        let own = own.split("#[cfg(test)]\npub(crate) fn texts_in").next().unwrap_or("");
        let js = include_str!("../../../ui/js/online/online.js");
        let pv = include_str!("../../../ui/js/privacy.js");
        for (src, lead, end) in [(own, "tr(\"", '"'), (own, "trf(\"", '"'), (js, "tr('", '\''), (pv, "tr('", '\'')] {
            texts.extend(texts_in(src, lead, end));
        }
        assert!(texts.len() > 80, "only {} texts found", texts.len());
        let missing = untranslated(&texts);
        assert!(missing.is_empty(), "{} missing (crates/hub-locales/locales/online.yml):\n{}", missing.len(), missing.join("\n"));
    }

    /// Google and Discord: only with the privacy statement accepted, the address for the browser
    /// answered, and a cancel ends the wait at once.
    #[test]
    fn a_browser_sign_in_starts_waits_and_cancels() {
        let v = run("ol_oauth", json!({ "lang": "de" }));
        assert_eq!(v["code"], "privacy");
        // (a second start gives the first one up)
        let first = run("ol_oauth", json!({ "agree": true }))["id"].as_u64().unwrap();
        let v = run("ol_oauth", json!({ "agree": true }));
        assert!(v["url"].as_str().unwrap().starts_with("https://127.0.0.1:9/auth/v1/authorize?provider=google&redirect_to="), "{v}");
        let v = run("ol_oauth", json!({ "agree": true, "provider": "discord" }));
        let url = v["url"].as_str().unwrap();
        assert!(url.starts_with("https://127.0.0.1:9/auth/v1/authorize?provider=discord&redirect_to=") && url.ends_with("&scopes=identify%20email%20guilds.join"), "{v}");
        assert_eq!(run("ol_oauth", json!({ "agree": true, "provider": "steam" }))["code"], "oauth_failed");
        let id = v["id"].as_u64().unwrap();
        run("ol_oauth_cancel", json!({}));
        let t = std::time::Instant::now();
        let w = run("ol_oauth_wait", json!({ "id": id, "agree": true, "lang": "nl" }));
        assert_eq!(w["code"], "oauth_cancelled");
        assert_eq!(w["error"], table("nl")["The sign-in was cancelled."].as_str());
        assert!(t.elapsed().as_secs() < 2);
        // (a number that is not the one waiting)
        assert_eq!(run("ol_oauth_wait", json!({ "id": first }))["code"], "oauth_cancelled");
    }

    #[test]
    fn the_servers_times_are_read() {
        assert_eq!(unix_of("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix_of("2026-10-16T10:00:00+00:00"), Some(1_792_144_800));
        assert_eq!(unix_of("2026-10-16T12:00:00.123456+02:00"), Some(1_792_144_800));
        assert_eq!(unix_of("2026-10-16T09:30:00-0030"), Some(1_792_144_800));
        assert_eq!(unix_of("garbage"), None);
        assert_eq!(seen_words(None), "");
    }

    #[test]
    fn a_friends_state_is_one_of_four() {
        let f = |online, status: &str| Friend { online, status: status.into(), ..Default::default() };
        assert_eq!(state_of(&f(false, "busy")), "offline");
        assert_eq!(state_of(&f(true, "busy")), "busy");
        assert_eq!(state_of(&f(true, "away")), "away");
        assert_eq!(state_of(&f(true, "online")), "online");
        assert_eq!(state_of(&f(true, "offline")), "offline");
    }

    /// Without a network the commands answer at once with "offline", in the page's language,
    /// and what is typed wrong is said before anything is sent.
    #[test]
    fn offline_and_typos_are_said_in_the_pages_language() {
        let v = run("ol_sign_in", json!({ "email": "not an address", "password": "12345678", "lang": "de" }));
        assert_eq!(v["code"], "invalid_email");
        assert_eq!(v["error"], table("de")["That is not an email address."].as_str());
        let t = std::time::Instant::now();
        let v = run("ol_sign_in", json!({ "email": "a@b.example", "password": "12345678" }));
        assert_eq!(v["code"], "offline");
        assert!(t.elapsed().as_secs() < 8);
        let s = run("ol_state", json!({}));
        assert_eq!(s["offline"], true);
        assert_ne!(s["mode"], "in");
        assert_eq!(s["privacy"]["version"], privacy::VERSION);
        assert_eq!(run("ol_add", json!({ "code": "AB12CD34" }))["code"], "not_signed_in");
        assert_eq!(run("ol_invite", json!({ "id": "u2" }))["code"], "no_session");
        assert_eq!(run("ol_leaderboard", json!({ "on": true }))["code"], "not_signed_in");
        assert_eq!(run("ol_report_stats", json!({ "points": 10, "level": 2 }))["code"], "not_signed_in");
        assert_eq!(call("ol_nothing", &json!({}), &Quiet), Err("unknown command: ol_nothing".into()));
    }

    #[test]
    fn the_service_record_is_read_from_the_page() {
        let s = stats_of(&json!({ "points": 1200, "level": 3, "duties": 7, "minutes": 95, "km": 41, "lang": "nl" }));
        assert_eq!(s, Stats { points: 1200, level: 3, duties: 7, minutes_driven: 95, km: 41 });
        assert_eq!(stats_of(&json!({})), Stats::default());
    }

    /// Hosting: the friends see the session's code only while the company lets them join without
    /// an invite.
    #[test]
    fn the_session_code_goes_to_friends_only_when_they_may_join() {
        let h = |friends_join| Some(Hosted { code: "HUB-ABC".into(), friends_join });
        assert_eq!(presence_of(None, false), ("online", None));
        assert_eq!(presence_of(None, true), ("away", None));
        assert_eq!(presence_of(h(false), false), ("busy", None));
        assert_eq!(presence_of(h(true), true), ("busy", Some("HUB-ABC".into())));
    }

    /// "Play without an account" is kept in the data folder, and only that.
    #[test]
    fn playing_without_an_account_is_kept() {
        let d = test_data_dir();
        assert_eq!(run("ol_local", json!({ "on": true }))["mode"], "local");
        assert_eq!(std::fs::read_to_string(d.join(FILE)).unwrap(), r#"{"local":true}"#);
        assert_eq!(run("ol_local", json!({ "on": false }))["mode"], "out");
    }
}
