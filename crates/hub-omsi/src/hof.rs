//! Depot files (`.hof`): the destinations a bus's IBIS and displays know (`[addterminus]`),
//! the stops (`[addbusstop]`), the routes (`[infosystem_trip]`) with their stop lists
//! (`[infosystem_busstop_list]`), and the strings each one has (`stringcount_terminus`,
//! `stringcount_busstop`). Read and written back with their comments and codepage.
//!
//! The format, as OMSI's own depot files have it:
//! * `[name]`, `[servicetrip]` (the destination of an empty run), `[global_strings]` (a count,
//!   then the strings);
//! * `stringcount_terminus` and `stringcount_busstop` - written without brackets - and their
//!   count on the next line: how many strings each destination and stop has;
//! * `[addterminus]`: code, name (the texture's id, which is also the stop it ends at), then the
//!   strings; `[addterminus_allexit]` the same. (The SDK's comment names a line for the stop
//!   between them, but no file OMSI ships has one: openOMSI checked every stock depot file, and
//!   reading one shifted every destination's strings by a place.) `[addterminus_list]` … `[end]` has one
//!   destination a line, its columns tab-separated: `{ALLEX}` or nothing, the code, the name,
//!   the strings;
//! * `[addbusstop]`: the stop's name, then its strings; `[addbusstop_list]` … `[end]` one a
//!   line (name, strings);
//! * `[infosystem_trip]`: the route's code, its name, the destination's code, the line; then
//!   its `[infosystem_busstop_list]` (a count, the stops) - or, in older files, one
//!   `[infosystem_busstop]` a stop (its first string taken as the stop).
//!
//! A keyword counts at the start of its line only: the indented examples in the files'
//! comments are comments. Tabs after a value (the files' column padding) are not part of it;
//! the spaces a display string is centred with are.

use crate::text::{keyword_of, parse_i32, CfgFile, Codepage};
use crate::ReadError;
use std::path::{Path, PathBuf};

/// A destination (`[addterminus]`, `[addterminus_allexit]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Terminus {
    /// Its IBIS code.
    pub code: i32,
    /// Its name, which is also the texture's id the displays look up.
    pub texture_id: String,
    /// The stop it ends at: its name (None for `_allexit`).
    pub terminus_stop: Option<String>,
    /// Everyone gets out there (`_allexit`).
    pub all_exit: bool,
    /// Its strings (`[stringcount_terminus]` of them).
    pub strings: Vec<String>,
}

/// A stop (`[addbusstop]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BusStop {
    pub ident: String,
    /// Its strings (`[stringcount_busstop]` of them).
    pub strings: Vec<String>,
}

/// A route of the IBIS (`[infosystem_trip]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfoTrip {
    /// Its IBIS route code (a number, as text).
    pub code: String,
    pub name: String,
    /// The terminus code it goes to (a number, as text).
    pub route: String,
    pub line: String,
    /// Further values, written after the four (OMSI reads four: the reader leaves this empty).
    pub extra: Vec<String>,
}

/// The comments of a file, kept where they were.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Comments {
    /// The lines at the top of the file.
    pub head: Vec<String>,
    /// What each destination string is for (`string0: IBIS display`), in step with the count.
    pub terminus: Vec<String>,
    /// What each stop string is for (`busstop string 0: …`).
    pub busstop: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hof {
    pub path: PathBuf,
    /// `[name]`.
    pub name: String,
    /// `[servicetrip]`: the destination of an empty run.
    pub service_trip: String,
    /// `[global_strings]`.
    pub global_strings: Vec<String>,
    pub string_count_terminus: usize,
    pub string_count_busstop: usize,
    pub termini: Vec<Terminus>,
    pub bus_stops: Vec<BusStop>,
    pub info_trips: Vec<InfoTrip>,
    /// The stops of each route, in step with `info_trips`.
    pub info_busstop_lists: Vec<Vec<String>>,
    pub codepage: Option<Codepage>,
}

/// The strings a destination has when the file does not say (OMSI's own layout), and a stop's.
const DEFAULT_TERMINUS_STRINGS: usize = 8;
const DEFAULT_BUSSTOP_STRINGS: usize = 4;

/// A value without the tabs the files pad their columns with.
fn val(s: &str) -> String {
    s.trim_end_matches(['\t', '\r']).to_string()
}

/// A count line (`stringcount_terminus`, bare or in brackets) at the start of a line.
fn is_count_line(line: &str, name: &str) -> bool {
    let t = line.trim_end();
    !line.starts_with([' ', '\t']) && (t.eq_ignore_ascii_case(name) || t.strip_prefix('[').and_then(|r| r.strip_suffix(']')).is_some_and(|k| k.eq_ignore_ascii_case(name)))
}

/// The counts of a depot file's strings: (destinations', stops').
fn string_counts(lines: &[String]) -> (usize, usize) {
    let count = |name: &str, default: usize| lines.iter().position(|l| is_count_line(l, name)).and_then(|i| lines.get(i + 1)).map_or(default, |v| parse_i32(v).max(0) as usize);
    (count("stringcount_terminus", DEFAULT_TERMINUS_STRINGS), count("stringcount_busstop", DEFAULT_BUSSTOP_STRINGS))
}

/// `n` strings: those given, the missing ones empty.
fn padded(mut v: Vec<String>, n: usize) -> Vec<String> {
    v.resize(n, String::new());
    v
}

impl Hof {
    pub fn parse(cfg: &CfgFile) -> Hof {
        let (nt, nb) = string_counts(&cfg.lines);
        let mut h = Hof { path: cfg.path.clone(), codepage: Some(cfg.codepage), string_count_terminus: nt, string_count_busstop: nb, ..Hof::default() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k.to_ascii_lowercase().as_str() {
                "name" => h.name = r.value().trim().to_string(),
                "servicetrip" => h.service_trip = r.value().trim().to_string(),
                "global_strings" => {
                    let n = parse_i32(r.value()).max(0) as usize;
                    h.global_strings = r.values(n).into_iter().map(val).collect();
                }
                kw @ ("addterminus" | "addterminus_allexit") => {
                    let all_exit = kw == "addterminus_allexit";
                    let code = parse_i32(r.value());
                    let texture_id = r.value().trim().to_string();
                    let terminus_stop = (!all_exit).then(|| texture_id.clone()).filter(|s| !s.is_empty());
                    let strings = padded(r.values(nt).into_iter().map(val).collect(), nt);
                    h.termini.push(Terminus { code, texture_id, terminus_stop, all_exit, strings });
                }
                "addbusstop" => {
                    let ident = r.value().trim().to_string();
                    let strings = padded(r.values(nb).into_iter().map(val).collect(), nb);
                    h.bus_stops.push(BusStop { ident, strings });
                }
                "addterminus_list" => {
                    for row in r.until("end") {
                        let cols: Vec<&str> = row.split('\t').collect();
                        let col = |i: usize| cols.get(i).copied().unwrap_or("");
                        if cols.len() < 3 || !col(1).trim().chars().all(|c| c.is_ascii_digit() || c == '-') || col(1).trim().is_empty() {
                            continue;
                        }
                        let all_exit = col(0).trim().to_ascii_lowercase().contains("allex");
                        let texture_id = col(2).trim().to_string();
                        let strings = padded(cols.iter().skip(3).take(nt).map(|s| val(s)).collect(), nt);
                        let terminus_stop = (!all_exit).then(|| texture_id.clone());
                        h.termini.push(Terminus { code: parse_i32(col(1)), texture_id, terminus_stop, all_exit, strings });
                    }
                }
                "addbusstop_list" => {
                    for row in r.until("end") {
                        let cols: Vec<&str> = row.split('\t').collect();
                        let ident = cols[0].trim();
                        if ident.is_empty() {
                            continue;
                        }
                        let strings = padded(cols.iter().skip(1).take(nb).map(|s| val(s)).collect(), nb);
                        h.bus_stops.push(BusStop { ident: ident.to_string(), strings });
                    }
                }
                "infosystem_trip" => {
                    let v = r.values(4);
                    let at = |i: usize| v.get(i).map_or("", |s| s.trim()).to_string();
                    h.info_trips.push(InfoTrip { code: at(0), name: at(1), route: at(2), line: at(3), extra: Vec::new() });
                    h.info_busstop_lists.push(Vec::new());
                }
                "infosystem_busstop_list" => {
                    let n = parse_i32(r.value()).max(0) as usize;
                    let stops: Vec<String> = r.values(n).into_iter().map(|s| s.trim().to_string()).collect();
                    if let Some(list) = h.info_busstop_lists.last_mut() {
                        *list = stops;
                    }
                }
                "infosystem_busstop" => {
                    let stop = r.value().trim().to_string();
                    if let Some(list) = h.info_busstop_lists.last_mut() {
                        list.push(stop);
                    }
                }
                _ => {}
            }
        }
        h
    }

    pub fn load(path: &Path) -> Result<Hof, ReadError> {
        let mut h = Hof::parse(&CfgFile::read(path)?);
        h.path = path.to_path_buf();
        Ok(h)
    }

    /// Only the depot's `[name]`, without reading the rest.
    pub fn read_name(path: &Path) -> Option<String> {
        CfgFile::read(path).ok()?.value("name").map(str::to_string)
    }

    /// The file's text again, with `comments` where they were.
    pub fn to_text(&self, comments: &Comments) -> String {
        let (nt, nb) = (self.string_count_terminus, self.string_count_busstop);
        let mut s = String::new();
        let line = |s: &mut String, l: &str| {
            s.push_str(l);
            s.push_str("\r\n");
        };
        for h in comments.head.iter().map(|h| h.trim()).filter(|h| !h.is_empty()) {
            // (indented: a comment, even one that looks like a keyword)
            line(&mut s, &format!("\t{h}"));
        }
        if !comments.head.is_empty() {
            line(&mut s, "");
        }
        s.push_str(&format!("[name]\r\n{}\r\n\r\n[servicetrip]\r\n{}\r\n\r\n", self.name, self.service_trip));
        if !self.global_strings.is_empty() {
            line(&mut s, "[global_strings]");
            line(&mut s, &self.global_strings.len().to_string());
            for g in &self.global_strings {
                line(&mut s, g);
            }
            line(&mut s, "");
        }
        s.push_str(&format!("stringcount_terminus\r\n{nt}\r\n"));
        for (k, n) in comments.terminus.iter().enumerate().filter(|(_, n)| !n.trim().is_empty()) {
            line(&mut s, &format!("\tstring{k}:\t{}", n.trim()));
        }
        s.push_str(&format!("\r\nstringcount_busstop\r\n{nb}\r\n"));
        for (k, n) in comments.busstop.iter().enumerate().filter(|(_, n)| !n.trim().is_empty()) {
            line(&mut s, &format!("\tbusstop string {k}:\t{}", n.trim()));
        }
        line(&mut s, "");
        for t in &self.termini {
            line(&mut s, if t.all_exit { "[addterminus_allexit]" } else { "[addterminus]" });
            line(&mut s, &t.code.to_string());
            line(&mut s, &t.texture_id);
            for k in 0..nt {
                line(&mut s, t.strings.get(k).map_or("", String::as_str));
            }
            s.push_str("................\r\n\r\n");
        }
        for b in &self.bus_stops {
            line(&mut s, "[addbusstop]");
            line(&mut s, &b.ident);
            for k in 0..nb {
                line(&mut s, b.strings.get(k).map_or("", String::as_str));
            }
            s.push_str("................\r\n\r\n");
        }
        for (i, t) in self.info_trips.iter().enumerate() {
            s.push_str("-----------------------------------\r\n\r\n[infosystem_trip]\r\n");
            for v in [&t.code, &t.name, &t.route, &t.line].into_iter().chain(&t.extra) {
                line(&mut s, v);
            }
            let stops = self.info_busstop_lists.get(i).map_or(&[][..], Vec::as_slice);
            s.push_str(&format!("\r\n[infosystem_busstop_list]\r\n{}\r\n", stops.len()));
            for st in stops {
                line(&mut s, st);
            }
            line(&mut s, "");
        }
        s
    }

    /// The file's bytes: its text in its codepage (Windows-1252 for a new one).
    pub fn to_bytes(&self, comments: &Comments) -> Vec<u8> {
        let cp = self.codepage.unwrap_or(Codepage::Windows1252);
        let mut out = if cp.is_utf16() { cp.bom().to_vec() } else { Vec::new() };
        out.extend(crate::text::encode(&self.to_text(comments), cp).0);
        out
    }

    /// Written to `path` in its codepage (`timetable::write_text`'s rules).
    pub fn save(&self, path: &Path, comments: &Comments) -> std::io::Result<()> {
        crate::timetable::write_text(path, &self.to_text(comments), self.codepage.unwrap_or(Codepage::Windows1252))
    }

    /// It has a route for `line`.
    pub fn has_line(&self, line: &str) -> bool {
        self.info_trips.iter().any(|t| t.line.trim().eq_ignore_ascii_case(line.trim()))
    }

    /// A destination by its code.
    pub fn terminus(&self, code: i32) -> Option<&Terminus> {
        self.termini.iter().find(|t| t.code == code)
    }

    /// A stop by its name, in any case.
    pub fn bus_stop(&self, ident: &str) -> Option<&BusStop> {
        self.bus_stops.iter().find(|b| crate::vfs::same_name(b.ident.trim(), ident.trim()))
    }
}

impl Comments {
    /// A depot file's comments: the lines at its top, and the notes on its strings
    /// (`string0: …` before its first destination, `busstop string 0: …`).
    pub fn read(cfg: &CfgFile) -> Comments {
        let (nt, nb) = string_counts(&cfg.lines);
        let mut c = Comments { head: Vec::new(), terminus: vec![String::new(); nt], busstop: vec![String::new(); nb] };
        let first_kw = cfg.lines.iter().position(|l| keyword_of(l).is_some() || is_count_line(l, "stringcount_terminus")).unwrap_or(cfg.lines.len());
        c.head = cfg.lines[..first_kw].iter().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).take(40).collect();
        let note = |rest: &str| -> Option<(usize, String)> {
            let digits: String = rest.trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
            let after = rest.trim_start()[digits.len()..].trim_start().strip_prefix(':')?;
            Some((digits.parse().ok()?, after.trim().to_string()))
        };
        for l in &cfg.lines {
            if keyword_of(l).is_some_and(|k| k.to_ascii_lowercase().starts_with("addterminus")) {
                break;
            }
            let t = l.trim();
            let low = t.to_ascii_lowercase();
            // (`low` has only ASCII letters lowered: the same length as `t`)
            let (list, rest) = if let Some(r) = low.strip_prefix("busstop string") {
                (&mut c.busstop, &t[t.len() - r.len()..])
            } else if let Some(r) = low.strip_prefix("string") {
                (&mut c.terminus, &t[t.len() - r.len()..])
            } else {
                continue;
            };
            if let Some((k, n)) = note(rest) {
                if let Some(slot) = list.get_mut(k).filter(|s| s.is_empty()) {
                    *slot = n;
                }
            }
        }
        c
    }
}

/// The `.hof` files of a folder, sorted by name.
pub fn depot_files(dir: &Path) -> Vec<PathBuf> {
    let entries = crate::vfs::listing(dir);
    let mut out: Vec<PathBuf> = entries.iter().flat_map(|l| &l.entries).filter(|e| !e.is_dir && e.path.extension().is_some_and(|x| x.eq_ignore_ascii_case("hof"))).map(|e| e.path.clone()).collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_depot_s_name_and_lines() {
        let d = std::env::temp_dir().join(format!("hub-omsi-hof-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("Berlin.HOF"), b"[name]\r\nBerlin Spandau\r\n").unwrap();
        std::fs::write(d.join("x.txt"), b"").unwrap();
        let files = depot_files(&d);
        assert_eq!(files.len(), 1);
        assert_eq!(Hof::read_name(&files[0]).as_deref(), Some("Berlin Spandau"));
        let h = Hof { info_trips: vec![InfoTrip { line: "10".into(), ..InfoTrip::default() }], ..Hof::default() };
        assert!(h.has_line(" 10") && !h.has_line("11"));
    }

    #[test]
    fn the_list_forms_and_the_old_stop_form_are_read() {
        let text = "[name]\t\t\r\nListdorf\t\t\r\n\r\nstringcount_terminus\t\t\r\n3\r\nstringcount_busstop\r\n2\r\n\r\n\t\t\tstring0\tstring1\r\n[addterminus_list]\t\t\r\n{ALLEX}\t0\tEmpty\t\t\tBlank.tga\t\r\n\t12\tMarket\tMARKET\t MARKET \tMarket.tga\r\n\tCode\tIdent\tIBIS\r\n[end]\r\n[addbusstop_list]\r\nMarket\tMARKET\tMarket\r\n\r\n[end]\r\n\t[infosystem_trip]\r\n\t{routecode}\r\n[infosystem_trip]\r\n1201\r\nA-B\r\n12\r\n12\r\n\r\n[infosystem_busstop]\r\nMARKET\r\nMarket\r\n[infosystem_busstop]\r\nCHURCH\r\nChurch\r\n";
        let h = Hof::parse(&CfgFile::from_str("x.hof", text));
        assert_eq!((h.name.as_str(), h.string_count_terminus, h.string_count_busstop), ("Listdorf", 3, 2));
        assert_eq!(h.termini.len(), 2);
        assert_eq!(h.termini[0], Terminus { code: 0, texture_id: "Empty".into(), terminus_stop: None, all_exit: true, strings: vec!["".into(), "".into(), "Blank.tga".into()] });
        assert_eq!(
            h.termini[1],
            Terminus { code: 12, texture_id: "Market".into(), terminus_stop: Some("Market".into()), all_exit: false, strings: vec!["MARKET".into(), " MARKET ".into(), "Market.tga".into()] }
        );
        assert_eq!(h.bus_stops, [BusStop { ident: "Market".into(), strings: vec!["MARKET".into(), "Market".into()] }]);
        // (the indented [infosystem_trip] is the comment's example)
        assert_eq!(h.info_trips.len(), 1);
        assert_eq!(h.info_busstop_lists, [vec!["MARKET".to_string(), "CHURCH".to_string()]]);
    }
}
