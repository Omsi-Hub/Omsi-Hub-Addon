//! The cloud saves' engine against a cloud in memory (`Fake`, shared by two "computers": two data
//! folders), as the function behaves (docs/ONLINE.md): two slots, `limit`, `conflict` on another
//! base version. The client's own requests are tested against a mock server in `hub-online`;
//! nothing here reaches the network. The companies are written as the company port keeps them
//! (`companies/<id>.json`), with only the fields these tests need.

use super::*;
use crate::Quiet;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct FakeCloud {
    /// slot → (name, version, sha256, the archive)
    slots: BTreeMap<u8, (String, u64, String, Vec<u8>)>,
    offline: bool,
    signed_out: bool,
    uploads: usize,
}

#[derive(Clone, Default)]
struct Fake(Rc<RefCell<FakeCloud>>);

impl Fake {
    fn slot(n: u8, s: &(String, u64, String, Vec<u8>)) -> Slot {
        Slot {
            slot: n,
            empty: false,
            name: s.0.clone(),
            version: s.1,
            size: s.3.len() as u64,
            sha256: s.2.clone(),
            updated_at: Some("2026-10-16T10:00:00Z".into()),
            device: "Laptop".into(),
            versions: vec![s.1],
        }
    }
    fn gate(&self) -> Result<(), Error> {
        let f = self.0.borrow();
        if f.signed_out {
            Err(Error::Code("not_signed_in"))
        } else if f.offline {
            Err(Error::Offline)
        } else {
            Ok(())
        }
    }
}

impl Remote for Fake {
    fn user(&mut self) -> Result<String, Error> {
        self.gate().map(|_| "me".into())
    }

    fn list(&mut self) -> Result<Cloud, Error> {
        self.gate()?;
        let f = self.0.borrow();
        let slots = (1..=2).map(|n| f.slots.get(&n).map(|s| Fake::slot(n, s)).unwrap_or(Slot { slot: n, empty: true, ..Default::default() })).collect();
        Ok(Cloud { max_slots: 2, max_bytes: saves::MAX_BYTES, slots })
    }

    fn upload(&mut self, slot: Option<u8>, name: &str, base: u64, archive: &[u8]) -> Result<Slot, SaveError> {
        self.gate()?;
        let mut f = self.0.borrow_mut();
        let n = match slot {
            Some(n) => n,
            None => (1..=2).find(|n| !f.slots.contains_key(n)).ok_or(SaveError::Failed(Error::Code("limit")))?,
        };
        let current = f.slots.get(&n).map_or(0, |s| s.1);
        if base != current {
            let now = f.slots.get(&n).map(|s| Fake::slot(n, s)).unwrap_or(Slot { slot: n, empty: true, ..Default::default() });
            return Err(SaveError::Conflict(now));
        }
        let packed = saves::compress(archive)?;
        f.slots.insert(n, (name.to_string(), current + 1, packed.sha256, archive.to_vec()));
        f.uploads += 1;
        Ok(Fake::slot(n, &f.slots[&n]))
    }

    fn download(&mut self, slot: u8) -> Result<(u64, String, Vec<u8>), SaveError> {
        self.gate()?;
        let f = self.0.borrow();
        let s = f.slots.get(&slot).ok_or(SaveError::Failed(Error::Code("no_cloud_save")))?;
        Ok((s.1, s.2.clone(), s.3.clone()))
    }

    fn delete(&mut self, slot: u8) -> Result<(), Error> {
        self.gate()?;
        self.0.borrow_mut().slots.remove(&slot);
        Ok(())
    }
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("omsihub-cloudsync-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A company founded on a computer (as the company port writes one: these fields and more).
fn company(data: &Path, name: &str) -> String {
    let id = files::unused_id(data, name);
    files::save(data, &json!({ "id": id, "name": name, "short": "", "profile": "Luc", "cash": 10_000, "map": "maps/Grundorf/global.cfg", "fleet": [{ "id": 1 }] })).unwrap();
    id
}

fn load(data: &Path, id: &str) -> Value {
    files::load(data, id).unwrap()
}

/// The company changed (a day's money).
fn change(data: &Path, id: &str, cash: i64) {
    let mut c = load(data, id);
    c["cash"] = json!(c["cash"].as_i64().unwrap() + cash);
    files::save(data, &c).unwrap();
}

fn cash(data: &Path, id: &str) -> i64 {
    load(data, id)["cash"].as_i64().unwrap()
}

// --- the archive -------------------------------------------------------------------------------------

#[test]
fn a_company_packs_the_same_and_comes_back_as_it_was_or_as_another() {
    let (a, b) = (temp("pack-a"), temp("pack-b"));
    let id = company(&a, "Stadtbus");
    let c = load(&a, &id);
    std::fs::write(files::live_file(&a, &id), "{\"breakdown\":3}\n").unwrap();
    std::fs::write(files::orders_file(&a, &id), "{\"build\":\"wash\"}\n").unwrap();
    std::fs::write(files::dir(&a).join(format!("{id}.live-2026-10-16.taken")), "{}\n").unwrap();
    // (the game's plans stay behind; another company's files too)
    let [plan, drive_plan] = files::plan_files(&a, &id);
    std::fs::write(&plan, "{}").unwrap();
    std::fs::write(files::live_file(&a, "stadtbus-2"), "{\"breakdown\":9}\n").unwrap();
    let packed = archive::pack(&a, &id).unwrap();
    assert_eq!(archive::pack(&a, &id).unwrap(), packed, "the same files pack the same");
    let u = archive::unpack(&packed).unwrap();
    assert_eq!(u.files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), [archive::COMPANY, "live-2026-10-16.taken", "live.jsonl", "orders.jsonl"]);
    assert_eq!(u.company, c);
    // (on another computer, for its driver; a stale plan of the day there goes)
    std::fs::create_dir_all(files::dir(&b)).unwrap();
    std::fs::write(files::plan_files(&b, &id)[0].clone(), "{}").unwrap();
    let back = archive::install(&b, &id, &packed, "Anna", None).unwrap();
    assert_eq!((back["id"].as_str(), back["profile"].as_str()), (Some("stadtbus"), Some("Anna")));
    assert_eq!(std::fs::read_to_string(files::orders_file(&b, &id)).unwrap(), "{\"build\":\"wash\"}\n");
    assert_eq!(std::fs::read_to_string(files::live_file(&b, &id)).unwrap(), "{\"breakdown\":3}\n");
    assert!(files::dir(&b).join(format!("{id}.live-2026-10-16.taken")).exists());
    assert!(!files::plan_files(&b, &id)[0].exists());
    assert_eq!(load(&b, &id)["fleet"], c["fleet"], "the rest of the company passes through");
    // (kept both: as another company, its own files, the first one untouched)
    let copy = archive::install(&a, "stadtbus-copy", &packed, "", Some("Stadtbus (copy)")).unwrap();
    assert_eq!((copy["name"].as_str(), copy["profile"].as_str()), (Some("Stadtbus (copy)"), Some("Luc")));
    assert_eq!(load(&a, "stadtbus"), c);
    assert!(files::orders_file(&a, "stadtbus-copy").exists());
    // (a newer save over an older company: what lay beside the old one goes)
    let mut plain = c.clone();
    plain["cash"] = json!(10_100);
    std::fs::create_dir_all(files::dir(&b)).unwrap();
    files::save(&b, &plain).unwrap();
    std::fs::remove_file(files::live_file(&b, &id)).unwrap();
    std::fs::remove_file(files::orders_file(&b, &id)).unwrap();
    std::fs::remove_file(files::dir(&b).join(format!("{id}.live-2026-10-16.taken"))).unwrap();
    let newer = archive::pack(&b, &id).unwrap();
    archive::install(&a, &id, &newer, "", None).unwrap();
    assert_eq!(cash(&a, &id), 10_100);
    assert!(!files::orders_file(&a, &id).exists() && !files::live_file(&a, &id).exists());
    assert!(!plan.exists() && !drive_plan.exists());
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn a_damaged_or_hostile_save_changes_nothing() {
    let data = temp("hostile");
    let id = company(&data, "Alt");
    let c = load(&data, &id);
    let good = archive::pack(&data, &id).unwrap();
    assert!(archive::unpack(b"PK\x03\x04 not ours").is_err());
    assert!(archive::unpack(&good[..good.len() - 3]).is_err(), "cut short");
    let mut evil = vec![("company.json".to_string(), std::fs::read(files::path_of(&data, &id)).unwrap()), ("../../evil.txt".to_string(), b"x".to_vec())];
    assert!(archive::unpack(&archive::pack_files(&mut evil)).is_err());
    let mut none = vec![("live.jsonl".to_string(), b"{}".to_vec())];
    assert!(archive::unpack(&archive::pack_files(&mut none)).is_err(), "no company in it");
    let mut broken = vec![(archive::COMPANY.to_string(), b"{ not json".to_vec())];
    assert!(archive::install(&data, &id, &archive::pack_files(&mut broken), "", None).is_err());
    let mut not_one = vec![(archive::COMPANY.to_string(), b"[1, 2]".to_vec())];
    assert!(archive::install(&data, &id, &archive::pack_files(&mut not_one), "", None).is_err());
    assert_eq!(load(&data, &id), c, "the company as it was");
    assert!(archive::allowed("live-2026-10-16.taken") && !archive::allowed("live-..\\x.taken") && !archive::allowed("company.JSON"));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_companies_folder_is_read_as_the_company_port_keeps_it() {
    let data = temp("files");
    assert_eq!(files::slug(" Stadtbus  Grundorf! "), "stadtbus-grundorf");
    assert_eq!(files::slug("???"), "company");
    let a = company(&data, "Stadtbus");
    assert_eq!(company(&data, "Stadtbus"), "stadtbus-2");
    let mut other = load(&data, &a);
    other["id"] = json!("ring");
    other["name"] = json!("Ring");
    other["profile"] = json!("Anna");
    files::save(&data, &other).unwrap();
    // (the plans and the side files beside them are no companies)
    std::fs::write(files::plan_files(&data, "ring")[0].clone(), "{}").unwrap();
    std::fs::write(files::dir(&data).join("broken.json"), "{").unwrap();
    let ids = |p: &str| files::list(&data, p).into_iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert_eq!(ids(""), ["ring", "stadtbus", "stadtbus-2"]);
    assert_eq!(ids("luc"), ["stadtbus", "stadtbus-2"]);
    assert_eq!(ids("Anna"), ["ring"]);
    let _ = std::fs::remove_dir_all(&data);
}

// --- the engine --------------------------------------------------------------------------------------

#[test]
fn an_old_company_without_sync_metadata_is_local() {
    let data = temp("old");
    let id = company(&data, "Alt");
    // (no metadata at all, an entry of before with fields it lacks, nonsense)
    assert_eq!(sync_of(&data, &id), Sync::default());
    assert_eq!(sync_of(&data, &id).place, Place::Local);
    std::fs::write(book_path(&data), format!(r#"{{"companies":{{"{id}":{{"slot":1}}}}}}"#)).unwrap();
    assert_eq!(sync_of(&data, &id).place, Place::Local);
    std::fs::write(book_path(&data), "not json").unwrap();
    assert_eq!(sync_of(&data, &id), Sync::default());
    let fake = Fake::default();
    let mut remote = fake.clone();
    let mut e = Engine { data: &data, remote: &mut remote };
    assert_eq!(e.check(&id), Ok(Decision::Local));
    assert_eq!(e.sync(&id), Ok(Decision::Local));
    assert_eq!(fake.0.borrow().uploads, 0, "a local company never goes up");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_company_put_in_the_cloud_syncs_its_changes_and_comes_down_on_another_pc() {
    let (a, b) = (temp("up-a"), temp("up-b"));
    let fake = Fake::default();
    let (mut ra, mut rb) = (fake.clone(), fake.clone());
    let id = company(&a, "Stadtbus");
    let Put::Done(slot) = Engine { data: &a, remote: &mut ra }.put(&id, None).unwrap() else { panic!() };
    assert_eq!((slot.slot, slot.version), (1, 1));
    let s = sync_of(&a, &id);
    assert_eq!((s.place, s.slot, s.version, s.user.as_str()), (Place::Cloud, 1, 1, "me"));
    // (unchanged: nothing to upload; changed: on top of version 1)
    assert_eq!(Engine { data: &a, remote: &mut ra }.sync(&id), Ok(Decision::Synced));
    assert_eq!(fake.0.borrow().uploads, 1);
    change(&a, &id, 500);
    assert_eq!(Engine { data: &a, remote: &mut ra }.check(&id), Ok(Decision::Upload));
    assert_eq!(Engine { data: &a, remote: &mut ra }.sync(&id), Ok(Decision::Synced));
    assert_eq!(sync_of(&a, &id).version, 2);
    // (the other PC: downloaded as a new company of its driver, linked to the slot)
    let got = Engine { data: &b, remote: &mut rb }.download(1, "Anna").unwrap().unwrap();
    assert_eq!(cash(&b, &got), cash(&a, &id));
    assert_eq!(load(&b, &got)["profile"], "Anna");
    assert_eq!((sync_of(&b, &got).slot, sync_of(&b, &got).version), (1, 2));
    assert_eq!(Engine { data: &b, remote: &mut rb }.check(&got), Ok(Decision::Synced), "the driver's name is no change");
    // (a change there comes back here as newer, downloaded when chosen)
    change(&b, &got, 70);
    Engine { data: &b, remote: &mut rb }.sync(&got).unwrap();
    let Ok(Decision::Newer(n)) = Engine { data: &a, remote: &mut ra }.check(&id) else { panic!() };
    assert_eq!(n.version, 3);
    Engine { data: &a, remote: &mut ra }.resolve(&id, "cloud", "x").unwrap();
    assert_eq!(cash(&a, &id), cash(&b, &got));
    assert_eq!(load(&a, &id)["profile"], "Luc", "this PC's driver keeps it");
    assert_eq!(Engine { data: &a, remote: &mut ra }.check(&id), Ok(Decision::Synced));
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn a_conflict_is_asked_and_never_loses_a_copy() {
    let (a, b) = (temp("conflict-a"), temp("conflict-b"));
    let fake = Fake::default();
    let (mut ra, mut rb) = (fake.clone(), fake.clone());
    let id = company(&a, "Stadtbus");
    Engine { data: &a, remote: &mut ra }.put(&id, None).unwrap();
    let other = Engine { data: &b, remote: &mut rb }.download(1, "Luc").unwrap().unwrap();
    let start = cash(&a, &id);
    // (both play on: the laptop uploads first)
    change(&b, &other, 1000);
    Engine { data: &b, remote: &mut rb }.sync(&other).unwrap();
    change(&a, &id, -300);
    // (this PC's upload is refused, nothing overwritten either way)
    let Ok(Decision::Conflict(now)) = Engine { data: &a, remote: &mut ra }.sync(&id) else { panic!() };
    assert_eq!((now.version, now.device.as_str()), (2, "Laptop"));
    assert_eq!(cash(&a, &id), start - 300);
    assert!(matches!(Engine { data: &a, remote: &mut ra }.check(&id), Ok(Decision::Conflict(_))));
    // (keep both: this PC's as a local copy, the cloud's under the company's own id)
    let copy = Engine { data: &a, remote: &mut ra }.resolve(&id, "both", "Stadtbus (this PC)").unwrap().unwrap();
    assert_eq!(cash(&a, &copy), start - 300);
    assert_eq!(load(&a, &copy)["name"], "Stadtbus (this PC)");
    assert_eq!(sync_of(&a, &copy).place, Place::Local);
    assert_eq!(cash(&a, &id), start + 1000);
    assert_eq!(Engine { data: &a, remote: &mut ra }.check(&id), Ok(Decision::Synced));
    // (another conflict, this PC's kept: on top of the cloud's version, which the cloud keeps)
    change(&b, &other, 1);
    Engine { data: &b, remote: &mut rb }.sync(&other).unwrap();
    change(&a, &id, 5);
    assert!(matches!(Engine { data: &a, remote: &mut ra }.check(&id), Ok(Decision::Conflict(_))));
    Engine { data: &a, remote: &mut ra }.resolve(&id, "local", "x").unwrap();
    assert_eq!(fake.0.borrow().slots[&1].1, 4);
    assert!(matches!(Engine { data: &b, remote: &mut rb }.check(&other), Ok(Decision::Newer(_))));
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn the_third_company_is_offered_keep_local_or_replace_and_never_blocked() {
    let data = temp("limit");
    let fake = Fake::default();
    let mut r = fake.clone();
    let ids: Vec<String> = ["One", "Two", "Three"].iter().map(|n| company(&data, n)).collect();
    for id in &ids[..2] {
        assert!(matches!(Engine { data: &data, remote: &mut r }.put(id, None), Ok(Put::Done(_))));
    }
    let Ok(Put::Limit(slots)) = Engine { data: &data, remote: &mut r }.put(&ids[2], None) else { panic!() };
    assert_eq!(slots.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["One", "Two"]);
    // ("keep it local": the company plays on as it is, a local save)
    assert_eq!(sync_of(&data, &ids[2]).place, Place::Local);
    assert!(files::load(&data, &ids[2]).is_some());
    assert_eq!(Engine { data: &data, remote: &mut r }.sync(&ids[2]), Ok(Decision::Local));
    // ("replace Two", confirmed: Two stays here as a local save, Three takes its slot)
    let Ok(Put::Done(s)) = Engine { data: &data, remote: &mut r }.put(&ids[2], Some(2)) else { panic!() };
    assert_eq!((s.slot, s.name.as_str()), (2, "Three"));
    assert_eq!(sync_of(&data, &ids[1]).place, Place::Local);
    assert!(files::load(&data, &ids[1]).is_some(), "its files stay");
    assert_eq!(sync_of(&data, &ids[2]).slot, 2);
    // (removed from the cloud: the company stays, local)
    Engine { data: &data, remote: &mut r }.remove(1).unwrap();
    assert_eq!(sync_of(&data, &ids[0]).place, Place::Local);
    assert!(files::load(&data, &ids[0]).is_some());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn offline_or_signed_out_everything_stays_local_and_nothing_changes() {
    let data = temp("offline");
    let fake = Fake::default();
    let mut r = fake.clone();
    let id = company(&data, "Stadtbus");
    let solo = company(&data, "Alone");
    Engine { data: &data, remote: &mut r }.put(&id, None).unwrap();
    let before = sync_of(&data, &id);
    change(&data, &id, 10);
    fake.0.borrow_mut().offline = true;
    assert_eq!(Engine { data: &data, remote: &mut r }.check(&id), Err(Error::Offline));
    assert_eq!(Engine { data: &data, remote: &mut r }.sync(&id), Err(Error::Offline));
    assert_eq!(Engine { data: &data, remote: &mut r }.put(&solo, None), Err(Error::Offline));
    assert_eq!(sync_of(&data, &id), before, "nothing forgotten");
    assert_eq!(sync_of(&data, &solo).place, Place::Local);
    fake.0.borrow_mut().offline = false;
    fake.0.borrow_mut().signed_out = true;
    assert_eq!(Engine { data: &data, remote: &mut r }.sync(&id), Err(Error::Code("not_signed_in")));
    // (back online: the change goes up)
    fake.0.borrow_mut().signed_out = false;
    assert_eq!(Engine { data: &data, remote: &mut r }.sync(&id), Ok(Decision::Synced));
    assert_eq!(sync_of(&data, &id).version, 2);
    // (signed in as another account: its link is not this account's - local here)
    let other = Sync { user: "someone else".into(), ..sync_of(&data, &id) };
    assert_eq!(decide(&other, "me", "Stadtbus", "x", &r.list().unwrap()), Decision::Local);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_slot_emptied_or_replaced_elsewhere_leaves_the_company_here_local() {
    let (a, b) = (temp("gone-a"), temp("gone-b"));
    let fake = Fake::default();
    let (mut ra, mut rb) = (fake.clone(), fake.clone());
    let id = company(&a, "Stadtbus");
    Engine { data: &a, remote: &mut ra }.put(&id, None).unwrap();
    // (the laptop put another company in its place)
    rb.delete(1).unwrap();
    let other = company(&b, "Landbus");
    Engine { data: &b, remote: &mut rb }.put(&other, Some(1)).unwrap();
    let before = cash(&a, &id);
    assert!(matches!(Engine { data: &a, remote: &mut ra }.check(&id), Ok(Decision::Other(_))));
    assert_eq!(sync_of(&a, &id).place, Place::Local);
    assert_eq!(cash(&a, &id), before, "never overwritten by another company");
    // (emptied: the same)
    let id2 = company(&a, "Ring");
    Engine { data: &a, remote: &mut ra }.put(&id2, None).unwrap();
    rb.delete(2).unwrap();
    assert_eq!(Engine { data: &a, remote: &mut ra }.check(&id2), Ok(Decision::Gone));
    assert_eq!(sync_of(&a, &id2).place, Place::Local);
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

// --- the commands ------------------------------------------------------------------------------------

/// Without an account the commands answer at once and quietly (the tests' client reaches
/// nothing: no session in its store).
#[test]
fn signed_out_the_commands_are_quiet() {
    let data = online::test_data_dir();
    let run = |cmd: &str, args: Value| call(cmd, &args, &Quiet).unwrap();
    let v = run("cs_state", json!({}));
    assert_eq!(v["signed_in"], false);
    assert_eq!(run("cs_open", json!({ "id": "" }))["code"], "bad_input");
    assert_eq!(run("cs_open", json!({ "id": "../settings" }))["code"], "bad_input");
    assert_eq!(run("cs_remove", json!({ "slot": 1 }))["code"], "not_signed_in");
    assert_eq!(run("cs_download", json!({}))["code"], "bad_input");
    // (a local company opens as it is; the picker lists the driver's companies)
    let id = company(&data, "Ringbus");
    assert_eq!(run("cs_open", json!({ "id": id }))["state"], "local");
    let list = run("cs_companies", json!({}));
    let mine = list.as_array().unwrap().iter().find(|c| c["id"] == id.as_str()).unwrap();
    assert_eq!(mine["label"], "Ringbus · Grundorf");
    assert_eq!(run("cs_local", json!({ "id": id, "lang": "nl" }))["ok"], true);
    assert!(call("cs_nothing", &json!({}), &Quiet).is_err());
    assert_eq!(millis(Some("2026-10-16T10:00:00Z")), Some(1_792_144_800_000));
    assert_eq!(millis(Some("1792144800")), Some(1_792_144_800_000));
    assert_eq!(millis(Some("1792144800000")), Some(1_792_144_800_000));
}

/// Every text of the cloud saves - this file's, the page's (`online/cloud.js`) - in the seven
/// languages.
#[test]
fn every_cloud_text_is_translated() {
    let own = include_str!("cloud.rs").replace("\r\n", "\n");
    let own = own.split("#[cfg(test)]\n#[path").next().unwrap_or("").to_string();
    let js = include_str!("../../../ui/js/online/cloud.js").replace("\r\n", "\n");
    let mut texts: Vec<String> = Vec::new();
    for (src, lead, end) in [(own.as_str(), "tr(\"", '"'), (own.as_str(), "trf(\"", '"'), (js.as_str(), "tr('", '\'')] {
        texts.extend(online::texts_in(src, lead, end));
    }
    // (the state's labels, said through `state_label`)
    texts.extend(STATES.iter().map(|x| x.1.to_string()));
    assert!(texts.len() > 30, "only {} texts found", texts.len());
    let missing = online::untranslated(&texts);
    assert!(missing.is_empty(), "{} missing (crates/hub-locales/locales/cloud-saves.yml):\n{}", missing.len(), missing.join("\n"));
}
