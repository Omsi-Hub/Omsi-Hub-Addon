//! The readers and writers on the synthetic OMSI 2 folder in `tests/fixtures/omsi2`: a made-up
//! map (Testdorf: its timetable, calendar, Chrono folders and AI lists), a bus with its cabin,
//! paints, numbers and depot file, a display font and two weather files. The files are as OMSI
//! writes them: `\r\n` line ends, Windows-1252, `global.cfg` and a weather file in UTF-16, names
//! in mixed case.

use chrono::NaiveDate;
use hub_omsi::hof::{depot_files, Comments, Hof};
use hub_omsi::map::{active_chrono_dirs, chrono_deactivated_lines, date_code, depot_hof_on, AiLists, Calendar, GlobalCfg};
use hub_omsi::text::{CfgFile, Codepage};
use hub_omsi::timetable::{lines_for, parse_busstops, parse_stnlinks, replace_block, stop_times, write_text, BusStopEntry, DayMask, Line, StnLink, Timetable, Track, Trip};
use hub_omsi::vehicle::{capacity, Bus, PassengerCabin};
use hub_omsi::weather::{Precipitation, Weather};
use hub_omsi::{font, Vfs};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/omsi2")
}

fn map_dir() -> PathBuf {
    root().join("maps/Testdorf")
}

fn ttdata() -> PathBuf {
    map_dir().join("TTData")
}

fn bus_dir() -> PathBuf {
    root().join("Vehicles/Testbus")
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hub-omsi-fx-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cfg(path: &Path) -> CfgFile {
    CfgFile::read(path).unwrap()
}

/// Seconds as minutes from `start` (to compare stop times).
fn mins(times: &[(f64, f64)], start: f64) -> Vec<(f64, f64)> {
    times.iter().map(|(a, b)| (((a - start) / 60.0 * 1000.0).round() / 1000.0, ((b - start) / 60.0 * 1000.0).round() / 1000.0)).collect()
}

// --- the timetable ---------------------------------------------------------------------------

#[test]
fn the_timetable_folder_is_read() {
    let tt = Timetable::load(&ttdata());
    let names: Vec<&str> = tt.lines.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["10", "N1"]);
    let ten = &tt.lines[0];
    assert!(ten.user_allowed);
    assert_eq!((ten.priority, ten.codepage), (1, Some(Codepage::Windows1252)));
    assert_eq!(ten.tours.iter().map(|t| (t.number.as_str(), t.ai_group.as_str(), t.days)).collect::<Vec<_>>(), [("1", "Busses", DayMask(799)), ("2", "Busses", DayMask(960))]);
    assert_eq!(
        ten.tours[0].trips.iter().map(|t| (t.trip.as_str(), t.profile, t.departure)).collect::<Vec<_>>(),
        [("10_out", 0, 360.0), ("10_back", 0, 375.0), ("10_OUT", 1, 1430.0), ("10_back", 0, 5.0)]
    );
    assert!(!tt.lines[1].user_allowed);

    // a trip of [station_typ2] stops, named from Busstops.cfg (Windows-1252), on its own track
    let out = tt.trip("10_OUT").unwrap();
    assert_eq!((out.track.as_str(), out.display_name.as_str(), out.terminus.as_str(), out.line.as_str()), ("10_out", "", "Kirche", "10"));
    assert_eq!(
        out.stops.iter().map(|s| (s.id, s.name.as_str(), s.track_index)).collect::<Vec<_>>(),
        [(1001, "Depot", None), (1002, "Grünau Markt", None), (1003, "Hauptstraße", None), (1004, "Kirche", None)]
    );
    assert_eq!(out.profiles.len(), 2);
    assert_eq!((out.profiles[0].name.as_str(), out.profiles[0].minutes, out.profiles[0].fixed_departures.clone()), ("standard", 10.0, vec![(1, 2.0)]));
    assert_eq!(tt.track_of(out).map(|t| t.entries.len()), Some(4));

    // a trip of the older [station] stops, on a track it names (whose file is in another case)
    let back = tt.trip("10_back").unwrap();
    assert_eq!(back.track, "10_back_track");
    assert_eq!(back.stops.iter().map(|s| (s.id, s.track_index)).collect::<Vec<_>>(), [(1004, Some(0)), (1003, Some(2)), (1002, Some(3)), (1001, Some(5))]);
    assert_eq!((back.stops[1].name.as_str(), back.stops[0].extra.len()), ("Hauptstraße", 5));
    let p = &back.profiles[0];
    assert_eq!((p.fixed_arrivals.clone(), p.fixed_departures.clone(), p.stopping.clone()), (vec![(2, 5.0)], vec![(2, 6.0)], vec![(2, 1)]));
    let track = tt.track_of(back).unwrap();
    assert_eq!(track.name, "10_Back_Track");
    assert_eq!(track.entries[2].spline, 22);
    assert_eq!((track.entries[2].path, track.entries[2].tile, track.entries[2].length), (0, 1, 300.0));

    assert_eq!(tt.stn_links.len(), 5);
    let l = tt.link(1001, 1002).unwrap();
    assert_eq!((l.length, l.params, l.entries.len()), (400.0, [2.5, 2.5, 3.0, 4.0, 0.0, 1.0], 2));
    assert_eq!(l.entries[1], [11.0, 0.0, 0.0, 150.0, -1.0, 0.0, 0.0]);
    assert_eq!(tt.bus_stops.len(), 4);
    assert_eq!(tt.bus_stops[1], BusStopEntry { name: "Grünau Markt".into(), tile: 0, object_id: 1002, params: [2.0, 0.0, 0.0] });
    assert_eq!(tt.stop_name(1003), Some("Hauptstraße"));
}

#[test]
fn every_timetable_file_reads_the_same_after_writing_back() {
    let mut files: Vec<PathBuf> = std::fs::read_dir(ttdata()).unwrap().flatten().map(|e| e.path()).collect();
    files.extend(std::fs::read_dir(map_dir().join("Chrono/0100_Works/TTData")).unwrap().flatten().map(|e| e.path()));
    let mut checked = 0;
    for f in &files {
        let c = cfg(f);
        let again = |text: String| CfgFile::from_bytes(f, &hub_omsi::text::encode(&text, c.codepage).0);
        let ext = f.extension().unwrap().to_string_lossy().to_lowercase();
        let name = f.file_name().unwrap().to_string_lossy().to_lowercase();
        match ext.as_str() {
            "ttl" => assert_eq!(Line::parse(&again(Line::parse(&c).to_text())), Line::parse(&c), "{}", f.display()),
            "ttp" => assert_eq!(Trip::parse(&again(Trip::parse(&c).to_text())), Trip::parse(&c), "{}", f.display()),
            "ttr" => assert_eq!(Track::parse(&again(Track::parse(&c).to_text())), Track::parse(&c), "{}", f.display()),
            _ if name == "stnlinks.cfg" => {
                let links = parse_stnlinks(&c);
                assert_eq!(parse_stnlinks(&again(links.iter().map(|l| l.to_text("a", "b")).collect())), links);
            }
            _ if name == "busstops.cfg" => {
                let stops = parse_busstops(&c);
                assert_eq!(parse_busstops(&again(stops.iter().map(BusStopEntry::to_text).collect())), stops);
            }
            _ => continue,
        }
        checked += 1;
    }
    assert_eq!(checked, files.len());
    // (a whole timetable too)
    let tt = Timetable::load(&ttdata());
    let dir = temp("tt");
    for l in &tt.lines {
        write_text(&dir.join(format!("{}.ttl", l.name)), &l.to_text(), l.codepage.unwrap()).unwrap();
    }
    for t in &tt.trips {
        write_text(&dir.join(format!("{}.ttp", t.name)), &t.to_text(), Codepage::Windows1252).unwrap();
    }
    for t in &tt.tracks {
        write_text(&dir.join(format!("{}.ttr", t.name)), &t.to_text(), Codepage::Windows1252).unwrap();
    }
    let names = |id: i64| tt.stop_name(id).unwrap_or("").to_string();
    write_text(&dir.join("StnLinks.cfg"), &tt.stn_links.iter().map(|l| l.to_text(&names(l.from_id), &names(l.to_id))).collect::<String>(), Codepage::Windows1252).unwrap();
    write_text(&dir.join("Busstops.cfg"), &tt.bus_stops.iter().map(BusStopEntry::to_text).collect::<String>(), Codepage::Windows1252).unwrap();
    let back = Timetable::load(&dir);
    let without_paths = |t: &Timetable| {
        let mut t = t.clone();
        t.lines.iter_mut().for_each(|l| l.path = PathBuf::new());
        t.trips.iter_mut().for_each(|x| x.path = PathBuf::new());
        t.tracks.iter_mut().for_each(|x| x.path = PathBuf::new());
        t
    };
    assert_eq!(without_paths(&back), without_paths(&tt));
    // (the written files are OMSI's: CRLF, Windows-1252)
    let bytes = std::fs::read(dir.join("Busstops.cfg")).unwrap();
    assert!(bytes.windows(2).any(|w| w == b"\r\n") && !bytes.windows(2).any(|w| w[1] == b'\n' && w[0] != b'\r'));
    assert!(bytes.windows(4).any(|w| w == b"Gr\xFCn"));
}

#[test]
fn stop_times_follow_the_links_and_the_profile() {
    let tt = Timetable::load(&ttdata());
    let out = tt.trip("10_out").unwrap();
    // standard: 10 min, departing stop 1 at 2 min; the stop between 2 and 10 by its links'
    // lengths (600 m of 1600)
    assert_eq!(mins(&stop_times(&tt, out, 0, 21_600.0), 21_600.0), [(0.0, 0.0), (2.0, 2.0), (5.0, 5.0), (10.0, 10.0)]);
    // evening: 8 min shared by 400, 600 and 1000 m
    assert_eq!(mins(&stop_times(&tt, out, 1, 0.0), 0.0), [(0.0, 0.0), (1.6, 1.6), (4.0, 4.0), (8.0, 8.0)]);
    // (a profile that is not there: the first)
    assert_eq!(stop_times(&tt, out, 7, 0.0), stop_times(&tt, out, 0, 0.0));
    // 10_back: the leg without a link takes the track's lanes between the stops' places (300 m);
    // stop 2 arrives at 5 and leaves at 6
    let back = tt.trip("10_back").unwrap();
    assert_eq!(hub_omsi::timetable::leg_lengths(&tt, back), [Some(1000.0), Some(300.0), Some(400.0)]);
    assert_eq!(mins(&stop_times(&tt, back, 0, 0.0), 0.0), [(0.0, 0.0), (3.846, 3.846), (5.0, 6.0), (12.0, 12.0)]);
    assert!((hub_omsi::timetable::trip_length(&tt, back) - 1700.0).abs() < 1e-9);
}

#[test]
fn the_lines_of_a_day_apply_chrono_the_calendar_and_the_day_masks() {
    let vfs = Vfs::new(root(), Some(temp("content")));
    let map = "maps/testdorf/GLOBAL.cfg";

    // 6 June 1989, a Tuesday: the works are over, line N1 is switched off
    let lines = lines_for(&vfs, map, d("1989-06-06")).unwrap();
    assert_eq!(lines.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["10"]);
    let ten = &lines[0];
    assert!(ten.user_allowed);
    assert_eq!(ten.termini, ["Depot", "Kirche"]);
    let (t1, t2) = (&ten.tours[0], &ten.tours[1]);
    assert!(t1.runs && !t2.runs);
    assert_eq!((t1.next_run, t2.next_run), (Some(d("1989-06-06")), Some(d("1989-06-11"))));
    assert_eq!(
        t1.trips.iter().map(|t| (t.index, t.name.as_str(), t.departure, t.arrival)).collect::<Vec<_>>(),
        [
            (1, "10_out", 21_600.0, 22_200.0),
            (2, "10_back", 22_500.0, 23_220.0),
            (3, "10_OUT", 85_800.0, 86_280.0),
            // (after midnight: the next day's seconds)
            (4, "10_back", 86_700.0, 87_420.0),
        ]
    );
    assert_eq!((t1.first, t1.last), (21_600.0, 87_420.0));
    let back = &t1.trips[1];
    assert_eq!((back.line.as_str(), back.from.as_str(), back.terminus.as_str()), ("10", "Kirche", "Depot"));
    assert!((back.km - 1.7).abs() < 1e-9 && (t1.trips[0].km - 2.0).abs() < 1e-9);
    assert_eq!(back.stops.iter().map(|s| (s.name.as_str(), s.id)).collect::<Vec<_>>(), [("Kirche", 1004), ("Hauptstraße", 1003), ("Grünau Markt", 1002), ("Depot", 1001)]);
    assert_eq!((back.stops[2].arr, back.stops[2].dep), (22_800.0, 22_860.0));

    // 30 May 1989: the works' Chrono folder has its own trip (a stop of its own Busstops.cfg,
    // added to the map's) and N1 still runs
    let lines = lines_for(&vfs, map, d("1989-05-30")).unwrap();
    assert_eq!(lines.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["10", "N1"]);
    let out = &lines[0].tours[0].trips[0];
    assert_eq!(out.stops.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Depot", "Grünau Markt", "Umleitung", "Kirche"]);
    assert_eq!(out.stops.iter().map(|s| (s.arr - out.departure) / 60.0).collect::<Vec<_>>(), [0.0, 5.0, 10.0, 15.0]);
    assert!(lines[1].tours[0].runs);

    // Ascension (a Thursday and a public holiday): the Sunday tour runs, the working-day one not
    let lines = lines_for(&vfs, map, d("1989-05-04")).unwrap();
    assert_eq!(lines[0].tours.iter().map(|t| t.runs).collect::<Vec<_>>(), [false, true]);
    // the summer holidays: tour 1 runs in the school holidays as well
    assert!(lines_for(&vfs, map, d("1989-07-04")).unwrap()[0].tours[0].runs);
    // 1996: the later Chrono folder switches line 10 off too
    assert!(lines_for(&vfs, map, d("1996-01-02")).unwrap().is_empty());
    assert!(matches!(lines_for(&vfs, "maps/Nowhere/global.cfg", d("1989-06-06")), Err(hub_omsi::ReadError::NotFound(_))));
}

#[test]
fn blocks_of_stnlinks_and_busstops_are_replaced_or_added() {
    let links_text = cfg(&ttdata().join("StnLinks.cfg")).to_text();
    let mut changed = parse_stnlinks(&CfgFile::from_str("", &links_text)).into_iter().find(|l| (l.from_id, l.to_id) == (1002, 1003)).unwrap();
    changed.length = 650.0;
    changed.entries.push([14.0, 0.0, 1.0, 50.0, -1.0, 0.0, 0.0]);
    let text = replace_block(&links_text, &changed.to_text("Grünau Markt", "Hauptstraße"));
    let links = parse_stnlinks(&CfgFile::from_str("", &text));
    assert_eq!(links.len(), 5);
    assert_eq!(links.iter().find(|l| (l.from_id, l.to_id) == (1002, 1003)), Some(&changed));
    assert_eq!(text.matches("Grünau Markt ==> Hauptstraße").count(), 1);
    assert!(text.contains("Time Table StnLinkList File") && text.contains("Depot ==> Grünau Markt"));
    // (the others as they were)
    let before = parse_stnlinks(&CfgFile::from_str("", &links_text));
    assert_eq!(links.iter().filter(|l| (l.from_id, l.to_id) != (1002, 1003)).collect::<Vec<_>>(), before.iter().filter(|l| (l.from_id, l.to_id) != (1002, 1003)).collect::<Vec<_>>());
    // a new link is added at the end
    let new = StnLink { from_id: 1003, to_id: 1002, length: 600.0, params: [1.0, 1.0, 2.0, 3.0, 0.0, 0.0], entries: vec![[30.0, 0.0, 1.0, 600.0, -1.0, 0.0, 0.0]] };
    let text = replace_block(&text, &new.to_text("Hauptstraße", "Grünau Markt"));
    let links = parse_stnlinks(&CfgFile::from_str("", &text));
    assert_eq!((links.len(), links.last()), (6, Some(&new)));
    assert_eq!(replace_block(&text, &new.to_text("Hauptstraße", "Grünau Markt")), text);

    // a stop by its object id (not its name)
    let stops_text = cfg(&ttdata().join("BUSSTOPS.cfg")).to_text();
    let renamed = BusStopEntry { name: "Grünau Mitte".into(), tile: 0, object_id: 1002, params: [2.0, 0.0, 0.0] };
    let stops = parse_busstops(&CfgFile::from_str("", &replace_block(&stops_text, &renamed.to_text())));
    assert_eq!(stops.len(), 4);
    assert_eq!((stops[0].name.as_str(), &stops[1]), ("Depot", &renamed));
    assert_eq!(replace_block(&stops_text, ""), stops_text);
}

#[test]
fn a_file_is_written_back_in_its_codepage() {
    let dir = temp("cp");
    let c = cfg(&ttdata().join("BUSSTOPS.cfg"));
    assert_eq!(c.codepage, Codepage::Windows1252);
    let p = dir.join("Busstops.cfg");
    write_text(&p, &c.to_text(), c.codepage).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), std::fs::read(ttdata().join("BUSSTOPS.cfg")).unwrap());
    let g = cfg(&map_dir().join("global.cfg"));
    assert_eq!((g.codepage, g.bom), (Codepage::Utf16Le, true));
    let p = dir.join("global.cfg");
    write_text(&p, &g.to_text(), g.codepage).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), std::fs::read(map_dir().join("global.cfg")).unwrap());
}

// --- the map ---------------------------------------------------------------------------------

#[test]
fn the_map_s_global_cfg_calendar_and_timezone_are_read() {
    let g = GlobalCfg::load(&map_dir().join("global.cfg")).unwrap();
    assert_eq!((g.name.as_str(), g.friendly_name.as_str()), ("Testdorf", "Testdorf (test map)"));
    assert_eq!(g.description, "A synthetic map for Omsi-Hub's tests.\nGrünau and Hauptstraße.");
    assert_eq!(g.tiles, [(0, 0), (1, 0)]);
    assert_eq!(g.entry_points.iter().map(|e| (e.index, e.name.as_str())).collect::<Vec<_>>(), [(0, "Depot"), (1, "Grünau Markt")]);
    assert_eq!((g.standard_depot.as_deref(), g.left_hand_traffic), (Some("Busses"), false));
    let tz = g.timezone.unwrap();
    assert_eq!((tz.offset_hours, tz.lat_lon(), tz.dst.len()), (1.0, Some((51.25, 7.5)), 1));

    let cal = Calendar::load(&map_dir()).unwrap();
    assert!(cal.is_holiday(d("1989-05-04")) && cal.is_holiday(d("1989-12-25")) && !cal.is_holiday(d("1989-05-05")));
    assert!(cal.in_holiday_range(d("1989-07-01")) && cal.in_holiday_range(d("1989-08-15")) && !cal.in_holiday_range(d("1989-08-16")));
    assert_eq!(cal.ranges[0].name, "Summer");
    assert_eq!(Calendar::load(&root()).unwrap(), Calendar::default());
    assert_eq!(date_code("1989-06-06"), Some(19_890_606));
}

#[test]
fn chrono_folders_apply_by_their_dates() {
    let names = |date: &str| active_chrono_dirs(&map_dir(), d(date)).iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    assert_eq!(names("1989-04-30"), Vec::<String>::new());
    assert_eq!(names("1989-05-01"), ["0100_Works"]);
    assert_eq!(names("1989-05-31"), ["0100_Works"]);
    assert_eq!(names("1989-06-06"), ["0200_Cut"]);
    assert_eq!(names("1996-01-02"), ["0200_Cut", "0300_Later"]);
    let works = &active_chrono_dirs(&map_dir(), d("1989-05-15"))[0];
    assert_eq!((works.title.as_str(), works.from, works.to), ("Works on the main road", Some(d("1989-05-01")), Some(d("1989-05-31"))));
    assert_eq!(chrono_deactivated_lines(&active_chrono_dirs(&map_dir(), d("1996-01-02"))), ["n1", "10"]);
    // (the folder's TTData in place of the map's, its stops added)
    let chrono = active_chrono_dirs(&map_dir(), d("1989-05-15"));
    let tt = Timetable::load_with_chrono(&ttdata(), &chrono, &[]);
    assert_eq!(tt.trip("10_out").unwrap().terminus, "Kirche (diversion)");
    assert_eq!((tt.bus_stops.len(), tt.stn_links.len(), tt.stop_name(1005)), (5, 5, Some("Umleitung")));
    let off = Timetable::load_with_chrono(&ttdata(), &[], &["N1".into()]);
    assert_eq!(off.lines.len(), 1);
}

#[test]
fn ai_lists_and_the_day_s_depot_file() {
    let a = AiLists::with_chrono(&map_dir(), &[]);
    assert_eq!(a.groups.iter().map(|g| (g.name.as_str(), g.is_depot)).collect::<Vec<_>>(), [("Cars", false), ("Busses", true)]);
    let cars = a.group("CARS").unwrap();
    assert_eq!(cars.vehicles.iter().map(|v| (v.file.as_str(), v.weight)).collect::<Vec<_>>(), [("vehicles/Car/car.bus", 70.0), ("vehicles/Van/van.ovh", 10.0)]);
    let busses = a.group("Busses").unwrap();
    assert_eq!(busses.hof.as_deref(), Some("Testdorf"));
    let t = &busses.typgroups[0];
    assert_eq!(t.file, "Vehicles/Testbus/Testbus.bus");
    assert_eq!(
        t.entries.iter().map(|e| (e.number.as_str(), e.plate.as_str(), e.paint.as_str(), e.from, e.to)).collect::<Vec<_>>(),
        [("101", "TD-V 101", "Red", None, Some(19_891_231)), ("102", "", "Blue", Some(19_890_601), None)]
    );
    assert!(t.entries[0].valid(19_890_530) && !t.entries[1].valid(19_890_530));
    // the Chrono folder's update gives the depot group another depot file and buses
    let later = AiLists::with_chrono(&map_dir(), &active_chrono_dirs(&map_dir(), d("1989-06-06")));
    assert_eq!(later.groups.len(), 2);
    assert_eq!(later.group("busses").unwrap().hof.as_deref(), Some("Testdorf 1989"));
    assert_eq!(later.group("busses").unwrap().typgroups[0].entries[0].number, "103");
    assert_eq!(depot_hof_on(&map_dir(), d("1989-05-30")).as_deref(), Some("Testdorf"));
    assert_eq!(depot_hof_on(&map_dir(), d("1989-06-06")).as_deref(), Some("Testdorf 1989"));
    assert_eq!(depot_hof_on(&root(), d("1989-06-06")), None);
}

// --- depot files -----------------------------------------------------------------------------

#[test]
fn a_depot_file_is_read() {
    assert_eq!(depot_files(&bus_dir()), [bus_dir().join("Testdorf.hof")]);
    let path = bus_dir().join("Testdorf.hof");
    assert_eq!(Hof::read_name(&path).as_deref(), Some("Testdorf"));
    let h = Hof::load(&path).unwrap();
    assert_eq!((h.name.as_str(), h.service_trip.as_str(), h.codepage), ("Testdorf", "Betriebsfahrt", Some(Codepage::Windows1252)));
    assert_eq!(h.global_strings, ["Testdorf", "Testdorf_Rollband"]);
    assert_eq!((h.string_count_terminus, h.string_count_busstop), (3, 2));
    // (the indented [addterminus] is the comment's example; a destination's name is also the stop it ends at)
    assert_eq!(h.termini.len(), 3);
    let t = |i: usize| (h.termini[i].code, h.termini[i].texture_id.as_str(), h.termini[i].terminus_stop.as_deref(), h.termini[i].all_exit, h.termini[i].strings.clone());
    assert_eq!(t(0), (0, "Leerfahrt", None, true, vec!["LEERFAHRT".into(), "".into(), "Blank.tga".into()]));
    assert_eq!(t(1), (104, "Kirche", Some("Kirche"), false, vec!["KIRCHE".into(), "   KIRCHE   ".into(), "Kirche.tga".into()]));
    assert_eq!(t(2), (101, "Depot", Some("Depot"), false, vec!["DEPOT".into(), "DEPOT".into(), "Depot.tga".into()]));
    assert_eq!(
        h.bus_stops.iter().map(|b| (b.ident.as_str(), b.strings.clone())).collect::<Vec<_>>(),
        [("Grünau Markt", vec!["GRÜNAU MARKT".to_string(), "Grünau Markt".to_string()]), ("Kirche", vec!["KIRCHE".to_string(), "Kirche".to_string()])]
    );
    assert_eq!(
        h.info_trips.iter().map(|t| (t.code.as_str(), t.name.as_str(), t.route.as_str(), t.line.as_str())).collect::<Vec<_>>(),
        [("1001", "DEPOT-KIRCHE", "104", "10"), ("1002", "KIRCHE-DEPOT", "101", "10")]
    );
    assert_eq!(h.info_busstop_lists[0], ["Depot", "Grünau Markt", "Hauptstraße", "Kirche"]);
    assert_eq!(h.info_busstop_lists[1], ["Kirche", "Depot"]);
    assert!(h.has_line("10") && !h.has_line("11"));
    assert_eq!(h.terminus(104).map(|t| t.texture_id.as_str()), Some("Kirche"));
    assert!(h.bus_stop("grünau markt").is_some());
    let c = Comments::read(&cfg(&path));
    assert_eq!(c.head, ["Depot file of Testdorf (synthetic)", "for Omsi-Hub's tests"]);
    assert_eq!(c.terminus, ["IBIS display", "Front, max 16 Zeichen", "Roller blind"]);
    assert_eq!(c.busstop, ["IBIS name", ""]);
}

#[test]
fn a_depot_file_reads_the_same_after_writing_back_in_its_codepage() {
    let path = bus_dir().join("Testdorf.hof");
    let h = Hof::load(&path).unwrap();
    let c = Comments::read(&cfg(&path));
    let bytes = h.to_bytes(&c);
    assert!(bytes.windows(4).any(|w| w == b"Gr\xFCn"), "written in Windows-1252");
    let again = CfgFile::from_bytes(&path, &bytes);
    assert_eq!(Hof::parse(&again), h);
    assert_eq!(Comments::read(&again), c);
    // saved and loaded
    let dir = temp("hof");
    let p = dir.join("Copy.hof");
    h.save(&p, &c).unwrap();
    assert_eq!(Hof::load(&p).unwrap(), Hof { path: p.clone(), ..h.clone() });
    // a new file with other comments and a changed destination
    let mut h2 = h.clone();
    h2.termini[1].strings[0] = "KIRCHE NORD".into();
    h2.termini.push(hub_omsi::hof::Terminus { code: 7, texture_id: "Nord".into(), terminus_stop: Some("Nordstraße".into()), all_exit: false, strings: vec!["NORD".into()] });
    let c2 = Comments { head: vec!["Made by a test".into(), "[name] in a comment".into()], terminus: vec!["".into(), "Front".into(), "".into()], busstop: vec!["".into(), "".into()] };
    let back = Hof::parse(&CfgFile::from_str("x.hof", &h2.to_text(&c2)));
    assert_eq!(back.termini[3].strings, ["NORD", "", ""]);
    assert_eq!(back.termini[1].strings[0], "KIRCHE NORD");
    assert_eq!((back.name.as_str(), back.termini.len()), ("Testdorf", 4));
    assert_eq!(Comments::read(&CfgFile::from_str("x.hof", &h2.to_text(&c2))), c2);
}

// --- a bus -----------------------------------------------------------------------------------

#[test]
fn a_bus_is_read_with_its_paints_numbers_and_cabin() {
    let b = Bus::load(&bus_dir().join("Testbus.bus")).unwrap();
    assert!(b.has_friendly_name && !b.ai_only && !b.rail && !b.is_rear_section());
    assert_eq!((b.manufacturer.as_str(), b.type_name.as_str(), b.default_paint.as_str(), b.name()), ("Testwerk", "TW 12 Ü", "Red", "Testwerk TW 12 Ü".to_string()));
    assert_eq!(b.description, "A synthetic test bus.\nLänge: 17,9 m");
    // (the indented [registration_free] is switched off)
    assert!(!b.registration_free);
    assert_eq!(b.paints, ["Blau", "Red"]);
    assert_eq!(b.ctc_dir.as_deref(), Some("Texture/Paints"));
    assert_eq!(b.numbers, [("101".to_string(), "TD-V 101".to_string()), ("102".to_string(), "TD-V 102 S".to_string()), ("103".to_string(), "TD-V 103".to_string())]);
    assert_eq!(b.registration_list.as_deref(), Some("Plates.org"));
    assert_eq!((b.model.as_deref(), b.sound.as_deref(), b.sound_ai.as_deref()), (Some("model/model.cfg"), Some("sound/sound.cfg"), Some("sound/sound_ai.cfg")));
    assert_eq!(b.scripts.scripts, [PathBuf::from("script/main.osc"), PathBuf::from("script/engine_diesel.osc")]);
    assert_eq!((b.scripts.constfiles.len(), b.scripts.varlists.len(), b.scripts.string_varlists.len()), (1, 1, 1));
    assert_eq!((b.driver_camera_x, b.left_hand_drive()), (Some(-0.8), Some(true)));
    assert_eq!(b.bounding_box, Some([2.5, 17.9, 3.0, 0.0, -2.5, 1.5]));
    assert_eq!(b.couple_back, Some(("testbus_TRAIL.bus".to_string(), false)));
    // (its files found in any case)
    let trail_path = b.couple_back_path().unwrap();
    assert_eq!(trail_path.file_name().unwrap(), "Testbus_trail.bus");
    let trail = Bus::load(&trail_path).unwrap();
    assert!(trail.is_rear_section() && !trail.has_friendly_name);
    assert_eq!(trail.paints, ["Blau", "Red"]);
    assert!(trail.passenger_cabin_path().is_some());

    let cabin = PassengerCabin::load(&b.passenger_cabin_path().unwrap()).unwrap();
    assert_eq!((cabin.seats(), cabin.standing(), cabin.entries, cabin.exits), (3, 2, 2, 1));
    assert_eq!(cabin.places[0].pos, [-0.8, 3.4, 1.2]);
    // seats and standing places of both sections
    assert_eq!(capacity(&b), Some((5, 5)));
    assert_eq!(capacity(&Bus::default()), None);
}

#[test]
fn a_bus_s_paints_are_every_item_of_every_paint_folder_in_every_content_folder() {
    let (root, content) = (temp("paints-root"), temp("paints-content"));
    let w = |base: &Path, rel: &str, text: &str| {
        let p = base.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    w(&root, "Vehicles/Bus/bus.bus", "[friendlyname]\r\nMaker\r\nCity\r\nStock\r\n\r\n[model]\r\nmodel\\model.cfg\r\n");
    w(&root, "Vehicles/Bus/model/model.cfg", "[CTC]\r\nColorscheme\r\nTexture\r\n4\r\n\r\n[CTC]\r\nColorscheme\r\n..\\Shared\\Paints\r\n4\r\n");
    // (two liveries in one file; a file of the content folder hides the one of its name here)
    w(&root, "Vehicles/Bus/Texture/a.cti", "[item]\r\nStock\r\n\r\n[item]\r\nNight\r\n");
    w(&root, "Vehicles/Bus/Texture/b.cti", "[item]\r\nOld\r\n");
    w(&root, "Vehicles/Shared/Paints/c.cti", "[item]\r\nShared\r\n");
    // (a repaint a mod put into openOMSI's content folder)
    w(&content, "Vehicles/Bus/Texture/b.cti", "[item]\r\nNew\r\n");
    w(&content, "Vehicles/Bus/Texture/mine.cti", "[item]\r\nMy livery\r\n");
    let bus = Bus::load(&root.join("Vehicles/Bus/bus.bus")).unwrap();
    assert_eq!(bus.ctc_dirs, ["Texture", "../Shared/Paints"]);
    assert_eq!(bus.paints, ["Stock", "Night", "Old", "Shared"]);
    let vfs = Vfs::new(root.clone(), Some(content.clone()));
    assert_eq!(bus.paints_in_vfs(&vfs), ["New", "My livery", "Stock", "Night", "Shared"]);
    // (the bus itself in the content folder, its model in the OMSI 2 folder)
    w(&content, "Vehicles/Bus/bus.bus", "[friendlyname]\r\nMaker\r\nCity\r\nStock\r\n\r\n[model]\r\nmodel\\model.cfg\r\n");
    let moved = Bus::load(&content.join("Vehicles/Bus/bus.bus")).unwrap();
    assert_eq!(moved.paints_in_vfs(&vfs), ["New", "My livery", "Stock", "Night", "Shared"]);
}

// --- fonts and the weather -------------------------------------------------------------------

#[test]
fn a_font_file_names_its_fonts() {
    assert_eq!(font::load_names(&root().join("Fonts/Testfont.oft")).unwrap(), ["Test Matrix 8", "Test Matrix 16 Ü"]);
}

#[test]
fn weather_files_are_read_and_written_back_in_their_codepage() {
    let grey = Weather::load(&root().join("Weather/Grey.owt")).unwrap();
    assert_eq!((grey.name.as_str(), grey.description.as_str(), grey.codepage), ("Grauer Tag", "Regen und Wind, kühl.", Some(Codepage::Windows1252)));
    assert_eq!((grey.fog_m, grey.temperature, grey.dew_point, grey.pressure, grey.wind_dir, grey.wind_speed), (200.0, 11.0, 9.5, 997.0, 230.0, 4.3));
    assert_eq!((grey.clouds.as_str(), grey.cloud_base_m, grey.precipitation), ("Overcast 1", 200.0, Precipitation::Rain));
    assert!((grey.intensity - 173.0 / 255.0).abs() < 1e-6 && (grey.ground_wet - 78.0 / 255.0).abs() < 1e-6);
    assert!(!grey.snow && !grey.snow_on_road);

    let snow = Weather::load(&root().join("Weather/Snow.owt")).unwrap();
    assert_eq!((snow.name.as_str(), snow.codepage, snow.precipitation, snow.clouds.as_str()), ("Schnee", Some(Codepage::Utf16Le), Precipitation::Snow, ""));
    assert!(snow.snow && snow.snow_on_road && (snow.intensity - 187.0 / 255.0).abs() < 1e-6 && snow.ground_wet == 0.0);

    let dir = temp("owt");
    for w in [grey, snow] {
        let p = dir.join(w.path.file_name().unwrap());
        w.save(&p).unwrap();
        assert_eq!(hub_omsi::text::detect(&std::fs::read(&p).unwrap()), w.codepage.unwrap());
        assert_eq!(Weather::load(&p).unwrap(), Weather { path: p.clone(), ..w });
    }
}
