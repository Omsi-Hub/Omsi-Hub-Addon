// The drive the player is choosing, and what it is chosen from: the settings (the app's and the
// look's), the duty being put together on the steps (`S.choice`), the maps, buses, weathers and
// the chosen map's lines on the chosen day, and the start of the drive in the player's
// openOMSI (`hub_drive`). Omsi-Hub's own; it stands where the old launcher's shared module did.
//
// The content comes from hub-api: `hub_maps`, `hub_buses`, `hub_weather`, `hub_lines {map,
// date}`, `hub_default_hof`, `hub_ibis`, `hub_situations {map}`; the settings are saved with
// `hub_settings_patch {patch}`. While a command is not there yet the page carries on: the list
// stays empty, and a setting is kept in the page (`localStorage`) until hub-api keeps it.

import { call, on } from '../api.js';
import { S, set, toast } from '../store.js';
import { setLanguage, t } from '../i18n.js';

/** A new duty: today's tour on the map chosen first, nine in the morning. */
export const CHOICE_DEFAULTS = {
  bus: '', paint: '', plate: '', number: '', hof: '', hof_manual: false,
  map: '', entry: -1, line: null, tour: null, free: false, composed: false,
  time: 9 * 60, start_trip: null, date: '1989-05-30',
  season: 'auto', phase: 'mid', own_date: null,
  weather: '', traffic: 30, passengers: true, schedule: true, autostart: false, on_foot: false,
  lan_mode: 'off', lan_addr: '',
};

/** The look's and the pages' settings, until the player changes them. */
export const SETTINGS_DEFAULTS = {
  dark_mode: true, accent: '#2a75f7', animations: true, stop_style: 'de', launcher_scale: 1,
  // (`bus_lead`: hub-core's `BUS_LEAD_DEFAULT`, what `hub_duty_extras` takes while it is not set)
  company_layout: 'sidebar', units: 'metric', bus_lead: 5, update_auto: false,
};

const KEPT_SETTINGS = 'omsihub.settings';
const KEPT_CHOICE = 'omsihub.choice';
const KEPT_FAVOURITES = 'omsihub.favourites';

function kept(key, fallback) {
  try {
    const v = JSON.parse(localStorage.getItem(key));
    return v == null ? fallback : v;
  } catch {
    return fallback;
  }
}

function keep(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // (a private window: kept for this session only)
  }
}

/** After `hub_boot` (and again after the setup): the settings, the duty last chosen, the
 * content read. */
export function startDrive(boot) {
  const settings = { ...SETTINGS_DEFAULTS, ...kept(KEPT_SETTINGS, {}), ...((boot && boot.settings) || {}) };
  const choice = { ...CHOICE_DEFAULTS, ...kept(KEPT_CHOICE, {}) };
  // (a server joined in an earlier session is not joined again by itself)
  if (choice.lan_mode === 'join') choice.lan_mode = 'off';
  Object.assign(choice, clockPatch(choice, settings));
  set({ settings, config: { profile: '' }, choice, favourites: kept(KEPT_FAVOURITES, []) });
  loadContent();
}

function running(mode) {
  set({ game: { running: true, mode: mode || S.game?.mode || null }, gameStarted: true, liveDash: { hidden: false }, poll: { ...S.poll, instances: [{ running: true }] } });
}

let listening = false;
/** The game's end (hub-api's `game-ended`), and the clock the duty may follow. */
export function watchGame() {
  if (listening) return;
  listening = true;
  on('game-ended', (p) => {
    set({ game: { running: false, ended: p || null }, gameStarted: false, live: null, poll: { ...S.poll, instances: [] } });
    loadSituations();
  });
  // (any drive started - a shift, a tour, a free drive, a company's duty: its live dashboard,
  // hub/dash.js; and one that runs already as the launcher opens)
  on('game-started', (p) => running(p?.mode));
  call('hub_game_state')
    .then((g) => g?.running && !S.game?.running && running())
    .catch(() => {});
  setInterval(followClock, 30000);
}

export async function loadContent() {
  const errors = { ...S.errors };
  const [maps, weather] = await Promise.all([
    call('hub_maps').catch((e) => ((errors.maps = e.message), [])),
    call('hub_weather').catch((e) => ((errors.weather = e.message), [])),
  ]);
  // (a weather's name comes in English where OMSI has none in the page's language)
  set({ maps, weather: weather.map((w) => ({ ...w, name: t(w.name) })), errors });
  fixChoice();
  loadLines();
  loadSituations();
  call('hub_buses')
    .then((vehicles) => {
      set({ vehicles });
      fixChoice();
      updateHof();
    })
    .catch((e) => set({ vehicles: [], errors: { ...S.errors, vehicles: e.message } }));
  loadProfiles();
}

/** A map or bus chosen before that is not there any more gives way to the first one. */
function fixChoice() {
  const c = { ...S.choice };
  if (S.maps?.length && !S.maps.some((m) => m.file === c.map)) {
    c.map = (S.maps.find((m) => /grundorf/i.test(m.file)) || S.maps[0]).file;
    // (an entry point or stop of the map before is none of this one's)
    Object.assign(c, { entry: -1, free_entry: -1, free_stop: '', free_line: '', free_route: '', start_trip: null });
  }
  if (S.vehicles?.length && !S.vehicles.some((v) => v.file === c.bus)) {
    c.bus = S.vehicles[0].file;
    c.paint = '';
    c.number = '';
  }
  const own = /^(cycle$|metar:|custom:)/i.test(c.weather || '');
  if (S.weather && c.weather && !own && !S.weather.some((w) => w.file === c.weather)) c.weather = '';
  set({ choice: c });
}

const norm = (f) => (f || '').replace(/\\/g, '/').toLowerCase();

/** A bus's file from `Vehicles/` on, in lower case (a server lists its buses so). */
export const busFile = (f) => {
  const n = norm(f);
  const at = n.lastIndexOf('vehicles/');
  return at > 0 ? n.slice(at) : n;
};

/** The buses the server joined allows (None: any). */
export function serverBuses(c = S.choice) {
  return c?.lan_mode === 'join' && c.lan_server?.vehicles?.length ? new Set(c.lan_server.vehicles.map(busFile)) : null;
}

/** The bus chosen, or one of ours the server joined allows. */
export function allowedBus(c = S.choice) {
  const allowed = serverBuses(c);
  if (!allowed || allowed.has(busFile(c.bus))) return {};
  const own = S.vehicles?.find((v) => allowed.has(busFile(v.file)));
  return own ? { bus: own.file, paint: '', number: '', hof_manual: false } : {};
}

/** What the choice keeps of a server's or a host's answer (`hub_lan_query`). */
export const serverJoined = (answer) => ({
  lan_server: { asked: !!answer, name: answer?.name || '', time: answer?.time || '', weather: answer?.weather || '', vehicles: answer?.vehicles || [] },
});

export const curMap = () => S.maps?.find((m) => m.file === S.choice?.map);
export const curBus = () => S.vehicles?.find((v) => v.file === S.choice?.bus);
export const curLine = () => S.lines.list?.find((l) => l.name === S.choice?.line);
export const curTour = () => curLine()?.tours.find((x) => x.number === S.choice?.tour);
export const realTime = () => !!S.settings?.use_real_time;
export const realDate = () => !!S.settings?.use_real_date;

/** The time and date the computer's clock gives the duty, while the settings ask for it. */
function clockPatch(c, st) {
  const now = new Date();
  const patch = {};
  if (st?.use_real_time) {
    const m = now.getHours() * 60 + now.getMinutes();
    if (c.time !== m) patch.time = m;
  }
  if (st?.use_real_date) {
    const year = st.use_real_year ? now.getFullYear() : parseInt(c.date, 10) || now.getFullYear();
    const d = `${String(year).padStart(4, '0')}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
    if (c.date !== d) patch.date = d;
  }
  return patch;
}

export function followClock() {
  if (!S.choice) return;
  const patch = clockPatch(S.choice, S.settings);
  if (Object.keys(patch).length) choose(patch);
}

let keeping = 0;
/** Change the duty (kept a moment later), and read again what depends on what changed. */
export function choose(patch) {
  const before = S.choice || CHOICE_DEFAULTS;
  const c = { ...before, ...patch };
  set({ choice: c });
  clearTimeout(keeping);
  keeping = setTimeout(() => keep(KEPT_CHOICE, S.choice), 400);
  if (c.map !== before.map || c.date !== before.date) loadLines();
  if (c.map !== before.map) loadSituations();
  if (c.map !== before.map || c.bus !== before.bus || c.date !== before.date || c.line !== before.line || c.free !== before.free) updateHof();
  else if (c.hof !== before.hof || c.hof_manual !== before.hof_manual) loadIbis();
}

/** The chosen map's lines on the chosen day; a line or tour that is not there that day gives
 * way to the first one that runs. */
export async function loadLines() {
  const c = S.choice;
  if (!c?.map) return;
  const key = `${c.map}|${c.date}`;
  if (S.lines.key === key && (S.lines.list || S.lines.loading)) return;
  const sameMap = S.lines.key.split('|')[0] === c.map;
  set({ lines: { key, list: sameMap ? S.lines.list : null, loading: true, error: null } });
  try {
    const list = await call('hub_lines', { map: c.map, date: c.date });
    if (S.lines.key !== key) return;
    set({ lines: { key, list, loading: false, error: null } });
    const line = list.find((l) => l.name === S.choice.line) || list.find((l) => l.user_allowed && l.tours.length) || null;
    const tour = line?.tours.find((x) => x.number === S.choice.tour) || line?.tours.find((x) => x.runs) || line?.tours[0] || null;
    if (line?.name !== S.choice.line || tour?.number !== S.choice.tour) choose({ line: line?.name ?? null, tour: tour?.number ?? null });
  } catch (e) {
    if (S.lines.key === key) set({ lines: { key, list: [], loading: false, error: e.message } });
  }
}

let hofAsked = '';
async function updateHof() {
  const c = S.choice;
  const line = c.free ? null : c.line;
  // (another bus or map chosen while this was asked: its answer is the one kept)
  const key = JSON.stringify([c.map, c.bus, c.date, line]);
  hofAsked = key;
  const hof = await call('hub_default_hof', { map: c.map, bus: c.bus, date: c.date, line }).catch(() => '');
  if (hofAsked !== key) return;
  set({ hof: hof || '' });
  loadIbis();
}

/** The depot file the drive takes: the one the player chose, else the bus's for the map. */
export const dutyHof = () => (S.choice.hof_manual ? S.choice.hof : S.hof) || '';

let ibisAsked = '';
/** What the IBIS knows of the chosen line with the chosen depot file. */
export async function loadIbis() {
  const c = S.choice;
  const hof = dutyHof();
  const key = c.free || !c.line || !c.bus ? '' : `${c.bus}|${hof}|${c.line}`;
  if (key === ibisAsked) return;
  ibisAsked = key;
  set({ ibis: null });
  if (!key) return;
  const ibis = await call('hub_ibis', { bus: c.bus, hof, line: c.line }).then((info) => ({ info }), (e) => ({ error: e.message }));
  if (ibisAsked !== key) return;
  if ('error' in ibis) ibisAsked = '';
  set({ ibis });
}

export async function loadSituations() {
  const map = S.choice?.map;
  if (!map) return;
  const situations = await call('hub_situations', { map }).catch(() => []);
  // (the answer for a map chosen before is not the chosen map's)
  if (S.choice?.map === map) set({ situations });
}

/** The drivers on this computer (`hub_profiles`: OMSI's personnel files and those openOMSI's
 * sessions name) and the chosen one's service record. */
export async function loadProfiles() {
  const got = await call('hub_profiles').catch(() => ({ names: [], current: '' }));
  const profiles = got.names || [];
  set({ profiles, accountDriver: got.account || null, config: { ...S.config, profile: got.current || '' } });
  const name = S.config?.profile || profiles[0];
  set({ profile: name ? await call('hub_profile', { name }).catch(() => null) : null });
}

/** A bus starred, to be listed first. */
export function toggleFavourite(file) {
  const k = String(file || '').split('\\').join('/').toLowerCase();
  const list = S.favourites.includes(k) ? S.favourites.filter((x) => x !== k) : [...S.favourites, k];
  set({ favourites: list });
  keep(KEPT_FAVOURITES, list);
}

let saving = Promise.resolve();
/** Change settings (`patch`): shown at once, saved behind (`hub_settings_patch`); the language
 * also changes the page's texts. Answers whether it was saved by the app. */
export async function saveSettings(patch) {
  set({ settings: { ...S.settings, ...patch } });
  let saved = true;
  saving = saving.then(async () => {
    try {
      const now = await call('hub_settings_patch', { patch });
      if (now && typeof now === 'object') set({ settings: { ...S.settings, ...now } });
      dropKept(Object.keys(patch));
    } catch {
      saved = false;
      // (hub-api does not keep it yet: the page does, and the language goes with the folders)
      keep(KEPT_SETTINGS, { ...kept(KEPT_SETTINGS, {}), ...patch });
      if ('language' in patch && S.settings.openomsi_folder && S.settings.omsi2_folder) {
        await call('hub_setup_save', { openomsi: S.settings.openomsi_folder, omsi2: S.settings.omsi2_folder, language: patch.language }).catch(() => {});
      }
    }
  });
  await saving;
  if ('language' in patch) await setLanguage(patch.language);
  return saved;
}

function dropKept(keys) {
  const k = kept(KEPT_SETTINGS, {});
  if (!keys.some((x) => x in k)) return;
  for (const x of keys) delete k[x];
  keep(KEPT_SETTINGS, k);
}

/** The trip of the tour picked to start with (its place), while the time is still its own. */
export function pickedTrip() {
  const c = S.choice;
  const st = c.start_trip;
  if (!st || c.free) return null;
  const [line, tour, index, time] = st;
  return line === c.line && tour === c.tour && (time === c.time || realTime()) ? index : null;
}

const hhmm = (minutes) => {
  const m = ((Math.round(minutes) % 1440) + 1440) % 1440;
  return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
};
const text = (v) => (v == null || v === '' ? null : String(v));

/** The duty as hub-api takes it (`hub_core::Duty`): the choice, and what `extra` adds or
 * changes (a shift's legs, a free route, the display font …). */
export async function duty(extra = {}) {
  const c = S.choice;
  const trip = pickedTrip();
  // (joined: the host's map, the one installed here where it is)
  const ask = () => Promise.race([call('hub_lan_query', { address: c.lan_addr.trim() }).catch(() => null), new Promise((r) => setTimeout(r, 3000))]);
  const joined = c.lan_mode === 'join' && !extra.tutorial && !extra.situation ? c.lan_map || (await ask())?.map : '';
  const hostMap = (joined || '').trim().replace(/\\/g, '/');
  const installed = S.maps?.find((m) => m.file.toLowerCase() === hostMap.toLowerCase())?.file;
  const season = c.season && c.season !== 'auto' ? `${c.season}${c.phase && c.phase !== 'mid' ? '-' + c.phase : ''}` : null;
  const d = {
    map: /maps\//i.test(hostMap) ? installed || hostMap : c.map,
    bus: c.bus,
    paint: text(c.paint),
    plate: text(c.plate?.trim()),
    number: text(c.number?.trim()),
    hof: text(dutyHof()),
    entry: c.entry,
    line: c.free ? null : text(c.line),
    tour: c.free ? null : text(c.tour),
    trip: trip != null ? String(trip) : null,
    whole_tour: trip != null,
    time: hhmm(c.time),
    date: c.date,
    weather: text(c.weather),
    traffic: Math.round(c.traffic),
    passengers: c.passengers,
    schedule: c.schedule,
    autostart: c.autostart,
    on_foot: c.on_foot,
    profile: S.config?.profile || null,
    lan: c.lan_mode === 'host' ? 'host' : c.lan_mode === 'join' ? `join:${c.lan_addr.trim()}` : 'off',
    season,
    ...extra,
  };
  // (hub-api's own types: a tour and a trip are words, the time HH:MM)
  for (const k of ['line', 'tour', 'trip', 'free_line', 'display_font']) if (d[k] != null) d[k] = String(d[k]);
  if (typeof d.time === 'number') d.time = hhmm(d.time);
  return d;
}

/** Start the drive in the player's openOMSI (`hub_drive`). */
export async function start(extra) {
  try {
    const d = await duty(extra);
    const r = await call('hub_drive', { duty: d });
    running(r?.mode);
    toast(t('The game is starting…'), 'ok');
    return r;
  } catch (e) {
    toast(t(e.message), 'error');
    return null;
  }
}
