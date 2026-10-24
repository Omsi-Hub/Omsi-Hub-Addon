// The phone and tablet companion on the Settings page (General › Phone & tablet): a phone or
// tablet in the same network - in its browser, the web app the server serves on the address, or
// in the Omsi-Hub Companion app - signs on, shows the duty, the route and the control room's
// messages, and works the bus's IBIS and ticket printer. Switched on here (`hub_companion_set`;
// off by default), it shows the address, the pairing code and the QR code (the web app's
// pairing page, which the phone's camera opens), and the paired devices, each to remove
// (`hub_companion_state`, `hub_companion_code`, `hub_companion_forget`; `crates/hub-companion`).

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call } from '../api.js';
import { Icon, Button, Switch, tr } from './bits.js';

/** The QR code (`{size, bits}`: its modules row by row), drawn as one path. */
function Qr({ qr }) {
  if (!qr?.bits) return null;
  const n = qr.size;
  let d = '';
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) if (qr.bits[y * n + x] === '1') d += `M${x + 4} ${y + 4}h1v1h-1z`;
  return html`<svg class="hub-companion-qr" viewBox=${`0 0 ${n + 8} ${n + 8}`} style="width:148px;height:148px;border-radius:8px" shape-rendering="crispEdges" aria-label=${tr('QR code for pairing')}><rect width="100%" height="100%" fill="#fff" /><path d=${d} fill="#000" /></svg>`;
}

const ago = (unix) => {
  const s = Math.max(0, Math.round(Date.now() / 1000 - unix));
  if (s < 90) return tr('just now');
  if (s < 5400) return tr('%{n} min ago', { n: Math.round(s / 60) });
  if (s < 129600) return tr('%{n} h ago', { n: Math.round(s / 3600) });
  return tr('%{n} days ago', { n: Math.round(s / 86400) });
};

/** The row: the switch, and while on how a phone pairs and which ones are paired. */
export function CompanionPanel() {
  const [s, setS] = useState(null);
  const [busy, setBusy] = useState(false);
  const run = (cmd, args) => {
    setBusy(true);
    return call(cmd, args)
      .then(setS)
      .catch(() => {})
      .finally(() => setBusy(false));
  };
  useEffect(() => {
    run('hub_companion_state');
    // (the devices' "connected" and the game's state move on by themselves)
    const t = setInterval(() => call('hub_companion_state').then(setS, () => {}), 5000);
    return () => clearInterval(t);
  }, []);
  if (!s) return null;
  return html`<div class="setting-row full hub-companion">
    <p class="muted small" style="margin:0 0 6px">${tr('Sign on, see your duty, the route and the control room\'s messages, and work the bus\'s IBIS and ticket printer on a phone or tablet in your network: in its browser, no app needed.')}</p>
    <${Switch} checked=${s.on} label="Phone & tablet" disabled=${busy} onChange=${(on) => run('hub_companion_set', { on })} />
    ${!s.on && s.wanted && s.error && html`<p class="small bad" style="margin:6px 0 0">${tr(s.error)}</p>`}
    ${s.on &&
    html`<div class="row wrap" style="gap:16px;align-items:flex-start;margin-top:12px">
        <${Qr} qr=${s.qr} />
        <div class="grow" style="min-width:200px">
          <p class="small" style="margin:0">${tr('Scan the code with the phone\'s camera, or open the address in its browser and type the pairing code:')}</p>
          <p style="margin:8px 0 0"><span class="muted small">${tr('Address')}</span><br /><b id="companion-address" style="user-select:all">${s.address || `http://${s.host}:${s.port}`}</b></p>
          <p style="margin:8px 0 0"><span class="muted small">${tr('Pairing code')}</span><br /><b id="companion-code" style="font-size:22px;letter-spacing:3px;user-select:all">${s.code}</b></p>
          <div class="row" style="gap:8px;margin-top:8px"><${Button} kind="ghost" icon="refresh" label="Another code" disabled=${busy} onClick=${() => run('hub_companion_code')} /></div>
          <p class="muted small" style="margin:8px 0 0">${s.game ? html`<${Icon} name="sports_esports" /> ${tr('The game is connected.')}` : tr('The phone shows the game as soon as you drive.')}</p>
        </div>
      </div>
      <p class="small" style="margin:14px 0 4px"><b>${tr('Paired devices')}</b></p>
      ${(s.devices || []).length === 0 && html`<p class="muted small" style="margin:0">${tr('None yet')}</p>`}
      ${(s.devices || []).map(
        (d) => html`<div class="row" style="gap:10px;align-items:center;margin-top:4px">
          <${Icon} name=${d.connected ? 'smartphone' : 'phonelink_off'} />
          <span class="grow">${d.name}<br /><span class="muted small">${d.connected ? tr('Connected') : tr('Last seen %{when}', { when: ago(d.seen) })}</span></span>
          <${Button} kind="ghost" icon="delete" label="Remove" disabled=${busy} onClick=${() => run('hub_companion_forget', { id: d.id })} />
        </div>`,
      )}
      <p class="muted small" style="margin:10px 0 0">${tr('Only devices in your own network reach it, and only with a pairing code. Your firewall may ask to let Omsi-Hub take connections: allow it for private networks.')}</p>`}
  </div>`;
}
