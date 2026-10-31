//! The OMSI 2 content as openOMSI sees it: its content folder (mods, its own files) first, then
//! the OMSI 2 folder. A path is relative to the OMSI 2 folder (`maps/Grundorf/global.cfg`),
//! with `/` or `\`, in any case - OMSI's content names files carelessly, and on macOS and Linux
//! the case matters to the system. Reads only: Omsi-Hub never writes into these folders.

use crate::text::CfgFile;
use crate::ReadError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Two file names are the same name but for case (any letters, not only ASCII: `Grünau` is
/// `GRÜNAU`), as Windows takes them.
pub fn same_name(a: &str, b: &str) -> bool {
    a == b || a.eq_ignore_ascii_case(b) || (!a.is_ascii() && a.to_lowercase() == b.to_lowercase())
}

/// A path as OMSI's files write it (`Vehicles\Foo\BAR.bus`), with `/` (blanks around it, `.`
/// parts and doubled slashes taken out). Its case stays: [`resolve_in`] finds it in any.
pub fn normalize(rel: &str) -> String {
    rel.trim().split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".").collect::<Vec<_>>().join("/")
}

/// A file or folder of `dir` by its name, in any case: the exact name first, else the first
/// one equal but for case. The name comes back as it is on disk.
pub fn find_child(dir: &Path, name: &str) -> Option<PathBuf> {
    let listing = listing(dir)?;
    if let Some(&i) = listing.exact.get(name) {
        return Some(listing.entries[i].path.clone());
    }
    let alike = listing.folded.get(&name.to_lowercase()).into_iter().flatten();
    alike.map(|&i| &listing.entries[i]).find(|e| same_name(&e.name, name)).map(|e| e.path.clone()).or_else(|| {
        // (a name the listing cannot have but the system finds: Windows's short names, `PROGRA~1`,
        // and a name it ends with a dot or a blank, which it drops)
        let exact = dir.join(name);
        (name.contains('~') || name.ends_with(['.', ' '])).then_some(exact).filter(|p| p.exists())
    })
}

/// One name of a folder as [`listing`] read it.
#[derive(Debug)]
pub struct Entry {
    /// As it is on disk.
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// A folder's names, in the order the system gave them, with their indexes by the exact name and
/// by the name in lower case (every one of that name, in their order).
#[derive(Debug)]
pub struct Listing {
    pub entries: Vec<Entry>,
    exact: HashMap<String, usize>,
    folded: HashMap<String, Vec<usize>>,
}

/// A folder's names, kept while the folder is unchanged (its time of change the same): every
/// part of every path looked up in any case is its folder read, and one look at the buses or a
/// bus's textures reads `Vehicles` or a texture folder thousands of times over. A folder changed
/// in the last seconds is read again each time (a coarse clock, as FAT's, may not show the next
/// change). In a [`Pass`], a folder checked once is not checked again. None: `dir` is no folder.
pub fn listing(dir: &Path) -> Option<Arc<Listing>> {
    static KEPT: Mutex<Option<HashMap<PathBuf, (SystemTime, Arc<Listing>)>>> = Mutex::new(None);
    if checked_in_pass(dir) {
        let kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, l)) = kept.as_ref().and_then(|k| k.get(dir)) {
            return Some(l.clone());
        }
    }
    let changed = std::fs::metadata(dir).ok()?.modified().ok();
    if let Some(changed) = changed {
        let kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, l)) = kept.as_ref().and_then(|k| k.get(dir)) {
            if *at == changed {
                return Some(l.clone());
            }
        }
    }
    let mut entries = Vec::new();
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let path = e.path();
        // (the kind from the folder's own listing; a link followed to what it points at)
        let is_dir = match e.file_type() {
            Ok(t) if !t.is_symlink() => t.is_dir(),
            _ => path.is_dir(),
        };
        entries.push(Entry { name: e.file_name().to_string_lossy().into_owned(), path, is_dir });
    }
    let (mut exact, mut folded) = (HashMap::new(), HashMap::<String, Vec<usize>>::new());
    for (i, e) in entries.iter().enumerate() {
        exact.entry(e.name.clone()).or_insert(i);
        folded.entry(e.name.to_lowercase()).or_default().push(i);
    }
    let l = Arc::new(Listing { entries, exact, folded });
    let settled = changed.filter(|c| SystemTime::now().duration_since(*c).is_ok_and(|d| d >= SETTLED));
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    let kept = kept.get_or_insert_with(HashMap::new);
    match settled {
        Some(changed) => {
            if kept.len() >= KEPT_MAX {
                kept.clear();
            }
            kept.insert(dir.to_path_buf(), (changed, l.clone()));
        }
        // (nor an older listing of it left for a pass to take)
        None => {
            kept.remove(dir);
        }
    }
    Some(l)
}

/// A folder changed longer ago than this is kept by [`listing`].
const SETTLED: Duration = Duration::from_secs(3);
/// The folders [`listing`] keeps at most, and the files a [`Kept`] (past it, it starts again).
const KEPT_MAX: usize = 10_000;

thread_local! {
    /// The folders and files checked in this thread's [`Pass`] (None: none is on).
    static PASS: std::cell::RefCell<Option<std::collections::HashSet<PathBuf>>> = const { std::cell::RefCell::new(None) };
}

/// A look at the content that only reads (the buses listed, a bus opened in 3D, a map's lines):
/// while it lasts, a folder or file whose time of change was checked once on this thread is
/// taken as unchanged after, and not checked again. Each check is the system opening the file;
/// on a computer whose virus scanner looks at every one that is a tenth of a millisecond, and
/// one look at the buses checks the same folders tens of thousands of times. Nothing that writes
/// may run in one: what it wrote would not be seen. A pass begun inside another is part of it.
#[must_use]
pub struct Pass {
    outer: bool,
}

impl Pass {
    pub fn begin() -> Pass {
        PASS.with(|p| {
            let mut p = p.borrow_mut();
            let outer = p.is_none();
            if outer {
                *p = Some(Default::default());
            }
            Pass { outer }
        })
    }
}

impl Drop for Pass {
    fn drop(&mut self) {
        if self.outer {
            PASS.with(|p| *p.borrow_mut() = None);
        }
    }
}

/// `path` was checked before in this thread's [`Pass`]; it is marked checked from now on (false:
/// no pass is on, or this is its first check).
pub fn checked_in_pass(path: &Path) -> bool {
    PASS.with(|p| p.borrow_mut().as_mut().is_some_and(|set| !set.insert(path.to_path_buf())))
}

/// What was read of files, each kept while the file is unchanged (its time of change and size
/// the same as when it was read; in a [`Pass`], checked once): a `static` of a reader that reads
/// the same files over and over (a bus's paints, the depot files a bus folder shares).
pub struct Kept<T> {
    files: Mutex<Option<HashMap<PathBuf, (SystemTime, u64, T)>>>,
}

impl<T: Clone> Kept<T> {
    pub const fn new() -> Kept<T> {
        Kept { files: Mutex::new(None) }
    }

    /// What `read` makes of the file `path`: kept, or read now (None: not read).
    pub fn get(&self, path: &Path, read: impl FnOnce(&Path) -> Option<T>) -> Option<T> {
        let lock = || self.files.lock().unwrap_or_else(|e| e.into_inner());
        let found = |stamp: Option<(SystemTime, u64)>| -> Option<T> {
            let k = lock();
            let (at, len, v) = k.as_ref()?.get(path)?;
            stamp.is_none_or(|s| s == (*at, *len)).then(|| v.clone())
        };
        if checked_in_pass(path) {
            if let Some(v) = found(None) {
                return Some(v);
            }
        }
        let stamp = std::fs::metadata(path).ok().and_then(|m| Some((m.modified().ok()?, m.len())));
        if let Some(v) = stamp.and_then(|s| found(Some(s))) {
            return Some(v);
        }
        let v = read(path);
        let mut k = lock();
        let k = k.get_or_insert_with(HashMap::new);
        match (stamp, &v) {
            (Some((at, len)), Some(v)) => {
                if k.len() >= KEPT_MAX {
                    k.clear();
                }
                k.insert(path.to_path_buf(), (at, len, v.clone()));
            }
            // (nor what was read before left for a pass to take)
            _ => {
                k.remove(path);
            }
        }
        v
    }
}

impl<T: Clone> Default for Kept<T> {
    fn default() -> Self {
        Kept::new()
    }
}

/// `rel` under `base` (`/` or `\`), each part found in any case.
pub fn resolve_in(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut p = base.to_path_buf();
    for part in rel.trim().split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        p = if part == ".." { p.parent()?.to_path_buf() } else { find_child(&p, part)? };
    }
    Some(p)
}

/// What the OMSI 2 folder `root` lacks of an original install (empty: it is one). The game,
/// and the folders every map and bus needs.
pub fn missing_omsi2_essentials(root: &Path) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !find_child(root, "Omsi.exe").is_some_and(|p| p.is_file()) {
        missing.push("Omsi.exe");
    }
    for dir in ["maps", "Vehicles", "Sceneryobjects", "Splines"] {
        if !find_child(root, dir).is_some_and(|p| p.is_dir()) {
            missing.push(dir);
        }
    }
    missing
}

/// The content folders, in the order they are searched.
#[derive(Debug, Clone, PartialEq)]
pub struct Vfs {
    bases: Vec<PathBuf>,
}

impl Vfs {
    /// The OMSI 2 folder `root`, with openOMSI's content folder before it.
    pub fn new(root: PathBuf, content: Option<PathBuf>) -> Vfs {
        Vfs::layered(content.into_iter().collect(), root)
    }

    /// The OMSI 2 folder `root`, with the folders `first` before it, searched in their order
    /// (openOMSI's content folder, then the archives it uses in place).
    pub fn layered(first: Vec<PathBuf>, root: PathBuf) -> Vfs {
        let mut bases: Vec<PathBuf> = Vec::new();
        for b in first {
            if b != root && !bases.contains(&b) {
                bases.push(b);
            }
        }
        bases.push(root);
        Vfs { bases }
    }

    /// The OMSI 2 folder.
    pub fn root(&self) -> &Path {
        self.bases.last().expect("a Vfs has its root")
    }

    /// The folders searched, the content folder first.
    pub fn bases(&self) -> &[PathBuf] {
        &self.bases
    }

    /// The file or folder `rel`, from the first folder that has it.
    pub fn resolve(&self, rel: &str) -> Option<PathBuf> {
        self.bases.iter().find_map(|b| resolve_in(b, rel))
    }

    pub fn is_file(&self, rel: &str) -> bool {
        self.resolve(rel).is_some_and(|p| p.is_file())
    }

    pub fn is_dir(&self, rel: &str) -> bool {
        self.bases.iter().any(|b| resolve_in(b, rel).is_some_and(|p| p.is_dir()))
    }

    pub fn read(&self, rel: &str) -> Result<Vec<u8>, ReadError> {
        let p = self.resolve(rel).ok_or_else(|| ReadError::NotFound(rel.to_string()))?;
        std::fs::read(&p).map_err(|e| ReadError::io(p, e))
    }

    /// A configuration file, decoded in its codepage.
    pub fn read_cfg(&self, rel: &str) -> Result<CfgFile, ReadError> {
        let p = self.resolve(rel).ok_or_else(|| ReadError::NotFound(rel.to_string()))?;
        CfgFile::read(p)
    }

    /// The names in the folder `rel` of every content folder, each once (in any case: the
    /// first folder's spelling), with whether it is a folder; sorted by name.
    pub fn list_dir(&self, rel: &str) -> Vec<(String, bool)> {
        let mut out: Vec<(String, bool)> = Vec::new();
        for b in &self.bases {
            let Some(entries) = resolve_in(b, rel).and_then(|d| listing(&d)) else { continue };
            for e in &entries.entries {
                if !out.iter().any(|(n, _)| same_name(n, &e.name)) {
                    out.push((e.name.clone(), e.is_dir));
                }
            }
        }
        out.sort_by_key(|(n, _)| n.to_lowercase());
        out
    }

    /// `path` (inside one of the folders) relative to the OMSI 2 folder's layout, with `/`.
    pub fn relative(&self, path: &Path) -> Option<String> {
        self.bases.iter().find_map(|b| path.strip_prefix(b).ok()).map(|r| r.to_string_lossy().replace('\\', "/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hub-omsi-vfs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_content_folder_comes_first_in_any_case() {
        let root = dir("root");
        let content = dir("content");
        std::fs::create_dir_all(root.join("Maps/Grundorf")).unwrap();
        std::fs::write(root.join("Maps/Grundorf/global.cfg"), "[name]\nroot").unwrap();
        std::fs::write(root.join("Maps/Grundorf/Holidays.txt"), "").unwrap();
        std::fs::create_dir_all(content.join("maps/grundorf")).unwrap();
        std::fs::write(content.join("maps/grundorf/global.cfg"), "[name]\ncontent").unwrap();
        let vfs = Vfs::new(root.clone(), Some(content.clone()));
        assert_eq!(vfs.root(), root.as_path());
        assert_eq!(vfs.read_cfg("maps\\Grundorf\\GLOBAL.cfg").unwrap().value("name"), Some("content"));
        assert!(vfs.is_file("maps/grundorf/holidays.txt"));
        assert!(vfs.is_dir("MAPS"));
        let names: Vec<String> = vfs.list_dir("maps/Grundorf").into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["global.cfg", "Holidays.txt"]);
        assert!(matches!(vfs.read("maps/none.cfg"), Err(ReadError::NotFound(_))));
        let p = vfs.resolve("maps/Grundorf/Holidays.txt").unwrap();
        assert_eq!(vfs.relative(&p).unwrap(), "Maps/Grundorf/Holidays.txt");
    }

    #[test]
    fn layered_folders_are_searched_in_their_order() {
        let root = dir("layered-root");
        let content = dir("layered-content");
        let archive = dir("layered-archive");
        for (b, name) in [(&root, "root"), (&archive, "archive")] {
            std::fs::create_dir_all(b.join("Vehicles/Bus")).unwrap();
            std::fs::write(b.join("Vehicles/Bus/bus.bus"), name).unwrap();
        }
        std::fs::create_dir_all(archive.join("Vehicles/Big")).unwrap();
        let vfs = Vfs::layered(vec![content.clone(), archive.clone(), content.clone(), root.clone()], root.clone());
        assert_eq!(vfs.bases(), [content, archive.clone(), root.clone()]);
        assert_eq!(vfs.read("Vehicles/Bus/bus.bus").unwrap(), b"archive");
        assert_eq!(vfs.list_dir("Vehicles").into_iter().map(|(n, _)| n).collect::<Vec<_>>(), ["Big", "Bus"]);
        assert_eq!(vfs.root(), root.as_path());
    }

    #[test]
    fn what_is_kept_of_a_file_is_read_again_once_it_changed_but_not_in_a_pass() {
        let d = dir("kept");
        let f = d.join("paint.cti");
        std::fs::write(&f, "a").unwrap();
        let kept: Kept<String> = Kept::new();
        let reads = std::cell::Cell::new(0);
        let get = || kept.get(&f, |p| (reads.set(reads.get() + 1), std::fs::read_to_string(p).ok()).1);
        assert_eq!((get().as_deref(), get().as_deref(), reads.get()), (Some("a"), Some("a"), 1));
        std::fs::write(&f, "bb").unwrap();
        assert_eq!((get().as_deref(), reads.get()), (Some("bb"), 2));
        // (in a pass, checked once: a change after it is not seen until the pass is over)
        {
            let _pass = Pass::begin();
            assert_eq!(get().as_deref(), Some("bb"));
            std::fs::write(&f, "ccc").unwrap();
            assert_eq!((get().as_deref(), reads.get()), (Some("bb"), 2));
        }
        assert_eq!((get().as_deref(), reads.get()), (Some("ccc"), 3));
        // a file gone is no longer kept
        std::fs::remove_file(&f).unwrap();
        assert_eq!(get(), None);
    }

    #[test]
    fn a_name_is_found_in_any_case_in_a_folder_just_changed() {
        let d = dir("listing");
        std::fs::write(d.join("Model.CFG"), "").unwrap();
        assert_eq!(find_child(&d, "model.cfg"), Some(d.join("Model.CFG")));
        std::fs::write(d.join("new.cti"), "").unwrap();
        assert_eq!(find_child(&d, "NEW.cti"), Some(d.join("new.cti")));
        assert!(find_child(&d, "none.cti").is_none());
        assert_eq!(listing(&d).unwrap().entries.len(), 2);
    }

    #[test]
    fn names_compare_in_any_case_and_paths_get_slashes() {
        assert!(same_name("TTData", "ttdata") && same_name("Grünau.HOF", "GRÜNAU.hof") && !same_name("a.cfg", "b.cfg"));
        assert_eq!(normalize(" Vehicles\\Foo\\\\.\\BAR.bus "), "Vehicles/Foo/BAR.bus");
        assert_eq!(normalize("model/model.cfg"), "model/model.cfg");
    }

    #[test]
    fn an_omsi2_folder_is_told_by_its_essentials() {
        let root = dir("omsi2");
        assert_eq!(missing_omsi2_essentials(&root), ["Omsi.exe", "maps", "Vehicles", "Sceneryobjects", "Splines"]);
        std::fs::write(root.join("omsi.EXE"), b"MZ").unwrap();
        for d in ["Maps", "vehicles", "Sceneryobjects", "Splines"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        assert!(missing_omsi2_essentials(&root).is_empty());
    }
}
