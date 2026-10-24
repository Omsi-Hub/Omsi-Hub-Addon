# Omsi-Hub as its own code: the plan

Status: phases 1–5 done, phase 6 under way: 0.1.0 is being prepared as a pre-release (2026-10-17;
see "Where it stands"). Owner: Luc. This repository is the new home of Omsi-Hub.

## The goal

Omsi-Hub becomes an **add-on app** beside the player's openOMSI. (It contained no openOMSI code until
2026-10-18; since then the 3D view, the photos and the livery studio read a bus with openOMSI's
readers from Luc's fork, MIT from before openOMSI became AGPL: `THIRD_PARTY_NOTICES.md`.) The player installs official
openOMSI (0.2.0 or newer, ideally the latest) and an original OMSI 2. Omsi-Hub then works like this:

- It is its own desktop app, with its own shell, UI, logic and file readers.
- It **finds and starts the player's openOMSI**, through its command line (`--map`, `--bus`,
  `--spawn` …) and through the official plugin API (`duty.start`, …).
- It **installs its plugin** (the Lua plugin, today `crates/omsihub-plugin`) into the player's
  openOMSI. That is the only part that runs inside the game.
- It keeps **its own data** (companies, careers, settings) and never writes to openOMSI's or OMSI 2's
  files.
- It no longer ships a game (`omsi-hub-game.exe` goes).

## Decisions (Luc, 2026-10-17)

- **All own code.** Not even official openOMSI crates as libraries. Omsi-Hub reads the OMSI 2 files
  it needs with its own readers.
- **What only our own game could do** waits for small upstream PRs to openOMSI, and degrades
  gracefully until then: the pause menu, the bus's display font, IBIS auto-typing, closed stops on the
  navigator, and the duty board docked in the navigator.
- **A clean start:** no history import. The old repository (`Luc-nbr/omsi-hub`, formerly the fork)
  stays as the archive.

## What there is today (in the old repository, branch `beta`)

- About **1,000 files exist only in Omsi-Hub**: about 230,000 lines of code, ours. The biggest parts:
  - the company and RosterFlow logic in `crates/omsi-launcher-core/src` (73k);
  - the launcher's API and views in `crates/omsi-launcher-api/src` (42k);
  - the UI in `launcher/ui` (45k);
  - the `omsihub-*` crates (33k);
  - game-side extras in `crates/omsi-app` (23k).
- **This code sits inside openOMSI's own launcher** (`omsi-launcher-core/-api/-model`,
  `launcher/src-tauri`, `launcher/ui`) and uses openOMSI crates throughout. The main ones are:
  - `omsi-cfg` (≈290 uses) and `omsi-ui` (≈290, mostly in `omsihub-game`, the game-drawn editors);
  - `omsi-sim` (≈80), `omsi-vehicle` (≈75), `omsi-map` and `omsi-texture` (≈60 each);
  - `omsi-timetable` (≈45), `omsi-content` and `omsi-net` (≈35 each);
  - `omsi-model`, `omsi-o3d`, `omsi-render` and `omsi-platform` (15–20 each).

## The new structure

| Crate / folder | What it is | Comes from |
| --- | --- | --- |
| `app/` (Tauri 2) | Omsi-Hub's own desktop shell: window, updater, finding and starting openOMSI, installing the plugin | new, written for Omsi-Hub |
| `ui/` | The web UI: home, bus company, RosterFlow, editors, online, settings | ported from `launcher/ui` (our parts); openOMSI's launcher pages are not taken |
| `crates/hub-core` | The bus company, RosterFlow, the economy, careers, multiplayer logic | ported from our `company/*`, `rosterflow*`, … in `omsi-launcher-core` |
| `crates/hub-api` | The app's commands and views | ported from our `omsihub/*` in `omsi-launcher-api` |
| `crates/hub-omsi` | **Our own OMSI 2 file readers**, only what Omsi-Hub needs: the OMSI text formats; maps (`global.cfg`, tiles for road geometry); timetables (`TTData`: lines, routes, trips, tours); vehicles (`.bus`, `.hof`, liveries); models (`.o3d`) and textures for the 3D preview | **new**, written from the file formats, not from openOMSI's code |
| `crates/hub-game` | Talking to the player's openOMSI: finding it and reading its version, the command line, the plugin's data files (bridge) | ported from `omsihub-plugin/src` (`detect`, `compat`, `bridge`, `install`) |
| `plugin/` | The Lua plugin that runs inside openOMSI | ported from `omsihub-plugin/lua`, on the official plugin API only |
| `crates/hub-locales`, `crates/hub-online` | Texts and the online client (Supabase) | ported as they are (already ours) |
| `online/`, `docs/` | Supabase schema and docs | ported |

**Rule for porting.** Only code Omsi-Hub wrote is taken. Where it calls openOMSI code, the call goes
to our own `hub-omsi` or `hub-game` instead, or the feature is redone without it. Nothing is copied
from openOMSI. One exception (Luc, 2026-10-18: "the same system as the old app"): the old app's
Bus3D, photos and livery studio keep calling openOMSI's readers (`omsi-cfg`, `omsi-model`,
`omsi-o3d`, `omsi-sim`, `omsi-texture`, `omsi-vehicle`), taken as git dependencies from Luc's fork,
pinned to one commit - openOMSI as it was up to 2026-10-13, MIT.

## What changes for players

- **They install official openOMSI themselves**, and Omsi-Hub next to it. On first start, Omsi-Hub asks
  for the openOMSI folder (it already detects installs) and the OMSI 2 folder.
- **Companies, careers and settings move over automatically** from the old data folder (`~/.openomsi/…`
  Omsi-Hub parts) on first start.
- **Existing installs of the old Omsi-Hub (the fork with its own game)** get one last update in the old
  repository. It says where the new Omsi-Hub is, and links to the download.
- **Features that needed our own game** (pause menu, display font, IBIS auto, closed stops, navigator
  dock) are off, with a note in the app ("needs a newer openOMSI"), until upstream has them.
- **The game-drawn editors** (`omsihub-game`) become web UI in the app, or wait. To be decided per
  editor in phase 1.

## Phases

1. **Inventory**, a document in this repository: per module of our code, the decision to port as it
   is, port with our own reader, redo, or drop; every openOMSI call it makes, and its replacement.
   Also the data folder and the save formats, so that players keep their companies.
2. **`hub-omsi`**, our OMSI 2 readers, with tests on small synthetic fixtures. No OMSI 2 content goes in
   the repository.
3. **`hub-core` + `hub-api`**: port the company, RosterFlow and the views onto `hub-omsi`. All of today's
   company tests must pass.
4. **`app/` + `ui/`**: the shell, the updater (this repository's releases), starting openOMSI, installing
   the plugin, settings, and the UI port.
5. **`plugin/` + `hub-game`**: the plugin on the official API only, its fallbacks, and the version check.
6. **Migration and release**:
   - the data move;
   - a release workflow (Windows + macOS) here;
   - the old repository's last update pointing here;
   - the website's download page.
7. **Upstream PRs** to openOMSI for the missing plugin functions and the pause-menu hook: one small PR
   each.

Phases 2–5 can run in parallel sessions once phase 1 fixes the interfaces.

## Where it stands (2026-10-18)

| Phase | |
| --- | --- |
| 1. Inventory | Done (`docs/INVENTORY.md`) |
| 2. `hub-omsi` | Done, with tests on a synthetic OMSI 2 folder |
| 3. `hub-core` + `hub-api` | Done: the company, RosterFlow, online, cloud saves, the timetable and depot editors, playing together (`tg_`, over openOMSI's LAN layer and relay), the line editor (`hub_le`, on the map's lanes), the 3D view, photos and livery studio (`b3d_`, `lak_`), on the crates of Luc's openOMSI fork |
| 4. `app/` + `ui/` | Done |
| 5. `plugin/` + `hub-game` | Done; a drive is tested end to end against a fake openOMSI |
| 6. Migration and release | The data move and the release workflow (Windows, macOS, Linux; pre-releases too) are done. Next: 0.1.0 as a pre-release (`CHANGELOG.md`, `docs/UPDATES.md` "A pre-release"), Luc's test on a real openOMSI and OMSI 2, the signing key, the repository public, then the old repository's last update and the website's download page |
| 7. Upstream PRs | Not started |

## Open points for Luc

- **The repository's GitHub address.** Settled: `github.com/Omsi-Hub/Omsi-Hub-Addon`, public:
  Omsi-Hub is proprietary from 19 October 2026 (`LICENSE`; the code before it was MIT). The
  releases are its own (docs/UPDATES.md).
- **The game-drawn editors** (line editor and others in `omsihub-game`): web UI in the app, or drop?
  Decided per editor in phase 1.
- **The app's name and identifier:** `Omsi-Hub`, with a new identifier (a fresh WebView profile) or the
  old one (to keep `localStorage`).
