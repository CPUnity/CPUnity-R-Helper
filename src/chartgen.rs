//! Charts for any song. This is the same method that made the charts of the two story songs (see
//! tools/game3), written in Rust so that the program can do it for a song the player adds:
//!
//! 1. The sound is cut into short overlapping windows and each is turned into a spectrum. For three
//!    bands (low: drums and bass, mid: most instruments, high: hats and sparkle) it takes how much
//!    louder the band got since the window before: that is where things *happen* in the music.
//! 2. The tempo and where the beat falls are found by trying every tempo from 70 to 180 BPM and
//!    every offset, and keeping the one that lines up with the most happenings.
//! 3. Every sixteenth of a beat is a place a note may go. A note goes where a band happened strongly
//!    enough (louder places get two notes, quiet bars get fewer), onto a lane chosen by the band, so
//!    that low sounds are on one side and high ones on the other. Each difficulty has its own
//!    thresholds, lanes and spacing.
//! 4. Songs differ a lot, so the thresholds of a difficulty are then raised or lowered until the
//!    number of notes per second is about what that difficulty plays like.
//!
//! No drawing and no sound card here: samples in, notes out, so it is all tested.

use crate::game3::{Note, Song};
use crate::games::Difficulty;

/// The sound is looked at at this rate (it is brought down to it first), through windows of 1024
/// samples (46 ms) moved on 256 at a time: this is what the story songs' charts were made with.
const RATE: u32 = 22_050;
const WINDOW: usize = 1024;
const HOP: usize = 256;
/// Longest song that is looked at (seconds); anything shorter than `MIN_SECS` is not a song.
pub const MAX_SECS: f32 = 15.0 * 60.0;
pub const MIN_SECS: f32 = 20.0;

/// What was found out about a song.
pub struct Analysis {
    pub len: f32,
    pub bpm: f32,
    /// When the first beat falls.
    pub offset: f32,
    /// Seconds between two windows.
    step: f32,
    /// How much each band got louder, per window: low, mid, high.
    bands: [Vec<f32>; 3],
}

// ---- the spectrum ---------------------------------------------------------------------------------

struct Fft {
    twiddle: Vec<(f32, f32)>,
    rev: Vec<usize>,
}

impl Fft {
    fn new(n: usize) -> Fft {
        let bits = n.trailing_zeros();
        let twiddle = (0..n / 2)
            .map(|k| {
                let a = -2.0 * std::f64::consts::PI * k as f64 / n as f64;
                (a.cos() as f32, a.sin() as f32)
            })
            .collect();
        let rev = (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect();
        Fft { twiddle, rev }
    }

    fn run(&self, re: &mut [f32], im: &mut [f32]) {
        let n = re.len();
        for i in 0..n {
            let j = self.rev[i];
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let (half, step) = (len / 2, n / len);
            for start in (0..n).step_by(len) {
                for k in 0..half {
                    let (wr, wi) = self.twiddle[k * step];
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            len *= 2;
        }
    }
}

/// A Gaussian blur of `x` (sigma in samples), its ends mirrored.
fn blur(x: &[f32], sigma: f32) -> Vec<f32> {
    let radius = (4.0 * sigma + 0.5) as i32;
    let weights: Vec<f32> = (-radius..=radius).map(|i| (-0.5 * (i as f32 / sigma).powi(2)).exp()).collect();
    let total: f32 = weights.iter().sum();
    let n = x.len() as i32;
    (0..n)
        .map(|i| {
            let mut sum = 0.0;
            for (k, w) in weights.iter().enumerate() {
                let mut j = i + k as i32 - radius;
                if j < 0 {
                    j = -j - 1;
                }
                if j >= n {
                    j = 2 * n - j - 1;
                }
                sum += w * x[j.clamp(0, n - 1) as usize];
            }
            sum / total
        })
        .collect()
}

fn std_dev(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    let mean = x.iter().sum::<f32>() / x.len() as f32;
    (x.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / x.len() as f32).sqrt()
}

/// The value below which `p` percent of `x` lies (linear between neighbours, as numpy does).
fn percentile(x: &[f32], p: f32) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    let mut v = x.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pos = (p / 100.0) * (v.len() - 1) as f32;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    v[lo] + (v[hi] - v[lo]) * (pos - lo as f32)
}

/// `samples` at `sr` Hz as samples at `RATE` (each new one is the average of the old ones it covers).
fn to_rate(samples: &[f32], sr: u32) -> Vec<f32> {
    if sr == RATE {
        return samples.to_vec();
    }
    let ratio = sr as f64 / RATE as f64;
    let n = (samples.len() as f64 / ratio).floor() as usize;
    (0..n)
        .map(|i| {
            let (a, b) = (i as f64 * ratio, (i + 1) as f64 * ratio);
            let (lo, hi) = (a.floor() as usize, (b.ceil() as usize).min(samples.len()));
            let mut sum = 0.0f64;
            for (k, v) in samples[lo.min(samples.len())..hi].iter().enumerate() {
                let (s0, s1) = (((lo + k) as f64).max(a), ((lo + k + 1) as f64).min(b));
                sum += *v as f64 * (s1 - s0).max(0.0);
            }
            (sum / (b - a)) as f32
        })
        .collect()
}

/// Looks at `samples` (mono, at `sr` Hz). None if it is too short, too long or silent.
pub fn analyze(samples: &[f32], sr: u32) -> Option<Analysis> {
    if sr < 8000 {
        return None;
    }
    let len = samples.len() as f32 / sr as f32;
    if !(MIN_SECS..=MAX_SECS).contains(&len) || samples.iter().all(|s| s.abs() < 1e-4) {
        return None;
    }
    let owned = to_rate(samples, sr);
    let (samples, sr) = (&owned[..], RATE);
    let (window_len, hop) = (WINDOW, HOP);
    let frames = samples.len() / hop + 1;
    let fft = Fft::new(window_len);
    let window: Vec<f32> = (0..window_len).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / window_len as f32).cos()).collect();
    let scale = 1.0 / window.iter().sum::<f32>();
    let bin_hz = sr as f32 / window_len as f32;
    let range = |lo: f32, hi: f32| ((lo / bin_hz).ceil() as usize, ((hi / bin_hz).ceil() as usize).min(window_len / 2 + 1));
    let ranges = [range(30.0, 150.0), range(150.0, 2000.0), range(2000.0, 9000.0)];
    let mut loud: [Vec<f32>; 3] = [Vec::with_capacity(frames), Vec::with_capacity(frames), Vec::with_capacity(frames)];
    let (mut re, mut im) = (vec![0.0f32; window_len], vec![0.0f32; window_len]);
    for f in 0..frames {
        // the window is centred on f * hop, with silence beyond the ends
        let start = (f * hop) as isize - (window_len / 2) as isize;
        for i in 0..window_len {
            let at = start + i as isize;
            re[i] = if at >= 0 && (at as usize) < samples.len() { samples[at as usize] * window[i] } else { 0.0 };
            im[i] = 0.0;
        }
        fft.run(&mut re, &mut im);
        for (b, (lo, hi)) in ranges.iter().enumerate() {
            let (lo, hi) = (*lo, (*hi).max(*lo + 1));
            let mut sum = 0.0;
            for k in lo..hi {
                let mag = (re[k] * re[k] + im[k] * im[k]).sqrt() * scale;
                sum += (mag * 30.0).ln_1p();
            }
            loud[b].push(sum / (hi - lo) as f32);
        }
    }
    // how much louder than the window before, never less than nothing
    let bands: [Vec<f32>; 3] = [0, 1, 2].map(|b| {
        let l = &loud[b];
        (0..l.len()).map(|i| if i == 0 { 0.0 } else { (l[i] - l[i - 1]).max(0.0) }).collect()
    });
    let step = hop as f32 / sr as f32;
    // the three bands together, each by its own spread, smoothed: this is what the tempo is found from
    let mut total = vec![0.0f32; frames];
    for b in &bands {
        let sd = std_dev(b) + 1e-9;
        for (t, v) in total.iter_mut().zip(b) {
            *t += v / sd;
        }
    }
    let env = blur(&total, 1.0);
    let (bpm, offset) = find_tempo(&env, step, len)?;
    Some(Analysis { len, bpm, offset, step, bands })
}

/// The mean of `env` at times `start, start + period, ...` below `end` (linear between windows).
fn mean_at(env: &[f32], step: f32, start: f32, period: f32, end: f32) -> f32 {
    let (mut sum, mut n) = (0.0f32, 0u32);
    let mut t = start;
    while t < end {
        let x = t / step;
        let i = x.floor() as usize;
        let v = if i + 1 >= env.len() { env[env.len() - 1] } else { env[i] + (env[i + 1] - env[i]) * (x - i as f32) };
        sum += v;
        n += 1;
        t += period;
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f32
    }
}

/// The tempo (BPM, 90 to 180) and the time of the first beat that line up with the most happenings.
fn find_tempo(env: &[f32], step: f32, len: f32) -> Option<(f32, f32)> {
    let mut best: Option<(f32, f32, f32)> = None;
    let mut bpm10 = 700u32;
    while bpm10 < 1810 {
        let bpm = bpm10 as f32 / 10.0;
        let period = 60.0 / bpm;
        let mut ph = 0.0f32;
        while ph < period {
            let v = mean_at(env, step, ph, period, len - 1.0);
            if best.map_or(true, |(bv, _, _)| v > bv) {
                best = Some((v, bpm, ph));
            }
            ph += 0.01;
        }
        bpm10 += 1;
    }
    let (_, mut bpm, mut phase) = best?;
    // half-time and double-time look alike to the search; sixteenths are only fine enough when the
    // tempo is read the quicker way, so it is folded into 90 to 180
    let mut folded = false;
    while bpm < 90.0 {
        bpm *= 2.0;
        folded = true;
    }
    while bpm >= 180.0 {
        bpm /= 2.0;
        folded = true;
    }
    if folded {
        let period = 60.0 / bpm;
        let (mut top, mut at) = (f32::MIN, 0.0f32);
        let mut ph = 0.0f32;
        while ph < period {
            let v = mean_at(env, step, ph, period, len - 1.0);
            if v > top {
                top = v;
                at = ph;
            }
            ph += 0.01;
        }
        phase = at;
    }
    Some((bpm, phase))
}

// ---- the notes ------------------------------------------------------------------------------------

struct Setting {
    lanes: usize,
    /// Which sixteenths of a beat (0 is the beat itself, 2 the off-beat) may hold a note.
    grid: &'static [usize],
    /// Per band, the strength it takes on a beat, an off-beat and a sixteenth.
    thr: [[f32; 3]; 3],
    /// Sixteenths that must pass before a lane is used again.
    gap: isize,
    /// Notes per beat at most.
    cap: usize,
    /// The lanes each band is played on, in the order they are taken.
    lane_sets: [&'static [u8]; 3],
    /// Notes per second a song of this difficulty should have.
    target: f32,
}

fn setting(d: Difficulty) -> Setting {
    match d {
        Difficulty::Easy => Setting {
            lanes: 4,
            grid: &[0, 2],
            thr: [[0.38, 0.6, 9.0], [0.45, 0.62, 9.0], [0.55, 0.8, 9.0]],
            gap: 3,
            cap: 1,
            lane_sets: [&[0, 1], &[2, 3], &[3, 2]],
            target: 1.65,
        },
        Difficulty::Normal => Setting {
            lanes: 6,
            grid: &[0, 1, 2, 3],
            thr: [[0.3, 0.45, 0.95], [0.38, 0.5, 0.85], [0.5, 0.6, 0.95]],
            gap: 2,
            cap: 2,
            lane_sets: [&[1, 2, 0], &[3, 4, 5], &[5, 0, 4, 1, 3, 2]],
            target: 3.1,
        },
        Difficulty::Hard => Setting {
            lanes: 8,
            grid: &[0, 1, 2, 3],
            thr: [[0.22, 0.32, 0.55], [0.28, 0.38, 0.55], [0.32, 0.42, 0.62]],
            gap: 2,
            cap: 3,
            lane_sets: [&[0, 1, 2, 3], &[3, 4, 2, 5], &[5, 6, 7, 4]],
            target: 4.4,
        },
    }
}

/// What the music does at every sixteenth of a beat: per band, how strong, as a share of its
/// loudest places; and how loud each bar is.
struct Slots {
    strength: [Vec<f32>; 3],
    bar_loudness: Vec<f32>,
    slot_len: f32,
}

fn slots(a: &Analysis) -> Slots {
    let slot_len = 60.0 / a.bpm / 4.0;
    let n = (((a.len - a.offset - 0.3) / slot_len).floor().max(0.0)) as usize;
    let around = |env: &[f32], t: f32| -> f32 {
        let (lo, hi) = (((t - 0.035) / a.step).ceil().max(0.0) as usize, ((t + 0.035) / a.step).floor().max(0.0) as usize);
        if lo >= env.len() || lo > hi {
            return 0.0;
        }
        env[lo..=hi.min(env.len() - 1)].iter().cloned().fold(0.0f32, f32::max)
    };
    let strength: [Vec<f32>; 3] = [0, 1, 2].map(|b| {
        let raw: Vec<f32> = (0..n).map(|i| around(&a.bands[b], a.offset + i as f32 * slot_len)).collect();
        let top = percentile(&raw, 95.0);
        let top = if top > 1e-9 { top } else { 1.0 };
        raw.iter().map(|v| v / top).collect()
    });
    let bar_loudness = (0..n / 16)
        .map(|b| (0..3).map(|k| strength[k][b * 16..(b + 1) * 16].iter().sum::<f32>() / 16.0).sum::<f32>())
        .collect();
    Slots { strength, bar_loudness, slot_len }
}

/// The notes of one difficulty with every threshold multiplied by `mult` (higher: fewer notes).
fn place(a: &Analysis, s: &Slots, d: Difficulty, mult: f32) -> Vec<Note> {
    let c = setting(d);
    let n = s.strength[0].len();
    let median = {
        let mut v = s.bar_loudness.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        if v.is_empty() {
            0.0
        } else if v.len() % 2 == 1 {
            v[v.len() / 2]
        } else {
            (v[v.len() / 2 - 1] + v[v.len() / 2]) / 2.0
        }
    };
    let mut used = vec![0u32; c.lanes];
    let mut free = vec![-99isize; c.lanes];
    let mut count = [0usize; 3];
    let mut last_lane: isize = -1;
    // (slot, lane, strength)
    let mut out: Vec<(usize, u8, f32)> = Vec::new();
    for i in 0..n {
        let (bar, j) = (i / 16, i % 4);
        if !c.grid.contains(&j) {
            continue;
        }
        let calm = bar < s.bar_loudness.len() && s.bar_loudness[bar] < 0.72 * median;
        let mut cand: Vec<(f32, usize)> = Vec::new();
        for band in 0..3 {
            let [on_beat, off_beat, sixteenth] = c.thr[band];
            let mut thr = if j == 0 { on_beat } else if j == 2 { off_beat } else { sixteenth };
            if calm {
                thr = if j == 0 || j == 2 { thr * 1.35 } else { 99.0 };
            }
            thr *= mult;
            let v = s.strength[band][i];
            if v > thr {
                cand.push((v / thr, band));
            }
        }
        cand.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal).then(y.1.cmp(&x.1)));
        let mut placed: Vec<(f32, u8)> = Vec::new();
        for (score, band) in cand {
            let most = if d != Difficulty::Easy && (i % 16 == 0 || score > 1.5) { 2 } else { 1 };
            if placed.len() >= most {
                break;
            }
            let set = c.lane_sets[band];
            let mut order: Vec<u8> = (0..set.len()).map(|k| set[(count[band] + k) % set.len()]).collect();
            if d == Difficulty::Hard {
                order.sort_by_key(|l| used[*l as usize]);
            }
            let lane = order.into_iter().find(|l| {
                i as isize - free[*l as usize] >= c.gap
                    && !placed.iter().any(|p| p.1 == *l)
                    && !(d == Difficulty::Hard && *l as isize == last_lane)
            });
            let Some(lane) = lane else { continue };
            count[band] += 1;
            placed.push((score, lane));
        }
        for (score, lane) in placed {
            free[lane as usize] = i as isize;
            last_lane = lane as isize;
            used[lane as usize] += 1;
            out.push((i, lane, score));
        }
    }
    // no more than `cap` notes in a beat: keep the strongest
    let mut keep: Vec<(usize, u8, f32)> = Vec::new();
    let mut at = 0;
    while at < out.len() {
        let beat = out[at].0 / 4;
        let mut end = at;
        while end < out.len() && out[end].0 / 4 == beat {
            end += 1;
        }
        let mut items = out[at..end].to_vec();
        items.sort_by(|x, y| y.2.partial_cmp(&x.2).unwrap_or(std::cmp::Ordering::Equal));
        keep.extend(items.into_iter().take(c.cap));
        at = end;
    }
    keep.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
    keep.into_iter().map(|(i, lane, _)| Note { t: ((a.offset + i as f32 * s.slot_len) * 1000.0).round() / 1000.0, lane }).collect()
}

/// The notes of difficulty `d` as the story songs' were made (thresholds as tuned for them).
pub fn chart(a: &Analysis, d: Difficulty) -> Vec<Note> {
    place(a, &slots(a), d, 1.0)
}

/// The notes of difficulty `d`, with the thresholds moved until the song has about as many notes a
/// second as that difficulty should.
pub fn chart_to_target(a: &Analysis, d: Difficulty) -> Vec<Note> {
    let s = slots(a);
    let want = setting(d).target;
    let rate = |mult: f32| place(a, &s, d, mult).len() as f32 / a.len;
    let (mut lo, mut hi) = (0.3f32, 3.0f32);
    // notes fall as the thresholds rise: find where it crosses the target
    let mut best = (f32::MAX, 1.0f32);
    for _ in 0..12 {
        let mid = (lo * hi).sqrt();
        let r = rate(mid);
        let off = (r - want).abs();
        if off < best.0 {
            best = (off, mid);
        }
        if r > want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    place(a, &s, d, best.1)
}

/// A song for the game from `samples` (mono, at `sr` Hz): the three charts are made to suit it.
pub fn make_song(title: &str, artist: &str, samples: &[f32], sr: u32) -> Option<Song> {
    let a = analyze(samples, sr)?;
    let notes = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard].map(|d| chart_to_target(&a, d));
    Some(Song::from_parts(title, artist, a.bpm, a.offset, a.len, notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A click on every beat of `bpm` (louder on the first of four), with a hat on the off-beats,
    /// first beat at `first` seconds: `secs` long at 22.05 kHz.
    fn click_track(bpm: f32, first: f32, secs: f32) -> Vec<f32> {
        let sr = 22_050usize;
        let mut x = vec![0.0f32; (secs * sr as f32) as usize];
        let beat = 60.0 / bpm;
        let mut k = 0;
        let mut seed = 12345u32;
        loop {
            let t = first + k as f32 * beat;
            if t >= secs - 0.2 {
                break;
            }
            let start = (t * sr as f32) as usize;
            // a thump, and a hat halfway
            for i in 0..2000 {
                let env = (-(i as f32) / 350.0).exp();
                let amp = if k % 4 == 0 { 0.9 } else { 0.6 };
                x[start + i] += amp * env * (2.0 * std::f32::consts::PI * 60.0 * i as f32 / sr as f32).sin();
            }
            let half = start + (beat * sr as f32 / 2.0) as usize;
            for i in 0..600 {
                if half + i < x.len() {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    x[half + i] += 0.04 * (-(i as f32) / 120.0).exp() * ((seed >> 16) as f32 / 32768.0 - 1.0);
                }
            }
            k += 1;
        }
        x
    }

    #[test]
    fn the_fft_finds_a_tone() {
        let fft = Fft::new(1024);
        let mut re: Vec<f32> = (0..1024).map(|i| (2.0 * std::f32::consts::PI * 64.0 * i as f32 / 1024.0).sin()).collect();
        let mut im = vec![0.0; 1024];
        fft.run(&mut re, &mut im);
        let mags: Vec<f32> = (0..512).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect();
        let peak = mags.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
        assert_eq!(peak, 64);
        assert!((mags[64] - 512.0).abs() < 1.0, "{}", mags[64]);
    }

    #[test]
    fn blur_keeps_a_constant_and_smooths_a_spike() {
        assert!(blur(&[2.0; 10], 1.0).iter().all(|v| (v - 2.0).abs() < 1e-5));
        let mut x = vec![0.0; 21];
        x[10] = 1.0;
        let b = blur(&x, 1.0);
        assert!(b[10] < 1.0 && b[10] > b[8] && b[9] > b[7]);
        assert!((b.iter().sum::<f32>() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn percentile_is_linear_between_neighbours() {
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0, 5.0], 50.0), 3.0);
        assert!((percentile(&[0.0, 10.0], 95.0) - 9.5).abs() < 1e-5);
    }

    #[test]
    fn it_finds_the_tempo_and_the_first_beat_of_a_click_track() {
        let x = click_track(120.0, 0.3, 40.0);
        let a = analyze(&x, 22_050).expect("a song");
        assert!((a.len - 40.0).abs() < 0.1);
        // the tempo may be found as the beat or as a multiple of it, but it must line up with the clicks
        let ratio = a.bpm / 120.0;
        assert!([0.5f32, 1.0, 2.0].iter().any(|r| (ratio - r).abs() < 0.02), "{} bpm", a.bpm);
        let beat = 60.0 / a.bpm;
        let from_click = ((a.offset - 0.3) / beat).rem_euclid(1.0);
        let off_by = from_click.min(1.0 - from_click) * beat;
        assert!(off_by < 0.03, "offset {} is {} s off the clicks", a.offset, off_by);
    }

    #[test]
    fn silence_and_very_short_or_long_audio_are_not_songs() {
        assert!(analyze(&vec![0.0; 22_050 * 40], 22_050).is_none());
        assert!(analyze(&click_track(120.0, 0.3, 5.0), 22_050).is_none());
        assert!(analyze(&[0.5; 100], 44_100).is_none());
    }

    #[test]
    fn charts_have_the_right_lanes_in_time_order_and_more_notes_when_harder() {
        let x = click_track(124.0, 0.5, 60.0);
        let a = analyze(&x, 22_050).unwrap();
        let counts: Vec<usize> = Difficulty::ALL
            .iter()
            .map(|d| {
                let c = chart_to_target(&a, *d);
                assert!(c.windows(2).all(|w| w[0].t <= w[1].t), "in time order");
                assert!(c.iter().all(|n| (n.lane as usize) < setting(*d).lanes && n.t >= 0.0 && n.t <= a.len));
                c.len()
            })
            .collect();
        assert!(counts[0] < counts[1] && counts[1] <= counts[2], "{counts:?}");
        assert!(counts[0] > 10, "{counts:?}");
    }

    #[test]
    fn the_target_pulls_the_density_towards_what_the_difficulty_plays_like() {
        let x = click_track(124.0, 0.5, 80.0);
        let a = analyze(&x, 22_050).unwrap();
        for d in Difficulty::ALL {
            let rate = chart_to_target(&a, d).len() as f32 / a.len;
            // a click track has only so many places for notes, so only an upper bound is certain
            assert!(rate <= setting(d).target * 1.35, "{:?} {rate}", d);
        }
    }

    #[test]
    fn make_song_gives_a_playable_song() {
        let x = click_track(124.0, 0.5, 60.0);
        let s = make_song("Click", "Test", &x, 22_050).unwrap();
        assert_eq!(s.title, "Click");
        assert!(s.len > 59.0 && s.bpm > 60.0);
        assert!(!s.notes(Difficulty::Hard).is_empty());
    }
}
