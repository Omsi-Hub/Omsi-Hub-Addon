//! Omsi-Hub's own translations: the tables of the texts the app shows - the bus company,
//! RosterFlow, the editors, online, the plugin's texts - in the languages Omsi-Hub is kept in.
//! Ported as it was from the old repository's `omsihub-locales`.
//!
//! The files keep their format (`_version: 2`, a text and its translation per language), so a
//! table moves between projects as it is. `hub-api` serves them to the page (`hub_texts`,
//! `ui/js/i18n.js`).

/// Every table of `locales/`: its file name and its text.
pub const TABLES: &[(&str, &str)] = &[
    ("app-omsihub.yml", include_str!("../locales/app-omsihub.yml")),
    ("app-theme.yml", include_str!("../locales/app-theme.yml")),
    ("bedrijf2.yml", include_str!("../locales/bedrijf2.yml")),
    ("bedrijfsdienst.yml", include_str!("../locales/bedrijfsdienst.yml")),
    ("bestemming.yml", include_str!("../locales/bestemming.yml")),
    ("bugs-company.yml", include_str!("../locales/bugs-company.yml")),
    ("bugs-timetable.yml", include_str!("../locales/bugs-timetable.yml")),
    ("bus3d.yml", include_str!("../locales/bus3d.yml")),
    ("casual.yml", include_str!("../locales/casual.yml")),
    ("cloud-saves.yml", include_str!("../locales/cloud-saves.yml")),
    ("companion.yml", include_str!("../locales/companion.yml")),
    ("company-agencies.yml", include_str!("../locales/company-agencies.yml")),
    ("company-agenda.yml", include_str!("../locales/company-agenda.yml")),
    ("company-breakdowns.yml", include_str!("../locales/company-breakdowns.yml")),
    ("company-bus-contracts.yml", include_str!("../locales/company-bus-contracts.yml")),
    ("company-chooser.yml", include_str!("../locales/company-chooser.yml")),
    ("company-common.yml", include_str!("../locales/company-common.yml")),
    ("company-complaints.yml", include_str!("../locales/company-complaints.yml")),
    ("company-contracts.yml", include_str!("../locales/company-contracts.yml")),
    ("company-costs.yml", include_str!("../locales/company-costs.yml")),
    ("company-currency.yml", include_str!("../locales/company-currency.yml")),
    ("company-drive.yml", include_str!("../locales/company-drive.yml")),
    ("company-dutycard.yml", include_str!("../locales/company-dutycard.yml")),
    ("company-easy.yml", include_str!("../locales/company-easy.yml")),
    ("company-holidays.yml", include_str!("../locales/company-holidays.yml")),
    ("company-home.yml", include_str!("../locales/company-home.yml")),
    ("company-income.yml", include_str!("../locales/company-income.yml")),
    ("company-lateness.yml", include_str!("../locales/company-lateness.yml")),
    ("company-ledger.yml", include_str!("../locales/company-ledger.yml")),
    ("company-livemap.yml", include_str!("../locales/company-livemap.yml")),
    ("company-messages.yml", include_str!("../locales/company-messages.yml")),
    ("company-multiplayer.yml", include_str!("../locales/company-multiplayer.yml")),
    ("company-names.yml", include_str!("../locales/company-names.yml")),
    ("company-ontime.yml", include_str!("../locales/company-ontime.yml")),
    ("company-ops.yml", include_str!("../locales/company-ops.yml")),
    ("company-pages-a.yml", include_str!("../locales/company-pages-a.yml")),
    ("company-pages-b.yml", include_str!("../locales/company-pages-b.yml")),
    ("company-pages.yml", include_str!("../locales/company-pages.yml")),
    ("company-planning.yml", include_str!("../locales/company-planning.yml")),
    ("company-realtime.yml", include_str!("../locales/company-realtime.yml")),
    ("company-reputation.yml", include_str!("../locales/company-reputation.yml")),
    ("company-research.yml", include_str!("../locales/company-research.yml")),
    ("company-shell.yml", include_str!("../locales/company-shell.yml")),
    ("company-staff.yml", include_str!("../locales/company-staff.yml")),
    ("company-tender-score.yml", include_str!("../locales/company-tender-score.yml")),
    ("company-together-gate.yml", include_str!("../locales/company-together-gate.yml")),
    ("company-together-leave.yml", include_str!("../locales/company-together-leave.yml")),
    ("company-together.yml", include_str!("../locales/company-together.yml")),
    ("company-variations.yml", include_str!("../locales/company-variations.yml")),
    ("controllers.yml", include_str!("../locales/controllers.yml")),
    ("crew.yml", include_str!("../locales/crew.yml")),
    ("dash.yml", include_str!("../locales/dash.yml")),
    ("dealer-delivery.yml", include_str!("../locales/dealer-delivery.yml")),
    ("dealer.yml", include_str!("../locales/dealer.yml")),
    ("depot-3d.yml", include_str!("../locales/depot-3d.yml")),
    ("depot-build.yml", include_str!("../locales/depot-build.yml")),
    ("drivers.yml", include_str!("../locales/drivers.yml")),
    ("easy-polish.yml", include_str!("../locales/easy-polish.yml")),
    ("fix_core_company_economy.yml", include_str!("../locales/fix_core_company_economy.yml")),
    ("fix_ui_editors.yml", include_str!("../locales/fix_ui_editors.yml")),
    ("fleet-number.yml", include_str!("../locales/fleet-number.yml")),
    ("fontfit.yml", include_str!("../locales/fontfit.yml")),
    ("fonts.yml", include_str!("../locales/fonts.yml")),
    ("game-modes.yml", include_str!("../locales/game-modes.yml")),
    ("hof.yml", include_str!("../locales/hof.yml")),
    ("lakauto.yml", include_str!("../locales/lakauto.yml")),
    ("lijnen.yml", include_str!("../locales/lijnen.yml")),
    ("lijnkleur.yml", include_str!("../locales/lijnkleur.yml")),
    ("lijnsoort.yml", include_str!("../locales/lijnsoort.yml")),
    ("line-existing.yml", include_str!("../locales/line-existing.yml")),
    ("livery-layers.yml", include_str!("../locales/livery-layers.yml")),
    ("logo.yml", include_str!("../locales/logo.yml")),
    ("mods.yml", include_str!("../locales/mods.yml")),
    ("multiplayer.yml", include_str!("../locales/multiplayer.yml")),
    ("online-oauth.yml", include_str!("../locales/online-oauth.yml")),
    ("online-signup.yml", include_str!("../locales/online-signup.yml")),
    ("online.yml", include_str!("../locales/online.yml")),
    ("openomsi-0227.yml", include_str!("../locales/openomsi-0227.yml")),
    ("openomsi-0230.yml", include_str!("../locales/openomsi-0230.yml")),
    ("openomsi-settings.yml", include_str!("../locales/openomsi-settings.yml")),
    ("planning-studio.yml", include_str!("../locales/planning-studio.yml")),
    ("privacy.yml", include_str!("../locales/privacy.yml")),
    ("rename.yml", include_str!("../locales/rename.yml")),
    ("ritkaart.yml", include_str!("../locales/ritkaart.yml")),
    ("rosterflow-beta66.yml", include_str!("../locales/rosterflow-beta66.yml")),
    ("rosterflow-breaks.yml", include_str!("../locales/rosterflow-breaks.yml")),
    ("rosterflow-days.yml", include_str!("../locales/rosterflow-days.yml")),
    ("rosterflow-deadheads.yml", include_str!("../locales/rosterflow-deadheads.yml")),
    ("rosterflow-gantt.yml", include_str!("../locales/rosterflow-gantt.yml")),
    ("rosterflow-interlining.yml", include_str!("../locales/rosterflow-interlining.yml")),
    ("rosterflow-lines.yml", include_str!("../locales/rosterflow-lines.yml")),
    ("rosterflow-optimise.yml", include_str!("../locales/rosterflow-optimise.yml")),
    ("rosterflow-relief.yml", include_str!("../locales/rosterflow-relief.yml")),
    ("rosterflow-roster-tab.yml", include_str!("../locales/rosterflow-roster-tab.yml")),
    ("rosterflow-routes.yml", include_str!("../locales/rosterflow-routes.yml")),
    ("rosterflow-rules.yml", include_str!("../locales/rosterflow-rules.yml")),
    ("rosterflow-spares.yml", include_str!("../locales/rosterflow-spares.yml")),
    ("rosterflow-timeplan.yml", include_str!("../locales/rosterflow-timeplan.yml")),
    ("rosterflow-timetable-copy.yml", include_str!("../locales/rosterflow-timetable-copy.yml")),
    ("rosterflow-together.yml", include_str!("../locales/rosterflow-together.yml")),
    ("rosterflow-tutorial.yml", include_str!("../locales/rosterflow-tutorial.yml")),
    ("rosterflow-unassign.yml", include_str!("../locales/rosterflow-unassign.yml")),
    ("rosterflow.yml", include_str!("../locales/rosterflow.yml")),
    ("routemap.yml", include_str!("../locales/routemap.yml")),
    ("sandbox.yml", include_str!("../locales/sandbox.yml")),
    ("sessie.yml", include_str!("../locales/sessie.yml")),
    ("slepen.yml", include_str!("../locales/slepen.yml")),
    ("snelheid.yml", include_str!("../locales/snelheid.yml")),
    ("steps.yml", include_str!("../locales/steps.yml")),
    ("tauri-addon.yml", include_str!("../locales/tauri-addon.yml")),
    ("tauri-company-more.yml", include_str!("../locales/tauri-company-more.yml")),
    ("tauri-company.yml", include_str!("../locales/tauri-company.yml")),
    ("tauri-editors.yml", include_str!("../locales/tauri-editors.yml")),
    ("tauri-omsihub.yml", include_str!("../locales/tauri-omsihub.yml")),
    ("tauri-rosterflow.yml", include_str!("../locales/tauri-rosterflow.yml")),
    ("telefoon.yml", include_str!("../locales/telefoon.yml")),
    ("tijd.yml", include_str!("../locales/tijd.yml")),
    ("tobii.yml", include_str!("../locales/tobii.yml")),
    ("update.yml", include_str!("../locales/update.yml")),
    ("vehicle-age.yml", include_str!("../locales/vehicle-age.yml")),
    ("vertalingen.yml", include_str!("../locales/vertalingen.yml")),
    ("verwijderen.yml", include_str!("../locales/verwijderen.yml")),
    ("welcome.yml", include_str!("../locales/welcome.yml")),
];

/// A text of a table's line: a YAML double-quoted string (`"…"`, `\"` and `\\` and `\n` in it)
/// or a plain one.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let Some(inner) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) else { return s.to_string() };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(o) => out.push(o),
            None => {}
        }
    }
    out
}

/// One table read: each English text with its translations (`lang` → text). A line
/// `"text":` starts an entry, `  xx: "…"` gives its translation; comments and `_version` are
/// skipped.
pub fn parse(table: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in table.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim_start().starts_with('#') || line.trim().is_empty() || line.starts_with("_version") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("  ") {
            // (a translation: `xx: "…"`)
            let Some((lang, text)) = rest.split_once(':') else { continue };
            if let Some(entry) = out.last_mut() {
                entry.1.push((lang.trim().to_string(), unquote(text)));
            }
        } else if let Some(key) = line.trim_end().strip_suffix(':') {
            out.push((unquote(key), Vec::new()));
        }
    }
    out
}

/// Every text of every table in `lang`: the English text → its translation. A text without a
/// translation into `lang` is left out (the page shows the English one).
pub fn texts(lang: &str) -> std::collections::BTreeMap<String, String> {
    let lang = lang.trim().to_ascii_lowercase();
    let mut out = std::collections::BTreeMap::new();
    for (_, table) in TABLES {
        for (key, langs) in parse(table) {
            if let Some((_, t)) = langs.iter().find(|(l, _)| l.eq_ignore_ascii_case(&lang)) {
                out.insert(key, t.clone());
            }
        }
    }
    out
}

/// The languages the tables have most of their texts in (each in at least half of them).
pub fn languages() -> Vec<String> {
    let mut count: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut entries = 0;
    for (_, table) in TABLES {
        for (_, langs) in parse(table) {
            entries += 1;
            for (l, _) in langs {
                *count.entry(l).or_default() += 1;
            }
        }
    }
    let mut out: Vec<String> = count.into_iter().filter(|(_, n)| *n * 2 >= entries).map(|(l, _)| l).collect();
    if !out.iter().any(|l| l == "en") {
        out.insert(0, "en".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{parse, texts, TABLES};

    #[test]
    fn the_tables_read_with_their_quotes() {
        let t = "_version: 2\r\n# a comment\r\n\"Say \\\"hi\\\"\":\r\n  nl: \"Zeg \\\"hoi\\\"\"\r\n  de: \"Sag \\\"hallo\\\"\"\r\n\"Two\":\r\n  nl: \"Twee\"\r\n";
        let p = parse(t);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].0, "Say \"hi\"");
        assert_eq!(p[0].1, [("nl".to_string(), "Zeg \"hoi\"".to_string()), ("de".to_string(), "Sag \"hallo\"".to_string())]);
        let nl = texts("nl");
        assert!(nl.len() > 1000, "{}", nl.len());
        assert!(super::languages().iter().any(|l| l == "nl"));
        // (every entry of every table has its text)
        for (name, table) in TABLES {
            assert!(parse(table).iter().all(|(k, _)| !k.is_empty()), "{name}");
        }
    }

    /// Every text of every table in each of the languages the app is kept in: not empty, with
    /// the same `%{name}` holes as the English (a text another table translates counts).
    #[test]
    fn every_text_is_translated_into_every_language() {
        fn holes(s: &str) -> Vec<&str> {
            let mut h: Vec<&str> = s.match_indices("%{").map(|(i, _)| &s[i..s[i..].find('}').map_or(s.len(), |e| i + e + 1)]).collect();
            h.sort();
            h
        }
        let mut missing = Vec::new();
        for lang in ["nl", "de", "fr", "ru", "uk", "pl"] {
            let table = texts(lang);
            for (name, t) in TABLES {
                for (key, _) in parse(t) {
                    match table.get(&key) {
                        Some(tr) if !tr.trim().is_empty() && holes(tr) == holes(&key) => {}
                        Some(tr) => missing.push(format!("{name}: {lang}: {key:?} has {tr:?}")),
                        None => missing.push(format!("{name}: {lang}: {key:?} is not translated")),
                    }
                }
            }
        }
        missing.dedup();
        assert!(missing.is_empty(), "{} texts:\n{}", missing.len(), missing.join("\n"));
    }

    /// Every file of the folder is in `TABLES` (a new one is taken in too), each a table.
    #[test]
    fn every_table_of_the_folder_is_listed() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut files: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".yml")).collect();
        files.sort();
        let listed: Vec<String> = TABLES.iter().map(|t| t.0.to_string()).collect();
        assert_eq!(files, listed);
        for (name, text) in TABLES {
            assert!(text.replace("\r\n", "\n").starts_with("_version: 2"), "{name}");
        }
    }
}
