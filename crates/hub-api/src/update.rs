//! The app's own updates, from the GitHub releases of its public repository `Omsi-Hub/Omsi-Hub-Addon`.
//! Nothing of openOMSI's updater: the look is ours, the installation Tauri's updater plugin's.
//!
//! * **The look** (here): the releases asked of GitHub's API a second after the start, again
//!   every half hour and when a game started from here has ended. Drafts and pre-releases are
//!   skipped (and so is a tag with a `-` in it, `0.3.0-beta1`); the newest version above this
//!   one is offered, with its page and its asset for this platform (the NSIS installer
//!   `…-setup.exe` on Windows, the `.app.tar.gz` on macOS, the `.AppImage` on Linux).
//! * **The offer**: a dialog over the page (`hub_update`, asked about once a second): Update now,
//!   Not now (that version is not offered again), What's new; "install updates without asking"
//!   (`update_auto`) while no game runs; Settings' "Check now" and "look for updates"
//!   (`update_check`).
//! * **The installation** (the app, [`Installer`]): `tauri-plugin-updater` downloads the asset
//!   named in the release's `latest.json`, checks its signature against the public key built
//!   into the app, installs it and starts the new version (the NSIS installer does that itself
//!   on Windows; on macOS the bundle and on Linux the AppImage file is replaced and the app
//!   restarted). On Linux only an AppImage updates itself: a copy from a `.deb` is offered the
//!   release's page. Without a registered
//!   installer (the tests, a build without a signing key) "Update now" says so and offers the
//!   release's page instead.
//!
//! The commands: `hub_update` (the state, the dialog, the Settings line) and
//! `hub_update_act {what}` with `what`: `install`, `dismiss`, `check`, `page`, `github`, `repo`,
//! `auto {on}`, `look {on}`. `OMSIHUB_NO_UPDATE=1` never looks. What Luc sets up for releases is
//! in docs/UPDATES.md.

use crate::online::{set_language, tr, trf};
use crate::{Answer, Platform};
use hub_core::settings::Settings;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// The repository whose releases are Omsi-Hub's: public, the installers and `latest.json` in each
/// (the release workflow puts them there).
pub const REPO: &str = "Omsi-Hub/Omsi-Hub-Addon";
pub const REPO_URL: &str = "https://github.com/Omsi-Hub/Omsi-Hub-Addon";
/// The releases, newest first (drafts are only listed to the repository's owners).
const API: &str = "https://api.github.com/repos/Omsi-Hub/Omsi-Hub-Addon/releases?per_page=20";
/// What Tauri's updater reads: the newest release's `latest.json` (GitHub's `latest` skips drafts
/// and pre-releases too).
pub const LATEST_JSON: &str = "https://github.com/Omsi-Hub/Omsi-Hub-Addon/releases/latest/download/latest.json";
/// The environment variable that switches the looks off.
pub const NO_UPDATE_ENV: &str = "OMSIHUB_NO_UPDATE";

/// This app's version.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// --- the versions and the releases ----------------------------------------------------------------

/// A version's numbers (`v0.2.10` → [0, 2, 10]); None for a test build (`-beta1`) or no
/// version.
pub fn parse_version(s: &str) -> Option<Vec<u64>> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    if s.is_empty() || s.contains('-') {
        return None;
    }
    // (build metadata, `+abc`, says nothing of the order)
    let s = s.split('+').next().unwrap_or("");
    s.split('.').map(|p| p.parse::<u64>().ok()).collect::<Option<Vec<u64>>>().filter(|v| !v.is_empty() && v.len() <= 4)
}

/// Whether version `a` is newer than `b` (a missing number is 0: `0.2` = `0.2.0`).
pub fn newer(a: &[u64], b: &[u64]) -> bool {
    let n = a.len().max(b.len());
    let at = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    (0..n).map(|i| (at(a, i), at(b, i))).find(|(x, y)| x != y).is_some_and(|(x, y)| x > y)
}

/// A test build (`0.2.0-beta3`): it is never updated, it would replace the build being tested.
pub fn is_test_build(v: &str) -> bool {
    v.contains('-')
}

/// A release's file for this platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A release on offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    /// Its page on GitHub ("What's new").
    pub page: String,
    pub asset: Option<Asset>,
    /// It has a `latest.json`: Tauri's updater can install it.
    pub installable: bool,
}

/// The platform's name as the assets have it: `windows`, `macos` or `linux`, with the processor.
fn platform() -> (&'static str, &'static str) {
    (std::env::consts::OS, if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" })
}

/// The names a processor goes by in the assets' names (Tauri's Linux bundles say `amd64`).
fn arch_names(arch: &str) -> &'static [&'static str] {
    match arch {
        "aarch64" => &["aarch64", "arm64"],
        _ => &["x64", "amd64", "x86_64"],
    }
}

/// The asset of a release for `os` and `arch`: Tauri's bundles (`Omsi-Hub_0.2.0_x64-setup.exe`,
/// `Omsi-Hub_aarch64.app.tar.gz` or `_universal`, `Omsi-Hub_0.2.0_amd64.AppImage`).
pub fn asset_for(assets: &[Value], os: &str, arch: &str) -> Option<Asset> {
    let all: Vec<Asset> = assets
        .iter()
        .filter_map(|a| Some(Asset { name: a.get("name")?.as_str()?.to_string(), url: a.get("browser_download_url")?.as_str()?.to_string(), size: a.get("size").and_then(Value::as_u64).unwrap_or(0) }))
        .collect();
    let lower = |a: &Asset| a.name.to_ascii_lowercase();
    let pick = |ends: &str| {
        let fits: Vec<&Asset> = all.iter().filter(|a| lower(a).ends_with(ends)).collect();
        // (this processor's first, then a universal one, then any)
        fits.iter().find(|a| arch_names(arch).iter().any(|n| lower(a).contains(n))).or_else(|| fits.iter().find(|a| lower(a).contains("universal"))).or_else(|| fits.first()).map(|a| (*a).clone())
    };
    match os {
        "windows" => pick("-setup.exe"),
        "macos" => pick(".app.tar.gz").or_else(|| pick(".dmg")),
        _ => pick(".appimage"),
    }
}

/// The newest release above `current` in GitHub's list (drafts, pre-releases and test tags
/// skipped), with its asset for `os`/`arch`.
pub fn pick(releases: &Value, current: &str, os: &str, arch: &str) -> Option<Release> {
    let now = parse_version(current)?;
    let mut best: Option<(Vec<u64>, Release)> = None;
    for r in releases.as_array()? {
        if r.get("draft").and_then(Value::as_bool).unwrap_or(true) || r.get("prerelease").and_then(Value::as_bool).unwrap_or(true) {
            continue;
        }
        let tag = r.get("tag_name").and_then(Value::as_str).unwrap_or("");
        let Some(v) = parse_version(tag) else { continue };
        if !newer(&v, &now) || best.as_ref().is_some_and(|(b, _)| !newer(&v, b)) {
            continue;
        }
        let assets = r.get("assets").and_then(Value::as_array).cloned().unwrap_or_default();
        let installable = assets.iter().any(|a| a.get("name").and_then(Value::as_str) == Some("latest.json"));
        let version = v.iter().map(u64::to_string).collect::<Vec<_>>().join(".");
        let page = r.get("html_url").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("{REPO_URL}/releases/tag/{tag}"));
        best = Some((v, Release { version, page, asset: asset_for(&assets, os, arch), installable }));
    }
    best.map(|b| b.1)
}

/// GitHub asked (a few seconds at most): the newest release above this version, or None.
fn look() -> Result<Option<Release>, String> {
    let text = ureq::get(API)
        .set("User-Agent", &format!("Omsi-Hub/{}", current_version()))
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => trf("GitHub answered %{code}", &[("code", &code.to_string())]),
            ureq::Error::Transport(_) => tr("GitHub could not be reached. Are you online?"),
        })?
        .into_string()
        .map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|_| tr("GitHub's answer could not be read"))?;
    let (os, arch) = platform();
    Ok(pick(&v, current_version(), os, arch))
}

// --- the installation: the app's -------------------------------------------------------------------

/// What installs an update: the app's Tauri updater plugin.
pub trait Installer: Send + Sync {
    /// Download `version` (its progress: done, total), check its signature and install it. The
    /// new version starts afterwards ([`Installer::restart`]); on Windows the installer ends
    /// this app itself, and this call may not return.
    fn install(&self, version: &str, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<(), String>;
    /// The app started again (the new version).
    fn restart(&self);
}

static INSTALLER: OnceLock<Arc<dyn Installer>> = OnceLock::new();

/// The app's installer, set once as it starts.
pub fn set_installer(i: Arc<dyn Installer>) {
    let _ = INSTALLER.set(i);
}

fn installer() -> Option<Arc<dyn Installer>> {
    INSTALLER.get().cloned()
}

/// Whether this copy can put a new version in its own place: on Linux only the AppImage can
/// (`APPIMAGE` is its path); a copy from a `.deb` or a build of its own is updated by hand.
fn self_updating(os: &str, appimage: bool) -> bool {
    os != "linux" || appimage
}

/// `release` installed from within the app: signed (`latest.json`), the app's updater there, and
/// this copy one that can be replaced.
fn can_install(release: &Release) -> bool {
    release.installable && installer().is_some() && self_updating(std::env::consts::OS, std::env::var_os("APPIMAGE").is_some())
}

// --- the state --------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Status {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading {
        release: Release,
        done: u64,
        total: u64,
    },
    Installing(Release),
    /// Installed: the new version starts.
    Restarting(Release),
    Failed(String),
}

struct State {
    status: Status,
    started: Instant,
    checked_once: bool,
    last_check: Option<Instant>,
    /// The version the player said "Not now" to (not offered again).
    dismissed_version: Option<String>,
    /// The failure closed.
    dismissed: bool,
    /// "Install without asking" started for this version already.
    auto_started: bool,
    game_was_running: bool,
    restarted: bool,
    /// The player asked for what runs now (Check now, Update now): a failure is said in the
    /// dialog. A look of its own that fails (offline) is said on the Settings line only.
    asked: bool,
    /// "Updated to Omsi-Hub x", said once after an update.
    updated: Option<String>,
}

fn state() -> MutexGuard<'static, State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State {
            status: Status::Idle,
            started: Instant::now(),
            checked_once: false,
            last_check: None,
            dismissed_version: None,
            dismissed: false,
            auto_started: false,
            game_was_running: false,
            restarted: false,
            asked: false,
            updated: just_updated(),
        })
    })
    .lock()
    .unwrap_or_else(|e| e.into_inner())
}

/// The version that ran before this one, from the data folder (`omsihub-update.json`), and this
/// one kept for the next start: Some(this version) once after an update.
fn just_updated() -> Option<String> {
    let path = hub_core::data_dir().join("omsihub-update.json");
    let before = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string));
    let now = current_version();
    let _ = std::fs::write(&path, json!({ "version": now }).to_string());
    let older = before.as_deref().and_then(parse_version).zip(parse_version(now)).is_some_and(|(b, n)| newer(&n, &b));
    older.then(|| now.to_string())
}

fn setting(key: &str, default: bool) -> bool {
    Settings::load().other.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn set_setting(key: &str, on: bool) -> Result<(), String> {
    let mut s = Settings::load();
    s.other.insert(key.into(), Value::Bool(on));
    s.save().map_err(|e| e.to_string())
}

/// Whether the app looks for updates (Settings, and not switched off for this run).
fn looking() -> bool {
    setting("update_check", true) && std::env::var_os(NO_UPDATE_ENV).is_none_or(|v| v.is_empty() || v == "0")
}

fn game_running() -> bool {
    crate::drive::state()["running"] == true
}

/// A look started on its own thread (the page's command answers at once); `asked`: by the
/// player.
fn check(s: &mut State, asked: bool) {
    if matches!(s.status, Status::Checking | Status::Downloading { .. } | Status::Installing(_) | Status::Restarting(_)) {
        return;
    }
    s.status = Status::Checking;
    s.asked = asked;
    s.checked_once = true;
    s.dismissed = false;
    s.last_check = Some(Instant::now());
    let lang = crate::online::language();
    let _ = std::thread::Builder::new().name("omsihub update look".into()).spawn(move || {
        set_language(&lang);
        let r = look();
        let mut s = state();
        s.status = match r {
            Ok(Some(rel)) => Status::Available(rel),
            Ok(None) => Status::UpToDate,
            Err(e) => {
                hub_core::log_line(&format!("update: the look failed: {e}"));
                Status::Failed(e)
            }
        };
    });
}

/// `release` downloaded and installed on its own thread; its progress into the state.
fn install(s: &mut State, release: Release) {
    let Some(inst) = installer().filter(|_| can_install(&release)) else {
        // (no updater in this build, or a release without `latest.json`: its page instead)
        s.status = Status::Failed(tr("This version cannot be installed from within the app. Download it from its page on GitHub."));
        s.dismissed = false;
        return;
    };
    s.status = Status::Downloading { release: release.clone(), done: 0, total: release.asset.as_ref().map_or(0, |a| a.size) };
    hub_core::log_line(&format!("update: installing {}", release.version));
    let _ = std::thread::Builder::new().name("omsihub update install".into()).spawn(move || {
        let mut progress = |done: u64, total: Option<u64>| {
            let mut s = state();
            if let Status::Downloading { done: d, total: t, .. } = &mut s.status {
                *d += done;
                if let Some(n) = total {
                    *t = n;
                }
            }
        };
        let r = inst.install(&release.version, &mut progress);
        let mut s = state();
        match r {
            Ok(()) => {
                hub_core::log_line(&format!("update: {} installed, the new version starts", release.version));
                s.status = Status::Restarting(release);
            }
            Err(e) => {
                hub_core::log_line(&format!("update: {} not installed: {e}", release.version));
                s.status = Status::Failed(e);
                s.dismissed = false;
            }
        }
    });
}

/// The download done: the installation follows (said so until the plugin is through).
fn downloaded(s: &mut State) {
    if let Status::Downloading { release, done, total } = &s.status {
        if *total > 0 && done >= total {
            s.status = Status::Installing(release.clone());
        }
    }
}

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

/// What the dialog's buttons and the Settings' switches do. An address for the page to open
/// (the answer's `open`: the page opens it in the system's browser), when there is one.
fn act(what: &str, args: &Value) -> Result<Option<String>, String> {
    let mut s = state();
    match what {
        "install" => {
            if let Status::Available(r) = s.status.clone() {
                s.asked = true;
                install(&mut s, r);
            }
        }
        "dismiss" => {
            if let Status::Available(r) = &s.status {
                s.dismissed_version = Some(r.version.clone());
            }
            s.dismissed = true;
        }
        "check" => {
            s.dismissed_version = None;
            check(&mut s, true);
        }
        "page" => {
            let page = match &s.status {
                Status::Available(r) | Status::Downloading { release: r, .. } => r.page.clone(),
                _ => format!("{REPO_URL}/releases/latest"),
            };
            return Ok(Some(page));
        }
        "github" => return Ok(Some(format!("{REPO_URL}/releases/latest"))),
        "repo" => return Ok(Some(REPO_URL.to_string())),
        "auto" => set_setting("update_auto", args.get("on").and_then(Value::as_bool).unwrap_or(false))?,
        "look" => set_setting("update_check", args.get("on").and_then(Value::as_bool).unwrap_or(true))?,
        _ => return Err(format!("unknown update action: {what}")),
    }
    Ok(None)
}

/// About once a second from the page: the looks due, the installation chosen beforehand, the
/// new version started once it is in place - and what the page shows of it.
fn tick() -> Value {
    let mut guard = state();
    let s = &mut *guard;
    let looking = looking();
    if !s.checked_once && s.started.elapsed() > Duration::from_secs(1) {
        if looking && !is_test_build(current_version()) {
            check(s, false);
        } else {
            s.checked_once = true;
        }
    }
    // (a game started from here: nothing is installed while it runs - once it ends, a look)
    let game = game_running();
    if s.game_was_running && !game && looking {
        s.dismissed_version = None;
        s.auto_started = false;
        check(s, false);
    }
    s.game_was_running = game;
    // (and every half hour while the app stays open)
    // (a look of its own that failed is never shown to be closed, and an offer put aside waits
    // for a newer version: both are looked again)
    let idle = match &s.status {
        Status::Idle | Status::UpToDate => true,
        Status::Failed(_) => s.dismissed || !s.asked,
        Status::Available(_) => s.dismissed,
        _ => false,
    };
    if looking && idle && s.last_check.is_some_and(|t| t.elapsed() > Duration::from_secs(30 * 60)) {
        check(s, false);
    }
    downloaded(s);
    let auto = setting("update_auto", false);
    if let Status::Available(r) = s.status.clone() {
        // (an offer put aside comes back for a newer version only)
        s.dismissed = s.dismissed_version.as_deref() == Some(r.version.as_str());
        // "install updates without asking" (not while a game runs)
        if auto && !s.dismissed && !s.auto_started && !game && can_install(&r) {
            s.auto_started = true;
            install(s, r);
        }
    }
    if let Status::Restarting(_) = s.status {
        if !s.restarted {
            s.restarted = true;
            if let Some(i) = installer() {
                // (once the page has its answer)
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(400));
                    i.restart();
                });
            }
        }
    }
    view(s, auto, game)
}

/// What the page shows: the dialog over it, the Settings line, the notice after an update.
fn view(s: &mut State, auto: bool, game: bool) -> Value {
    let current = current_version();
    // (while a game runs the offer waits)
    let open = !game
        && match &s.status {
            Status::Available(_) => !s.dismissed,
            Status::Failed(_) => !s.dismissed && s.asked,
            Status::Downloading { .. } | Status::Installing(_) | Status::Restarting(_) => true,
            _ => false,
        };
    let dialog = open.then(|| match &s.status {
        Status::Available(rel) => json!({
            "kind": "available", "icon": "system_update",
            "title": trf("Omsi-Hub %{version} is available", &[("version", &rel.version)]),
            "text": trf("You have %{current}. Update now? Omsi-Hub downloads the new version (%{size}) from GitHub, installs it and starts again. Your companies and settings stay as they are.", &[("current", current), ("size", &rel.asset.as_ref().map_or("?".into(), |a| mb(a.size)))]),
            "auto": auto, "installable": can_install(rel),
        }),
        Status::Downloading { release, done, total } => json!({
            "kind": "busy", "icon": "download",
            "title": trf("Downloading Omsi-Hub %{version}", &[("version", &release.version)]),
            "text": trf("%{done} of %{total} from %{source}", &[("done", &mb(*done)), ("total", &mb(*total)), ("source", &format!("github.com/{REPO}"))]),
            "frac": if *total > 0 { (*done as f64 / *total as f64).min(1.0) } else { 0.0 },
        }),
        Status::Installing(release) | Status::Restarting(release) => json!({
            "kind": "busy", "icon": "install_desktop",
            "title": trf("Installing Omsi-Hub %{version}", &[("version", &release.version)]),
            "text": tr("The new version is put in place; Omsi-Hub starts again in a moment."),
            "frac": 1.0,
        }),
        Status::Failed(msg) => json!({ "kind": "failed", "icon": "error", "title": tr("Not updated"), "text": msg }),
        _ => Value::Null,
    });
    let line = match &s.status {
        Status::UpToDate | Status::Idle if is_test_build(current) => trf("%{version} is a test build: it is not updated", &[("version", current)]),
        Status::UpToDate => trf("%{version} is the latest version", &[("version", current)]),
        Status::Available(rel) => trf("%{version} is available", &[("version", &rel.version)]),
        Status::Failed(_) => tr("The last check failed"),
        Status::Checking => tr("Checking for updates..."),
        _ => trf("This is Omsi-Hub %{version}", &[("version", current)]),
    };
    let busy = matches!(s.status, Status::Checking | Status::Downloading { .. } | Status::Installing(_) | Status::Restarting(_));
    let updated = s.updated.take().map(|v| trf("Updated to Omsi-Hub %{version}", &[("version", &v)]));
    json!({
        "dialog": dialog,
        "line": line,
        "busy": busy,
        "repo": format!("github.com/{REPO}"),
        "updated": updated,
        "current": current,
        "look": looking(),
        "auto": auto,
    })
}

/// A `hub_update…` command.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    set_language(args.get("lang").and_then(Value::as_str).unwrap_or(""));
    match cmd {
        "hub_update" => Ok(tick()),
        "hub_update_act" => {
            let url = act(args.get("what").and_then(Value::as_str).unwrap_or(""), args)?;
            let mut v = tick();
            v["open"] = json!(url);
            Ok(v)
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool, pre: bool, assets: &[(&str, u64)]) -> Value {
        json!({
            "tag_name": tag, "draft": draft, "prerelease": pre, "html_url": format!("{REPO_URL}/releases/tag/{tag}"),
            "assets": assets.iter().map(|(n, size)| json!({ "name": n, "size": size, "browser_download_url": format!("{REPO_URL}/releases/download/{tag}/{n}") })).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn versions_are_compared_by_their_numbers() {
        assert_eq!(parse_version("v0.2.10"), Some(vec![0, 2, 10]));
        assert_eq!(parse_version("1.0"), Some(vec![1, 0]));
        assert_eq!(parse_version("0.3.0+build7"), Some(vec![0, 3, 0]));
        assert_eq!(parse_version("0.3.0-beta1"), None);
        assert_eq!(parse_version("latest"), None);
        assert!(newer(&[0, 2, 10], &[0, 2, 9]));
        assert!(!newer(&[0, 2, 9], &[0, 2, 10]));
        assert!(!newer(&[0, 2], &[0, 2, 0]), "0.2 is 0.2.0");
        assert!(newer(&[1], &[0, 9, 9]));
        assert!(is_test_build("0.2.0-beta3") && !is_test_build(current_version()));
    }

    /// Drafts, pre-releases and test tags are never offered; the newest above this version is,
    /// with its installer for the platform.
    #[test]
    fn the_newest_proper_release_above_this_version_is_offered() {
        let win = [
            ("Omsi-Hub_0.3.0_x64-setup.exe", 9_000_000),
            ("Omsi-Hub_0.3.0_x64_en-US.msi", 9_100_000),
            ("latest.json", 900),
            ("Omsi-Hub_aarch64.app.tar.gz", 12_000_000),
            ("Omsi-Hub_x64.app.tar.gz", 12_500_000),
            ("Omsi-Hub_0.3.0_aarch64.dmg", 13_000_000),
        ];
        let list = json!([
            release("v0.5.0", true, false, &win),
            release("v0.4.0", false, true, &win),
            release("v0.4.1-beta2", false, false, &win),
            release("v0.3.0", false, false, &win),
            release("v0.2.5", false, false, &[]),
            release("v0.0.9", false, false, &win),
        ]);
        let r = pick(&list, "0.2.0", "windows", "x64").unwrap();
        assert_eq!(r.version, "0.3.0");
        assert!(r.installable);
        assert_eq!(r.page, format!("{REPO_URL}/releases/tag/v0.3.0"));
        assert_eq!(r.asset.as_ref().map(|a| (a.name.as_str(), a.size)), Some(("Omsi-Hub_0.3.0_x64-setup.exe", 9_000_000)));
        assert_eq!(pick(&list, "0.2.0", "macos", "aarch64").unwrap().asset.unwrap().name, "Omsi-Hub_aarch64.app.tar.gz");
        assert_eq!(pick(&list, "0.2.0", "macos", "x64").unwrap().asset.unwrap().name, "Omsi-Hub_x64.app.tar.gz");
        // (up to date, or newer than every release: nothing)
        assert_eq!(pick(&list, "0.3.0", "windows", "x64"), None);
        assert_eq!(pick(&list, "1.0.0", "windows", "x64"), None);
        // (a release without `latest.json` is offered, to be downloaded from its page)
        let r = pick(&list, "0.2.4", "windows", "x64").unwrap();
        assert_eq!(r.version, "0.3.0");
        let bare = json!([release("v0.2.5", false, false, &[("Omsi-Hub_0.2.5_x64-setup.exe", 1)])]);
        assert!(!pick(&bare, "0.2.4", "windows", "x64").unwrap().installable);
        // (a test build is offered nothing; GitHub's error object is no list)
        assert_eq!(pick(&list, "0.2.0-beta1", "windows", "x64"), None);
        assert_eq!(pick(&json!({ "message": "rate limited" }), "0.2.0", "windows", "x64"), None);
    }

    /// Linux: Tauri's AppImage (`amd64`, `aarch64`), never the `.deb`; and only an AppImage
    /// replaces itself.
    #[test]
    fn linux_takes_the_appimage_for_its_processor() {
        let a = |n: &str| json!({ "name": n, "size": 1, "browser_download_url": n });
        let assets = vec![a("Omsi-Hub_0.3.0_amd64.deb"), a("Omsi-Hub_0.3.0_aarch64.AppImage"), a("Omsi-Hub_0.3.0_amd64.AppImage"), a("Omsi-Hub_0.3.0_amd64.AppImage.sig")];
        assert_eq!(asset_for(&assets, "linux", "x64").unwrap().name, "Omsi-Hub_0.3.0_amd64.AppImage");
        assert_eq!(asset_for(&assets, "linux", "aarch64").unwrap().name, "Omsi-Hub_0.3.0_aarch64.AppImage");
        assert_eq!(asset_for(&[a("Omsi-Hub_0.3.0_amd64.deb")], "linux", "x64"), None);
        assert!(self_updating("linux", true) && !self_updating("linux", false));
        assert!(self_updating("windows", false) && self_updating("macos", false));
    }

    #[test]
    fn a_universal_mac_bundle_is_taken_when_there_is_no_own_one() {
        let assets =
            vec![json!({ "name": "Omsi-Hub_universal.app.tar.gz", "size": 5, "browser_download_url": "u" }), json!({ "name": "Omsi-Hub_0.3.0_universal.dmg", "size": 6, "browser_download_url": "d" })];
        assert_eq!(asset_for(&assets, "macos", "aarch64").unwrap().name, "Omsi-Hub_universal.app.tar.gz");
        assert_eq!(asset_for(&assets, "windows", "x64"), None);
    }

    /// The repository is this app's own, not openOMSI's or the old fork's.
    #[test]
    fn the_releases_are_this_repositorys() {
        assert_eq!(REPO, "Omsi-Hub/Omsi-Hub-Addon");
        assert!(API.contains(REPO) && LATEST_JSON.contains(REPO) && REPO_URL.ends_with(REPO));
        assert!(LATEST_JSON.ends_with("/releases/latest/download/latest.json"));
    }

    /// Every text of the updates - this file's and the page's (`update.js`) - in the six
    /// languages.
    #[test]
    fn every_update_text_is_translated() {
        let own = include_str!("update.rs").replace("\r\n", "\n");
        let own = own.split("#[cfg(test)]\nmod tests").next().unwrap_or("").to_string();
        let js = include_str!("../../../ui/js/update.js").replace("\r\n", "\n");
        let mut texts = Vec::new();
        for (src, lead, end) in [(own.as_str(), "tr(\"", '"'), (own.as_str(), "trf(\"", '"'), (js.as_str(), "tr('", '\'')] {
            texts.extend(crate::online::texts_in(src, lead, end));
        }
        assert!(texts.len() > 15, "only {} texts found", texts.len());
        let missing = crate::online::untranslated(&texts);
        assert!(missing.is_empty(), "{} missing (crates/hub-locales/locales/update.yml):\n{}", missing.len(), missing.join("\n"));
    }
}
