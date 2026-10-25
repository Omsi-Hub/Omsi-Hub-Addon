// Settings: Omsi-Hub's own tabs (`hub/settings-extra.js` lists their rows) in the wide sheet
// under the bar. Every change is saved at once.

import { html, useState } from '../../vendor/preact-htm.js';
import { useLang } from '../i18n.js';
import { S, useStore } from '../store.js';
import { saveSettings } from '../hub/drive.js';
import { Button, Seg, Slider, Switch, tr } from '../hub/bits.js';
import { LANGUAGE_NAMES, SETTINGS_TABS } from '../hub/settings-extra.js';
import { Select } from '../ui.js';

function Row({ row, s }) {
  const value = s.settings?.[row.key];
  if (row.when && !row.when(s.settings || {})) return null;
  if (row.type === 'custom') return row.view();
  if (row.type === 'toggle') return html`<div class="setting-row full"><${Switch} checked=${!!value} label=${row.label} onChange=${(on) => saveSettings({ [row.key]: on })} /></div>`;
  if (row.type === 'slider') {
    return html`<div class="setting-row full"><${Slider} label=${row.label} value=${value ?? row.def} min=${row.min} max=${row.max} step=${row.step} format=${row.format}
      onChange=${(v) => saveSettings({ [row.key]: row.store ? row.store(v) : v })} /></div>`;
  }
  if (row.type === 'select') {
    return html`<div class="setting-row"><span class="name">${tr(row.label)}</span><${Select} value=${value ?? row.options[0][0]} options=${row.options} onChange=${(v) => saveSettings({ [row.key]: v })} /></div>`;
  }
  if (row.type === 'language') {
    const langs = s.boot?.languages || ['en'];
    return html`<div class="setting-row"><span class="name">${tr(row.label)}</span><${Select} raw value=${s.lang} options=${langs.map((l) => [l, LANGUAGE_NAMES[l] || l])} onChange=${(l) => saveSettings({ language: l })} /></div>`;
  }
  if (row.type === 'button') {
    return html`<div class="setting-row full" id=${row.id}><div><${Button} icon=${row.icon} label=${row.label} onClick=${() => {
      const patch = row.patch();
      if (patch && Object.keys(patch).length) saveSettings(patch);
    }} /></div></div>`;
  }
  return null;
}

export function Settings() {
  useLang();
  const s = useStore();
  // (opened on a tab when asked: the start's "Install mods" opens the tab "Mods", `go('settings', null, 'mods')`)
  const [tab, setTab] = useState(() => {
    const asked = S.hub.settingsTab;
    delete S.hub.settingsTab;
    return asked || SETTINGS_TABS[0].id;
  });
  const t = SETTINGS_TABS.find((x) => x.id === tab) || SETTINGS_TABS[0];
  return html`<div class="hub-settings" id="settings">
    <${Seg} value=${t.id} options=${SETTINGS_TABS.map((x) => [x.id, x.label])} onChange=${setTab} />
    <div class="settings-grid">
      ${t.cols.map((c) => html`<section class="settings-col">
        ${c.title && html`<h3 class="hub-caps"><b>${tr(c.title)}</b></h3>`}
        ${c.rows.map((row) => html`<${Row} row=${row} s=${s} />`)}
      </section>`)}
    </div>
  </div>`;
}
