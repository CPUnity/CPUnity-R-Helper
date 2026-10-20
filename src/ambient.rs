//! The shop's live backdrops: where each spark, column of characters and curtain of light is at a
//! given time. Everything is a pure function of the time, so it is tested; `ui/backdrop.rs` draws it.
//! Positions are fractions of the window (0 to 1 across and down).

fn hash(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(2_654_435_761).wrapping_add(salt.wrapping_mul(40_503)) | 1;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

// ---- Embers ----

/// How many sparks there are.
pub const EMBERS: u32 = 64;

/// A spark: where it is, how big, and how hot (1 is white-orange, 0 has gone dark red).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ember {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub heat: f32,
}

/// Spark `i` at time `t` seconds: it rises from below the window, sways, and cools on the way up.
pub fn ember(i: u32, t: f32) -> Ember {
    let period = 9.0 + hash(i, 1) * 9.0;
    let phase = (t / period + hash(i, 2)).fract();
    let base = hash(i, 3);
    let sway = (t * (0.4 + hash(i, 4) * 0.5) + hash(i, 5) * 6.28).sin() * 0.035;
    Ember { x: base + sway, y: 1.04 - phase * 1.1, size: 1.2 + hash(i, 6) * 2.6, heat: (1.0 - phase).powf(1.4) }
}

/// The colour of a spark of this heat (red, orange, near white).
pub fn ember_colour(heat: f32) -> [u8; 3] {
    let h = heat.clamp(0.0, 1.0);
    [(150.0 + 105.0 * h) as u8, (24.0 + 170.0 * h * h) as u8, (8.0 + 90.0 * h * h * h) as u8]
}

// ---- Data rain ----

/// How many columns of characters there are, and how long a trail is.
pub const COLUMNS: u32 = 38;
pub const TRAIL: u32 = 12;

/// What the rain is made of.
pub const GLYPHS: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', '<', '>', '/', '|', '+', '=', ':'];

/// A column: where across, how fast it falls (in window heights per second), and where its head is
/// (down from the top, in window heights; it starts above the window and ends below).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    pub x: f32,
    pub head: f32,
    pub speed: f32,
}

pub fn column(i: u32, t: f32) -> Column {
    let speed = 0.12 + hash(i, 11) * 0.16;
    let travel = 1.0 + TRAIL as f32 * 0.04 + 0.3;
    let phase = (t * speed / travel + hash(i, 12)).fract();
    Column { x: (i as f32 + 0.5) / COLUMNS as f32, head: phase * travel - 0.2, speed }
}

/// The character at `row` of a trail (0 is the head), which changes now and then.
pub fn glyph(i: u32, row: u32, t: f32) -> char {
    let tick = (t * (2.0 + hash(i, 13) * 3.0)) as u32 / 2;
    GLYPHS[(hash(i.wrapping_mul(131).wrapping_add(row), tick) * GLYPHS.len() as f32) as usize % GLYPHS.len()]
}

/// How bright the trail is at `row` (the head is brightest, the tail has faded).
pub fn trail_alpha(row: u32) -> f32 {
    (1.0 - row as f32 / TRAIL as f32).clamp(0.0, 1.0).powf(1.5)
}

// ---- Northern lights ----

/// How many curtains there are, and how many points each is drawn from.
pub const CURTAINS: u32 = 4;
pub const STEPS: usize = 48;

/// One slice of a curtain: where across, where its middle is (down the window), how tall it is,
/// and how bright.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slice {
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub alpha: f32,
}

/// The colour of curtain `i`: green, teal, violet, green.
pub fn curtain_colour(i: u32) -> [u8; 3] {
    match i % 4 {
        0 => [60, 255, 150],
        1 => [40, 210, 220],
        2 => [150, 90, 255],
        _ => [90, 255, 110],
    }
}

/// Curtain `i` at time `t`: `STEPS` slices from the left of the window to the right. It sways
/// slowly, and the brightness swells and fades along it.
pub fn curtain(i: u32, t: f32) -> Vec<Slice> {
    let drift = 0.05 + hash(i, 21) * 0.07;
    let base = 0.22 + 0.5 * hash(i, 22);
    (0..STEPS)
        .map(|s| {
            let x = s as f32 / (STEPS - 1) as f32;
            let wave = (x * 5.0 + t * drift * 6.0 + hash(i, 23) * 6.28).sin() * 0.06 + (x * 11.0 - t * drift * 9.0).sin() * 0.02;
            let swell = ((x * 3.0 - t * drift * 4.0 + hash(i, 24) * 6.28).sin() * 0.5 + 0.5).powf(1.3);
            Slice { x, y: base + wave, height: 0.14 + 0.16 * swell, alpha: 0.05 + 0.2 * swell }
        })
        .collect()
}

// ---- Sakura petals ----

/// How many petals there are.
pub const PETALS: u32 = 46;

/// A petal: where it is, how big, how it is turned, how much it is flipped over (1 is face on, 0
/// is edge on), how bright, and how deep its pink is (0 pale, 1 deep).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Petal {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub angle: f32,
    pub flip: f32,
    pub alpha: f32,
    pub tone: f32,
}

/// Petal `i` at time `t`: it falls from above the window, drifts to one side, sways, and turns over
/// as it goes.
pub fn petal(i: u32, t: f32) -> Petal {
    let period = 16.0 + hash(i, 31) * 16.0;
    let phase = (t / period + hash(i, 32)).fract();
    let sway = (t * (0.5 + hash(i, 33) * 0.6) + hash(i, 34) * 6.28).sin() * 0.03;
    Petal {
        x: hash(i, 35) * 1.2 - 0.1 + phase * (0.1 + hash(i, 36) * 0.12) + sway,
        y: -0.05 + phase * 1.1,
        size: 5.0 + hash(i, 37) * 7.0,
        angle: t * (0.3 + hash(i, 38) * 0.9) + hash(i, 39) * 6.28,
        flip: (t * (0.8 + hash(i, 40) * 1.2) + hash(i, 41) * 6.28).cos().abs().max(0.25),
        alpha: 0.45 + hash(i, 42) * 0.5,
        tone: hash(i, 43),
    }
}

/// The colour of a petal of this tone (pale pink to deeper pink).
pub fn petal_colour(tone: f32) -> [u8; 3] {
    let k = tone.clamp(0.0, 1.0);
    [255, (214.0 - 66.0 * k) as u8, (226.0 - 52.0 * k) as u8]
}

/// How many points the outline of a petal has.
pub const PETAL_POINTS: usize = 12;

/// The outline of a petal, one unit tall: round, pointed at the bottom and notched at the top,
/// around its middle (0, 0). It can be drawn as a fan from the middle.
pub fn petal_outline() -> [(f32, f32); PETAL_POINTS] {
    let mut out = [(0.0, 0.0); PETAL_POINTS];
    for (k, p) in out.iter_mut().enumerate() {
        let a = std::f32::consts::TAU * k as f32 / PETAL_POINTS as f32;
        let (s, c) = (a.sin(), a.cos());
        // an egg: wide at the top, narrowing to the point at the bottom
        let wide = 0.36 * (1.0 + 0.28 * c);
        let mut r = 1.0 / ((s / wide).powi(2) + (c / 0.5).powi(2)).sqrt();
        if k == 0 {
            r *= 0.7; // the notch
        }
        *p = (s * r, -c * r);
    }
    out
}

// ---- Synthwave grid ----

/// Where the horizon is (down the window, 0 to 1), how big the sun is (as a fraction of the window's
/// height) and how many lines the grid has.
pub const HORIZON: f32 = 0.58;
pub const SUN_RADIUS: f32 = 0.2;
pub const GRID_ROWS: u32 = 12;
pub const GRID_SIDE_LINES: i32 = 9;

/// The horizontal lines of the grid, as how far down the window they are. They come towards you:
/// each line moves down, faster as it gets nearer, and a new one starts at the horizon.
pub fn grid_rows(t: f32) -> Vec<f32> {
    let phase = (t * 0.3).rem_euclid(1.0);
    (0..GRID_ROWS)
        .map(|k| {
            let d = (k as f32 + phase) / GRID_ROWS as f32;
            HORIZON + (1.0 - HORIZON) * d * d
        })
        .collect()
}

/// The lines that run away from you: where each meets the bottom of the window (across, 0 to 1).
/// They all meet at the middle of the horizon.
pub fn grid_columns() -> Vec<f32> {
    (-GRID_SIDE_LINES..=GRID_SIDE_LINES).map(|j| 0.5 + j as f32 * 0.17).collect()
}

/// The stripes cut out of the sun, from just above the horizon upwards: how high up each is and how
/// thick, both as fractions of the sun's radius. They get thinner as they go up.
pub fn sun_stripes() -> [(f32, f32); 6] {
    [(0.10, 0.09), (0.26, 0.07), (0.40, 0.055), (0.53, 0.04), (0.65, 0.03), (0.76, 0.02)]
}

// ---- Bubbles ----

/// How many bubbles there are, and how many beams of light.
pub const BUBBLES: u32 = 42;
pub const SHAFTS: u32 = 4;

/// A bubble: where it is, how big (in pixels, the radius) and how bright.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bubble {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub alpha: f32,
}

/// Bubble `i` at time `t`: it rises slowly from below the window, wobbling from side to side, and
/// fades in at the bottom and out at the top.
pub fn bubble(i: u32, t: f32) -> Bubble {
    let period = 14.0 + hash(i, 51) * 16.0;
    let phase = (t / period + hash(i, 52)).fract();
    let wobble = (t * (0.6 + hash(i, 53) * 0.8) + hash(i, 54) * 6.28).sin() * 0.018;
    let edge = (phase * 12.0).min(1.0) * ((1.0 - phase) * 6.0).min(1.0);
    Bubble { x: hash(i, 56) + wobble, y: 1.05 - phase * 1.12, radius: 2.5 + hash(i, 55) * 7.5, alpha: (0.35 + 0.45 * hash(i, 57)) * edge }
}

/// A beam of light from the surface: where it starts across the top, how wide it is there, how far
/// it leans by the bottom (all as fractions of the window's width) and how bright.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shaft {
    pub x: f32,
    pub width: f32,
    pub lean: f32,
    pub alpha: f32,
}

pub fn shaft(i: u32, t: f32) -> Shaft {
    let sway = (t * (0.08 + hash(i, 61) * 0.06) + hash(i, 62) * 6.28).sin();
    let breathe = (t * (0.15 + hash(i, 65) * 0.1) + hash(i, 66) * 6.28).sin() * 0.5 + 0.5;
    Shaft {
        x: (i as f32 + 0.5 + (hash(i, 63) - 0.5) * 0.6) / SHAFTS as f32 + sway * 0.03,
        width: 0.05 + hash(i, 64) * 0.06,
        lean: 0.12 + sway * 0.05,
        alpha: 0.05 + 0.07 * breathe,
    }
}

// ---- Scanlines ----

/// The gap between two scanlines, in pixels, and how long the bright band takes to go down the window.
pub const SCAN_SPACING: f32 = 3.0;
pub const SCAN_PERIOD: f32 = 8.0;

/// Where the bright band is (down the window, 0 to 1; it starts a little above and ends a little below).
pub fn scan_band(t: f32) -> f32 {
    -0.15 + (t / SCAN_PERIOD).rem_euclid(1.0) * 1.3
}

/// How steady the picture is: a faint flicker, between 0.8 and 1.
pub fn scan_flicker(t: f32) -> f32 {
    0.9 + 0.1 * (t * 37.0).sin() * (t * 11.0).sin()
}

/// Whether the cursor is showing (it blinks).
pub fn cursor_on(t: f32) -> bool {
    (t * 1.6).rem_euclid(1.0) < 0.5
}

// ---- Fireflies ----

/// How many fireflies there are.
pub const FIREFLIES: u32 = 30;

/// A firefly: where it is, how much it glows (0 is dark) and how big.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Firefly {
    pub x: f32,
    pub y: f32,
    pub glow: f32,
    pub size: f32,
}

/// Firefly `i`: it wanders in loops round a spot of its own, and lights up now and then, with long
/// dark spells between.
pub fn firefly(i: u32, t: f32) -> Firefly {
    let (s1, s2) = (0.12 + hash(i, 73) * 0.15, 0.1 + hash(i, 74) * 0.12);
    let x = 0.05 + hash(i, 71) * 0.9 + 0.045 * (t * s1 + hash(i, 75) * 6.28).sin() + 0.02 * (t * s2 * 2.3 + hash(i, 76) * 6.28).sin();
    let y = 0.15 + hash(i, 72) * 0.8 + 0.05 * (t * s2 + hash(i, 77) * 6.28).cos() + 0.02 * (t * s1 * 1.9 + hash(i, 78) * 6.28).sin();
    let glow = ((t * (0.35 + hash(i, 79) * 0.3) + hash(i, 80) * 6.28).sin() * 0.5 + 0.5).powi(3);
    Firefly { x, y, glow, size: 1.6 + hash(i, 81) * 1.6 }
}

/// The colour of a firefly this bright: yellow-green, going paler at the brightest.
pub fn firefly_colour(glow: f32) -> [u8; 3] {
    let g = glow.clamp(0.0, 1.0);
    [(170.0 + 70.0 * g) as u8, 255, (70.0 + 80.0 * g) as u8]
}

// ---- Snowfall ----

/// How many flakes there are.
pub const FLAKES: u32 = 110;

/// A snowflake: where it is, how big (the radius in pixels) and how bright. The bigger ones are
/// nearer: they fall faster and sway more.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flake {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub alpha: f32,
}

pub fn flake(i: u32, t: f32) -> Flake {
    let depth = hash(i, 91);
    let fall = 0.035 + depth * 0.06;
    let phase = (t * fall / 1.1 + hash(i, 92)).fract();
    let sway = (t * (0.3 + hash(i, 93) * 0.5) + hash(i, 94) * 6.28).sin() * (0.01 + 0.02 * depth);
    Flake { x: hash(i, 95) + sway + phase * 0.05, y: -0.04 + phase * 1.08, size: 1.0 + depth * 2.6, alpha: 0.35 + 0.55 * depth }
}

// ---- Storm ----

/// How many raindrops there are, how far each one leans (pixels sideways for each pixel down) and
/// how often lightning may strike, in seconds.
pub const DROPS: u32 = 120;
pub const RAIN_SLANT: f32 = 0.2;
pub const FLASH_EVERY: f32 = 11.0;

/// A raindrop: where its head is, how long its streak is (in pixels) and how bright.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    pub x: f32,
    pub y: f32,
    pub len: f32,
    pub alpha: f32,
}

pub fn drop(i: u32, t: f32) -> Drop {
    let speed = 1.1 + hash(i, 101) * 0.7;
    let phase = (t * speed / 1.15 + hash(i, 102)).fract();
    Drop { x: hash(i, 103) * 1.2 - 0.1 + phase * 0.12, y: -0.05 + phase * 1.1, len: 10.0 + hash(i, 104) * 14.0, alpha: 0.22 + hash(i, 105) * 0.4 }
}

fn decay(d: f32) -> f32 {
    if d < 0.0 {
        0.0
    } else {
        (-d * 9.0).exp()
    }
}

/// The strike for the stretch of `FLASH_EVERY` seconds that `t` is in: whether there is one (about
/// four in five have), and when in that stretch.
fn strike(epoch: u32) -> Option<f32> {
    (hash(epoch, 111) >= 0.2).then(|| 1.0 + hash(epoch, 112) * (FLASH_EVERY - 3.0))
}

/// How bright the sky flashes at time `t`, 0 to 1: a main flash and, a moment later, a weaker echo.
pub fn flash(t: f32) -> f32 {
    let t = t.max(0.0);
    let epoch = (t / FLASH_EVERY).floor();
    let local = t - epoch * FLASH_EVERY;
    match strike(epoch as u32) {
        Some(at) => decay(local - at).max(0.65 * decay(local - at - 0.2)).min(1.0),
        None => 0.0,
    }
}

/// The bolt for the stretch `t` is in: a jagged line down from the top (the main one, then a
/// branch), as fractions of the window.
pub fn bolt(t: f32) -> Vec<Vec<(f32, f32)>> {
    let epoch = (t.max(0.0) / FLASH_EVERY).floor() as u32;
    let mut x = 0.12 + hash(epoch, 113) * 0.76;
    let reach = 0.3 + hash(epoch, 114) * 0.25;
    let steps = 9;
    let mut main = vec![(x, 0.0)];
    for s in 1..=steps {
        x += (hash(epoch, 120 + s) - 0.5) * 0.06;
        main.push((x, reach * s as f32 / steps as f32));
    }
    let from = main[4];
    let dir = if hash(epoch, 115) < 0.5 { -1.0 } else { 1.0 };
    let branch = (0..4).map(|s| (from.0 + dir * 0.025 * s as f32 + (hash(epoch, 140 + s) - 0.5) * 0.02, from.1 + 0.045 * s as f32)).collect();
    vec![main, branch]
}

// ---- Constellations ----

/// How many stars there are.
pub const STARS: u32 = 44;

/// A star: where it is, how brightly it twinkles (0.35 to 1) and how big.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    pub x: f32,
    pub y: f32,
    pub glow: f32,
    pub size: f32,
}

/// Star `i`: it hangs at a spot of its own, drifting a hair about it, and twinkles slowly.
pub fn star(i: u32, t: f32) -> Star {
    let x = 0.03 + hash(i, 111) * 0.94 + 0.012 * (t * (0.04 + hash(i, 115) * 0.04) + hash(i, 116) * 6.28).sin();
    let y = 0.03 + hash(i, 112) * 0.94 + 0.012 * (t * (0.04 + hash(i, 117) * 0.04) + hash(i, 118) * 6.28).cos();
    let glow = 0.35 + 0.65 * ((t * (0.25 + hash(i, 113) * 0.5) + hash(i, 114) * 6.28).sin() * 0.5 + 0.5);
    Star { x, y, glow, size: 0.9 + hash(i, 119) * 1.7 }
}

/// How far apart two stars can be (as a fraction of the window's height) and still be joined by a line.
pub const LINK_REACH: f32 = 0.2;

/// How strongly two stars are joined, 0 (not at all) to 1 (touching). `aspect` is the window's width
/// over its height, so that distances are the same in both directions.
pub fn link(a: Star, b: Star, aspect: f32) -> f32 {
    let (dx, dy) = ((a.x - b.x) * aspect, a.y - b.y);
    let d = (dx * dx + dy * dy).sqrt();
    if d >= LINK_REACH {
        0.0
    } else {
        1.0 - d / LINK_REACH
    }
}

// ---- Sunbeams ----

/// How many beams of light there are.
pub const BEAMS: u32 = 5;

/// How far a beam leans to the right on its way down, as a fraction of the window's height.
pub const BEAM_LEAN: f32 = 0.3;

/// A beam: where it starts along the top (a fraction of the width), how wide it is (the same) and how
/// bright it is now (a low alpha, 0.04 to 0.14).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beam {
    pub x: f32,
    pub width: f32,
    pub alpha: f32,
}

/// Beam `i`: the beams stand side by side with a little scatter, sway a few hundredths of the width, and
/// swell and fade at their own slow pace.
pub fn beam(i: u32, t: f32) -> Beam {
    let slot = (i as f32 + 0.5 + (hash(i, 121) - 0.5) * 0.5) / BEAMS as f32;
    let x = -0.15 + slot * 1.2 + 0.03 * (t * (0.05 + hash(i, 122) * 0.05) + hash(i, 123) * 6.28).sin();
    let alpha = 0.04 + 0.10 * ((t * (0.12 + hash(i, 124) * 0.14) + hash(i, 125) * 6.28).sin() * 0.5 + 0.5);
    Beam { x, width: 0.05 + hash(i, 126) * 0.09, alpha }
}

/// How many specks of dust there are.
pub const MOTES: u32 = 36;

/// A speck of dust: where it is, how big and how bright.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mote {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub alpha: f32,
}

/// Speck `i`: it rises very slowly from the bottom to the top and sways, fading in and out at the ends.
pub fn mote(i: u32, t: f32) -> Mote {
    let period = 40.0 + hash(i, 131) * 40.0;
    let phase = (t / period + hash(i, 132)).fract();
    let x = hash(i, 133) + 0.03 * (t * (0.15 + hash(i, 134) * 0.2) + hash(i, 135) * 6.28).sin();
    let alpha = (phase * std::f32::consts::PI).sin() * (0.35 + 0.65 * ((t * (0.3 + hash(i, 136) * 0.4) + hash(i, 137) * 6.28).sin() * 0.5 + 0.5));
    Mote { x, y: 1.05 - phase * 1.1, size: 0.8 + hash(i, 138) * 1.4, alpha }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embers_rise_cool_and_come_round_again() {
        for i in 0..EMBERS {
            let mut last = ember(i, 0.0);
            let mut wrapped = false;
            let mut t = 0.0;
            while t < 40.0 {
                t += 0.25;
                let e = ember(i, t);
                assert!(e.heat >= 0.0 && e.heat <= 1.0 && e.size > 1.0 && e.x > -0.1 && e.x < 1.1, "{e:?}");
                assert!(e.y > -0.1 && e.y < 1.05, "{e:?}");
                if e.y > last.y + 0.01 {
                    wrapped = true;
                } else {
                    assert!(e.y <= last.y + 0.01, "it only goes up until it wraps");
                }
                last = e;
            }
            assert!(wrapped, "spark {i} came round again in 40 s");
        }
    }

    #[test]
    fn the_lower_a_spark_the_hotter_it_is() {
        let fresh = ember_colour(1.0);
        let old = ember_colour(0.05);
        assert!(fresh[0] > old[0] && fresh[1] > old[1] && fresh[2] > old[2]);
        assert!(old[0] > old[1] && old[1] >= old[2], "cooled to red");
        assert_eq!(ember_colour(9.0), ember_colour(1.0), "out of range is clamped");
    }

    #[test]
    fn embers_are_the_same_every_time() {
        assert_eq!(ember(5, 12.5), ember(5, 12.5));
        assert_ne!(ember(5, 12.5), ember(6, 12.5));
    }

    #[test]
    fn rain_columns_are_spread_across_and_fall_down_and_start_over() {
        let xs: Vec<f32> = (0..COLUMNS).map(|i| column(i, 0.0).x).collect();
        assert!(xs.windows(2).all(|w| w[1] > w[0]) && xs[0] > 0.0 && *xs.last().unwrap() < 1.0);
        for i in 0..COLUMNS {
            let mut last = column(i, 0.0).head;
            let mut wrapped = false;
            let mut t = 0.0;
            while t < 60.0 {
                t += 0.2;
                let c = column(i, t);
                assert!(c.head >= -0.21 && c.head < 1.7, "{c:?}");
                assert!(c.speed > 0.1 && c.speed < 0.3);
                if c.head < last - 0.05 {
                    wrapped = true;
                }
                last = c.head;
            }
            assert!(wrapped, "column {i} wrapped within a minute");
        }
    }

    #[test]
    fn rain_characters_come_from_the_set_change_over_time_and_the_trail_fades() {
        let mut seen = std::collections::BTreeSet::new();
        for row in 0..TRAIL {
            for step in 0..40 {
                let g = glyph(3, row, step as f32 * 0.5);
                assert!(GLYPHS.contains(&g));
                seen.insert(g);
            }
        }
        assert!(seen.len() > 8, "{} different characters", seen.len());
        assert_eq!(glyph(3, 2, 7.0), glyph(3, 2, 7.0));
        assert_eq!(trail_alpha(0), 1.0);
        assert!((0..TRAIL).map(trail_alpha).collect::<Vec<_>>().windows(2).all(|w| w[1] < w[0]));
        assert_eq!(trail_alpha(TRAIL + 5), 0.0);
    }

    #[test]
    fn a_curtain_runs_across_the_window_and_sways() {
        for i in 0..CURTAINS {
            let a = curtain(i, 0.0);
            assert_eq!(a.len(), STEPS);
            assert!(a[0].x == 0.0 && (a[STEPS - 1].x - 1.0).abs() < 1e-6);
            assert!(a.windows(2).all(|w| w[1].x > w[0].x));
            for s in &a {
                assert!(s.y > 0.05 && s.y < 0.95 && s.height > 0.1 && s.height < 0.4 && s.alpha > 0.0 && s.alpha < 0.3, "{s:?}");
            }
            assert_ne!(a, curtain(i, 30.0), "it moves");
        }
        assert_eq!(curtain(0, 3.0), curtain(0, 3.0));
        let colours: std::collections::BTreeSet<_> = (0..CURTAINS).map(curtain_colour).collect();
        assert!(colours.len() >= 3);
    }

    #[test]
    fn petals_fall_drift_and_come_round_again() {
        for i in 0..PETALS {
            let mut last = petal(i, 0.0);
            let mut wrapped = false;
            let mut t = 0.0;
            while t < 70.0 {
                t += 0.25;
                let p = petal(i, t);
                assert!(p.y >= -0.06 && p.y <= 1.06 && p.x > -0.2 && p.x < 1.4, "{p:?}");
                assert!(p.flip >= 0.25 && p.flip <= 1.0 && p.alpha > 0.4 && p.alpha < 1.0 && p.size >= 5.0 && p.size <= 12.0, "{p:?}");
                if p.y < last.y - 0.01 {
                    wrapped = true;
                }
                last = p;
            }
            assert!(wrapped, "petal {i} came round again in 70 s");
        }
        assert_eq!(petal(3, 8.0), petal(3, 8.0));
    }

    #[test]
    fn a_petal_is_pale_pink_to_deeper_pink() {
        let pale = petal_colour(0.0);
        let deep = petal_colour(1.0);
        assert!(pale[0] == 255 && pale[1] > deep[1] && pale[2] > deep[2]);
        assert_eq!(petal_colour(5.0), deep);
    }

    #[test]
    fn the_petal_outline_is_one_unit_tall_notched_and_can_be_drawn_as_a_fan() {
        let o = petal_outline();
        let (min_y, max_y) = o.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
        let height = max_y - min_y;
        assert!(height > 0.8 && height < 1.2, "height {height}");
        let max_x = o.iter().map(|p| p.0.abs()).fold(0.0, f32::max);
        assert!(max_x > 0.2 && max_x < 0.45, "width {max_x}");
        // the top (point 0) is notched: lower than its neighbours' height would suggest
        assert!(o[0].1.abs() < o[1].1.abs() + 0.5 && o[0].0 == 0.0);
        // star-shaped around the middle: the angle only goes round one way
        let mut turn = 0.0_f32;
        for k in 0..PETAL_POINTS {
            let (a, b) = (o[k], o[(k + 1) % PETAL_POINTS]);
            turn += (a.0 * b.1 - a.1 * b.0).signum();
        }
        assert_eq!(turn.abs() as usize, PETAL_POINTS, "every step goes the same way round");
    }

    #[test]
    fn the_grid_comes_towards_you_and_loops() {
        let a = grid_rows(0.0);
        assert_eq!(a.len(), GRID_ROWS as usize);
        assert!(a.windows(2).all(|w| w[1] > w[0]), "nearer lines are lower");
        assert!(a.iter().all(|y| *y >= HORIZON && *y <= 1.0));
        // the gaps widen towards you (perspective)
        let gaps: Vec<f32> = a.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.windows(2).all(|g| g[1] > g[0]));
        // it moves down, and after one loop (1 / 0.3 s) it is the same
        let b = grid_rows(0.5);
        assert!(b.iter().zip(&a).all(|(y2, y1)| y2 > y1));
        let loop_t = 1.0 / 0.3;
        for (x, y) in grid_rows(loop_t).iter().zip(&a) {
            assert!((x - y).abs() < 1e-3);
        }
    }

    #[test]
    fn the_lines_that_run_away_are_spread_evenly_and_run_off_both_sides() {
        let c = grid_columns();
        assert_eq!(c.len(), (2 * GRID_SIDE_LINES + 1) as usize);
        assert!((c[c.len() / 2] - 0.5).abs() < 1e-6, "one straight down the middle");
        assert!(c[0] < 0.0 && *c.last().unwrap() > 1.0);
        assert!(c.windows(2).all(|w| ((w[1] - w[0]) - 0.17).abs() < 1e-4));
    }

    #[test]
    fn the_sun_stripes_get_thinner_going_up_and_stay_inside_the_sun() {
        let s = sun_stripes();
        assert!(s.windows(2).all(|w| w[1].0 > w[0].0 && w[1].1 < w[0].1));
        assert!(s.iter().all(|(up, th)| *up > 0.0 && up + th / 2.0 < 1.0 && *th > 0.0));
    }

    #[test]
    fn bubbles_rise_and_the_beams_stay_on_the_window() {
        for i in 0..BUBBLES {
            let mut t = 0.0;
            while t < 40.0 {
                let b = bubble(i, t);
                assert!(b.x > -0.1 && b.x < 1.1 && b.y > -0.1 && b.y < 1.1 && b.radius >= 2.5 && b.radius <= 10.0, "{b:?}");
                assert!(b.alpha >= 0.0 && b.alpha <= 0.8, "{b:?}");
                let (a, c) = (bubble(i, t).y, bubble(i, t + 0.1).y);
                assert!(c < a || (a - c).abs() > 0.5, "it goes up until it comes round again");
                t += 0.5;
            }
        }
        for i in 0..SHAFTS {
            for t in [0.0, 13.0, 90.0] {
                let s = shaft(i, t);
                assert!(s.x > 0.0 && s.x < 1.0 && s.width > 0.0 && s.alpha > 0.0 && s.alpha < 0.2, "{s:?}");
            }
        }
    }

    #[test]
    fn the_scan_band_goes_down_and_loops_and_the_cursor_blinks() {
        assert!(scan_band(0.0) < 0.0 && scan_band(SCAN_PERIOD * 0.99) > 1.0);
        assert!(scan_band(1.0) > scan_band(0.5));
        assert!((scan_band(SCAN_PERIOD) - scan_band(0.0)).abs() < 1e-4);
        for k in 0..400 {
            let f = scan_flicker(k as f32 * 0.13);
            assert!((0.8..=1.0).contains(&f), "{f}");
        }
        let on = (0..100).filter(|k| cursor_on(*k as f32 * 0.05)).count();
        assert!(on > 30 && on < 70, "on about half the time: {on}");
    }

    #[test]
    fn fireflies_wander_near_home_and_go_dark_between_glows() {
        for i in 0..FIREFLIES {
            let mut dark = 0;
            let mut t = 0.0;
            while t < 120.0 {
                let f = firefly(i, t);
                assert!(f.x > -0.05 && f.x < 1.05 && f.y > 0.0 && f.y < 1.1, "{f:?}");
                assert!((0.0..=1.0).contains(&f.glow) && f.size >= 1.6, "{f:?}");
                if f.glow < 0.1 {
                    dark += 1;
                }
                t += 1.0;
            }
            assert!(dark > 20, "firefly {i} is dark a good part of the time ({dark})");
        }
        assert!(firefly_colour(1.0)[0] > firefly_colour(0.0)[0]);
    }

    #[test]
    fn snow_falls_and_the_nearer_flakes_fall_faster_and_bigger() {
        for i in 0..FLAKES {
            let mut t = 0.0;
            while t < 60.0 {
                let f = flake(i, t);
                assert!(f.x > -0.1 && f.x < 1.15 && f.y >= -0.05 && f.y <= 1.05 && f.size >= 1.0 && f.size <= 3.7, "{f:?}");
                t += 1.0;
            }
        }
        let (far, near) = ((0..FLAKES).min_by(|a, b| flake(*a, 0.0).size.total_cmp(&flake(*b, 0.0).size)).unwrap(), (0..FLAKES).max_by(|a, b| flake(*a, 0.0).size.total_cmp(&flake(*b, 0.0).size)).unwrap());
        let drop_in = |i: u32| (flake(i, 0.5).y - flake(i, 0.0).y + 1.08).rem_euclid(1.08);
        assert!(drop_in(near) > drop_in(far));
        assert!(flake(near, 0.0).alpha > flake(far, 0.0).alpha);
    }

    #[test]
    fn rain_slants_and_lightning_flashes_once_in_a_while_with_a_bolt() {
        for i in 0..DROPS {
            let d = drop(i, 3.0);
            assert!(d.x > -0.15 && d.x < 1.15 && d.y >= -0.05 && d.y <= 1.05 && d.len >= 10.0 && d.len <= 24.0, "{d:?}");
        }
        // across several minutes: some quiet stretches, mostly flashes, never above 1
        let mut flashes = 0;
        let mut quiet = 0;
        for epoch in 0..60u32 {
            let mut peak = 0.0f32;
            let mut t = epoch as f32 * FLASH_EVERY;
            while t < (epoch + 1) as f32 * FLASH_EVERY {
                let f = flash(t);
                assert!((0.0..=1.0).contains(&f));
                peak = peak.max(f);
                t += 0.02;
            }
            if peak > 0.5 {
                flashes += 1;
            } else {
                quiet += 1;
            }
        }
        assert!(flashes > 35 && quiet > 3, "{flashes} flashes, {quiet} quiet");
        assert_eq!(flash(-5.0), flash(0.0));
        let b = bolt(20.0);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].len(), 10);
        assert!(b[0].windows(2).all(|w| w[1].1 > w[0].1), "it only goes down");
        assert!(b.iter().flatten().all(|p| p.0 > 0.0 && p.0 < 1.0 && p.1 >= 0.0 && p.1 < 0.7));
        assert_eq!(bolt(20.0), bolt(21.0), "one bolt for the whole stretch");
    }

    #[test]
    fn stars_stay_in_the_window_twinkle_and_join_up_when_close() {
        for i in 0..STARS {
            let mut t = 0.0;
            while t < 120.0 {
                let s = star(i, t);
                assert!(s.x > 0.0 && s.x < 1.0 && s.y > 0.0 && s.y < 1.0, "{s:?}");
                assert!((0.35..=1.0).contains(&s.glow) && s.size >= 0.9, "{s:?}");
                t += 3.0;
            }
        }
        let (a, b) = (Star { x: 0.5, y: 0.5, glow: 1.0, size: 1.0 }, Star { x: 0.52, y: 0.5, glow: 1.0, size: 1.0 });
        assert!(link(a, a, 1.6) > 0.99 && link(a, b, 1.6) > 0.0 && link(a, b, 1.6) < 1.0);
        assert_eq!(link(a, Star { x: 0.9, y: 0.9, ..b }, 1.6), 0.0, "far apart: no line");
        assert!(link(a, b, 2.0) < link(a, b, 1.0), "the same gap is wider in a wider window");
        let pairs = (0..STARS).flat_map(|i| (i + 1..STARS).map(move |j| (i, j))).filter(|(i, j)| link(star(*i, 0.0), star(*j, 0.0), 1.6) > 0.0).count();
        assert!(pairs > 15 && pairs < 300, "{pairs} lines");
    }

    #[test]
    fn beams_lean_and_swell_and_dust_rises_through_them() {
        for i in 0..BEAMS {
            let mut t = 0.0;
            let (mut low, mut high) = (1.0f32, 0.0f32);
            while t < 200.0 {
                let b = beam(i, t);
                assert!(b.x > -0.25 && b.x < 1.1 && b.width >= 0.05 && b.width <= 0.14, "{b:?}");
                assert!((0.04..=0.14).contains(&b.alpha), "{b:?}");
                low = low.min(b.alpha);
                high = high.max(b.alpha);
                t += 1.0;
            }
            assert!(high - low > 0.04, "beam {i} swells and fades");
        }
        for i in 0..MOTES {
            let mut t = 0.0;
            while t < 160.0 {
                let m = mote(i, t);
                assert!(m.x > -0.05 && m.x < 1.05 && m.y > -0.1 && m.y < 1.06, "{m:?}");
                assert!((0.0..=1.0).contains(&m.alpha) && m.size >= 0.8, "{m:?}");
                t += 2.0;
            }
            assert!(mote(i, 1.0).y < mote(i, 0.0).y || mote(i, 1.0).y > 0.9, "dust goes up (or has just come round to the bottom)");
        }
    }
}
