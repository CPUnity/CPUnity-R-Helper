//! Trophies: things to be proud of, each with a description, some of them hidden until they are
//! earned. Earning every other one earns the Platinum trophy, and with it the Platinum skin.
//!
//! A trophy is worked out from what the program already knows (`Facts`: wins, deaths, worlds, the
//! shop), so there is nothing to call from the games: the program asks `Book::sync` now and then and
//! it says what is new. Once earned a trophy stays earned, whatever happens to the facts later (a
//! New Game+ will take the worlds away again, the trophies it earned stay).
//! Nothing here draws anything, so all of it is tested.

/// How much a trophy is worth, for the colour of its cup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

impl Tier {
    pub fn label(self) -> &'static str {
        match self {
            Tier::Bronze => "Bronze",
            Tier::Silver => "Silver",
            Tier::Gold => "Gold",
            Tier::Platinum => "Platinum",
        }
    }

    /// The colour of its cup (r, g, b).
    pub fn colour(self) -> [u8; 3] {
        match self {
            Tier::Bronze => [205, 127, 50],
            Tier::Silver => [190, 196, 206],
            Tier::Gold => [240, 194, 79],
            Tier::Platinum => [214, 232, 244],
        }
    }
}

/// What the program knows that trophies are worked out from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Facts {
    /// Whether each game has been won (without a cheat).
    pub won: [bool; 3],
    /// Deaths in each game, all time.
    pub deaths: [u32; 3],
    pub cheats_earned: usize,
    pub cheats_total: usize,
    /// You said yes to the cat: Act I, the story before the Gold Edition, has begun.
    pub act1: bool,
    pub gold: bool,
    pub void: bool,
    pub underworld: bool,
    /// What has been bought, and what the shop sells in all.
    pub bought: usize,
    pub for_sale: usize,
    pub skins_bought: usize,
    pub skins_for_sale: usize,
    /// Tokens earned over all time.
    pub tokens: u64,
    /// How many of the Underworld's whispers have been heard, and how many there are.
    pub whispers: u32,
    pub whispers_total: u32,
    /// Games played at the Devil's Table, won there, and lost to the Shadow Monarch.
    pub poker_played: u32,
    pub poker_won: u32,
    pub poker_takes: u32,
    /// The King of Hearts and the Queen of Diamonds were dealt to you together.
    pub poker_together: bool,
    /// The Underworld is held back in this build: its trophies are not listed, earned or needed for Platinum.
    pub underworld_held: bool,
}

impl Facts {
    pub fn total_deaths(&self) -> u32 {
        self.deaths.iter().sum()
    }
}

/// One trophy.
pub struct Trophy {
    pub id: &'static str,
    pub name: &'static str,
    /// What it takes (shown once it is earned, and for the ones that are not hidden).
    pub description: &'static str,
    /// A hidden trophy is not described, or even named, until it is earned.
    pub hidden: bool,
    pub tier: Tier,
    /// Whether the facts earn it. (Platinum is earned by the other trophies, not by this.)
    pub earned: fn(&Facts) -> bool,
}

pub const PLATINUM: &str = "platinum";

/// What a hidden trophy says about itself until it is earned.
pub const HIDDEN_NAME: &str = "Hidden trophy";
pub const HIDDEN_HINT: &str = "Keep playing to find out.";

fn never(_: &Facts) -> bool {
    false
}

/// Every trophy: the Platinum one first, then the rest in the order they are listed.
pub const TROPHIES: &[Trophy] = &[
    Trophy {
        id: PLATINUM,
        name: "Platinum",
        description: "Earn every other trophy. It comes with the Platinum skin.",
        hidden: false,
        tier: Tier::Platinum,
        earned: never,
    },
    Trophy {
        id: "ten_deaths",
        name: "Practice Run",
        description: "Lose 10 times in the hidden games.",
        hidden: false,
        tier: Tier::Bronze,
        earned: |f| f.total_deaths() >= 10,
    },
    Trophy {
        id: "hundred_deaths",
        name: "Hard Lessons",
        description: "Lose 100 times in the hidden games.",
        hidden: false,
        tier: Tier::Silver,
        earned: |f| f.total_deaths() >= 100,
    },
    Trophy {
        id: "act1",
        name: "Act I",
        description: "Agree to help the cat. The story begins.",
        hidden: true,
        tier: Tier::Bronze,
        earned: |f| f.act1,
    },
    Trophy {
        id: "game1_win",
        name: "Time Waits for No One",
        description: "Win Game 1, the cat in Father Time's clock, without a cheat.",
        hidden: false,
        tier: Tier::Bronze,
        earned: |f| f.won[0],
    },
    Trophy {
        id: "game2_win",
        name: "Staff of Ages",
        description: "Win Game 2, the golden dragon and the angel cats, without a cheat.",
        hidden: false,
        tier: Tier::Silver,
        earned: |f| f.won[1],
    },
    Trophy {
        id: "game3_win",
        name: "Last Note",
        description: "Clear the first song of Game 3 without a cheat.",
        hidden: false,
        tier: Tier::Silver,
        earned: |f| f.won[2],
    },
    Trophy {
        id: "cheat_first",
        name: "Cheat Code",
        description: "Earn a cheat by losing enough times in its game.",
        hidden: false,
        tier: Tier::Bronze,
        earned: |f| f.cheats_earned >= 1,
    },
    Trophy {
        id: "cheat_all",
        name: "Cheat Sheet",
        description: "Earn every cheat.",
        hidden: false,
        tier: Tier::Silver,
        earned: |f| f.cheats_total > 0 && f.cheats_earned >= f.cheats_total,
    },
    Trophy {
        id: "first_buy",
        name: "First Purchase",
        description: "Buy something in the shop.",
        hidden: false,
        tier: Tier::Bronze,
        earned: |f| f.bought >= 1,
    },
    Trophy {
        id: "tokens_100",
        name: "Pocket Change",
        description: "Earn 100 CR-Tokens in all.",
        hidden: false,
        tier: Tier::Bronze,
        earned: |f| f.tokens >= 100,
    },
    Trophy {
        id: "tokens_500",
        name: "Token Hoard",
        description: "Earn 500 CR-Tokens in all.",
        hidden: false,
        tier: Tier::Silver,
        earned: |f| f.tokens >= 500,
    },
    Trophy {
        id: "wardrobe",
        name: "Wardrobe",
        description: "Own every skin the shop sells.",
        hidden: false,
        tier: Tier::Gold,
        earned: |f| f.skins_for_sale > 0 && f.skins_bought >= f.skins_for_sale,
    },
    Trophy {
        id: "sold_out",
        name: "Sold Out",
        description: "Own everything the shop sells.",
        hidden: false,
        tier: Tier::Gold,
        earned: |f| f.for_sale > 0 && f.bought >= f.for_sale,
    },
    Trophy {
        id: "gold",
        name: "Golden Hour",
        description: "Unlock the Gold Edition.",
        hidden: true,
        tier: Tier::Silver,
        earned: |f| f.gold,
    },
    Trophy {
        id: "void",
        name: "Into the Void",
        description: "Unlock The Void.",
        hidden: true,
        tier: Tier::Silver,
        earned: |f| f.void,
    },
    Trophy {
        id: "underworld",
        name: "Down We Go",
        description: "Open The Underworld.",
        hidden: true,
        tier: Tier::Gold,
        earned: |f| f.underworld,
    },
    Trophy {
        id: "listener",
        name: "Hearing Things",
        description: "Hear every whisper in the Underworld.",
        hidden: true,
        tier: Tier::Silver,
        earned: |f| f.whispers_total > 0 && f.whispers >= f.whispers_total,
    },
    Trophy {
        id: "poker_played",
        name: "Dealt In",
        description: "Play a game at the Devil's Table.",
        hidden: false,
        tier: Tier::Bronze,
        earned: |f| f.poker_played >= 1,
    },
    Trophy {
        id: "poker_won",
        name: "Devil's Luck",
        description: "Beat everyone at the Devil's Table.",
        hidden: false,
        tier: Tier::Gold,
        earned: |f| f.poker_won >= 1,
    },
    Trophy {
        id: "poker_soul",
        name: "Paid in Full",
        description: "Lose a game at the Devil's Table to the Shadow Monarch.",
        hidden: true,
        tier: Tier::Silver,
        earned: |f| f.poker_takes >= 1,
    },
    Trophy {
        id: "poker_together",
        name: "Playing God",
        description: "Be dealt the King of Hearts and the Queen of Diamonds together, and bring the lovers together.",
        hidden: true,
        tier: Tier::Gold,
        earned: |f| f.poker_together,
    },
];

/// The trophy with this id.
pub fn trophy(id: &str) -> Option<&'static Trophy> {
    TROPHIES.iter().find(|t| t.id == id)
}

/// How a trophy is shown in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub id: &'static str,
    pub name: String,
    pub description: String,
    pub earned: bool,
    /// A hidden trophy that has not been earned: nothing is told about it.
    pub secret: bool,
    pub tier: Tier,
    /// When it was earned (seconds since 1970), if it was and the time is known.
    pub when: Option<u64>,
}

/// The trophies that were earned, in the order they were earned (with when).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Book {
    earned: Vec<(String, u64)>,
    /// Testing: trophies are not earned by playing, only given and taken by hand.
    manual: bool,
    /// The Underworld is held back (see `release.rs`): its trophies do not count. Not saved; set by `sync`.
    held: bool,
}

impl Book {
    /// Whether the Underworld's trophies are held back.
    pub fn holds_underworld(&self) -> bool {
        self.held
    }

    fn live(&self, id: &str) -> bool {
        !crate::release::trophy_held(id, self.held)
    }

    pub fn has(&self, id: &str) -> bool {
        self.earned.iter().any(|(e, _)| e == id)
    }

    /// When `id` was earned, if it was and the time was written down.
    pub fn when(&self, id: &str) -> Option<u64> {
        self.earned.iter().find(|(e, _)| e == id).map(|(_, t)| *t).filter(|t| *t > 0)
    }

    /// How many have been earned.
    pub fn count(&self) -> usize {
        self.earned.len()
    }

    /// (earned, all of them).
    pub fn progress(&self) -> (usize, usize) {
        (self.earned.iter().filter(|(e, _)| self.live(e)).count(), TROPHIES.iter().filter(|t| self.live(t.id)).count())
    }

    /// Whether every trophy but Platinum has been earned.
    pub fn all_but_platinum(&self) -> bool {
        TROPHIES.iter().filter(|t| t.id != PLATINUM && self.live(t.id)).all(|t| self.has(t.id))
    }

    fn add(&mut self, id: &str, now: u64) -> bool {
        if self.has(id) || trophy(id).is_none() {
            return false;
        }
        self.earned.push((id.to_string(), now));
        true
    }

    /// Earns every trophy the facts now earn (and Platinum when that leaves none) and returns the new
    /// ones in the order they were earned. `now` is the time, in seconds since 1970.
    pub fn sync(&mut self, facts: &Facts, now: u64) -> Vec<&'static Trophy> {
        let mut new = Vec::new();
        self.held = facts.underworld_held;
        if self.manual {
            return new;
        }
        let held = self.held;
        for t in TROPHIES.iter().filter(|t| t.id != PLATINUM && !crate::release::trophy_held(t.id, held)) {
            if (t.earned)(facts) && self.add(t.id, now) {
                new.push(t);
            }
        }
        if self.all_but_platinum() && self.add(PLATINUM, now) {
            new.extend(trophy(PLATINUM));
        }
        new
    }

    /// Testing: whether trophies are only given and taken by hand (see `give` and `take`).
    pub fn manual(&self) -> bool {
        self.manual
    }

    pub fn set_manual(&mut self, on: bool) {
        self.manual = on;
    }

    /// Testing: gives a trophy by hand (whatever the facts say). Returns it if it was not earned yet.
    pub fn give(&mut self, id: &str, now: u64) -> Option<&'static Trophy> {
        if self.add(id, now) {
            trophy(id)
        } else {
            None
        }
    }

    /// Testing: takes a trophy away by hand. Platinum goes with any other trophy (it needs all of
    /// them). True when something was taken.
    pub fn take(&mut self, id: &str) -> bool {
        let before = self.earned.len();
        if trophy(id).is_none() {
            return false;
        }
        self.earned.retain(|(e, _)| e != id && !(id != PLATINUM && e == PLATINUM));
        self.earned.len() != before
    }

    /// Every trophy as it is shown: Platinum first, then the earned ones (newest first) and then the
    /// ones still to earn, the hidden ones last and told about as little as possible.
    pub fn lines(&self) -> Vec<Line> {
        let line = |t: &'static Trophy| {
            let earned = self.has(t.id);
            let secret = t.hidden && !earned;
            Line {
                id: t.id,
                name: if secret { HIDDEN_NAME.to_string() } else { t.name.to_string() },
                description: if secret { HIDDEN_HINT.to_string() } else { t.description.to_string() },
                earned,
                secret,
                tier: if secret { Tier::Bronze } else { t.tier },
                when: self.when(t.id),
            }
        };
        let mut out = vec![line(trophy(PLATINUM).expect("platinum is in the list"))];
        // earned: newest first
        for (id, _) in self.earned.iter().rev() {
            if let Some(t) = trophy(id).filter(|t| t.id != PLATINUM && self.live(t.id)) {
                out.push(line(t));
            }
        }
        let todo = |hidden: bool| TROPHIES.iter().filter(move |t| t.id != PLATINUM && self.live(t.id) && !self.has(t.id) && t.hidden == hidden);
        out.extend(todo(false).map(line));
        out.extend(todo(true).map(line));
        out
    }

    /// The config file's line.
    pub fn to_text(&self) -> String {
        let list: Vec<String> = self.earned.iter().map(|(id, t)| format!("{id}:{t}")).collect();
        let mut text = format!("trophies={}\n", list.join(","));
        if self.manual {
            text.push_str("trophies_manual=1\n");
        }
        text
    }

    /// Reads one config line; false when it is not ours. Ids that do not exist (a trophy that was
    /// taken out, a mistake in the file) are dropped, and so are repeats.
    pub fn read_line(&mut self, name: &str, value: &str) -> bool {
        if name == "trophies_manual" {
            self.manual = value.trim() == "1" || value.trim().eq_ignore_ascii_case("true");
            return true;
        }
        if name != "trophies" {
            return false;
        }
        self.earned.clear();
        for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (id, when) = part.split_once(':').unwrap_or((part, "0"));
            self.add(id.trim(), when.trim().parse().unwrap_or(0));
        }
        true
    }

    /// Testing: takes every trophy away again.
    pub fn clear(&mut self) {
        self.earned.clear();
    }
}

/// A date, as "2026-10-18", from seconds since 1970 (UTC).
pub fn date(secs: u64) -> String {
    // (days since 1970 to a calendar date: Howard Hinnant's civil-from-days)
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rich() -> Facts {
        Facts {
            won: [true; 3],
            deaths: [40, 40, 40],
            cheats_earned: 5,
            cheats_total: 5,
            act1: true,
            gold: true,
            void: true,
            underworld: true,
            bought: 19,
            for_sale: 19,
            skins_bought: 9,
            skins_for_sale: 9,
            tokens: 900,
            whispers: 12,
            whispers_total: 12,
            poker_played: 3,
            poker_won: 1,
            poker_takes: 1,
            poker_together: true,
            underworld_held: false,
        }
    }

    #[test]
    fn ids_are_unique_and_every_trophy_is_described() {
        for (i, t) in TROPHIES.iter().enumerate() {
            assert!(!t.name.is_empty() && t.description.len() > 8, "{}", t.id);
            assert!(TROPHIES.iter().skip(i + 1).all(|o| o.id != t.id), "{} twice", t.id);
            assert!(!t.id.contains([',', ':', '=', '\n']), "{} would break the save file", t.id);
        }
        assert_eq!(TROPHIES[0].id, PLATINUM, "platinum is listed first");
        assert!(!trophy(PLATINUM).unwrap().hidden && trophy(PLATINUM).unwrap().tier == Tier::Platinum);
    }

    #[test]
    fn act_one_is_earned_by_agreeing_to_help_the_cat_and_nothing_else() {
        let t = trophy("act1").unwrap();
        assert!(t.hidden && t.tier == Tier::Bronze);
        assert!(!(t.earned)(&Facts::default()));
        assert!((t.earned)(&Facts { act1: true, ..Facts::default() }));
        // it is not one of the held-back Underworld trophies: it counts for Platinum now
        assert!(!crate::release::UNDERWORLD_TROPHIES.contains(&"act1"));
    }

    #[test]
    fn the_worlds_are_hidden_and_the_ordinary_ones_are_not() {
        for id in ["gold", "void", "underworld", "listener", "act1"] {
            assert!(trophy(id).unwrap().hidden, "{id}");
        }
        for id in ["ten_deaths", "game1_win", "first_buy", "wardrobe"] {
            assert!(!trophy(id).unwrap().hidden, "{id}");
        }
    }

    #[test]
    fn while_the_underworld_is_held_its_trophies_are_not_listed_earned_or_needed_for_platinum() {
        let held = Facts { underworld_held: true, ..rich() };
        let mut book = Book::default();
        let ids: Vec<&str> = book.sync(&held, 1).iter().map(|t| t.id).collect();
        assert!(ids.contains(&PLATINUM), "Platinum comes without them: {ids:?}");
        for id in crate::release::UNDERWORLD_TROPHIES {
            assert!(!ids.contains(&id) && !book.has(id), "{id}");
        }
        let (earned, all) = book.progress();
        assert_eq!(earned, all, "everything that can be earned has been");
        assert_eq!(all, TROPHIES.len() - crate::release::UNDERWORLD_TROPHIES.len());
        assert!(book.lines().iter().all(|l| !crate::release::UNDERWORLD_TROPHIES.contains(&l.id)));
        // when the Underworld arrives, they are there to earn, and Platinum (already earned) stays
        let ids: Vec<&str> = book.sync(&rich(), 2).iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), crate::release::UNDERWORLD_TROPHIES.len(), "{ids:?}");
        assert!(book.has(PLATINUM));
        assert!(!book.holds_underworld());
    }

    #[test]
    fn bringing_the_two_lovers_together_earns_playing_god_and_it_is_hidden() {
        assert!(trophy("poker_together").unwrap().hidden);
        let mut book = Book::default();
        assert!(book.sync(&Facts::default(), 1).is_empty());
        let ids: Vec<&str> = book.sync(&Facts { poker_together: true, ..Facts::default() }, 2).iter().map(|t| t.id).collect();
        assert_eq!(ids, ["poker_together"]);
        assert_eq!(trophy("poker_together").unwrap().name, "Playing God");
    }

    #[test]
    fn hearing_every_whisper_earns_the_listener_trophy_and_not_before() {
        let mut book = Book::default();
        let few = Facts { whispers: 11, whispers_total: 12, ..Facts::default() };
        assert!(book.sync(&few, 1).is_empty());
        let all = Facts { whispers: 12, whispers_total: 12, ..Facts::default() };
        let ids: Vec<&str> = book.sync(&all, 2).iter().map(|t| t.id).collect();
        assert_eq!(ids, ["listener"]);
        // with no whispers to hear there is nothing to earn
        assert!(Book::default().sync(&Facts::default(), 3).is_empty());
        assert!(!trophy("listener").unwrap().description.is_empty());
    }

    #[test]
    fn nothing_is_earned_from_nothing() {
        let mut book = Book::default();
        assert!(book.sync(&Facts::default(), 100).is_empty());
        assert_eq!(book.progress(), (0, TROPHIES.len()));
    }

    #[test]
    fn deaths_wins_and_the_shop_earn_their_trophies() {
        let mut book = Book::default();
        let mut f = Facts { deaths: [4, 3, 3], ..Facts::default() };
        let ids: Vec<&str> = book.sync(&f, 1).iter().map(|t| t.id).collect();
        assert_eq!(ids, ["ten_deaths"]);
        f.won[0] = true;
        f.bought = 1;
        f.tokens = 120;
        let ids: Vec<&str> = book.sync(&f, 2).iter().map(|t| t.id).collect();
        assert_eq!(ids, ["game1_win", "first_buy", "tokens_100"]);
        // nothing is earned twice
        assert!(book.sync(&f, 3).is_empty());
        assert_eq!(book.when("game1_win"), Some(2));
    }

    #[test]
    fn a_trophy_stays_earned_when_the_facts_go_away_again() {
        let mut book = Book::default();
        book.sync(&Facts { gold: true, void: true, ..Facts::default() }, 5);
        assert!(book.has("gold") && book.has("void"));
        // a New Game+ takes the worlds away
        assert!(book.sync(&Facts::default(), 6).is_empty());
        assert!(book.has("gold") && book.has("void"));
    }

    #[test]
    fn platinum_comes_with_the_last_one_and_not_before() {
        let mut book = Book::default();
        let mut f = rich();
        f.underworld = false;
        let new = book.sync(&f, 10);
        assert!(!book.has(PLATINUM), "one is missing");
        assert_eq!(new.len(), TROPHIES.len() - 2);
        f.underworld = true;
        let new: Vec<&str> = book.sync(&f, 11).iter().map(|t| t.id).collect();
        assert_eq!(new, ["underworld", PLATINUM], "the last one, then platinum");
        assert!(book.all_but_platinum() && book.progress() == (TROPHIES.len(), TROPHIES.len()));
    }

    #[test]
    fn a_hidden_trophy_tells_nothing_until_it_is_earned() {
        let mut book = Book::default();
        let lines = book.lines();
        let secret = lines.iter().find(|l| l.id == "underworld").unwrap();
        assert!(secret.secret && !secret.earned);
        assert_eq!(secret.name, HIDDEN_NAME);
        assert_eq!(secret.description, HIDDEN_HINT);
        let all_text: String = lines.iter().filter(|l| l.secret).map(|l| format!("{}{}", l.name, l.description)).collect();
        for t in TROPHIES.iter().filter(|t| t.hidden) {
            assert!(!all_text.contains(t.name) && !all_text.contains(t.description), "{} leaks", t.id);
        }
        book.sync(&Facts { underworld: true, ..Facts::default() }, 9);
        let lines = book.lines();
        let shown = lines.iter().find(|l| l.id == "underworld").unwrap();
        assert!(shown.earned && !shown.secret);
        assert_eq!(shown.name, "Down We Go");
        assert_eq!(shown.when, Some(9));
    }

    #[test]
    fn the_list_has_platinum_first_then_earned_newest_first_then_the_rest_hidden_last() {
        let mut book = Book::default();
        book.sync(&Facts { deaths: [10, 0, 0], ..Facts::default() }, 1);
        book.sync(&Facts { deaths: [10, 0, 0], bought: 1, ..Facts::default() }, 2);
        let lines = book.lines();
        assert_eq!(lines.len(), TROPHIES.len(), "every trophy is listed once");
        assert_eq!(lines[0].id, PLATINUM);
        assert_eq!((lines[1].id, lines[2].id), ("first_buy", "ten_deaths"));
        let first_secret = lines.iter().position(|l| l.secret).unwrap();
        assert!(lines[first_secret..].iter().all(|l| l.secret), "the hidden ones come last");
        assert!(lines[3..first_secret].iter().all(|l| !l.earned && !l.secret));
    }

    #[test]
    fn the_save_line_round_trips_and_ignores_what_it_does_not_know() {
        let mut book = Book::default();
        book.sync(&rich(), 1_760_000_000);
        let text = book.to_text();
        assert!(text.starts_with("trophies=") && text.ends_with('\n'));
        let mut back = Book::default();
        assert!(back.read_line("trophies", text.trim_end().strip_prefix("trophies=").unwrap()));
        assert_eq!(back, book);
        let mut odd = Book::default();
        odd.read_line("trophies", "gold:5, nonsense:7, gold:9, void, ,underworld:x");
        assert_eq!(odd.count(), 3);
        assert_eq!(odd.when("gold"), Some(5), "the first one stands");
        assert_eq!(odd.when("void"), None, "no time written down");
        assert!(odd.has("underworld") && !odd.has("nonsense"));
        assert!(!odd.read_line("something_else", "x"));
    }

    #[test]
    fn dates_are_right() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(951_782_400), "2000-02-29");
        assert_eq!(date(1_760_745_600), "2025-10-18");
        assert_eq!(date(1_781_740_800), "2026-06-18");
    }

    #[test]
    fn cheats_and_the_shop_trophies_need_something_to_exist() {
        // an empty shop is not "sold out", and no cheats is not "every cheat"
        let f = Facts::default();
        assert!(!(trophy("sold_out").unwrap().earned)(&f));
        assert!(!(trophy("wardrobe").unwrap().earned)(&f));
        assert!(!(trophy("cheat_all").unwrap().earned)(&f));
    }

    #[test]
    fn by_hand_trophies_can_be_given_and_taken_and_playing_earns_nothing() {
        let mut book = Book::default();
        book.set_manual(true);
        assert!(book.sync(&rich(), 1).is_empty() && book.count() == 0, "playing earns nothing in manual mode");
        let t = book.give("gold", 5).expect("new");
        assert_eq!(t.name, "Golden Hour");
        assert!(book.give("gold", 6).is_none() && book.give("nonsense", 6).is_none());
        assert_eq!(book.when("gold"), Some(5));
        assert!(book.take("gold") && !book.has("gold"));
        assert!(!book.take("gold") && !book.take("nonsense"));
        // platinum needs the rest: taking one takes it too
        for t in TROPHIES {
            book.give(t.id, 7);
        }
        assert_eq!(book.count(), TROPHIES.len());
        assert!(book.take("void") && !book.has(PLATINUM) && book.count() == TROPHIES.len() - 2);
        // saved: the mode and the trophies
        let text = book.to_text();
        assert!(text.contains("trophies_manual=1"));
        let mut back = Book::default();
        for line in text.lines() {
            let (k, v) = line.split_once('=').unwrap();
            assert!(back.read_line(k, v));
        }
        assert_eq!(back, book);
        // and back to earning by playing
        back.set_manual(false);
        assert!(!back.to_text().contains("manual"));
        assert!(!back.sync(&rich(), 9).is_empty());
    }
}
