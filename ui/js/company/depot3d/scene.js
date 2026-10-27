// The yard in metres, from the depot page's plan (`company_more::depot::plan`) and its layout
// (`plan.layout`, `hub_core::company::yard`): every module where the player put it down, turned
// as it stands - the offices a storey a level, the workshop's bays (an open roof of trusses so
// the buses in them show), the washing bay, the diesel station, the blocks of twelve spaces with
// their chargers -, building work where a module will stand, the gate and the street at the
// front, the land still to buy beside the yard, a verge of grass with trees round it all, and
// the town round that (`city.js`). The buses stand where the plan says: in the bays, in the
// spaces, before the workshop when they wait for it, and in the street when the yard has no room.
// What use leaves on the ground (oil in the spaces, tyre tracks at the gate and before the
// workshop, cracks) is a layer of its own (`decals`).
//
// Each module is drawn in a frame of its own (x across, z from its back to its front, its front
// at +z) and turned onto the yard. Each area has its number (`AREAS`): the Build list's row
// lights it up.

import { Mesh } from './mesh.js';
import { MAT } from './palette.js';
import { buildTown, seeded, tree } from './city.js';

export const AREAS = { parking: 1, workshop: 2, wash: 3, fuel: 4, charging: 5, offices: 6, land: 7 };

/** A bus's length in metres by its size (`BusSize`). */
const LENGTH = { midi: 10.5, solo: 12, articulated: 18, double: 11.5 };
const VERGE = 16;

/** A module's frame onto the yard: its own (lx, y, lz), lw wide and ld deep, turned `rot` with
 * its corner at (x0, z0). */
export function frame(rot, x0, z0, lw, ld) {
  const k = ((rot % 4) + 4) % 4;
  const pt = ([x, y, z]) => [[x0 + x, y, z0 + z], [x0 + z, y, z0 + lw - x], [x0 + lw - x, y, z0 + ld - z], [x0 + ld - z, y, z0 + x]][k];
  const dir = ([x, y, z]) => [[x, y, z], [z, y, -x], [-x, y, -z], [-z, y, x]][k];
  return { pt, dir, heading: (h) => h + (k * Math.PI) / 2 };
}

/** The yard: `{ mesh, decals, buses, labels, bounds, world, picks }` (`picks`: each module's
 * place in metres, to point at; `bounds` the depot's plot, `world` the town's). `pal` is the
 * look's palette (`palette.js`), `level` how finely it is drawn (`quality.js`: without trees at
 * the lowest, the town smaller). */
export function buildScene(p, pal, level = { trees: true, town: 2 }) {
  const L = p.layout;
  const cell = L.cell;
  const m = new Mesh();
  const marks = new Mesh();
  // (what was added to the areas: `depot::Upgrade`)
  const up = new Set(p.upgrades || []);
  const buses = [];
  const labels = [];
  const picks = [];
  const W = L.cols * cell;
  const D = L.rows * cell;
  const MW = L.max_cols * cell;
  const street = { z0: D + 5, z1: D + 15 };

  // the base: the town with the depot's plot in it, the yard's asphalt on its grass, the street
  // at the front
  const bx0 = -VERGE;
  const bx1 = MW + VERGE;
  const bz0 = -VERGE;
  const bz1 = street.z1 + 6;
  const gate = { x0: L.gate.x * cell, x1: (L.gate.x + L.gate.w) * cell };
  const world = buildTown(m, marks, { plot: { x0: bx0, x1: bx1, z0: bz0, z1: street.z0 }, street, gate }, pal, level, 7 + L.max_cols * 13);
  m.flat(0, 0, W, D, 0.02, pal.asphalt);
  // the land still to buy: its edge marked, a label to say so
  if (MW > W) {
    m.dashed(W + 1, 1, MW - 1, D - 1, pal.plot, AREAS.land, { w: 0.35 });
    for (let z = 4; z < D - 2; z += 12) m.box(MW - 1.2, 0, z, MW - 1, 1.2, z + 0.2, pal.fence, { area: AREAS.land });
    if (L.land) labels.push({ key: 'land', pos: [(W + Math.min(MW, W + 40)) / 2, 2.5, D / 2], built: false, works: null });
  }
  m.flat(gate.x0, D, gate.x1, street.z0 - 2.5, 0.03, pal.asphalt);
  // the yard's wear: the tracks from the gate, cracks here and there
  const wear = seeded(11 + L.cols * 7 + L.rows);
  marks.decal((gate.x0 + gate.x1) / 2, D - 6, gate.x1 - gate.x0, 18, 0, MAT.tyres, 0.5);
  for (let k = 0; k < (W * D) / 900; k++) marks.decal(2 + wear() * (W - 4), 2 + wear() * (D - 4), 3 + wear() * 4, 3 + wear() * 4, wear() * 6, MAT.cracks, 0.35 + wear() * 0.4);
  // the fence round the yard, open at the gate; a gatehouse beside it
  const post = (x, z) => m.box(x - 0.08, 0, z - 0.08, x + 0.08, 1.9, z + 0.08, pal.fence);
  const rail = (x0, z0, x1, z1) => m.box(Math.min(x0, x1) - 0.04, 1.75, Math.min(z0, z1) - 0.04, Math.max(x0, x1) + 0.04, 1.85, Math.max(z0, z1) + 0.04, pal.fence);
  const fence = (x0, z0, x1, z1) => {
    const len = Math.hypot(x1 - x0, z1 - z0);
    const n = Math.max(1, Math.round(len / 3));
    for (let k = 0; k <= n; k++) post(x0 + ((x1 - x0) * k) / n, z0 + ((z1 - z0) * k) / n);
    rail(x0, z0, x1, z1);
  };
  fence(0, 0, W, 0);
  fence(0, 0, 0, D);
  fence(W, 0, W, D);
  fence(0, D, gate.x0, D);
  fence(gate.x1, D, W, D);
  m.box(gate.x1 + 1, 0, D - 4.5, gate.x1 + 4.5, 2.8, D - 1, pal.wall, { top: pal.roof });
  m.box(gate.x1 + 1.02, 1.1, D - 1.04, gate.x1 + 4.48, 2.1, D - 0.98, pal.glass, { glow: 0.9 });
  m.box(gate.x0 - 0.5, 0.9, D - 0.15, gate.x1 - 0.5, 1.05, D + 0.15, pal.barrier);

  // the modules, by area, from the back and the west
  const order = (a, b) => a.y - b.y || a.x - b.x;
  const of = (area) => L.modules.filter((o) => o.area === area && !o.building).sort(order);
  const shape = (area) => L.shapes[area];
  const put = (o, draw) => {
    const s = shape(o.area);
    const lw = s.w * cell;
    const ld = s.h * cell;
    const f = frame(o.rot, o.x * cell, o.y * cell, lw, ld);
    const lm = new Mesh();
    draw(lm, { x0: 0, x1: lw, z0: 0, z1: ld }, f);
    m.append(lm, f);
    picks.push({ id: o.id, area: o.area, title: o.title, what: o.what, works: o.works, building: o.building, rot: o.rot, x0: o.x * cell, z0: o.y * cell, x1: (o.x + o.w) * cell, z1: (o.y + o.h) * cell });
    return f;
  };
  const tag = (key, f, lx, y, lz, o) => labels.push({ key, head: o.title, pos: f.pt([lx, y, lz]), built: !o.building, works: o.works });
  // building work: an amber outline, a site fence and a crane's mast
  const works = (mm, r, area, h = 4) => {
    mm.dashed(r.x0, r.z0, r.x1, r.z1, pal.warn, area, { w: 0.4 });
    for (let x = r.x0 + 1; x < r.x1 - 1; x += 3) mm.box(x, 0, r.z1 - 0.6, x + 2.2, 1.1, r.z1 - 0.45, pal.warn, { area });
    mm.box(r.x0 + 1.5, 0, r.z0 + 1.5, r.x0 + 2.3, h + 10, r.z0 + 2.3, pal.warn, { area });
    mm.box(r.x0 + 1.5, h + 9.4, r.z0 + 1.5, Math.min(r.x1, r.x0 + 14), h + 10, r.z0 + 2.3, pal.warn, { area });
  };
  for (const o of L.modules.filter((x) => x.building)) {
    const s = shape(o.area);
    const f = put(o, (mm, r) => works(mm, r, AREAS[o.area] || 0, 4));
    tag(o.area, f, (s.w * cell) / 2, 8, (s.h * cell) / 2, o);
  }

  // the offices: a storey a level, windows round every floor, lit at night
  for (const o of of('offices')) {
    const floors = Math.max(1, p.offices.floors || 1);
    const h = 1 + floors * 3.6;
    const f = put(o, (mm, r) => {
      const A = AREAS.offices;
      mm.box(r.x0, 0, r.z0 + 4, r.x1, h, r.z1, pal.wall, { area: A, top: pal.roof });
      for (let k = 0; k < floors; k++) {
        const y0 = 1.9 + k * 3.6;
        mm.box(r.x0 - 0.05, y0, r.z0 + 5, r.x1 + 0.05, y0 + 1.6, r.z1 - 1, pal.glass, { area: A, glow: 0.8 });
        mm.box(r.x0 + 1, y0, r.z1 - 0.05, r.x1 - 1, y0 + 1.6, r.z1 + 0.05, pal.glass, { area: A, glow: 0.8 });
      }
      mm.box(r.x0 + 2, h, r.z0 + 7, r.x0 + 5.5, h + 1.2, r.z0 + 10, pal.metal, { area: A });
      const cx = (r.x0 + r.x1) / 2;
      mm.box(cx - 2, 2.9, r.z1, cx + 2, 3.2, r.z1 + 2, pal.trim, { area: A });
      // what was added: the canteen and the training room in glass at the back, the control
      // room on the roof with its mast, the solar panels beside it
      const back = (x0, x1) => {
        mm.box(x0, 0, r.z0 + 0.6, x1, 3.4, r.z0 + 4, pal.glass, { area: A, glow: 0.7, top: pal.roof });
        mm.box(x0 - 0.1, 3.4, r.z0 + 0.5, x1 + 0.1, 3.7, r.z0 + 4.1, pal.trim, { area: A });
      };
      if (up.has('canteen')) back(r.x0 + 0.8, cx - 0.4);
      if (up.has('training_room')) back(cx + 0.4, r.x1 - 0.8);
      if (up.has('control_room')) {
        mm.box(r.x1 - 7, h, r.z1 - 7, r.x1 - 1.5, h + 2.6, r.z1 - 1.5, pal.wall, { area: A, top: pal.roof });
        mm.box(r.x1 - 7.05, h + 0.9, r.z1 - 7.05, r.x1 - 1.45, h + 2.1, r.z1 - 1.45, pal.glass, { area: A, glow: 0.9 });
        mm.box(r.x1 - 2.4, h + 2.6, r.z1 - 2.4, r.x1 - 2.2, h + 8, r.z1 - 2.2, pal.metal, { area: A });
      }
      if (up.has('solar')) for (let z = r.z0 + 11; z < r.z1 - 8; z += 2.2) for (let x = r.x0 + 1.5; x < r.x1 - 9; x += 3.4) mm.box(x, h + 0.3, z, x + 3, h + 0.45, z + 1.7, pal.solar, { area: A });
      if (o.works) works(mm, r, A, h);
    });
    tag('offices', f, 10, h + 3, 14, o);
  }

  // the workshop: a hall a bay, the bays side by side make one; the buses in their bays, nose
  // to the yard, the ones waiting for a bay before the first
  const H = 8.5;
  const bays = of('workshop');
  bays.forEach((o, k) => {
    const f = put(o, (mm, r, fr) => {
      const t = 0.5;
      const A = AREAS.workshop;
      mm.box(r.x0, 0, r.z0, r.x1, H, r.z0 + t, pal.wall, { area: A });
      mm.box(r.x0, 0, r.z0, r.x0 + t, H, r.z1, pal.wall, { area: A });
      mm.box(r.x1 - t, 0, r.z0, r.x1, H, r.z1, pal.wall, { area: A });
      mm.box(r.x0, H - 1.6, r.z1 - t, r.x1, H, r.z1, pal.wall, { area: A });
      for (let z = r.z0 + 3; z < r.z1 - 1; z += 3.2) mm.box(r.x0, H - 0.2, z, r.x1, H + 0.4, z + 0.45, pal.roof, { area: A });
      mm.box(r.x0, H - 0.1, r.z0, r.x1, H + 0.5, r.z0 + 2.2, pal.roof, { area: A });
      // inside: its walls of blocks, its floor coated
      mm.box(r.x0 + t, 0, r.z0 + t, r.x1 - t, H - 0.4, r.z0 + t + 0.04, pal.hallWall, { area: A });
      mm.box(r.x0 + t, 0, r.z0 + t, r.x0 + t + 0.04, H - 0.4, r.z1 - 0.2, pal.hallWall, { area: A });
      mm.box(r.x1 - t - 0.04, 0, r.z0 + t, r.x1 - t, H - 0.4, r.z1 - 0.2, pal.hallWall, { area: A });
      mm.flat(r.x0 + 0.5, r.z0 + 0.5, r.x1 - 0.5, r.z1 - 0.2, 0.04, pal.hallFloor, A);
      const cx = (r.x0 + r.x1) / 2;
      mm.flat(cx - 0.5, r.z0 + 4, cx + 0.5, r.z1 - 3, 0.06, pal.pit, A);
      mm.box(r.x0 + 0.6, H - 1.6, r.z1 - 0.15, r.x1 - 0.6, H - 0.6, r.z1 + 0.1, pal.door, { area: A });
      mm.box(cx - 1.2, H - 0.7, (r.z0 + r.z1) / 2 - 0.2, cx + 1.2, H - 0.5, (r.z0 + r.z1) / 2 + 0.2, pal.lamp, { glow: 1 });
      // (the tracks of the buses in and out on its forecourt, oil under the pit)
      marks.append(mark(cx, r.z1 + 7, 3.4, 14, 0, MAT.tyres, 0.55), fr);
      marks.append(mark(cx, (r.z0 + r.z1) / 2, 2.2, 6, 0, MAT.oil, 0.5), fr);
      // what was added: the paint booth at the back of the last bay, the inspection lane's
      // rollers and lines in the first, a rack of tyres along a wall
      const last = k === bays.length - 1;
      if (up.has('paint_booth') && last) {
        mm.box(r.x0 + 0.8, 0, r.z0 + 0.8, r.x1 - 0.8, 4.6, r.z0 + 6, pal.boothWall, { area: A });
        mm.box(r.x0 + 1, 4.6, r.z0 + 1.2, r.x1 - 1, 5.4, r.z0 + 5.4, pal.metal, { area: A });
        mm.box(r.x0 + 0.85, 1.4, r.z0 + 6, r.x1 - 0.85, 3.6, r.z0 + 6.04, pal.lamp, { area: A, glow: 0.8 });
        mm.prism(cx, r.z0 + 3, 0.45, 0.45, H, H + 2.4, pal.metal, { n: 8, area: A });
      }
      if (up.has('inspection_lane') && k === 0) {
        for (const x of [cx - 1.4, cx + 1.4]) mm.flat(x - 0.4, r.z1 - 6, x + 0.4, r.z1 - 4.6, 0.07, pal.metal, A);
        for (const x of [r.x0 + 1.1, r.x1 - 1.1]) mm.flat(x - 0.06, r.z0 + 6, x + 0.06, r.z1 - 1, 0.06, pal.warn, A);
      }
      if (up.has('tyre_store') && k === 0)
        for (let z = r.z0 + 7; z < r.z1 - 6; z += 1.1) for (let y = 0.45; y < 2.6; y += 0.62) mm.prism(r.x0 + 1.05, z, 0.45, 0.45, y - 0.3, y, pal.tyre, { n: 8, area: A });
    });
    const bus = p.in_ws[k];
    if (bus) buses.push(place(bus, f, 4, 15, 'workshop'));
    if (k === 0) tag('workshop', f, 4, H + 3.5, 14, o);
  });
  if (bays.length) {
    const s = shape('workshop');
    const first = bays[0];
    const f = frame(first.rot, first.x * cell, first.y * cell, s.w * cell, s.h * cell);
    p.in_ws.slice(bays.length, bays.length + 6).forEach((bus, k) => buses.push(place(bus, f, 8 + k * 14, s.h * cell + 6, 'waiting', Math.PI / 2)));
  }

  // the wash lanes: a portal over each lane, its brushes; two gantries over each once they are
  // added; the vacuum posts of the interior cleaning before it, the underbody wash's jets in its
  // floor, the recycled water's tanks on its roof
  of('wash').forEach((o, k) => {
    const f = put(o, (mm, r) => {
      const cx = (r.x0 + r.x1) / 2;
      const A = AREAS.wash;
      mm.flat(r.x0 + 1, r.z0, r.x1 - 1, r.z1, 0.04, pal.washLane, A);
      mm.box(r.x0, 0, r.z0, r.x0 + 0.6, 5.5, r.z1, pal.wall, { area: A });
      mm.box(r.x1 - 0.6, 0, r.z0, r.x1, 5.5, r.z1, pal.wall, { area: A });
      // (tiled inside, to the height the water reaches)
      mm.box(r.x0 + 0.6, 0, r.z0, r.x0 + 0.64, 3.2, r.z1, pal.washWall, { area: A });
      mm.box(r.x1 - 0.64, 0, r.z0, r.x1 - 0.6, 3.2, r.z1, pal.washWall, { area: A });
      mm.box(r.x0, 5.5, r.z0, r.x1, 6.1, r.z1, pal.roof, { area: A });
      for (const z of p.wash.gates ? [r.z0 + 8, r.z0 + 18] : [r.z0 + 13]) {
        mm.box(r.x0 + 0.6, 4.4, z - 0.4, r.x1 - 0.6, 4.9, z + 0.4, pal.metal, { area: A });
        mm.prism(cx - 2.1, z, 0.55, 0.55, 0.3, 4.4, pal.brush, { n: 8, area: A });
        mm.prism(cx + 2.1, z, 0.55, 0.55, 0.3, 4.4, pal.brush, { n: 8, area: A });
      }
      if (up.has('interior'))
        for (const x of [r.x0 + 1.4, r.x1 - 1.4]) {
          mm.box(x - 0.3, 0, r.z1 + 1.2, x + 0.3, 1.8, r.z1 + 1.8, pal.pump, { area: A });
          mm.box(x - 0.2, 1.8, r.z1 + 1.4, x + 0.2, 2.8, r.z1 + 1.6, pal.brush, { area: A });
        }
      if (up.has('underbody')) for (let z = r.z0 + 3; z < r.z1 - 3; z += 5) mm.flat(cx - 1.6, z, cx + 1.6, z + 0.5, 0.06, pal.brush, A, 0.5);
      if (up.has('recycling')) for (const z of [r.z0 + 4, r.z0 + 8]) mm.prism(cx, z, 1.3, 1.3, 6.1, 8.4, pal.tank, { n: 12, area: A });
      if (o.works) works(mm, r, A, 6);
    });
    if (k === 0) tag('wash', f, 6, 9, 14, o);
  });

  // the diesel station: a canopy on four columns over its pumps, a band in the accent
  for (const o of of('fuel')) {
    const f = put(o, (mm, r) => {
      const A = AREAS.fuel;
      const inset = 2;
      for (const [x, z] of [
        [r.x0 + inset, r.z0 + inset],
        [r.x1 - inset, r.z0 + inset],
        [r.x0 + inset, r.z1 - inset],
        [r.x1 - inset, r.z1 - inset],
      ])
        mm.box(x - 0.3, 0, z - 0.3, x + 0.3, 5.6, z + 0.3, pal.metal, { area: A });
      mm.box(r.x0, 5.6, r.z0, r.x1, 6.5, r.z1, pal.roof, { area: A, top: pal.tank });
      mm.box(r.x0 - 0.05, 5.7, r.z1 - 0.05, r.x1 + 0.05, 6.4, r.z1 + 0.05, pal.trim, { area: A, glow: 0.6 });
      mm.box(r.x0 + 0.5, 5.5, r.z0 + 0.5, r.x1 - 0.5, 5.6, r.z1 - 0.5, pal.lamp, { glow: 1 });
      // its pumps, on islands of three a row (a second row from the fourth); AdBlue beside each
      const pumps = Math.max(1, p.fuel.pumps || 1);
      const rows = pumps > 3 ? [(r.z0 + r.z1) / 2 - 3.2, (r.z0 + r.z1) / 2 + 3.2] : [(r.z0 + r.z1) / 2];
      for (let k = 0; k < pumps; k++) {
        const row = rows.length > 1 ? Math.floor(k / Math.ceil(pumps / 2)) : 0;
        const inRow = rows.length > 1 ? (row ? pumps - Math.ceil(pumps / 2) : Math.ceil(pumps / 2)) : pumps;
        const i = row ? k - Math.ceil(pumps / 2) : k;
        const x = r.x0 + ((i + 1) * (r.x1 - r.x0)) / (inRow + 1);
        const zc = rows[row];
        mm.box(x - 1.6, 0, zc - 1.2, x + 1.6, 0.2, zc + 1.2, pal.concrete, { area: A });
        mm.box(x - 0.5, 0.2, zc - 0.35, x + 0.5, 2, zc + 0.35, pal.pump, { area: A });
        mm.box(x - 0.35, 1.3, zc + 0.35, x + 0.35, 1.8, zc + 0.4, pal.screen, { glow: 1 });
        if (up.has('adblue')) mm.box(x + 0.7, 0.2, zc - 0.25, x + 1.15, 1.5, zc + 0.25, pal.adblue, { area: A });
      }
      // the bulk tank, on its pad behind the canopy
      if (up.has('bulk_tank')) {
        mm.box(r.x1 - 5.2, 0, r.z0 - 0.2, r.x1 - 0.4, 0.3, r.z0 + 3.4, pal.concrete, { area: A });
        for (const x of [r.x1 - 3.8, r.x1 - 1.8]) mm.prism(x, r.z0 + 1.6, 0.85, 0.85, 0.3, 4.6, pal.tank, { n: 12, area: A });
      }
      if (o.works) works(mm, r, A, 6);
    });
    tag('fuel', f, 10, 10, 10, o);
  }

  // the parking: blocks of spaces, a charger at the head of the spaces it serves; the bus in
  // each space as the plan lists them
  const pk = p.parking;
  const per = Math.max(1, pk.per);
  const ps = shape('parking');
  let n = 0;
  of('parking').forEach((o, k) => {
    const first = n;
    const f = put(o, (mm, r, fr) => {
      const sw = (r.x1 - r.x0) / per;
      for (let s = 0; s <= per; s++) mm.flat(r.x0 + s * sw - 0.07, r.z0, r.x0 + s * sw + 0.07, r.z1, 0.05, pal.paint, AREAS.parking);
      mm.flat(r.x0, r.z0 - 0.07, r.x1, r.z0 + 0.07, 0.05, pal.paint, AREAS.parking);
      // over the spaces: a hall of concrete, else carports on their posts; the lights' masts
      if (up.has('hall')) {
        const hh = 5.2;
        mm.box(r.x0, 0, r.z0, r.x1, hh, r.z0 + 0.4, pal.garage, { area: AREAS.parking });
        for (const x of [r.x0, r.x1 - 0.4]) mm.box(x, 0, r.z0, x + 0.4, hh, r.z1, pal.garage, { area: AREAS.parking });
        for (let x = r.x0 + sw * 2; x < r.x1 - 1; x += sw * 2) mm.box(x - 0.25, 0, r.z1 - 0.6, x + 0.25, hh, r.z1 - 0.1, pal.garage, { area: AREAS.parking });
        mm.box(r.x0 - 0.2, hh, r.z0 - 0.2, r.x1 + 0.2, hh + 0.5, r.z1 + 0.3, pal.garage, { area: AREAS.parking, top: pal.townRoof });
        mm.box(r.x0, hh - 1.2, r.z1 - 0.1, r.x1, hh, r.z1 + 0.05, pal.trim, { area: AREAS.parking, glow: 0.3 });
        mm.box(r.x0 + 0.5, hh - 0.15, (r.z0 + r.z1) / 2 - 0.15, r.x1 - 0.5, hh - 0.05, (r.z0 + r.z1) / 2 + 0.15, pal.lamp, { glow: 1 });
      } else if (up.has('carports')) {
        for (let x = r.x0; x <= r.x1 + 0.01; x += sw * 2) for (const z of [r.z0 + 1, r.z1 - 1.5]) mm.box(x - 0.12, 0, z - 0.12, x + 0.12, 4.4, z + 0.12, pal.metal, { area: AREAS.parking });
        mm.box(r.x0 - 0.3, 4.4, r.z0 + 0.4, r.x1 + 0.3, 4.6, r.z1 - 0.8, pal.roof, { area: AREAS.parking });
        if (up.has('solar')) for (let x = r.x0 + 0.4; x < r.x1 - 2; x += 2.3) mm.box(x, 4.6, r.z0 + 1, x + 2.1, 4.72, r.z1 - 1.4, pal.solar, { area: AREAS.parking });
      }
      if (up.has('lights'))
        for (const x of [r.x0 + 0.5, r.x1 - 0.5]) {
          mm.box(x - 0.12, 0, r.z1 + 0.4, x + 0.12, 9, r.z1 + 0.64, pal.metal, { area: AREAS.parking });
          mm.box(x - 0.5, 8.7, r.z1 + 0.2, x + 0.5, 9.1, r.z1 + 0.84, pal.lamp, { glow: 1 });
        }
      // (oil where the buses stand)
      for (let s = 0; s < per; s++) if (wear() < 0.6) marks.append(mark(r.x0 + (s + 0.5) * sw, (r.z0 + r.z1) / 2 + (wear() - 0.5) * 4, 1.8 + wear() * 1.4, 2.4 + wear() * 1.6, wear() * 6, MAT.oil, 0.5 + wear() * 0.4), fr);
      for (let s = 0; s < per; s++) {
        if (first + s >= pk.chargers) break;
        const cx = r.x0 + (s + 0.5) * sw;
        mm.box(cx - 0.3, 0, r.z0 + 0.25, cx + 0.3, 1.7, r.z0 + 0.7, pal.charger, { area: AREAS.charging });
        mm.box(cx - 0.22, 1.25, r.z0 + 0.7, cx + 0.22, 1.55, r.z0 + 0.74, pal.ok, { area: AREAS.charging, glow: 1 });
      }
    });
    const sw = (ps.w * cell) / per;
    for (let s = 0; s < per; s++) {
      const bus = p.parked[n];
      if (bus) buses.push(place(bus, f, (s + 0.5) * sw, (ps.h * cell) / 2 + 0.5, 'parked'));
      n++;
    }
    if (k === 0) tag('parking', f, (ps.w * cell) / 2, 7, 10, o);
  });
  // the buses the yard has no space for: along the street, a warning ring round each
  const streetFrame = frame(0, 0, 0, 0, 0);
  p.parked.slice(n).filter(Boolean).slice(0, 10).forEach((bus, k) => buses.push(place(bus, streetFrame, bx0 + 12 + k * 14, street.z0 + 3, 'street', Math.PI / 2)));

  // trees on the verge, round the land but not on the street
  if (level.trees) {
    const rnd = seeded(7 + L.cols * 31 + L.rows);
    for (let x = bx0 + 4; x < bx1 - 2; x += 7 + rnd() * 5) tree(m, pal, rnd, x, bz0 + 4 + rnd() * 6);
    for (let z = bz0 + 14; z < street.z0 - 6; z += 8 + rnd() * 6) {
      tree(m, pal, rnd, bx0 + 4 + rnd() * 7, z);
      tree(m, pal, rnd, bx1 - 4 - rnd() * 7, z);
    }
  }

  return { mesh: m, decals: marks, buses, labels, picks, bounds: { x0: bx0, x1: bx1, z0: bz0, z1: bz1 }, world, yard: { w: W, d: D } };
}

/** A mark on the ground in a module's own frame (appended turned with it: `Mesh.decal`). */
function mark(x, z, w, l, turn, mat, s) {
  const m = new Mesh();
  m.decal(x, z, w, l, turn, mat, s);
  return m;
}

/** A bus where it stands, in a module's frame: its centre, its heading (0: nose to the
 * module's front) and what it is. */
function place(bus, f, lx, lz, at, heading = 0) {
  const [x, , z] = f.pt([lx, 0, lz]);
  const len = LENGTH[bus.size] || LENGTH.solo;
  return { id: bus.id, number: bus.number, tip: bus.tip, file: bus.file || '', paint: bus.paint || '', out: !!bus.out, x, z, heading: f.heading(heading), len, height: bus.size === 'double' ? 4.3 : 3.1, articulated: bus.size === 'articulated', at };
}

/** The module of the yard under a point of the ground (metres), if any. */
export function moduleUnder(picks, x, z) {
  return picks.find((o) => x >= o.x0 && x <= o.x1 && z >= o.z0 && z <= o.z1) || null;
}

/** How high the ghost of a module of `area` stands (metres). */
export const ghostHeight = (area) => ({ parking: 0.6, workshop: 8.5, wash: 6, fuel: 6.5, offices: 8 })[area] || 3;

/** The ghost of a module being put down: a block of its shape (the renderer tints it) with a
 * band at its front, and its aprons on the ground. */
export function ghostMesh(L, area, x, y, rot, pal) {
  const cell = L.cell;
  const m = new Mesh();
  const s = L.shapes[area];
  if (!s) return m;
  const lw = s.w * cell;
  const ld = s.h * cell;
  const f = frame(rot, x * cell, y * cell, lw, ld);
  const lm = new Mesh();
  const h = ghostHeight(area);
  lm.box(0.2, 0.06, 0.2, lw - 0.2, h, ld - 0.2, pal.wall);
  lm.box(0.2, 0.06, ld - 0.6, lw - 0.2, Math.min(h, 1.2) + 0.05, ld - 0.2, pal.paint);
  if (s.front) lm.flat(0, ld, lw, ld + s.front * cell, 0.08, pal.paint);
  if (s.back) lm.flat(0, -s.back * cell, lw, 0, 0.08, pal.paint);
  m.append(lm, f);
  return m;
}

/** The land's grid, every apron and the gate's lane, for the build mode. */
export function gridMesh(L, pal) {
  const cell = L.cell;
  const m = new Mesh();
  const W = L.cols * cell;
  const D = L.rows * cell;
  for (let x = 0; x <= L.cols; x += 2) m.flat(x * cell - 0.04, 0, x * cell + 0.04, D, 0.06, pal.paint);
  for (let y = 0; y <= L.rows; y += 2) m.flat(0, y * cell - 0.04, W, y * cell + 0.04, 0.06, pal.paint);
  const box = (a, col, w) => m.dashed(a.x * cell + 0.3, a.y * cell + 0.3, (a.x + a.w) * cell - 0.3, (a.y + a.h) * cell - 0.3, col, 0, { w, y: 0.07 });
  for (const o of L.modules) for (const a of o.aprons) box(a, pal.warn, 0.2);
  box(L.gate, pal.barrier, 0.25);
  return m;
}
