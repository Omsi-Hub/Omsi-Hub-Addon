# The plugin

Omsi-Hub's Lua plugin: the only part of Omsi-Hub that runs inside the player's openOMSI, on its
official plugin API. `hub-game` installs these files into openOMSI's `Plugins/omsihub/` and talks to
the plugin through plain files (`crates/hub-game/src/bridge.rs`).

- `main.lua`: the entry point (the duty, the reports, the handshake); the other files are its
  modules (`board`, `card`, `career`, `company`, `companion`, `duty_strip`, `messages`, `ontime`, `signon`,
  `start`, `takeover`, `traffic`, `fleet`, `trip`, `watch`, `hubjson`, `layout`, `navigator`, `notes`). `start` takes the duty (`duty.start`) in a
  call of its own: openOMSI stops a plugin when one call into it runs longer than 50 ms, the game's
  time inside the API counted, and taking a duty can take that long by itself. `navigator` puts
  everything the plugin shows while driving into openOMSI's navigator, one part under the other
  (the notifications of `notes`, the control room, the company and signing on, the duty board, on
  time, the duty strip), on a panel that takes clicks without F10 (`grab`); the navigator is
  moved and sized as it always is. In an openOMSI without the navigator's board each part is a
  panel of its own, and `layout` gives every draggable panel the
  driver's size and place (a size row under it with F10; kept in `omsi.data.layout`). `traffic` writes what is on the road (`data/traffic.json`)
  for the company's fleet map; it only reads the game. `fleet` gives the game the trips the
  company's own buses run in a company's duty (`data/fleet.json`, `omsi.timetable.set_ai_trips`),
  so its AI runs those alone, each as late as the fleet map has it. `takeover` takes a bus over halfway along its trip
  (the assignment's `take_over`): the AI's bus of the tour off the road, the player's bus put where
  it was with the duty at its next stop - else where the fleet map has it, else at that stop. `companion` talks to Omsi-Hub's phone and
  tablet companion while it is on (`data/companion.json`, datagrams by UDP; docs/COMPANION.md):
  it presses only the keys of the bus's IBIS, ticket printer and change dispenser.
- `theme.json`, `theme-loading.svg`, `theme-logo.svg`: Omsi-Hub's look for openOMSI's own screens,
  an interface theme (openOMSI's docs/THEMES.md) of the plugin's name: the loading screen with the
  logo's ring as the loading bar (`id="progress"`), the pause menu's panels of glass. `main.lua`
  puts it in force once (`ui.set_theme`), over openOMSI's own look only.
- `harness/`: our own test harness (`hub-plugin-harness`, on mlua). It mocks the official API
  functions the plugin uses, so `cargo test` runs the plugin without openOMSI.
  `cargo run -p hub-plugin-harness --example fakegame` runs the plugin on it with the companion's
  server (`crates/hub-companion`), driving a duty of two trips, for trying the Omsi-Hub Companion
  app without openOMSI: it prints the pairing code and the bus's keys the app presses.

A change to a `.lua` file needs the plugin's `VERSION` raised in `main.lua` and in
`crates/hub-game/src/install.rs` (a test keeps the two the same).
