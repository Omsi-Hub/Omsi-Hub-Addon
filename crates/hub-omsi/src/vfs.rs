//! The OMSI 2 content as openOMSI sees it: its content folder (mods, its own files) first, then
//! the OMSI 2 folder. A path is relative to the OMSI 2 folder (`maps/Grundorf/global.cfg`),
//! with `/` or `\`, in any case - OMSI's content names files carelessly, and on macOS and Linux
//! the case matters to the system. Reads only: Omsi-Hub never writes into these folders.

use crate::text::CfgFile;
use crate::ReadError;
use std::path::{Path, PathBuf};

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
    let exact = dir.join(name);
    let entries: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).collect();
    if let Some(p) = entries.iter().find(|p| p.file_name().is_some_and(|n| n.to_string_lossy() == name)) {
        return Some(p.clone());
    }
    entries.into_iter().find(|p| p.file_name().is_some_and(|n| same_name(&n.to_string_lossy(), name))).or_else(|| exact.exists().then_some(exact))
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
            let Some(dir) = resolve_in(b, rel).filter(|d| d.is_dir()) else { continue };
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if !out.iter().any(|(n, _)| same_name(n, &name)) {
                    out.push((name, e.path().is_dir()));
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
