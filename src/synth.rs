//! A tiny sound maker: tones and filtered noise laid onto a timeline, rendered to plain samples.
//!
//! The scene sounds of Game 3 (the portal opening, the ending) are built from this, so they are the
//! same every time and can be tested without a speaker: `Mix` is a mono buffer, `tone` and `noise`
//! put a sound on it at a time (seconds), and `finish` turns it into samples between -1 and 1.
//! Nothing here plays anything (see `audio.rs`).

use std::f32::consts::TAU;

/// The sample rate everything is made at.
pub const SR: u32 = 44_100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Saw,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Low,
    High,
    Band,
}

/// A tone that glides from `f0` to `f1` Hz over its length: it swells in over `attack` seconds and
/// then dies away exponentially to nothing by its end. `lowpass` softens it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub wave: Wave,
    pub f0: f32,
    pub f1: f32,
    pub gain: f32,
    pub attack: f32,
    pub lowpass: Option<f32>,
}

impl Tone {
    pub fn new(f0: f32, f1: f32, gain: f32, attack: f32) -> Tone {
        Tone { wave: Wave::Sine, f0, f1, gain, attack, lowpass: None }
    }
    pub fn wave(mut self, wave: Wave) -> Tone {
        self.wave = wave;
        self
    }
    pub fn lp(mut self, hz: f32) -> Tone {
        self.lowpass = Some(hz);
        self
    }
}

/// Noise through a filter whose cutoff glides from `f0` to `f1` Hz; same envelope as a tone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Noise {
    pub filter: Filter,
    pub f0: f32,
    pub f1: f32,
    pub q: f32,
    pub gain: f32,
    pub attack: f32,
}

impl Noise {
    pub fn new(filter: Filter, f0: f32, f1: f32, gain: f32, attack: f32) -> Noise {
        Noise { filter, f0, f1, q: 1.0, gain, attack }
    }
    pub fn q(mut self, q: f32) -> Noise {
        self.q = q;
        self
    }
}

/// A two-pole filter (the usual audio-cookbook shapes).
#[derive(Debug, Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    fn set(&mut self, kind: Filter, fc: f32, q: f32) {
        let fc = fc.clamp(20.0, SR as f32 * 0.45);
        let w = TAU * fc / SR as f32;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q.max(0.05));
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = match kind {
            Filter::Low => ((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0),
            Filter::High => ((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0),
            Filter::Band => (alpha, 0.0, -alpha),
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// How loud a sound of `gain` is `t` seconds in: up over `attack`, then down exponentially to a
/// ten-thousandth of itself at `dur`.
fn envelope(gain: f32, attack: f32, dur: f32, t: f32) -> f32 {
    let attack = attack.max(0.001).min(dur * 0.9);
    if t < attack {
        gain * t / attack
    } else {
        let f = ((t - attack) / (dur - attack).max(1e-4)).clamp(0.0, 1.0);
        gain * (1e-4f32).powf(f)
    }
}

/// A mono timeline.
pub struct Mix {
    buf: Vec<f32>,
    rng: u32,
}

impl Mix {
    /// An empty mix `seconds` long. The same `seed` makes the same noise.
    pub fn new(seconds: f32, seed: u32) -> Mix {
        Mix { buf: vec![0.0; (seconds.max(0.0) * SR as f32) as usize], rng: seed.wrapping_mul(2_654_435_761) | 1 }
    }

    pub fn seconds(&self) -> f32 {
        self.buf.len() as f32 / SR as f32
    }

    /// A number from 0 up to 1 (xorshift).
    pub fn rnd(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    fn span(&self, at: f32, dur: f32) -> (usize, usize) {
        let a = (at.max(0.0) * SR as f32) as usize;
        let n = (dur.max(0.0) * SR as f32) as usize;
        (a.min(self.buf.len()), (a + n).min(self.buf.len()))
    }

    /// Puts a tone on the timeline `at` seconds in, lasting `dur`.
    pub fn tone(&mut self, at: f32, dur: f32, p: Tone) {
        let (a, b) = self.span(at, dur);
        let mut phase = 0.0f32;
        let mut lp = Biquad::default();
        if let Some(hz) = p.lowpass {
            lp.set(Filter::Low, hz, 0.707);
        }
        let ratio = (p.f1.max(1.0) / p.f0.max(1.0)).max(1e-3);
        for i in a..b {
            let t = (i - a) as f32 / SR as f32;
            let f = p.f0 * ratio.powf(t / dur.max(1e-3));
            phase = (phase + f / SR as f32).fract();
            let raw = match p.wave {
                Wave::Sine => (phase * TAU).sin(),
                Wave::Square => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Saw => 2.0 * phase - 1.0,
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
            };
            let s = if p.lowpass.is_some() { lp.run(raw) } else { raw };
            self.buf[i] += s * envelope(p.gain, p.attack, dur, t);
        }
    }

    /// Puts filtered noise on the timeline.
    pub fn noise(&mut self, at: f32, dur: f32, p: Noise) {
        let (a, b) = self.span(at, dur);
        let mut f = Biquad::default();
        let ratio = (p.f1.max(1.0) / p.f0.max(1.0)).max(1e-3);
        for i in a..b {
            let t = (i - a) as f32 / SR as f32;
            if (i - a) % 32 == 0 {
                f.set(p.filter, p.f0 * ratio.powf(t / dur.max(1e-3)), p.q);
            }
            let white = self.rnd() * 2.0 - 1.0;
            self.buf[i] += f.run(white) * envelope(p.gain, p.attack, dur, t);
        }
    }

    /// A tone that stays: it fades in over `fade_in`, holds, fades out over `fade_out` at the end,
    /// and swells and ebbs a quarter either way about once every three seconds.
    pub fn drone(&mut self, at: f32, dur: f32, f0: f32, f1: f32, gain: f32, fade_in: f32, fade_out: f32) {
        let (a, b) = self.span(at, dur);
        let mut phase = 0.0f32;
        for i in a..b {
            let t = (i - a) as f32 / SR as f32;
            let f = f0 + (f1 - f0) * t / dur.max(1e-3);
            phase = (phase + f / SR as f32).fract();
            let up = (t / fade_in.max(1e-3)).clamp(0.0, 1.0);
            let down = ((dur - t) / fade_out.max(1e-3)).clamp(0.0, 1.0);
            let sway = 1.0 + 0.25 * (TAU * 0.35 * t).sin();
            self.buf[i] += (phase * TAU).sin() * gain * up.min(down) * sway;
        }
    }

    /// `n` short ticks and crackles between `t0` and `t1`, louder `gain`, at random.
    pub fn crackle(&mut self, t0: f32, t1: f32, n: usize, gain: f32) {
        for _ in 0..n {
            let at = t0 + (t1 - t0) * self.rnd().powf(0.8);
            let dur = 0.012 + self.rnd() * 0.02;
            let f = 800.0 + self.rnd() * 3000.0;
            let g = gain * (0.4 + self.rnd() * 0.6);
            self.tone(at, dur, Tone::new(f, f, g, 0.001).wave(Wave::Square));
        }
    }

    /// Mixes other samples (at `SR`) in at `at`, times `gain`, with a fade in and out at the ends of
    /// the stretch that is used (`from` to `to`, seconds into `samples`).
    pub fn add(&mut self, at: f32, samples: &[f32], from: f32, to: f32, gain: f32, fade_in: f32, fade_out: f32) {
        let s0 = ((from.max(0.0) * SR as f32) as usize).min(samples.len());
        let s1 = ((to.max(0.0) * SR as f32) as usize).min(samples.len()).max(s0);
        let start = (at.max(0.0) * SR as f32) as usize;
        let n = s1 - s0;
        for k in 0..n {
            let Some(slot) = self.buf.get_mut(start + k) else { break };
            let t = k as f32 / SR as f32;
            let up = (t / fade_in.max(1e-3)).clamp(0.0, 1.0);
            let down = ((n - k) as f32 / SR as f32 / fade_out.max(1e-3)).clamp(0.0, 1.0);
            *slot += samples[s0 + k] * gain * up.min(down);
        }
    }

    /// Multiplies the whole mix by a loudness that goes straight between these (time, loudness)
    /// points, and stays at the first before it and at the last after it.
    pub fn level(&mut self, points: &[(f32, f32)]) {
        if points.is_empty() {
            return;
        }
        for (i, s) in self.buf.iter_mut().enumerate() {
            let t = i as f32 / SR as f32;
            let g = if t <= points[0].0 {
                points[0].1
            } else if t >= points[points.len() - 1].0 {
                points[points.len() - 1].1
            } else {
                let k = points.windows(2).position(|w| t >= w[0].0 && t < w[1].0).unwrap_or(0);
                let (a, b) = (points[k], points[k + 1]);
                a.1 + (b.1 - a.1) * (t - a.0) / (b.0 - a.0).max(1e-6)
            };
            *s *= g;
        }
    }

    /// The samples: squeezed gently so nothing goes past 1, and never louder than `peak`.
    pub fn finish(mut self, peak: f32) -> Vec<f32> {
        for s in &mut self.buf {
            *s = (*s * 1.1).tanh();
        }
        let top = self.buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        if top > peak {
            let k = peak / top;
            for s in &mut self.buf {
                *s *= k;
            }
        }
        self.buf
    }
}

/// Mono samples from interleaved ones (`channels` of them): the average of each frame.
pub fn to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    samples.chunks(channels).map(|f| f.iter().sum::<f32>() / f.len() as f32).collect()
}

/// Samples at `from` Hz as samples at `SR` (nearest sample, which is enough for a background bed).
pub fn to_sr(samples: &[f32], from: u32) -> Vec<f32> {
    if from == SR || from == 0 {
        return samples.to_vec();
    }
    let n = (samples.len() as u64 * SR as u64 / from as u64) as usize;
    (0..n).map(|i| samples[((i as u64 * from as u64 / SR as u64) as usize).min(samples.len().saturating_sub(1))]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    fn window(s: &[f32], a: f32, b: f32) -> &[f32] {
        &s[(a * SR as f32) as usize..((b * SR as f32) as usize).min(s.len())]
    }

    #[test]
    fn a_tone_is_the_pitch_asked_for_and_silent_before_and_after_it() {
        let mut m = Mix::new(2.0, 1);
        m.tone(0.5, 0.5, Tone::new(440.0, 440.0, 0.8, 0.01));
        let s = m.finish(1.0);
        assert_eq!(s.len(), 2 * SR as usize);
        assert!(window(&s, 0.0, 0.49).iter().all(|x| *x == 0.0), "nothing before it");
        assert!(window(&s, 1.01, 2.0).iter().all(|x| *x == 0.0), "nothing after it");
        // rising zero crossings in the loudest part: about 440 a second
        let part = window(&s, 0.5, 0.6);
        let up = part.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((40..=48).contains(&up), "{up} crossings in a tenth of a second");
    }

    #[test]
    fn a_sound_swells_in_over_its_attack_then_dies_away() {
        let mut m = Mix::new(3.0, 1);
        m.tone(0.0, 2.0, Tone::new(220.0, 220.0, 0.5, 0.5));
        let s = m.finish(1.0);
        let early = rms(window(&s, 0.0, 0.05));
        let peak = rms(window(&s, 0.45, 0.55));
        let late = rms(window(&s, 1.8, 1.99));
        assert!(early < peak * 0.3 && late < peak * 0.05, "{early} {peak} {late}");
    }

    #[test]
    fn the_same_seed_makes_the_same_noise_and_another_seed_a_different_one() {
        let make = |seed| {
            let mut m = Mix::new(1.0, seed);
            m.noise(0.0, 0.5, Noise::new(Filter::Low, 2000.0, 500.0, 0.5, 0.01));
            m.finish(1.0)
        };
        assert_eq!(make(3), make(3));
        assert_ne!(make(3), make(4));
    }

    #[test]
    fn filters_take_the_noise_where_they_say() {
        let band = |f: f32| {
            let mut m = Mix::new(1.0, 5);
            m.noise(0.0, 0.9, Noise::new(Filter::Band, f, f, 0.5, 0.01).q(8.0));
            m.finish(1.0)
        };
        // a narrow band is much quieter than white noise through nothing much
        let mut white = Mix::new(1.0, 5);
        white.noise(0.0, 0.9, Noise::new(Filter::Low, 20_000.0, 20_000.0, 0.5, 0.01));
        let white = white.finish(1.0);
        assert!(rms(window(&band(1000.0), 0.0, 0.3)) < rms(window(&white, 0.0, 0.3)) * 0.6);
        // a low-pass leaves little above its cutoff: its zero crossings are far fewer than white noise's
        let mut low = Mix::new(1.0, 5);
        low.noise(0.0, 0.9, Noise::new(Filter::Low, 200.0, 200.0, 0.5, 0.01));
        let low = low.finish(1.0);
        let cross = |s: &[f32]| s.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
        assert!(cross(window(&low, 0.0, 0.3)) * 4 < cross(window(&white, 0.0, 0.3)));
    }

    #[test]
    fn nothing_goes_past_the_peak_asked_for_however_much_is_piled_on() {
        let mut m = Mix::new(1.0, 9);
        for _ in 0..40 {
            m.tone(0.0, 0.9, Tone::new(100.0, 90.0, 1.0, 0.01).wave(Wave::Saw));
        }
        let s = m.finish(0.9);
        assert!(s.iter().all(|x| x.abs() <= 0.9 + 1e-6));
        assert!(s.iter().any(|x| x.abs() > 0.5));
    }

    #[test]
    fn a_level_curve_ducks_and_restores_the_mix() {
        let mut m = Mix::new(4.0, 1);
        m.drone(0.0, 4.0, 100.0, 100.0, 0.5, 0.01, 0.01);
        m.level(&[(1.0, 1.0), (1.5, 0.1), (3.0, 0.1), (3.2, 1.0)]);
        let s = m.finish(1.0);
        let before = rms(window(&s, 0.5, 0.9));
        let low = rms(window(&s, 2.0, 2.5));
        let after = rms(window(&s, 3.5, 3.9));
        assert!(low < before * 0.3 && after > before * 0.6, "{before} {low} {after}");
    }

    #[test]
    fn other_samples_mix_in_at_a_time_with_fades_and_only_the_part_asked_for() {
        let tone: Vec<f32> = (0..SR as usize * 2).map(|i| (i as f32 * 0.05).sin()).collect();
        let mut m = Mix::new(3.0, 1);
        m.add(1.0, &tone, 0.5, 1.5, 0.8, 0.2, 0.2);
        let s = m.finish(1.0);
        assert!(window(&s, 0.0, 0.99).iter().all(|x| *x == 0.0) && window(&s, 2.05, 3.0).iter().all(|x| *x == 0.0));
        assert!(rms(window(&s, 1.0, 1.02)) < rms(window(&s, 1.4, 1.6)) * 0.3, "fades in");
        assert!(rms(window(&s, 1.98, 2.0)) < rms(window(&s, 1.4, 1.6)) * 0.3, "fades out");
    }

    #[test]
    fn stereo_becomes_mono_and_other_rates_become_ours() {
        assert_eq!(to_mono(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
        assert_eq!(to_mono(&[1.0, 2.0], 1), vec![1.0, 2.0]);
        let half: Vec<f32> = (0..22_050).map(|i| i as f32).collect();
        let up = to_sr(&half, 22_050);
        assert_eq!(up.len(), 44_100);
        assert_eq!(up[0], 0.0);
        assert_eq!(to_sr(&half, SR).len(), half.len());
    }

    #[test]
    fn writing_past_the_end_of_the_mix_is_harmless() {
        let mut m = Mix::new(0.5, 1);
        m.tone(0.4, 5.0, Tone::new(200.0, 200.0, 1.0, 0.01));
        m.tone(9.0, 1.0, Tone::new(200.0, 200.0, 1.0, 0.01));
        m.noise(0.45, 5.0, Noise::new(Filter::High, 100.0, 100.0, 1.0, 0.01));
        m.crackle(0.0, 5.0, 10, 0.1);
        assert_eq!(m.finish(1.0).len(), (0.5 * SR as f32) as usize);
    }
}
