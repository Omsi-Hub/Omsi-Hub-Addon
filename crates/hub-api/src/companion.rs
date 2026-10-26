//! The phone and tablet companion (`hub_companion_`): `hub-companion`'s server, run by the app
//! while the player has it on (Settings › General › Phone & tablet; off by default), and what
//! it needs of the rest of Omsi-Hub - the plugin's folder, the assignment last given, and the
//! map's routes and roads as the route map reads them (`content::mapview`).
//!
//! * `hub_companion_state` - on or off, the port, and while on: the address, the pairing code,
//!   the QR code of the pairing address (`{size, bits}`, as the duty card draws one), the paired
//!   devices and whether the game is heard;
//! * `hub_companion_set {on, port?}` - switched on or off (kept in the settings: `companion`,
//!   `companion_port`), answered with the state;
//! * `hub_companion_code` - another pairing code;
//! * `hub_companion_forget {id}` - a paired device removed;
//! * `hub_companion_firewall` - Windows' firewall set to let the phones in (see [`firewall`]).
//!
//! [`start_if_on`] starts it with the app.

use crate::{arg, Answer, Platform};
use hub_companion::{Companion, Config, Source};
use hub_core::settings::Settings;
use hub_game::bridge::Assignment;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The settings' keys.
const ON: &str = "companion";
const PORT: &str = "companion_port";

/// Windows' firewall and the phones: a player should never have to deal with it. Windows lets
/// nothing reach a program it has no rule for, asks once when it first listens - for private
/// networks only, while a home network is often called public - and a question clicked away
/// leaves a rule that blocks it for good. So while the server runs, the add-on looks whether
/// Windows lets the phones in (its own rule for the port, no rule that blocks the program), and
/// when it does not, sets that right once: the rules blocking the program removed, one rule
/// letting the port in on every kind of network - Windows asks whether Omsi-Hub may (UAC), the
/// only thing the player sees of it. The server itself still lets only the home network in
/// (`hub_companion::server::lan_peer`) and only paired devices on.
pub mod firewall {
    use std::sync::Mutex;

    /// The rule's name in Windows' firewall.
    pub const RULE: &str = "Omsi-Hub Phone & tablet";

    /// How Windows stands to the phones (`state()`'s `firewall`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Gate {
        /// Not looked at yet (or not Windows).
        Unknown,
        Checking,
        /// The phones get in.
        Open,
        /// They do not; the button sets it right.
        Shut,
        /// Windows asks the player whether Omsi-Hub may.
        Asking,
        /// The player said no to Windows' question.
        Declined,
    }

    impl Gate {
        pub fn word(self) -> Option<&'static str> {
            Some(match self {
                Gate::Unknown => return None,
                Gate::Checking => "checking",
                Gate::Open => "open",
                Gate::Shut => "shut",
                Gate::Asking => "asking",
                Gate::Declined => "declined",
            })
        }
    }

    static GATE: Mutex<Gate> = Mutex::new(Gate::Unknown);

    pub fn gate() -> Gate {
        *GATE.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set(g: Gate) {
        *GATE.lock().unwrap_or_else(|e| e.into_inner()) = g;
    }

    /// A text in a PowerShell script, quoted.
    fn quoted(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    /// The program's rules, whatever language Windows speaks: how many firewall profiles are on,
    /// how many inbound rules block the program, and the ports the add-on's own rule lets in -
    /// `on=3 block=0 ours=47811`.
    pub fn check_script(exe: &str) -> String {
        let (exe, rule) = (quoted(exe), quoted(RULE));
        [
            "$ErrorActionPreference='SilentlyContinue'".to_string(),
            format!("$p=[IO.Path]::GetFullPath({exe})"),
            "$on=@(Get-NetFirewallProfile | Where-Object { $_.Enabled -eq 'True' }).Count".to_string(),
            "$f=@(Get-NetFirewallApplicationFilter | Where-Object { $_.Program -and ([Environment]::ExpandEnvironmentVariables($_.Program) -ieq $p) })".to_string(),
            "$block=@($f | Get-NetFirewallRule | Where-Object { $_.Direction -eq 'Inbound' -and $_.Enabled -eq 'True' -and $_.Action -eq 'Block' }).Count".to_string(),
            format!("$ours=@(Get-NetFirewallRule -DisplayName {rule} | Where-Object {{ $_.Enabled -eq 'True' -and $_.Action -eq 'Allow' -and $_.Direction -eq 'Inbound' }} | Get-NetFirewallPortFilter | ForEach-Object {{ $_.LocalPort }})"),
            "\"on=$on block=$block ours=$($ours -join ',')\"".to_string(),
        ]
        .join("\n")
    }

    /// What the check said: whether the phones get in on `port` (None: it could not tell).
    pub fn open(out: &str, port: u16) -> Option<bool> {
        let line = out.lines().rev().find(|l| l.trim_start().starts_with("on="))?;
        let field = |k: &str| line.split_whitespace().find_map(|w| w.strip_prefix(k)).map(str::to_string);
        let on: u32 = field("on=")?.parse().ok()?;
        let block: u32 = field("block=")?.parse().ok()?;
        let ours = field("ours=").unwrap_or_default();
        let has_port = ours.split(',').any(|p| p.trim() == port.to_string() || p.trim().eq_ignore_ascii_case("any"));
        Some(on == 0 || (block == 0 && has_port))
    }

    /// Setting it right (run with the administrator's rights): the program's blocking rules
    /// removed, and the one rule letting the port in on every kind of network.
    pub fn fix_script(exe: &str, port: u16) -> String {
        let (exe, rule) = (quoted(exe), quoted(RULE));
        let about = quoted("Lets phones and tablets in the home network reach Omsi-Hub (Settings > General > Phone & tablet).");
        [
            "$ErrorActionPreference='Stop'".to_string(),
            format!("$p=[IO.Path]::GetFullPath({exe})"),
            "Get-NetFirewallApplicationFilter | Where-Object { $_.Program -and ([Environment]::ExpandEnvironmentVariables($_.Program) -ieq $p) } | Get-NetFirewallRule | Where-Object { $_.Direction -eq 'Inbound' -and $_.Action -eq 'Block' } | Remove-NetFirewallRule".to_string(),
            format!("Get-NetFirewallRule -DisplayName {rule} -ErrorAction SilentlyContinue | Remove-NetFirewallRule"),
            format!("New-NetFirewallRule -DisplayName {rule} -Description {about} -Direction Inbound -Action Allow -Program $p -Protocol TCP -LocalPort {port} -Profile Any | Out-Null"),
        ]
        .join("\n")
    }

    /// A script as PowerShell's -EncodedCommand takes it: UTF-16LE in base64.
    pub fn encoded(script: &str) -> String {
        const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
        for c in bytes.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            for i in 0..4 {
                out.push(if i <= c.len() { B64[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
            }
        }
        out
    }

    #[cfg(windows)]
    fn powershell(args: &[&str]) -> std::io::Result<std::process::Output> {
        use std::os::windows::process::CommandExt;
        // (no console window flashing up)
        std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass"]).args(args).creation_flags(0x0800_0000).output()
    }

    /// Whether the phones get in on `port` (None: not Windows, or it could not tell).
    pub fn check(exe: &str, port: u16) -> Option<bool> {
        #[cfg(windows)]
        {
            let out = powershell(&["-EncodedCommand", &encoded(&check_script(exe))]).ok()?;
            open(&String::from_utf8_lossy(&out.stdout), port)
        }
        #[cfg(not(windows))]
        {
            let _ = (exe, port);
            None
        }
    }

    /// Set it right, asking Windows for the rights (UAC): Err(true) when the player said no,
    /// Err(false) when it failed.
    pub fn fix(exe: &str, port: u16) -> Result<(), bool> {
        #[cfg(windows)]
        {
            // (an elevated PowerShell of its own, waited for; 1223 is Windows' "cancelled")
            let inner = encoded(&fix_script(exe, port));
            let outer = format!(
                "try {{ $p = Start-Process powershell.exe -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ArgumentList '-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-EncodedCommand','{inner}'; exit $p.ExitCode }} catch {{ exit 1223 }}"
            );
            match powershell(&["-Command", &outer]).map(|o| o.status.code()) {
                Ok(Some(0)) => Ok(()),
                Ok(Some(1223)) => Err(true),
                _ => Err(false),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (exe, port);
            Err(false)
        }
    }

    /// Look (and with `fix_it`, set it right when it is shut), on a thread of its own: the check
    /// takes a few seconds, Windows' question as long as the player takes.
    pub fn watch(port: u16, fix_it: bool) {
        if !cfg!(windows) {
            return;
        }
        let Ok(exe) = std::env::current_exe() else { return };
        let exe = exe.to_string_lossy().to_string();
        set(Gate::Checking);
        let _ = std::thread::Builder::new().name("companion firewall".into()).spawn(move || {
            let open = check(&exe, port);
            if open != Some(false) {
                set(if open == Some(true) { Gate::Open } else { Gate::Unknown });
                return;
            }
            if !fix_it {
                set(Gate::Shut);
                return;
            }
            set(Gate::Asking);
            let done = fix(&exe, port);
            hub_core::log_line(match done {
                Ok(()) => "companion: the firewall set to let the phones in",
                Err(true) => "companion: the firewall left as it was (declined)",
                Err(false) => "companion: the firewall could not be set",
            });
            set(match done {
                Ok(()) if check(&exe, port) != Some(false) => Gate::Open,
                Err(true) => Gate::Declined,
                _ => Gate::Shut,
            });
        });
    }
}

/// The server while it runs, and the last error starting it.
static RUNNING: Mutex<Option<Companion>> = Mutex::new(None);
static FAILED: Mutex<String> = Mutex::new(String::new());
/// The assignment the plugin was last given (`addon::give`).
static ASSIGNED: Mutex<Option<Assignment>> = Mutex::new(None);

/// The assignment just given to the plugin: the phone's duty card.
pub fn assigned(a: &Assignment) {
    *ASSIGNED.lock().unwrap_or_else(|e| e.into_inner()) = Some(a.clone());
}

/// What the server needs of Omsi-Hub.
struct Hub;

/// The map as the launcher names it (`maps/<name>/global.cfg`), from the game's path of it.
fn map_name(game: &str) -> String {
    let m = game.replace('\\', "/");
    match m.to_lowercase().rfind("maps/") {
        Some(i) => m[i..].to_string(),
        None => m,
    }
}

fn global_of(map: &str) -> Option<PathBuf> {
    hub_core::paths::resolve_content(&map_name(map)).filter(|p| p.is_file())
}

impl Source for Hub {
    fn plugin_dir(&self) -> Option<PathBuf> {
        crate::addon::game_plugin_dir().filter(|d| d.is_dir())
    }

    fn assignment(&self) -> Option<Assignment> {
        ASSIGNED.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn route(&self, map: &str, date: &str, trip: &str) -> Option<Vec<[f64; 2]>> {
        let global = global_of(map)?;
        Some(crate::content::mapview::route_line(&map_name(map), &global, date, trip)).filter(|r| r.len() >= 2)
    }

    fn roads(&self, map: &str, date: &str, at: [f64; 2], r: f64) -> Vec<(f64, Vec<[f64; 2]>)> {
        global_of(map).map(|g| crate::content::mapview::roads_near(&map_name(map), &g, date, at, r)).unwrap_or_default()
    }

    fn world_of(&self, map: &str, tile: [i64; 2], local: [f64; 2]) -> Option<[f64; 2]> {
        let global = global_of(map)?;
        Some(crate::content::mapview::world_of(&map_name(map), &global, "", tile, local))
    }

    fn bus_model(&self, bus_file: &str) -> Option<(PathBuf, PathBuf)> {
        let bus = hub_core::paths::resolve_content(bus_file).filter(|p| p.is_file())?;
        let cfg = hub_companion::devices::model_of_bus(&bus)?;
        // (the game's folder the bus is read in: OMSI 2's own when it lies there - its Fonts and
        // Texture are there - else the content folder it came from)
        let omsi = hub_core::paths::root().filter(|r| bus.starts_with(r)).or_else(|| hub_core::paths::bases().into_iter().find(|b| bus.starts_with(b)))?;
        Some((cfg, omsi))
    }
}

/// This computer's name, as the app shows it.
fn computer_name() -> String {
    let env = ["COMPUTERNAME", "HOSTNAME"].iter().find_map(|k| std::env::var(k).ok());
    let file = || std::fs::read_to_string("/etc/hostname").ok();
    env.or_else(file).map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| "Omsi-Hub".into())
}

fn setting_on(s: &Settings) -> bool {
    s.other.get(ON).and_then(Value::as_bool).unwrap_or(false)
}

fn setting_port(s: &Settings) -> u16 {
    s.other.get(PORT).and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok()).filter(|p| *p >= 1024).unwrap_or(hub_companion::DEFAULT_PORT)
}

/// Start the server (`port`), stopping one that runs.
fn start(port: u16) -> Result<(), String> {
    let mut running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    // (the old one lets its ports go first)
    *running = None;
    let cfg =
        Config { port, udp_port: hub_companion::UDP_PORT, name: computer_name(), version: env!("CARGO_PKG_VERSION").into(), devices_file: hub_core::data_dir().join(hub_companion::DEVICES_FILE) };
    match Companion::start(cfg, Arc::new(Hub)) {
        Ok(c) => {
            hub_core::log_line(&format!("companion: listening on port {}", c.port()));
            *running = Some(c);
            FAILED.lock().unwrap_or_else(|e| e.into_inner()).clear();
            Ok(())
        }
        Err(e) => {
            let text = format!("The port {port} is taken or not allowed: {e}");
            hub_core::log_line(&format!("companion: {text}"));
            *FAILED.lock().unwrap_or_else(|e| e.into_inner()) = text.clone();
            Err(text)
        }
    }
}

/// With the app: the server started when the player has it on.
pub fn start_if_on() {
    let s = Settings::load();
    if setting_on(&s) {
        let port = setting_port(&s);
        if start(port).is_ok() {
            // (looked at; set right only when the player turns it on or asks - Windows' question
            // should not come out of the blue as the app starts)
            firewall::watch(port, false);
        }
    }
}

fn state() -> Value {
    let s = Settings::load();
    let running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    let Some(c) = running.as_ref() else {
        return json!({ "on": false, "wanted": setting_on(&s), "port": setting_port(&s), "error": FAILED.lock().unwrap_or_else(|e| e.into_inner()).clone() });
    };
    let mut v = c.status();
    let url = v["pair_url"].as_str().unwrap_or("").to_string();
    v["qr"] = hub_core::qr::encode(&url, hub_core::qr::Ecc::M).or_else(|| hub_core::qr::encode(&url, hub_core::qr::Ecc::L)).map(|q| json!({ "size": q.size, "bits": q.bits() })).unwrap_or(Value::Null);
    v["wanted"] = json!(true);
    v["firewall"] = json!(firewall::gate().word());
    v
}

pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    match cmd {
        "hub_companion_state" => Ok(state()),
        "hub_companion_set" => {
            let on = arg::<bool>(args, "on")?;
            let mut s = Settings::load();
            let port = arg::<Option<u16>>(args, "port")?.filter(|p| *p >= 1024).unwrap_or_else(|| setting_port(&s));
            s.other.insert(ON.into(), json!(on));
            s.other.insert(PORT.into(), json!(port));
            s.save().map_err(|e| e.to_string())?;
            if on {
                // (a port that will not open says so in the state)
                if start(port).is_ok() {
                    firewall::watch(port, true);
                }
            } else {
                *RUNNING.lock().unwrap_or_else(|e| e.into_inner()) = None;
            }
            Ok(state())
        }
        "hub_companion_code" => {
            if let Some(c) = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                c.new_code();
            }
            Ok(state())
        }
        "hub_companion_firewall" => {
            let port = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(Companion::port);
            if let Some(port) = port {
                firewall::watch(port, true);
            }
            Ok(state())
        }
        "hub_companion_forget" => {
            let id = arg::<String>(args, "id")?;
            if let Some(c) = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                c.forget(&id);
            }
            Ok(state())
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_games_map_path_is_the_launchers_name_of_it() {
        assert_eq!(map_name("C:\\OMSI 2\\maps\\Grundorf\\global.cfg"), "maps/Grundorf/global.cfg");
        assert_eq!(map_name("maps/Grundorf/global.cfg"), "maps/Grundorf/global.cfg");
        assert_eq!(map_name("/home/luc/.steam/steamapps/common/OMSI 2/Maps/Ruhrau/global.cfg"), "Maps/Ruhrau/global.cfg");
    }

    #[test]
    fn the_firewall_is_read_whatever_windows_says_and_set_right_for_the_port() {
        use firewall::*;
        assert_eq!(open("on=3 block=2 ours=", 47811), Some(false));
        assert_eq!(open("on=3 block=0 ours=", 47811), Some(false));
        assert_eq!(open("on=3 block=0 ours=47811", 47811), Some(true));
        assert_eq!(open("on=3 block=0 ours=50000", 47811), Some(false));
        assert_eq!(open("on=3 block=1 ours=47811", 47811), Some(false));
        // the firewall switched off: nothing to set right
        assert_eq!(open("on=0 block=2 ours=", 47811), Some(true));
        assert_eq!(open("something else", 47811), None);
        let exe = r"C:\Users\O'Brien\AppData\Local\Omsi-Hub\omsi-hub.exe";
        let fix = fix_script(exe, 47811);
        assert!(fix.contains(r"'C:\Users\O''Brien\AppData\Local\Omsi-Hub\omsi-hub.exe'"), "{fix}");
        assert!(fix.contains("-LocalPort 47811 -Profile Any") && fix.contains("Remove-NetFirewallRule"));
        assert!(check_script(exe).contains("Get-NetFirewallApplicationFilter"));
        // -EncodedCommand's base64 of UTF-16LE
        assert_eq!(encoded("a"), "YQA=");
        assert_eq!(encoded("Hi!"), "SABpACEA");
    }

    #[test]
    fn off_by_default_and_a_port_below_1024_is_not_taken() {
        let s = Settings::default();
        assert!(!setting_on(&s));
        assert_eq!(setting_port(&s), hub_companion::DEFAULT_PORT);
        let mut s = Settings::default();
        s.other.insert(PORT.into(), json!(80));
        assert_eq!(setting_port(&s), hub_companion::DEFAULT_PORT);
        s.other.insert(PORT.into(), json!(50000));
        assert_eq!(setting_port(&s), 50000);
    }
}
