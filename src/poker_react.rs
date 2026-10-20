//! How the people at the table react to their hands: the Bear's bleeding eye, Father Time's
//! quiver and his beard falling off, the King's cackle, and the table talk that goes with them.
//!
//! Logic only (the table screen draws the faces). Nothing here changes how anyone plays: the
//! computer players never look at it, and the Bear's eye is worked out from his real cards, so it
//! is a tell you can learn but not a trick.
use crate::poker_cards::{self as cards, Card};
use crate::poker_game::{Elimination, BEAR, CAT, FATHER, MONARCH, PLAYERS, YOU};
use crate::poker_sha::sha256;
use crate::poker_story::Moment;
use crate::poker_table::Chips;
use std::sync::OnceLock;

/// The share of all starting hands, at each end, that can make the Bear's eye bleed: the very best
/// and the very worst, exactly the same amount of each.
pub const BLEED_SHARE: f32 = 0.06;
/// How often the eye bleeds when the hand is that good or that bad.
pub const BLEED_CHANCE: f32 = 0.75;
/// The part of a stack lost in one hand that makes Father Time quiver and his beard fall.
pub const BEARD_LOSS: f32 = 0.35;
/// The part of a stack won in one hand that is a big win.
pub const BIG_WIN: f32 = 0.5;
/// The part of a stack won in one hand that Father Time needs to stick his beard back on.
pub const BEARD_BACK_WIN: f32 = 0.3;
/// Hands after which the beard is back anyway.
pub const BEARD_HANDS: u64 = 10;

/// Which way the Bear's eye is telling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tell {
    /// One of the best starting hands.
    Great,
    /// One of the worst.
    Awful,
}

/// How a hand has left someone looking, for a hand or two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Look {
    Calm,
    /// A good pot: a small smile, a nod.
    Pleased,
    /// A huge pot: the King cackles and turns his head right round, the Bear roars.
    Triumphant,
    /// Lost a lot at once: Father Time quivers.
    Rattled,
    /// Lost a lot at once and is angry about it (everyone but Father Time).
    Furious,
}

// ---- the Bear's eye -------------------------------------------------------------------------------

/// The Chen score of two cards, in half points (so everything is a whole number): a rough value of a
/// starting hand. Only used to put the 1,326 starting hands in order.
fn chen2(a: Card, b: Card) -> i32 {
    let (ra, rb) = (cards::rank(a) as i32, cards::rank(b) as i32);
    let points = |r: i32| match r {
        12 => 20,
        11 => 16,
        10 => 14,
        9 => 12,
        r => r + 2,
    };
    let (hi, lo) = (ra.max(rb), ra.min(rb));
    let mut s = points(hi);
    if ra == rb {
        return (s * 2).max(10);
    }
    if cards::suit(a) == cards::suit(b) {
        s += 4;
    }
    let gap = hi - lo - 1;
    s -= match gap {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => 10,
    };
    if gap <= 1 && hi < 10 {
        s += 2;
    }
    s
}

/// Every starting hand's place from worst (0) to best (1,325), ties broken by the cards themselves
/// so that the same amount sits at each end.
fn order() -> &'static [u16] {
    static ORDER: OnceLock<Vec<u16>> = OnceLock::new();
    ORDER.get_or_init(|| {
        let mut combos: Vec<(i32, u8, u8)> = Vec::with_capacity(1326);
        for a in 0..52u8 {
            for b in (a + 1)..52u8 {
                combos.push((chen2(a, b), a, b));
            }
        }
        combos.sort();
        let mut table = vec![0u16; 52 * 52];
        for (place, &(_, a, b)) in combos.iter().enumerate() {
            table[a as usize * 52 + b as usize] = place as u16;
            table[b as usize * 52 + a as usize] = place as u16;
        }
        table
    })
}

/// How good two cards are as a starting hand: 0 (the worst) to 1,325 (the best).
pub fn hand_place(hole: [Card; 2]) -> usize {
    order()[hole[0] as usize * 52 + hole[1] as usize] as usize
}

/// How many starting hands are at each end.
pub fn tell_size() -> usize {
    (1326.0 * BLEED_SHARE).round() as usize
}

/// Whether two cards are in the group the Bear's eye can bleed for.
pub fn tell_for(hole: [Card; 2]) -> Option<Tell> {
    let p = hand_place(hole);
    if p >= 1326 - tell_size() {
        Some(Tell::Great)
    } else if p < tell_size() {
        Some(Tell::Awful)
    } else {
        None
    }
}

/// A number from 0 to 1 for a hand's eye, worked out from the same seeds as the shuffle, so once the
/// seed is shown after the hand it can be checked that the eye was not picked by hand.
pub fn tell_roll(server_seed: &[u8; 32], client_seed: &str, hand_no: u64) -> f32 {
    let mut data = b"CPUNITY-POKER-TELL|v1|".to_vec();
    data.extend_from_slice(server_seed);
    data.push(b'|');
    data.extend_from_slice(client_seed.as_bytes());
    data.push(b'|');
    data.extend_from_slice(hand_no.to_string().as_bytes());
    let h = sha256(&data);
    u32::from_be_bytes([h[0], h[1], h[2], h[3]]) as f32 / (u32::MAX as f32 + 1.0)
}

/// Whether the Bear's eye bleeds for these cards and this roll.
pub fn eye_bleeds(hole: [Card; 2], roll: f32) -> Option<Tell> {
    if roll < BLEED_CHANCE {
        tell_for(hole)
    } else {
        None
    }
}

// ---- the faces ---------------------------------------------------------------------------------

/// The Moment to talk about when someone is knocked out.
pub fn elimination_moment(e: Elimination) -> Moment {
    match e.seat {
        YOU => Moment::YouOut(e.killer),
        CAT => Moment::CatOut(e.killer),
        BEAR => Moment::BearOut(e.killer),
        FATHER => Moment::FatherOut(e.killer),
        _ => Moment::MonarchOut(e.killer),
    }
}

#[derive(Debug, Clone)]
pub struct Reactor {
    looks: [Look; PLAYERS],
    /// The last hand each look lasts through.
    until: [u64; PLAYERS],
    beard_off: bool,
    beard_off_at: u64,
    bleeding: Option<Tell>,
    moments: Vec<Moment>,
}

impl Default for Reactor {
    fn default() -> Self {
        Reactor::new()
    }
}

impl Reactor {
    pub fn new() -> Self {
        Reactor {
            looks: [Look::Calm; PLAYERS],
            until: [0; PLAYERS],
            beard_off: false,
            beard_off_at: 0,
            bleeding: None,
            moments: Vec::new(),
        }
    }

    /// A new hand is dealt: works out whether the Bear's eye bleeds. `bear_hole` is `None` when he is
    /// out; `roll` comes from `tell_roll`.
    pub fn deal(&mut self, bear_hole: Option<[Card; 2]>, roll: f32) {
        self.bleeding = bear_hole.and_then(|h| eye_bleeds(h, roll));
    }

    /// Whether the Bear's eye is bleeding in this hand, and the way it is telling (which only the
    /// program knows: show the blood, never the meaning).
    pub fn bleeding(&self) -> Option<Tell> {
        self.bleeding
    }

    /// Father Time has lost his beard.
    pub fn beard_off(&self) -> bool {
        self.beard_off
    }

    /// How a seat looks now.
    pub fn look(&self, seat: usize) -> Look {
        self.looks.get(seat).copied().unwrap_or(Look::Calm)
    }

    /// Something to talk about, in the order it happened; each is given once.
    pub fn take_moments(&mut self) -> Vec<Moment> {
        std::mem::take(&mut self.moments)
    }

    /// A hand is over. `before` and `after` are everyone's chips (the Monarch's seat is 0 until he
    /// sits), `holes` their cards and `showdown` whether cards were turned over.
    pub fn settle(&mut self, hand_no: u64, before: &[Chips], after: &[Chips], holes: &[Option<[Card; 2]>], showdown: bool) {
        self.bleeding = None;
        for i in 0..PLAYERS {
            if hand_no > self.until[i] {
                self.looks[i] = Look::Calm;
            }
        }
        for i in 0..PLAYERS.min(before.len()).min(after.len()) {
            let (b, a) = (before[i], after[i]);
            if b == 0 {
                continue;
            }
            let (lost, won) = (b.saturating_sub(a) as f32 / b as f32, a.saturating_sub(b) as f32 / b as f32);
            let mut look = None;
            if lost >= BEARD_LOSS {
                look = Some(if i == FATHER { Look::Rattled } else { Look::Furious });
            } else if won >= BIG_WIN {
                look = Some(Look::Triumphant);
            } else if won >= 0.2 {
                look = Some(Look::Pleased);
            }
            if let Some(l) = look {
                self.looks[i] = l;
                self.until[i] = hand_no + 1;
            }
            if won >= BIG_WIN {
                match i {
                    YOU => self.moments.push(Moment::YouWinBig),
                    BEAR => self.moments.push(Moment::BearWinsBig),
                    MONARCH => self.moments.push(Moment::MonarchWinsBig),
                    _ => {}
                }
            }
            if i == FATHER {
                if lost >= BEARD_LOSS && !self.beard_off {
                    self.beard_off = true;
                    self.beard_off_at = hand_no;
                    self.moments.push(Moment::FatherBeardFalls);
                } else if self.beard_off && hand_no > self.beard_off_at && won >= BEARD_BACK_WIN {
                    self.beard_off = false;
                }
            }
            if i == CAT {
                let weak = holes.get(CAT).copied().flatten().map_or(false, |h| hand_place(h) < 663);
                if weak && !showdown && won > 0.0 {
                    self.moments.push(Moment::CatBluffWorked);
                } else if weak && showdown && lost >= 0.15 {
                    self.moments.push(Moment::CatBluffCaught);
                }
            }
        }
        // (even a man who is out of the game grows it back in time)
        if self.beard_off && hand_no >= self.beard_off_at + BEARD_HANDS {
            self.beard_off = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poker_cards::parse_all;

    fn hole(s: &str) -> [Card; 2] {
        let v = parse_all(s);
        [v[0], v[1]]
    }

    #[test]
    fn the_same_amount_of_hands_sit_at_each_end() {
        let mut great = 0;
        let mut awful = 0;
        for a in 0..52u8 {
            for b in (a + 1)..52u8 {
                match tell_for([a, b]) {
                    Some(Tell::Great) => great += 1,
                    Some(Tell::Awful) => awful += 1,
                    None => {}
                }
            }
        }
        assert_eq!(great, awful, "50/50");
        assert_eq!(great, tell_size());
        assert!((70..90).contains(&great), "{great}");
    }

    #[test]
    fn the_best_hands_are_great_and_the_worst_are_awful_and_the_middle_says_nothing() {
        for h in ["As Ah", "Ks Kd", "Qh Qc", "Ac Kc"] {
            assert_eq!(tell_for(hole(h)), Some(Tell::Great), "{h}");
        }
        for h in ["7c 2d", "8s 2h", "8h 3c"] {
            assert_eq!(tell_for(hole(h)), Some(Tell::Awful), "{h}");
        }
        for h in ["Th 9h", "Js 4s", "6c 6d", "Ad 7c", "Kh 5d", "Td 6c"] {
            assert_eq!(tell_for(hole(h)), None, "{h}");
        }
        // the order is a real order: better cards are never placed below worse ones
        assert!(hand_place(hole("As Ah")) > hand_place(hole("Ks Kd")));
        assert!(hand_place(hole("Ks Kd")) > hand_place(hole("Th 9h")));
        assert!(hand_place(hole("Th 9h")) > hand_place(hole("7c 2d")));
        assert_eq!(hand_place(hole("As Kd")), hand_place([hole("As Kd")[1], hole("As Kd")[0]]));
    }

    #[test]
    fn the_eye_bleeds_the_same_whether_the_hand_is_great_or_awful_and_not_always() {
        let mut bleeds_great = 0;
        let mut bleeds_awful = 0;
        let mut n_great = 0;
        let mut n_awful = 0;
        let seed = [9u8; 32];
        for hand in 0..4000u64 {
            let roll = tell_roll(&seed, "t", hand);
            for (h, kind) in [(hole("As Ah"), Tell::Great), (hole("7c 2d"), Tell::Awful)] {
                let bled = eye_bleeds(h, roll) == Some(kind);
                match kind {
                    Tell::Great => {
                        n_great += 1;
                        bleeds_great += bled as u32;
                    }
                    Tell::Awful => {
                        n_awful += 1;
                        bleeds_awful += bled as u32;
                    }
                }
            }
        }
        let (g, a) = (bleeds_great as f32 / n_great as f32, bleeds_awful as f32 / n_awful as f32);
        assert!((g - BLEED_CHANCE).abs() < 0.03 && (a - BLEED_CHANCE).abs() < 0.03, "{g} {a}");
        // and an average hand never bleeds
        for hand in 0..500u64 {
            assert_eq!(eye_bleeds(hole("Th 9h"), tell_roll(&seed, "t", hand)), None);
        }
    }

    #[test]
    fn the_roll_is_fixed_by_the_seeds_and_changes_every_hand() {
        let s = [1u8; 32];
        assert_eq!(tell_roll(&s, "a", 5), tell_roll(&s, "a", 5));
        assert_ne!(tell_roll(&s, "a", 5), tell_roll(&s, "a", 6));
        assert_ne!(tell_roll(&s, "a", 5), tell_roll(&s, "b", 5));
        assert_ne!(tell_roll(&s, "a", 5), tell_roll(&[2u8; 32], "a", 5));
        for h in 0..200 {
            let r = tell_roll(&s, "a", h);
            assert!((0.0..1.0).contains(&r));
        }
    }

    #[test]
    fn the_bears_eye_bleeds_only_for_a_tell_and_stops_when_the_hand_is_over() {
        let mut r = Reactor::new();
        r.deal(Some(hole("As Ah")), 0.1);
        assert_eq!(r.bleeding(), Some(Tell::Great));
        assert!(r.take_moments().is_empty(), "nobody says a word about the eye");
        r.settle(1, &[1000; 5], &[1000; 5], &[None; 5], false);
        assert_eq!(r.bleeding(), None);
        r.deal(Some(hole("As Ah")), 0.9);
        assert_eq!(r.bleeding(), None, "the roll said no");
        r.deal(Some(hole("Th 9h")), 0.1);
        assert_eq!(r.bleeding(), None, "an ordinary hand");
        r.deal(None, 0.1);
        assert_eq!(r.bleeding(), None, "he is out");
    }

    #[test]
    fn father_times_beard_falls_when_he_loses_a_lot_at_once_and_comes_back() {
        let mut r = Reactor::new();
        // a small loss: nothing
        r.settle(1, &[1000, 1000, 1000, 1000, 0], &[1000, 1000, 1000, 900, 0], &[None; 5], true);
        assert!(!r.beard_off());
        assert_eq!(r.look(FATHER), Look::Calm);
        // a big one
        r.settle(2, &[1000, 1000, 1000, 900, 0], &[1000, 1000, 1500, 400, 0], &[None; 5], true);
        assert!(r.beard_off());
        assert_eq!(r.look(FATHER), Look::Rattled);
        assert_eq!(r.look(BEAR), Look::Triumphant);
        let m = r.take_moments();
        assert!(m.contains(&Moment::FatherBeardFalls) && m.contains(&Moment::BearWinsBig));
        // it falls only once while it is off
        r.settle(3, &[1000, 1000, 1500, 400, 0], &[1000, 1000, 1700, 200, 0], &[None; 5], true);
        assert!(!r.take_moments().contains(&Moment::FatherBeardFalls));
        // a good pot puts it back
        r.settle(4, &[1000, 1000, 1700, 200, 0], &[1000, 1000, 1500, 600, 0], &[None; 5], true);
        assert!(!r.beard_off());
        // losing everything takes it too
        r.settle(5, &[1000, 1000, 1500, 600, 0], &[1000, 1000, 2100, 0, 0], &[None; 5], true);
        assert!(r.beard_off());
        // and it grows back by itself in time
        for h in 6..6 + BEARD_HANDS + 1 {
            r.settle(h, &[1000, 1000, 2100, 0, 0], &[1000, 1000, 2100, 0, 0], &[None; 5], false);
        }
        assert!(!r.beard_off());
    }

    #[test]
    fn looks_follow_the_chips_and_fade() {
        let mut r = Reactor::new();
        r.settle(1, &[1000, 1000, 1000, 1000, 1000], &[1300, 600, 1100, 1000, 1000], &[None; 5], true);
        assert_eq!(r.look(YOU), Look::Pleased);
        assert_eq!(r.look(CAT), Look::Furious, "40% lost, and angry about it");
        assert_eq!(r.look(BEAR), Look::Calm, "a small gain is nothing");
        // a huge pot: the one who won it cackles
        r.settle(2, &[1300, 600, 1100, 1000, 1000], &[1300, 1800, 1100, 400, 1000], &[None; 5], true);
        assert_eq!(r.look(CAT), Look::Triumphant);
        assert_eq!(r.look(FATHER), Look::Rattled);
        // two hands later they have calmed down
        r.settle(4, &[1300, 1800, 1100, 400, 1000], &[1300, 1800, 1100, 400, 1000], &[None; 5], false);
        assert_eq!(r.look(CAT), Look::Calm);
        assert_eq!(r.look(FATHER), Look::Calm);
    }

    #[test]
    fn the_king_is_talked_about_when_a_bluff_works_or_is_caught() {
        let mut r = Reactor::new();
        let holes_weak = [None, Some(hole("7c 2d")), None, None, None];
        // everyone folded to him
        r.settle(1, &[1000; 5], &[900, 1150, 1000, 950, 0], &holes_weak, false);
        assert!(r.take_moments().contains(&Moment::CatBluffWorked));
        // he was called and lost with trash
        r.settle(2, &[900, 1150, 1000, 950, 0], &[900, 850, 1300, 950, 0], &holes_weak, true);
        assert!(r.take_moments().contains(&Moment::CatBluffCaught));
        // a strong hand is not a bluff
        let strong = [None, Some(hole("As Ah")), None, None, None];
        r.settle(3, &[900, 850, 1300, 950, 0], &[900, 1000, 1300, 800, 0], &strong, false);
        assert!(!r.take_moments().contains(&Moment::CatBluffWorked));
    }

    #[test]
    fn a_knockout_is_talked_about_by_who_went_out() {
        let e = |seat, killer| Elimination { seat, hand: 1, killer };
        assert_eq!(elimination_moment(e(YOU, MONARCH)), Moment::YouOut(MONARCH));
        assert_eq!(elimination_moment(e(CAT, YOU)), Moment::CatOut(YOU));
        assert_eq!(elimination_moment(e(BEAR, YOU)), Moment::BearOut(YOU));
        assert_eq!(elimination_moment(e(FATHER, BEAR)), Moment::FatherOut(BEAR));
        assert_eq!(elimination_moment(e(MONARCH, YOU)), Moment::MonarchOut(YOU));
    }
}
