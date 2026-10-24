// The mods in the player's openOMSI on Omsi-Hub's Settings page (the tab "Mods"), installed with
// openOMSI's own installer (`hub_mods`, hub-api's `mods.rs`): a .zip or .7z, or a folder (a .rar
// only where the installer reads one, `rar`: Omsi-Hub's does not, it is refused with what to do),
// chosen and installed into the content folder of the openOMSI in use - its progress followed
// until it is done, failed or cancelled -, and the mods there, each switched off and on or
// deleted (twice asked). What is installed here is on openOMSI's own Mods page as well.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call, openPath, pickFile, pickFolder } from '../api.js';
import { toast } from '../store.js';
import { Button, Icon, Switch, tr } from './bits.js';
import { Search } from '../ui.js';

const STATES = {
  queued: 'Waiting',
  planning: 'Looking at the mod',
  checking: 'Checking the free space',
  unpacking: 'Unpacking',
  copying: 'Copying',
  moving: 'Putting it in place',
  done: 'Installed',
  failed: 'Failed',
  cancelled: 'Cancelled',
};

const KINDS = { bus: ['directions_bus', 'Bus'], map: ['map', 'Map'], archive: ['inventory_2', 'Archive'], other: ['extension', 'Other'] };

const active = (j) => !j.finished;

/** A size in MB or GB. */
function size(b) {
  if (!b) return '';
  return b >= 1e9 ? `${(b / 1e9).toFixed(1)} GB` : `${Math.max(0.1, b / 1e6).toFixed(1)} MB`;
}

/** The tab: install a mod, the installs of this run, the mods there. */
export function ModsPanel() {
  const [s, setS] = useState(null);
  const [err, setErr] = useState('');
  const [busy, setBusy] = useState(false);
  const [find, setFind] = useState('');
  const [asked, setAsked] = useState('');
  const load = () =>
    call('hub_mods')
      .then((v) => (setS(v), setErr('')))
      .catch((e) => setErr(String(e)));
  const run = (cmd, args) => {
    setBusy(true);
    setErr('');
    return call(cmd, args)
      .then((v) => (setS(v.state || v), v))
      .catch((e) => (setErr(String(e)), null))
      .finally(() => setBusy(false));
  };
  useEffect(() => {
    load();
  }, []);
  // (followed while an install runs)
  const running = (s?.jobs || []).some(active);
  useEffect(() => {
    if (!running) return undefined;
    const t = setInterval(load, 800);
    return () => clearInterval(t);
  }, [running]);
  // (an install that ended: said once)
  const [seen, setSeen] = useState({});
  useEffect(() => {
    for (const j of s?.jobs || []) {
      if (j.finished && !seen[j.id] && seen[j.id] !== undefined) {
        if (j.state === 'done') toast(tr('%{name} is installed', { name: j.name }), 'ok');
        else if (j.state === 'failed') toast(tr('%{name} could not be installed', { name: j.name }), 'bad');
      }
    }
    setSeen(Object.fromEntries((s?.jobs || []).map((j) => [j.id, !!j.finished])));
  }, [s]);
  const install = (path) => path && run('hub_mods_install', { path });
  const file = async () => install(await pickFile(tr('Choose a mod'), [{ name: tr('Mod archives'), extensions: s?.rar ? ['zip', '7z', 'rar'] : ['zip', '7z'] }]));
  const folder = async () => install(await pickFolder(tr('Choose a mod folder')));

  if (!s) return html`<div class="setting-row full"><span class="muted small">${err ? tr(err) : ''}</span></div>`;
  const q = find.trim().toLowerCase();
  const mods = (s.mods || []).filter((m) => !q || m.name.toLowerCase().includes(q) || m.paths.some((p) => p.toLowerCase().includes(q)));
  return html`<div class="setting-row full hub-mods">
    <p class="muted small" style="margin:0 0 6px">${tr("A bus, a map, scenery or a whole OMSI folder - as a .zip or .7z, or a folder (a .rar: unpack it first). openOMSI's own installer puts it into its content folder; the original OMSI 2 folder is never written to.")}</p>
    <div class="row wrap" style="gap:12px;margin-top:8px">
      <${Button} icon="upload_file" label="Install a mod file" disabled=${busy} onClick=${file} />
      <${Button} kind="ghost" icon="drive_folder_upload" label="Install a mod folder" disabled=${busy} onClick=${folder} />
      <${Button} kind="ghost" icon="open_in_new" label="Content folder" onClick=${() => openPath(s.content)} />
    </div>
    ${err && html`<p class="small bad" style="margin:8px 0 0">${tr(err)}</p>`}
    ${(s.jobs || []).length > 0 &&
    html`<div id="mods-jobs" style="margin-top:12px">
      ${s.jobs.map((j) => html`<${Job} key=${j.id} j=${j} onCancel=${() => run('hub_mods_cancel', { id: j.id })} />`)}
      ${!running && html`<${Button} kind="ghost" small icon="clear_all" label="Clear the finished installs" onClick=${() => run('hub_mods_clear')} />`}
    </div>`}
    <div class="row wrap" style="gap:12px;align-items:center;margin-top:16px">
      <b class="grow">${tr('Installed mods')} <span class="muted small">(${(s.mods || []).length})</span></b>
      <${Search} value=${find} onInput=${setFind} placeholder="Search the mods" />
    </div>
    ${s.running && html`<p class="small warn" style="margin:6px 0 0">${tr('Close openOMSI to switch mods off or on, or to delete them.')}</p>`}
    ${mods.length === 0 && html`<p class="muted small" style="margin:8px 0 0">${(s.mods || []).length ? tr('No mod matches.') : tr('No mods are installed yet.')}</p>`}
    <div id="mods-list" style="margin-top:8px">
      ${mods.map((m) => {
        const [icon, kind] = KINDS[m.kind] || KINDS.other;
        const sure = asked === m.id;
        return html`<div key=${m.id} class="row wrap hub-mod" style="gap:12px;align-items:center;padding:6px 0;border-top:1px solid rgba(128,128,128,0.16)" data-mod=${m.id}>
          <${Icon} name=${icon} />
          <span class="grow" style="min-width:180px;word-break:break-all">
            <b class=${m.enabled ? '' : 'muted'}>${m.name}</b>
            <br /><span class="muted small">${tr(kind)}${m.bytes ? ` · ${size(m.bytes)}` : ''} · ${m.paths.join(', ')}</span>
          </span>
          <${Switch} checked=${m.enabled} label=${m.enabled ? 'On' : 'Off'} onChange=${(on) => !busy && !s.running && run('hub_mods_enable', { id: m.id, on })} />
          ${sure
            ? html`<${Button} kind="danger" small icon="delete" label="Delete for good" disabled=${busy || s.running} onClick=${() => (setAsked(''), run('hub_mods_remove', { id: m.id }))} />
                <${Button} kind="ghost" small label="Keep" onClick=${() => setAsked('')} />`
            : html`<${Button} kind="ghost" small icon="delete" label="Delete" disabled=${busy || s.running} onClick=${() => setAsked(m.id)} />`}
        </div>`;
      })}
    </div>
  </div>`;
}

/** One install: its state, its progress, what it said. */
function Job({ j, onCancel }) {
  const total = j.bytes_total || j.files_total;
  const done = j.bytes_total ? j.bytes_done : j.files_done;
  const part = total ? Math.min(1, done / total) : 0;
  const tone = j.state === 'failed' ? 'bad' : j.state === 'done' ? 'ok' : '';
  return html`<div class="hub-mod-job" style="margin-bottom:10px" data-job=${j.id}>
    <div class="row wrap" style="gap:8px;align-items:center">
      <b class="grow" style="word-break:break-all">${j.name}</b>
      <span class=${'small ' + tone}>${tr(STATES[j.state] || j.state)}</span>
      ${active(j) && html`<${Button} kind="ghost" small icon="close" label="Cancel" onClick=${onCancel} />`}
    </div>
    ${active(j) && html`<div class="hub-progress" style="max-width:none;margin-top:6px"><div style=${`width:${part * 100}%`}></div></div>`}
    ${j.message && html`<p class="muted small" style="margin:4px 0 0;word-break:break-word">${j.message}</p>`}
    ${j.state === 'done' && (j.installed || []).length > 0 && html`<p class="small" style="margin:4px 0 0;word-break:break-all">${tr('Put in place: %{what}', { what: j.installed.join(', ') })}</p>`}
    ${(j.kept_aside || []).length > 0 && html`<p class="muted small" style="margin:4px 0 0">${tr('Kept aside until their bus is installed: %{what}', { what: j.kept_aside.join(', ') })}</p>`}
    ${(j.warnings || []).map((w) => html`<p class="small warn" style="margin:4px 0 0;word-break:break-word">${w}</p>`)}
  </div>`;
}
