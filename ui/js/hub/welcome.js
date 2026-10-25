// The welcome, as the old Omsi-Hub's (`launcher/ui/js/omsihub/welcome.js`, after its game-drawn
// launcher's `welcome.rs`): what a player sees the first time Omsi-Hub opens, in place of the
// start. In Luc's order: the language - every one Omsi-Hub speaks, as tiles with their flags,
// spoken at once; the animations, on or off, with a little scene that shows what "on" means;
// the bus stop signs the maps mark the stops with (German, British, French or Dutch: the old
// app's `stop_style`);
// the game's folders the first time (the openOMSI the add-on drives, the original OMSI 2), or
// when they are not right; the account - signed in, or on locally without sync; the driver - the
// account's, or one of this computer's (a new one, or one already there) with their personnel
// pass; and the way on - the tour, or straight to the start.
//
// One account belongs to one driver (`drivers::link_account`): signed in, the driver is the
// account's own (the display name's, made the first time); on locally, the player makes or
// chooses a driver as before, and plays without the friends, the leaderboard or cloud saves.
//
// It lies under a bar of its own steps on the start's ground, as one sheet in the middle of the
// window; going on, the next step slides in from the side it lies on - not with the setting
// "animations" off. `welcome_done` is written when it is finished or skipped; it opens when that
// is not set (and always on the first run, from `app.js`).

import { html, useEffect, useRef, useState } from '../../vendor/preact-htm.js';
import { S, set, useStore, toast } from '../store.js';
import { call, pickFolder } from '../api.js';
import { setLanguage } from '../i18n.js';
import { saveSettings, loadProfiles } from './drive.js';
import { Icon, tr } from './bits.js';
import { LOGO_SVG } from './logo.js';
import { Flag } from './bar.js';
import { LANGUAGE_NAMES } from './settings-extra.js';
import { useOnline, SignedOut, Profile } from '../online/online.js';
import { STOP_SIGNS, StopSign, stopSign } from './stopsigns.js';

let cssLoaded = false;
function loadCss() {
  if (cssLoaded) return;
  cssLoaded = true;
  const sheet = document.createElement('link');
  sheet.rel = 'stylesheet';
  sheet.href = 'css/hub/welcome.css';
  document.head.append(sheet);
}

/** The steps: their word and icon in the bar (`Step::label`). */
const LABELS = {
  language: ['Language', 'language'],
  animations: ['Animations', 'bolt'],
  signs: ['Stop signs', 'location_on'],
  folder: ['Game folders', 'folder_open'],
  account: ['Account', 'account_circle'],
  driver: ['Your driver', 'person'],
  ready: ['Ready', 'sports_score'],
};

/** The steps in their order: the folders' only when they have to be pointed to. */
export const stepsOf = (folder) => ['language', 'animations', 'signs', ...(folder ? ['folder'] : []), 'account', 'driver', 'ready'];

const W = () => S.hubWelcome;
const put = (patch) => S.hubWelcome && set({ hubWelcome: { ...S.hubWelcome, ...patch } });

/** What the welcome calls when the folders are kept on the first run (`app.js`: the app boots). */
let foldersKept = null;

/** The first run (`app.js`): the welcome from its first step, with the folders. */
export function welcomeFirstRun(onFolders) {
  foldersKept = onFolders;
  if (!S.hubWelcome?.open) open('language', true);
}

/** Whether the welcome opens as the launcher starts: when it was not finished or skipped
 * (`welcome::due`), not in the self-check. */
export async function welcomeAtStart() {
  if (S.hubWelcome?.open || S.boot?.selftest) return;
  if (S.settings?.welcome_done === true) return;
  open('language', false);
}

function open(step, folder) {
  loadCss();
  set({ hubWelcome: { open: true, step, folder, side: 0, seq: 0, typing: false, made: null, focus: step === 'driver' } });
}

/** The welcome open at `step` (the self-check's look at each of them). */
export const welcomeStep = (step) => open(step, false);

/** Show the welcome again from its first step (the settings): `welcome_done` is false until it
 * is finished again. */
export async function showWelcomeAgain() {
  await saveSettings({ welcome_done: false });
  open('language', false);
}

/** To another step: the sheet's slides in from the side it lies on (`Welcome::go_to`). */
function goTo(to) {
  const w = W();
  if (!w || to === w.step) return;
  const steps = stepsOf(w.folder);
  const forward = steps.indexOf(to) > steps.indexOf(w.step);
  put({ step: to, focus: to === 'driver', side: forward ? 1 : -1, seq: w.seq + 1 });
}
const next = () => {
  const steps = stepsOf(W().folder);
  const n = steps[steps.indexOf(W().step) + 1];
  if (n) goTo(n);
};
const back = () => {
  const steps = stepsOf(W().folder);
  const k = steps.indexOf(W().step);
  if (k > 0) goTo(steps[k - 1]);
};

/** The welcome is over (finished, or skipped): `welcome_done` is written and what comes after
 * begins (`welcome::apply`): the start (with the tour). Skipped before the folders were kept on
 * the first run, it goes back to them. */
async function finish(then) {
  const w = W();
  if (!w || w.leaving) return;
  if (foldersKept && w.folder) {
    goTo('folder');
    toast(tr('Choose your openOMSI and OMSI 2 folders first.'), 'info');
    return;
  }
  put({ leaving: true });
  const still = S.settings?.animations === false;
  await new Promise((r) => setTimeout(r, still ? 0 : 160));
  await saveSettings({ welcome_done: true });
  set({ hubWelcome: null });
  const { go } = await import('./hub.js');
  go('drive', 'mode');
  if (then === 'tour') import('./tour.js').then((m) => m.startTour?.()).catch(() => {});
}

// --- the bar ---------------------------------------------------------------------------------

/** The bar along the top: the brand, the welcome's steps as a route (done ones a way back),
 * and "Skip" on the right (`welcome::bar`). */
function Bar() {
  const w = useStore().hubWelcome;
  const steps = stepsOf(w.folder);
  const at = steps.indexOf(w.step);
  return html`<header class="hub-bar hub-glass hub-wbar">
    <span class="hub-brand" dangerouslySetInnerHTML=${{ __html: LOGO_SVG }}></span>
    <nav class="hub-steps">
      ${steps.map((id, k) => {
        const [label, icon] = LABELS[id];
        const done = k < at;
        return html`${k > 0 && html`<i class=${'hub-step-join' + (k <= at ? ' done' : '')}></i>`}
          <button type="button" class=${'hub-step' + (done ? ' done open' : '') + (k === at ? ' now' : '')} onClick=${() => done && goTo(id)}>
            <${Icon} name=${icon} size=${15} /><span>${tr(label)}</span></button>`;
      })}
    </nav>
    <span class="hub-bar-fill"></span>
    ${!(foldersKept && w.folder) && html`<button type="button" class="hub-wskip" onClick=${() => finish('start')}>${tr('Skip')}</button>`}
  </header>`;
}

// --- the sheet -------------------------------------------------------------------------------

/** A step's title and the line under it (`welcome::heading`). */
function heading(step, signedIn) {
  switch (step) {
    case 'animations':
      return [tr('Animations on or off?'), tr('Screens that slide in, tiles that rise under the mouse, a bus between the menus. Some like it lively, others calm.')];
    case 'signs':
      return [tr('Which bus stop signs?'), tr('The maps mark every stop with a bus stop sign. Choose the ones you know; you can change them any time in the settings.')];
    case 'folder':
      return [tr('Where are openOMSI and OMSI 2?'), tr('Omsi-Hub starts the openOMSI you have installed, on the maps and buses of the original OMSI 2. Point once to both folders.')];
    case 'account':
      return [tr('Sign in, or play locally?'), tr('With an account your driver has friends, the leaderboard and cloud saves on every computer. Without one, everything stays on this computer.')];
    case 'driver':
      return signedIn
        ? [tr('Your driver'), tr("Your account's driver: their duties, kilometres and level go with your account.")]
        : [tr('Who is driving?'), tr('Your duties, kilometres and level are kept per driver, on this computer.')];
    case 'ready': {
      const name = chosenDriver();
      return [name ? tr('Ready to go, %{name}', { name }) : tr('Ready to go'), tr('That is everything. Would you like a short tour of the launcher first?')];
    }
    default:
      return [tr('Welcome to Omsi-Hub'), tr('Choose the language of the launcher and the game. It applies at once, and you can change it any time in the settings.')];
  }
}

/** "STEP 2 OF 6". */
const eyebrow = (step) => {
  const steps = stepsOf(W().folder);
  return tr('Step %{n} of %{total}', { n: steps.indexOf(step) + 1, total: steps.length }).toUpperCase();
};

/** The account is signed in and is this driver's. */
const signedIn = (v) => v?.mode === 'in' && !v.need_profile;

function Sheet() {
  const s = useStore();
  const v = useOnline();
  const w = s.hubWelcome;
  const step = w.step;
  const [title, sub] = heading(step, signedIn(v));
  const Body = { language: Languages, animations: Animations, signs: Signs, folder: Folders, account: Account, driver: Driver, ready: Ready }[step];
  const move = w.side === 0 ? 'rise' : w.side > 0 ? 'right' : 'left';
  return html`<section class="hub-wsheet">
    <div class=${'hub-wmove ' + move} key=${w.seq}>
      <div class="hub-whead">
        <div class="eyebrow">${eyebrow(step)}</div>
        <h1>${title}</h1>
        <p>${sub}</p>
        <span class="hub-wlogo" dangerouslySetInnerHTML=${{ __html: LOGO_SVG }}></span>
      </div>
      <div class="hub-wbody"><${Body} /></div>
    </div>
    <${Foot} step=${step} v=${v} />
  </section>`;
}

/** A button of the foot: the big ones of the welcome (46 high). */
const Btn = ({ kind = '', icon, label, onClick, raw, id }) =>
  html`<button type="button" id=${id} class=${'hub-wbtn ' + kind} onClick=${onClick}>${icon && html`<${Icon} name=${icon} size=${19} />`}${label && (raw ? label : tr(label))}</button>`;

/** The sheet's foot: Back on the left (not on the first step), the way on on the right - its
 * word and what it does are the step's (`welcome::foot`). */
function Foot({ step, v }) {
  const w = useStore().hubWelcome;
  const steps = stepsOf(w.folder);
  let way = null;
  if (step === 'language' || step === 'animations' || step === 'signs') way = html`<${Btn} kind="primary" icon="chevron_right" label="Next" onClick=${next} />`;
  else if (step === 'folder') way = html`<${FoldersWay} />`;
  else if (step === 'account') way = signedIn(v) ? html`<${Btn} kind="primary" icon="chevron_right" label="Next" onClick=${next} />` : v?.mode === 'in' ? null : html`<${Btn} icon="chevron_right" label="Continue locally without sync" onClick=${playLocally} />`;
  else if (step === 'driver') way = html`<${Btn} kind="primary" icon="chevron_right" label="Next" onClick=${driverNext} />`;
  return html`<footer class="hub-wfoot">
    ${steps.indexOf(step) > 0 && html`<${Btn} kind="back" icon="chevron_left" label="Back" onClick=${back} />`}
    <span class="grow"></span>${way}
  </footer>`;
}

// --- the language ----------------------------------------------------------------------------

/** A language's flag: the bar's four as the bar has them, the others as simple as they still
 * read small (`welcome::paint_flag`). */
function LangFlag({ code }) {
  if (['en', 'de', 'fr', 'nl'].includes(code)) return html`<span class="hub-wflag"><${Flag} code=${code} /></span>`;
  const rows = (parts) => parts.map((c, k) => html`<rect x="0" y=${(20 * k) / parts.length} width="30" height=${20 / parts.length + 0.05} fill=${c} />`);
  const body = { ru: rows(['#ffffff', '#0039a6', '#d52b1e']), uk: rows(['#0057b7', '#ffd700']), pl: rows(['#ffffff', '#dc143c']), es: rows(['#aa151b', '#f1bf00', '#f1bf00', '#aa151b']) }[code];
  if (!body) return html`<span class="hub-wflag script">${code.toUpperCase()}</span>`;
  return html`<span class="hub-wflag"><svg viewBox="0 0 30 20" preserveAspectRatio="none">${body}</svg></span>`;
}

/** Every language Omsi-Hub speaks as a tile with its flag and its own name; a click speaks it
 * at once (`welcome::languages`). */
function Languages() {
  const s = useStore();
  const cur = s.lang || 'en';
  const pick = (code) => (foldersKept && W()?.folder ? setLanguage(code) : saveSettings({ language: code }));
  return html`<div class="hub-wlangs">
    ${(s.boot?.languages || ['en']).map((code) => {
      const on = code === cur;
      return html`<button type="button" class=${'hub-wtile hub-wlang' + (on ? ' on' : '')} onClick=${() => !on && pick(code)}>
        <${LangFlag} code=${code} /><b>${LANGUAGE_NAMES[code] || code}</b>${on && html`<${Icon} name="check" size=${17} />`}</button>`;
    })}
  </div>`;
}

// --- the animations --------------------------------------------------------------------------

const ease = (t) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2);
const smooth = (t) => {
  const x = Math.min(1, Math.max(0, t));
  return x * x * (3 - 2 * x);
};

/** The bus of the little scene at `t` seconds: how far along its route and how much of it is
 * seen - it pulls away, stops at the middle stop, goes on to the last and fades out there
 * (`welcome::bus_at`). */
function busAt(t) {
  const c = ((t % 4.6) + 4.6) % 4.6;
  if (c < 1.5) return [0.5 * ease(c / 1.5), Math.min(1, c / 0.3)];
  if (c < 2.2) return [0.5, 1];
  if (c < 3.7) return [0.5 + 0.5 * ease((c - 2.2) / 1.5), 1];
  return [1, 1 - smooth((c - 3.7) / 0.6)];
}

/** The scene's three tiles at `t`: the one the mouse is over, where the mouse is between them,
 * how high that tile has risen (`welcome::tiles_at`). */
function tilesAt(t) {
  const SLOT = 1.15;
  const p = (((t % (3 * SLOT)) + 3 * SLOT) % (3 * SLOT)) / SLOT;
  const k = Math.min(2, Math.floor(p));
  const u = p - k;
  const from = k === 0 ? 2 : k - 1;
  const pointer = from + (k - from) * ease(Math.min(1, u / 0.32));
  const lift = smooth((u - 0.22) / 0.18) * (1 - smooth((u - 0.84) / 0.16));
  return [k, pointer, lift];
}

/** A city bus seen from the side, its wheels on `y` at `x` (its middle) (`welcome::mini_bus`). */
function MiniBus({ x, y, s, a }) {
  if (a <= 0.01) return null;
  const w = 50 * s;
  const h = 19 * s;
  const bx = x - w / 2;
  const by = y - h;
  return html`<g opacity=${a}>
    <rect x=${bx + 2} y=${by + h - 2} width=${w - 4} height="4" rx="2" fill="rgba(0,0,0,0.5)" style="filter:blur(2px)" />
    <rect x=${bx} y=${by} width=${w} height=${h} rx=${4.5 * s} fill="rgb(238,241,246)" />
    <rect x=${bx + 4 * s} y=${by + 3 * s} width=${w - 10 * s} height=${6.5 * s} rx=${1.5 * s} fill="rgb(28,40,62)" />
    <rect x=${bx + w - 5 * s} y=${by + 3 * s} width=${3 * s} height=${9.5 * s} rx=${1.2 * s} fill="rgb(28,40,62)" />
    ${[0.33, 0.62].map((dx) => html`<rect x=${bx + w * dx} y=${by + 3 * s} width=${s} height=${h - 7 * s} fill="rgb(150,160,178)" />`)}
    <rect x=${bx + 2 * s} y=${by + h - 5.5 * s} width=${w - 4 * s} height=${1.8 * s} style="fill:var(--hub-accent)" />
    <circle cx=${bx + w - 1.6 * s} cy=${by + h - 6.5 * s} r=${1.3 * s} fill="rgb(255,210,63)" />
    ${[bx + 11 * s, bx + w - 12 * s].map((wx) => html`<circle cx=${wx} cy=${by + h} r=${4.2 * s} fill="rgb(14,18,28)" /><circle cx=${wx} cy=${by + h} r=${1.7 * s} fill="rgb(160,168,184)" />`)}
  </g>`;
}

/** The little scene: three tiles with the mouse over one and a bus on a route with three stops
 * - moving (`playing`), or standing still (`welcome::scene`). */
function Scene({ playing }) {
  const ref = useRef();
  const [w, setW] = useState(420);
  const [t, setT] = useState(0);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    // (the stage is scaled as a whole: its own lengths are the layout's)
    const measure = () => setW(el.offsetWidth || 420);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  useEffect(() => {
    if (!playing) return;
    let raf = 0;
    const t0 = performance.now();
    const tick = (now) => {
      setT((now - t0) / 1000);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing]);
  const h = 140;
  const gap = 8;
  const th = Math.min(h * 0.28, 38);
  const tw = Math.min((w - 36 - 2 * gap) / 3, th * 2.6);
  const tx = (w - 3 * tw - 2 * gap) / 2;
  const ty = 16;
  const [at, pointer, lift] = playing ? tilesAt(t) : [0, 0, 0];
  const y = h * 0.76;
  const [x0, x1] = [26, w - 26];
  const [along, seen] = playing ? busAt(t) : [0, 1];
  const px = tx + pointer * (tw + gap) + tw * 0.62;
  const py = ty + th * 0.62 - 3 * lift;
  const pt = (pts) => pts.map(([dx, dy]) => `${px + dx},${py + dy}`).join(' ');
  return html`<div class="hub-wscene" ref=${ref}>
    <svg width=${w} height=${h} viewBox=${`0 0 ${w} ${h}`}>
      <g class="roads">
        <line x1=${w * 0.55} y1="-10" x2=${w * 0.8} y2=${h + 10} stroke-width="9" />
        <line x1="-10" y1=${h * 0.5} x2=${w + 10} y2=${h * 0.38} stroke-width="6" />
        <line x1=${w * 0.12} y1=${h + 10} x2=${w * 0.3} y2="-10" stroke-width="5" />
      </g>
      ${[0, 1, 2].map((k) => {
        const up = k === at ? lift : 0;
        const r = [tx + k * (tw + gap) - 1.5 * up, ty - 3 * up, tw + 3 * up, th + 2 * up];
        return html`<g class=${'tile' + (up > 0.01 ? ' up' : '')} style=${`--up:${up}`}>
          ${up > 0.01 && html`<rect x=${r[0]} y=${r[1] + 3 * up} width=${r[2]} height=${r[3]} rx="6" fill=${`rgba(0,0,0,${0.45 * up})`} style=${`filter:blur(${3 + 2 * up}px)`} />`}
          <rect class="face" x=${r[0]} y=${r[1]} width=${r[2]} height=${r[3]} rx="6" />
          <rect class="bar" x=${r[0] + 7} y=${r[1] + r[3] / 2 - 2} width=${r[2] * 0.45} height="4" rx="2" style=${`opacity:${0.22 + 0.2 * up}`} />
        </g>`;
      })}
      ${playing && html`<polygon points=${pt([[-1.2, -1.8], [-1.2, 15.6], [4.2, 11.4], [11.6, 10.6]])} fill="rgba(10,14,24,0.9)" /><polygon points=${pt([[0, 0], [0, 13], [3.6, 9.8], [9, 9.4]])} fill="#fff" />`}
      <line x1=${x0} y1=${y} x2=${x1} y2=${y} stroke="rgb(18,58,107)" stroke-width="7" stroke-linecap="round" />
      <line x1=${x0} y1=${y} x2=${x1} y2=${y} stroke-width="4" stroke-linecap="round" style="stroke:var(--hub-accent)" />
      ${[x0, (x0 + x1) / 2, x1].map((sx) => html`<circle cx=${sx} cy=${y} r="6" fill="rgb(10,107,61)" /><circle cx=${sx} cy=${y} r="4.4" fill="rgb(255,210,63)" />`)}
      <${MiniBus} x=${x0 + (x1 - x0) * along} y=${y - 7} s=${Math.min(1, Math.max(0.8, h / 140))} a=${seen} />
    </svg>
  </div>`;
}

/** The animations, on or off: two cards, each with its little scene; a click sets the setting
 * at once (`welcome::animations`). */
function Animations() {
  const s = useStore();
  const on = s.settings?.animations !== false;
  const [hover, setHover] = useState(false);
  const cards = [
    [true, 'Animations on', 'Screens slide in, tiles rise under the mouse and a bus drives between the menus.'],
    [false, 'Animations off', 'Everything stands still and each screen is there at once: calmer, and lighter for an older computer.'],
  ];
  return html`<div class="hub-wcards anim">
    ${cards.map(([v, title, line]) => html`<button type="button" class=${'hub-wtile hub-wcard' + (on === v ? ' on' : '')}
        onMouseEnter=${() => v && setHover(true)} onMouseLeave=${() => v && setHover(false)} onClick=${() => on !== v && saveSettings({ animations: v })}>
      <${Scene} playing=${v && (on || hover)} />
      <span class="t"><span class=${'hub-radio' + (on === v ? ' on' : '')}></span><b>${tr(title)}</b></span>
      <span class="line">${tr(line)}</span>
    </button>`)}
  </div>`;
}

// --- the stop signs -------------------------------------------------------------------------

/** The bus stop signs as tiles, each with its sign; a click chooses it at once (German until
 * another is chosen). */
function Signs() {
  useStore();
  const cur = stopSign();
  return html`<div class="hub-wsigns">
    ${STOP_SIGNS.map(([id, name]) => {
      const on = id === cur;
      return html`<button type="button" class=${'hub-wtile hub-wsign' + (on ? ' on' : '')} data-sign=${id} onClick=${() => !on && saveSettings({ stop_style: id })}>
        <span class="hub-wsignpic"><${StopSign} style=${id} size=${64} /></span><b>${tr(name)}</b>${on && html`<${Icon} name="check" size=${17} />`}</button>`;
    })}
  </div>`;
}

// --- the folders -----------------------------------------------------------------------------

/** Where the folders usually are: the fields' hints (paths, not texts to translate). */
const hints = () =>
  S.boot?.platform === 'macos'
    ? { openomsi: '/Applications/openOMSI.app', omsi2: '~/OMSI 2' }
    : { openomsi: 'C:\\Games\\openOMSI', omsi2: 'C:\\Program Files (x86)\\Steam\\steamapps\\common\\OMSI 2' };

/** The two folders as typed or chosen (else the ones found), and what each is: openOMSI's
 * version and whether it is new enough (`hub_setup_openomsi`), OMSI 2's whether it is whole
 * (`hub_setup_omsi2`). */
function useFolders() {
  const w = useStore().hubWelcome;
  const [found, setFound] = useState({ openomsi: [], omsi2: [] });
  const [checks, setChecks] = useState({ openomsi: null, omsi2: null });
  useEffect(() => {
    call('hub_setup_suggest').then(setFound, () => {});
  }, []);
  const st = S.boot?.settings || {};
  const folders = {
    openomsi: w.openomsi ?? (st.openomsi_folder || found.openomsi?.[0] || ''),
    omsi2: w.omsi2 ?? (st.omsi2_folder || found.omsi2?.[0] || ''),
  };
  useEffect(() => {
    let gone = false;
    const look = (k, cmd) => (folders[k] ? call(cmd, { folder: folders[k] }).then((c) => !gone && setChecks((x) => ({ ...x, [k]: { folder: folders[k], ...c } })), () => {}) : setChecks((x) => ({ ...x, [k]: null })));
    look('openomsi', 'hub_setup_openomsi');
    look('omsi2', 'hub_setup_omsi2');
    return () => (gone = true);
  }, [folders.openomsi, folders.omsi2]);
  const fresh = (k) => (checks[k]?.folder === folders[k] ? checks[k] : null);
  return { folders, found, openomsi: fresh('openomsi'), omsi2: fresh('omsi2') };
}

/** The folders kept, as the first-run setup kept them (`hub_setup_save`, with the language),
 * and the app booted on them (the drivers and the content read). */
async function keepFolders(folders) {
  try {
    await call('hub_setup_save', { openomsi: folders.openomsi.trim(), omsi2: folders.omsi2.trim(), language: S.lang });
    const then = foldersKept;
    foldersKept = null;
    put({ folder: false });
    await then?.();
    loadProfiles();
    return true;
  } catch (e) {
    toast(String(e?.message || e), 'error');
    return false;
  }
}

function FoldersWay() {
  const f = useFolders();
  const ok = f.openomsi?.ok && f.omsi2?.ok;
  return ok
    ? html`<${Btn} id="welcome-folders-save" kind="primary" icon="check" label="Use these folders" onClick=${async () => (await keepFolders(f.folders)) && goTo('account')} />`
    : foldersKept
      ? null
      : html`<${Btn} icon="chevron_right" label="Skip for now" onClick=${next} />`;
}

/** One folder: its path (typed, or chosen with Browse), the ones found, and what it is. */
function FolderField({ id, caps, hint, value, found, check, title }) {
  const browse = async () => {
    const p = await pickFolder(title).catch(() => null);
    if (p) put({ [id]: p });
  };
  const said = check && (check.say || check.text) ? tr(check.say || check.text, check.vars) : '';
  const verdict = check && (check.ok ? ['check_circle', 'ok', check.version ? `openOMSI ${check.version}${said ? ' · ' + said : ''}` : said || tr('Found')] : ['warning', 'warn', said]);
  return html`<div class="hub-wfolder">
    <div class="hub-wcaps">${caps}</div>
    <div class="row">
      <label class="hub-wfield"><${Icon} name="folder_open" size=${18} /><input id=${'welcome-' + id} value=${value} placeholder=${hint} spellcheck="false" onInput=${(e) => put({ [id]: e.currentTarget.value })} /></label>
      <${Btn} label="Browse" onClick=${browse} />
    </div>
    ${found?.length > 1 && !check?.ok && html`<p class="note">${tr('Found')}: ${found.map((x) => html`<a href="#" class="chip" onClick=${(e) => (e.preventDefault(), put({ [id]: x }))}>${x}</a>`)}</p>`}
    ${verdict && html`<div class=${'verdict ' + verdict[1]}><${Icon} name=${verdict[0]} size=${17} /><span id=${'welcome-' + id + '-check'}>${verdict[2]}</span></div>`}
  </div>`;
}

/** The folders: the openOMSI to drive and the original OMSI 2 (`welcome::folder`). */
function Folders() {
  const f = useFolders();
  const h = hints();
  return html`<div class="hub-wfolders">
    <${FolderField} id="openomsi" caps=${tr('openOMSI folder')} hint=${h.openomsi} value=${f.folders.openomsi} found=${f.found.openomsi} check=${f.openomsi} title=${tr('Your openOMSI folder')} />
    <${FolderField} id="omsi2" caps=${tr('OMSI 2 folder')} hint=${h.omsi2} value=${f.folders.omsi2} found=${f.found.omsi2} check=${f.omsi2} title=${tr('The OMSI 2 folder (with maps and Vehicles in it)')} />
    <p class="note">${tr('openOMSI 0.2.22 or newer. Steam, the box version, a folder you moved yourself - all fine. Omsi-Hub only reads them and never writes into the OMSI 2 folder.')}</p>
  </div>`;
}

// --- the account -----------------------------------------------------------------------------

/** On without an account (`ol_local`): the driver is one of this computer's. */
async function playLocally() {
  await call('ol_local', { on: true }).catch(() => {});
  next();
}

/** The account: signing in (email, Google, or an account made on the website), the display
 * name the first time - which is the driver's -, or the account signed in. */
function Account() {
  const v = useOnline();
  if (!v) return html`<p class="note">…</p>`;
  if (v.mode === 'in' && v.need_profile) return html`<div class="hub-waccount"><${Profile} v=${v} /><p class="note">${tr("Your display name is your driver's name too.")}</p></div>`;
  if (v.mode === 'in') {
    const name = v.me?.name || '';
    return html`<div class="hub-waccount in">
      <div class="hub-wpass"><div class="band"><span>OMSI-HUB  ·  ${tr('Account').toUpperCase()}</span><${Icon} name="cloud" size=${18} /></div>
        <div class="who"><span class="mono">${name.slice(0, 1).toUpperCase()}</span><span><b>${name}</b><small>${tr('Signed in')}</small></span></div>
        <hr /><p>${tr('Friends, the leaderboard and your cloud saves go with %{name}.', { name })}</p></div>
      <button type="button" class="hub-wghost" onClick=${() => call('ol_sign_out').catch(() => {})}><${Icon} name="close" size=${17} />${tr('Sign out')}</button>
    </div>`;
  }
  return html`<div class="hub-waccount"><${SignedOut} v=${v} welcome /></div>`;
}

// --- the driver ------------------------------------------------------------------------------

/** The driver chosen (one of this computer's), or the one just made. */
function chosenDriver() {
  const cur = (S.config?.profile || '').trim();
  if (!cur) return null;
  const low = cur.toLowerCase();
  return (S.profiles || []).find((n) => n.toLowerCase() === low) || (W()?.made && W().made.toLowerCase() === low ? W().made : null);
}

/** Choose a driver already on this computer. */
async function chooseDriver(name) {
  try {
    const config = await call('hub_profile_select', { name });
    set({ config, profile: await call('hub_profile', { name }).catch(() => null) });
    put({ typing: false, typed: '', passSeq: (W().passSeq || 0) + 1 });
  } catch (e) {
    toast(e.message, 'error');
  }
}

/** Make the driver typed and choose them; their pass pops in, with the personnel number and
 * code made now (`welcome::create`). */
async function createDriver() {
  const name = (W().typed || '').trim();
  if (!name) {
    put({ focus: true });
    toast(tr('Type a name for your driver first.'), 'info');
    return false;
  }
  try {
    await call('hub_profile_create', { name, sex: 'M' });
    await loadProfiles();
    await chooseDriver(name);
    const p = await call('hub_profile_personnel', { name, make: true }).catch(() => null);
    put({ made: name, typing: false, typed: '', personnel: { name, p } });
    return true;
  } catch (e) {
    toast(e.message, 'error');
    return false;
  }
}

/** Next on the driver's step: with a name typed the driver is made first; without a driver the
 * field asks for one (`welcome::driver_next`). */
async function driverNext() {
  const w = W();
  const typed = !!(w.typed || '').trim();
  if ((w.typing || !chosenDriver()) && typed) {
    if (await createDriver()) next();
  } else if (chosenDriver()) next();
  else {
    put({ focus: true });
    toast(tr('Type a name for your driver first.'), 'info');
  }
}

/** The new driver's name and Create (Enter makes them; Escape gives up when there is a driver
 * to go back to) (`welcome::new_driver`). */
function NewDriver({ canCancel }) {
  const w = useStore().hubWelcome;
  const ref = useRef();
  useEffect(() => {
    if (w.focus) {
      ref.current?.focus();
      put({ focus: false });
    }
  }, [w.focus]);
  const cancel = () => put({ typing: false, typed: '' });
  return html`<div class="hub-wnew">
    <div class="hub-wcaps">${tr('New driver')}</div>
    <label class="hub-wfield"><${Icon} name="person" size=${18} /><input ref=${ref} value=${w.typed || ''} placeholder=${tr("The new driver's name")} spellcheck="false"
      onInput=${(e) => put({ typed: e.currentTarget.value })} onKeyDown=${(e) => (e.key === 'Enter' ? createDriver() : e.key === 'Escape' && canCancel ? cancel() : null)} /></label>
    <div class="row">
      <${Btn} kind="primary small" icon="add" label="Create" onClick=${createDriver} />
      ${canCancel && html`<${Btn} kind="small" icon="close" label="Cancel" onClick=${cancel} />`}
    </div>
    <p class="note">${tr("The driver's name is the name of their personnel file, as in OMSI 2: the game adds every duty to it.")}</p>
  </div>`;
}

/** The chosen driver's personnel pass: their initial and name, their level, and - when they
 * were made - the personnel number and code they sign on with (`welcome::pass`). `account`:
 * the account's driver (no other driver to make or choose), `pass` the account's own numbers
 * once fetched (`ol_state`'s `me.personnel`: the same on every computer). */
function Pass({ name, account, pass }) {
  const s = useStore();
  const w = s.hubWelcome;
  const known = pass || (w.personnel?.name === name ? w.personnel.p : undefined);
  useEffect(() => {
    if (known !== undefined) return;
    // (not over the numbers `createDriver` made meanwhile: this look may answer after them)
    const tell = (p) => W()?.personnel?.name !== name && put({ personnel: { name, p } });
    call('hub_profile_personnel', { name, make: false }).then(tell, () => tell(null));
  }, [name, known]);
  const level = s.profile && String(s.profile.name || '').toLowerCase() === name.toLowerCase() ? s.profile.level || 1 : 1;
  return html`<div>
    <div class=${'hub-wpass' + (known ? ' numbers' : '')} key=${w.passSeq || 0}>
      <div class="band"><span>OPENOMSI  ·  ${tr('Personnel pass').toUpperCase()}</span><${Icon} name=${account ? 'cloud' : 'directions_bus'} size=${18} /></div>
      <div class="who"><span class="mono">${name.slice(0, 1).toUpperCase()}</span><span><b>${name}</b><small>${tr('Bus driver · level %{level}', { level })}</small></span></div>
      <hr />
      ${known
        ? html`<div class="nums">${[['Personnel number', known.number], ['Code', known.code]].map(([k, v]) => html`<span><small>${tr(k).toUpperCase()}</small><b>${v}</b></span>`)}</div>
            <p>${tr('You sign on with these before every duty.')}</p>`
        : html`<p class="wrap">${tr('Your personnel number and code are made when you first sign on for a duty.')}</p>`}
    </div>
    ${!account && html`<button type="button" class="hub-wghost" onClick=${() => put({ typing: true, typed: '', focus: true })}><${Icon} name="add" size=${17} />${tr('New driver')}</button>`}
  </div>`;
}

/** The drivers on this computer as rows to choose (`welcome::driver_list`). */
function DriverList({ names, chosen, account }) {
  return html`<div class="hub-wdrivers">
    <div class="hub-wcaps">${tr('On this computer').toUpperCase()}<em>${names.length}</em></div>
    <div class="list">${names.map((n) => {
      const on = !!chosen && chosen.toLowerCase() === n.toLowerCase();
      // (the account's driver is chosen by signing in: here it is the account's, not a choice)
      const theirs = !!account && account.toLowerCase() === n.toLowerCase();
      return html`<button type="button" class=${'hub-wtile hub-wdriver' + (on ? ' on' : '')} title=${theirs ? tr('This driver has the account on this computer') : undefined} onClick=${() => !on && chooseDriver(n)}>
        <span class="mono">${n.slice(0, 1).toUpperCase()}</span><b>${n}</b>${theirs && html`<${Icon} name="cloud" size=${16} />`}${on && html`<${Icon} name="check" size=${18} />`}</button>`;
    })}</div>
  </div>`;
}

/** The driver: the account's (signed in), or a new one or the chosen one's pass, and beside it
 * the drivers already on this computer (`welcome::driver`). */
function Driver() {
  const s = useStore();
  const v = useOnline();
  const w = s.hubWelcome;
  // (the account's driver: `drivers::link_account` made and chose it at the sign-in)
  useEffect(() => {
    loadProfiles();
  }, [signedIn(v)]);
  const names = s.profiles || [];
  const chosen = chosenDriver();
  if (signedIn(v)) return html`<div class="hub-wdriverstep"><div class="left">${chosen ? html`<${Pass} name=${chosen} account pass=${v.me?.personnel} />` : html`<p class="note">…</p>`}</div></div>`;
  const form = w.typing || !chosen;
  return html`<div class=${'hub-wdriverstep' + (names.length ? ' two' : '')}>
    <div class="left">${form ? html`<${NewDriver} canCancel=${!!chosen} />` : html`<${Pass} name=${chosen} />`}</div>
    ${names.length > 0 && html`<${DriverList} names=${names} chosen=${chosen} account=${s.accountDriver} />`}
  </div>`;
}

// --- ready -----------------------------------------------------------------------------------

/** The way on: the tour or straight to the start, as tiles with a photo each
 * (`welcome::ready`). */
function Ready() {
  const tiles = [
    ['tour', 'Take the tour', 'route', 'mode-tour', 'A minute along the launcher: where you set up a duty, your service record and the settings.'],
    ['start', 'Start right away', 'play_arrow', 'mode-free', 'Straight to the start: choose how you want to drive today.'],
  ];
  return html`<div class="hub-wready">
    ${tiles.map(([then, title, icon, pic, text]) => html`<button type="button" class=${'hub-wtile hub-wgo ' + then} onClick=${() => finish(then)}>
      <span class="photo" style=${`background-image:url("img/hub/${pic}.jpg")`}></span>
      <span class="words"><i><${Icon} name=${icon} size=${20} /></i><b>${tr(title)}</b><span>${tr(text)}</span></span>
    </button>`)}
  </div>`;
}

// --- the whole -------------------------------------------------------------------------------

/** The welcome in place of the page while it is open. */
export function Welcome() {
  const s = useStore();
  const w = s.hubWelcome;
  useEffect(() => {
    if (!w) return;
    // (the steps without a text field go on with Enter as well)
    const key = (e) => {
      if (e.key !== 'Enter') return;
      if (document.activeElement && /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) return;
      const st = W()?.step;
      if (st === 'language' || st === 'animations' || st === 'signs') next();
      // (a tile with the keyboard on it is what Enter presses: "Take the tour" started the
      // launcher instead)
      else if (st === 'ready' && document.activeElement?.tagName !== 'BUTTON') finish('start');
    };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, [!!w]);
  if (!w?.open) return null;
  return html`<div class=${'hub-welcome' + (w.leaving ? ' leaving' : '')}>
    <${Bar} />
    <div class="hub-wroom"><${Sheet} /></div>
  </div>`;
}
