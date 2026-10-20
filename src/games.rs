//! The hidden games' difficulty, and when Options starts offering it.
//!
//! There is one game per look of the program: Game 1 (the cat in Father Time's clock) belongs to
//! the ordinary look, Game 2 to the Gold Edition and Game 3 to "The Void". Options has a Games
//! section at the very top, but it is not there until the game of the look you are in has killed you
//! 10 times in all. It shows the death total and offers Hard only; 20 deaths unlock Normal and 35
//! unlock Easy (Game 3's difficulties open sooner: Hard at once, Normal at 1, Easy at 10). The deaths are counted over all time, for each game, and nothing takes them away.
//! Every game also has cheats that Options offers (as switches, off until you turn them on) once that
//! game has killed you enough times: 50, 75 or 100 (see `Cheat`).
//! Nothing here draws anything, so all of it is tested.

/// How hard a game is. Hard is how the games were before there was a choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    Easy,
    Normal,
    #[default]
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    /// How many deaths in a game it takes to unlock this difficulty in Options (all time). Hard is
    /// there as soon as the Games section is.
    pub fn deaths_to_unlock(self) -> u32 {
        match self {
            Difficulty::Hard => 10,
            Difficulty::Normal => 20,
            Difficulty::Easy => 35,
        }
    }

    /// The same for a particular game: Game 3 (the rhythm game) is quicker to open up, as its
    /// difficulties are picked on the screen before each run: Hard is there from the start, Normal
    /// after one death and Easy after ten.
    pub fn unlock_at(self, game: usize) -> u32 {
        match (game, self) {
            (2, Difficulty::Hard) => 0,
            (2, Difficulty::Normal) => 1,
            (2, Difficulty::Easy) => 10,
            _ => self.deaths_to_unlock(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
        }
    }

    /// The difficulty with this key; anything else is Hard.
    pub fn from_key(key: &str) -> Difficulty {
        match key.trim().to_ascii_lowercase().as_str() {
            "easy" => Difficulty::Easy,
            "normal" => Difficulty::Normal,
            _ => Difficulty::Hard,
        }
    }
}

/// How many games there are: one for each look of the program.
pub const GAMES: usize = 3;

/// How many deaths in a game bring up the Games section in Options (the same for every game).
pub const DEATHS_TO_SHOW: u32 = 10;

/// The look of the program, which decides which game's setting Options offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    Standard,
    Gold,
    Void,
}

impl Edition {
    /// Which game belongs to this look (0 is Game 1).
    pub fn game(self) -> usize {
        match self {
            Edition::Standard => 0,
            Edition::Gold => 1,
            Edition::Void => 2,
        }
    }
}

/// The name of a game, for Options.
pub fn game_name(game: usize) -> &'static str {
    match game {
        0 => "Game 1: the cat in Father Time's clock",
        1 => "Game 2: the golden dragon and the angel cats",
        _ => "Game 3: the Void",
    }
}

/// One line on what a difficulty changes in a game.
pub fn describe(game: usize, d: Difficulty) -> String {
    match game {
        0 => {
            let r = crate::game1::Rules::of(d);
            let speed = match d {
                Difficulty::Easy => "much quicker",
                Difficulty::Normal => "quicker",
                Difficulty::Hard => "a little quicker than it used to be",
            };
            format!("Survive {} seconds. The leap comes back {} ({:.1} s).", r.duration as u32, speed, r.leap_cooldown)
        }
        1 => {
            let r = crate::game2::Rules::of(d);
            format!("{} lives. The king comes after {} angels and takes {} hits.", r.lives, r.kills_for_king, r.king_hp)
        }
        _ => {
            let keys: Vec<&str> = crate::game3::lane_keys(d).iter().map(|k| k.label()).collect();
            format!("{} lanes: {}.", keys.len(), keys.join(" "))
        }
    }
}

/// A cheat, earned by dying in its game (all time). Options shows it, as a switch, once it is earned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cheat {
    /// Game 1: the clock's hands run at half speed.
    BorrowedTime,
    /// Game 2: the staff fires much faster.
    RapidFire,
    /// Game 2: nothing costs a life.
    Invincible,
    /// Game 3: pressing a key with no note near it costs nothing.
    NoMashPenalty,
    /// Game 3: every note counts as a perfect hit.
    PerfectScore,
}

impl Cheat {
    pub const ALL: [Cheat; 5] = [Cheat::BorrowedTime, Cheat::RapidFire, Cheat::Invincible, Cheat::NoMashPenalty, Cheat::PerfectScore];

    /// The game it belongs to (0 is Game 1).
    pub fn game(self) -> usize {
        match self {
            Cheat::BorrowedTime => 0,
            Cheat::RapidFire | Cheat::Invincible => 1,
            Cheat::NoMashPenalty | Cheat::PerfectScore => 2,
        }
    }

    /// How many deaths in its game earn it.
    pub fn deaths_to_unlock(self) -> u32 {
        match self {
            Cheat::BorrowedTime | Cheat::RapidFire | Cheat::NoMashPenalty => 50,
            Cheat::Invincible => 75,
            Cheat::PerfectScore => 100,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Cheat::BorrowedTime => "Borrowed Time",
            Cheat::RapidFire => "Rapid fire",
            Cheat::Invincible => "Invincible",
            Cheat::NoMashPenalty => "No button-mashing penalty",
            Cheat::PerfectScore => "Perfect score",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Cheat::BorrowedTime => "The clock's hands run at half speed. The time you must survive is the same.",
            Cheat::RapidFire => "The staff fires more than three times as fast.",
            Cheat::Invincible => "Nothing costs a life. You still flash and pop when you are hit.",
            Cheat::NoMashPenalty => "Pressing a key with no note near it no longer costs patience.",
            Cheat::PerfectScore => "Every note counts as a perfect hit.",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Cheat::BorrowedTime => "borrowed_time",
            Cheat::RapidFire => "rapid_fire",
            Cheat::Invincible => "invincible",
            Cheat::NoMashPenalty => "no_mash_penalty",
            Cheat::PerfectScore => "perfect_score",
        }
    }

    pub fn from_key(key: &str) -> Option<Cheat> {
        let key = key.trim().to_ascii_lowercase();
        Cheat::ALL.into_iter().find(|c| c.key() == key)
    }

    fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

/// Which cheats are switched on (for one game when it is handed to that game).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cheats(u8);

impl Cheats {
    pub const NONE: Cheats = Cheats(0);

    pub fn has(self, c: Cheat) -> bool {
        self.0 & c.bit() != 0
    }

    pub fn with(self, c: Cheat, on: bool) -> Cheats {
        Cheats(if on { self.0 | c.bit() } else { self.0 & !c.bit() })
    }
}

/// The most songs a playlist holds.
pub const MAX_PLAYLIST: usize = 100;

/// Whether `id` can be a song id (letters, digits and dashes, so that it is safe as a file name).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// What is kept between runs: the deaths in each game (over all time), the difficulty chosen and
/// the cheats switched on.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Progress {
    deaths: [u32; GAMES],
    difficulty: [Difficulty; GAMES],
    cheats: Cheats,
    /// Game 3: the song the story is on (0 or 1: the checkpoint), the latency setting in
    /// milliseconds, the volume (None until it is set: 70), and whether the Underworld is open.
    stage3: u8,
    /// Game 1: part one is survived, so the next go starts at part two (the checkpoint).
    checkpoint1: bool,
    /// The cat before the crown: whether you agreed to help him (kept for good), and how many times you said no.
    cat_helped: bool,
    cat_declines: u8,
    /// The Ally of Darkness: the alley quest for the key to the clock tower.
    ally: crate::ally::State,
    latency_ms: i16,
    volume: Option<u8>,
    underworld: bool,
    /// Game 3: the songs of the playlist (ids from songs.rs), in order.
    playlist: Vec<String>,
}

impl Progress {
    /// You died in `game`. Returns the difficulty that unlocked because of it, if one did (Hard at
    /// 10, which also brings up the Games section; Normal at 20; Easy at 35).
    pub fn record_death(&mut self, game: usize) -> Option<Difficulty> {
        let deaths = self.deaths.get_mut(game)?;
        *deaths = deaths.saturating_add(1);
        let now = *deaths;
        Difficulty::ALL.into_iter().find(|d| d.unlock_at(game) == now)
    }

    /// All the deaths in `game`.
    pub fn deaths(&self, game: usize) -> u32 {
        self.deaths.get(game).copied().unwrap_or(0)
    }

    /// Whether Options offers the Games section for `game`.
    pub fn shown(&self, game: usize) -> bool {
        self.deaths(game) >= DEATHS_TO_SHOW
    }

    /// Whether `d` can be chosen in `game`.
    pub fn unlocked(&self, game: usize, d: Difficulty) -> bool {
        self.deaths(game) >= d.unlock_at(game)
    }

    /// The difficulties that can be chosen in `game`, easiest first (none before the section shows).
    pub fn available(&self, game: usize) -> Vec<Difficulty> {
        Difficulty::ALL.into_iter().filter(|d| self.unlocked(game, *d)).collect()
    }

    /// The difficulty `game` is played at: the one chosen, if it is unlocked, otherwise Hard.
    pub fn difficulty(&self, game: usize) -> Difficulty {
        let chosen = self.difficulty.get(game).copied().unwrap_or_default();
        if self.unlocked(game, chosen) { chosen } else { Difficulty::Hard }
    }

    /// Chooses a difficulty; one that is not unlocked yet is ignored.
    pub fn set_difficulty(&mut self, game: usize, d: Difficulty) {
        if self.unlocked(game, d) {
            if let Some(slot) = self.difficulty.get_mut(game) {
                *slot = d;
            }
        }
    }

    /// Whether `c` has been earned.
    pub fn cheat_unlocked(&self, c: Cheat) -> bool {
        self.deaths(c.game()) >= c.deaths_to_unlock()
    }

    /// The cheats `game` has earned, in the order Options lists them.
    pub fn cheats_unlocked(&self, game: usize) -> Vec<Cheat> {
        Cheat::ALL.into_iter().filter(|c| c.game() == game && self.cheat_unlocked(*c)).collect()
    }

    /// Whether `c` is switched on (a cheat that is not earned never is).
    pub fn cheat_on(&self, c: Cheat) -> bool {
        self.cheat_unlocked(c) && self.cheats.has(c)
    }

    /// Switches a cheat on or off; one that is not earned yet is ignored.
    pub fn set_cheat(&mut self, c: Cheat, on: bool) {
        if self.cheat_unlocked(c) {
            self.cheats = self.cheats.with(c, on);
        }
    }

    /// The cheats to play `game` with.
    pub fn active(&self, game: usize) -> Cheats {
        Cheat::ALL.into_iter().filter(|c| c.game() == game && self.cheat_on(*c)).fold(Cheats::NONE, |a, c| a.with(c, true))
    }

    /// The cheat the death just recorded in `game` earned, if one (they come at the exact count).
    pub fn new_cheat(&self, game: usize) -> Option<Cheat> {
        let deaths = self.deaths(game);
        Cheat::ALL.into_iter().find(|c| c.game() == game && c.deaths_to_unlock() == deaths)
    }

    /// Game 1: whether the checkpoint is kept (the next go starts at part two).
    pub fn game1_checkpoint(&self) -> bool {
        self.checkpoint1
    }

    pub fn set_game1_checkpoint(&mut self, on: bool) {
        self.checkpoint1 = on;
    }

    /// Whether you agreed to help the cat (the talk before Game 1).
    pub fn cat_helped(&self) -> bool {
        self.cat_helped
    }

    pub fn set_cat_helped(&mut self, on: bool) {
        self.cat_helped = on;
    }

    /// The alley quest (read it, or change it with `ally_mut`).
    pub fn ally(&self) -> &crate::ally::State {
        &self.ally
    }

    pub fn ally_mut(&mut self) -> &mut crate::ally::State {
        &mut self.ally
    }

    /// Whether the alley can be gone down: the cat has been said yes to (before that it is just a dark alley).
    pub fn ally_open(&self) -> bool {
        self.cat_helped
    }

    /// How many times you have told the cat no.
    pub fn cat_declines(&self) -> u8 {
        self.cat_declines
    }

    /// The cat forgets you: as if you had never spoken (a test button).
    pub fn reset_cat(&mut self) {
        self.cat_helped = false;
        self.cat_declines = 0;
    }

    pub fn add_cat_decline(&mut self) {
        self.cat_declines = self.cat_declines.saturating_add(1).min(99);
    }

    /// Game 3: which song of the story is next (0 is the first; 1 is the checkpoint, kept between runs).
    pub fn game3_stage(&self) -> usize {
        (self.stage3 as usize).min(crate::game3::SONGS - 1)
    }

    pub fn set_game3_stage(&mut self, stage: usize) {
        self.stage3 = stage.min(crate::game3::SONGS - 1) as u8;
    }

    /// Game 3: how many milliseconds to shift the notes against the music, to match the speakers.
    pub fn latency_ms(&self) -> i32 {
        self.latency_ms as i32
    }

    pub fn set_latency_ms(&mut self, ms: i32) {
        self.latency_ms = ms.clamp(-150, 150) as i16;
    }

    /// Game 3's volume, 0 to 100.
    pub fn volume(&self) -> u8 {
        self.volume.unwrap_or(70)
    }

    pub fn set_volume(&mut self, v: u8) {
        self.volume = Some(v.min(100));
    }

    /// Whether the Underworld has been opened (by being killed at the end of the second song).
    pub fn underworld(&self) -> bool {
        self.underworld
    }

    pub fn open_underworld(&mut self) {
        self.underworld = true;
    }

    /// Game 3: the playlist, as song ids in the order they play.
    pub fn playlist(&self) -> &[String] {
        &self.playlist
    }

    pub fn set_playlist(&mut self, ids: Vec<String>) {
        self.playlist = ids.into_iter().filter(|i| valid_id(i)).take(MAX_PLAYLIST).collect();
    }

    /// The config file's lines, `name=a,b,c`.
    pub fn to_text(&self) -> String {
        let join = |v: Vec<String>| v.join(",");
        format!(
            "game_deaths={}\ngame_difficulty={}\ngame_cheats={}\ngame1_checkpoint={}\ncat_helped={}\ncat_declines={}\nally={}\ngame3_stage={}\ngame3_latency={}\ngame3_volume={}\ngame3_underworld={}\ngame3_playlist={}\n",
            join(self.deaths.iter().map(|d| d.to_string()).collect()),
            join(self.difficulty.iter().map(|d| d.key().to_string()).collect()),
            join(Cheat::ALL.into_iter().filter(|c| self.cheats.has(*c)).map(|c| c.key().to_string()).collect()),
            self.checkpoint1 as u8,
            self.cat_helped as u8,
            self.cat_declines,
            self.ally.encode(),
            self.stage3,
            self.latency_ms,
            self.volume.map(|v| v.to_string()).unwrap_or_default(),
            self.underworld as u8,
            self.playlist.join(","),
        )
    }

    /// Reads one config line; false when it is not one of ours. Missing or bad entries are left as
    /// they were. (`game_shown` was in earlier versions: it is read and ignored.)
    pub fn read_line(&mut self, name: &str, value: &str) -> bool {
        let parts = value.split(',').map(str::trim);
        match name {
            "game_deaths" => {
                for (slot, p) in self.deaths.iter_mut().zip(parts) {
                    if let Ok(n) = p.parse() {
                        *slot = n;
                    }
                }
            }
            "game_difficulty" => {
                for (slot, p) in self.difficulty.iter_mut().zip(parts) {
                    *slot = Difficulty::from_key(p);
                }
            }
            "game_cheats" => {
                self.cheats = Cheats::NONE;
                for c in value.split(',').filter_map(Cheat::from_key) {
                    self.cheats = self.cheats.with(c, true);
                }
            }
            "game1_checkpoint" => self.checkpoint1 = value.trim() == "1" || value.trim().eq_ignore_ascii_case("true"),
            "cat_helped" => self.cat_helped = value.trim() == "1" || value.trim().eq_ignore_ascii_case("true"),
            "cat_declines" => self.cat_declines = value.trim().parse::<u8>().unwrap_or(0).min(99),
            "ally" => self.ally = crate::ally::State::decode(value),
            "game3_stage" => self.stage3 = value.trim().parse::<u8>().unwrap_or(0).min(crate::game3::SONGS as u8 - 1),
            "game3_latency" => self.latency_ms = value.trim().parse::<i32>().unwrap_or(0).clamp(-150, 150) as i16,
            "game3_volume" => self.volume = value.trim().parse::<u8>().ok().map(|v| v.min(100)),
            "game3_underworld" => self.underworld = value.trim() == "1" || value.trim().eq_ignore_ascii_case("true"),
            "game3_playlist" => self.set_playlist(value.split(',').map(|p| p.trim().to_string()).collect()),
            "game_shown" => {}
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulties_have_names_keys_and_hard_is_the_default() {
        assert_eq!(Difficulty::default(), Difficulty::Hard);
        for d in Difficulty::ALL {
            assert_eq!(Difficulty::from_key(d.key()), d);
            assert!(!d.label().is_empty());
        }
        assert_eq!(Difficulty::from_key(" Easy \r"), Difficulty::Easy);
        assert_eq!(Difficulty::from_key("nonsense"), Difficulty::Hard);
    }

    #[test]
    fn each_look_has_its_own_game_and_the_same_deaths_unlock_the_same_things() {
        assert_eq!((Edition::Standard.game(), Edition::Gold.game(), Edition::Void.game()), (0, 1, 2));
        assert_eq!(DEATHS_TO_SHOW, 10);
        assert_eq!(
            (Difficulty::Hard.deaths_to_unlock(), Difficulty::Normal.deaths_to_unlock(), Difficulty::Easy.deaths_to_unlock()),
            (10, 20, 35)
        );
        for g in 0..GAMES {
            assert!(!game_name(g).is_empty() && !describe(g, Difficulty::Easy).is_empty());
        }
    }

    /// Dies `n` times in `game`, collecting what unlocked.
    fn die(p: &mut Progress, game: usize, n: u32) -> Vec<Difficulty> {
        (0..n).filter_map(|_| p.record_death(game)).collect()
    }

    #[test]
    fn ten_deaths_show_the_section_with_hard_only_then_normal_at_twenty_and_easy_at_thirty_five() {
        let mut p = Progress::default();
        assert!(!p.shown(0) && p.available(0).is_empty());
        assert_eq!(die(&mut p, 0, 9), vec![]);
        assert!(!p.shown(0));
        assert_eq!(die(&mut p, 0, 1), vec![Difficulty::Hard]);
        assert!(p.shown(0));
        assert_eq!(p.available(0), vec![Difficulty::Hard]);
        assert_eq!(die(&mut p, 0, 9), vec![]);
        assert_eq!(p.available(0), vec![Difficulty::Hard], "19 deaths: still Hard only");
        assert_eq!(die(&mut p, 0, 1), vec![Difficulty::Normal]);
        assert_eq!(p.available(0), vec![Difficulty::Normal, Difficulty::Hard]);
        assert_eq!(die(&mut p, 0, 14), vec![]);
        assert_eq!(p.available(0), vec![Difficulty::Normal, Difficulty::Hard]);
        assert_eq!(die(&mut p, 0, 1), vec![Difficulty::Easy]);
        assert_eq!(p.available(0), Difficulty::ALL.to_vec());
        assert_eq!(die(&mut p, 0, 50), vec![], "each unlocks once");
        assert_eq!(p.deaths(0), 85, "the total keeps counting");
    }

    #[test]
    fn game_three_opens_hard_at_once_normal_at_one_death_and_easy_at_ten() {
        let mut p = Progress::default();
        assert_eq!(p.available(2), vec![Difficulty::Hard]);
        assert_eq!(p.difficulty(2), Difficulty::Hard);
        p.set_difficulty(2, Difficulty::Normal);
        assert_eq!(p.difficulty(2), Difficulty::Hard, "Normal is not open yet");
        assert_eq!(die(&mut p, 2, 1), vec![Difficulty::Normal]);
        assert_eq!(p.available(2), vec![Difficulty::Normal, Difficulty::Hard]);
        p.set_difficulty(2, Difficulty::Normal);
        assert_eq!(p.difficulty(2), Difficulty::Normal);
        assert_eq!(die(&mut p, 2, 8), vec![]);
        assert_eq!(p.available(2), vec![Difficulty::Normal, Difficulty::Hard]);
        assert_eq!(die(&mut p, 2, 1), vec![Difficulty::Easy]);
        assert_eq!(p.available(2), Difficulty::ALL.to_vec());
        // the other games are as they were
        assert!(p.available(0).is_empty());
    }

    #[test]
    fn every_game_has_its_own_total() {
        let mut p = Progress::default();
        die(&mut p, 0, 12);
        assert!(p.shown(0) && !p.shown(1) && !p.shown(2));
        assert_eq!(die(&mut p, 1, 10), vec![Difficulty::Hard]);
        assert!(p.shown(1) && !p.shown(2));
        assert_eq!(p.deaths(1), 10);
        assert_eq!(die(&mut p, 2, 35), vec![Difficulty::Normal, Difficulty::Easy], "Game 3: Hard is there from the start");
        // there is no fourth game
        assert_eq!(p.record_death(3), None);
        assert!(!p.shown(3) && p.available(3).is_empty() && p.deaths(3) == 0);
    }

    #[test]
    fn only_an_unlocked_difficulty_can_be_chosen_or_played() {
        let mut p = Progress::default();
        p.set_difficulty(0, Difficulty::Easy);
        assert_eq!(p.difficulty(0), Difficulty::Hard, "nothing is unlocked: it is Hard");
        die(&mut p, 0, 20);
        p.set_difficulty(0, Difficulty::Easy);
        assert_eq!(p.difficulty(0), Difficulty::Hard, "Easy is not unlocked yet, so the choice is ignored");
        p.set_difficulty(0, Difficulty::Normal);
        assert_eq!(p.difficulty(0), Difficulty::Normal);
        die(&mut p, 0, 15);
        p.set_difficulty(0, Difficulty::Easy);
        assert_eq!(p.difficulty(0), Difficulty::Easy);
        // each game keeps its own
        assert_eq!(p.difficulty(1), Difficulty::Hard);
        p.set_difficulty(9, Difficulty::Easy); // harmless
    }

    #[test]
    fn the_totals_and_choices_survive_the_config_file() {
        let mut p = Progress::default();
        die(&mut p, 0, 40);
        die(&mut p, 1, 22);
        p.set_difficulty(0, Difficulty::Easy);
        p.set_difficulty(1, Difficulty::Normal);
        let mut loaded = Progress::default();
        for line in p.to_text().lines() {
            let (name, value) = line.split_once('=').unwrap();
            assert!(loaded.read_line(name, value));
        }
        assert_eq!(loaded, p);
        assert_eq!((loaded.difficulty(0), loaded.difficulty(1), loaded.difficulty(2)), (Difficulty::Easy, Difficulty::Normal, Difficulty::Hard));
        assert_eq!((loaded.deaths(0), loaded.deaths(1), loaded.deaths(2)), (40, 22, 0));
        // rubbish does no harm, and the old line from the earlier version is ignored
        let mut r = Progress::default();
        assert!(r.read_line("game_deaths", "x,,-3,4"));
        assert!(r.read_line("game_shown", "1,1,1"));
        assert!(!r.read_line("gold_on", "true"));
        assert_eq!((r.deaths(0), r.deaths(1), r.shown(0)), (0, 0, false));
    }

    #[test]
    fn each_game_has_cheats_that_come_at_fifty_seventy_five_and_a_hundred_deaths() {
        assert_eq!(Cheat::ALL.iter().filter(|c| c.game() == 0).count(), 1);
        assert_eq!(Cheat::ALL.iter().filter(|c| c.game() == 1).count(), 2);
        assert_eq!(Cheat::ALL.iter().filter(|c| c.game() == 2).count(), 2);
        let at = |c: Cheat| c.deaths_to_unlock();
        assert_eq!((at(Cheat::BorrowedTime), at(Cheat::RapidFire), at(Cheat::Invincible)), (50, 50, 75));
        assert_eq!((at(Cheat::NoMashPenalty), at(Cheat::PerfectScore)), (50, 100));
        for c in Cheat::ALL {
            assert!(!c.label().is_empty() && !c.blurb().is_empty());
            assert_eq!(Cheat::from_key(c.key()), Some(c));
        }
        assert_eq!(Cheat::from_key("nonsense"), None);
    }

    #[test]
    fn a_cheat_is_offered_at_its_count_in_its_own_game_only() {
        let mut p = Progress::default();
        die(&mut p, 0, 49);
        assert!(p.cheats_unlocked(0).is_empty());
        p.record_death(0);
        assert_eq!(p.new_cheat(0), Some(Cheat::BorrowedTime));
        assert_eq!(p.cheats_unlocked(0), vec![Cheat::BorrowedTime]);
        assert!(p.cheats_unlocked(1).is_empty() && p.cheats_unlocked(2).is_empty(), "the other games have not earned theirs");
        p.record_death(0);
        assert_eq!(p.new_cheat(0), None, "each is announced once");
        // game 2: rapid fire at 50, invincible at 75
        die(&mut p, 1, 50);
        assert_eq!(p.cheats_unlocked(1), vec![Cheat::RapidFire]);
        die(&mut p, 1, 24);
        assert_eq!(p.cheats_unlocked(1), vec![Cheat::RapidFire]);
        p.record_death(1);
        assert_eq!(p.new_cheat(1), Some(Cheat::Invincible));
        assert_eq!(p.cheats_unlocked(1), vec![Cheat::RapidFire, Cheat::Invincible]);
        // game 3: no penalty at 50, perfect score at 100
        die(&mut p, 2, 50);
        assert_eq!(p.cheats_unlocked(2), vec![Cheat::NoMashPenalty]);
        die(&mut p, 2, 49);
        assert_eq!(p.cheats_unlocked(2), vec![Cheat::NoMashPenalty]);
        p.record_death(2);
        assert_eq!(p.cheats_unlocked(2), vec![Cheat::NoMashPenalty, Cheat::PerfectScore]);
    }

    #[test]
    fn a_cheat_is_off_until_switched_on_and_cannot_be_switched_on_before_it_is_earned() {
        let mut p = Progress::default();
        p.set_cheat(Cheat::RapidFire, true);
        die(&mut p, 1, 50);
        assert!(!p.cheat_on(Cheat::RapidFire), "earning it does not turn it on, and the early switch was ignored");
        assert_eq!(p.active(1), Cheats::NONE);
        p.set_cheat(Cheat::RapidFire, true);
        p.set_cheat(Cheat::Invincible, true); // not earned (75)
        assert!(p.cheat_on(Cheat::RapidFire) && !p.cheat_on(Cheat::Invincible));
        assert!(p.active(1).has(Cheat::RapidFire) && !p.active(1).has(Cheat::Invincible));
        assert_eq!(p.active(0), Cheats::NONE, "another game's cheats do not leak across");
        p.set_cheat(Cheat::RapidFire, false);
        assert_eq!(p.active(1), Cheats::NONE);
    }

    #[test]
    fn the_cheats_switched_on_survive_the_config_file() {
        let mut p = Progress::default();
        die(&mut p, 0, 50);
        die(&mut p, 1, 80);
        p.set_cheat(Cheat::BorrowedTime, true);
        p.set_cheat(Cheat::Invincible, true);
        let mut loaded = Progress::default();
        for line in p.to_text().lines() {
            let (name, value) = line.split_once('=').unwrap();
            assert!(loaded.read_line(name, value));
        }
        assert_eq!(loaded, p);
        assert!(loaded.cheat_on(Cheat::BorrowedTime) && loaded.cheat_on(Cheat::Invincible) && !loaded.cheat_on(Cheat::RapidFire));
        let mut r = Progress::default();
        assert!(r.read_line("game_cheats", "rubbish,,borrowed_time"));
        assert_eq!(r.cheats, Cheats::NONE.with(Cheat::BorrowedTime, true), "the unknown names are skipped");
        assert!(!r.cheat_on(Cheat::BorrowedTime), "and without the deaths it is not on");
    }

    #[test]
    fn game_one_keeps_its_checkpoint_between_runs() {
        let mut p = Progress::default();
        assert!(!p.game1_checkpoint());
        p.set_game1_checkpoint(true);
        let mut loaded = Progress::default();
        for line in p.to_text().lines() {
            let (name, value) = line.split_once('=').unwrap();
            assert!(loaded.read_line(name, value), "{name}");
        }
        assert!(loaded.game1_checkpoint());
        assert_eq!(loaded, p);
        p.set_game1_checkpoint(false);
        assert!(!p.game1_checkpoint());
    }

    #[test]
    fn the_alley_quest_is_kept_and_only_opens_once_the_cat_is_helped() {
        let mut p = Progress::default();
        assert!(!p.ally_open());
        p.set_cat_helped(true);
        assert!(p.ally_open());
        p.ally_mut().enter();
        p.ally_mut().light(0);
        p.ally_mut().take_hand(0);
        let mut loaded = Progress::default();
        for line in p.to_text().lines() {
            let (name, value) = line.split_once('=').unwrap();
            assert!(loaded.read_line(name, value), "{name}");
        }
        assert_eq!(loaded, p);
        assert!(loaded.ally().has_hand(0) && !loaded.ally().has_hand(1));
    }

    #[test]
    fn the_cat_remembers_whether_you_helped_and_how_often_you_said_no() {
        let mut p = Progress::default();
        assert!(!p.cat_helped() && p.cat_declines() == 0);
        p.add_cat_decline();
        p.add_cat_decline();
        p.set_cat_helped(true);
        let mut loaded = Progress::default();
        for line in p.to_text().lines() {
            let (name, value) = line.split_once('=').unwrap();
            assert!(loaded.read_line(name, value), "{name}");
        }
        assert!(loaded.cat_helped() && loaded.cat_declines() == 2);
        assert_eq!(loaded, p);
        for _ in 0..200 {
            p.add_cat_decline();
        }
        assert_eq!(p.cat_declines(), 99);
        // the test button makes him forget
        p.reset_cat();
        assert!(!p.cat_helped() && p.cat_declines() == 0);
        assert_eq!(p, Progress { ..Progress::default() });
    }

    #[test]
    fn game_three_keeps_its_checkpoint_latency_volume_and_the_open_underworld() {
        let mut p = Progress::default();
        assert_eq!((p.game3_stage(), p.latency_ms(), p.volume(), p.underworld()), (0, 0, 70, false));
        p.set_game3_stage(1);
        p.set_latency_ms(40);
        p.set_volume(35);
        p.open_underworld();
        let mut loaded = Progress::default();
        for line in p.to_text().lines() {
            let (name, value) = line.split_once('=').unwrap();
            assert!(loaded.read_line(name, value));
        }
        assert_eq!(loaded, p);
        assert_eq!((loaded.game3_stage(), loaded.latency_ms(), loaded.volume(), loaded.underworld()), (1, 40, 35, true));
        // out-of-range values are brought back in, and rubbish is ignored
        p.set_game3_stage(9);
        p.set_latency_ms(9000);
        p.set_volume(250);
        assert_eq!((p.game3_stage(), p.latency_ms(), p.volume()), (1, 150, 100));
        let mut r = Progress::default();
        r.read_line("game3_stage", "x");
        r.read_line("game3_latency", "-999");
        r.read_line("game3_volume", "nope");
        assert_eq!((r.game3_stage(), r.latency_ms(), r.volume()), (0, -150, 70));
    }

    #[test]
    fn the_playlist_is_kept_in_order_and_bad_ids_are_dropped() {
        let mut p = Progress::default();
        p.set_playlist(vec!["story-2".into(), "../evil".into(), "abc123".into(), "".into(), "story-1".into()]);
        assert_eq!(p.playlist(), ["story-2", "abc123", "story-1"]);
        let mut loaded = Progress::default();
        for line in p.to_text().lines() {
            let (k, v) = line.split_once('=').unwrap();
            loaded.read_line(k, v);
        }
        assert_eq!(loaded.playlist(), p.playlist());
        p.set_playlist((0..300).map(|i| format!("id{i}")).collect());
        assert_eq!(p.playlist().len(), MAX_PLAYLIST);
        p.set_playlist(vec![]);
        let mut again = Progress::default();
        again.read_line("game3_playlist", "");
        assert!(again.playlist().is_empty());
    }

    #[test]
    fn game_three_describes_its_lanes() {
        assert_eq!(describe(2, Difficulty::Easy), "4 lanes: D F J K.");
        assert_eq!(describe(2, Difficulty::Normal), "6 lanes: S D F J K L.");
        assert_eq!(describe(2, Difficulty::Hard), "8 lanes: A S D F J K L ;.");
    }
}
