// The depot in 3D on the depot page: the yard from `scene.js` drawn by `renderer.js`, its
// labels floating over the buildings, the buses to pick (clicked: picked for the workshop's
// list, clicked again: let go), the modules to move or pull down (clicked: their card). In the
// build mode (`placing`: a module of an area to put down, or one to move) its ghost follows the
// pointer on the yard's grid, green where it fits (`yard.js`), R turns it, a click puts it down,
// Escape or a right click stops. Drag to turn the yard round (in 2D: to move it), right-drag (or
// Shift) to move it,
// scroll to zoom; the camera is kept for when the page comes back. Each bus is drawn as its own
// model in its livery once that is there (`skins.js`), until then as the plain one. The staff at
// work walk about the yard (`people.js`, their figures `figures.js`); with the animations off
// they stand still. A bus that goes out on its tours drives out of the gate, one back from them
// drives in through the wash lane to its space, one going to the workshop drives there
// (`drive.js`: each change of where a bus is, once the page saw it before). How finely it is
// drawn is chosen in its menu (`quality.js`). Without WebGL2
// the page draws its plan from above instead (`onFail`).

import { html, useEffect, useRef, useState, tr, Icon, IconBtn, Btn, ui, setUi } from '../kit.js';
import { actMore } from '../more.js';
import { project } from '../../hub/bus3d/math.js';
import { buildScene, AREAS, moduleUnder, ghostMesh, ghostHeight, gridMesh } from './scene.js';
import { rectOf, apronsOf, whyNot } from './yard.js';
import { busMesh, instances, pickBus, skinned } from './bus.js';
import { skin, skinKey } from './skins.js';
import { personMesh, people, rolesOf, rounds } from './people.js';
import { DepotRenderer, VIEW, ZOOM } from './renderer.js';
import { palette, rgb01 } from './palette.js';
import { LEVELS, chooseQuality, chosenQuality, levelOf, stepDown } from './quality.js';
import { figure } from './figures.js';
import { grid, route, at as along, length } from './drive.js';

/** The menu's levels: what is chosen, its name, and what it gives up. */
const QUALITY = [
  ['auto', 'Automatic', ''],
  ['high', 'High', 'Shadows, smooth edges, full sharpness'],
  ['medium', 'Medium', 'Softer shadows, a little less sharp'],
  ['low', 'Low', 'No shadows or trees: for slower computers'],
];
const NAMES = { high: 'Now: high', medium: 'Now: medium', low: 'Now: low' };
/** Why a module does not fit, in words (as `hub_core::company::yard::fits` says it). */
const WHY = {
  land: 'It does not fit on your land here: put it elsewhere or buy more land.',
  gate: 'It would block the way in from the gate.',
  taken: 'Something stands there already.',
  lane: 'Keep the lanes in front of the buildings and spaces free.',
};
const GREEN = [0.2, 0.78, 0.42, 0.7];
const RED = [0.92, 0.26, 0.22, 0.7];

/** The areas' icons, as the Build list has them (`depot::Area::icon`), for an area it does not
 * list. */
const ICONS = { parking: 'local_parking', workshop: 'construction', wash: 'water_drop', fuel: 'inventory_2', charging: 'bolt', offices: 'groups' };

const still = () => document.documentElement.classList.contains('hub-still');

export function Depot3D({ p, names, w, h, selected, hover, onPick, onFail, placing, onPlaced, flat }) {
  const canvas = useRef(null);
  const tags = useRef(null);
  const r = useRef(null);
  const scene = useRef(null);
  const drag = useRef(null);
  const [under, setUnder] = useState(null);
  const [quality, setQuality] = useState(chosenQuality);
  const [level, setLevel] = useState(() => levelOf(chosenQuality()));
  const [menu, setMenu] = useState(false);
  const [, setBuilt] = useState(0);
  const [ghost, setGhost] = useState(null);
  const [rot, setRot] = useState(0);
  const [sel, setSel] = useState(null);
  const [sure, setSure] = useState(false);
  const pointer = useRef(null);
  // the buses driving (id -> { moments, t0, kind }), where each was last to stand (id -> its
  // place, null: out on its tours), the yard's modules they last stood among, the buses drawn
  const drives = useRef(new Map());
  const where = useRef(null);
  const yardKey = useRef('');
  const shown = useRef([]);
  const loop = useRef(0);
  const latest = useRef(null);
  const [skins, setSkins] = useState(0);
  const crowd = useRef(0);
  const auto = useRef(quality === 'auto');
  auto.current = quality === 'auto';
  const pal = palette(p.look);
  const key = JSON.stringify([p, pal.dark, pal.accent]);

  // the renderer, on a canvas of its own for each level (smoothing is the context's); its
  // context let go when the page goes
  useEffect(() => {
    const el = canvas.current;
    const slow = () => auto.current && level !== 'low' && setLevel(stepDown(level));
    const view = new DepotRenderer(el, LEVELS[level], { onFrame: (vp, cw, ch) => placeTags(vp, cw, ch), onSlow: slow, onLost: () => onFail?.() });
    if (!view.ok) {
      onFail?.();
      return undefined;
    }
    r.current = view;
    view.setBusModel(busMesh(pal));
    if (LEVELS[level].staff) view.setPersonModel(personMesh());
    view.resize(w, h);
    const kept = ui('depot').cam;
    view.aim(kept || VIEW, true);
    // scrolling zooms towards the point under the pointer
    const wheel = (e) => {
      e.preventDefault();
      const g = view.goal;
      const zoom = Math.max(ZOOM[0], Math.min(ZOOM[1], g.zoom * Math.exp(e.deltaY * 0.0012)));
      const b = el.getBoundingClientRect();
      const hit = view.ground(e.clientX - b.left, e.clientY - b.top);
      const f = 1 - zoom / g.zoom;
      view.aim(hit ? { zoom, tx: g.tx + (hit[0] - g.tx) * f, tz: g.tz + (hit[1] - g.tz) * f } : { zoom }, still());
      keep();
    };
    el.addEventListener('wheel', wheel, { passive: false });
    return () => {
      el.removeEventListener('wheel', wheel);
      view.destroy();
      r.current = null;
    };
  }, [level]);

  useEffect(() => r.current?.resize(w, h), [w, h]);

  // the yard again when the depot (or the look) changed
  useEffect(() => {
    const view = r.current;
    if (!view) return;
    const s = buildScene(p, pal, LEVELS[level]);
    scene.current = s;
    view.setYard(s.mesh, s.bounds, pal, s.world);
    view.setDecals(s.decals);
    drive(s);
    buses(true);
    setBuilt((n) => n + 1);
  }, [key, level]);

  useEffect(() => () => cancelAnimationFrame(loop.current), []);

  /** Each bus that is to stand somewhere else than it did (or is out on its tours now, or back
   * from them) sent on its way there: a few seconds after each other, at most a dozen at once
   * (the others there at once). Not when the page first shows the yard, its modules moved, or
   * the animations off. */
  function drive(s) {
    const L = p.layout;
    const lay = JSON.stringify(L.modules.map((o) => [o.id, o.x, o.y, o.rot, o.building]));
    const snap = where.current === null || lay !== yardKey.current || still();
    yardKey.current = lay;
    const now = performance.now() / 1000;
    const before = where.current || new Map();
    const next = new Map();
    const g = snap ? null : grid(L);
    const same = (a, b) => (a === null || b === null ? a === b : Math.hypot(a.x - b.x, a.z - b.z) < 0.5 && Math.abs(Math.sin((a.heading - b.heading) / 2)) < 0.05);
    let started = 0;
    for (const b of s.buses) {
      const to = b.out ? null : { x: b.x, z: b.z, heading: b.heading };
      next.set(b.id, to);
      const had = before.get(b.id);
      if (snap) {
        drives.current.delete(b.id);
        continue;
      }
      if (had !== undefined && same(had, to)) continue;
      // (from where it is: on its way, or where it stood)
      const d = drives.current.get(b.id);
      const on = d ? poseOf(d, now) : undefined;
      const from = on !== undefined ? on : had === undefined ? null : had;
      drives.current.delete(b.id);
      if (!from && !to) continue;
      if (started >= 12) continue;
      const kind = !to ? 'leave' : !from ? 'arrive' : 'move';
      const moments = route(g, from || to, to, kind);
      if (moments.length < 2) continue;
      drives.current.set(b.id, { moments, t0: now + started * 2.5, kind });
      started++;
    }
    for (const id of [...drives.current.keys()]) if (!next.has(id)) drives.current.delete(id);
    where.current = next;
    if (drives.current.size) run();
  }

  /** Where a bus on its drive is now (null: not on the yard yet; undefined: the drive done). */
  function poseOf(d, now) {
    const t = now - d.t0;
    if (t < 0 && d.kind === 'arrive') return null;
    if (t >= length(d.moments)) return undefined;
    return along(d.moments, Math.max(0, t));
  }

  /** The drives shown, a picture each, while there are any; the shadows now and then. */
  function run() {
    if (loop.current) return;
    let shadow = 0;
    const tick = (ms) => {
      const more = drives.current.size > 0;
      const moved = !more || ms - shadow > 150;
      if (moved) shadow = ms;
      latest.current?.(moved);
      loop.current = more ? requestAnimationFrame(tick) : 0;
    };
    loop.current = requestAnimationFrame(tick);
  }

  // the people at work: their rounds from the yard and the staff, walked about some thirty
  // times a second while the page shows (standing still when the animations are off)
  useEffect(() => {
    const view = r.current;
    const s = scene.current;
    crowd.current = 0;
    if (!view || !s || !LEVELS[level].staff) return undefined;
    const list = rounds(s, p.staff, p.colour);
    crowd.current = list.length;
    if (!list.length) return undefined;
    const show = (t) => {
      const { rest, figures } = people(list, t, (role) => view.figureModel(role));
      view.setPeople(rest, t);
      view.setFigures(figures);
    };
    // each role's figure asked for: drawn from the next picture on (at once when they stand)
    let gone = false;
    for (const role of rolesOf(list))
      figure(role).then((m) => {
        if (!m || gone || r.current !== view) return;
        view.setFigureModel(role, m);
        if (still()) show(0);
      });
    let frame = 0;
    let last = 0;
    const t0 = performance.now();
    const tick = (now) => {
      frame = requestAnimationFrame(tick);
      if (now - last < 32) return;
      last = now;
      const t = (now - t0) / 1000;
      show(t);
    };
    if (still()) show(0);
    else frame = requestAnimationFrame(tick);
    return () => {
      gone = true;
      cancelAnimationFrame(frame);
    };
  }, [key, level, skins]);

  // the buses' own models, as they come
  useEffect(() => buses(true), [skins]);

  // (the labels drawn again: a picture puts them where they belong)
  useEffect(() => r.current?.invalidate());

  // the picked bus and the one under the pointer
  useEffect(() => buses(false), [selected, under]);

  /** The buses to the renderer: each as its own model where that is there (asked for where it
   * is not: `skins.js`), else as the plain one. */
  function buses(moved) {
    const view = r.current;
    const s = scene.current;
    if (!view || !s) return;
    const come = () => setSkins((n) => n + 1);
    const own = new Map();
    for (const b of s.buses) {
      const k = skin(b.file, b.paint, come);
      if (!k) continue;
      const key = skinKey(b.file, b.paint);
      view.setSkin(key, k.model, k.image);
      // (its own length and height: the ring and the pointer go by them)
      b.len = k.model.length;
      b.height = k.model.height;
      own.set(b.id, key);
    }
    // (each where it is now: on its way, in its place, or not on the yard - out on its tours)
    const now = performance.now() / 1000;
    const list = [];
    for (const b of s.buses) {
      const d = drives.current.get(b.id);
      const pose = d ? poseOf(d, now) : undefined;
      // (its drive done: where it stands, or gone out on its tours)
      if (d && pose === undefined) drives.current.delete(b.id);
      if (pose) list.push({ ...b, ...pose });
      else if (pose === undefined && !b.out) list.push(b);
    }
    shown.current = list;
    const states = { picked: selected, hover: under };
    view.setBuses(instances(list, rgb01(p.colour), { ...states, own: (b) => own.has(b.id) }), moved);
    view.setSkinned(skinned(list, (b) => own.get(b.id), states), moved);
  }
  latest.current = buses;

  // (and again on a new renderer: another graphics level starts unlit)
  useEffect(() => r.current?.highlight(AREAS[hover] || 0), [hover, level]);

  // 2D: the yard from straight above (and back to the view from the corner)
  useEffect(() => {
    if (r.current) r.current.aim(flat ? { yaw: 0, pitch: 1.48 } : { yaw: VIEW.yaw, pitch: VIEW.pitch }, still());
  }, [flat, level]);

  // the build mode: the grid and every lane on, the ghost following the pointer; R turns it,
  // Escape stops
  useEffect(() => {
    const view = r.current;
    if (!view) return undefined;
    setSel(null);
    setGhost(null);
    view.setOverlay('ghost', null);
    if (!placing) {
      view.setOverlay('grid', null);
      return undefined;
    }
    setRot(placing.rot || 0);
    view.setOverlay('grid', gridMesh(p.layout, pal), [1, 1, 1, 0.3], 0.35);
    // (the ghost at once where the pointer is, if it is over the yard)
    if (pointer.current) aimGhost(pointer.current);
    // (before the page's own keys: Escape here stops the build mode, it does not go back)
    const onKey = (e) => {
      if (e.target?.closest?.('input, textarea, [contenteditable]')) return;
      if (e.key === 'Escape') onPlaced?.();
      else if (e.key === 'r' || e.key === 'R') setRot((k) => (k + 1) % 4);
      else return;
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [placing?.area, placing?.move, key, level]);

  useEffect(() => {
    if (placing && pointer.current) aimGhost(pointer.current);
  }, [rot]);

  /** The ghost where the pointer is: its cells on the grid, whether it fits. The pointer holds
   * it by its middle - the module with its lanes, halfway up it (a tall hall seen from the
   * corner stands over the ground the pointer is on, not beside it). */
  function aimGhost(e) {
    const view = r.current;
    const sc = scene.current;
    const L = p.layout;
    if (!view || !sc || !placing) return;
    const [px, py] = at(e);
    const hit = view.ground(px, py, ghostHeight(placing.area) / 2) || view.ground(px, py);
    if (!hit) return;
    const b = sc.bounds;
    const gx0 = hit[0] + (b.x0 + b.x1) / 2;
    const gz0 = hit[1] + (b.z0 + b.z1) / 2;
    const all = [rectOf(L, placing.area, 0, 0, rot), ...apronsOf(L, placing.area, 0, 0, rot)];
    const x0 = Math.min(...all.map((a) => a.x));
    const y0 = Math.min(...all.map((a) => a.y));
    const cx = (x0 + Math.max(...all.map((a) => a.x + a.w))) / 2;
    const cy = (y0 + Math.max(...all.map((a) => a.y + a.h))) / 2;
    const x = Math.round(gx0 / L.cell - cx);
    const y = Math.round(gz0 / L.cell - cy);
    if (ghost && ghost.x === x && ghost.y === y && ghost.rot === rot) return;
    const why = whyNot(L, placing.area, x, y, rot, placing.move ?? null);
    setGhost({ x, y, rot, why });
    view.setOverlay('ghost', ghostMesh(L, placing.area, x, y, rot, pal), why ? RED : GREEN, 0.6);
  }

  async function putDown() {
    if (!ghost || ghost.why) return;
    const args = { x: ghost.x, y: ghost.y, rot: ghost.rot };
    const done = placing.move ? await actMore('depot_move', { id: placing.move, ...args }) : await actMore('depot_build_at', { area: placing.area, ...args });
    if (done?.ok) onPlaced?.();
  }

  /** The module under a point of the canvas. */
  function moduleAt(e) {
    const view = r.current;
    const sc = scene.current;
    if (!view || !sc) return null;
    const [px, py] = at(e);
    const hit = view.ground(px, py);
    if (!hit) return null;
    const b = sc.bounds;
    return moduleUnder(sc.picks, hit[0] + (b.x0 + b.x1) / 2, hit[1] + (b.z0 + b.z1) / 2);
  }

  /** The labels over what they name, where the camera now shows it. */
  function placeTags(vp, cw, ch) {
    const box = tags.current;
    const s = scene.current;
    if (!box || !s) return;
    [...box.children].forEach((el, k) => {
      const t = s.labels[k];
      if (!t) return;
      const q = project(vp, t.pos);
      const on = q[3] > 0 && q[0] > -1.1 && q[0] < 1.1 && q[1] > -1.1 && q[1] < 1.1;
      el.style.display = on ? '' : 'none';
      if (!on) return;
      // (kept inside the view: a label by its edge was cut off by it - its words and its icon)
      const w = el.offsetWidth;
      const h = el.offsetHeight;
      const x = Math.max(4, Math.min(cw - w - 4, (q[0] * 0.5 + 0.5) * cw - w / 2));
      const y = Math.max(4, Math.min(ch - h - 4, (0.5 - q[1] * 0.5) * ch - h));
      el.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
    });
  }

  const keep = () => {
    clearTimeout(keep.t);
    keep.t = setTimeout(() => r.current && setUi('depot', { cam: { ...r.current.goal } }), 400);
  };

  const at = (e) => {
    const b = canvas.current.getBoundingClientRect();
    return [e.clientX - b.left, e.clientY - b.top];
  };
  const busAt = (e) => {
    const view = r.current;
    const s = scene.current;
    if (!view || !s) return null;
    const [x, y] = at(e);
    const { o, d } = view.ray(x, y);
    const list = shown.current;
    const k = pickBus(list, o, d);
    return k < 0 ? null : list[k];
  };

  const down = (e) => {
    if (!r.current) return;
    canvas.current.setPointerCapture?.(e.pointerId);
    drag.current = { x: e.clientX, y: e.clientY, moved: 0, pan: e.button === 2 || e.button === 1 || e.shiftKey };
  };
  const move = (e) => {
    const view = r.current;
    if (!view) return;
    // (no button held, no drag: one let go where the canvas did not hear it is over)
    if (drag.current && e.buttons === 0) drag.current = null;
    const g = drag.current;
    pointer.current = { clientX: e.clientX, clientY: e.clientY };
    if (!g && placing) {
      aimGhost(e);
      return;
    }
    if (!g) {
      const b = busAt(e);
      const id = b ? b.id : null;
      if (id !== under) setUnder(id);
      return;
    }
    const dx = e.clientX - g.x;
    const dy = e.clientY - g.y;
    g.x = e.clientX;
    g.y = e.clientY;
    g.moved += Math.abs(dx) + Math.abs(dy);
    if (g.moved < 4) return;
    const c = view.goal;
    if (g.pan || flat) {
      const k = view.matrices().dist * 0.0016;
      const right = [Math.cos(c.yaw), -Math.sin(c.yaw)];
      const away = [-Math.sin(c.yaw), -Math.cos(c.yaw)];
      view.aim({ tx: c.tx - right[0] * dx * k + away[0] * dy * k, tz: c.tz - right[1] * dx * k + away[1] * dy * k }, true);
    } else view.aim({ yaw: c.yaw - dx * 0.008, pitch: c.pitch + dy * 0.006 }, true);
    if (under !== null) setUnder(null);
  };
  const up = (e) => {
    const g = drag.current;
    drag.current = null;
    if (!g) return;
    if (g.moved < 4 && placing) {
      if (e.button === 2) onPlaced?.();
      else if (e.button === 0) putDown();
    } else if (g.moved < 4 && e.button === 0) {
      const b = busAt(e);
      if (b) {
        onPick?.(b.id);
        setSel(null);
      } else {
        const o = moduleAt(e);
        setSel(o && o.id !== sel ? o.id : null);
        setSure(false);
      }
    } else keep();
  };

  const aim = (patch) => {
    r.current?.aim(patch, still());
    keep();
  };
  const s = scene.current;
  const tagFor = (t) => {
    const k = t.key;
    const n = { icon: ICONS[k], label: k === 'land' ? tr('More land') : t.head, ...names[k] };
    const lit = hover && t.key === hover;
    if (t.works) return html`<div class=${'dp-tag works' + (lit ? ' lit' : '')}><${Icon} name="construction" size=${13} /><span>${n.label}<small>${t.works}</small></span></div>`;
    if (!t.built) return html`<div class=${'dp-tag plot' + (lit ? ' lit' : '')}><${Icon} name="add" size=${13} /><span>${n.label}</span></div>`;
    return html`<div class=${'dp-tag' + (lit ? ' lit' : '')}><${Icon} name=${n.icon || 'info'} size=${13} /><span>${n.label}</span></div>`;
  };
  const tip = under && s?.buses.find((b) => b.id === under);
  const card = sel && s?.picks.find((o) => o.id === sel);
  const hint = placing ? tr('Click to put it down, R to turn it, Esc to stop') : flat ? tr('Drag to move, scroll to zoom') : tr('Drag to turn, right-drag to move, scroll to zoom');
  return html`<div class="cm-plan cm-plan3d" data-buses=${s ? s.buses.length : 0} data-people=${crowd.current} style=${`width:${w}px;height:${h}px`}>
    <canvas key=${level} ref=${canvas} class=${under ? 'on-bus' : ''} style=${`width:${w}px;height:${h}px`} onPointerDown=${down} onPointerMove=${move} onPointerUp=${up} onPointerLeave=${() => {
      pointer.current = null;
      if (!drag.current && under !== null) setUnder(null);
    }} onContextMenu=${(e) => e.preventDefault()}></canvas>
    <div class="dp-tags" ref=${tags}>${s?.labels.map(tagFor)}</div>
    ${tip && !placing && html`<div class="dp-tip">${tip.tip}</div>`}
    ${placing && html`<div class="dp-place">
      <span class=${'dp-why' + (ghost?.why ? ' no' : '')}>${ghost?.why ? tr(WHY[ghost.why]) : placing.move ? tr('Move it where it fits: it costs nothing.') : tr('Put it down where it fits, its lane kept free.')}</span>
      <${Btn} kind="ghost" icon="refresh" label=${tr('Turn')} onClick=${() => setRot((k) => (k + 1) % 4)} />
      <${Btn} kind="ghost" icon="close" label=${tr('Stop')} onClick=${() => onPlaced?.()} />
    </div>`}
    ${card && !placing && html`<div class="dp-card">
      <b>${card.title}</b>
      <span>${card.works || card.what}</span>
      ${!card.building && html`<div class="row">
        <${Btn} kind="ghost" icon="open_with" label=${tr('Move')} onClick=${() => setUi('depot', { placing: { area: card.area, move: card.id, rot: card.rot } })} />
        <${Btn} kind=${sure ? 'danger' : 'ghost'} icon="close" label=${sure ? tr('Sure? Pull it down') : tr('Pull down')} onClick=${async () => {
          if (!sure) return setSure(true);
          const done = await actMore('depot_demolish', { id: card.id });
          if (done?.ok) setSel(null);
          setSure(false);
        }} />
      </div>`}
    </div>`}
    <div class="dp-tools">
      <${IconBtn} icon="zoom_in" size=${17} title=${tr('Zoom in')} onClick=${() => aim({ zoom: (r.current?.goal.zoom || 1) / 1.35 })} />
      <${IconBtn} icon="zoom_out" size=${17} title=${tr('Zoom out')} onClick=${() => aim({ zoom: (r.current?.goal.zoom || 1) * 1.35 })} />
      <${IconBtn} icon="north" size=${17} title=${tr('From above')} onClick=${() => aim({ yaw: 0, pitch: 1.48 })} />
      <${IconBtn} icon="restart_alt" size=${17} title=${tr('Reset the view')} onClick=${() => aim({ ...VIEW })} />
      <${IconBtn} icon="tune" size=${17} title=${tr('Graphics')} onClick=${() => setMenu(!menu)} />
    </div>
    ${menu && html`<div class="dp-menu" role="menu">
      <div class="co-caps">${tr('Graphics')}</div>
      ${QUALITY.map(([q, label, under]) => html`<button type="button" role="menuitemradio" aria-checked=${q === quality} class=${q === quality ? 'on' : ''} onClick=${() => {
        chooseQuality(q);
        setQuality(q);
        setLevel(levelOf(q));
        setMenu(false);
      }}><span>${tr(label)}<small>${q === 'auto' ? tr(NAMES[level]) : tr(under)}</small></span>${q === quality && html`<${Icon} name="check" size=${16} />`}</button>`)}
    </div>`}
    ${!placing && html`<div class="dp-hint">${hint}</div>`}
  </div>`;
}
