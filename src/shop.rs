//! CR-Tokens and the shop: what the mini-games pay, what the things in the shop cost, and what is
//! kept between runs (the balance, what is owned, which skin is on).
//!
//! The currency is the CR-Token, drawn as the CR logo. It is earned in the three hidden games and
//! spent in the Shop (the top of Options, once the first token has been earned). Skins and
//! backdrops are bought once and kept. Trying one on is always free.
//!
//! The rules are meant to be easy: most things cost about two or three runs, and a player who just
//! wants to see everything should get there in a short while. Everything is a plain function here
//! so that all of it is tested; nothing draws anything.

use crate::games::Difficulty;

/// What a difficulty multiplies a payout by: Hard pays in full, Normal three quarters, Easy half.
pub fn difficulty_percent(d: Difficulty) -> u32 {
    match d {
        Difficulty::Hard => 100,
        Difficulty::Normal => 75,
        Difficulty::Easy => 50,
    }
}

/// The first win of each game pays this much on top.
pub const FIRST_WIN_BONUS: u32 = 50;
/// A win after the first one pays this much on top (the game's own number, see `Run`).
const REPEAT_WIN_GAME1: u32 = 30;
const REPEAT_WIN_GAME2: u32 = 40;
/// Game 3 pays by accuracy: this for clearing a song, plus up to `GAME3_ACCURACY` more.
const GAME3_CLEAR: u32 = 30;
const GAME3_ACCURACY: u32 = 30;
/// A token for every this many seconds survived in Game 1, and for every angel shot in Game 2
/// (two each).
const GAME1_SECS_PER_TOKEN: f32 = 6.0;
const GAME2_PER_KILL: u32 = 2;
/// What the death total is worth when the wallet opens (once): this many tokens for each death, up
/// to `START_CAP` in all.
pub const START_PER_DEATH: u32 = 1;
pub const START_CAP: u32 = 60;

/// How a run ended, as far as paying for it goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Run {
    /// Game 1: `secs` survived, and whether the clock ran out with the cat alive.
    Game1 { secs: f32, won: bool },
    /// Game 2: angels shot, and whether the king fell.
    Game2 { kills: u32, won: bool },
    /// Game 3: a story song. `progress` is how far through the music you got (0 to 1; 1 when it
    /// was cleared), `accuracy` 0 to 1.
    Game3 { progress: f32, accuracy: f32, cleared: bool },
}

impl Run {
    /// The game this run belongs to (0 is Game 1).
    pub fn game(&self) -> usize {
        match self {
            Run::Game1 { .. } => 0,
            Run::Game2 { .. } => 1,
            Run::Game3 { .. } => 2,
        }
    }

    /// Whether the run was won (a song cleared).
    pub fn won(&self) -> bool {
        match *self {
            Run::Game1 { won, .. } | Run::Game2 { won, .. } => won,
            Run::Game3 { cleared, .. } => cleared,
        }
    }

    /// What the run earns before the difficulty and the first-win bonus: the part that comes from
    /// how well it went.
    fn base(&self) -> u32 {
        match *self {
            Run::Game1 { secs, .. } => (secs.max(0.0) / GAME1_SECS_PER_TOKEN) as u32,
            Run::Game2 { kills, .. } => kills * GAME2_PER_KILL,
            Run::Game3 { progress, accuracy, cleared } => {
                if cleared {
                    GAME3_CLEAR + (accuracy.clamp(0.0, 1.0) * GAME3_ACCURACY as f32).round() as u32
                } else {
                    // a failed song still pays for the part you played: up to a fifth of a clear
                    (progress.clamp(0.0, 1.0) * 10.0) as u32
                }
            }
        }
    }

    /// What winning pays on top of `base`, for a win that is not the game's first.
    fn win_bonus(&self) -> u32 {
        match self {
            Run::Game1 { .. } => REPEAT_WIN_GAME1,
            Run::Game2 { .. } => REPEAT_WIN_GAME2,
            Run::Game3 { .. } => 0,
        }
    }
}

/// The tokens a run pays. `first_win` is whether this is the first time the game has been won (it
/// pays `FIRST_WIN_BONUS` on top of the usual bonus, which it replaces). `cheated` is true when any
/// cheat was on: a cheating run pays nothing.
pub fn payout(run: Run, d: Difficulty, first_win: bool, cheated: bool) -> u32 {
    if cheated {
        return 0;
    }
    let bonus = if !run.won() {
        0
    } else if first_win {
        FIRST_WIN_BONUS
    } else {
        run.win_bonus()
    };
    (run.base() + bonus) * difficulty_percent(d) / 100
}

/// What kind of thing a shop item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A whole new look for the program.
    Skin,
    /// A picture behind the tabs.
    Backdrop,
}

/// One thing the shop sells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    pub kind: Kind,
    pub price: u32,
}

/// The prices, by how much work went into the thing.
pub const PRICE_BACKDROP_SMALL: u32 = 20;
pub const PRICE_BACKDROP: u32 = 30;
pub const PRICE_BACKDROP_BIG: u32 = 45;
/// A skin that changes the colours and the card edges.
pub const PRICE_SKIN_BASIC: u32 = 40;
/// A skin with a trim of its own on every card (bubbles, a monospaced typeface and brackets).
pub const PRICE_SKIN_MID: u32 = 60;
/// A premium skin: one that paints a scene across the window as well.
pub const PRICE_SKIN_PREMIUM: u32 = 80;

/// Everything the shop sells, skins first. (Skins that are earned by playing, not bought, are not
/// here.)
pub const CATALOG: &[Item] = &[
    Item {
        id: "fraying",
        name: "Fraying",
        blurb: "Ash-grey cards with ragged, unravelling edges and a faded accent.",
        kind: Kind::Skin,
        price: PRICE_SKIN_BASIC,
    },
    Item {
        id: "sakura",
        name: "Sakura",
        blurb: "Ink-dark plum and indigo, cream paper text, blossom pink and vermilion. Cards get a thin shoji frame, a seal and a blossom.",
        kind: Kind::Skin,
        price: PRICE_SKIN_PREMIUM,
    },
    Item {
        id: "synthwave",
        name: "Synthwave",
        blurb: "Deep indigo with hot magenta and cyan neon: glowing lines top and bottom of every card.",
        kind: Kind::Skin,
        price: PRICE_SKIN_BASIC,
    },
    Item {
        id: "prism",
        name: "Prism",
        blurb: "Near black, with an accent and a card edge that slowly go round the whole colour wheel.",
        kind: Kind::Skin,
        price: PRICE_SKIN_BASIC,
    },
    Item {
        id: "aurora",
        name: "Aurora",
        blurb: "Dark teal with northern-lights green, and card edges that shimmer from green to violet.",
        kind: Kind::Skin,
        price: PRICE_SKIN_BASIC,
    },
    Item {
        id: "deepsea",
        name: "Deep Sea",
        blurb: "Navy and teal, with a string of bubbles along the bottom of every card and a column rising up its side.",
        kind: Kind::Skin,
        price: PRICE_SKIN_MID,
    },
    Item {
        id: "frost",
        name: "Frost",
        blurb: "A winter pine forest: ice-blue cards with snow on top, icicles underneath, cracked corners and a little pine tree, and snowy pines along the bottom of the window.",
        kind: Kind::Skin,
        price: PRICE_SKIN_PREMIUM,
    },
    Item {
        id: "terminal",
        name: "Terminal",
        blurb: "Green monospaced text on black, square cards with corner brackets and faint scanlines.",
        kind: Kind::Skin,
        price: PRICE_SKIN_MID,
    },
    Item {
        id: "descent",
        name: "Descent",
        blurb: "Near black with a deep red glow rising from below, sharp square cards, and a single red thread running down into the dark.",
        kind: Kind::Skin,
        price: PRICE_SKIN_PREMIUM,
    },
    Item {
        id: "embers",
        name: "Embers",
        blurb: "Sparks drifting up from below, cooling as they rise.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP_SMALL,
    },
    Item {
        id: "datarain",
        name: "Data rain",
        blurb: "Columns of falling characters in the accent colour.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP,
    },
    Item {
        id: "petals",
        name: "Sakura petals",
        blurb: "Cherry-blossom petals drifting down, turning as they fall.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP,
    },
    Item {
        id: "synthgrid",
        name: "Synthwave grid",
        blurb: "A striped sun on the horizon and a neon grid rolling towards you.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP_BIG,
    },
    Item {
        id: "northern",
        name: "Northern lights",
        blurb: "Slow curtains of colour that sway and fade.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP_BIG,
    },
    Item {
        id: "bubbles",
        name: "Bubbles",
        blurb: "Slow bubbles rising through beams of light from above.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP,
    },
    Item {
        id: "scanlines",
        name: "Scanlines",
        blurb: "Fine screen lines with a bright band rolling down and a blinking cursor.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP,
    },
    Item {
        id: "fireflies",
        name: "Fireflies",
        blurb: "Soft lights wandering about and blinking on and off.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP_SMALL,
    },
    Item {
        id: "snowfall",
        name: "Snowfall",
        blurb: "Snow drifting down, the nearer flakes bigger and faster.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP_SMALL,
    },
    Item {
        id: "storm",
        name: "Storm",
        blurb: "Rain slanting down, with lightning flashing far off.",
        kind: Kind::Backdrop,
        price: PRICE_BACKDROP_BIG,
    },
];

/// Skins that are earned, not bought: the shop does not sell them (`Wallet::grant` gives them).
pub const EARNED: &[Item] = &[Item {
    id: "platinum",
    name: "Platinum",
    blurb: "A cut-gem look: glimmering crystal facets with a band of light sweeping across, bevelled cards with a medallion and corner gems, and prism glints. Earned with the Platinum trophy.",
    kind: Kind::Skin,
    price: 0,
}];

/// Skins that come with the program: everyone has them from the start (nothing is bought or earned).
pub const PACKAGED: &[Item] = &[
    Item {
        id: "midnight",
        name: "Midnight",
        blurb: "Deep blue-black with a periwinkle accent and a thin starlit line along the top of every card.",
        kind: Kind::Skin,
        price: 0,
    },
    Item {
        id: "copper",
        name: "Copper",
        blurb: "Warm charcoal with a copper accent and a copper corner on every card.",
        kind: Kind::Skin,
        price: 0,
    },
];

/// Whether the item comes with the program (see `PACKAGED`).
pub fn is_packaged(id: &str) -> bool {
    PACKAGED.iter().any(|i| i.id == id)
}

/// The item with this id: sold, earned or packaged.
pub fn item(id: &str) -> Option<&'static Item> {
    CATALOG.iter().chain(EARNED).chain(PACKAGED).find(|i| i.id == id)
}

/// Why a purchase did not go through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyError {
    Unknown,
    AlreadyOwned,
    TooExpensive { missing: u32 },
}

/// The ids of the games for the first-win flags.
const GAME_COUNT: usize = 3;

/// What is kept between runs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Wallet {
    balance: u32,
    lifetime: u64,
    owned: Vec<String>,
    /// The skin that is on ("" for none).
    skin: String,
    /// Whether the wallet was opened with the starting tokens (the death total).
    started: bool,
    /// Which games have been won for the first time.
    first_wins: [bool; GAME_COUNT],
    /// Whether the player has been told about the shop.
    revealed: bool,
}

impl Wallet {
    pub fn balance(&self) -> u32 {
        self.balance
    }

    /// Every token ever earned.
    pub fn lifetime(&self) -> u64 {
        self.lifetime
    }

    /// Whether Options shows the shop: after the first token.
    pub fn visible(&self) -> bool {
        self.lifetime > 0 || self.revealed
    }

    /// Whether the shop's welcome has been shown.
    pub fn revealed(&self) -> bool {
        self.revealed
    }

    pub fn mark_revealed(&mut self) {
        self.revealed = true;
    }

    /// Opens the wallet with the starting tokens, `START_PER_DEATH` for each death so far, the
    /// first time it is called. Returns what was given (0 after the first time).
    pub fn open(&mut self, total_deaths: u32) -> u32 {
        if self.started {
            return 0;
        }
        self.started = true;
        let n = total_deaths.saturating_mul(START_PER_DEATH).min(START_CAP);
        self.earn(n);
        n
    }

    pub fn earn(&mut self, n: u32) {
        self.balance = self.balance.saturating_add(n);
        self.lifetime = self.lifetime.saturating_add(n as u64);
    }

    /// Takes `n` tokens (a buy-in). False, and nothing taken, when there are not enough.
    pub fn spend(&mut self, n: u32) -> bool {
        if self.balance < n {
            return false;
        }
        self.balance -= n;
        true
    }

    /// Whether `game` has been won before this run; marks it won. True only the first time.
    pub fn take_first_win(&mut self, game: usize) -> bool {
        match self.first_wins.get_mut(game) {
            Some(slot) if !*slot => {
                *slot = true;
                true
            }
            _ => false,
        }
    }

    /// Whether the first win of `game` is still to come (without marking it).
    pub fn first_win_pending(&self, game: usize) -> bool {
        self.first_wins.get(game).map(|w| !*w).unwrap_or(false)
    }

    pub fn owns(&self, id: &str) -> bool {
        if is_packaged(id) {
            return true;
        }
        if crate::release::story() {
            self.owned.iter().any(|o| o == id)
        } else {
            // a build without the story has no tokens and no shop: its backdrops are free, its skins are not
            // for anyone (what an older save owns is kept in the file but not used)
            CATALOG.iter().any(|i| i.id == id && i.kind == Kind::Backdrop)
        }
    }

    /// Gives an earned skin (see `EARNED`). Anything else, or one that is already owned, is ignored;
    /// nothing is paid. True when it was given just now.
    pub fn grant(&mut self, id: &str) -> bool {
        if !EARNED.iter().any(|i| i.id == id) || self.owns(id) {
            return false;
        }
        self.owned.push(id.to_string());
        true
    }

    /// Whether `game` (0 to 2) has been won without a cheat, for the trophies.
    pub fn has_won(&self, game: usize) -> bool {
        self.first_wins.get(game).copied().unwrap_or(false)
    }

    /// Testing: takes an earned skin away again (and it is taken off if it was on).
    pub fn revoke(&mut self, id: &str) {
        if EARNED.iter().any(|i| i.id == id) {
            self.owned.retain(|o| o != id);
            if self.skin == id {
                self.skin.clear();
            }
        }
    }

    /// How many things have been bought.
    pub fn owned_count(&self) -> usize {
        self.owned.iter().filter(|id| CATALOG.iter().any(|i| i.id == id.as_str())).count()
    }

    /// What the shop still sells of one kind: everything not owned. It is worked out from what is
    /// owned each time, so an item that stops being owned is for sale again.
    pub fn for_sale(&self, kind: Kind) -> Vec<&'static Item> {
        CATALOG.iter().filter(|i| i.kind == kind && !self.owns(i.id)).collect()
    }

    pub fn buy(&mut self, id: &str) -> Result<(), BuyError> {
        let it = item(id).ok_or(BuyError::Unknown)?;
        if self.owns(id) {
            return Err(BuyError::AlreadyOwned);
        }
        if self.balance < it.price {
            return Err(BuyError::TooExpensive { missing: it.price - self.balance });
        }
        self.balance -= it.price;
        self.owned.push(id.to_string());
        Ok(())
    }

    /// The skin that is on, if it is still owned.
    pub fn skin(&self) -> Option<&str> {
        let s = self.skin.as_str();
        (!s.is_empty() && self.owns(s)).then_some(s)
    }

    /// Puts a skin on (None takes it off). A skin that is not owned is ignored.
    pub fn set_skin(&mut self, id: Option<&str>) {
        match id {
            None => self.skin.clear(),
            Some(id) if self.owns(id) && item(id).map(|i| i.kind == Kind::Skin).unwrap_or(false) => self.skin = id.to_string(),
            Some(_) => {}
        }
    }

    /// Testing: sets the balance. A balance above 0 brings the shop up, and the starting tokens
    /// are not given afterwards (they would be on top of what was set).
    pub fn set_balance(&mut self, n: u32) {
        self.balance = n;
        if n > 0 {
            self.started = true;
            self.revealed = true;
        }
    }

    /// Testing: empties the balance (what was earned in all stays counted).
    pub fn reset_balance(&mut self) {
        self.balance = 0;
    }

    /// Testing: takes back everything that was bought (no refund) and the skin that was on.
    pub fn reset_purchases(&mut self) {
        self.owned.clear();
        self.skin.clear();
    }

    /// Testing: back to a wallet nobody has touched: no tokens, nothing bought, no first wins, the
    /// shop hidden again and the starting tokens to come.
    pub fn reset_all(&mut self) {
        *self = Wallet::default();
    }

    /// The config file's lines.
    pub fn to_text(&self) -> String {
        format!(
            "shop_balance={}\nshop_lifetime={}\nshop_owned={}\nshop_skin={}\nshop_started={}\nshop_first_wins={}\nshop_revealed={}\n",
            self.balance,
            self.lifetime,
            self.owned.join(","),
            self.skin,
            self.started as u8,
            self.first_wins.iter().map(|w| (*w as u8).to_string()).collect::<Vec<_>>().join(","),
            self.revealed as u8,
        )
    }

    /// Reads one config line; false when it is not one of ours.
    pub fn read_line(&mut self, name: &str, value: &str) -> bool {
        let flag = |v: &str| v.trim() == "1" || v.trim().eq_ignore_ascii_case("true");
        match name {
            "shop_balance" => self.balance = value.trim().parse().unwrap_or(0),
            "shop_lifetime" => self.lifetime = value.trim().parse().unwrap_or(0),
            "shop_owned" => {
                self.owned.clear();
                for id in value.split(',').map(str::trim) {
                    if item(id).is_some() && !self.owns(id) {
                        self.owned.push(id.to_string());
                    }
                }
            }
            "shop_skin" => self.skin = value.trim().to_string(),
            "shop_started" => self.started = flag(value),
            "shop_first_wins" => {
                for (slot, p) in self.first_wins.iter_mut().zip(value.split(',')) {
                    *slot = flag(p);
                }
            }
            "shop_revealed" => self.revealed = flag(value),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn spending_takes_tokens_and_never_more_than_there_are() {
        let mut w = Wallet::default();
        w.earn(25);
        assert!(!w.spend(26));
        assert_eq!(w.balance(), 25);
        assert!(w.spend(20));
        assert_eq!(w.balance(), 5);
        assert_eq!(w.lifetime(), 25, "spending does not undo what was earned");
        assert!(w.spend(5) && w.balance() == 0);
        assert!(!w.spend(1));
    }

    use super::*;

    #[test]
    fn a_hard_clear_of_game_one_pays_the_first_win_bonus_then_the_repeat() {
        let run = Run::Game1 { secs: 60.0, won: true };
        assert_eq!(payout(run, Difficulty::Hard, true, false), 10 + FIRST_WIN_BONUS);
        assert_eq!(payout(run, Difficulty::Hard, false, false), 10 + 30);
    }

    #[test]
    fn a_failed_run_pays_for_the_time_survived_only() {
        let run = Run::Game1 { secs: 31.0, won: false };
        assert_eq!(payout(run, Difficulty::Hard, true, false), 5, "no win bonus, even when it would be the first");
        assert_eq!(payout(Run::Game2 { kills: 10, won: false }, Difficulty::Hard, false, false), 20);
    }

    #[test]
    fn easier_difficulties_pay_less() {
        let run = Run::Game2 { kills: 20, won: true };
        let hard = payout(run, Difficulty::Hard, false, false);
        let normal = payout(run, Difficulty::Normal, false, false);
        let easy = payout(run, Difficulty::Easy, false, false);
        assert_eq!((hard, normal, easy), (80, 60, 40));
    }

    #[test]
    fn cheats_pay_nothing() {
        for run in [
            Run::Game1 { secs: 60.0, won: true },
            Run::Game2 { kills: 25, won: true },
            Run::Game3 { progress: 1.0, accuracy: 1.0, cleared: true },
        ] {
            assert_eq!(payout(run, Difficulty::Hard, true, true), 0);
        }
    }

    #[test]
    fn game_three_pays_by_accuracy_and_a_failure_pays_a_little() {
        let clear = |a: f32| Run::Game3 { progress: 1.0, accuracy: a, cleared: true };
        assert_eq!(payout(clear(1.0), Difficulty::Hard, false, false), 60);
        assert_eq!(payout(clear(0.5), Difficulty::Hard, false, false), 45);
        assert_eq!(payout(clear(0.9), Difficulty::Hard, true, false), 57 + FIRST_WIN_BONUS);
        assert_eq!(payout(Run::Game3 { progress: 0.5, accuracy: 0.9, cleared: false }, Difficulty::Hard, false, false), 5);
        assert_eq!(payout(Run::Game3 { progress: 5.0, accuracy: 9.0, cleared: false }, Difficulty::Hard, false, false), 10, "out of range is clamped");
        assert_eq!(payout(Run::Game3 { progress: -1.0, accuracy: -1.0, cleared: false }, Difficulty::Hard, false, false), 0);
    }

    #[test]
    fn every_run_knows_its_game() {
        assert_eq!(Run::Game1 { secs: 0.0, won: false }.game(), 0);
        assert_eq!(Run::Game2 { kills: 0, won: false }.game(), 1);
        assert_eq!(Run::Game3 { progress: 0.0, accuracy: 0.0, cleared: false }.game(), 2);
    }

    #[test]
    fn the_catalog_is_consistent() {
        for (i, a) in CATALOG.iter().enumerate() {
            assert!(!a.name.is_empty() && !a.blurb.is_empty() && a.price > 0);
            assert!(CATALOG.iter().skip(i + 1).all(|b| b.id != a.id), "ids are unique");
            assert_eq!(item(a.id), Some(a));
        }
        assert!(item("nope").is_none());
        assert!(CATALOG.iter().any(|i| i.kind == Kind::Skin) && CATALOG.iter().any(|i| i.kind == Kind::Backdrop));
    }

    #[test]
    fn a_fresh_wallet_is_hidden_and_the_first_token_shows_the_shop() {
        let mut w = Wallet::default();
        assert!(!w.visible() && w.balance() == 0);
        w.earn(1);
        assert!(w.visible() && w.balance() == 1 && w.lifetime() == 1);
    }

    #[test]
    fn the_wallet_opens_once_with_a_token_for_every_death_up_to_a_cap() {
        let mut w = Wallet::default();
        assert_eq!(w.open(40), 40);
        assert_eq!(w.balance(), 40);
        assert_eq!(w.open(99), 0, "only once");
        assert_eq!(w.balance(), 40);
        assert_eq!(Wallet::default().open(500), START_CAP, "a lot of deaths is capped");
        assert_eq!(Wallet::default().open(0), 0);
    }

    #[test]
    fn buying_takes_the_price_and_cannot_be_done_twice_or_without_the_tokens() {
        let mut w = Wallet::default();
        assert_eq!(w.buy("nope"), Err(BuyError::Unknown));
        w.earn(30);
        assert_eq!(w.buy("fraying"), Err(BuyError::TooExpensive { missing: 10 }));
        assert_eq!(w.balance(), 30);
        w.earn(10);
        assert_eq!(w.buy("fraying"), Ok(()));
        assert_eq!(w.balance(), 0);
        assert!(w.owns("fraying"));
        assert_eq!(w.buy("fraying"), Err(BuyError::AlreadyOwned));
        assert_eq!(w.lifetime(), 40, "spending does not lower what was earned");
    }

    #[test]
    fn a_skin_must_be_owned_to_be_put_on_and_a_backdrop_is_not_a_skin() {
        let mut w = Wallet::default();
        w.set_skin(Some("fraying"));
        assert_eq!(w.skin(), None);
        w.earn(100);
        w.buy("fraying").unwrap();
        w.buy("embers").unwrap();
        w.set_skin(Some("embers"));
        assert_eq!(w.skin(), None);
        w.set_skin(Some("fraying"));
        assert_eq!(w.skin(), Some("fraying"));
        w.set_skin(None);
        assert_eq!(w.skin(), None);
    }

    #[test]
    fn the_first_win_is_counted_once_per_game() {
        let mut w = Wallet::default();
        assert!(w.first_win_pending(1));
        assert!(w.take_first_win(1));
        assert!(!w.take_first_win(1));
        assert!(!w.first_win_pending(1) && w.first_win_pending(0));
        assert!(!w.take_first_win(9), "no such game");
    }

    #[test]
    fn the_testing_resets_and_the_balance_editor_do_what_they_say() {
        let mut w = Wallet::default();
        w.open(10);
        w.earn(200);
        w.buy("fraying").unwrap();
        w.set_skin(Some("fraying"));
        w.take_first_win(0);
        w.reset_purchases();
        assert!(!w.owns("fraying") && w.skin().is_none());
        assert_eq!(w.balance(), 170, "no refund, nothing else touched");
        assert!(!w.first_win_pending(0));
        w.reset_balance();
        assert_eq!(w.balance(), 0);
        assert!(w.visible() && w.lifetime() == 210, "still counted as earned");
        w.set_balance(500);
        assert_eq!(w.balance(), 500);
        assert_eq!(w.open(50), 0, "the starting tokens are not added to a set balance");
        w.reset_all();
        assert_eq!(w, Wallet::default());
        assert!(!w.visible());
        let mut fresh = Wallet::default();
        fresh.set_balance(7);
        assert!(fresh.visible() && fresh.open(5) == 0);
        fresh.set_balance(0);
        assert_eq!(fresh.balance(), 0);
    }

    #[test]
    fn the_wallet_survives_the_config_file() {
        let mut w = Wallet::default();
        w.open(12);
        w.earn(200);
        w.buy("fraying").unwrap();
        w.buy("datarain").unwrap();
        w.set_skin(Some("fraying"));
        w.take_first_win(2);
        w.mark_revealed();
        let mut back = Wallet::default();
        for line in w.to_text().lines() {
            let (k, v) = line.split_once('=').unwrap();
            assert!(back.read_line(k, v), "{k}");
        }
        assert_eq!(back, w);
        assert!(!back.read_line("something_else", "1"));
    }

    #[test]
    fn bad_config_values_are_ignored_and_unknown_items_are_dropped() {
        let mut w = Wallet::default();
        w.read_line("shop_balance", "lots");
        w.read_line("shop_owned", "fraying, bogus, fraying");
        w.read_line("shop_skin", "bogus");
        assert_eq!(w.balance(), 0);
        assert_eq!(w.owned, vec!["fraying".to_string()]);
        assert_eq!(w.skin(), None, "not owned: not on");
    }

    #[test]
    fn skins_come_in_three_price_tiers() {
        let prices: Vec<(&str, u32)> = CATALOG.iter().filter(|i| i.kind == Kind::Skin).map(|i| (i.id, i.price)).collect();
        assert!(prices.iter().all(|(_, p)| [40, 60, 80].contains(p)), "{prices:?}");
        for tier in [40, 60, 80] {
            assert!(prices.iter().any(|(_, p)| *p == tier), "nothing at {tier}");
        }
    }

    #[test]
    fn what_is_bought_leaves_the_shop_and_returns_if_it_is_lost() {
        let mut w = Wallet::default();
        w.read_line("shop_balance", "500");
        let before = w.for_sale(Kind::Skin).len();
        assert_eq!(before, CATALOG.iter().filter(|i| i.kind == Kind::Skin).count());
        w.buy("frost").unwrap();
        assert_eq!(w.for_sale(Kind::Skin).len(), before - 1);
        assert!(w.for_sale(Kind::Skin).iter().all(|i| i.id != "frost"));
        assert_eq!(w.owned_count(), 1);
        // lost from the saved list: for sale again
        w.read_line("shop_owned", "");
        assert_eq!(w.for_sale(Kind::Skin).len(), before);
        assert!(w.for_sale(Kind::Skin).iter().any(|i| i.id == "frost"));
        // everything bought: nothing left
        for item in CATALOG {
            w.read_line("shop_balance", "9999");
            let _ = w.buy(item.id);
        }
        assert!(w.for_sale(Kind::Skin).is_empty() && w.for_sale(Kind::Backdrop).is_empty());
    }

    #[test]
    fn an_earned_skin_is_granted_not_sold() {
        let mut w = Wallet::default();
        assert!(item("platinum").is_some() && CATALOG.iter().all(|i| i.id != "platinum"));
        assert!(w.for_sale(Kind::Skin).iter().all(|i| i.id != "platinum"), "never for sale");
        assert!(!w.grant("frost"), "only earned skins are granted");
        assert!(w.grant("platinum") && w.owns("platinum"));
        assert!(!w.grant("platinum"), "once");
        assert_eq!(w.owned_count(), 0, "it was not bought");
        w.set_skin(Some("platinum"));
        assert_eq!(w.skin(), Some("platinum"));
        // it is saved and read back
        let mut back = Wallet::default();
        for line in w.to_text().lines() {
            let (k, v) = line.split_once('=').unwrap();
            back.read_line(k, v);
        }
        assert!(back.owns("platinum") && back.skin() == Some("platinum"));
    }

    #[test]
    fn the_packaged_skins_are_everyones_from_the_start_and_cannot_be_bought() {
        let mut w = Wallet::default();
        for id in ["midnight", "copper"] {
            assert!(is_packaged(id) && w.owns(id) && item(id).is_some_and(|i| i.kind == Kind::Skin && i.price == 0));
            w.set_skin(Some(id));
            assert_eq!(w.skin(), Some(id));
            assert_eq!(w.buy(id), Err(BuyError::AlreadyOwned));
        }
        assert!(!is_packaged("sakura") && !w.owns("sakura") && !CATALOG.iter().any(|i| is_packaged(i.id)));
    }
}
