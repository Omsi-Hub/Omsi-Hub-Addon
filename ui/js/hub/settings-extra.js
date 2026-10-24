// The Settings page's tabs and rows (`pages/settings.js` draws them): the look - light or dark,
// the accent, the launcher's size, the animations, the company's layout -, the language and the
// phone and tablet companion (`companion.js`);
// openOMSI and OMSI 2 (the folders, the version, the plugin: `addon.js`); the mods installed into
// openOMSI (`mods.js`); openOMSI's own settings
// (`gamesettings.js`, its `settings.cfg`); the game controllers - the wheel, the pedals, the
// button boxes (`controllers.js`, `Inputs/gamectrler.cfg`); the updates; and the
// bus models' categories and regions the company goes by. Every change is saved at once
// (`saveSettings`).
//
// A row is `{ type, key, label, … }`: `toggle`, `slider` (`min`, `max`, `step`, `format`,
// `store`), `select` (`options` [value, label]), `button` (`patch()` returns the settings to
// change, or does something itself), or `custom` (`view()` draws it).

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { PRESETS, hexOf } from './theme.js';
import { STOP_SIGNS } from './stopsigns.js';
import { UpdateRow } from '../update.js';
import { showPrivacy } from '../privacy.js';
import { AddonPanel } from './addon.js';
import { ModsPanel } from './mods.js';
import { GameSettingsPanel } from './gamesettings.js';
import { ControllersPanel } from './controllers.js';
import { CompanionPanel } from './companion.js';

const pct = (v) => `${Math.round(v * 100)} %`;

/** The language names as each language writes its own. */
export const LANGUAGE_NAMES = { en: 'English', de: 'Deutsch', fr: 'Français', nl: 'Nederlands', pl: 'Polski', ru: 'Русский', uk: 'Українська', cs: 'Čeština', it: 'Italiano', es: 'Español' };

export const SETTINGS_TABS = [
  {
    id: 'general',
    label: 'General',
    cols: [
      {
        title: 'Look',
        rows: [
          { type: 'slider', key: 'launcher_scale', label: 'Launcher size', min: 0.6, max: 2, step: 0.05, def: 1, format: pct, store: (v) => Math.round(v * 20) / 20 },
          { type: 'toggle', key: 'animations', label: 'Animations' },
          { type: 'select', key: 'accent', label: 'Accent colour', options: PRESETS.map(([name, c]) => [hexOf(c), name]) },
          { type: 'toggle', key: 'dark_mode', label: 'Dark mode' },
          // (the stops on the maps as the bus stop signs of a country: `stopsigns.js`)
          { type: 'select', key: 'stop_style', label: 'Bus stop signs', options: STOP_SIGNS },
          // (the live dashboard's speeds and distances: kept, and read by it, but there was no
          // place to choose them)
          { type: 'select', key: 'units', label: 'Units', options: [['metric', 'Metric (km/h, km)'], ['imperial', 'Imperial (mph, miles)']] },
          // (the bus company's pages round a sidebar, or found with the command palette: `company/shell.js`)
          { type: 'select', key: 'company_layout', label: 'Company layout', options: [['sidebar', 'Sidebar'], ['palette', 'Command palette']] },
          { type: 'button', label: 'Start the tour', icon: 'route', patch: () => (import('./hub.js').then((h) => (h.go('drive', 'mode'), import('./tour.js').then((m) => setTimeout(() => m.startTour(), 300)))), {}) },
          // (the Privacy Statement, as the app has it built in: `privacy.js`)
          { type: 'button', id: 'settings-privacy', label: 'Privacy Statement', icon: 'lock', patch: () => (showPrivacy(), {}) },
        ],
      },
      {
        title: 'Language',
        rows: [{ type: 'language', key: 'language', label: 'Language' }],
      },
      // (the Omsi-Hub Companion app on a phone or tablet: `companion.js`)
      {
        title: 'Phone & tablet',
        rows: [{ type: 'custom', id: 'hub-companion', view: () => html`<${CompanionPanel} />` }],
      },
    ],
  },
  { id: 'addon', label: 'openOMSI and OMSI 2', cols: [{ title: 'Your openOMSI and OMSI 2', rows: [{ type: 'custom', id: 'hub-addon', view: () => html`<${AddonPanel} />` }] }] },
  // (mods installed into openOMSI with its own installer, and switched off, on or deleted)
  { id: 'mods', label: 'Mods', cols: [{ title: 'Mods in openOMSI', rows: [{ type: 'custom', id: 'hub-mods', view: () => html`<${ModsPanel} />` }] }] },
  { id: 'game', label: "openOMSI's settings", cols: [{ title: "openOMSI's settings", rows: [{ type: 'custom', id: 'hub-game-settings', view: () => html`<${GameSettingsPanel} />` }] }] },
  // (the wheel, the pedals, joysticks and button boxes, and openOMSI's settings round them)
  { id: 'controllers', label: 'Controllers', cols: [{ title: 'Game controllers', rows: [{ type: 'custom', id: 'hub-controllers', view: () => html`<${ControllersPanel} />` }] }] },
  { id: 'updates', label: 'Updates', cols: [{ title: 'Updates', rows: [{ type: 'custom', id: 'hub-update', view: () => html`<${UpdateRow} />` }] }] },
  // (the bus models' categories and regions, for every company: what the dealer, the market and
  // the shop of the bus company go by, `company/vehicles.js`)
  { id: 'vehicles', label: 'Vehicles', cols: [{ title: 'Bus categories and regions', rows: [{ type: 'custom', id: 'hub-vehicles', view: () => html`<${Vehicles} />` }] }] },
];

/** The models' categories and regions (loaded with the tab: the company's list). */
function Vehicles() {
  const [m, setM] = useState(null);
  useEffect(() => {
    import('../company/vehicles.js').then(setM).catch(() => {});
  }, []);
  return m ? html`<${m.VehicleClasses} />` : null;
}
