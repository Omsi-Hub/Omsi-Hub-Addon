# What changed

Each version of the Omsi-Hub add-on, newest first. The release on GitHub links here.

## Unreleased

### New

- **Phone & tablet in the browser, no app needed**: with Settings › General › Phone & tablet
  on, Omsi-Hub serves the Companion as a web app on its address (`http://<your computer>:47811`).
  Scan the QR code with the phone's camera, or open the address in the phone's browser and type
  the pairing code; then everything the Companion app does is there - signing on with your
  driver's pass, the duty and the duty order, taking a duty from the map's lines, the navigator,
  the stops and trips, the control room's messages, the break, and the bus's IBIS and ticket
  printer drawn as in the cab. The QR code now opens this page (the Companion app's own
  scanner reads it too).
- **Phones get in, whatever Windows' firewall said**: on Windows, turning Phone & tablet on
  sees to it that the firewall lets phones and tablets in - on a network Windows calls public
  too, and after its question was once clicked away. Windows asks once whether Omsi-Hub may;
  if that is clicked away, Settings says so beside the pairing code with a button *Let phones
  in*.
- **A shift between two times**: besides its length and the part of the day, a work shift can
  now be asked for by its times - choose *From – to* in the shift step and set, say, 05:00 and
  12:00. The shifts that fill those hours come into the list at once, each beginning at or after
  the first time and ending by the second; a shift from 22:00 to 06:00 runs past midnight.
- **Sign in with Discord**: the account panel and the welcome have *Sign in with Discord* under
  *Sign in with Google*, through your browser as Google does. An account made with Discord on
  the website now signs in here too, without a password, and signing in with Discord links that
  Discord account to yours for the roles on the Omsi-Hub Discord server, as on the website.
- **The mod installer on the start page**: *Install mods* among the links under the ways to
  drive opens Settings straight on its tab Mods, to install a mod file or folder into openOMSI.
- **Searching for a button's action**: on Settings › Controllers, giving a button of the wheel
  or button box an action opens a search field over the actions, still by group - type *door*
  or *indicator* and only those are left. The arrow keys and Enter pick one, Escape closes it.

### Changed

- **Everything in the game is in the navigator, and notifications click away with the mouse**:
  the company's notices, service control's and the control room's messages, the offences,
  the reliefs and the on-time alerts are no longer cards at the top right that cannot be
  clicked; they come into the navigator under its map, the newest on top, each with a close
  button that works with the mouse as it is (no F10). An urgent message or an alert stays until
  you close it, the others go by themselves after a while. The control room's inbox, the
  company's panel and signing on, the duty board, the on-time card and the duty strip are in
  the navigator too, one under the other, instead of panels of their own scattered over the
  screen: move the navigator by dragging its map, size it at its corner (Settings › General ›
  Navigator as a widget) and Shift+N shows and hides it. Only typing your personnel number
  still asks for F10. An openOMSI without the navigator's board keeps the panels of their own.
- **Buses for the AI traffic are no longer offered to drive**: a bus with "AI" or "KI" in its
  folder, its file name or its name (`MAN_SD202_AI`, `KI-Busse`, `Citaro KI`), or marked
  `[ai_veh_type]`, is left out of the bus list of a drive, the company's market and dealer,
  the line editor's bus choice and the livery studio; a drive with such a bus, chosen before,
  is refused with a word why. Such a bus the company bought before stays in the fleet; give
  its tours another bus to drive them yourself.
- **Signing on with your personnel number is back, and always asked for a company's duty**:
  in the bus company mode you sign on in the game with your driver's pass (personnel number
  and code, made for you the first time and shown on the company's panel and in the
  Companion) before the duty order and the IBIS; there is no switch to leave it out. A drive
  of your own (a free drive, a line and tour you chose) never asks for it.

## 1.0.1

A patch on 1.0.0. It still needs **openOMSI 0.2.22 or newer** and an original **OMSI 2**.

### New

- **Mods straight into openOMSI**: the new tab "Mods" on the Settings page installs a `.zip`, a
  `.7z` or a folder with openOMSI's own installer into the content folder of the openOMSI you
  play, its progress shown and cancelled if you want. It lists the mods there as openOMSI keeps
  them, so they are on openOMSI's own Mods page too, each switched off and on or deleted - never
  while the game runs. A `.rar` is refused, with what to do instead.
- **A co-owner drives a duty from the company's pages**: "Drive this duty" in RosterFlow and
  the planning now starts the duty in your own game, given to yourself when your role may give
  duties and taken otherwise, where before it asked the boss's launcher ("Only the boss can do
  this").
- **Every text in every language**: the texts that lacked a translation are translated (Dutch,
  German, French, Russian, Ukrainian, Polish), wrong translations put right, and the folders'
  checks, the drive's errors and the company ledger's lines are said in your language too.
- **Privacy Statement 4** (effective 24 October 2026), as Omsi-Hub is now: the add-on with its
  plugin in openOMSI and the Companion app, the profile photo, the files sent with a cloud save,
  multiplayer as openOMSI's, and bug reports.

### Fixed

- **The mods you see in openOMSI are the ones Omsi-Hub reads**: Omsi-Hub follows the openOMSI
  that openOMSI's own launcher plays (for the lists, the drive and the plugin), and Settings says
  so when that is another than the one you chose. Archives openOMSI uses in place (a mod too big
  to unpack) are in the lists too.
- **The buses of your own lines are on the fleet map**: the map and the duty card take the
  trips' routes from the timetables openOMSI reads, the line editor's lines included.
- **The livery studio no longer paints a part the model repeats everywhere at once**: a picture,
  text, shape or brush stroke on one of the NG313's four door posts, say, lands only on that one,
  in the studio and in the saved livery.

- **A drive no longer stops with "ran longer than 50 ms"**: openOMSI switches a plugin off when
  one call into it takes longer than 50 ms, the game's own work inside counted, and taking a
  duty (the button, the start by itself, a duty's next part, a relief, a short-turn, the phone)
  makes its route and the timetable paper, which on a large map takes about that long alone.
  The plugin (1.14.0) now takes the duty in a call of its own and does the rest in the next.
- **Your own liveries are offered for every bus, as openOMSI offers them**: every livery of a
  `.cti` file (one file may hold several), every paint folder of the bus's model, and the
  repaints in openOMSI's content folder (where its mods and the livery studio put them), not
  only the OMSI 2 folder. The company's livery choice for its buses lists them too.
- **macOS no longer says Omsi-Hub "is damaged and can't be opened"**: the app as a whole is
  now signed (ad hoc, without an Apple Developer ID), where before only its program was and the
  bundle around it was not, which macOS calls damaged once a browser has downloaded it. It is
  still not notarised: the first time, macOS says it cannot check the app; choose Done, then
  System Settings › Privacy & Security › "Open Anyway". 1.0.0 already downloaded opens after
  `xattr -cr /Applications/Omsi-Hub.app` in Terminal.

## 1.0.0

The first Omsi-Hub that is an add-on: its own app beside the openOMSI you have installed, instead
of a game of its own. It needs **openOMSI 0.2.22 or newer** and an original **OMSI 2**, and it
never changes their files.

### In it

- **Phone and tablet**: the Omsi-Hub Companion app (iPhone, iPad, Android) pairs with Omsi-Hub
  in your own network - switched on in Settings › General › Phone & tablet, then its QR code
  scanned, or the address and the six-digit code typed. While you drive it shows your duty and
  its trips, the trip's stops with their delays, the route on the map with the next turn, your
  break between trips and the control room's messages (answered there too); you sign on there,
  sign the duty order or drive without a duty, and work the bus's IBIS and ticket printer as
  in the cab; the bus on its map moves every frame of the screen (the game's place thirty times
  a second). Off by default; only paired devices of your network get in (docs/COMPANION.md).
- **The bus's own IBIS and ticket printer on the phone and tablet, 1:1**: Omsi-Hub reads the
  devices of the bus you drive from its own files - the `model.cfg`, the `.o3d` parts, the
  textures, OMSI's fonts - and the app draws their screens as the game draws them: the pages,
  the text fields in the bus's font, the touch fields. On an iPad you work a touch-screen ALMEX
  or IBIS with your finger; the keys beside the screen come in the rows they have on the device.
  Only the taps and keys of the bus's devices can be pressed. This is a port of the device
  pipeline of Luc's career add-on for OMSI 2 (github.com/Luc-nbr/Omsi-career-addon); the plugin
  is 1.13.0.
- **Releases and updates** come from the public `Omsi-Hub/Omsi-Hub-Addon`, and the libraries'
  licences travel with the app (`third-party-licenses.txt`).
- **The welcome** on the first start, as in the old Omsi-Hub: the language, the animations, your
  openOMSI folder and your OMSI 2 folder (it suggests the ones it finds, on Steam too, and checks
  openOMSI's version), your **account** - sign in, or continue locally without sync -, your driver
  and the way on (the tour, or straight to the start). Before the first drive, Omsi-Hub puts its
  plugin into openOMSI (Settings, "openOMSI and OMSI 2", switches it off or removes it).
- **Windows blocking openOMSI** (Smart App Control, "os error 4551"): Omsi-Hub takes the
  "downloaded from the internet" mark off openOMSI's program and its libraries before it starts
  them, and where Windows still refuses it, says so and what can be done.
- **The depot in 3D**: the yard as a model to turn round and zoom into (and from straight above,
  2D), every bus in it as its own model in the livery it wears (baked once from the bus as the
  game draws it), a hundred and more of them smoothly. Build it up as you like: put each block of
  spaces, workshop bay, wash lane, diesel station and the offices where you want them on the
  yard's grid, turn and move them, pull them down, buy more land beside it. The graphics can be
  set lower for slower computers.
- **Your depot, built by you**: a company founded now starts with its land only and a building
  credit for a first depot (spent before its cash). Until it has a workshop its buses go to a
  garage outside, dearer and a day longer; until it has spaces they stand in the street. Every
  area grows - more wash lanes, up to six diesel pumps - and has things to add: lights and
  cameras, carports or a parking hall, a paint booth, an inspection lane, a tyre store, wash
  gantries, interior cleaning, an underbody wash, water recycling, a bulk tank, AdBlue, a canteen,
  a control room, a training room and a solar roof, each with what it costs and does. Companies
  of before keep the yard they rented.
- **The depot's look**: its walls', roofs', trims' and doors' own colours, and walls of steel
  cladding, brick or plaster.
- **The depot in its town**: streets with their pavements, worn road markings, zebra crossings,
  street lamps, trees and parked cars round it, the trading estate's halls next door and blocks
  of flats further out, their windows lit in the evening; the surfaces textured (asphalt, concrete,
  brick, the wash bay's tiles, the workshop's coated floor ...) with cracks, tyre tracks and oil
  where they are used; and the company's people at work - mechanics at the buses in the bays,
  cleaners in the wash lanes, drivers on their way to their buses -, each role a cartoon figure
  of its own (the driver, the mechanic, the cleaner, the service employee, the inspector, the
  researcher) that walks and, where it works, stands at ease.
- **The depot as a dashboard**: beside 3D and 2D, the depot at a glance - tonight's buses on the
  yard, in the workshop and in the street, the staff, the upkeep, and each area with how full it
  is, what has been added and what is being built.
- **A profile photo**, the same in the app and on the website: upload it in the account panel or
  on your service record (a click on the picture), or on omsihub.com; your friends, the
  leaderboard and the website show it.
- **The leaderboard gets your figures**: signed in, Omsi-Hub sends your driver's points, level,
  duties, time driven and kilometres whenever they change (after every drive).
- **One account, one driver**: signed in, your driver is your account's (the display name's), with
  the friends, the leaderboard and the cloud saves. Other drivers on the same computer play locally.
- **Your old saves** from the Omsi-Hub that came with its own game are offered on the first start:
  shown first, then copied (your companies, careers and settings). The old folder is left as it was.
- **The bus stop signs** on the maps - the drive's route map, the line editor and the company's
  fleet map -, as you choose them in the welcome or the settings: the German H (the default), the
  British roundel, the French, the Dutch, the Polish, the Russian or the Ukrainian sign. The route map of the
  drive's steps is the old one again: its stops with their times, dragged to move, the wheel or
  the + and - to zoom.
- **Omsi-Hub's panels in the game, your size and your place**: with F10 each one (the duty board
  beside the navigator, the company's panel, the duty strip, the on-time card, the control room's
  messages) shows a size row: smaller, larger (60 to 200 %), and back to how it came. Dragged to
  another place, a panel comes there again in your next drive. In an openOMSI with the navigator's
  board (plugin UI 3, in openOMSI after 0.3.36), the duty board is in the navigator again, as in
  the old Omsi-Hub, and is sized and moved with the navigator.
- **Blue** is Omsi-Hub's colour unless you choose another (Settings, Accent colour).
- **The drivers on this computer**, as in the old Omsi-Hub: "Who is driving today?" is the drive's
  first step again, with a tile for each driver (OMSI's own personnel files, `Drivers/<name>.odr`),
  a new one made in place and one deleted after asking. The game is started with the driver
  chosen (`--driver`), so it keeps their figures; the **service record** shows them with the
  level, the experience and the recent runs openOMSI wrote down.
- **Four ways to drive**: a work shift built from the timetable's real trips, a tour, a free drive,
  and the bus company. Omsi-Hub starts openOMSI with the map, the bus and the duty; the plugin
  shows the duty in the game and sends the trips back when you are done.
- **The bus company and RosterFlow**: buses, drivers, lines, the days' plans and the rosters, with
  control-room messages and instructions carried out in the game.
- **The editors**: the timetable editor (a map's lines and tours) and the depot editor (depot files
  of your own, for the IBIS and the destination displays). What they write goes into openOMSI's
  content folder or Omsi-Hub's own data folder, never into the OMSI 2 folder.
- **Online, optional**: an account (email or Google) for friends, the leaderboard and up to two
  cloud saves. Without one, everything works offline.
- **Windows, macOS (Apple silicon and Intel) and Linux** (an AppImage, or a `.deb` / `.rpm`).
- **English and six translations**: Dutch, German, French, Russian, Ukrainian and Polish, as the old
  Omsi-Hub had them.
- **openOMSI's own settings in Omsi-Hub** (Settings → openOMSI's settings): every key of
  `~/.openomsi/settings.cfg` that openOMSI's user guide names, 62 of them: the graphics (with
  Enhanced and Enhanced+, the view distance, the video memory, LED displays, rain and puddles),
  the mirrors, the screen and the navigator, the game's language, sound and the radio, driving and
  controls, and the passengers and the traffic. Omsi-Hub leaves the rest of the file as it is;
  "openOMSI's default" takes a key out again. Not while the game runs.
- **The installers in Omsi-Hub's look**: on Windows a welcome and a finish page with the logo, a
  picture beside every page's title, the licence, a choice of language (English, Dutch, German,
  French, Russian, Ukrainian) and a folder of its own in the Start menu; on macOS a disk image
  with its own window to drag Omsi-Hub into Applications; on Linux the packages' description.
- **The maps are drawn**, as the old Omsi-Hub drew them: the route map of the drive's steps, the
  company's duty map and its fleet map show the roads the game's own city map draws, read from the
  map's tiles (its splines' and its junctions' lanes), and the way each trip drives on its own
  lanes, with its stops.
- **Depot files read right**: a destination's strings were read a place off (a line OMSI's files
  do not have was taken for the stop it ends at), so its IBIS text was lost.
- **The bus in 3D** on the bus step, as the old Omsi-Hub drew it: the page's own 3D view reads
  every bus as the game reads it (its meshes, its textures, the livery and the bus options chosen,
  what its scripts show at rest), turned and zoomed with the mouse, with the views from the
  front, the side, the back and the corner, and the driver's and a passenger's view.
- **The buses' photos** on the fleet's, the dealer's and Casual's tiles and on the bus step, taken
  by the same 3D view (transparent, from the front corner) and kept until the bus changes. The
  photos the old Omsi-Hub took are still shown until then.
- **The livery studio**, as in the old Omsi-Hub: paint a bus in layers on the bus itself -
  colour areas, stripes, texts, pictures, own shapes and the brush, mirrored onto the other side -
  and save it into the game as a livery of its own (openOMSI's content folder, never the OMSI 2
  folder). The liveries begun before are opened from the same folder.
- **The line editor**, as in the old Omsi-Hub: a line of your own on any map - click its stops
  on the map and the way between them is found over the map's own lanes (the kerb side, lane
  changes), the way back made from the stops across the road, a leg dragged through a point;
  its destinations, its timetable and its kind of service; saved into the map's timetable so
  you and the timetable's buses drive it. For the bus company: its map, its lines, the estimate
  and "Confirm and pay". A line of the map's own opens to change it.
- **Multiplayer**, as in the old Omsi-Hub (on the start, "Multiplayer"): host your next duty
  and give friends its session code (at home and over the internet), join a friend's session by
  their code, or join a server that is always on - openOMSI's official one is listed first, and
  you can add your own. The game opens and joins the session itself (openOMSI's `--lan-host` and
  `--lan-join`), with the map, time and weather of the host; your driver's name is the one the
  others see.
- **Playing together**, as in the old Omsi-Hub: the boss shares the open company with a session
  code, friends join it in their own Omsi-Hub with the role the boss gives them and drive their
  duties in their own game. Over the LAN or a VPN, and over the internet through a Cloudflare
  tunnel whose address goes over openOMSI's own relay (no port forwarding needed).
- **The game's buses and traffic on the fleet map**: while you drive on the company's map, your
  bus and every bus of the company's tours the game drives are where the game has them, with the
  game's delay; the game's cars, trams and bicycles, the buses of the lines the company does not
  run and the other players' buses in a LAN game are drawn too (a switch in the panel hides the
  traffic). Needs an openOMSI with `omsi.traffic.list` and `omsi.timetable.buses` (0.2.22); the
  launcher shows whether yours has them.

### Not in it yet

- **What needs a newer openOMSI plugin API** than the official releases have: the bus's own display
  font, the IBIS typed in for you, closed stops struck through on the navigator, and the duty board
  docked in the navigator. The launcher shows which ones your openOMSI supports.

### Good to know

- **Updates.** Omsi-Hub 1.0 does not install updates itself yet: when a newer Omsi-Hub is out,
  the app says so and opens its page to download it.
- **macOS.** The app is not notarised: the first time, right-click it and choose Open.
- **Something wrong?** Tell us at [info@omsihub.com](mailto:info@omsihub.com) or in an issue, with
  the log: `omsi-hub.log` in Omsi-Hub's data folder (`%APPDATA%\Omsi-Hub` on Windows,
  `~/Library/Application Support/Omsi-Hub` on macOS, `~/.local/share/Omsi-Hub` on Linux).
