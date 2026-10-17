# Changelog

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
