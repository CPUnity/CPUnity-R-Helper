// Sound for Game 3 and the scenes around it: one song at a time, played from the file inside the
// program, and short pre-rendered effects (see game3_sound.rs). The sound card is owned by a thread
// of its own, because the output stream cannot be moved between threads; the program talks to it
// through a channel and never waits for it. Without a sound card nothing here fails: `new` gives
// None, and the game runs on its own clock.
use crate::{game3_sound, synth};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The two songs of Game 3, as they are in the program (see assets/game3-charts.txt for the notes).
pub static SONG_MP3: [&[u8]; 2] = [include_bytes!("../assets/game3-song1.mp3"), include_bytes!("../assets/game3-song2.mp3")];
/// The organ that fills the Void when the staff breaks, and the sound of the Underworld opening.
static ORGAN_MP3: &[u8] = include_bytes!("../assets/game3-organ.mp3");
static UNDERWORLD_MP3: &[u8] = include_bytes!("../assets/underworld-intro.mp3");

enum Cmd {
    /// Plays an mp3 after `lead_in` seconds of quiet.
    Track { bytes: Arc<[u8]>, lead_in: f32 },
    StopTrack,
    /// Mono samples at `synth::SR`.
    Effect(Vec<f32>),
    StopEffects,
    Volume(f32),
}

/// When the current track was started (and how long its quiet lead-in is).
type Clock = Arc<Mutex<Option<(Instant, f32)>>>;

/// The long effects, which take a moment to make: they are made ahead of time where they can be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scene {
    /// The portal opening and the monarch coming out of it.
    Portal,
    /// The monarch's kill and the Underworld opening.
    Ending,
}

enum Slot {
    Empty,
    /// Being made; `Some(g)` if it is to be played as soon as it is done (unless the effects were
    /// stopped since generation `g`).
    Rendering(Option<u32>),
    Ready(Vec<f32>),
}

#[derive(Clone)]
pub struct Audio {
    tx: Sender<Cmd>,
    clock: Clock,
    slots: Arc<[Mutex<Slot>; 2]>,
    /// Counts the times the effects were stopped, so that a late effect is not played after that.
    generation: Arc<AtomicU32>,
}

fn clip(cell: &'static OnceLock<Option<Vec<f32>>>, bytes: &'static [u8]) -> Option<&'static [f32]> {
    cell.get_or_init(|| decode_mono(bytes)).as_deref()
}

fn render(scene: Scene) -> Vec<f32> {
    static ORGAN: OnceLock<Option<Vec<f32>>> = OnceLock::new();
    static UNDERWORLD: OnceLock<Option<Vec<f32>>> = OnceLock::new();
    match scene {
        Scene::Portal => game3_sound::portal(clip(&ORGAN, ORGAN_MP3)),
        Scene::Ending => game3_sound::ending_sound(clip(&UNDERWORLD, UNDERWORLD_MP3)),
    }
}

fn slot_of(scene: Scene) -> usize {
    match scene {
        Scene::Portal => 0,
        Scene::Ending => 1,
    }
}

/// The slider's 0 to 100 as a gain: a little steeper than straight, as ears are.
pub fn gain(volume: u8) -> f32 {
    (volume.min(100) as f32 / 100.0).powf(1.5)
}

impl Audio {
    /// Opens the default sound card (None when there is none).
    pub fn new(volume: u8) -> Option<Audio> {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let (ready_tx, ready_rx) = mpsc::sync_channel::<bool>(1);
        let clock: Clock = Arc::new(Mutex::new(None));
        let thread_clock = clock.clone();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || {
                let Ok((_stream, handle)) = OutputStream::try_default() else {
                    let _ = ready_tx.send(false);
                    return;
                };
                let _ = ready_tx.send(true);
                let mut volume = gain(volume);
                let mut track: Option<Sink> = None;
                let mut effects: Vec<Sink> = Vec::new();
                while let Ok(cmd) = rx.recv() {
                    effects.retain(|s| !s.empty());
                    match cmd {
                        Cmd::Track { bytes, lead_in } => {
                            if let Some(old) = track.take() {
                                old.stop();
                            }
                            if let (Ok(sink), Ok(decoder)) = (Sink::try_new(&handle), Decoder::new(Cursor::new(bytes))) {
                                sink.set_volume(volume);
                                sink.append(decoder.delay(Duration::from_secs_f32(lead_in.max(0.0))));
                                track = Some(sink);
                            }
                            // started now, even if there was no sound: the game still has to run
                            if let Ok(mut c) = thread_clock.lock() {
                                *c = Some((Instant::now(), lead_in));
                            }
                        }
                        Cmd::StopTrack => {
                            if let Some(old) = track.take() {
                                old.stop();
                            }
                        }
                        Cmd::Effect(samples) => {
                            if let Ok(sink) = Sink::try_new(&handle) {
                                sink.set_volume(volume);
                                sink.append(SamplesBuffer::new(1, synth::SR, samples));
                                effects.push(sink);
                            }
                        }
                        Cmd::StopEffects => {
                            for s in effects.drain(..) {
                                s.stop();
                            }
                        }
                        Cmd::Volume(v) => {
                            volume = v;
                            if let Some(t) = &track {
                                t.set_volume(v);
                            }
                            for s in &effects {
                                s.set_volume(v);
                            }
                        }
                    }
                }
            })
            .ok()?;
        ready_rx
            .recv()
            .unwrap_or(false)
            .then(|| Audio { tx, clock, slots: Arc::new([Mutex::new(Slot::Empty), Mutex::new(Slot::Empty)]), generation: Arc::new(AtomicU32::new(0)) })
    }

    /// Starts making `scene` in the background, so that `play_scene` has it at once.
    pub fn prepare(&self, scene: Scene) {
        let Ok(mut slot) = self.slots[slot_of(scene)].lock() else { return };
        if matches!(*slot, Slot::Empty) {
            *slot = Slot::Rendering(None);
            drop(slot);
            self.spawn_render(scene);
        }
    }

    /// Plays `scene` now (or as soon as it is made).
    pub fn play_scene(&self, scene: Scene) {
        let Ok(mut slot) = self.slots[slot_of(scene)].lock() else { return };
        let now = self.generation.load(Ordering::SeqCst);
        match std::mem::replace(&mut *slot, Slot::Empty) {
            Slot::Ready(samples) => {
                let _ = self.tx.send(Cmd::Effect(samples));
            }
            Slot::Rendering(_) => *slot = Slot::Rendering(Some(now)),
            Slot::Empty => {
                *slot = Slot::Rendering(Some(now));
                drop(slot);
                self.spawn_render(scene);
            }
        }
    }

    fn spawn_render(&self, scene: Scene) {
        let (slots, tx, generation) = (self.slots.clone(), self.tx.clone(), self.generation.clone());
        let _ = std::thread::Builder::new().name("audio-render".into()).spawn(move || {
            let samples = render(scene);
            let Ok(mut slot) = slots[slot_of(scene)].lock() else { return };
            match *slot {
                Slot::Rendering(Some(g)) if g == generation.load(Ordering::SeqCst) => {
                    let _ = tx.send(Cmd::Effect(samples));
                    *slot = Slot::Empty;
                }
                _ => *slot = Slot::Ready(samples),
            }
        });
    }

    /// Starts the song `bytes` (an mp3); `lead_in` seconds of quiet come before it.
    pub fn play_track(&self, bytes: Arc<[u8]>, lead_in: f32) {
        if let Ok(mut c) = self.clock.lock() {
            *c = None;
        }
        let _ = self.tx.send(Cmd::Track { bytes, lead_in });
    }

    pub fn stop_track(&self) {
        if let Ok(mut c) = self.clock.lock() {
            *c = None;
        }
        let _ = self.tx.send(Cmd::StopTrack);
    }

    /// Where the song is, in seconds (negative during the lead-in). None until the sound card has
    /// been given the song.
    pub fn track_time(&self) -> Option<f32> {
        let c = self.clock.lock().ok()?;
        c.map(|(at, lead)| at.elapsed().as_secs_f32() - lead)
    }

    /// Plays mono samples at `synth::SR` over whatever is playing.
    pub fn play_effect(&self, samples: Vec<f32>) {
        let _ = self.tx.send(Cmd::Effect(samples));
    }

    pub fn stop_effects(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let _ = self.tx.send(Cmd::StopEffects);
    }

    pub fn set_volume(&self, volume: u8) {
        let _ = self.tx.send(Cmd::Volume(gain(volume)));
    }
}

/// An mp3 from the program as mono samples at `synth::SR`, for mixing into an effect.
pub fn decode_mono(bytes: &'static [u8]) -> Option<Vec<f32>> {
    let decoder = Decoder::new(Cursor::new(bytes)).ok()?;
    let (channels, rate) = (decoder.channels() as usize, decoder.sample_rate());
    let samples: Vec<f32> = decoder.map(|s| s as f32 / 32768.0).collect();
    if samples.is_empty() {
        return None;
    }
    Some(synth::to_sr(&synth::to_mono(&samples, channels), rate))
}

/// The mp3 of a song of the library (None if its file cannot be read any more).
pub fn source_bytes(source: &crate::songs::Source) -> Option<Arc<[u8]>> {
    match source {
        crate::songs::Source::Story(i) => SONG_MP3.get(*i).map(|b| Arc::from(*b)),
        crate::songs::Source::File(path) => std::fs::read(path).ok().map(Arc::from),
    }
}

/// An mp3 file's sound as mono samples and its sample rate, for finding the beat of a song that was
/// added. None if it cannot be read or is longer than a song may be.
pub fn decode_file(bytes: Vec<u8>) -> Option<(Vec<f32>, u32)> {
    let decoder = Decoder::new(Cursor::new(bytes)).ok()?;
    let (channels, rate) = (decoder.channels().max(1) as usize, decoder.sample_rate());
    let longest = (crate::chartgen::MAX_SECS as usize + 1) * rate as usize;
    let mut mono: Vec<f32> = Vec::new();
    let (mut sum, mut n) = (0.0f32, 0usize);
    for s in decoder {
        sum += s as f32 / 32768.0;
        n += 1;
        if n == channels {
            mono.push(sum / channels as f32);
            sum = 0.0;
            n = 0;
            if mono.len() > longest {
                return None;
            }
        }
    }
    (!mono.is_empty()).then_some((mono, rate))
}
