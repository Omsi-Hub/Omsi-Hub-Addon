# Architecture

Omsi-Hub is its own desktop app. It works with the openOMSI the player installed (0.2.22 or newer)
through openOMSI's command line and its plugin API, and reads OMSI 2's files with its own readers -
except the bus as the game reads it (its meshes, textures, liveries and the look its scripts give
it) for the 3D view, the photos and the livery studio: that is read with openOMSI's own readers
and vehicle simulation, from Luc's fork of openOMSI (git dependencies pinned in `Cargo.toml`;
openOMSI as it was before it became AGPL, under its MIT licence: `THIRD_PARTY_NOTICES.md`, Luc's
decision of 2026-10-18). See `PLAN.md` for why, `INVENTORY.md` for what is ported.

It runs on **Windows, macOS and Linux** from the first version. The platform specifics are
collected under [Platforms](#platforms).

## The parts

```
app/            the desktop shell: Tauri 2 (stock), the window, the commands, the bytes route
ui/             the page: Preact + htm (vendor/), our api.js, store.js, i18n.js, the pages
crates/
  hub-omsi      our OMSI 2 readers and writers (no openOMSI code)
  hub-core      the domain: the company, RosterFlow, careers; our timetable types; data folder
  hub-api       the commands: call(cmd, args) routed by prefix; the 3D view's bus (content/bus3d,
                b3d_*) and the livery studio (lak, lak_*) on openOMSI's readers (omsi-*)
  hub-livery    the livery studio's core (the old omsihub-livery): projects, bake, paint, export
  hub-game      the player's openOMSI: found, checked, started; the plugin installed; the bridge
  hub-locales   the texts (ported as they were)
  hub-online    the account, friends, leaderboard, cloud saves (ported as they were)
  hub-companion the phone and tablet companion's server (docs/COMPANION.md)
  hub-devices   a bus's devices (IBIS, ticket printer, any screen with keys or touch fields) read
                from its model.cfg, .o3d parts, textures and OMSI's fonts: the forms the app draws
                (a port of Luc's career add-on, github.com/Luc-nbr/Omsi-career-addon)
plugin/         the Lua plugin that runs inside openOMSI; harness/ tests it on mlua
fixtures/       fake openOMSI and OMSI 2 folders for the setup's tests
```

Dependencies go one way: `app → hub-api → hub-core → hub-omsi`, and `hub-api → hub-game,
hub-locales, hub-online, hub-companion`; `hub-companion → hub-game, hub-devices`. `hub-omsi`,
`hub-game` and `hub-devices` depend on nothing of ours (`hub-devices` decodes textures with
openOMSI's `omsi-texture`).

## The contracts

**`hub-omsi`** (the readers):

- `text`: `Codepage` with `detect`, `decode`, `decode_text` and `encode`; `CfgFile { path,
  lines, codepage, bom }` with `from_bytes`, `read`, `reader()`, `blocks(kw)`, `value(kw)` and
  `to_bytes()`; `Reader` with `next_keyword`, `str`, `value`, `values(n)`, `run`, `i32`, `f64`
  and `until`; and `parse_i32`/`parse_f64`.
- `vfs`: `Vfs::new(root, content)` and `Vfs::layered(first, root)`. It has `resolve(rel)`,
  `read`, `read_cfg`, `list_dir`, `is_file`/`is_dir` and `relative`. It searches the content
  folder first, then the archives openOMSI uses in place (`hub_core::paths::archive_bases`:
  what Omsi-Hub reads of each `Archives/*.zip`, copied into its cache), then OMSI 2, in any
  case, with either slash. Also `find_child`, `resolve_in` and `missing_omsi2_essentials`.
  The openOMSI whose content folder is read is the one openOMSI's own launcher plays
  (`~/.openomsi/launcher.json`'s `game`), else the one chosen in the setup
  (`hub_core::paths::openomsi_folder`).
- `timetable`:
  - the types `Timetable { lines, trips, tracks, stn_links, bus_stops }`, `Line`, `Tour`
    (`days: DayMask`), `TourTrip`, `Trip`, `TripStop`, `TripProfile`, `Track`, `TrackEntry`,
    `StnLink` and `BusStopEntry`;
  - their `parse`/`to_text`, `parse_stnlinks`, `parse_busstops`, `replace_block` and
    `stop_times`;
  - `write_text(path, text, codepage)`;
  - the day: `lines_for(&Vfs, map, NaiveDate) -> Vec<ServiceLine>`, with `ServiceTour`,
    `ServiceTrip` and `ServiceStop`.
- `map`: `GlobalCfg`, `Calendar` (`Holidays.txt`: `is_holiday`, `in_holiday_range`),
  `ChronoDir`, `active_chrono_dirs`, `chrono_deactivated_lines`, `AiLists`/`AiGroup`/
  `TypGroup`, `depot_hof_on`, `date_code`; `GlobalCfg.tile_files`, `version`,
  `world_coordinates` and `tile_edge()`.
- `roads`: a map's roads from its tiles (`[spline]`, `[spline_h]`, `[object]`,
  `[splineAttachement]`) and their `.sli` lanes (`[path]`), as openOMSI's `docs/FORMATS.md`
  describes the files: `MapRoads::read`, `roads()` (each spline's street lanes as a line with its
  width), `route(track)` (a `.ttr`'s lanes in their directions, a junction object bridged
  straight), `object(tile, id)` (a stop's place) and `joins()` (how many splines meet their next
  one: the check of the geometry on a real map, logged when the app reads a map; the ignored
  `real_data` test asserts it on an installed OMSI 2).
- `hof`: `Hof { name, service_trip, global_strings, string_count_*, termini, bus_stops,
  info_trips, info_busstop_lists }` with `Terminus`, `BusStop`, `InfoTrip` and `Comments`;
  `load`, `parse`, `read_name`, `to_text`, `has_line`, `depot_files`.
- `vehicle`: `Bus` (the `.bus` header) and `PassengerCabin` (`seats`, `standing`).
- `font::font_names` (`.oft`) and `weather::Weather` (`.owt`).

**All of it works** (the hub-omsi port), with tests on a synthetic OMSI 2 folder
(`crates/hub-omsi/tests/fixtures`) and an ignored test on a real install
(`HUB_OMSI_REAL_ROOT=<OMSI 2 folder> cargo test -p hub-omsi --test real_data -- --ignored`).
The field names follow what our old code used, so a port mostly changes its `use` lines.
Renamed:

| Old | New |
|---|---|
| `Tour.extra` (the day mask as text) | `Tour.days: DayMask` |
| `Trip.stations`, `stations_legacy` | `Trip.stops: Vec<TripStop>` |
| `TripProfile.factor` | `minutes` |
| `man_dep_time`, `man_arr_time`, `other_stopping` | `fixed_departures`, `fixed_arrivals`, `stopping` |
| `BusStopEntry.group` | `tile` |
| `Vehicle` | `vehicle::Bus` |
| `PassengerCabin.pass_positions` | `places` |
| Calendar and Chrono `i32` dates | `NaiveDate` (`date_code` still gives `YYYYMMDD`) |
| `Vehicle::cameras_driver[0].pos[0]` | `Bus::driver_camera_x` (and `left_hand_drive()`) |
| `Vehicle::scripts.{scripts, constfiles, varlists}` | `Bus::scripts.{scripts, constfiles, varlists}` (`BusScripts`, paths with `/`) |
| `Vehicle::registration_list: Option<(file, _, _)>` | `Bus::registration_list: Option<String>`, `registration_automatic`, `registration_free` |
| `Vehicle::name`, `numbers_with_plates()` | `Bus::name()`, `Bus::numbers` (filled by `Bus::load`) |
| `typgroup_entry_valid(e, d)` | `TypGroupEntry::valid(d)` |
| `ailists_with_chrono(dir, chrono)` | `AiLists::with_chrono(dir, chrono)` |
| `GlobalCfg.raw_tiles` | `GlobalCfg.tiles` |
| `omsi_map::TimeZone` | `map::TimeZone` (`GlobalCfg::load` fills `GlobalCfg.timezone`) |

**Added by the hub-omsi port** (nothing renamed or taken away):

- `text`: `Codepage::Utf16Le`/`Utf16Be` (maps' `global.cfg` and some weather files are UTF-16),
  `Codepage::bom()`/`is_utf16()`; `Reader::value`, `values(n)` and `run` (values that stop at
  the next keyword). **A keyword counts only at the start of its line** (`keyword_of`), as OMSI
  reads it: depot and bus files keep indented examples (`<TAB>[infosystem_trip]`) and
  switched-off keywords (`<TAB>[registration_free]`) in their comments.
- `timetable`: `DayMask::runs_on(&Calendar, date)` and the constants `HOLIDAY`, `SCHOOL_DAYS`,
  `SCHOOL_HOLIDAYS`, `ALWAYS`; `Trip.train_reverse`; `TripStop.extra` (`[station]`'s values
  after the name); `Trip::load`, `Track::load`; `Timetable::stop_name`, `track_of`;
  `leg_lengths`, `trip_length`, `service_lines`, `parse_i64`.
- `map`: `GlobalCfg.timezone` and `TimeZone`/`Dst` (`timezone.txt`); `date_of_code`;
  `ChronoDir.deactivate_lines`, `.title`, `ChronoDir::read`, `active_on`;
  `TypGroupEntry.plate`; `AiLists::update`, `depot_group`.
- `hof`: `Comments::read(&CfgFile)`; `Hof::to_bytes`, `save`, `terminus(code)`, `bus_stop(name)`.
- `vehicle`: `Bus.has_friendly_name`, `paints`, `ctc_dir`, `scripts: BusScripts`, `sound_ai`,
  `driver_camera_x`, `coupling_front`, `number_list`, `registration_automatic`,
  `registration_free`; `Bus::name`, `read_numbers`, `couple_back_path`, `passenger_cabin_path`,
  `left_hand_drive`; `paints_in(dir)`; `capacity(&Bus)` (seats and standing places of all its
  sections).
- `weather`: `Weather.fog_height`, `wind_dir`, `wind_speed`, `dew_point`, `pressure`,
  `cloud_base_m`, `precip_values`, `ground_wet_values`, `snow_on_road`, `codepage`;
  `Weather::to_text`, `save`; `Precipitation::code`/`of_code`.

**What the files turned out to mean** (checked on the OMSI 2 maps, buses and depot files
installed here):

- A tour's day mask: bits 0-6 Monday to Sunday, bit 7 public holidays, bit 8 school days, bit 9
  school holidays. It runs when the day's bit (the public holiday's on one, else the weekday's)
  and its school bit are set; a mask without school bits (older maps) ignores the school
  calendar.
- Chrono folders apply in the order of their names, from `[startdate]` to `[enddate]`. Their
  `.ttl`/`.ttp`/`.ttr` take the place of the map's; their `StnLinks.cfg`/`Busstops.cfg` hold
  only what they add (a link of the same two stops, a stop of the same id, replaced); their
  `ailists.cfg` replaces the map's, their `ailists_#upd.cfg` replaces its groups of the same
  name; `[deactivate_lines]` switch lines off.
- `[standarddepot]` in `global.cfg` names the AI depot group (`ailists.cfg`) whose depot file
  the player's bus gets: `depot_hof_on` takes it, else the first depot group.
- A `.ttp`'s `[trip]` names its track first (empty: the track named like the trip):
  `Trip.display_name` is that value as written, `Trip.track` the track driven.
- In a `.hof`, `stringcount_terminus` and `stringcount_busstop` have no brackets;
  `[addterminus_list]`/`[addbusstop_list]` hold one entry a line, tab-separated; older files
  give a route's stops as `[infosystem_busstop]` one by one.
- `replace_block` tells a `[StnLink]` by its two stops and a `[busstop]` by its object id
  (their first values are a length and a name). The old `replace_block(&text, "")`, which took
  the line editor's marked part out, has no counterpart: an empty block leaves the text as it is.

**`hub-core`** (the domain):

- `timetable::{LineInfo, TourInfo, TripInfo, StopInfo}`: our own, with the old JSON field names
  (the saves and the pages read them). `list_lines(map, date)` builds them from
  `hub_omsi::timetable::lines_for`, and `DEFAULT_DATE` comes with them.
- `Duty`: a drive as the page sends it, with the old JSON.
- `paths`: `data_dir()`, `root()`, `content_dir()`, `vfs()` and `resolve_content(rel)`.
- `settings::Settings` (`settings.json`).
- `log_line`.
- `migrate::{scan, import}`.
- `display_bus_name` and `vehicle_type_label`.
- `company`: the bus company, RosterFlow, careers (ported), with `lines`, `linehof`,
  `owndepot`, `depot`, `service`, `compose`, `busoptions`, `busfonts`, `qr`, `ttstore`; `trips`
  (the drivers' trip records), `vehicles` (the installed buses) and `ttrun` (a trip's stop times
  and IBIS codes).

**`hub-api`** (the commands):

- `call(cmd, &Value, &dyn Platform) -> Result<Value, String>`.
- `call_bytes(cmd, &Value, &dyn Platform) -> Result<Vec<u8>, String>`.
- `Platform { emit, quit, emitter }`.
- `arg::<T>(args, name)`, `to_json` and `not_ported(cmd)`.

| Prefix | Module | Now |
|---|---|---|
| `hub_` | `hub`, `drive`, `live`, `content` | Boot, texts, first-run setup, migration, drives and their live dashboard; the drive's steps (`content`: maps, buses, lines, shifts, weather, depots, bus options, fonts, the route map's roads) |
| `addon_` | `addon` | The plugin: state, folder, version, enable/remove, texts, assignment, reports booked, the bridge looked at |
| `hub_companion_` | `companion` | The phone and tablet companion: on/off, the pairing code and QR code, the paired devices; started with the app when on |
| `hub_mods` | `mods` | Mods installed into the content folder of the openOMSI in use with openOMSI's own installer (`omsi-launcher-core`'s `install`, noted in `Mods/.mods.json` as openOMSI's launcher notes them), followed while they install, switched off, on and deleted (`mods`) |
| `co_` | `company` | Ported (`co_more_*` too); the fleet map's and the duty map's roads from `hub_omsi::roads`; bus photos: kept by `co_photo_save`, taken by the page from the 3D model (`ui/js/hub/photos.js`) |
| `rf_` | `rosterflow` | Ported |
| `tg_` | `together` | Later (needs a relay) |
| `ol_` | `online` | Ported |
| `cs_` | `cloud` | Ported |
| `tt_` | `timetable` | Ported: a map's lines and tours, saved into the content folder's copy of its `TTData`, and the reset |
| `de_` | `depot` | Ported: the player's own depot files (`hub_core::owndepot`), one open at a time |

**`hub-game`** (the game):

- `detect::inspect(folder, home) -> Install { exe, version, version_from, plugins_dir }`.
- `check_folder(folder, home) -> Checked { install, compat }`, and `check_folder_probing(folder,
  home, cache)`, which asks `openomsi --version` once when no file says the version.
- `compat::{FIRST_SUPPORTED = 0.2.22, estimate, negotiate, check}`.
- `install::{install, disable, status, plugin_dir}`.
- `bridge::{Assignment, Leg, Hello, Report, Dash, write_assignment, read_hello, read_dash, ReportReader, ReplyReader, from_log}`.
- `launch::LaunchSpec::args()` (official flags only, `OFFICIAL_FLAGS`); `export_glb` adds `--export-glb <file>`.
- `process::export_glb(exe, spec, wait)`: the bus written as glTF by the player's openOMSI (not used by the pages: the 3D view reads the bus itself, `content::bus3d`).
- `process::{start, Game, ask_version, ensure_executable}`.
- `find::{omsi2_candidates, openomsi_candidates, steam_roots, steam_libraries, xdg_user_dir}`.
- `gamesettings::{FIELDS, GROUPS, path, value, checked, with, set}`: openOMSI's own `~/.openomsi/settings.cfg`, the keys its user guide names (`addon_game_settings`, `addon_game_setting`).

**`hub-companion`** (the phone and tablet companion, docs/COMPANION.md):

- `Companion::start(Config, Arc<dyn Source>)`, `status()`, `forget(id)`, `new_code()`; dropping it
  stops it. `Source` is what the add-on knows: the plugin's folder, the last assignment, and the
  map's routes and roads (hub-api's `companion`, on `content::mapview::{route_line, roads_near,
  world_of}`).
- `http` (requests, routes, the commands a device may send), `pairing` (six-digit codes, device
  keys kept as SHA-256 in `companion-devices.json`), `heard` (the plugin's datagrams, paged lists
  put together), `queue` (`data/companion.json`), `state` (the state a device sees, the break
  between trips), `nav` (the next turn on the trip's route), `devices` (the bus's devices for
  `state.devices`, their forms and pictures by id, the stands from the plugin's `scr`, the forms
  kept in `companion-forms/` of the data folder), `server` (the threads). `Source::bus_model`
  gives the `model.cfg` of the bus the plugin names and the game folder it is read in.

**`hub-devices`** (a bus's devices as the game draws them; a port of the device pipeline of Luc's
career add-on for OMSI 2, `github.com/Luc-nbr/Omsi-career-addon`, rule by rule, in English):

- `cfg` (`schermcfg.ts`: `model.cfg` read as Omsi.exe reads it - whole-line headings, the mesh
  index of existing `.o3d`s only, `[useTextTexture]`'s index, the `[newanim]` chains, the
  materials), `o3d` (`o3d.ts`, encrypted files included), `text` (Windows-1252, UTF-16, CR/LF);
- `modules` (`busmodule.ts`: the devices grouped from their parts' names, the keys' labels and
  roles) and `layout` (`busvorm.ts`/`busscherm.ts`: the keys in the rows they have on the device);
- `form` (`schermvorm.ts`: `needed_numbers`, then `build` - the anim chains with the scripts'
  start values (`scripts`), the anchors' vote on the background, the flat piece, (s, t) on its
  plane, triangles facing the driver, layers, text fields, touch fields, the keys off the screen
  within `OFF_THE_SCREEN_MM`, depth order, the id from the content);
- `textures` (`schermtextuur.ts`/`textuur.ts`: names that cannot leave the folder, OMSI's search
  order, ids of 20 hex characters, the material's alpha rules, PNG), `ctc` (`kleurstelling.ts`:
  repaints), `fonts` (`oft.ts`: `.oft` and the raw bitmaps), `protocol` (the form and the stand
  as the app reads them), `testbus` (a synthetic bus for the tests and the fake game).

## The data folder

Omsi-Hub keeps its data in its own folder, never in `~/.openomsi`:

- `%APPDATA%\Omsi-Hub` on Windows;
- `~/Library/Application Support/Omsi-Hub` on macOS;
- `$XDG_DATA_HOME/Omsi-Hub` on Linux.

`OMSIHUB_DATA_DIR` moves it (the tests and the selftest use that). It holds `settings.json` (the
two folders, the language, `setup_done`, `migrated`) and `omsi-hub.log`. The ports add the
company saves here under the old file names (`companies/`, `careers/`, `trips/`, `lines/` …).

The old Omsi-Hub's saves in `~/.openomsi` are found by `hub_core::migrate::scan`, which only
reads (`OMSIHUB_OLD_DATA_DIR` points it elsewhere: the tests and the selftest use
`fixtures/old-openomsi`). The shell offers them once after the setup (`setup.js`
`MigrateOffer`), with what `migrate::preview` found: the companies by name, the careers, the
own lines, the depot files, the settings taken over and the driver. `import` copies them 1:1
into the data folder (never moves or changes the old ones); from the old `settings.cfg` it takes
the keys the new app still uses (`migrate::OWN_KEYS`: the look, the company's layout, the
navigator's `nav_*`, `ibis_auto`, the updates, `bus_lead` …, made tidy as the old launcher read
them) and the language; `launcher.json`'s profile becomes the settings' `driver`.

## Finding and starting openOMSI

1. **The first-run setup** (`ui/js/setup.js`, the `hub_setup_*` commands):
   - The player picks the openOMSI folder, or one `find` offers.
   - `hub_game::check_folder` finds the program (`openomsi.exe`, `openomsi` or the macOS bundle,
     in any case).
   - It reads the version without starting the game where a file says it: the Windows
     `ProductVersion`, the bundle's `Info.plist`, or the folder's name
     (`openOMSI-0.2.30-linux-x64`).
   - Otherwise it asks `openomsi --version` once. openOMSI answers that from its command-line
     parser and ends before any of the game starts. The answer is kept per program file in
     `openomsi-versions.json` in the data folder. This is the usual case on Linux.
   - `compat` refuses anything older than 0.2.22, with a message.
   - The OMSI 2 folder must have `Omsi.exe`, `maps`, `Vehicles`, `Sceneryobjects` and
     `Splines`.
   - Both folders are saved in `settings.json`.
2. **The plugin**: `addon_enable` installs `plugin/*.lua` (embedded at build time) into
   openOMSI's `Plugins/omsihub/`, switches it on and parks it off. The content folder follows
   openOMSI's rule, the same on every system:
   - the program's folder (beside the bundle on macOS), or `openOMSI/` inside the OMSI 2
     folder;
   - `~/.openomsi/content` when that folder may not be written to (Program Files, `/opt`,
     `/Applications`).

   `hub_core::paths::content_dir_of` finds the same folder without writing anything.
3. **A drive** (`hub_drive {duty}`; a company's duty through `co_act` `launch_start`, which
   first hands the company's part to `company::plugin::assign_duty`):
   - The page's `Duty` becomes a `LaunchSpec` (`hub_api::drive::launch_spec`): `--root`,
     `--map`, `--bus`, `--line`/`--tour`/`--trip`, `--time`, `--date` … official flags only. A
     duty of parts (a shift, a company's duty) puts its first part there (`--whole-tour`);
     entry -1 is `--auto-entry`. `--company`, `--duty-leg` and `--free-line` are never used.
   - The plugin is made ready (`addon::ready`: put in or updated unless the player parked it
     off) and given the assignment (`addon::give`, `bridge::Assignment`): the company and its
     duty, the legs (`bridge::Leg`, from `compose`'s `line|tour|first@stop|trips@stop`; the
     plugin's `legs.lua` chains them with `duty.start`, takes a relief's first trip from its
     stop and ends the duty after the last part), the free route, the sheet, controls, fare,
     contract and crew, the display font and the IBIS (off where the game lacks them: `compat`
     says so), the navigator's settings and the sign-on's personnel number.
   - The game starts in its own folder (`process::start`, which sets the executable bit a ZIP
     may have lost).
   - **The bridge's loop** (`drive::watch`), every 2 s while it runs and once at its end:
     `addon::book` reads `data/reports.jsonl` on (per plugin folder, kept in `addon.json`; a new
     folder from its end; the game's log for a plugin without file functions) and books:

     | Report | Booked into |
     |---|---|
     | `trip` | the driver's record `trips/<driver>.jsonl` (the careers and the company's day read it; the career is read again, a driving test judged) |
     | `note`, `defect`, `fault` of a company's duty | the company's orders `companies/<id>.orders.jsonl` (carried out by the open company: `remote::take`) |
     | `live` (a `LiveEvent`) | the company's live file `companies/<id>.live.jsonl` |
     | `duty_start`, `duty_end` | the log |

     The open company takes `data/replies.jsonl` (the messages) and `data/status.json` (service
     control's dispatcher) itself (`company::messages::tick`) and writes `data/messages.json`;
     the on-time brief goes to `data/ontime.json` from `data/live.json`
     (`company::ontime::refresh`); notices go to `data/notices.json` as they come
     (`addon::notify_game`); the plugin's texts are `texts.lua`.
   - **The live dashboard** (`hub/dash.js`, `hub-api`'s `live.rs`): the page gets `game-started`
     (with the way it is driven: `company`, `shift`, `tour`, `free`) for every drive and shows
     the dashboard in place of the launcher while the game runs. The plugin writes
     `data/dash.json` every two seconds on any drive (`dash.lua`, `bridge::Dash`: the bus's
     speed, place and passengers, the duty under way and its trip's stops, what the drive came
     to); the bridge's loop samples it (`live::sample`: the speed and delay over the drive, the
     way the bus went, the notices and messages); `hub_live` gives the view, with the duty's plan
     from the map's timetable of the day (`timetable_for`, the legs' lengths for the distance to
     each stop), a company's sheet (signing on and off, breaks) and the driver's week and month
     from `trips/<driver>.jsonl`.
   - When it ends, the page gets `game-ended` (`code`, `seconds`, and what was `booked`).
   - `crates/hub-api/tests/drive.rs` drives a company's duty end to end against a fake openOMSI
     (`tests/support/fake_openomsi.rs`, Cargo's `fake-openomsi` program) that writes the
     plugin's files and ends.

## How a feature port plugs in

1. **Data and logic** go into `hub-core` (`company/…`). OMSI 2 files come from `hub-omsi`
   (fill in the `todo!()` readers you need, with synthetic tests), never from openOMSI.
2. **Commands** go into the prefix's module in `hub-api` (`co_` → `company.rs` …):
   - replace `not_ported` with a `match cmd`;
   - read the arguments with `arg`;
   - answer with `to_json`.
   Bytes answers go in `call_bytes`, and the page shows them with `bytesUrl(cmd, args)`.
3. **The page**:
   - a module under `ui/js/` (`ui/js/company/…`), its entry in `ui/js/pages/index.js`;
   - it calls `call` from `api.js`, keeps state through `store.js` (`set`, `useStore`, under its
     own key) and translates with `t` from `i18n.js`;
   - its CSS uses the `--hub-*` tokens (`ui/css/tokens.css`);
   - its mock answers go in `ui/js/mock.js`, so it opens in a browser (`node tools/serve-ui.mjs`,
     then `http://localhost:5174/?mock`).
4. **Texts** go into `crates/hub-locales/locales/*.yml` (English key, then translations).
5. **Checks**:
   - Rust tests beside the code;
   - page checks in `ui/js/selftest.js` (`CHECKS`), run in the app with `OMSIHUB_SELFTEST=1`.
     `OMSIHUB_SELFTEST_FIXTURES=<repo>/fixtures` also goes through the first-run setup.
   - The plugin's behaviour in `plugin/harness/tests/`.

## Building

```
cargo test --workspace          # every crate, the plugin harness, the app's unit tests
cargo build -p omsi-hub         # the app (target/debug/omsi-hub)
cd app && npx @tauri-apps/cli@2 build   # the packages: NSIS; .app/.dmg; AppImage, .deb, .rpm
```

The toolchain is pinned in `rust-toolchain.toml`. CI (`.github/workflows/ci.yml`) runs on
Windows, macOS and Linux (ubuntu-latest):

- fmt, clippy and the tests;
- `node --check` on the page's scripts;
- a Tauri build. On Linux it also makes the AppImage, `.deb` and `.rpm`, and runs the app's
  selftest through the first-run setup under Xvfb.

## Platforms

| | Windows | macOS | Linux |
|---|---|---|---|
| Packages | NSIS installer, with WebView2's bootstrapper | `.app`, `.dmg` | AppImage (the updater's), `.deb`, `.rpm` (`app/tauri.linux.conf.json`) |
| Web view | WebView2 | WKWebView | WebKitGTK 4.1 |
| Data folder | `%APPDATA%Omsi-Hub` | `~/Library/Application Support/Omsi-Hub` | `$XDG_DATA_HOME/Omsi-Hub` (`~/.local/share/Omsi-Hub`) |
| openOMSI's program | `openomsi.exe` | `openOMSI.app/Contents/MacOS/openomsi` | `openomsi`, from the `openOMSI-<version>-linux-x64.zip` release |
| Its version | `ProductVersion`, else the folder's name | `Info.plist`, else the folder's name | the folder's name, else `openomsi --version` (once) |
| Bytes route | `http://hub.localhost/bytes/…` | `hub://localhost/bytes/…` | `hub://localhost/bytes/…` |

**Building on Linux** needs Tauri's packages. On Debian or Ubuntu:

```
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev   librsvg2-dev patchelf libssl-dev build-essential
```

**openOMSI on Linux.** The setup offers the folders it finds (`hub_game::find`):

- `~/openOMSI*`;
- `openOMSI` and `openOMSI-*-linux-x64` folders in the Downloads folder (and the desktop's
  translated name for it, from `~/.config/user-dirs.dirs`), on the Desktop and in `~/Games`,
  `~/Applications`, `~/.local/share` and `/opt`.

A program unpacked without its executable bit gets it before it is started
(`process::ensure_executable`).

**OMSI 2 on Linux** is the Windows game in Steam, through Proton. The setup looks in:

- `~/.steam/steam`, `~/.steam/root` and `~/.local/share/Steam`;
- the Flatpak's `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`;
- every extra library their `libraryfolders.vdf` lists.

In each one it checks `steamapps/common/OMSI 2`.

**File names in any case.** OMSI 2's content names its files carelessly, and Linux's file
systems keep case apart. `hub_omsi::vfs` finds every part of a path in any case; where two names
differ only in case, the exact one wins. `crates/hub-omsi/tests/case_sensitive.rs` proves this on
a case-sensitive temp folder. The Linux CI job sets `OMSIHUB_REQUIRE_CASE_SENSITIVE=1`, so the
test cannot skip there unnoticed. On Windows the same proof runs in a folder made case-sensitive
with `fsutil file setCaseSensitiveInfo <dir> enable`, by pointing `TMP` at it.

The readers rely on it for every path an OMSI file names, which is written with `\` and in any
case: a map's `TTData`, `Chrono`, `Holidays.txt`, `ailists.cfg` (and Chrono's
`ailists_#upd.cfg`), `timezone.txt`; a line's trips and a trip's track (matched by name in any
case); Chrono's copies of the map's files (taking their place whatever their case); a bus's
model, cabin, rear section, `[CTC]` paint folder and number lists. Names compare in any case
for non-ASCII letters too (`vfs::same_name`: `Grünau` is `GRÜNAU`), and the paths the readers
keep (`Bus.model`, `AiVehicle.file`, `TypGroup.file` …) have `/` (`vfs::normalize`). The second
test in `case_sensitive.rs` builds a mixed-case map and bus and reads them by other cases.
