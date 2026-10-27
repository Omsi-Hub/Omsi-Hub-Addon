// The depot page, as our game-drawn launcher's (`company/depot.rs`): its four figures; the yard
// drawn from above - the offices, the workshop hall with its bays, the washing bay, the diesel
// station, the parking's blocks with their chargers, and the buses where they stand tonight -;
// the areas to build, level by level (a bigger depot); and the workshop's jobs. Calm, as the
// other company pages: what is built is drawn in the card's quiet greys with a hairline, what
// could be built is a dashed plot, building work is amber, the buses wear the company's colour.
// A bus clicked on the plan is picked for the workshop's list (and clicked again let go).
//
// The yard is drawn in 3D (`depot3d/`) where the computer can, or from straight above (2D); the
// plan drawn flat stays where there is no WebGL2. Or the depot as a dashboard: the buses where
// they stand tonight, the staff, and each area with what is asked of it against what it has. A row of the Build list under the pointer
// lights its area up in the yard; a module more (a block of spaces, a bay, a wash lane ...) is
// put down where the player wants it (the build mode: `placing`), more land bought. A row opens
// to what can be added to its area (the upgrades: a paint booth, a gantry, a bulk tank ...); a
// new company's building credit stands over the list while some is left, and the last row is
// the depot's look - its walls', roofs', trims' and doors' colours and what its walls are of.

import { html, tr, ui, setUi, useSize, textW, Icon, Btn, Section, Figure, Tag, Meter, Plate, IconBtn, Seg, rgbOf, inkOn, fill } from './kit.js';
import { useMore, actMore, TextIn, box, fillOf, useListSize } from './more.js';
import { Depot3D } from './depot3d/view.js';

const ASPHALT = [25, 31, 44];
const ROOF = [38, 46, 62];
const STREET = [31, 37, 50];
const WARN = '232, 170, 70';
const OK = '52, 178, 110';
const EARLY = '42, 117, 247';
const EARLY_SOFT = '122, 168, 255';
const faint = (a) => `rgba(var(--co-faint-rgb), ${a})`;

export function DepotPage() {
  const st = ui('depot', { selected: null, view: '3d', hover: null, no3d: false, placing: null, open: null });
  const v = useMore('depot', { selected: st.selected });
  const [ref, size] = useSize();
  let body = null;
  if (v?.figures && size.w > 0) {
    const W = size.w;
    const H = size.h;
    const gap = 16;
    const leftW = Math.max(420, (W - gap) * 0.56);
    const rx = leftW + gap;
    const rw = Math.max(0, W - leftW - gap);
    const fh = 100;
    const fw = (leftW - 3 * 12) / 4;
    const bh = Math.min(6 * 62 + 56, H * 0.62);
    const pw = leftW - 36;
    const ph = Math.max(0, H - fh - 12 - 58);
    const names = Object.fromEntries(v.build.rows.map((a) => [a.key, { label: a.label, icon: a.icon }]));
    const pick = (id) => setUi('depot', { selected: st.selected === id ? null : id });
    const views = st.no3d ? [['2d', tr('Plan')], ['dash', tr('Dashboard')]] : [['3d', tr('3D')], ['2d', tr('2D')], ['dash', tr('Dashboard')]];
    const tools = html`<${Seg} value=${st.view} options=${views} onChange=${(view) => setUi('depot', { view, placing: null })} style=${`height:26px;width:${st.no3d ? 170 : 200}px`} />`;
    const dash = st.view === 'dash' && v.dashboard;
    body = html`
      ${v.figures.map((f, k) => html`<div style=${box(k * (fw + 12), 0, fw, fh)}><${Figure} f=${f} small=${fw < 170} /></div>`)}
      <${Section} title=${dash ? tr('The depot at a glance') : v.plan.title} tools=${tools} style=${box(0, fh + 12, leftW, Math.max(0, H - fh - 12))}>
        ${dash
          ? html`<${Dashboard} d=${v.dashboard} w=${pw} h=${ph} hover=${st.hover} />`
          : !st.no3d && ph >= 120 && v.plan.layout
          ? html`<${Depot3D} p=${v.plan} names=${names} w=${pw} h=${ph} selected=${st.selected} hover=${st.hover} onPick=${pick} onFail=${() => setUi('depot', { no3d: true, placing: null })} placing=${st.placing} onPlaced=${() => setUi('depot', { placing: null })} flat=${st.view === '2d'} />`
          : html`<${Plan} p=${v.plan} w=${pw} h=${ph} selected=${st.selected} />`}
      </${Section}>
      <${Section} title=${v.build.title} style=${box(rx, 0, rw, bh)}><${Build} b=${v.build} land=${v.plan.layout?.land} look=${v.plan.look} place=${!st.no3d && !!v.plan.layout} placing=${st.placing} open=${st.open} /></${Section}>
      <${Section} title=${v.workshop.title} style=${box(rx, bh + 12, rw, Math.max(0, H - bh - 12))}><${Workshop} ws=${v.workshop} selected=${st.selected} /></${Section}>`;
  }
  return html`<div class="cm-page" ref=${ref}>${body}</div>`;
}

// --- the plan -----------------------------------------------------------------------------

/** A plot that could be built on: a dashed outline, each side's dashes from its corner
 * (`depot::dashed`). */
function dashed(r, colour) {
  const dash = 6;
  let d = '';
  const seg = (ax, ay, bx, by) => {
    const len = Math.hypot(bx - ax, by - ay);
    const n = Math.max(1, Math.floor(len / (dash * 2)));
    const dx = (bx - ax) / Math.max(len, 1e-3);
    const dy = (by - ay) / Math.max(len, 1e-3);
    for (let k = 0; k < n; k++) {
      const s = k * dash * 2;
      const e = s + Math.min(dash, len - s);
      d += `M${ax + dx * s} ${ay + dy * s}L${ax + dx * e} ${ay + dy * e}`;
    }
  };
  seg(r.x, r.y, r.x + r.w, r.y);
  seg(r.x + r.w, r.y, r.x + r.w, r.y + r.h);
  seg(r.x + r.w, r.y + r.h, r.x, r.y + r.h);
  seg(r.x, r.y + r.h, r.x, r.y);
  return html`<path d=${d} stroke=${colour} stroke-width="1" fill="none" />`;
}

const rr = (r, rad, f, extra = {}) => html`<rect x=${r.x} y=${r.y} width=${Math.max(0, r.w)} height=${Math.max(0, r.h)} rx=${rad} fill=${f} ...${extra} />`;
const edge = (r, rad, w, colour) => html`<rect x=${r.x + w / 2} y=${r.y + w / 2} width=${Math.max(0, r.w - w)} height=${Math.max(0, r.h - w)} rx=${Math.max(0, rad - w / 2)} fill="none" stroke=${colour} stroke-width=${w} />`;
const inset = (r, d) => ({ x: r.x + d, y: r.y + d, w: r.w - 2 * d, h: r.h - 2 * d });

/** A building of the plan: its roof, its name, how far it is built (`depot::building`). */
function building(r, b, svg, words) {
  if (b.built) {
    svg.push(rr(r, 6, fillOf(ROOF)));
    svg.push(edge(r, 6, 1, 'var(--hub-hairline)'));
  } else svg.push(dashed(r, faint(0.7)));
  if (b.works) {
    svg.push(edge(inset(r, 2), 5, 1.5, `rgba(${WARN}, 0.8)`));
    words.push(html`<${TextIn} x=${r.x + 8} y=${r.y + r.h - 18} w=${r.w - 16} h=${14} px=${12.5} weight=${700} tone="warn">${b.works}</${TextIn}>`);
  }
  words.push(html`<${TextIn} x=${r.x + 8} y=${r.y + 4} w=${r.w - 16} h=${14} px=${12} weight=${700} tone=${b.built ? 'dim' : 'faint'}>${b.head}</${TextIn}>`);
}

/** A bus seen from above: the company's colour, its roof's hatch, its number where there is
 * room; lifted when it is the one picked (`depot::bus_top`). */
function busTop(r, bus, colour, lifted, pick) {
  const rgb = rgbOf(colour);
  const fs = Math.min(11, Math.max(7, r.w * 0.42));
  return html`<g class="cm-bus" onClick=${() => pick(bus.id)}><title>${bus.tip}</title>
    <rect x=${r.x - 2} y=${r.y - 2} width=${r.w + 4} height=${r.h + 4} fill="transparent" />
    ${lifted && rr(inset(r, -2), 4, 'var(--hub-text)')}
    ${rr(r, 3, `rgb(${rgb.join(',')})`)}
    <rect x=${r.x + 2} y=${r.y + 3} width=${Math.max(0, r.w - 4)} height=${Math.max(2, r.h * 0.12)} fill="rgba(0,0,0,0.22)" />
    ${r.h >= 30 && r.w >= 14 && html`<text x=${r.x + r.w / 2} y=${r.y + r.h * 0.6} font-size=${fs / 1.303} font-weight="700" fill=${inkOn(rgb)} text-anchor="middle" dominant-baseline="middle">${bus.number}</text>`}
  </g>`;
}

function Plan({ p, w, h, selected }) {
  if (h < 120 || w <= 0) return null;
  const pick = (id) => setUi('depot', { selected: selected === id ? null : id });
  const svg = [];
  const words = [];
  const colour = p.colour;
  const streetH = 30;
  const yard = { x: 0, y: 0, w, h: h - streetH - 10 };
  const street = { x: -6, y: yard.h + 10, w: w + 12, h: streetH };
  svg.push(rr(yard, 8, fillOf(ASPHALT)));
  svg.push(edge(yard, 8, 1, 'var(--hub-hairline)'));
  svg.push(rr(street, 0, fillOf(STREET)));
  for (let x = street.x + 8; x < street.x + street.w - 8; x += 26) svg.push(rr({ x, y: street.y + street.h / 2 - 0.75, w: 14, h: 1.5 }, 0, faint(0.5)));
  // the gate
  const gate = { x: yard.w / 2 - 30, y: yard.h - 1.5, w: 60, h: 3 };
  svg.push(rr(gate, 0, fillOf(ASPHALT)));
  svg.push(rr({ x: gate.x, y: gate.y + gate.h + 2, w: gate.w, h: street.y - gate.y - gate.h - 2 }, 0, fillOf(ASPHALT)));
  const buses = [];
  // the buildings along the top
  const pad = 12;
  const g = 10;
  const top = { x: pad, y: pad, w: yard.w - 2 * pad, h: Math.max(80, yard.h * 0.34) };
  const ow = top.w * 0.2;
  const ww = top.w * 0.48;
  const sw = top.w - ow - ww - 2 * g;
  const offices = { x: top.x, y: top.y, w: ow, h: top.h };
  building(offices, p.offices, svg, words);
  if (p.offices.built) {
    for (let k = 0; k < p.offices.floors; k++) svg.push(rr({ x: offices.x + 8, y: offices.y + 22 + k * 12, w: offices.w - 16, h: 8 }, 2, faint(0.35)));
    words.push(html`<${TextIn} x=${offices.x + 8} y=${offices.y + offices.h - 18} w=${offices.w - 16} h=${14} px=${12.5} tone="dim">${p.offices.room}</${TextIn}>`);
  }
  // the hall: a bay a level, the buses being worked on in them, the ones waiting before it
  const hall = { x: offices.x + ow + g, y: top.y, w: ww, h: top.h };
  building(hall, p.hall, svg, words);
  const bayW = (hall.w - 16) / p.hall.max;
  for (let k = 0; k < p.hall.max; k++) {
    const br = { x: hall.x + 8 + k * bayW + 2, y: hall.y + 22, w: bayW - 4, h: hall.h - 30 };
    if (k < p.hall.bays) {
      svg.push(rr(br, 3, fillOf(ASPHALT)));
      svg.push(rr({ x: br.x + br.w / 2 - 2, y: br.y + 6, w: 4, h: br.h - 12 }, 0, 'rgba(0,0,0,0.35)'));
      const bus = p.in_ws[k];
      if (bus) {
        const bw = Math.min(br.w * 0.62, 26);
        buses.push(busTop({ x: br.x + br.w / 2 - bw / 2, y: br.y + 6, w: bw, h: Math.min(br.h - 12, bw * 3.4) }, bus, colour, selected === bus.id, pick));
      }
    } else svg.push(dashed(br, faint(0.35)));
  }
  p.in_ws.slice(p.hall.bays, p.hall.bays + 6).forEach((bus, k) => {
    buses.push(html`<g class="cm-bus" onClick=${() => pick(bus.id)}><title>${bus.tip}</title>${rr({ x: hall.x + 8 + k * 30, y: hall.y + hall.h + 4, w: 24, h: 14 }, 3, `rgba(${WARN}, 0.8)`)}</g>`);
  });
  // the washing bay and the diesel station
  const side = { x: hall.x + ww + g, y: top.y, w: sw, h: top.h };
  const wash = { x: side.x, y: side.y, w: side.w, h: (side.h - g) * 0.5 };
  building(wash, p.wash, svg, words);
  if (p.wash.built) {
    const lane = { x: wash.x + 10, y: wash.y + 22, w: wash.w - 20, h: wash.h - 30 };
    svg.push(rr(lane, 3, `rgba(${EARLY}, 0.12)`));
    if (p.wash.gates) for (let k = 0; k < 2; k++) svg.push(rr({ x: lane.x + lane.w * (0.35 + 0.3 * k) - 1.5, y: lane.y, w: 3, h: lane.h }, 0, `rgba(${EARLY_SOFT}, 0.6)`));
  }
  const fuel = { x: side.x, y: wash.y + wash.h + g, w: side.w, h: (side.h - g) * 0.5 };
  building(fuel, p.fuel, svg, words);
  if (p.fuel.built) for (let k = 0; k < p.fuel.pumps; k++) svg.push(rr({ x: fuel.x + 16 + k * 18 - 5, y: fuel.y + fuel.h / 2 + 6 - 8, w: 10, h: 16 }, 2, 'rgba(var(--co-soft-rgb), 0.7)'));
  // the parking: rows of two blocks of twelve spaces, a level a block
  const pk = p.parking;
  const park = { x: pad, y: top.y + top.h + 26, w: yard.w - 2 * pad, h: Math.max(40, yard.h - pad - top.y - top.h - 26) };
  words.push(html`<${TextIn} x=${park.x} y=${park.y - 18} w=${park.w * 0.5} h=${14} px=${12} weight=${700} tone="dim">${pk.label}</${TextIn}>`);
  if (pk.works) words.push(html`<${TextIn} x=${park.x + park.w * 0.5} y=${park.y - 18} w=${park.w * 0.5} h=${14} px=${12.5} weight=${700} tone="warn" align="right">${pk.works}</${TextIn}>`);
  const rows = Math.floor(pk.blocks / 2);
  const bg = 14;
  const blockW = (park.w - bg) / 2;
  const blockH = Math.max(16, (park.h - (rows - 1) * 8) / rows);
  const slotW = blockW / pk.per;
  let n = 0;
  for (let b = 0; b < pk.blocks; b++) {
    const br = { x: park.x + (b % 2) * (blockW + bg), y: park.y + Math.floor(b / 2) * (blockH + 8), w: blockW, h: blockH };
    const built = b < pk.built;
    const buildingNow = !built && b === pk.built && pk.building;
    if (built) {
      for (let s = 0; s < pk.per; s++) {
        const sr = { x: br.x + s * slotW, y: br.y, w: slotW, h: br.h };
        svg.push(rr({ x: sr.x, y: sr.y, w: 1, h: sr.h }, 0, faint(0.55)));
        if (s + 1 === pk.per) svg.push(rr({ x: sr.x + sr.w - 1, y: sr.y, w: 1, h: sr.h }, 0, faint(0.55)));
        // (a charger at the head of every space it serves)
        if (n < pk.chargers) svg.push(html`<circle cx=${sr.x + sr.w / 2} cy=${sr.y + 3} r="2.2" fill=${`rgb(${OK})`} />`);
        const bus = p.parked[n];
        if (bus && !bus.out) {
          const bw = Math.min(slotW * 0.66, 22);
          const bh = Math.min(sr.h - 10, bw * 3.6);
          buses.push(busTop({ x: sr.x + sr.w / 2 - bw / 2, y: sr.y + 6, w: bw, h: bh }, bus, colour, selected === bus.id, pick));
        }
        n++;
      }
    } else svg.push(dashed(br, buildingNow ? `rgba(${WARN}, 0.8)` : faint(0.3)));
  }
  // the buses the yard has no space for: in the street
  p.parked.slice(n).filter((bus) => bus && !bus.out).slice(0, 12).forEach((bus, k) => {
    const r = { x: street.x + 10 + k * 40, y: street.y + 7, w: 34, h: street.h - 14 };
    const rgb = rgbOf(colour);
    buses.push(html`<g class="cm-bus" onClick=${() => pick(bus.id)}><title>${bus.tip}</title>${rr(inset(r, -1.5), 3, `rgb(${WARN})`)}${rr(r, 3, `rgb(${rgb.join(',')})`)}</g>`);
  });
  return html`<div class="cm-plan" style=${`width:${w}px;height:${h}px`}>
    <svg width=${w} height=${h} style="position:absolute;left:0;top:0;overflow:visible">${svg}${buses}</svg>
    ${words}
  </div>`;
}

// --- the dashboard ---------------------------------------------------------------------------

/** A meter's state as an icon beside its words (never the colour alone). */
const STATE = { ok: 'check_circle', warn: 'warning', danger: 'error' };

/** The depot as a dashboard (`company_more::depot::dashboard`): the buses tonight and the staff
 * in small figures, the money, then a card an area - its level, a meter of what is asked of it
 * against what it has, the building work on it and what was added to it. An area's card under
 * the pointer lights its row of the Build list, as the yard does. */
function Dashboard({ d, w, h, hover }) {
  const tile = (t) => html`<div class="dp-dash-tile" title=${`${t.label}: ${t.value}`}>
    <span class="t-soft"><${Icon} name=${t.icon} size=${16} /></span><b class=${'t-' + (t.tone || 'text')}>${t.value}</b><small>${t.label}</small>
  </div>`;
  const card = (a) => {
    const m = a.meter;
    const lit = (on) => () => setUi('depot', { hover: on ? a.key : null });
    return html`<div class=${'dp-dash-card' + (hover === a.key ? ' lit' : '')} onMouseEnter=${lit(true)} onMouseLeave=${lit(false)}>
      <div class="head">
        <span class="t-accent2"><${Icon} name=${a.icon} size=${20} /></span>
        <b class="grow cut">${a.label}</b>
        <span class="cm-pips" title=${a.level} style="position:static">${Array.from({ length: a.max }, (_, i) => html`<i class=${i < a.lv ? 'on' : i === a.lv && a.works ? 'works' : ''}></i>`)}</span>
      </div>
      <div class="load" title=${m.pct ? `${m.line} (${m.pct})` : m.line}>
        <${Meter} frac=${m.frac} tone=${m.tone} h=${8} />
        <span class=${'t-' + (m.tone === 'dim' ? 'faint' : m.tone === 'danger' ? 'danger' : m.tone === 'warn' ? 'warn' : 'soft')}>
          ${STATE[m.tone] && html`<${Icon} name=${STATE[m.tone]} size=${14} />`}<span class="cut">${m.line}</span>${m.pct && html`<em>${m.pct}</em>`}
        </span>
      </div>
      ${a.works && html`<div class="works t-warn"><${Icon} name="construction" size=${14} /><span class="cut">${a.works}</span></div>`}
      ${a.added.length > 0 && html`<div class="added">${a.added.map((u) => html`<span class="dp-dash-chip" title=${u.label}><${Icon} name=${u.icon} size=${13} /><span class="cut">${u.label}</span></span>`)}</div>`}
    </div>`;
  };
  return html`<div class="co-scroll dp-dash" style=${`width:${w}px;height:${h}px`}>
    <div class="dp-dash-top">
      <div class="dp-dash-group"><div class="co-caps">${d.fleet_title}</div><div class="dp-dash-tiles">${d.fleet.map(tile)}</div></div>
      <div class="dp-dash-group"><div class="co-caps">${d.people_title}</div><div class="dp-dash-tiles">${d.people.map(tile)}</div></div>
    </div>
    <div class="dp-dash-money">${d.money.map((m) => html`<div><span class="co-caps">${m.label}</span><b class=${'t-' + (m.tone || 'text')}>${m.value}</b></div>`)}</div>
    <div class="dp-dash-areas">${d.areas.map(card)}</div>
  </div>`;
}

// --- the buildings ---------------------------------------------------------------------------

/** The look's colours as the depot has them unless the company chose its own (`palette.js`). */
const OWN = [
  ['wall', 'Walls', '#e9ebef'],
  ['roof', 'Roofs', '#b7bec9'],
  ['trim', 'Trims', '#2a75f7'],
  ['door', 'Doors', '#8c95a3'],
];
const FINISHES = [
  ['cladding', 'Steel cladding'],
  ['brick', 'Brick'],
  ['plaster', 'Plaster'],
];

function Build({ b, land, look, place, placing, open }) {
  const [ref, size] = useListSize();
  const W = Math.max(0, size.w - 10);
  // (more land, beside the areas: bought at once; the look, last)
  const rows = land ? [...b.rows, { key: 'land', icon: 'add', label: tr('More land'), effect: land.tip, cost: land.cost, buyLand: true, tip: land.tip }] : [...b.rows];
  if (look) rows.push({ key: 'look', icon: 'palette', label: tr("The depot's look"), effect: tr(FINISHES.find(([k]) => k === look.finish)?.[1] || 'Steel cladding'), look: true });
  const go = (a) => {
    if (a.buyLand) return actMore('depot_land');
    // (a module more is put down where the player wants it, in the yard; without it on its
    // first free spot)
    if (a.place && place && !a.locked) return setUi('depot', { placing: placing?.area === a.key && !placing.move ? null : { area: a.key }, selected: null });
    return actMore('depot_build', { area: a.key });
  };
  const toggle = (key) => setUi('depot', { open: open === key ? null : key });
  const setLook = (patch) => actMore('depot_look', { ...look, ...patch });
  const items = [];
  let y = 0;
  // the building credit, while some is left; what to build first on an empty plot
  if (b.credit) {
    items.push(html`<div class="cm-credit" style=${box(0, y, W, 40)} title=${b.credit.tip}>
      <span class="cm-icon t-ok" style="left:8px;top:9px"><${Icon} name="payments" size=${20} /></span>
      <${TextIn} x=${38} y=${10} w=${W - 48} h=${20} px=${13.5} weight=${600}>${tr('Building credit left')}: ${b.credit.amount}</${TextIn}>
    </div>`);
    y += 46;
  }
  if (b.hint) {
    items.push(html`<p class="co-para cm-hint" style=${box(0, y, W, 40) + ';margin:0'}>${b.hint}</p>`);
    y += 46;
  }
  rows.forEach((a, k) => {
    const lw = textW(a.label, 14.5, 700);
    const tw = a.level ? textW(a.level, 12, 700) + 16 : 0;
    const lx = 38 + lw + 12;
    const pips = a.max > 0 && a.max <= 10 ? a.max * 9 + 4 : 0;
    const lit = (on) => () => setUi('depot', { hover: on ? a.key : null });
    const more = a.upgrades?.length > 0 || a.look;
    const opened = open === a.key;
    const added = a.upgrades?.filter((u) => u.built).length || 0;
    items.push(html`<div style=${box(0, y, W, 56)} class="cm-row cm-buildrow" onMouseEnter=${lit(true)} onMouseLeave=${lit(false)}>
      ${k > 0 && html`<i class="cm-hair" style="top:-3px"></i>`}
      <span class="cm-icon t-accent2" style="left:4px;top:9px"><${Icon} name=${a.icon} size=${22} /></span>
      <${TextIn} x=${38} y=${6} w=${W - 260} h=${24} px=${14.5} weight=${700}>${a.label}</${TextIn}>
      ${!a.level
        ? null
        : pips
        ? lx + pips < W - 220 && html`<span class="cm-pips" title=${a.level} style=${`left:${lx}px;top:12px`}>${Array.from({ length: a.max }, (_, i) => html`<i class=${i < a.lv ? 'on' : i === a.lv && a.works ? 'works' : ''}></i>`)}</span>`
        : lx + tw < W - 220 && html`<span class="cm-icon" style=${`left:${lx}px;top:7px`}><${Tag} text=${a.level} tone="soft" /></span>`}
      <${TextIn} x=${38} y=${32} w=${W - 260} h=${20} px=${13.5} tone="soft">${a.effect}${added ? `  ·  +${added}` : ''}</${TextIn}>
      ${more && html`<${IconBtn} icon=${opened ? 'expand_less' : 'expand_more'} size=${18} title=${a.look ? tr('Choose the look') : tr('What can be added')} style=${`position:absolute;right:188px;top:13px;width:30px;height:30px`} onClick=${() => toggle(a.key)} />`}
      ${!a.look && html`<span class="cm-act" style="right:0;top:9px;width:180px;height:38px">
        ${a.works
          ? html`<span class="cm-said t-warn">${a.works}</span>`
          : a.cost
            ? html`<${Btn} icon=${a.locked ? 'lock' : a.place && place ? 'open_with' : 'add'} label=${a.cost} title=${a.tip} kind=${placing?.area === a.key && !placing.move ? 'on' : ''} style="width:180px;height:38px" onClick=${() => go(a)} />`
            : html`<span class="cm-said t-ok">${a.complete}</span>`}
      </span>`}
    </div>`);
    y += 62;
    if (!opened) return;
    // what can be added to the area
    for (const u of a.upgrades || []) {
      items.push(html`<div style=${box(24, y, W - 24, 48)} class="cm-row cm-uprow">
        <span class=${'cm-icon ' + (u.built ? 't-ok' : 't-soft')} style="left:4px;top:12px"><${Icon} name=${u.built ? 'check_circle' : u.icon} size=${18} /></span>
        <${TextIn} x=${32} y=${3} w=${W - 250} h=${20} px=${13.5} weight=${700}>${u.label}</${TextIn}>
        <${TextIn} x=${32} y=${24} w=${W - 250} h=${18} px=${12.5} tone="soft">${u.what}</${TextIn}>
        <span class="cm-act" style="right:0;top:7px;width:160px;height:34px">
          ${u.built
            ? html`<span class="cm-said t-ok">${tr('Added')}</span>`
            : u.works
              ? html`<span class="cm-said t-warn">${u.works}</span>`
              : u.needs
                ? html`<span class="cm-said t-faint">${u.needs}</span>`
                : u.cost && html`<${Btn} icon="add" label=${u.cost} title=${u.busy ? tr('Wait for the building work on this area') : u.tip} disabled=${u.busy} style="width:160px;height:34px" onClick=${() => actMore('depot_upgrade', { key: u.key })} />`}
        </span>
      </div>`);
      y += 52;
    }
    // the look: a colour each (or the depot's own), what the walls are of
    if (a.look) {
      for (const [key, label, own] of OWN) {
        const mine = look[key];
        items.push(html`<div style=${box(24, y, W - 24, 40)} class="cm-row cm-uprow">
          <${TextIn} x=${4} y=${10} w=${W - 220} h=${20} px=${13.5} weight=${600}>${tr(label)}</${TextIn}>
          <input type="color" class="cm-colour" value=${mine || own} title=${tr(label)} style="position:absolute;right:96px;top:5px;width:44px;height:30px" onChange=${(e) => setLook({ [key]: e.currentTarget.value })} />
          <${Btn} kind="ghost" label=${tr('Default')} disabled=${!mine} style="position:absolute;right:0;top:4px;width:88px;height:32px;padding:0" onClick=${() => setLook({ [key]: null })} />
        </div>`);
        y += 44;
      }
      items.push(html`<div style=${box(24, y, W - 24, 40)} class="cm-row cm-uprow">
        <${TextIn} x=${4} y=${10} w=${W - 340} h=${20} px=${13.5} weight=${600}>${tr('Walls of')}</${TextIn}>
        <span style="position:absolute;right:0;top:4px"><${Seg} value=${look.finish} options=${FINISHES.map(([k, l]) => [k, tr(l)])} onChange=${(finish) => setLook({ finish })} style="height:30px;width:300px" /></span>
      </div>`);
      y += 48;
    }
  });
  return html`<div class="co-scroll cm-list" ref=${ref}>
    <div style=${`position:relative;height:${y}px`}>${size.w > 0 && items}</div>
  </div>`;
}

// --- the workshop ---------------------------------------------------------------------------

function Workshop({ ws, selected }) {
  const [ref, size] = useListSize();
  if (ws.empty) return html`<p class="co-para" style="font-size:var(--co-rows)">${ws.empty}</p>`;
  const W = Math.max(0, size.w - 10);
  const rows = [];
  let y = 0;
  for (const j of ws.jobs) {
    rows.push(html`<div class="cm-row" style=${box(0, y, W, 50)}>
      <span style="position:absolute;left:0;top:4px"><${Plate} p=${{ number: j.number }} h=${20} /></span>
      <${TextIn} x=${plateW(j.number) + 10} y=${3} w=${W - plateW(j.number) - 140} h=${22} px=${14.5} weight=${700}>${j.title}</${TextIn}>
      <${TextIn} x=${plateW(j.number) + 10} y=${27} w=${W - plateW(j.number) - 140} h=${20} px=${13.5} weight=${500} tone=${j.tone}>${j.what}</${TextIn}>
      ${j.cost && html`<${TextIn} x=${W - 120} y=${2} w=${80} h=${18} px=${13.5} weight=${500} tone="soft" align="right">${j.cost}</${TextIn}>`}
      ${j.cancel && html`<${IconBtn} icon="close" size=${15} title=${ws.cancel_tip} style=${`position:absolute;left:${W - 30}px;top:2px;width:28px;height:28px`} onClick=${() => actMore('depot_cancel', { id: j.id })} />`}
    </div>`);
    y += 56;
  }
  if (ws.jobs.length) rows.push(html`<i class="cm-hair" style=${`top:${y - 3}px;width:${W}px`}></i>`);
  for (const b of ws.buses) {
    const on = selected === b.id;
    const pw = plateW(b.number);
    const lw = W * 0.42;
    const bw = Math.min((W * 0.58 - 12) / 3, 110);
    const subTone = b.tone === 'warn' ? 'warn' : on ? 'onaccent' : 'dim';
    rows.push(html`<div class="cm-row" style=${box(0, y, W, 50)}>
      <button type="button" class=${'cm-pick' + (on ? ' on' : '')} style=${box(0, 0, lw, 50)} onClick=${() => setUi('depot', { selected: on ? null : b.id })}></button>
      <span style="position:absolute;left:6px;top:4px;pointer-events:none"><${Plate} p=${{ number: b.number }} h=${20} /></span>
      <${TextIn} x=${pw + 14} y=${3} w=${lw - pw - 18} h=${22} px=${14} weight=${700} tone=${on ? 'onaccent' : 'text'} style="pointer-events:none">${b.name}</${TextIn}>
      <${TextIn} x=${pw + 14} y=${27} w=${110} h=${20} px=${13.5} weight=${500} tone=${subTone} style="pointer-events:none">${b.sub}</${TextIn}>
      <span title=${b.cond.tip} style=${box(pw + 128, 35, Math.max(10, lw - pw - 136), 5)}><${Meter} frac=${b.cond.frac} tone=${b.cond.tone} h=${5} /></span>
      ${b.jobs.map((k, i) => html`<${Btn} kind="ghost" label=${k.label} title=${k.tip} style=${box(W - (3 - i) * (bw + 4), 8, bw, 34) + ';padding:0'} onClick=${() => actMore('depot_order', { id: b.id, kind: k.kind })} />`)}
    </div>`);
    y += 56;
  }
  return html`<div class="co-scroll cm-list" ref=${ref}><div style=${`position:relative;height:${y}px`}>${size.w > 0 && rows}</div></div>`;
}

/** The width of a line's plate 20 high (`company::plate`). */
const plateW = (number) => Math.max(20 * 1.8, textW(number, 12, 900) + 14);
