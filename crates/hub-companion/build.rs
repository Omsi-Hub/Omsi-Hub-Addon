//! The Companion app's web build (assets/companion-web, `tools/companion-web.mjs`: every file
//! gzipped) embedded in the server, as a list of its paths and bytes (`web`). Without the
//! folder the list is empty and the server says so on its page.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "gz") {
            out.push(p);
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let web = manifest.join("../../assets/companion-web");
    println!("cargo:rerun-if-changed={}", web.display());
    let mut files = Vec::new();
    walk(&web, &mut files);
    files.sort();
    let mut src = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for f in &files {
        let rel = f.strip_prefix(&web).unwrap().to_string_lossy().replace('\\', "/");
        let path = format!("/{}", rel.strip_suffix(".gz").unwrap());
        let abs = f.canonicalize().unwrap();
        let _ = writeln!(src, "    ({path:?}, include_bytes!({:?})),", abs.to_string_lossy());
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("web_files.rs");
    std::fs::write(out, src).unwrap();
}
