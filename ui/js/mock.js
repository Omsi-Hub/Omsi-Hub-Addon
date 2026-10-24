// The browser mock of hub-api: what the app's Rust answers, faked, so the page can be opened
// in a plain browser (ui/index.html, or `?mock`). A port adds the answers its page needs to
// `answers` below (or a module of its own under ./mock/, merged here). The first-run setup
// accepts any folder whose name contains "openOMSI" (a version in the name, as
// `openOMSI-0.2.30`) and "OMSI 2"; what it keeps stays in this browser (`localStorage`), so a
// reload opens the start. `?intro=off` leaves the opening out, `?page=<page>` opens a page
// (`settings`, `bus company`, `drive:bus` … as `OMSIHUB_PAGE` would); `?drive` opens it with a
// drive running (its live dashboard: ./mock/live.js).

import { companyAnswers } from './mock/company.js';
import { rosterflowAnswers } from './mock/rosterflow.js';
import { liveAnswers, liveRouteMap, mockDriveAtStart, startMockDrive } from './mock/live.js';
import { depotAtlas, depotHas } from './mock/depotmodels.js';

const listeners = {};
const params = new URLSearchParams(location.search);
const KEPT = 'omsihub.mock.settings';

function kept() {
  try {
    return JSON.parse(localStorage.getItem(KEPT)) || {};
  } catch {
    return {};
  }
}
const settings = { openomsi_folder: '', omsi2_folder: '', language: '', setup_done: false, migrated: false, ...kept() };
const keep = () => {
  try {
    localStorage.setItem(KEPT, JSON.stringify(settings));
  } catch {
    // (a private window: this session only)
  }
};

function versionIn(folder) {
  const m = /(\d+)\.(\d+)(?:\.(\d+))?/.exec(folder);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3] || 0)] : null;
}

function older(a, b) {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] < b[i];
  return false;
}

function checkOpenomsi(folder) {
  if (!/openomsi/i.test(folder || '')) return { ok: false, problem: 'no_game', text: 'No openOMSI was found in this folder' };
  const v = versionIn(folder);
  const tooOld = !!v && older(v, [0, 2, 22]);
  return {
    ok: !tooOld,
    too_old: tooOld,
    version: v ? v.join('.') : null,
    version_from: v ? 'folder_name' : null,
    plugins_dir: folder + '/Plugins',
    text: tooOld ? `This openOMSI is ${v.join('.')}: Omsi-Hub needs 0.2.22 or newer. Update openOMSI.` : '',
    compat: { level: tooOld ? 'unsupported' : v ? 'supported' : 'unknown', features: [], notes: [] },
  };
}

/** The plugin's state in the mock's openOMSI. */
let addonPlugin = 'missing';

/** The old Omsi-Hub's saves the mock finds with `?migrate` (`migrate::preview`'s shape). */
const OLD = { folder: 'C:/Users/Player/.openomsi', companies: [['stadtbus', 'Stadtbus Grundorf']], careers: ['luc'], lines: ['Grundorf'], depots: ['oo_Grundorf'], settings: ['accent', 'dark_mode', 'company_layout'], language: 'de', driver: 'Luc', files: 11, bytes: 6000 };
const oldSaves = () => (params.has('migrate') && !settings.migrated ? { offer: true, folder: OLD.folder, companies: OLD.companies.map((c) => c[0]), careers: OLD.careers, files: OLD.files, preview: OLD } : { offer: false, companies: [], careers: [], files: 0 });

function checkOmsi2(folder) {
  const ok = /omsi ?2/i.test(folder || '');
  return { ok, missing: ok ? [] : ['Omsi.exe'], text: ok ? '' : 'This is not the OMSI 2 folder: Omsi.exe not found' };
}

// --- the drive's content: two maps, three buses, Grundorf's lines --------------------------

const MAPS = [
  { file: 'maps/Grundorf/global.cfg', name: 'Grundorf', friendly: 'Grundorf', entry_points: [{ name: 'Depot' }, { name: 'Bahnhof' }, { name: 'Marktplatz' }] },
  { file: 'maps/Ruhrau/global.cfg', name: 'Ruhrau', friendly: 'Ruhrau', entry_points: [{ name: 'Betriebshof' }] },
];

const BUSES = [
  { file: 'Vehicles/MAN_SD200/SD202.bus', name: 'MAN SD202', manufacturer: 'MAN', folder: 'MAN_SD200', type_name: 'Double-decker', description: 'Berlin double-decker, 1980s', paints: ['BVG'], default_paint: 'BVG', numbers: [['2571', 'B-V 2571']], missing_packs: [], hofs: ['Grundorf.hof'] },
  { file: 'Vehicles/MAN_NL_NG/NL202.bus', name: 'MAN NL202', manufacturer: 'MAN', folder: 'MAN_NL_NG', type_name: 'Low-floor', description: 'City bus, low floor', paints: ['Weiss', 'Grundorf'], default_paint: 'Weiss', numbers: [['17', '']], missing_packs: [], hofs: ['Grundorf.hof'] },
  { file: 'Vehicles/MAN_NG/NG272.bus', name: 'MAN NG272', manufacturer: 'MAN', folder: 'MAN_NG', type_name: 'Articulated', description: 'Articulated city bus', paints: ['Weiss'], default_paint: 'Weiss', numbers: [], missing_packs: [], hofs: [] },
];

const STOPS = ['Bahnhof', 'Marktplatz', 'Kirche', 'Schule', 'Waldweg', 'Endstation'];
const stopsFrom = (dep, back) => (back ? [...STOPS].reverse() : STOPS).map((name, k) => ({ name, id: 100 + (back ? STOPS.length - 1 - k : k), arr: dep + k * 180, dep: dep + k * 180 + (k ? 20 : 0) }));
function trip(line, index, dep, back) {
  const stops = stopsFrom(dep, back);
  return { name: `${line} ${back ? 'Endstation-Bahnhof' : 'Bahnhof-Endstation'}`, index, line, from: stops[0].name, terminus: stops[stops.length - 1].name, departure: dep, arrival: stops[stops.length - 1].arr, stops, km: 6.4 };
}
function tour(line, number, start, n) {
  const trips = Array.from({ length: n }, (_, k) => trip(line, k + 1, start + k * 1500, k % 2 === 1));
  return { number, ai_group: 'Stadtbus', first: trips[0].departure, last: trips[n - 1].arrival, days: 'Mon-Fri', runs: true, next_run: null, trips };
}
const LINES = [
  { name: '1', user_allowed: true, termini: ['Bahnhof', 'Endstation'], tours: [tour('1', '1', 5 * 3600 + 600, 12), tour('1', '2', 6 * 3600, 10)] },
  { name: '2', user_allowed: true, termini: ['Endstation', 'Bahnhof'], tours: [tour('2', '1', 7 * 3600 + 900, 8)] },
  { name: 'E', user_allowed: false, termini: ['Bahnhof', 'Schule'], tours: [tour('E', '1', 7 * 3600, 2)] },
];

// --- the editors: a timetable to edit and a depot file of the player's ---------------------

const ttLine = (l) => ({ path: `maps/Grundorf/TTData/${l.name}.ttl`, name: l.name, user_allowed: l.user_allowed, priority: 1, own: false, tours: l.tours.map((t) => ({ number: t.number, ai_group: t.ai_group, days: 31, trips: t.trips.map((x) => ({ trip: x.name, profile: 0, departure: x.departure / 60 })) })) });
const TT = { dir: 'maps/Grundorf', folder: 'Grundorf', lines: LINES.map(ttLine), trips: LINES.flatMap((l) => [0, 1].map((k) => ({ name: l.tours[0].trips[k].name, line: l.name, terminus: l.tours[0].trips[k].terminus, profiles: ['0', '1'] }))) };

const DEPOT = {
  key: 'grundorf-eigen', file: 'grundorf-eigen.hof', name: 'Grundorf (own)', map: 'maps/Grundorf/global.cfg', dirty: false,
  termini: [
    { code: 1, name: 'Bahnhof', all_exit: false, strings: ['Bahnhof', 'Bahnhof', '', 'Bahnhof', '1 Bahnhof', '', '', '', '', '', '', ''], special: false, marked: false },
    { code: 2, name: 'Endstation', all_exit: false, strings: ['Endstation', 'Endstation', 'über Marktplatz', 'Endstation', '1 Endstation', '', '', '', '', '', '', ''], special: false, marked: false },
    { code: 999, name: 'Betriebsfahrt', all_exit: true, strings: ['Betriebsfahrt', 'Betriebsfahrt', '', '', '', '', '', '', '', '', '', ''], special: true, marked: true },
  ],
  stops: STOPS.map((s, k) => ({ ident: `GD_${k}`, strings: [s, s.toUpperCase()] })),
  trips: [{ code: '101', line: '1', name: 'Bahnhof - Endstation', route: '1', stops: STOPS.map((_, k) => `GD_${k}`) }],
  service_trip: 'Betriebsfahrt',
  notes: [], stop_notes: [], global_strings: ['Grundorf'], string_count: 12,
  labels: ['ibis', 'front_top', 'front_bottom', 'side', 'roller_blind', 'clear_name', 'extra_sign', 'bitmap', 'special', 'rear', 'other', 'other'].map((role) => ({ role, note: '', limit: role === 'ibis' ? 16 : 24 })),
  blocks: [], issues: [],
};
const DEPOTS = [{ key: DEPOT.key, name: DEPOT.name, map: DEPOT.map, termini: DEPOT.termini.length }];

const GAME_CFG = { msaa: '4', fullscreen: '1' };
const GAME_FIELDS = [
  ['Graphics', [['msaa', 'Anti-aliasing (MSAA)', 'choice', [['1', 'Off'], ['2', '2×'], ['4', '4×'], ['8', '8×']]], ['fullscreen', 'Full screen', 'switch'], ['shadows', 'Shadows', 'switch'], ['view_distance', 'View distance (metres)', 'number', [100, 10000, 50, null]], ['render_scale', 'Render scale', 'text', 'auto, or a fraction such as 0.75']]],
  ['Screen and navigator', [['ui_scale', "Size of the game's menus", 'number', [0.5, 2, 0.05, 1]], ['navigator', 'Navigator', 'switch']]],
  ['Language', [['language', "The game's language", 'choice', [['ENG', 'English'], ['DEU', 'Deutsch'], ['FRA', 'Français']]]]],
  ['Driving and controls', [['drive_keys', 'Driving keys', 'choice', [['simple', 'Simple'], ['wasd', 'W A S D'], ['arrows', 'Arrow keys'], ['omsi', 'As in OMSI 2']]]]],
  ['Passengers and traffic', [['boarding', 'Boarding', 'choice', [['auto', 'Automatic'], ['pay', 'Passengers pay at the driver'], ['walk', 'Passengers walk in']]]]],
  // (the game controllers' groups: the tab "Controllers")
  ['Wheel and pedals', [['wheel_range', "Steering wheel's turn (degrees)", 'number', [180, 1800, 30, 900]], ['ctrl_deadzone', 'Dead zone of the wheel and the pedals', 'number', [0, 0.3, 0.01, 0]], ['pedal_brake', 'Brake pedal strength (1: as the pedal reads)', 'number', [0.5, 2, 0.05, 1]], ['ctrl_off', 'Controllers switched off', 'text', 'their names, separated by |']]],
  ['Gamepad', [['pad_type', 'Gamepad type', 'choice', [['auto', 'Automatic'], ['xbox', 'Xbox'], ['ps4', 'PlayStation 4'], ['ps5', 'PlayStation 5']]], ['pad_deadzone', 'Stick dead zone', 'number', [0, 0.4, 0.01, 0.08]]]],
  ['Force feedback', [['ff_enabled', 'Force feedback and vibration', 'switch'], ['ff_invert', 'Force feedback the other way', 'switch']]],
];
const CONTROLLER_GROUPS = ['Wheel and pedals', 'Gamepad', 'Force feedback'];
// (`addon_controllers`: a wheel of OMSI 2's file and its shifter)
const axes = (...set) => Array.from({ length: 8 }, (_, k) => set.find(([s]) => s === k)?.slice(1) || [-1, 0]);
let PADS = [
  { name: 'Logitech G25 Racing Wheel USB', second: '1', axes: axes([0, 0, 0], [1, 1, 5], [5, 2, 9], [7, 3, 3]), buttons: [['automatic_D', '0'], ['automatic_N', '0'], ['automatic_R', '0'], ['', '0'], ['horn', '1']], ff_scale: [1.01, 1], ff_invert: null, latching: [], more: [] },
  { name: 'T500 RS Gear Shift', second: '0', axes: axes(), buttons: [['blinker_warn_toggle', '0']], ff_scale: [1, 1], ff_invert: null, latching: [0], more: [[0, 'kw_fernlicht_toggle']] },
];
const controllers = () => ({ path: '~/.openomsi/Inputs/gamectrler.cfg', from: 'openomsi', save_to: '~/.openomsi/Inputs/gamectrler.cfg', error: null, devices: PADS, actions: ['sim_pause', 'horn', 'kw_fernlicht_toggle', 'IBIS_vor', 'cp_microphone'], names: { ibis_vor: 'IBIS: next stop' }, running: false });
const gameSettings = () => ({
  path: '~/.openomsi/settings.cfg', exists: true, running: false,
  groups: GAME_FIELDS.map(([title, fields]) => ({
    title,
    controllers: CONTROLLER_GROUPS.includes(title),
    fields: fields.map(([key, label, kind, x]) => ({ key, label, kind, value: GAME_CFG[key] ?? null, ...(kind === 'choice' ? { options: x } : kind === 'number' ? { min: x[0], max: x[1], step: x[2], default: x[3] } : kind === 'text' ? { hint: x } : {}) })),
  })),
});

// the phone and tablet companion (`hub_companion_*`)
let companionOn = false;
let companionCode = '482913';
let companionDevices = [{ id: 'a1', name: "Luc's iPhone", since: Math.floor(Date.now() / 1000) - 86400 * 3, seen: Math.floor(Date.now() / 1000) - 20, connected: true }];
const companionQr = () => {
  const n = 29;
  let bits = '';
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) bits += (x * 7 + y * 13 + ((x * y) % 5)) % 3 === 0 || ((x < 7 || x > n - 8) && y < 7) || (x < 7 && y > n - 8) ? '1' : '0';
  return { size: n, bits };
};
const companionState = () =>
  companionOn
    ? { on: true, wanted: true, host: '192.168.1.20', port: 47811, address: 'http://192.168.1.20:47811', code: companionCode, pair_url: `omsihub://pair?host=192.168.1.20&port=47811&code=${companionCode}&name=Omsi-Hub`, name: 'Omsi-Hub', devices: companionDevices, game: false, qr: companionQr() }
    : { on: false, wanted: false, port: 47811, error: '' };

const answers = {
  hub_version: () => ({ app: '0.1.0', min_openomsi: '0.2.22', plugin: '1.8.0', bridge: 1 }),
  hub_boot: () => ({ version: answers.hub_version(), data_dir: '(mock)', settings, setup_needed: !settings.setup_done, languages: ['en', 'nl', 'de', 'fr', 'pl'], intro: params.get('intro') || '', page: params.get('page') || '' }),
  hub_texts: () => ({}),
  // (`?migrate` offers the old saves once, as `hub_setup_state` does when ~/.openomsi has some)
  hub_setup_state: () => ({ settings, openomsi: null, omsi2: null, migrate: oldSaves() }),
  hub_setup_suggest: () => ({ openomsi: [], omsi2: [] }),
  hub_setup_openomsi: (a) => checkOpenomsi(a.folder),
  hub_setup_omsi2: (a) => checkOmsi2(a.folder),
  hub_setup_save: (a) => {
    const o = checkOpenomsi(a.openomsi);
    if (!o.ok) throw o.text;
    const r = checkOmsi2(a.omsi2);
    if (!r.ok) throw r.text;
    Object.assign(settings, { openomsi_folder: a.openomsi, omsi2_folder: a.omsi2, setup_done: true });
    if (a.language) settings.language = a.language;
    keep();
    return { ok: true, settings, openomsi: o };
  },
  hub_settings_patch: (a) => {
    Object.assign(settings, a.patch || {});
    keep();
    return settings;
  },
  hub_migrate_skip: () => ((settings.migrated = true), keep(), { ok: true }),
  hub_migrate_scan: () => ({ folder: OLD.folder, items: [], companies: OLD.companies.map((c) => c[0]), careers: OLD.careers, preview: OLD }),
  hub_migrate_import: () => ((settings.migrated = true), keep(), { imported: OLD.files, preview: OLD, settings }),
  hub_game_state: () => ({ running: false }),
  hub_log: () => null,
  // (the online account and the cloud saves, signed out: everything works offline)
  ol_avatar: () => ({ ok: true, message: 'Profile photo saved.' }),
  ol_state: () => ({ mode: 'out', friends: [], invites: [], requests: [], badge: 0, online_count: 0, offline: false, together: false, hosting: false, privacy: { version: 2, needed: false } }),
  // (as Rust's: the state, refreshed)
  ol_refresh: () => answers.ol_state(),
  ol_seen: () => ({ ok: true }),
  ol_presence: () => ({ ok: true }),
  cs_state: () => ({ companies: {}, slots: 2, used: 0, local_label: 'Local save', cloud_label: 'Cloud save' }),
  cs_open: () => ({}),
  addon_state: () => ({ folder: settings.openomsi_folder, game: checkOpenomsi(settings.openomsi_folder), plugin: { state: addonPlugin }, plugin_version: '1.7.0', hello: null, checked: false, assigned: null }),
  addon_enable: (a) => ((addonPlugin = a.on ? 'on' : 'off'), { state: addonPlugin, version: '1.7.0' }),
  addon_remove: () => ((addonPlugin = 'missing'), { state: 'missing' }),
  addon_set_folder: (a) => {
    const c = checkOpenomsi(a.folder);
    if (c.ok) (settings.openomsi_folder = a.folder), keep();
    return { folder: settings.openomsi_folder, game: checkOpenomsi(settings.openomsi_folder), plugin: { state: addonPlugin }, plugin_version: '1.7.0', checked: false, ...(c.ok ? {} : { refused: { folder: a.folder, problem: c.text } }) };
  },
  addon_version: () => ({ version: checkOpenomsi(settings.openomsi_folder).version, min: '0.2.22' }),
  addon_texts: () => ({ language: settings.language || 'en' }),
  addon_reports: () => ({ reports: 0, trips: 0, orders: 0, live: 0, drivers: [], companies: [] }),
  addon_poll: () => ({ reports: 0, trips: 0, orders: 0, live: 0, drivers: [], companies: [] }),
  // (openOMSI's settings.cfg, a few of each group, kept in this page only)
  addon_game_settings: () => gameSettings(),
  addon_controllers: () => controllers(),
  addon_controllers_save: (a) => ((PADS = a.devices), { written: ['~/.openomsi/Inputs/gamectrler.cfg'], path: '~/.openomsi/Inputs/gamectrler.cfg' }),
  addon_game_setting: (a) => ((a.value == null || a.value === '' ? delete GAME_CFG[a.key] : (GAME_CFG[a.key] = String(a.value))), gameSettings()),

  // the drive
  hub_maps: () => MAPS,
  hub_buses: () => BUSES,
  hub_weather: () => [{ file: 'weather/sunny.owt', name: 'Sunny' }, { file: 'weather/rain.owt', name: 'Rain' }],
  hub_lines: (a) => (/grundorf/i.test(a.map || '') ? LINES : []),
  hub_default_hof: () => 'Grundorf.hof',
  // (the line's routes in the depot file, as `content::ibis` answers)
  hub_ibis: (a) => ({ hof: a.hof || 'Grundorf.hof', line: a.line, line_code: String(a.line || '').replace(/\D/g, '') || '1', routes: [{ code: '101', name: 'Endstation', terminus: 'Endstation' }, { code: '102', name: 'Bahnhof', terminus: 'Bahnhof' }], destinations: 3 }),
  hub_situations: () => [],
  hub_map_facts: () => ({}),
  hub_own_lines: () => [],
  hub_shifts: (a) => {
    // (two shifts of line 1's tours: from the tour's first trip, as long as asked)
    const want = (a.minutes || 120) * 60;
    return LINES[0].tours.map((t) => {
      const trips = [];
      for (const x of t.trips) if (!trips.length || x.arrival - t.trips[0].departure <= want) trips.push(x);
      const first = trips[0];
      return { key: [`1|${t.number}|${first.index}`], start: first.departure, end: trips[trips.length - 1].arrival, seconds: trips[trips.length - 1].arrival - first.departure, trips: trips.map((x) => ['1', t.number, { ...x, stops: x.stops.length }]), first: { line: '1', tour: t.number, index: first.index, departure: first.departure } };
    });
  },
  hub_weather_extras: () => ({ airport: 'EDDB', airports: [['EDDB', 'Berlin Brandenburg'], ['EDDH', 'Hamburg']], cycle: false }),
  hub_duty_extras: (a) => {
    // (the bus's lead time as the settings have it, 5 minutes by default as in Rust)
    const lead = settings.bus_lead ?? 5;
    const m = Math.max(0, (a.minutes || 0) - lead);
    return { time: `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`, date: a.date, lead, display_font: null };
  },
  hub_more_setvars: () => [],
  hub_more_bus_options: () => ({ options: [], reading: false }),
  hub_more_font_view: () => ({ failed: true, options: [] }),
  // (a text on the bus's own display font, drawn by the game program: not in a browser)
  hub_display_preview: () => ({ failed: true }),
  // (the bus in 3D is written by the player's openOMSI: not in a browser)
  // (the 3D view reads the bus from the OMSI 2 folder: the mock has none)
  b3d_env: () => ({ cloudSize: 2000 }),
  b3d_open: () => {
    throw 'The 3D view reads the bus from your OMSI 2 folder: open Omsi-Hub itself';
  },
  b3d_look: () => {
    throw 'The 3D view reads the bus from your OMSI 2 folder: open Omsi-Hub itself';
  },
  b3d_photo_has: () => ({ has: false }),
  // (the 3D depot's bus models: drawn by the mock, `mock/depotmodels.js`)
  b3d_depot_has: (a) => depotHas(a),
  // (the livery studio: no liveries begun in the mock, the studio's own fonts)
  lak_projects: () => [],
  lak_fonts: () => ['Hanken Grotesk', 'Roboto'],
  hub_more_weather_read: (a) => ({ custom: { raw: a.weather, vis: 1200, br: 0.8, wd: 270, ws: 6.5, temp_c: -3, rh: 88, p: 1013, c: 0.75, cb: 50, precip: 2, pi: 120, wet: 0.4, snow_cover: 1, snow_on_road: 0 } }),
  hub_more_weather_write: (a) => ({ weather: a.custom?.raw || 'custom:' }),
  hub_more_bus_best: () => null,
  hub_depots: (a) => ({
    key: `${a.bus}|${a.map?.file || ''}`, bus: a.bus, map: a.map?.friendly || a.map?.name || '', want: 'Grundorf.hof', lacks: false, total: 3,
    rows: [{ key: 'Grundorf.hof', name: 'Grundorf.hof', file: 'Grundorf.hof', map: 'Grundorf', maps: true, destinations: 3, known: 3, fit: { kind: 'maps' } }],
    used: { row: 0 }, own: [], own_in_use: null, refusal: null, source: null, source_name: '', source_destinations: 0, recommended: 0,
  }),
  hub_route_map: (a) => (/Riverside/.test(a?.map || '') ? liveRouteMap() : {
    roads: [{ p: [0, 0, 400, 0, 400, 300, 900, 300], w: 8, main: true }, { p: [400, 0, 400, -200], w: 6 }], route: [[0, 0, 400, 0, 400, 300, 900, 300]],
    stops: [{ k: 0, at: [0, 0] }, { k: 1, at: [400, 150] }, { k: 2, at: [900, 300] }], entries: [{ index: 0, name: 'Depot', at: [400, -200] }],
    lo: [0, -200], hi: [900, 300], colors: ['#06102a', '#24345a', '#2d4170', '#f58620'],
  }),
  hub_drive: (a) => {
    // (the drive runs until it is ended: hub_game_stop, ./mock/live.js)
    const d = a.duty || {};
    const mode = d.company ? 'company' : (d.legs || []).length ? 'shift' : d.line && d.tour ? 'tour' : 'free';
    startMockDrive(mode, emit);
    return { started: true, pid: 4242, mode, args: ['--map', d.map] };
  },
  hub_update: () => ({ kind: 'none', shown: '0.1.0', line: 'Omsi-Hub 0.1.0 is the newest.', repo: 'github.com/Omsi-Hub/Omsi-Hub-Releases', auto: false }),
  hub_drive_args: (a) => ({ args: ['--root', settings.openomsi_folder, '--map', a.duty.map, '--bus', a.duty.bus, ...(a.duty.line ? ['--line', a.duty.line, '--tour', a.duty.tour || ''] : []), '--time', a.duty.time] }),
  hub_update_act: (a) => ({ kind: 'none', line: a.what === 'check' ? 'Omsi-Hub 0.1.0 is the newest.' : '', repo: 'github.com/Omsi-Hub/Omsi-Hub-Releases', auto: false }),

  // the timetable editor
  tt_load: (a) => (/grundorf/i.test(a.map || '') ? TT : { dir: '', folder: '', lines: [], trips: [] }),
  tt_save: (a) => ({ saved: (a.lines || []).length, error: null, lines: (a.lines || []).map((l) => ({ name: l.name, path: l.path, line: l })) }),
  tt_edit: (a) => ({ line: a.line, tour: a.tour || 0, made: 0 }),
  tt_reset: () => ({ done: "The timetable of %{map} is the map's own again (the edited copy was removed)", done_args: { map: 'Grundorf', n: 0 }, own: 0, own_error: null }),

  // the depot editor
  de_show: () => ({ entries: DEPOTS, view: DEPOT, changed_elsewhere: null, error: null }),
  de_open: () => DEPOT,
  de_edit: () => ({ view: DEPOT, sel: null }),
  de_save: () => ({ view: { ...DEPOT, dirty: false }, entries: DEPOTS, copies: 0, name: DEPOT.name }),
  de_delete: () => ({ entries: [], name: DEPOT.name }),
  de_sources: () => [
    { source: { kind: 'empty', path: null }, text: "Empty, laid out as OMSI's own", args: {} },
    { source: { kind: 'map', path: 'Vehicles/MAN_SD200/Grundorf.hof' }, text: "A copy of the map's: %{hof}", args: { hof: 'Grundorf.hof' } },
    { source: { kind: 'own', path: 'depots/grundorf-eigen.hof' }, text: 'A copy of mine: %{name}', args: { name: DEPOT.name } },
  ],
  de_make: () => ({ entries: DEPOTS, view: DEPOT }),
  tg_gate: () => ({ on: true, label: '' }),
  hub_lan_check: ({ text }) => (/^OMSI-/i.test(text || '') ? { ok: true, text: 'A session code: the host is found over the internet or here at home.', local: [] } : { ok: false, text: 'Not a session code or an address', local: [] }),
  hub_lan_query: ({ address }) => ({ name: address === 'openomsi' ? 'openOMSI | Official Server' : 'Luc\'s game', motd: '', map: 'maps/Grundorf/global.cfg', players: 2, max_players: 8, version: '0.2.30', icon: null, time: '08:15', weather: '', password: false, vehicles: [], reached_at: '' }),
  hub_lan_servers: () => [{ name: 'openOMSI | Official Server', address: 'openomsi' }],
  hub_lan_servers_save: () => true,
  hub_lan_tunnel: () => null,
  // (the phone and tablet companion: `hub/companion.js`; a QR code of plain stripes stands in)
  hub_companion_state: () => companionState(),
  hub_companion_set: ({ on }) => ((companionOn = !!on), companionState()),
  hub_companion_code: () => ((companionCode = String(100000 + Math.floor(Math.random() * 900000))), companionState()),
  hub_companion_forget: ({ id }) => ((companionDevices = companionDevices.filter((d) => d.id !== id)), companionState()),
  hub_lan_hosted: () => [],
  hub_profiles: () => ({ names: DRIVERS.slice(), current: DRIVERS[0] || '' }),
  hub_profile: ({ name }) => driverRecord(name),
  hub_profile_select: ({ name }) => ({ profile: name }),
  hub_profile_personnel: ({ make }) => (make ? { number: '416207', code: '3851' } : null),
  hub_profile_create: ({ name }) => {
    if (!DRIVERS.includes(name)) DRIVERS.push(name);
    return driverRecord(name);
  },
  hub_profile_delete: ({ name }) => {
    const k = DRIVERS.indexOf(name);
    if (k >= 0) DRIVERS.splice(k, 1);
    return true;
  },

  ...companyAnswers,
  ...rosterflowAnswers,
  ...liveAnswers(emit),
};

// (the drivers on this computer, as `hub_profiles` lists them, and a service record of each)
const DRIVERS = ['Luc', 'OMSI-Fan'];
const RUN = { time: Math.floor(Date.now() / 1000) - 3600, driver: 'Luc', map: 'maps/Grundorf/global.cfg', bus: 'Vehicles/MAN_SD200/SD202.bus', line: '5', tour: '1', seconds: 2460, metres: 14200, stops: 18, early: 1, late: 2, tickets: 9, cash: 14.4, crashes: 0, hurt: 0, jolts: 3, driving: 94, comfort: 88, ticketing: 100 };
const driverRecord = (name) => ({
  name, file: `Drivers/${name}.odr`, hours: name === 'Luc' ? 12.6 : 0, km: name === 'Luc' ? 431.5 : 0, xp: name === 'Luc' ? 1460 : 0, level: name === 'Luc' ? 3 : 1, next_level_xp: name === 'Luc' ? 2250 : 250,
  stops: 640, early: 31, late: 58, tickets: 412, cash: 655.2, crashes: 3, hurt: 0, rating_driving: 91, rating_comfort: 86, rating_tickets: 97,
  sessions: name === 'Luc' ? [RUN, { ...RUN, time: RUN.time - 86400, line: null, tour: null, metres: 5100, seconds: 900 }] : [], exists: true,
  record: { runs: name === 'Luc' ? 2 : 0, first: RUN.time - 86400, maps: [], buses: [], hours: Array(24).fill(0), jolts: 3, km: 19.3 },
  trips: [], trip_totals: { trips: 0, timed: 0, stops: 0, early: 0, late: 0, average: null, worst: null, spotless: 0 },
});
function emit(event, payload) {
  backend.emit(event, payload);
}

export const backend = {
  async call(cmd, args) {
    const a = answers[cmd];
    if (!a) {
      // (kept for a look at what a page asked that the mock lacks: `__OMSIHUB_MOCK_MISSING__`)
      (globalThis.__OMSIHUB_MOCK_MISSING__ ||= {})[cmd] = (globalThis.__OMSIHUB_MOCK_MISSING__[cmd] || 0) + 1;
      throw `not ported yet: ${cmd}`;
    }
    return a(args || {});
  },
  async callBytes(cmd, args) {
    if (cmd === 'b3d_depot') return depotAtlas(args || {});
    (globalThis.__OMSIHUB_MOCK_MISSING__ ||= {})[`bytes:${cmd}`] = 1;
    throw `not ported yet: ${cmd}`;
  },
  on(event, fn) {
    (listeners[event] ||= []).push(fn);
    return () => { listeners[event] = listeners[event].filter((f) => f !== fn); };
  },
  /** For a page's work: pretend Rust sent `event`. */
  emit(event, payload) {
    for (const f of listeners[event] || []) f(payload);
  },
};

mockDriveAtStart(emit);
