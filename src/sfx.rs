// The short sound effects of the Devil's Table. Each one is an mp3 in assets/sfx/ that is built into
// the program (replace the file to change the sound); it is decoded the first time it is played and
// kept. They go through the same sound card thread as everything else (see audio.rs), at the volume
// of the Games volume slider, and nothing here fails if there is no sound card.
use crate::audio::{self, Audio};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    /// Chips handed to you: your stack at the start of the game, and a pot you win.
    Chips,
    /// The deck shuffled, as a hand begins.
    Shuffle,
    /// A card put down on the felt. There are four takes of it; `n` picks one (any number will do).
    Card(usize),
}

static CHIPS_MP3: &[u8] = include_bytes!("../assets/sfx/chips.mp3");
static SHUFFLE_MP3: &[u8] = include_bytes!("../assets/sfx/shuffle.mp3");
static CARD_MP3: [&[u8]; CARD_TAKES] = [
    include_bytes!("../assets/sfx/card1.mp3"),
    include_bytes!("../assets/sfx/card2.mp3"),
    include_bytes!("../assets/sfx/card3.mp3"),
    include_bytes!("../assets/sfx/card4.mp3"),
];
pub const CARD_TAKES: usize = 4;

type Cell = OnceLock<Option<Vec<f32>>>;

fn samples(sfx: Sfx) -> Option<&'static [f32]> {
    static CHIPS: Cell = OnceLock::new();
    static SHUFFLE: Cell = OnceLock::new();
    static CARDS: [Cell; CARD_TAKES] = [OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new()];
    match sfx {
        Sfx::Chips => CHIPS.get_or_init(|| audio::decode_mono(CHIPS_MP3)).as_deref(),
        Sfx::Shuffle => SHUFFLE.get_or_init(|| audio::decode_mono(SHUFFLE_MP3)).as_deref(),
        Sfx::Card(n) => {
            let i = n % CARD_TAKES;
            CARDS[i].get_or_init(|| audio::decode_mono(CARD_MP3[i])).as_deref()
        }
    }
}

/// Plays `sfx` now (nothing happens if it cannot be decoded).
pub fn play(audio: &Audio, sfx: Sfx) {
    if let Some(s) = samples(sfx) {
        audio.play_effect(s.to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_in_the_program_can_be_decoded() {
        for sfx in [Sfx::Chips, Sfx::Shuffle, Sfx::Card(0), Sfx::Card(1), Sfx::Card(2), Sfx::Card(3), Sfx::Card(9)] {
            let s = samples(sfx).unwrap_or_else(|| panic!("{sfx:?} does not decode"));
            assert!(s.len() > 1000, "{sfx:?} is {} samples", s.len());
            let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.1 && peak <= 1.01, "{sfx:?} peaks at {peak}");
        }
    }

    #[test]
    fn a_card_is_short_and_a_shuffle_is_longer() {
        let secs = |s: Sfx| samples(s).unwrap().len() as f32 / crate::synth::SR as f32;
        assert!(secs(Sfx::Card(0)) < 0.5);
        assert!(secs(Sfx::Shuffle) > secs(Sfx::Card(0)));
    }
}
