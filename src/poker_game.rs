//! The Underworld poker game as a tournament: who sits where, how the blinds rise, who knocked whom
//! out, when the Shadow Monarch arrives, how it ends, and what the tokens do.
//!
//! This is logic only (the table screen asks it things). The seats go round the table from you:
//! you, the dead King on your left, the Demon Bear across, Father Time on your right and, once he
//! sits down, the Shadow Monarch.
use crate::poker_table::{Chips, Event, Table};

pub const YOU: usize = 0;
pub const CAT: usize = 1;
pub const BEAR: usize = 2;
pub const FATHER: usize = 3;
pub const MONARCH: usize = 4;
/// Everyone who ever sits at the table, the Monarch included.
pub const PLAYERS: usize = 5;

/// Chips each of you starts with.
pub const START_CHIPS: Chips = 1000;
/// Real tokens it costs to sit down.
pub const BUY_IN: u64 = 20;
/// Hands played at each level of the blinds.
pub const HANDS_PER_LEVEL: u64 = 5;
/// The blinds, small and big, at each level (the last one stays).
pub const LEVELS: [(Chips, Chips); 12] = [
    (10, 20),
    (20, 40),
    (30, 60),
    (50, 100),
    (75, 150),
    (100, 200),
    (150, 300),
    (200, 400),
    (300, 600),
    (500, 1000),
    (800, 1600),
    (1500, 3000),
];

/// Tokens paid out for each finishing place (first to fifth) for a buy-in of `BUY_IN`. The winner
/// takes most of it, second place gets the stake back and nobody else is paid.
pub const PAYOUTS: [u64; PLAYERS] = [80, 20, 0, 0, 0];

/// The hand number (counted from 1) before which the Shadow Monarch sits down: after this many hands
/// have been played, or as soon as someone is out.
pub const MONARCH_ARRIVES_AFTER: u64 = 8;

/// Someone who ran out of chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elimination {
    pub seat: usize,
    /// The hand it happened in.
    pub hand: u64,
    /// Who took the chips that put them out.
    pub killer: usize,
}

/// How it ended, for the story: who won, and how the others went out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ending {
    /// You beat them all. `first_out` is the first to go and `last` the last one you beat.
    Victory { first_out: usize, last: usize },
    /// The dead King won, which nobody saw coming (you went out to `you_out_by`).
    CatWins { you_out_by: usize },
    /// Father Time won, patiently (you went out to `you_out_by`).
    FatherWins { you_out_by: usize },
    /// The Bear won with the Monarch at the table: you were knocked out by `by` and the
    /// Monarch went out before, or after, you.
    BearWins { you_out_by: usize, monarch_out_first: bool },
    /// The Monarch won and took your soul, and the Bear went out to him and you went out to him too.
    MonarchSweep,
    /// The Monarch won: you knocked the Bear out, then lost to him one on one.
    MonarchLate,
    /// The Monarch won: you were the first to go (to `by`), and he then beat the Bear.
    MonarchAfterYou { you_out_by: usize },
    /// The Monarch won: the Bear went out first (to `bear_out_by`) and you went out to the Monarch.
    MonarchAfterBear { bear_out_by: usize },
}

impl Ending {
    /// The Monarch won, so your soul is his.
    pub fn soul_taken(self) -> bool {
        matches!(
            self,
            Ending::MonarchSweep | Ending::MonarchLate | Ending::MonarchAfterYou { .. } | Ending::MonarchAfterBear { .. }
        )
    }

    pub fn you_won(self) -> bool {
        matches!(self, Ending::Victory { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Tournament {
    pub table: Table,
    /// Hands started so far.
    pub hands: u64,
    pub eliminations: Vec<Elimination>,
    /// The Monarch has taken a seat.
    pub monarch_in: bool,
    /// Chips the Monarch sat down with.
    pub monarch_stack: Chips,
}

impl Tournament {
    /// You, the King, the Demon Bear and Father Time, `START_CHIPS` each.
    pub fn new() -> Self {
        let mut table = Table::new(&[START_CHIPS; 4], LEVELS[0].0, LEVELS[0].1);
        table.button = 3; // (the first hand moves it to you)
        Tournament { table, hands: 0, eliminations: Vec::new(), monarch_in: false, monarch_stack: 0 }
    }

    /// The level of the blinds the next hand is played at (0 is the first).
    pub fn level(&self) -> usize {
        ((self.hands / HANDS_PER_LEVEL) as usize).min(LEVELS.len() - 1)
    }

    /// Hands left at this level, counting the next one.
    pub fn hands_left_in_level(&self) -> u64 {
        HANDS_PER_LEVEL - self.hands % HANDS_PER_LEVEL
    }

    /// Whether the Monarch should sit down before the next hand: not yet in, and either enough hands
    /// have been played or one of you is out.
    pub fn monarch_due(&self) -> bool {
        !self.monarch_in && (self.hands >= MONARCH_ARRIVES_AFTER || !self.eliminations.is_empty())
    }

    /// The Monarch buys in for as many chips as the biggest stack at the table (at least ten big
    /// blinds): he comes to win it all.
    pub fn seat_monarch(&mut self) -> usize {
        let biggest = self.table.alive().iter().map(|&i| self.table.seats[i].stack).max().unwrap_or(START_CHIPS);
        let (_, bb) = LEVELS[self.level()];
        self.monarch_stack = biggest.max(bb * 10);
        self.monarch_in = true;
        // (the seats are numbered YOU, BEAR, MONARCH: a seat for the Monarch is made even if someone
        // is out, so the numbers never move)
        while self.table.seats.len() <= MONARCH {
            self.table.add_seat(0);
        }
        self.table.seats[MONARCH].stack = self.monarch_stack;
        MONARCH
    }

    /// Starts the next hand with this deck, setting the blinds for its level. False when the game is over.
    pub fn start_hand(&mut self, deck: crate::poker_fair::Deck) -> bool {
        if self.over() {
            return false;
        }
        let (sb, bb) = LEVELS[self.level()];
        self.table.small_blind = sb;
        self.table.big_blind = bb;
        let started = self.table.start_hand(deck);
        if started {
            self.hands += 1;
        }
        started
    }

    /// Call when the hand is over: notes who went out and who knocked them out. Returns the new ones.
    pub fn finish_hand(&mut self) -> Vec<Elimination> {
        let mut new = Vec::new();
        let Some(result) = self.table.result.clone() else {
            return new;
        };
        for e in &self.table.history {
            if let Event::Busted { seat } = e {
                if self.eliminations.iter().any(|x| x.seat == *seat) {
                    continue;
                }
                // the one who took the biggest pot the busted player was in
                let killer = result
                    .pots
                    .iter()
                    .filter(|p| p.eligible.contains(seat))
                    .max_by_key(|p| p.amount)
                    .and_then(|p| p.winners.iter().copied().find(|w| w != seat))
                    .or_else(|| (0..self.table.seats.len()).find(|&i| i != *seat && self.table.seats[i].stack > 0))
                    .unwrap_or(*seat);
                let elim = Elimination { seat: *seat, hand: self.hands, killer };
                self.eliminations.push(elim);
                new.push(elim);
            }
        }
        new
    }

    /// Seats that still have chips, counting the Monarch as present only once he sits.
    pub fn alive(&self) -> Vec<usize> {
        self.table.alive()
    }

    /// The tournament is decided: one player left with chips. (The Monarch always sits down first:
    /// when one of you is out before he has come, he comes at once and the game goes on.)
    pub fn over(&self) -> bool {
        self.monarch_in && self.alive().len() <= 1 && self.hands > 0
    }

    /// You are out.
    pub fn you_out(&self) -> bool {
        self.eliminations.iter().any(|e| e.seat == YOU)
    }

    pub fn winner(&self) -> Option<usize> {
        if self.over() {
            self.alive().first().copied()
        } else {
            None
        }
    }

    /// How it ended; `None` while it is still going.
    pub fn ending(&self) -> Option<Ending> {
        let winner = self.winner()?;
        let out = |seat: usize| self.eliminations.iter().position(|e| e.seat == seat);
        let by = |seat: usize| self.eliminations.iter().find(|e| e.seat == seat).map(|e| e.killer);
        Some(match winner {
            YOU => {
                let first = self.eliminations.first().map(|e| e.seat).unwrap_or(BEAR);
                let last = self.eliminations.last().map(|e| e.seat).unwrap_or(BEAR);
                Ending::Victory { first_out: first, last }
            }
            CAT => Ending::CatWins { you_out_by: by(YOU).unwrap_or(CAT) },
            FATHER => Ending::FatherWins { you_out_by: by(YOU).unwrap_or(FATHER) },
            BEAR => Ending::BearWins {
                you_out_by: by(YOU).unwrap_or(BEAR),
                monarch_out_first: out(MONARCH).map_or(false, |m| out(YOU).map_or(false, |y| m < y)),
            },
            _ => {
                let bear_by = by(BEAR);
                let you_by = by(YOU);
                match (out(YOU), out(BEAR)) {
                    (Some(y), Some(b)) if b < y => {
                        // the Bear went out first
                        if bear_by == Some(MONARCH) && you_by == Some(MONARCH) {
                            Ending::MonarchSweep
                        } else if bear_by == Some(YOU) {
                            Ending::MonarchLate
                        } else {
                            Ending::MonarchAfterBear { bear_out_by: bear_by.unwrap_or(MONARCH) }
                        }
                    }
                    (Some(_), _) => Ending::MonarchAfterYou { you_out_by: you_by.unwrap_or(MONARCH) },
                    _ => Ending::MonarchSweep,
                }
            }
        })
    }

    /// Your finishing place (1 to 5) once the game is over or you are out; `None` while you are in.
    pub fn your_place(&self) -> Option<usize> {
        if self.winner() == Some(YOU) {
            return Some(1);
        }
        // (five places: the first out is fifth)
        self.eliminations.iter().position(|e| e.seat == YOU).map(|i| PLAYERS - i)
    }

    /// Tokens paid for your finishing place.
    pub fn payout(&self) -> Option<u64> {
        Some(PAYOUTS[self.your_place()? - 1])
    }
}

/// The tokens left after the buy-in and the payout (negative: lost).
pub fn net_tokens(payout: u64) -> i64 {
    payout as i64 - BUY_IN as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poker_fair::deck_for;
    use crate::poker_table::Action;

    #[test]
    fn the_blinds_rise_every_few_hands_and_stop_at_the_top() {
        let mut t = Tournament::new();
        assert_eq!(t.level(), 0);
        t.hands = HANDS_PER_LEVEL - 1;
        assert_eq!(t.level(), 0);
        assert_eq!(t.hands_left_in_level(), 1);
        t.hands = HANDS_PER_LEVEL;
        assert_eq!(t.level(), 1);
        t.hands = 10_000;
        assert_eq!(t.level(), LEVELS.len() - 1);
        for w in LEVELS.windows(2) {
            assert!(w[1].0 > w[0].0 && w[1].1 > w[0].1);
        }
        for (sb, bb) in LEVELS {
            assert_eq!(bb, sb * 2, "{sb}/{bb}");
        }
    }

    #[test]
    fn four_sit_down_to_start_and_the_seats_go_round_from_you() {
        let t = Tournament::new();
        assert_eq!(t.table.seats.len(), 4);
        assert_eq!(t.alive(), vec![YOU, CAT, BEAR, FATHER]);
        assert!(t.table.seats.iter().all(|s| s.stack == START_CHIPS));
        assert_eq!((YOU, CAT, BEAR, FATHER, MONARCH), (0, 1, 2, 3, 4));
        assert_eq!(PAYOUTS.len(), PLAYERS);
    }

    #[test]
    fn the_monarch_comes_after_a_while_or_when_someone_is_out_and_never_twice() {
        let mut t = Tournament::new();
        assert!(!t.monarch_due());
        t.hands = MONARCH_ARRIVES_AFTER;
        assert!(t.monarch_due());
        let seat = t.seat_monarch();
        assert_eq!(seat, MONARCH);
        assert!(!t.monarch_due(), "already sitting");
        assert_eq!(t.table.seats.len(), PLAYERS);
        assert!(t.table.seats[MONARCH].stack >= LEVELS[t.level()].1 * 10);
        // someone out: due at once
        let mut t = Tournament::new();
        t.eliminations.push(Elimination { seat: BEAR, hand: 3, killer: YOU });
        assert!(t.monarch_due());
    }

    /// Plays hands until `loser` is out, with `winner` and `loser` pushing everything in every time and
    /// the others folding.
    fn knock_out(t: &mut Tournament, winner: usize, loser: usize, deck_seed: u64) {
        for k in 0..400u64 {
            let deck = deck_for(&[deck_seed as u8; 32], "k", deck_seed * 1000 + k);
            if !t.start_hand(deck) {
                return;
            }
            while t.table.running {
                let s = t.table.to_act.unwrap();
                let l = t.table.legal(s);
                let a = if s == winner || s == loser {
                    if l.can_raise {
                        Action::Raise(l.max_raise_to)
                    } else {
                        Action::Call
                    }
                } else if l.can_check {
                    Action::Check
                } else {
                    Action::Fold
                };
                t.table.act(s, a).unwrap();
            }
            t.finish_hand();
            if t.eliminations.iter().any(|e| e.seat == loser) {
                return;
            }
        }
    }

    #[test]
    fn a_knockout_names_the_one_who_took_the_chips_and_the_game_goes_on() {
        let mut t = Tournament::new();
        knock_out(&mut t, BEAR, YOU, 1);
        assert_eq!(t.eliminations.len(), 1);
        let e = t.eliminations[0];
        assert!(e.seat == YOU || e.seat == BEAR);
        assert_ne!(e.killer, e.seat);
        assert_eq!(t.alive().len(), 3);
        assert!(!t.over() && t.monarch_due());
        let biggest = t.alive().iter().map(|&i| t.table.seats[i].stack).max().unwrap();
        t.seat_monarch();
        assert_eq!(t.alive().len(), 4);
        assert_eq!(t.table.seats[MONARCH].stack, biggest.max(LEVELS[t.level()].1 * 10));
    }

    fn finished(monarch_in: bool, elims: &[(usize, usize)], winner: usize) -> Tournament {
        let mut t = Tournament::new();
        t.hands = 20;
        t.monarch_in = monarch_in;
        while t.table.seats.len() < PLAYERS {
            t.table.add_seat(0);
        }
        for s in t.table.seats.iter_mut() {
            s.stack = 0;
        }
        t.table.seats[winner].stack = 5000;
        t.eliminations = elims.iter().enumerate().map(|(i, &(seat, killer))| Elimination { seat, hand: i as u64 + 1, killer }).collect();
        t
    }

    #[test]
    fn who_wins_decides_the_ending() {
        let mk = |elims: &[(usize, usize)], winner: usize| finished(true, elims, winner).ending().unwrap();
        // you win
        assert_eq!(
            mk(&[(CAT, YOU), (FATHER, YOU), (MONARCH, YOU), (BEAR, YOU)], YOU),
            Ending::Victory { first_out: CAT, last: BEAR }
        );
        assert_eq!(
            mk(&[(BEAR, YOU), (FATHER, YOU), (CAT, YOU), (MONARCH, YOU)], YOU),
            Ending::Victory { first_out: BEAR, last: MONARCH }
        );
        // the Bear wins
        assert_eq!(mk(&[(YOU, BEAR), (MONARCH, BEAR)], BEAR), Ending::BearWins { you_out_by: BEAR, monarch_out_first: false });
        assert_eq!(mk(&[(MONARCH, BEAR), (YOU, BEAR)], BEAR), Ending::BearWins { you_out_by: BEAR, monarch_out_first: true });
        // a surprise winner
        assert_eq!(mk(&[(YOU, BEAR), (BEAR, CAT)], CAT), Ending::CatWins { you_out_by: BEAR });
        assert_eq!(mk(&[(YOU, FATHER)], FATHER), Ending::FatherWins { you_out_by: FATHER });
        // the Monarch wins
        assert_eq!(mk(&[(BEAR, MONARCH), (YOU, MONARCH)], MONARCH), Ending::MonarchSweep);
        assert_eq!(mk(&[(BEAR, YOU), (YOU, MONARCH)], MONARCH), Ending::MonarchLate);
        assert_eq!(mk(&[(YOU, BEAR), (BEAR, MONARCH)], MONARCH), Ending::MonarchAfterYou { you_out_by: BEAR });
        assert_eq!(mk(&[(YOU, MONARCH), (BEAR, MONARCH)], MONARCH), Ending::MonarchAfterYou { you_out_by: MONARCH });
        assert_eq!(mk(&[(BEAR, CAT), (YOU, MONARCH)], MONARCH), Ending::MonarchAfterBear { bear_out_by: CAT });
        // only the Monarch's wins cost a soul
        assert!(Ending::MonarchSweep.soul_taken());
        for e in [
            Ending::BearWins { you_out_by: BEAR, monarch_out_first: false },
            Ending::CatWins { you_out_by: BEAR },
            Ending::FatherWins { you_out_by: BEAR },
            Ending::Victory { first_out: BEAR, last: MONARCH },
        ] {
            assert!(!e.soul_taken(), "{e:?}");
        }
        assert!(Ending::Victory { first_out: BEAR, last: BEAR }.you_won());
        assert!(!Ending::CatWins { you_out_by: BEAR }.you_won());
    }

    #[test]
    fn the_game_is_not_over_until_the_monarch_has_sat() {
        let t = finished(false, &[(CAT, YOU), (FATHER, YOU), (BEAR, YOU)], YOU);
        assert!(!t.over() && t.ending().is_none());
    }

    #[test]
    fn tokens_are_paid_by_place_and_the_winner_takes_most() {
        let place = |elims: &[(usize, usize)], winner: usize| {
            let t = finished(true, elims, winner);
            (t.your_place().unwrap(), t.payout().unwrap())
        };
        assert_eq!(place(&[(CAT, YOU), (FATHER, YOU), (MONARCH, YOU), (BEAR, YOU)], YOU), (1, 80));
        // second: the last one out before the winner
        assert_eq!(place(&[(CAT, BEAR), (FATHER, BEAR), (MONARCH, BEAR), (YOU, BEAR)], BEAR), (2, 20));
        // third, fourth and fifth are not paid
        assert_eq!(place(&[(CAT, BEAR), (FATHER, BEAR), (YOU, BEAR), (MONARCH, BEAR)], BEAR), (3, 0));
        assert_eq!(place(&[(CAT, BEAR), (YOU, BEAR), (FATHER, BEAR), (MONARCH, BEAR)], BEAR), (4, 0));
        assert_eq!(place(&[(YOU, BEAR), (CAT, BEAR), (FATHER, BEAR), (MONARCH, BEAR)], BEAR), (5, 0));
        assert_eq!(net_tokens(80), 60);
        assert_eq!(net_tokens(20), 0);
        assert_eq!(net_tokens(0), -(BUY_IN as i64));
        // the whole pot is paid out, no more (five buy-ins is not what is paid: the Monarch pays with his soul)
        assert!(PAYOUTS.iter().sum::<u64>() <= BUY_IN * 4 + BUY_IN);
        // while you are still in, there is no place
        let mut t = finished(true, &[(CAT, BEAR)], BEAR);
        t.table.seats[YOU].stack = 100;
        assert_eq!(t.your_place(), None);
    }

    // ---- whole games played by the computer players ----

    use crate::poker_ai::{bear, cat_king, father_time, monarch, Brain, Personality};

    fn stand_in() -> Personality {
        Personality { name: "you", aggression: 0.5, bluff: 0.25, trap: 0.15, caution: 0.03, adapt: 0.5, sizing: 1.0, sims: 40, wild: 0.0, loose: 0.0, lure: 0.0 }
    }

    /// Plays a whole tournament with a computer player in every seat; returns it when it is over.
    fn play_all(sims: usize, seed: u64) -> Tournament {
        let mk = |p: Personality, k: u64| Brain::new(Personality { sims, ..p }, seed * 17 + k);
        let mut brains = vec![mk(stand_in(), 1), mk(cat_king(), 2), mk(bear(), 3), mk(father_time(), 4), mk(monarch(), 5)];
        let mut t = Tournament::new();
        let mut total_chips = START_CHIPS * 4;
        let mut n = 0;
        while !t.over() {
            if t.monarch_due() {
                t.seat_monarch();
                total_chips += t.monarch_stack;
            }
            n += 1;
            assert!(n < 900, "a game that never ends");
            let deck = deck_for(&[(seed % 200) as u8 + 1; 32], "play", n);
            assert!(t.start_hand(deck));
            while t.table.running {
                let s = t.table.to_act.unwrap();
                for (i, b) in brains.iter_mut().enumerate() {
                    if let Some(v) = t.table.view(i) {
                        b.watch(&v);
                    }
                }
                let v = t.table.view(s).unwrap();
                let a = brains[s].decide(&v);
                t.table.act(s, a).expect("the computer plays by the rules");
            }
            t.finish_hand();
            let chips: u64 = t.table.seats.iter().map(|s| s.stack).sum();
            assert_eq!(chips, total_chips, "chips are never made or lost (hand {n})");
        }
        t
    }

    #[test]
    fn a_whole_game_between_the_computers_ends_with_one_ending_and_everyone_placed() {
        let t = play_all(30, 3);
        let e = t.ending().expect("it ended");
        assert_eq!(t.alive().len(), 1);
        // everyone else went out, once
        let mut out: Vec<usize> = t.eliminations.iter().map(|e| e.seat).collect();
        out.sort();
        out.dedup();
        assert_eq!(out.len(), PLAYERS - 1);
        assert!(!out.contains(&t.winner().unwrap()));
        // you are out (or won), so you have a place and the ending matches who won
        assert!(t.your_place().is_some());
        match t.winner().unwrap() {
            YOU => assert!(e.you_won()),
            MONARCH => assert!(e.soul_taken()),
            _ => assert!(!e.soul_taken() && !e.you_won()),
        }
        assert!(t.hands > MONARCH_ARRIVES_AFTER.min(5));
    }

    #[test]
    #[ignore = "slow: run with --release --ignored --nocapture"]
    fn how_long_a_game_takes() {
        let mut v = Vec::new();
        for g in 0..40u64 {
            v.push(play_all(60, g + 100).hands);
        }
        v.sort();
        eprintln!("hands: min {} median {} max {}", v[0], v[v.len() / 2], v.last().unwrap());
    }
}
