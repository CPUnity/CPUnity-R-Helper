//! The Starlight keyboard effect, made the way Razer Chroma's own Starlight makes it (worked out
//! from a USB capture of Chroma driving a Blade 16 keyboard).
//!
//! Stars twinkle at random on the keys: about ten new ones a second, each on a key that is not
//! busy, each fading up and down in a straight line, about a second at the longest. Every star
//! has its own peak (from a dim glow to nearly full brightness) and its own speed, so some are
//! quick flashes and some are slow breaths. The color is the picked one. With a second color,
//! every star is followed on its key by the same star in the second color: it starts when the
//! first one is at its brightest and reaches three quarters of its peak (the same peak on the pad).
//!
//! This is pure arithmetic (no device, no clock): `advance` takes the seconds that have passed
//! and `grid` is the picture to paint. The keyboard animation worker (in the app) does the
//! painting. The random numbers come from a seed, so a test can replay a night sky.
use crate::keymap::{KeyGrid, COLS, ROWS};
use crate::tidal::{has_light, PAD_LEDS};

/// The one color of the capture (green was picked there).
pub const DEFAULT_COLOR: [u8; 3] = [0, 255, 0];
/// Starlight's second color when it is switched on (the capture used blue).
pub const DEFAULT_COLOR_B: [u8; 3] = [0, 0, 255];

/// How bright the second color's star gets, against the first color's, as captured: three
/// quarters on the keyboard, the same on the cooling pad's strip.
const SECOND_PEAK: f32 = 0.75;
const PAD_SECOND_PEAK: f32 = 1.0;

/// New stars per second over the whole keyboard (about 11 in the capture), and along the cooling
/// pad's strip (about 2.5: the same share of its lights twinkle at once as on the keyboard).
const SPAWN_PER_SEC: f32 = 10.0;
const PAD_SPAWN_PER_SEC: f32 = 2.4;
/// How long one star lives, in seconds (rise and fall together).
const LIFE_MIN: f32 = 0.25;
const LIFE_MAX: f32 = 1.1;
/// How much of its life a star spends getting brighter (it fades out a little slower).
const RISE_PART: f32 = 0.46;
/// A star's peak, as a part of full brightness: from a dim glow to nearly full.
const PEAK_MIN: f32 = 38.0 / 255.0;
const PEAK_MAX: f32 = 252.0 / 255.0;
/// Most stars at once (a stalled clock must not fill the keyboard).
const MAX_STARS: usize = 40;

#[derive(Debug, Clone, Copy)]
struct Star {
    /// Where it shines: `row * COLS + col` on the keyboard, the light's number on the pad.
    cell: usize,
    age: f32,
    life: f32,
    peak: f32,
}

impl Star {
    /// How long the star takes to reach its peak, and when the second color's star starts.
    fn rise(&self) -> f32 {
        self.life * RISE_PART
    }

    /// How bright the star is `age` seconds after it was born, 0.0 to its peak.
    fn level_at(&self, age: f32) -> f32 {
        if age <= 0.0 || age >= self.life {
            return 0.0;
        }
        let rise = self.rise();
        if age < rise {
            self.peak * age / rise
        } else {
            self.peak * ((self.life - age) / (self.life - rise))
        }
        .clamp(0.0, 1.0)
    }

    /// The second color's star: the same one, started when this one peaked. `peak` is how bright
    /// it gets against the first.
    fn second_level(&self, peak: f32) -> f32 {
        peak * self.level_at(self.age - self.rise())
    }

    /// The star is over for both colors.
    fn over(&self) -> bool {
        self.age >= self.rise() + self.life
    }
}

/// A small random number generator (xorshift), so the sky can be replayed from a seed.
#[derive(Debug, Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A number from 0.0 up to (not including) 1.0.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

#[derive(Debug, Clone)]
pub struct Starlight {
    stars: Vec<Star>,
    rng: Rng,
    /// The cooling pad's strip instead of the keyboard.
    pad: bool,
}

impl Starlight {
    /// A night sky over the keyboard.
    pub fn new(seed: u64) -> Self {
        Self { stars: Vec::new(), rng: Rng(seed | 1), pad: false }
    }

    /// A night sky along the cooling pad's strip.
    pub fn new_pad(seed: u64) -> Self {
        Self { stars: Vec::new(), rng: Rng(seed | 1), pad: true }
    }

    /// Lets `secs` seconds go by: stars grow older and die, and new ones are born.
    pub fn advance(&mut self, secs: f32) {
        if !secs.is_finite() || secs <= 0.0 {
            return;
        }
        // (a long stall, for example the PC sleeping, only counts as a moment)
        let secs = secs.min(1.0);
        for star in &mut self.stars {
            star.age += secs;
        }
        self.stars.retain(|star| !star.over());

        let expected = secs * if self.pad { PAD_SPAWN_PER_SEC } else { SPAWN_PER_SEC };
        let mut births = expected.floor() as usize;
        if self.rng.unit() < expected.fract() {
            births += 1;
        }
        for _ in 0..births {
            self.spawn();
        }
    }

    fn spawn(&mut self) {
        if self.stars.len() >= MAX_STARS {
            return;
        }
        // a key with a light that has no star on it yet (a few tries, then the sky is just full)
        for _ in 0..8 {
            let cell = if self.pad {
                (self.rng.next() % PAD_LEDS as u64) as usize
            } else {
                let row = (self.rng.next() % ROWS as u64) as usize;
                let col = (self.rng.next() % COLS as u64) as usize;
                if !has_light(row, col) {
                    continue;
                }
                row * COLS + col
            };
            if self.stars.iter().any(|s| s.cell == cell) {
                continue;
            }
            let star = Star {
                cell,
                age: 0.0,
                life: self.rng.between(LIFE_MIN, LIFE_MAX),
                peak: self.rng.between(PEAK_MIN, PEAK_MAX),
            };
            self.stars.push(star);
            return;
        }
    }

    /// How many stars are shining.
    pub fn count(&self) -> usize {
        self.stars.len()
    }

    /// The color a star gives its key or light now. `b` is the second color, if it is switched on.
    fn color_of(&self, star: &Star, a: [u8; 3], b: Option<[u8; 3]>) -> [u8; 3] {
        star_color(star, a, b, if self.pad { PAD_SECOND_PEAK } else { SECOND_PEAK })
    }

    /// The picture of the sky over the keyboard now.
    pub fn grid(&self, a: [u8; 3], b: Option<[u8; 3]>) -> KeyGrid {
        let mut grid = [[[0u8; 3]; COLS]; ROWS];
        for star in self.stars.iter().filter(|_| !self.pad) {
            grid[star.cell / COLS][star.cell % COLS] = self.color_of(star, a, b);
        }
        grid
    }

    /// The picture of the sky along the cooling pad's strip now.
    pub fn pad_grid(&self, a: [u8; 3], b: Option<[u8; 3]>) -> [[u8; 3]; PAD_LEDS] {
        let mut leds = [[0u8; 3]; PAD_LEDS];
        for star in self.stars.iter().filter(|_| self.pad) {
            leds[star.cell] = self.color_of(star, a, b);
        }
        leds
    }
}

/// The color a star gives its key now: the first color at the star's brightness, and the second
/// (if any) at the brightness of the star that follows it.
fn star_color(star: &Star, a: [u8; 3], b: Option<[u8; 3]>, second_peak: f32) -> [u8; 3] {
    let first = a.map(|c| (f32::from(c) * star.level_at(star.age)).round() as u8);
    let second = b.map_or([0, 0, 0], |color| {
        color.map(|c| (f32::from(c) * star.second_level(second_peak)).round() as u8)
    });
    [
        first[0].saturating_add(second[0]),
        first[1].saturating_add(second[1]),
        first[2].saturating_add(second[2]),
    ]
}

/// A number from 0.0 up to (not including) 1.0 that depends only on these three (a hash).
fn unit_hash(cell: usize, salt: u64, n: u64) -> f32 {
    let mut z = (cell as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ n.wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// One key twinkling on its own, `t` seconds in (for the keys of Custom Keys that are set to
/// Starlight). It needs no state: every key has its own beat, a chance of a star each time round,
/// and a star of its own length and peak, all worked out from the key and the time. `a` is the
/// star's color and `b` the color of the star that follows it, if there is one. Dark between stars.
pub fn lone_star(cell: usize, t: f32, a: [u8; 3], b: Option<[u8; 3]>) -> [u8; 3] {
    // the time between two chances, long enough for a star and the one that follows it
    let span = 1.7 + 2.3 * unit_hash(cell, 1, 0);
    let x = t.max(0.0) + span * unit_hash(cell, 2, 0);
    let round = (x / span).floor();
    let local = x - round * span;
    let n = round as u64;
    if unit_hash(cell, 3, n) > 0.8 {
        return [0, 0, 0];
    }
    let life = LIFE_MIN + (LIFE_MAX - LIFE_MIN) * unit_hash(cell, 4, n);
    let peak = PEAK_MIN + (PEAK_MAX - PEAK_MIN) * unit_hash(cell, 5, n);
    let room = (span - life * (1.0 + RISE_PART)).max(0.0);
    let star = Star { cell, age: local - room * unit_hash(cell, 6, n), life, peak };
    star_color(&star, a, b, SECOND_PEAK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sky(seed: u64, seconds: f32, step: f32) -> Vec<KeyGrid> {
        let mut s = Starlight::new(seed);
        let mut frames = Vec::new();
        let mut t = 0.0;
        while t < seconds {
            s.advance(step);
            frames.push(s.grid(DEFAULT_COLOR, None));
            t += step;
        }
        frames
    }

    fn lit(grid: &KeyGrid) -> usize {
        grid.iter().flatten().filter(|px| **px != [0, 0, 0]).count()
    }

    #[test]
    fn about_as_many_stars_shine_as_in_the_capture() {
        // the capture: 6 keys lit on average, never more than 15 at once
        let frames = sky(1, 120.0, 0.04);
        let mean = frames.iter().map(lit).sum::<usize>() as f32 / frames.len() as f32;
        let most = frames.iter().map(lit).max().unwrap();
        assert!((4.0..=9.0).contains(&mean), "mean {}", mean);
        assert!(most <= 20, "most {}", most);
    }

    #[test]
    fn stars_are_green_dim_to_nearly_full_and_never_on_keys_without_a_light() {
        let frames = sky(2, 60.0, 0.04);
        let mut brightest = 0u8;
        for grid in &frames {
            for (r, row) in grid.iter().enumerate() {
                for (c, px) in row.iter().enumerate() {
                    assert_eq!((px[0], px[2]), (0, 0), "only the picked color");
                    if *px != [0, 0, 0] {
                        assert!(has_light(r, c), "a star on a key with no light");
                    }
                    brightest = brightest.max(px[1]);
                }
            }
        }
        assert!((200..=252).contains(&brightest), "brightest {}", brightest);
    }

    #[test]
    fn a_star_fades_up_and_down_and_is_gone() {
        let mut s = Starlight::new(3);
        s.advance(0.1); // one star at least, soon
        let mut seen = Vec::new();
        let mut track: Option<(usize, usize)> = None;
        for _ in 0..200 {
            s.advance(0.02);
            let g = s.grid(DEFAULT_COLOR, None);
            if track.is_none() {
                track = (0..ROWS)
                    .flat_map(|r| (0..COLS).map(move |c| (r, c)))
                    .find(|(r, c)| g[*r][*c][1] > 0);
            }
            if let Some((r, c)) = track {
                seen.push(g[r][c][1]);
                if seen.len() > 3 && g[r][c][1] == 0 {
                    break;
                }
            }
        }
        let top = seen.iter().copied().max().unwrap_or(0);
        let at = seen.iter().position(|v| *v == top).unwrap();
        assert!(top >= 38 && at > 0 && at < seen.len() - 1, "{:?}", seen);
        assert!(seen.windows(2).take(at).all(|w| w[1] >= w[0]), "gets brighter first: {:?}", seen);
        assert_eq!(*seen.last().unwrap(), 0, "and goes out");
    }

    #[test]
    fn the_same_seed_is_the_same_sky_and_another_seed_is_not() {
        assert_eq!(sky(5, 5.0, 0.04), sky(5, 5.0, 0.04));
        assert_ne!(sky(5, 5.0, 0.04), sky(6, 5.0, 0.04));
    }

    #[test]
    fn with_a_second_color_every_star_is_followed_by_the_same_star_in_it() {
        // the capture: green up and down, then blue starting at the green's peak and reaching
        // three quarters of it
        let mut s = Starlight::new(11);
        let mut track: Option<(usize, usize)> = None;
        let (mut green, mut blue) = (Vec::new(), Vec::new());
        for _ in 0..400 {
            s.advance(0.02);
            let g = s.grid([0, 255, 0], Some([0, 0, 255]));
            if track.is_none() {
                track = (0..ROWS)
                    .flat_map(|r| (0..COLS).map(move |c| (r, c)))
                    .find(|(r, c)| g[*r][*c][1] > 0);
            }
            if let Some((r, c)) = track {
                green.push(g[r][c][1]);
                blue.push(g[r][c][2]);
                if green.len() > 5 && g[r][c] == [0, 0, 0] {
                    break;
                }
            }
        }
        let green_top = *green.iter().max().unwrap();
        let blue_top = *blue.iter().max().unwrap();
        assert!(green_top >= 38, "{:?}", green);
        assert!((i32::from(blue_top) - (f32::from(green_top) * 0.75) as i32).abs() <= 2);
        let green_peak = green.iter().position(|v| *v == green_top).unwrap();
        let blue_start = blue.iter().position(|v| *v > 0).unwrap();
        assert!(blue_start >= green_peak, "the blue starts when the green peaks");
        assert!(blue_start <= green_peak + 3);
        assert!(blue.iter().rposition(|v| *v > 0) > green.iter().rposition(|v| *v > 0), "the blue goes out last");
    }

    #[test]
    fn the_pad_has_about_as_many_lights_twinkling_as_in_its_capture() {
        // the capture: 1.4 of the 18 lights lit on average, never more than 6 at once
        let mut s = Starlight::new_pad(21);
        let (mut total, mut most, mut frames) = (0usize, 0usize, 0usize);
        for _ in 0..3000 {
            s.advance(0.04);
            let lit = s.pad_grid(DEFAULT_COLOR, None).iter().filter(|px| **px != [0, 0, 0]).count();
            total += lit;
            most = most.max(lit);
            frames += 1;
            assert_eq!(lit_keys(&s), 0, "nothing on the keyboard's picture");
        }
        let mean = total as f32 / frames as f32;
        assert!((0.8..=2.4).contains(&mean), "mean {}", mean);
        assert!(most <= 9, "most {}", most);
    }

    fn lit_keys(s: &Starlight) -> usize {
        lit(&s.grid(DEFAULT_COLOR, None))
    }

    #[test]
    fn on_the_pad_the_second_color_reaches_the_same_peak() {
        let mut s = Starlight::new_pad(13);
        let (mut green, mut blue) = (0u8, 0u8);
        for _ in 0..1500 {
            s.advance(0.02);
            for px in s.pad_grid([0, 255, 0], Some([0, 0, 255])) {
                green = green.max(px[1]);
                blue = blue.max(px[2]);
            }
        }
        assert!(green >= 200 && blue >= 200, "{} {}", green, blue);
        assert!((i32::from(green) - i32::from(blue)).abs() <= 8, "{} {}", green, blue);
    }

    #[test]
    fn black_is_dark() {
        let mut dark = Starlight::new(9);
        for _ in 0..50 {
            dark.advance(0.04);
        }
        assert_eq!(lit(&dark.grid([0, 0, 0], None)), 0);
    }

    #[test]
    fn time_that_makes_no_sense_changes_nothing_and_a_long_stall_is_a_moment() {
        let mut s = Starlight::new(4);
        s.advance(f32::NAN);
        s.advance(-3.0);
        s.advance(0.0);
        assert_eq!(s.count(), 0);
        s.advance(3600.0);
        assert!(s.count() <= 12, "{}", s.count());
    }

    #[test]
    fn a_lone_star_twinkles_on_and_off_in_its_own_time() {
        let a = [0, 200, 0];
        let (mut dark, mut lit_, mut brightest) = (0, 0, 0u8);
        for cell in [3usize, 17, 40, 61] {
            for step in 0..4000 {
                let px = lone_star(cell, step as f32 * 0.02, a, None);
                assert_eq!((px[0], px[2]), (0, 0), "only the picked color");
                assert!(px[1] <= 200);
                if px == [0, 0, 0] { dark += 1 } else { lit_ += 1 }
                brightest = brightest.max(px[1]);
            }
        }
        assert!(dark > lit_, "mostly dark with stars now and then: {dark} / {lit_}");
        assert!(lit_ > 200, "but it does twinkle: {lit_}");
        assert!(brightest > 120, "some stars are bright: {brightest}");
        // two keys do not twinkle together, and the same key at the same time always shows the same
        let same = (0..2000).filter(|i| lone_star(5, *i as f32 * 0.05, a, None) == lone_star(6, *i as f32 * 0.05, a, None)).count();
        assert!(same < 1900);
        assert_eq!(lone_star(9, 12.3, a, None), lone_star(9, 12.3, a, None));
        // a time before the start is simply the start
        let _ = lone_star(9, -4.0, a, None);
    }

    #[test]
    fn a_lone_stars_second_color_follows_the_first() {
        let (a, b) = ([255, 0, 0], [0, 0, 255]);
        let (mut first_only, mut second_seen, mut both) = (false, false, false);
        for step in 0..20000 {
            let px = lone_star(12, step as f32 * 0.01, a, Some(b));
            assert!(px[1] == 0, "only the two colors");
            let (red, blue) = (px[0] > 0, px[2] > 0);
            first_only |= red && !blue;
            second_seen |= blue && !red;
            both |= red && blue;
        }
        assert!(first_only && second_seen && both, "{first_only} {second_seen} {both}");
    }
}
