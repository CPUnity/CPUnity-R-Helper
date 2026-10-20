//! Per-key keyboard lighting: where each key of the Razer Blade 16 sits in the 6 x 15 custom-frame
//! matrix, and the "custom keys" layout (which keys are lit, in which color).
//!
//! The matrix positions are read off the Blade 16 (2023) layout and the Left | Right tests done
//! on real hardware (column 0 holds Tab / Caps / Shift / Ctrl, U and J sit in column 7; the bottom
//! two rows were corrected from test reports). Wide keys cover several cells. The right edge of
//! rows 4 and 5 is the least certain, so the app offers a column shift to line the map up (see
//! `KeyLayout::grid`). Each key can also move (breathing, fire and so on), see `KeyFx`, and some
//! effects take a second color for the key.
use crate::fire::Fire;

/// Rows of the custom frame (top to bottom).
pub const ROWS: usize = 6;
/// Columns of the custom frame (left to right).
pub const COLS: usize = 15;

/// The colors of every cell of the custom frame, `grid[row][col]`.
pub type KeyGrid = [[[u8; 3]; COLS]; ROWS];

/// One key: its id (stable, used in saved files), the short label shown in the editor, and the
/// cells it covers: `row`, then `span` cells starting at `col`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyDef {
    pub id: &'static str,
    pub label: &'static str,
    pub row: u8,
    pub col: u8,
    pub span: u8,
}

const fn k(id: &'static str, label: &'static str, row: u8, col: u8, span: u8) -> KeyDef {
    KeyDef { id, label, row, col, span }
}

/// Every key, ordered by row and then by column (the editor draws them in this order).
pub const KEYS: &[KeyDef] = &[
    // Row 0
    k("ESC", "Esc", 0, 0, 1),
    k("F1", "F1", 0, 1, 1),
    k("F2", "F2", 0, 2, 1),
    k("F3", "F3", 0, 3, 1),
    k("F4", "F4", 0, 4, 1),
    k("F5", "F5", 0, 5, 1),
    k("F6", "F6", 0, 6, 1),
    k("F7", "F7", 0, 7, 1),
    k("F8", "F8", 0, 8, 1),
    k("F9", "F9", 0, 9, 1),
    k("F10", "F10", 0, 10, 1),
    k("F11", "F11", 0, 11, 1),
    k("F12", "F12", 0, 12, 1),
    k("DEL", "Del", 0, 13, 1),
    k("PWR", "Pwr", 0, 14, 1),
    // Row 1
    k("GRAVE", "`", 1, 0, 1),
    k("1", "1", 1, 1, 1),
    k("2", "2", 1, 2, 1),
    k("3", "3", 1, 3, 1),
    k("4", "4", 1, 4, 1),
    k("5", "5", 1, 5, 1),
    k("6", "6", 1, 6, 1),
    k("7", "7", 1, 7, 1),
    k("8", "8", 1, 8, 1),
    k("9", "9", 1, 9, 1),
    k("0", "0", 1, 10, 1),
    k("MINUS", "-", 1, 11, 1),
    k("EQUALS", "=", 1, 12, 1),
    k("BKSP", "Bksp", 1, 13, 2),
    // Row 2
    k("TAB", "Tab", 2, 0, 1),
    k("Q", "Q", 2, 1, 1),
    k("W", "W", 2, 2, 1),
    k("E", "E", 2, 3, 1),
    k("R", "R", 2, 4, 1),
    k("T", "T", 2, 5, 1),
    k("Y", "Y", 2, 6, 1),
    k("U", "U", 2, 7, 1),
    k("I", "I", 2, 8, 1),
    k("O", "O", 2, 9, 1),
    k("P", "P", 2, 10, 1),
    k("LBRACKET", "[", 2, 11, 1),
    k("RBRACKET", "]", 2, 12, 1),
    k("BSLASH", "\\", 2, 13, 2),
    // Row 3
    k("CAPS", "Caps", 3, 0, 1),
    k("A", "A", 3, 1, 1),
    k("S", "S", 3, 2, 1),
    k("D", "D", 3, 3, 1),
    k("F", "F", 3, 4, 1),
    k("G", "G", 3, 5, 1),
    k("H", "H", 3, 6, 1),
    k("J", "J", 3, 7, 1),
    k("K", "K", 3, 8, 1),
    k("L", "L", 3, 9, 1),
    k("SEMI", ";", 3, 10, 1),
    k("QUOTE", "'", 3, 11, 1),
    k("ENTER", "Enter", 3, 12, 3),
    // Row 4 (hardware-tested: Left Shift covers two cells, Z starts at column 2; the right Shift
    // is not at columns 11 to 13, so it is assumed to be the last cell; the arrow keys are in row 5)
    k("LSHIFT", "Shift", 4, 0, 2),
    k("Z", "Z", 4, 2, 1),
    k("X", "X", 4, 3, 1),
    k("C", "C", 4, 4, 1),
    k("V", "V", 4, 5, 1),
    k("B", "B", 4, 6, 1),
    k("N", "N", 4, 7, 1),
    k("M", "M", 4, 8, 1),
    k("COMMA", ",", 4, 9, 1),
    k("PERIOD", ".", 4, 10, 1),
    k("SLASH", "/", 4, 11, 1),
    k("RSHIFT", "Shift", 4, 14, 1),
    // Row 5 (hardware-tested: cell 3 is unused, the left Alt is cell 4 and the right Alt cell 8,
    // the right Fn is cell 9, Up, Right and Down are cells 12, 13 and 14; Left is cell 11)
    k("LCTRL", "Ctrl", 5, 0, 1),
    k("LFN", "Fn", 5, 1, 1),
    k("WIN", "Win", 5, 2, 1),
    k("LALT", "Alt", 5, 4, 1),
    k("SPACE", "Space", 5, 5, 3),
    k("RALT", "Alt", 5, 8, 1),
    k("RFN", "Fn", 5, 9, 1),
    k("RCTRL", "Ctrl", 5, 10, 1),
    k("LEFT", "<", 5, 11, 1),
    k("UP", "Up", 5, 12, 1),
    k("RIGHT", ">", 5, 13, 1),
    k("DOWN", "Dn", 5, 14, 1),
];

/// Ready-made key sets for the editor: a name and the key ids it lights.
pub const QUICK_SETS: &[(&str, &[&str])] = &[
    ("WASD", &["W", "A", "S", "D"]),
    ("Arrows", &["UP", "DOWN", "LEFT", "RIGHT"]),
    ("WASD + Arrows", &["W", "A", "S", "D", "UP", "DOWN", "LEFT", "RIGHT"]),
    (
        "FPS",
        &["Q", "W", "E", "R", "A", "S", "D", "F", "LSHIFT", "LCTRL", "SPACE", "1", "2", "3", "4"],
    ),
    ("Numbers", &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]),
    ("F-keys", &["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12"]),
    ("HJKL", &["H", "J", "K", "L"]),
    ("Modifiers", &["LSHIFT", "RSHIFT", "LCTRL", "RCTRL", "LALT", "RALT", "WIN", "LFN", "RFN"]),
];

/// The key with this id, if there is one.
pub fn key_by_id(id: &str) -> Option<&'static KeyDef> {
    KEYS.iter().find(|key| key.id == id)
}

fn encode_hex(c: [u8; 3]) -> String {
    format!("{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn decode_hex(text: &str) -> Option<[u8; 3]> {
    if text.len() != 6 || !text.is_ascii() {
        return None;
    }
    let part = |i: usize| u8::from_str_radix(&text[i..i + 2], 16).ok();
    Some([part(0)?, part(2)?, part(4)?])
}

/// What a single key does in Custom Keys: stand still in its color, or move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyFx {
    /// The key's color, steady.
    #[default]
    Static,
    /// The color fades up and down.
    Breathing,
    /// The color flickers like a flame (the key burns like its place in Chroma's Fire).
    Fire,
    /// Every color of the rainbow in turn (the key's own color is not used).
    Spectrum,
    /// The color blinks on and off.
    Flash,
    /// The key twinkles now and then like a star (with a second color, a second star follows each).
    Starlight,
    /// The key goes from its color to the second one through every hue between, and back.
    Dipp,
    /// A hand like a clock's sweeps round the middle of the keyboard, and the key lights as it
    /// passes and fades behind it.
    Wheel,
}

impl KeyFx {
    /// (New ones go on the end: the editor's brush is saved as a place in this list.)
    pub const ALL: [KeyFx; 8] = [
        KeyFx::Static,
        KeyFx::Breathing,
        KeyFx::Fire,
        KeyFx::Spectrum,
        KeyFx::Flash,
        KeyFx::Starlight,
        KeyFx::Dipp,
        KeyFx::Wheel,
    ];

    pub fn name(self) -> &'static str {
        match self {
            KeyFx::Static => "Static",
            KeyFx::Breathing => "Breathing",
            KeyFx::Fire => "Fire",
            KeyFx::Spectrum => "Spectrum",
            KeyFx::Flash => "Flash",
            KeyFx::Starlight => "Starlight",
            KeyFx::Dipp => "Color Dipp",
            KeyFx::Wheel => "Wheel",
        }
    }

    /// Whether the key can have a second color with this effect (Breathing: the two take turns;
    /// Fire: cold and hot; Starlight: the star that follows; Color Dipp: where it goes to; Wheel:
    /// the end of the tail).
    pub fn can_two(self) -> bool {
        matches!(self, KeyFx::Breathing | KeyFx::Fire | KeyFx::Starlight | KeyFx::Dipp | KeyFx::Wheel)
    }

    /// Whether the effect is nothing without a second color.
    pub fn needs_two(self) -> bool {
        self == KeyFx::Dipp
    }

    /// One letter that marks the effect on a key in the editor (and in the saved text).
    pub fn code(self) -> char {
        match self {
            KeyFx::Static => 's',
            KeyFx::Breathing => 'b',
            KeyFx::Fire => 'f',
            KeyFx::Spectrum => 'r',
            KeyFx::Flash => 'x',
            KeyFx::Starlight => 'l',
            KeyFx::Dipp => 'd',
            KeyFx::Wheel => 'w',
        }
    }

    pub fn from_code(code: &str) -> Option<KeyFx> {
        Self::ALL.into_iter().find(|fx| code.len() == 1 && code.starts_with(fx.code()))
    }

    /// Where the effect is in the saved settings the app keeps as a number (the editor's brush).
    pub fn index(self) -> u8 {
        Self::ALL.iter().position(|fx| *fx == self).unwrap_or(0) as u8
    }

    pub fn from_index(index: u8) -> KeyFx {
        Self::ALL.get(index as usize).copied().unwrap_or_default()
    }
}

/// Which keys are lit and in which color; every other key gets `background` (black = off).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLayout {
    pub background: [u8; 3],
    /// `(key id, color)`, each id at most once.
    pub keys: Vec<(String, [u8; 3])>,
    /// The keys that move, `(key id, effect)`. A lit key that is not listed is `Static`.
    pub fx: Vec<(String, KeyFx)>,
    /// The second color of the keys that have one, `(key id, color)` (see `KeyFx::can_two`).
    pub second: Vec<(String, [u8; 3])>,
}

impl Default for KeyLayout {
    fn default() -> Self {
        Self { background: [0, 0, 0], keys: Vec::new(), fx: Vec::new(), second: Vec::new() }
    }
}

impl KeyLayout {
    pub fn color_of(&self, id: &str) -> Option<[u8; 3]> {
        self.keys.iter().find(|(k, _)| k == id).map(|(_, c)| *c)
    }

    /// Lights `id` in `color` (replaces its color if it is already lit). Unknown ids are ignored.
    pub fn set(&mut self, id: &str, color: [u8; 3]) {
        if key_by_id(id).is_none() {
            return;
        }
        match self.keys.iter_mut().find(|(k, _)| k == id) {
            Some(entry) => entry.1 = color,
            None => self.keys.push((id.to_string(), color)),
        }
    }

    pub fn set_many(&mut self, ids: &[&str], color: [u8; 3]) {
        for id in ids {
            self.set(id, color);
        }
    }

    pub fn clear_key(&mut self, id: &str) {
        self.keys.retain(|(k, _)| k != id);
        self.fx.retain(|(k, _)| k != id);
        self.second.retain(|(k, _)| k != id);
    }

    /// Turns every key off (the background color stays).
    pub fn clear_keys(&mut self) {
        self.keys.clear();
        self.fx.clear();
        self.second.clear();
    }

    /// What `id` does (`Static` for a key that stands still or is not lit).
    pub fn fx_of(&self, id: &str) -> KeyFx {
        self.fx.iter().find(|(k, _)| k == id).map_or(KeyFx::Static, |(_, fx)| *fx)
    }

    /// The key's second color, if it has one.
    pub fn second_of(&self, id: &str) -> Option<[u8; 3]> {
        self.second.iter().find(|(k, _)| k == id).map(|(_, c)| *c)
    }

    /// Lights `id` in `color` with the effect `fx`.
    pub fn set_with(&mut self, id: &str, color: [u8; 3], fx: KeyFx) {
        self.set_with_second(id, color, None, fx);
    }

    /// Lights `id` in `color` with the effect `fx` and, for an effect that can use one, a second color.
    pub fn set_with_second(&mut self, id: &str, color: [u8; 3], second: Option<[u8; 3]>, fx: KeyFx) {
        self.set(id, color);
        if self.color_of(id).is_none() {
            return; // an unknown key
        }
        self.fx.retain(|(k, _)| k != id);
        self.second.retain(|(k, _)| k != id);
        if fx != KeyFx::Static {
            self.fx.push((id.to_string(), fx));
            if let (true, Some(second)) = (fx.can_two(), second) {
                self.second.push((id.to_string(), second));
            }
        }
    }

    /// Lights every key in `ids` the same way.
    pub fn set_many_with(&mut self, ids: &[&str], color: [u8; 3], fx: KeyFx) {
        self.set_many_with_second(ids, color, None, fx);
    }

    /// Lights every key in `ids` the same way, with a second color.
    pub fn set_many_with_second(&mut self, ids: &[&str], color: [u8; 3], second: Option<[u8; 3]>, fx: KeyFx) {
        for id in ids {
            self.set_with_second(id, color, second, fx);
        }
    }

    /// True when at least one lit key moves, so the layout needs the animation worker.
    pub fn is_animated(&self) -> bool {
        self.fx.iter().any(|(id, fx)| *fx != KeyFx::Static && self.color_of(id).is_some())
    }

    /// Saved form: `rrggbb` for the background, then `;ID:rrggbb` for every lit key, with
    /// `:` and the effect's letter after the color for a key that moves (`;W:ff0000:b`), and `:`
    /// and the second color after that for a key that has one (`;W:ff0000:b:0000ff`).
    pub fn encode(&self) -> String {
        let mut text = encode_hex(self.background);
        for (id, color) in &self.keys {
            text.push(';');
            text.push_str(id);
            text.push(':');
            text.push_str(&encode_hex(*color));
            let fx = self.fx_of(id);
            if fx != KeyFx::Static {
                text.push(':');
                text.push(fx.code());
                if let Some(second) = self.second_of(id) {
                    text.push(':');
                    text.push_str(&encode_hex(second));
                }
            }
        }
        text
    }

    /// Reads the saved form. Unknown keys and malformed entries are skipped; a bad background
    /// color makes the whole text invalid.
    pub fn decode(text: &str) -> Option<KeyLayout> {
        let mut parts = text.split(';');
        let background = decode_hex(parts.next()?.trim())?;
        let mut layout = KeyLayout { background, ..KeyLayout::default() };
        for part in parts {
            if let Some((id, rest)) = part.trim().split_once(':') {
                let mut bits = rest.split(':');
                let hex = bits.next().unwrap_or("");
                let code = bits.next().unwrap_or("");
                let second = bits.next().and_then(|h| decode_hex(h.trim()));
                if let Some(color) = decode_hex(hex.trim()) {
                    let fx = KeyFx::from_code(code.trim()).unwrap_or_default();
                    layout.set_with_second(id.trim(), color, second, fx);
                }
            }
        }
        Some(layout)
    }

    /// The custom frame for this layout. `shift` moves every key by that many columns (the editor's
    /// "fine-tune" setting, to line the map up with the real keys); cells pushed off the matrix
    /// are dropped.
    pub fn grid(&self, shift: i8) -> KeyGrid {
        let mut grid = [[self.background; COLS]; ROWS];
        for (id, color) in &self.keys {
            let Some(key) = key_by_id(id) else {
                continue;
            };
            for offset in 0..key.span as i32 {
                let col = key.col as i32 + offset + shift as i32;
                if (0..COLS as i32).contains(&col) {
                    grid[key.row as usize][col as usize] = *color;
                }
            }
        }
        grid
    }

    /// The picture `t` seconds after the animation began, for a layout with keys that move.
    /// `speed` is a percent of the standard speed, see `command::STANDARD_SPEED` (it sets how fast every
    /// effect but Fire goes), and `flame` is the running fire that the burning keys take their flicker from.
    pub fn grid_at(&self, shift: i8, t: f32, speed: u8, flame: &Fire) -> KeyGrid {
        let period = crate::command::breath_period(speed);
        let pace = crate::command::speed_factor(speed);
        let mut grid = [[self.background; COLS]; ROWS];
        for (id, color) in &self.keys {
            let Some(key) = key_by_id(id) else {
                continue;
            };
            let fx = self.fx_of(id);
            let second = self.second_of(id);
            for offset in 0..key.span as i32 {
                let col = key.col as i32 + offset + shift as i32;
                if !(0..COLS as i32).contains(&col) {
                    continue;
                }
                let (row, col) = (key.row as usize, col as usize);
                grid[row][col] = match fx {
                    KeyFx::Static => *color,
                    KeyFx::Breathing => {
                        // two colors take turns: one breath each
                        let shown = match second {
                            Some(b) if crate::command::breath_cycle(t, period) % 2 == 1 => b,
                            _ => *color,
                        };
                        scale(shown, crate::command::breath_level_with(t, period))
                    }
                    KeyFx::Fire => match second {
                        // cold when the flame is low, hot when it is high
                        Some(cold) => mix(cold, *color, flame.key_heat(row, col).unwrap_or(1.0)),
                        None => scale(*color, flame.key_level(row, col).unwrap_or(1.0)),
                    },
                    KeyFx::Starlight => crate::starlight::lone_star(row * COLS + col, t * pace, *color, second),
                    KeyFx::Dipp => crate::dipp::color_at(*color, second.unwrap_or(crate::dipp::DEFAULT_B), t * pace),
                    KeyFx::Wheel => wheel_color(row, col, t * pace, *color, second),
                    // a whole turn of the rainbow takes two breaths; neighbours are a little apart
                    KeyFx::Spectrum => hue((t / (period * 2.0) + col as f32 * 0.03).rem_euclid(1.0)),
                    KeyFx::Flash => {
                        if (t / (period / 2.0)).rem_euclid(1.0) < 0.5 {
                            *color
                        } else {
                            [0, 0, 0]
                        }
                    }
                };
            }
        }
        grid
    }
}

/// Seconds one turn of the wheel takes at the standard speed.
pub const WHEEL_TURN_SECS: f32 = 4.0;
/// How far behind the hand the light trails, in radians (about 110 degrees).
const WHEEL_TAIL: f32 = 1.9;

/// How far round the middle of the keyboard the cell at `row`, `col` is, in radians clockwise from
/// straight up (0 to a whole turn).
fn wheel_angle(row: usize, col: usize) -> f32 {
    let dx = col as f32 + 0.5 - COLS as f32 / 2.0;
    let dy = row as f32 + 0.5 - ROWS as f32 / 2.0;
    dx.atan2(-dy).rem_euclid(std::f32::consts::TAU)
}

/// The cell's color when the wheel's hand is `secs` seconds into its turns: the hand sweeps clockwise
/// like a clock's, a cell lights as it passes (the first color) and fades behind it, turning into the
/// second color, if there is one, along the tail. A cell the hand has not reached yet is dark.
pub fn wheel_color(row: usize, col: usize, secs: f32, a: [u8; 3], b: Option<[u8; 3]>) -> [u8; 3] {
    wheel_color_dir(row, col, secs, a, b, false)
}

/// `wheel_color`, with the hand going the other way round (counter-clockwise) when `ccw`.
pub fn wheel_color_dir(row: usize, col: usize, secs: f32, a: [u8; 3], b: Option<[u8; 3]>, ccw: bool) -> [u8; 3] {
    let tau = std::f32::consts::TAU;
    let hand = (secs / WHEEL_TURN_SECS).rem_euclid(1.0) * tau;
    // (the other way round is the same picture in a mirror)
    let angle = if ccw { (tau - wheel_angle(row, col)).rem_euclid(tau) } else { wheel_angle(row, col) };
    let behind = (hand - angle).rem_euclid(tau);
    if behind >= WHEEL_TAIL {
        return [0, 0, 0];
    }
    let along = behind / WHEEL_TAIL;
    let shown = b.map_or(a, |b| mix(a, b, along));
    scale(shown, (1.0 - along).powf(1.6))
}

/// `a` going over to `b`: `u` 0.0 is `a`, 1.0 is `b`.
pub fn mix(a: [u8; 3], b: [u8; 3], u: f32) -> [u8; 3] {
    let u = u.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * u).round() as u8)
}

fn scale(color: [u8; 3], level: f32) -> [u8; 3] {
    let level = level.clamp(0.0, 1.0);
    color.map(|c| (f32::from(c) * level).round() as u8)
}

/// The fully bright color at `h` (0..1) around the color wheel.
fn hue(h: f32) -> [u8; 3] {
    let x = h.rem_euclid(1.0) * 6.0;
    let f = x.fract();
    let (r, g, b) = match x as u32 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_inside_the_matrix_and_in_order() {
        let mut taken = [[false; COLS]; ROWS];
        let mut last = (0u8, 0u8);
        for key in KEYS {
            assert!(KEYS.iter().filter(|other| other.id == key.id).count() == 1, "{}", key.id);
            assert!(!key.id.contains(|c: char| matches!(c, ';' | ':' | ',' | '|' | '=')));
            assert!((key.row as usize) < ROWS);
            assert!(key.span >= 1 && key.col as usize + key.span as usize <= COLS, "{}", key.id);
            assert!((key.row, key.col) > last || last == (0, 0), "order at {}", key.id);
            last = (key.row, key.col);
            for offset in 0..key.span {
                let cell = &mut taken[key.row as usize][(key.col + offset) as usize];
                assert!(!*cell, "{} overlaps another key", key.id);
                *cell = true;
            }
        }
    }

    #[test]
    fn quick_sets_only_use_known_keys() {
        for (name, ids) in QUICK_SETS {
            assert!(!ids.is_empty(), "{}", name);
            for id in ids.iter() {
                assert!(key_by_id(id).is_some(), "{} uses unknown key {}", name, id);
            }
        }
    }

    #[test]
    fn set_replace_clear_and_unknown() {
        let mut layout = KeyLayout::default();
        layout.set("W", [255, 0, 0]);
        layout.set("W", [0, 255, 0]);
        layout.set("NOPE", [1, 2, 3]);
        assert_eq!(layout.keys.len(), 1);
        assert_eq!(layout.color_of("W"), Some([0, 255, 0]));
        layout.clear_key("W");
        assert_eq!(layout.color_of("W"), None);
        layout.set_many(&["W", "A", "S", "D"], [9, 9, 9]);
        assert_eq!(layout.keys.len(), 4);
        layout.clear_keys();
        assert!(layout.keys.is_empty());
    }

    #[test]
    fn encode_decode_round_trip() {
        let mut layout = KeyLayout { background: [1, 2, 3], ..KeyLayout::default() };
        layout.set("W", [255, 0, 0]);
        layout.set("SPACE", [0, 128, 255]);
        layout.set("1", [10, 20, 30]);
        let text = layout.encode();
        assert_eq!(text, "010203;W:ff0000;SPACE:0080ff;1:0a141e");
        assert_eq!(KeyLayout::decode(&text), Some(layout));
        let empty = KeyLayout::default();
        assert_eq!(empty.encode(), "000000");
        assert_eq!(KeyLayout::decode("000000"), Some(empty));
    }

    #[test]
    fn keys_that_move_are_saved_and_read_back() {
        let mut layout = KeyLayout::default();
        layout.set_with("W", [255, 0, 0], KeyFx::Breathing);
        layout.set_with("A", [0, 255, 0], KeyFx::Static);
        layout.set_with("S", [0, 0, 255], KeyFx::Fire);
        layout.set_with("D", [1, 2, 3], KeyFx::Spectrum);
        layout.set_with("F", [4, 5, 6], KeyFx::Flash);
        layout.set_with("NOPE", [4, 5, 6], KeyFx::Fire);
        assert!(layout.is_animated());
        let text = layout.encode();
        assert_eq!(text, "000000;W:ff0000:b;A:00ff00;S:0000ff:f;D:010203:r;F:040506:x");
        assert_eq!(KeyLayout::decode(&text), Some(layout.clone()));
        // a key that stops moving, or goes dark, is forgotten
        layout.set_with("W", [255, 0, 0], KeyFx::Static);
        assert_eq!(layout.fx_of("W"), KeyFx::Static);
        layout.clear_key("S");
        assert_eq!(layout.fx_of("S"), KeyFx::Static);
        layout.clear_keys();
        assert!(!layout.is_animated());
        // old text without effects still reads, and an unknown letter is a plain key
        let old = KeyLayout::decode("000000;W:ff0000;A:00ff00:q").unwrap();
        assert_eq!(old.keys.len(), 2);
        assert!(!old.is_animated());
        // painting a color over a key keeps what it does
        let mut keep = KeyLayout::default();
        keep.set_with("W", [1, 1, 1], KeyFx::Fire);
        keep.set("W", [9, 9, 9]);
        assert_eq!(keep.fx_of("W"), KeyFx::Fire);
        assert_eq!(KeyFx::from_index(KeyFx::Flash.index()), KeyFx::Flash);
        assert_eq!(KeyFx::from_index(200), KeyFx::Static);
    }

    #[test]
    fn keys_that_move_move_and_the_rest_stand_still() {
        let mut layout = KeyLayout::default();
        layout.set_with("W", [200, 100, 50], KeyFx::Breathing);
        layout.set_with("A", [200, 100, 50], KeyFx::Static);
        layout.set_with("S", [200, 100, 50], KeyFx::Flash);
        layout.set_with("D", [200, 100, 50], KeyFx::Spectrum);
        layout.set_with("F", [200, 100, 50], KeyFx::Fire);
        let (w, a, s, d, f) = (
            key_by_id("W").unwrap(),
            key_by_id("A").unwrap(),
            key_by_id("S").unwrap(),
            key_by_id("D").unwrap(),
            key_by_id("F").unwrap(),
        );
        let flame = Fire::new(7);
        let at = |t: f32| layout.grid_at(0, t, 0, &flame);
        let cell = |g: &KeyGrid, k: &KeyDef| g[k.row as usize][k.col as usize];
        // static is the color all the time
        assert_eq!(cell(&at(0.0), a), [200, 100, 50]);
        assert_eq!(cell(&at(1.7), a), [200, 100, 50]);
        // breathing starts dark and gets brighter, and never goes past its color
        assert_eq!(cell(&at(0.0), w), [0, 0, 0]);
        let mid = crate::command::breath_period(0) / 2.0;
        assert!(cell(&at(mid), w)[0] > 150 && cell(&at(mid), w)[0] <= 200);
        // flash is on, then off
        // (a blink takes half a breath: on for the first half of it)
        let quarter = crate::command::breath_period(0) / 4.0;
        assert_eq!(cell(&at(0.01), s), [200, 100, 50]);
        assert_eq!(cell(&at(quarter + 0.01), s), [0, 0, 0]);
        // spectrum shows the rainbow and not the key's own color
        assert_ne!(cell(&at(0.0), d), [200, 100, 50]);
        assert_ne!(cell(&at(0.0), d), cell(&at(3.0), d));
        // fire keeps the key's hue and is never brighter than its color
        let burn = cell(&at(0.0), f);
        assert!(burn[0] <= 200 && burn[1] <= 100 && burn[2] <= 50);
        assert!(burn[0] >= 25, "the flame never goes out completely: {:?}", burn);
        // keys that are not lit stay the background
        assert_eq!(at(1.0)[0][0], [0, 0, 0]);
    }

    #[test]
    fn the_rainbow_goes_all_the_way_round() {
        assert_eq!(hue(0.0), [255, 0, 0]);
        assert_eq!(hue(1.0 / 3.0), [0, 255, 0]);
        assert_eq!(hue(2.0 / 3.0), [0, 0, 255]);
        assert_eq!(hue(1.0), [255, 0, 0]);
    }

    #[test]
    fn decode_is_forgiving() {
        let layout = KeyLayout::decode("000000;W:ff0000;BOGUS:00ff00;A:zz0000;S;D:0000ff").unwrap();
        assert_eq!(layout.keys.len(), 2);
        assert_eq!(layout.color_of("W"), Some([255, 0, 0]));
        assert_eq!(layout.color_of("D"), Some([0, 0, 255]));
        assert_eq!(KeyLayout::decode("not-hex"), None);
        assert_eq!(KeyLayout::decode(""), None);
        assert_eq!(KeyLayout::decode("ééé"), None);
    }

    #[test]
    fn grid_paints_keys_background_and_shift() {
        let mut layout = KeyLayout { background: [5, 5, 5], ..KeyLayout::default() };
        layout.set("W", [255, 0, 0]);
        layout.set("SPACE", [0, 0, 255]);
        let grid = layout.grid(0);
        assert_eq!(grid[2][2], [255, 0, 0]);
        assert_eq!(grid[2][3], [5, 5, 5]);
        for col in 5..8 {
            assert_eq!(grid[5][col], [0, 0, 255]);
        }
        assert_eq!(grid[5][4], [5, 5, 5]);
        assert_eq!(grid[5][8], [5, 5, 5]);
        let shifted = layout.grid(1);
        assert_eq!(shifted[2][3], [255, 0, 0]);
        assert_eq!(shifted[2][2], [5, 5, 5]);
        // pushed off the left edge: the cell is dropped, nothing panics
        let mut esc = KeyLayout::default();
        esc.set("ESC", [1, 1, 1]);
        assert_eq!(esc.grid(-1)[0][0], [0, 0, 0]);
        let mut pwr = KeyLayout::default();
        pwr.set("PWR", [1, 1, 1]);
        assert_eq!(pwr.grid(2)[0][14], [0, 0, 0]);
    }

    #[test]
    fn a_second_color_is_saved_and_read_back_and_only_where_it_means_something() {
        let mut layout = KeyLayout::default();
        layout.set_with_second("W", [255, 0, 0], Some([0, 0, 255]), KeyFx::Breathing);
        layout.set_with_second("A", [0, 255, 0], Some([1, 2, 3]), KeyFx::Static); // nothing to go with
        layout.set_with_second("S", [9, 9, 9], Some([4, 5, 6]), KeyFx::Dipp);
        layout.set_with_second("D", [7, 7, 7], None, KeyFx::Starlight);
        layout.set_with_second("F", [8, 8, 8], Some([1, 1, 1]), KeyFx::Flash); // flash has one color
        assert_eq!(layout.second_of("W"), Some([0, 0, 255]));
        assert_eq!(layout.second_of("A"), None);
        assert_eq!(layout.second_of("F"), None);
        let text = layout.encode();
        assert_eq!(text, "000000;W:ff0000:b:0000ff;A:00ff00;S:090909:d:040506;D:070707:l;F:080808:x");
        assert_eq!(KeyLayout::decode(&text), Some(layout.clone()));
        // painting again replaces the second color, a plain paint over the key keeps it
        layout.set("W", [1, 1, 1]);
        assert_eq!(layout.second_of("W"), Some([0, 0, 255]));
        layout.set_with("W", [1, 1, 1], KeyFx::Breathing);
        assert_eq!(layout.second_of("W"), None);
        layout.clear_key("S");
        assert_eq!(layout.second_of("S"), None);
        layout.clear_keys();
        assert!(layout.second.is_empty());
        // saves from before (an effect and no second color, an unknown letter, a bad second color)
        let old = KeyLayout::decode("000000;W:ff0000:b;A:00ff00:q:0000ff;S:0000ff:b:zzzzzz").unwrap();
        assert_eq!(old.second_of("W"), None);
        assert_eq!(old.fx_of("A"), KeyFx::Static);
        assert_eq!(old.fx_of("S"), KeyFx::Breathing);
        assert_eq!(old.second_of("S"), None);
    }

    #[test]
    fn the_new_effects_are_added_on_the_end_so_saved_brushes_still_mean_the_same() {
        assert_eq!(KeyFx::from_index(0), KeyFx::Static);
        assert_eq!(KeyFx::from_index(1), KeyFx::Breathing);
        assert_eq!(KeyFx::from_index(4), KeyFx::Flash);
        assert_eq!(KeyFx::from_index(5), KeyFx::Starlight);
        assert_eq!(KeyFx::from_index(6), KeyFx::Dipp);
        assert_eq!(KeyFx::from_index(7), KeyFx::Wheel);
        let codes: std::collections::HashSet<char> = KeyFx::ALL.iter().map(|fx| fx.code()).collect();
        assert_eq!(codes.len(), KeyFx::ALL.len(), "every effect has its own letter");
        for fx in KeyFx::ALL {
            assert_eq!(KeyFx::from_code(&fx.code().to_string()), Some(fx));
            assert_eq!(KeyFx::from_index(fx.index()), fx);
            assert!(!fx.name().is_empty());
        }
        assert!(KeyFx::Breathing.can_two() && KeyFx::Fire.can_two() && KeyFx::Starlight.can_two());
        assert!(KeyFx::Dipp.can_two() && KeyFx::Wheel.can_two());
        assert!(!KeyFx::Static.can_two() && !KeyFx::Spectrum.can_two() && !KeyFx::Flash.can_two());
        assert!(KeyFx::Dipp.needs_two() && !KeyFx::Breathing.needs_two());
    }

    #[test]
    fn two_breathing_colors_take_turns_one_breath_each() {
        let mut layout = KeyLayout::default();
        layout.set_with_second("W", [200, 0, 0], Some([0, 0, 200]), KeyFx::Breathing);
        let flame = Fire::new(1);
        let period = crate::command::breath_period(0);
        let w = key_by_id("W").unwrap();
        let at = |t: f32| layout.grid_at(0, t, 0, &flame)[w.row as usize][w.col as usize];
        let first = at(period * 0.5);
        let second = at(period * 1.5);
        assert!(first[0] > 100 && first[2] == 0, "red first: {first:?}");
        assert!(second[2] > 100 && second[0] == 0, "then blue: {second:?}");
        assert_eq!(at(period * 2.5), first, "and round again");
        // one color alone breathes as before
        let mut one = KeyLayout::default();
        one.set_with("W", [200, 0, 0], KeyFx::Breathing);
        assert_eq!(one.grid_at(0, period * 1.5, 0, &flame)[w.row as usize][w.col as usize][2], 0);
    }

    #[test]
    fn a_fire_key_with_two_colors_goes_between_cold_and_hot_and_stays_in_them() {
        let mut layout = KeyLayout::default();
        layout.set_with_second("W", [250, 120, 0], Some([20, 0, 90]), KeyFx::Fire);
        let w = key_by_id("W").unwrap();
        let mut flame = Fire::new(3);
        let (mut low, mut high) = ([255u8; 3], [0u8; 3]);
        for _ in 0..400 {
            flame.advance(0.05);
            let px = layout.grid_at(0, 0.0, 0, &flame)[w.row as usize][w.col as usize];
            assert!(px[0] >= 20 && px[0] <= 250 && px[1] <= 120 && px[2] <= 90, "{px:?}");
            if px[0] < low[0] { low = px }
            if px[0] > high[0] { high = px }
        }
        assert!(high[0] > low[0] + 60, "it flickers across the range: {low:?} to {high:?}");
        assert!(low[2] > 20, "the low end leans to the cold color: {low:?}");
        let heat = flame.key_heat(w.row as usize, w.col as usize).unwrap();
        assert!((0.0..=1.0).contains(&heat));
        assert_eq!(flame.key_heat(5, 3), None, "no key there");
    }

    #[test]
    fn starlight_keys_twinkle_and_color_dipp_keys_go_from_one_color_to_the_other() {
        let mut layout = KeyLayout::default();
        layout.set_with_second("W", [0, 200, 0], Some([0, 0, 200]), KeyFx::Starlight);
        layout.set_with_second("A", [255, 0, 0], Some([136, 0, 255]), KeyFx::Dipp);
        layout.set_with("S", [255, 0, 0], KeyFx::Dipp);
        let flame = Fire::new(1);
        let (w, a, s) = (key_by_id("W").unwrap(), key_by_id("A").unwrap(), key_by_id("S").unwrap());
        let cell = |g: &KeyGrid, k: &KeyDef| g[k.row as usize][k.col as usize];
        let (mut dark, mut lit_) = (0, 0);
        for step in 0..1500 {
            let g = layout.grid_at(0, step as f32 * 0.03, 100, &flame);
            if cell(&g, w) == [0, 0, 0] { dark += 1 } else { lit_ += 1 }
            assert_eq!(cell(&g, w)[0], 0);
        }
        assert!(dark > 0 && lit_ > 0, "{dark} {lit_}");
        // the dipp starts on its color, rests, goes across and comes to the second color
        assert_eq!(cell(&layout.grid_at(0, 0.0, 100, &flame), a), [255, 0, 0]);
        let ends: Vec<[u8; 3]> = (0..400).map(|i| cell(&layout.grid_at(0, i as f32 * 0.1, 100, &flame), a)).collect();
        assert!(ends.contains(&[136, 0, 255]));
        assert!(ends.iter().any(|c| c[1] > 100), "through the hues between: green shows");
        // with no second color it still goes somewhere (the usual violet) instead of standing still
        assert!((0..400).any(|i| cell(&layout.grid_at(0, i as f32 * 0.1, 100, &flame), s) != [255, 0, 0]));
        // the faster speed gets there sooner
        let sooner = (0..400).map(|i| i as f32 * 0.05).find(|t| cell(&layout.grid_at(0, *t, 250, &flame), a) == [136, 0, 255]);
        let later = (0..400).map(|i| i as f32 * 0.05).find(|t| cell(&layout.grid_at(0, *t, 50, &flame), a) == [136, 0, 255]);
        assert!(sooner.is_some() && sooner < later.or(Some(f32::MAX)));
    }

    #[test]
    fn the_wheel_sweeps_clockwise_like_a_clocks_hand_with_a_fading_tail() {
        let white = [240, 240, 240];
        // straight up from the middle is 12 o'clock, to the right 3, down 6, to the left 9
        assert!(wheel_angle(0, 7) < 0.4 || wheel_angle(0, 7) > std::f32::consts::TAU - 0.4);
        assert!((wheel_angle(2, 14) - std::f32::consts::FRAC_PI_2).abs() < 0.5);
        assert!((wheel_angle(5, 7) - std::f32::consts::PI).abs() < 0.5);
        assert!((wheel_angle(3, 0) - 1.5 * std::f32::consts::PI).abs() < 0.5);
        // a quarter turn in, the hand points right: the keys at the right are the brightest,
        // the ones just before them (the top right) fade, the ones the hand has yet to reach are dark
        let quarter = WHEEL_TURN_SECS / 4.0;
        let right = wheel_color(2, 14, quarter, white, None);
        let before = wheel_color(0, 12, quarter, white, None);
        let ahead = wheel_color(5, 7, quarter, white, None);
        assert!(right[0] > 200, "{right:?}");
        assert!(before[0] > 0 && before[0] < right[0], "{before:?}");
        assert_eq!(ahead, [0, 0, 0]);
        // a whole turn later it is the same picture
        assert_eq!(wheel_color(2, 14, quarter + WHEEL_TURN_SECS, white, None), right);
        // with a second color the tail turns into it as it fades
        let (a, b) = ([255, 0, 0], [0, 0, 255]);
        let head = wheel_color(2, 14, quarter, a, Some(b));
        let tail = wheel_color(0, 12, quarter, a, Some(b));
        assert!(head[0] > head[2] && tail[2] > 0, "{head:?} {tail:?}");
        // every lit cell is the dark, or a mix of the two colors and never brighter than them
        for step in 0..200 {
            for row in 0..ROWS {
                for col in 0..COLS {
                    let px = wheel_color(row, col, step as f32 * 0.07, a, Some(b));
                    assert_eq!(px[1], 0);
                }
            }
        }
        // in a layout: only the lit keys, at the speed asked for
        let mut layout = KeyLayout::default();
        layout.set_with_second("UP", a, Some(b), KeyFx::Wheel);
        let flame = Fire::new(1);
        let lit_cells = |t: f32, speed: u8| layout.grid_at(0, t, speed, &flame).iter().flatten().filter(|c| **c != [0, 0, 0]).count();
        assert!((0..80).any(|i| lit_cells(i as f32 * 0.05, 100) == 1));
        assert!((0..80).any(|i| lit_cells(i as f32 * 0.05, 100) == 0));
        let up = key_by_id("UP").unwrap();
        let sees = |speed: u8| (0..4000).map(|i| i as f32 * 0.005).find(|t| layout.grid_at(0, *t, speed, &flame)[up.row as usize][up.col as usize] != [0, 0, 0]);
        assert!(sees(250).unwrap() < sees(50).unwrap());
    }
}
