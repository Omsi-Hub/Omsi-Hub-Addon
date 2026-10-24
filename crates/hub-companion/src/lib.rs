//! The phone and tablet companion: the server on the home network that the Omsi-Hub Companion
//! app (iPhone, iPad, Android) pairs with, what Omsi-Hub's plugin tells it of the game, and
//! the commands the app sends back into the game - docs/COMPANION.md has the protocol.
//!
//! ```text
//!  openOMSI ── plugin ──UDP 127.0.0.1:47812──▶ this crate ◀──HTTP :47811── the app
//!                 ▲                               │
//!                 └──── data/companion.json ──────┘
//! ```
//!
//! * The plugin (`plugin/companion.lua`) reads the game twice a second and sends it as small
//!   JSON datagrams ([`heard`]); it carries out the commands of `data/companion.json`
//!   ([`queue`]) and acks each.
//! * The server ([`server`], [`http`], [`pairing`]): off by default, on the home network only,
//!   behind a pairing code; a long-polled state, commands answered when the game has done them,
//!   the trip's route and the map's roads for the app's map; where the bus is thirty times a
//!   second besides ([`pos`], the plugin's `data/pos.json`), for the app's map to follow it.
//! * The state ([`state`]) adds what only the add-on knows - the duty it assigned, the
//!   driver's pass, the control room's inbox - and works out the break between trips and the
//!   next turn ([`nav`]) on the route the add-on reads from the map.
//!
//! What the add-on knows comes through a [`Source`] (hub-api's `companion`, which reads the
//! map with openOMSI's readers); this crate depends on nothing of that, and its tests run on a
//! fake one.

pub mod devices;
pub mod heard;
pub mod http;
pub mod nav;
pub mod pairing;
pub mod pos;
pub mod queue;
pub mod server;
pub mod state;
pub mod web;

use hub_game::bridge::Assignment;
use serde_json::{json, Value};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;

/// The protocol's version (`/api/hello`'s `api`).
pub const API: u32 = 1;
/// The port the app finds the server on, and the one the plugin's datagrams come to.
pub const DEFAULT_PORT: u16 = 47811;
pub const UDP_PORT: u16 = 47812;
/// The paired devices' file, in the data folder.
pub const DEVICES_FILE: &str = "companion-devices.json";

/// What the add-on knows that the server needs. Map points are the add-on's own map metres
/// (a tile's place in `global.cfg`'s grid and the metres in it); the server turns them into
/// the game's with the place the plugin gives along with its tile.
pub trait Source: Send + Sync {
    /// The plugin's folder in the player's openOMSI (where `data/` is).
    fn plugin_dir(&self) -> Option<PathBuf>;
    /// The assignment the plugin was last given.
    fn assignment(&self) -> Option<Assignment>;
    /// The company's currency sign ("" for the default, "€").
    fn currency(&self) -> String {
        String::new()
    }
    /// The route the timetable's trip `trip` of map `map` (`maps/<name>/global.cfg`) drives on
    /// `date` (YYYY-MM-DD, "" for any), as a line through the lanes.
    fn route(&self, map: &str, date: &str, trip: &str) -> Option<Vec<[f64; 2]>>;
    /// The map's roads within `r` metres of `at`: their width and their line.
    fn roads(&self, map: &str, date: &str, at: [f64; 2], r: f64) -> Vec<(f64, Vec<[f64; 2]>)>;
    /// The map metres of a place given by its tile (x, y of the grid) and the metres in it.
    fn world_of(&self, map: &str, tile: [i64; 2], local: [f64; 2]) -> Option<[f64; 2]>;
    /// The bus the plugin names (its `.bus` file, relative to the game's folder): its
    /// `model.cfg`, and the game's folder it is read in (OMSI's fonts and shared textures are
    /// there). `None`: its devices are not shown.
    fn bus_model(&self, _bus_file: &str) -> Option<(PathBuf, PathBuf)> {
        None
    }
}

/// How the server runs.
#[derive(Debug, Clone)]
pub struct Config {
    /// The port the devices reach it on (0: any free one, for the tests), and the plugin's.
    pub port: u16,
    pub udp_port: u16,
    /// This computer's name, shown in the app; the add-on's version.
    pub name: String,
    pub version: String,
    /// Where the paired devices are kept.
    pub devices_file: PathBuf,
}

/// The companion running; dropping it stops it.
pub struct Companion {
    server: server::Server,
}

impl Companion {
    /// Start listening (the devices on every network address, the plugin on this computer's).
    pub fn start(cfg: Config, source: Arc<dyn Source>) -> std::io::Result<Companion> {
        let listen = SocketAddr::from((Ipv4Addr::UNSPECIFIED, cfg.port));
        let udp = SocketAddr::from((Ipv4Addr::LOCALHOST, cfg.udp_port));
        let sid = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1);
        let shared = Arc::new(server::Shared::new(cfg, source, random_code(), sid));
        Ok(Companion { server: server::Server::start(listen, udp, shared)? })
    }

    /// The port it listens on.
    pub fn port(&self) -> u16 {
        self.server.addr.port()
    }

    /// What the launcher's settings show: where a phone finds it, the pairing code and the
    /// address with it (for the QR code), the paired devices and whether the game is heard.
    pub fn status(&self) -> Value {
        let sh = &self.server.shared;
        let port = self.port();
        let host = lan_address().map(|a| a.to_string()).unwrap_or_else(|| "127.0.0.1".into());
        let now = server::unix_now();
        let i = sh.lock();
        let code = i.pairing.code().to_string();
        let devices: Vec<Value> = i
            .pairing
            .devices
            .iter()
            .map(|d| {
                let seen = i.seen.get(&d.hash).copied().unwrap_or(d.seen);
                json!({ "id": d.hash, "name": d.name, "since": d.since, "seen": seen, "connected": now.saturating_sub(seen) <= server::SEEN_FOR && i.seen.contains_key(&d.hash) })
            })
            .collect();
        let game = i.heard.fresh(sh.now(), state::GAME_FRESH);
        json!({
            "on": true,
            "host": host, "port": port,
            "address": format!("http://{host}:{port}"),
            "code": code,
            "pair_url": pair_url(&host, port, &code),
            "web": web::present(),
            "name": sh.cfg.name,
            "devices": devices,
            "game": game,
        })
    }

    /// A paired device removed (by the id `status` gives): whether it was paired.
    pub fn forget(&self, id: &str) -> bool {
        let sh = &self.server.shared;
        let mut i = sh.lock();
        let gone = i.pairing.forget(id);
        if gone {
            i.seen.remove(id);
            let _ = std::fs::write(&sh.cfg.devices_file, pairing::devices_json(&i.pairing.devices));
        }
        gone
    }

    /// Another pairing code (the one on show was seen by someone else).
    pub fn new_code(&self) {
        self.server.shared.lock().pairing.set_code(random_code());
    }
}

/// The address the QR code holds: the web app's pairing page (`web`). The phone's camera opens
/// it in the browser, which pairs at once; the Companion app's own scanner reads it too.
pub fn pair_url(host: &str, port: u16, code: &str) -> String {
    let h = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    format!("http://{h}:{port}/pair?code={code}")
}

/// This computer's address on the home network: the one its traffic to the internet would
/// leave by (nothing is sent), when that is a home network's.
pub fn lan_address() -> Option<std::net::IpAddr> {
    let s = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    s.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback() && server::lan_peer(ip)).then_some(ip)
}

fn random_bytes<const N: usize>() -> [u8; N] {
    use ring::rand::SecureRandom;
    let mut b = [0u8; N];
    // (the system's random source; should it ever fail, the time is better than nothing)
    if ring::rand::SystemRandom::new().fill(&mut b).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        for (i, x) in b.iter_mut().enumerate() {
            *x = (t >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(97);
        }
    }
    b
}

/// A pairing code: six digits.
pub fn random_code() -> String {
    let b = random_bytes::<4>();
    format!("{:06}", u32::from_le_bytes(b) % 1_000_000)
}

/// A device key: 128 random bits as hex.
pub fn random_key() -> String {
    random_bytes::<16>().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Nothing;

    impl Source for Nothing {
        fn plugin_dir(&self) -> Option<PathBuf> {
            None
        }
        fn assignment(&self) -> Option<Assignment> {
            None
        }
        fn route(&self, _: &str, _: &str, _: &str) -> Option<Vec<[f64; 2]>> {
            None
        }
        fn roads(&self, _: &str, _: &str, _: [f64; 2], _: f64) -> Vec<(f64, Vec<[f64; 2]>)> {
            Vec::new()
        }
        fn world_of(&self, _: &str, _: [i64; 2], _: [f64; 2]) -> Option<[f64; 2]> {
            None
        }
    }

    #[test]
    fn codes_and_keys_are_random_and_the_pairing_address_is_escaped() {
        let (a, b) = (random_key(), random_key());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(random_code().len() == 6 && random_code().bytes().all(|c| c.is_ascii_digit()));
        assert_eq!(pair_url("192.168.1.20", 47811, "482913"), "http://192.168.1.20:47811/pair?code=482913");
        assert_eq!(pair_url("fd00::20", 47811, "482913"), "http://[fd00::20]:47811/pair?code=482913");
    }

    #[test]
    fn the_server_answers_on_the_network_and_hears_the_plugin() {
        use std::io::{Read, Write};
        let dir = std::env::temp_dir().join(format!("hub-companion-start-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // (the plugin's port: a free one)
        let udp = UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let cfg = Config { port: 0, udp_port: udp, name: "PC".into(), version: "0.1.0".into(), devices_file: dir.join(DEVICES_FILE) };
        let c = Companion::start(cfg, Arc::new(Nothing)).unwrap();
        let ask = |path: &str| {
            let mut s = std::net::TcpStream::connect(("127.0.0.1", c.port())).unwrap();
            s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).unwrap();
            let mut out = String::new();
            s.read_to_string(&mut out).unwrap();
            out
        };
        assert!(ask("/api/hello").contains("\"app\":\"omsi-hub\""));
        assert!(ask("/api/state").starts_with("HTTP/1.1 401"));
        // the web app, its page without a key, its largest file whole
        assert!(ask("/").starts_with(if web::present() { "HTTP/1.1 200" } else { "HTTP/1.1 503" }));
        if web::present() {
            let mut s = std::net::TcpStream::connect(("127.0.0.1", c.port())).unwrap();
            s.write_all(b"GET /canvaskit.wasm HTTP/1.1\r\nHost: x\r\nAccept-Encoding: gzip, deflate\r\n\r\n").unwrap();
            let mut out = Vec::new();
            s.read_to_end(&mut out).unwrap();
            let at = out.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            let head = String::from_utf8_lossy(&out[..at]).to_string();
            assert!(head.starts_with("HTTP/1.1 200") && head.contains("Content-Encoding: gzip\r\n") && head.contains("application/wasm"));
            assert!(head.contains(&format!("Content-Length: {}\r\n", out.len() - at)));
        }
        let st = c.status();
        assert_eq!((st["port"].as_u64(), st["code"].as_str().map(str::len)), (Some(c.port() as u64), Some(6)));
        // a datagram from the plugin
        let out = UdpSocket::bind("127.0.0.1:0").unwrap();
        out.send_to(br#"{"t":"live","clock":1.0}"#, ("127.0.0.1", udp)).unwrap();
        for _ in 0..100 {
            if c.status()["game"] == true {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(c.status()["game"], true);
        let old = c.status()["code"].clone();
        c.new_code();
        assert_eq!(c.status()["code"].as_str().map(str::len), Some(6));
        let _ = old;
        assert!(!c.forget("nobody"));
        drop(c);
        let _ = std::fs::remove_dir_all(dir);
    }
}
