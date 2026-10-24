//! The ledger's lines in the page's language. hub-core books each line as an English sentence
//! with its values in it (`Company::book`), and the save keeps it so: a line is read back
//! against the sentences hub-core writes (`LINES`) and said with that sentence's translation,
//! each value in its turn said too where it is a text of its own (a role, a part, a goal).
//! A line none of them fits stays as it was booked (a bus's number and name, a person).

use super::words::{tr, trf};

/// The sentences hub-core books, `%{name}` where a value goes: the most particular first (a
/// line is said with the first that fits).
pub const LINES: &[&str] = &[
    // the lines' takings and payments (`income::book`, `day`, `levels`)
    "%{line}: fares collected for the authority",
    "%{line}: fares paid over to the authority",
    "Line %{n}: fare association's share",
    "Line %{n}: service not started",
    // contracts of the lines (`bus_contracts`, `variations`, `concessions`, `ownline`)
    "Line %{n}: gross cost contract",
    "Line %{n}: net subsidy contract",
    "Line %{n}: private hire contract",
    "Line %{n}: %{k} journeys not paid",
    "Line %{n}: contract handed back early",
    "Line %{n}: contract ended for default, the replacement's extra cost",
    "Line %{n}: contract variation agreed",
    "Line %{n}: contract variation, the penalty",
    "Line %{n}: route approved anew",
    "Line %{n}: stop fees",
    "Line %{n} \"%{title}\": %{what}",
    "Line %{n}: %{what}",
    "Tender line %{n}",
    "Concession line %{n} (bought)",
    "Concession line %{n} (asking price)",
    "Line licence %{n}",
    "Line %{n} (own trip)",
    "Line %{n}",
    // the buses (`market`, `dealer`, `day`, `training`, `parts`, `livery`)
    "%{count} × %{bus} (new, contract %{no})",
    "%{count} × %{bus} (used, contract %{no})",
    "%{bus} (used, %{km} km)",
    "%{bus} (new)",
    "%{bus} (returned early)",
    "%{bus} (free first service)",
    "%{bus} (own service, redone)",
    "%{bus} (own service)",
    "%{bus} (own repair)",
    "Rental bus for %{bus} (line %{line})",
    "Accident damage, bus %{bus}",
    "Spare parts (%{n}), delivered at once",
    "Spare parts (%{n}), ordered by the stores",
    "Spare parts (%{n})",
    "Spare part from the stores: %{part} (%{bus})",
    "Livery design \"%{name}\"",
    // the people (`pay`, `staff`, `agency`, `plan`, `day`)
    "Drivers: paid hours of the day (%{drivers}, %{hours} h)",
    "Drivers: guaranteed hours of the week (%{drivers}, %{hours} h)",
    "%{role}: hours of the day (%{n})",
    "Job advert: %{role}",
    "Bonus: %{name}",
    "Agency drivers cancelled on the day (%{n})",
    "%{name} (booked): %{paid} h %{paid_min}, %{drove} h %{drove_min} driven",
    "Agency driver, line %{n} tour %{tour}",
    "Traffic fines of the drivers (%{n})",
    // the depot (`depot`, `yard`)
    "Fuel and power bought outside the depot",
    "Fuel and power the depot saved",
    "Street parking (%{n})",
    "Upkeep of the depot's buildings",
    "Land for the depot %{n}",
    // services, rewards, research and the rest (`contracts`, `complaints`, `adverts`, `research`,
    // `finance`, `sandbox`)
    "Internet on the buses: %{gb} GB topped up (%{provider})",
    "Internet on the buses: %{provider} (%{gb} GB a day)",
    "Cleaning company (%{buses} buses)",
    "Goal reached: %{what}",
    "Trophy milestone %{at}",
    "Complaint: refund for %{name}",
    "Complaint: day pass for %{name}",
    "Advert contract %{no} (%{advertiser}) ended early",
    "Advert contract %{no} (%{advertiser})",
    "%{topic} %{stage}/%{stages}",
    "%{topic}: stopped",
    "%{topic}: everything researched",
    "%{loan} (early repayment fee)",
    "Sandbox: balance set",
];

/// The values that are sentences of their own: a goal reached (`contracts::Aim::title`).
const VALUES: &[&str] = &[
    "Complete %{count} trips successfully",
    "Carry %{count} passengers",
    "Drive %{count} km on the lines",
    "Improve your company rating to %{stars} stars",
    "No dropped trips for %{days} days in a row",
    "At least %{pct} % of the trips on time for %{days} days in a row",
    "Run line %{n} on time for %{days} days in a row",
    "Close %{days} days in profit",
];

/// A ledger line as the page shows it: in the page's language where it is known.
pub fn say(text: &str) -> String {
    said(text, LINES, 2).unwrap_or_else(|| value(text, 2))
}

/// `text` said with the first of `sentences` it fits, its values said in turn (`depth` deep).
fn said(text: &str, sentences: &[&str], depth: u8) -> Option<String> {
    sentences.iter().find_map(|s| {
        let got = read_back(s, text)?;
        let vals: Vec<(&str, String)> = got.into_iter().map(|(k, v)| (k, value(v, depth.saturating_sub(1)))).collect();
        Some(trf(s, &vals.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>()))
    })
}

/// A value in the page's language: a text of the tables, a sentence of its own, a label with a
/// name or a number after it ("Driving school (Anna)", "Workshop bays 2"), or as it is.
fn value(v: &str, depth: u8) -> String {
    let t = tr(v);
    if t != v || depth == 0 {
        return t;
    }
    if let Some(s) = said(v, VALUES, 0).or_else(|| said(v, LINES, depth - 1)) {
        return s;
    }
    if let Some((label, rest)) = v.strip_suffix(')').and_then(|w| w.rsplit_once(" (")) {
        return format!("{} ({})", value(label, depth - 1), value(rest, depth - 1));
    }
    if let Some((label, n)) = v.rsplit_once(' ').filter(|(l, n)| !l.is_empty() && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
        let l = tr(label);
        if l != label {
            return format!("{l} {n}");
        }
    }
    v.to_string()
}

/// `text` read back against `sentence`: each `%{name}` with the part of `text` it stands for
/// (none where the words around them differ, or a value would be empty).
fn read_back<'a, 's>(sentence: &'s str, text: &'a str) -> Option<Vec<(&'s str, &'a str)>> {
    // (the words and the names between them: words[0] name[0] words[1] ... words[n])
    let mut words: Vec<&str> = Vec::new();
    let mut names: Vec<&str> = Vec::new();
    let mut rest = sentence;
    while let Some(at) = rest.find("%{") {
        let end = at + rest[at..].find('}')?;
        words.push(&rest[..at]);
        names.push(&rest[at + 2..end]);
        rest = &rest[end + 1..];
    }
    words.push(rest);
    let (first, last) = (words[0], words[words.len() - 1]);
    if names.is_empty() {
        return (text == sentence).then(Vec::new);
    }
    if text.len() < first.len() + last.len() {
        return None;
    }
    let mut mid = text.strip_prefix(first)?.strip_suffix(last)?;
    let mut out = Vec::with_capacity(names.len());
    for (i, name) in names.iter().enumerate() {
        let v = if i + 1 < names.len() {
            let w = words[i + 1];
            // (a value is at least one character: the words after it found from there on)
            let start = mid.char_indices().nth(1).map_or(mid.len(), |(b, _)| b);
            let at = start + mid.get(start..)?.find(w)?;
            let v = &mid[..at];
            mid = &mid[at + w.len()..];
            v
        } else {
            mid
        };
        if v.is_empty() {
            return None;
        }
        out.push((*name, v));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{read_back, say, LINES, VALUES};
    use crate::company::words::set_language;

    #[test]
    fn a_line_is_read_back_against_its_sentence() {
        assert_eq!(read_back("Line %{n}: stop fees", "Line 5: stop fees"), Some(vec![("n", "5")]));
        assert_eq!(read_back("Advert contract %{no} (%{advertiser})", "Advert contract 3 (Hotel (Am Markt))"), Some(vec![("no", "3"), ("advertiser", "Hotel (Am Markt)")]));
        assert_eq!(read_back("Line %{n}: stop fees", "Line 5: route approved anew"), None);
        assert_eq!(read_back("%{bus} (new)", " (new)"), None);
        assert_eq!(read_back("Sandbox: balance set", "Sandbox: balance set"), Some(vec![]));
    }

    #[test]
    fn the_ledger_s_lines_are_said_in_the_page_s_language() {
        set_language("de");
        assert_eq!(say("Accident damage, bus 104"), crate::company::words::trf("Accident damage, bus %{bus}", &[("bus", "104")]));
        assert_ne!(say("Accident damage, bus 104"), "Accident damage, bus 104");
        // (a line within a line, a goal within a line, a role, a part)
        let own = say("Line 5 (own trip): fares collected for the authority");
        assert!(own.contains('5') && !own.contains("own trip") && !own.contains("fares collected"), "{own}");
        let goal = say("Goal reached: Carry 500 passengers");
        assert!(goal.contains("500") && !goal.contains("Carry"), "{goal}");
        assert!(!say("Bus drivers: hours of the day (12)").contains("Bus drivers"));
        assert!(!say("Spare part from the stores: Battery (104)").contains("Battery"));
        // (what fits no sentence stays: a bus, a person)
        assert_eq!(say("104 Citaro"), "104 Citaro");
        assert_eq!(say("Anna Weber"), "Anna Weber");
        set_language("en");
        assert_eq!(say("Line 5 (own trip): fares collected for the authority"), "Line 5 (own trip): fares collected for the authority");
    }

    /// Every sentence in the seven languages (with the same values), and every sentence hub-core
    /// books here: a new one is said too.
    #[test]
    fn every_ledger_sentence_is_known_and_translated() {
        let blank = |s: &str| {
            let mut out = String::new();
            let mut rest = s;
            while let Some(at) = rest.find('{') {
                out.push_str(&rest[..at]);
                out.push_str("%{}");
                rest = rest[at..].find('}').map_or("", |e| &rest[at + e + 1..]);
            }
            out.push_str(rest);
            out.replace("%%{}", "%{}")
        };
        let mut missing = Vec::new();
        for lang in ["nl", "de", "fr", "ru", "uk", "pl", "es"] {
            let table = hub_locales::texts(lang);
            for s in LINES.iter().chain(VALUES) {
                match table.get(*s) {
                    Some(t) if t.matches("%{").count() == s.matches("%{").count() => {}
                    _ => missing.push(format!("{lang}: {s:?}")),
                }
            }
        }
        assert!(missing.is_empty(), "{} not translated:\n{}", missing.len(), missing.join("\n"));
        // (each sentence's values are apart: words between every two)
        assert!(LINES.iter().all(|s| !s.contains("}%{")));
        // the sentences hub-core books, as the code writes them (`format!` or a text), and those
        // made just before the booking
        let known: Vec<String> = LINES.iter().map(|s| blank(s)).collect();
        let de = hub_locales::texts("de");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hub-core/src/company");
        let mut unknown = Vec::new();
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".rs") || name.contains("test") {
                continue;
            }
            let src = std::fs::read_to_string(e.path()).unwrap().replace("\r\n", "\n");
            let src = src.split("#[cfg(test)]").next().unwrap_or("");
            let lines: Vec<&str> = src.lines().collect();
            for (i, l) in lines.iter().enumerate() {
                if !l.contains(".book(BookingKind") && !l.contains(".book(k, ") && !l.contains(".book(kind, ") {
                    continue;
                }
                // (the booking's text and what was written into it on the lines before)
                for w in lines[i.saturating_sub(12)..=i].iter().flat_map(|l| l.split("format!(\"").skip(1)).chain(l.split("false").next().into_iter().flat_map(|l| l.split(", \"").skip(1))) {
                    let w = w.replace("\\\"", "\u{1}");
                    let Some(s) = w.split('"').next() else { continue };
                    let s = blank(&s.replace('\u{1}', "\""));
                    // (a bus's number and name, a person, words that are no sentence; and what
                    // LINES says word for word: "Line 5: gross cost contract", "Agency drivers
                    // cancelled ...", "Complaint: refund for ...", a line's name before its cost)
                    let parts = ["Line %{}%{}", "%{}s cancelled on the day (%{})", "Line %{} \"%{}\"", "%{}: %{}", "Complaint: %{} for %{}", "Line %{}: %{} contract"];
                    if !s.chars().any(|c| c.is_ascii_lowercase()) || parts.contains(&s.as_str()) || ["line", "bus", "Someone", "goal-%{}-soon"].contains(&s.as_str()) {
                        continue;
                    }
                    // (a sentence without values is said as a text of the tables: "Own mechanics")
                    let fits = known.contains(&s) || de.contains_key(&s) || known.iter().any(|k| s.ends_with(k.trim_start_matches("%{}")) && k.starts_with("%{}") && k.len() > 3);
                    if !fits {
                        unknown.push(format!("{name}:{}: {s}", i + 1));
                    }
                }
            }
        }
        unknown.sort();
        unknown.dedup();
        assert!(unknown.is_empty(), "booked but not in ledger::LINES:\n{}", unknown.join("\n"));
    }
}
