# The plugin

Omsi-Hub's Lua plugin: the only part of Omsi-Hub that runs inside the player's openOMSI, on its
official plugin API. `hub-game` installs these files into openOMSI's `Plugins/omsihub/` and talks to
the plugin through plain files (`crates/hub-game/src/bridge.rs`).

- `main.lua`: the entry point (the duty, the reports, the handshake); the other files are its
  modules (`board`, `card`, `career`, `company`, `companion`, `duty_strip`, `messages`, `ontime`, `signon`,
  `start`, `traffic`, `trip`, `watch`, `hubjson`, `layout`). `start` takes the duty (`duty.start`) in a
  call of its own: openOMSI stops a plugin when one call into it runs longer than 50 ms, the game's
  time inside the API counted, and taking a duty can take that long by itself. `layout` gives every draggable panel the
  driver's size and place (a size row under it with F10; kept in `omsi.data.layout`). `traffic` writes what is on the road (`data/traffic.json`)
  for the company's fleet map; it only reads the game. `companion` talks to Omsi-Hub's phone and
  tablet companion while it is on (`data/companion.json`, datagrams by UDP; docs/COMPANION.md):
  it presses only the keys of the bus's IBIS, ticket printer and change dispenser.
- `harness/`: our own test harness (`hub-plugin-harness`, on mlua). It mocks the official API
  functions the plugin uses, so `cargo test` runs the plugin without openOMSI.
  `cargo run -p hub-plugin-harness --example fakegame` runs the plugin on it with the companion's
  server (`crates/hub-companion`), driving a duty of two trips, for trying the Omsi-Hub Companion
  app without openOMSI: it prints the pairing code and the bus's keys the app presses.

A change to a `.lua` file needs the plugin's `VERSION` raised in `main.lua` and in
`crates/hub-game/src/install.rs` (a test keeps the two the same).
