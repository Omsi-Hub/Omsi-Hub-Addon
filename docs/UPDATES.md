# The app's updates

Omsi-Hub updates itself from the GitHub releases of its own public repository
`github.com/Omsi-Hub/Omsi-Hub-Addon`: the installers and `latest.json` in each release. Nothing of
openOMSI's updater is used.

## How it works

There are two halves.

**The look** (`crates/hub-api/src/update.rs`, ours):

- It asks GitHub's API for the releases a second after the start, every half hour, and when a
  game started from Omsi-Hub has ended.
- It skips drafts, pre-releases, and tags with a `-` in them (`v0.3.0-beta1`).
- It compares the versions by their numbers (`0.2.10` > `0.2.9`) and offers the newest one
  above the running version.
- A test build (a version with a `-`) is never offered anything.
- It finds the platform's asset (the table below): the NSIS installer on Windows, the
  `.app.tar.gz` on macOS, the `.AppImage` on Linux.
- The page shows the offer in a dialog (`ui/js/update.js`): Update now, Not now, What's new, and
  "Install updates without asking".
- Settings has "Check now", "Look for updates" (`update_check`) and "Install updates without
  asking" (`update_auto`), kept in `settings.json`. It is on unless the player switched it off:
  a new version downloads and installs itself, without a click, while no game runs.
- `OMSIHUB_NO_UPDATE=1` switches the looks off.

**The installation** (`app/src/main.rs`, Tauri's `tauri-plugin-updater`):

- "Update now" makes the plugin read the newest release's `latest.json`
  (`releases/latest/download/latest.json`).
- It downloads the asset that file names and checks its signature against the public key in
  `app/tauri.conf.json` (`plugins.updater.pubkey`).
- Then it installs the new version and starts it:
  - On Windows, the NSIS installer runs in passive mode. It ends Omsi-Hub and starts the new
    version itself.
  - On macOS, the `.app` bundle is replaced and the app restarts.
  - On Linux, the AppImage file the player started (`$APPIMAGE`) is replaced and the app
    restarts. Only an AppImage updates itself: a copy installed from a `.deb` or `.rpm` (or built by hand)
    is offered the release's page instead.
- Nothing is installed while a game runs.

Until the public key is in `tauri.conf.json`, the app does not use the plugin. "Update now"
then becomes "Download from GitHub", which opens the release's page. The same happens for a
release without a `latest.json`.

## The installers' look

- **Windows (NSIS):** `app/tauri.windows.conf.json`, `bundle.windows.nsis`: the pictures of
  `app/installer/` (`nsis-sidebar.bmp` 164 × 314 on the welcome and finish pages,
  `nsis-header.bmp` 150 × 57 beside the other pages' titles, the uninstaller's too), the app's
  icon, the licence page (`bundle.licenseFile`: `LICENSE`), the languages Tauri has texts for
  (English, Dutch, German, French, Russian, Ukrainian; Polish has none, so Polish players get
  English) with a choice at the start, per-user install (no administrator), and a Start menu
  folder of its own.
- **macOS (DMG):** `app/tauri.macos.conf.json`, `bundle.macOS.dmg`: `dmg-background.png`
  (660 × 400, light, as Finder writes the icons' names in black), the app at (165, 210) and
  Applications at (495, 210), the arrow between them.
- **Linux:** the packages' description is `bundle.longDescription`; the menu entry's category
  comes from `bundle.category` (`SimulationGame`).
- The pictures are drawn by `node tools/installer-art.mjs` from `tools/installer-art.html`
  (Playwright and Python with Pillow) and committed. CI builds the NSIS installer and the DMG on
  every push, so a wrong picture or setting shows there first.

## The release files

| Platform | `latest.json` key | Updater file (signed: `<file>.sig` beside it) | For a first download |
|---|---|---|---|
| Windows x64 | `windows-x86_64` | `Omsi-Hub_<version>_x64-setup.exe` | the same |
| macOS Apple silicon | `darwin-aarch64` | `Omsi-Hub_aarch64.app.tar.gz` | `Omsi-Hub_<version>_aarch64.dmg` |
| Linux x64 | `linux-x86_64` | `Omsi-Hub_<version>_amd64.AppImage` | the same, or the `.deb` / `.rpm` (not self-updating) |
| Linux ARM64 | `linux-aarch64` | `Omsi-Hub_<version>_aarch64.AppImage` | the same |

The names are Tauri's bundler's, from `productName` (`Omsi-Hub`) and the version. The look
matches the processor in the name: `x64`, `amd64` or `x86_64`; `aarch64` or `arm64`; a macOS
`universal` bundle when there is no own one.

## What Luc sets up (once)

0. **The releases' repository.** The app reads the releases without signing in, so they live in
   a public repository: this one, `Omsi-Hub/Omsi-Hub-Addon`. The release workflow writes them
   with this repository's own token; no secret is needed for it.

   The openOMSI fork Cargo fetches (`Luc-nbr/openOMSI`) is private: CI and the release read it
   with the secret `FORKED_OPEN_OMSI` (a fine-grained token with "Contents: Read-only" on
   `Luc-nbr/openOMSI`), or the fork is made public.

   `update.rs` (`REPO`, `API`, `LATEST_JSON`) and the updater's endpoint in `tauri.conf.json`
   point there; a copy built before 1.0.0 looked in `Omsi-Hub-Releases` and has to be updated
   by hand once.

1. **Make the signing key**, on your own computer:

   ```
   npx @tauri-apps/cli@2 signer generate -w ~/.tauri/omsi-hub-updater.key
   ```

   Choose a password. This makes `omsi-hub-updater.key` (private: never commit it) and
   `omsi-hub-updater.key.pub` (public). Keep the private key and its password somewhere safe. If
   they are lost, installed copies can no longer update themselves: players would have to
   download the next version by hand.

2. **Put the public key in the app.** Paste the contents of `omsi-hub-updater.key.pub` into
   `app/tauri.conf.json`, `plugins.updater.pubkey`, and commit it. It is public by design.

3. **Add two secrets to the repository** (GitHub → Settings → Secrets and variables → Actions):
   - `TAURI_SIGNING_PRIVATE_KEY`: the contents of `omsi-hub-updater.key`;
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password.

4. **The release workflow** (`.github/workflows/release.yml`, in place) signs the updater files and publishes `latest.json` once steps 1–3 are done; until then it builds the installers only. CI's own
   `tauri build --no-bundle` stays as it is: it needs no key. The release build adds
   `createUpdaterArtifacts`, with the two secrets in its environment. With `tauri-action` that
   is:

   ```yaml
   - uses: tauri-apps/tauri-action@v0
     env:
       GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
       TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
       TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
     with:
       projectPath: app
       tagName: v__VERSION__
       releaseName: Omsi-Hub __VERSION__
       releaseDraft: true
       prerelease: false
       includeUpdaterJson: true
       args: --config '{"bundle":{"createUpdaterArtifacts":true}}'
   ```

   Run it on `windows-latest`, `macos-latest` and `ubuntu-22.04`:
   - On macOS, use `--target aarch64-apple-darwin` and `--target x86_64-apple-darwin`, or
     `universal-apple-darwin`.
   - On Linux, the bundle targets are `appimage`, `deb` and `rpm` (`app/tauri.linux.conf.json`). The runner needs
     Tauri's packages first: `sudo apt-get install -y libwebkit2gtk-4.1-dev
     libayatana-appindicator3-dev librsvg2-dev patchelf`. Build on the oldest Ubuntu you
     support: an AppImage runs on the same or newer glibc only.

   Each run uploads its installer, its `.sig` and its part of `latest.json` to the same draft
   release.

5. **Publish each release** as a normal release: not a draft, not a pre-release. Its tag is
   `v<version>`, and the version is the workspace's (`Cargo.toml`, `[workspace.package]
   version`) and `app/tauri.conf.json`'s. A draft or a pre-release is never offered. That makes
   a pre-release the place for a test build.

## A pre-release (for testers)

1. **Write what is in it** in `CHANGELOG.md` (the release links there), under the version.
2. **Build it:** Actions → Release → Run workflow, on `main`, with "pre-release" on (it is off by default).
   The workflow checks that `Cargo.toml` and `app/tauri.conf.json` have the same version, and makes
   a draft named `Omsi-Hub <version>` with the pre-release mark and the installers of the four
   builds. Run it again after a fix: it uploads to the same draft.
3. **Try the installers yourself**, then publish the draft (Releases → the draft → Edit →
   Publish release). It stays a pre-release.

What it means for players:

- **Nobody is offered it.** The app's look skips pre-releases, and Tauri's updater reads
  `releases/latest`, which is never a pre-release. Testers download and install it themselves.
- **Testers are offered the next normal release** above it. So give that one a higher version
  (after a 0.1.0 pre-release, the first normal release is 0.1.1 or 0.2.0): a normal 0.1.0 would
  not be newer than the 0.1.0 they have.
- **While the repository is private**, only people with access to it see the release and its
  files: the download links do not work for anyone else.

`latest.json`, if you ever write it by hand:

```json
{
  "version": "0.2.0",
  "notes": "What changed",
  "pub_date": "2026-11-01T12:00:00Z",
  "platforms": {
    "windows-x86_64": { "signature": "<contents of Omsi-Hub_0.2.0_x64-setup.exe.sig>", "url": "https://github.com/Omsi-Hub/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_0.2.0_x64-setup.exe" },
    "darwin-aarch64": { "signature": "<contents of Omsi-Hub_aarch64.app.tar.gz.sig>", "url": "https://github.com/Omsi-Hub/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_aarch64.app.tar.gz" },
    "darwin-x86_64": { "signature": "<contents of Omsi-Hub_x64.app.tar.gz.sig>", "url": "https://github.com/Omsi-Hub/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_x64.app.tar.gz" },
    "linux-x86_64": { "signature": "<contents of Omsi-Hub_0.2.0_amd64.AppImage.sig>", "url": "https://github.com/Omsi-Hub/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_0.2.0_amd64.AppImage" }
  }
}
```

## Still to check on the first real release

- That an AppImage replaces itself and starts again on a Linux desktop. The look and the asset
  choice are tested; the replacement is the plugin's and has not been tried yet.
- That the NSIS installer starts the new version after a passive update on Windows.
- That macOS Gatekeeper lets the replaced bundle start. Without an Apple Developer ID
  signature and notarization (the bundle is signed ad hoc, `signingIdentity: "-"` in
  app/tauri.macos.conf.json), the first start after a manual download already needs System
  Settings › Privacy & Security › "Open Anyway" (macOS 15 no longer lets right-click → Open
  through). An update keeps the bundle's quarantine state, but this has not been tried yet.
