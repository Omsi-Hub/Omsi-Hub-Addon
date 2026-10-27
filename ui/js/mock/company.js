// The bus company's mock answers (`co_*`): what `omsi-launcher-api`'s `omsihub/company` (and
// `company_more`) answer, faked for the browser (`?mock`). One small company - "Grundorf Bus" on
// the map Grundorf: four buses, seven staff (five drivers), three lines - and a second one to
// choose. The shapes follow the old Rust views (`serde_json::json!` there), the words come ready
// in English. A little state is kept so that the clock runs, messages are read and actions show.

import { moduleAt, spot, whyNot } from '../company/depot3d/yard.js';

// --- what the company is ----------------------------------------------------------------------

const PALETTE = ['#f28c28', '#e03c31', '#c2185b', '#7b1fa2', '#283593', '#1e88e5', '#00897b', '#43a047', '#fdd835', '#6d4c41', '#455a64', '#f5f5f5'];
const TABS = ['Overview', 'Fleet', 'Staff', 'Lines', 'Finances', 'Planning', 'Career', 'Depot', 'Concessions', 'Map', 'Research', 'Workshop', 'Contracts', 'Inbox', 'Agenda', 'Operations', 'Reputation', 'Sandbox', 'Multiplayer'];
const ICONS = ['signal_cellular_alt', 'directions_bus', 'groups', 'route', 'payments', 'calendar_month', 'military_tech', 'garage', 'public', 'map', 'search', 'construction', 'description', 'chat', 'event', 'traffic', 'star', 'sports_esports', 'wifi_tethering'];
const RAIL = [[0, 14, 13], [5, 15, 3, 9, 1], [2], [4, 8, 12, 16, 6], [7, 11, 10]];
const RAIL_HEADS = ['Overview', 'Operations', 'People', 'Business', 'Company'];
const HOME_TAB = 100;
const MAP = 'maps/Grundorf/global.cfg';

const companies = [
  { id: 'grundorf-bus', name: 'Grundorf Bus', short: 'GB', colours: ['#f28c28', '#283593'], map: MAP, map_name: 'Grundorf', depot: 'Betriebshof Grundorf', mode: 'normal', cash: 18734500, trophies: 14, reputation: 63.4, date: '2026-10-19', minute: 7 * 60 + 42, played: 12 },
  { id: 'ruhrau-verkehr', name: 'Ruhrau Verkehr', short: 'RV', colours: ['#43a047', '#f5f5f5'], map: 'maps/Ruhrau/global.cfg', map_name: 'Ruhrau', depot: 'Betriebshof Ruhrau', mode: 'casual', cash: 4210000, trophies: 3, reputation: 48, date: '2026-05-02', minute: 9 * 60, played: 60 * 26 },
  // (a third with the RosterFlow add-on on, so that the browser's self-check has RosterFlow to
  // look at: its Planning page is RosterFlow's - `mock/rosterflow.js` - on Grundorf's lines)
  { id: 'grundorf-linien', name: 'Grundorf Linien', short: 'GL', colours: ['#1e88e5', '#f5f5f5'], map: MAP, map_name: 'Grundorf', depot: 'Betriebshof Grundorf', mode: 'realistic', cash: 26450000, trophies: 21, reputation: 71.2, date: '2026-10-19', minute: 6 * 60 + 15, played: 60 * 3, rosterflow: true },
];

const st = { open: null, gen: 1, speed: 0, until: 16 * 60, dispatcher: true, minor: false, launch: null, notices: [], weather: 'clear', realtime: false, friendsJoin: false, board: false, read: new Set([3, 4, 5]), gone: new Set(), feeds: [] };
const co = () => companies.find((c) => c.id === st.open) || null;
/** Whether the company open has the RosterFlow add-on on (each company its own, as the game's). */
const rosterflowOn = () => !!co()?.rosterflow;
const bump = () => (st.gen += 1);
const say = (text, error = false) => st.notices.push({ text, error });

const LINES = [
  { name: '101', number: '101', colour: '#e03c31', caption: 'Bahnhof – Waldfriedhof', km: 9.8, fare: 260, pax: 1840, type: 'city', hof: 'Grundorf.hof', public: 'Stadtlinie 101' },
  { name: '102', number: '102', colour: '#1e88e5', caption: 'Marktplatz – Gewerbegebiet Nord', km: 7.2, fare: 240, pax: 1215, type: 'city', hof: 'Grundorf.hof', public: '' },
  { name: '205', number: '205', colour: '#43a047', caption: 'Grundorf – Ruhrau (Schnellbus)', km: 18.4, fare: 390, pax: 640, type: 'regional', hof: 'Grundorf.hof', public: 'SchnellBus 205' },
];
const plate = (l) => ({ number: l.number, colour: l.colour });
const lineOf = (name) => LINES.find((l) => l.name === name);
/** The line editor's page for a line (`lines::edit_to`): the company's own line 205 is the line
 * editor's third. */
const editTo = (l) => (l && l.name === '205' ? 'line-editor=3' : null);

const BUSES = [
  { id: 1, number: '17', name: 'MAN NL202', file: 'Vehicles/MAN_NL_NG/NL202.bus', livery: 'Grundorf', plate: 'GD-VB 17', kind: 'Solo bus, diesel', km: 412300, year: 2004, cond: 0.82, how: 'Bought' },
  { id: 2, number: '18', name: 'MAN NL202', file: 'Vehicles/MAN_NL_NG/NL202.bus', livery: 'Weiss', plate: 'GD-VB 18', kind: 'Solo bus, diesel', km: 538900, year: 2002, cond: 0.64, how: 'Bought' },
  { id: 3, number: '31', name: 'MAN NG272', file: 'Vehicles/MAN_NG/NG272.bus', livery: 'Weiss', plate: 'GD-VB 31', kind: 'Articulated bus, diesel', km: 288150, year: 2008, cond: 0.91, how: 'Leased until 30 Jun 2028' },
  { id: 4, number: '2571', name: 'MAN SD202', file: 'Vehicles/MAN_SD200/SD202.bus', livery: 'BVG', plate: 'B-V 2571', kind: 'Double-decker, diesel', km: 801440, year: 1986, cond: 0.38, how: 'Bought', workshop: true },
];

const STAFF = [
  { id: 1, name: 'Jan de Vries', role: 'Driver', icon: 'directions_bus', wage: 312000, portrait: 12, since: 'Since 3 Mar 2025', colour: '#f28c28', grade: 0.86, happy: 0.78, stress: 0.32, friendly: 0.9, licence: 'D' },
  { id: 2, name: 'Anke Müller', role: 'Driver', icon: 'directions_bus', wage: 298000, portrait: 41, since: 'Since 1 Apr 2025', colour: '#43a047', grade: 0.72, happy: 0.66, stress: 0.45, friendly: 0.81, licence: 'D' },
  { id: 3, name: 'Pieter Bakker', role: 'Driver', icon: 'directions_bus', wage: 305000, portrait: 7, since: 'Since 12 May 2025', colour: '#1e88e5', grade: 0.79, happy: 0.58, stress: 0.61, friendly: 0.7, licence: 'D' },
  { id: 4, name: 'Sabine Krüger', role: 'Driver', icon: 'directions_bus', wage: 289000, portrait: 88, since: 'Since 2 Jun 2025', colour: '#c2185b', grade: 0.64, happy: 0.83, stress: 0.22, friendly: 0.88, licence: 'D' },
  { id: 5, name: 'Henk Janssen', role: 'Driver', icon: 'directions_bus', wage: 276000, portrait: 23, since: 'Since 15 Sep 2026', colour: '#fdd835', grade: 0.51, happy: 0.7, stress: 0.3, friendly: 0.62, licence: 'D1', trial: true },
  { id: 6, name: 'Mareike Vogel', role: 'Mechanic', icon: 'construction', wage: 331000, portrait: 55, since: 'Since 3 Mar 2025', colour: '#7b1fa2', grade: 0.81, happy: 0.74, stress: 0.4, friendly: 0.77 },
  { id: 7, name: 'Tobias Schmitt', role: 'Dispatcher', icon: 'support_agent', wage: 318000, portrait: 64, since: 'Since 3 Mar 2025', colour: '#455a64', grade: 0.77, happy: 0.69, stress: 0.5, friendly: 0.73 },
];
const drivers = () => STAFF.filter((e) => e.role === 'Driver');
const staffOf = (id) => STAFF.find((e) => e.id === id);
const firstName = (id) => staffOf(id)?.name.split(' ')[0] || '';

/** The day's tours (`DayPlan`): each line's tours, their bus and duties (who drives, from, to). */
const TOURS = [
  { line: '101', tour: '1', bus: 1, duties: [[5 * 60 + 20, 11 * 60 + 40, 1], [11 * 60 + 40, 18 * 60 + 10, 4]] },
  { line: '101', tour: '2', bus: 2, duties: [[6 * 60, 12 * 60 + 15, 2], [12 * 60 + 15, 19 * 60 + 5, null]] },
  { line: '102', tour: '1', bus: 3, duties: [[5 * 60 + 45, 12 * 60 + 30, 3], [12 * 60 + 30, 20 * 60, 5]] },
  { line: '102', tour: '2', bus: null, duties: [[14 * 60, 22 * 60 + 10, null]] },
  { line: '205', tour: '1', bus: 3, duties: [[6 * 60 + 30, 9 * 60 + 50, 'you'], [15 * 60 + 10, 18 * 60 + 40, 1]] },
];

// --- words ------------------------------------------------------------------------------------

const pad = (n) => String(n).padStart(2, '0');
const hhmm = (m) => `${pad(Math.floor(m / 60) % 24)}:${pad(Math.floor(m) % 60)}`;
const grouped = (n) => Math.round(n).toLocaleString('en-GB');
const eur = (cents) => (cents < 0 ? '-' : '') + '€' + grouped(Math.abs(cents) / 100);
const fine = (cents) => '€' + (cents / 100).toFixed(2);
const signed = (cents) => [(cents > 0 ? '+' : '') + eur(cents), cents > 0 ? 'ok' : cents < 0 ? 'danger' : 'text'];
const compact = (cents) => (Math.abs(cents) >= 100000000 ? (cents / 100000000).toFixed(1) + 'M' : Math.abs(cents) >= 100000 ? '€' + Math.round(cents / 100000) + 'k' : eur(cents));
const length = (m) => `${Math.floor(m / 60)} h ${pad(m % 60)}`;
const WD = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const WDL = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const MONL = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
const parse = (d) => new Date(d + 'T12:00:00Z');
const fmt = (t) => `${t.getUTCFullYear()}-${pad(t.getUTCMonth() + 1)}-${pad(t.getUTCDate())}`;
const addDays = (d, n) => fmt(new Date(parse(d).getTime() + n * 86400000));
const dayLabel = (d) => {
  const t = parse(d);
  return `${WD[t.getUTCDay()]} ${t.getUTCDate()} ${MON[t.getUTCMonth()]} ${t.getUTCFullYear()}`;
};
const shortDate = (d) => {
  const t = parse(d);
  return `${t.getUTCDate()} ${MON[t.getUTCMonth()]}`;
};
const ago = (minutes) => (minutes < 60 ? `${minutes} min` : minutes < 60 * 24 ? `${Math.floor(minutes / 60)} h` : minutes < 2 * 1440 ? '1 day' : `${Math.floor(minutes / 1440)} days`);
const hm = () => hhmm(co()?.minute || 0);

/** The last thirty days' results (cents), the oldest first: a quiet weekday rhythm. */
function lastDays() {
  return Array.from({ length: 30 }, (_, k) => {
    const wd = parse(addDays(co().date, k - 30)).getUTCDay();
    const base = wd === 0 ? -42000 : wd === 6 ? 18000 : 61000;
    return base + ((k * 7919) % 23) * 1700 - 19000;
  });
}
const monthResult = () => lastDays().slice(-19).reduce((s, x) => s + x, 0);

// --- the start: the chooser and the founding ----------------------------------------------------

function modeJson(key) {
  const m = { casual: ['Casual', 'autorenew'], normal: ['Normal', 'tune'], realistic: ['Realistic', 'badge'] }[key] || ['Normal', 'tune'];
  return { key, label: m[0], icon: m[1], casual: key === 'casual' };
}
const markOf = (c) => ({ short: c.short, colours: c.colours, logo: null });
const homeOf = (c) => `${c.map_name}  ·  ${c.depot}  ·  ${modeJson(c.mode).label}`;

function chooser() {
  const tiles = companies
    .filter((c) => !st.gone.has(c.id))
    .map((c, k) => {
      const month = c.id === 'grundorf-bus' ? 421500 : -38000;
      const [mt, mtone] = signed(month);
      return {
        id: c.id, name: c.name, mark: markOf(c), mode: modeJson(c.mode),
        clock: c.id === 'grundorf-bus' ? null : ['In step with the real clock', 'dim'],
        home: `${c.map_name}  ·  ${c.depot}`, day: `${dayLabel(c.date)}  ·  ${eur(c.cash)}`, negative: false, sandbox: null,
        map: `${c.map_name} (DE)`, map_file: c.map, colours: c.colours,
        pill: { text: modeJson(c.mode).label, tone: c.mode === 'realistic' ? 'accent' : c.mode === 'normal' ? 'early' : 'ok' },
        stats: [
          { label: 'Cash', icon: 'payments', value: compact(c.cash), full: eur(c.cash), tone: 'ok' },
          { label: 'Buses', icon: 'directions_bus', value: c.id === 'grundorf-bus' ? String(BUSES.length) : '2', tone: 'text' },
          { label: 'Lines', icon: 'route', value: c.id === 'grundorf-bus' ? String(LINES.length) : '1', tone: 'text' },
          { label: 'This month', icon: month < 0 ? 'trending_down' : 'trending_up', value: compact(month), full: mt, tone: mtone },
        ],
        when: `In-game ${dayLabel(c.date)}  ·  played ${ago(c.played)} ago`,
        last: k === 0,
      };
    });
  const n = tiles.length;
  return {
    modes: {
      title: 'How do you want to play?', text: 'Your own company in a time of its own, or one run together with others.',
      single: { title: 'Singleplayer', text: 'Found and run your own companies, in a time of their own that runs while you play.', foot: n === 1 ? '1 company on this computer' : `${n} companies on this computer` },
      multi: { title: 'Join a company', text: 'A friend runs a bus company: join it with their session code, get a role, and drive its duties in your own game.', foot: 'With a session code', tip: 'The boss shares the company on its Multiplayer page and sends you the code.', off: null },
    },
    companies: {
      back: 'Play mode',
      title: n ? 'Your companies' : 'Found your first company',
      text: n ? 'Choose a company to continue your bus operations, or create a new one.' : 'You have no company yet: give it a name, colours, a map and a depot.',
      tiles, continue: 'Continue', open: 'Open', last_label: 'Last played', new: 'New company', new_sub: 'Start a new bus company and build your network from scratch.', delete_tip: 'Delete this company',
      join: null,
      ask_title: 'Delete %{name}?', ask_text: 'The company is deleted for good: its buses, staff, lines, money and history. Your service record as a driver stays. This cannot be undone.',
      ask_yes: 'Delete for good', ask_no: 'Cancel',
    },
  };
}

function wizard(a) {
  const name = a.name || '';
  const short = name.split(/\s+/).filter(Boolean).map((w) => w[0].toUpperCase()).join('').slice(0, 4) || 'SG';
  const mode = a.mode || co()?.mode || 'casual';
  const map = Math.min(Number(a.map) || 0, 1);
  const cards = ['casual', 'normal', 'realistic'].map((m) => ({
    mode: modeJson(m),
    pitch: { casual: 'Drive and grow without paperwork: the company runs itself while you drive.', normal: 'Run the company yourself: buses, staff, lines and money.', realistic: 'Every rule of a real operator: licences, working hours, tenders.' }[m],
    capital: `Starting capital ${{ casual: '€250,000', normal: '€150,000', realistic: '€80,000' }[m]}`,
    bullets: { casual: ['The planning is done for you', 'No loans or tenders', 'Buy at the list price'], normal: ['Plan your own duties', 'Hire, train and pay staff', 'Loans and concessions'], realistic: ['Working-time rules', 'Tenders with penalties', 'RosterFlow add-on'] }[m],
  }));
  return {
    title: 'Found your bus company',
    intro: 'Run your own transport company on a map: buy, lease or rent buses, hire drivers and run the map\'s lines or your own. The company has its own clock: simulate its time by the hour or by days, and what happens is told as it comes; what you drive yourself counts as it was driven.',
    company_caps: 'THE COMPANY', name_label: 'Name', name_hint: 'Stadtbus Grundorf', short_label: 'Short name (plates and logo)', short_hint: short,
    colours: ['Main colour', 'Second colour'], palette: PALETTE,
    no_logo: 'No logo picture: the short name is the mark.', choose_logo: 'Choose a logo picture', remove_logo: 'Remove the picture',
    map_label: 'Home map', depot_label: "Depot (the buses' depot file)",
    maps: ['Grundorf', 'Ruhrau'], map, depots: map === 0 ? ['Grundorf.hof', 'grundorf-eigen.hof'] : ['Ruhrau.hof'],
    no_maps: 'No maps found: the company needs a map to be at home on.', no_depot: 'The map has no depot file.',
    mode_caps: 'GAME MODE', cards, mode,
    mode_note: 'Switch the game mode at any time in the company\'s settings: nothing is lost.',
    company_mode: {
      caps: 'COMPANY MODE', key: a.company_mode || 'career', note: 'A sandbox does not count for the rankings.',
      cards: [
        { key: 'career', label: 'Career', icon: 'military_tech', info: 'Earn your way up: levels, rankings and trophies.' },
        { key: 'sandbox', label: 'Sandbox', icon: 'sports_esports', info: 'Set the rules yourself: money, prices, what breaks.' },
      ],
    },
    country: { caps: 'COUNTRY', key: a.country || 'de', note: "The names of the company's people come from this country.", search: 'Search a country', none: 'No country found',
      options: [{ key: 'de', flag: '🇩🇪', label: 'Germany', words: 'germany deutschland' }, { key: 'nl', flag: '🇳🇱', label: 'Netherlands', words: 'netherlands nederland holland' }, { key: 'gb', flag: '🇬🇧', label: 'United Kingdom', words: 'united kingdom britain england' }, { key: 'pl', flag: '🇵🇱', label: 'Poland', words: 'poland polska' }] },
    currency: { label: 'Currency', codes: ['EUR', 'GBP', 'PLN'], options: ['Euro (€)', 'Pound (£)', 'Złoty (zł)'] },
    first_label: 'First company day', whens: ['Today', "The map's date", 'A date of my own'], when: Number(a.when) || 0, date: a.date || '2026-10-17',
    begins: `The company begins on ${dayLabel(a.date || '2026-10-17')}.`, logo: null,
    buying_label: 'Buying buses', casual_buying: 'Casual buys at the list price, in one click: the buses are yours at once.',
    buyings: ['Simple', 'By contract'], buying_says: ['A model, a number, the list price: the buses are yours at once.', 'Haggle with the dealer, agree on extras and sign a contract; new buses are delivered.'],
    vehicles: { caps: 'THE BUSES THE COMPANY WORKS WITH', proposed: 0, modes: [[0, "The map's region"], [1, 'Every bus'], [2, 'My own choice']], choose: 'Choose buses', say: "Buses of the map's region (Germany): 3 models." },
    map_file: map === 0 ? MAP : 'maps/Ruhrau/global.cfg',
    cancel: companies.length ? 'Cancel' : null, found: 'Found the company', need_name: 'Give the company a name.',
    popup_name: { icon: 'badge', title: 'A name first', text: 'The company needs a name: it is on its buses, its papers and its contracts.', unlock: 'Type one in the field "Name"; the short name is made of it.', go: null },
    popup_map: { icon: 'map', title: 'No map', text: 'The company needs a map to be at home on: none was found in the OMSI folder.', unlock: 'Install a map, or choose the OMSI folder under Setup.', go: null },
  };
}

// --- the shell the clock sends ------------------------------------------------------------------

const unread = () => MESSAGES.filter((m) => !st.read.has(m.id) && !m.deleted).length;

function badge(tab) {
  if (tab === 13) return unread() ? { n: String(unread()), tone: 'accent', why: `${unread()} unread` } : null;
  if (tab === 5) return { n: '2', tone: 'warn', why: '1 duty without a driver\n1 tour without a bus' };
  if (tab === 16) return { n: '1', tone: 'warn', why: '1 complaint to answer' };
  if (tab === 11) return BUSES.some((b) => b.workshop) ? { n: '1', tone: 'warn', why: '1 bus in the workshop' } : null;
  return null;
}

/** Casual's pages (`casual::ALL`): its tab, name, icon and the page above it. */
const CASUAL_PAGES = [[101, 'Buses', 'directions_bus', 100], [102, 'Lines', 'route', 100], [103, 'Drivers', 'groups', 100], [104, 'Money', 'payments', 100], [105, 'Company', 'garage', 100], [106, 'Drive a shift', 'play_arrow', 100], [107, 'Bus shop', 'storefront', 101], [108, 'A new line', 'add_road', 102]];

function shell() {
  const notices = st.notices.splice(0);
  const base = { gen: st.gen, notices, popup: null, companies: companies.filter((c) => !st.gone.has(c.id)).length };
  const c = co();
  if (!c) return { open: false, ...base };
  const casual = c.mode === 'casual';
  const pages = RAIL.flat();
  const rail = RAIL.map((g, i) =>
    g.map((tab) => {
      const b = badge(tab);
      return { tab, icon: ICONS[tab], name: TABS[tab], badge: b && { n: b.n, tone: b.tone }, tip: b ? `${TABS[tab]}\n${b.why}` : TABS[tab], away: false, group: RAIL_HEADS[i] };
    }),
  );
  return {
    open: true, ...base,
    id: c.id, name: c.name, mark: markOf(c), mode: modeJson(c.mode), home: homeOf(c), sandbox: null,
    status: [
      { icon: 'payments', value: eur(c.cash), tone: 'ok', text: 'text', tip: 'Cash: the money the company has now', tab: casual ? null : 4 },
      { icon: 'emoji_events', value: grouped(c.trophies), tone: 'warn', text: 'text', tip: 'Trophies: earned with goals and contracts, spent on research', tab: casual ? null : 10 },
      { icon: 'star', value: `${c.reputation.toFixed(1)} %`, tone: 'early', text: 'text', tip: 'Reputation: what passengers and the authority think of the company', tab: casual ? null : 16 },
    ],
    clock: {
      label: 'COMPANY TIME', date: dayLabel(c.date), time: hm(), live: st.realtime, in_game: false, lead: null, speed: st.speed, real: st.realtime,
      tip: st.realtime ? 'In step with the real clock: the company\'s time goes on while Omsi-Hub is closed, and is caught up when you open the company.' : "The company's time stands still while it is closed: the real clock is off in the company's settings.",
    },
    rail: casual ? [CASUAL_PAGES.slice(0, 5).map(([tab, name, icon]) => ({ tab, icon, name, tip: name, badge: null, away: false }))] : rail, pages: casual ? CASUAL_PAGES.slice(0, 5).map((x) => x[0]) : pages, together: { on: true, label: '' }, bell: unread() + 2,
    names: TABS, casual, cover: null, asking: false, reports: !!st.reports, away: false, launch: !!st.launch, sheet: false,
    casual_pages: casual ? { pages: Object.fromEntries([[HOME_TAB, 'Home', 'home', HOME_TAB], ...CASUAL_PAGES].map(([tab, name, icon, parent]) => [tab, { name, icon, parent }])), home: `${c.name} · Casual` } : null,
    today_read: true,
    money: { code: 'EUR', rate: 1, symbol: '€', at: 'before', thousands: ',', decimal: '.', places: 2, fare_step: 0.05 },
  };
}

/** The clock runs on (minutes): the hour turned asks the views again; midnight closes the day. */
function runClock(minutes) {
  const c = co();
  if (!c) return;
  const before = Math.floor(c.minute / 60);
  c.minute += minutes;
  while (c.minute >= 1440) {
    c.minute -= 1440;
    c.date = addDays(c.date, 1);
    const [r] = signed(lastDays()[29]);
    c.cash += lastDays()[29];
    say(`The day is closed: ${r}.`);
  }
  if (Math.floor(c.minute / 60) !== before) bump();
}

// --- the views ---------------------------------------------------------------------------------

function tile(to, name, icon, lines, more = {}) {
  const k = to.startsWith('tab:') ? Number(to.slice(4)) : null;
  const b = k != null ? badge(k) : null;
  return { to, name, icon, lines, badge: b && { n: b.n, tone: b.tone }, tip: b?.why || '', big: false, extra: null, ...more };
}

function home() {
  const c = co();
  const [res, tone] = signed(monthResult());
  const staff = STAFF;
  const sections = [
    ['At a glance', [
      tile('tab:0', 'Overview', ICONS[0], [[`This month: ${res}`, tone]], { big: true, extra: { kind: 'money', cash: eur(c.cash), tone: 'ok', days: lastDays(), none: 'No day closed yet' } }),
      tile('tab:14', 'Agenda', ICONS[14], [['2 things need you this week', 'warn']], { big: true, badge: { n: '2', tone: 'warn' }, extra: { kind: 'agenda', title: 'Coming up', list: agendaItems().slice(0, 8), none: 'Nothing coming up' } }),
      tile('tab:13', 'Inbox', ICONS[13], [[unread() ? `${unread()} unread` : 'Nothing unread', unread() ? 'accent2' : 'dim']], { big: true, extra: { kind: 'messages', list: MESSAGES.filter((m) => !m.deleted).slice(0, 8).map((m) => [m.title, shortDate(m.date), !st.read.has(m.id)]), none: 'No messages yet' } }),
    ]],
    ['Operations', [
      tile('tab:5', 'Planning', ICONS[5], [['Covered today: 3 of 5 tours', 'warn'], ['1 without a bus · 2 without a driver', 'dim']]),
      tile('tab:15', 'Operations', ICONS[15], [['1 bus running late', 'warn'], ['Service control: bring late buses back on time', 'dim']]),
      tile('tab:3', 'Lines', ICONS[3], [[`${LINES.length} lines`, 'soft'], ['All in service', 'dim']]),
      tile('tab:9', 'Map', ICONS[9], [['3 buses on the road now', 'soft'], ['Every line and bus where it is', 'dim']]),
      tile('tab:1', 'Fleet', ICONS[1], [[`${BUSES.length} buses`, 'soft'], ['3 ready today', 'dim'], ['1 in the workshop', 'warn']]),
    ]],
    ['People', [
      tile('tab:2', 'Staff', ICONS[2], [[`${staff.length} people`, 'soft'], [`${staff.length - 1} at work today`, 'dim']], { big: true, extra: { kind: 'roles', list: [['Driver', 5], ['Mechanic', 1], ['Dispatcher', 1]] } }),
      tile('hire', 'Hire staff', 'person', [['6 applicants this week', 'soft']]),
      tile('courses', 'Training', 'badge', [['Courses for drivers and staff', 'dim']]),
    ]],
    ['Business', [
      tile('tab:4', 'Finances', ICONS[4], [[`This month: ${res}`, tone], ['Loans: €60,000', 'dim']]),
      tile('tab:8', 'Concessions', ICONS[8], [['2 concessions held', 'soft'], ['1 tender open', 'dim']]),
      tile('tab:12', 'Contracts', ICONS[12], [[`${c.trophies} trophies`, 'soft'], ['2 goals running', 'dim']]),
      tile('tab:16', 'Reputation', ICONS[16], [[`${Math.round(c.reputation)} of 100`, 'soft'], ['1 complaint to answer', 'warn']]),
      tile('tab:6', 'Career', ICONS[6], [['Level 4  ·  Route driver', 'soft'], ['2,140 of 3,000 points', 'dim']]),
    ]],
    ['Company', [
      tile('tab:7', 'Depot', ICONS[7], [['4 of 8 places taken', 'soft']]),
      tile('tab:11', 'Workshop', ICONS[11], [['1 bus in the workshop', 'soft'], ['Spare parts and repairs', 'dim']]),
      tile('tab:10', 'Research', ICONS[10], [['Ticket machines  ·  40%', 'soft'], ['3 researched', 'dim']]),
      tile('settings', 'Settings', 'settings', [['Name, mark, date and buying', 'dim']]),
    ]],
  ].map(([title, tiles]) => ({ title, dot: tiles.some((t) => t.badge), tiles }));
  const ways = [['dealer', 'Dealer', 'airport_shuttle'], ['adverts', 'Advertising', 'campaign'], ['loan', 'Take a loan', 'receipt_long'], ['new_line', 'Make a new line', 'alt_route'], ['licences', 'Train drivers', 'key'], ['my_duties', 'My duties', 'schedule'], ['rankings', 'Rankings', 'stars'], ['statistics', 'Statistics', 'trending_up'], ['report', 'Show the report', 'description'], ['chooser', 'Other company', 'sync_alt']].map(([to, label, icon]) => ({ to, label, icon }));
  return { sections, ways };
}

const MESSAGES = [
  { id: 1, icon: 'construction', tone: 'warn', kind: 'Event', title: 'Road works on the Hauptstraße', text: 'The town digs up the Hauptstraße for a new water main: line 101 takes the diversion over the Kirchweg, five minutes longer each way.', date: '2026-10-18',
    event: { title: 'WHAT IT DOES', effects: [['Line 101: +5 minutes a trip', 'warn'], ['Passengers on line 101: -8 %', 'danger']], status: 'Until Fri 23 Oct', days: [] }, go: { icon: 'route', label: 'To the lines', tab: 3 } },
  { id: 2, icon: 'emoji_events', tone: 'ok', kind: 'Goal', title: 'Goal: 95 % on time this week', text: 'The authority rewards a punctual week: keep 95 % of the trips within two minutes of the timetable.', date: '2026-10-19',
    goal: { trophies: '3', reward: '€2,500', bar: { frac: 0.72, tone: 'ok', label: '91 % of 95 %' }, rule: 'Counted Monday to Sunday, every trip of the company.', state: 'Running: 5 days left' }, go: { icon: 'description', label: 'To the contracts', tab: 12 } },
  { id: 3, icon: 'directions_bus', tone: 'danger', kind: 'Fleet', title: 'Bus 2571 broke down', text: 'The SD202 did not start this morning: it was towed to the workshop. The mechanic expects two days.', date: '2026-10-19', go: { icon: 'construction', label: 'To the workshop', tab: 11 } },
  { id: 4, icon: 'newspaper', tone: 'accent', kind: 'Press', title: 'Grundorfer Anzeiger: "Clean buses, friendly drivers"', text: 'The local paper rode line 102 for a week and liked what it saw. Reputation +1.5.', date: '2026-10-16' },
  { id: 5, icon: 'gavel', tone: 'early', kind: 'Tender', title: 'Tender open: line 310 Grundorf – Flughafen', text: 'The district asks for bids for a new airport line, eight years from 1 January. Bids close on 30 October.', date: '2026-10-15', go: { icon: 'public', label: 'To the concessions', tab: 8 } },
];

function agendaItems() {
  const d = co().date;
  return [
    { date: d, icon: 'person_off', tone: 'warn', time: '14:00', text: 'Line 102, tour 2: no driver yet', note: 'From 14:00 to 22:10', needs: true, go: { tab: 5 }, when: 'Today', group: 'planning' },
    { date: d, icon: 'construction', tone: 'accent2', time: '', text: 'Bus 2571 in the workshop', note: 'Back on Wed 21 Oct', needs: false, go: { tab: 11 }, when: 'Today', group: 'fleet' },
    { date: addDays(d, 1), icon: 'beach_access', tone: 'early', time: '', text: 'Anke Müller on holiday', note: 'Until Fri 23 Oct', needs: false, go: { ui: { people: { tab: 4 } }, tab: 2 }, when: 'Tomorrow', group: 'staff' },
    { date: addDays(d, 2), icon: 'directions_bus', tone: 'ok', time: '', text: 'Bus 2571 back from the workshop', note: '', needs: false, go: { tab: 1 }, when: 'Wed', group: 'fleet' },
    { date: addDays(d, 4), icon: 'event_busy', tone: 'warn', time: '', text: 'Road works on the Hauptstraße end', note: 'Line 101 back on its route', needs: false, go: null, when: 'Fri', group: 'events' },
    { date: addDays(d, 11), icon: 'gavel', tone: 'danger', time: '12:00', text: 'Bids close: line 310', note: 'Grundorf – Flughafen', needs: true, go: { tab: 8 }, when: '30 Oct', group: 'tenders' },
  ];
}

function bell() {
  const msgs = MESSAGES.filter((m) => !m.deleted && !st.read.has(m.id));
  const needs = agendaItems().filter((e) => e.needs);
  return {
    all: 'All',
    inbox: { title: 'Inbox', n: msgs.length, list: msgs.map((m) => ({ id: m.id, icon: m.icon, tone: m.tone, title: m.title, date: shortDate(m.date) })), none: 'Nothing unread' },
    agenda: { title: 'Needs you', n: needs.length, list: needs, none: 'Nothing needs you this week' },
  };
}

function overview() {
  const c = co();
  const days = lastDays().slice(-14);
  const hi = Math.max(...days);
  const lo = Math.min(0, ...days);
  const top = Math.ceil(hi / 20000) * 20000;
  const bottom = Math.floor(lo / 20000) * 20000;
  const scale = [];
  for (let v = bottom; v <= top; v += 20000) scale.push([v, eur(v)]);
  const [res, tone] = signed(monthResult());
  return {
    figures: [
      { label: 'Cash', value: eur(c.cash), under: 'Credit room €84,000', tone: 'ok', tip: 'The money the company has now' },
      { label: 'This month', value: res, under: 'Since 1 Oct', tone },
      { label: 'Passengers', value: grouped(3695), under: 'Yesterday', tone: 'text' },
      { label: 'On time', value: '91 %', under: 'This week', tone: 'warn' },
      { label: 'Fleet', value: `${BUSES.length - 1} of ${BUSES.length}`, under: '1 in the workshop', tone: 'text' },
      { label: 'Staff', value: String(STAFF.length), under: '5 drivers', tone: 'text' },
    ],
    chart_title: 'THE LAST 14 DAYS',
    chart: { empty: false, top, bottom, scale, span: 14, dates: days.map((_, k) => shortDate(addDays(c.date, k - 14))), bars: days.map((v, k) => ({ k, v, tip: `${shortDate(addDays(c.date, k - 14))}: ${signed(v)[0]}` })) },
    needs: {
      title: 'NEEDS YOU', none: 'Nothing needs you now.',
      list: [
        { icon: 'person_off', tone: 'warn', text: 'Line 102, tour 2 has no driver today from 14:00', button: 'Plan', go: { tab: 5 } },
        { icon: 'directions_bus', tone: 'danger', text: 'Line 102, tour 2 has no bus today', button: 'Plan', go: { tab: 5 } },
        { icon: 'sentiment_dissatisfied', tone: 'warn', text: 'A passenger complains about a missed connection at the Bahnhof', button: 'Answer', go: { tab: 16 } },
        { icon: 'gavel', tone: 'accent', text: 'Tender open: line 310 Grundorf – Flughafen', button: 'Bid', go: { tab: 8 } },
      ],
    },
    feed: feed(),
    today: {
      title: 'TODAY',
      covered: { bar: { label: 'Tours covered', right: '3 of 5', frac: 0.6, tone: 'warn', tip: 'Tours with a bus and every duty driven', click: true } },
      next: { duty: 'grundorf-bus|2026-10-19|205|1|0', text: 'Your next duty: 205 tour 1, 06:30 – 09:50', tip: 'Line 205 · Grundorf – Ruhrau', button: 'Drive' },
    },
    last: { title: 'THE LAST DAY CLOSED', none: 'No day closed yet.', button: 'Report', report: { date: dayLabel(addDays(c.date, -1)), result: signed(lastDays()[29])[0], tone: signed(lastDays()[29])[1], tip: 'Income less costs', facts: ['3,695 passengers', '94 % of the trips on time', 'No breakdown'] } },
  };
}

function feed() {
  const rows = [
    { day: 'TODAY' },
    { at: '07:40', tone: 'warn', text: 'Bus 18 runs 4 minutes late on line 101 (Marktplatz).', minor: false },
    { at: '07:12', tone: 'ok', text: 'Jan de Vries signed on for line 101, tour 1.', minor: true },
    { at: '06:30', tone: 'accent', text: 'You signed on for line 205, tour 1.', minor: false },
    { at: '05:58', tone: 'danger', text: 'Bus 2571 did not start: towed to the workshop (€320).', minor: false },
    { day: 'YESTERDAY' },
    { at: '23:55', tone: 'ok', text: 'The day is closed: +€1,840.', minor: false },
    { at: '18:20', tone: 'text', text: 'Sabine Krüger handed bus 17 over: "Wiper squeaks".', minor: true },
  ].filter((r) => st.minor || !r.minor);
  return { title: "THE COMPANY'S DAY", toggle: 'Every event', tip: 'Show the ordinary run of the day too', minor: st.minor, empty: null, rows };
}

// - the fleet -

const condTone = (f) => (f >= 0.7 ? 'ok' : f >= 0.45 ? 'warn' : 'danger');
function fleet(a) {
  const tab = Math.min(Number(a.tab) || 0, 2);
  const tabs = [`Buses (${BUSES.length})`, 'Dealer', 'Advertising'];
  let body;
  if (tab === 2) {
    body = {
      kind: 'adverts', offers_title: 'OFFERS THIS WEEK', note: 'Advertisers pay for the outside of your buses: a contract runs for weeks, the bus wears their livery.', to_contract: 'To the contract', no_offers: 'No offers this week.',
      offers: [
        { no: 1, advertiser: 'Bäckerei Hoffmann', trade: 'Bakery', terms: '2 buses · 12 weeks', tone: 'ok', money: '€180 a week', all: '€4,320 in all', lock: null },
        { no: 2, advertiser: 'Autohaus Grundorf', trade: 'Car dealer', terms: '1 articulated bus · 26 weeks', tone: 'accent2', money: '€140 a week', all: '€3,640 in all', lock: 'Needs an articulated bus free' },
      ],
      running_title: 'CONTRACTS RUNNING', no_running: 'No advertising runs.', look: 'Look', end_tip: 'Ending early costs a fee',
      running: [{ no: 7, advertiser: 'Stadtwerke Grundorf', tone: 'early', what: 'Bus 17 · full wrap', until: 'Until 31 Dec 2026', end: 'End early (€300)', again: 'Press again to end' }],
    };
  } else if (tab === 1) {
    body = { kind: 'dealer' };
  } else {
    body = {
      kind: 'buses', order: 'An MAN NL202 is on order: delivered on Mon 2 Nov', empty: 'No bus yet.', to_dealer: 'To the dealer',
      number_tip: "The bus's fleet number: type your own, or leave it empty for the automatic one.",
      buses: BUSES.map((b) => ({
        id: b.id, number: b.number, auto: b.auto ?? b.number, own: b.auto != null, name: b.name, file: b.file, livery: b.livery, advert: b.id === 1 ? 'Stadtwerke Grundorf' : null,
        sub: `${b.plate}  ·  ${b.kind}`, facts: `${grouped(b.km)} km  ·  ${2026 - b.year} years`,
        condition: { frac: b.cond, tone: condTone(b.cond), tip: `Condition ${Math.round(b.cond * 100)} %` },
        tone: b.workshop ? 'danger' : b.id === 4 ? 'warn' : 'ok', status: b.workshop ? 'In the workshop until Wed' : TOURS.some((t) => t.bus === b.id) ? 'On line ' + TOURS.find((t) => t.bus === b.id).line : 'In the depot',
      })),
    };
  }
  return { tabs, tab, body };
}

function vehicle(a) {
  const b = BUSES.find((x) => x.id === Number(a.id));
  if (!b) return { gone: true };
  return {
    title: `Bus ${b.number}  ·  ${b.name}`, file: b.file, name: b.name, bus_kind: b.kind,
    livery: { label: 'Livery', value: b.livery, options: [['Grundorf', 'Grundorf'], ['Weiss', 'White'], ['BVG', 'BVG']] },
    design: 'Design a livery', design_tip: 'Open the livery studio on this bus',
    rows: [
      { a: 'Plate', b: b.plate }, { a: 'First registered', b: String(b.year) }, { a: 'Kilometres', b: `${grouped(b.km)} km` },
      { a: 'Condition', b: `${Math.round(b.cond * 100)} %`, tone: condTone(b.cond) }, { a: 'How it is held', b: b.how }, { a: 'Worth now', b: eur(Math.round((1 - (2026 - b.year) / 45) * 28000000)), strong: true },
    ],
    age: { on: false, year: String(b.year), label: 'Set its age myself', placeholder: 'Year of first registration', apply: 'Apply', tip: 'The age counts for its value and its breakdowns' },
    tone: b.workshop ? 'danger' : 'ok', status: b.workshop ? 'In the workshop: back on Wed 21 Oct' : 'Ready for service',
    crew: { notes_title: 'HANDOVER NOTES TODAY', no_notes: 'No notes today.', notes: b.id === 1 ? [{ says: 'Wiper squeaks', at: '18:20', by: 'Sabine Krüger' }] : [], defects_title: 'DEFECTS', no_defects: 'No defects.', defects: b.workshop ? [{ icon: 'build', bus: b.number, kind: 'Starter motor', text: 'did not start', date: '19 Oct', at: '05:58', who: 'Pieter Bakker', held_text: 'held in the workshop', tone: 'warn', state: 'Being repaired' }] : [] },
    sell: b.how === 'Bought' ? 'Sell' : 'Give back', service: 'Service', train: 'Train drivers', close: 'Close',
  };
}

// - the staff -

const need = (label, frac, tip, bold) => ({ label, frac, tone: label === 'Stress' ? (frac > 0.55 ? 'danger' : frac > 0.4 ? 'warn' : 'ok') : condTone(frac), tip, bold: !!bold });
const needsOf = (e) => [need('Grade', e.grade, 'How well they work', true), need('Satisfaction', e.happy), need('Stress', e.stress), need('Friendliness', e.friendly)];
function doing(e) {
  const t = TOURS.find((x) => x.duties.some((d) => d[2] === e.id && d[0] <= co().minute && co().minute < d[1]));
  if (e.role !== 'Driver') return ['At work', '08:00 – 16:30', 'ok'];
  if (t) return [`On line ${t.line}`, `Tour ${t.tour}`, 'accent'];
  return e.id === 2 ? ['On holiday from tomorrow', '', 'early'] : ['Free now', 'Next duty later today', 'soft'];
}

function people(a) {
  const tab = Math.min(Number(a.tab) || 0, 6);
  const tabs = [`My employees (${STAFF.length})`, 'Hire employees', 'Licences & training', 'Work schedule', 'Holidays', 'Agencies', 'Settings'];
  let body;
  const roles = ['Driver', 'Mechanic', 'Dispatcher', 'Cleaner', 'Ticket inspector'];
  if (tab === 0 && a.open != null && staffOf(Number(a.open))) {
    const e = staffOf(Number(a.open));
    body = {
      kind: 'person', id: e.id, back: 'Back to the employees', portrait: e.portrait, name: e.name, role: e.role, since: e.since, doing: doing(e), leaves: null,
      needs_title: 'HOW THEY ARE', needs: needsOf(e).map((n) => ({ ...n, value: `${Math.round(n.frac * 100)} %`, note: n.label === 'Stress' ? 'Long duties raise it, days off lower it' : 'Pay, rest and a good bus raise it' })),
      pay_title: 'PAY', pay: [['Gross wage a month', eur(e.wage)], ["With the employer's share", eur(Math.round(e.wage * 1.21))], ['Hourly', fine(Math.round(e.wage / 165))]], hourly_note: 'Paid for the hours on duty, at least 120 a month.',
      ask: e.id === 3 ? 'Asks for a raise of 5 %' : null, grant: 'Grant', refuse: 'Refuse', raise: 'Raise 5 %', keep: e.trial ? 'Keep after the trial' : null, dismiss: 'Dismiss',
      work_title: 'WORK', facts: [['Duties this month', '14'], ['Hours this week', '31 h 20'], ['Trips on time', '93 %'], ['Licence', e.licence || '-']],
      courses_title: 'COURSES', courses: [
        { icon: 'badge', label: 'Articulated buses', tip: 'May drive articulated buses', state: { done: e.id !== 5 } },
        { icon: 'school', label: 'Customer service', tip: 'Friendlier to passengers', state: e.id === 4 ? { until: 'Until Thu 22 Oct' } : { book: 'Book (€450)', kind: 'service' } },
        { icon: 'eco', label: 'Eco driving', tip: 'Uses less diesel', state: { lock: 'Research first', kind: 'eco' } },
      ],
      licences_note: e.licence === 'D1' ? 'Licence D1: midibuses only until the D exam.' : null,
    };
  } else if (tab === 0) {
    body = {
      kind: 'list', filter: Number(a.filter) || 0, chips: ['All (7)', 'Drivers (5)', 'Workshop (1)', 'Office (1)'], sort: Number(a.sort) || 0, sorts: ['Role', 'Name', 'Wage', 'Grade', 'Satisfaction'], sort_label: 'Sort by', descending: !!a.descending, dir_tip: 'Ascending or descending', empty: null, to_applicants: 'To the applicants',
      cards: STAFF.map((e) => ({ id: e.id, portrait: e.portrait, name: e.name, role: e.role, icon: e.icon, wage: `${eur(e.wage)} a month`, tag: e.trial ? ['On trial', 'warn'] : null, doing: doing(e), needs: needsOf(e) })),
    };
  } else if (tab === 1) {
    const pick = Math.min(Number(a.hire_role) || 0, roles.length - 1);
    const names = [['Lukas Weber', 34, 'D', 'Bus driver for 6 years'], ['Fenna Visser', 27, 'D', 'Bus driver for 2 years'], ['Murat Yilmaz', 45, 'D1', 'Lorry driver, D1 since 2019']];
    body = {
      kind: 'hire', pick, chips: roles.map((r, k) => `${r} (${k === 0 ? 3 : k === 1 ? 2 : 1})`), hire: 'Hire', none: 'Nobody applies for this role this week.', locked: null,
      advert: { label: 'Job advert (€600)', tip: 'New applicants at once', why: null, prominent: false, none_left: null, round: 'New applicants on Monday' },
      foot: 'Applicants stay a week; who is not hired looks elsewhere.',
      cards: (pick === 0 ? names : names.slice(0, pick === 1 ? 2 : 1)).map(([name, age, lic, who], k) => ({
        no: 10 + k, portrait: 100 + k * 13, name, who: `${age} years  ·  ${who}`, grade: `${68 + k * 7} %`, grade_tone: 'ok', frac: 0.68 + k * 0.07, grade_tip: 'How well they will work',
        asks: `Asks ${eur(285000 + k * 9000)} a month`, against: 'Your drivers earn €296,000 on average',
        bring: pick === 0 ? { licence: `Licence ${lic}`, d1: lic === 'D1', licence_tip: 'Which buses they may drive', skills: 'Articulated buses', skills_tip: 'Courses they bring' } : { licence: null, friendliness: 'Friendliness', value: `${70 + k * 5} %`, tone: 'ok' },
        reliable: k === 2 ? 'Was ill often' : 'Reliable', reliable_tip: 'What their last employer said',
      })),
    };
  } else if (tab === 2) {
    const group = Math.min(Number(a.group) || 0, 2);
    const heads = [
      { k: 0, icon: 'badge', title: 'Licence D', under: 'Every bus', count: '4 of 5', all: false, open: true, tip: 'The full bus licence', kind: 'd', train_all: 'Train all' },
      { k: 1, icon: 'airport_shuttle', title: 'Articulated', under: '18 m buses', count: '4 of 5', all: false, open: true, tip: 'Articulated buses', kind: 'artic', train_all: 'Train all' },
      { k: 2, icon: 'school', title: 'Customer service', under: 'Friendliness +10 %', count: '1 of 5', all: false, open: true, tip: 'A day course', kind: 'service', train_all: 'Train all' },
      { k: 3, icon: 'eco', title: 'Eco driving', under: 'Diesel -6 %', count: '0 of 5', all: false, open: false, tip: 'Needs research', kind: 'eco', lock: 'Research first' },
    ];
    body = {
      kind: 'training', group, groups: ['Licences', 'Bus types', 'Courses'], nobody: null, none: null, driver: 'DRIVER', first: 0, per: 6, total: heads.length, paging: null, prev_tip: 'Earlier', next_tip: 'Later',
      heads, rows: drivers().map((e) => ({ id: e.id, name: e.name, licence: `Licence ${e.licence}`, cells: heads.map((h) => (h.lock ? { lock: true } : e.id === 5 && h.k < 2 ? { book: 'Book' } : e.id === 4 && h.k === 2 ? { until: 'Thu 22 Oct' } : h.k === 2 && e.id !== 1 ? { book: 'Book' } : { has: true })) })),
      foot: 'A course takes the driver off the roster on its days; the company pays for it.',
    };
  } else if (tab === 3) {
    body = {
      kind: 'schedule', pick: Math.min(Number(a.hours_role) || 0, 1), chips: ['Mechanic', 'Dispatcher'], open: true, from: 8 * 60, to: 16 * 60 + 30, outside: 1, most: 3,
      core: 'CORE HOURS', all_there: 'Everyone of the role is at work in these hours.', outside_title: 'ON CALL OUTSIDE THEM', notes: ['Outside the core hours one is on call for breakdowns.', 'On call costs a quarter of the wage.', 'Night work needs the research "Night shifts".'], locked: null,
      today_title: 'TODAY', rows: [['At work', '1 of 1'], ['On call', '1'], ['Hours this week', '32 h']], each_title: 'EACH', each: [['Mareike Vogel', '08:00 – 16:30', false]],
    };
  } else if (tab === 4) {
    const days = Array.from({ length: 28 }, (_, k) => {
      const date = addDays(co().date, k);
      const t = parse(date);
      const off = k >= 1 && k <= 4 ? 1 : 0;
      return { date, day: String(t.getUTCDate()), month: t.getUTCDate() === 1 || k === 0 ? MON[t.getUTCMonth()] : null, gone: false, today: k === 0, on: k === 0, count: `${off}/2`, asked: k === 9 ? 1 : 0, frac: off / 2, tone: off ? 'early' : 'dim', short: 0, tip: off ? 'Anke Müller' : '' };
    });
    body = {
      kind: 'holidays', limit_title: 'AT MOST OFF AT ONCE', limit: 2, most: 5, fewer: 'Fewer', more: 'More', by_fleet: 'By the fleet: 1', own: true, reset: 'By the fleet', mode: 'Requests are answered by you.',
      calendar_title: 'THE COMING FOUR WEEKS', weekdays: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'], days, legend: [['Off', 'early'], ['Asked', 'warn'], ['Too few left', 'danger']],
      day: { title: dayLabel(co().date), off_title: 'Off', off: [], asked_title: 'Asked', asked: [], nobody: 'Nobody', cover: 'Every duty can be covered.', cover_tone: 'ok' },
      notes: ['Each employee has 28 days a year.'], waiting_title: 'WAITING FOR YOUR ANSWER', none: 'No request waits.', done_title: 'ANSWERED',
      waiting: [holiday(1, true)], done: [holiday(2, false)],
    };
  } else if (tab === 5) {
    const days = Array.from({ length: 21 }, (_, k) => {
      const date = addDays(co().date, k);
      const t = parse(date);
      return { date, day: String(t.getUTCDate()), month: k === 0 ? MON[t.getUTCMonth()] : null, gone: false, today: k === 0, on: date === (a.aday || co().date), count: k === 3 ? '1' : null, frac: 0.5, tone: 'accent2', tip: '' };
    });
    body = {
      kind: 'agencies', calendar_title: 'THE COMING THREE WEEKS', weekdays: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'], days, title: 'BOOK AGENCY DRIVERS',
      day: { date: a.aday || co().date, title: dayLabel(a.aday || co().date), why: null, count_title: 'Drivers', count: Number(a.acount) || 1, most: 4, fewer: 'Fewer', more: 'More', mode: Number(a.amode) || 0, modes: ['Whole day', 'Some hours', 'Open duties'], from: Number(a.afrom) || 6 * 60, to: Number(a.ato) || 14 * 60,
        duties: [{ key: '102|2|0', label: 'Line 102, tour 2, duty 1', time: '14:00 – 22:10', on: (a.aduties || []).includes('102|2|0') }], no_duties: 'No open duty that day.', far: 'Too far ahead: the timetable is not read yet.',
        mode_note: 'An agency driver costs €38 an hour, at least 4 hours.', book: 'Book', quote: '€304 for the day', quote_tone: 'soft' },
      notes: ['Booked drivers come whether you need them or not.'], coming_title: 'BOOKED', none: 'Nothing booked.', lately_title: 'LATELY',
      coming: [{ no: 1, date: dayLabel(addDays(co().date, 3)), cost: '€304', what: '1 driver · whole day', names: 'Agentur Mobil', given: null, cancel: 'Cancel', fee: 'Cancelling now costs €60', gone: false }], lately: [],
    };
  } else {
    body = {
      kind: 'settings', auto: true, auto_label: 'Hire by myself when too few', auto_note: 'The office hires from the applicants when a role has fewer than its minimum.',
      roles: roles.slice(0, 3).map((r, k) => ({ role: r, icon: ['directions_bus', 'construction', 'support_agent'][k], name: r, n: [5, 1, 1][k], now: `${[5, 1, 1][k]} now` })), fewer: 'Fewer', more: 'More',
      follow: true, follow_label: 'Follow the fleet', follow_note: 'The minimums grow with the buses.', grant: false, grant_label: 'Grant raises by myself', grant_note: 'Raises of up to 3 % are granted without asking.',
    };
  }
  return { tabs, tab, info: `Wages ${eur(STAFF.reduce((s, e) => s + Math.round(e.wage * 1.21), 0))} a month with the employer's share  ·  8 duties today`, body };
}

function holiday(no, open) {
  const e = staffOf(open ? 3 : 2);
  return {
    no, portrait: e.portrait, name: e.name, role: e.role, role_icon: e.icon, icon: 'beach_access', dates: open ? '2 Nov – 6 Nov' : '20 Oct – 23 Oct', length: open ? '5 days' : '4 days',
    reason: open ? 'Family visit' : 'Autumn holiday', priority: open ? ['Matters a lot', 'warn'] : ['Would like it', 'dim'], refused: null, asked: 'Asked on 12 Oct', due: open ? 'Answer by 26 Oct' : null, left: open ? '14 days left' : '18 days left',
    advice: open ? { approve: true, text: 'The planner says: approve, the duties can be covered.' } : null, over: null, open, approve: 'Approve', reject: 'Reject', state: open ? '' : 'Approved', tone: 'ok',
  };
}

// - the lines -

function lines(a) {
  const sel = a.selected && lineOf(a.selected) ? a.selected : null;
  const rows = LINES.map((l) => ({
    name: l.name, on: l.name === sel, plate: plate(l), caption: l.caption, own: l.name === '205' ? ['Own', 'A line of your own, made in the line editor'] : null, pending: null,
    fare: fine(l.fare), fare_tip: 'The fare: what a trip costs', edit: editTo(l) ? 'Edit the line' : null, edit_to: editTo(l), timetable: 'Timetable', remove_tip: 'Give the line up',
    bar: { label: 'Covered today', right: l.name === '102' ? '1 of 2' : '2 of 2', frac: l.name === '102' ? 0.5 : 1, tone: l.name === '102' ? 'warn' : 'ok', tip: 'Tours with a bus and every duty driven', click: 'planning' },
    km: `${l.km} km`,
  }));
  const right = {};
  if (sel) {
    const l = lineOf(sel);
    right.tours = {
      card: {
        title: 'IN PUBLIC', public: l.public, public_caps: 'ITS TITLE', hint: l.caption, save: 'Save', hof_caps: 'DEPOT FILE', hof: l.hof, hofs: [['Grundorf.hof', 'Grundorf.hof'], ['grundorf-eigen.hof', 'Grundorf (own)']], hof_words: 'Its destinations are in the depot file.', hof_tone: 'ok',
        advert_caps: 'HOW IT IS ADVERTISED', advert_colour: l.colour, plate: plate(l), public_name: `Line ${l.number}`, under: l.caption, colour_caps: 'COLOUR', palette: PALETTE.slice(0, 10), colour: l.colour, more_tip: 'Another colour',
      },
      income: {
        title: 'PASSENGERS AND TAKINGS', type_caps: 'KIND OF LINE', type: l.type, types: [['city', 'City line'], ['regional', 'Regional line'], ['school', 'School line'], ['night', 'Night line']], type_note: 'The kind sets who rides and when.', authority: null, figures: `${grouped(l.pax)} passengers a day`,
        fares_caps: 'FARES', fare_tip: 'Change the fare', fares: [['Single', fine(l.fare)], ['Day ticket', fine(l.fare * 2.4)], ['Month', eur(l.fare * 30)]],
        bands_caps: 'BY THE TIME OF DAY', bands_note: 'Passengers yesterday', none: null, bands: [['Early', '210', 0.3], ['Morning peak', '620', 1], ['Day', '480', 0.77], ['Evening peak', '410', 0.66], ['Evening', '120', 0.19]],
        money_caps: 'YESTERDAY', money: [['Tickets', eur(l.pax * l.fare * 0.6), 'ok'], ['Subsidy', eur(42000), 'ok'], ['Late running', eur(-3500), 'danger']],
      },
      connections: { title: 'PROTECTED CONNECTIONS', add: 'Add', save: 'Save', note: 'The bus waits for these at most the minutes given.', none: 'No protected connection.', stops: ['Bahnhof', 'Marktplatz', 'Kirche', 'Schule', 'Waldfriedhof'], cols: ['Stop', 'Connects with', 'Time', 'Waits', 'Onward'], what_hint: 'RB 23 to Ruhrau', remove: 'Remove',
        rows: l.name === '101' ? [{ stop: 'Bahnhof', what: 'RB 23 to Ruhrau', time: '07:12', max_hold: 3, onward: false }] : [] },
      title: 'TOURS TODAY', add: 'Add a line', planning: 'Planning', reading: null, none: null,
      rows: TOURS.filter((t) => t.line === sel).map((t) => ({ head: `Tour ${t.tour}`, when: `${hhmm(t.duties[0][0])} – ${hhmm(t.duties[t.duties.length - 1][1])}`, who: t.duties.map((d) => (d[2] === 'you' ? 'You' : d[2] ? firstName(d[2]) : '—')).join(', '), what: t.bus ? `Bus ${BUSES.find((b) => b.id === t.bus).number}` : 'No bus', tone: t.bus ? 'ok' : 'warn' })),
    };
  } else {
    right.add = {
      title: 'LINES TO ADD', make: 'Make a new line', make_tip: 'Draw a line of your own in the line editor', reading: null, error: null,
      switch: { mine: !!a.mine, labels: ["The map's lines", 'My own lines'], none: false, none_tip: '' }, all_run: null,
      rows: (a.mine ? [] : [
        { name: '1', plate: { number: '1', colour: '' }, caption: 'Bahnhof – Endstation', sub: '2 tours  ·  6.4 km  ·  Mon–Fri', icon: 'add', label: 'Take it on', tip: 'Run the line from tomorrow' },
        { name: '2', plate: { number: '2', colour: '' }, caption: 'Endstation – Bahnhof', sub: '1 tour  ·  6.4 km  ·  Mon–Fri', icon: 'add', label: 'Take it on', tip: 'Run the line from tomorrow' },
        { name: 'E', plate: { number: 'E', colour: '' }, caption: 'Bahnhof – Schule', sub: '1 tour  ·  school days', icon: 'gavel', label: 'Apply', tip: 'Its concession is tendered' },
      ]),
      hidden: '1 line is hidden: it needs a bus type the company does not work with.',
    };
  }
  return { ours: { title: 'YOUR LINES', empty: null, rows }, selected: sel, right, foot: 'A line runs as its timetable says; its tours need a bus and drivers in the planning.' };
}

function addLine(a) {
  return {
    title: `Take line ${a.name} on`, intro: 'What the line needs on its busiest day, against what the company has.', needs_caps: 'WHAT', need_caps: 'NEEDS', have_caps: 'HAS',
    rows: [['Buses (solo)', '2', '2', true], ['Drivers', '4', '5', true], ['Licence D', '4', '4', true]], short: null, fine: 'The company can run it from tomorrow.', asking: null,
    dealer: null, hire: null, cancel: 'Cancel', main: 'Take it on', main_icon: 'add',
  };
}

function fare(a) {
  const l = lineOf(a.line) || LINES[0];
  const f = a.fare != null ? Number(a.fare) : l.fare;
  const lo = 100;
  const hi = 600;
  const points = Array.from({ length: 21 }, (_, k) => {
    const p = lo + ((hi - lo) * k) / 20;
    const pax = l.pax * Math.exp(-(p - 150) / 380);
    return [p, Math.round(pax), Math.round((pax * p) / 100)];
  });
  const at = points.reduce((b, p) => (Math.abs(p[0] - f) < Math.abs(b[0] - f) ? p : b));
  const best = points.reduce((b, p) => (p[2] > b[2] ? p : b))[0];
  return {
    title: `Fare of line ${l.number}`, band: 'Fares on lines like this one lie between €1.00 and €6.00.', slider: 'Single ticket', fare: f, fare_text: fine(f), lo, hi, p0: 250, best, p0_label: 'The usual fare', best_label: 'Most takings',
    table: [['Day ticket', fine(f * 2.4)], ['Month', eur(f * 30)]], cap: null,
    chart: { points, pax: at[1], rev: at[2], pax_label: 'Passengers a day', rev_label: 'Takings a day', scale: [4, 3, 2, 1, 0].map((k) => [grouped((l.pax * 1.5 * k) / 4), eur((l.pax * 3 * k * 100) / 4)]), p0_mark: 'Usual', best_mark: 'Most takings', lo_text: fine(lo), hi_text: fine(hi) },
    says: `At ${fine(f)} about ${grouped(at[1])} passengers a day ride, bringing ${eur(at[2] * 100)}.`, warn: f > 450 ? 'Above €4.50 the authority may object.' : null, now: `Now: ${fine(l.fare)}`,
    cancel: 'Cancel', set: 'Set the fare',
  };
}

// - the money -

function money(a) {
  const c = co();
  const [res, tone] = signed(monthResult());
  const income = [['Tickets', 1620000], ['Subsidies', 840000], ['Advertising', 72000], ['Contracts', 25000]];
  const costs = [['Wages', -1140000], ['Diesel', -486000], ['Repairs', -112000], ['Leasing', -96000], ['Interest', -18000], ['Late running', -9000]];
  const inSum = income.reduce((s, x) => s + x[1], 0);
  const outSum = costs.reduce((s, x) => s + x[1], 0);
  const waterfall = [{ label: 'Income', from: 0, to: inSum, amount: eur(inSum), tone: 'ok', total: true }];
  let run = inSum;
  for (const [label, v] of costs) {
    waterfall.push({ label, from: run, to: run + v, amount: eur(v), tone: 'danger' });
    run += v;
  }
  waterfall.push({ label: 'Result', from: 0, to: run, amount: eur(run), tone: run >= 0 ? 'ok' : 'danger', total: true });
  const bars = (list, sum, t) => list.map(([label, v]) => ({ label, amount: eur(v), frac: Math.abs(v) / Math.max(...list.map((x) => Math.abs(x[1]))), share: `${Math.round((v / sum) * 100)} %`, tone: t }));
  const ledger = [
    ['2026-10-19', 'Repair', 'Bus 2571: towed to the workshop', -32000, 'costs'],
    ['2026-10-18', 'Day', 'Tickets of the day', 182400, 'income'],
    ['2026-10-18', 'Wages', 'Wages of the week', -264000, 'costs'],
    ['2026-10-17', 'Advertising', 'Stadtwerke Grundorf: bus 17', 18000, 'income'],
    ['2026-10-15', 'Bus', 'MAN NL202 ordered: deposit', -2400000, 'invest'],
    ['2026-10-12', 'Loan', 'Sparkasse Grundorf: repayment', -150000, 'costs'],
  ].map(([d, kind, text, v, cat], k) => ({ date: shortDate(d), kind, text, amount: signed(v)[0], tone: signed(v)[1], cat, mark: k === 0 ? ['Breakdown', 'warn', 'A breakdown of the morning'] : null }));
  return {
    easy: false,
    figures: [
      { label: 'Cash', value: eur(c.cash), under: 'Now', tone: 'ok', icon: 'payments' },
      { label: 'This month', value: res, under: 'Since 1 Oct', tone, icon: 'trending_up' },
      { label: 'Gross margin', value: '18 %', under: 'Income less running costs', tone: 'text', icon: 'percent' },
      { label: 'Debt', value: '€60,000', under: '1 loan', tone: 'warn', icon: 'account_balance' },
      { label: 'Fleet value', value: '€412,000', under: '4 buses', tone: 'text', icon: 'directions_bus', open: 'fleet_value' },
    ],
    month: {
      pl_title: 'PROFIT AND LOSS', pl_sub: MONL[parse(c.date).getUTCMonth()] + ' ' + parse(c.date).getUTCFullYear(), months: [['', 'This month'], ['2026-09', 'September 2026'], ['2026-08', 'August 2026']], month: a.month || '',
      empty: null, waterfall, income_title: 'INCOME', income_total: eur(inSum), income: bars(income, inSum, 'ok'), no_income: 'No income yet.', costs_title: 'COSTS', costs_total: eur(outSum), costs: bars(costs, outSum, 'danger'), no_costs: 'No costs yet.',
      result_label: 'Result', result: eur(run), result_tone: run >= 0 ? 'ok' : 'danger', pax: '62,410 passengers this month', capital: 'Invested: €24,000 (a bus on order)', late: 'Deductions for late running: €90',
    },
    loans: {
      title: 'LOANS', take: 'Take a loan', contract: 'Contract', repay: 'Repay', none: 'No loans.',
      credit: { caps: 'CREDIT ROOM', value: '€84,000', tone: 'ok', bar: { frac: 0.42, tip: '€60,000 of €144,000 used', label: '€60,000 used', right: '€144,000' }, say: 'The bank lends on the fleet and the takings.', rate: 'Interest 4.9 % a year', lines: [['Fleet value', '€412,000'], ['Takings a month', '€25,570'], ['Rating', 'B+']] },
      list: [{ id: 1, what: 'Sparkasse Grundorf · €60,000', sub: '4.9 % · €1,500 a month · until Mar 2030', paper: true }],
    },
    fares: { title: 'TICKET PRICES', tip: 'Change the fare', empty: null, rows: LINES.map((l) => ({ name: l.name, plate: plate(l), fare: fine(l.fare), vs: 'usual €2.50', pax: `${grouped(l.pax)} a day`, tone: 'ok' })) },
    ledger: { title: 'LEDGER', filters: [['all', 'All'], ['income', 'Income'], ['costs', 'Costs'], ['invest', 'Investments']], search: 'Search', heads: ['Date', 'Kind', 'What', '', 'Amount'], none: 'Nothing found.', rows: ledger },
  };
}

function fleetValue() {
  return {
    title: "The fleet's value", head: ['Age', 'Buses', 'New', 'Now', 'Lost'],
    rows: [{ band: 'Under 10 years', buses: '1', new: '€320,000', now: '€236,000', lost: '€84,000' }, { band: '10 to 25 years', buses: '2', new: '€540,000', now: '€148,000', lost: '€392,000' }, { band: 'Over 25 years', buses: '1', new: '€260,000', now: '€28,000', lost: '€232,000', tip: 'MAN SD202 (1986)' }],
    total: ['All', '4', '€1,120,000', '€412,000', '€708,000'], note: 'A bus loses value with its age and its kilometres.', other: null, close: 'Close',
  };
}

// - the inbox and the agenda -

function inbox(a) {
  const filter = a.filter || 'all';
  const kinds = { all: null, events: 'Event', goals: 'Goal', fleet: 'Fleet', other: 'Press' };
  const list = MESSAGES.filter((m) => !m.deleted && (!kinds[filter] || m.kind === kinds[filter]));
  const card = (m) => ({ ...m, read: st.read.has(m.id), date: shortDate(m.date), leave: null, bar: null });
  const open = MESSAGES.find((m) => m.id === Number(a.open) && !m.deleted);
  return {
    filters: [['all', `All (${MESSAGES.filter((m) => !m.deleted).length})`], ['events', 'Events'], ['goals', 'Goals'], ['fleet', 'Fleet'], ['other', 'Other']], filter,
    mark_all: 'Mark all read', delete_read: 'Delete the read', delete_read_tip: 'Delete every read message', empty: 'No messages here.',
    list: list.map(card), trophies_title: 'TROPHIES', trophies: { frac: 0.7, label: `${co().trophies} of 20 to the next milestone`, tip: 'Trophies are earned with goals' },
    message_title: 'MESSAGE', pick: 'Choose a message on the left.', today: 'Today', unread_label: 'Mark unread', delete: 'Delete',
    open: open ? card(open) : null,
  };
}

function agenda(a) {
  const all = agendaItems();
  const groups = [['planning', 'Planning', 'calendar_month'], ['fleet', 'Fleet', 'directions_bus'], ['staff', 'Staff', 'groups'], ['events', 'Events', 'event'], ['tenders', 'Tenders', 'gavel']].map(([key, label, icon]) => ({ key, label, icon, n: all.filter((x) => x.group === key).length, on: !a.groups || a.groups.includes(key) }));
  const shown = all.filter((x) => groups.find((g) => g.key === x.group)?.on && (!a.mine || x.needs));
  const d = co().date;
  const out = {
    title: 'Agenda', sub: 'Everything coming in the next weeks', mode: a.mode === 'month' ? 1 : 0, modes: ['List', 'Month'], groups,
    mine: { on: !!a.mine, n: all.filter((x) => x.needs).length, text: `${all.filter((x) => x.needs).length} need you`, label: 'Only what needs me', tip: 'Show only what you must decide' },
  };
  if (a.mode === 'month') {
    const t = parse(d);
    const first = new Date(Date.UTC(t.getUTCFullYear(), t.getUTCMonth(), 1, 12));
    const start = new Date(first.getTime() - ((first.getUTCDay() + 6) % 7) * 86400000);
    const sel = a.day || d;
    const weeks = Array.from({ length: 6 }, (_, w) => Array.from({ length: 7 }, (_, k) => {
      const date = fmt(new Date(start.getTime() + (w * 7 + k) * 86400000));
      const items = shown.filter((x) => x.date === date);
      return { date, n: parse(date).getUTCDate(), in: parse(date).getUTCMonth() === t.getUTCMonth(), today: date === d, past: date < d, sel: date === sel, needs: items.some((x) => x.needs), marks: items.map((x) => ({ group: x.group, tone: x.tone, n: 1 })) };
    }));
    out.month = { label: `${MONL[t.getUTCMonth()]} ${t.getUTCFullYear()}`, prev: null, next: null, prev_label: 'The month before', next_label: 'The month after', this: true, this_key: null, this_label: 'This month', weekdays: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'], weeks,
      day: { label: dayLabel(sel).toUpperCase(), items: shown.filter((x) => x.date === sel), empty: 'Nothing this day.' } };
  } else {
    const dates = [...new Set(shown.map((x) => x.date))];
    out.list = { range: `${shortDate(d)} – ${shortDate(addDays(d, 28))}`, count: `${shown.length} entries`, empty: 'Nothing coming up.',
      days: dates.map((date) => ({ label: date === d ? 'Today' : date === addDays(d, 1) ? 'Tomorrow' : dayLabel(date), sub: dayLabel(date), today: date === d, past: false, ops: date === d ? { tab: 15, label: 'Service control' } : null, items: shown.filter((x) => x.date === date) })) };
  }
  return out;
}

// - the duty to drive -

function launch() {
  const c = co();
  const l = lineOf('205');
  return {
    mark: markOf(c), caps: 'A DUTY OF THE COMPANY', name: c.name, title: 'Line 205, tour 1, duty 1', duty: st.launch?.duty || 'grundorf-bus|2026-10-19|205|1|0',
    print: 'PRINT THE DUTY CARD', to: 'OPEN IN THE PLANNING', back: 'BACK', reading: null,
    tiles: [['Day', dayLabel(c.date)], ['Sign on', '06:15'], ['Sign off', '10:05'], ['Length', '3 h 50'], ['Bus', `Bus 31  ·  MAN NG272`], ['Trips', '4']],
    duty_caps: 'THE DUTY', no_rows: 'No rows.', colour: l.colour,
    rows: [
      { kind: 'on', time: '06:15', title: 'Sign on', at: 'Betriebshof Grundorf' },
      { kind: 'trip', empty: true, dep: '06:20', arr: '06:30', deadhead: 'To the first stop', route: 'Betriebshof – Grundorf Bahnhof' },
      { kind: 'trip', dep: '06:30', arr: '07:05', line: '205', route: 'Grundorf Bahnhof → Ruhrau Markt', length: '18.4 km · 35 min' },
      { kind: 'break', text: '15 min break at Ruhrau Markt' },
      { kind: 'trip', dep: '07:20', arr: '07:58', line: '205', route: 'Ruhrau Markt → Grundorf Bahnhof', length: '18.4 km · 38 min' },
      { kind: 'trip', dep: '08:10', arr: '08:45', line: '205', route: 'Grundorf Bahnhof → Ruhrau Markt', length: '18.4 km · 35 min' },
      { kind: 'trip', dep: '09:10', arr: '09:50', line: '205', route: 'Ruhrau Markt → Grundorf Bahnhof', length: '18.4 km · 40 min' },
      { kind: 'off', time: '10:05', title: 'Sign off', at: 'Betriebshof Grundorf' },
    ],
    map_caps: 'THE ROUTES', map: MAP, date: c.date, trips: ['205 Grundorf-Ruhrau', '205 Ruhrau-Grundorf'],
    bus_caps: 'THE BUS', bus: [['Bus', '31  ·  MAN NG272'], ['Livery', 'Weiss'], ['Plate', 'GD-VB 31'], ['Depot', 'Grundorf.hof']], bus_note: 'Fixed by the planning',
    start: {
      blocked: false, caps: 'START', weather_label: 'Weather', weather: st.weather, weathers: [['clear', 'Clear'], ['cloudy', 'Cloudy'], ['rain', 'Rain'], ['snow', 'Snow'], ['fog', 'Fog']],
      when: `The game starts at 06:15 on ${dayLabel(c.date)}`, fixed: 'The map, the bus and the time are fixed by the duty', go: 'START THE DUTY',
      // (shared, Jan and Piet drive at the same time: alone or together - `launch::together_card`)
      together: {
        alone: 'Drive alone', with: 'Drive together', along: "At the same time: Jan, Piet  ·  The company's world is open (Jan), its clock at 06:02: Jan drive in it.",
        alone_tip: 'Single player: the duty in a game of your own, at your own pace.',
        with_tip: "Together: in the company's world, on its clock. You can join it from 20 min before your duty's start until 5 min after it; the world closes with the game of whoever opened it.",
      },
    },
  };
}

// - the company's settings and the clock's dialog -

function settings(a) {
  const c = co();
  const modes = ['casual', 'normal', 'realistic'].map(modeJson);
  return {
    title: 'Company settings', mark_caps: 'THE MARK', mark: markOf(c), logo_text: 'No logo picture: the short name is the mark.', choose_logo: 'Choose a logo picture', remove_logo: null,
    country: wizard({}).country, mode_caps: 'GAME MODE', modes, mode: c.mode, mode_info: 'Switch at any time: nothing is lost.',
    company_mode: null,
    addons: c.mode === 'casual' ? null : { caps: 'ADD-ONS', on: rosterflowOn(), label: 'RosterFlow, the professional scheduling studio', note: 'The Planning page becomes RosterFlow: duties, blocks, the timetable and the drivers\' days.' },
    buying: { caps: 'BUYING BUSES', mode: 1, modes: ['Simple', 'By contract'], say: 'Haggle with the dealer, agree on extras and sign a contract; new buses are delivered.' },
    currency: { caps: 'CURRENCY', value: 0, codes: ['EUR', 'GBP', 'PLN'], options: ['Euro (€)', 'Pound (£)', 'Złoty (zł)'], say: 'Money is shown in this currency.' },
    vehicles: null,
    friends: { caps: 'MULTIPLAYER', on: st.friendsJoin, label: 'Friends can join without an invite', say: st.friendsJoin ? 'While you share the company, your Omsi-Hub friends see a Join button in their friends list: no session code needed.' : 'Off: friends join when you invite them from your friends list, or with the session code.' },
    board: { caps: 'ONLINE LEADERBOARD', on: st.board, disabled: false, label: 'Show this company on the online leaderboard', say: st.board ? 'Its name, colour, map and figures (this month\'s profit and passengers, cash, buses, lines, staff, trips on time and reputation) are on omsihub.com, with your display name.' : 'Off: nothing of this company leaves this PC. At most 5 of your companies can be on the board.', url: 'https://omsihub.com/leaderboard.html', open: 'Open the leaderboard' },
    date_caps: "THE COMPANY'S DATE", now: `${dayLabel(c.date)}  ·  ${hm()}`,
    realtime: { on: st.realtime, label: 'In step with the real clock', say: "The company's time goes on while Omsi-Hub is closed." },
    date: a.date || c.date, earliest: null, warn: 'Moving the date does not simulate the days between.',
    delete: 'Delete the company', close: 'Close', move: a.date && a.date !== c.date ? `Move to ${dayLabel(a.date)}` : null,
  };
}

function time() {
  return {
    title: "The company's clock", in_game: null, simulate: 'SIMULATE',
    steps: [['+15 min', 'minutes', 15], ['+1 h', 'minutes', 60], ['+3 h', 'minutes', 180], ['To the morning', 'morning'], ['To midnight', 'midnight'], ['+1 day', 'days', 1], ['+7 days', 'days', 7]].map(([label, step, n]) => ({ label, step, n, tip: `Simulate: ${label}` })),
    until_label: 'Until', until: st.until, until_button: 'Simulate until then',
    run: 'RUN THE CLOCK', speeds: [['Stop', 0], ['×60', 1], ['×600', 10], ['×3600', 60]], speed: st.speed, speed_tip: 'Stop the clock', say: 'The clock runs while this page is open.',
    real: st.realtime, real_words: st.realtime ? 'In step with the real clock.' : 'The real clock is off: the time stands still while the company is closed.',
    decisions: 'DECISIONS', dispatcher: st.dispatcher, dispatcher_label: 'The dispatcher decides breakdowns by himself',
    quick: 'Close the day quickly', quick_tip: 'Simulate to midnight without stopping for anything', close: 'Close',
  };
}

function confirm(a) {
  const what = a.what;
  const t = {
    sell: ['sell', 'Sell the bus?', 'The bus is sold to the dealer at its value now.', 'Sell', true],
    dismiss: ['logout', 'Dismiss?', 'They leave at the end of the month; the notice is paid.', 'Dismiss', true],
    remove_line: ['close', `Give line ${a.name} up?`, 'Its tours stop from tomorrow; its concession is returned.', 'Give it up', true],
    delete_company: ['delete', `Delete ${co()?.name || 'the company'}?`, 'The company is deleted for good: its buses, staff, lines, money and history. This cannot be undone.', 'Delete for good', true],
    game_mode: ['tune', `Switch to ${modeJson(a.name || 'normal').label}?`, 'The company keeps everything it has; the pages change with the mode.', 'Switch', false],
    holiday: ['beach_access', 'Approve over the limit?', 'More drivers would be off at once than the limit allows.', 'Approve', false],
  }[what] || ['help', 'Are you sure?', 'This cannot be undone.', 'Yes', false];
  return { icon: t[0], title: t[1], text: t[2], button: t[3], danger: t[4], cancel: 'Cancel' };
}

function report() {
  const c = co();
  return {
    title: `The report of ${dayLabel(addDays(c.date, -1))}`, close: 'Close',
    figures: [{ label: 'Result', value: signed(lastDays()[29])[0], tone: signed(lastDays()[29])[1] }, { label: 'Passengers', value: '3,695', tone: 'text' }, { label: 'On time', value: '94 %', tone: 'ok' }, { label: 'Trips driven', value: '212 of 214', tone: 'text' }],
    money_caps: 'THE MONEY', money: [['Tickets', '+€5,420', 'ok'], ['Subsidies', '+€2,800', 'ok'], ['Wages', '-€3,800', 'danger'], ['Diesel', '-€1,620', 'danger']],
    lines_caps: 'THE LINES', lines_hint: 'A click: its money', lines: LINES.map((l) => ({ line: l.name, plate: plate(l), text: `${l.caption}  ·  ${grouped(l.pax)} passengers`, net: '+€640', tone: 'ok', open: st.reportLine === l.name, rows: [['Tickets', eur(l.pax * l.fare * 0.6), 'ok'], ['Wages', '-€1,210', 'danger'], ['Diesel', '-€540', 'danger']], note: 'Its share of the day.' })),
    drops: [], drops_caps: 'TRIPS DROPPED', drops_hint: '', drops_more: null, late: null,
    facts_caps: 'WHAT HAPPENED', facts: [{ tone: 'warn', text: 'Line 102, tour 2 ran without a driver from 14:00: 6 trips dropped.', fixes: [{ icon: 'event', label: 'To the planning', to: 'planning' }] }],
  };
}

// - the career (its first part: the driver's duties) -

function career() {
  const c = co();
  const week = Array.from({ length: 7 }, (_, k) => {
    const date = addDays(c.date, k);
    const t = parse(date);
    return { day: WD[t.getUTCDay()], date: shortDate(date), today: k === 0, weekend: t.getUTCDay() % 6 === 0, duties: k % 3 === 2 ? [] : [{ plate: plate(lineOf('205')), time: '06:30', tip: 'Line 205, tour 1' }] };
  });
  return {
    parts: ['My duties', 'Progress', 'Training', 'Workshop', 'Rankings', 'Statistics'], part: 0,
    level: 'Company level 3',
    profile: { initials: 'LR', name: 'Luc Ruigrok', level: 'Level 4', rank: 'Route driver', company: c.name, bar: { frac: 0.71, tip: '2,140 of 3,000 points', label: '860 points to level 5' },
      stats: [{ icon: 'route', value: '128', label: 'Trips' }, { icon: 'schedule', value: '94 %', label: 'On time' }, { icon: 'speed', value: '2,410 km', label: 'Driven' }, { icon: 'star', value: '4.6', label: 'Rating' }] },
    body: {
      easy: false, week_title: 'THE COMING WEEK', week, title: 'MY DUTIES', up: 'Next', print: 'Print the duty card', drive: 'Drive', drive_tip: 'Drive this duty in OMSI 2', back_tip: 'Give it back', empty: null,
      rows: [0, 1, 3].map((k) => ({ duty: `grundorf-bus|${addDays(c.date, k)}|205|1|0`, date: dayLabel(addDays(c.date, k)), time: '06:30 – 09:50', plate: plate(lineOf('205')), words: 'Line 205, tour 1  ·  Grundorf – Ruhrau', bus: 'Bus 31  ·  MAN NG272', bus_tone: 'soft' })),
      side: { title: 'HOW IT WORKS', steps: [{ title: 'Plan yourself in', text: 'Give yourself a duty in the planning, as any driver.' }, { title: 'Drive it', text: 'Drive opens OMSI 2 on the duty: its bus, its time, its trips.' }, { title: 'It counts', text: 'What you drove counts for the company as it was driven.' }] },
    },
  };
}

// --- the more pages (`co_more_view`) -----------------------------------------------------------

/** The longest duty the player set (`planning::tools`). */
let longest = 450;

function planning(a) {
  if (rosterflowOn()) return { rosterflow: true };
  const c = co();
  const day = Math.min(Number(a.day) || 0, 6);
  const date = addDays(c.date, day);
  const tabs = Array.from({ length: 7 }, (_, k) => {
    const d = addDays(c.date, k);
    const t = parse(d);
    const weekend = t.getUTCDay() % 6 === 0;
    return { caps: (k === 0 ? 'Company day' : WDL[t.getUTCDay()]).toUpperCase(), big: `${WD[t.getUTCDay()]} ${t.getUTCDate()} ${MON[t.getUTCMonth()]}`, on: k === day, dot: weekend ? 'faint' : k < 2 ? 'warn' : 'ok', tip: weekend ? 'No tours this day' : k < 2 ? '2 open' : 'Every tour planned' };
  });
  const sel = a.sel || null;
  const groups = LINES.map((l) => {
    const tours = TOURS.map((t, ti) => ({ t, ti })).filter(({ t }) => t.line === l.name);
    const open = tours.reduce((n, { t }) => n + t.duties.filter((d) => d[2] == null).length, 0);
    const noBus = tours.filter(({ t }) => !t.bus).length;
    const tags = [];
    if (noBus) tags.push({ text: `${noBus} without a bus`, icon: 'directions_bus' });
    if (open) tags.push({ text: `${open} without a driver`, icon: 'person' });
    return {
      name: l.name, line: { plate: plate(l), colour: l.colour, caption: l.caption }, folded: false, none: false, idle: false, count: tours.length, tags, tag_tone: 'warn',
      fold_tip: 'Collapse the line to its heading', tt: { label: 'Timetable', tip: "The line's trips and timetable: see them and change them here" },
      service: { kind: 'in', text: `In service  ·  ${tours.length - (open || noBus ? 1 : 0)} of ${tours.length} tours covered`, tone: open || noBus ? 'warn' : 'ok', stop_tip: 'Take the line out of service' },
      tours: tours.map(({ t, ti }) => tourRow(t, ti, sel, l)),
    };
  });
  return {
    day, date, tabs,
    tools: {
      text: `${dayLabel(date)}: the roster of this weekday repeats every week.`, tone: 'text86', icon: 'event',
      fill: { label: 'FILL THE ROSTER', tip: 'Fix in the roster what the dispatcher would take: free buses, and free drivers by the working-time rules. Only what is planned runs.' },
      clear: { label: a.clear_armed ? 'PRESS AGAIN' : 'CLEAR THE DAY', widest: ['CLEAR THE DAY', 'PRESS AGAIN'], armed: !!a.clear_armed, tip: 'Take every bus and driver of this weekday out of the roster' },
      repeat: { label: 'REPEAT MON–FRI', tip: "This weekday's roster on the other days too (theirs is replaced)" },
      longest: { label: 'LONGEST DUTY', value: longest, choices: Array.from({ length: 17 }, (_, k) => ({ value: 120 + k * 30, label: `${Math.floor((120 + k * 30) / 60)}:${String((120 + k * 30) % 60).padStart(2, '0')}` })), tip: 'A driver works at most this long a day: longer tours are split into duties no longer than this, and the dispatcher gives nobody more.' },
    },
    gantt: { t0: 5 * 60, t1: 23 * 60, head: { fold: { label: 'COLLAPSE ALL', icon: 'unfold_less', tip: 'Fold every line to its heading: what it has planned, and what is open', all: false } }, groups, names: LINES.map((l) => l.name) },
    side: sidePanel(sel, a.arm, day === 0),
  };
}

const whoName = (who) => (who === 'you' ? 'You' : who ? staffOf(who).name : 'Nobody');

/** A duty or a tour chosen (`duty_panel`, `tour_panel`), else the day (`day_panel`). */
function sidePanel(sel, arm, today) {
  const ti = sel ? TOURS.findIndex((t) => t.line === sel.line && t.tour === sel.tour) : -1;
  if (ti < 0) return dayPanel(arm);
  const t = TOURS[ti];
  const l = lineOf(t.line);
  const tt = { label: "The line's trips and timetable", tip: "See the line's departures and the tours they make, and change them" };
  const bus = BUSES.find((b) => b.id === t.bus);
  if (sel.k != null && t.duties[sel.k]) {
    const [from, to, who] = t.duties[sel.k];
    return {
      kind: 'duty', title: 'DUTY', close: 'Close', ti, k: sel.k, line: t.line, tour: t.tour, colour: l.colour, head: `Line ${t.line}, tour ${t.tour}, duty ${sel.k + 1}`, tt, by_player: false,
      facts: [{ text: `${hhmm(from)} – ${hhmm(to)}  ·  ${length(to - from)}  ·  ${Math.round((to - from) / 40)} trips`, tone: 'text86' }, { text: `${l.caption.split(' – ')[0]} → ${l.caption.split(' – ')[1] || l.caption}`, tone: 'dim' }, { text: `Driven by: ${whoName(who)}`, tone: who ? 'text' : 'warn' }],
      options_title: 'WHILE IT IS OPEN TODAY', options: who == null && today ? [{ label: 'An agency driver', note: '€38 an hour', fill: 'agency', key: `${ti}|${sel.k}`, on: false }, { label: 'Leave it to the central', note: '', fill: null, key: `${ti}|${sel.k}`, on: false }] : [],
      cands_title: 'WHO COULD DRIVE IT',
      cands: [{ who: 'player', name: 'You', state: 'Drive it yourself', tone: 'dim', swatch: 'accent', on: who === 'you' }, ...drivers().map((e) => ({ who: `staff:${e.id}`, name: e.name, state: e.id === 2 ? 'On holiday from tomorrow' : 'Free then', tone: e.id === 2 ? 'warn' : 'ok', swatch: e.colour, on: who === e.id })), { who: null, name: 'Nobody', state: '', tone: 'dim', swatch: null, on: false }],
      drive: today ? 'DRIVE THIS DUTY' : null,
      // (split in two at its middle, as often as wanted: `planning::duty_panel`)
      split: to - from >= 120 ? { label: 'Split this duty', note: `At ${hhmm(Math.round((from + to) / 2 / 10) * 10)}, ${l.caption.split(' – ')[0]}`, tip: 'A new duty begins at the relief point nearest its middle: each half can be split again.' } : null,
    };
  }
  return {
    kind: 'tour', title: 'TOUR', close: 'Close', ti, line: t.line, tour: t.tour, colour: l.colour, head: `Line ${t.line}, tour ${t.tour}`, tt,
    facts: [{ text: `${hhmm(t.duties[0][0])} – ${hhmm(t.duties[t.duties.length - 1][1])}  ·  ${Math.round(l.km * 8)} km  ·  ${t.duties.length} duties`, tone: 'text86' }, { text: `Bus: ${bus ? `${bus.number} ${bus.name}` : 'none'}`, tone: bus ? 'text' : 'warn' }],
    options_title: 'WHILE IT HAS NO BUS TODAY', options: !bus && today ? [{ label: 'A rental bus', note: '€310 a day', fill: 'rental', key: `${ti}`, on: false, live: true }, { label: 'Drop its trips', note: '', fill: 'drop', key: `${ti}`, on: false, live: true }] : [],
    cands_title: 'WHICH BUS COULD RUN IT',
    cands: [...BUSES.map((b) => ({ bus: b.id, name: `${b.number}  ${b.name}`, state: b.workshop ? 'In the workshop' : TOURS.some((x) => x.bus === b.id && x !== t) ? 'On another tour then' : 'Free', tone: b.workshop ? 'err' : 'ok', on: b.id === t.bus })), { bus: null, name: 'No bus', state: '', tone: 'dim', on: false }],
  };
}

/** The more pages' actions (`co_more_act`): the planning's - a driver or a bus given, the day's
 * tools, a line's sheet opened -, the research's, the workshop's and the sheet's. */
function moreAct(a) {
  if (String(a.act || '').startsWith('pe_')) return planEditAct(a);
  const other = researchAct(a) || workshopAct(a) || depotAct(a);
  if (other) {
    bump();
    return other;
  }
  const t = TOURS[Number(a.ti)];
  switch (a.act) {
    case 'plan_give_driver':
      if (t && t.duties[a.k]) t.duties[a.k][2] = a.who === 'player' ? 'you' : a.who ? Number(String(a.who).split(':')[1]) : null;
      break;
    case 'plan_give_bus':
      if (t) t.bus = a.bus == null ? null : Number(a.bus);
      break;
    case 'plan_split':
      if (t && t.duties[a.k]) {
        const [from, to, who] = t.duties[a.k];
        const at = Math.round((from + to) / 2 / 10) * 10;
        t.duties.splice(a.k, 1, [from, at, who], [at, to, null]);
        say(`Duty cut on ${t.line}/${t.tour} at ${hhmm(at)}`);
      }
      break;
    case 'plan_duty_max':
      longest = Number(a.minutes) || longest;
      break;
    case 'plan_drive':
      st.launch = { duty: `${st.open}|${co().date}|${t?.line}|${t?.tour}|${a.k}` };
      break;
    case 'plan_fill':
      say('The roster is filled: 1 duty and 1 tour stay open (nobody free).');
      break;
    case 'plan_repeat':
      say("This weekday's roster is on the other weekdays too.");
      break;
    case 'plan_clear':
      say('The day is cleared.');
      break;
    case 'plan_timetable':
      // (RosterFlow on: its Timeplan takes the line; else the planning's own sheet)
      if (rosterflowOn()) return { hand_over: `timetable=${a.line}` };
      sheet = lineOf(a.line) ? { line: a.line, tour: a.tour || null, date: a.date || co().date, sel: null } : null;
      return { plan_edit: !!sheet };
    default:
  }
  bump();
  return {};
}

function tourRow(t, ti, sel, l) {
  const bus = BUSES.find((b) => b.id === t.bus);
  return {
    ti, line: t.line, tour: t.tour, label: `Tour ${t.tour}`, colour: l.colour, unplanned: false, on: !!(sel && sel.line === t.line && sel.tour === t.tour && sel.k == null), by_player: false,
    bus: bus ? { kind: 'bus', text: `${bus.number}  ${bus.name}`, suggested: false, problem: null, tip: "The tour's bus: tap a bus in the list, then here - or drag it here. Tap it twice: the line's timetable" } : { kind: 'none', tone: 'warn', icon: 'warning', text: 'No bus', tip: 'No bus free' },
    duties: t.duties.map(([from, to, who], k) => {
      const times = `${hhmm(from)} – ${hhmm(to)}`;
      const out = { k, from, to, times, open: who == null, chosen: !!(sel && sel.line === t.line && sel.tour === t.tour && sel.k === k), faint: bus ? 1 : 0.5, warn: false, tip: `${times}  ·  ${who === 'you' ? 'You' : who ? staffOf(who).name : 'Nobody'}` };
      if (who == null) Object.assign(out, { tone: 'warn', icon: 'warning', words: 'No driver', full: `No driver  ${times}` });
      else if (who === 'you') Object.assign(out, { fill: 'accent', alpha: 1, ink: 'white', label: 'You', short: '', edge: false, shades: [] });
      else Object.assign(out, { fill: staffOf(who).colour, alpha: 1, ink: 'dark', label: firstName(who), short: staffOf(who).name.split(' ').map((w) => w[0]).join(''), edge: false, shades: [[from, from + 10]] });
      return out;
    }),
  };
}

function dayPanel(arm) {
  const armKey = arm ? (arm.driver ? 'd:' + arm.driver : 'b:' + arm.bus) : '';
  return {
    kind: 'day', title: "THE COMPANY'S DAY", summary: '3 of 5 tours covered', ok: false, gaps: '2 duties without a driver, 1 tours without a bus.',
    heads: { morning: 'THIS MORNING', open: 'OPEN DUTIES', drivers: 'DRIVERS', buses: 'BUSES' },
    morning: [{ text: 'Bus 2571 did not start: towed to the workshop (€320).', tone: 'err' }],
    open: [
      { ti: 1, duty: 1, line: '101', tour: '2', title: 'Line 101, tour 2, duty 2  ·  No driver', fill: '→ Trips dropped', tone: 'warn', hard: false },
      { ti: 3, duty: null, line: '102', tour: '2', title: 'Line 102, tour 2  ·  No bus', fill: '→ Trips dropped', tone: 'warn', hard: false },
    ],
    hint: 'Tap one, then a duty - or drag it there; or drag a duty here.',
    me: { who: 'player', name: 'You', state: 'Your own duties', on: armKey === 'd:player' },
    staff: drivers().map((e) => ({ who: `staff:${e.id}`, name: e.name, colour: e.colour, state: e.id === 2 ? '6 h 15' : e.id === 5 ? '7 h 30' : '6 h 20', tone: 'text82', on: armKey === `d:staff:${e.id}` })),
    no_staff: 'Nobody on the payroll yet.',
    buses: BUSES.map((b) => ({ id: b.id, name: `${b.number}  ${b.name}`, state: b.workshop ? 'In the workshop' : `${TOURS.filter((t) => t.bus === b.id).length} tours`, tone: b.workshop ? 'err' : 'text82', on: armKey === `b:${b.id}` })),
    no_buses: 'No bus in the fleet yet.',
  };
}

// - Casual's pages (its home and its buses; the others are left out of the mock) -

function casual(a) {
  if (a.page === 'buses') {
    return {
      page: 'buses', order: null, number_tip: "The bus's fleet number: type your own, or leave it empty for the automatic one.",
      head: { name: 'Buses', sub: `${BUSES.length} buses · repairs and services happen by themselves`, action: ['Buy a bus', 'add'], help: { icon: 'help', title: 'Buses', text: 'Your buses.', unlock: '', go: null }, help_tip: 'What is this page?' },
      cards: BUSES.map((b) => ({
        id: b.id, number: b.number, auto: b.auto ?? b.number, own: b.auto != null, name: b.name, file: b.file, livery: b.livery, age: `${2026 - b.year} years old`,
        cond: b.cond, cond_tone: condTone(b.cond), word: b.cond >= 0.7 ? 'Good condition' : 'Worn: repaired soon', doing: b.workshop ? 'In the workshop: back on Wed' : 'Free today', tone: b.workshop ? 'warn' : 'dim',
      })),
    };
  }
  if ((a.page || 'home') !== 'home') return {};
  const c = co();
  const go = (tab) => ({ kind: 'open', tab });
  return {
    page: 'home', today_caps: 'TODAY', date: `${dayLabel(c.date)}  ·  ${hm()}`, sentence: 'Your two buses run line 3 all day; 410 passengers rode so far.',
    solved_caps: 'SOLVED BY ITSELF', solved: ['Bus 12 went to the workshop at night: back tomorrow.', 'Driver Lisa Brandt was ill: Tom Becker drove her shift.'], calm: 'Nothing to solve today.',
    drive: 'Drive a shift', drive_sub: '3 shifts today',
    tiles: [
      { page: 101, icon: 'directions_bus', name: 'Buses', tone: 'text', big: '2', under: 'Both on the road', action: { icon: 'storefront', label: 'Buy a bus', act: go(107) } },
      { page: 102, icon: 'route', name: 'Lines', tone: 'text', big: '1', under: 'Line 3 · 410 passengers today', action: { icon: 'add_road', label: 'A new line', act: go(108) } },
      { page: 103, icon: 'groups', name: 'Drivers', tone: 'text', big: '3', under: 'All at work', action: { icon: 'person_add', label: 'Hire now', act: { kind: 'hire_now' } } },
      { page: 104, icon: 'payments', name: 'Money', tone: 'ok', big: eur(c.cash), under: 'This week -€380', action: { icon: 'bar_chart', label: 'The weeks', act: go(104) } },
    ],
    coming_caps: 'COMING UP', coming: [{ icon: 'construction', tone: 'accent2', when: 'Tomorrow', text: 'Bus 12 back from the workshop', note: '', go: null }], no_coming: 'Nothing coming up.',
    tips_caps: 'TIPS', tips: [{ id: 'fare', warn: false, text: 'Line 3 is full in the morning: a second bus would earn more.', button: { icon: 'storefront', label: 'Bus shop', act: go(107) } }], no_tips: 'No tips now.', not_now: 'Not now',
    hint: { text: 'Want to plan duties and hire staff yourself? Switch to Normal.', yes: 'Switch to Normal', no: 'Not now' },
  };
}

// - the dealer (the Fleet page's second tab) -

const MODELS = BUSES.slice(0, 3).concat([{ name: 'MAN SD202', file: 'Vehicles/MAN_SD200/SD202.bus' }]).filter((b, k, all) => all.findIndex((x) => x.file === b.file) === k);
function dealer(a) {
  const tab = Math.min(Number(a.tab) || 0, 3);
  let body;
  if (tab === 0) {
    const level = a.level || [];
    const cards = level.length
      ? MODELS.map((m, k) => ({ key: m.file, name: m.name, file: m.file, title: m.name, sub: ['Solo bus · 12 m · diesel', 'Articulated bus · 18 m · diesel', 'Double-decker · 12 m · diesel'][k] || 'Bus', price: ['€238,000', '€312,000', '€296,000'][k] || '€250,000', badge: k === 0 ? ['In stock', 'ok'] : null, to: { bus: m.file } }))
      : [{ key: 'MAN', name: 'MAN', file: MODELS[0].file, title: 'MAN', sub: `${MODELS.length} models`, price: 'from €238,000', badge: null, to: { level: ['MAN'] } }];
    body = {
      kind: 'showroom', searching: false, crumbs: ['Every maker', ...level], count: `${cards.length} ${level.length ? 'models' : 'makers'}`, empty: 'Nothing matches the filters.', cards,
      filters: { search: 'Search a bus', size_label: 'Size', sizes: ['Any', 'Midi', 'Solo', 'Articulated', 'Double-decker'], drive_label: 'Drive', drives: ['Any', 'Diesel', 'Electric', 'Hybrid'], price_label: 'Price at most', any: 'Any price' },
    };
  } else if (tab === 1 || tab === 2) {
    body = {
      kind: 'offers', note: tab === 1 ? "Today's offers: new buses at a lower price, for a day." : 'Used buses from other operators, as they are.', empty: 'No offers today.',
      cards: [{ id: tab * 10 + 1, name: 'MAN NL202', file: MODELS[0].file, line: tab === 1 ? 'New · Grundorf livery' : '2009 · 612,000 km', price: tab === 1 ? '€219,000' : '€48,500', saving: tab === 1 ? 'Save €19,000' : null, condition: tab === 2 ? { frac: 0.58, tone: 'warn', tip: 'Condition 58 %' } : null, who: tab === 1 ? 'MAN Truck & Bus' : 'Stadtwerke Ruhrau', badge: tab === 1 ? 'Today' : null }],
    };
  } else {
    body = { kind: 'orders', on_order_title: 'ON ORDER', none: 'Nothing on order.', on_order: [{ what: 'MAN NL202 · Grundorf livery', no: 'Contract 2026-014', when: 'Delivered on Mon 2 Nov' }], contracts_title: 'CONTRACTS', no_contracts: 'No contracts yet.', open_tip: 'Open the contract', contracts: [{ no: '2026-014', date: '15 Oct 2026', what: 'MAN NL202', seller: 'MAN Truck & Bus', amount: '€238,000' }] };
  }
  return { tabs: ['Showroom', "Today's offers", 'Used buses', 'Orders'], tab, reading: null, body, buying: { label: 'BUYING', tip: 'How the company buys buses', mode: 1, modes: ['Simple', 'By contract'] } };
}

// - service control -

function ops() {
  const c = co();
  const l101 = lineOf('101');
  return {
    title: 'Service control', mode: 'The dispatcher decides by himself; you can step in.', updated: `updated ${hm()}`, alert: 'Tell me from', alert_at: 5,
    clock: { caption: 'Company time', time: hm(), date: dayLabel(c.date) },
    notice: { value: 5, options: [[3, '3 min'], [5, '5 min'], [10, '10 min']] }, bell: { n: 1, tip: '1 thing waits for service control' },
    kpis: { on_time: { label: 'On time now', pct: '91 %', under: '29 of 32 trips', frac: 0.91, tone: 'warn' }, buses: { label: 'Buses late', n: 1, under: 'more than 5 min' }, drivers: { label: 'Drivers missing', n: 0, under: 'now' }, breaks: { label: 'Breakdowns', n: 1, under: 'today' } },
    incidents: { title: 'INCIDENTS', live: 'Live', sub: 'What runs late or broke down, the worst first, with the ways out.', cols: ['Severity', 'Line', 'Bus', 'Delay', 'Cause', 'Ways out'], clear: 'Everything else runs on time', clear_sub: 'The last buses back on time', normal: 'Back on time' },
    buses: [{ kind: 'bus', sev: { key: 'medium', label: 'Medium' }, plate: plate(l101), bus: '18', delay: '+7 min', at: 'Marktplatz', cause: 'Traffic', detail: 'Road works on the Hauptstraße', map: 'bus:2', to: { key: 'staff:2', label: 'Anke Müller' }, min: 7,
      options: [{ icon: 'fast_forward', short: 'Short turn', label: 'Turn short at the Kirche', effect: 'Back on time at the next trip; 2 stops not served', best: true, good: true, args: { bus: 2, way: 'short' } }, { icon: 'hourglass_empty', short: 'Wait', label: 'Let it run late', effect: 'About 7 minutes late until the end', best: false, good: false, args: { bus: 2, way: 'none' } }] }],
    breaks: [{ kind: 'break', sev: { key: 'high', label: 'High' }, plate: null, bus: '2571', delay: '—', at: 'Depot', cause: 'Breakdown', detail: 'Starter motor', done: 'Bus 31 runs in its place', none: null, options: [] }],
    // (a late driver someone covers, and a duty nobody drives: `ops::open_json`)
    drivers: [{ kind: 'driver', sev: { key: 'ok', label: 'Cover sent' }, plate: plate(lineOf('205')), bus: '22', delay: '+30 min', at: 'at Bahnhof', cause: 'Driver late', detail: 'Jonas Weber signs on at 08:10', done: 'Anke Müller drives the first trips until 08:10', options: [] }],
    open: [{ kind: 'open', sev: { key: 'high', label: 'High' }, plate: plate(lineOf('102')), bus: '31', delay: 'from 08:05', at: 'at Bahnhof', cause: 'No driver', detail: 'Ill · 6 trips lost if nobody covers it', map: 'bus:3', nothing: 'Left as it is: 6 trips dropped, €0.00 in penalties, −0.4 reputation.',
      options: [{ icon: 'person', short: 'Tim Becker', label: 'Tim Becker drives it (standby: free all day)', effect: 'Saves 6 trips (−€0.00)', best: true, good: true, args: { line: '102', tour: '3', duty: 0, fill: { kind: 'colleague', id: 7 } } },
        { icon: 'badge', short: 'Agency driver', label: 'The agency sends a driver for it', effect: 'Saves 6 trips (−€0.00); costs €148.00', best: false, good: true, args: { line: '102', tour: '3', duty: 0, fill: { kind: 'agency' } } },
        { icon: 'close', short: 'Drop trips', label: 'Drop its trips still to come today', effect: 'Left as it is: 6 trips dropped', best: false, good: false, args: { line: '102', tour: '3', duty: 0, fill: { kind: 'drop' } } }] }],
    msgs: [],
    normal: [{ plate: plate(lineOf('102')), time: '07:31', age: 11 }, { plate: plate(lineOf('205')), time: '07:20', age: 22 }],
    map: { title: 'LIVE NETWORK MAP', sub: 'Every bus where it is', legend: [['ok', 'On time'], ['warn', 'Late'], ['danger', 'Very late'], ['dim', 'In the depot']], zoom_in: 'Zoom in', zoom_out: 'Zoom out', fit: 'Show all' },
    diversions: { title: 'DIVERSIONS AND ROADWORKS', sub: 'Planned diversions: the lines take the way round on those days.', events: ['Road works on the Hauptstraße: line 101 over the Kirchweg until Fri 23 Oct'], rows: [], none: 'No diversions planned.',
      form: { today: c.date, from: 'From', until: 'Until', closed: 'Stops closed', closed_hint: 'Kirche', via: 'Way round', via_hint: 'Kirchweg', note: 'Note', lines: 'Lines', all: 'all lines', options: LINES.map((l) => [l.name, l.number]), stops: ['Bahnhof', 'Marktplatz', 'Kirche', 'Schule', 'Waldfriedhof'], add: 'Add', remove: 'Remove' } },
    done: [{ time: '06:02', text: 'Bus 31 sent in the place of bus 2571 on line 102.' }], caps: { done: 'DONE TODAY' },
    log: [], log_caps: { title: 'MESSAGES', empty: 'No messages today.', all: 'To every driver', all_to: { key: 'all', label: 'Every driver' }, cols: ['Time', 'From', 'To', 'Message', 'State'] },
    compose: { title: 'A message to the driver', button: 'Message', to: 'To', instruction: 'Instruction', none: 'None', template: 'Template', own: 'Own words', own_ph: 'Up to 240 characters', priority: 'Priority', priorities: [['info', 'Info'], ['urgent', 'Urgent']], cancel: 'Cancel', send: 'Send', templates: [{ key: 'late', label: 'You run %{min} minutes late: drive on calmly.' }, { key: 'call', label: 'Call the dispatcher at the next stop.' }] },
    advice: 'How to run on time', best: "The controller's suggestion",
  };
}

// - the reputation -

function reputation(a) {
  const c = co();
  const tab = a.tab === 'complaints' ? 'complaints' : 'overview';
  const base = { title: 'Reputation', lead: 'What passengers and the authority think of the company.', tab, tabs: [{ key: 'overview', label: 'Overview' }, { key: 'complaints', label: 'Complaints', badge: '1' }] };
  if (tab === 'complaints') {
    const id = Number(a.id) || 1;
    return {
      ...base,
      body: {
        title: 'COMPLAINTS', note: 'Answered well, a complaint costs less reputation.', empty: 'No complaints.',
        rows: [{ id: 1, on: id === 1, open: true, title: 'Missed my train', sub: 'Line 101 · 18 Oct', state: 'Waits for your answer', state_tone: 'warn', severity: 'Medium', severity_tone: 'warn' }, { id: 2, on: id === 2, open: false, title: 'Dirty seats', sub: 'Line 102 · 12 Oct', state: 'Settled', state_tone: 'ok', severity: 'Low', severity_tone: 'dim' }],
        talk: { id, title: id === 1 ? 'Mrs. Hoffmann' : 'Mr. Klein', sub: id === 1 ? 'Line 101 · 18 Oct' : 'Line 102 · 12 Oct', mood_label: 'MOOD', mood: id === 1 ? 0.35 : 0.8, open: id === 1, state: 'Settled: the passenger was content.', state_tone: 'ok',
          talk: [{ text: id === 1 ? 'Your bus left the Bahnhof two minutes early and I missed the 07:12 to Ruhrau!' : 'The seats on line 102 were sticky.', mine: false }, ...(id === 2 ? [{ text: 'We are sorry: the bus is cleaned every night from now on.', mine: true }] : [])],
          choices: [{ key: 'sorry', label: 'Apologise' }, { key: 'voucher', label: 'Send a day ticket' }, { key: 'reject', label: 'Reject it', danger: true }], fix: { text: 'Protect the connection with the RB 23 at the Bahnhof', go: 'Lines', tab: 3 } },
      },
    };
  }
  return {
    ...base,
    body: {
      head: { title: 'REPUTATION', info: 'From 0 to 100: it moves passengers, tenders and the authority.', frac: c.reputation / 100, value: c.reputation.toFixed(1), under: 'of 100', say_tone: 'ok', say_icon: 'trending_up', say: 'Good and rising', by: '+2.1 this week', chart_title: 'THE LAST WEEKS', chart: [['W38', 55], ['W39', 57], ['W40', 58], ['W41', 61], ['W42', 63]], trend: 'Rising for five weeks', trend_tone: 'ok', caps: 'Above 70 the authority prefers your bids.' },
      log_title: 'WHAT MOVED IT', log: [{ date: '16 Oct', text: 'Grundorfer Anzeiger: "Clean buses, friendly drivers"', change: '+1.5', tone: 'ok' }, { date: '14 Oct', text: '6 trips dropped on line 102', change: '-0.8', tone: 'danger' }],
      effects_title: 'WHAT RAISES AND LOWERS IT', effects_info: 'Each week', positive_title: 'RAISES IT', negative_title: 'LOWERS IT',
      positive: [{ icon: 'schedule', name: 'Punctuality', text: '91 % on time', value: '+1.2' }, { icon: 'airline_seat_recline_normal', name: 'Clean buses', text: 'Cleaned every night', value: '+0.6' }],
      negative: [{ icon: 'event_busy', name: 'Trips dropped', text: '6 this week', value: '-0.8' }, { icon: 'directions_bus', name: 'Old buses', text: 'Average age 21 years', value: '-0.3' }],
      drives: 'Punctuality weighs most.', week_title: 'THIS WEEK BY CAUSE', week_any: true, week_none: 'Nothing this week.',
      week: [{ label: 'Punctuality', frac: 0.8, tone: 'ok', value: '+1.2', zero: false }, { label: 'Cleanliness', frac: 0.4, tone: 'ok', value: '+0.6', zero: false }, { label: 'Trips dropped', frac: 0.55, tone: 'danger', value: '-0.8', zero: false }, { label: 'Complaints', frac: 0, tone: 'dim', value: '0', zero: true }],
      tips_title: 'WHAT TO DO', tips_sub: 'The quickest ways up',
      tips: [{ icon: 'event', title: 'Cover every tour', text: 'A dropped trip costs the most.', page: 'Planning', go: 'To the planning', tab: 5 }, { icon: 'chat', title: 'Answer the complaint', text: 'Mrs. Hoffmann waits for an answer.', page: 'Reputation', go: 'Answer', complaints: true }],
    },
  };
}

// - the late running's advice -

const lateAdvice = () => ({ list: [] });

/** The finances' deductions for late running (`late::finance_view`): the days kept, by tier and
 * cause, and each trip with a deduction. */
function late() {
  const c = co();
  const trip = (k, date, line, tour, dep, minutes, amount, cause, bus, driver) => ({
    line: `Linie${line}`, number: line, tour, block: `${line}/${tour}`, block_id: `B|Linie${line}|${tour}`, duty: 0, duty_text: 'Duty 1', dep, arr: dep + 38, time: `${hhmm(dep)}–${hhmm(dep + 38)}`, trip: `${line}_${k}`,
    place: `Line ${line}  ·  block ${line}/${tour}  ·  Duty 1  ·  trip ${line}_${k}`, bus: { id: bus, text: `Bus ${BUSES.find((b) => b.id === bus)?.number || bus}` }, driver: { id: driver, text: staffOf(driver)?.name || 'Agency driver' }, bus_caps: 'Bus', driver_caps: 'Driver',
    dropped: false, minutes, label: `+${minutes}`, tone: minutes >= 15 ? 'danger' : 'warn', head: `${hhmm(dep)} trip ${minutes} min late`, chain: [cause, ...(minutes >= 10 ? ['before that: the layover at the Bahnhof was too short to take it up'] : [])],
    deduction: `${minutes >= 15 ? 40 : 18} % of the trip's value deducted`, lost: minutes >= 15 ? '12 passengers lost' : null, date: dayLabel(addDays(c.date, -date)), amount,
  });
  return {
    title: 'Deductions for late running', span: `The last 7 days closed, from ${dayLabel(addDays(c.date, -7))}`, close: 'Close',
    money: {
      caps: 'Deductions for late running', total: '€214',
      tiers: [{ label: '5 to 9 minutes', trips: '6 trips', amount: '€58', tone: 'warn' }, { label: '10 to 19 minutes', trips: '3 trips', amount: '€96', tone: 'danger' }, { label: '20 minutes and more', trips: '1 trips', amount: '€60', tone: 'danger' }],
      causes: [{ label: 'Traffic', trips: '5 trips', amount: '€112' }, { label: 'The driver', trips: '3 trips', amount: '€64' }, { label: 'The bus', trips: '2 trips', amount: '€38' }],
      carried: '4 of these trips carried a delay on from the trip before: the layover was too short to take it up.',
      how_label: 'How was this worked out?',
      how: ['A trip counts as late from 5 whole minutes behind the timetable, at its start or its end: 4 min 59 s behind is still on time.', 'Each late trip pays a share of its value under the contract: 10 % up to 5 minutes, rising evenly to 100 % at 30 minutes.', 'Casual deducts nothing; school transport counts twice.'],
    },
    trips_caps: 'Each deduction',
    trips: [
      trip(1, 1, '101', '2', 7 * 60 + 12, 22, '€60', 'Traffic in the rush hour on the Hauptstraße: +14 min', 3, 2),
      trip(2, 1, '102', '1', 16 * 60 + 40, 11, '€34', 'The driver took longer at the stops: +6 min', 2, 3),
      trip(3, 3, '205', '1', 8 * 60 + 5, 7, '€18', 'Bus 2571 was slow to start: +7 min', 4, 1),
    ],
  };
}

// --- the more pages' rest (`co_more_view`) ------------------------------------------------------

function concessions() {
  return {
    tenders: {
      title: 'TENDERS', count: 2, note: null, empty: null,
      rows: [
        { id: 1, number: '310', caption: 'Grundorf – Flughafen', on: false, dim: false, badge: { text: 'NEW', tone: 'accent' }, chips: [{ text: '8 years', tone: 'dim', icon: 'schedule' }, { text: '4 buses', tone: 'dim', icon: 'directions_bus' }], state: 'Bids close on 30 Oct', tone: 'text', countdown: ['11 days', 'warn'], bid: 'No bid yet', bid_tone: 'dim' },
        { id: 2, number: 'E', caption: 'Bahnhof – Schule', on: false, dim: false, badge: null, chips: [{ text: '4 years', tone: 'dim', icon: 'schedule' }, { text: '1 bus', tone: 'dim', icon: 'directions_bus' }], state: 'Opens on 2 Nov', tone: 'dim', countdown: null, bid: '', bid_tone: 'dim' },
      ],
    },
    auction: { title: 'THE TENDER', none: 'Choose a tender on the left.' },
    held: {
      title: 'YOUR CONTRACTS', sub: 'Concessions and own lines', empty: null,
      rows: [{ number: '101', caption: 'Bahnhof – Waldfriedhof', rag: { text: 'GREEN', tone: 'ok' }, until: 'Until 31 Dec 2031', tone: 'text', price: '€14,000 a month from the district', points: '0 default points', unpaid: null, change: 'Request a change', line: '101' }, { number: '102', caption: 'Marktplatz – Gewerbegebiet Nord', rag: { text: 'AMBER', tone: 'warn' }, until: 'Until 31 Dec 2029', tone: 'text', price: '€9,500 a month from the district', points: '3 default points', unpaid: null, change: 'Request a change', line: '102' }],
      own: [{ number: '205', caption: 'Grundorf – Ruhrau (Schnellbus)', direct: 'OWN LINE', text: 'Your own line: the takings are yours.' }],
    },
  };
}

function contracts() {
  return {
    figures: { trophies: { label: 'Trophies', value: String(co().trophies), tip: 'Earned with goals' }, rating: { label: 'Rating', value: '4.2', stars: 4.2, tip: "The authority's rating" }, running: { label: 'Goals running', value: '2', under: '2 goals running' }, milestone: { label: 'Next milestone', bar: { frac: 0.7, label: `${co().trophies} of 20`, tip: '20 trophies: a new research' } } },
    title: 'Contracts', lead: 'Goals with a reward, service contracts and the contracts on your buses.', parts: ['Goals', 'Services', 'Bus contracts', 'Bids'], icons: ['flag', 'wifi', 'directions_bus', 'gavel'], part: 0,
    body: {
      kind: 'goals',
      offered: { title: 'OFFERED', until: 'Until Sunday', empty: 'No goals offered.', cards: [{ id: 3, icon: 'groups', tone: 'accent', title: '25,000 passengers this week', rule: 'Every line of the company counts.', trophies: '2', reward: '€1,500', days: '7 days', take: 'Take it', primary: true }] },
      running: { title: 'RUNNING', empty: 'No goal runs.', cards: [{ id: 2, icon: 'schedule', tone: 'ok', title: '95 % on time this week', rule: 'Within two minutes of the timetable', bar: { frac: 0.72, tone: 'ok', label: '91 % of 95 %' }, trophies: '3', reward: '€2,500', when: '5 days left', when_tone: 'soft', give_up: 'Give up' }] },
      finished: { title: 'FINISHED', empty: 'None finished yet.', rows: [{ icon: 'airline_seat_recline_normal', tone: 'ok', title: 'A clean fleet', what: 'Every bus cleaned for a week', done: true, trophies: '1', reward: '€800', ago: '9 days ago' }] },
    },
  };
}

// (the yard laid out, as `hub_core::company::yard` keeps it: its levels, the modules where they
// stand, the land bought; the build mode's actions change them)
// (`?mock&empty=1`: a company founded now, its land only and its building credit)
const EMPTY = new URLSearchParams(location.search).has('empty');
const SITE = EMPTY
  ? { parking: 0, workshop: 0, wash: 0, fuel: 0, offices: 0, charging: 0, land: 0, next: 0, modules: [], upgrades: [], credit: 471000, look: { wall: null, roof: null, trim: null, door: null, finish: 'cladding' } }
  : { parking: 2, workshop: 2, wash: 2, fuel: 4, offices: 2, charging: 2, land: 0, next: 0, modules: [], upgrades: ['gantry', 'carports', 'paint_booth', 'bulk_tank', 'canteen', 'solar'], credit: 0, look: { wall: null, roof: null, trim: null, door: null, finish: 'brick' } };
/** What can be added to each area (`depot::Upgrade`): key, icon, label, what it does, price. */
const UPGRADES = {
  parking: [['lights', 'videocam', 'Lights and cameras', 'The staff feel safer, the buses are daubed on less', '€29,300'], ['carports', 'home', 'Carports', 'The buses stand dry: they stay clean longer', '€94,800'], ['hall', 'garage', 'Parking hall', 'The buses stand indoors: they stay clean much longer', '€474,300']],
  workshop: [['paint_booth', 'livery_brush', 'Paint booth', 'Liveries painted in house: 30 % cheaper', '€158,100'], ['inspection_lane', 'speed', 'Inspection lane', 'Repairs found sooner: 15 % cheaper', '€84,300'], ['tyre_store', 'build', 'Tyre store', 'Repairs and overhauls 7 % cheaper', '€42,200']],
  wash: [['gantry', 'local_car_wash', 'Wash gantries', 'Every lane washes four times the buses a night', '€200,300'], ['interior', 'airline_seat_recline_normal', 'Interior cleaning', 'A wash cleans inside too: it does more', '€47,400'], ['underbody', 'rainy', 'Underbody wash', 'The buses stay clean longer', '€36,900'], ['recycling', 'autorenew', 'Water recycling', 'The wash lanes cost 40 % less to keep', '€63,200']],
  fuel: [['bulk_tank', 'local_gas_station', 'Bulk tank', 'Diesel bought in bulk: 3 % cheaper at the depot', '€73,800'], ['adblue', 'water_drop', 'AdBlue dispenser', 'No detours for AdBlue: 1 % cheaper at the depot', '€12,600']],
  offices: [['canteen', 'group', 'Canteen', 'The staff are more satisfied', '€94,800'], ['control_room', 'monitor', 'Control room', 'The service watched: the passengers notice', '€126,500'], ['training_room', 'school', 'Training room', 'The staff are a little more satisfied', '€63,200']],
  charging: [['solar', 'wb_sunny', 'Solar roof', 'Power for the chargers 8 % cheaper', '€147,600']],
};
const upgradesOf = (a) =>
  (UPGRADES[a] || []).map(([key, icon, label, what, cost]) => {
    const built = SITE.upgrades.includes(key);
    return { key, icon, label, what, built, ...(built ? {} : SITE[a] ? { cost, busy: false, tip: '21 days of work' } : { needs: `Build the ${a} first` }) };
  });
const SHAPES = { parking: { w: 24, h: 10, front: 5, back: 0 }, workshop: { w: 4, h: 14, front: 6, back: 0 }, wash: { w: 6, h: 14, front: 4, back: 3 }, fuel: { w: 10, h: 10, front: 4, back: 2 }, offices: { w: 10, h: 12, front: 2, back: 0 } };
const TITLES = { parking: ['Parking spaces', '12 spaces'], workshop: ['Workshop bays', 'A bay: one bus worked on at once'], wash: ['Washing bay', 'A lane: washes 40 buses a night'], fuel: ['Diesel station', 'Fills 15 diesel buses a night'], offices: ['Offices and rest rooms', 'Room for 20 people'] };
const wantedOf = (lv, a) => (a === 'parking' || a === 'workshop' || a === 'wash' ? lv[a] : lv[a] > 0 ? 1 : 0);
const yardNow = () => ({ cell: 2, cols: 48 + 20 * SITE.land, rows: 64, max_cols: 88, gate: { x: 20, y: 54, w: 8, h: 10 }, shapes: SHAPES, modules: SITE.modules });

function yardOf(lv) {
  for (const a of Object.keys(SHAPES)) {
    while (SITE.modules.filter((m) => m.area === a).length > wantedOf(lv, a)) SITE.modules.splice(SITE.modules.map((m) => m.area).lastIndexOf(a), 1);
  }
  for (const a of ['offices', 'workshop', 'wash', 'fuel', 'parking']) {
    while (SITE.modules.filter((m) => m.area === a).length < wantedOf(lv, a)) {
      const at = spot(yardNow(), a);
      if (!at) {
        if (SITE.land < 2) SITE.land++;
        else break;
        continue;
      }
      SITE.modules.push(moduleAt(yardNow(), ++SITE.next, a, at.x, at.y, at.rot, { title: TITLES[a][0], what: TITLES[a][1] }));
    }
  }
  return { ...yardNow(), land: SITE.land < 2 ? { cost: SITE.land ? '€216,000' : '€162,000', can: true, tip: '40 m more land beside the yard, yours at once' } : null };
}

function depotAct(a) {
  const L = yardNow();
  switch (a.act) {
    case 'depot_build_at': {
      if (whyNot(L, a.area, a.x, a.y, a.rot)) return {};
      SITE[a.area] += 1;
      SITE.modules.push(moduleAt(L, ++SITE.next, a.area, a.x, a.y, a.rot, { title: TITLES[a.area][0], what: TITLES[a.area][1] }));
      return { ok: true };
    }
    case 'depot_move': {
      const m = SITE.modules.find((x) => x.id === a.id);
      if (!m || whyNot(L, m.area, a.x, a.y, a.rot, m.id)) return {};
      Object.assign(m, moduleAt(L, m.id, m.area, a.x, a.y, a.rot, { title: m.title, what: m.what }));
      return { ok: true };
    }
    case 'depot_demolish': {
      const k = SITE.modules.findIndex((x) => x.id === a.id);
      if (k < 0) return {};
      const [m] = SITE.modules.splice(k, 1);
      SITE[m.area] = m.area === 'parking' || m.area === 'workshop' || m.area === 'wash' ? SITE[m.area] - 1 : 0;
      return { ok: true };
    }
    case 'depot_build':
      if (SITE[a.area] !== undefined) SITE[a.area] += 1;
      return {};
    case 'depot_upgrade':
      if (!SITE.upgrades.includes(a.key)) SITE.upgrades.push(a.key);
      return { ok: true };
    case 'depot_look':
      SITE.look = { wall: a.wall || null, roof: a.roof || null, trim: a.trim || null, door: a.door || null, finish: a.finish || 'cladding' };
      return { ok: true };
    case 'depot_land':
      if (SITE.land < 2) SITE.land++;
      return { ok: true };
    default:
      return null;
  }
}

/** The depot as a dashboard (`company_more::depot::dashboard`), from the mock's site. */
function depotDash() {
  const has = (k) => SITE.upgrades.includes(k);
  const load = (need, cap, line, none) => {
    if (!cap) return { frac: need ? 1 : 0, tone: need ? 'danger' : 'dim', line: none };
    const f = need / cap;
    return { frac: Math.min(1, f), tone: f > 1 ? 'danger' : f > 0.85 ? 'warn' : 'ok', line, pct: `${Math.round(f * 100)} %` };
  };
  const fleet = EMPTY ? 0 : 22;
  const washes = SITE.wash * (has('gantry') ? 40 : 10);
  const added = (a) => (UPGRADES[a] || []).filter(([k]) => has(k)).map(([, icon, label]) => ({ icon, label }));
  const area = (key, icon, label, lv, max, meter, works = null) => ({ key, icon, label, lv, max, level: `level ${lv} of ${max}`, meter, added: added(key), works });
  return {
    fleet_title: 'BUSES TONIGHT', people_title: 'STAFF',
    fleet: [{ icon: 'local_parking', label: 'On the yard', value: String(EMPTY ? 0 : 19), tone: 'text' }, { icon: 'construction', label: 'In the workshop', value: String(EMPTY ? 0 : 2), tone: 'text' }, { icon: 'warning', label: 'In the street', value: String(EMPTY ? 0 : 1), tone: EMPTY ? 'text' : 'warn' }],
    people: [{ icon: 'directions_bus', label: 'Drivers', value: EMPTY ? '0' : '14' }, { icon: 'build', label: 'Mechanics', value: EMPTY ? '0' : '3' }, { icon: 'local_car_wash', label: 'Cleaners', value: EMPTY ? '0' : '2' }, { icon: 'groups', label: 'Office', value: EMPTY ? '0' : '3' }],
    money: [{ label: 'Upkeep a month', value: EMPTY ? '€0' : '€6,240' }, { label: 'Cleanliness', value: '84 %', tone: 'ok' }, { label: 'Building credit left', value: `€${SITE.credit.toLocaleString('en')}` }],
    areas: [
      area('parking', 'local_parking', 'Parking spaces', SITE.parking, 8, load(EMPTY ? 0 : 20, SITE.parking * 12, `20 buses for ${SITE.parking * 12} spaces`, 'No spaces: the buses stand in the street')),
      area('workshop', 'construction', 'Workshop bays', SITE.workshop, 6, SITE.workshop ? load(3, SITE.workshop, `2 of ${SITE.workshop} bays busy, 1 jobs waiting`, '') : { frac: 0, tone: 'dim', line: 'None: a garage outside does the work, dearer' }),
      area('wash', 'water_drop', 'Washing bay', SITE.wash, 4, load(fleet, washes, `${fleet} buses, washes ${washes} a night`, 'None: the fleet is cleaned by hand'), EMPTY ? null : 'Interior cleaning ready Mon 2 Nov'),
      area('fuel', 'inventory_2', 'Diesel station', SITE.fuel, 6, load(EMPTY ? 0 : 18, SITE.fuel * 15, `18 diesel buses, fills ${SITE.fuel * 15} a night`, 'None: diesel is bought outside, dearer')),
      area('charging', 'bolt', 'Charging points', SITE.charging, 10, load(EMPTY ? 0 : 4, SITE.charging * 2, `4 electric buses, charges ${SITE.charging * 2} a night`, 'None: electric buses charge outside, dearer')),
      area('offices', 'groups', 'Offices and rest rooms', SITE.offices, 3, load(EMPTY ? 0 : 22, [0, 20, 50, 120][SITE.offices], `22 people, room for ${[0, 20, 50, 120][SITE.offices]}`, 'None: the staff have no rooms')),
    ],
  };
}

function depot() {
  const c = co();
  // (out on their tours by the clock, as `company_more::depot` has it: the 3D yard has them
  // drive out and back)
  const TOURS = { 1: [7 * 60, 13 * 60], 2: [8 * 60, 18 * 60], 3: [6 * 60, 7 * 60 + 50] };
  const out = (b) => !!TOURS[b.id] && c.minute >= TOURS[b.id][0] && c.minute < TOURS[b.id][1];
  const busAt = (b) => ({ id: b.id, number: b.number, size: b.kind.startsWith('Articulated') ? 'articulated' : 'solo', file: b.file, paint: b.livery, tip: `${b.number} ${b.name}  ·  condition ${Math.round(b.cond * 100)} %`, out: out(b) });
  const bld = (head, works = null) => ({ built: true, head, works });
  // (`?mock&buses=120`: a full yard, for the 3D depot's speed)
  const many = Number(new URLSearchParams(location.search).get('buses')) || 0;
  if (many && !depot.inner) {
    depot.inner = true;
    const kinds = [['Vehicles/MAN_NL_NG/NL202.bus', 'Grundorf'], ['Vehicles/MAN_NL_NG/NL202.bus', 'Weiss'], ['Vehicles/MAN_NG/NG272.bus', 'Grundorf'], ['Vehicles/MAN_NL_NG/NL202.bus', 'BVG'], ['Vehicles/MAN_NL_NG/NL202.bus', 'Hamburg']];
    const fleet = Array.from({ length: many }, (_, k) => {
      const [file, paint] = kinds[k % 7 === 0 ? 2 : k % 5 === 0 ? 3 : k % 3 === 0 ? 1 : k % 11 === 0 ? 4 : 0];
      return { id: 1000 + k, number: String(100 + k), size: /NG272/.test(file) ? 'articulated' : 'solo', file, paint, tip: `${100 + k} MAN  ·  ${paint}` };
    });
    if (!depot.full) {
      depot.full = true;
      Object.assign(SITE, { parking: 8, workshop: 6, offices: 3 });
    }
    const v = depot();
    depot.inner = false;
    return { ...v, plan: { ...v.plan, wash: { ...v.plan.wash, gates: true }, fuel: { ...v.plan.fuel, pumps: 3 }, offices: { ...v.plan.offices, floors: 3 }, parking: { ...v.plan.parking, chargers: 24 }, in_ws: fleet.slice(0, 8), parked: fleet.slice(8) } };
  }
  return {
    figures: [
      { label: 'Parking', value: '3 / 12', under: '9 spaces free', tone: 'text' },
      { label: 'Workshop', value: '1 / 2', under: '1 bay free', tone: 'text' },
      { label: 'Cleanliness', value: '84 %', under: 'of the fleet', tone: 'ok' },
      { label: 'Upkeep', value: '€1,850', under: 'a month for the buildings', tone: 'text' },
    ],
    plan: {
      title: 'The depot from above', colour: c.colours[0],
      offices: { ...bld('OFFICES'), built: SITE.offices > 0, floors: SITE.offices, room: 'room for 50' }, hall: { ...bld('WORKSHOP'), built: SITE.workshop > 0, bays: SITE.workshop, max: 6 }, wash: { ...bld('WASHING BAY'), built: SITE.wash > 0, lanes: SITE.wash, gates: SITE.upgrades.includes('gantry') }, fuel: { ...bld('DIESEL STATION'), built: SITE.fuel > 0, pumps: SITE.fuel },
      parking: { label: 'PARKING', works: null, blocks: 8, built: SITE.parking, building: false, per: 12, chargers: SITE.charging * 2 },
      layout: yardOf(SITE),
      in_ws: EMPTY ? [] : BUSES.filter((b) => b.workshop).map(busAt), parked: EMPTY ? [] : BUSES.filter((b) => !b.workshop).map(busAt),
      upgrades: SITE.upgrades, look: SITE.look,
      staff: EMPTY ? { drivers: 0, mechanics: 0, cleaners: 0, office: 0, service: 0, inspectors: 0, researchers: 0 } : { drivers: 14, mechanics: 3, cleaners: 2, office: 3, service: 1, inspectors: 1, researchers: 1 },
    },
    build: {
      title: 'Build',
      rows: [
        { key: 'parking', icon: 'local_parking', label: 'Parking', level: `level ${SITE.parking} of 8`, lv: SITE.parking, max: 8, effect: `${SITE.parking * 12} spaces`, cost: '€70,300', locked: false, place: true, tip: 'Build the next level: 14 days of work, then €350 a month', upgrades: upgradesOf('parking') },
        { key: 'workshop', icon: 'construction', label: 'Workshop', level: `level ${SITE.workshop} of 6`, lv: SITE.workshop, max: 6, effect: `${SITE.workshop} bays`, cost: '€210,800', locked: false, place: true, tip: 'Build the next level: 30 days of work, then €2,110 a month', upgrades: upgradesOf('workshop') },
        { key: 'wash', icon: 'local_car_wash', label: 'Washing bay', level: `level ${SITE.wash} of 4`, lv: SITE.wash, max: 4, effect: SITE.wash ? `${SITE.wash} lanes: washes ${SITE.wash * (SITE.upgrades.includes('gantry') ? 40 : 10)} buses a night` : 'None: the fleet is cleaned by hand', cost: '€94,800', locked: false, place: true, tip: 'Build the next level: 21 days of work, then €470 a month', upgrades: upgradesOf('wash') },
        { key: 'fuel', icon: 'local_gas_station', label: 'Diesel station', level: `level ${SITE.fuel} of 6`, lv: SITE.fuel, max: 6, effect: `Fills ${SITE.fuel * 15} diesel buses a night`, cost: SITE.fuel ? '€31,600' : '€126,500', locked: false, place: !SITE.fuel, tip: 'Build the next level: 7 days of work, then €160 a month', upgrades: upgradesOf('fuel') },
        { key: 'charging', icon: 'bolt', label: 'Charging points', level: `level ${SITE.charging} of 10`, lv: SITE.charging, max: 10, effect: `Charges ${SITE.charging * 2} electric buses a night`, cost: '€52,700', locked: false, place: false, tip: 'Build the next level: 10 days of work, then €260 a month', upgrades: upgradesOf('charging') },
        { key: 'offices', icon: 'groups', label: 'Offices and rest rooms', level: `level ${SITE.offices} of 3`, lv: SITE.offices, max: 3, effect: `Room for ${[0, 20, 50, 120][SITE.offices]} people`, cost: '€232,000', locked: false, place: !SITE.offices, tip: 'Build the next level: 45 days of work, then €1,160 a month', upgrades: upgradesOf('offices') },
      ],
      credit: SITE.credit ? { amount: `€${SITE.credit.toLocaleString('en')}`, tip: 'Building credit: what your company was given to build its first depot with, spent before your cash' } : null,
      hint: EMPTY && !SITE.modules.length ? 'Your land is empty: build parking spaces, a workshop, a diesel station and offices to start with.' : null,
    },
    dashboard: depotDash(),
    workshop: {
      title: 'Workshop', empty: null, cancel_tip: 'Take the job back',
      jobs: [{ id: 1, number: '2571', title: 'Repair  ·  MAN SD202', what: 'Starter motor · until Wed 21 Oct', tone: 'warn', cost: '€640', cancel: false }],
      buses: BUSES.filter((b) => !b.workshop).map((b) => ({ id: b.id, number: b.number, name: b.name, sub: `${grouped(b.km)} km`, tone: b.cond < 0.7 ? 'warn' : 'text', cond: { frac: b.cond, tone: condTone(b.cond), tip: `Condition: ${Math.round(b.cond * 100)} of 100` }, jobs: [{ kind: 'service', label: 'Service', tip: 'A day: condition +15' }, { kind: 'overhaul', label: 'Overhaul', tip: 'A week: condition +45' }, { kind: 'clean', label: 'Clean', tip: 'A night' }] })),
    },
  };
}

// (the research tree as `company::research::TOPICS` has it: key, section, title, icon, stages,
// money (€) and trophies of the first stage, researcher-days of work, what it needs - another
// topic (a stage's `<topic>_<n>`) or the company's level -, and what it does in a few words)
const SECTIONS = [
  ['Buses', 'Buses and their running', [36, 150, 140], 'directions_bus'],
  ['Upgrades', 'Bus upgrades', [70, 128, 210], 'star'],
  ['Passengers', 'Passenger satisfaction', [52, 98, 178], 'person'],
  ['Premises', 'Company premises', [52, 150, 82], 'home'],
  ['Staff', 'Staff and their work', [124, 84, 196], 'groups'],
  ['Workshop', 'Workshop and spare parts', [180, 106, 48], 'construction'],
  ['Area', 'Expand the area of operation', [30, 134, 172], 'location_on'],
  ['Tools', 'Additional tools', [216, 170, 54], 'add'],
];
const TOPICS = [
  ['articulated', 'Buses', 'Articulated buses', 'directions_bus', 1, 40000, 0, 6, ['level:2'], 'Articulated buses at the dealers'],
  ['double_deckers', 'Buses', 'Double-deckers', 'directions_bus', 1, 60000, 25, 10, ['level:4'], 'Double-deckers at the dealers'],
  ['fuel', 'Buses', 'Reduce fuel consumption', 'speed', 5, 15000, 0, 4, [], '−2 % fuel a stage'],
  ['dirt', 'Buses', 'Reduce dirt on the buses', 'water_drop', 3, 10000, 5, 4, [], '−10 % dirt a stage'],
  ['wear', 'Buses', 'Reduce wear and tear', 'construction', 4, 20000, 0, 5, [], '−5 % wear a stage'],
  ['air_conditioning', 'Upgrades', 'Air conditioning', 'ac_unit', 1, 25000, 10, 6, [], '+2 % passengers'],
  ['passenger_info', 'Upgrades', 'Passenger information system', 'departure_board', 1, 30000, 15, 7, [], '+2 % passengers'],
  ['wlan', 'Upgrades', 'WLAN', 'wifi', 1, 20000, 20, 6, ['passenger_info'], '+2 % passengers, internet contracts'],
  ['usb', 'Upgrades', 'USB charging ports', 'bolt', 1, 15000, 20, 5, ['air_conditioning'], '+1.5 % passengers'],
  ['premium_seats', 'Upgrades', 'Premium seats', 'airline_seat_recline_normal', 1, 45000, 35, 9, ['usb'], '+3 % passengers'],
  ['stops', 'Passengers', 'Bus stop upgrades', 'location_on', 2, 30000, 15, 6, [], '+1.5 % passengers a stage'],
  ['app', 'Passengers', 'Mobile app', 'smartphone', 3, 25000, 20, 6, ['passenger_info'], '+1 % passengers a stage'],
  ['ticket_machines', 'Passengers', 'Ticket vending machines', 'confirmation_number', 3, 35000, 10, 6, [], '+1 % passengers, −3 % late trips a stage'],
  ['priority', 'Passengers', 'Bus priority on the routes', 'traffic', 3, 40000, 25, 8, ['stops_1'], '−10 % late trips a stage'],
  ['premises', 'Premises', 'Expand the company premises', 'garage', 3, 50000, 15, 8, [], '+6 bus places a stage'],
  ['external_depots', 'Premises', 'External depots', 'home', 1, 80000, 40, 12, ['premises_1', 'level:3'], '+10 bus places, repairs out there'],
  ['auto_cleaning', 'Staff', 'Automatic cleaning of the buses', 'autorenew', 1, 15000, 10, 5, [], 'The fleet stays at least 55 % clean'],
  ['auto_washing', 'Staff', 'Automatic washing of the buses', 'water_drop', 1, 20000, 15, 6, ['auto_cleaning'], '+50 % buses washed a night'],
  ['auto_refuelling', 'Staff', 'Automatic refuelling of the buses', 'sync_alt', 1, 25000, 20, 7, ['auto_cleaning'], '+50 % buses refuelled a night'],
  ['auto_repair', 'Staff', 'Automatic repair at the external depots', 'construction', 1, 30000, 30, 8, ['external_depots'], 'Broken buses back a day sooner'],
  ['ticket_inspectors', 'Staff', 'Ticket inspectors', 'badge', 1, 20000, 10, 5, [], 'Ticket inspectors can be hired'],
  ['inspection', 'Staff', 'Working method of the ticket inspectors', 'visibility', 3, 10000, 15, 4, ['ticket_inspectors'], '+15 % passengers checked a stage'],
  ['food', 'Staff', 'Better food for the employees', 'groups', 3, 12000, 10, 4, [], 'Happier, calmer staff'],
  ['parts', 'Workshop', 'Cheaper spare parts', 'inventory_2', 3, 15000, 0, 5, [], '−10 % spare part prices a stage'],
  ['repairs', 'Workshop', 'Faster repairs', 'timer', 3, 20000, 15, 6, [], '−10 % repair time a stage'],
  ['area', 'Area', 'More concessions', 'map', 3, 60000, 30, 10, ['level:3'], '+1 concession a stage'],
  ['park_and_ride', 'Area', 'Park and ride', 'local_parking', 1, 40000, 25, 8, ['area_1'], '+2 % passengers'],
  ['shift_schedule', 'Tools', 'Shift schedule', 'calendar_month', 1, 10000, 5, 4, [], 'Core hours for the staff'],
];
/** The company's level (`levels`), the stages researched and the research under way. */
const LEVEL = 2;
const researched = { passenger_info: 1, air_conditioning: 1, fuel: 2, parts: 1, auto_cleaning: 1 };
const rsRun = { topic: 'ticket_machines', stage: 1, share: 0.4, paused: false };

/** A topic by its key, or by a stage's (`<topic>_<n>`); and whether a need is met. */
const topicOf = (key) => TOPICS.find((t) => t[0] === key) || TOPICS.find((t) => t[4] > 1 && key === `${t[0]}_${key.slice(-1)}`);
function needMet(n) {
  if (n.startsWith('level:')) return LEVEL >= Number(n.slice(6));
  const t = topicOf(n);
  if (!t) return false;
  return (researched[t[0]] || 0) >= (t[0] === n ? t[4] : Number(n.slice(-1)));
}
function needText(n) {
  if (n.startsWith('level:')) return `Company level ${n.slice(6)}`;
  const t = topicOf(n);
  return !t ? n : t[0] !== n ? `${t[2]}, stage ${n.slice(-1)}` : t[2];
}

function research() {
  const card = ([key, sec, title, icon, stages, money, trophies, work, needs, effect]) => {
    const done = researched[key] || 0;
    const running = rsRun.topic === key ? rsRun : null;
    const state = done >= stages ? 'done' : running ? 'running' : needs.every(needMet) ? 'open' : 'locked';
    const [, label, colour] = SECTIONS.find((x) => x[0] === sec);
    const out = {
      key, title, icon, state, done, stages, frac: Math.min(1, (done + (running?.share || 0)) / stages), effect, info: effect, section: label, colour,
      stage: stages > 1 && done < stages ? `Stage ${done + 1} of ${stages}` : null,
    };
    if (state === 'done') out.said = 'Researched';
    else if (state === 'running') Object.assign(out, { said: `Researching: ${Math.round(running.share * 100)} %`, left: 'about 12 days to go' });
    else {
      const k = 1 + done * 0.5;
      const t = trophies + done * 10;
      const short = t - co().trophies;
      Object.assign(out, {
        work: `${Math.round(work * k)} days`, work_all: `${Math.round(work * k)} researcher-days of work`, money: eur(money * k * 100), money_tone: money * k * 100 > co().cash ? 'danger' : 'text',
        trophies: short > 0 ? `${t} (${short} missing)` : String(t), trophies_tone: short > 0 ? 'danger' : 'text',
      });
      if (state === 'open') Object.assign(out, { start: 'Start research', primary: !rsRun.topic });
      else {
        const list = needs.filter((n) => !needMet(n)).map(needText);
        Object.assign(out, { needs: `Needs: ${list.join(', ')}`, needs_list: list });
      }
    }
    return out;
  };
  const run = rsRun.topic && topicOf(rsRun.topic);
  const crew = { staff: 'Researchers at work: 1, 1.0 researcher-days of work a day.', staff_tone: 'soft', crew: '1 researcher' };
  const secOf = (key) => SECTIONS.find((x) => x[0] === key);
  return {
    title: 'Research', sub: 'Invest in new technologies and improvements to make your operations more efficient, sustainable and profitable.',
    strip: [
      { icon: 'payments', value: eur(co().cash), tone: 'ok', tip: 'Cash' },
      { icon: 'emoji_events', value: grouped(co().trophies), tone: 'warn', tip: 'Trophies: earned with goals and contracts, spent on research' },
      { icon: 'star', value: `${co().reputation.toFixed(1)} %`, tone: 'accent2', tip: 'Rating: how the authority and the passengers see the company' },
    ],
    banner: run
      ? {
          name: run[2], key: run[0], stage: run[4] > 1 ? `Stage ${rsRun.stage} of ${run[4]}` : null, pct: `${Math.round(rsRun.share * 100)} %`, days: rsRun.paused ? 'Paused' : '12 days left', ...crew, effect: run[9],
          paused: rsRun.paused, pause: 'Pause', resume: 'Resume', pause_tip: 'Pausing keeps the work done: nobody works on it until you resume it.',
          what: run[4] > 1 ? `Researching: ${run[2]}, stage ${rsRun.stage} of ${run[4]}` : `Researching: ${run[2]}`, share: rsRun.share, tone: rsRun.paused ? 'warn' : 'accent',
          line: `${Math.round(rsRun.share * 100)} % done, about 12 days to go at today's pace  ·  ${crew.staff}`,
          icon: run[3], colour: secOf(run[1])[2], motif: secOf(run[1])[3],
          stop: 'Stop the research', again: 'Click again to stop', stop_tip: 'Stopping gives back the trophies and half the money; the work done is lost.',
        }
      : { none: 'No research is running: choose one below and start it.', ...crew },
    sections: SECTIONS.map(([sec, label, colour, motif]) => {
      const cards = TOPICS.filter((t) => t[1] === sec).map(card);
      return { title: label, colour, motif, cards, count: `${cards.filter((c) => c.state === 'done').length} of ${cards.length} researched` };
    }),
    info_tip: 'What it does', detail_caps: 'DETAILS', pick: 'Choose a research to see what it does.', cost_caps: 'NEXT STAGE', needs_caps: 'NEEDS FIRST',
    words: {
      money: 'Money', trophies: 'Trophies', work: 'Work', done: 'Researched', locked: 'Locked', now: 'Current research', available: 'Available', running: 'Under way', locked_needs: 'Locked: needs %{what}',
      effect: 'Effect', cost: 'Cost', duration: 'Duration', researchers: 'Researchers', needs: 'Prerequisites', none: 'None', close: 'Close',
      info_done: 'This research has already been completed. Its benefits are now active in your operations.',
      info_running: 'This research is under way: its benefits come when its stage is done.',
      info_open: 'Everything it needs is there: start it when you like. One research runs at a time.',
      info_locked: 'It waits for what it needs: research that first, or reach the level it asks.',
    },
  };
}

/** The research's actions (`research::act`): one started, paused, stopped (null: not one of them). */
function researchAct(a) {
  switch (a.act) {
    case 'research_start':
      if (rsRun.topic) return { popup: { icon: 'search', title: 'One research at a time', text: 'Stop the research under way first, or wait until it is done.', unlock: '', go: null } };
      Object.assign(rsRun, { topic: a.key, stage: (researched[a.key] || 0) + 1, share: 0, paused: false });
      say('The research begins tomorrow.');
      return {};
    case 'research_pause':
      rsRun.paused = !!a.on;
      return {};
    case 'research_stop':
      Object.assign(rsRun, { topic: null, share: 0, paused: false });
      say('The research is stopped.');
      return {};
    default:
      return null;
  }
}

// (the spare parts as `company::parts::Part::ALL` has them: label, icon, list price (€), what the
// stores keep of it for twelve buses)
const PARTS = [
  ['Engine', 'settings', 3500, 1], ['Headlight', 'flashlight_on', 150, 3], ['Indicator', 'highlight', 40, 3], ['Wiper', 'rainy', 45, 3], ['Air conditioner', 'ac_unit', 1400, 1],
  ['Router', 'wifi', 300, 1], ['Brake pads', 'stop_circle', 180, 3], ['Tyre set', 'autorenew', 1200, 2], ['Oil canister', 'water_drop', 110, 4], ['V-belt', 'sync_alt', 70, 2],
];
const stock = [1, 3, 2, 3, 0, 1, 1, 2, 4, 2];
const partsAuto = { on: true };
const EXPRESS_FEE = 4500;

function workshop(a) {
  const counts = PARTS.map((_, k) => Math.max(0, Number(a.counts?.[k]) || 0));
  const express = !!a.express;
  // (the research "Cheaper spare parts", stage 1: 10 % off the list price)
  const price = (k) => Math.round(PARTS[k][2] * 0.9 * 100);
  const n = counts.reduce((x, y) => x + y, 0);
  const sum = counts.reduce((x, c, k) => x + c * price(k), 0) + (n && express ? EXPRESS_FEE : 0);
  const worth = PARTS.reduce((x, _, k) => x + stock[k] * price(k), 0);
  const short = PARTS.filter((p, k) => stock[k] < Math.ceil(p[3] / 2)).length;
  const inWs = BUSES.filter((b) => b.workshop);
  const cards = PARTS.map(([label, icon, list], k) => ({ label, icon, price: eur(price(k)), was: eur(list * 100), count: counts[k], stock: stock[k], line: eur(counts[k] * price(k)), line_on: counts[k] > 0 }));
  const rows = cards.map((c, k) => {
    const keep = PARTS[k][3];
    const low = Math.ceil(keep / 2);
    return { ...c, keep, low, min: `min ${low}`, level: `${stock[k]} / ${keep}`, tone: stock[k] === 0 ? 'danger' : stock[k] <= low ? 'warn' : 'ok', coming: k === 4 ? '1 on order' : null };
  });
  return {
    figures: [
      { label: 'Mechanics', value: '1', under: 'at work today', tone: 'text' },
      { label: 'Waiting for parts', value: String(inWs.length), under: 'buses in the workshop', tone: inWs.length ? 'warn' : 'text' },
      { label: 'On order', value: '1', under: 'parts on their way', tone: 'text' },
      { label: 'In store', value: eur(worth), under: 'worth of spare parts', tone: 'text' },
    ],
    cards, most: 30, in_store: 'In store',
    foot: { ways: ['Delivery at 16:00: free', `At once: ${eur(EXPRESS_FEE)}`], note: express ? 'In the stores as soon as it is bought.' : 'Ordered now, it comes today at 16:00.', sum: `Sum: ${eur(n ? sum : 0)}`, sum_tone: n && sum > co().cash ? 'danger' : 'text', chosen: n, buy: 'Buy', clear: 'Choose nothing' },
    head: { title: 'Workshop', sub: 'Maintain your fleet, order spare parts and keep your buses on the road.' },
    kpis: [
      { icon: 'users', tone: 'info', label: 'Mechanics at work', value: '1 / 1', under: 'at work today' },
      { icon: 'directions_bus', tone: inWs.length ? 'warn' : 'ok', label: 'Buses in the workshop', value: `${inWs.length} / ${BUSES.length}`, under: inWs.length ? `${inWs.length} waiting for parts` : 'none waiting for parts' },
      { icon: 'inventory_2', tone: 'accent', label: 'Parts on order', value: '1', under: `${eur(worth)} in store` },
    ],
    table: { title: 'Spare parts', sub: 'Check your stock levels and order what you need', cols: ['Part', 'Stock', 'Unit price', 'Quantity', 'Line total'], rows, less: 'One less', more: 'One more' },
    summary: { title: 'Order summary', sub: 'Choose delivery option and place your order', ways: [['Today 16:00', 'Free'], ['At once', eur(EXPRESS_FEE)]], total: 'Order total', amount: eur(n ? sum : 0), order: 'Order' },
    side: {
      waiting: { title: 'Waiting for parts', rows: inWs.map((b) => ({ number: b.number, parts: 'Air conditioner', since: 'waiting since Mon 19 Oct' })), none: 'No bus waits for a part.' },
      orders: { title: 'On order', rows: [{ what: '1 parts, today 16:00', parts: '1 × Air conditioner  ·  ordered by the stores' }], none: 'Nothing is on order.' },
      repair: { title: 'Buses in repair', rows: inWs.map((b) => ({ number: b.number, what: 'Repair', until: 'until Wed 21 Oct', parts: 'waits for: Air conditioner' })), none: 'No buses are currently in repair.', none_sub: 'Once a bus is in the workshop, it will appear here.' },
      auto: { title: 'Automatic orders', on: partsAuto.on, label: 'The stores order by themselves', text: 'Below the minimum, the stores order up to what they keep.', row: short ? `Under their minimum now: ${short} parts` : 'Every part is above its minimum.', row_tone: !short ? 'ok' : partsAuto.on ? 'accent' : 'warn' },
      mechanics: { title: 'Mechanics', text: '1 mechanic: enough for 6 buses.', tone: 'text', link: 'Staff page', tab: 2 },
      defects: { title: 'Defects reported', sub: "From the drivers' walk-around checks and what went wrong on the way: each is a repair job here.", none: 'No open defects.', rows: [{ icon: 'build', bus: '17', kind: 'Wiper', text: 'squeaks', date: '18 Oct', at: '18:20', who: 'Sabine Krüger', held_text: '', tone: 'dim', state: 'Noted' }] },
    },
  };
}

/** The workshop's actions (`workshop::act`): parts bought, the stores' own ordering (null: not one
 * of them). */
function workshopAct(a) {
  switch (a.act) {
    case 'parts_auto':
      partsAuto.on = !!a.on;
      return {};
    case 'parts_buy': {
      const counts = (a.counts || []).map((x) => Math.max(0, Number(x) || 0));
      if (!counts.some((x) => x > 0)) return {};
      if (a.express) counts.forEach((c, k) => k < stock.length && (stock[k] += c));
      say(a.express ? 'Bought: the spare parts are in the stores now.' : 'Ordered: the spare parts come today at 16:00.');
      return { bought: true };
    }
    default:
      return null;
  }
}

/** The mock's map: a grid of roads, the company's line along it. */
const GRID = [[0, 0, 2400, 0], [0, 800, 2400, 800], [0, 1600, 2400, 1600], [0, 0, 0, 1600], [800, 0, 800, 1600], [1600, 0, 1600, 1600], [2400, 0, 2400, 1600]];
const ROUTE = [200, 0, 800, 0, 800, 800, 1600, 800, 1600, 1600, 2300, 1600];

/** Where a vehicle is `m` metres along the line `p` (to and fro), and its heading. */
function along(p, m) {
  const seg = [];
  let len = 0;
  for (let i = 0; i + 3 < p.length; i += 2) {
    const l = Math.hypot(p[i + 2] - p[i], p[i + 3] - p[i + 1]);
    seg.push([p[i], p[i + 1], p[i + 2], p[i + 3], l]);
    len += l;
  }
  let d = ((m % (2 * len)) + 2 * len) % (2 * len);
  const back = d > len;
  if (back) d = 2 * len - d;
  for (const [x0, y0, x1, y1, l] of seg) {
    if (d <= l) {
      const f = d / l;
      const h = (Math.atan2(x1 - x0, y1 - y0) * 180) / Math.PI + (back ? 180 : 0);
      return [x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, (h + 360) % 360];
    }
    d -= l;
  }
  return [p[0], p[1], 0];
}

/** The fleet map's panel and buses (`fleetmap::view`): two of the company's buses on its line, and
 * the game's traffic as the plugin writes it while the game runs (`v.traffic`), moving with the
 * real clock. */
function fleetmap() {
  const s = Date.now() / 1000;
  const l = LINES[0];
  const bus = (key, m, tone) => {
    const [x, y, h] = along(ROUTE, m);
    return { key, at: [x, y], h, colour: l.colour, tag: l.number, state: 'service', tone, min: 0, player: false, on: false };
  };
  // (the plugin's places every two seconds)
  const t2 = Math.floor(s / 2) * 2;
  const vehicles = GRID.flatMap((p, k) => [0, 1].map((j) => {
    const [x, y, h] = along(p, t2 * (9 + k + j * 3) + j * 700 + k * 300);
    return [`${k}-${j}`, Math.round(x), Math.round(y), Math.round(h), ['car', 'car', 'taxi', 'truck', 'car', 'tram', 'bicycle'][(k + j) % 7]];
  }));
  const g = along(GRID[2], t2 * 8 + 400);
  const p = along(GRID[4], t2 * 10);
  return {
    built: true, title: 'FLEET MAP', live: { text: "Live: the game's time", tone: 'ok' }, clock: `${hhmm(co().minute)}:00`, secs: co().minute * 60, playing: false, is_live: true, speed: 0,
    tips: { run: 'Run the clock', stop: 'Stop the clock', x10: 'Ten times as fast', x60: 'Sixty times as fast', now: 'Back to now', close: 'Close' },
    counts: 'On the road: 2  ·  at a terminus: 0  ·  tours today: 2',
    legend: LINES.map((x, i) => ({ plate: plate(x), colour: x.colour, caption: x.caption, n: i ? 0 : 2 })), no_lines: 'The company runs no line on this day.',
    buses: [bus(`${l.name}|1`, s * 9, 'ok'), bus(`${l.name}|2`, s * 9 + 2600, 'warn')],
    traffic: {
      label: "The game's traffic",
      vehicles,
      buses: [{ key: 'game|62|4|1205', at: [g[0], g[1]], h: g[2], tag: '62' }],
      players: [{ key: 'player|Sam', at: [p[0], p[1]], h: p[2], tag: 'Sam' }],
    },
    picked: null, card: null,
  };
}

// - a line's trips and timetable, the planning's own sheet (`plan_edit`) -

/** The sheet open (`co_more_act` "plan_timetable"): its line, day, the tour to mark, the departure
 * chosen. Only the fixed lines' view (a concession's timetable is the authority's: nothing to
 * change); a line of the company's own is shown the same way in the mock. */
let sheet = null;

function planEdit() {
  const l = sheet && lineOf(sheet.line);
  if (!l) return { none: true };
  const [a, b] = l.caption.split(' – ');
  const ends = [b || l.caption, a || l.caption];
  const tours = TOURS.filter((t) => t.line === l.name);
  const run = Math.round(l.km * 2.6);
  // (each tour's trips back and forth over its duties' span; their departures make the lanes)
  const rows = tours.map((t) => {
    const from = t.duties[0][0];
    const to = t.duties[t.duties.length - 1][1];
    const legs = [];
    for (let m = from + 10, k = 0; m + run <= to; m += run + 10, k++) legs.push([m, m + run, k % 2]);
    return { number: t.tour, title: `Tour ${t.tour}`, fate: 'kept', tip: 'Runs as it does now', bus: (BUSES.find((x) => x.id === t.bus) || {}).number || null, lose: false, legs, focus: sheet.tour === t.tour };
  });
  const sel = sheet.sel;
  const lanes = ends.map((dest, k) => {
    const deps = rows
      .flatMap((r) => r.legs.filter((g) => g[2] === k).map((g) => ({ dep: g[0], arr: g[1], marked: r.focus })))
      .sort((x, y) => x.dep - y.dep)
      .map((d) => ({ dir: k, dep: d.dep, arr: d.arr, at: null, text: hhmm(d.dep), until: hhmm(d.arr), marked: d.marked, on: !!sel && sel.dir === k && Math.abs(sel.dep - d.dep) < 0.5 }));
    return { word: 'Trips', to: `→ ${dest}`, dest, count: `${deps.length} departures`, deps };
  });
  const chosen = sel && lanes[sel.dir]?.deps.find((d) => d.on);
  if (!chosen) sheet.sel = null;
  const trips = lanes.reduce((n, x) => n + x.deps.length, 0);
  const t0 = Math.floor(Math.min(...rows.flatMap((r) => r.legs.map((g) => g[0])), 5 * 60) / 60) * 60;
  const t1 = Math.ceil(Math.max(...rows.flatMap((r) => r.legs.map((g) => g[1])), 22 * 60) / 60) * 60;
  const fig = (label, v) => ({ label, now: v, then: v, tone: 'text' });
  return {
    line: l.name, plate: plate(l), title: `Trips and timetable of line ${l.number}`, caption: l.public || l.caption,
    days: null, date: dayLabel(sheet.date || co().date), note: "The concession's timetable is the authority's: it runs as it is.", tone: 'warn', editable: false,
    whole: [t0, t1], lanes, tools: null, sel: sheet.sel,
    departures: 'DEPARTURES', whole_day: 'Whole day', hint: 'The wheel over the departures zooms into the hours',
    choose: 'Choose a departure on the axis or in the list: drag it, type its time, or take it out.',
    copy: 'Copy', take_out: 'Take the trip out', shift_tip: 'The whole trip so many minutes earlier or later',
    tours_title: `TOURS: THE BUSES IT NEEDS (${rows.length})`, legend: null, no_tours: 'No tours on these days.', rows,
    needs: {
      caps: 'WHAT IT NEEDS', now: 'NOW', changed: 'CHANGED',
      figures: [fig('Trips', String(trips)), fig('Tours', String(rows.length)), fig('Buses at the busiest hour', String(rows.length)), fig('Duties for drivers', String(tours.reduce((n, t) => n + t.duties.length, 0))), fig('Kilometres a day', `${grouped(trips * l.km)} km`)],
      fleet: `The fleet has ${BUSES.length} buses for the line.`, fleet_tone: 'dim', tours: null, replan: null,
    },
    pattern: null, list_caps: 'AS A LIST', foot: null, dirty: false, close: 'Close', undo: 'Undo', again: 'Start again', again_tip: "The line's timetable as it is saved",
    leave: { ask: 'Close without saving the changes?', yes: 'Close without saving', no: 'Keep changing' },
  };
}

/** The sheet's actions (`plan_edit::act`): a departure chosen, closed (a fixed line changes nothing). */
function planEditAct(a) {
  if (a.act === 'pe_close') {
    sheet = null;
    return {};
  }
  if (!sheet) return { closed: true };
  if (a.act === 'pe_sel') sheet.sel = a.pick ? { dir: Number(a.pick.dir) || 0, dep: Number(a.pick.dep) || 0, at: a.pick.at ?? null } : null;
  return { sel: sheet.sel };
}

// - the buses the company works with (`co_vehicles`: the wizard's, the settings', the launcher's) -

function vehicles(a) {
  const cats = [['solo', 'Solo buses'], ['artic', 'Articulated buses'], ['double', 'Double-deckers']];
  const regions = [['de', 'Germany (DE)'], ['gb', 'United Kingdom (GB)']];
  const all = [['Vehicles/MAN_NL_NG/NL202.bus', 'MAN NL202', 'solo', 2], ['Vehicles/MAN_NG/NG272.bus', 'MAN NG272', 'artic', 1], ['Vehicles/MAN_SD200/SD202.bus', 'MAN SD202', 'double', 1]];
  const q = String(a.search || '').toLowerCase();
  const rows = all
    .filter(([, name, cat]) => (!q || name.toLowerCase().includes(q)) && (!a.cat || a.cat === cat))
    .map(([file, name, cat, owned]) => ({ file, name, maker: 'MAN', initials: name.split(' ').map((w) => w[0]).join(''), guess: `${cats.find((c) => c[0] === cat)[1]}, Germany`, owned: a.scope === 'company' ? owned : 0, on: true, cat, cat_fixed: false, guess_cat: cat, region: 'de', region_fixed: false, guess_region: 'de' }));
  return {
    reading: null, error: null, note: a.scope === 'launcher' ? 'The categories and regions count for every company.' : null,
    mode: 0, modes: [[0, "The map's region"], [1, 'Every bus'], [2, 'My own choice']], count: `${rows.length} of ${all.length} models`, quick: [['all', 'All on'], ['region', "The map's region"]],
    search: 'Search a bus', cats, regions, all_cats: 'Every category', all_regions: 'Every region', shown_of: `${rows.length} of ${all.length} shown`,
    switch_on: 'All shown on', switch_off: 'All shown off', none: 'No bus matches.', fixed_tip: 'Set by you', as_guessed: 'as guessed', owned_tip: 'In the fleet: it stays while the company has it',
    rows,
  };
}

// --- the answers -------------------------------------------------------------------------------

const VIEWS = { home, bell, overview, fleet, vehicle, people, lines, add_line: addLine, fare, money, fleet_value: fleetValue, inbox, agenda, launch, settings, time, confirm, report, career, dealer, ops, reputation, late_advice: lateAdvice, late, casual };
const MORE = { planning, concessions, contracts, depot, research, workshop, fleetmap, plan_edit: planEdit };

function doAct(a) {
  const c = co();
  const what = a.act;
  switch (what) {
    case 'hour':
      runClock(60);
      return {};
    case 'tomorrow':
    case 'quick_close':
      runClock(1440 - c.minute + 4 * 60);
      bump();
      return {};
    case 'step': {
      if (a.step === 'morning') runClock(((24 + 4) * 60 - c.minute) % 1440 || 1440);
      else if (a.step === 'midnight') runClock(1440 - c.minute);
      else if (a.step === 'until') runClock(((Number(a.minute) - c.minute) % 1440 + 1440) % 1440);
      else if (a.step === 'days') runClock(1440 * (Number(a.n) || 1));
      else runClock(Number(a.n) || 60);
      return {};
    }
    case 'play':
      st.speed = Number(a.speed) || 0;
      return {};
    case 'until':
      st.until = Number(a.minute) || st.until;
      return {};
    case 'dispatcher':
      st.dispatcher = !!a.on;
      bump();
      return {};
    case 'feed_minor':
      st.minor = !!a.on;
      bump();
      return {};
    case 'show_report':
      st.reports = true;
      return {};
    case 'close_reports':
      st.reports = false;
      return {};
    case 'report_line':
      st.reportLine = st.reportLine === a.line ? null : a.line;
      return {};
    case 'close_away':
      return {};
    case 'realtime':
      st.realtime = !!a.on;
      bump();
      return {};
    case 'friends_join':
      st.friendsJoin = !!a.on;
      bump();
      return {};
    case 'board':
      st.board = !!a.on;
      bump();
      return {};
    case 'rosterflow':
      co().rosterflow = !!a.on;
      bump();
      return { rosterflow: rosterflowOn() };
    case 'inbox_read':
      st.read.add(Number(a.id));
      bump();
      return {};
    case 'inbox_unread':
      st.read.delete(Number(a.id));
      bump();
      return {};
    case 'inbox_mark_all':
      MESSAGES.forEach((m) => st.read.add(m.id));
      bump();
      return {};
    case 'inbox_delete':
      MESSAGES.forEach((m) => m.id === Number(a.id) && (m.deleted = true));
      bump();
      return {};
    case 'inbox_delete_read':
      MESSAGES.forEach((m) => st.read.has(m.id) && (m.deleted = true));
      bump();
      return {};
    case 'drive_duty':
      st.launch = { duty: a.duty };
      bump();
      return {};
    case 'launch_close':
      st.launch = null;
      bump();
      return {};
    case 'launch_weather':
      st.weather = a.weather || 'clear';
      bump();
      return {};
    case 'launch_fix':
      st.launch = null;
      return { tab: 5 };
    case 'launch_start':
      return { popup: { icon: 'sports_esports', title: 'Not in the browser', text: 'The duty would start OMSI 2 now: the browser mock has no game to start.', unlock: '', go: null } };
    case 'confirmed':
      if (a.what === 'game_mode' && a.name) c.mode = a.name;
      say('Done.');
      bump();
      return {};
    case 'set_fare': {
      const l = lineOf(a.line);
      if (l && a.fare != null) l.fare = Number(a.fare);
      bump();
      return { done: true };
    }
    case 'line_colour': {
      const l = lineOf(a.name);
      if (l) l.colour = a.colour;
      bump();
      return {};
    }
    case 'line_title': {
      const l = lineOf(a.name);
      if (l) l.public = a.text;
      bump();
      return {};
    }
    case 'add_line':
      say(`Line ${a.name} runs from tomorrow.`);
      return { done: true };
    case 'hire':
      say('The new employee works for Grundorf Bus from today.');
      bump();
      return {};
    case 'open_loan':
      return { popup: { icon: 'receipt_long', title: 'A loan', text: 'The bank offers up to €84,000 at 4.9 % a year.', unlock: '', go: null } };
    case 'move_date':
      if (a.date) c.date = a.date;
      bump();
      return {};
    case 'bus_number': {
      // (`market::set_number`: an empty one back to the automatic number)
      const b = BUSES.find((x) => x.id === Number(a.id));
      const n = String(a.number || '').trim();
      if (!b) return {};
      if (BUSES.some((x) => x !== b && x.number === n)) return { popup: { icon: 'error', title: 'This cannot be done', text: 'Another bus of the fleet has this number.', unlock: '', go: null } };
      if (!n || n === b.auto) [b.number, b.auto] = [b.auto ?? b.number, undefined];
      else [b.number, b.auto] = [n, b.auto ?? b.number];
      bump();
      return {};
    }
    case 'new_take':
      return { joined: 5 };
    case 'design_livery':
      return { hand_over: 'livery=' + (BUSES.find((b) => b.id === Number(a.id))?.file || '') + '|' };
    case 'next_day':
      runClock(1440 - c.minute + 6 * 60);
      return {};
    default:
      bump();
      return {};
  }
}

export const companyAnswers = {
  co_boot: () => ({ open: st.open, chooser: chooser() }),
  co_open: (a) => {
    if (!companies.find((c) => c.id === a.id && !st.gone.has(c.id))) throw `no company ${a.id}`;
    st.open = a.id;
    st.speed = 0;
    st.launch = null;
    bump();
    return { open: a.id };
  },
  co_reread: (a) => ({ open: a.id }),
  co_delete: (a) => {
    st.gone.add(a.id);
    if (st.open === a.id) st.open = null;
    bump();
    say(`${companies.find((c) => c.id === a.id)?.name || a.id} is deleted.`);
    return { deleted: a.id, chooser: chooser() };
  },
  co_leave: () => {
    st.speed = 0;
    st.launch = null;
    return {};
  },
  co_found: (a) => {
    const name = String(a.name || '').trim();
    if (!name) return { refused: 'name' };
    const id = name.toLowerCase().replace(/[^a-z0-9]+/g, '-');
    const colours = (a.colours || [0, 4]).map((k) => PALETTE[k] || PALETTE[0]);
    companies.push({ ...companies[0], id, name, short: (a.short || name.slice(0, 2)).toUpperCase(), colours, mode: a.mode || 'normal', cash: 15000000, trophies: 0, reputation: 50, date: a.date || '2026-10-17', minute: 4 * 60, played: 0 });
    st.open = id;
    bump();
    say(`${name} is founded. Add a line, buy a bus and hire drivers.`);
    return { open: id, tab: HOME_TAB };
  },
  co_wizard: (a) => wizard(a),
  co_tick: (a) => {
    if (co() && !a.still && !a.modal && st.speed > 0) runClock(st.speed * 0.25);
    return shell();
  },
  co_company: () => co(),
  co_view: (a) => {
    if (a.view === 'chooser') return chooser();
    if (!co()) return { none: true };
    const f = VIEWS[a.view];
    if (!f) return {};
    return f(a);
  },
  co_act: (a) => (co() ? doAct(a) : {}),
  co_hand_over: (a) => {
    st.speed = 0;
    // (a line's timetable is the line editor's page for the line named: `lines::edit_to`, null
    // for a line the company does not have)
    const m = /^(?:timetable=|plan-line=)([^|]*)/.exec(a.page || '');
    return { edit: m ? editTo(lineOf(m[1])) : null };
  },
  co_tour: () => ({ at: null }),
  co_line_of: (a) => ({ name: LINES[Number(a.id) - 1]?.name || null }),
  // (a small town's roads, and the duty's route over them, in the map's metres)
  co_duty_map: () => ({
    roads: [[0, 0, 2400, 0], [0, 800, 2400, 800], [0, 1600, 2400, 1600], [0, 0, 0, 1600], [800, 0, 800, 1600], [1600, 0, 1600, 1600], [2400, 0, 2400, 1600]].map((p, k) => ({ p, w: 8, main: k === 1 })),
    route: [[200, 0, 800, 0, 800, 800, 1600, 800, 1600, 1600, 2300, 1600]],
    stops: [{ at: [200, 0], k: -1 }, { at: [800, 800], k: -1 }, { at: [1600, 800], k: -1 }, { at: [2300, 1600], k: -1 }],
    lo: [0, 0], hi: [2400, 1600],
  }),
  co_dutycard_save: () => ({ ok: true }),
  co_vehicles: (a) => vehicles(a),
  co_vehicles_act: (a) => ({ choice: { ...(a.choice || {}), mode: a.mode ?? 0 } }),
  co_more_view: (a) => {
    if (!co()) return { none: true };
    const f = MORE[a.view];
    return f ? f(a) : {};
  },
  co_more_act: (a) => (co() ? moreAct(a) : {}),
  co_more_map: () => ({
    roads: GRID.map((p, k) => ({ p, w: 8, main: k === 1 })),
    lines: [{ p: ROUTE, colour: LINES[0].colour }],
    stops: [{ at: [800, 0], k: -1 }, { at: [800, 800], k: -1 }, { at: [1600, 800], k: -1 }],
    lo: [0, 0], hi: [2400, 1600],
  }),
};
