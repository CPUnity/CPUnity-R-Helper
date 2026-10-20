//! Father Time, the animated backdrop: where the picture goes, and everything in it that moves.
//!
//! The picture (`assets/backdrop-father-*.png`, drawn by tools/father_time.py) is 1200 x 1600
//! pixels. This file only works out positions in those picture pixels, from the time, so it can be
//! tested without a window; `ui/backdrop.rs` turns them into drawing. Nothing here touches the
//! screen.

/// Size of the picture in pixels.
pub const PICTURE: (f32, f32) = (1200.0, 1600.0);
/// The stage is the part of the picture shown above the tabs, where the logo would be: the clock,
/// the head and the beard. These are the picture rows it runs from and to.
pub const STAGE_TOP: f32 = 85.0;
pub const STAGE_BOTTOM: f32 = 1110.0;

/// How big the picture is drawn (points per picture pixel) in a panel `width` points wide.
pub fn scale_for(width: f32) -> f32 {
    (width / PICTURE.0).clamp(0.30, 0.55)
}

/// How big the picture is drawn when the stage has to fit a window `width` by `height` points
/// as large as it will go (points per picture pixel): never below 0.2, so a tiny window still
/// shows something.
pub fn fit_scale(width: f32, height: f32) -> f32 {
    let wide = width / PICTURE.0;
    let tall = height / (STAGE_BOTTOM - STAGE_TOP);
    wide.min(tall).max(0.2)
}

/// How long the program's window takes to stretch to the game and back, in seconds.
pub const WINDOW_STRETCH_LEN: f64 = 0.4;

/// The size (points) the program's window stretches to for the game on a monitor `monitor` points
/// across: wide, so the stage can be big, but never more than the monitor has room for.
pub fn game_window_size(monitor: (f32, f32)) -> (f32, f32) {
    let w = (monitor.0 * 0.78).clamp(900.0, 1500.0).min((monitor.0 - 40.0).max(500.0));
    let h = (monitor.1 * 0.86).clamp(640.0, 1080.0).min((monitor.1 - 80.0).max(420.0));
    (w, h)
}

/// A smooth 0..1 for a stretch that is `t` of the way through (slow, quick, slow).
pub fn ease(t: f64) -> f32 {
    let t = t.clamp(0.0, 1.0) as f32;
    t * t * (3.0 - 2.0 * t)
}

/// Height in points of the stage.
pub fn stage_height(scale: f32) -> f32 {
    (STAGE_BOTTOM - STAGE_TOP) * scale
}

/// Where the picture's top left corner goes, for a stage whose top left is `stage_min` and that is
/// `width` points wide: centred, with the unused top rows of the picture above the stage.
pub fn origin(stage_min: (f32, f32), width: f32, scale: f32) -> (f32, f32) {
    (stage_min.0 + (width - PICTURE.0 * scale) / 2.0, stage_min.1 - STAGE_TOP * scale)
}

/// A number from 0 to 1 that depends only on the three inputs.
fn hash01(a: u32, b: u32, c: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B) ^ c.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Whether a point is inside a polygon (any shape).
pub fn inside(p: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let mut in_ = false;
    let mut j = polygon.len().wrapping_sub(1);
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[j]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            in_ = !in_;
        }
        j = i;
    }
    in_
}

/// Something small that is drawn as a dot: where, how big (picture pixels) and how bright (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dot {
    pub pos: (f32, f32),
    pub radius: f32,
    pub strength: f32,
}

// ---- digits running down the wing -------------------------------------------------------------

/// The wing's membrane, in picture pixels.
pub const WING: [(f32, f32); 10] = [
    (950.0, 800.0),
    (1165.0, 600.0),
    (1122.0, 718.0),
    (1175.0, 790.0),
    (1102.0, 849.0),
    (1115.0, 935.0),
    (1032.0, 938.0),
    (995.0, 1020.0),
    (899.0, 1022.0),
    (775.0, 1150.0),
];

/// One digit of the rain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Digit {
    pub pos: (f32, f32),
    pub ch: char,
    /// 1 for the lowest digit of a stream, fading to about 0.17 up the tail.
    pub strength: f32,
}

const ROW: f32 = 24.0;
const COLUMN: f32 = 20.0;

/// The digits on the wing `t` seconds in: columns of streams that slide down, each column at its
/// own speed, the lowest digit of a stream the brightest, the digits changing as they fall.
pub fn rain(t: f32) -> Vec<Digit> {
    let xs = WING.iter().map(|p| p.0);
    let ys = WING.iter().map(|p| p.1);
    let (x0, x1) = (xs.clone().fold(f32::MAX, f32::min), xs.fold(f32::MIN, f32::max));
    let (y0, y1) = (ys.clone().fold(f32::MAX, f32::min), ys.fold(f32::MIN, f32::max));
    let rows = ((y1 - y0) / ROW).ceil() as i64 + 2;
    let mut out = Vec::new();
    let mut column = 0u32;
    let mut x = x0 + 6.0;
    while x < x1 {
        let speed = 70.0 + 70.0 * hash01(column, 1, 0);
        let len = 8 + (hash01(column, 2, 0) * 9.0) as i64; // 8 to 16 digits
        let gap = 1 + (hash01(column, 3, 0) * 4.0) as i64; // then 1 to 4 empty rows
        let cycle = len + gap;
        let shift = hash01(column, 4, 0) * cycle as f32 * ROW + speed * t.max(0.0);
        let steps = (shift / ROW).floor();
        let frac = shift - steps * ROW;
        let steps = steps as i64;
        for row in 0..rows {
            let id = row - steps; // the digit travels with its id
            let index = id.rem_euclid(cycle);
            if index >= len {
                continue;
            }
            let pos = (x, y0 + row as f32 * ROW + frac);
            if !inside(pos, &WING) {
                continue;
            }
            let from_head = (len - 1 - index) as f32;
            let strength = if index == len - 1 {
                1.0
            } else {
                0.17 + 0.75 * (1.0 - from_head / len as f32).powf(1.4)
            };
            let flicker = (t * 2.0).max(0.0) as u32;
            let digit = (hash01(column, id as i32 as u32, flicker) * 10.0) as u8 % 10;
            out.push(Digit { pos, ch: (b'0' + digit) as char, strength });
        }
        column += 1;
        x += COLUMN;
    }
    out
}

// ---- the hourglass on the staff ----------------------------------------------------------------

/// Middle of the hourglass.
pub const HOURGLASS: (f32, f32) = (212.0, 300.0);

fn ring_point(rx: f32, ry: f32, tilt: f32, degrees: f32) -> (f32, f32) {
    let (sd, cd) = degrees.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    let (x, y) = (rx * cd, ry * sd);
    (HOURGLASS.0 + x * ct - y * st, HOURGLASS.1 + x * st + y * ct)
}

/// Two glowing orbs circling the hourglass on its rings, each with a short fading trail.
pub fn orbs(t: f32) -> Vec<Dot> {
    let mut out = Vec::new();
    for (tilt, speed, start) in [(-22.0_f32, 70.0_f32, 40.0_f32), (38.0, -50.0, 215.0)] {
        for k in 0..5 {
            let degrees = start + speed * t - speed.signum() * 9.0 * k as f32;
            out.push(Dot {
                pos: ring_point(104.0, 30.0, tilt, degrees),
                radius: 5.0 - k as f32 * 0.7,
                strength: 1.0 - k as f32 * 0.2,
            });
        }
    }
    out
}

/// The sand falling through the neck of the hourglass: short dashes that run down and start again.
pub fn sand(t: f32) -> Vec<Dot> {
    (0..6)
        .map(|k| {
            let run = (t * 64.0 + k as f32 * 9.0).rem_euclid(54.0);
            Dot { pos: (HOURGLASS.0, HOURGLASS.1 - 8.0 + run), radius: 1.7, strength: 1.0 }
        })
        .collect()
}

/// Motes of time spiralling up out of the glass and fading.
pub fn motes(t: f32) -> Vec<Dot> {
    (0..16)
        .map(|k| {
            let u = (t * 0.12 + k as f32 / 16.0).rem_euclid(1.0);
            let x = HOURGLASS.0 + 40.0 * (u * 11.0 + k as f32).sin() * (0.3 + u);
            let y = HOURGLASS.1 - 90.0 - u * 150.0;
            Dot { pos: (x, y), radius: 3.2 * (1.0 - u) + 0.9, strength: (1.0 - u).powf(1.2) }
        })
        .collect()
}

// ---- the beard's sand ---------------------------------------------------------------------------

/// Where the strands of the beard end and turn to sand.
pub const BEARD_TIPS: [(f32, f32); 3] = [(600.0, 1092.0), (574.0, 1030.0), (626.0, 1030.0)];

/// Grains of sand drifting down from the beard. `presence` is how much beard there is (0 to 1).
pub fn grains(t: f32, presence: f32) -> Vec<Dot> {
    let mut out = Vec::new();
    for (e, tip) in BEARD_TIPS.iter().enumerate() {
        for k in 0..10u32 {
            let reach = hash01(e as u32, k, 1);
            let fall = hash01(e as u32, k, 2);
            let u = (t * 0.22 + k as f32 / 10.0 + e as f32 * 0.31).rem_euclid(1.0);
            out.push(Dot {
                pos: (tip.0 + 8.0 + u * (40.0 + 80.0 * reach), tip.1 + 6.0 + u * (60.0 + 110.0 * fall)),
                radius: 3.2 * (1.0 - u) + 0.9,
                strength: (1.0 - u).powf(1.3) * presence.clamp(0.0, 1.0),
            });
        }
    }
    out
}

// ---- the eyes ---------------------------------------------------------------------------------

/// The right eye (the left one is its mirror image about x = 600).
pub const EYE: [(f32, f32); 5] =
    [(630.0, 552.0), (668.0, 524.0), (728.0, 534.0), (764.0, 562.0), (712.0, 574.0)];

/// How shut the eyes are (0 open, 1 closed): a quick blink every five seconds or so.
pub fn blink(t: f32) -> f32 {
    let p = t.rem_euclid(5.3);
    if p < 0.2 {
        1.0 - (p / 0.1 - 1.0).abs()
    } else {
        0.0
    }
}

// ---- the beard is clicked ----------------------------------------------------------------------

/// A box around the beard in picture pixels (top left, bottom right): what can be clicked.
pub const BEARD_BOX: ((f32, f32), (f32, f32)) = ((520.0, 764.0), (682.0, 1100.0));
/// The beard's outline, a little generous, so it can be painted over and turned into pixels.
pub const BEARD: [(f32, f32); 9] = [
    (528.0, 764.0),
    (672.0, 764.0),
    (682.0, 850.0),
    (664.0, 940.0),
    (632.0, 1010.0),
    (602.0, 1098.0),
    (572.0, 1010.0),
    (536.0, 940.0),
    (518.0, 850.0),
];
/// How long the whole reaction lasts, in seconds. A click while it runs is ignored.
pub const REACTION: f32 = 6.5;
/// How far the jaw drops, in picture pixels.
pub const JAW_DROP: f32 = 56.0;
const BIT: f32 = 9.0;

/// A piece of the beard that has turned into a pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bit {
    pub pos: (f32, f32),
    pub size: f32,
    pub strength: f32,
}

/// What Father Time is doing `since` seconds after his beard was clicked.
#[derive(Debug, Clone, PartialEq)]
pub struct Reaction {
    /// How much of the beard is painted over (1: gone, 0: back).
    pub beard_gone: f32,
    /// How far the jaw hangs open, in picture pixels.
    pub jaw: f32,
    /// How visible the "How could you?" is (0..1).
    pub speech: f32,
    pub bits: Vec<Bit>,
}

fn ease_out(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

fn ease_in_out(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The jaw: drops fast, hangs there with a tremble, then closes.
pub fn jaw(since: f32) -> f32 {
    if since < 0.0 || since >= REACTION {
        0.0
    } else if since < 0.35 {
        JAW_DROP * ease_out(since / 0.35)
    } else if since < 3.2 {
        JAW_DROP + 2.0 * (since * 28.0).sin()
    } else if since < 3.8 {
        JAW_DROP * (1.0 - ease_in_out((since - 3.2) / 0.6))
    } else {
        0.0
    }
}

/// Everything about the reaction, or `None` before the click and after it is over.
pub fn reaction(since: f32) -> Option<Reaction> {
    if !(0.0..REACTION).contains(&since) {
        return None;
    }
    let beard_gone = if since < 4.2 { 1.0 } else { (1.0 - (since - 4.2) / 2.0).clamp(0.0, 1.0) };
    let speech = ((since - 0.5) / 0.2).min((3.4 - since) / 0.3).clamp(0.0, 1.0);
    let mut bits = Vec::new();
    if since < 3.0 {
        let (min_x, max_x) = (BEARD_BOX.0 .0, BEARD_BOX.1 .0);
        let (min_y, max_y) = (BEARD_BOX.0 .1, BEARD_BOX.1 .1);
        let mut cy = min_y;
        let mut row = 0u32;
        while cy < max_y {
            let mut cx = min_x;
            let mut col = 0u32;
            while cx < max_x {
                let origin = (cx + BIT / 2.0, cy + BIT / 2.0);
                if inside(origin, &BEARD) {
                    let delay = hash01(col, row, 7) * 0.12;
                    let age = (since - delay).max(0.0);
                    let life = 1.6 + hash01(col, row, 8) * 0.9;
                    if age < life {
                        let vx = (origin.0 - 600.0) * 1.1 + (hash01(col, row, 9) - 0.5) * 110.0;
                        let vy = -50.0 - hash01(col, row, 10) * 150.0;
                        let pos = (origin.0 + vx * age, origin.1 + vy * age + 0.5 * 320.0 * age * age);
                        let k = age / life;
                        bits.push(Bit { pos, size: BIT * (1.0 - 0.55 * k), strength: 1.0 - k });
                    }
                }
                cx += BIT;
                col += 1;
            }
            cy += BIT;
            row += 1;
        }
    }
    Some(Reaction { beard_gone, jaw: jaw(since), speech, bits })
}

// ---- the jaw ----------------------------------------------------------------------------------

/// How a shape of the open mouth is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    None,
    /// The background colour: hides the picture underneath.
    Background,
    /// A faint wash of the accent colour.
    Dim,
    /// A strong wash of the accent colour (teeth).
    Bright,
}

/// A shape in picture pixels. `line` is how strong its outline is (0 for none).
#[derive(Debug, Clone, PartialEq)]
pub struct Poly {
    pub points: Vec<(f32, f32)>,
    pub closed: bool,
    pub fill: Fill,
    pub line: f32,
}

fn mirrored(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    points.iter().map(|p| (1200.0 - p.0, p.1)).collect()
}

fn moved(points: &[(f32, f32)], dy: f32) -> Vec<(f32, f32)> {
    points.iter().map(|p| (p.0, p.1 + dy)).collect()
}

/// The lower face with the jaw `drop` picture pixels open. The picture has the closed jaw in it,
/// so this first paints over it, then draws the mouth, the teeth and the jaw lower down. Empty
/// when the jaw is shut.
pub fn face(drop: f32) -> Vec<Poly> {
    if drop < 0.5 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut add = |points: Vec<(f32, f32)>, closed: bool, fill: Fill, line: f32| {
        out.push(Poly { points, closed, fill, line });
    };
    // paint over the closed jaw (the part of the face below the cheeks)
    add(vec![(462.0, 698.0), (738.0, 698.0), (680.0, 786.0), (600.0, 820.0), (520.0, 786.0)], true, Fill::Background, 0.0);
    // the inside of the mouth, between the upper lip and the lower one that has dropped
    add(vec![(524.0, 738.0), (676.0, 738.0), (680.0, 744.0 + drop), (600.0, 772.0 + drop), (520.0, 744.0 + drop)], true, Fill::Dim, 0.0);
    // the snout's end and the upper lip
    add(vec![(536.0, 700.0), (550.0, 712.0), (600.0, 736.0), (650.0, 712.0), (664.0, 700.0)], false, Fill::None, 0.7);
    add(vec![(524.0, 738.0), (600.0, 730.0), (676.0, 738.0)], false, Fill::None, 0.8);
    // the upper fangs
    for fang in [[(644.0, 706.0), (656.0, 774.0), (670.0, 712.0)], [(690.0, 692.0), (706.0, 748.0), (716.0, 686.0)]] {
        add(fang.to_vec(), true, Fill::Bright, 1.0);
        add(mirrored(&fang), true, Fill::Bright, 1.0);
    }
    // the lower jaw, hanging open
    let half = [(600.0, 772.0), (680.0, 744.0), (738.0, 704.0), (676.0, 784.0), (600.0, 816.0)];
    add(moved(&half, drop), true, Fill::Background, 0.0);
    add(moved(&mirrored(&half), drop), true, Fill::Background, 0.0);
    add(moved(&[(520.0, 744.0), (600.0, 772.0), (680.0, 744.0)], drop), false, Fill::None, 0.9);
    add(moved(&[(738.0, 704.0), (676.0, 784.0), (600.0, 816.0), (524.0, 784.0), (462.0, 704.0)], drop), false, Fill::None, 0.9);
    // the cheeks follow the jaw down
    add(vec![(736.0, 700.0), (738.0, 704.0 + drop)], false, Fill::None, 0.7);
    add(vec![(464.0, 700.0), (462.0, 704.0 + drop)], false, Fill::None, 0.7);
    // lower teeth, pointing up from the lip
    for x in [548.0_f32, 574.0, 626.0, 652.0] {
        let lip = 772.0 - 0.35 * (x - 600.0).abs();
        add(moved(&[(x - 5.0, lip), (x, lip - 16.0), (x + 5.0, lip)], drop), true, Fill::Bright, 1.0);
    }
    out
}

/// Where the words go when he says "How could you?" (centre, picture pixels).
pub const SPEECH_AT: (f32, f32) = (600.0, 955.0);

// ---- Father Time in The Void ----------------------------------------------------------------------
//
// In The Void he has no staff (you have it), the clock behind him spins by itself, far too fast and
// in no way in step with your computer, he falls apart into pixels and torn strips all the time, and
// when he is clicked he speaks, in the Void's signs.

/// The three things he says when he is clicked, one of them each time, in this order.
pub const VOID_LINES: [&str; 3] = [
    "We're distant, but spirit I can feel.",
    "So you ended Heaven and started The Void? At least I quit smoking because of you.",
    "The chosen shall fall when the moon does, that is the way",
];

/// Which of his lines comes next: the first one to begin with, and then each after the one he just
/// said (`last`) in order, round to the first again.
pub fn void_next(last: Option<usize>) -> usize {
    last.map_or(0, |k| (k + 1) % VOID_LINES.len())
}

/// Signs appear this fast (a second), after a short wait, stay up a while, and fade.
pub const SPEECH_CPS: f32 = 16.0;
const SPEECH_WAIT: f32 = 0.3;
const SPEECH_HOLD: f32 = 12.0;
const SPEECH_FADE: f32 = 0.8;

/// How long a line of `chars` letters is up, in seconds.
pub fn void_speech_len(chars: usize) -> f32 {
    SPEECH_WAIT + chars as f32 / SPEECH_CPS + SPEECH_HOLD + SPEECH_FADE
}

/// How much of his line is showing `since` seconds after the click: how many letters, and how
/// visible. None before the click and after the line is gone.
pub fn void_speech(since: f32, chars: usize) -> Option<(usize, f32)> {
    if since < 0.0 || since >= void_speech_len(chars) {
        return None;
    }
    let shown = (((since - SPEECH_WAIT) * SPEECH_CPS).max(0.0) as usize).min(chars);
    let end = void_speech_len(chars);
    let alpha = (since / 0.15).min(1.0).min((end - since) / SPEECH_FADE).clamp(0.0, 1.0);
    Some((shown, alpha))
}

/// A hand that turns in jerks: for `SPIN_WINDOW` seconds at a time it runs at some speed between
/// `lo` and `hi` degrees a second, now and then backwards, now and then not at all. Degrees, 0 to 360.
const SPIN_WINDOW: f32 = 0.4;

fn spin_speed(window: u32, salt: u32, lo: f32, hi: f32) -> f32 {
    let pick = hash01(window, salt, 2);
    let dir = if pick < 0.12 {
        -1.0
    } else if pick < 0.20 {
        0.0
    } else {
        1.0
    };
    dir * (lo + (hi - lo) * hash01(window, salt, 1))
}

fn spin(t: f32, salt: u32, lo: f32, hi: f32) -> f32 {
    let k = (t / SPIN_WINDOW) as u32;
    let mut angle = 360.0 * hash01(salt, 77, 3);
    for j in 0..k {
        angle += spin_speed(j, salt, lo, hi) * SPIN_WINDOW;
    }
    angle += spin_speed(k, salt, lo, hi) * (t - k as f32 * SPIN_WINDOW);
    angle.rem_euclid(360.0)
}

/// The hour, minute and second hand (degrees clockwise from 12) of the clock behind him, `t` seconds
/// in. It does not know what time it is: it only ever spins.
pub fn void_hands(t: f32) -> (f32, f32, f32) {
    let t = t.max(0.0).rem_euclid(48.0);
    (spin(t, 1, 40.0, 110.0), spin(t, 2, 240.0, 520.0), spin(t, 3, 500.0, 1100.0))
}

/// How hard he is glitching (0 to 1): always, and badly.
pub const VOID_GLITCH: f32 = 0.85;

/// The glitch of his picture at time `t`: the torn strips and the pixelated frames (see `gold.rs`).
pub fn void_glitch(t: f32) -> crate::gold::GlitchFrame {
    crate::gold::glitch_frame_with(t, VOID_GLITCH)
}

/// How far (picture pixels) a cyan and a magenta copy of him sit to each side of him; they jump from
/// one glitch picture to the next.
pub fn void_ghosts(t: f32) -> (f32, f32) {
    let frame = (t.max(0.0) * 14.0) as u32;
    (-6.0 - 34.0 * hash01(frame, 31, 1), 6.0 + 34.0 * hash01(frame, 32, 1))
}

/// The picture made of big blocks, for an image with transparent parts: every `block` x `block`
/// square of the RGBA image becomes one pixel, with the average opacity and the average colour of
/// what is visible in it (so transparent black does not darken the edges). Returns (width, height,
/// RGBA bytes).
pub fn pixelate_alpha(width: usize, height: usize, rgba: &[u8], block: usize) -> (usize, usize, Vec<u8>) {
    let block = block.max(1);
    let (w, h) = (width.div_ceil(block), height.div_ceil(block));
    let mut out = vec![0u8; w * h * 4];
    for by in 0..h {
        for bx in 0..w {
            let (mut rgb, mut a_sum, mut n) = ([0.0_f32; 3], 0.0_f32, 0.0_f32);
            for y in by * block..((by + 1) * block).min(height) {
                for x in bx * block..((bx + 1) * block).min(width) {
                    let i = (y * width + x) * 4;
                    let a = rgba[i + 3] as f32 / 255.0;
                    for c in 0..3 {
                        rgb[c] += rgba[i + c] as f32 * a;
                    }
                    a_sum += a;
                    n += 1.0;
                }
            }
            let o = (by * w + bx) * 4;
            if a_sum > 0.0 && n > 0.0 {
                for c in 0..3 {
                    out[o + c] = (rgb[c] / a_sum).round().clamp(0.0, 255.0) as u8;
                }
                out[o + 3] = (a_sum / n * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    (w, h, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_window_fits_the_stage_as_big_as_it_will_go() {
        // a wide window is limited by its height, a tall one by its width
        let s = fit_scale(2400.0, 1025.0);
        assert!((s - 1.0).abs() < 1e-4);
        assert!((stage_height(s) - 1025.0).abs() < 0.01);
        let s = fit_scale(600.0, 5000.0);
        assert!((s - 0.5).abs() < 1e-4);
        // it fits: never wider or taller than the window
        for (w, h) in [(1100.0, 940.0), (800.0, 600.0), (1920.0, 1080.0), (560.0, 480.0)] {
            let s = fit_scale(w, h);
            assert!(PICTURE.0 * s <= w + 0.01 || s == 0.2);
            assert!(stage_height(s) <= h + 0.01 || s == 0.2);
        }
        // nothing to show in is still a size
        assert_eq!(fit_scale(0.0, 0.0), 0.2);
        assert_eq!(fit_scale(-5.0, 100.0), 0.2);
    }

    #[test]
    fn the_program_window_stretches_wide_for_the_game_within_the_monitor() {
        let (w, h) = game_window_size((1920.0, 1080.0));
        assert!(w > 570.0 * 2.0 && w <= 1500.0 && h <= 1080.0 - 80.0 + 0.01, "{w} x {h}");
        // a small screen: it still fits, with room for the title bar and taskbar
        let (w, h) = game_window_size((1366.0, 768.0));
        assert!(w <= 1366.0 - 40.0 && h <= 768.0 - 80.0 + 0.01 && w >= 900.0 && h >= 640.0 - 0.01, "{w} x {h}");
        // a huge one is capped
        let (w, h) = game_window_size((7680.0, 4320.0));
        assert_eq!((w, h), (1500.0, 1080.0));
        // and the game fits in what it is given
        assert!(fit_scale(w, h - 40.0) > 0.9);
        // the stretch eases from 0 to 1 and stays in range
        assert_eq!((ease(-1.0), ease(0.0), ease(1.0), ease(9.0)), (0.0, 0.0, 1.0, 1.0));
        assert!((ease(0.5) - 0.5).abs() < 1e-6 && ease(0.25) < 0.25 && ease(0.75) > 0.75);
    }

    #[test]
    fn the_picture_is_centred_with_the_stage_on_top() {
        let s = scale_for(570.0 - 24.0);
        assert!((0.3..=0.55).contains(&s));
        assert_eq!(scale_for(10_000.0), 0.55);
        assert_eq!(scale_for(10.0), 0.30);
        let (ox, oy) = origin((12.0, 100.0), 546.0, s);
        // the middle of the picture is the middle of the stage
        assert!((ox + 600.0 * s - (12.0 + 273.0)).abs() < 0.01);
        // the first stage row is at the top of the stage
        assert!((oy + STAGE_TOP * s - 100.0).abs() < 0.01);
        assert!(stage_height(s) > 200.0 && stage_height(s) < 600.0);
        // the clock, its head and the beard are all inside the stage
        let clock = crate::info::CLOCK_CENTER;
        assert!(clock.1 - 440.0 >= STAGE_TOP && clock.1 + 440.0 <= STAGE_BOTTOM + 5.0);
        assert!(BEARD_BOX.1 .1 <= STAGE_BOTTOM);
    }

    #[test]
    fn polygon_test_works_for_concave_shapes() {
        let l = [(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (4.0, 4.0), (4.0, 10.0), (0.0, 10.0)];
        assert!(inside((2.0, 2.0), &l) && inside((8.0, 2.0), &l) && inside((2.0, 8.0), &l));
        assert!(!inside((8.0, 8.0), &l) && !inside((-1.0, 5.0), &l));
        assert!(!inside((1.0, 1.0), &[]));
    }

    #[test]
    fn the_rain_stays_on_the_wing_and_runs_down() {
        let a = rain(0.0);
        assert!(a.len() > 100, "{} digits", a.len());
        assert!(a.iter().all(|d| inside(d.pos, &WING) && d.ch.is_ascii_digit()));
        assert!(a.iter().all(|d| (0.1..=1.0).contains(&d.strength)));
        assert!(a.iter().any(|d| d.strength == 1.0), "streams have bright heads");
        // a moment later every column has moved down: its digits sit a little lower (at most 8
        // points in a twentieth of a second, so a shift of "almost a whole row" would be upward)
        let b = rain(0.05);
        for d in a.iter().take(40) {
            let same_column: Vec<&Digit> = b.iter().filter(|e| e.pos.0 == d.pos.0).collect();
            assert!(!same_column.is_empty());
            let shift = (same_column[0].pos.1 - d.pos.1).rem_euclid(ROW);
            assert!(shift > 0.5 && shift < 8.1, "column {} moved {}", d.pos.0, shift);
        }
        // deterministic: the same moment gives the same picture
        assert_eq!(rain(1.7), rain(1.7));
        assert_ne!(rain(1.7), rain(2.9));
    }

    #[test]
    fn the_hourglass_things_stay_near_the_hourglass() {
        for t in [0.0, 1.3, 7.7, 50.0] {
            for d in orbs(t) {
                assert!((d.pos.0 - HOURGLASS.0).abs() <= 110.0 && (d.pos.1 - HOURGLASS.1).abs() <= 110.0);
                assert!(d.radius > 0.0 && (0.0..=1.0).contains(&d.strength));
            }
            for d in sand(t) {
                assert!(d.pos.1 >= HOURGLASS.1 - 8.0 && d.pos.1 <= HOURGLASS.1 + 46.0);
            }
            for d in motes(t) {
                assert!(d.pos.1 <= HOURGLASS.1 - 90.0 && d.pos.1 >= HOURGLASS.1 - 240.0);
                assert!(d.radius > 0.0 && (0.0..=1.0).contains(&d.strength));
            }
        }
        assert_eq!(orbs(0.0).len(), 10);
        assert_ne!(orbs(0.0), orbs(0.5));
    }

    #[test]
    fn the_beards_sand_only_falls_while_there_is_a_beard() {
        assert!(grains(1.0, 1.0).iter().any(|d| d.strength > 0.3));
        assert!(grains(1.0, 0.0).iter().all(|d| d.strength == 0.0));
        assert!(grains(1.0, 1.0).iter().all(|d| d.pos.1 >= 1030.0));
    }

    #[test]
    fn he_blinks_briefly_and_not_all_the_time() {
        assert_eq!(blink(0.0), 0.0);
        assert!((blink(0.1) - 1.0).abs() < 0.001);
        assert_eq!(blink(2.0), 0.0);
        let shut = (0..1000).filter(|i| blink(*i as f32 * 0.01) > 0.0).count();
        assert!(shut < 100, "eyes are open most of the time");
        // the eye shape really is an eye: convex, and on the right of the middle
        assert!(EYE.iter().all(|p| p.0 > 600.0));
    }

    #[test]
    fn the_jaw_drops_hangs_and_closes() {
        assert_eq!(jaw(-1.0), 0.0);
        assert_eq!(jaw(0.0), 0.0);
        assert!(jaw(0.17) > 0.0 && jaw(0.17) < JAW_DROP);
        assert!((jaw(1.0) - JAW_DROP).abs() <= 2.01, "hangs open");
        assert!(jaw(3.5) > 0.0 && jaw(3.5) < JAW_DROP, "closing");
        assert_eq!(jaw(4.0), 0.0);
        assert_eq!(jaw(REACTION + 1.0), 0.0);
    }

    #[test]
    fn the_reaction_runs_its_course() {
        assert!(reaction(-0.1).is_none());
        assert!(reaction(REACTION).is_none());
        let start = reaction(0.0).unwrap();
        assert_eq!(start.beard_gone, 1.0, "the beard is gone at once");
        assert!(start.bits.len() > 150, "{} pixels", start.bits.len());
        assert!(start.bits.iter().all(|b| inside(b.pos, &BEARD)), "at first the pixels are the beard");
        let mid = reaction(1.0).unwrap();
        assert!(mid.speech > 0.99 && mid.jaw > 40.0);
        // pixels scatter, fall and fade
        assert!(mid.bits.iter().all(|b| b.strength >= 0.0 && b.strength <= 1.0 && b.size > 0.0));
        assert!(mid.bits.iter().any(|b| b.pos.1 > 1100.0 || b.pos.0 < 480.0 || b.pos.0 > 720.0));
        assert!(reaction(3.1).unwrap().bits.is_empty(), "all the pixels are gone");
        // the jaw is shut again before the beard comes back
        let back = reaction(5.2).unwrap();
        assert!(back.jaw == 0.0 && back.beard_gone > 0.0 && back.beard_gone < 1.0);
        assert_eq!(reaction(REACTION - 0.01).unwrap().beard_gone, 0.0);
        assert_eq!(reaction(2.0), reaction(2.0));
    }

    #[test]
    fn the_open_mouth_follows_the_jaw() {
        assert!(face(0.0).is_empty());
        let shapes = face(40.0);
        assert!(shapes.iter().all(|p| !p.points.is_empty()));
        // the first thing painted is over the old jaw, in the background colour
        assert_eq!(shapes[0].fill, Fill::Background);
        // the lowest point of the lower jaw has moved down by the drop
        let lowest = |v: &Vec<Poly>| v.iter().flat_map(|p| p.points.iter()).map(|p| p.1).fold(0.0, f32::max);
        assert!((lowest(&face(40.0)) - lowest(&face(20.0)) - 20.0).abs() < 0.01);
        // symmetric about the middle: for every point there is one on the other side
        for p in shapes.iter().flat_map(|p| p.points.iter()) {
            if (p.0 - 600.0).abs() > 1.0 {
                assert!(shapes.iter().flat_map(|q| q.points.iter()).any(|q| (q.0 - (1200.0 - p.0)).abs() < 0.01 && (q.1 - p.1).abs() < 0.01), "{:?}", p);
            }
        }
    }

    /// Writes one frame as text for tools/preview_father.py (`FT_DUMP=file FT_T=12.5 FT_SINCE=1.0
    /// cargo test dump_frame -- --ignored`), so the animation can be looked at without a window.
    #[test]
    #[ignore]
    fn dump_frame() {
        let (Ok(path), t, since) = (
            std::env::var("FT_DUMP"),
            std::env::var("FT_T").ok().and_then(|v| v.parse().ok()).unwrap_or(10.0_f32),
            std::env::var("FT_SINCE").ok().and_then(|v| v.parse().ok()).unwrap_or(-1.0_f32),
        ) else {
            return;
        };
        let mut text = String::new();
        let react = reaction(since);
        for d in rain(t) {
            text += &format!("D {} {} {} {}\n", d.pos.0, d.pos.1, d.ch, d.strength);
        }
        let presence = react.as_ref().map_or(1.0, |r| 1.0 - r.beard_gone);
        for (k, list) in [("O", orbs(t)), ("S", sand(t)), ("M", motes(t)), ("G", grains(t, presence))] {
            for d in list {
                text += &format!("{} {} {} {} {}\n", k, d.pos.0, d.pos.1, d.radius, d.strength);
            }
        }
        text += &format!("B {}\n", blink(t));
        if let Some(r) = react {
            text += &format!("R {} {} {}\n", r.beard_gone, r.jaw, r.speech);
            for b in r.bits {
                text += &format!("P {} {} {} {}\n", b.pos.0, b.pos.1, b.size, b.strength);
            }
            for p in face(r.jaw) {
                let pts: Vec<String> = p.points.iter().map(|q| format!("{},{}", q.0, q.1)).collect();
                text += &format!("F {:?} {} {} {}\n", p.fill, p.closed, p.line, pts.join(" "));
            }
        }
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn he_says_his_three_things_in_order_and_starts_again_from_the_first() {
        assert_eq!(VOID_LINES.len(), 3);
        assert_eq!(VOID_LINES[0], "We're distant, but spirit I can feel.");
        assert_eq!(VOID_LINES[1], "So you ended Heaven and started The Void? At least I quit smoking because of you.");
        assert_eq!(VOID_LINES[2], "The chosen shall fall when the moon does, that is the way");
        let mut last = None;
        let mut said = Vec::new();
        for _ in 0..7 {
            let k = void_next(last);
            said.push(k);
            last = Some(k);
        }
        assert_eq!(said, [0, 1, 2, 0, 1, 2, 0]);
        // a line number from somewhere else cannot send him off the end
        assert!(void_next(Some(99)) < 3);
    }

    #[test]
    fn his_lines_stay_up_long_enough_to_read_with_five_seconds_more_than_they_did() {
        for line in VOID_LINES {
            let n = line.chars().count();
            // typed out, then held for at least twelve seconds, then faded
            let typed = SPEECH_WAIT + n as f32 / SPEECH_CPS;
            assert!(void_speech_len(n) - typed >= 12.0, "{n}");
            assert_eq!(void_speech(typed + 11.0, n).map(|(shown, alpha)| (shown, alpha)), Some((n, 1.0)));
        }
    }

    #[test]
    fn his_line_types_out_stays_a_while_and_fades() {
        let n = VOID_LINES[1].chars().count();
        assert_eq!(void_speech(-0.1, n), None);
        let first = void_speech(0.1, n).unwrap();
        assert_eq!(first.0, 0);
        assert!(first.1 > 0.0 && first.1 < 1.0);
        // letters come in order, never go back, and all are there before it fades
        let mut last = 0;
        for k in 0..400 {
            let t = k as f32 * 0.05;
            if let Some((shown, alpha)) = void_speech(t, n) {
                assert!(shown >= last && shown <= n && (0.0..=1.0).contains(&alpha));
                last = shown;
            }
        }
        assert_eq!(last, n);
        let done = SPEECH_WAIT + n as f32 / SPEECH_CPS;
        assert_eq!(void_speech(done + 0.5, n), Some((n, 1.0)));
        let end = void_speech_len(n);
        assert!(void_speech(end - 0.2, n).unwrap().1 < 0.4);
        assert_eq!(void_speech(end, n), None);
        // the longest line is up for a good long while but not forever
        assert!(end > 10.0 && end < 20.0, "{end}");
    }

    #[test]
    fn the_clock_in_the_void_spins_fast_and_does_not_follow_the_computers_time() {
        // it knows nothing but how long it has been going: the same every time, from nothing else
        assert_eq!(void_hands(3.7), void_hands(3.7));
        // fast: the minute hand covers hundreds of degrees a second, the second hand more, the hour
        // hand the least
        let travel = |salt: u32, lo: f32, hi: f32| -> f32 {
            (0..120u32).map(|j| spin_speed(j, salt, lo, hi).abs()).sum::<f32>() / 120.0
        };
        let (h, m, s) = (travel(1, 40.0, 110.0), travel(2, 240.0, 520.0), travel(3, 500.0, 1100.0));
        assert!(h > 25.0 && m > 200.0 && s > 400.0 && h < m && m < s, "{h} {m} {s}");
        // it sometimes goes backwards and sometimes stops
        let speeds: Vec<f32> = (0..120u32).map(|j| spin_speed(j, 2, 240.0, 520.0)).collect();
        assert!(speeds.iter().any(|v| *v < 0.0) && speeds.iter().any(|v| *v == 0.0) && speeds.iter().filter(|v| **v > 0.0).count() > 70);
        // it never jumps by more than the speed allows between two close moments
        for k in 0..400 {
            let t = k as f32 * 0.037;
            let (a, b) = (void_hands(t), void_hands(t + 0.001));
            let step = |x: f32, y: f32| ((y - x + 540.0).rem_euclid(360.0) - 180.0).abs();
            if (t + 0.001).rem_euclid(48.0) > 0.001 {
                assert!(step(a.0, b.0) < 0.2 && step(a.1, b.1) < 0.6 && step(a.2, b.2) < 1.3, "t {t}: {a:?} {b:?}");
            }
        }
        // the angles are proper clock angles
        for k in 0..100 {
            let (a, b, c) = void_hands(k as f32 * 0.31);
            assert!([a, b, c].iter().all(|x| (0.0..360.0).contains(x)));
        }
        // and it is not the time the computer shows at the same moment
        let real = crate::info::hand_angles(10, 10, 30);
        assert_ne!(void_hands(0.0), (real.0, real.1, real.2));
    }

    #[test]
    fn he_is_always_glitching_with_pixelated_frames_and_torn_strips_and_the_ghosts_jump() {
        let frames: Vec<_> = (0..140).map(|k| void_glitch(k as f32 / 14.0)).collect();
        let pixel = frames.iter().filter(|f| f.block.is_some()).count();
        assert!(pixel > 30 && pixel < 100, "{pixel} pixelated pictures in ten seconds");
        assert!(frames.iter().all(|f| f.bands.len() >= 4), "torn strips in every picture");
        assert_eq!(void_glitch(1.23), void_glitch(1.23));
        // the ghosts sit one each side, well apart, and move from picture to picture
        let mut seen = std::collections::HashSet::new();
        for k in 0..60 {
            let (c, m) = void_ghosts(k as f32 / 14.0);
            assert!(c <= -6.0 && c >= -40.0 && m >= 6.0 && m <= 40.0);
            seen.insert((c * 100.0) as i32);
        }
        assert!(seen.len() > 20);
        assert_eq!(void_ghosts(0.5), void_ghosts(0.5));
    }

    #[test]
    fn pixelating_a_picture_with_transparent_parts_keeps_the_colour_of_what_is_there() {
        // 4 x 2: the left half opaque white, the right half fully transparent black
        let mut img = vec![0u8; 4 * 2 * 4];
        for y in 0..2 {
            for x in 0..2 {
                let i = (y * 4 + x) * 4;
                img[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let (w, h, out) = pixelate_alpha(4, 2, &img, 2);
        assert_eq!((w, h), (2, 1));
        assert_eq!(&out[0..4], &[255, 255, 255, 255]);
        assert_eq!(&out[4..8], &[0, 0, 0, 0]);
        // a block half covered is half opaque and still white, not grey
        let (_, _, half) = pixelate_alpha(4, 2, &img, 4);
        assert_eq!(&half[0..4], &[255, 255, 255, 128]);
        // sizes that do not divide evenly round up
        let (w, h, out) = pixelate_alpha(5, 3, &vec![200u8; 5 * 3 * 4], 2);
        assert_eq!((w, h, out.len()), (3, 2, 3 * 2 * 4));
        assert_eq!(&out[0..4], &[200, 200, 200, 200]);
    }
}
