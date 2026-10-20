//! The sounds of Game 3's scenes, made with `synth`: the portal opening and the shadow monarch
//! coming through it (it follows the same timeline as the picture, `monarch::beat`, so each sound
//! lands on what it belongs to), the ending, the thud of a miss and the chime of a checkpoint.
//!
//! The organ under the portal is a recording (`assets/game3-organ.mp3`); it is handed in here as
//! plain samples, already speeded up to the scene's pace, and laid under the made sounds.

use crate::game3::ending;
use crate::monarch::{END, KICKS, NORMAL_UNTIL, PULL_AT, SPEED};
use crate::synth::{Filter, Mix, Noise, Tone, Wave};

/// How long the organ recording is, once it has been speeded up (seconds), and when it comes in.
/// It starts where the hole starts to tear open, and its peak is made to land on the lunge.
pub const ORGAN_AT: f32 = 3.49;

/// Real seconds after the moon struck at which the scene reaches written time `w` (the scene is
/// played faster than it was written, except while the staff breaks).
pub fn real_of(w: f32) -> f32 {
    if w <= NORMAL_UNTIL {
        w
    } else {
        NORMAL_UNTIL + (w - NORMAL_UNTIL) / SPEED
    }
}

/// The portal scene: the gold seams stitching, the sand turning back, the hole tearing open, the
/// organ coming up under it (when given), him being written, his talons, the fire, his eyes, each
/// lurch and the lunge. As long as the scene plus its last echo.
pub fn portal(organ: Option<&[f32]>) -> Vec<f32> {
    let end = real_of(END);
    let mut m = Mix::new(end + 3.2, 11);
    let r = real_of;
    let tone = |m: &mut Mix, w: f32, dur: f32, p: Tone| m.tone(r(w), dur, p);

    // the sand turning back: each tick swells in backwards and closes with a small click
    let mut w = 1.5;
    while w < 5.2 {
        m.noise(r(w), 0.16, Noise::new(Filter::High, 2600.0, 5200.0, 0.06, 0.15));
        tone(&mut m, w + 0.16, 0.04, Tone::new(1500.0, 900.0, 0.07, 0.002).wave(Wave::Triangle));
        w += 0.42;
    }
    // the gold seams stitching shut
    for k in 0..6 {
        let f = 2100.0 + k as f32 * 170.0;
        tone(&mut m, 0.8 + k as f32 * 0.6, 0.5, Tone::new(f, f + 500.0, 0.03, 0.05));
    }
    // the hole tears open, then the low pull of it stays
    m.noise(r(3.0), 3.6, Noise::new(Filter::Band, 260.0, 3400.0, 0.34, 2.2).q(2.2));
    m.noise(r(3.4), 4.0, Noise::new(Filter::Low, 140.0, 700.0, 0.3, 1.4));
    m.crackle(r(3.0), r(6.2), 46, 0.07);
    tone(&mut m, 3.0, 0.9, Tone::new(180.0, 42.0, 0.5, 0.02));
    // under the organ, only a faint sub so the low end stays steady
    m.drone(r(3.0), r(23.0) - r(3.0), 41.0, 33.0, 0.12, r(6.4) - r(3.0), r(23.0) - r(19.6));
    // his two lines being written: a faint low blip for every letter
    for (w0, n) in [(6.4, 33), (12.4, 35)] {
        for i in 0..n {
            let f = 190.0 + m.rnd() * 70.0;
            tone(&mut m, w0 + i as f32 / 13.0, 0.05, Tone::new(f, 150.0, 0.022, 0.004).lp(900.0));
        }
    }
    // the talons coming out and the fingers uncurling: slow scrapes and a creak
    for i in 0..9 {
        let w = 10.7 + i as f32 * 0.36 + m.rnd() * 0.2;
        let (dur, f, g) = (0.3 + m.rnd() * 0.2, 3200.0 + m.rnd() * 900.0, 0.05 + m.rnd() * 0.04);
        m.noise(r(w), dur, Noise::new(Filter::Band, f, 1400.0, g, 0.05).q(9.0));
    }
    tone(&mut m, 10.6, 3.6, Tone::new(68.0, 54.0, 0.05, 1.5).wave(Wave::Saw).lp(240.0));
    // violet fire in the right hand
    m.noise(r(14.4), 2.4, Noise::new(Filter::Low, 600.0, 2200.0, 0.2, 1.4));
    m.crackle(r(14.4), r(16.6), 34, 0.06);
    // his eyes opening
    tone(&mut m, 15.4, 1.8, Tone::new(440.0, 110.0, 0.1, 0.08));
    tone(&mut m, 15.5, 1.5, Tone::new(662.0, 165.0, 0.06, 0.08));
    tone(&mut m, 15.4, 1.4, Tone::new(60.0, 40.0, 0.4, 0.2));
    // each lurch: a rising scrape into a hit that knocks the window, with glitching on top
    for (i, (a, b)) in [(0.4, 0.7), (1.45, 1.75), (2.4, 2.65), (2.9, 3.25)].into_iter().enumerate() {
        let (w0, w1) = (PULL_AT + a, PULL_AT + b);
        let i = i as f32;
        tone(&mut m, w0, r(w1) - r(w0) + 0.02, Tone::new(140.0 + i * 40.0, 520.0 + i * 160.0, 0.09 + i * 0.02, 0.12).wave(Wave::Saw).lp(1800.0));
    }
    for (d, a) in KICKS {
        let t = r(PULL_AT + d);
        m.tone(t, 0.6, Tone::new(120.0, 30.0, 0.9 * a, 0.003));
        m.noise(t, 0.3, Noise::new(Filter::Low, 1400.0, 200.0, 0.55 * a, 0.003));
        for k in 0..10 {
            let (dt, dur, f) = (k as f32 * 0.012 + m.rnd() * 0.03, 0.02 + m.rnd() * 0.03, 300.0 + m.rnd() * 2400.0);
            m.tone(t + dt, dur, Tone::new(f, f, 0.09 * a, 0.001).wave(Wave::Square));
        }
    }
    // the lunge: a stack of wrong notes climbing, then the big hit and a long low tail
    let (l0, l1) = (r(PULL_AT + 2.95), r(PULL_AT + 3.25));
    for f in [220.0, 233.0, 311.0, 466.0] {
        m.tone(l0, l1 - l0 + 0.03, Tone::new(f, f * 2.1, 0.05, 0.2).wave(Wave::Saw).lp(2600.0));
    }
    m.tone(l1, 2.8, Tone::new(90.0, 26.0, 1.0, 0.004));
    m.noise(l1, 1.1, Noise::new(Filter::Low, 5000.0, 120.0, 0.8, 0.004));
    m.crackle(l1, l1 + 0.55, 30, 0.1);
    m.tone(l1 + 0.3, 3.2, Tone::new(55.0, 34.0, 0.4, 0.4));
    // the organ, if there is one: it comes in as the hole opens and goes as the scene ends
    if let Some(organ) = organ {
        let len = organ.len() as f32 / crate::synth::SR as f32;
        m.add(ORGAN_AT, organ, 0.0, len.min(end - ORGAN_AT), 0.9, 0.5, 1.4);
    }
    m.finish(0.95)
}

/// The ending, `ending::STRIKE` seconds of him creeping through while the user's heart beats
/// faintly and faster, his lurches, the lunge, the talon through the heart (after which the heart
/// does not beat again), his two lines being written, and then the Underworld's own intro sound
/// (`assets/underworld-intro.mp3`, given as mono samples; a low bell stands in when there is none).
pub fn ending_sound(intro: Option<&[f32]>) -> Vec<f32> {
    let b = ending::STRIKE;
    let lu = ending::LUNGE;
    let mut m = Mix::new(b + ending::UNDERWORLD_AT + 6.5, 17);
    // a low hum under the approach, and his heavy lurches (the window knocks)
    m.tone(0.0, b, Tone::new(41.0, 36.0, 0.22, 1.2));
    m.tone(0.0, b, Tone::new(83.0, 72.0, 0.07, 1.2));
    for (t, a) in [(2.09, 0.9), (3.69, 1.0), (lu, 0.8)] {
        m.tone(t, 0.6, Tone::new(120.0, 30.0, 0.7 * a, 0.003));
        m.noise(t, 0.3, Noise::new(Filter::Low, 1400.0, 200.0, 0.45 * a, 0.003));
    }
    // the user's heart: faint, and quickening as he comes. A tone for the body of the beat and a
    // soft thump on top, so that small speakers carry it
    for t in ending::heart_beats() {
        let g = 0.13 + 0.2 * (t / b);
        m.tone(t, 0.16, Tone::new(112.0, 52.0, g, 0.004));
        m.noise(t, 0.09, Noise::new(Filter::Low, 260.0, 100.0, g * 0.55, 0.003));
        m.tone(t + 0.2, 0.18, Tone::new(96.0, 48.0, g * 0.7, 0.004));
    }
    // the wrong notes climbing up to the lunge
    for f in [220.0, 233.0, 311.0, 466.0] {
        m.tone(lu - 0.62, 0.6, Tone::new(f, f * 2.1, 0.06, 0.3).wave(Wave::Saw).lp(2600.0));
    }
    m.noise(lu - 0.62, 0.6, Noise::new(Filter::Band, 300.0, 4000.0, 0.2, 0.45).q(3.0));
    // the talon goes through: a crack, a wet hard thud, and the beat does not come again
    m.tone(b, 2.4, Tone::new(80.0, 22.0, 1.0, 0.003));
    m.noise(b, 0.9, Noise::new(Filter::Low, 6000.0, 120.0, 0.9, 0.003));
    m.noise(b, 0.28, Noise::new(Filter::Band, 900.0, 180.0, 0.75, 0.002).q(1.2));
    for k in 0..24 {
        let (dt, dur, f) = (k as f32 * 0.01 + m.rnd() * 0.05, 0.02 + m.rnd() * 0.04, 300.0 + m.rnd() * 2800.0);
        m.tone(b + dt, dur, Tone::new(f, f, 0.1, 0.001).wave(Wave::Square));
    }
    // his lines being written
    for (d, n) in [(ending::LINE1_AT, ending::LINE1.chars().count()), (ending::LINE2_AT, ending::LINE2.chars().count())] {
        for i in 0..n {
            let f = 170.0 + m.rnd() * 60.0;
            m.tone(b + d + i as f32 / ending::CPS, 0.05, Tone::new(f, 140.0, 0.05, 0.004).lp(700.0));
        }
    }
    // the Underworld: its intro sound, or a low bell
    let bell = b + ending::UNDERWORLD_AT;
    match intro {
        Some(intro) => {
            let len = intro.len() as f32 / crate::synth::SR as f32;
            m.add(bell, intro, 0.0, len, 0.9, 0.05, 1.0);
        }
        None => {
            m.tone(bell, 6.0, Tone::new(98.0, 97.0, 0.28, 0.02));
            m.tone(bell, 5.0, Tone::new(270.0, 268.0, 0.1, 0.02));
            m.tone(bell, 4.0, Tone::new(441.0, 439.0, 0.05, 0.02));
        }
    }
    // everything but the bell and the writing is ducked after the strike, and the bell is not
    m.level(&[(b + 0.15, 1.0), (b + 0.9, 0.14), (b + ending::UNDERWORLD_AT - 0.2, 0.14), (b + ending::UNDERWORLD_AT - 0.1, 1.0)]);
    m.finish(0.95)
}

/// A dull thud: a miss, a stray press, or running out of patience.
pub fn thud() -> Vec<f32> {
    let mut m = Mix::new(0.3, 3);
    m.noise(0.0, 0.18, Noise::new(Filter::Low, 300.0, 300.0, 0.3, 0.002));
    m.tone(0.0, 0.18, Tone::new(90.0, 45.0, 0.25, 0.002));
    m.finish(0.8)
}

/// Three rising notes: a checkpoint.
pub fn chime() -> Vec<f32> {
    let mut m = Mix::new(2.2, 5);
    for (i, f) in [392.0, 587.0, 784.0].into_iter().enumerate() {
        m.tone(i as f32 * 0.16, 1.4, Tone::new(f, f, 0.18, 0.02));
    }
    m.finish(0.8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::SR;

    fn rms(s: &[f32], a: f32, b: f32) -> f32 {
        let part = &s[(a * SR as f32) as usize..((b * SR as f32) as usize).min(s.len())];
        (part.iter().map(|x| x * x).sum::<f32>() / part.len().max(1) as f32).sqrt()
    }

    #[test]
    fn the_scene_is_played_at_the_scenes_pace() {
        assert_eq!(real_of(1.0), 1.0, "normal speed while the staff breaks");
        assert!((real_of(END) - crate::monarch::REAL_END).abs() < 1e-4);
        assert!(real_of(15.6) < 15.6 && real_of(15.6) > real_of(10.0));
    }

    #[test]
    fn the_portal_sound_is_quiet_at_first_tears_open_and_peaks_at_the_lunge() {
        let s = portal(None);
        let end = real_of(END);
        assert!(s.len() as f32 / SR as f32 > end + 3.0);
        assert!(s.iter().all(|x| x.abs() <= 0.951 && x.is_finite()));
        let early = rms(&s, 0.0, 1.0);
        let tearing = rms(&s, real_of(4.5), real_of(5.5));
        let lunge = real_of(PULL_AT + 3.25);
        let peak = rms(&s, lunge, lunge + 0.3);
        assert!(early < tearing * 0.5, "{early} {tearing}");
        assert!(peak > tearing, "the lunge is the loudest part: {peak} {tearing}");
        // and it is the same every time
        assert_eq!(portal(None), s);
    }

    #[test]
    fn the_organ_is_laid_under_the_scene_from_the_hole_opening_to_the_end() {
        let organ: Vec<f32> = (0..SR as usize * 15).map(|i| 0.5 * (i as f32 * 0.03).sin()).collect();
        let (with, without) = (portal(Some(&organ)), portal(None));
        let end = real_of(END);
        assert!(rms(&with, 6.0, 9.0) > rms(&without, 6.0, 9.0) * 1.3);
        assert!(rms(&with, 0.0, 3.0) < rms(&with, 6.0, 9.0) * 0.3, "it is not there before the hole opens");
        // it fades out as the scene ends, so the last second is as quiet as without it
        assert!(rms(&with, end - 0.1, end) < rms(&with, 6.0, 9.0) * 0.5);
        assert!(with.iter().all(|x| x.abs() <= 0.951));
    }

    #[test]
    fn the_ending_has_a_faint_heartbeat_that_stops_at_the_strike_and_then_the_bell() {
        let s = ending_sound(None);
        let b = ending::STRIKE;
        assert!(s.len() as f32 / SR as f32 > b + ending::UNDERWORLD_AT + 6.0);
        assert!(s.iter().all(|x| x.abs() <= 0.951 && x.is_finite()));
        // on a beat it is heard, between the beats it is quieter
        let beats = ending::heart_beats();
        let on = rms(&s, beats[2], beats[2] + 0.3);
        let between = rms(&s, beats[2] + 0.45, beats[3] - 0.05);
        assert!(on > between * 1.5, "{on} {between}");
        // the beat is faint compared with the strike
        assert!(rms(&s, b, b + 0.2) > on * 2.0);
        // after the strike it is ducked to almost nothing until the bell
        let after = rms(&s, b + 3.0, b + 6.0);
        assert!(after < on, "{after} {on}");
        let bell = rms(&s, b + ending::UNDERWORLD_AT + 0.3, b + ending::UNDERWORLD_AT + 2.0);
        assert!(bell > after);
        assert_eq!(ending_sound(None), s);
        // the Underworld's own sound takes the bell's place when there is one
        let intro: Vec<f32> = (0..SR as usize * 4).map(|i| 0.6 * (i as f32 * 0.02).sin()).collect();
        let with = ending_sound(Some(&intro));
        let at = b + ending::UNDERWORLD_AT;
        assert!(rms(&with, at + 0.5, at + 2.0) > bell * 1.2, "it is the louder of the two");
        assert!(with.iter().all(|x| x.abs() <= 0.951));
    }

    #[test]
    fn the_small_sounds_are_short_and_not_silent() {
        for (s, secs) in [(thud(), 0.3), (chime(), 2.2)] {
            assert!((s.len() as f32 / SR as f32 - secs).abs() < 0.01);
            assert!(s.iter().any(|x| x.abs() > 0.1) && s.iter().all(|x| x.abs() <= 0.801));
        }
    }
}
