//! Omsi-Hub's cloud saves, the app's side (docs/ONLINE.md, "Cloud saves"; the client is
//! `hub_online::saves`). Ported from the old `omsihub/cloud.rs`, with the old
//! `company/cloud.rs` archive beside it ([`archive`]).
//!
//! A company is a **local save** unless the player puts it in the cloud: then it is linked to
//! one of the account's 2 slots, and its sync metadata (`Sync`: the slot, the version last
//! synced, its sha256, "cloud" or "local") is kept in the app's own data folder
//! (`omsihub-cloud.json`), not in the company's file. A company without metadata - every company
//! of before - is local. Local companies are unlimited and play as before, company multiplayer
//! included; without an account, or offline, everything is local and nothing here blocks or says
//! more than "offline".
//!
//! * **Opening** a cloud company asks `list` first (`cs_open`): the cloud has a newer version -
//!   download it?; both changed - which copy to keep (the cloud's, this PC's, or both: this PC's
//!   kept as a local copy)? A save is never replaced without the player's choice.
//! * **Uploading**: a changed cloud company goes up when its pages are left (`closed`, or
//!   `cs_sync`), and every ten minutes while it changes (the worker), on top of the version last
//!   synced - another computer's newer version is a conflict, never overwritten.
//! * **The limit**: "Put in the cloud" with both slots taken offers to keep the company local or
//!   to replace one of the two (`cs_put` with `replace`, after the page's confirm).
//!
//! The commands: `cs_state` (the chooser's badges, the account panel's slots: no network),
//! `cs_companies` (the driver's companies, to choose one to put in the cloud),
//! `cs_open {id}`, `cs_resolve {id, keep: cloud|local|both}`, `cs_put {id, replace?}`,
//! `cs_local {id}`, `cs_download {slot}`, `cs_remove {slot}`, `cs_sync {id}`.
//!
//! **The company's files** are read here as the company port keeps them, in the data folder
//! (`files`): `companies/<id>.json` and beside it `<id>.orders.jsonl`, `<id>.live.jsonl`,
//! `<id>.live-<date>.taken` and the game's plans `<id>.liveplan.json` and
//! `<id>.drive.liveplan.json`. Only the fields this needs are read from a company (`id`, `name`,
//! `short`, `profile`, `map`, `map_name`); the rest of its JSON passes through as it is. Once
//! `hub_core::company::store` is ported, `files` can call it instead.

use crate::online::{self, tr, trf};
use crate::{Answer, Platform};
use hub_online::saves::{self, sha256_hex};
use hub_online::{Cloud, Error, SaveError, Slot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Duration;

// --- the company's files ---------------------------------------------------------------------------

/// The company files of the data folder, as the company port's `store` keeps them.
pub mod files {
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    pub fn dir(data: &Path) -> PathBuf {
        data.join("companies")
    }

    pub fn path_of(data: &Path, id: &str) -> PathBuf {
        dir(data).join(format!("{id}.json"))
    }

    /// What the game reported since the day's last close.
    pub fn live_file(data: &Path, id: &str) -> PathBuf {
        dir(data).join(format!("{id}.live.jsonl"))
    }

    /// The phone's orders still waiting.
    pub fn orders_file(data: &Path, id: &str) -> PathBuf {
        dir(data).join(format!("{id}.orders.jsonl"))
    }

    /// The game's plans of the day (written from the company whenever it plans its day).
    pub fn plan_files(data: &Path, id: &str) -> [PathBuf; 2] {
        [dir(data).join(format!("{id}.liveplan.json")), dir(data).join(format!("{id}.drive.liveplan.json"))]
    }

    /// A company's id from its name (`Stadtbus Grundorf` → `stadtbus-grundorf`).
    pub fn slug(name: &str) -> String {
        let mut s = String::new();
        for c in name.trim().to_lowercase().chars() {
            if c.is_alphanumeric() {
                s.push(c);
            } else if !s.ends_with('-') && !s.is_empty() {
                s.push('-');
            }
        }
        let s = s.trim_end_matches('-').to_string();
        if s.is_empty() {
            "company".to_string()
        } else {
            s
        }
    }

    /// An id for a new company of that name that no file has yet.
    pub fn unused_id(data: &Path, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while path_of(data, &id).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    /// Company `id`'s JSON (None: it is not there, or not a company).
    pub fn load(data: &Path, id: &str) -> Option<Value> {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(path_of(data, id)).ok()?).ok()?;
        v.is_object().then_some(v)
    }

    /// A text field of a company.
    pub fn text(c: &Value, key: &str) -> String {
        c.get(key).and_then(Value::as_str).unwrap_or("").to_string()
    }

    /// Written whole, through a file beside it (`id` from the company's own field).
    pub fn save(data: &Path, c: &Value) -> std::io::Result<()> {
        let id = text(c, "id");
        let d = dir(data);
        std::fs::create_dir_all(&d)?;
        let tmp = d.join(format!("{id}.json.tmp"));
        std::fs::write(&tmp, serde_json::to_string_pretty(c).unwrap_or_default())?;
        std::fs::rename(&tmp, path_of(data, &id))
    }

    /// The companies of a driver (all of them for an empty name), by name: (id, company).
    pub fn list(data: &Path, profile: &str) -> Vec<(String, Value)> {
        let Ok(rd) = std::fs::read_dir(dir(data)) else { return Vec::new() };
        let mut out: Vec<(String, Value)> = rd
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                // (`<id>.json` only: not `<id>.liveplan.json` and the like)
                let id = n.strip_suffix(".json").filter(|id| !id.contains('.'))?.to_string();
                load(data, &id).map(|c| (id, c))
            })
            .filter(|(_, c)| profile.trim().is_empty() || text(c, "profile").trim().eq_ignore_ascii_case(profile.trim()))
            .collect();
        out.sort_by_key(|(id, c)| (text(c, "name").to_lowercase(), id.clone()));
        out
    }
}

/// A company as one cloud save (the old `company/cloud.rs`): what another computer needs to go on
/// with it, packed into one small archive that the same files always pack the same (its sha256
/// tells a change).
///
/// What goes in, under names of its own (the company's id is not in them, so a save can come back
/// as another company - "keep both"):
///
/// * `company.json` - the company's file, as it is on the disk;
/// * `orders.jsonl` - the phone's orders still waiting;
/// * `live.jsonl` and `live-<date>.taken` - what the game reported since the day's last close:
///   the next close books it, on whichever computer it runs.
///
/// Not the game's plans of the day: the app writes them from the company whenever it plans its
/// day, and a stale one is taken away when a save comes in (`install`).
///
/// The archive: `MAGIC`, then each file in its name's order as the name's length (u16, little
/// endian), the name, the length (u64) and the bytes.
pub mod archive {
    use super::files;
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    pub const MAGIC: &[u8] = b"OMSIHUB-SAVE 1\n";
    /// The company's own file in the archive.
    pub const COMPANY: &str = "company.json";
    const ORDERS: &str = "orders.jsonl";
    const LIVE: &str = "live.jsonl";
    /// At most this many files (a company has four kinds; taken files of a day or two).
    const MAX_FILES: usize = 64;

    /// The files of company `id` that go with it: their names in the archive and where they are.
    fn members(data: &Path, id: &str) -> Vec<(String, PathBuf)> {
        let mut out = vec![(COMPANY.to_string(), files::path_of(data, id)), (ORDERS.into(), files::orders_file(data, id)), (LIVE.into(), files::live_file(data, id))];
        if let Ok(rd) = std::fs::read_dir(files::dir(data)) {
            let head = format!("{id}.live-");
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if let Some(date) = name.strip_prefix(head.as_str()).and_then(|r| r.strip_suffix(".taken")) {
                    out.push((format!("live-{date}.taken"), e.path()));
                }
            }
        }
        out
    }

    /// A name the archive may hold (nothing that could reach outside the companies' folder).
    pub(super) fn allowed(name: &str) -> bool {
        match name {
            COMPANY | ORDERS | LIVE => true,
            _ => name.strip_prefix("live-").and_then(|r| r.strip_suffix(".taken")).is_some_and(|d| !d.is_empty() && d.len() <= 10 && d.bytes().all(|b| b.is_ascii_digit() || b == b'-')),
        }
    }

    /// Where a file of the archive goes for company `id`.
    fn target(data: &Path, id: &str, name: &str) -> PathBuf {
        match name {
            COMPANY => files::path_of(data, id),
            ORDERS => files::orders_file(data, id),
            LIVE => files::live_file(data, id),
            // (`live-<date>.taken`)
            _ => files::dir(data).join(format!("{id}.{name}")),
        }
    }

    /// Company `id` packed: its file and what lies beside it that it needs.
    pub fn pack(data: &Path, id: &str) -> Result<Vec<u8>, String> {
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        for (name, path) in members(data, id) {
            match std::fs::read(&path) {
                Ok(bytes) => out.push((name, bytes)),
                Err(e) if name == COMPANY => return Err(format!("cannot read {}: {e}", path.display())),
                Err(_) => {}
            }
        }
        Ok(pack_files(&mut out))
    }

    pub(super) fn pack_files(files: &mut [(String, Vec<u8>)]) -> Vec<u8> {
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let mut out = MAGIC.to_vec();
        for (name, bytes) in files.iter() {
            out.extend((name.len() as u16).to_le_bytes());
            out.extend(name.as_bytes());
            out.extend((bytes.len() as u64).to_le_bytes());
            out.extend(bytes);
        }
        out
    }

    /// An archive read and checked: the company and every file as it was packed.
    #[derive(Debug, Clone)]
    pub struct Unpacked {
        pub company: Value,
        pub files: Vec<(String, Vec<u8>)>,
    }

    pub fn unpack(archive: &[u8]) -> Result<Unpacked, String> {
        let mut rest = archive.strip_prefix(MAGIC).ok_or("not a company's cloud save")?;
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        while !rest.is_empty() {
            if files.len() >= MAX_FILES {
                return Err("too many files in the save".into());
            }
            let take = |rest: &mut &[u8], n: usize| -> Result<Vec<u8>, String> {
                if rest.len() < n {
                    return Err("the save is cut short".into());
                }
                let (a, b) = rest.split_at(n);
                *rest = b;
                Ok(a.to_vec())
            };
            let n = u16::from_le_bytes(take(&mut rest, 2)?.try_into().unwrap_or_default()) as usize;
            let name = String::from_utf8(take(&mut rest, n)?).map_err(|_| "a file's name is not text")?;
            let len = u64::from_le_bytes(take(&mut rest, 8)?.try_into().unwrap_or_default());
            let bytes = take(&mut rest, usize::try_from(len).unwrap_or(usize::MAX))?;
            if !allowed(&name) || files.iter().any(|f| f.0 == name) {
                return Err(format!("the save holds a file it may not: {name:?}"));
            }
            files.push((name, bytes));
        }
        let bytes = files.iter().find(|f| f.0 == COMPANY).map(|f| f.1.clone()).ok_or("the save holds no company")?;
        let company: Value = serde_json::from_slice(&bytes).map_err(|_| "the save's company cannot be read")?;
        if !company.is_object() {
            return Err("the save's company cannot be read".into());
        }
        Ok(Unpacked { company, files })
    }

    /// A save put on the disk as company `id` (a new id: "keep both"), for the driver `profile`
    /// (empty: the save's own), renamed when `name` is given. What lay beside an older company
    /// `id` goes first - the save's files replace it all - and the company's file is written
    /// last, whole: a save that cannot be read changes nothing.
    pub fn install(data: &Path, id: &str, archive: &[u8], profile: &str, name: Option<&str>) -> Result<Value, String> {
        let u = unpack(archive)?;
        let mut c = u.company;
        c["id"] = Value::from(id);
        if !profile.trim().is_empty() {
            c["profile"] = Value::from(profile.trim());
        }
        if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
            c["name"] = Value::from(n.trim());
        }
        std::fs::create_dir_all(files::dir(data)).map_err(|e| e.to_string())?;
        for (n, path) in members(data, id) {
            if n != COMPANY {
                let _ = std::fs::remove_file(path);
            }
        }
        for p in files::plan_files(data, id) {
            let _ = std::fs::remove_file(p);
        }
        for (n, bytes) in u.files.iter().filter(|f| f.0 != COMPANY) {
            let path = target(data, id, n);
            std::fs::write(&path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        }
        files::save(data, &c).map_err(|e| e.to_string())?;
        Ok(c)
    }
}

// --- the sync metadata ----------------------------------------------------------------------------

/// Where a company is kept.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    /// On this computer only (every company of before).
    #[default]
    Local,
    Cloud,
}

/// A company's sync metadata. Every field has its default: an older entry, or none, is a
/// local company.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(default)]
pub struct Sync {
    pub place: Place,
    /// The account it is in the cloud of (the user id): signed in as another, it is local here.
    pub user: String,
    /// Its cloud slot (1 or 2).
    pub slot: u8,
    /// The cloud's version last synced (uploaded or downloaded).
    pub version: u64,
    /// That version's sha256 in the cloud (compressed).
    pub sha256: String,
    /// The sha256 of the company as it was packed then (`archive::pack`): another one now is a
    /// change of this computer's.
    pub local_sha256: String,
    /// When it was last synced (Unix seconds).
    pub synced_at: u64,
}

impl Sync {
    fn linked(&self, user: &str) -> bool {
        self.place == Place::Cloud && (1..=saves::MAX_SLOTS).contains(&self.slot) && !user.is_empty() && self.user == user
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
struct Book {
    companies: BTreeMap<String, Sync>,
}

const FILE: &str = "omsihub-cloud.json";

fn book_path(data: &Path) -> PathBuf {
    data.join(FILE)
}

fn read_book(data: &Path) -> Book {
    std::fs::read_to_string(book_path(data)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// A company's metadata (the default - local - when it has none).
pub fn sync_of(data: &Path, id: &str) -> Sync {
    read_book(data).companies.remove(id).unwrap_or_default()
}

/// Written whole, through a file beside it.
fn write_sync(data: &Path, id: &str, s: Option<Sync>) {
    let mut b = read_book(data);
    match s {
        Some(s) => b.companies.insert(id.to_string(), s),
        None => b.companies.remove(id),
    };
    let path = book_path(data);
    let tmp = path.with_extension("json.tmp");
    if let Ok(text) = serde_json::to_string_pretty(&b) {
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The company packed, with its sha256 (None: it is not there).
fn packed(data: &Path, id: &str) -> Option<(Vec<u8>, String)> {
    let a = archive::pack(data, id).ok()?;
    let sha = sha256_hex(&a);
    Some((a, sha))
}

// --- the cloud, as the engine sees it ---------------------------------------------------------------

/// The account's cloud saves: the client signed in (`Online`), or the tests' own.
pub trait Remote {
    /// The player's user id (`not_signed_in` signed out, `offline` without a network).
    fn user(&mut self) -> Result<String, Error>;
    fn list(&mut self) -> Result<Cloud, Error>;
    /// The archive up into `slot` (None: the first free) on top of `base` (0: new).
    fn upload(&mut self, slot: Option<u8>, name: &str, base: u64, archive: &[u8]) -> Result<Slot, SaveError>;
    /// A slot's current version: its version, its sha256 and the archive.
    fn download(&mut self, slot: u8) -> Result<(u64, String, Vec<u8>), SaveError>;
    fn delete(&mut self, slot: u8) -> Result<(), Error>;
}

/// The client of `online.rs`: each step under its lock, the file's transfer without it
/// (the friends' calls go on meanwhile).
pub struct Online;

impl Remote for Online {
    fn user(&mut self) -> Result<String, Error> {
        online::with(|c| {
            if !c.signed_in() {
                return Err(Error::Code("not_signed_in"));
            }
            // (the cloud waits for the privacy statement agreed to: everything is local meanwhile)
            if c.privacy_needed() {
                return Err(Error::Code("privacy"));
            }
            c.access_token()?;
            c.user_id().map(str::to_string).ok_or(Error::Code("server"))
        })
    }

    fn list(&mut self) -> Result<Cloud, Error> {
        online::with(|c| c.saves_list())
    }

    fn upload(&mut self, slot: Option<u8>, name: &str, base: u64, archive: &[u8]) -> Result<Slot, SaveError> {
        let packed = saves::compress(archive)?;
        let device = saves::device_name();
        let (begun, transfer) = online::with(|c| c.saves_upload_begin(slot, name, &packed, base, &device).map(|b| (b, c.transfer())))?;
        transfer.put(&begun, &packed)?;
        online::with(|c| match c.saves_upload_commit(&begun) {
            Err(SaveError::Failed(Error::Offline)) => c.saves_upload_commit(&begun),
            r => r,
        })
    }

    fn download(&mut self, slot: u8) -> Result<(u64, String, Vec<u8>), SaveError> {
        let (fetch, transfer) = online::with(|c| c.saves_download(slot, None).map(|f| (f, c.transfer())))?;
        let bytes = transfer.get(&fetch)?;
        Ok((fetch.version, fetch.sha256, saves::decompress(&bytes)?))
    }

    fn delete(&mut self, slot: u8) -> Result<(), Error> {
        online::with(|c| c.saves_delete(slot)).map(|_| ())
    }
}

// --- what is decided --------------------------------------------------------------------------------

/// A cloud company held against its slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Not in this account's cloud.
    Local,
    Synced,
    /// Changed here only: to upload.
    Upload,
    /// Changed in the cloud only: to offer.
    Newer(Slot),
    /// Changed on both: to ask.
    Conflict(Slot),
    /// The slot is empty now (removed from the cloud on another computer).
    Gone,
    /// The slot holds another company now (replaced on another computer).
    Other(Slot),
}

/// What to do with company `name` whose metadata is `s`, packed now as `local_sha`, against
/// `cloud` (`user` the account signed in).
pub fn decide(s: &Sync, user: &str, name: &str, local_sha: &str, cloud: &Cloud) -> Decision {
    if !s.linked(user) {
        return Decision::Local;
    }
    let Some(slot) = cloud.slot(s.slot) else { return Decision::Gone };
    let changed_here = local_sha != s.local_sha256;
    let changed_there = slot.version != s.version || slot.sha256 != s.sha256;
    // (another company in the slot: its name is not ours and it is not the version we know)
    if changed_there && !slot.name.trim().eq_ignore_ascii_case(name.trim()) {
        return Decision::Other(slot.clone());
    }
    match (changed_here, changed_there) {
        (false, false) => Decision::Synced,
        (true, false) => Decision::Upload,
        (false, true) => Decision::Newer(slot.clone()),
        (true, true) => Decision::Conflict(slot.clone()),
    }
}

/// What came of putting a company in the cloud.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Put {
    Done(Slot),
    /// Both slots taken: keep it local, or replace one of these.
    Limit(Vec<Slot>),
}

/// The engine: the companies of `data` against a `Remote`.
pub struct Engine<'a> {
    pub data: &'a Path,
    pub remote: &'a mut dyn Remote,
}

impl Engine<'_> {
    fn company_name(&self, id: &str) -> Option<String> {
        files::load(self.data, id).map(|c| files::text(&c, "name"))
    }

    /// The metadata after a sync of `slot` with `local_sha` as the company is here.
    fn linked(&self, id: &str, user: &str, slot: &Slot, local_sha: String) {
        write_sync(
            self.data,
            id,
            Some(Sync { place: Place::Cloud, user: user.into(), slot: slot.slot, version: slot.version, sha256: slot.sha256.clone(), local_sha256: local_sha, synced_at: now() }),
        );
    }

    /// Company `id` made local (its files stay; its cloud copy stays until removed).
    pub fn unlink(&self, id: &str) {
        write_sync(self.data, id, None);
    }

    /// Company `id` held against the cloud (a company unlinked by what the cloud says now -
    /// emptied or replaced elsewhere - is made local here).
    pub fn check(&mut self, id: &str) -> Result<Decision, Error> {
        let s = sync_of(self.data, id);
        if s.place != Place::Cloud {
            return Ok(Decision::Local);
        }
        let user = self.remote.user()?;
        let Some(name) = self.company_name(id) else { return Ok(Decision::Local) };
        let Some((_, sha)) = packed(self.data, id) else { return Ok(Decision::Local) };
        let cloud = self.remote.list()?;
        let d = decide(&s, &user, &name, &sha, &cloud);
        if matches!(d, Decision::Gone | Decision::Other(_)) {
            self.unlink(id);
        }
        Ok(d)
    }

    /// A change of this computer's uploaded, on top of the version last synced (a newer one in
    /// the cloud is a conflict: nothing is overwritten). The decision it came to.
    pub fn sync(&mut self, id: &str) -> Result<Decision, Error> {
        let s = sync_of(self.data, id);
        if s.place != Place::Cloud {
            return Ok(Decision::Local);
        }
        let user = self.remote.user()?;
        if !s.linked(&user) {
            return Ok(Decision::Local);
        }
        let (Some(name), Some((a, sha))) = (self.company_name(id), packed(self.data, id)) else { return Ok(Decision::Local) };
        if sha == s.local_sha256 {
            return Ok(Decision::Synced);
        }
        match self.remote.upload(Some(s.slot), &name, s.version, &a) {
            Ok(slot) => {
                self.linked(id, &user, &slot, sha);
                Ok(Decision::Synced)
            }
            Err(SaveError::Conflict(now)) => Ok(if now.empty {
                self.unlink(id);
                Decision::Gone
            } else if !now.name.trim().eq_ignore_ascii_case(name.trim()) {
                self.unlink(id);
                Decision::Other(now)
            } else {
                Decision::Conflict(now)
            }),
            Err(SaveError::Failed(e)) => Err(e),
        }
    }

    /// "Put this company in the cloud": into the first free slot, or into `replace` - that slot
    /// emptied first, and the company of this computer it held made local.
    pub fn put(&mut self, id: &str, replace: Option<u8>) -> Result<Put, Error> {
        let user = self.remote.user()?;
        let s = sync_of(self.data, id);
        if s.linked(&user) {
            // (what the sync decided: a conflict is said, a company the cloud no longer has -
            // emptied, or another in its slot - is put in as a new one below)
            match self.sync(id)? {
                Decision::Conflict(_) => return Err(Error::Code("conflict")),
                Decision::Gone | Decision::Other(_) => {}
                _ => {
                    let cloud = self.remote.list()?;
                    return cloud.slot(s.slot).cloned().map(Put::Done).ok_or(Error::Code("no_cloud_save"));
                }
            }
        }
        let name = self.company_name(id).ok_or(Error::Code("server"))?;
        let (a, sha) = packed(self.data, id).ok_or(Error::Code("server"))?;
        if let Some(n) = replace {
            // (one too large for the cloud fails before the slot's save is deleted for it)
            saves::compress(&a)?;
            self.remote.delete(n)?;
            for (other, o) in read_book(self.data).companies {
                if o.linked(&user) && o.slot == n {
                    self.unlink(&other);
                }
            }
        }
        match self.remote.upload(replace, &name, 0, &a) {
            Ok(slot) => {
                self.linked(id, &user, &slot, sha);
                Ok(Put::Done(slot))
            }
            Err(SaveError::Failed(Error::Code("limit"))) => Ok(Put::Limit(self.remote.list()?.slots.into_iter().filter(|x| !x.empty).collect())),
            // (the slot filled meanwhile, from another computer)
            Err(SaveError::Conflict(_)) => Err(Error::Code("conflict")),
            Err(SaveError::Failed(e)) => Err(e),
        }
    }

    /// The cloud's version of slot `n` put on this computer as company `id` (`profile` the
    /// driver's), linked to the slot.
    fn fetch_into(&mut self, user: &str, n: u8, id: &str, profile: &str) -> Result<Slot, Error> {
        let (version, sha, a) = self.remote.download(n).map_err(|e| e.error())?;
        archive::install(self.data, id, &a, profile, None).map_err(|_| Error::Code("bad_save"))?;
        let local = packed(self.data, id).map(|p| p.1).unwrap_or_default();
        let cloud = self.remote.list().unwrap_or(Cloud { max_slots: saves::MAX_SLOTS, max_bytes: saves::MAX_BYTES, slots: Vec::new() });
        let slot = cloud.slot(n).cloned().filter(|s| s.version == version).unwrap_or(Slot { slot: n, version, sha256: sha, ..Default::default() });
        self.linked(id, user, &slot, local);
        Ok(slot)
    }

    /// The player's answer to a newer version or a conflict: `cloud` (the cloud's replaces this
    /// computer's), `local` (this computer's goes up over it; the cloud keeps its older
    /// versions), `both` (this computer's kept as a local copy, the cloud's downloaded). The id
    /// of the local copy, for `both`.
    pub fn resolve(&mut self, id: &str, keep: &str, copy_name: &str) -> Result<Option<String>, Error> {
        let user = self.remote.user()?;
        let s = sync_of(self.data, id);
        if !s.linked(&user) {
            return Ok(None);
        }
        let profile = files::load(self.data, id).map(|c| files::text(&c, "profile")).unwrap_or_default();
        match keep {
            "local" => {
                let (name, (a, sha)) = (self.company_name(id).ok_or(Error::Code("server"))?, packed(self.data, id).ok_or(Error::Code("server"))?);
                let cloud = self.remote.list()?;
                let base = cloud.slot(s.slot).map_or(0, |x| x.version);
                match self.remote.upload(Some(s.slot), &name, base, &a) {
                    Ok(slot) => self.linked(id, &user, &slot, sha),
                    Err(e) => return Err(e.error()),
                }
                Ok(None)
            }
            "both" => {
                let (a, _) = packed(self.data, id).ok_or(Error::Code("server"))?;
                let copy = files::unused_id(self.data, copy_name);
                archive::install(self.data, &copy, &a, "", Some(copy_name)).map_err(|_| Error::Code("server"))?;
                self.fetch_into(&user, s.slot, id, &profile)?;
                Ok(Some(copy))
            }
            _ => {
                self.fetch_into(&user, s.slot, id, &profile)?;
                Ok(None)
            }
        }
    }

    /// "Download to this PC": the company linked to slot `n` brought up to the cloud's version
    /// (unless it changed here too: a conflict, asked), or the slot's company put on this
    /// computer as a new one. The company's id.
    pub fn download(&mut self, n: u8, profile: &str) -> Result<Result<String, Decision>, Error> {
        let user = self.remote.user()?;
        let here = read_book(self.data).companies.into_iter().find(|(id, o)| o.linked(&user) && o.slot == n && files::path_of(self.data, id).exists());
        if let Some((id, _)) = here {
            return match self.check(&id)? {
                d @ Decision::Conflict(_) => Ok(Err(d)),
                // (unlinked by the check: put on this computer as a new one)
                Decision::Gone | Decision::Other(_) => self.download(n, profile),
                // (Local: the copy here cannot be read - the cloud's takes its place; asked again
                // it would find the same company and never end)
                _ => {
                    self.fetch_into(&user, n, &id, profile)?;
                    Ok(Ok(id))
                }
            };
        }
        let cloud = self.remote.list()?;
        let slot = cloud.slot(n).filter(|s| !s.empty).cloned().ok_or(Error::Code("no_cloud_save"))?;
        let id = files::unused_id(self.data, &slot.name);
        self.fetch_into(&user, n, &id, profile)?;
        Ok(Ok(id))
    }

    /// Slot `n` removed from the cloud; the company of this computer it held stays, local.
    pub fn remove(&mut self, n: u8) -> Result<(), Error> {
        let user = self.remote.user()?;
        self.remote.delete(n)?;
        for (id, o) in read_book(self.data).companies {
            if o.linked(&user) && o.slot == n {
                self.unlink(&id);
            }
        }
        Ok(())
    }
}

// --- the app's state: what the page is shown, the worker -------------------------------------

#[derive(Default)]
struct State {
    worker: bool,
    /// The cloud as `list` last said it, and when.
    cloud: Option<Cloud>,
    listed_at: u64,
    user: String,
    offline: bool,
    signed_in: bool,
    /// Uploading now.
    busy: HashSet<String>,
    /// Companies found in conflict (asked when opened).
    conflict: HashSet<String>,
    /// A company's last upload error.
    failed: HashMap<String, Error>,
    /// To upload soon: their pages were left.
    queue: Vec<String>,
    /// When each linked company was last tried.
    tried: HashMap<String, u64>,
    /// The sha256 of each company as packed, by its file's size and time.
    shas: HashMap<String, (u64, std::time::SystemTime, String)>,
    /// Notices for the page (said once).
    notices: Vec<String>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Default::default);
/// The engine's work runs one at a time (the worker's uploads, the page's commands).
static WORK: Mutex<()> = Mutex::new(());

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn work() -> MutexGuard<'static, ()> {
    WORK.lock().unwrap_or_else(|e| e.into_inner())
}

fn data() -> PathBuf {
    hub_core::data_dir()
}

/// The driver's name the companies are kept under (`driver` in the settings, as the drive and
/// the company's chooser read it; none chosen: every company is the driver's).
fn profile() -> String {
    hub_core::settings::Settings::load().other.get("driver").and_then(Value::as_str).unwrap_or("").to_string()
}

/// How often a changed cloud company goes up, and the cloud is listed again.
const UPLOAD_EVERY: u64 = 10 * 60;
const LIST_EVERY: u64 = 5 * 60;

/// The company's sha256 as packed now (read again only when its file changed).
fn local_sha(id: &str) -> Option<String> {
    let d = data();
    let meta = std::fs::metadata(files::path_of(&d, id)).ok()?;
    let key = (meta.len(), meta.modified().ok()?);
    if let Some((len, at, sha)) = state().shas.get(id) {
        if (*len, *at) == key {
            return Some(sha.clone());
        }
    }
    let (_, sha) = packed(&d, id)?;
    state().shas.insert(id.to_string(), (key.0, key.1, sha.clone()));
    Some(sha)
}

/// The account's state into the view after a call: signed in or out, offline.
fn note<T>(r: &Result<T, Error>) {
    let mut st = state();
    match r {
        Err(Error::Offline) => st.offline = true,
        Err(Error::Code("not_signed_in" | "privacy") | Error::SignedOut) => {
            st.signed_in = false;
            st.cloud = None;
        }
        Ok(_) => {
            st.offline = false;
            st.signed_in = true;
        }
        Err(_) => st.offline = false,
    }
}

/// The cloud listed again into the view.
fn relist(remote: &mut dyn Remote) -> Result<(), Error> {
    let r = remote.user().and_then(|u| remote.list().map(|c| (u, c)));
    note(&r);
    let (user, cloud) = r?;
    let mut st = state();
    st.user = user;
    st.cloud = Some(cloud);
    st.listed_at = now();
    Ok(())
}

/// One company's sync on the worker (or after its pages were left).
fn sync_now(id: &str) {
    let _w = work();
    state().busy.insert(id.to_string());
    let d = data();
    let r = Engine { data: &d, remote: &mut Online }.sync(id);
    note(&r);
    // (the content is built and sent off the lock: minutes of work the page's commands must not
    // wait for)
    drop(_w);
    let gone = trf("%{name} is no longer in the cloud: it stays on this PC as a local save.", &[("name", &name_of(id))]);
    // (the content beside the save, for the website's bus company: `cloud_content`)
    if matches!(r, Ok(Decision::Synced)) {
        content_of(&d, id);
    }
    let mut st = state();
    st.busy.remove(id);
    st.tried.insert(id.to_string(), now());
    st.shas.remove(id);
    match r {
        Ok(Decision::Conflict(_)) => {
            st.conflict.insert(id.to_string());
        }
        Ok(Decision::Gone | Decision::Other(_)) => st.notices.push(gone),
        Ok(_) => {
            st.failed.remove(id);
            st.conflict.remove(id);
            // (the slot's new version: listed again soon)
            st.listed_at = 0;
        }
        Err(Error::Offline | Error::Code("not_signed_in" | "privacy") | Error::SignedOut) => {}
        Err(e) => {
            st.failed.insert(id.to_string(), e);
        }
    }
}

/// The content beside a cloud company's save sent into its slot (at most every half hour;
/// `cloud_content::send`). What goes wrong stays in the log: the save itself went up.
fn content_of(d: &Path, id: &str) {
    let s = sync_of(d, id);
    if s.place != Place::Cloud {
        return;
    }
    if let Err(e) = crate::cloud_content::send(d, id, s.slot) {
        crate::launcher::log_to_file(&format!("cloud content of {id}: {e:?}"));
    }
}

fn name_of(id: &str) -> String {
    files::load(&data(), id).map(|c| files::text(&c, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| id.to_string())
}

/// The worker: the cloud listed every few minutes, a changed cloud company uploaded when its
/// pages were left and every ten minutes, never under the view's lock during a request.
fn worker() {
    loop {
        // (signed in with the privacy statement agreed to: before, the cloud is not asked)
        // (another driver than the account's plays locally: `drivers::plays_online`)
        let signed = online::with(|c| c.signed_in() && !c.privacy_needed()) && crate::drivers::plays_online();
        if signed {
            let stale = { now().saturating_sub(state().listed_at) >= LIST_EVERY };
            if stale {
                let _w = work();
                let _ = relist(&mut Online);
            }
            let (user, queued, tried) = {
                let mut st = state();
                (st.user.clone(), std::mem::take(&mut st.queue), st.tried.clone())
            };
            if !user.is_empty() && !state().offline {
                for (id, s) in read_book(&data()).companies {
                    if !s.linked(&user) || state().conflict.contains(&id) {
                        continue;
                    }
                    let due = queued.contains(&id) || tried.get(&id).is_none_or(|t| now().saturating_sub(*t) >= UPLOAD_EVERY);
                    if due && local_sha(&id).is_some_and(|sha| sha != s.local_sha256) {
                        sync_now(&id);
                    } else if due {
                        state().tried.insert(id, now());
                    }
                }
            }
        } else {
            let mut st = state();
            st.signed_in = false;
            st.cloud = None;
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn start_worker() {
    let mut st = state();
    if st.worker {
        return;
    }
    st.worker = true;
    let _ = std::thread::Builder::new().name("omsihub cloud saves".into()).spawn(worker);
}

/// The company's pages were left: a change goes up soon (on the worker's thread). For the
/// company port, as the page leaves a company.
pub fn closed(id: &str) {
    if !id.is_empty() && sync_of(&data(), id).place == Place::Cloud {
        state().queue.push(id.to_string());
        start_worker();
    }
}

// --- the page ---------------------------------------------------------------------------------------

/// Unix milliseconds of a slot's time (RFC 3339, or a number in seconds or milliseconds).
fn millis(t: Option<&str>) -> Option<i64> {
    let t = t?.trim();
    if let Ok(n) = t.parse::<i64>() {
        return Some(if n > 100_000_000_000 { n } else { n * 1000 });
    }
    online::unix_of(t).map(|s| s * 1000)
}

fn slot_json(s: &Slot, linked: Option<&str>) -> Value {
    json!({ "slot": s.slot, "empty": s.empty, "name": s.name, "size": s.size, "version": s.version, "device": s.device, "at": millis(s.updated_at.as_deref()), "linked": linked })
}

/// What a company's states are called (the last: a local save).
const STATES: &[(&str, &str)] =
    &[("synced", "Synced"), ("uploading", "Uploading"), ("newer", "Newer in the cloud"), ("conflict", "Changed on both"), ("offline", "Offline"), ("error", "Not synced"), ("local", "Local save")];

fn state_label(st: &str) -> String {
    tr(STATES.iter().find(|x| x.0 == st).unwrap_or(&STATES[STATES.len() - 1]).1)
}

/// The chooser's badges and the account panel's slots, from what was last fetched.
fn view() -> Value {
    start_worker();
    let d = data();
    let book = read_book(&d);
    let mut companies = serde_json::Map::new();
    let (user, cloud, offline, signed_in) = {
        let st = state();
        (st.user.clone(), st.cloud.clone(), st.offline, st.signed_in)
    };
    for (id, s) in &book.companies {
        if s.place != Place::Cloud || !files::path_of(&d, id).exists() {
            continue;
        }
        // (in the cloud of another account: local here)
        if !user.is_empty() && s.user != user {
            continue;
        }
        let sha = local_sha(id);
        let st = state();
        let changed = sha.as_deref().is_some_and(|x| x != s.local_sha256);
        let there = cloud.as_ref().and_then(|c| c.slot(s.slot));
        let newer = there.is_some_and(|x| x.version != s.version);
        let kind = if !signed_in || offline || user.is_empty() {
            "offline"
        } else if st.conflict.contains(id) || (changed && newer) {
            "conflict"
        } else if st.busy.contains(id) || (changed && !st.failed.contains_key(id)) {
            "uploading"
        } else if st.failed.contains_key(id) {
            "error"
        } else if newer {
            "newer"
        } else {
            "synced"
        };
        let tip = st.failed.get(id).map(|e| tr(e.text()));
        companies.insert(id.clone(), json!({ "place": "cloud", "state": kind, "label": state_label(kind), "tip": tip, "slot": s.slot }));
    }
    let linked = |n: u8| book.companies.iter().find(|(id, s)| s.linked(&user) && s.slot == n && files::path_of(&d, id).exists()).map(|(id, _)| id.clone());
    let slots: Vec<Value> = cloud.as_ref().map(|c| c.slots.iter().map(|s| slot_json(s, linked(s.slot).as_deref())).collect()).unwrap_or_default();
    let notices = std::mem::take(&mut state().notices);
    json!({
        "signed_in": signed_in && !user.is_empty(), "offline": offline, "listed": cloud.is_some(),
        "max_slots": cloud.as_ref().map_or(saves::MAX_SLOTS, |c| c.max_slots), "slots": slots,
        "companies": companies, "local_label": state_label("local"), "cloud_label": tr("Cloud save"), "notices": notices,
    })
}

/// The driver's companies, each with its whole name for the picker: "Land (LAN) · Grundorf" -
/// the name, the short name and the home map (also the account panel's company board).
pub(crate) fn companies() -> Value {
    let list = files::list(&data(), &profile());
    json!(list
        .iter()
        .map(|(id, c)| {
            let (name, short) = (files::text(c, "name"), files::text(c, "short"));
            let map_path = files::text(c, "map");
            let map = Some(files::text(c, "map_name")).filter(|n| !n.trim().is_empty()).or_else(|| Path::new(&map_path).parent()?.file_name()?.to_str().map(str::to_string)).unwrap_or_default();
            let mut label = name.trim().to_string();
            if !short.trim().is_empty() && !short.trim().eq_ignore_ascii_case(name.trim()) {
                label = format!("{label} ({})", short.trim());
            }
            if !map.trim().is_empty() {
                label = format!("{label} · {}", map.trim());
            }
            json!({ "id": id, "name": name, "label": label })
        })
        .collect::<Vec<_>>())
}

/// A copy's name: "<name> (this PC)".
fn copy_name(id: &str) -> String {
    trf("%{name} (this PC)", &[("name", &name_of(id))])
}

/// The two copies of a company as the question shows them: when and where each was saved.
fn sides(id: &str, cloud: &Slot) -> Value {
    let d = data();
    let local_at = std::fs::metadata(files::path_of(&d, id)).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|t| t.as_millis() as i64);
    json!({
        "local": { "at": local_at, "device": saves::device_name() },
        "cloud": { "at": millis(cloud.updated_at.as_deref()), "device": cloud.device, "version": cloud.version },
    })
}

fn error(e: Error) -> Value {
    json!({ "error": tr(e.text()), "code": e.code() })
}

/// A company about to be opened: what the cloud says of it, and what to ask.
fn open(id: &str) -> Value {
    let d = data();
    if sync_of(&d, id).place != Place::Cloud {
        return json!({ "state": "local" });
    }
    let _w = work();
    let r = Engine { data: &d, remote: &mut Online }.check(id);
    note(&r);
    let _ = relist(&mut Online);
    match r {
        Ok(Decision::Newer(slot)) => {
            let mut v = sides(id, &slot);
            v["ask"] = json!("newer");
            v["name"] = json!(name_of(id));
            v
        }
        Ok(Decision::Conflict(slot)) => {
            state().conflict.insert(id.to_string());
            let mut v = sides(id, &slot);
            v["ask"] = json!("conflict");
            v["name"] = json!(name_of(id));
            v["copy"] = json!(copy_name(id));
            v
        }
        Ok(Decision::Upload) => {
            state().queue.push(id.to_string());
            json!({ "state": "uploading" })
        }
        Ok(Decision::Gone | Decision::Other(_)) => json!({ "state": "local", "notice": trf("%{name} is no longer in the cloud: it stays on this PC as a local save.", &[("name", &name_of(id))]) }),
        Ok(Decision::Synced | Decision::Local) => json!({ "state": "synced" }),
        // (offline, signed out, the server busy: opened as it is, quietly)
        Err(e) => json!({ "state": "offline", "code": e.code() }),
    }
}

fn resolve(id: &str, keep: &str) -> Value {
    let d = data();
    let _w = work();
    let name = name_of(id);
    let copy = copy_name(id);
    let r = Engine { data: &d, remote: &mut Online }.resolve(id, keep, &copy);
    note(&r);
    {
        let mut st = state();
        st.shas.remove(id);
        if r.is_ok() {
            st.conflict.remove(id);
            st.failed.remove(id);
        }
        st.listed_at = 0;
    }
    match r {
        Ok(copy_id) => json!({
            "ok": true, "reread": id, "copy": copy_id,
            "message": match keep {
                "local" => trf("This PC's %{name} is in the cloud now.", &[("name", &name)]),
                "both" => trf("Both kept: the cloud's %{name}, and this PC's as %{copy}.", &[("name", &name), ("copy", &copy)]),
                _ => trf("The cloud's %{name} is on this PC now.", &[("name", &name)]),
            },
        }),
        Err(e) => error(e),
    }
}

fn put(id: &str, replace: Option<u8>) -> Value {
    let d = data();
    let _w = work();
    let name = name_of(id);
    let r = Engine { data: &d, remote: &mut Online }.put(id, replace);
    note(&r);
    state().listed_at = 0;
    let _ = relist(&mut Online);
    match r {
        Ok(Put::Done(slot)) => {
            let id = id.to_string();
            let _ = std::thread::Builder::new().name("omsihub cloud content".into()).spawn(move || content_of(&data(), &id));
            json!({ "ok": true, "slot": slot.slot, "message": trf("%{name} is in the cloud now.", &[("name", &name)]) })
        }
        Ok(Put::Limit(slots)) => json!({
            "limit": true, "text": tr(Error::Code("limit").text()),
            "slots": slots.iter().map(|s| slot_json(s, None)).collect::<Vec<_>>(),
        }),
        Err(e) => error(e),
    }
}

fn download(n: u8) -> Value {
    let d = data();
    let _w = work();
    let r = Engine { data: &d, remote: &mut Online }.download(n, &profile());
    note(&r);
    state().listed_at = 0;
    match r {
        Ok(Ok(id)) => {
            state().shas.remove(&id);
            json!({ "ok": true, "reread": id, "message": trf("%{name} is on this PC now.", &[("name", &name_of(&id))]) })
        }
        Ok(Err(_)) => error(Error::Code("conflict")),
        Err(e) => error(e),
    }
}

fn remove(n: u8) -> Value {
    let d = data();
    let _w = work();
    let r = Engine { data: &d, remote: &mut Online }.remove(n);
    note(&r);
    state().listed_at = 0;
    let _ = relist(&mut Online);
    match r {
        Ok(()) => json!({ "ok": true, "message": tr("Removed from the cloud. A company of it on this PC stays as a local save.") }),
        Err(e) => error(e),
    }
}

/// The account `user` deleted (its cloud saves with it): its companies here are local again -
/// not those in the cloud of another account signed in on this computer before (`None`: all).
pub fn forget_account(user: Option<&str>) {
    let d = data();
    let _w = work();
    for (id, s) in read_book(&d).companies {
        if user.is_none_or(|u| s.user == u) {
            write_sync(&d, &id, None);
        }
    }
    let mut st = state();
    st.cloud = None;
    st.user.clear();
    st.conflict.clear();
    st.failed.clear();
}

/// A `cs_` command.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    online::set_language(args.get("lang").and_then(Value::as_str).unwrap_or(""));
    let id = args.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let slot = args.get("slot").and_then(Value::as_u64).map(|n| n.min(255) as u8);
    // (an id is a file's name in the companies' folder: nothing that reaches outside it)
    let ok_id = !id.is_empty() && !id.contains(['/', '\\', '.']) && files::path_of(&data(), &id).exists();
    // (the cloud saves are the account's driver's: another driver keeps theirs here)
    if matches!(cmd, "cs_resolve" | "cs_put" | "cs_sync" | "cs_download" | "cs_remove") && !crate::drivers::plays_online() {
        return Ok(error(Error::Code("not_signed_in")));
    }
    Ok(match cmd {
        "cs_state" => view(),
        "cs_companies" => companies(),
        "cs_open" if ok_id => open(&id),
        "cs_resolve" if ok_id => resolve(&id, args.get("keep").and_then(Value::as_str).unwrap_or("cloud")),
        "cs_put" if ok_id => put(&id, args.get("replace").and_then(Value::as_u64).map(|n| n.min(255) as u8)),
        "cs_local" if ok_id => {
            // (after a sync that runs now: it would link the company again as it ends)
            let _w = work();
            let d = data();
            Engine { data: &d, remote: &mut Online }.unlink(&id);
            json!({ "ok": true, "message": trf("%{name} stays local.", &[("name", &name_of(&id))]) })
        }
        "cs_sync" if ok_id => {
            closed(&id);
            json!({})
        }
        "cs_download" => match slot {
            Some(n) => download(n),
            None => error(Error::Code("bad_input")),
        },
        "cs_remove" => match slot {
            Some(n) => remove(n),
            None => error(Error::Code("bad_input")),
        },
        "cs_open" | "cs_resolve" | "cs_put" | "cs_local" | "cs_sync" => error(Error::Code("bad_input")),
        _ => return Err(format!("unknown command: {cmd}")),
    })
}

#[cfg(test)]
#[path = "cloud_tests.rs"]
mod tests;
