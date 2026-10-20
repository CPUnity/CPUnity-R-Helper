//! The computer players of the Hold'em table: the Demon Bear and the Shadow Monarch.
//!
//! A `Brain` is only ever given a `poker_table::View`: its own two cards, the board, the chips and
//! the public record of the hand. It has no way to reach the deck or anyone else's cards (the tests
//! check that two worlds that look the same to it get the same decision).
//!
//! What it does:
//! * **Equity**: it plays the hand out a few hundred times at random (its own cards and the board
//!   fixed, the rest dealt from the cards it cannot see) and counts how often it wins. Opponents who
//!   have shown strength are dealt better hands than random ones in those games (the best of a few
//!   random hands), so a raise from a tight player is respected.
//! * **Pot odds**: it calls when its equity beats the price, with a margin.
//! * **Value, bluffs and traps**: it bets and raises for value, semi-bluffs with draws, bluffs at
//!   players who fold too much, and sometimes slow-plays a monster. Which of these it does a lot is its
//!   personality.
//! * **Opponent model**: it counts how often each opponent enters pots, raises, bets and folds to bets,
//!   and uses that: it bluffs more at folders, calls lighter against maniacs and gives passive players
//!   credit when they suddenly bet.
//! * **Tournament sense**: it knows how many big blinds it has left, shoves or folds when short, and is
//!   more careful when calling for its life while a third player could still bust first.
use crate::poker_cards::{self as cards, Card};
use crate::poker_table::{Action, Chips, Event, Street, View};

/// What a computer player is like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Personality {
    pub name: &'static str,
    /// How often it bets and raises rather than checks and calls (0..1).
    pub aggression: f32,
    /// How often it bluffs when the moment is right (0..1).
    pub bluff: f32,
    /// How often it slow-plays a very strong hand (0..1).
    pub trap: f32,
    /// How much it needs on top of the pot odds before it calls (a part of a hand's equity).
    pub caution: f32,
    /// How much it learns from watching its opponents (0..1).
    pub adapt: f32,
    /// How big its bets are, as a multiple of the usual (1.0 is the usual).
    pub sizing: f32,
    /// Hands it plays out per decision.
    pub sims: usize,
    /// How often it does something wild for no good reason (0..1): a big bluff, or a call it should not make.
    pub wild: f32,
    /// How readily it plays a hand (0..1): it opens and calls before the flop with more than the best
    /// hands, and calls a little under the pot odds when the stacks are deep (it is indulging).
    pub loose: f32,
    /// How often it baits with a strong hand (0..1): a small bet to be called, or a flat call to let the
    /// other player keep betting into it.
    pub lure: f32,
}

/// The Demon Bear: big, loud and relentless. Raises a lot, overbets and bluffs often.
pub fn bear() -> Personality {
    Personality {
        name: "Demon Bear",
        aggression: 0.85,
        bluff: 0.5,
        trap: 0.08,
        caution: 0.01,
        adapt: 0.6,
        sizing: 1.2,
        sims: 700,
        wild: 0.0,
        loose: 0.6,
        lure: 0.15,
    }
}

/// The dead Monarch King Cat: loud and rude, and as dangerous as the Bear: he plays as many hands, reads
/// the table as well and bluffs only now and then, with a wild streak that looks stupid and is not
/// always. He likes to let you think you are winning.
pub fn cat_king() -> Personality {
    Personality {
        name: "Dead King",
        aggression: 0.84,
        bluff: 0.3,
        trap: 0.1,
        caution: 0.015,
        adapt: 0.6,
        sizing: 1.15,
        sims: 700,
        wild: 0.03,
        loose: 0.6,
        lure: 0.3,
    }
}

/// Father Time: patient and stubborn. He calls too much, rarely raises and likes to slow-play a
/// good hand, and he has seen every trick before.
pub fn father_time() -> Personality {
    Personality {
        name: "Father Time",
        aggression: 0.5,
        bluff: 0.18,
        trap: 0.4,
        caution: 0.0,
        adapt: 0.3,
        sizing: 0.9,
        sims: 400,
        wild: 0.0,
        loose: 0.9,
        lure: 0.4,
    }
}

/// The Shadow Monarch: patient and cold. Plays fewer hands, traps, watches everything and
/// punishes what it has learned.
pub fn monarch() -> Personality {
    Personality {
        name: "Shadow Monarch",
        aggression: 0.8,
        bluff: 0.4,
        trap: 0.25,
        caution: 0.02,
        adapt: 1.0,
        sizing: 0.95,
        sims: 900,
        wild: 0.0,
        loose: 0.45,
        lure: 0.3,
    }
}

/// What it has learned about one opponent.
#[derive(Debug, Clone, Default)]
struct Opp {
    hands: u32,
    vpip: u32,
    pfr: u32,
    bets: u32,
    calls: u32,
    faced: u32,
    folds_to_bet: u32,
}

impl Opp {
    fn rate(count: u32, of: u32, prior: f32) -> f32 {
        (count as f32 + prior * 4.0) / (of as f32 + 4.0)
    }
    /// How often it puts chips in before the flop.
    fn vpip(&self) -> f32 {
        Self::rate(self.vpip, self.hands, 0.5)
    }
    /// How often it raises before the flop.
    fn pfr(&self) -> f32 {
        Self::rate(self.pfr, self.hands, 0.25)
    }
    /// Bets and raises for each call: above 1 is aggressive.
    fn aggression(&self) -> f32 {
        (self.bets as f32 + 1.0) / (self.calls as f32 + 1.5)
    }
    /// How often it folds when it faces a bet.
    fn fold_to_bet(&self) -> f32 {
        Self::rate(self.folds_to_bet, self.faced, 0.4)
    }
}

/// What one opponent has done in the hand being played.
#[derive(Debug, Clone, Copy, Default)]
struct HandActs {
    raises: u32,
    calls: u32,
    /// The biggest raise, as a part of the pot before it.
    biggest: f32,
}

#[derive(Debug, Clone)]
pub struct Brain {
    pub personality: Personality,
    rng: u64,
    opp: Vec<Opp>,
    acts: Vec<HandActs>,
    hand_no: u64,
    seen: usize,
    street: Street,
    raised_this_street: bool,
    entered: Vec<bool>,
    pot_before: Chips,
}

impl Brain {
    pub fn new(personality: Personality, seed: u64) -> Self {
        Brain {
            personality,
            rng: seed | 1,
            opp: Vec::new(),
            acts: Vec::new(),
            hand_no: 0,
            seen: 0,
            street: Street::Preflop,
            raised_this_street: true,
            entered: Vec::new(),
            pot_before: 0,
        }
    }

    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    /// A number from 0 to 1.
    fn unit(&mut self) -> f32 {
        (self.rand() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Takes in what has happened at the table since it last looked. It can be called as often as
    /// you like (and should be after every action, so nothing is missed while it is folded).
    pub fn watch(&mut self, v: &View) {
        let n = v.stacks.len();
        if self.opp.len() < n {
            self.opp.resize(n, Opp::default());
        }
        if v.hand_no != self.hand_no {
            self.hand_no = v.hand_no;
            self.seen = 0;
        }
        let mut pot: Chips = 0;
        for e in v.history.iter().skip(self.seen) {
            match e {
                Event::HandStart { .. } => {
                    self.street = Street::Preflop;
                    self.raised_this_street = true; // (the big blind is a bet to answer)
                    self.acts = vec![HandActs::default(); n];
                    self.entered = vec![false; n];
                    for (i, o) in self.opp.iter_mut().enumerate() {
                        if v.in_hand.get(i).copied().unwrap_or(false) {
                            o.hands += 1;
                        }
                    }
                    pot = 0;
                }
                Event::Blind { amount, .. } => pot += amount,
                Event::Fold { seat } => {
                    if self.raised_this_street {
                        self.opp[*seat].faced += 1;
                        self.opp[*seat].folds_to_bet += 1;
                    }
                }
                Event::Check { .. } => {}
                Event::Call { seat, amount, .. } => {
                    let o = &mut self.opp[*seat];
                    o.faced += 1;
                    o.calls += 1;
                    if self.street == Street::Preflop && !self.entered[*seat] {
                        o.vpip += 1;
                        self.entered[*seat] = true;
                    }
                    self.acts[*seat].calls += 1;
                    pot += amount;
                }
                Event::Raise { seat, added, bet, .. } => {
                    let o = &mut self.opp[*seat];
                    o.bets += 1;
                    if !*bet {
                        o.faced += 1;
                    }
                    if self.street == Street::Preflop {
                        if !self.entered[*seat] {
                            o.vpip += 1;
                            self.entered[*seat] = true;
                        }
                        o.pfr += 1;
                    }
                    let a = &mut self.acts[*seat];
                    a.raises += 1;
                    a.biggest = a.biggest.max(*added as f32 / pot.max(1) as f32);
                    self.raised_this_street = true;
                    pot += added;
                }
                Event::Board { street, .. } => {
                    self.street = *street;
                    self.raised_this_street = false;
                }
                Event::Uncalled { amount, .. } => pot = pot.saturating_sub(*amount),
                Event::Show { .. } | Event::Won { .. } | Event::Busted { .. } => {}
            }
        }
        self.pot_before = pot;
        self.seen = v.history.len();
    }

    /// How strong the hands of `seat` are likely to be, 0 (anything) to 0.9, from what it did this hand
    /// and what it is like.
    fn tightness(&self, seat: usize, v: &View) -> f32 {
        let a = self.acts.get(seat).copied().unwrap_or_default();
        let mut t = 0.12 * a.calls.min(3) as f32;
        if a.raises >= 1 {
            t += 0.3;
        }
        if a.raises >= 2 {
            t += 0.2;
        }
        t += 0.2 * a.biggest.min(1.0);
        if v.all_in.get(seat).copied().unwrap_or(false) && a.raises + a.calls > 0 {
            t += 0.1;
        }
        // players who raise a lot have wider ranges, players who rarely do have narrower ones
        if let Some(o) = self.opp.get(seat) {
            let loose = (o.pfr() - 0.25) * 1.2 + (o.aggression() - 1.0).clamp(-0.5, 1.5) * 0.12;
            // everyone notices a player who raises every hand, however little they study the table
            let learn = self.personality.adapt.max(0.75);
            t -= loose * learn * (a.raises.min(1) as f32);
            if o.pfr() > 0.55 && a.raises > 0 {
                t *= 0.3;
            }
            if a.raises == 0 && a.calls == 0 && self.street != Street::Preflop {
                // a quiet player who bets for the first time late has it
            }
        }
        t.clamp(0.0, 0.9)
    }

    /// The share of the pot it expects to win: 1.0 for always, 0.0 for never, with ties counted as halves.
    fn equity(&mut self, v: &View, opps: &[usize]) -> f32 {
        let hole = v.hole;
        let mut used = [false; 52];
        used[hole[0] as usize] = true;
        used[hole[1] as usize] = true;
        for &c in &v.board {
            used[c as usize] = true;
        }
        let rest: Vec<Card> = (0..52u8).filter(|c| !used[*c as usize]).collect();
        let tight: Vec<f32> = opps.iter().map(|&s| self.tightness(s, v)).collect();
        let need_board = 5 - v.board.len();
        let sims = self.personality.sims.max(50);
        let mut won = 0.0f32;
        let mut deck = rest;
        let m = deck.len();
        let mut board = v.board.clone();
        for _ in 0..sims {
            // a partial shuffle: the cards from `next` on have not been dealt
            let mut next = 0usize;
            board.truncate(v.board.len());
            for _ in 0..need_board {
                let j = next + (self.rand() % (m - next) as u64) as usize;
                deck.swap(next, j);
                board.push(deck[next]);
                next += 1;
            }
            let mut mine = board.clone();
            mine.extend_from_slice(&hole);
            let my = cards::score(&mine);
            let mut best_other = 0u32;
            let mut ties = 1u32;
            for &t in &tight {
                // the best of a few random hands: the stronger the story, the more it picks from
                let extra = (t * 4.0).floor() as usize;
                let frac = t * 4.0 - extra as f32;
                let tries = (1 + extra + usize::from(self.unit() < frac)).min(4);
                let mut best = 0u32;
                for _ in 0..tries {
                    if m - next < 2 {
                        break;
                    }
                    let mut theirs = board.clone();
                    for _ in 0..2 {
                        let j = next + (self.rand() % (m - next) as u64) as usize;
                        deck.swap(next, j);
                        theirs.push(deck[next]);
                        next += 1;
                    }
                    best = best.max(cards::score(&theirs));
                }
                if best > best_other {
                    best_other = best;
                    ties = 1;
                } else if best == best_other && best > 0 {
                    ties += 1;
                }
            }
            if my > best_other {
                won += 1.0;
            } else if my == best_other {
                won += 1.0 / ties as f32;
            }
        }
        won / sims as f32
    }

    /// Decides what to do when it is this seat's turn.
    pub fn decide(&mut self, v: &View) -> Action {
        self.watch(v);
        let me = v.seat;
        let p = self.personality;
        let l = v.legal;
        let stack = v.stacks[me];
        let to_call = l.call;
        let bb = v.big_blind.max(1);
        let pot = v.pot;
        let opps: Vec<usize> =
            (0..v.stacks.len()).filter(|&i| i != me && v.in_hand[i] && !v.folded[i]).collect();
        let n_opp = opps.len().max(1);
        let can_act_behind = opps.iter().any(|&i| !v.all_in[i]);
        let m_ratio = stack as f32 / (v.small_blind + v.big_blind).max(1) as f32;
        let mut eq = self.equity(v, &opps);
        // Facing a big bet or a shove, what counts is the player who made it (and anyone who has called). People who have not
        // acted yet are not assumed to join in: counting each of them as a hand to beat made a shove almost impossible to call.
        if to_call as f32 >= 0.4 * stack as f32 {
            let live: Vec<usize> = opps.iter().copied().filter(|&i| v.bets[i] >= (v.current_bet / 2).max(1) || v.all_in[i]).collect();
            if !live.is_empty() && live.len() < opps.len() {
                eq = self.equity(v, &live);
            }
        }
        let fair = 1.0 / (n_opp as f32 + 1.0);

        // what the people left are like
        let (mut fold_rate, mut aggro) = (0.0f32, 0.0f32);
        let mut counted = 0.0f32;
        for &o in &opps {
            if let Some(op) = self.opp.get(o) {
                fold_rate += op.fold_to_bet();
                aggro += op.aggression();
                counted += 1.0;
            }
        }
        if counted > 0.0 {
            fold_rate /= counted;
            aggro /= counted;
        } else {
            fold_rate = 0.4;
            aggro = 1.0;
        }
        // (how much weight the model gets)
        let w = p.adapt;
        let fold_rate = 0.4 + (fold_rate - 0.4) * w;
        let aggro = 1.0 + (aggro - 1.0) * w;

        // the chips it would need to put in to raise to `to`, as a raise-to total
        let raise_to = |frac: f32| -> Chips {
            let size = (frac * p.sizing * (pot + to_call) as f32) as Chips;
            let target = v.bets[me] + to_call + size.max(bb);
            target.clamp(l.min_raise_to, l.max_raise_to)
        };
        let shove = l.max_raise_to;
        let committed_share = |to: Chips| -> bool { to as f32 >= 0.6 * (v.bets[me] + stack) as f32 };
        let mut go = |this: &mut Self, to: Chips| -> Action {
            if !l.can_raise {
                return if to_call == 0 { Action::Check } else { Action::Call };
            }
            let _ = this;
            if committed_share(to) {
                Action::Raise(shove)
            } else {
                Action::Raise(to)
            }
        };

        let postflop = v.street != Street::Preflop;
        let spr = if pot > 0 { stack as f32 / pot as f32 } else { 10.0 };

        // --- a wild moment: a big bluff, or a call nobody should make ---
        if p.wild > 0.0 && self.unit() < p.wild && m_ratio >= 7.0 {
            let price = to_call as f32 / (pot + to_call).max(1) as f32;
            if l.can_raise && to_call as f32 <= 0.3 * stack as f32 && self.unit() < 0.6 {
                let to = raise_to(1.2 + 0.6 * self.unit());
                return go(self, to);
            }
            if to_call == 0 {
                return Action::Check;
            }
            if price < 0.45 {
                return Action::Call;
            }
        }

        // --- short stacks: shove or fold ---
        if m_ratio < 7.0 && !postflop && l.can_raise {
            // the more people left to beat, the more it needs; the shorter it is, the less
            let need = (fair * 1.15 + 0.03 * (m_ratio - 3.0).max(0.0) * 0.1).min(0.7);
            let push = eq > need - (7.0 - m_ratio) * 0.012;
            let facing_big = to_call as f32 > 0.25 * stack as f32 && to_call > bb;
            let call_need = to_call as f32 / (pot + to_call).max(1) as f32 + p.caution;
            if facing_big {
                return if eq > call_need.max(fair * 1.1) { go(self, shove) } else { Action::Fold };
            }
            return if push { go(self, shove) } else if to_call == 0 { Action::Check } else { Action::Fold };
        }

        // --- nothing to call ---
        if to_call == 0 {
            // a very strong hand: bet it, or sometimes wait
            if eq > 0.82 && n_opp <= 2 {
                let trap = p.trap * if postflop { 1.0 } else { 0.0 } * if v.street == Street::River { 0.2 } else { 1.0 };
                if self.unit() < trap && can_act_behind {
                    return Action::Check;
                }
                // bait: a small bet that is easy to call
                if postflop && self.unit() < p.lure {
                    let to = raise_to(0.22 + 0.15 * self.unit());
                    return go(self, to);
                }
                let to = raise_to(0.7 + 0.3 * self.unit());
                return go(self, to);
            }
            let value_line = fair * (1.28 - 0.25 * p.aggression) + 0.02 - 0.03 * p.loose;
            if eq > value_line + 0.06 {
                if self.unit() < 0.55 + 0.45 * p.aggression {
                    let to = raise_to(0.45 + 0.35 * eq);
                    return go(self, to);
                }
                return Action::Check;
            }
            // a bluff: only at players who fold, on a pot that is worth taking, and not into a crowd
            let bluff_chance = p.bluff * fold_rate * 1.6 / (n_opp as f32).powf(1.5);
            let chance_bonus = if postflop && v.street != Street::River && eq > 0.28 { 1.3 } else { 1.0 };
            if !postflop {
                // before the flop: raise the blinds when the table is folding
                if v.street == Street::Preflop && eq > fair * 0.95 && self.unit() < p.aggression * 0.6 {
                    let to = raise_to(0.9);
                    return go(self, to);
                }
            } else if self.unit() < bluff_chance * chance_bonus && spr > 0.5 && aggro < 3.0 {
                let to = raise_to(0.5 + 0.25 * self.unit());
                return go(self, to);
            }
            return Action::Check;
        }

        // --- facing a bet ---
        let price = to_call as f32 / (pot + to_call) as f32;
        let mut margin = p.caution;
        // against a maniac a call is lighter, against a rock a bet means something
        margin -= ((aggro - 1.5).clamp(-1.0, 2.0)) * 0.012 * w;
        // 3-handed, calling for the whole stack while another player could bust first
        let alive = v.stacks.iter().filter(|s| **s > 0).count();
        if alive >= 3 && to_call as f32 >= 0.5 * stack as f32 {
            margin += 0.05;
        }
        // an indulgent player calls a little under the odds
        margin = (margin - 0.04 * p.loose).max(-0.04);
        // deep stacks pay for a call later: before the river a call is a little cheaper than it looks
        let price = if v.street != Street::River && stack as f32 >= 6.0 * to_call as f32 { price * (1.0 - 0.14 * p.loose) } else { price };

        // before the flop: it opens the pot and calls a raise of a few blinds with many more hands than the best ones
        if v.street == Street::Preflop && to_call <= bb * 3 && m_ratio >= 12.0 {
            if v.current_bet <= bb && l.can_raise {
                let raise_bar = fair * (1.5 - 0.25 * p.loose - 0.2 * p.aggression);
                if eq > raise_bar && self.unit() < 0.35 + 0.6 * p.aggression {
                    let to = raise_to(0.9 + 0.3 * self.unit());
                    return go(self, to);
                }
            }
            if eq > fair * (1.1 - 0.16 * p.loose) {
                return Action::Call;
            }
        }

        // a raise for value
        let raise_line = (0.64 + 0.04 * (n_opp as f32 - 1.0)).min(0.82) - 0.06 * p.aggression;
        if eq > raise_line && l.can_raise {
            // bait: a strong hand that only calls, so that the bettor keeps betting into it
            if postflop && v.street != Street::River && can_act_behind && self.unit() < p.lure * 0.7 {
                return Action::Call;
            }
            if self.unit() < 0.35 + 0.6 * p.aggression {
                let to = raise_to(0.75 + 0.4 * self.unit());
                return go(self, to);
            }
        }
        // a semi-bluff or a bluff-raise: draws and nerve, at someone who folds
        if postflop && v.street != Street::River && l.can_raise && eq > 0.28 && eq < price + 0.12 {
            let chance = p.bluff * 0.75 * (0.3 + fold_rate) / n_opp as f32;
            if self.unit() < chance && to_call as f32 <= 0.35 * stack as f32 && spr > 1.0 {
                let to = raise_to(1.0);
                return go(self, to);
            }
        }
        if eq > price + margin {
            return Action::Call;
        }
        // a tiny bet into a pot that pays 6 to 1 or better is called with anything that is alive
        if price < 0.14 && eq > 0.18 {
            return Action::Call;
        }
        // occasionally it will not be pushed off a hand it could bluff-catch with
        let catch = p.aggression * 0.04 * (1.0 + w * (1.0 - fold_rate)) * if eq > price * 0.8 { 1.0 } else { 0.0 };
        if self.unit() < catch {
            return Action::Call;
        }
        Action::Fold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poker_cards::parse;
    use crate::poker_fair::deck_for;
    use crate::poker_table::Table;

    fn cs(s: &str) -> Vec<Card> {
        s.as_bytes().chunks(2).map(|c| parse(std::str::from_utf8(c).unwrap()).unwrap()).collect()
    }

    fn view_with(hole: &str, board: &str, opponents: usize) -> View {
        let h = cs(hole);
        let n = opponents + 1;
        View {
            seat: 0,
            hole: [h[0], h[1]],
            board: cs(board),
            street: match board.len() / 2 {
                0 => Street::Preflop,
                3 => Street::Flop,
                4 => Street::Turn,
                _ => Street::River,
            },
            hand_no: 1,
            button: 0,
            small_blind: 10,
            big_blind: 20,
            stacks: vec![1000; n],
            bets: vec![0; n],
            committed: vec![0; n],
            folded: vec![false; n],
            all_in: vec![false; n],
            in_hand: vec![true; n],
            pot: 100,
            current_bet: 0,
            legal: crate::poker_table::Legal { can_check: true, call: 0, can_raise: true, min_raise_to: 20, max_raise_to: 1000 },
            history: vec![Event::HandStart { hand_no: 1, button: 0, small_blind: 1, big_blind: 2 }],
        }
    }

    /// Before the flop, with everyone to act after the blinds (they are 10 and 20): the seat has `hole` and
    /// has 20 to call.
    fn facing_the_blinds(hole: &str) -> View {
        let mut v = view_with(hole, "", 4);
        v.pot = 30;
        v.current_bet = 20;
        v.bets[1] = 10;
        v.bets[2] = 20;
        v.legal = crate::poker_table::Legal { can_check: false, call: 20, can_raise: true, min_raise_to: 40, max_raise_to: 1000 };
        v
    }

    /// How often a personality plays (calls or raises) a hand before the flop, over a spread of hands.
    fn plays_before_the_flop(p: Personality) -> (usize, usize, usize) {
        let ranks = "AKQJT98765432";
        let (mut played, mut raised, mut total) = (0, 0, 0);
        for (i, a) in ranks.chars().enumerate() {
            for (j, b) in ranks.chars().enumerate() {
                if j < i {
                    continue;
                }
                // a pair, a suited hand and an offsuit one for each pair of ranks
                let hands: Vec<String> = if i == j { vec![format!("{a}s{b}h")] } else { vec![format!("{a}s{b}s"), format!("{a}s{b}h")] };
                for h in hands {
                    let mut brain = Brain::new(Personality { sims: 120, ..p }, (i * 13 + j) as u64);
                    match brain.decide(&facing_the_blinds(&h)) {
                        Action::Call => played += 1,
                        Action::Raise(_) => {
                            played += 1;
                            raised += 1;
                        }
                        _ => {}
                    }
                    total += 1;
                }
            }
        }
        (played, raised, total)
    }

    #[test]
    fn they_play_a_good_many_hands_before_the_flop() {
        for p in [bear(), cat_king(), father_time(), monarch()] {
            let (played, raised, total) = plays_before_the_flop(p);
            assert!(played * 100 >= total * 35, "{} plays only {played} of {total}", p.name);
            assert!(played * 100 <= total * 90, "{} plays {played} of {total}: that is every hand", p.name);
            assert!(raised * 100 >= total * 6, "{} raises only {raised} of {total}", p.name);
        }
    }

    #[test]
    fn the_king_bluffs_less_and_is_as_strong_as_the_bear() {
        let (k, b) = (cat_king(), bear());
        assert!(k.bluff < 0.4 && k.bluff < b.bluff, "the King bluffs {}", k.bluff);
        assert!(k.sims >= b.sims, "he thinks as hard as the Bear: {} v {}", k.sims, b.sims);
        assert!(k.adapt >= 0.5, "and reads the table");
        assert!(k.aggression >= 0.8 && k.loose >= b.loose - 0.1);
    }

    #[test]
    fn a_strong_hand_is_sometimes_a_bait_and_sometimes_a_real_bet() {
        let (mut small, mut big) = (0, 0);
        for seed in 0..120 {
            let mut b = Brain::new(Personality { sims: 120, ..cat_king() }, seed);
            let v = view_with("AsAh", "2c7d9h", 1);
            if let Action::Raise(to) = b.decide(&v) {
                if to < 50 {
                    small += 1;
                } else {
                    big += 1;
                }
            }
        }
        assert!(small >= 8, "baits: {small}");
        assert!(big >= 40, "and bets properly: {big}");
    }

    #[test]
    fn equity_is_close_to_the_known_numbers() {
        let mut b = Brain::new(Personality { sims: 6000, ..bear() }, 7);
        // aces against one random hand win about 85%
        let v = view_with("AsAh", "", 1);
        let e = b.equity(&v, &[1]);
        assert!((0.82..0.88).contains(&e), "AA {e}");
        // seven-two offsuit about 35%
        let v = view_with("7c2d", "", 1);
        let e = b.equity(&v, &[1]);
        assert!((0.31..0.39).contains(&e), "72o {e}");
        // the nuts on the river against anyone is a lock; a dead hand is not
        let v = view_with("AsKs", "QsJsTs2c3d", 1);
        assert!(b.equity(&v, &[1]) > 0.99);
        let v = view_with("2h3h", "AsKsQsJs9d", 1);
        assert!(b.equity(&v, &[1]) < 0.35);
        // three more opponents make aces worth less
        let v = view_with("AsAh", "", 3);
        let e = b.equity(&v, &[1, 2, 3]);
        assert!((0.58..0.68).contains(&e), "AA vs 3 {e}");
    }

    #[test]
    fn a_hand_that_shows_strength_is_respected() {
        let mut b = Brain::new(Personality { sims: 4000, ..monarch() }, 9);
        let mut v = view_with("QsQh", "2c7d9h", 1);
        let loose = b.equity(&v, &[1]);
        // the opponent raised twice, big
        b.acts = vec![HandActs::default(); 2];
        b.acts[1] = HandActs { raises: 2, calls: 0, biggest: 1.0 };
        v.history.clear();
        let tight = b.equity(&v, &[1]);
        assert!(tight < loose - 0.08, "{loose} -> {tight}");
    }

    #[test]
    fn it_checks_trash_and_bets_the_nuts_and_folds_to_a_big_bet_with_nothing() {
        let mut bet = 0;
        let mut checks = 0;
        for seed in 0..40 {
            let mut b = Brain::new(Personality { sims: 300, ..bear() }, seed);
            // the nuts on the river: it must never fold or check-fold; it bets (or traps rarely)
            let v = view_with("AsKs", "QsJsTs2c3d", 1);
            match b.decide(&v) {
                Action::Raise(_) => bet += 1,
                Action::Check => checks += 1,
                a => panic!("{a:?}"),
            }
        }
        assert!(bet >= 36, "bets the nuts: {bet} bets, {checks} checks");
        // air facing a pot-sized bet on the river: fold (almost always)
        let mut folds = 0;
        for seed in 0..40 {
            let mut b = Brain::new(Personality { sims: 300, ..monarch() }, seed);
            let mut v = view_with("2h3h", "AsKsQsJs9d", 1);
            v.pot = 400;
            v.bets[1] = 200;
            v.current_bet = 200;
            v.legal = crate::poker_table::Legal { can_check: false, call: 200, can_raise: true, min_raise_to: 400, max_raise_to: 1000 };
            if b.decide(&v) == Action::Fold {
                folds += 1;
            }
        }
        assert!(folds >= 38, "folds air to a big bet: {folds}/40");
    }

    #[test]
    fn it_calls_when_the_price_is_right_and_raises_with_a_monster() {
        let mut calls = 0;
        let mut raises = 0;
        for seed in 0..40 {
            let mut b = Brain::new(Personality { sims: 300, ..monarch() }, seed);
            // top set on a dry flop facing a half-pot bet
            let mut v = view_with("QsQh", "Qd7c2s", 1);
            v.pot = 300;
            v.bets[1] = 100;
            v.current_bet = 100;
            v.legal = crate::poker_table::Legal { can_check: false, call: 100, can_raise: true, min_raise_to: 200, max_raise_to: 1000 };
            match b.decide(&v) {
                Action::Call => calls += 1,
                Action::Raise(_) => raises += 1,
                Action::Fold => panic!("folded top set"),
                _ => {}
            }
        }
        assert!(calls + raises == 40 && raises >= 10, "{calls} calls, {raises} raises");
    }

    #[test]
    fn short_stacks_shove_good_hands_and_fold_bad_ones() {
        let mut shove = 0;
        let mut fold = 0;
        for seed in 0..30 {
            let mut b = Brain::new(Personality { sims: 400, ..bear() }, seed);
            let mut v = view_with("AsAh", "", 1);
            v.stacks[0] = 100;
            v.pot = 30;
            v.bets = vec![0, 20];
            v.current_bet = 20;
            v.legal = crate::poker_table::Legal { can_check: false, call: 20, can_raise: true, min_raise_to: 40, max_raise_to: 100 };
            if let Action::Raise(100) = b.decide(&v) {
                shove += 1;
            }
            let mut b = Brain::new(Personality { sims: 400, ..bear() }, seed);
            let mut v = view_with("7c2d", "", 1);
            v.stacks[0] = 100;
            v.pot = 30;
            v.bets = vec![0, 20];
            v.current_bet = 20;
            v.legal = crate::poker_table::Legal { can_check: false, call: 20, can_raise: true, min_raise_to: 40, max_raise_to: 100 };
            if b.decide(&v) == Action::Fold {
                fold += 1;
            }
        }
        assert!(shove >= 28, "aces shove: {shove}/30");
        assert!(fold >= 24, "72o folds: {fold}/30");
    }

    #[test]
    fn two_worlds_that_look_the_same_get_the_same_decision() {
        // the hidden cards (the other players' and the deck) differ; what the player can see does not
        let mk = |other: [&str; 2], tail: &str| {
            let mut t = Table::new(&[1000, 1000, 1000], 10, 20);
            let mut deck: Vec<Card> = Vec::new();
            for h in ["AsKd", other[0], other[1]] {
                deck.extend(cs(h));
            }
            let board = cs(&format!("Td7c2h{tail}"));
            let mut spare: Vec<Card> = (0..52u8).filter(|c| !deck.contains(c) && !board.contains(c)).collect();
            deck.push(spare.remove(0));
            deck.extend(&board[0..3]);
            deck.push(spare.remove(0));
            deck.push(board[3]);
            deck.push(spare.remove(0));
            deck.push(board[4]);
            deck.extend(spare);
            t.start_hand(deck.try_into().unwrap());
            t
        };
        // seat 1 is dealt first (after the button): it holds AsKd in both worlds
        let mut a = mk(["QcJc", "8h8d"], "3s4s");
        let mut b = mk(["9h9d", "6c6d"], "5h5d");
        for t in [&mut a, &mut b] {
            t.act(0, Action::Call).unwrap();
            t.act(1, Action::Call).unwrap();
            t.act(2, Action::Check).unwrap();
        }
        for seed in 0..5 {
            let mut ba = Brain::new(bear(), seed);
            let mut bb = Brain::new(bear(), seed);
            let (va, vb) = (a.view(1).unwrap(), b.view(1).unwrap());
            assert_eq!(va, vb);
            assert_eq!(ba.decide(&va), bb.decide(&vb), "seed {seed}");
        }
    }

    #[test]
    fn it_learns_who_folds_and_who_raises() {
        let mut t = Table::new(&[10_000, 10_000], 10, 20);
        let mut b = Brain::new(monarch(), 3);
        // seat 1 folds to every raise from seat 0, hand after hand
        for h in 0..30u64 {
            t.start_hand(deck_for(&[1; 32], "a", h));
            while t.running {
                let s = t.to_act.unwrap();
                let v = t.view(s).unwrap();
                b.watch(&v);
                let l = v.legal;
                let act = if s == 0 && l.can_raise {
                    Action::Raise(l.min_raise_to)
                } else if s == 1 {
                    Action::Fold
                } else if l.can_check {
                    Action::Check
                } else {
                    Action::Call
                };
                t.act(s, act).unwrap();
            }
            if let Some(v) = t.view(0) {
                b.watch(&v);
            }
        }
        assert!(b.opp[1].fold_to_bet() > 0.75, "{}", b.opp[1].fold_to_bet());
        assert!(b.opp[0].pfr() > 0.4 && b.opp[0].aggression() > 1.0, "{} {}", b.opp[0].pfr(), b.opp[0].aggression());
        assert!(b.opp[1].pfr() < 0.1);
    }

    // ---- whole tournaments against simple opponents ----

    enum Ctl {
        Ai(Brain),
        Station,
        Rock,
        Maniac(u64),
    }

    fn rand_of(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    fn ctl_action(c: &mut Ctl, v: &View) -> Action {
        let l = v.legal;
        let check_or_call = if l.can_check { Action::Check } else { Action::Call };
        match c {
            Ctl::Ai(b) => b.decide(v),
            Ctl::Station => check_or_call,
            Ctl::Rock => {
                let (a, b) = (cards::rank(v.hole[0]), cards::rank(v.hole[1]));
                let strong = a == b && a >= 7 || a.min(b) >= 10 || (a.max(b) == 12 && a.min(b) >= 9);
                if v.street == Street::Preflop {
                    if strong {
                        if l.can_raise && v.current_bet <= v.big_blind {
                            Action::Raise((v.big_blind * 3).clamp(l.min_raise_to, l.max_raise_to))
                        } else {
                            check_or_call
                        }
                    } else if l.can_check {
                        Action::Check
                    } else {
                        Action::Fold
                    }
                } else {
                    let mut all = v.board.clone();
                    all.extend_from_slice(&v.hole);
                    if cards::category(cards::score(&all)) >= cards::Category::Pair && l.can_raise && l.can_check {
                        Action::Raise((v.pot / 2).clamp(l.min_raise_to, l.max_raise_to))
                    } else if cards::category(cards::score(&all)) >= cards::Category::TwoPair {
                        check_or_call
                    } else if l.can_check {
                        Action::Check
                    } else {
                        Action::Fold
                    }
                }
            }
            Ctl::Maniac(st) => {
                let r = rand_of(st) % 100;
                if r < 55 && l.can_raise {
                    let span = l.max_raise_to - l.min_raise_to;
                    Action::Raise(l.min_raise_to + if span == 0 { 0 } else { rand_of(st) % (span.min(v.pot * 2).max(1) + 1) }.min(span))
                } else if r < 90 || l.can_check {
                    check_or_call
                } else {
                    Action::Fold
                }
            }
        }
    }

    /// Plays one tournament (the seats are `poker_game`'s: you, the King, the Bear, Father Time and,
    /// when he sits down, the Monarch) and returns the winner and the hands it took.
    fn tournament(ctls: &mut [Ctl], seed: u64) -> Option<(usize, u64)> {
        use crate::poker_game::Tournament;
        let mut t = Tournament::new();
        let mut n = 0u64;
        while !t.over() {
            if t.monarch_due() {
                t.seat_monarch();
            }
            n += 1;
            if n > 700 {
                return None;
            }
            let deck = deck_for(&[(seed % 251) as u8; 32], "sim", seed * 1000 + n);
            if !t.start_hand(deck) {
                return None;
            }
            while t.table.running {
                let s = t.table.to_act.unwrap();
                for (i, c) in ctls.iter_mut().enumerate() {
                    if let (Ctl::Ai(b), Some(v)) = (c, t.table.view(i)) {
                        b.watch(&v);
                    }
                }
                let v = t.table.view(s).unwrap();
                let a = ctl_action(&mut ctls[s], &v);
                if t.table.act(s, a).is_err() {
                    let fallback = if v.legal.can_check { Action::Check } else { Action::Call };
                    t.table.act(s, fallback).unwrap();
                }
            }
            t.finish_hand();
        }
        t.winner().map(|w| (w, t.hands))
    }

    fn quick(p: Personality) -> Personality {
        Personality { sims: 120, ..p }
    }

    /// A reasonable player to stand in for you.
    fn stand_in() -> Personality {
        Personality { name: "you", aggression: 0.5, bluff: 0.25, trap: 0.15, caution: 0.03, adapt: 0.5, sizing: 1.0, sims: 120, wild: 0.0, loose: 0.0, lure: 0.0 }
    }

    #[test]
    #[ignore = "slow: run with --release --ignored --nocapture"]
    fn the_bear_and_the_monarch_beat_simple_opponents() {
        use crate::poker_game::{BEAR, MONARCH};
        let games = 30u64;
        let (mut bear_wins, mut mon_wins) = (0, 0);
        for g in 0..games {
            let mut ctls = vec![Ctl::Station, Ctl::Maniac(g * 7 + 1), Ctl::Ai(Brain::new(quick(bear()), g)), Ctl::Rock, Ctl::Station];
            if tournament(&mut ctls, g).map(|w| w.0) == Some(BEAR) {
                bear_wins += 1;
            }
            let mut ctls = vec![Ctl::Rock, Ctl::Station, Ctl::Maniac(g * 9 + 1), Ctl::Station, Ctl::Ai(Brain::new(quick(monarch()), g + 99))];
            if tournament(&mut ctls, g + 1000).map(|w| w.0) == Some(MONARCH) {
                mon_wins += 1;
            }
        }
        eprintln!("bear won {bear_wins} of {games}, monarch won {mon_wins} of {games} (a fifth would be luck)");
        assert!(bear_wins as f32 > 0.4 * games as f32, "{bear_wins}");
        assert!(mon_wins as f32 > 0.4 * games as f32, "{mon_wins}");
    }

    /// All five as computers: who wins, and how long a game is. The four who start take turns in the
    /// seats, so the seat does not decide it.
    #[test]
    #[ignore = "slow: run with --release --ignored --nocapture"]
    fn balance_report() {
        let sims = 0usize; // (0: each one's own number of sims, as in the game)
        let mut wins = std::collections::BTreeMap::<&str, u32>::new();
        let mut hands = Vec::new();
        for g in 0..100u64 {
            let mut four = vec![stand_in(), cat_king(), bear(), father_time()];
            four.rotate_left((g % 4) as usize);
            let mut ctls: Vec<Ctl> = four.into_iter().map(|p| Ctl::Ai(Brain::new(Personality { sims: if sims == 0 { p.sims } else { sims }, ..p }, g * 13 + 1))).collect();
            ctls.push(Ctl::Ai(Brain::new(Personality { sims: if sims == 0 { monarch().sims } else { sims }, ..monarch() }, g + 99)));
            let names: Vec<&'static str> = ctls
                .iter()
                .map(|c| match c {
                    Ctl::Ai(b) => b.personality.name,
                    _ => "?",
                })
                .collect();
            if let Some((w, h)) = tournament(&mut ctls, g + 9000) {
                *wins.entry(names[w]).or_default() += 1;
                hands.push(h);
            }
        }
        hands.sort();
        eprintln!("wins {wins:?}");
        eprintln!("hands: median {} max {} min {}", hands[hands.len() / 2], hands.last().unwrap(), hands[0]);
    }
}
