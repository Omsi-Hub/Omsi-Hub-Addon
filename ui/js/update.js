// The app's own updates, the page's side (crates/hub-api/src/update.rs; this repository's GitHub
// releases, installed by Tauri's updater plugin): a new version offered in a dialog over the
// page - Update now, Not now, What's new, and "install updates without asking" -, its download
// and its installation, a failure with Try again; "Updated to Omsi-Hub x" in the top right corner
// for a few seconds after an update; the Settings page's row with "Check now", what the last look
// found and the switches. Looking, downloading, installing and starting the new version are the
// Rust side's: the page asks for the state every few seconds, once a second while something
// runs (`hub_update`).
//
// Where it goes: `useUpdates()` once in the shell, `UpdateDialog` and `UpdatedNotice` once at the
// shell's root, `UpdateRow` on the Settings page.

import { html, useEffect } from '../vendor/preact-htm.js';
import { call, openUrl } from './api.js';
import { language } from './i18n.js';
import { get, set, useStore } from './store.js';
import { Btn, Icon, loadSheet, Switch, tr } from './online/bits.js';

loadSheet();

/** The last answer as text: an answer like it changes nothing (the whole page would be drawn
 * again every second for nothing). */
let last = '';

const ask = (what, args = {}) =>
  call(what ? 'hub_update_act' : 'hub_update', { ...args, what, lang: language() })
    .then((u) => {
      // ("Updated to ..." once, kept on screen for its nine seconds)
      if (u.updated) set({ hubUpdated: { text: u.updated, at: Date.now() } });
      if (u.open) openUrl(u.open).catch(() => {});
      const text = JSON.stringify(u);
      if (text !== last) {
        last = text;
        set({ hubUpdate: u });
      }
      return u;
    })
    .catch(() => null);

/** The state asked for while the app is open (not in its selftest): once a second while the
 * dialog is up (a download's bar) or a look runs ("Check now" says what it found at once), else
 * every few seconds. */
export function useUpdates() {
  useEffect(() => {
    if (window.__OMSIHUB_SELFTEST__) return;
    let k = 0;
    const next = () => {
      const u = get().hubUpdate;
      k = setTimeout(() => ask().then(next), u?.dialog || u?.busy ? 1000 : 4000);
    };
    ask().then(next);
    return () => clearTimeout(k);
  }, []);
}

/** One of the dialog's buttons pressed (`install`, `dismiss`, `check`, `page`, `github`, `repo`),
 * or a switch (`auto`, `look` with `{on}`). */
export const updateAct = (what, args) => ask(what, args);

/** The dialog over the page while there is something to say. */
export function UpdateDialog() {
  const d = useStore((s) => s.hubUpdate?.dialog);
  if (!d) return null;
  const busy = d.kind === 'busy';
  return html`<div class="ol-overlay hub-update">
    <div class="ol-dialog" role="dialog" aria-modal="true" aria-label=${d.title}>
      <div class="head"><span class=${'mark' + (d.kind === 'failed' ? ' bad' : '')}><${Icon} name=${d.icon} size=${20} /></span><h2>${d.title}</h2></div>
      <p>${d.text}</p>
      ${busy && html`<div class="bar"><i style=${`width:${Math.round((d.frac || 0) * 1000) / 10}%`}></i></div>`}
      ${d.kind === 'available' && d.installable && html`<div class="auto"><${Switch} checked=${d.auto} label=${tr('Install updates without asking from now on')} onChange=${(on) => updateAct('auto', { on })} /></div>`}
      ${d.kind === 'available' &&
      html`<div class="foot"><${Btn} kind="ghost" icon="open_in_new" label=${tr("What's new")} onClick=${() => updateAct('page')} /><span class="ol-fill"></span>
        <${Btn} label=${tr('Not now')} onClick=${() => updateAct('dismiss')} />
        ${d.installable
          ? html`<${Btn} kind="primary" icon="download" label=${tr('Update now')} onClick=${() => updateAct('install')} />`
          : html`<${Btn} kind="primary" icon="open_in_new" label=${tr('Download from GitHub')} onClick=${() => (updateAct('page'), updateAct('dismiss'))} />`}</div>`}
      ${d.kind === 'failed' &&
      html`<div class="foot"><${Btn} kind="ghost" icon="open_in_new" label=${tr('Open on GitHub')} onClick=${() => updateAct('github')} /><span class="ol-fill"></span>
        <${Btn} icon="refresh" label=${tr('Try again')} onClick=${() => updateAct('check')} /><${Btn} label=${tr('Close')} onClick=${() => updateAct('dismiss')} /></div>`}
    </div>
  </div>`;
}

/** "Updated to Omsi-Hub x" in the top right corner, faded in and out. */
export function UpdatedNotice() {
  const n = useStore((s) => s.hubUpdated);
  useEffect(() => {
    if (!n) return;
    const k = setTimeout(() => get().hubUpdated === n && set({ hubUpdated: null }), 9000);
    return () => clearTimeout(k);
  }, [n?.at]);
  if (!n) return null;
  return html`<div class="hub-updated" key=${n.at}><${Icon} name="check_circle" size=${20} /><b>${n.text}</b></div>`;
}

/** The Settings page's updates: Check now, what the last look found, the switches, and the
 * repository's releases on GitHub. */
export function UpdateRow() {
  const u = useStore((s) => s.hubUpdate);
  useEffect(() => {
    if (!u) ask();
  }, []);
  return html`<div class="ol-update-row">
    <div class="ol-row">
      <${Btn} icon="refresh" label=${u?.busy ? tr('Checking...') : tr('Check now')} disabled=${!!u?.busy} onClick=${() => updateAct('check')} />
      <span class="ol-text ol-fill" id="update-line">${u?.line || ''}</span>
    </div>
    <${Switch} checked=${u ? u.look : true} label=${tr('Look for updates')} onChange=${(on) => updateAct('look', { on })} />
    <${Switch} checked=${!!u?.auto} label=${tr('Install updates without asking')} onChange=${(on) => updateAct('auto', { on })} />
    <div class="ol-row"><${Btn} kind="ghost" icon="open_in_new" label=${u?.repo || 'github.com/Omsi-Hub/Omsi-Hub-Releases'} onClick=${() => updateAct('repo')} /></div>
  </div>`;
}
