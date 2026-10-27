// The company's people at work in the depot (`plan.staff`): small figures, each with its round -
// the mechanics at the buses in the bays (or between the hall and the stores), the cleaners up
// and down the wash lanes (or among the spaces), the drivers between the offices and the buses
// they take out, the office's people across the yard and at the pumps. Each walks or works with
// its arms and legs moving; where it is is worked out from the time, so the round needs no state
// of its own. Each is drawn as its role's figure (`figures.js`: the driver, the mechanic, the
// cleaner, and for the office's people the service employee, the inspector or the researcher),
// its walk while it walks and its idle while it works; until that figure is there, as one small
// model for all of them whose arms and legs `renderer.js` swings.

import { Mesh } from './mesh.js';
import { rgb01 } from './palette.js';
import { seeded } from './city.js';

/** Floats a figure: x, z, heading, phase; r, g, b, walking (0: at work where it stands). */
export const PERSON = 8;
/** Floats a role's figure: x, z, heading, how far between the two frames; the frame's row in
 * the bones, the next one's, walking (the walk's skin) or not, and one spare. */
export const FIGURE = 8;
/** How many are shown at most (the rest are about somewhere else). */
const MOST = { drivers: 24, mechanics: 12, cleaners: 8, office: 8 };
/** Their clothes: the overalls, the high-visibility vests, the drivers' shirts in the company's
 * colour, the office's white. */
const WEAR = { mechanics: '#2f5f9e', cleaners: '#f2c230', office: '#e8ebef' };
// (an easy pace: the figures' walk, drawn larger than life, goes about this fast as it was made)
const WALK = 0.9;
/** How much larger than life the figures are drawn (`renderer.js`), so they are seen from above
 * the yard: their strides too. */
const LARGER = 1.45;

/** The figure (1.8 m, its front at +z): its parts in the area slot - 1 and 2 the legs, 3 the
 * body (its clothes' colour), 4 and 5 the arms, 0 the head and the shoes. */
export function personMesh() {
  const m = new Mesh();
  const trousers = [0.22, 0.24, 0.3];
  const skin = [0.86, 0.68, 0.55];
  const shoe = [0.12, 0.12, 0.14];
  const white = [1, 1, 1];
  for (const [x, part] of [
    [-0.11, 1],
    [0.11, 2],
  ]) {
    m.box(x - 0.08, 0.08, -0.07, x + 0.08, 0.86, 0.07, trousers, { area: part });
    m.box(x - 0.08, 0, -0.08, x + 0.08, 0.08, 0.12, shoe, { area: part });
  }
  m.box(-0.22, 0.86, -0.12, 0.22, 1.45, 0.12, white, { area: 3 });
  for (const [x, part] of [
    [-0.29, 4],
    [0.29, 5],
  ]) {
    m.box(x - 0.06, 0.9, -0.06, x + 0.06, 1.42, 0.06, white, { area: part });
    m.box(x - 0.05, 0.82, -0.05, x + 0.05, 0.9, 0.05, skin, { area: part });
  }
  m.box(-0.04, 1.45, -0.04, 0.04, 1.52, 0.04, skin);
  m.prism(0, 0, 0.12, 0.11, 1.52, 1.78, skin, { n: 8 });
  return m;
}

/** Where a module stands, as `scene.js`'s picks have it: its centre, its corners. */
const centre = (o) => [(o.x0 + o.x1) / 2, (o.z0 + o.z1) / 2];

/**
 * The rounds of the people at work, from the yard (`scene.js`: its picks and buses) and the
 * staff (`plan.staff`); `colour` the company's (the drivers' shirts). A list of
 * `at(t) -> [x, z, heading, walking]` with each one's colour and phase.
 */
export function rounds(scene, staff, colour) {
  if (!staff) return [];
  const rnd = seeded(5);
  const picks = scene.picks.filter((o) => !o.building);
  const of = (area) => picks.filter((o) => o.area === area);
  const offices = of('offices')[0];
  const bays = of('workshop');
  const lanes = of('wash');
  const pumps = of('fuel')[0];
  const spaces = of('parking');
  const yard = scene.yard;
  const home = offices ? centre(offices) : [yard.w / 2, yard.d - 6];
  const out = [];
  const wear = (k) => rgb01(WEAR[k]);
  const shirt = rgb01(colour || '#2a75f7');

  /** A walk to and fro along the points `pts` (and back), at walking pace, from `phase`. */
  const walk = (pts, phase) => {
    const legs = [];
    let total = 0;
    for (let k = 0; k < pts.length - 1; k++) {
      const len = Math.hypot(pts[k + 1][0] - pts[k][0], pts[k + 1][1] - pts[k][1]);
      legs.push(len);
      total += len;
    }
    // (a short stop at either end)
    const stop = 4;
    const round = total * 2 / WALK + stop * 2;
    return (t) => {
      let s = (t + phase * round) % round;
      const back = s > round / 2;
      if (back) s -= round / 2;
      if (s < stop || total < 0.1) {
        const p = back ? pts[0] : pts[pts.length - 1];
        return [p[0], p[1], 0, 0];
      }
      let d = (s - stop) * WALK;
      if (back) d = total - d;
      for (let k = 0; k < legs.length; k++) {
        if (d <= legs[k] || k === legs.length - 1) {
          const f = legs[k] ? Math.min(1, d / legs[k]) : 0;
          const [a, b] = [pts[k], pts[k + 1]];
          const heading = Math.atan2(b[0] - a[0], b[1] - a[1]) + (back ? Math.PI : 0);
          return [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, heading, 1];
        }
        d -= legs[k];
      }
      return [pts[0][0], pts[0][1], 0, 0];
    };
  };
  /** At work where it stands, facing `heading`. */
  const stay = (x, z, heading) => () => [x, z, heading, 0];
  // the office's people by who they are, in turn (all as service staff where the plan does not
  // say)
  const desk = [['service', staff.service], ['inspector', staff.inspectors], ['researcher', staff.researchers]].filter(([, n]) => n > 0);
  const deskOf = (k) => {
    if (!desk.length) return 'service';
    const total = desk.reduce((a, [, n]) => a + n, 0);
    let at = k % total;
    for (const [role, n] of desk) {
      if (at < n) return role;
      at -= n;
    }
    return desk[0][0];
  };
  const ROLE = { drivers: 'driver', mechanics: 'mechanic', cleaners: 'cleaner' };
  let desks = 0;
  const add = (kind, at) => out.push({ at, colour: kind === 'drivers' ? shirt : wear(kind), phase: rnd() * 6.28, role: ROLE[kind] || deskOf(desks++) });

  // the mechanics: two at each bus in a bay, the others to and fro between the hall and the
  // offices (the stores)
  const inBay = scene.buses.filter((b) => b.at === 'workshop');
  let mechanics = Math.min(staff.mechanics || 0, MOST.mechanics);
  for (const b of inBay) {
    for (const side of [-1, 1]) {
      if (mechanics <= 0) break;
      const c = Math.cos(b.heading);
      const s = Math.sin(b.heading);
      const off = side * 1.9;
      const along = (rnd() - 0.5) * b.len * 0.6;
      add('mechanics', stay(b.x + c * off + s * along, b.z - s * off + c * along, b.heading - side * Math.PI / 2));
      mechanics--;
    }
  }
  for (let k = 0; k < mechanics && bays.length; k++) {
    const bay = bays[k % bays.length];
    add('mechanics', walk([centre(bay), home], rnd()));
  }

  // the cleaners: up and down a wash lane, else among the spaces
  const cleaners = Math.min(staff.cleaners || 0, MOST.cleaners);
  for (let k = 0; k < cleaners; k++) {
    const lane = lanes[k % Math.max(1, lanes.length)];
    if (lane) {
      const [cx, cz] = centre(lane);
      const wide = lane.x1 - lane.x0 > lane.z1 - lane.z0;
      const half = (wide ? lane.x1 - lane.x0 : lane.z1 - lane.z0) / 2 + 3;
      add('cleaners', walk(wide ? [[cx - half, cz + 1.5], [cx + half, cz + 1.5]] : [[cx + 1.5, cz - half], [cx + 1.5, cz + half]], rnd()));
    } else if (spaces.length) {
      const sp = spaces[k % spaces.length];
      add('cleaners', walk([[sp.x0 + 2, sp.z1 + 1], [sp.x1 - 2, sp.z1 + 1]], rnd()));
    }
  }

  // the drivers: from the offices to a bus in the spaces and back
  const parked = scene.buses.filter((b) => b.at === 'parked' && !b.out);
  const drivers = Math.min(staff.drivers || 0, MOST.drivers, parked.length + 4);
  for (let k = 0; k < drivers; k++) {
    const b = parked[Math.floor(rnd() * parked.length)];
    const to = b ? [b.x + Math.cos(b.heading) * 1.8, b.z - Math.sin(b.heading) * 1.8] : [home[0] + (rnd() - 0.5) * 20, home[1] + 14];
    const mid = [(home[0] + to[0]) / 2 + (rnd() - 0.5) * 6, Math.max(home[1], to[1]) + 3 + rnd() * 4];
    add('drivers', walk([home, mid, to], rnd()));
  }

  // the office's people: one at the pumps when there are some, the others across the yard
  let office = Math.min(staff.office || 0, MOST.office);
  if (pumps && office > 0) {
    const [px, pz] = centre(pumps);
    add('office', walk([[px - 5, pz + 4], [px + 5, pz + 4]], rnd()));
    office--;
  }
  for (let k = 0; k < office; k++) {
    const there = picks[Math.floor(rnd() * picks.length)];
    add('office', there ? walk([home, centre(there)], rnd()) : stay(home[0] + k * 1.2, home[1] + 8, 0));
  }
  return out;
}

/** The roles whose figures the people of `list` are. */
export const rolesOf = (list) => [...new Set(list.map((p) => p.role))];

/**
 * The people at time `t` (seconds): `figures` by role (`FIGURE` floats each) for those whose
 * figure `modelOf(role)` has (its `clips`, `fps` and `speed`), and `rest` (`PERSON` floats each)
 * as the plain model for the others.
 */
export function people(list, t, modelOf = () => null) {
  const rest = [];
  const figures = new Map();
  for (const p of list) {
    const [x, z, heading, walking] = p.at(t);
    const m = modelOf(p.role);
    if (!m) {
      rest.push(x, z, heading, p.phase, p.colour[0], p.colour[1], p.colour[2], walking);
      continue;
    }
    // (the walk played to the pace the round goes, so the feet do not slide)
    const clip = m.clips[walking ? 'walk' : 'idle'];
    const at = ((t + p.phase) * (walking ? WALK / (m.speed * LARGER) : 1) * m.fps) % clip.frames;
    const k = Math.floor(at);
    if (!figures.has(p.role)) figures.set(p.role, []);
    figures.get(p.role).push(x, z, heading, at - k, clip.row + k, clip.row + ((k + 1) % clip.frames), walking ? 1 : 0, 0);
  }
  return { rest: new Float32Array(rest), figures: new Map([...figures].map(([k, v]) => [k, new Float32Array(v)])) };
}
