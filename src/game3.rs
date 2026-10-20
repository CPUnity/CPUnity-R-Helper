//! Game 3, "Patience for Death": a rhythm game against the shadow monarch, in The Void.
//!
//! Notes fall down lanes towards a line; press the lane's key as a note crosses it. Every miss makes
//! him step closer and drains his patience (the bar on the left); a clean stretch gives it back.
//! Pressing a key with no note near it costs patience too, more and more the longer you keep doing
//! it, so button-mashing loses fast. Run out and he lunges and the run is over.
//!
//! The story is two songs. Clearing the first with patience left is a checkpoint (it is kept, so
//! coming back puts you on the second). Clearing the second is the end: nobody beats him, a perfect
//! run included. He takes you, and that is what opens The Underworld. How well you played only
//! decides the rank, which is for show.
//!
//! Difficulty (`Difficulty`, chosen on the screen before each run, and in Options like the other games') decides the lanes: Easy has
//! D F J K, Normal S D F J K L (both hands, no thumbs), and Hard all eight (A S D F J K L ;). The notes come from charts
//! made from the songs' audio (`assets/game3-charts.txt`).
//!
//! Everything here is pure (no drawing, no sound, no clock): the time is passed in, so it is tested.

use crate::games::{Cheat, Cheats, Difficulty};
use std::sync::OnceLock;

/// A hit this close to the note's time (seconds) is perfect, and this close is good.
pub const PERFECT: f32 = 0.045;
pub const GOOD: f32 = 0.10;
/// How long a note takes to fall from the top to the line.
pub const APPROACH: f32 = 1.55;
/// The count-in before the music starts (the song's clock starts at minus this).
pub const LEAD_IN: f32 = 1.2;
/// A song is over this long after its length.
pub const TAIL: f32 = 1.0;
/// How many songs the story has.
pub const SONGS: usize = 2;
/// The most lanes there are.
pub const MAX_LANES: usize = 8;

// ---- the keys ----------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneKey {
    A,
    S,
    D,
    F,
    J,
    K,
    L,
    Semicolon,
}

impl LaneKey {
    pub fn label(self) -> &'static str {
        match self {
            LaneKey::A => "A",
            LaneKey::S => "S",
            LaneKey::D => "D",
            LaneKey::F => "F",
            LaneKey::J => "J",
            LaneKey::K => "K",
            LaneKey::L => "L",
            LaneKey::Semicolon => ";",
        }
    }
}

/// The keys of each lane, left to right.
pub fn lane_keys(d: Difficulty) -> &'static [LaneKey] {
    use LaneKey::*;
    match d {
        Difficulty::Easy => &[D, F, J, K],
        Difficulty::Normal => &[S, D, F, J, K, L],
        Difficulty::Hard => &[A, S, D, F, J, K, L, Semicolon],
    }
}

pub fn lanes(d: Difficulty) -> usize {
    lane_keys(d).len()
}

/// The lane `key` plays at this difficulty, if it plays one.
pub fn lane_of(d: Difficulty, key: LaneKey) -> Option<usize> {
    lane_keys(d).iter().position(|k| *k == key)
}

// ---- the songs and their charts ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub t: f32,
    pub lane: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub bpm: f32,
    /// When the first beat falls.
    pub offset: f32,
    /// How long the music is, in seconds.
    pub len: f32,
    notes: [Vec<Note>; 3],
}

fn slot(d: Difficulty) -> usize {
    match d {
        Difficulty::Easy => 0,
        Difficulty::Normal => 1,
        Difficulty::Hard => 2,
    }
}

impl Song {
    pub fn notes(&self, d: Difficulty) -> &[Note] {
        &self.notes[slot(d)]
    }

    /// A song made from its parts (the charts are for Easy, Normal and Hard, in that order).
    pub fn from_parts(title: &str, artist: &str, bpm: f32, offset: f32, len: f32, mut notes: [Vec<Note>; 3]) -> Song {
        for n in &mut notes {
            n.sort_by(|a, b| a.t.total_cmp(&b.t));
        }
        let clean = |s: &str| s.replace(['|', '\n', '\r'], "/").trim().to_string();
        Song { title: clean(title), artist: clean(artist), bpm, offset, len, notes }
    }

    /// The song in the chart file's format (`parse_charts` reads it back).
    pub fn to_text(&self) -> String {
        let mut out = format!("song {} | {} | {:.2} | {:.3} | {:.2}\n", self.title, self.artist, self.bpm, self.offset, self.len);
        for (name, notes) in ["easy", "normal", "hard"].iter().zip(&self.notes) {
            out.push_str(name);
            for n in notes {
                out.push_str(&format!(" {:.3}:{}", n.t, n.lane));
            }
            out.push('\n');
        }
        out
    }
}

/// Reads the chart file: `song title | artist | bpm | offset | length`, then an `easy`, a `normal`
/// and a `hard` line of `time:lane` pairs. Lines starting with `#` are notes to the reader.
pub fn parse_charts(text: &str) -> Vec<Song> {
    let mut songs: Vec<Song> = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if let Some(rest) = line.strip_prefix("song ") {
            let p: Vec<&str> = rest.split('|').map(str::trim).collect();
            if p.len() >= 5 {
                songs.push(Song {
                    title: p[0].to_string(),
                    artist: p[1].to_string(),
                    bpm: p[2].parse().unwrap_or(120.0),
                    offset: p[3].parse().unwrap_or(0.0),
                    len: p[4].parse().unwrap_or(60.0),
                    notes: [Vec::new(), Vec::new(), Vec::new()],
                });
            }
            continue;
        }
        let (name, body) = line.split_once(' ').unwrap_or((line, ""));
        let which = match name {
            "easy" => 0,
            "normal" => 1,
            "hard" => 2,
            _ => continue,
        };
        let Some(song) = songs.last_mut() else { continue };
        let mut notes: Vec<Note> = body
            .split_whitespace()
            .filter_map(|pair| {
                let (t, lane) = pair.split_once(':')?;
                Some(Note { t: t.parse().ok()?, lane: lane.parse().ok()? })
            })
            .collect();
        notes.sort_by(|a, b| a.t.total_cmp(&b.t));
        song.notes[which] = notes;
    }
    songs
}

/// The two songs of the story.
pub fn songs() -> &'static [Song] {
    static SONGS_ONCE: OnceLock<Vec<Song>> = OnceLock::new();
    SONGS_ONCE.get_or_init(|| parse_charts(include_str!("../assets/game3-charts.txt")))
}

/// What a difficulty plays like, in a line or two (shown on the screen before the run).
pub fn blurb(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "Four lanes under two fingers on each hand. Slow and sparse, one note at a time. A way to learn the songs.",
        Difficulty::Normal => "Six lanes for both hands, no thumbs. A steady stream with the odd two-note chord.",
        Difficulty::Hard => "All eight lanes, every finger. The fastest notes, with two-note chords coming often.",
    }
}

/// How many notes a difficulty has in each song.
pub fn note_counts(d: Difficulty) -> Vec<usize> {
    songs().iter().map(|s| s.notes(d).len()).collect()
}

/// Notes per second, over both songs.
pub fn density(d: Difficulty) -> f32 {
    let (notes, secs) = songs().iter().fold((0usize, 0.0f32), |(n, t), s| (n + s.notes(d).len(), t + s.len));
    if secs > 0.0 {
        notes as f32 / secs
    } else {
        0.0
    }
}

// ---- playing -------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    /// The song ended with patience left.
    Cleared,
    /// Patience ran out.
    Lost,
}

/// How a press or a note was judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judge {
    Perfect,
    Good,
    Miss,
    /// A key pressed with no note near it.
    Stray,
}

/// What clearing a song means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The first song: a checkpoint, and the second is next.
    Checkpoint,
    /// The second: he takes you, and The Underworld opens.
    Death,
    /// A song the player added, or a story song played to practise: nothing happens but the card.
    Practice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Waiting,
    Hit,
    Missed,
}

/// A note as the picture needs it: how many seconds until it reaches the line (negative once it
/// has gone past), its lane, and whether it was missed (a hit one is not shown).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Falling {
    pub dt: f32,
    pub lane: usize,
    pub missed: bool,
}

pub struct Game {
    /// Which song of the story this is (0 for the first; a song the player added is 0 too).
    pub stage: usize,
    pub difficulty: Difficulty,
    cheats: Cheats,
    /// The song's clock: it starts at minus `LEAD_IN`.
    pub t: f32,
    /// 0 to 100.
    pub patience: f32,
    pub score: u32,
    pub combo: u32,
    pub best: u32,
    /// Perfect, good and missed notes.
    pub counts: [u32; 3],
    pub strays: u32,
    /// How hard you have been mashing lately (0 to 6); it cools off.
    pub mash: f32,
    pub phase: Phase,
    /// How long ago the game ended.
    pub since_end: f32,
    /// The last thing judged: what, in which lane, and how long ago.
    pub judged: Option<(Judge, usize, f32)>,
    /// Per lane: how lit it is after a press or a hit (0 to 1).
    pub flash: [f32; MAX_LANES],
    /// Glitching and window shaking from misses (0 to 1).
    pub glitch: f32,
    pub shake: f32,
    /// A clean run makes his head flare (0 to 1).
    pub flare: f32,
    /// How far into his lunge he is once patience has run out (0 to 1).
    pub lunge: f32,
    notes: Vec<(Note, State)>,
    song_len: f32,
    song: Song,
    /// False for a song the player added: it is just played (no checkpoint, no ending).
    story: bool,
    death_reported: bool,
    /// The song was ended with the test switch (`finish_now`): it pays no tokens.
    skipped: bool,
}

impl Game {
    /// A run of song `stage` (0 is the first), with the cheats switched on for Game 3.
    pub fn new(stage: usize, difficulty: Difficulty, cheats: Cheats) -> Game {
        let stage = stage.min(SONGS - 1);
        let song = songs()[stage.min(songs().len().saturating_sub(1))].clone();
        Game::start(stage, song, true, difficulty, cheats)
    }

    /// A run of a song the player added (or of a story song, to practise): it ends in a card, and
    /// nothing is saved, counted or unlocked by it.
    pub fn custom(song: Song, difficulty: Difficulty, cheats: Cheats) -> Game {
        Game::start(0, song, false, difficulty, cheats)
    }

    fn start(stage: usize, song: Song, story: bool, difficulty: Difficulty, cheats: Cheats) -> Game {
        Game {
            stage,
            difficulty,
            cheats,
            t: -LEAD_IN,
            patience: 100.0,
            score: 0,
            combo: 0,
            best: 0,
            counts: [0; 3],
            strays: 0,
            mash: 0.0,
            phase: Phase::Playing,
            since_end: 0.0,
            judged: None,
            flash: [0.0; MAX_LANES],
            glitch: 0.0,
            shake: 0.0,
            flare: 0.0,
            lunge: 0.0,
            notes: song.notes(difficulty).iter().map(|n| (*n, State::Waiting)).collect(),
            song_len: song.len,
            song,
            story,
            death_reported: false,
            skipped: false,
        }
    }

    pub fn song(&self) -> &Song {
        &self.song
    }

    /// Whether this is the story (as opposed to a song the player added).
    pub fn story(&self) -> bool {
        self.story
    }

    /// Whether a cheat was on, or the song was ended with the test switch: it pays no tokens.
    pub fn cheated(&self) -> bool {
        self.cheats != Cheats::NONE || self.skipped
    }

    /// How long the song is, in seconds.
    pub fn song_len(&self) -> f32 {
        self.song_len
    }

    pub fn lanes(&self) -> usize {
        lanes(self.difficulty)
    }

    pub fn cheats(&self) -> Cheats {
        self.cheats
    }

    pub fn total_notes(&self) -> usize {
        self.notes.len()
    }

    /// 0 to 1: how far through the music.
    pub fn progress(&self) -> f32 {
        (self.t / self.song_len.max(1.0)).clamp(0.0, 1.0)
    }

    /// The notes to draw right now: those within `APPROACH` of the line, and those just gone by.
    pub fn falling(&self) -> Vec<Falling> {
        let mut out = Vec::new();
        for (n, s) in &self.notes {
            let dt = n.t - self.t;
            if dt > APPROACH {
                break;
            }
            if dt < -0.25 || *s == State::Hit {
                continue;
            }
            out.push(Falling { dt, lane: n.lane as usize, missed: *s == State::Missed });
        }
        out
    }

    /// Rank, for show only: S, A, B or C by accuracy.
    pub fn accuracy(&self) -> f32 {
        let total = self.counts.iter().sum::<u32>();
        if total == 0 {
            0.0
        } else {
            (self.counts[0] as f32 + 0.5 * self.counts[1] as f32) / total as f32
        }
    }

    pub fn rank(&self) -> char {
        let a = self.accuracy();
        if a > 0.95 {
            'S'
        } else if a > 0.88 {
            'A'
        } else if a > 0.75 {
            'B'
        } else {
            'C'
        }
    }

    /// What clearing this song means (None until it is cleared).
    pub fn outcome(&self) -> Option<Outcome> {
        (self.phase == Phase::Cleared).then(|| match (self.story, self.stage) {
            (false, _) => Outcome::Practice,
            (true, 0) => Outcome::Checkpoint,
            (true, _) => Outcome::Death,
        })
    }

    /// Ends the song as cleared right now, as if its last note had gone by with patience left.
    /// Only for the test switch in Options (Take life), which is to go.
    pub fn finish_now(&mut self) {
        self.skipped = true;
        self.t = self.song_len + TAIL;
        self.phase = Phase::Cleared;
        self.since_end = 0.0;
    }

    /// True once, the first time it is asked after patience ran out: for counting deaths.
    pub fn take_death(&mut self) -> bool {
        if self.phase == Phase::Lost && !self.death_reported {
            self.death_reported = true;
            return true;
        }
        false
    }

    /// Moves the clock on by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        self.advance_to(self.t + dt);
    }

    /// Moves the clock to `t` (it never goes back; a long gap is cut to a quarter of a second so
    /// that a stutter does not miss a hundred notes at once).
    pub fn advance_to(&mut self, t: f32) {
        let dt = (t - self.t).clamp(0.0, 0.25);
        self.t += dt;
        let t = self.t;
        self.glitch = (self.glitch - 2.7 * dt).max(0.0);
        self.shake = (self.shake - 3.0 * dt).max(0.0);
        self.mash = (self.mash - 0.72 * dt).max(0.0);
        self.flare = (self.flare - 0.048 * dt).max(0.0);
        for f in &mut self.flash {
            *f = (*f - 4.2 * dt).max(0.0);
        }
        if let Some((_, _, age)) = &mut self.judged {
            *age += dt;
        }
        match self.phase {
            Phase::Lost => {
                self.since_end += dt;
                self.lunge = (self.lunge + 2.7 * dt).min(1.0);
                return;
            }
            Phase::Cleared => {
                self.since_end += dt;
                return;
            }
            Phase::Playing => {}
        }
        // Perfect score: every note is hit exactly on time, whatever the player does
        if self.cheats.has(Cheat::PerfectScore) {
            for i in 0..self.notes.len() {
                let (n, s) = self.notes[i];
                if n.t > t + 0.004 {
                    break;
                }
                if s == State::Waiting {
                    self.notes[i].1 = State::Hit;
                    self.judge(Judge::Perfect, n.lane as usize);
                }
            }
        }
        // notes that have gone by unplayed
        for i in 0..self.notes.len() {
            let (n, s) = self.notes[i];
            if n.t > t - GOOD {
                break;
            }
            if s == State::Waiting {
                self.notes[i].1 = State::Missed;
                self.judge(Judge::Miss, n.lane as usize);
            }
        }
        if self.patience <= 0.0 {
            self.phase = Phase::Lost;
            self.since_end = 0.0;
            self.glitch = self.glitch.max(0.5);
        } else if t > self.song_len + TAIL {
            self.phase = Phase::Cleared;
            self.since_end = 0.0;
        }
    }

    /// A key was pressed for `lane`, at the clock's time now. Returns how it was judged (None when
    /// the game is not being played, or the Perfect score cheat plays for you).
    pub fn press(&mut self, lane: usize) -> Option<Judge> {
        if self.phase != Phase::Playing || self.cheats.has(Cheat::PerfectScore) || lane >= self.lanes() {
            return None;
        }
        self.flash[lane] = self.flash[lane].max(0.55);
        let t = self.t;
        let mut best: Option<(usize, f32)> = None;
        for (i, (n, s)) in self.notes.iter().enumerate() {
            if n.t - t > GOOD {
                break;
            }
            if *s != State::Waiting || n.lane as usize != lane {
                continue;
            }
            let d = (n.t - t).abs();
            if d <= GOOD && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        match best {
            Some((i, d)) => {
                self.notes[i].1 = State::Hit;
                let kind = if d <= PERFECT { Judge::Perfect } else { Judge::Good };
                self.judge(kind, lane);
                Some(kind)
            }
            None => self.stray(lane),
        }
    }

    /// A press with no note near it: it costs patience (the cost climbs while it keeps happening),
    /// unless the No button-mashing penalty cheat is on.
    fn stray(&mut self, lane: usize) -> Option<Judge> {
        if self.cheats.has(Cheat::NoMashPenalty) {
            return None;
        }
        self.strays += 1;
        self.mash = (self.mash + 1.0).min(6.0);
        let cost = (1.0 + (self.mash - 1.0) * 1.6).min(10.0);
        self.patience = (self.patience - cost).max(0.0);
        self.combo = 0;
        self.glitch = self.glitch.max(0.35 + 0.1 * self.mash).min(1.0);
        self.shake = self.shake.max(0.25 + 0.08 * self.mash).min(1.0);
        self.judged = Some((Judge::Stray, lane, 0.0));
        Some(Judge::Stray)
    }

    fn judge(&mut self, kind: Judge, lane: usize) {
        self.judged = Some((kind, lane, 0.0));
        match kind {
            Judge::Perfect | Judge::Good => {
                let perfect = kind == Judge::Perfect;
                self.counts[if perfect { 0 } else { 1 }] += 1;
                self.combo += 1;
                self.best = self.best.max(self.combo);
                let mult = 1.0 + (self.combo / 10).min(10) as f32 * 0.1;
                self.score += ((if perfect { 300.0 } else { 100.0 }) * mult).round() as u32;
                self.patience = (self.patience + if perfect { 1.2 } else { 0.4 }).min(100.0);
                if let Some(f) = self.flash.get_mut(lane) {
                    *f = 1.0;
                }
                self.flare = (self.flare + 0.05).min(1.0);
            }
            Judge::Miss => {
                self.counts[2] += 1;
                self.combo = 0;
                self.patience = (self.patience - 8.0).max(0.0);
                self.glitch = 1.0;
                self.shake = 1.0;
                self.flare = 0.0;
            }
            Judge::Stray => {}
        }
    }
}

// ---- the ending ----------------------------------------------------------------------------------

/// What happens when the second song is cleared: he creeps out of the portal while the user's heart
/// beats, lunges, and drives a talon through it. Times are seconds from the start of the ending;
/// the monarch himself is drawn by the same scene as the portal (`monarch::beat`), at `scene_time`.
pub mod ending {
    use crate::monarch;

    /// When the lunge peaks, and when the talon goes through the heart.
    pub const LUNGE: f32 = 4.06;
    pub const STRIKE: f32 = 4.34;
    /// When the hand starts coming down.
    pub const HAND_FROM: f32 = 3.7;
    /// After the strike: when each line starts to be written, when the lines start to fade (and
    /// how long that takes), and when the Underworld shows.
    pub const LINE1_AT: f32 = 1.15;
    pub const LINE2_AT: f32 = 2.65;
    pub const LINES_FADE_AT: f32 = 6.3;
    pub const LINES_FADE: f32 = 0.6;
    pub const UNDERWORLD_AT: f32 = 7.1;
    pub const LINE1: &str = "You were patient.";
    pub const LINE2: &str = "It does not matter.";
    /// Letters per second.
    pub const CPS: f32 = 14.0;

    fn smooth(x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }

    /// When the heart beats (the start of each lub), faster as he comes; the last is well before
    /// the strike, so that the heart is quiet when it is hit.
    pub fn heart_beats() -> Vec<f32> {
        let mut out = Vec::new();
        let mut t = 0.35;
        while t < STRIKE - 0.45 {
            out.push(t);
            t += 1.15 - 0.2 * (t / STRIKE);
        }
        out
    }

    /// How much the heart swells (0 to 1) `r` seconds in: a lub, then a smaller dub 0.2 s later.
    pub fn heart_pulse(r: f32) -> f32 {
        let mut p: f32 = 0.0;
        for b in heart_beats() {
            let d = r - b;
            if (0.0..1.2).contains(&d) {
                p = p.max((-d * 9.0).exp());
                let e = d - 0.2;
                if e >= 0.0 {
                    p = p.max(0.65 * (-e * 10.0).exp());
                }
            }
        }
        p
    }

    /// How visible the heart is: it comes up slowly, and is gone from the strike.
    pub fn heart_alpha(r: f32) -> f32 {
        if r >= STRIKE {
            0.0
        } else {
            (r / 2.4).clamp(0.0, 1.0) * 0.78
        }
    }

    /// The monarch scene's own time (`monarch::beat`) for `r` seconds into the ending: he creeps from
    /// the middle of his pull to the peak of the lunge, more slowly than the scene was written.
    pub fn scene_time(r: f32) -> f32 {
        monarch::PULL_AT + 1.6 + r.clamp(0.0, LUNGE) * (1.65 / LUNGE)
    }

    /// The talon: how far down it has come (0 to 1) and whether it has closed on the heart. None
    /// until it starts to come down.
    pub fn hand(r: f32) -> Option<(f32, bool)> {
        if r < HAND_FROM {
            return None;
        }
        let p = ((r - HAND_FROM) / (STRIKE - HAND_FROM)).clamp(0.0, 1.0);
        Some((p.powf(2.2), r >= STRIKE))
    }

    /// How far the heart's burst has got, seconds since the strike (None before it, and after it is gone).
    pub fn burst(r: f32) -> Option<f32> {
        let since = r - STRIKE;
        (0.0..1.2).contains(&since).then_some(since)
    }

    /// The white flash at the strike (0 to 1) and the dark that follows it (0 to 1).
    pub fn flash(r: f32) -> f32 {
        let since = r - STRIKE;
        if since > 0.0 {
            (1.0 - since * 5.0).max(0.0)
        } else {
            0.0
        }
    }

    pub fn black(r: f32) -> f32 {
        smooth((r - STRIKE - 0.05) / 0.7)
    }

    /// How much of `text` is written `since` seconds after the strike, when it starts at `start`.
    pub fn written(text: &str, since: f32, start: f32) -> usize {
        (((since - start) * CPS).floor().max(0.0) as usize).min(text.chars().count())
    }

    /// How visible his two lines are (they fade together, slowly, once both are written).
    pub fn lines_alpha(r: f32) -> f32 {
        let since = r - STRIKE;
        if since < LINE1_AT || since >= UNDERWORLD_AT {
            0.0
        } else {
            1.0 - smooth((since - LINES_FADE_AT) / LINES_FADE)
        }
    }

    /// How visible the Underworld's name is.
    pub fn underworld_alpha(r: f32) -> f32 {
        smooth((r - STRIKE - UNDERWORLD_AT) / 1.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HARD: Difficulty = Difficulty::Hard;

    fn game(stage: usize, d: Difficulty) -> Game {
        Game::new(stage, d, Cheats::NONE)
    }

    /// Plays every note of the song perfectly, as the clock gets to it.
    fn play_perfectly(g: &mut Game) {
        let notes: Vec<Note> = g.song().notes(g.difficulty).to_vec();
        let mut i = 0;
        while g.phase == Phase::Playing {
            g.step(1.0 / 120.0);
            while i < notes.len() && notes[i].t <= g.t {
                g.t = g.t.max(notes[i].t);
                g.press(notes[i].lane as usize);
                i += 1;
            }
        }
    }

    #[test]
    fn a_song_is_written_and_read_back() {
        let note = |t, lane| Note { t, lane };
        let song = Song::from_parts(
            "A | Song",
            "Some\nBody",
            123.456,
            0.25,
            61.5,
            [vec![note(2.0, 1), note(1.0, 0)], vec![note(1.5, 5)], vec![note(0.5, 7), note(0.5, 2)]],
        );
        assert_eq!((song.title.as_str(), song.artist.as_str()), ("A / Song", "Some/Body"));
        assert_eq!(song.notes(Difficulty::Easy)[0].t, 1.0, "kept in time order");
        let back = parse_charts(&song.to_text());
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].title, song.title);
        assert!((back[0].bpm - 123.46).abs() < 0.01 && (back[0].len - 61.5).abs() < 0.01);
        for d in Difficulty::ALL {
            assert_eq!(back[0].notes(d), song.notes(d));
        }
    }

    #[test]
    fn a_song_the_player_added_ends_in_a_card_and_never_in_the_story() {
        let note = |t, lane| Note { t, lane };
        let song = Song::from_parts("Mine", "Me", 120.0, 0.0, 6.0, [vec![note(1.0, 0)], vec![note(1.0, 0)], vec![note(1.0, 0)]]);
        let mut g = Game::custom(song, HARD, Cheats::NONE);
        assert!(!g.story());
        assert_eq!(g.song().title, "Mine");
        assert_eq!(g.total_notes(), 1);
        for _ in 0..(10.0 / 0.05) as usize {
            g.step(0.05);
        }
        // the one note went by (a miss costs 8), and the song ended with patience left
        assert_eq!(g.phase, Phase::Cleared);
        assert_eq!(g.outcome(), Some(Outcome::Practice));
    }

    #[test]
    fn harder_difficulties_have_more_notes_and_every_note_has_a_lane() {
        let (e, n, h) = (density(Difficulty::Easy), density(Difficulty::Normal), density(HARD));
        assert!(e < n && n < h, "{e} {n} {h}");
        for d in Difficulty::ALL {
            assert_eq!(note_counts(d).len(), SONGS);
            for s in songs() {
                assert!(s.notes(d).iter().all(|note| (note.lane as usize) < lanes(d)));
                assert!((0..lanes(d)).all(|l| s.notes(d).iter().any(|note| note.lane as usize == l)), "every lane is used");
            }
            assert!(!blurb(d).is_empty());
        }
    }

    #[test]
    fn the_difficulty_decides_the_lanes_and_their_keys() {
        assert_eq!(lanes(Difficulty::Easy), 4);
        assert_eq!(lanes(Difficulty::Normal), 6);
        assert_eq!(lanes(HARD), 8);
        let names = |d| lane_keys(d).iter().map(|k| k.label()).collect::<Vec<_>>();
        assert_eq!(names(Difficulty::Easy), vec!["D", "F", "J", "K"]);
        assert_eq!(names(Difficulty::Normal), vec!["S", "D", "F", "J", "K", "L"]);
        assert_eq!(names(HARD), vec!["A", "S", "D", "F", "J", "K", "L", ";"]);
        assert_eq!(lane_of(Difficulty::Normal, LaneKey::J), Some(3));
        assert_eq!(lane_of(Difficulty::Normal, LaneKey::A), None, "no pinky-A on Normal");
        assert_eq!(lane_of(Difficulty::Easy, LaneKey::A), None);
        assert_eq!(lane_of(HARD, LaneKey::Semicolon), Some(7));
    }

    #[test]
    fn the_charts_are_read_for_both_songs_and_every_difficulty() {
        let s = songs();
        assert_eq!(s.len(), SONGS);
        assert!(s[0].len > 90.0 && s[0].len < 110.0 && s[1].len > 150.0, "the second is the longer one");
        assert!(s[0].len < s[1].len);
        for song in s {
            assert!(!song.title.is_empty() && !song.artist.is_empty() && song.bpm > 60.0);
            let counts: Vec<usize> = Difficulty::ALL.iter().map(|d| song.notes(*d).len()).collect();
            assert!(counts[0] > 100 && counts[0] < counts[1] && counts[1] < counts[2], "{counts:?}");
            for d in Difficulty::ALL {
                let notes = song.notes(d);
                assert!(notes.windows(2).all(|w| w[0].t <= w[1].t), "in time order");
                assert!(notes.iter().all(|n| (n.lane as usize) < lanes(d)), "every note has a lane");
                assert!(notes.first().unwrap().t >= 0.0 && notes.last().unwrap().t < song.len, "inside the song");
                // every lane is used
                for l in 0..lanes(d) {
                    assert!(notes.iter().any(|n| n.lane as usize == l), "lane {l} of {d:?} is never used");
                }
            }
        }
    }

    #[test]
    fn the_chart_reader_takes_what_it_understands_and_skips_the_rest() {
        let text = "# a comment\nsong A Title | An Artist | 120 | 0.5 | 30\neasy 1.0:0 2.0:3 bad 0.5:1\nnormal\nhard 3.0:7\nunknown 1:1\n";
        let s = parse_charts(text);
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].title.as_str(), s[0].artist.as_str(), s[0].bpm, s[0].offset, s[0].len), ("A Title", "An Artist", 120.0, 0.5, 30.0));
        assert_eq!(s[0].notes(Difficulty::Easy), &[Note { t: 0.5, lane: 1 }, Note { t: 1.0, lane: 0 }, Note { t: 2.0, lane: 3 }][..], "sorted, the bad pair skipped");
        assert!(s[0].notes(Difficulty::Normal).is_empty());
        assert_eq!(s[0].notes(Difficulty::Hard).len(), 1);
        assert!(parse_charts("easy 1:1\n").is_empty(), "a chart with no song is nothing");
    }

    #[test]
    fn a_hit_is_perfect_or_good_by_how_close_it_is_and_a_note_is_hit_once() {
        let mut g = game(0, HARD);
        let n = g.song().notes(HARD)[0];
        g.t = n.t - 0.02;
        assert_eq!(g.press(n.lane as usize), Some(Judge::Perfect));
        assert_eq!(g.counts, [1, 0, 0]);
        assert_eq!(g.press(n.lane as usize), Some(Judge::Stray), "it is gone: the same press is now a stray");
        let mut g = game(0, HARD);
        g.t = n.t + 0.08;
        assert_eq!(g.press(n.lane as usize), Some(Judge::Good));
        assert_eq!(g.counts, [0, 1, 0]);
        assert!(g.score == 100 && g.combo == 1);
    }

    #[test]
    fn a_note_nobody_plays_is_missed_and_costs_patience_and_the_combo() {
        let mut g = game(0, HARD);
        g.combo = 5;
        let n = g.song().notes(HARD)[0];
        g.t = n.t - 0.05;
        g.step(0.2);
        assert_eq!(g.counts[2], 1);
        assert_eq!(g.combo, 0);
        assert!((g.patience - 92.0).abs() < 1e-3);
        assert_eq!(g.glitch.max(g.shake) > 0.5, true);
        assert_eq!(g.judged.map(|j| j.0), Some(Judge::Miss));
    }

    #[test]
    fn a_clean_stretch_gives_patience_back_and_the_combo_makes_the_score_grow() {
        let mut g = game(0, HARD);
        g.patience = 50.0;
        g.combo = 29;
        let notes = g.song().notes(HARD).to_vec();
        g.t = notes[0].t;
        g.press(notes[0].lane as usize);
        assert!((g.patience - 51.2).abs() < 1e-4);
        assert_eq!(g.score, 390, "combo 30 is worth 1.3 times: 300 x 1.3");
        g.patience = 99.9;
        g.t = notes[1].t;
        g.press(notes[1].lane as usize);
        assert_eq!(g.patience, 100.0, "it never goes past full");
    }

    #[test]
    fn pressing_a_key_with_no_note_near_it_costs_more_each_time_and_cools_off() {
        let mut g = game(0, HARD);
        // nothing is close to the start: the first notes are further off than the good window
        let mut costs = Vec::new();
        for _ in 0..4 {
            let before = g.patience;
            assert_eq!(g.press(0), Some(Judge::Stray));
            costs.push(before - g.patience);
        }
        assert!((costs[0] - 1.0).abs() < 1e-4, "{costs:?}");
        assert!(costs.windows(2).all(|w| w[1] > w[0] + 1.0), "the cost climbs: {costs:?}");
        assert_eq!(g.strays, 4);
        assert_eq!(g.combo, 0);
        // a long way into a run, hammering ends it
        let mut g = game(0, HARD);
        let mut presses = 0;
        while g.patience > 0.0 && presses < 100 {
            g.press(presses % 8);
            presses += 1;
        }
        assert!(presses <= 14, "{presses} presses to run out");
        // it cools off: after a few quiet seconds a stray costs the least again
        let mut g = game(0, HARD);
        for _ in 0..3 {
            g.press(0);
        }
        g.t = 0.0;
        g.step(0.2);
        g.t += 0.0;
        for _ in 0..40 {
            g.step(0.1);
            // keep it from dying of misses: this part only watches the cooling
            g.patience = 100.0;
        }
        assert!(g.mash < 0.01, "{}", g.mash);
    }

    #[test]
    fn running_out_of_patience_loses_and_the_death_is_counted_once() {
        let mut g = game(0, HARD);
        g.patience = 5.0;
        g.t = g.song().notes(HARD)[0].t + 0.2;
        g.step(0.01);
        assert_eq!(g.phase, Phase::Lost);
        assert!(g.take_death());
        assert!(!g.take_death());
        assert_eq!(g.outcome(), None);
        for _ in 0..30 {
            g.step(0.1);
        }
        assert_eq!(g.lunge, 1.0);
        assert_eq!(g.press(0), None, "nothing more can be played");
    }

    #[test]
    fn clearing_the_first_song_is_a_checkpoint_and_the_second_is_the_death_however_well_you_play() {
        for (stage, outcome) in [(0, Outcome::Checkpoint), (1, Outcome::Death)] {
            let mut g = game(stage, Difficulty::Easy);
            play_perfectly(&mut g);
            assert_eq!(g.phase, Phase::Cleared, "stage {stage}");
            assert_eq!(g.outcome(), Some(outcome));
            assert_eq!(g.counts[2], 0);
            assert!(g.rank() == 'S' && g.strays == 0);
            assert!(!g.take_death(), "clearing is not a death by itself");
        }
    }

    #[test]
    fn a_sloppy_clear_wins_just_the_same_and_only_the_rank_differs() {
        let mut g = game(0, Difficulty::Easy);
        // play every third note, and nothing else
        let notes: Vec<Note> = g.song().notes(Difficulty::Easy).to_vec();
        let mut i = 0;
        while g.phase == Phase::Playing && i < 100_000 {
            g.step(1.0 / 120.0);
            g.patience = g.patience.max(30.0); // sloppy, not dead (this test is about the end)
            while i < notes.len() && notes[i].t <= g.t {
                if i % 3 == 0 {
                    g.press(notes[i].lane as usize);
                }
                i += 1;
            }
        }
        assert_eq!(g.phase, Phase::Cleared);
        assert_eq!(g.outcome(), Some(Outcome::Checkpoint));
        assert_eq!(g.rank(), 'C');
    }

    #[test]
    fn the_notes_to_draw_are_those_about_to_arrive_and_those_just_missed() {
        let mut g = game(0, Difficulty::Easy);
        assert!(g.falling().is_empty(), "nothing is near at the start");
        let n = g.song().notes(Difficulty::Easy)[0];
        g.t = n.t - 1.0;
        let f = g.falling();
        let near = g.song().notes(Difficulty::Easy).iter().filter(|m| m.t - g.t <= APPROACH).count();
        assert_eq!(f.len(), near);
        assert!(f.iter().any(|x| (x.dt - 1.0).abs() < 1e-4 && x.lane == n.lane as usize && !x.missed));
        g.t = n.t - 0.01;
        g.press(n.lane as usize);
        assert!(g.falling().iter().all(|x| (x.dt - 0.01).abs() > 1e-4), "a hit note is gone");
        let mut g = game(0, Difficulty::Easy);
        g.t = n.t + 0.15;
        g.step(0.01);
        assert_eq!(g.falling().iter().filter(|f| f.missed).count(), 1, "a missed one drifts by, dim");
    }

    #[test]
    fn the_perfect_score_cheat_plays_every_note_perfectly_and_ignores_presses() {
        let mut g = Game::new(0, HARD, Cheats::NONE.with(Cheat::PerfectScore, true));
        assert_eq!(g.press(0), None);
        while g.phase == Phase::Playing {
            g.step(1.0 / 60.0);
        }
        assert_eq!(g.counts[0] as usize, g.total_notes());
        assert_eq!((g.counts[1], g.counts[2], g.strays), (0, 0, 0));
        assert_eq!(g.patience, 100.0);
        assert_eq!(g.outcome(), Some(Outcome::Checkpoint), "it only gets you to the end");
    }

    #[test]
    fn the_perfect_score_cheat_survives_a_stutter_in_the_frame_rate() {
        let mut g = Game::new(0, HARD, Cheats::NONE.with(Cheat::PerfectScore, true));
        while g.phase == Phase::Playing {
            g.step(0.24); // frames a quarter of a second long
        }
        assert_eq!(g.counts[2], 0, "no note is missed even with huge frames");
    }

    #[test]
    fn the_no_mashing_cheat_makes_a_stray_press_cost_nothing() {
        let mut g = Game::new(0, HARD, Cheats::NONE.with(Cheat::NoMashPenalty, true));
        for k in 0..50 {
            assert_eq!(g.press(k % 8), None);
        }
        assert_eq!((g.patience, g.strays, g.mash), (100.0, 0, 0.0));
        // real hits and misses still count
        g.t = g.song().notes(HARD)[0].t;
        assert!(g.press(g.song().notes(HARD)[0].lane as usize).is_some());
    }

    #[test]
    fn a_lane_that_is_not_there_is_ignored() {
        let mut g = game(0, Difficulty::Easy);
        assert_eq!(g.press(4), None);
        assert_eq!(g.press(9), None);
        assert_eq!(g.strays, 0);
    }

    #[test]
    fn a_long_frame_does_not_skip_through_the_song() {
        let mut g = game(0, HARD);
        g.step(10.0);
        assert!(g.t <= -LEAD_IN + 0.25 + 1e-4);
    }

    #[test]
    fn finishing_now_in_the_second_song_is_the_kill() {
        let mut g = Game::new(SONGS - 1, HARD, Cheats::default());
        assert_eq!(g.outcome(), None);
        assert!(!g.cheated());
        g.finish_now();
        assert!(g.cheated(), "the test switch pays no tokens");
        assert_eq!(g.outcome(), Some(Outcome::Death));
        assert!(!g.take_death(), "it is a clear, not a loss");
        let mut first = Game::new(0, HARD, Cheats::default());
        first.finish_now();
        assert_eq!(first.outcome(), Some(Outcome::Checkpoint));
    }

    #[test]
    fn the_ending_creeps_lunges_strikes_and_then_writes_his_words() {
        use ending::*;
        // the heart beats, faster as he comes, and is quiet before the strike
        let beats = heart_beats();
        assert!(beats.len() >= 4 && beats[0] > 0.2);
        let gaps: Vec<f32> = beats.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.windows(2).all(|w| w[1] < w[0]), "quickening: {gaps:?}");
        assert!(beats.iter().all(|b| *b < STRIKE - 0.4));
        assert!(heart_pulse(beats[1] + 0.001) > 0.9 && heart_pulse(beats[1] + 0.2 + 0.001) > 0.5, "lub and dub");
        assert!(heart_pulse(STRIKE + 0.5) < 0.01, "it does not beat again");
        assert!(heart_alpha(0.0) < 0.01 && heart_alpha(3.0) > 0.7 && heart_alpha(STRIKE) == 0.0);
        // him: the scene's time runs from mid-pull to the peak of the lunge, and stays there
        assert!((scene_time(LUNGE) - (crate::monarch::PULL_AT + 3.25)).abs() < 1e-3);
        assert_eq!(scene_time(LUNGE + 5.0), scene_time(LUNGE));
        assert!(crate::monarch::beat(scene_time(LUNGE)).over > 0.99, "he is right up at the screen");
        // the talon comes down, closes at the strike
        assert_eq!(hand(1.0), None);
        let (a, closed_a) = hand(HAND_FROM + 0.2).unwrap();
        let (b, closed_b) = hand(STRIKE - 0.01).unwrap();
        assert!(a < b && b > 0.9 && !closed_a && !closed_b);
        assert_eq!(hand(STRIKE).map(|h| h.1), Some(true));
        assert!(burst(STRIKE - 0.1).is_none() && burst(STRIKE + 0.3).is_some() && burst(STRIKE + 2.0).is_none());
        // flash then dark
        assert!(flash(STRIKE - 0.1) == 0.0 && flash(STRIKE + 0.01) > 0.9 && flash(STRIKE + 0.5) == 0.0);
        assert!(black(STRIKE) == 0.0 && black(STRIKE + 1.0) == 1.0);
    }

    #[test]
    fn his_lines_are_fully_written_and_held_before_they_fade_and_the_underworld_follows() {
        use ending::*;
        // the whole second line is written well before the lines start to go
        let finishes = LINE2_AT + LINE2.chars().count() as f32 / CPS;
        assert!(finishes + 1.5 < LINES_FADE_AT, "{finishes}: it stays up at least a second and a half once written");
        assert_eq!(written(LINE2, finishes + 0.01, LINE2_AT), LINE2.chars().count());
        assert_eq!(written(LINE2, LINE2_AT - 1.0, LINE2_AT), 0);
        assert_eq!(written(LINE1, LINE1_AT + 0.5, LINE1_AT), 7);
        // visible from the first line until the Underworld: full until the fade, then going
        let r = |since: f32| STRIKE + since;
        assert_eq!(lines_alpha(r(LINE1_AT - 0.1)), 0.0);
        assert_eq!(lines_alpha(r(finishes + 1.0)), 1.0);
        assert!(lines_alpha(r(LINES_FADE_AT + LINES_FADE * 0.5)) < 0.9);
        assert!(lines_alpha(r(LINES_FADE_AT + LINES_FADE + 0.01)) < 0.001);
        assert!(UNDERWORLD_AT > LINES_FADE_AT + LINES_FADE);
        assert!(underworld_alpha(r(UNDERWORLD_AT - 0.1)) == 0.0 && underworld_alpha(r(UNDERWORLD_AT + 2.0)) == 1.0);
    }
}
