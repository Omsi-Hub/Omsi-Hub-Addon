// The company board, the page's side (docs/ONLINE.md "The company board"; the Rust side is
// crates/hub-api/src/board.rs): the account panel's "Company leaderboard" beside the cloud saves -
// the driver's companies, each switched onto omsihub.com's company board or off it (at most 5).
// The same switch is in each company's settings. Rust sends the figures and keeps them up to date.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call, openUrl } from '../api.js';
import { language } from '../i18n.js';
import { Btn, Switch, toast, tr } from './bits.js';

const ask = (name, args = {}) => call(name, { ...args, lang: language() });

/** The driver's companies on the company board, or not. */
export function CompanyBoard() {
  const [v, setV] = useState(null);
  const [busy, setBusy] = useState('');
  const [read, setRead] = useState(0);
  useEffect(() => {
    ask('ol_companies')
      .then(setV)
      .catch(() => {});
  }, [read]);
  if (!v) return null;
  const flip = async (id, on) => {
    setBusy(id);
    let r;
    try {
      r = await ask('ol_company_board', { id, on });
    } catch (e) {
      r = { error: String(e?.message || e) };
    }
    setBusy('');
    if (r?.error) toast(r.error, r.code === 'offline' ? 'info' : 'error');
    else if (r?.message) toast(r.message, 'ok');
    setRead((n) => n + 1);
  };
  const list = v.companies || [];
  return html`<section class="hubcs-panel">
    <div class="ol-caps"><b>${tr('Company leaderboard')}</b><span>${tr('%{n} of %{max} shown', { n: v.count || 0, max: v.max || 5 })}</span></div>
    ${v.blocked && html`<p class="hubcs-quiet">${v.blocked}</p>`}
    ${!list.length && html`<p class="hubcs-quiet">${tr('No companies yet: found one, then show it here.')}</p>`}
    ${list.map((c) => html`<div class="hubcs-slot"><div class="hubcs-who">
      <${Switch} checked=${c.on} disabled=${!!busy || !!v.blocked || (!c.on && v.count >= v.max)} label=${c.label} onChange=${(on) => flip(c.id, on)} /></div></div>`)}
    <p class="hubcs-note">${tr('A company on the board shows its name, colour, map and figures (this month\'s profit and passengers, cash, buses, lines, staff, trips on time and reputation) on omsihub.com, with your display name. Off until you switch it on.')}</p>
    ${v.url && html`<div class="ol-row"><${Btn} kind="ghost" icon="emoji_events" label=${tr('Open the leaderboard')} onClick=${() => openUrl(v.url).catch(() => {})} /></div>`}
  </section>`;
}
