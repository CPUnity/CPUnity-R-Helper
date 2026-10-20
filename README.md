<p align="center">
  <img src="assets/logo.png" alt="CPUnity logo" width="160">
</p>

# CPUnity:R-Helper

A small Windows app to control a Razer Blade **without Synapse**: performance modes, CPU/GPU boost,
fan control, keyboard and logo lighting, battery charge limit, and a built-in switch that parks
Razer Synapse so it stops fighting your power settings.

Fork of [R-Helper](https://github.com/Fatalution/r-helper) by Fatalution (MIT), which is built on
[razer-ctl](https://github.com/blauzim/razer-ctl)'s `librazer` (MIT).

<p align="center">
  <img src="docs/screenshot.png" alt="CPUnity:R-Helper, Main tab" width="300">
  &nbsp;&nbsp;
  <img src="docs/screenshot-lighting.png" alt="CPUnity:R-Helper, Lighting tab" width="300">
</p>
<p align="center"><sub>Main tab (left) and Lighting tab (right)</sub></p>

## Features

**Performance**
- Battery, Silent, Balanced, Performance, Hyperboost and Custom modes.
- In Custom: CPU and GPU boost presets, including CPU Overclock.
- AC / battery profile: pick a mode for each power source. An explicit choice is applied when the
  app starts and whenever you plug or unplug. "Default" keeps the original behaviour on plug /
  unplug (Performance on AC, Battery on battery) and leaves the current mode alone at launch.
- Optional "Overclock on AC power" (Custom mode + CPU Overclock).

**Fan**
- Auto or Manual with an RPM slider, live RPM readout, and a **Max Fan** button (Manual + slider
  at maximum, one click).

**Lighting and battery**
- Lid logo mode (Static, Breathing, Off), keyboard brightness, "backlight always on".
- All lighting lives on its own **Lighting** tab (click it next to Main), away from the main page.
- **Keyboard effects (opt-in).** Tick **Toggle App Lighting Control** to get Static, Breathing,
  Spectrum, Wave (both directions) and Off, plus a color picker. It is off by default: R-Helper
  only sends an effect when you click one, so Razer Chroma / Synapse stay in charge until you ask
  otherwise.
- **Two colors.** Tick "Two colors" next to the color button (keyboard and cooling pad): Breathing
  alternates between both colors, and Static shows both (keyboard: Left | Right, Top | Bottom or
  Gradient; pad: two halves or a gradient). Breathing has a **Layout** row too: Alternate (the
  firmware effect), or Left | Right / Top | Bottom / Gradient (keyboard) and Two halves / Gradient
  (pad), where both colors breathe together. Those layouts are drawn by R-Helper frame by frame, so
  they only animate while R-Helper is running.
- **Sync keyboard + cooling pad.** A switch between the Keyboard & Logo card and the Cooling Pad
  card (shown while a pad is connected). When it is on, the separate keyboard effect and pad
  lighting controls are greyed out and one set of controls (Static / Breathing / Off, color, two
  colors, layout, speed, pad brightness) sets both at once. The pad shows Top | Bottom as two
  halves. Turning it on also turns on Toggle App Lighting Control for both devices.
- **Cooling pad fan: app control, profiles and drop-downs.** The Cooling Pad card on the Main tab is
  a drop-down with a **Toggle App Control** switch (off by default): until it is ticked R-Helper
  never changes the pad's fan, and the last fan setting is only put back at startup (and when the
  pad is plugged in) while it is on. **Fan profiles** holds up to 10 named speeds in exact RPM
  (steps of 50), each in its own drop-down with Apply / Use current / rename / delete. The Razer
  Synapse Standby card is a drop-down too, and switching on Sync folds the keyboard and pad
  lighting drop-downs away.
- **Sensors tab (opt-in).** A **Read sensors** switch starts live readings with *now / average /
  maximum*: GPU temperature, power, clock and load, GPU memory junction (only if the NVIDIA driver
  reports it), and on the CPU side package power (when Windows exposes its Energy Meter counter),
  clock, load and the hottest ACPI thermal zone. Nothing is read, and no thread runs, while the
  switch is off; the built-in sources need no driver and no administrator rights. CPU package
  temperature and PL1 / PL2 need a program that has a kernel driver, so **Options > Sensor
  sources** lets you switch on any of **HWiNFO** (shared memory), **LibreHardwareMonitor** (its web
  server), a **CSV log file** (ThrottleStop / Intel XTU / HWiNFO logging; those two have no live
  interface) and the **PawnIO** driver (direct CPU registers; needs PawnIO installed, its signed
  `IntelMSR.bin` module and R-Helper run as administrator; add `Nvidia.bin` next to it for the GPU
  memory junction temperature). Several can run at once, and for each
  reading (CPU package temperature, PL1, PL2, package power, GPU temperature, memory junction, GPU
  power) you pick which source it comes from, or leave it on Auto.
- **Sensor graphs and logging.** Below the readings, the Sensors tab draws live graphs
  (temperature, power with PL1 / PL2 limit lines, clock speed, load) over 1 min to 1 hour, with a
  gradient fill, a pulsing "now" dot, a hover read-out and clickable legend entries. **Log readings
  to a CSV file** writes one line a second to `%APPDATA%\CPUnity-R-Helper\logs` (**Open folder**
  jumps there). Both only run while **Read sensors** is on. The graphs are a drop-down behind a
  **Live graphs** switch (off = nothing recorded or drawn), and a **Refresh every** slider with a
  **Default** button (1 s) sets how often the sensors are read. Sensor settings (including whether
  *Read sensors* was on, and hidden graph lines) are remembered; logging is not, it is always off
  after a start.
- **Files tab.** One click opens any place R-Helper keeps data (settings, sensor logs,
  diagnostic log, program folder) in File Explorer. Below the tiles, a drop-down per program that
  R-Helper reads from (ThrottleStop / XTU / HWiNFO log, PawnIO modules, LibreHardwareMonitor,
  HWiNFO) lets you type where its file is or pick it with a folder button.
  The PawnIO one has a **Get Modules Here** drop-down with 🌐 links to the driver and the module
  downloads.
- **Log viewer.** In the Logging card of the Sensors tab, under the logging switch: open any sensor log as graphs with a lowest /
  average / highest table, instead of reading rows of numbers.
- **Changelog and Info tabs.** *Changelog* lists every change per version (0.6.0 is the first
  version after 0.5.1). *Info* shows a welcome from the author, a where-to-start guide, *The Rebellion*, *About CPUnity*,
  *Future Endeavors*, *Ways to contact* (click the email to copy it, or use the world button for
  the GitHub page) and a *P.S* drop-down. Its text is built into the program
  (`assets/info.txt`) and cannot be changed from inside it.
- **Always run as administrator (opt-in).** Options > *Always run as administrator* restarts
  R-Helper elevated at launch (needed by the PawnIO source); there is also a *Restart as
  administrator now* button. Under it, **Skip the permission prompt at login** can create a
  scheduled task for that (after a confirmation window that shows what it will do), so no Windows
  prompt appears at every login.
- **Close button minimizes to the tray (opt-in).** Options > *Close button minimizes to tray*:
  the X hides the window and leaves R-Helper running in the notification area; click the icon to
  show it again, right-click for Show / Quit. Off (default): X quits.
- **Drop-downs remember their state.** Whether each card drop-down is open or closed is kept
  between launches.
- **Switches instead of checkboxes.** On / off options are side-flip switches: dark grey when off,
  your accent color (Options) when on.
- **Breathing speed.** Breathing has a **Speed** row (keyboard and cooling pad): Standard keeps the
  device's own Breathing; Slow, Fast and Faster are drawn by R-Helper, so they work for a single
  color and for Alternate too, but only while R-Helper is running.
- **Custom Keys (keyboard).** Pick the "Custom Keys" effect to light exactly the keys you want:
  choose a brush color, click keys on the on-screen keyboard to toggle them (right-click turns a
  key off), and set the color for all other keys (or leave them dark). Quick sets give you WASD,
  WASD + arrows, FPS, numbers, F-keys, HJKL and modifiers in one click. Custom layouts can be saved in
  lighting presets. If a different key lights than the one you clicked, use "Key map fine-tune"
  to shift the map by a column.
- **Lighting presets: up to 20 each for the keyboard, the cooling pad, and both together.** The
  presets card is a drop-down on the Lighting tab that only lists the presets you have saved and
  says how many ("3 of 20 saved"). Set the lights up, optionally type a name, press **Save as
  new**; **Apply** brings a preset back, **Update** replaces it with the current lighting and **X**
  deletes it. A "both" preset can hold the keyboard part, the pad part, or both. Presets live in
  `config.txt`, so they work with Synapse parked.
- **Restore lighting at startup (opt-in).** Off by default. When on, the last keyboard effect and
  pad lighting you applied are put back when the app starts (and when the pad is plugged in again),
  provided "Toggle App Lighting Control" is on for that device.
- **Battery Health Optimizer** with a charge limit from 50% to 80% in 5% steps.

**Razer Synapse Standby** (built in, so you no longer need a separate Synapse killer tool)
- **Standby** stops Synapse services, scheduled tasks, autostarts and processes. The laptop keeps
  the last settings that were applied.
- **Keep Chroma lighting running** leaves only the Chroma half of the Razer engine alive, so
  animated RGB keeps working.
- **Auto-park at every boot / logon** installs a startup task (`SynapseStandby-Guard`) that parks
  Synapse on every boot.
- **Resume Synapse** undoes everything Standby changed and removes the boot task.

**Cooling pad** (new, shown only when a Razer Laptop Cooling Pad is plugged in)
- Fan: Manual or Off. In Manual, pick a speed from a dropdown in 5% steps (15-100% of the pad's
  maximum, shown with the RPM), or use one of **three saved presets** (click to apply, right-click to
  save the current speed into that slot). An "Exact RPM" slider (500-3200 RPM) is tucked away for
  fine tuning.
- Your last fan setting is re-applied when the pad is plugged back in or the PC wakes from sleep.
- **Lighting is opt-in.** "Toggle App Lighting Control" is off by default: while it is off,
  R-Helper never opens or touches the pad's lights, so Razer Chroma / Synapse stay in charge. When
  you switch it on (Lighting tab) you get Static / Breathing / Off, color and brightness, and the
  lights only change when you click something there, or when you chose "Restore lighting at
  startup".

**Look**
- Dark card layout, resizable window that fits its content, selectable UI font.
- Accent color with presets, RGB boxes, hex, a visual picker and 10 saved colors. The logo and a
  faint backdrop behind every tab follow it (Options > UI Options > **Backdrop** turns it on;
  **Backdrop style** picks Circuit, Predator cat, Cat pack, Fire dragons, Constellations or
  Sunbeams).
- **Options > UI Options > Skins** puts on **Midnight** (deep blue-black and periwinkle, a starlit
  line on every card) or **Copper** (warm charcoal and copper, a copper corner on every card).
  **Pairs** puts on a skin and the backdrop made for it with one click (Midnight with
  Constellations, Copper with Sunbeams). A small box beside each opens a preview. Each skin has a
  lighting preset of its own (Lighting Presets > Skin Sets).
- **Options > Animate backdrop** (shown for the backdrops that move) turns their movement off.
- **Options > Start minimized to tray** starts the program hidden next to the clock.
- **Options > Tabs** (a drop-down) switches the Lighting, Sensors, Files, Changelog and Info tabs on or off, so
  you can keep just Main. The choice is remembered.
- Optional `logo.png` and `banner.png` (see Custom images below).

## Supported models

The model is detected from the laptop's model number. The controls only work on these Blades:

| Model | Model number |
|---|---|
| Razer Blade 14 (2022) | RZ09-0427 |
| Razer Blade 15 (2022) | RZ09-0421 |
| Razer Blade 17 (2022) | RZ09-0423 |
| Razer Blade 14 (2023) | RZ09-0482 |
| Razer Blade 16 (2023) | RZ09-0483 |
| Razer Blade 16 (2025) | RZ09-0528 |
| Razer Blade 14 (2025) | RZ09-05306 |

**Tested on:** Razer Blade 16 (2023) only. The original R-Helper features should behave as upstream
on the other models, but the CPUnity additions (Synapse Standby, Max Fan, battery limit values,
launch profile) have not been tried there. Reports are welcome.

On an unsupported laptop the app starts but shows "No Razer Device" and the controls stay inactive.

## Install

1. Download the zip from the Releases page and extract it anywhere.
2. Run `CR-Helper.exe`. No installer, no driver, no admin rights needed for normal use.
3. Windows SmartScreen may warn about an unsigned app. Choose **More info > Run anyway** if you
   trust the download.

Only the Synapse Standby buttons ask for administrator rights (a Windows UAC prompt).

## Build from source

Requires Windows and the stable Rust toolchain.

```
cargo build --release
```

Output: `target/release/CR-Helper.exe`. The included GitHub Actions workflows also build it:
`build` on every push, and `release` when you push a tag such as `v0.5.0`.

## Synapse Standby in detail

Razer Synapse can re-apply its own power profile when it starts, which can override limits you set
elsewhere (for example Intel XTU), and it keeps background services running. Standby stops that.

- **Standby** (Keep Chroma unticked) kills the Razer processes and stops and disables every
  Razer service, scheduled task and autostart entry. Kernel drivers (`rz*.sys`) are not touched, so
  your keyboard and mouse keep working.
- **Keep Chroma lighting running** ticked leaves the Chroma half of the Razer engine alive and only
  parks a short list of services (Razer Experience Service, Game Manager, RGS and Chroma Stream
  Server). The Razer Elevation Service is left alone.
- Everything it changes is recorded in `C:\ProgramData\SynapseStandby\state.json`, so **Resume
  Synapse** can put it all back.
- The boot guard task runs as SYSTEM and uses its own copy of the script in
  `C:\ProgramData\SynapseStandby\`.
- Only settings stored in the laptop's firmware persist while Synapse is parked. Software-driven
  features (Chroma effects, macros, per-game profiles) need Synapse or the Chroma engine running.
- While fully parked, starting Razer apps by hand can cause repeated UAC prompts. Click **Resume
  Synapse** first.

To undo everything: click **Resume Synapse**. To remove the app's files: see below.

## Custom images and fonts

- **Banner**: `banner.png` (or `.jpg`) fills the header space right of the title. It is cropped
  from the center to fit. Options > Custom Image shows the exact size for your screen; a 7:1 image
  such as 1400 x 200 px works well. Keep key art centered.
- **Logo**: `logo.png` (or `.jpg`), a small square image shown left of the title.
- **Font**: choose one in Options, or select **Custom (font.ttf)** and place `font.ttf` / `font.otf`
  next to the exe.

Put the files next to the exe or in `%APPDATA%\CPUnity-R-Helper\`, then restart. To bake the logo and
banner into the exe, place `logo.png` / `banner.png` in the `assets/` folder and rebuild; a file
next to the exe still overrides the baked-in one.

## Where data is kept

| What | Where |
|---|---|
| Settings, log, saved colors | `%APPDATA%\CPUnity-R-Helper\config.txt`, `log.txt` |
| Synapse Standby state (written by Standby, removed by Resume) and the boot-guard script copy | `C:\ProgramData\SynapseStandby\` |
| Boot task (only with Auto-park) | Task Scheduler: `SynapseStandby-Guard` |
| Launch at startup | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `CPUnity-R-Helper` |

To reset the app, close it and delete `config.txt`. Before deleting everything, click **Resume
Synapse** and untick "launch at startup" so the boot task and registry entry are removed too.

## Troubleshooting

- **Window laggy while Razer software runs**: Chroma and Synapse share the laptop's USB channel.
  The app backs off automatically, and parking Synapse removes the contention entirely.
- **Something looks wrong**: open `%APPDATA%\CPUnity-R-Helper\log.txt` and include it in a bug
  report. It records mode changes, slow device polls and Synapse actions.
- **Debug mode** (Options > Advanced) unlocks CPU/GPU combinations the firmware forbids and shows
  extra messages. Not saved; leave it off unless you know why you want it.

## Coming soon

CPUnity:R-Helper is more than a settings panel, and the rest is still being built:

- **Mini games**, hidden in the program for the curious to find.
- **A shop with its own currency**: play, earn, and spend it on new skins and backdrops.
- **An achievement system**: trophies for the things you find and finish.
- **More looks, more worlds, more lighting presets**, and so much more.

None of it is in this version. It arrives in updates, and each one will be in the changelog.

## Known limits

- The battery limit is written as a percentage register. 80% is the value Synapse itself uses.
  Every step (50 to 80% in 5% increments) was tested and holds on the Blade 16 (2023); on other
  models verify with a charge test before relying on it.
- **Raising the limit by a small step may not restart charging.** This is how the laptop's
  firmware behaves, not an app bug. Once the limit has been reached, charging only resumes after
  the battery has dropped about 10 points below the limit. Example: held at 70%, setting 80%
  starts charging (gap of 10), but going from 75% to 80% or from 55% to 60% (gap of 5) does
  nothing. To charge right away, set a limit at least 10 points above the current charge (80% is
  simplest), then lower it to the value you want; charging continues up to that value. Staying
  just under the limit is also normal: with an 80% limit the battery cycles between roughly
  70 and 80%.
- Keyboard effects use Razer's standard extended-matrix-effect command and are new; they are not
  yet confirmed on every Blade model. Wave and Spectrum ignore the picked color. If an effect
  ignores the color or looks choppy, try the "Keyboard effect tuning" switches in Options > Advanced
  and click the effect again; `log.txt` records which variant was sent.
- Two-color Static on the cooling pad is experimental (the pad's LED layout is assumed); if it
  looks wrong, tell me what you see. Two-color Breathing with the Alternate layout uses a standard
  Razer effect.
- Breathing with Left | Right, Top | Bottom, Gradient or Two halves, and the Custom Keys effect's
  key map, are new. The split Breathing layouts are repainted by R-Helper while it runs (they stop
  when you close it, and may look steppier than the firmware's own Breathing). On the cooling pad
  they are experimental and fall back to Alternate if the pad stops accepting frames. The Custom
  Keys map is built for the Blade 16 (2023) layout; on other models some keys may be off.
- Cooling pad support is new and has only been tried with one pad (USB 1532:0F43). Fan "Auto"
  (temperature-driven) is not included yet; use Manual or Off.
- Balanced and Performance can have the same CPU wattage on some laptops; the difference may be
  only fans or GPU behaviour, set by the firmware, not by this app.

## Shoutouts

Huge thanks to everyone who made this possible:

- **[Fatalution](https://github.com/Fatalution/r-helper)** - R-Helper, the app this is built on (MIT).
- **[blauzim](https://github.com/blauzim/razer-ctl) and tdakhran** - razer-ctl / `librazer`, the
  device layer (MIT).
- **[razer-laptop-control-no-dkms](https://github.com/Razer-Linux/razer-laptop-control-no-dkms)
  contributors** - protocol groundwork.
- **[NxRoot](https://github.com/NxRoot/SynapseKiller)** - SynapseKiller, the idea behind Synapse
  Standby (keep your Razer settings without Synapse running in the background). Please support
  their project too: if Synapse Standby saves you time, give
  [SynapseKiller](https://github.com/NxRoot/SynapseKiller) a star on GitHub.
- **[Robak08](https://github.com/Robak08/r-helper)** - Razer Laptop Cooling Pad support in their
  R-Helper fork (MIT), the groundwork for the Cooling Pad card.
- **egui / eframe, image-rs, hidapi-rs, windows-rs, ab_glyph, strum, serde, anyhow, rand, clap,
  embed-resource** - the Rust crates that carry the app.
- **The Rust project and community.**
- **CPUnity** - the project and its vision.
- **Volk-Märchen:VonV_T** - testing, art direction, relentless bug-hunting.
- **Claude (Anthropic)** - development assistant.
- **ChatGPT (OpenAI)** - logo and banner art.

## License

MIT license, see [LICENSE](LICENSE), [NOTICE](NOTICE) and
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md). The upstream notices must stay with any copy.

CPUnity:R-Helper is an independent, unofficial project. It is not affiliated with, endorsed by,
or sponsored by Razer Inc. "Razer", "Synapse", "Chroma" and "Blade" are trademarks of Razer Inc.,
used here only to describe compatibility.
