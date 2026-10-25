// The game controllers on Omsi-Hub's Settings page (the tab "Controllers"): the steering wheel,
// the pedals, joysticks, shifters and button boxes as openOMSI drives them, from its
// `Inputs/gamectrler.cfg` (`addon_controllers`, hub-game's `controllers.rs`).
//
// Each device: what its eight axes do (steering, the pedals, looking round; reversed; the
// response curve), its force feedback, and what every button and hat direction presses - one
// or more key actions, held for as long as the button is, or a latching switch. The webview
// sees the connected devices itself (the browser's game controllers): their axes move on the
// page, a button pressed lights its line (and picks its device), and "Set up step by step" finds
// the wheel and the pedals by turning and pressing them, as openOMSI's launcher does. The axes are
// DirectInput's in the file (X, Y, Z, the rotations, the sliders), and the webview numbers a
// wheel's, a pedal set's or a joystick's the same; a gamepad (the webview's standard layout)
// needs no setting up - openOMSI drives it with its sticks and triggers. Every change is saved at
// once (`addon_controllers_save`), and the settings round them - dead zone, the pedals' strength,
// the wheel's turn, the gamepad, force feedback - are openOMSI's own (`gamesettings.js`).

import { html, useEffect, useRef, useState } from '../../vendor/preact-htm.js';
import { call, openPath } from '../api.js';
import { toast } from '../store.js';
import { Button, Icon, Slider, tr } from './bits.js';
import { Select } from '../ui.js';
import { GameSettingsPanel } from './gamesettings.js';

// (this tab's own look, loaded with it)
if (!document.querySelector('link[data-pads-css]')) {
  const sheet = document.createElement('link');
  sheet.rel = 'stylesheet';
  sheet.href = 'css/hub/controllers.css';
  sheet.dataset.padsCss = '1';
  document.head.append(sheet);
}

export const AXES = ['X axis', 'Y axis', 'Z axis', 'X rotation', 'Y rotation', 'Z rotation', 'Slider 1', 'Slider 2'];
const FUNCS = [[-1, 'Not used'], [0, 'Steering'], [1, 'Throttle'], [2, 'Brake'], [3, 'Clutch'], [4, 'Throttle and brake'], [5, 'Look left / right'], [6, 'Look up / down']];
const SHAPES = [[0, 'Linear'], [8, 'Progressive'], [4, 'Degressive'], [24, 'Bi-progressive'], [20, 'Bi-degressive']];
const CURVE = 4 | 8 | 0x10;
/** The first button number of the hats' directions (four a hat: up, right, down, left). */
const HATS = 128;
const DIRS = ['up', 'right', 'down', 'left'];

/** What an action does, in words: OMSI's actions have German names of their own. */
const KNOWN = {
  throttle: 'Throttle', brake: 'Brake', throttle_amplify: 'Throttle (full, kickdown)', clutch: 'Clutch',
  steering_left: 'Steer left', steering_right: 'Steer right', steering_neutral: 'Steering to centre',
  parking_brake_toggle: 'Parking brake', parking_brake_set: 'Parking brake on', parking_brake_release: 'Parking brake off',
  blinker_left_set: 'Indicator left', blinker_right_set: 'Indicator right', blinker_left_toggle: 'Indicator left (toggle)', blinker_right_toggle: 'Indicator right (toggle)',
  blinker_off: 'Indicators off', blinker_warn_toggle: 'Hazard lights', gear_up: 'Gear up (manual gearbox)', gear_down: 'Gear down (manual gearbox)', horn: 'Horn',
  kw_scheinwerfer_toggle: 'Headlights', kw_standlicht_toggle: 'Sidelights', kw_fernlicht_toggle: 'High beam', kw_m_enginestart: 'Starter', kw_wipermode_up: 'Wipers (next mode)',
  cp_batterietrennschalter_toggle: 'Battery / ignition', automatic_D: 'Gear D', automatic_N: 'Gear N (automatic gearbox)', automatic_R: 'Gear R (automatic gearbox)',
  kw_s_N: 'Gear N (manual gearbox)', kw_s_R: 'Gear R (manual gearbox)',
  bus_doorfront0: 'Front door (leaf 1)', bus_doorfront1: 'Front door (leaf 2)', bus_dooraft: 'Release rear doors',
  door_1: 'Door 1 (front), any bus', door_2: 'Door 2, any bus', door_3: 'Door 3, any bus', door_4: 'Door 4, any bus', doors_all: 'All doors, any bus', doors_release: 'Door release (passengers open), any bus',
  ticket_give: 'Sell the requested ticket', pax_door_request: 'Passenger: request entry / exit',
  view_set_driver: "Driver's view", view_set_passenger: 'Passenger view', view_set_outside: 'Outside view', view_toggle_viewpoint: 'Next view', view_toggle_interior: 'Cabin and outside, one key',
  view_interiorcam_plus: 'Next interior camera', view_interiorcam_minus: 'Previous interior camera', view_reset_direction: 'Reset the view direction', view_reset_all_directions: 'Reset all view directions',
  view_look_left: 'Look left (hold)', view_look_right: 'Look right (hold)', view_look_up: 'Look up (hold)', view_look_down: 'Look down (hold)', view_zoom_in: 'Zoom in', view_zoom_out: 'Zoom out',
  head_tracking_recenter: 'Head tracking: centre again', cp_microphone: 'Microphone',
  exit: 'Quit', sim_pause: 'Pause', open_menu: 'Open / close the main menu', screenshot: 'Screenshot', quicksave: 'Quicksave', voice_radio: 'Multiplayer: bus radio (hold)',
  radio_power: 'Radio: on / off', radio_next: 'Radio: next station', radio_previous: 'Radio: previous station', radio_volume_up: 'Radio: louder', radio_volume_down: 'Radio: quieter',
  toggel_mouse_ctrl: 'Toggle mouse steering', toggel_ctrler: 'Toggle game controllers',
};
const H_GEARS = ['R', '1', '2', '3', '4', '5', '6', '7', '8', '9', '10'].map((g) => `kw_s_${g}_fest`);
/** The actions a button is mostly given, by what they are for (openOMSI's launcher's groups). */
const GROUPS = [
  ['Driving', ['horn', 'parking_brake_toggle', 'parking_brake_set', 'parking_brake_release', 'kw_m_enginestart', 'cp_batterietrennschalter_toggle', 'throttle_amplify']],
  ['Gearbox', ['gear_up', 'gear_down', 'automatic_D', 'automatic_N', 'automatic_R', ...H_GEARS]],
  ['Lights and indicators', ['blinker_left_set', 'blinker_right_set', 'blinker_off', 'blinker_left_toggle', 'blinker_right_toggle', 'blinker_warn_toggle', 'kw_scheinwerfer_toggle', 'kw_fernlicht_toggle', 'kw_wipermode_up']],
  ['Doors and tickets', ['doors_all', 'door_1', 'door_2', 'door_3', 'door_4', 'doors_release', 'bus_doorfront0', 'bus_doorfront1', 'bus_dooraft', 'ticket_give', 'pax_door_request']],
  ['Views', ['view_look_left', 'view_look_right', 'view_look_up', 'view_look_down', 'view_reset_direction', 'view_reset_all_directions', 'view_toggle_viewpoint', 'view_toggle_interior', 'view_set_driver', 'view_set_passenger', 'view_set_outside', 'view_interiorcam_plus', 'view_interiorcam_minus', 'view_zoom_in', 'view_zoom_out', 'head_tracking_recenter']],
  ['The game', ['open_menu', 'sim_pause', 'screenshot', 'quicksave', 'voice_radio', 'toggel_ctrler', 'toggel_mouse_ctrl', 'radio_power', 'radio_next', 'radio_previous', 'radio_volume_up', 'radio_volume_down']],
];

/** A name as the file has it: the letters and digits only, in lower case (DirectInput's and the
 * webview's names of one device differ in spacing, case and a maker's name in front). */
const norm = (s) => (s || '').toLowerCase().replace(/[^\p{L}\p{N}]/gu, '');
/** One device under two names, as openOMSI matches them. */
export function namesMatch(a, b) {
  const [x, y] = [norm(a), norm(b)];
  return !!x && !!y && (x === y || x.includes(y) || y.includes(x));
}
const findBy = (list, name) => list.find((x) => x.name === name) || list.find((x) => namesMatch(x.name, name));

/** A device's name from the webview's id: without the vendor and product numbers it adds
 * ("G25 Racing Wheel (Vendor: 046d Product: c299)", Firefox's "046d-c299-G25 Racing Wheel"). */
export function padName(id) {
  const s = String(id || '')
    .replace(/^[0-9a-f]{1,4}-[0-9a-f]{1,4}-/i, '')
    .replace(/\s*\((?=[^()]*(?:Vendor:|Product:|STANDARD GAMEPAD|XInput))[^()]*\)\s*$/i, '')
    .trim();
  return s || String(id || '');
}

/** A button's name: its number from 1, or a hat's direction. */
export function buttonName(n) {
  if (n >= HATS) return tr('Hat %{n} %{dir}', { n: Math.floor((n - HATS) / 4) + 1, dir: tr(DIRS[(n - HATS) % 4]) });
  return tr('Button %{n}', { n: n + 1 });
}

function humanize(name) {
  const words = String(name)
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .split(/[_\-\s./\\]+/)
    .filter((w) => w && !['cp', 'kw', 'bus', 'toggle', 'mouse', 'lod'].includes(w.toLowerCase()));
  const s = words.join(' ') || String(name);
  return s[0].toUpperCase() + s.slice(1);
}

/** An action in words: Omsi-Hub's own, else OMSI 2's key assignment's (`names`), else its name. */
export function actionText(a, names = {}) {
  const gear = /^kw_s_(.+)_fest$/.exec(a);
  if (gear) return tr('Gear %{gear} (H-pattern)', { gear: gear[1] });
  if (KNOWN[a]) return tr(KNOWN[a]);
  return names[String(a).toLowerCase()] || humanize(a);
}

export const newDevice = (name) => ({ name, second: '0', axes: Array.from({ length: 8 }, () => [-1, 0]), buttons: [], ff_scale: null, ff_invert: null, latching: [], more: [] });
const actionsOf = (d, n) => [d.buttons[n]?.[0] || '', ...d.more.filter(([b]) => b === n).map(([, a]) => a)].filter(Boolean);

// --- the connected devices, as the webview sees them ---------------------------------------

/** The game controllers connected now, read 20 times a second: per device its name, whether the
 * webview knows it as a gamepad (`standard`), its axes (DirectInput's slot and -1 .. 1; a
 * gamepad's sticks), how many buttons and hats it has and which are pressed. `listeners` hear
 * every button that goes down (the device's name, the button). */
function usePads() {
  const [pads, setPads] = useState([]);
  const listeners = useRef(new Set());
  useEffect(() => {
    if (!navigator.getGamepads) return undefined;
    // (per device: the axes that are hats - a hat at rest reads past 1 - and the buttons down)
    const seen = new Map();
    let last = '';
    const tick = () => {
      const list = [];
      let pads = [];
      try {
        pads = [...(navigator.getGamepads() || [])];
      } catch {
        pads = [];
      }
      for (const gp of pads) {
        if (!gp || gp.connected === false) continue;
        const key = `${gp.index}:${gp.id}`;
        const name = padName(gp.id);
        const m = seen.get(key) || { hats: new Set(), down: new Set() };
        seen.set(key, m);
        const standard = gp.mapping === 'standard';
        if (!standard) gp.axes.forEach((v, k) => Math.abs(v) > 1.05 && m.hats.add(k));
        const hatAxes = [...m.hats].sort((a, b) => a - b);
        const axes = standard ? gp.axes.slice(0, 4).map((v, k) => [k, v]) : gp.axes.slice(0, 8).map((v, k) => [k, v]).filter(([k]) => !m.hats.has(k));
        const pressed = new Set();
        gp.buttons.forEach((b, n) => (b.pressed || b.value > 0.5) && pressed.add(n));
        hatAxes.forEach((k, h) => {
          const v = gp.axes[k];
          if (Math.abs(v) > 1.0001) return;
          // (eight places round from up, -1 .. 1 in sevenths)
          const at = Math.round(((v + 1) / 2) * 7);
          [[7, 0, 1], [1, 2, 3], [3, 4, 5], [5, 6, 7]].forEach((dirs, d) => dirs.includes(at) && pressed.add(HATS + h * 4 + d));
        });
        for (const n of pressed) if (!m.down.has(n)) listeners.current.forEach((f) => f(name, n));
        m.down = pressed;
        const triggers = standard ? [gp.buttons[6]?.value || 0, gp.buttons[7]?.value || 0] : null;
        list.push({ key, name, standard, axes, triggers, buttons: gp.buttons.length, hats: hatAxes.length, pressed: [...pressed] });
      }
      const sig = JSON.stringify(list.map((p) => [p.key, p.axes.map(([k, v]) => [k, Math.round(v * 200)]), p.triggers?.map((v) => Math.round(v * 100)), p.pressed, p.hats]));
      if (sig !== last) {
        last = sig;
        setPads(list);
      }
    };
    const id = setInterval(tick, 50);
    window.addEventListener('gamepadconnected', tick);
    window.addEventListener('gamepaddisconnected', tick);
    tick();
    return () => {
      clearInterval(id);
      window.removeEventListener('gamepadconnected', tick);
      window.removeEventListener('gamepaddisconnected', tick);
    };
  }, []);
  return [pads, listeners];
}

/** The axis moving most just now (its slot), for a moment: its line lights up. */
function useMoving(axes) {
  const prev = useRef(new Map());
  const [moving, setMoving] = useState(null);
  const timer = useRef(0);
  useEffect(() => {
    let best = null;
    let most = 0.04;
    for (const [k, v] of axes || []) {
      const d = Math.abs(v - (prev.current.get(k) ?? v));
      if (d > most) [best, most] = [k, d];
      prev.current.set(k, v);
    }
    if (best != null) {
      setMoving(best);
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setMoving(null), 700);
    }
  }, [axes]);
  useEffect(() => () => clearTimeout(timer.current), []);
  return moving;
}

/** A bar: from the middle (-1 .. 1) or filled from the left (0 .. 1). */
function Meter({ value, centre = true }) {
  const v = Math.max(-1, Math.min(1, Number(value) || 0));
  const style = centre ? (v >= 0 ? `left:50%;width:${v * 50}%` : `left:${50 + v * 50}%;width:${-v * 50}%`) : `left:0;width:${Math.max(0, v) * 100}%`;
  return html`<div class=${'pads-meter' + (centre ? ' centre' : '')}><i style=${style}></i></div>`;
}

/** The axis as the game takes it (its function, reversed, its curve; openOMSI's `axis_shape`). */
function gameValue(v, [f, flags]) {
  const x = flags & 1 ? -v : v;
  if (f === 5 || f === 6) return { centre: true, x };
  const shape = (u) => {
    if (flags & 2) u = u * 1.25 - 0.125;
    const bi = flags & 0x10;
    if (flags & 4) u = bi ? Math.sin((u - 0.5) * Math.PI) * 0.5 + 0.5 : Math.sin((u * Math.PI) / 2);
    else if (flags & 8) {
      if (bi) {
        const d = (u - 0.5) * 2;
        u = 0.5 + Math.sign(d) * d * d * 0.5;
      } else u = u * u;
    }
    return Math.min(1, Math.max(0, u));
  };
  const p = shape((x + 1) / 2);
  if (f === 0 || f === 4) return { centre: true, x: p * 2 - 1 };
  return { centre: false, x: p };
}

// --- the set-up assistant ---------------------------------------------------------------

const STEPS = [
  ['Let go of everything', 'Take your hands off the wheel and your feet off the pedals, the wheel in the middle; then press Next.'],
  ['Steering', 'Turn the wheel (or move the stick) all the way to the LEFT and hold it there; then press Next. No wheel (pedals only): Skip.'],
  ['Throttle', 'Press the throttle pedal all the way down and hold it; then press Next. No pedals: Skip.'],
  ['Brake', 'Press the brake pedal all the way down and hold it; then press Next. No brake pedal: Skip.'],
  ['Clutch', 'Press the clutch pedal all the way down and hold it; then press Next. No clutch: Skip.'],
];

/** The axis that moved most from `rest` to `now` (at least a sixth of its travel), none of
 * `exclude`: [slot, how far, signed]. */
function movedMost(rest, now, exclude) {
  let best = null;
  for (let k = 0; k < 8; k++) {
    if (exclude.includes(k) || now?.[k] == null) continue;
    const d = now[k] - (rest[k] ?? 0);
    if (Math.abs(d) > 0.33 && (!best || Math.abs(d) > Math.abs(best[1]))) best = [k, d];
  }
  return best;
}

/** What the assistant found: per slot [function, reversed] or null (openOMSI's `wizard_result`). */
export function wizardResult(rest, at) {
  const axes = Array(8).fill(null);
  const steer = at[0] && movedMost(rest, at[0], []);
  // (turned left the value falls: else the axis runs the other way)
  if (steer) axes[steer[0]] = [0, steer[1] > 0];
  const taken = steer ? [steer[0]] : [];
  const pedal = (i, ex) => at[i] && movedMost(rest, at[i], ex);
  const throttle = pedal(1, taken);
  const brake = pedal(2, taken);
  if (throttle && brake && throttle[0] === brake[0] && throttle[1] * brake[1] < 0) {
    // (both pedals on one axis: the throttle towards its top, else reversed)
    axes[throttle[0]] = [4, throttle[1] < 0];
  } else {
    // (a pedal pressed goes towards 1)
    if (throttle) axes[throttle[0]] = [1, throttle[1] < 0];
    if (brake && brake[0] !== throttle?.[0]) axes[brake[0]] = [2, brake[1] < 0];
  }
  const ex = [...taken, ...(throttle ? [throttle[0]] : []), ...(brake ? [brake[0]] : [])];
  const clutch = pedal(3, ex);
  if (clutch) axes[clutch[0]] = [3, clutch[1] < 0];
  return axes;
}

function Wizard({ name, live, onDone, onClose }) {
  const [step, setStep] = useState(0);
  const [rest, setRest] = useState(null);
  const [at, setAt] = useState([]);
  const [error, setError] = useState('');
  const now = () => {
    const a = Array(8).fill(null);
    (live?.axes || []).forEach(([k, v]) => (a[k] = v));
    return a;
  };
  // (Escape closes it, as the Cancel button)
  useEffect(() => {
    const k = (e) => e.key === 'Escape' && onClose();
    addEventListener('keydown', k);
    return () => removeEventListener('keydown', k);
  }, [onClose]);
  // (the axis moved most from where it rested: lit while it is held)
  const moving = rest && step > 0 ? movedMost(rest, now(), []) : null;
  const next = (skip) => {
    setError('');
    if (step === 0) {
      setRest(now());
      return setStep(1);
    }
    const here = skip ? null : now();
    if (here && !movedMost(rest, here, [])) return setError('Nothing moved far enough: hold it all the way, or skip this step.');
    const done = [...at];
    done[step - 1] = here;
    setAt(done);
    if (step < STEPS.length - 1) return setStep(step + 1);
    const axes = wizardResult(rest, done);
    if (!axes.some(Boolean)) return setError('Nothing was found: start again and hold each control all the way.');
    onDone(axes);
  };
  return html`<div class="pads-wizard" role="dialog">
    <div class="pads-card">
      <div class="pads-steps">${STEPS.map((s, k) => html`<span class=${k === step ? 'on' : k < step ? 'done' : ''}>${k + 1}. ${tr(s[0])}</span>`)}</div>
      <h3>${tr(STEPS[step][0])}</h3>
      <p class="muted">${tr(STEPS[step][1])}</p>
      ${!live && html`<p class="small warn">${tr('The device is not connected: plug it in to set it up.')}</p>`}
      <div class="pads-live">
        ${(live?.axes || []).map(([k, v]) => html`<div class=${'pads-live-axis' + (moving?.[0] === k ? ' moving' : '')}><span>${tr(AXES[k])}</span><${Meter} value=${v} /></div>`)}
      </div>
      ${error && html`<p class="small bad">${tr(error)}</p>`}
      <div class="row" style="gap:8px;justify-content:flex-end;margin-top:14px">
        <${Button} kind="ghost" label="Cancel" onClick=${onClose} />
        ${step > 0 && html`<${Button} label="Skip" onClick=${() => next(true)} />`}
        <${Button} kind="primary" icon="arrow_forward" label=${step === STEPS.length - 1 ? 'Done' : 'Next'} disabled=${!live} onClick=${() => next(false)} />
      </div>
    </div>
  </div>`;
}

// --- the page -------------------------------------------------------------------------------

/** The devices of the file and the connected ones, the one shown, its changes saved. */
export function ControllersPanel() {
  const [cfg, setCfg] = useState(null);
  const [error, setError] = useState('');
  const [game, setGameNow] = useState(null);
  // (the settings as last answered, for a change made before the page drew them)
  const gameRef = useRef(null);
  const setGame = (v) => {
    gameRef.current = v;
    setGameNow(v);
  };
  const offQueue = useRef(Promise.resolve());
  const [saving, setSaving] = useState('');
  const [sel, setSel] = useState(null);
  const [wizard, setWizard] = useState(null);
  const [lit, setLit] = useState(null);
  // (the line lit for four seconds: put out by a timer, not by whether the page happens to be
  // drawn again)
  const litTimer = useRef(0);
  useEffect(() => () => clearTimeout(litTimer.current), []);
  const [pads, listeners] = usePads();
  const devicesRef = useRef(null);
  const timer = useRef(0);
  const pending = useRef(Promise.resolve());

  const load = () =>
    call('addon_controllers').then(
      (v) => ((devicesRef.current = v.devices), setCfg(v), setError('')),
      (e) => setError(String(e?.message || e)),
    );
  const loadGame = () => call('addon_game_settings').then(setGame, () => {});
  useEffect(() => {
    load();
    loadGame();
    return () => {
      if (timer.current) flush();
    };
  }, []);

  function flush() {
    clearTimeout(timer.current);
    timer.current = 0;
    const devices = devicesRef.current;
    setSaving('saving');
    pending.current = pending.current.then(() =>
      call('addon_controllers_save', { devices }).then(
        (r) => (setSaving('saved'), setCfg((c) => c && { ...c, path: r.path, from: 'openomsi' })),
        (e) => (setSaving(''), toast(tr('Not saved: %{e}', { e: String(e?.message || e) }), 'error')),
      ),
    );
  }
  function setDevices(devices) {
    devicesRef.current = devices;
    setCfg((c) => ({ ...c, devices }));
    clearTimeout(timer.current);
    timer.current = setTimeout(flush, 400);
  }
  /** The device `name` changed by `f` (on a copy). */
  function change(name, f) {
    setDevices(
      (devicesRef.current || []).map((d) => {
        if (d.name !== name) return d;
        const c = JSON.parse(JSON.stringify(d));
        f(c);
        return c;
      }),
    );
  }
  function add(name) {
    if (!findBy(devicesRef.current || [], name)) setDevices([...(devicesRef.current || []), newDevice(name)]);
    setSel(name);
  }

  // (the devices switched off: openOMSI's setting `ctrl_off`)
  const offField = game?.groups.flatMap((g) => g.fields).find((f) => f.key === 'ctrl_off');
  const offs = (offField?.value || '').split('|').filter(Boolean);
  const isOff = (name) => offs.some((o) => namesMatch(o, name));
  // (one after the other, each from the answer before it: two devices switched off quickly
  // kept only the second)
  const setOn = (name, on) => {
    offQueue.current = offQueue.current.then(() => {
      const now = gameRef.current?.groups.flatMap((g) => g.fields).find((f) => f.key === 'ctrl_off');
      const rest = (now?.value || '').split('|').filter(Boolean).filter((o) => !namesMatch(o, name));
      return call('addon_game_setting', { key: 'ctrl_off', value: (on ? rest : [...rest, name]).join('|') }).then(setGame, (e) => toast(tr(String(e?.message || e)), 'error'));
    });
  };

  const devices = cfg?.devices || [];
  const list = [
    ...devices.map((d) => ({ name: d.name, cfg: d, live: findBy(pads.filter((p) => !p.standard), d.name) || null })),
    ...pads.filter((p) => !devices.some((d) => namesMatch(d.name, p.name))).map((p) => ({ name: p.name, cfg: null, live: p })),
  ];
  const entry = list.find((e) => e.name === sel) || list[0];

  // (a button pressed: its device shown and its line lit)
  useEffect(() => {
    const press = (name, n) => {
      if (wizard) return;
      const hit = list.find((e) => e.cfg && namesMatch(e.name, name)) || list.find((e) => namesMatch(e.name, name));
      if (hit) setSel(hit.name);
      setLit({ name, n });
      clearTimeout(litTimer.current);
      litTimer.current = setTimeout(() => setLit(null), 4000);
    };
    listeners.current.add(press);
    return () => listeners.current.delete(press);
  });

  if (error) {
    return html`<div class="setting-row full"><p class="small bad">${tr(error)}</p><div><${Button} small icon="refresh" label="Try again" onClick=${load} /></div></div>`;
  }
  if (!cfg) return html`<div class="setting-row full"><span class="muted small">${tr('Reading the controllers…')}</span></div>`;

  return html`<div class="pads">
    <p class="muted small pads-intro">${tr('Your steering wheel, pedals, joysticks, shifters and button boxes as openOMSI drives them. Move a control or press a button: it shows here. Every change is saved at once.')}</p>
    <div class="pads-main">
      <aside class="pads-list">
        <div class="hub-caps"><b>${tr('Devices')}</b>${saving && html`<span>${saving === 'saving' ? tr('Saving…') : html`<${Icon} name="check" size=${14} /> ${tr('Saved')}`}</span>`}</div>
        ${list.length
          ? list.map((e) => html`<${DeviceRow} e=${e} on=${e === entry} off=${isOff(e.name)} onClick=${() => setSel(e.name)} />`)
          : html`<p class="muted small">${tr('No game controller is connected, and none is set up yet. Connect a wheel, pedals or a joystick and move it or press one of its buttons: it shows up here. A gamepad works without setting up.')}</p>`}
        <div class="pads-file">
          ${cfg.path
            ? html`<a href="#" class="hub-path small" title=${cfg.path} onClick=${(ev) => (ev.preventDefault(), openPath(cfg.path))}><${Icon} name="description" size=${14} />${cfg.path}</a>`
            : html`<span class="muted small">${tr('No gamectrler.cfg yet: it is made with the first change.')}</span>`}
          ${cfg.from === 'omsi2' && html`<p class="muted small">${tr("These come from OMSI 2's own file. Your changes are saved in openOMSI's own; OMSI 2's stays as it is.")}</p>`}
          ${cfg.error && html`<p class="small bad">${tr(cfg.error)}</p>`}
        </div>
      </aside>
      <div class="pads-detail">
        ${entry &&
        (entry.cfg
          ? html`<${DeviceDetail}
              key=${entry.cfg.name}
              e=${entry}
              names=${cfg.names || {}}
              actions=${cfg.actions || []}
              off=${isOff(entry.name)}
              ffInvert=${game?.groups.flatMap((g) => g.fields).find((f) => f.key === 'ff_invert')?.value === '1'}
              lit=${lit && namesMatch(lit.name, entry.name) ? lit.n : null}
              change=${(f) => change(entry.cfg.name, f)}
              setOn=${(on) => setOn(entry.cfg.name, on)}
              setUp=${() => setWizard(entry.cfg.name)}
              remove=${() => (setDevices(devicesRef.current.filter((d) => d.name !== entry.cfg.name)), setSel(null))}
            />`
          : entry.live.standard
            ? html`<${PadDetail} e=${entry} off=${isOff(entry.name)} setOn=${(on) => setOn(entry.name, on)} />`
            : html`<${NewDevice} e=${entry} off=${isOff(entry.name)} setOn=${(on) => setOn(entry.name, on)} add=${() => add(entry.name)} setUp=${() => setWizard(entry.name)} />`)}
      </div>
    </div>
    <section class="pads-settings">
      <h4 class="hub-caps"><b>${tr('For every device')}</b></h4>
      <${GameSettingsPanel} controllers hide=${['ctrl_off']} onChange=${setGame} />
    </section>
    ${wizard &&
    html`<${Wizard}
      name=${wizard}
      live=${findBy(pads.filter((p) => !p.standard), wizard)}
      onClose=${() => setWizard(null)}
      onDone=${(axes) => {
        if (!findBy(devicesRef.current || [], wizard)) devicesRef.current = [...(devicesRef.current || []), newDevice(wizard)];
        const name = findBy(devicesRef.current, wizard).name;
        change(name, (c) => {
          c.axes = c.axes.map(([, flags], k) => (axes[k] ? [axes[k][0], (flags & ~1) | (axes[k][1] ? 1 : 0)] : [-1, flags & ~1]));
        });
        setSel(name);
        setWizard(null);
        toast(tr('Set up: check the axes below, and give the buttons what they do.'), 'ok');
      }}
    />`}
  </div>`;
}

function kindOf(e) {
  if (e.live?.standard) return 'pad';
  if (e.cfg && e.cfg.axes.every(([f]) => f < 0) && e.cfg.buttons.some(([a]) => a)) return 'buttons';
  if (e.live && !e.live.axes.length) return 'buttons';
  return 'wheel';
}

function DeviceRow({ e, on, off, onClick }) {
  const kind = kindOf(e);
  const [dot, text] = off ? ['', 'Switched off'] : e.cfg ? (e.live ? ['ok', 'Connected'] : ['', 'Not connected']) : kind === 'pad' ? ['ok', 'Ready to drive'] : ['accent', 'Not set up yet'];
  return html`<button type="button" class=${'pads-device' + (on ? ' on' : '') + (e.live ? '' : ' away')} title=${e.name} onClick=${onClick}>
    <${Icon} name=${kind === 'pad' ? 'sports_esports' : kind === 'buttons' ? 'keyboard' : 'tune'} size=${20} />
    <span class="what"><b>${e.name}</b><span><i class=${'dot ' + dot}></i>${tr(text)}</span></span>
  </button>`;
}

/** The device's head: its name, connected or not, on or off, set up again, taken out. */
function DeviceHead({ name, e, off, setOn, setUp, remove }) {
  const [sure, setSure] = useState(false);
  useEffect(() => {
    if (!sure) return undefined;
    const t = setTimeout(() => setSure(false), 4000);
    return () => clearTimeout(t);
  }, [sure]);
  return html`<div class="pads-head">
    <div class="grow" style="min-width:0">
      <h3>${name}</h3>
      <span class=${'pads-tag' + (off ? '' : e.live ? ' ok' : '')}>${tr(off ? 'Switched off' : e.live ? 'Connected' : 'Not connected')}</span>
    </div>
    <label class="hub-switch pads-use"><span>${tr('Use this device')}</span><input type="checkbox" checked=${!off} onChange=${(ev) => setOn(ev.currentTarget.checked)} /><i></i></label>
    ${setUp && html`<${Button} small icon="touch_app" label="Set up step by step" disabled=${!e.live || !e.live.axes.length} title=${e.live ? undefined : 'Not connected'} onClick=${setUp} />`}
    ${remove && html`<${Button} small kind=${sure ? 'danger' : 'ghost'} icon="delete" label=${sure ? 'Really remove?' : ''} title="Remove this device (its axes and buttons are forgotten)" onClick=${() => (sure ? remove() : setSure(true))} />`}
  </div>`;
}

function NewDevice({ e, off, setOn, add, setUp }) {
  const wheel = e.live.axes.length > 1;
  return html`<div class="pads-card">
    <${DeviceHead} name=${e.name} e=${e} off=${off} setOn=${setOn} />
    <p class="muted">${tr(wheel ? 'Not set up yet: the assistant finds the wheel and the pedals in a few steps. Until then openOMSI steers with its X axis.' : 'Not set up yet: give its buttons what they do.')}</p>
    <${LiveAxes} live=${e.live} />
    <div class="row wrap" style="gap:8px;margin-top:12px">
      ${wheel && html`<${Button} kind="primary" icon="touch_app" label="Set up step by step" onClick=${setUp} />`}
      <${Button} kind=${wheel ? '' : 'primary'} icon="add" label="Set up its buttons only" onClick=${add} />
    </div>
  </div>`;
}

function LiveAxes({ live }) {
  const moving = useMoving(live?.axes);
  if (!live?.axes?.length) return null;
  return html`<div class="pads-live">
    ${live.axes.map(([k, v]) => html`<div class=${'pads-live-axis' + (moving === k ? ' moving' : '')}><span>${tr(AXES[k])}</span><${Meter} value=${v} /></div>`)}
  </div>`;
}

/** A gamepad: openOMSI drives it as it is (the sticks, the triggers, its default buttons). */
function PadDetail({ e, off, setOn }) {
  const [lx, ly, rx, ry] = [0, 1, 2, 3].map((k) => e.live.axes.find(([a]) => a === k)?.[1] ?? 0);
  const rows = [['Left stick: steering', lx, true], ['Left stick: up and down', ly, true], ['Right stick: looking round', rx, true], ['Right stick: up and down', ry, true], ['Right trigger: throttle', e.live.triggers?.[1] ?? 0, false], ['Left trigger: brake', e.live.triggers?.[0] ?? 0, false]];
  return html`<div class="pads-card">
    <${DeviceHead} name=${e.name} e=${e} off=${off} setOn=${setOn} />
    <p class="muted">${tr('A gamepad needs no setting up: the left stick steers, the right trigger is the throttle and the left one the brake. With "Default gamepad buttons" on (below), its buttons work the indicators, the doors, the views, the gears, the menu and the pause.')}</p>
    <div class="pads-live">${rows.map(([label, v, centre]) => html`<div class="pads-live-axis"><span>${tr(label)}</span><${Meter} value=${v} centre=${centre} /></div>`)}</div>
  </div>`;
}

function DeviceDetail({ e, names, actions, off, ffInvert, lit, change, setOn, setUp, remove }) {
  const d = e.cfg;
  const kind = kindOf(e);
  const steers = d.axes.some(([f]) => f === 0);
  const ff = steers || d.ff_invert != null || (d.ff_scale || [1, 1]).some((v) => v !== 1);
  return html`
    <div class="pads-card">
      <${DeviceHead} name=${d.name} e=${e} off=${off} setOn=${setOn} setUp=${kind !== 'buttons' ? setUp : null} remove=${remove} />
      ${!e.live && html`<p class="muted small">${tr('Not connected: plug it in to see it answer. What it does is kept.')}</p>`}
    </div>
    ${kind !== 'buttons' && html`<${AxesCard} d=${d} live=${e.live} change=${change} setUp=${setUp} />`}
    ${ff && html`<${FeedbackCard} d=${d} ffInvert=${ffInvert} change=${change} />`}
    <${ButtonsCard} d=${d} live=${e.live} names=${names} actions=${actions} lit=${lit} change=${change} />
  `;
}

function AxesCard({ d, live, change, setUp }) {
  const liveOf = (k) => live?.axes?.find(([a]) => a === k)?.[1];
  const moving = useMoving(live?.axes);
  // (a connected device: the axes it has and the ones set up; else every axis set up)
  const slots = [...Array(8).keys()].filter((k) => d.axes[k][0] >= 0 || liveOf(k) != null);
  const has = (f) => d.axes.findIndex(([x]) => x === f || (x === 4 && (f === 1 || f === 2)));
  return html`<div class="pads-card">
    <div class="hub-caps"><b>${tr('Axes and pedals')}</b><span>${tr('Move a control: its line lights up.')}</span></div>
    <div class="pads-chips">
      ${[[0, 'Steering'], [1, 'Throttle'], [2, 'Brake'], [3, 'Clutch']].map(([f, label]) => {
        const k = has(f);
        return html`<span class=${'pads-chip' + (k >= 0 ? ' set' : '')}>${tr(label)}<b>${k >= 0 ? tr(AXES[k]) : tr('Not used')}</b></span>`;
      })}
    </div>
    ${has(0) < 0 && html`<p class="small warn row" style="gap:10px">${tr('No axis steers yet.')}${live && html`<${Button} small kind="primary" icon="touch_app" label="Set up step by step" onClick=${setUp} />`}</p>`}
    ${!slots.length && html`<p class="muted small">${tr('No axis is set up; plug the device in to see its axes.')}</p>`}
    <div class="pads-axes">
      ${slots.length > 0 && html`<div class="pads-axis head"><span></span><span></span><span></span><span>${tr('Reversed')}</span><span>${tr('Response curve')}</span></div>`}
      ${slots.map((k) => {
        const a = d.axes[k];
        const raw = liveOf(k);
        const g = raw != null && a[0] >= 0 ? gameValue(raw, a) : null;
        const fits = a[0] === 0 || a[0] === 4 ? SHAPES.filter(([v]) => v === 0 || v & 0x10) : SHAPES;
        const curves = fits.some(([v]) => v === (a[1] & CURVE)) ? fits : [...fits, ...SHAPES.filter(([v]) => v === (a[1] & CURVE))];
        return html`<div class=${'pads-axis' + (moving === k ? ' moving' : '') + (a[0] < 0 ? ' unused' : '')}>
          <span class="name">${tr(AXES[k])}</span>
          <${Meter} value=${g ? g.x : raw ?? 0} centre=${g ? g.centre : true} />
          <${Select} value=${a[0]} options=${FUNCS} onChange=${(v) => change((c) => (c.axes[k][0] = Number(v)))} />
          <span>${a[0] >= 0 && html`<label class="hub-switch bare"><input type="checkbox" checked=${!!(a[1] & 1)} onChange=${(ev) => change((c) => (c.axes[k][1] = (c.axes[k][1] & ~1) | (ev.currentTarget.checked ? 1 : 0)))} /><i></i></label>`}</span>
          <span>${a[0] >= 0 && a[0] < 5 && html`<${Select} value=${a[1] & CURVE} options=${curves} onChange=${(v) => change((c) => (c.axes[k][1] = (c.axes[k][1] & ~CURVE) | Number(v)))} />`}</span>
        </div>`;
      })}
    </div>
  </div>`;
}

const pct = (v) => `${Math.round(v * 100)} %`;

function FeedbackCard({ d, ffInvert, change }) {
  const [steering, vibration] = d.ff_scale || [1, 1];
  const set = (i, v) => change((c) => {
    const s = c.ff_scale ? [...c.ff_scale] : [1, 1];
    s[i] = Math.round(v * 100) / 100;
    c.ff_scale = s;
  });
  const dirs = [['', ffInvert ? 'As the setting says (the other way)' : 'As the setting says (normal)'], ['0', 'Normal'], ['1', 'The other way']];
  return html`<div class="pads-card">
    <div class="hub-caps"><b>${tr('Force feedback of this wheel')}</b></div>
    <div class="setting-row full"><${Slider} label="Steering force (centring and resistance)" value=${steering} min=${0} max=${2} step=${0.05} format=${pct} onChange=${(v) => set(0, v)} /></div>
    <div class="setting-row full"><${Slider} label="Vibration" value=${vibration} min=${0} max=${2} step=${0.05} format=${pct} onChange=${(v) => set(1, v)} /></div>
    <div class="setting-row"><span class="name">${tr("The wheel's motor")}</span>
      <${Select} value=${d.ff_invert == null ? '' : d.ff_invert ? '1' : '0'} options=${dirs} onChange=${(v) => change((c) => (c.ff_invert = v === '' ? null : v === '1'))} /></div>
  </div>`;
}

/** The actions to choose from, by group (`Other`: the rest of `keyboard.cfg`'s and those set). */
function actionGroups(actions, d, names) {
  const grouped = new Set(GROUPS.flatMap(([, a]) => a.map((x) => x.toLowerCase())));
  const rest = new Map();
  for (const a of [...actions, ...d.buttons.map(([x]) => x), ...d.more.map(([, x]) => x)]) {
    if (a && !grouped.has(a.toLowerCase()) && !rest.has(a.toLowerCase())) rest.set(a.toLowerCase(), a);
  }
  const other = [...rest.values()].sort((x, y) => actionText(x, names).localeCompare(actionText(y, names)));
  return [...GROUPS, ['Other', other]];
}

/** A text as the actions' search reads it: lower case, without its accents. */
const fold = (s) => String(s).toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '');

/** The action chosen (`value` '' : none), and a list of the actions by group to search for one:
 * every word typed is in its name as the page shows it, OMSI 2's own or its group's. The arrows
 * and Enter pick, Escape or a click beside it closes it (`onClose`: closed with none picked);
 * `open` opens it at once. */
function ActionSelect({ value, groups, names, onChange, placeholder = 'Nothing', open: first = false, onClose }) {
  const [open, setOpen] = useState(first);
  const [q, setQ] = useState('');
  const [hi, setHi] = useState(-1);
  const box = useRef(null);
  const field = useRef(null);
  const list = useRef(null);
  const words = fold(q).split(/\s+/).filter(Boolean);
  const found = groups
    .map(([title, l]) => [title, l.filter((a) => ((hay) => words.every((w) => hay.includes(w)))(fold(`${actionText(a, names)} ${a} ${tr(title)}`)))])
    .filter(([, l]) => l.length > 0);
  // (with no words typed, "Nothing" first: the action taken off)
  const choices = [...(value && !words.length ? [''] : []), ...found.flatMap(([, l]) => l)];
  const close = () => {
    setOpen(false);
    setQ('');
    onClose?.();
  };
  const pick = (a) => {
    setOpen(false);
    setQ('');
    if (a !== value) onChange(a);
    else onClose?.();
  };
  useEffect(() => {
    if (!open) return undefined;
    field.current?.focus();
    setHi(Math.max(0, choices.indexOf(value)));
    const away = (ev) => box.current && !box.current.contains(ev.target) && close();
    document.addEventListener('mousedown', away);
    return () => document.removeEventListener('mousedown', away);
  }, [open]);
  useEffect(() => {
    // (the first: from the top, its group's title in sight)
    if (hi <= 0 && list.current) list.current.scrollTop = 0;
    else list.current?.querySelector('.hi')?.scrollIntoView({ block: 'nearest' });
  }, [hi, open]);
  const key = (ev) => {
    if (ev.key === 'Escape') {
      ev.stopPropagation();
      close();
    } else if (ev.key === 'ArrowDown' || ev.key === 'ArrowUp') {
      ev.preventDefault();
      if (choices.length) setHi((h) => (h + (ev.key === 'ArrowDown' ? 1 : choices.length - 1)) % choices.length);
    } else if (ev.key === 'Enter' && choices[hi] != null) {
      ev.preventDefault();
      pick(choices[hi]);
    }
  };
  const option = (a, k, text) =>
    html`<button type="button" class=${'pads-option' + (k === hi ? ' hi' : '') + (a === value ? ' on' : '')} onMouseMove=${() => k !== hi && setHi(k)} onClick=${() => pick(a)}>${text}</button>`;
  let k = choices[0] === '' ? 1 : 0;
  return html`<div class=${'pads-pick' + (open ? ' open' : '')} ref=${box}>
    ${open
      ? html`<label class="search"><${Icon} name="search" size=${16} /><input ref=${field} class="input" type="search" spellcheck="false" value=${q} placeholder=${tr('Search for an action…')} onInput=${(ev) => (setQ(ev.currentTarget.value), setHi(0))} onKeyDown=${key} /></label>
          <div class="pads-found" ref=${list}>
            ${choices[0] === '' && option('', 0, tr(placeholder))}
            ${found.map(([title, l]) => html`<div class="pads-group">${tr(title)}</div>${l.map((a) => option(a, k++, actionText(a, names)))}`)}
            ${!choices.length && html`<p class="muted small">${tr('No action has these words.')}</p>`}
          </div>`
      : html`<button type="button" class=${'select pads-action' + (value ? '' : ' none')} onClick=${() => setOpen(true)}>${value ? actionText(value, names) : tr(placeholder)}</button>`}
  </div>`;
}

function ButtonsCard({ d, live, names, actions, lit, change }) {
  const [all, setAll] = useState(false);
  const [adding, setAdding] = useState(null);
  // (a button pressed stays listed once its light is out: its action may still be being chosen)
  const [touched, setTouched] = useState(() => new Set());
  useEffect(() => {
    if (lit != null) setTouched((t) => (t.has(lit) ? t : new Set(t).add(lit)));
  }, [lit]);
  const groups = actionGroups(actions, d, names);
  const count = live ? live.buttons : 0;
  const hats = live ? live.hats : 0;
  const bound = new Set();
  d.buttons.forEach(([a], n) => a && bound.add(n));
  d.more.forEach(([n]) => bound.add(n));
  d.latching.forEach((n) => bound.add(n));
  d.buttons.forEach(([, c], n) => c === '1' && bound.add(n));
  if (lit != null) bound.add(lit);
  touched.forEach((n) => bound.add(n));
  if (all) {
    for (let n = 0; n < count; n++) bound.add(n);
    for (let n = 0; n < hats * 4; n++) bound.add(HATS + n);
  }
  const shown = [...bound].sort((a, b) => a - b);
  const pressed = new Set(live?.pressed || []);
  useEffect(() => {
    if (lit != null) document.getElementById(`pads-b-${lit}`)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }, [lit]);

  const pad = (c, n) => {
    while (c.buttons.length <= n) c.buttons.push(['', '0']);
  };
  const bind = (n, action) =>
    change((c) => {
      pad(c, n);
      let next = action;
      if (!next) {
        // (the first action taken off: the next one moves up)
        const i = c.more.findIndex(([b]) => b === n);
        if (i >= 0) {
          next = c.more[i][1];
          c.more.splice(i, 1);
        }
      } else c.more = c.more.filter(([b, a]) => b !== n || a.toLowerCase() !== action.toLowerCase());
      c.buttons[n] = [next, c.buttons[n][1] || '0'];
      if (!next) c.latching = c.latching.filter((b) => b !== n);
    });
  const assign = (n, action) => {
    setAdding(null);
    if (!action) return;
    const has = actionsOf(d, n);
    if (!has.length) return bind(n, action);
    if (has.some((a) => a.toLowerCase() === action.toLowerCase())) return;
    change((c) => c.more.push([n, action]));
  };
  const unbind = (n, action) => {
    if ((d.buttons[n]?.[0] || '').toLowerCase() === action.toLowerCase()) return bind(n, '');
    change((c) => (c.more = c.more.filter(([b, a]) => b !== n || a.toLowerCase() !== action.toLowerCase())));
  };
  const flag = (n, which, on) =>
    change((c) => {
      pad(c, n);
      if (which === 'hold') c.buttons[n][1] = on ? '1' : '0';
      else c.latching = on ? [...new Set([...c.latching, n])].sort((a, b) => a - b) : c.latching.filter((b) => b !== n);
    });

  return html`<div class="pads-card">
    <div class="hub-caps"><b>${tr('Buttons')}</b><span>${tr('Press a button on the device: its line lights up.')}</span></div>
    ${(count > 0 || hats > 0) && html`<label class="hub-switch pads-all"><span>${tr('Every button of the device (%{n})', { n: count + hats * 4 })}</span><input type="checkbox" checked=${all} onChange=${(ev) => setAll(ev.currentTarget.checked)} /><i></i></label>`}
    ${!shown.length && html`<p class="muted small">${tr(live ? 'No button does anything yet. Press one to give it an action.' : 'No button does anything yet. Plug the device in and press a button to give it an action.')}</p>`}
    <div class="pads-buttons">
      ${shown.map((n) => {
        const acts = actionsOf(d, n);
        const hold = d.buttons[n]?.[1] === '1';
        const latch = d.latching.includes(n);
        return html`<div id=${`pads-b-${n}`} class=${'pads-button' + (lit === n ? ' lit' : '') + (pressed.has(n) ? ' down' : '')}>
          <span class="kbd">${buttonName(n)}</span>
          <div class="acts">
            <${ActionSelect} value=${acts[0] || ''} groups=${groups} names=${names} onChange=${(v) => bind(n, v)} />
            ${acts.slice(1).map((a) => html`<span class="pads-also">+ ${actionText(a, names)}<button type="button" title=${tr('Remove')} onClick=${() => unbind(n, a)}><${Icon} name="close" size=${14} /></button></span>`)}
            ${adding === n
              ? html`<${ActionSelect} value="" groups=${groups} names=${names} placeholder="Another action…" open onChange=${(v) => assign(n, v)} onClose=${() => setAdding(null)} />`
              : acts.length > 0 && html`<button type="button" class="pads-more" title=${tr('This button does another action as well')} onClick=${() => setAdding(n)}><${Icon} name="add" size=${16} /></button>`}
          </div>
          <span class="flags">
            <button type="button" class=${'pads-flag' + (hold ? ' on' : '')} title=${tr('The action is held for as long as the button is (OMSI: continuous)')} onClick=${() => flag(n, 'hold', !hold)}>${tr('Held')}</button>
            <button type="button" class=${'pads-flag' + (latch ? ' on' : '')} title=${tr('A latching switch (an indicator stalk, a lit hazard button): when it comes out the action is switched back')} onClick=${() => flag(n, 'latch', !latch)}>${tr('Latching')}</button>
          </span>
        </div>`;
      })}
    </div>
  </div>`;
}
