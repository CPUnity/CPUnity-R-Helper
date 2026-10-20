# Changelog

## 0.6.0 - 2026-10-20

### Added
- **Log file name presets:** two drop-downs under the File name box in the Logging card, which work together. **Built-in preset** is what goes around the name: **Date** (the default), **Time**, **Date and time**, or **None**. **Saved preset** lists the file names you have saved (up to 50): pick one and it fills the File name box, or pick **No preset** for no name of your own. **Save name** keeps what is in the File name box, **Remove** takes the shown one out; the built-in ones cannot be removed. Put together, Date and time with `3d-mark-cpu` makes `2026-10-20_3d-mark-cpu_114530.csv`; Date alone makes `2026-10-20_3d-mark-cpu.csv`; with no name the file is just `2026-10-20.csv`, `114530.csv` or `2026-10-20_114530.csv` (and `sensors-114530.csv` with nothing at all). A line under them shows the name the next log will have. Like the File name box, both are locked while logging is on.
- **UI Options** in Options, with three drop-downs: **Skins**, **Pairs** and **Backdrop**. A small box beside every skin, pair and backdrop opens a preview window (click it again, or the window's X, to close it); nothing has to be put on to see it.
- **Two skins come with the program:** **Midnight** (deep blue-black, a periwinkle accent, a thin starlit line along the top of every card) and **Copper** (warm charcoal, a copper accent, a copper corner on every card).
- **Two new backdrops, made for them:** **Constellations** (stars that twinkle and drift, joined by faint lines where they lie close) and **Sunbeams** (soft slanting beams of warm light that swell and fade, with dust rising through them).
- **Pairs:** a skin and the backdrop made for it, put on together with one click, each with its preview. **Midnight** and **Copper** are the first two.
- **Backdrops:** the picture behind every tab (Options > UI Options > Backdrop: a switch, a style and an **Animate backdrop** switch for the ones that move). six to pick from: Circuit, Predator cat, Cat pack, Fire dragons, and the two new ones, Constellations and Sunbeams.
- **A Help tab** (it is on when you first run this version, and can be switched off in Options > Settings > Tabs like the others). It explains each function in plain words, one drop-down per function, in the same text size as Info and Changelog. The cards that used to carry their explanation in small print now keep a **What does this do?** link that opens the Help tab at that function. The AC / battery profile's explanation moved there first, and the Logging card's is there too. From now on, descriptions of functions go in Help.
- **The Log viewer is a window of its own.** Opening a log (from the Log viewer drop-down, or **View live** / **View it** under the logging switch) opens a resizable window with the graphs at full size: the time axis shows real clock times (or time since the first reading; a switch flips it), **drag** a graph to move through the log, **scroll** to zoom around the pointer (ctrl + scroll and a pinch work too, a sideways wheel moves), **double-click** to show everything, and the arrow keys, + and -, Home and End do the same. A **crosshair** follows the pointer across every graph at once, with a read-out of the exact time and each line's value there. The text in the window is the same size as the Info tab's, and the panel on the left is a table: a switch for every line (remembered) with its **Lowest, Average and Highest** (temperatures, watts, clocks and loads), for the part on screen or for the whole log (a choice at the top); the vertical axis follows what is on screen. A strip under the graphs shows the whole log with the part you are looking at lit: drag it to move, click to jump. Zoomed in far enough every reading gets a dot, and a long log is thinned to the pixels without losing its spikes. A log that is still being written can be opened and followed live (it is read again every few seconds, and a window looking at the end keeps looking at the end).
- **Options inside the Log viewer** (the **Options** button on top): every part has its own text size (numbers panel, buttons, options panel, graph titles, axis numbers, crosshair read-out, the strip underneath, and an "Everything" slider, with Compact / Standard / Large / Huge presets); the numbers panel can sit left, right or be hidden, with the columns you want (Lowest, Average, Highest, Latest), decimals, row spacing, and a button that copies the table for a spreadsheet; temperatures in °C or °F; which graphs to show; the vertical axis fits what is on screen, the whole log, or reaches down to zero; grid lines and their strength; line thickness, glow, colour under the lines, steps instead of slopes, dots on readings, an average line and a marker on the highest and lowest reading of each line; the crosshair, read-out box, horizontal guide and "jump to the nearest reading"; the strip's height; what the mouse wheel does and how fast; a colour for every line; and keeping the window above the others. All of it is remembered.
- **The numbers panel fits itself to its width:** drag it narrower, or make the window shorter, and its text and spacing shrink together so the whole table stays in view (measured from the table itself, so it never clips or flickers while dragging; it stops at 75 % of the text size, where the panel stops getting narrower and scrolls). When the panel is thin it puts each line's numbers on a row under its name instead of shrinking the text, using the height of the window, so the names and numbers stay as large as they can, the how-to text moved to the bottom of the table so it never pushes the numbers out of sight, the columns stay lined up across all groups, and pointing at a line's name lights that line up.
- **CPU watts at short intervals:** the time between two reads of the energy counter was taken before the other register reads, so a slow read skewed the watts, which short intervals magnify. It is now taken the moment the counter is read.
- **Per-core CPU clocks:** the CPU group now shows the clock of the fastest core and the average of the performance cores and of the efficiency cores, read from each core's own register through the PawnIO driver (the old "Clock" is now "Clock (all threads)", Windows' own figure). They are in the logs, the live graph and the Log viewer.
- **GPU clock** is read through NVAPI (what Afterburner and HWiNFO use) first, and NVML only if that does not answer; NVML could report one unchanging value.
- **Sensors tab tidied with drop-downs:** the CPU and GPU groups of the table fold away by clicking their heading (remembered), the list of where the readings come from folds into a **Sources** drop-down that opens by itself when something needs attention, and the small print moved to the Help tab (new **Sensors** topic, which also explains why a short refresh interval shows higher maximums than a slower program).
- **Lighting tab:** the Keyboard & Logo card and the Sync Keyboard + Cooling Pad card now fold away like the other cards (remembered).
- **Logging can start and stop by itself.** **Start**: right away, or **when a program starts** (type its .exe name or pick one that is running; a new file is made every time it starts, named after the program and the time when you gave no name). **Stop after a set time** takes an exact length in hours, minutes and seconds (with quick buttons for 1, 5, 10, 30 minutes, 1 and 2 hours); the last reading is taken when the time is up. With a program, **Stop when the program closes** ends the file when it exits, and a log that ended by its time limit waits for the program to close and start again before it makes the next one. With no time limit and nothing to stop it, a log runs until you switch it off, endlessly if you like. Waiting for a program is remembered between launches; a log that starts right away never is. The line under the switch says what the log is doing (waiting, recording with its time and readings, how long is left, finished and why).
- **IA cores temperature is gone from Sensors** (the reading, its graph line and its CSV column). Older logs that still have the column open fine; it is ignored. A saved choice for it in the sensor sources is ignored too.
- **The tab row wraps** onto a second line when the tabs do not fit the window. Before, a row wider than the window made everything under it run off the right edge.
- **AC / battery profile: Custom has its own CPU and GPU drop-downs, and Last Known Settings.** Pick Custom for AC or for battery in the AC / battery profile and a CPU drop-down (Low, Medium, High, Boost, Overclock) and a GPU drop-down (Low, Medium, High) appear for it. Default there means the levels you last picked on the main page. **Last Known Settings** (a switch for AC and one for battery, separately) puts back exactly what the laptop was running last on that power source, the mode and for Custom the CPU and GPU, at launch and when you plug or unplug; it replaces that source's choices while it is on, and a source with it off keeps its hand-set profile. What is running is written down once it has stayed the same for two checks in a row (and never in the few seconds after a profile was applied). Until something has been written down, the hand-set profile applies. Turning the switch on or off starts the record afresh.
- **Custom Keys** has more for each key. Breathing, Fire, Starlight, Color Dipp and Wheel can take a **second color** (a "Two colors" switch next to the brush; Color Dipp always uses one): Breathing takes turns between the two, Fire goes between the second color when the flame is low and the first when it is high, a Starlight star is followed by the same star in the second, and Color Dipp goes from the first color to the second through the hues between. **Starlight**, **Color Dipp** and **Wheel** are new key effects. Wheel is a hand like a clock's sweeping clockwise round the middle of the keyboard, with a tail that fades (use All keys for the whole keyboard). A strip along a key in the editor shows its second color. Saved layouts from before still load.
- **Wheel**, a keyboard effect in the Lighting card and a mode of the cooling pad's Light, made the way Razer's own is (worked out from USB captures of it, both ways round): the rainbow is laid round a point on the Shift row and turns once a second, so each key shows the color that is at its angle; the pad's two halves of lights are half a rainbow apart. Choose clockwise or counter-clockwise and the speed. A **Choose colors** switch (keyboard and pad) swaps the rainbow for two colors you pick: laid out the same way, it goes from the first to the second through the hues between, as Color Dipp does, over half a turn and back over the other half. With Sync on, the pad follows the keyboard's Wheel and its colors. R-Helper draws it, so it runs while R-Helper is open. Saved in presets.
- **Wave** (left and right) has a **Choose colors** switch in the Lighting card: with it on, the wave washes between two colors you pick instead of the built-in rainbow, at your speed. Like Wheel it is drawn by R-Helper. Saved in presets.
- **Options** is laid out on separate back panels, one for each group of settings (UI Options, Appearance, Settings, Advanced), so they are easy to tell apart. **Settings** is a panel of its own with a drop-down that holds Tabs, Window and startup, AC / battery profile and Sensor sources, each still on its own panel inside it. **Appearance** (font, custom image, accent colour) is a drop-down on its panel too, closed until opened. The window has one fixed width.
- **Rainbow**, a new keyboard and cooling pad effect: every color of the rainbow at once, laid out across the keyboard (or along the pad's light strip) and scrolling along, with a Speed slider. With **Sync** on it is one of its effects. R-Helper draws it, so it only runs while R-Helper is open. Presets and "Restore lighting at startup" remember it.
- A **Presets** tab in Lighting Presets with ready-made lighting for the keyboard and the cooling pad together: Deep Sea, Aurora and Rainbow.
- **Speed sliders** for the lighting effects, on the keyboard and on the cooling pad (each keeps its own): Breathing, Fire, Tidal, Starlight, Spectrum, Color Dipp and Rainbow each get a Speed slider from 25% to 250% (100% is how the effect goes by default) and a Standard button. Breathing's four speed buttons became a slider (a saved choice is converted). With Sync on, the pad follows the keyboard's speed. Presets leave the speeds alone.
- Fixed: the Lighting tab could be wider than the window (the cooling pad's mode row, Sync's effect row and the preset rows now wrap).
- Ready-made lighting for the keyboard and the cooling pad together, in two tabs of Lighting Presets: **Presets** (Deep Sea, Aurora, Rainbow) and **Skin Sets** (one for each skin that comes with the program, see UI Options: Midnight and Copper; more tabs of the kind will follow). A skin's preset is put on by itself when you put the skin on, as long as the lighting toggles for the keyboard and the pad are on (there is a switch to turn this off). A preset sets the look of the lights and leaves your brightness alone.
- **Spectrum** on the **cooling pad**, and the keyboard's **Spectrum** now made to match Razer Chroma's own: every light shows the same color, which goes round the rainbow in about 38 seconds and rests a few seconds on a blue. R-Helper draws it, so it only runs while R-Helper is open. With **Sync** on, Spectrum is one of its effects. Presets and "Restore lighting at startup" remember it.
- **Color Dipp**, a new keyboard and cooling pad effect: pick two colors (**From** and **To**) and the light slides from the first to the second through every hue in between (red to violet passes orange, yellow, green and blue), pauses three seconds on each color, and slides back. The keyboard and the pad keep their own colors, and with **Sync** on Color Dipp is one of its effects. R-Helper draws it, so it only runs while R-Helper is open. Presets and "Restore lighting at startup" remember it.
- **Fire** keyboard effect, made to match Razer Chroma's own Fire: dim and cold along the top of the keyboard, hot and bright at the bottom, every key flickering. Pick your own **Hot** and **Cold** colors (or go back to Razer's orange and red). R-Helper draws it, so it only burns while R-Helper is open. Presets and "Restore lighting at startup" remember it.
- **Custom Keys: an effect for each key.** Pick an effect with the brush (Static, Breathing, Fire, Spectrum or Flash) and click keys to paint them with it, so part of the keyboard can stand still while the rest breathes or burns. Moving keys show a small mark in the editor, share the Speed choice, are saved with the layout and presets, and are drawn by R-Helper, so they only move while it is open.
- **Starlight** keyboard and cooling pad effect, made to match Razer Chroma's own Starlight: stars twinkle on random keys (or along the pad's light strip), each fading up and down at its own speed and brightness. Pick a color, or switch on a second one so that every star is followed by the same star in it. The keyboard and the pad keep their own colors, and with **Sync** on Starlight is one of its effects and both twinkle in the same colors. R-Helper draws it, so it only twinkles while R-Helper is open. Presets and "Restore lighting at startup" remember it.
- **Tidal** keyboard effect, made to match Razer Chroma's own Tidal: each key shows one color, rests, shows a second color and rests, and the waves run diagonally across the keyboard. Pick your two colors and whether the waves jet **Out** from a line or come **In** towards it. R-Helper draws it, so it only runs while R-Helper is open. Presets and "Restore lighting at startup" remember it.
- The keyboard's and the cooling pad's **Fire** colors are now separate. With **Sync** on, Fire is one of its effects and both burn in the same colors.
- **Tidal** on the **cooling pad** too: the waves run out from the middle of its light strip (or come in from the ends). The pad keeps its own colors and direction, and with **Sync** on Tidal is one of its effects and both run the same colors.
- **Fire** on the **cooling pad** too, along its light strip and made the same way as Razer Chroma's. The pad's light strip is now driven as the 18 lights it really has (it was treated as 15), so two-color Static and Breathing line up with the pad's ends.
- **Two colors** for lighting, on the keyboard and on the cooling pad (tick "Two colors" next to
  the color button, pick a second color):
  - Breathing alternates between the two colors.
  - Keyboard Static shows both colors as Left | Right, Top | Bottom or a Gradient.
  - Cooling pad Static shows both colors as two halves or a gradient. This one is experimental:
    the pad's LED layout is a guess, and it falls back to a single color if the pad rejects it.
- **Breathing layouts** for two colors: Left | Right, Top | Bottom and Gradient on the keyboard,
  Two halves and Gradient on the cooling pad (new "Layout" row under Breathing). These are
  software-animated by R-Helper (a background worker repaints the keyboard; the pad is sent frames
  from its own worker), so they run only while R-Helper is open. Pad version is experimental and
  falls back to the firmware's alternating Breathing if the pad rejects frames.
- **Lighting presets are now a drop-down with 20 slots per bank** (was 10). It lists only saved
  presets and shows "3 of 20 saved"; **Save as new** uses the first free slot, **Update** replaces
  a preset, **X** deletes it. Presets saved by earlier versions load unchanged.
- **Sync keyboard + cooling pad** switch on the Lighting tab: one set of controls (Static /
  Breathing / Off, colors, layout, speed, pad brightness) drives both devices, and the separate
  keyboard effect and cooling pad lighting controls are greyed out while it is on.
- **Cooling pad fan: Toggle App Control, named profiles, drop-downs.** The fan card is a drop-down
  with an opt-in switch (off by default: the fan is only touched, and the last setting only
  restored at startup, when it is on). Up to 10 named fan profiles with an exact RPM replace the
  three percentage presets (your old presets become profiles on first start).
- Razer Synapse Standby is a drop-down; the keyboard and cooling pad lighting parts are drop-downs
  too and fold away when Sync is switched on (logo mode, keyboard brightness and "always on" stay
  visible).
- **Sensors tab** (opt-in, off by default): GPU temperature / power / clock / load / memory
  junction through NVIDIA's NVML, CPU package power / clock / load and the hottest ACPI thermal zone
  through Windows performance counters, each with now / average / maximum and a reset button. The
  readings (and their thread) only exist while the **Read sensors** switch is on.
- **Sensor sources** (Options): the Sensors tab can read from HWiNFO's shared memory,
  LibreHardwareMonitor's web server, a CSV log file (ThrottleStop / Intel XTU / HWiNFO) and the
  PawnIO driver (direct MSR reads: package temperature, PL1 / PL2, package power), any number at
  once. For each of the seven readings you choose the source ("Where each reading comes from"),
  or leave it on Auto (PawnIO, HWiNFO, LibreHardwareMonitor, log file, built-in). New CPU rows:
  package temperature, PL1, PL2, and a status line per source under the table. Sources are only
  read while **Read sensors** is on. Older `sensor_source` settings carry over. When the chosen source gives
  nothing, the Sensors tab says why (not found, access denied, inactive, ...). With HWiNFO, a
  listing of everything it shares is written to `%APPDATA%\CPUnity-R-Helper\hwinfo_sensors.txt`.
- **Sensor graphs** on the Sensors tab: temperature, power (with dashed PL1 / PL2 lines), clock
  speed and load over 1 min / 5 min / 15 min / 30 min / 1 hour. Glowing lines over a gradient
  fill, a pulsing "now" dot, hover read-out with exact values, click a legend name to hide a line.
  The last hour is kept in memory while the sensors are on. The graphs sit in a **drop-down**
  (closing it stops all drawing) behind a **Live graphs** switch: off means nothing is recorded or
  drawn at all.
- **Sensor refresh interval**: a slider (0.25 s to 10 s, default 1 s) on the Sensors card with a
  **Default** button that returns to the factory 1 second. It applies immediately, also while the
  sensors are running, and is remembered.
- **Always run as administrator** (Options): R-Helper restarts itself elevated at launch (one
  Windows permission prompt; if you decline, it keeps running normally). Options also shows
  whether it is elevated now and has a **Restart as administrator now** button. The PawnIO source
  needs this.
- **Start at login without the permission prompt** (Options, shown under *Always run as
  administrator*): a drop-down with **Set up...** opens a window that lists exactly what will
  happen and the PowerShell commands, and does nothing until you click Continue. It creates one
  Task Scheduler task ("CPUnity R-Helper", at logon, highest privileges, allowed on battery) and
  switches off *Launch at Windows startup*. Needs R-Helper to be running as administrator once;
  **Remove** deletes the task again.
- **GPU memory junction temperature through PawnIO**: with `Nvidia.bin` (from the PawnIO.Modules
  releases) next to `IntelMSR.bin`, the PawnIO source also reads the NVIDIA GPU's memory
  temperature, so PawnIO alone can fill every reading. The module's argument layout and value
  format are inferred from its code, so what it returns is also written once to
  `%APPDATA%\CPUnity-R-Helper\pawnio_nvidia.txt` for checking against another tool.
- **Core temperatures**: two more CPU readings next to the package temperature, in the table,
  the graphs (new lines in the Temperature chart), the CSV logs and the per-reading source
  picks: **Core temperature (hottest)** and **IA cores temperature**. Core temperature comes from
  HWiNFO / LibreHardwareMonitor ("Core Max", or the hottest "Core N" reading) and from PawnIO,
  which reads each core's own thermal register (R-Helper's sampling thread visits every logical
  processor briefly to do it). IA cores temperature is HWiNFO's "CPU IA Cores" reading, so it
  needs HWiNFO (or a log file that has it).
- **Log file names**: a *File name* box in the Logging card (editable while logging is off) sets
  the name of the next log. The computer's date is put in front automatically, for example
  `2026-10-17_Cinebench run.csv`; an empty name gives `2026-10-17_sensors-043001.csv`, and a name
  that already exists gets `-2`, `-3`... so no log is ever overwritten. The box shows the exact
  file name that will be used, and the name is remembered.
- **Log viewer** (Sensors tab, a drop-down inside the Logging card under the logging switch): pick one of the CSV logs R-Helper wrote
  (or browse to one) and see the whole file as the same graphs plus a lowest / average / highest
  table per reading, with the start, end and length of the log. Nothing animates, so it costs
  nothing while you look.
- **Changelog tab**: every change since 0.5.1, per version in drop-downs (the newest open).
- **The logo in the header follows the accent color** (and so does the big one on the Info tab):
  the built-in logo's red glow takes the accent's hue, grey metal stays grey. A `logo.png` you put
  next to the program is shown as it is.
- **Start minimized to tray**: **Options > Start minimized to tray** starts CR-Helper hidden next
  to the clock instead of opening its window; click the tray icon to open it. Good with **Launch
  at Windows startup**. The tray icon stays while CR-Helper runs.
- **Choose your tabs**: **Options > Tabs** (a drop-down) switches Lighting, Sensors, Files, Changelog and Info
  on or off, for a plain Main-only window like the original R-Helper. All of them are shown to
  begin with, and your choice is remembered. Main is always there; with the others off the tab row
  disappears.
- **Info tab**: the full-size CPUnity logo across the top (it takes the accent color too, and the text on this page is larger), then a personal welcome from the author ("The Continuation of Rebellion:
  CR-Helper"), a short where-to-start guide to the tabs, **The Rebellion**, **About CPUnity**, **CPUnity: Future
  Endeavors**, **Ways to contact** (an email alias that copies to the clipboard when clicked, and a
  world button that opens the program's GitHub page) and a closed **P.S** drop-down. The text is built into the program and cannot be
  changed from inside it.
- **Files tab**: big tiles that open each place R-Helper stores data in File Explorer: the
  settings folder, `config.txt` (selected), the sensor logs folder, `log.txt`, the Synapse standby
  script and the program folder. Below the tiles, drop-downs for the other programs R-Helper
  reads from (ThrottleStop / Intel XTU / HWiNFO log file, PawnIO's IntelMSR.bin and Nvidia.bin plus
  the driver, LibreHardwareMonitor's address, HWiNFO's shared memory): each has the "use this
  source" switch, a box to type the location and a folder button that opens a Windows file
  picker, with a found / not found status.
- **Get Modules Here** (Files tab, inside the PawnIO drop-down): 🌐 buttons that open the PawnIO
  driver page (pawnio.eu) and the PawnIO.Modules releases page in your browser, each with where it
  leads and what to download (`IntelMSR.bin` for the CPU, `Nvidia.bin` for GPU memory temperature).
- **Sensor settings are remembered**: whether *Read sensors* was on (it starts again with
  R-Helper), the sources and per-reading picks, refresh interval, graphs on / off, drop-down open /
  closed, graph time span and which graph lines you hid. **Logging is the exception: it is never
  remembered and is always off after a start.** (Averages and maximums start fresh each time.)
- **Sensor logging**: a switch writes every reading (once per refresh interval) to a new CSV file in
  `%APPDATA%\CPUnity-R-Helper\logs`, with an "Open folder" button.
- **Close button minimizes to the tray** (Options, off by default): the X hides the window to a
  notification-area icon (click to show, right-click for Show / Quit) instead of quitting.
- **Drop-downs remember whether they were open or closed** between launches (stored in
  `config.txt`).
- **Toggle switches**: every on / off option is a side-flip switch, dark grey when off and the
  accent color from Options when on (replaces the checkboxes).
- **Breathing speed** (Standard / Slow / Fast / Faster) for the keyboard and the cooling pad.
  Standard keeps the firmware's own Breathing; the other speeds are drawn by R-Helper (single
  color, Alternate, halves and gradient), so they only run while it is open.
- **Custom Keys** keyboard effect: choose which individual keys are lit and in what color, from
  an on-screen keyboard (click to toggle, right-click to turn off), with quick sets (WASD, arrows,
  WASD + arrows, FPS, numbers, F-keys, HJKL, modifiers), a background color for all other keys, and a key map
  fine-tune shift. Custom layouts are stored in lighting presets.
- Lighting presets store the second color and layout too, and the preset list shows both swatches.
  Presets saved with one color keep working.

### Fixed
- **Profile switching is sturdier.** (Chasing an odd start, Custom with the CPU on Overclock and the GPU on Low after using Battery mode, turned out to be the saved AC profile doing what it was set to, but these four real problems were found on the way.)
  1. When a saved profile switched the laptop to Custom (the AC "CPU Overclock" option, or Custom chosen for AC or Battery), the CPU and GPU boosts were written the instant the mode was sent, before the laptop had committed it, and the laptop refuses boost commands until then. The CPU Overclock often failed, and the GPU was never set at all, so it stayed on whatever the laptop fell back to (Low). Now the program waits until the laptop reports Custom, applies the CPU and GPU boosts you last picked (each retried up to three times), reads them back, and logs what it did.
  2. Your last Custom CPU and GPU boosts are now remembered (in the settings file, `custom_cpu` and `custom_gpu`), so a profile that uses Custom comes back as you left it.
  3. If reading the laptop right after a profile switch failed (normal for a moment: its two zones disagree while the mode settles), the program used to write a default profile over your real settings (logo off, keyboard brightness 50, lights-always-on off, battery care on). It no longer writes anything; it only refreshes the panel.
  4. Windows reports "charger state unknown" for a moment after logon or wake, and that used to count as "on battery", so a start could apply the Battery profile and flip to the AC profile a second later. Unknown is no longer a state: the program asks again for up to two seconds, and a change of charger is accepted only when two readings in a row agree.
- **AC / Battery switching** now follows one rule everywhere: a profile left on "Default" leaves the laptop alone (it already did at launch, but plugging in or pulling out the charger forced Performance or Battery mode regardless). When either side has a profile of yours, the switch happens (so leaving an AC Overclock still lets go of it on battery).
- **Battery mode with the window minimized or in the tray:** pulling out or plugging in the charger used to be noticed only once the window was open again. It is watched in the background now (about every 3 seconds), and the switch is logged to `%APPDATA%\CPUnity-R-Helper\log.txt`.
- The key map in Custom Keys no longer grows past its card when keys have effects: the keys are drawn at exactly their size instead of growing to fit their marks. That growth used to stretch the card, and every card below it, past the edge of the window (cutting off text, tabs and the key map's right side).

### Changed
- **Nothing reads sensors that nobody uses.** Switching **Read sensors** off now also switches logging off (before, the logging switch stayed on and a new log began by itself when the readings were switched on again). Switching the Sensors tab off (Options > Settings > Tabs) stops everything about it: the readings, their sources and logging (also one waiting for its program). With the tab back on, the readings start again if Read sensors was on; logging stays off until you switch it on. Lighting is not tied to its tab: effects keep running with the Lighting tab off.
- **Nothing animates, repaints or plays while the window is minimized or in the tray.** The window draws nothing then (animations stop), and the program wakes about once a second just to watch the laptop and the charger. Everything picks up where it was when the window comes back.
- **Performance Mode, Fan Control and Battery are drop-downs too**, like the other cards of the main tab, and remember whether they were open. Closed, each says what matters in its title: the mode and power source, the fan mode and speed, the charge limit. The AC Power / Battery indicator and the hidden-modes eye of Performance Mode moved inside its drop-down.
- **The AC / battery profile moved to the main tab**, under Razer Synapse Standby (it was in Options). The changelog text is now the same size as the Info text.
- **The "Overclock on AC power" switch is gone**: choosing Custom for AC and Overclock in its CPU drop-down does the same, so the switch was only a second way to say it. A setting saved with the switch on becomes Custom with the CPU on Overclock when the program starts.
- **The program file is now called `CR-Helper.exe`** (it was `cpunity-rhelper.exe`). The window
  title, logo and settings folder are unchanged, so your settings carry over. If you use "Launch at
  Windows startup" or the login task, switch them off and on once so they point at the new file.
- **Existing users:** the cooling pad fan no longer restores at startup until "Toggle App Control"
  on the Cooling Pad card is ticked once.
- Keyboard animation frames use a faster HID exchange (no pre-delay, shorter wait, one attempt;
  it falls back to the patient exchange if the keyboard refuses) and no longer pause between
  frames, for smoother split Breathing.
- Custom Keys map corrected from hardware tests: Left Shift covers two cells (Z starts one cell
  later), the arrow keys sit in the bottom row, and the bottom row's Alt / Space / Fn cells and
  the right Shift were moved.
- Bundled artwork refreshed (the program file grows by about 1.7 MB).

### Coming soon
Not in this version, but on the way: mini games hidden in the program, a shop with its own currency for new skins and backdrops, an achievement system, and so much more.

## 0.5.1 - 2026-10-17

### Added
- **Lighting tab**: all lighting controls moved off the main page onto their own tab (Main /
  Lighting). The Cooling Pad fan card stays on Main.
- **Lighting presets**: 10 named slots for the keyboard, 10 for the cooling pad and 10 for both
  together (a "both" preset can hold either part or both). Save / Apply / Clear per slot, stored in
  `config.txt`.
- **Restore lighting at startup (opt-in, off by default)**: puts the last applied keyboard effect and
  pad lighting back at launch and when the pad is plugged in again. Requires "Toggle App Lighting
  Control" for that device. Without it nothing is applied at launch, as before.
- **Keyboard effects (opt-in)** in the Lighting card: Static, Breathing, Spectrum, Wave (left /
  right) and Off with a color picker, behind a **Toggle App Lighting Control** checkbox that is off
  by default. Effects are only sent when you click one (or restored at startup if you opt in).
  Brightness keeps using the existing Keyboard Brightness slider. Uses the Razer extended matrix
  effect command (0x0F02) aimed at the keyboard backlight LED with transaction id 0x1F; Options >
  Advanced > Keyboard effect tuning can switch to the older standard command (0x030A), a random
  transaction id, or an "Off" before Static / Breathing. Static is painted as a custom frame
  (every key set to the color), the way razer-laptop-control does it, because the firmware's own
  Static effect ignores the color on some laptops (switchable in the same place). Not yet verified on every Blade model.
- **Cooling Pad** card for the Razer Laptop Cooling Pad (USB 1532:0F43), shown only while a pad is
  connected. Fan: Manual / Off, a speed dropdown in 5% steps, three saveable speed presets and an
  exact RPM slider. Lighting (Static / Breathing / Off, color, brightness) is opt-in via "Toggle App
  Lighting Control", off by default, and is not applied at launch unless you opt in to restoring it, so Razer
  Chroma / Synapse keep control of the pad's lights unless you ask otherwise. Runs on its own
  background thread, so the window never waits on the pad, and re-applies the last fan setting when
  the pad is plugged back in or after sleep. Based on the cooling pad work in
  [Robak08/r-helper](https://github.com/Robak08/r-helper) (MIT). Fan "Auto" (temperature-driven) is
  not included yet.

## 0.5.0 - first CPUnity:R-Helper release

Fork of [R-Helper](https://github.com/Fatalution/r-helper) by Fatalution (MIT), which is built on
razer-ctl's `librazer` (MIT). Developed and tested on a Razer Blade 16 (2023, RZ09-0483).

### Added
- **Razer Synapse Standby** card: Standby / Resume Synapse buttons, "Keep Chroma lighting running"
  and "Auto-park at every boot / logon". Built in, so a separate
  Synapse killer tool (such as NxRoot's SynapseKiller, which inspired it) is no longer needed.
- **Max Fan** button: one click = Manual fan mode with the slider at maximum.
- **Battery charge limit** selector, 50% to 80% in 5% steps.
- **AC / battery profile**: an explicit mode chosen in Options is applied once when the app starts
  and on every power-source change. "Default" keeps the old behaviour on plug / unplug
  (Performance on AC, Battery on battery) and leaves the current mode alone at launch.
  Optional "Overclock on AC power" (Custom mode + CPU Overclock).
- **Options window**: accent color (presets, RGB boxes, hex, visual picker, 10 saved colors),
  UI font, keep on top, launch at Windows startup, AC / battery profile, Custom Image help and an
  Advanced (developer) section.
- **Custom images**: optional `logo.png` (left of the title) and `banner.png` (fills the header
  space right of the title); can be baked into the exe from `assets/`. Options shows the exact
  banner size for your screen.
- **Look**: dark card layout, accent-colored title, resizable window that auto-fits its content.
- **App icon**: new multi-size CPUnity icon (16 to 256 px).
- Device model shown in the footer.
- **Shoutouts** section in Options (and in the README) crediting the projects and people behind it.
- Diagnostic log at `%APPDATA%\CPUnity-R-Helper\log.txt`.

### Fixed
- Window freezing while Razer Chroma / Synapse were running (they share the laptop's HID channel,
  and failed reads were retried after 500 ms on the UI thread). Reads now retry after 15 ms and
  polling backs off automatically when a poll is slow. Writes keep upstream's 500 ms back-off.
- CPU / GPU boost rows showing stale values right after launch.
- Highlighted performance mode not following the device after an AC / battery auto-switch.
- Device model showing "Connected Device" instead of the model name.

### Changed
- The Debug toggle moved from the footer to Options > Advanced (developer). It is not saved and
  resets on restart.
- Polling reduced from about 7 device reads every 0.5 s to about 3 every 1 s.

### Known behavior
- **Battery charge limit**: all steps (50 to 80%) were tested on a Razer Blade 16 (2023) and hold.
  After the limit is reached, the firmware resumes charging only once the battery is about
  10 points below the limit. Raising the limit by a small step (for example 55% to 60%, or 75% to
  80%) therefore may not restart charging; set a limit at least 10 points above the current
  charge (80% works) and then lower it to the value you want. This is firmware behavior, not a bug
  in the app.
