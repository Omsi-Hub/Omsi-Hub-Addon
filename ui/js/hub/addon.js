// Omsi-Hub beside the player's own openOMSI: the Settings page's "openOMSI and OMSI 2" tab -
// the two folders the first-run setup chose (chosen again through it, or openOMSI's alone:
// `addon_set_folder`), the version read from openOMSI's folder, how well Omsi-Hub works with it
// (supported, with limits, not supported - estimated from the version until the first drive,
// then checked with the plugin's own handshake), the duty the plugin was last given, and the
// Omsi-Hub plugin switched on or off there, or taken out (`addon_state`, `addon_enable`,
// `addon_remove`; the folders' checks are `hub_setup_openomsi` and `hub_setup_omsi2`).

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call, openPath, pickFolder } from '../api.js';
import { S, set } from '../store.js';
import { Icon, Button, Switch, tr } from './bits.js';

const LEVELS = {
  supported: ['check_circle', 'Supported', 'ok'],
  limited: ['info', 'Supported with limits', 'warn'],
  unsupported: ['block', 'Not supported', 'bad'],
  unknown: ['help', 'Checked at the first drive', 'warn'],
};

const FROM = {
  executable: 'read from the program',
  bundle: 'read from the app bundle',
  folder_name: 'read from the folder name',
  program: 'asked of the program',
};

/** The tab's one row: the folder, the game in it, the compatibility, the plugin. */
export function AddonPanel() {
  const [s, setS] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const run = (cmd, args) => {
    setBusy(true);
    setErr('');
    return call(cmd, args)
      .then((v) => {
        if (v && v.refused) setErr(tr(v.refused.say || v.refused.problem || '', v.refused.vars));
        // (the plugin's commands answer its status: the whole state read again)
        return v && 'folder' in v ? setS(v) : call('addon_state').then(setS);
      })
      .catch((e) => setErr(tr(String(e))))
      .finally(() => setBusy(false));
  };
  const another = async () => {
    const f = await pickFolder(tr('Your openOMSI folder'));
    if (f) run('addon_set_folder', { folder: f });
  };
  useEffect(() => {
    run('addon_state');
  }, []);
  const [omsi2, setOmsi2] = useState(null);
  useEffect(() => {
    if (S.settings?.omsi2_folder) call('hub_setup_omsi2', { folder: S.settings.omsi2_folder }).then(setOmsi2, () => {});
  }, []);
  if (!s) return html`<div class="setting-row full"><span class="muted small">${err ? tr(err) : ''}</span></div>`;
  const inst = s.game;
  const level = inst?.compat && LEVELS[inst.compat.level];
  const plugin = s.plugin;
  // (a path is shown as the system writes it: `\` on Windows, `/` on Linux and macOS)
  const folder = (icon, name, path, check, hint) => html`<div class="row wrap hub-folder" style="gap:12px;align-items:center;margin-top:8px">
      <${Icon} name=${icon} />
      <span style="min-width:150px"><b>${name}</b>${hint && html`<br /><span class="muted small">${tr(hint)}</span>`}</span>
      <a href="#" class="grow hub-path" id=${name === 'openOMSI' ? 'set-openomsi' : 'set-omsi2'} style="word-break:break-all" onClick=${(e) => (e.preventDefault(), path && openPath(path))}>${path || tr('Not chosen')}</a>
      ${check && !check.ok && html`<span class="small bad">${tr(check.say || check.text || '', check.vars)}</span>`}
    </div>`;
  return html`<div class="setting-row full hub-addon">
    <p class="muted small" style="margin:0 0 6px">${tr('Omsi-Hub works with the openOMSI you have installed and your original OMSI 2. It keeps its own data and never changes their files.')}</p>
    ${folder('folder', 'openOMSI', s.folder, inst, '0.2.22 or newer')}
    ${folder('folder_open', 'OMSI 2', S.settings?.omsi2_folder, omsi2, 'Steam, Steam with Proton, or retail')}
    <div class="row wrap" style="gap:12px;margin-top:12px">
      <${Button} icon="folder_open" label="Choose the folders again" disabled=${busy} onClick=${() => set({ setup: true })} />
      <${Button} kind="ghost" icon="drive_folder_upload" label="Another openOMSI folder" disabled=${busy} onClick=${another} />
      ${S.boot?.data_dir && html`<${Button} kind="ghost" icon="open_in_new" label="Omsi-Hub's data folder" onClick=${() => openPath(S.boot.data_dir)} />`}
    </div>
    ${err && html`<p class="small bad" style="margin:8px 0 0">${tr(err)}</p>`}
    ${inst &&
    html`<div style="margin-top:12px">
      <div class="row wrap" style="gap:8px;align-items:center">
        <b>${inst.version ? tr('openOMSI %{version}', { version: inst.version }) : tr('Version not found')}</b>
        ${inst.version_from && html`<span class="muted small">(${tr(FROM[inst.version_from])})</span>`}
      </div>
      ${level &&
      html`<div class=${'row wrap ' + level[2]} style="gap:8px;align-items:center;margin-top:8px">
        <${Icon} name=${level[0]} /><b>${tr(level[1])}</b>
        <span class="muted small">${inst.compat.confirmed ? tr('Checked with this openOMSI') : tr('Estimated from the version')}</span>
      </div>`}
      ${(inst.compat?.notes || []).map((n) => html`<p class="small" style="margin:4px 0 0">${tr(n)}</p>`)}
      <p class="muted small" id="addon-checked" style="margin:4px 0 0">${s.checked
        ? tr('Checked at the first drive: openOMSI %{version} told the plugin what it offers.', { version: s.hello?.game || inst.version || '?' })
        : tr('Checked at the first drive: the plugin then tells Omsi-Hub what this openOMSI offers.')}</p>
      <ul class="small" style="margin:8px 0 0;padding-left:18px">
        ${(inst.compat?.features || []).map(
          (f) => html`<li class=${f.on ? '' : 'muted'}>${tr(f.label)}${!f.on && f.fallback ? html` <span class="muted">(${tr(f.fallback)})</span>` : ''}</li>`,
        )}
      </ul>
      <div class="row wrap" style="gap:12px;align-items:center;margin-top:12px">
        <span class="grow"><${Switch}
          checked=${plugin && (plugin.state === 'on' || plugin.state === 'outdated')}
          label="Omsi-Hub plugin in this openOMSI"
          onChange=${(on) => !busy && run('addon_enable', { on })}
        /></span>
        ${plugin?.state === 'outdated' && html`<${Button} icon="download" label="Update the plugin" disabled=${busy} onClick=${() => run('addon_enable', { on: true })} />`}
      </div>
      <p class="muted small" style="margin:4px 0 0">${plugin ? pluginLine(plugin, s.plugin_version) : ''}</p>
      <p class="muted small" style="margin:4px 0 0;word-break:break-all">${tr('Plugins folder: %{dir}', { dir: inst.plugins_dir || '-' })}</p>
      ${s.assigned && html`<p class="muted small" id="addon-assigned" style="margin:4px 0 0">${s.assigned.company
        ? tr('Last drive: %{company}, line %{line}, tour %{tour}', { company: s.assigned.company, line: s.assigned.line, tour: s.assigned.tour })
        : tr('Last drive: a drive of your own (%{driver})', { driver: s.assigned.driver })}</p>`}
      ${plugin && plugin.state !== 'missing' && html`<div class="row" style="margin-top:8px"><${Button} kind="ghost" icon="delete" label="Take the plugin out of openOMSI" disabled=${busy} onClick=${() => run('addon_remove')} /></div>`}
    </div>`}
  </div>`;
}

function pluginLine(p, ours) {
  if (p.state === 'missing') return tr('Not installed');
  if (p.state === 'off') return tr('Switched off in this openOMSI');
  if (p.state === 'outdated') return tr('Version %{version} installed, this Omsi-Hub has %{ours}', { version: p.version || '?', ours });
  return tr('Version %{version} installed', { version: p.version || ours });
}
