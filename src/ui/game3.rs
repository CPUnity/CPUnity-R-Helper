// Game 3 on screen: the shadow monarch waits behind the lanes while the notes fall, and he comes
// closer as patience runs out. Clearing the second song plays his kill: a heart that beats, a talon
// through it, and the Underworld. The rules are in game3.rs, the sounds in game3_sound.rs, the
// songs and the sound card in audio.rs, and the monarch himself is drawn by ui/monarch.rs.
use super::monarch as monarch_ui;
use super::monarch_art;
use crate::audio::{self, Audio, Scene};
use crate::game3::{self, ending, Game, Judge, LaneKey, Outcome, Phase};
use crate::games::{Cheat, Difficulty};
use crate::songs::{self, Item};
use crate::{game3_sound, monarch};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};

const CYAN: [u8; 3] = [63, 240, 255];
const MAGENTA: [u8; 3] = [255, 63, 200];
const LILAC: [u8; 3] = [196, 168, 255];
const CREAM: [u8; 3] = [255, 241, 184];
const PALE: [u8; 3] = [232, 222, 255];
const DIM: [u8; 3] = [140, 122, 184];
const SALMON: [u8; 3] = [255, 138, 122];

fn rgba(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

fn hex(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

/// The colour of each lane, left to right.
fn lane_colours(lanes: usize) -> Vec<[u8; 3]> {
    let list: &[u32] = match lanes {
        4 => &[0x3ff0ff, 0xc4a8ff, 0xff3fc8, 0xfff1b8],
        5 => &[0x3ff0ff, 0xc4a8ff, 0xfff1b8, 0xff3fc8, 0xff9bf0],
        _ => &[0x2bd1c0, 0x3ff0ff, 0x8fb4ff, 0xc4a8ff, 0xff9bf0, 0xff3fc8, 0xff8a7a, 0xfff1b8],
    };
    list.iter().map(|c| hex(*c)).collect()
}

fn hash01(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    ((x.wrapping_mul(0x297A_2D39) >> 8) & 0xFFFF) as f32 / 65535.0
}

fn key_of(key: egui::Key) -> Option<LaneKey> {
    use egui::Key::*;
    Some(match key {
        A => LaneKey::A,
        S => LaneKey::S,
        D => LaneKey::D,
        F => LaneKey::F,
        J => LaneKey::J,
        K => LaneKey::K,
        L => LaneKey::L,
        Semicolon => LaneKey::Semicolon,
        _ => return None,
    })
}

/// What the player asked for with the keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    None,
    /// Escape: back to the program.
    Quit,
    /// Another go at the same song.
    Retry,
    /// On from the checkpoint to the second song.
    Next,
    /// The ending is over: the Underworld is open.
    Ended,
}

/// What the songs screen asks the program to do (it keeps the files and the settings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Choose an mp3 to add.
    AddSong,
    /// Take a song away (its files too).
    RemoveSong(String),
    /// The playlist is now these song ids, in this order.
    Playlist(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// How it works, and the difficulties.
    Menu,
    /// The songs the player added, and the playlist.
    Songs,
}

/// Songs of the player's own being played: one, or a playlist one after the other.
struct Custom {
    queue: Vec<Item>,
    at: usize,
    playlist: bool,
}

/// One go at a song, from the screen where the difficulty is chosen to the end card.
pub struct Run {
    pub game: Game,
    audio: Option<Audio>,
    screen: Screen,
    /// Which story song the menu is for (the checkpoint), and the cheats, for starting other games.
    story_stage: usize,
    /// Whether the Underworld is open, which is what allows songs of your own.
    underworld: bool,
    library: Vec<Item>,
    playlist: Vec<String>,
    custom: Option<Custom>,
    requests: Vec<Request>,
    /// What the songs screen says it is doing, and the last thing that happened.
    busy: Option<String>,
    note: Option<String>,
    /// The difficulties that are unlocked, and how many deaths the player has in this game (for the
    /// ones that are not).
    available: Vec<Difficulty>,
    deaths: u32,
    /// Set when a different difficulty was picked on the first screen, until `take_chosen` reads it.
    chosen: Option<Difficulty>,
    /// How late the sound reaches the player, in seconds (a setting).
    latency: f32,
    began: bool,
    /// When the ending began (egui time), once the second song is cleared.
    ending: Option<f64>,
    lost_seen: bool,
    cleared_seen: bool,
    cleared_reported: bool,
}

impl Run {
    pub fn new(game: Game, audio: Option<Audio>, latency_ms: i32, available: Vec<Difficulty>, deaths: u32) -> Run {
        Run {
            story_stage: game.stage,
            game,
            audio,
            screen: Screen::Menu,
            underworld: false,
            library: Vec::new(),
            playlist: Vec::new(),
            custom: None,
            requests: Vec::new(),
            busy: None,
            note: None,
            available,
            deaths,
            chosen: None,
            latency: latency_ms as f32 / 1000.0,
            began: false,
            ending: None,
            lost_seen: false,
            cleared_seen: false,
            cleared_reported: false,
        }
    }

    /// Gives the first screen the player's own songs (once the Underworld is open).
    pub fn with_songs(mut self, library: Vec<Item>, playlist: Vec<String>) -> Run {
        self.underworld = true;
        self.library = library;
        self.playlist = playlist;
        self.playlist.retain(|id| self.library.iter().any(|i| &i.id == id));
        self
    }

    /// The library changed (a song was added or taken away).
    pub fn set_library(&mut self, library: Vec<Item>) {
        self.library = library;
        let before = self.playlist.len();
        self.playlist.retain(|id| self.library.iter().any(|i| &i.id == id));
        if self.playlist.len() != before {
            self.requests.push(Request::Playlist(self.playlist.clone()));
        }
    }

    /// Something is being worked on (None when it is done).
    pub fn set_busy(&mut self, busy: Option<String>) {
        self.busy = busy;
    }

    pub fn set_note(&mut self, note: String) {
        self.note = Some(note);
    }

    /// What the songs screen is waiting to have done.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    fn reset_flags(&mut self) {
        self.began = false;
        self.ending = None;
        self.lost_seen = false;
        self.cleared_seen = false;
        self.cleared_reported = false;
    }

    /// Plays `queue` (one song, or a playlist) from the start, at the difficulty chosen.
    fn play_custom(&mut self, queue: Vec<Item>, playlist: bool) {
        let Some(first) = queue.first() else { return };
        self.game = Game::custom(first.song.clone(), self.game.difficulty, self.game.cheats());
        self.custom = Some(Custom { queue, at: 0, playlist });
        self.reset_flags();
        self.begin();
    }

    /// Back to the songs screen from a song of the player's own.
    fn leave_custom(&mut self) {
        if let Some(a) = &self.audio {
            a.stop_track();
            a.stop_effects();
        }
        self.custom = None;
        self.game = Game::new(self.story_stage, self.game.difficulty, self.game.cheats());
        self.reset_flags();
        self.screen = Screen::Songs;
    }

    /// The same song again, or the next one of the playlist.
    fn custom_again(&mut self, next: bool) {
        let Some(c) = &mut self.custom else { return };
        if next {
            c.at += 1;
        }
        let song = c.queue[c.at.min(c.queue.len() - 1)].song.clone();
        self.game = Game::custom(song, self.game.difficulty, self.game.cheats());
        self.reset_flags();
        self.begin();
    }

    /// What clearing the song means, once (None until it is cleared): for saving the checkpoint,
    /// counting the death and opening the Underworld.
    pub fn take_cleared(&mut self) -> Option<Outcome> {
        let outcome = self.game.outcome()?;
        if self.cleared_reported {
            return None;
        }
        self.cleared_reported = true;
        Some(outcome)
    }

    /// Test switch (Options > Take life): straight to the kill at the end of the second song.
    pub fn take_life(&mut self) {
        if let Some(a) = &self.audio {
            a.stop_track();
            a.stop_effects();
            a.prepare(Scene::Ending);
        }
        self.custom = None;
        self.game = Game::new(game3::SONGS - 1, self.game.difficulty, self.game.cheats());
        self.reset_flags();
        self.began = true;
        self.game.finish_now();
    }

    /// The difficulty the player picked on the first screen, once (for saving it).
    pub fn take_chosen(&mut self) -> Option<Difficulty> {
        self.chosen.take()
    }

    /// Picks a difficulty before the song begins (one that is not unlocked is ignored).
    fn pick(&mut self, d: Difficulty) {
        if self.began || d == self.game.difficulty || !self.available.contains(&d) {
            return;
        }
        self.game = Game::new(self.story_stage, d, self.game.cheats());
        self.chosen = Some(d);
    }

    /// The next (or the previous) unlocked difficulty.
    fn pick_next(&mut self, step: i32) {
        let all = Difficulty::ALL;
        let at = all.iter().position(|d| *d == self.game.difficulty).unwrap_or(0) as i32;
        for k in 1..=all.len() as i32 {
            let d = all[(at + step * k).rem_euclid(all.len() as i32) as usize];
            if self.available.contains(&d) {
                self.pick(d);
                return;
            }
        }
    }

    fn begin(&mut self) {
        self.began = true;
        if let Some(a) = &self.audio {
            a.stop_effects();
            let bytes: Option<std::sync::Arc<[u8]>> = match &self.custom {
                Some(c) => audio::source_bytes(&c.queue[c.at.min(c.queue.len() - 1)].source),
                None => Some(std::sync::Arc::from(audio::SONG_MP3[self.game.stage.min(audio::SONG_MP3.len() - 1)])),
            };
            if let Some(bytes) = bytes {
                a.play_track(bytes, game3::LEAD_IN);
            }
            if self.custom.is_none() && self.game.stage + 1 >= game3::SONGS {
                // the ending takes a moment to make; it is ready by the time the song is over
                a.prepare(Scene::Ending);
            }
        }
    }

    /// Takes the game's clock on to now: the song's own clock if there is sound, else the frame time.
    fn advance(&mut self, dt: f32) {
        match &self.audio {
            Some(a) => {
                if let Some(t) = a.track_time() {
                    self.game.advance_to(t - self.latency);
                }
            }
            None => self.game.step(dt),
        }
    }

    /// The sounds that go with the song ending one way or the other.
    fn end_sounds(&mut self, now: f64) {
        if self.game.phase == Phase::Lost && !self.lost_seen {
            self.lost_seen = true;
            if let Some(a) = &self.audio {
                a.stop_track();
                a.play_effect(game3_sound::thud());
            }
        }
        if let (Some(outcome), false) = (self.game.outcome(), self.cleared_seen) {
            self.cleared_seen = true;
            if let Some(a) = &self.audio {
                a.stop_track();
                match outcome {
                    Outcome::Checkpoint | Outcome::Practice => a.play_effect(game3_sound::chime()),
                    Outcome::Death => {
                        a.stop_effects();
                        a.play_scene(Scene::Ending);
                    }
                }
            }
            if outcome == Outcome::Death {
                self.ending = Some(now);
            }
        }
    }

    /// Plays the game a frame and draws it.
    pub fn show(&mut self, ui: &mut egui::Ui) -> Event {
        let ctx = ui.ctx().clone();
        let avail = ui.available_size();
        let (rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
        let now = ctx.input(|i| i.time);
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.1);
        let clock = (now % 3600.0) as f32;
        let (presses, space, esc, nav) = ctx.input(|i| {
            let mut presses = Vec::new();
            let mut space = false;
            let mut esc = false;
            // on the first screen: left / right, or 1 / 2 / 3, to pick the difficulty
            let mut nav: (i32, Option<usize>) = (0, None);
            for e in &i.events {
                if let egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } = e {
                    if modifiers.ctrl || modifiers.alt || modifiers.command {
                        continue;
                    }
                    match *key {
                        egui::Key::Escape => esc = true,
                        egui::Key::Space | egui::Key::Enter => space = true,
                        egui::Key::ArrowLeft => nav.0 -= 1,
                        egui::Key::ArrowRight => nav.0 += 1,
                        egui::Key::Num1 => nav.1 = Some(0),
                        egui::Key::Num2 => nav.1 = Some(1),
                        egui::Key::Num3 => nav.1 = Some(2),
                        other => {
                            if let Some(k) = key_of(other) {
                                presses.push(k);
                            }
                        }
                    }
                }
            }
            (presses, space, esc, nav)
        });
        if esc {
            if self.custom.is_some() {
                self.leave_custom();
            } else if !self.began && self.screen == Screen::Songs {
                self.screen = Screen::Menu;
            } else {
                return Event::Quit;
            }
        }

        let mut event = Event::None;
        let mut start_clicked = false;
        if !self.began && self.screen == Screen::Songs {
            // the songs screen takes its own keys and clicks (see `songs_ui`)
        } else if !self.began {
            let layout = menu_layout(rect);
            if self.underworld && ui.interact(layout.songs, ui.id().with("game3_songs"), egui::Sense::click()).clicked() {
                self.screen = Screen::Songs;
            }
            for (i, card) in layout.cards.iter().enumerate() {
                let resp = ui.interact(*card, ui.id().with(("game3_difficulty", i)), egui::Sense::click());
                if resp.clicked() {
                    self.pick(Difficulty::ALL[i]);
                }
            }
            start_clicked = ui.interact(layout.start, ui.id().with("game3_begin"), egui::Sense::click()).clicked();
            if nav.0 != 0 {
                self.pick_next(nav.0.signum());
            }
            if let Some(i) = nav.1 {
                self.pick(Difficulty::ALL[i]);
            }
            if space || start_clicked {
                self.begin();
            }
        } else if self.game.phase == Phase::Playing {
            self.advance(dt);
            for key in presses {
                if let Some(lane) = game3::lane_of(self.game.difficulty, key) {
                    self.game.press(lane);
                }
            }
        } else {
            self.game.step(dt);
        }
        self.end_sounds(now);

        let (phase, outcome, since_end) = (self.game.phase, self.game.outcome(), self.game.since_end);
        let space = space && self.began;
        let custom = self.custom.as_ref().map(|c| c.playlist && c.at + 1 < c.queue.len());
        match (phase, outcome, self.ending) {
            (Phase::Lost, _, _) if space && since_end > 0.6 && custom.is_some() => self.custom_again(false),
            (_, Some(Outcome::Practice), _) if space && since_end > 0.5 => match custom {
                Some(true) => self.custom_again(true),
                _ => self.leave_custom(),
            },
            (Phase::Lost, _, _) if space && since_end > 0.6 => event = Event::Retry,
            (_, Some(Outcome::Checkpoint), _) if space && since_end > 0.5 => event = Event::Next,
            (_, Some(Outcome::Death), Some(start)) if space && (now - start) as f32 > ending::STRIKE + ending::UNDERWORLD_AT + 1.2 => {
                event = Event::Ended
            }
            _ => {}
        }

        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(5, 2, 11));
        match self.ending {
            Some(start) => paint_ending(&painter, rect, &self.game, (now - start).max(0.0) as f32, clock),
            None => self.paint_song(&painter, rect, clock),
        }
        if !self.began && self.screen == Screen::Songs {
            self.songs_ui(ui, rect);
        }
        ctx.request_repaint();
        event
    }

    /// The songs screen: add an mp3, play one, or build a playlist. (Plain widgets over the dark.)
    fn songs_ui(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let (w, h) = (rect.width(), rect.height());
        let width = (w * 0.86).min(1000.0);
        let area = Rect::from_min_size(pos2(rect.center().x - width / 2.0, rect.top() + h * 0.05), vec2(width, h * 0.9));
        let diff = self.game.difficulty;
        let mut play: Option<(Vec<Item>, bool)> = None;
        let mut back = false;
        let mut playlist = self.playlist.clone();
        let mut remove: Option<String> = None;
        let mut add = false;
        let mut pick: Option<Difficulty> = None;
        let busy = self.busy.clone();
        let note = self.note.clone();
        let faint = Color32::from_rgb(DIM[0], DIM[1], DIM[2]);
        ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
            ui.horizontal(|ui| {
                if ui.button("< Back").clicked() {
                    back = true;
                }
                ui.label(egui::RichText::new("CUSTOM SONGS").size(22.0).color(Color32::from_rgb(PALE[0], PALE[1], PALE[2])));
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Add any mp3 and the program listens to it: it finds the tempo and where the bass, the middle and the hats hit, and builds notes for all three difficulties from that. Play it on its own, or put songs in a playlist that plays one after the other. Nothing here counts as a death or opens anything.",
                )
                .color(faint),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(busy.is_none(), egui::Button::new("Add an mp3...")).clicked() {
                    add = true;
                }
                if let Some(b) = &busy {
                    ui.spinner();
                    ui.label(b.as_str());
                } else if let Some(n) = &note {
                    ui.label(egui::RichText::new(n.as_str()).color(Color32::from_rgb(CREAM[0], CREAM[1], CREAM[2])));
                }
            });
            ui.horizontal(|ui| {
                ui.label("Difficulty:");
                for d in Difficulty::ALL {
                    let open = self.available.contains(&d);
                    let keys = game3::lane_keys(d).iter().map(|k| k.label()).collect::<Vec<_>>().join(" ");
                    let resp = ui.add_enabled(open, egui::SelectableLabel::new(diff == d, format!("{} ({})", d.label(), keys)));
                    if resp.clicked() {
                        pick = Some(d);
                    }
                }
            });
            ui.separator();
            ui.label(egui::RichText::new("Songs").strong());
            egui::ScrollArea::vertical().id_salt("game3_library").max_height(h * 0.32).auto_shrink([false, true]).show(ui, |ui| {
                for item in &self.library {
                    ui.horizontal(|ui| {
                        if ui.button("Play").clicked() {
                            play = Some((vec![item.clone()], false));
                        }
                        let listed = playlist.iter().position(|i| *i == item.id);
                        if ui.button(if listed.is_some() { "In playlist (take out)" } else { "Add to playlist" }).clicked() {
                            match listed {
                                Some(at) => {
                                    playlist.remove(at);
                                }
                                None => playlist.push(item.id.clone()),
                            }
                        }
                        let kind = if item.imported() { "" } else { "  (story)" };
                        ui.label(format!("{}{}  ·  {}  ·  {} notes", item.song.title, kind, songs::clock(item.song.len), item.song.notes(diff).len()));
                        if item.imported() && ui.small_button("Remove").clicked() {
                            remove = Some(item.id.clone());
                        }
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("Playlist ({})", playlist.len())).strong());
                if ui.add_enabled(!playlist.is_empty(), egui::Button::new("Play the playlist")).clicked() {
                    let queue: Vec<Item> = playlist.iter().filter_map(|id| self.library.iter().find(|i| &i.id == id).cloned()).collect();
                    play = Some((queue, true));
                }
                if ui.add_enabled(!playlist.is_empty(), egui::Button::new("Clear")).clicked() {
                    playlist.clear();
                }
            });
            egui::ScrollArea::vertical().id_salt("game3_playlist").max_height(h * 0.2).auto_shrink([false, true]).show(ui, |ui| {
                let mut at = 0;
                while at < playlist.len() {
                    let title = self.library.iter().find(|i| i.id == playlist[at]).map_or("(missing)".to_string(), |i| i.song.title.clone());
                    let (mut up, mut down, mut out) = (false, false, false);
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", at + 1));
                        up = ui.small_button("Up").clicked();
                        down = ui.small_button("Down").clicked();
                        out = ui.small_button("Take out").clicked();
                        ui.label(title);
                    });
                    if up && at > 0 {
                        playlist.swap(at, at - 1);
                    } else if down && at + 1 < playlist.len() {
                        playlist.swap(at, at + 1);
                    } else if out {
                        playlist.remove(at);
                        continue;
                    }
                    at += 1;
                }
                if playlist.is_empty() {
                    ui.label(egui::RichText::new("Empty. Use \"Add to playlist\" on the songs above.").color(faint));
                }
            });
        });
        if playlist != self.playlist {
            self.playlist = playlist;
            self.requests.push(Request::Playlist(self.playlist.clone()));
        }
        if add {
            self.note = None;
            self.requests.push(Request::AddSong);
        }
        if let Some(id) = remove {
            self.requests.push(Request::RemoveSong(id));
        }
        if let Some(d) = pick {
            self.pick(d);
        }
        if let Some((queue, is_playlist)) = play {
            self.play_custom(queue, is_playlist);
        }
        if back {
            self.screen = Screen::Menu;
        }
    }

    /// The first screen: how the game works, and the three difficulties to pick from.
    fn paint_menu(&self, painter: &Painter, rect: Rect, clock: f32) {
        let game = &self.game;
        let (w, h) = (rect.width(), rect.height());
        let k = (h / 640.0).clamp(0.8, 1.4);
        let layout = menu_layout(rect);
        painter.rect_filled(rect, 0.0, rgba([5, 2, 12], 0.82));
        let cx = rect.center().x;
        ghost_text(painter, pos2(cx, rect.top() + h * 0.07), "PATIENCE FOR DEATH", (w * 0.034).clamp(22.0, 40.0), PALE, 1.0);
        let song = game.song();
        painter.text(
            pos2(cx, rect.top() + h * 0.115),
            Align2::CENTER_CENTER,
            format!("song {} of {}  ·  {}  ·  {}", game.stage + 1, game3::SONGS, song.title, song.artist),
            FontId::monospace(12.0 * k),
            rgba(DIM, 1.0),
        );

        // how it works
        let text_font = FontId::proportional(14.0 * k);
        let mut y = rect.top() + h * 0.15;
        let notes = [
            "Notes fall down the lanes. Press a lane's key just as its note reaches the line at the bottom.".to_string(),
            format!(
                "Patience is your life. A perfect hit (within {} ms) or a good one (within {} ms) gives a little back; every missed note costs 8.",
                (game3::PERFECT * 1000.0).round(),
                (game3::GOOD * 1000.0).round()
            ),
            "A key pressed with no note near it costs patience too, and the faster you mash, the more each press costs.".to_string(),
            "If patience runs out he reaches you, and you start the song again.".to_string(),
            "Reach the end of a song with patience left. There are two songs: finishing the first is saved as a checkpoint, so leaving and coming back puts you on the second.".to_string(),
            "Hitting everything early or late? Options > Games has Volume and Audio delay.".to_string(),
        ];
        for line in notes {
            let galley = painter.layout(line, text_font.clone(), rgba(PALE, 0.9), layout.column.width());
            let size = galley.size();
            painter.galley(pos2(layout.column.left(), y), galley, rgba(PALE, 0.9));
            y += size.y + 6.0 * k;
        }

        // the three difficulties
        for (i, d) in Difficulty::ALL.iter().enumerate() {
            let card = layout.cards[i];
            let open = self.available.contains(d);
            let picked = *d == game.difficulty;
            let hovered = open && painter.ctx().input(|inp| inp.pointer.hover_pos()).is_some_and(|p| card.contains(p));
            let dim = if open { 1.0 } else { 0.4 };
            painter.rect_filled(card, 6.0, rgba(if picked { [34, 18, 62] } else { [14, 7, 28] }, 0.92));
            let edge = if picked { CREAM } else if hovered { LILAC } else { [90, 62, 150] };
            painter.rect_stroke(card, 6.0, Stroke::new(if picked { 2.0 } else { 1.0 }, rgba(edge, if open { 1.0 } else { 0.5 })), StrokeKind::Inside);
            let inner = card.shrink(12.0 * k);
            painter.text(pos2(inner.left(), inner.top() + 12.0 * k), Align2::LEFT_CENTER, d.label().to_uppercase(), FontId::proportional(22.0 * k), rgba(if picked { CREAM } else { PALE }, dim));
            painter.text(pos2(inner.right(), inner.top() + 12.0 * k), Align2::RIGHT_CENTER, format!("{}", i + 1), FontId::monospace(12.0 * k), rgba(DIM, dim));
            // the keys, in the colours of their lanes
            let keys = game3::lane_keys(*d);
            let cols = lane_colours(keys.len());
            let gap = 4.0;
            let size = ((inner.width() - gap * (keys.len() as f32 - 1.0)) / keys.len() as f32).min(30.0 * k);
            for (j, key) in keys.iter().enumerate() {
                let r = Rect::from_min_size(pos2(inner.left() + j as f32 * (size + gap), inner.top() + 34.0 * k), vec2(size, size));
                painter.rect_filled(r, 4.0, rgba(cols[j], 0.16 * dim));
                painter.rect_stroke(r, 4.0, Stroke::new(1.5, rgba(cols[j], dim)), StrokeKind::Inside);
                painter.text(r.center(), Align2::CENTER_CENTER, key.label(), FontId::proportional(size * 0.55), rgba(cols[j], dim));
            }
            let galley = painter.layout(game3::blurb(*d).to_string(), FontId::proportional(13.0 * k), rgba(PALE, 0.85 * dim), inner.width());
            painter.galley(pos2(inner.left(), inner.top() + (34.0 + 30.0) * k + 12.0), galley, rgba(PALE, 0.85 * dim));
            let counts = game3::note_counts(*d);
            painter.text(
                pos2(inner.left(), inner.bottom() - 26.0 * k),
                Align2::LEFT_CENTER,
                format!("about {:.1} notes a second", game3::density(*d)),
                FontId::monospace(11.0 * k),
                rgba(DIM, dim),
            );
            painter.text(
                pos2(inner.left(), inner.bottom() - 8.0 * k),
                Align2::LEFT_CENTER,
                format!("{} notes in all", counts.iter().sum::<usize>()),
                FontId::monospace(11.0 * k),
                rgba(DIM, dim),
            );
            if !open {
                painter.rect_filled(card, 6.0, rgba([5, 2, 12], 0.55));
                painter.text(card.center(), Align2::CENTER_CENTER, "LOCKED", FontId::proportional(20.0 * k), rgba(MAGENTA, 0.9));
                painter.text(
                    card.center() + vec2(0.0, 22.0 * k),
                    Align2::CENTER_CENTER,
                    format!("unlocks at {} death{}", d.unlock_at(2), if d.unlock_at(2) == 1 { "" } else { "s" }),
                    FontId::monospace(11.0 * k),
                    rgba(DIM, 1.0),
                );
                painter.text(
                    card.center() + vec2(0.0, 40.0 * k),
                    Align2::CENTER_CENTER,
                    format!("you have {}  ·  {} to go", self.deaths, d.unlock_at(2).saturating_sub(self.deaths)),
                    FontId::monospace(11.0 * k),
                    rgba(PALE, 0.8),
                );
            }
        }

        painter.text(
            pos2(layout.cards[2].right(), layout.cards[0].top() - 10.0 * k),
            Align2::RIGHT_CENTER,
            format!("deaths in this game: {}", self.deaths),
            FontId::monospace(11.0 * k),
            rgba(DIM, 1.0),
        );
        if self.underworld {
            let hovered = painter.ctx().input(|inp| inp.pointer.hover_pos()).is_some_and(|p| layout.songs.contains(p));
            painter.rect_filled(layout.songs, 6.0, rgba([30, 14, 54], 0.95));
            painter.rect_stroke(layout.songs, 6.0, Stroke::new(1.2, rgba(if hovered { CREAM } else { LILAC }, 0.9)), StrokeKind::Inside);
            painter.text(layout.songs.center(), Align2::CENTER_CENTER, "CUSTOM SONGS", FontId::proportional(14.0 * k), rgba(CREAM, 1.0));
        }
        // begin
        let pulse = 0.75 + 0.25 * (clock * 3.0).sin();
        painter.rect_filled(layout.start, 6.0, rgba([40, 20, 70], 0.95));
        painter.rect_stroke(layout.start, 6.0, Stroke::new(1.5, rgba(CYAN, pulse)), StrokeKind::Inside);
        painter.text(layout.start.center(), Align2::CENTER_CENTER, format!("BEGIN ON {}", game.difficulty.label().to_uppercase()), FontId::proportional(16.0 * k), rgba(PALE, 1.0));
        painter.text(
            pos2(cx, rect.bottom() - h * 0.022),
            Align2::CENTER_CENTER,
            "Space or Enter to begin  ·  left / right, 1 2 3 or a click to choose  ·  Esc to leave",
            FontId::monospace(11.0 * k),
            rgba(DIM, 1.0),
        );
    }

    fn paint_song(&self, painter: &Painter, rect: Rect, clock: f32) {
        let game = &self.game;
        let (w, h) = (rect.width(), rect.height());
        let shake = game.shake;
        let rect_s = rect.translate(vec2((hash01((clock * 60.0) as u32, 1) - 0.5) * 14.0 * shake, (hash01((clock * 60.0) as u32, 2) - 0.5) * 10.0 * shake));
        // the shadow monarch behind the lanes: nearer as patience goes, and on you when it is gone
        let near = 1.0 - game.patience / 100.0;
        let base = monarch::PULL_AT + 1.6 + near * 0.9;
        let peak = monarch::LUNGE_AT + monarch::LUNGE_PEAK;
        let scene = base + (peak - base) * game.lunge.powf(1.5);
        monarch_ui::paint_with(painter, rect_s, (w, h), scene, clock, false, true);
        if !self.began {
            if self.screen == Screen::Menu {
                self.paint_menu(painter, rect, clock);
            } else {
                painter.rect_filled(rect, 0.0, rgba([5, 2, 12], 0.88));
            }
            return;
        }

        let n = game.lanes();
        let cols = lane_colours(n);
        let k = (h / 600.0).clamp(0.7, 1.6);
        let total = (w * (0.5 + 0.035 * (n as f32 - 4.0))).min(n as f32 * 92.0 * k);
        let lw = total / n as f32;
        let x0 = rect_s.center().x - total / 2.0;
        let (top, hit, bottom) = (rect_s.top() + h * 0.04, rect_s.top() + h * 0.80, rect_s.top() + h * 0.975);
        let violet = |a: f32| rgba([155, 107, 255], a);

        // the panel and its lane lines
        let panel = Rect::from_min_max(pos2(x0 - 14.0, top), pos2(x0 + total + 14.0, bottom));
        painter.rect_filled(panel, 0.0, rgba([5, 2, 12], 0.70));
        painter.rect_stroke(panel, 0.0, Stroke::new(1.0, violet(0.55)), StrokeKind::Inside);
        for i in 0..=n {
            let x = x0 + i as f32 * lw;
            let a = if i == 0 || i == n { 0.5 } else { 0.16 };
            painter.line_segment([pos2(x, top), pos2(x, bottom)], Stroke::new(1.0, violet(a)));
        }
        // the lines where the notes are to be hit, and the key of each lane
        for i in 0..n {
            let x = x0 + i as f32 * lw;
            let f = game.flash[i];
            let c = cols[i];
            let r = Rect::from_min_max(pos2(x + 2.0, hit - 14.0), pos2(x + lw - 2.0, hit + 14.0));
            painter.rect_filled(r, 0.0, rgba(c, 0.10 + 0.4 * f));
            painter.rect_stroke(r, 0.0, Stroke::new(2.0, rgba(c, 1.0)), StrokeKind::Inside);
            if f > 0.0 {
                // a glow rising from the line
                let mut m = egui::Mesh::default();
                let (a, b) = (pos2(x + 2.0, hit - lw * 2.0), pos2(x + lw - 2.0, hit - lw * 2.0));
                m.colored_vertex(a, rgba(c, 0.0));
                m.colored_vertex(b, rgba(c, 0.0));
                m.colored_vertex(pos2(b.x, hit), rgba(c, 0.32 * f));
                m.colored_vertex(pos2(a.x, hit), rgba(c, 0.32 * f));
                m.add_triangle(0, 1, 2);
                m.add_triangle(0, 2, 3);
                painter.add(Shape::mesh(m));
            }
            let label = game3::lane_keys(game.difficulty)[i].label();
            let size = if label.len() > 2 { (lw * 0.17).max(10.0) } else { (lw * 0.22).clamp(10.0, 22.0) };
            painter.text(pos2(x + lw / 2.0, bottom - 10.0), Align2::CENTER_CENTER, label, FontId::proportional(size), rgba(c, 1.0));
        }
        // the notes
        let span = hit - top;
        for f in game.falling() {
            let y = hit - f.dt / game3::APPROACH * span;
            let x = x0 + f.lane as f32 * lw;
            let c = if f.missed { [106, 90, 143] } else { cols[f.lane] };
            let alpha = if f.missed { 0.35 } else { ((y - top) / 18.0 + 0.2).clamp(0.0, 1.0) };
            let (nh, pad) = ((lw * 0.3).max(12.0), (lw * 0.07).max(3.0));
            let r = Rect::from_min_max(pos2(x + pad, y - nh / 2.0), pos2(x + lw - pad, y + nh / 2.0));
            if !f.missed {
                painter.rect_filled(r.expand(4.0), 3.0, rgba(c, 0.18 * alpha));
            }
            painter.rect_filled(r, 0.0, rgba(c, alpha));
            painter.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), 3.0)), 0.0, rgba([255, 255, 255], 0.7 * alpha));
            painter.rect_filled(r.shrink2(vec2(pad, 6.0f32.min(nh / 3.0))), 0.0, rgba([5, 2, 12], 0.55 * alpha));
        }

        // patience on the left, the score on the right
        let mono = |size: f32| FontId::monospace(size * k);
        let bw = 14.0;
        let bx = (panel.left() - 56.0).max(rect_s.left() + 10.0);
        if panel.left() - rect_s.left() > 48.0 {
            let (by, bh) = (top + 26.0, (bottom - top) - 70.0);
            let bar = Rect::from_min_size(pos2(bx, by), vec2(bw, bh));
            painter.rect_filled(bar, 0.0, rgba([5, 2, 12], 0.8));
            let f = (game.patience / 100.0).clamp(0.0, 1.0);
            let col = if f > 0.5 { CYAN } else if f > 0.25 { LILAC } else { MAGENTA };
            painter.rect_filled(Rect::from_min_max(pos2(bx, by + bh * (1.0 - f)), pos2(bx + bw, by + bh)), 0.0, rgba(col, 1.0));
            painter.rect_stroke(bar, 0.0, Stroke::new(1.0, violet(0.6)), StrokeKind::Inside);
            painter.text(pos2(bx + bw / 2.0, by + bh + 16.0), Align2::CENTER_CENTER, "PATIENCE", mono(10.0), rgba(DIM, 1.0));
        }
        let rx = panel.right() + 30.0;
        if rect_s.right() - rx > 110.0 {
            let label = |y: f32, s: &str| painter.text(pos2(rx, top + y), Align2::LEFT_CENTER, s, mono(11.0), rgba(DIM, 1.0));
            label(34.0, "SCORE");
            painter.text(pos2(rx, top + 64.0), Align2::LEFT_CENTER, format!("{:07}", game.score), mono(26.0), rgba(PALE, 1.0));
            label(104.0, "COMBO");
            let combo_col = if game.combo >= 20 { CREAM } else { PALE };
            painter.text(pos2(rx, top + 144.0), Align2::LEFT_CENTER, game.combo.to_string(), FontId::proportional(40.0 * k), rgba(combo_col, 1.0));
            painter.text(pos2(rx, top + 176.0), Align2::LEFT_CENTER, format!("P {}  G {}", game.counts[0], game.counts[1]), mono(12.0), rgba(DIM, 1.0));
            painter.text(pos2(rx, top + 194.0), Align2::LEFT_CENTER, format!("M {}  X {}", game.counts[2], game.strays), mono(12.0), rgba(DIM, 1.0));
        } else {
            painter.text(
                pos2(rect_s.center().x, top + 18.0),
                Align2::CENTER_CENTER,
                format!("{:07}   x{}", game.score, game.combo),
                mono(15.0),
                rgba(PALE, 1.0),
            );
        }
        // what the last press or note was
        if let Some((kind, _, age)) = game.judged {
            if age < 0.6 {
                let (text, col) = match kind {
                    Judge::Perfect => ("PERFECT", CYAN),
                    Judge::Good => ("GOOD", LILAC),
                    Judge::Miss => ("MISS", MAGENTA),
                    Judge::Stray => (if game.mash >= 3.0 { "STOP MASHING" } else { "NO NOTE" }, SALMON),
                };
                let size = (lw * 0.36).clamp(18.0, 34.0);
                ghost_text(painter, pos2(rect_s.center().x, hit - lw.min(60.0) * 1.6 - age * 18.0), text, size, col, 1.0 - age / 0.6);
            }
        }
        // the cheats that are on
        let mut cheats = Vec::new();
        if game.cheats().has(Cheat::NoMashPenalty) {
            cheats.push("NO MASH PENALTY");
        }
        if game.cheats().has(Cheat::PerfectScore) {
            cheats.push("PERFECT SCORE");
        }
        if self.began && !cheats.is_empty() {
            painter.text(pos2(rect_s.center().x, rect_s.bottom() - 8.0), Align2::CENTER_CENTER, format!("CHEATS: {}", cheats.join(" · ")), mono(11.0), rgba(SALMON, 1.0));
        }

        // before the music
        let big = (lw * 0.5).clamp(20.0, 40.0);
        let mid = rect_s.center();
        if self.began && game.phase == Phase::Playing && game.t < 0.0 {
            ghost_text(painter, pos2(mid.x, rect_s.top() + h * 0.45), &format!("{}", (-game.t).ceil() as i32), (lw * 1.2).clamp(40.0, 90.0), PALE, 0.9);
        }
        // patience gone: he has you
        if game.phase == Phase::Lost {
            let fail = (game.since_end / 1.0).clamp(0.0, 1.0);
            painter.rect_filled(rect_s, 0.0, rgba([5, 2, 12], (fail * 0.9).min(0.8)));
            if game.since_end > 0.6 {
                ghost_text(painter, pos2(mid.x, rect_s.top() + h * 0.46), "HE HAS RUN OUT OF PATIENCE", big * 0.8, MAGENTA, 1.0);
                let note = if game.stage > 0 { "checkpoint kept  ·  " } else { "" };
                painter.text(
                    pos2(mid.x, rect_s.top() + h * 0.46 + big),
                    Align2::CENTER_CENTER,
                    format!("{}you have to reach the end of the song first  ·  score {}  ·  Space to try again  ·  Esc to {}", note, game.score, if self.custom.is_some() { "go back" } else { "leave" }),
                    mono(12.0),
                    rgba(DIM, 1.0),
                );
            }
        }
        // a song cleared: the checkpoint, or the card of a song of your own
        if matches!(game.outcome(), Some(Outcome::Checkpoint) | Some(Outcome::Practice)) {
            let checkpoint = game.outcome() == Some(Outcome::Checkpoint);
            let a = (game.since_end / 0.5).clamp(0.0, 1.0);
            painter.rect_filled(rect_s, 0.0, rgba([5, 2, 12], 0.74 * a));
            ghost_text(painter, pos2(mid.x, rect_s.top() + h * 0.30), if checkpoint { "CHECKPOINT REACHED" } else { "SONG CLEARED" }, big * 0.9, CREAM, a);
            let (head, foot) = match (&self.custom, checkpoint) {
                (_, true) => ("song 1 of 2  ·  he is still waiting  ·  song 2 is next, and it is longer".to_string(), "leave and come back: you will resume on song 2  ·  Space to continue".to_string()),
                (Some(c), false) if c.playlist && c.at + 1 < c.queue.len() => (
                    format!("{}  ·  {} of {} in the playlist", game.song().title, c.at + 1, c.queue.len()),
                    format!("next: {}  ·  Space to go on  ·  Esc to stop", c.queue[c.at + 1].song.title),
                ),
                (Some(c), false) if c.playlist => (format!("{}  ·  the playlist is done", game.song().title), "Space or Esc to go back to custom songs".to_string()),
                _ => (format!("{}  ·  custom song, nothing saved", game.song().title), "Space or Esc to go back to custom songs".to_string()),
            };
            let lines = [
                (head, DIM, 0.37),
                (format!("rank {} (for show only)  ·  score {}  ·  best combo {}", game.rank(), game.score, game.best), PALE, 0.46),
                (
                    format!(
                        "{} perfect  ·  {} good  ·  {} missed  ·  {} stray presses  ·  {:.1}% accuracy",
                        game.counts[0],
                        game.counts[1],
                        game.counts[2],
                        game.strays,
                        game.accuracy() * 100.0
                    ),
                    PALE,
                    0.52,
                ),
                (foot, DIM, 0.60),
            ];
            for (text, col, at) in lines {
                painter.text(pos2(mid.x, rect_s.top() + h * at), Align2::CENTER_CENTER, text, mono(13.0), rgba(col, a));
            }
        }
        // misses tear the picture
        if game.glitch > 0.0 {
            let bars = (2.0 + game.glitch * 5.0).round() as u32;
            let frame = (clock * 24.0) as u32;
            for i in 0..bars {
                let y = rect.top() + hash01(frame, i * 3 + 1) * h * 0.9;
                let bh = 6.0 + hash01(frame, i * 3 + 2) * 34.0 * game.glitch;
                let dx = (hash01(frame, i * 3 + 3) - 0.5) * 90.0 * game.glitch;
                let line = |yy: f32, c: [u8; 3]| painter.rect_filled(Rect::from_min_size(pos2(rect.left() + dx, yy), vec2(w, 1.5)), 0.0, rgba(c, 0.5));
                line(y, CYAN);
                line(y + bh - 1.5, MAGENTA);
            }
        }
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        if let Some(a) = &self.audio {
            a.stop_track();
            a.stop_effects();
        }
    }
}

/// Where things are on the first screen.
struct MenuLayout {
    /// The column the writing sits in.
    column: Rect,
    /// Easy, Normal and Hard.
    cards: [Rect; 3],
    /// The begin button.
    start: Rect,
    /// The button for the songs of your own.
    songs: Rect,
}

fn menu_layout(rect: Rect) -> MenuLayout {
    let (w, h) = (rect.width(), rect.height());
    let width = (w * 0.86).min(1000.0);
    let left = rect.center().x - width / 2.0;
    let column = Rect::from_min_size(pos2(left, rect.top() + h * 0.15), vec2(width, h * 0.30));
    let gap = 16.0;
    let cw = (width - 2.0 * gap) / 3.0;
    let (top, bottom) = (rect.top() + h * 0.47, rect.top() + h * 0.88);
    let cards = [0, 1, 2].map(|i| Rect::from_min_max(pos2(left + i as f32 * (cw + gap), top), pos2(left + i as f32 * (cw + gap) + cw, bottom)));
    let start = Rect::from_center_size(pos2(rect.center().x, rect.top() + h * 0.935), vec2(240.0, 34.0));
    let songs = Rect::from_min_size(pos2(left + width - 170.0, rect.top() + h * 0.03), vec2(170.0, 30.0));
    MenuLayout { column, cards, start, songs }
}

/// Text with a cyan and a magenta ghost either side, centred on `at`.
fn ghost_text(painter: &Painter, at: Pos2, text: &str, size: f32, col: [u8; 3], alpha: f32) {
    let font = FontId::proportional(size);
    painter.text(at + vec2(-2.0, 0.0), Align2::CENTER_CENTER, text, font.clone(), rgba(CYAN, 0.6 * alpha));
    painter.text(at + vec2(2.0, 0.0), Align2::CENTER_CENTER, text, font.clone(), rgba(MAGENTA, 0.6 * alpha));
    painter.text(at, Align2::CENTER_CENTER, text, font, rgba(col, alpha));
}

/// Types `text` (`count` letters of it) centred on `at` without sliding as it grows.
fn typed(painter: &Painter, at: Pos2, text: &str, count: usize, size: f32, alpha: f32) {
    if count == 0 || alpha <= 0.0 {
        return;
    }
    let font = FontId::proportional(size);
    let whole = painter.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE).size().x;
    let shown: String = text.chars().take(count).collect();
    let left = pos2(at.x - whole / 2.0, at.y);
    for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
        painter.text(left + vec2(dx, dy), Align2::LEFT_CENTER, &shown, font.clone(), rgba([0, 0, 0], 0.85 * alpha));
    }
    painter.text(left, Align2::LEFT_CENTER, &shown, font, rgba(PALE, alpha));
}

fn cubic(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            pos2(w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x, w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y)
        })
        .collect()
}

/// A heart of size `sz` with its point at the bottom, centred on `c`.
fn heart_outline(c: Pos2, sz: f32) -> Vec<Pos2> {
    let p = |x: f32, y: f32| pos2(c.x + x * sz, c.y + y * sz);
    let mut pts = cubic(p(0.0, 0.95), p(-1.55, -0.1), p(-0.85, -1.1), p(0.0, -0.4), 24);
    pts.extend(cubic(p(0.0, -0.4), p(0.85, -1.1), p(1.55, -0.1), p(0.0, 0.95), 24));
    pts
}

/// The monarch's talon `sprite` drawn at `at`, `s` screen points to each unit, turned by `rot`.
fn hand_at(painter: &Painter, sprite: &monarch_art::Sprite, at: Pos2, s: f32, rot: f32, alpha: f32) {
    let (x0, y0, x1, y1) = sprite.units();
    let (sin, cos) = rot.sin_cos();
    let corner = |x: f32, y: f32| pos2(at.x + (x * cos - y * sin) * s, at.y + (x * sin + y * cos) * s);
    sprite.draw_quad(painter, [corner(x0, y0), corner(x1, y0), corner(x1, y1), corner(x0, y1)], monarch_art::fade(alpha));
}

/// The kill: he creeps out of the portal while the heart beats, lunges, and the talon goes through
/// the heart; then his two lines, and the Underworld. `r` is seconds since it began.
fn paint_ending(painter: &Painter, rect: Rect, game: &Game, r: f32, clock: f32) {
    let (w, h) = (rect.width(), rect.height());
    let since = r - ending::STRIKE;
    // the hit shakes everything for a moment
    let shake = if (0.0..0.45).contains(&since) { 1.0 - since / 0.45 } else { 0.0 };
    let rect_s = rect.translate(vec2((hash01((clock * 60.0) as u32, 5) - 0.5) * 16.0 * shake, (hash01((clock * 60.0) as u32, 6) - 0.5) * 12.0 * shake));
    // without the arms and the fire that reach out of the portal: the only arm in the kill is the one
    // that comes down for the heart
    monarch_ui::paint_with(painter, rect_s, (w, h), ending::scene_time(r), clock, false, false);

    let (hx, hy) = (rect_s.center().x, rect_s.top() + h * 0.78);
    // the user's heart, faint, beating faster as he comes
    if r < ending::STRIKE {
        let (a, p) = (ending::heart_alpha(r), ending::heart_pulse(r));
        let sz = w.min(h) * 0.075 * (1.0 + 0.14 * p);
        let c = pos2(hx, hy);
        for (grow, glow) in [(2.1, 0.05), (1.7, 0.08), (1.35, 0.12)] {
            painter.circle_filled(c, sz * grow * (1.0 + 0.3 * p), rgba([255, 63, 108], glow * a * (0.6 + 0.8 * p)));
        }
        let outline = heart_outline(c, sz);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(pos2(hx, hy - sz * 0.1), rgba([255, 70, 110], a * 0.95));
        for p in &outline {
            mesh.colored_vertex(*p, rgba([90, 8, 40], a));
        }
        let n = outline.len();
        for i in 0..n {
            mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
        }
        painter.add(Shape::mesh(mesh));
        painter.add(Shape::closed_line(outline, Stroke::new(2.0, rgba([255, 150, 175], a * 0.8))));
    }
    // the talon: it starts to come down just before the lunge peaks, and closes on the heart as it goes through
    if let Some((e, closed)) = ending::hand(r) {
        let hs = h / 400.0;
        let tip = 165.0 * hs;
        let rot = std::f32::consts::FRAC_PI_2;
        // the one arm of the kill: from above the screen down to the wrist of the talon
        let arm_to = |palm: Pos2, scale: f32, alpha: f32| {
            let wrist = pos2(palm.x, palm.y - 52.0 * scale);
            let start = pos2(palm.x, rect_s.top() - h * 0.12);
            if wrist.y > start.y + 4.0 {
                let mid = pos2(palm.x + 14.0 * hs, (start.y + wrist.y) / 2.0);
                monarch_ui::paint_arm(painter, &[start, mid, wrist], 30.0 * hs, 17.0 * scale, alpha);
            }
        };
        match monarch_art::art(painter.ctx()) {
            Some(art) => {
                if !closed {
                    let p = ((r - ending::HAND_FROM) / (ending::STRIKE - ending::HAND_FROM)).clamp(0.0, 1.0);
                    let palm_end = hy - tip * 0.8;
                    let palm_y = rect_s.top() - h * 0.25 + (palm_end - (rect_s.top() - h * 0.25)) * e;
                    let sway = (r * 9.0).sin() * w * 0.004 * (1.0 - e);
                    let (palm, scale, alpha) = (pos2(hx + sway, palm_y), hs * (0.7 + 0.4 * e), (p * 3.0).min(1.0));
                    arm_to(palm, scale, alpha);
                    hand_at(painter, &art.hand_open, palm, scale, rot, alpha);
                } else {
                    let sink = h * 0.015 * (since * 3.0).clamp(0.0, 1.0);
                    let palm = pos2(hx, hy - tip * 0.42 + sink);
                    arm_to(palm, hs * 1.1, 1.0);
                    hand_at(painter, &art.hand_curled, palm, hs * 1.1, rot, 1.0);
                }
            }
            None => {
                // without the painted talon: a dark claw coming down
                let y = rect_s.top() - h * 0.25 + (hy - tip * 0.8 - (rect_s.top() - h * 0.25)) * e;
                let claw = vec![pos2(hx - 26.0 * hs, y), pos2(hx + 26.0 * hs, y), pos2(hx, y + tip * 0.9)];
                painter.add(Shape::convex_polygon(claw, rgba([10, 4, 22], 1.0), Stroke::new(1.5, rgba([138, 92, 245], 0.8))));
            }
        }
    }
    // the heart bursts
    if let Some(d) = ending::burst(r) {
        for i in 0..40u32 {
            let ang = hash01(i, 11) * std::f32::consts::TAU;
            let sp = 0.25 + hash01(i, 12) * 0.75;
            let sz = 2.0 + hash01(i, 13) * 3.0;
            let dist = sp * d * h * 0.55;
            let al = (1.0 - d / 1.1).max(0.0);
            let p = pos2(hx + ang.cos() * dist, hy + ang.sin() * dist * 0.7 + d * d * h * 0.08);
            painter.circle_filled(p, sz * (1.0 + 0.5 * (1.0 - al)), Color32::from_rgba_unmultiplied(255, (70.0 + 80.0 * sp) as u8, 110, (al * 255.0) as u8));
        }
    }
    // the flash, then the dark
    let flash = ending::flash(r);
    if flash > 0.0 {
        painter.rect_filled(rect, 0.0, rgba([232, 214, 255], flash));
    }
    let black = ending::black(r);
    if black > 0.0 {
        painter.rect_filled(rect, 0.0, rgba([5, 2, 12], black));
    }

    // his two lines, written slowly and faded together; then the name of what he opened
    let fs = (w * 0.034).clamp(14.0, 26.0);
    let lines_a = ending::lines_alpha(r);
    if lines_a > 0.0 {
        let c = rect.center();
        let y = rect.top() + h * 0.42;
        typed(painter, pos2(c.x, y), ending::LINE1, ending::written(ending::LINE1, since, ending::LINE1_AT), fs, lines_a);
        typed(painter, pos2(c.x, y + fs * 1.7), ending::LINE2, ending::written(ending::LINE2, since, ending::LINE2_AT), fs, lines_a);
    }
    let ua = ending::underworld_alpha(r);
    if ua > 0.0 {
        let c = rect.center();
        let big = (w * 0.07).clamp(26.0, 64.0);
        let jitter = if hash01((clock * 30.0) as u32, 9) < 0.04 { (hash01((clock * 30.0) as u32, 10) - 0.5) * 14.0 } else { 0.0 };
        ghost_text(painter, pos2(c.x + jitter, rect.top() + h * 0.40), "THE UNDERWORLD", big, CREAM, ua);
        let mono = FontId::monospace(12.0);
        painter.text(pos2(c.x, rect.top() + h * 0.40 + big * 0.8), Align2::CENTER_CENTER, "A NEW WAY OF LOOKING AT THINGS IS OPEN", mono.clone(), rgba(DIM, ua));
        if game.counts.iter().sum::<u32>() > 0 {
            painter.text(
                pos2(c.x, rect.top() + h * 0.56),
                Align2::CENTER_CENTER,
                format!("rank {} (for show only)  ·  score {}  ·  best combo {}  ·  {} stray presses", game.rank(), game.score, game.best, game.strays),
                mono.clone(),
                rgba(PALE, ua),
            );
        }
        painter.text(pos2(c.x, rect.top() + h * 0.62), Align2::CENTER_CENTER, "Space to go on", mono, rgba(DIM, ua));
    }
}
