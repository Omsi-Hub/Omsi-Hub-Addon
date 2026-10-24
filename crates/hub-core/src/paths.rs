//! Where things are: Omsi-Hub's own data folder, and the player's folders.
//!
//! **The data folder** is Omsi-Hub's alone, under the system's app data folder:
//! `%APPDATA%\Omsi-Hub` on Windows, `~/Library/Application Support/Omsi-Hub` on macOS,
//! `$XDG_DATA_HOME/Omsi-Hub` (`~/.local/share/Omsi-Hub`) on Linux. Never `~/.openomsi`, which is
//! openOMSI's (the old Omsi-Hub's saves there are imported once, `migrate`). The environment
//! variable `OMSIHUB_DATA_DIR` puts it elsewhere (tests, a portable install).
//!
//! **The player's folders** - the openOMSI folder and the OMSI 2 folder - come from the
//! settings (`settings.json`, chosen in the first-run setup); the openOMSI folder is the one
//! openOMSI's own launcher plays when it names one ([`openomsi_folder`]). Omsi-Hub reads the
//! OMSI 2 content through them ([`vfs`]: the content folder first, then the archives openOMSI
//! uses in place - what Omsi-Hub reads of them copied into its cache, [`archive_bases`] - then
//! the OMSI 2 folder) and never writes there.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The environment variable that moves the data folder.
pub const DATA_DIR_ENV: &str = "OMSIHUB_DATA_DIR";
/// The data folder's name under the system's app data folder.
pub const APP_FOLDER: &str = "Omsi-Hub";

/// The folder the data folder would be in, without the override.
pub fn system_data_dir() -> PathBuf {
    // (Roaming AppData on Windows, Application Support on macOS, XDG_DATA_HOME on Linux)
    #[cfg(feature = "native")]
    return dirs::data_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default()).join(APP_FOLDER);
    // (the website: the folder the page lays out in the browser, the cloud save's files in it)
    #[cfg(not(feature = "native"))]
    PathBuf::from("/data")
}

/// Omsi-Hub's data folder, made when it is first asked for.
pub fn data_dir() -> PathBuf {
    static MADE: Mutex<Option<PathBuf>> = Mutex::new(None);
    let d = std::env::var_os(DATA_DIR_ENV).filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(system_data_dir);
    let mut made = MADE.lock().unwrap_or_else(|e| e.into_inner());
    if made.as_ref() != Some(&d) && std::fs::create_dir_all(&d).is_ok() {
        *made = Some(d.clone());
    }
    d
}

/// The environment variable that puts the home folder elsewhere (the tests: a home of their
/// own, so that the computer's own openOMSI, its `launcher.json`, is not followed).
pub const HOME_DIR_ENV: &str = "OMSIHUB_HOME_DIR";

/// The user's home folder (where openOMSI keeps `~/.openomsi`).
pub fn home_dir() -> PathBuf {
    if let Some(h) = std::env::var_os(HOME_DIR_ENV).filter(|v| !v.is_empty()) {
        return PathBuf::from(h);
    }
    #[cfg(feature = "native")]
    return dirs::home_dir().unwrap_or_default();
    #[cfg(not(feature = "native"))]
    PathBuf::from("/home")
}

/// The OMSI 2 folder from the settings, when it is set and there.
pub fn root() -> Option<PathBuf> {
    let s = crate::settings::Settings::load();
    Some(PathBuf::from(s.omsi2_folder)).filter(|p| !p.as_os_str().is_empty() && p.is_dir())
}

/// openOMSI's content folder for the openOMSI in use (where it reads mods and its plugins
/// from), when there is one. See [`openomsi_folder`] and [`content_dir_of`].
pub fn content_dir() -> Option<PathBuf> {
    content_dir_of(&openomsi_folder()?, &home_dir())
}

/// The folder of the openOMSI in use: the one openOMSI's own launcher plays
/// ([`launcher_openomsi`]) - its mods are the ones the player sees in openOMSI, and a player
/// often has more than one openOMSI (each release unpacked into a folder of its own, each with
/// its own content folder) - else the one chosen in the setup.
pub fn openomsi_folder() -> Option<PathBuf> {
    openomsi_folder_of(&crate::settings::Settings::load().openomsi_folder, &home_dir())
}

/// [`openomsi_folder`] for the folder `chosen` in the setup and the home folder `home`.
pub fn openomsi_folder_of(chosen: &str, home: &Path) -> Option<PathBuf> {
    launcher_openomsi(home).or_else(|| Some(PathBuf::from(chosen.trim())).filter(|p| !p.as_os_str().is_empty()))
}

/// The folder of the openOMSI openOMSI's own launcher plays: its `game` in
/// `~/.openomsi/launcher.json`, when that is an openOMSI program that is there (the launcher
/// takes no program of another name) - the folder holding the bundle on macOS.
pub fn launcher_openomsi(home: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(home.join(".openomsi").join("launcher.json")).ok()?;
    let config: serde_json::Value = serde_json::from_str(&text).ok()?;
    let game = PathBuf::from(config.get("game")?.as_str()?.trim());
    let named = game.file_stem().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case("openomsi"));
    if !named || !game.is_file() {
        return None;
    }
    let dir = game.parent()?;
    if dir.ends_with("Contents/MacOS") {
        return Some(dir.parent()?.parent()?.parent()?.to_path_buf());
    }
    Some(dir.to_path_buf())
}

/// The content folder of the openOMSI in `folder`, by openOMSI's own rule, the same on
/// Windows, macOS and Linux: the folder of its program (beside the `.app` bundle on macOS), or
/// its `openOMSI` folder when it was unpacked into the OMSI 2 folder - unless that may not be
/// written to (Program Files, `/opt`, `/Applications`), when it is `~/.openomsi/content`.
///
/// Omsi-Hub writes nothing to find it: the folder beside the program counts when openOMSI has
/// used it (its `Plugins` or `Mods` is there) or when there is no `~/.openomsi/content`.
/// (hub-game's `detect` makes the same choice by a write test when it installs the plugin.)
/// `folder` may be the bundle itself, as the setup takes it.
pub fn content_dir_of(folder: &Path, home: &Path) -> Option<PathBuf> {
    use hub_omsi::vfs::find_child;
    let is_bundle = |p: &Path| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("app")) && p.join("Contents/MacOS").is_dir();
    let folder = if is_bundle(folder) { folder.parent()? } else { folder };
    let bundle = std::fs::read_dir(folder).ok()?.flatten().map(|e| e.path()).any(|p| is_bundle(&p));
    // (Omsi-Hub's own build of the game by its name too, as the setup finds it)
    let program = ["openomsi.exe", "openomsi", "omsi-hub-game.exe", "omsi-hub-game"].iter().any(|n| find_child(folder, n).is_some_and(|p| p.is_file()));
    if !program && !bundle {
        return None;
    }
    // (openOMSI unpacked into the original OMSI 2 folder: the game and its maps are there)
    let omsi2 = find_child(folder, "Omsi.exe").is_some() && find_child(folder, "maps").is_some();
    let beside = if omsi2 { folder.join("openOMSI") } else { folder.to_path_buf() };
    let fallback = home.join(".openomsi").join("content");
    let used = ["Plugins", "Mods"].iter().any(|n| find_child(&beside, n).is_some_and(|p| p.is_dir()));
    if used || (beside.is_dir() && !fallback.is_dir()) {
        Some(beside)
    } else {
        Some(fallback).filter(|f| f.is_dir())
    }
}

/// `rel` inside `base` (either slash), each part found in any case as far as it is there, the
/// rest as `rel` names it: the path of a file or folder that is there, or of one to be made
/// beside what is there (so that on a case-sensitive file system a `Vehicles` folder is not
/// made again beside a `vehicles` one). (The old `omsi_cfg::resolve_path`.)
pub fn resolve_path(base: &Path, rel: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    let mut found = true;
    for part in rel.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        if part == ".." {
            p.pop();
            continue;
        }
        let next = if found { hub_omsi::vfs::find_child(&p, part) } else { None };
        found = next.is_some();
        p = next.unwrap_or_else(|| p.join(part));
    }
    p
}

/// The names in the folder `dir` with whether each is a folder (None: no folder that reads).
pub fn list_dir(dir: &Path) -> Option<Vec<(std::ffi::OsString, bool)>> {
    let entries = std::fs::read_dir(dir).ok()?;
    Some(entries.flatten().map(|e| (e.file_name(), e.path().is_dir())).collect())
}

/// The paths of what is in the folder `dir` (empty: none, or no folder).
pub fn read_dir_paths(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default()
}

/// The content folders, the content folder first, then the OMSI 2 folder (empty when the
/// OMSI 2 folder is not set).
pub fn bases() -> Vec<PathBuf> {
    vfs().map(|v| v.bases().to_vec()).unwrap_or_default()
}

/// Every folder of `rel` (`Vehicles`) across the content folders, by its name (in any case,
/// the first folder's spelling), with each content folder's copy of it: the content folder's
/// first. Sorted by name.
pub fn merged_folders(rel: &str) -> Vec<(String, Vec<PathBuf>)> {
    let bases = bases();
    let mut out: Vec<(String, Vec<PathBuf>)> = Vec::new();
    for b in &bases {
        let Some(dir) = hub_omsi::vfs::resolve_in(b, rel).filter(|d| d.is_dir()) else { continue };
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for e in entries.flatten().filter(|e| e.path().is_dir()) {
            let name = e.file_name().to_string_lossy().into_owned();
            match out.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&name)) {
                Some((_, copies)) => copies.push(e.path()),
                None => out.push((name, vec![e.path()])),
            }
        }
    }
    out.sort_by_key(|(n, _)| n.to_lowercase());
    out
}

/// The OMSI 2 content as openOMSI sees it: the content folder first, then the archives it uses
/// in place ([`archive_bases`]), then the OMSI 2 folder.
pub fn vfs() -> Option<hub_omsi::vfs::Vfs> {
    let root = root()?;
    let content = content_dir();
    let mut first: Vec<PathBuf> = content.iter().cloned().collect();
    first.extend(content.as_deref().map(archive_bases).unwrap_or_default());
    Some(hub_omsi::vfs::Vfs::layered(first, root))
}

/// The folder of openOMSI's content folder that holds the archives it uses in place: a mod
/// too big to unpack is kept there as a `.zip`, and the game reads it as a folder.
pub const ARCHIVES: &str = "Archives";

/// The archives openOMSI uses in place in the content folder `content`, in their names' order
/// (as openOMSI mounts them), each as a folder of Omsi-Hub's own cache holding what Omsi-Hub
/// reads of it ([`archive_copy`]). An archive that cannot be read is left out (and logged).
pub fn archive_bases(content: &Path) -> Vec<PathBuf> {
    let Some(dir) = hub_omsi::vfs::find_child(content, ARCHIVES).filter(|d| d.is_dir()) else { return Vec::new() };
    let mut zips: Vec<PathBuf> = read_dir_paths(&dir).into_iter().filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"))).collect();
    zips.sort();
    let cache = data_dir().join("cache").join("archives");
    zips.iter().filter_map(|z| archive_copy(z, &cache)).collect()
}

#[cfg(feature = "native")]
/// The content folders of an archive Omsi-Hub reads: its buses, maps, depots, fonts, drivers and
/// weather - not the scenery, splines, textures and sounds, which only the game and the 3D view
/// (through openOMSI's own readers) read.
const ARCHIVE_FOLDERS: [&str; 8] = ["Vehicles", "maps", "HOFs", "Fonts", "Drivers", "Weather", "TicketPacks", "Announcements"];

#[cfg(feature = "native")]
/// Files of those folders Omsi-Hub never reads: models, pictures, sounds, a map's tiles.
const ARCHIVE_SKIPPED: [&str; 16] = ["o3d", "x", "dds", "bmp", "jpg", "jpeg", "png", "tga", "wav", "ogg", "mp3", "map", "rdy", "dll", "exe", "pdf"];

#[cfg(feature = "native")]
/// The largest file taken out of an archive (a text file is far smaller).
const ARCHIVE_MAX_FILE: u64 = 16 << 20;

#[cfg(feature = "native")]
/// Whether the file `rel` of an archive (`/`, from the content folder's top), `size` bytes, is
/// one Omsi-Hub reads: in one of [`ARCHIVE_FOLDERS`], not one of [`ARCHIVE_SKIPPED`] - but for
/// a map's picture and a font's bitmap.
fn archive_reads(rel: &str, size: u64) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    let (Some(top), Some(name)) = (parts.first(), parts.last()) else { return false };
    if parts.len() < 2 || size > ARCHIVE_MAX_FILE || !ARCHIVE_FOLDERS.iter().any(|f| f.eq_ignore_ascii_case(top)) {
        return false;
    }
    let lower = name.to_lowercase();
    let ext = lower.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
    let picture = top.eq_ignore_ascii_case("maps") && parts.len() == 3 && lower.starts_with("picture.");
    let font = top.eq_ignore_ascii_case("Fonts") && ext == "bmp";
    picture || font || !ARCHIVE_SKIPPED.contains(&ext)
}

/// The cache's copy of the archive `zip` in `cache`: `<name>-<stamp>`, the stamp from the
/// archive's path, size and time, so that a changed archive is taken out again (and the copy
/// of the archive before it removed). Made beside its place first and put there whole.
fn archive_copy(zip: &Path, cache: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(zip).ok()?;
    let time = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or_default();
    let stem = zip.file_stem()?.to_string_lossy().into_owned();
    // (FNV-1a: the same stamp in every version of Omsi-Hub)
    let stamp = format!("{}|{}|{time}", zip.display(), meta.len()).bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    let copy = cache.join(format!("{stem}-{stamp:016x}"));
    if copy.is_dir() {
        return Some(copy);
    }
    // (one archive taken out at a time: two commands asking at once do not write the same copy)
    static TAKING: Mutex<()> = Mutex::new(());
    let _taking = TAKING.lock().unwrap_or_else(|e| e.into_inner());
    if copy.is_dir() {
        return Some(copy);
    }
    // (an older copy of this archive: `<name>-` and a stamp)
    for old in read_dir_paths(cache) {
        let name = old.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.strip_prefix(&format!("{stem}-")).is_some_and(|s| s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit())) {
            let _ = std::fs::remove_dir_all(&old);
        }
    }
    let part = cache.join(format!("{stem}-{stamp:016x}.part"));
    let _ = std::fs::remove_dir_all(&part);
    let made = std::fs::create_dir_all(&part).map_err(|e| e.to_string()).and_then(|()| unpack_archive(zip, &part)).and_then(|n| std::fs::rename(&part, &copy).map(|()| n).map_err(|e| e.to_string()));
    match made {
        Ok(n) => {
            crate::log_line(&format!("archive {}: {n} files read into {}", zip.display(), copy.display()));
            Some(copy)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&part);
            crate::log_line(&format!("archive {}: {e}", zip.display()));
            None
        }
    }
}

/// The files of the archive `zip` Omsi-Hub reads ([`archive_reads`]) written into `to`, laid out
/// as openOMSI mounts it (from the archive's folder holding the content folders, often
/// `OMSI 2/`): how many.
#[cfg(feature = "native")]
fn unpack_archive(zip: &Path, to: &Path) -> Result<usize, String> {
    // (openOMSI's own reader, mounted once a run: an archive put in the place of another while
    // Omsi-Hub runs reads wrong, is left out and logged, and is read at the next start)
    let mount = omsi_cfg::vfs::mount_zip(zip).map_err(|e| e.to_string())?;
    let mut n = 0;
    let mut todo = vec![(mount, String::new())];
    while let Some((dir, rel)) = todo.pop() {
        for (name, is_dir) in omsi_cfg::vfs::list_dir(&dir).unwrap_or_default() {
            let name = name.to_string_lossy().into_owned();
            // (a name is one part: nothing written outside `to`)
            if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':']) {
                continue;
            }
            let rel = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let path = dir.join(&name);
            if is_dir {
                todo.push((path, rel));
                continue;
            }
            if !archive_reads(&rel, omsi_cfg::vfs::file_size(&path).unwrap_or(u64::MAX)) {
                continue;
            }
            let bytes = omsi_cfg::vfs::read(&path).map_err(|e| format!("{rel}: {e}"))?;
            let out = to.join(&rel);
            std::fs::create_dir_all(out.parent().unwrap_or(to)).and_then(|()| std::fs::write(&out, bytes)).map_err(|e| format!("{}: {e}", out.display()))?;
            n += 1;
        }
    }
    Ok(n)
}

/// (the website: no archives)
#[cfg(not(feature = "native"))]
fn unpack_archive(_zip: &Path, _to: &Path) -> Result<usize, String> {
    Err("archives are read by the desktop app".into())
}

/// A content file by its path relative to the OMSI 2 folder (`maps/Grundorf/global.cfg`,
/// either slash, any case), found in the content folder first, then in the OMSI 2 folder.
pub fn resolve_content(rel: &str) -> Option<PathBuf> {
    vfs()?.resolve(rel)
}

/// The OMSI 2 folder of this computer, for the tests that read real content (only read; they
/// pass over what is not there): `OMSI2_DIR`, else Steam's own place - on Windows, and on Linux
/// where Steam (native or Flatpak) runs OMSI 2 through Proton.
#[cfg(test)]
pub(crate) fn installed_omsi2() -> Option<PathBuf> {
    let home = home_dir();
    let steam = |base: PathBuf| base.join("steamapps").join("common").join("OMSI 2");
    let mut tries: Vec<PathBuf> = std::env::var_os("OMSI2_DIR").map(PathBuf::from).into_iter().collect();
    if cfg!(windows) {
        tries.push(steam(PathBuf::from("C:/Program Files (x86)/Steam")));
    } else {
        tries.push(steam(home.join(".steam").join("steam")));
        tries.push(steam(home.join(".local").join("share").join("Steam")));
        tries.push(steam(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")));
    }
    tries.into_iter().find(|p| p.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hub-core-paths-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn openomsi_s_content_folder_follows_its_rule_on_every_system() {
        let home = dir("home");
        // a Linux release unpacked in the home folder: its own folder
        let linux = home.join("openOMSI-0.2.30-linux-x64");
        std::fs::create_dir_all(&linux).unwrap();
        std::fs::write(linux.join("openomsi"), b"").unwrap();
        assert_eq!(content_dir_of(&linux, &home), Some(linux.clone()));
        // once openOMSI went to ~/.openomsi/content (its folder could not be written), that one
        std::fs::create_dir_all(home.join(".openomsi/content/Plugins")).unwrap();
        assert_eq!(content_dir_of(&linux, &home), Some(home.join(".openomsi/content")));
        // ... unless it used its own folder
        std::fs::create_dir_all(linux.join("Plugins")).unwrap();
        assert_eq!(content_dir_of(&linux, &home), Some(linux.clone()));
        // inside the OMSI 2 folder: its openOMSI folder
        let omsi2 = dir("omsi2");
        std::fs::write(omsi2.join("Omsi.exe"), b"").unwrap();
        std::fs::create_dir_all(omsi2.join("maps")).unwrap();
        std::fs::write(omsi2.join("openomsi.exe"), b"").unwrap();
        std::fs::create_dir_all(omsi2.join("openOMSI/Mods")).unwrap();
        assert_eq!(content_dir_of(&omsi2, &home), Some(omsi2.join("openOMSI")));
        // macOS: the folder holding the bundle, also when the bundle itself was chosen
        let apps = dir("apps");
        std::fs::create_dir_all(apps.join("openOMSI.app/Contents/MacOS")).unwrap();
        std::fs::create_dir_all(apps.join("Mods")).unwrap();
        assert_eq!(content_dir_of(&apps, &home), Some(apps.clone()));
        assert_eq!(content_dir_of(&apps.join("openOMSI.app"), &home), Some(apps.clone()));
        // no openOMSI at all
        assert_eq!(content_dir_of(&home, &home), None);
    }

    /// A zip archive of `files`, stored (as the zip tools write the files they cannot shrink).
    #[cfg(feature = "native")]
    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        let crc32 = |b: &[u8]| {
            let mut c = !0u32;
            for &x in b {
                c ^= x as u32;
                for _ in 0..8 {
                    c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
                }
            }
            !c
        };
        let (mut out, mut cd) = (Vec::new(), Vec::new());
        for (name, data) in files {
            let (at, crc, len, n) = (out.len() as u32, crc32(data), data.len() as u32, name.len() as u16);
            // local header: signature, version, flags, method, time and date, crc, sizes, name, extra
            out.extend(0x0403_4b50u32.to_le_bytes());
            out.extend([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(crc.to_le_bytes().into_iter().chain(len.to_le_bytes()).chain(len.to_le_bytes()).chain(n.to_le_bytes()).chain([0, 0]));
            out.extend(name.as_bytes());
            out.extend(*data);
            // central header: the same, then comment, disk, attributes and the local header's place
            cd.extend(0x0201_4b50u32.to_le_bytes());
            cd.extend([20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            cd.extend(crc.to_le_bytes().into_iter().chain(len.to_le_bytes()).chain(len.to_le_bytes()).chain(n.to_le_bytes()));
            cd.extend([0u8; 12]);
            cd.extend(at.to_le_bytes());
            cd.extend(name.as_bytes());
        }
        let (cd_at, count) = (out.len() as u32, files.len() as u16);
        out.extend(&cd);
        out.extend(0x0605_4b50u32.to_le_bytes());
        out.extend([0, 0, 0, 0]);
        out.extend(count.to_le_bytes().into_iter().chain(count.to_le_bytes()).chain((cd.len() as u32).to_le_bytes()).chain(cd_at.to_le_bytes()).chain([0, 0]));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, out).unwrap();
    }

    #[cfg(feature = "native")]
    #[test]
    fn what_omsi_hub_reads_of_an_archive() {
        assert!(archive_reads("Vehicles/Big Bus/big.bus", 100));
        assert!(archive_reads("vehicles/Big Bus/model/model.cfg", 100));
        assert!(archive_reads("maps/Big/TTData/1.ttl", 100));
        assert!(archive_reads("maps/Big/Picture.JPG", 100));
        assert!(archive_reads("Fonts/a.bmp", 100) && archive_reads("HOFs/a.hof", 100));
        assert!(!archive_reads("maps/Big/texture/picture.jpg", 100));
        assert!(!archive_reads("maps/Big/tile_0_0.map", 100));
        assert!(!archive_reads("Vehicles/Big Bus/Texture/body.dds", 100));
        assert!(!archive_reads("Vehicles/Big Bus/model/body.o3d", 100));
        assert!(!archive_reads("Sceneryobjects/a/a.sco", 100) && !archive_reads("readme.txt", 100));
        assert!(!archive_reads("Vehicles/Big Bus/big.bus", ARCHIVE_MAX_FILE + 1));
    }

    /// An archive openOMSI uses in place is read from the cache's copy of what Omsi-Hub reads of
    /// it, laid out from its folder holding the content folders; taken out once.
    #[cfg(feature = "native")]
    #[test]
    fn an_archive_used_in_place_is_read_from_the_cache() {
        let base = dir("archive");
        let zip = base.join("content/Archives/Big Map.zip");
        let files: [(&str, &[u8]); 9] = [
            ("Readme.txt", b"read me"),
            ("OMSI 2/maps/Big/global.cfg", b"[name]\r\nBig\r\n"),
            ("OMSI 2/maps/Big/picture.jpg", b"\xff\xd8"),
            ("OMSI 2/maps/Big/tile_0_0.map", b"tile"),
            ("OMSI 2/maps/Big/texture/road.dds", b"DDS "),
            ("OMSI 2/Vehicles/Big Bus/big.bus", b"[friendlyname]\r\nBig\r\n"),
            ("OMSI 2/Vehicles/Big Bus/Texture/body.bmp", b"BM"),
            ("OMSI 2/Sceneryobjects/a/a.sco", b"sco"),
            ("OMSI 2/Fonts/f.bmp", b"BM"),
        ];
        write_zip(&zip, &files);
        let cache = base.join("cache");
        let copy = archive_copy(&zip, &cache).unwrap();
        assert!(copy.file_name().unwrap().to_string_lossy().starts_with("Big Map-"));
        for f in ["maps/Big/global.cfg", "maps/Big/picture.jpg", "Vehicles/Big Bus/big.bus", "Fonts/f.bmp"] {
            assert!(copy.join(f).is_file(), "{f}");
        }
        for f in ["maps/Big/tile_0_0.map", "maps/Big/texture", "Vehicles/Big Bus/Texture", "Sceneryobjects", "Readme.txt", "OMSI 2"] {
            assert!(!copy.join(f).exists(), "{f}");
        }
        assert_eq!(std::fs::read(copy.join("maps/Big/global.cfg")).unwrap(), b"[name]\r\nBig\r\n");
        // asked again: the same copy, nothing left half-made
        assert_eq!(archive_copy(&zip, &cache), Some(copy.clone()));
        assert_eq!(read_dir_paths(&cache).len(), 1);
        // in the lists between the content folder and the OMSI 2 folder
        let root = dir("archive-root");
        let vfs = hub_omsi::vfs::Vfs::layered(vec![base.join("content"), copy.clone()], root.clone());
        assert!(vfs.is_file("maps/big/global.cfg") && vfs.is_dir("Vehicles/Big Bus"));
        // another archive of that name (put in the place of this one): taken out anew, the old copy gone
        let other = base.join("other/Big Map.zip");
        write_zip(&other, &files[..2]);
        let again = archive_copy(&other, &cache).unwrap();
        assert_ne!(again, copy);
        assert!(!copy.exists() && again.join("maps/Big/global.cfg").is_file());
        // not an archive: left out, nothing left behind
        let broken = base.join("content/Archives/Broken.zip");
        std::fs::write(&broken, b"not a zip").unwrap();
        assert_eq!(archive_copy(&broken, &cache), None);
        assert_eq!(read_dir_paths(&cache), [again]);
    }

    /// The openOMSI in use is the one openOMSI's own launcher plays, when its `launcher.json`
    /// names one that is there; else the one chosen in the setup.
    #[test]
    fn the_openomsi_openomsi_s_launcher_plays_is_followed() {
        let home = dir("launcher-home");
        let chosen = dir("launcher-chosen");
        let played = dir("launcher-played");
        let exe = played.join(if cfg!(windows) { "openomsi.exe" } else { "openomsi" });
        std::fs::write(&exe, b"").unwrap();
        let chosen_s = chosen.to_string_lossy().into_owned();
        // no launcher.json: the chosen folder
        assert_eq!(launcher_openomsi(&home), None);
        assert_eq!(openomsi_folder_of(&chosen_s, &home), Some(chosen.clone()));
        assert_eq!(openomsi_folder_of(" ", &home), None);
        let write = |game: &Path| {
            std::fs::create_dir_all(home.join(".openomsi")).unwrap();
            let text = serde_json::json!({ "root": "C:/OMSI 2", "game": game.to_string_lossy() }).to_string();
            std::fs::write(home.join(".openomsi/launcher.json"), text).unwrap();
        };
        // the game the launcher plays: its folder
        write(&exe);
        assert_eq!(launcher_openomsi(&home), Some(played.clone()));
        assert_eq!(openomsi_folder_of(&chosen_s, &home), Some(played.clone()));
        assert_eq!(openomsi_folder_of("", &home), Some(played.clone()));
        // a game that is not there (renamed, removed), or of another name: the chosen folder
        write(&played.join("gone").join("openomsi.exe"));
        assert_eq!(openomsi_folder_of(&chosen_s, &home), Some(chosen.clone()));
        std::fs::write(played.join("omsi.exe"), b"").unwrap();
        write(&played.join("omsi.exe"));
        assert_eq!(launcher_openomsi(&home), None);
        // a broken launcher.json
        std::fs::write(home.join(".openomsi/launcher.json"), "{ not json").unwrap();
        assert_eq!(openomsi_folder_of(&chosen_s, &home), Some(chosen.clone()));
        // macOS: the folder holding the bundle
        let macos = played.join("openOMSI.app/Contents/MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::write(macos.join("openomsi"), b"").unwrap();
        write(&macos.join("openomsi"));
        assert_eq!(launcher_openomsi(&home), Some(played.clone()));
    }

    #[test]
    fn the_data_folder_is_omsi_hub_s_own() {
        let d = system_data_dir();
        assert_eq!(d.file_name().unwrap(), APP_FOLDER);
        assert!(!d.to_string_lossy().contains(".openomsi"));
        if cfg!(target_os = "linux") {
            // ($XDG_DATA_HOME, else ~/.local/share)
            let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home_dir().join(".local/share"));
            assert_eq!(d, xdg.join(APP_FOLDER));
        }
    }

    /// A path is found in any case as far as it is there; the rest is as asked, one folder a
    /// part whichever slash it was written with (no `\` in a name on Linux and macOS).
    #[test]
    fn a_path_keeps_the_case_of_what_is_there() {
        let base = std::env::temp_dir().join(format!("omsihub-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("vehicles").join("MAN_SD202")).unwrap();
        std::fs::write(base.join("vehicles").join("MAN_SD202").join("SD202.BUS"), "").unwrap();
        assert_eq!(resolve_path(&base, "Vehicles\\man_sd202\\sd202.bus"), base.join("vehicles").join("MAN_SD202").join("SD202.BUS"));
        assert_eq!(resolve_path(&base, "VEHICLES/New/Folder"), base.join("vehicles").join("New").join("Folder"));
        assert_eq!(resolve_path(&base, "maps\\Grundorf/TTData"), base.join("maps").join("Grundorf").join("TTData"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
