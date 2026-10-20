//! The Alley (the first area of the Ally of Darkness): a city canyon you search wall by wall (logic only).
//!
//! It is an escape room in the old browser style: four walls to turn between, things on the walls that
//! open into close-ups, a bar of the things you carry (select one, then click what to use it on, or click
//! a second one to put the two together), and three hints that cost time. Every code comes from clues in
//! the alley itself, and the codes change with each run (`State::seed`). Items are found where they would
//! be: the screwdriver is in a maintenance locker, the foil is a gum wrapper in the dumpster. What takes
//! thought is what you do with them.
//!
//! The chain, in order:
//! 1. The notice board by the dumpster has a work order and a calendar: the circled days are the locker's code.
//! 2. The locker holds a screwdriver, which opens the utility cabinet.
//! 3. The neon sign flickers four digits, one at a time: the code of the vending machine's service door.
//! 4. The service door holds a blown fuse and a token. The dumpster holds a gum wrapper: its foil mends the fuse.
//! 5. The mended fuse goes into the cabinet, which wakes a panel of switches (every light must shine).
//! 6. The power runs up the walls into the clock sign: route the current to it, and it gives up the hour hand.
//! 7. The hour hand lets you look into the past, where the bricked-up passage on the south wall is open.
//!
//! Plain rules on top of `ally::State`; the painter is in `ui/ally.rs`.

use crate::ally::{Puzzle, State};
use crate::ally_room::Era;

// ---------------------------------------------------------------------------------------------- flags
/// The foil was taken from the dumpster.
pub const F_FOIL: u32 = 1;
/// The locker is open.
pub const F_LOCKER: u32 = 1 << 1;
/// The screwdriver was taken.
pub const F_DRIVER: u32 = 1 << 2;
/// The cabinet's screws are out.
pub const F_CABINET: u32 = 1 << 3;
/// The vending machine's service door is open.
pub const F_VENDING: u32 = 1 << 4;
/// The fuse and the token were taken.
pub const F_VENDED: u32 = 1 << 5;
/// A working fuse is in the cabinet.
pub const F_FUSE_IN: u32 = 1 << 6;
/// The panel is solved: the walls have power.
pub const F_POWER: u32 = 1 << 7;
/// Seen the passage open in the past.
pub const F_PASSAGE: u32 = 1 << 8;
/// The shield over the arch is down.
pub const F_SHIELD: u32 = 1 << 9;

/// While only the first area is built, the hour hand alone opens the hatch (the other two come with the
/// later areas).
pub const FIRST_AREA_ONLY: bool = true;

// ---------------------------------------------------------------------------------------------- items
/// A thing you can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Foil,
    BurntFuse,
    Fuse,
    Screwdriver,
    Token,
    Hand(usize),
    WarehouseKey,
}

impl Item {
    /// In the order of the bar.
    pub const ALL: [Item; 9] = [
        Item::Foil,
        Item::BurntFuse,
        Item::Fuse,
        Item::Screwdriver,
        Item::Token,
        Item::Hand(0),
        Item::Hand(1),
        Item::Hand(2),
        Item::WarehouseKey,
    ];

    fn bit(self) -> u32 {
        match self {
            Item::Foil => 1,
            Item::BurntFuse => 1 << 1,
            Item::Fuse => 1 << 2,
            Item::Screwdriver => 1 << 3,
            Item::Token => 1 << 4,
            _ => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Item::Foil => "Gum wrapper",
            Item::BurntFuse => "Blown fuse",
            Item::Fuse => "Mended fuse",
            Item::Screwdriver => "Screwdriver",
            Item::Token => "Brass token",
            Item::Hand(0) => "The hour hand",
            Item::Hand(1) => "The minute hand",
            Item::Hand(_) => "The second hand",
            Item::WarehouseKey => "The warehouse key",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Item::Foil => "Silver foil inside, still bright. It is thin, and it carries current.",
            Item::BurntFuse => "A glass fuse with a snapped, blackened wire. It will not carry anything.",
            Item::Fuse => "The blown fuse, its gap bridged with foil. Rough, and it should hold.",
            Item::Screwdriver => "A flat-head screwdriver with a taped handle: MAINTENANCE.",
            Item::Token => "A heavy brass token, stamped with a circle and a bar. It is for something with a slot.",
            Item::Hand(0) => "A slender hand of a clock. It is a little heavier than it should be, as if it remembers.",
            Item::Hand(1) => "The minute hand. It is cold, and it points forward.",
            Item::Hand(_) => "The second hand. It does not stop shaking.",
            Item::WarehouseKey => "A big iron key with a tag: DOCK 9.",
        }
    }

    pub fn held(self, state: &State) -> bool {
        match self {
            Item::Hand(i) => state.has_hand(i),
            Item::WarehouseKey => state.has_warehouse_key(),
            _ => state.has_item(self.bit()),
        }
    }

    fn give(self, state: &mut State) {
        match self {
            Item::Hand(i) => {
                state.light(i);
                state.take_hand(i);
            }
            Item::WarehouseKey => state.find_warehouse_key(),
            _ => state.give_item(self.bit()),
        }
    }

    fn take(self, state: &mut State) {
        if self.bit() != 0 {
            state.drop_item(self.bit());
        }
    }

    /// What you carry, in the order of the bar.
    pub fn carried(state: &State) -> Vec<Item> {
        Item::ALL.iter().copied().filter(|i| i.held(state)).collect()
    }
}

/// Puts two things together. None when they do not go together (nothing is used up then).
pub fn combine(state: &mut State, a: Item, b: Item) -> Option<&'static str> {
    let pair = |x: Item, y: Item| (a == x && b == y) || (a == y && b == x);
    if pair(Item::Foil, Item::BurntFuse) && Item::Foil.held(state) && Item::BurntFuse.held(state) {
        Item::Foil.take(state);
        Item::BurntFuse.take(state);
        Item::Fuse.give(state);
        return Some("You peel the foil off the wrapper and wind it round the snapped wire, end to end. Ugly, but it is a fuse again.");
    }
    None
}

// ----------------------------------------------------------------------------------------------- views
pub const VIEWS: usize = 4;

pub fn view_name(view: usize) -> &'static str {
    match view % VIEWS {
        0 => "The street",
        1 => "The east wall",
        2 => "Behind you",
        _ => "The west wall",
    }
}

/// What a click can land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotId {
    Cat,
    Neon,
    Vending,
    Dumpster,
    Notice,
    Locker,
    Cabinet,
    ClockSign,
    Hatch,
    Passage,
}

/// A place on a wall (x0, y0, x1, y1 as fractions of the room), in `era` (None: all times). The painter draws
/// each thing in its own rectangle.
pub struct Spot {
    pub view: usize,
    pub era: Option<Era>,
    pub rect: [f32; 4],
    pub id: SpotId,
}

const fn spot(view: usize, rect: [f32; 4], id: SpotId) -> Spot {
    Spot { view, era: None, rect, id }
}

pub const SPOTS: &[Spot] = &[
    spot(0, [0.06, 0.16, 0.33, 0.38], SpotId::Neon),
    spot(0, [0.66, 0.34, 0.84, 0.82], SpotId::Vending),
    spot(0, [0.465, 0.46, 0.535, 0.64], SpotId::Cat),
    spot(1, [0.08, 0.50, 0.46, 0.88], SpotId::Dumpster),
    spot(1, [0.56, 0.20, 0.86, 0.62], SpotId::Notice),
    spot(2, [0.35, 0.07, 0.65, 0.50], SpotId::ClockSign),
    spot(2, [0.34, 0.86, 0.66, 0.99], SpotId::Hatch),
    spot(2, [0.04, 0.34, 0.19, 0.85], SpotId::Passage),
    spot(3, [0.07, 0.28, 0.30, 0.84], SpotId::Locker),
    spot(3, [0.46, 0.24, 0.78, 0.76], SpotId::Cabinet),
];

/// The rectangle of a thing (the painter draws it there).
pub fn rect_of(id: SpotId) -> [f32; 4] {
    SPOTS.iter().find(|s| s.id == id).map_or([0.0; 4], |s| s.rect)
}

/// The thing at a point of a view (fractions of the room).
pub fn spot_at(view: usize, x: f32, y: f32) -> Option<SpotId> {
    SPOTS
        .iter()
        .rev()
        .find(|s| s.view == view % VIEWS && x >= s.rect[0] && x <= s.rect[2] && y >= s.rect[1] && y <= s.rect[3])
        .map(|s| s.id)
}

// ---------------------------------------------------------------------------------------------- codes
/// A small deterministic random source.
struct Rng(u64);

impl Rng {
    fn new(seed: u32, salt: u64) -> Rng {
        Rng((seed as u64 ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15)).wrapping_mul(0xD1B5_4A32_D192_ED03) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() >> 11) as usize % n.max(1)
    }
}

/// The locker's code: three different days of the month (1 to 9), circled on the calendar, read in date order.
pub fn locker_code(seed: u32) -> [u8; 3] {
    let mut rng = Rng::new(seed, 11);
    let mut days = Vec::new();
    while days.len() < 3 {
        let d = 1 + rng.below(9) as u8;
        if !days.contains(&d) {
            days.push(d);
        }
    }
    days.sort_unstable();
    [days[0], days[1], days[2]]
}

/// The vending machine's service door: four digits (never all the same), shown by the neon sign.
pub fn neon_code(seed: u32) -> [u8; 4] {
    let mut rng = Rng::new(seed, 23);
    loop {
        let code = [rng.below(10) as u8, rng.below(10) as u8, rng.below(10) as u8, rng.below(10) as u8];
        if code.iter().any(|d| *d != code[0]) {
            return code;
        }
    }
}

/// How long the neon sign takes to run through its digits and go dark (seconds).
pub const NEON_CYCLE: f32 = 4.0 * 1.5 + 2.0;

/// What the sign shows at a time: the position (0 to 3) and the digit, or None while it is dark.
pub fn neon_at(seed: u32, t: f32) -> Option<(usize, u8)> {
    let phase = t.rem_euclid(NEON_CYCLE);
    let slot = (phase / 1.5) as usize;
    if slot < 4 && phase - slot as f32 * 1.5 < 1.1 {
        Some((slot, neon_code(seed)[slot]))
    } else {
        None
    }
}

// --------------------------------------------------------------------------------------- the switches
/// The panel behind the cabinet: a square of lights, and a press on one turns it and its neighbours over.
/// Every light has to shine. It starts from the solved panel and is pressed a few times, so it can be solved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switches {
    pub size: usize,
    pub on: Vec<bool>,
}

impl Switches {
    pub const SIZE: usize = 4;

    pub fn make(seed: u32) -> Switches {
        let size = Switches::SIZE;
        let mut panel = Switches { size, on: vec![true; size * size] };
        let mut rng = Rng::new(seed, 37);
        let mut pressed = vec![false; size * size];
        let mut count = 0;
        while count < 5 {
            let at = rng.below(size * size);
            if !pressed[at] {
                pressed[at] = true;
                count += 1;
                panel.press(at);
            }
        }
        if panel.solved() {
            panel.press(0);
        }
        panel
    }

    pub fn press(&mut self, at: usize) {
        let (row, col) = (at / self.size, at % self.size);
        let flip = |panel: &mut Switches, r: usize, c: usize| {
            let i = r * panel.size + c;
            panel.on[i] = !panel.on[i];
        };
        flip(self, row, col);
        if row > 0 {
            flip(self, row - 1, col);
        }
        if row + 1 < self.size {
            flip(self, row + 1, col);
        }
        if col > 0 {
            flip(self, row, col - 1);
        }
        if col + 1 < self.size {
            flip(self, row, col + 1);
        }
    }

    pub fn solved(&self) -> bool {
        self.on.iter().all(|b| *b)
    }
}

/// The circuit that lights the clock sign.
pub fn sign_puzzle(seed: u32) -> Puzzle {
    Puzzle::make(7, 300 + seed as u64)
}

// ---------------------------------------------------------------------------------------- what a click does
/// What the painter should do after a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Nothing,
    /// A line to show.
    Say(&'static str),
    /// Something is yours now (with the line).
    Got(Item, &'static str),
    /// A close-up opens.
    Look(Look),
    /// The cat's talk.
    Talk,
    /// The clock sign's circuit opens.
    Circuit,
    /// You go out through the hatch.
    Out,
    /// The passage in the past.
    Passage,
}

/// The close-ups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Notice,
    Locker,
    Neon,
    Vending,
    Panel,
    /// The shield over the arch: a code-breaking puzzle.
    Shield,
}

/// The unused item `on`, used on `id` (or just a click when `item` is None).
pub fn click(state: &mut State, id: SpotId, item: Option<Item>, era: Era) -> Reaction {
    use SpotId::*;
    match id {
        Cat => Reaction::Talk,
        Neon => match item {
            Some(_) => Reaction::Say("That is not going to do anything for a sign made of glass and gas."),
            None => Reaction::Look(Look::Neon),
        },
        Vending => {
            if state.flag(F_VENDING) {
                return Reaction::Say("Dead as a stone. The service bay is open and empty now.");
            }
            match item {
                Some(Item::Screwdriver) => Reaction::Say("The service door has a keypad lock, not screws."),
                Some(Item::Token) => Reaction::Say("The coin slot is jammed, and the machine has no power anyway."),
                Some(_) => Reaction::Say("No."),
                None => Reaction::Look(Look::Vending),
            }
        }
        Dumpster => {
            if state.flag(F_FOIL) {
                return Reaction::Say("You have been through it. Wet cardboard, a broken umbrella, nothing more.");
            }
            state.set_flag(F_FOIL);
            Item::Foil.give(state);
            Reaction::Got(
                Item::Foil,
                "Under a pizza box, a crumpled gum wrapper. Silver foil on the inside, still bright.",
            )
        }
        Notice => Reaction::Look(Look::Notice),
        Locker => {
            if state.flag(F_LOCKER) {
                return Reaction::Say("Empty now, except for some rags and a smell of oil.");
            }
            Reaction::Look(Look::Locker)
        }
        Cabinet => cabinet(state, item),
        ClockSign => {
            if state.flag(F_POWER) {
                if state.is_lit(0) {
                    Reaction::Say("The clock face glows. The hour hand is yours; it points at a time that is not now.")
                } else {
                    Reaction::Circuit
                }
            } else {
                Reaction::Say("A dead billboard with a clock face and no hands. Not a flicker of power.")
            }
        }
        Hatch => {
            if state.has_key() || (FIRST_AREA_ONLY && state.has_hand(0)) {
                Reaction::Out
            } else if item.is_some() {
                Reaction::Say("It does not fit. The keyholes are cut for the hands of a clock.")
            } else {
                Reaction::Say("An ornate iron hatch with three keyholes, each cut for the hand of a clock.")
            }
        }
        Passage => {
            if era == Era::Past {
                Reaction::Say("In the past the arch is open: a back lot lit by a street that is gone. You can see it, but the past will not let you walk through it. The way is in the present.")
            } else if !state.flag(F_POWER) {
                Reaction::Say("A dark arch in the wall. Dead static, and a faint smell of ozone.")
            } else if !state.flag(F_SHIELD) {
                Reaction::Look(Look::Shield)
            } else {
                state.set_flag(F_PASSAGE);
                Reaction::Passage
            }
        }
    }
}

fn cabinet(state: &mut State, item: Option<Item>) -> Reaction {
    if !state.flag(F_CABINET) {
        return match item {
            Some(Item::Screwdriver) => {
                state.set_flag(F_CABINET);
                Reaction::Say("Four slotted screws, and the cover swings open. Inside: a fuse holder with a gap, and a panel of switches, dead.")
            }
            Some(_) => Reaction::Say("It is screwed shut with four slotted screws. That is not going to turn them."),
            None => Reaction::Say("A utility cabinet, screwed shut. Four slotted screws."),
        };
    }
    if !state.flag(F_FUSE_IN) {
        return match item {
            Some(Item::Fuse) => {
                Item::Fuse.take(state);
                state.set_flag(F_FUSE_IN);
                Reaction::Say("The mended fuse clicks into the holder. A hum, and the switches on the panel begin to glow.")
            }
            Some(Item::BurntFuse) => Reaction::Say("It is blown. It will not carry anything as it is."),
            Some(_) => Reaction::Say("It does not go in the fuse holder."),
            None => Reaction::Say("The fuse holder is empty, and the panel is dead without it."),
        };
    }
    if !state.flag(F_POWER) {
        return Reaction::Look(Look::Panel);
    }
    Reaction::Say("The cabinet hums. The power runs up the wall in a thousand thin lines.")
}

/// A code was tried on a lock. Returns the reaction when it opens (None when it is wrong).
pub fn try_lock(state: &mut State, look: Look, digits: &[u8]) -> Option<Reaction> {
    let seed = state.seed();
    match look {
        Look::Locker if digits == locker_code(seed) => {
            state.set_flag(F_LOCKER | F_DRIVER);
            Item::Screwdriver.give(state);
            Some(Reaction::Got(
                Item::Screwdriver,
                "The padlock drops open. Rags, a rusted oil can and a screwdriver with a taped handle.",
            ))
        }
        Look::Vending if digits == neon_code(seed) => {
            state.set_flag(F_VENDING | F_VENDED);
            Item::BurntFuse.give(state);
            Item::Token.give(state);
            Some(Reaction::Got(
                Item::BurntFuse,
                "The service door pops open. Inside, a blown fuse and a brass token.",
            ))
        }
        _ => None,
    }
}

/// The shield's code: four numbers from 1 to 6 (they may repeat).
pub fn shield_code(seed: u32) -> [u8; 4] {
    let mut rng = Rng::new(seed, 53);
    [
        1 + rng.below(6) as u8,
        1 + rng.below(6) as u8,
        1 + rng.below(6) as u8,
        1 + rng.below(6) as u8,
    ]
}

/// How many tries before the shield wipes the board and lets you start over.
pub const SHIELD_TRIES: usize = 10;

/// Marks on a guess: (right number in the right place, right number in the wrong place).
pub fn shield_score(code: [u8; 4], guess: [u8; 4]) -> (u8, u8) {
    let exact = (0..4).filter(|i| code[*i] == guess[*i]).count() as u8;
    let mut common = 0u8;
    for d in 1..=6u8 {
        let a = code.iter().filter(|c| **c == d).count();
        let b = guess.iter().filter(|g| **g == d).count();
        common += a.min(b) as u8;
    }
    (exact, common - exact)
}

/// The shield is broken: it blinks and goes out.
pub fn shield_dropped(state: &mut State) -> &'static str {
    state.set_flag(F_SHIELD);
    "The numbers stutter, blink like a shield failing, and go out. The arch is open on the dark."
}

/// The panel is solved.
pub fn panel_solved(state: &mut State) -> &'static str {
    state.set_flag(F_POWER);
    "Every light shines. Power goes up the wall: the thin lines of the circuit board wake and run into the sky."
}

/// The clock sign's circuit is solved: it lights, and gives up the hour hand.
pub fn circuit_solved(state: &mut State) -> (Item, &'static str) {
    Item::Hand(0).give(state);
    (
        Item::Hand(0),
        "The clock sign flares. A hand drops from its face and rings on the street: the hour hand. Something shifts, as if the street had a past.",
    )
}

// ------------------------------------------------------------------------------------------------ hints
/// What the player should do next (for the hints).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    Locker,
    Cabinet,
    Vending,
    Foil,
    Mend,
    Fuse,
    Panel,
    Circuit,
    Shield,
    Hatch,
}

pub fn goal(state: &State) -> Goal {
    if !state.flag(F_LOCKER) {
        Goal::Locker
    } else if !state.flag(F_CABINET) {
        Goal::Cabinet
    } else if !state.flag(F_VENDING) {
        Goal::Vending
    } else if !state.flag(F_FUSE_IN) && !Item::Fuse.held(state) {
        if Item::Foil.held(state) {
            Goal::Mend
        } else {
            Goal::Foil
        }
    } else if !state.flag(F_FUSE_IN) {
        Goal::Fuse
    } else if !state.flag(F_POWER) {
        Goal::Panel
    } else if !state.is_lit(0) {
        Goal::Circuit
    } else if !state.flag(F_SHIELD) {
        Goal::Shield
    } else {
        Goal::Hatch
    }
}

/// A hint, from a nudge (0) to the answer (2).
pub fn hint(state: &State, level: usize) -> &'static str {
    let lines: [&str; 3] = match goal(state) {
        Goal::Locker => [
            "The padlocked locker on the west wall wants a code. Where would a maintenance worker leave it?",
            "The notice board on the east wall has a work order and a calendar.",
            "The work order says the code is the circled days, in date order. Turn the wheels to them.",
        ],
        Goal::Cabinet => [
            "The cabinet beside the locker is screwed shut. What was in the locker?",
            "The screws are slotted. The screwdriver fits.",
            "Select the screwdriver in the bar, then click the cabinet.",
        ],
        Goal::Vending => [
            "The vending machine's service door has a four-digit lock. The code is somewhere on this street.",
            "The neon sign on the same wall is not flickering at random.",
            "Open the sign and watch it: it shows one digit at a time, in order. Write them down and use them on the service door.",
        ],
        Goal::Foil => [
            "The cabinet's fuse holder needs a working fuse, and the one you have is blown. What could bridge a snapped wire?",
            "Something thin and metal. Somebody threw away a gum wrapper.",
            "Search the dumpster on the east wall for the wrapper.",
        ],
        Goal::Mend => [
            "You have a blown fuse and something that carries current.",
            "Put the two things together.",
            "Select the gum wrapper in the bar, then click the blown fuse.",
        ],
        Goal::Fuse => [
            "You have a working fuse now. The cabinet has an empty holder.",
            "The cabinet on the west wall.",
            "Select the mended fuse, then click the cabinet.",
        ],
        Goal::Panel => [
            "The panel in the cabinet is awake. Every light must shine.",
            "A press turns the light over and the ones next to it.",
            "Work on one corner at a time. Pressing the same switch twice undoes it, so the order does not matter.",
        ],
        Goal::Circuit => [
            "The power runs up the walls. The dead clock sign behind you needs it.",
            "Click the clock sign on the south wall.",
            "Turn the tiles so the current runs from the battery on the left to the lamp on the right.",
        ],
        Goal::Shield => [
            "The power woke the arch in the wall behind you. Numbers fall through it like rain.",
            "Click the arch. It is a code-breaking game: four numbers from 1 to 6.",
            "Gold mark: right number in the right place. Pale mark: right number in the wrong place. Work it out like Mastermind: change one thing at a time.",
        ],
        Goal::Hatch => [
            "You have the hour hand. Something in the street has keyholes for it.",
            "Behind you, in the road.",
            "Click the hatch on the south wall.",
        ],
    };
    lines[level.min(2)]
}

/// A hint costs this much time (seconds).
pub const HINT_COST: u32 = 30;

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> State {
        let mut s = State::default();
        s.enter();
        s.seed_with(12345);
        s
    }

    #[test]
    fn codes_follow_the_seed() {
        for seed in 1..200u32 {
            let l = locker_code(seed);
            assert!(l[0] < l[1] && l[1] < l[2] && l.iter().all(|d| (1..=9).contains(d)));
            let n = neon_code(seed);
            assert!(n.iter().all(|d| *d < 10) && n.iter().any(|d| *d != n[0]));
        }
        let different = (1..50u32).filter(|s| neon_code(*s) != neon_code(1)).count();
        assert!(different > 40);
    }

    #[test]
    fn the_neon_sign_shows_the_code_in_order_then_goes_dark() {
        let seed = 77;
        let code = neon_code(seed);
        for i in 0..4 {
            assert_eq!(neon_at(seed, i as f32 * 1.5 + 0.3), Some((i, code[i])));
            assert_eq!(neon_at(seed, i as f32 * 1.5 + 1.3), None);
        }
        assert_eq!(neon_at(seed, 7.0), None);
        assert_eq!(neon_at(seed, NEON_CYCLE + 0.2), Some((0, code[0])));
    }

    #[test]
    fn the_switches_can_always_be_solved_and_never_start_solved() {
        for seed in 1..300u32 {
            let p = Switches::make(seed);
            assert!(!p.solved());
            // pressing the same ones again undoes the scramble; find it by trying every press set of size 5
            // (pressing is its own undo, so some set of presses solves it: the ones used to make it)
            let mut rng = Rng::new(seed, 37);
            let mut pressed = vec![false; 16];
            let mut count = 0;
            let mut q = p.clone();
            while count < 5 {
                let at = rng.below(16);
                if !pressed[at] {
                    pressed[at] = true;
                    count += 1;
                    q.press(at);
                }
            }
            // (when make had to nudge an already solved panel the answer differs; that cannot happen at size 4)
            assert!(q.solved(), "seed {seed}");
        }
    }

    #[test]
    fn a_press_turns_the_light_and_its_neighbours() {
        let mut p = Switches { size: 4, on: vec![true; 16] };
        p.press(5);
        let off: Vec<usize> = (0..16).filter(|i| !p.on[*i]).collect();
        assert_eq!(off, vec![1, 4, 5, 6, 9]);
        p.press(0);
        assert_eq!(p.on.iter().filter(|b| !**b).count(), 4);
    }

    #[test]
    fn the_whole_chain_can_be_played() {
        let mut s = fresh();
        let seed = s.seed();
        // nothing opens without the code
        assert!(try_lock(&mut s, Look::Locker, &[9, 9, 9]).is_none());
        let code = locker_code(seed);
        assert!(matches!(try_lock(&mut s, Look::Locker, &code), Some(Reaction::Got(Item::Screwdriver, _))));
        assert_eq!(goal(&s), Goal::Cabinet);
        // the cabinet only yields to the screwdriver
        assert!(matches!(click(&mut s, SpotId::Cabinet, None, Era::Now), Reaction::Say(_)));
        assert!(!s.flag(F_CABINET));
        click(&mut s, SpotId::Cabinet, Some(Item::Screwdriver), Era::Now);
        assert!(s.flag(F_CABINET));
        // the vending machine
        assert_eq!(goal(&s), Goal::Vending);
        assert!(try_lock(&mut s, Look::Vending, &[0, 0, 0, 0]).is_none());
        let n = neon_code(seed);
        assert!(try_lock(&mut s, Look::Vending, &n).is_some());
        assert!(Item::BurntFuse.held(&s) && Item::Token.held(&s));
        assert_eq!(goal(&s), Goal::Foil);
        // the foil is in the dumpster, and only once
        assert!(matches!(click(&mut s, SpotId::Dumpster, None, Era::Now), Reaction::Got(Item::Foil, _)));
        assert!(matches!(click(&mut s, SpotId::Dumpster, None, Era::Now), Reaction::Say(_)));
        assert_eq!(goal(&s), Goal::Mend);
        // the blown fuse does not go in
        click(&mut s, SpotId::Cabinet, Some(Item::BurntFuse), Era::Now);
        assert!(!s.flag(F_FUSE_IN));
        // foil and fuse make a fuse, either way round, and use both up
        assert!(combine(&mut s, Item::BurntFuse, Item::Foil).is_some());
        assert!(!Item::Foil.held(&s) && !Item::BurntFuse.held(&s) && Item::Fuse.held(&s));
        assert!(combine(&mut s, Item::Foil, Item::Token).is_none());
        click(&mut s, SpotId::Cabinet, Some(Item::Fuse), Era::Now);
        assert!(s.flag(F_FUSE_IN) && !Item::Fuse.held(&s));
        assert_eq!(goal(&s), Goal::Panel);
        // the sign is dead until the panel is solved
        assert!(matches!(click(&mut s, SpotId::ClockSign, None, Era::Now), Reaction::Say(_)));
        assert!(matches!(click(&mut s, SpotId::Cabinet, None, Era::Now), Reaction::Look(Look::Panel)));
        panel_solved(&mut s);
        assert!(matches!(click(&mut s, SpotId::ClockSign, None, Era::Now), Reaction::Circuit));
        // the hatch stays shut without the hand
        assert!(matches!(click(&mut s, SpotId::Hatch, None, Era::Now), Reaction::Say(_)));
        let (item, _) = circuit_solved(&mut s);
        assert_eq!(item, Item::Hand(0));
        assert!(s.has_hand(0) && s.is_lit(0));
        assert_eq!(goal(&s), Goal::Shield);
        assert!(matches!(click(&mut s, SpotId::Hatch, None, Era::Now), Reaction::Out));
        // the arch: open to look at in the past, never to walk through; the shield holds it in the present
        assert!(matches!(click(&mut s, SpotId::Passage, None, Era::Past), Reaction::Say(_)));
        assert!(matches!(click(&mut s, SpotId::Passage, None, Era::Now), Reaction::Look(Look::Shield)));
        shield_dropped(&mut s);
        assert_eq!(goal(&s), Goal::Hatch);
        assert!(matches!(click(&mut s, SpotId::Passage, None, Era::Now), Reaction::Passage));
        assert!(matches!(click(&mut s, SpotId::Passage, None, Era::Past), Reaction::Say(_)));
    }

    #[test]
    fn the_shield_scores_like_mastermind() {
        assert_eq!(shield_score([1, 2, 3, 4], [1, 2, 3, 4]), (4, 0));
        assert_eq!(shield_score([1, 2, 3, 4], [4, 3, 2, 1]), (0, 4));
        assert_eq!(shield_score([1, 1, 2, 2], [1, 2, 1, 2]), (2, 2));
        assert_eq!(shield_score([1, 1, 1, 1], [1, 2, 3, 4]), (1, 0));
        assert_eq!(shield_score([1, 2, 3, 4], [1, 1, 1, 1]), (1, 0));
        assert_eq!(shield_score([6, 6, 5, 5], [5, 5, 6, 6]), (0, 4));
        for seed in 1..100u32 {
            let c = shield_code(seed);
            assert!(c.iter().all(|d| (1..=6).contains(d)));
            assert_eq!(shield_score(c, c), (4, 0));
        }
        // a patient player always gets there: change one number at a time, keep what scores
        let code = shield_code(31337);
        let mut guess = [1u8; 4];
        let mut tries = 0;
        for pos in 0..4 {
            for d in 1..=6u8 {
                let mut g = guess;
                g[pos] = d;
                tries += 1;
                if shield_score(code, g).0 > shield_score(code, guess).0 || d == code[pos] {
                    guess = g;
                    if code[pos] == d {
                        break;
                    }
                }
            }
        }
        assert_eq!(guess, code);
        assert!(tries >= 4);
    }

    #[test]
    fn nothing_skips_a_step() {
        let mut s = fresh();
        // cannot get the fuse, the cabinet or the power without the earlier steps
        click(&mut s, SpotId::Cabinet, Some(Item::Fuse), Era::Now);
        assert!(!s.flag(F_FUSE_IN) && !s.flag(F_CABINET));
        assert!(matches!(click(&mut s, SpotId::ClockSign, None, Era::Now), Reaction::Say(_)));
        assert!(try_lock(&mut s, Look::Panel, &[1]).is_none());
        assert!(combine(&mut s, Item::Foil, Item::BurntFuse).is_none());
    }

    #[test]
    fn every_spot_is_on_a_wall_and_the_goal_always_has_a_hint() {
        for sp in SPOTS {
            assert!(sp.view < VIEWS);
            assert!(sp.rect[0] < sp.rect[2] && sp.rect[1] < sp.rect[3]);
            assert!(sp.rect.iter().all(|v| (0.0..=1.0).contains(v)));
        }
        let s = fresh();
        for level in 0..5 {
            assert!(!hint(&s, level).is_empty());
        }
        assert_eq!(spot_at(3, 0.15, 0.5), Some(SpotId::Locker));
        assert_eq!(spot_at(0, 0.5, 0.5), Some(SpotId::Cat));
        assert_eq!(spot_at(0, 0.02, 0.02), None);
    }

    #[test]
    fn the_whole_state_survives_a_save() {
        let mut s = fresh();
        let code = locker_code(s.seed());
        try_lock(&mut s, Look::Locker, &code);
        s.add_spent(95);
        let back = State::decode(&s.encode());
        assert_eq!(back, s);
        assert!(Item::Screwdriver.held(&back) && back.flag(F_LOCKER));
        // an old save with no new fields still reads
        let old = State::decode("0:0:0:0:1:0");
        assert!(!old.flag(F_FOIL) && old.spent() == 0);
    }
}
