# Third-party code in Omsi-Hub

Omsi-Hub is proprietary software (`LICENSE`). It builds on the code below, each under its own
licence, whose notices travel with it; Omsi-Hub's licence takes away none of the rights they give.

## openOMSI's readers, vehicle simulation and mod installer

The 3D view, the bus photos and the livery studio read a bus as the game reads it with crates of
openOMSI: `omsi-cfg`, `omsi-model`, `omsi-sim`, `omsi-texture` and `omsi-vehicle`, and what they
build on (`omsi-script`, `omsi-o3d`, `omsi-geometry`, `omsi-content`, `omsi-scenery`, `omsi-map`,
`omsi-timetable`, `omsi-world`); the tab "Mods" installs mods with openOMSI's own installer,
`omsi-launcher-core`. Cargo fetches them from Luc's fork of openOMSI
(<https://github.com/Luc-nbr/openOMSI>), pinned to one commit in `Cargo.toml`; none of their code
is in this repository.

That fork's openOMSI code is openOMSI as it was up to 13 October 2026, when it was under the MIT
licence (openOMSI became AGPL-3.0 on 14 October 2026; the fork never took a commit of openOMSI
from after that), with Luc's own changes. Its licence:

```
MIT License

Copyright (c) 2026 usonskyyyy

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

Omsi-Hub's own code from that fork (its `omsihub*` crates and the launcher's Omsi-Hub pages: the
3D view, the photos, the livery studio, `crates/hub-livery`) is Omsi-Hub's and under Omsi-Hub's
licence.

## The website's bus company: WASI in the browser

`ui/vendor/browser_wasi_shim/` is `@bjorn3/browser_wasi_shim` 0.4.2 (its `dist/`), by bjorn3 and
its contributors, under the MIT licence (`ui/vendor/browser_wasi_shim/LICENSE-MIT`; it is also
offered under Apache-2.0). It answers WASI's calls of `crates/hub-web` in the browser.

The cloud save's zstd on the website is `ruzstd` (crates.io), under the MIT licence.

## Fonts

- **Hanken Grotesk** (`ui/fonts/`, `crates/hub-livery/fonts/`): SIL Open Font License 1.1,
  `HankenGrotesk-OFL.txt` beside the files.
- **Roboto** (`crates/hub-livery/fonts/`, the livery studio's texts): SIL Open Font License 1.1,
  Copyright 2011 The Roboto Project Authors (<https://github.com/googlefonts/roboto-classic>).

## The page's libraries

- **Preact** (MIT licence, `preact-LICENSE.txt`) and **htm** (Apache-2.0, `htm-LICENSE.txt`), in
  `ui/vendor/preact-htm.js`, their licences beside the file.

## The Rust libraries

Every library the app is built with, its version, licence and address, and the licence files they
came with are in `ui/third-party-licenses.txt`, which the app carries. `python3 tools/licenses.py`
writes it again after a change to `Cargo.lock`. They are under permissive licences (MIT, Apache-2.0,
BSD, ISC, Zlib, Unicode-3.0 and the like); the few under the MPL-2.0 (`cssparser`, `selectors` and
their helpers, through the webview) are used unchanged, and their source is at the address given.
