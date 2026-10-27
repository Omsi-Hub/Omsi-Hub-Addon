// The buses driving about the yard: a bus that goes out on its tours drives out of its space,
// across the yard and out of the gate into the street; one that comes back drives in from the
// street, through a wash lane (when the depot has one, a stop under its brushes) and backs into
// its space; one going to the workshop (or back from it, or to a space a bus of the street may
// have meanwhile) drives from where it stands to where it goes. Its way across the yard is
// found on the yard's grid round the modules (`route`), its corners rounded, and timed: a
// bus's place and heading at a moment of it (`at`).
//
// The yard in metres: x across (0 to the land's width), z from the back to the front, the
// street beyond z = rows * cell; a bus's heading 0 has its nose to +z (`scene.js`'s `frame`).

import { apronsOf } from './yard.js';

/** Metres a second: forward, backing into a space. */
const SPEED = 7;
const BACK = 2.6;
/** Seconds under the wash lane's brushes. */
const WASH = 2.5;
/** The street's lanes: out to the east on the far one, in from the east on the near one. */
const OUT_LANE = 12.5;
const IN_LANE = 8.5;
/** How far down the street a bus drives before it is gone (and comes from). */
const AWAY = 90;

const DIRS = [
  [1, 0],
  [0, 1],
  [-1, 0],
  [0, -1],
];

/** The yard's grid: which cells a bus may drive on (1: on the land, on no module; 2: across a
 * block of parking spaces, between the buses - where the blocks wall a part of the yard off;
 * 0: not), and its gate. */
export function grid(L) {
  const cols = L.cols;
  const rows = L.rows;
  const free = new Uint8Array(cols * rows).fill(1);
  for (const o of L.modules)
    for (let j = Math.max(0, o.y); j < Math.min(rows, o.y + o.h); j++) for (let i = Math.max(0, o.x); i < Math.min(cols, o.x + o.w); i++) free[j * cols + i] = o.area === 'parking' && !o.building ? 2 : 0;
  const c = L.cell;
  const gx = (L.gate.x + L.gate.w / 2) * c;
  return { L, cols, rows, cell: c, free, W: cols * c, D: rows * c, gx };
}

const inside = (g, x, z) => x >= 0 && z >= 0 && x < g.W && z < g.D;
const freeAt = (g, x, z) => inside(g, x, z) && g.free[Math.floor(z / g.cell) * g.cols + Math.floor(x / g.cell)] === 1;
/** What a cell more costs across a block of spaces. */
const ACROSS = 8;
const centre = (g, [i, j]) => [(i + 0.5) * g.cell, (j + 0.5) * g.cell];

/** The free cell nearest a point (on the grid's land). */
function cellNear(g, x, z) {
  const i0 = Math.max(0, Math.min(g.cols - 1, Math.floor(x / g.cell)));
  const j0 = Math.max(0, Math.min(g.rows - 1, Math.floor(z / g.cell)));
  for (let r = 0; r < Math.max(g.cols, g.rows); r++)
    for (let dj = -r; dj <= r; dj++)
      for (let di = -r; di <= r; di++) {
        if (Math.max(Math.abs(di), Math.abs(dj)) !== r) continue;
        const i = i0 + di;
        const j = j0 + dj;
        if (i >= 0 && j >= 0 && i < g.cols && j < g.rows && g.free[j * g.cols + i] === 1) return [i, j];
      }
  return [i0, j0];
}

/** The cells from one to another on the free ones, the fewest turns among the shortest
 * (null: no way). */
function cells(g, a, b) {
  const n = g.cols * g.rows;
  const cost = new Float64Array(n * 4).fill(Infinity);
  const back = new Int32Array(n * 4).fill(-1);
  const open = [];
  for (let d = 0; d < 4; d++) {
    const s = (a[1] * g.cols + a[0]) * 4 + d;
    cost[s] = 0;
    open.push([0, s]);
  }
  const goal = b[1] * g.cols + b[0];
  let end = -1;
  while (open.length) {
    // (the grid is small: the cheapest taken by a look along the list)
    let k = 0;
    for (let m = 1; m < open.length; m++) if (open[m][0] < open[k][0]) k = m;
    const [c, s] = open[k];
    open[k] = open[open.length - 1];
    open.pop();
    if (c > cost[s]) continue;
    const cell = s >> 2;
    if (cell === goal) {
      end = s;
      break;
    }
    const i = cell % g.cols;
    const j = (cell - i) / g.cols;
    const d0 = s & 3;
    for (let d = 0; d < 4; d++) {
      const ni = i + DIRS[d][0];
      const nj = j + DIRS[d][1];
      const f = ni < 0 || nj < 0 || ni >= g.cols || nj >= g.rows ? 0 : g.free[nj * g.cols + ni];
      if (!f) continue;
      const t = (nj * g.cols + ni) * 4 + d;
      const nc = c + (f === 2 ? ACROSS : 1) + (d === d0 ? 0 : d === (d0 + 2) % 4 ? 4 : 0.6);
      if (nc < cost[t]) {
        cost[t] = nc;
        back[t] = s;
        open.push([nc, t]);
      }
    }
  }
  if (end < 0) return null;
  const out = [];
  for (let s = end; s >= 0; s = back[s]) out.push([(s >> 2) % g.cols, Math.floor((s >> 2) / g.cols)]);
  return out.reverse();
}

/** The way across the yard from a point to another, as its corners (metres); null: there is
 * none (walled in by the modules). */
function across(g, from, to) {
  const path = cells(g, cellNear(g, from[0], from[1]), cellNear(g, to[0], to[1]));
  if (!path) return null;
  const pts = [from];
  for (let k = 1; k < path.length - 1; k++) {
    const [a, b, c] = [path[k - 1], path[k], path[k + 1]];
    if (b[0] - a[0] !== c[0] - b[0] || b[1] - a[1] !== c[1] - b[1]) pts.push(centre(g, b));
  }
  pts.push(to);
  return pts;
}

/** Where a bus standing in a space (or a bay, or a lane) is once it drove out of it forwards:
 * past the module, a cell on. */
function outOf(g, b) {
  const f = [Math.sin(b.heading), Math.cos(b.heading)];
  let x = b.x;
  let z = b.z;
  for (let k = 0; k < 80 && inside(g, x, z) && !freeAt(g, x, z); k++) {
    x += f[0] * 0.5;
    z += f[1] * 0.5;
  }
  return [x + f[0] * g.cell * 0.5, z + f[1] * g.cell * 0.5];
}

/** The gate: just inside it and just outside it. */
const gateIn = (g) => [g.gx, g.D - g.cell * 1.5];
const gateOut = (g) => [g.gx, g.D + 2];

/** A wash lane to drive through, its way in and out (null: none). */
function washLane(g) {
  const o = g.L.modules.find((m) => m.area === 'wash' && !m.building);
  const s = o && g.L.shapes?.wash;
  if (!s) return null;
  const mid = (a) => [(a.x + a.w / 2) * g.cell, (a.y + a.h / 2) * g.cell];
  const aprons = apronsOf(g.L, 'wash', o.x, o.y, o.rot || 0);
  if (aprons.length < 2) return null;
  // (in at its back, out at its front: the aprons as `apronsOf` gives them)
  const inner = mid(o);
  return { in: mid(aprons[1]), mid: inner, out: mid(aprons[0]) };
}

/** A bus's way, as legs: `pts` driven at `speed` (`rev`: backwards), or a `wait`. */
function legs(g, from, to, kind) {
  const L = [];
  const fwd = (pts) => L.push({ pts, rev: false });
  // (straight on where the modules leave no way: better than no drive)
  const to_ = (a, b) => fwd(across(g, a, b) || [a, b]);
  const outside = (p) => p[1] >= g.D;
  // from where it is to the yard's lanes
  let p;
  if (kind === 'leave' && outside(from)) {
    // (a bus of the street goes from where it stands)
    fwd([[from.x, from.z], [from.x + 10, g.D + OUT_LANE], [g.gx + AWAY, g.D + OUT_LANE]]);
    return L;
  }
  if (kind === 'arrive') {
    const s = [g.gx + AWAY, g.D + IN_LANE];
    fwd([s, [g.gx + 10, g.D + IN_LANE], gateOut(g), gateIn(g)]);
    p = gateIn(g);
  } else if (outside(from)) {
    fwd([[from.x, from.z], [g.gx + 6, from.z], gateOut(g), gateIn(g)]);
    p = gateIn(g);
  } else {
    p = outOf(g, from);
    fwd([[from.x, from.z], p]);
  }
  // back from its tours: through the wash lane first - in at its back and out at its front, or
  // (its back walled in) in at the front and backed out again
  const lane = kind === 'arrive' && washLane(g);
  const through = lane && across(g, p, lane.in);
  const into = lane && !through && across(g, p, lane.out);
  if (through) {
    fwd(through);
    fwd([lane.in, lane.mid]);
    L.push({ wait: WASH });
    fwd([lane.mid, lane.out]);
    p = lane.out;
  } else if (into) {
    fwd(into);
    fwd([lane.out, lane.mid]);
    L.push({ wait: WASH });
    L.push({ pts: [lane.mid, lane.out], rev: true });
    p = lane.out;
  }
  // to where it goes
  if (kind === 'leave') {
    to_(p, gateIn(g));
    fwd([gateIn(g), gateOut(g), [g.gx + 6, g.D + OUT_LANE], [g.gx + AWAY, g.D + OUT_LANE]]);
  } else if (outside(to)) {
    to_(p, gateIn(g));
    fwd([gateIn(g), gateOut(g), [to.x - 6, to.z], [to.x, to.z]]);
  } else {
    // (up to the space's front, then backed into it)
    const front = outOf(g, to);
    to_(p, front);
    L.push({ pts: [front, [to.x, to.z]], rev: true });
  }
  return L;
}

/** A polyline's corners rounded (radius up to `r` metres). */
function rounded(pts, r = 5) {
  const clean = pts.filter((q, k) => k === 0 || Math.hypot(q[0] - pts[k - 1][0], q[1] - pts[k - 1][1]) > 0.05);
  if (clean.length < 3) return clean;
  const out = [clean[0]];
  for (let k = 1; k < clean.length - 1; k++) {
    const [a, b, c] = [clean[k - 1], clean[k], clean[k + 1]];
    const la = Math.hypot(b[0] - a[0], b[1] - a[1]);
    const lc = Math.hypot(c[0] - b[0], c[1] - b[1]);
    const t = Math.min(r, la / 2, lc / 2);
    const p0 = [b[0] + ((a[0] - b[0]) * t) / la, b[1] + ((a[1] - b[1]) * t) / la];
    const p1 = [b[0] + ((c[0] - b[0]) * t) / lc, b[1] + ((c[1] - b[1]) * t) / lc];
    for (let s = 0; s <= 6; s++) {
      const u = s / 6;
      out.push([(1 - u) * (1 - u) * p0[0] + 2 * u * (1 - u) * b[0] + u * u * p1[0], (1 - u) * (1 - u) * p0[1] + 2 * u * (1 - u) * b[1] + u * u * p1[1]]);
    }
  }
  out.push(clean[clean.length - 1]);
  return out;
}

/** A bus's drive from `from` (a place and heading: where it stands, or where it is on its way)
 * to `to` (where it is to stand; null: away on its tours): `kind` 'leave', 'arrive' (from the
 * street, back from its tours) or 'move'. Its moments: `{ t, x, z, h }`, t in seconds. */
export function route(g, from, to, kind) {
  const ls = legs(g, from, to, kind);
  // the forward legs one after another are one way, its corners rounded across them
  const runs = [];
  for (const l of ls) {
    const last = runs[runs.length - 1];
    if (l.pts && !l.rev && last?.pts && !last.rev) last.pts.push(...l.pts.slice(1));
    else runs.push(l.pts ? { pts: [...l.pts], rev: l.rev } : l);
  }
  const out = [];
  let t = 0;
  let h = kind === 'arrive' ? -Math.PI / 2 : from.heading;
  for (const run of runs) {
    if (run.wait) {
      const q = out[out.length - 1];
      t += run.wait;
      if (q) out.push({ ...q, t });
      continue;
    }
    const pts = run.rev ? run.pts : rounded(run.pts);
    for (let k = 0; k < pts.length; k++) {
      const [x, z] = pts[k];
      if (k > 0) {
        const [px, pz] = pts[k - 1];
        const len = Math.hypot(x - px, z - pz);
        if (len < 1e-3) continue;
        h = run.rev ? Math.atan2(px - x, pz - z) : Math.atan2(x - px, z - pz);
        t += len / (run.rev ? BACK : SPEED);
      } else if (out.length && Math.hypot(out[out.length - 1].x - x, out[out.length - 1].z - z) < 1e-3) continue;
      out.push({ t, x, z, h });
    }
  }
  // (the heading of each moment that of the way on from it: the first the way's own)
  for (let k = 0; k < out.length - 1; k++) if (out[k + 1].t > out[k].t && Math.hypot(out[k + 1].x - out[k].x, out[k + 1].z - out[k].z) > 1e-3) out[k].h = out[k + 1].h;
  // (in its space it stands as the space has it)
  if (to && out.length) out[out.length - 1].h = to.heading;
  return out;
}

/** Where a bus on its drive is `t` seconds into it: `{ x, z, heading }`. */
export function at(moments, t) {
  if (!moments.length) return null;
  if (t <= moments[0].t) return { x: moments[0].x, z: moments[0].z, heading: moments[0].h };
  for (let k = 1; k < moments.length; k++) {
    const b = moments[k];
    if (t > b.t) continue;
    const a = moments[k - 1];
    const u = b.t > a.t ? (t - a.t) / (b.t - a.t) : 1;
    // (the heading turned the short way round)
    let dh = b.h - a.h;
    dh = Math.atan2(Math.sin(dh), Math.cos(dh));
    return { x: a.x + (b.x - a.x) * u, z: a.z + (b.z - a.z) * u, heading: a.h + dh * Math.min(1, u * 1.5) };
  }
  const e = moments[moments.length - 1];
  return { x: e.x, z: e.z, heading: e.h };
}

/** How long a drive takes (seconds). */
export const length = (moments) => (moments.length ? moments[moments.length - 1].t : 0);
