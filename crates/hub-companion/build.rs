//! The Companion app's web build embedded in the server (`web`): assets/companion-web, as
//! `tools/companion-web.mjs` makes it - every file gzipped under a short name in `files/`, and
//! `index.tsv` saying which address each one is served at (`/index.html\tfiles/…`). Without the
//! folder the list is empty and the server says so on its page.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let web = manifest.join("../../assets/companion-web");
    println!("cargo:rerun-if-changed={}", web.display());
    let index = std::fs::read_to_string(web.join("index.tsv")).unwrap_or_default();
    let mut src = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for line in index.lines() {
        let Some((path, file)) = line.split_once('\t') else { continue };
        let at = web.join(file.trim());
        assert!(path.starts_with('/') && at.is_file(), "assets/companion-web/index.tsv names {file:?}, which is not there: run node tools/companion-web.mjs");
        let abs = at.canonicalize().unwrap();
        let _ = writeln!(src, "    ({path:?}, include_bytes!({:?})),", abs.to_string_lossy());
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("web_files.rs");
    std::fs::write(out, src).unwrap();
}
