//! The Hold'em table: seats, blinds, betting rounds, side pots and showdown (no-limit).
//!
//! `Table` only knows the rules. It is given a shuffled deck for each hand (see `poker_fair`) and
//! then asks each player in turn for an `Action`. The computer players are never handed the table:
//! they get a `View`, which holds only what a player at a real table can see, plus their own two
//! cards (see `Table::view`).
//!
//! Every chip is accounted for: `Table::total_chips` stays the same through any hand, which the
//! tests check on thousands of random hands.
use crate::poker_cards::{self as cards, Card};
use crate::poker_fair::Deck;

pub type Chips = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
}

impl Street {
    pub fn label(self) -> &'static str {
        match self {
            Street::Preflop => "Preflop",
            Street::Flop => "Flop",
            Street::Turn => "Turn",
            Street::River => "River",
        }
    }
}

/// What a player can do. `Raise(to)` is the new total bet of the round ("raise to 300"), which is
/// also how a first bet is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Fold,
    Check,
    Call,
    Raise(Chips),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// The hand is over, or it is another seat's turn.
    NotYourTurn,
    /// Checking while there is a bet to call.
    CannotCheck,
    /// Raising less than the minimum (and not all-in).
    RaiseTooSmall { min: Chips },
    /// Raising more than the stack, or not raising at all.
    BadRaise,
    /// Raising when the betting is not open to this player.
    CannotRaise,
}

/// What happened at the table, in order. This is the public record of a hand.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    HandStart { hand_no: u64, button: usize, small_blind: usize, big_blind: usize },
    Blind { seat: usize, amount: Chips },
    Fold { seat: usize },
    Check { seat: usize },
    Call { seat: usize, amount: Chips, all_in: bool },
    /// `to` is the new total bet of the round; `bet` is a first bet, otherwise a raise.
    Raise { seat: usize, to: Chips, added: Chips, bet: bool, all_in: bool },
    /// Cards dealt to the board: the street they start.
    Board { street: Street, cards: Vec<Card> },
    /// An unmatched part of a bet goes back.
    Uncalled { seat: usize, amount: Chips },
    /// A player's cards turned over at showdown.
    Show { seat: usize, hole: [Card; 2], score: u32 },
    /// A pot (or a part of it) won.
    Won { seat: usize, amount: Chips, pot: usize },
    /// A player is out of chips.
    Busted { seat: usize },
}

#[derive(Debug, Clone)]
pub struct Seat {
    pub stack: Chips,
    /// Chips put in this betting round.
    pub bet: Chips,
    /// Chips put in all hand.
    pub committed: Chips,
    pub hole: Option<[Card; 2]>,
    pub folded: bool,
    pub all_in: bool,
    /// Dealt into the current hand.
    pub in_hand: bool,
    acted: bool,
    raise_locked: bool,
}

impl Seat {
    fn new(stack: Chips) -> Self {
        Seat { stack, bet: 0, committed: 0, hole: None, folded: false, all_in: false, in_hand: false, acted: false, raise_locked: false }
    }

    /// Still in the hand and able to bet (not folded, not all-in).
    fn live(&self) -> bool {
        self.in_hand && !self.folded && !self.all_in
    }

    fn in_pot(&self) -> bool {
        self.in_hand && !self.folded
    }
}

/// A pot at the end of a hand.
#[derive(Debug, Clone, PartialEq)]
pub struct Pot {
    pub amount: Chips,
    pub eligible: Vec<usize>,
    pub winners: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HandResult {
    pub pots: Vec<Pot>,
    /// What each seat won in all (0 for seats that won nothing).
    pub won: Vec<Chips>,
    /// Whether cards were turned over (otherwise everyone else folded).
    pub showdown: bool,
}

/// What the player to act may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Legal {
    pub can_check: bool,
    /// Chips to call (0 when checking; less than the bet when the stack is short).
    pub call: Chips,
    pub can_raise: bool,
    /// The smallest raise-to (the stack, when that is all that is left).
    pub min_raise_to: Chips,
    /// The most that can be raised to: everything.
    pub max_raise_to: Chips,
}

/// Everything one player may know: their own cards, the board, the chips and what has been done.
/// Nothing else is in it: no other player's cards and no deck.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub seat: usize,
    pub hole: [Card; 2],
    pub board: Vec<Card>,
    pub street: Street,
    pub hand_no: u64,
    pub button: usize,
    pub small_blind: Chips,
    pub big_blind: Chips,
    pub stacks: Vec<Chips>,
    pub bets: Vec<Chips>,
    pub committed: Vec<Chips>,
    pub folded: Vec<bool>,
    pub all_in: Vec<bool>,
    pub in_hand: Vec<bool>,
    pub pot: Chips,
    pub current_bet: Chips,
    pub legal: Legal,
    /// The public record of the hand so far.
    pub history: Vec<Event>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub seats: Vec<Seat>,
    pub button: usize,
    pub small_blind: Chips,
    pub big_blind: Chips,
    pub street: Street,
    pub board: Vec<Card>,
    pub to_act: Option<usize>,
    pub current_bet: Chips,
    last_raise: Chips,
    pub hand_no: u64,
    deck: Deck,
    deck_pos: usize,
    pub history: Vec<Event>,
    pub result: Option<HandResult>,
    /// A hand is being played.
    pub running: bool,
}

impl Table {
    pub fn new(stacks: &[Chips], small_blind: Chips, big_blind: Chips) -> Self {
        Table {
            seats: stacks.iter().map(|&s| Seat::new(s)).collect(),
            button: stacks.len().saturating_sub(1),
            small_blind,
            big_blind,
            street: Street::Preflop,
            board: Vec::new(),
            to_act: None,
            current_bet: 0,
            last_raise: big_blind,
            hand_no: 0,
            deck: [0; 52],
            deck_pos: 0,
            history: Vec::new(),
            result: None,
            running: false,
        }
    }

    /// Adds a seat with `stack` chips (between hands). Returns its number.
    pub fn add_seat(&mut self, stack: Chips) -> usize {
        self.seats.push(Seat::new(stack));
        self.seats.len() - 1
    }

    /// All the chips on the table: stacks and what is in the pot.
    pub fn total_chips(&self) -> Chips {
        self.seats.iter().map(|s| s.stack + s.committed).sum()
    }

    /// The chips in the middle: everything put in this hand.
    pub fn pot(&self) -> Chips {
        self.seats.iter().map(|s| s.committed).sum()
    }

    /// Seats that still have chips.
    pub fn alive(&self) -> Vec<usize> {
        (0..self.seats.len()).filter(|&i| self.seats[i].stack > 0).collect()
    }

    fn next_where(&self, from: usize, f: impl Fn(&Seat) -> bool) -> Option<usize> {
        let n = self.seats.len();
        (1..=n).map(|k| (from + k) % n).find(|&i| f(&self.seats[i]))
    }

    /// The seats dealt in, in dealing order (the first one after the button first).
    pub fn deal_order(&self) -> Vec<usize> {
        let n = self.seats.len();
        (1..=n).map(|k| (self.button + k) % n).filter(|&i| self.seats[i].in_hand).collect()
    }

    /// For testing only (the developer rows of the gear): puts `want` in this seat's hand for the hand that has just
    /// been dealt, by swapping each card with wherever it was. The deck is no longer the fair one. False when the
    /// seat has no cards, or the board has already begun.
    pub fn rig_hole(&mut self, seat: usize, want: [Card; 2]) -> bool {
        if want[0] == want[1] || !self.board.is_empty() {
            return false;
        }
        let Some(old) = self.seats.get(seat).and_then(|s| s.hole) else { return false };
        let swap = |t: &mut Table, a: Card, b: Card| {
            if a == b {
                return;
            }
            let map = |c: &mut Card| {
                if *c == a {
                    *c = b;
                } else if *c == b {
                    *c = a;
                }
            };
            t.deck.iter_mut().for_each(map);
            for s in &mut t.seats {
                if let Some(h) = s.hole.as_mut() {
                    h.iter_mut().for_each(map);
                }
            }
        };
        swap(self, old[0], want[0]);
        let now = self.seats[seat].hole.map_or(old[1], |h| h[1]);
        swap(self, now, want[1]);
        self.seats[seat].hole == Some(want)
    }

    /// Starts a hand with this (already shuffled) deck. False when fewer than two seats have chips.
    pub fn start_hand(&mut self, deck: Deck) -> bool {
        if self.running || self.alive().len() < 2 {
            return false;
        }
        self.hand_no += 1;
        self.deck = deck;
        self.board.clear();
        self.history.clear();
        self.result = None;
        self.street = Street::Preflop;
        for s in &mut self.seats {
            s.bet = 0;
            s.committed = 0;
            s.hole = None;
            s.folded = false;
            s.all_in = false;
            s.acted = false;
            s.raise_locked = false;
            s.in_hand = s.stack > 0;
        }
        // the button moves on to the next seat that is playing
        self.button = self.next_where(self.button, |s| s.in_hand).unwrap_or(0);
        let order = self.deal_order();
        let n = order.len();
        for (i, &seat) in order.iter().enumerate() {
            self.seats[seat].hole = Some([self.deck[2 * i], self.deck[2 * i + 1]]);
        }
        self.deck_pos = 2 * n;
        let (sb, bb) = if n == 2 {
            (self.button, order[0].min(order[1]) + order[0].max(order[1]) - self.button)
        } else {
            (order[0], order[1])
        };
        self.history.push(Event::HandStart { hand_no: self.hand_no, button: self.button, small_blind: sb, big_blind: bb });
        self.running = true;
        self.post(sb, self.small_blind);
        self.post(bb, self.big_blind);
        self.current_bet = self.big_blind;
        self.last_raise = self.big_blind;
        // first to act: after the big blind (heads-up: the small blind, who is the button)
        let first = if n == 2 { sb } else { self.next_where(bb, |_| true).unwrap_or(sb) };
        self.to_act = None;
        self.find_actor(first, true);
        if self.to_act.is_none() {
            self.close_round();
        }
        true
    }

    fn post(&mut self, seat: usize, amount: Chips) {
        let put = amount.min(self.seats[seat].stack);
        let s = &mut self.seats[seat];
        s.stack -= put;
        s.bet += put;
        s.committed += put;
        if s.stack == 0 {
            s.all_in = true;
        }
        self.history.push(Event::Blind { seat, amount: put });
    }

    /// Sets `to_act` to the first seat from `from` on (`from` included when `include`) that has to
    /// act, or leaves it `None` when the round is over.
    fn find_actor(&mut self, from: usize, include: bool) {
        let n = self.seats.len();
        let start = if include { 0 } else { 1 };
        self.to_act = (start..start + n)
            .map(|k| (from + k) % n)
            .find(|&i| {
                let s = &self.seats[i];
                s.live() && (!s.acted || s.bet < self.current_bet)
            });
    }

    fn others_can_respond(&self, seat: usize) -> bool {
        self.seats.iter().enumerate().any(|(i, s)| i != seat && s.live())
    }

    /// What `seat` may do now.
    pub fn legal(&self, seat: usize) -> Legal {
        let s = &self.seats[seat];
        let to_call = self.current_bet.saturating_sub(s.bet);
        let min_to = if self.current_bet == 0 { self.big_blind } else { self.current_bet + self.last_raise };
        let max_to = s.bet + s.stack;
        Legal {
            can_check: to_call == 0,
            call: to_call.min(s.stack),
            can_raise: s.stack > to_call && !s.raise_locked && self.others_can_respond(seat),
            min_raise_to: min_to.min(max_to),
            max_raise_to: max_to,
        }
    }

    /// Applies an action by the player to act.
    pub fn act(&mut self, seat: usize, action: Action) -> Result<(), ActionError> {
        if !self.running || self.to_act != Some(seat) {
            return Err(ActionError::NotYourTurn);
        }
        let legal = self.legal(seat);
        match action {
            Action::Fold => {
                self.seats[seat].folded = true;
                self.history.push(Event::Fold { seat });
            }
            Action::Check => {
                if !legal.can_check {
                    return Err(ActionError::CannotCheck);
                }
                self.history.push(Event::Check { seat });
            }
            Action::Call => {
                let amount = legal.call;
                let s = &mut self.seats[seat];
                s.stack -= amount;
                s.bet += amount;
                s.committed += amount;
                if s.stack == 0 {
                    s.all_in = true;
                }
                let all_in = s.all_in;
                if amount == 0 {
                    self.history.push(Event::Check { seat });
                } else {
                    self.history.push(Event::Call { seat, amount, all_in });
                }
            }
            Action::Raise(to) => {
                if !legal.can_raise {
                    return Err(ActionError::CannotRaise);
                }
                if to <= self.current_bet || to > legal.max_raise_to {
                    return Err(ActionError::BadRaise);
                }
                if to < legal.min_raise_to {
                    return Err(ActionError::RaiseTooSmall { min: legal.min_raise_to });
                }
                let bet_before = self.current_bet;
                let added = to - self.seats[seat].bet;
                let s = &mut self.seats[seat];
                s.stack -= added;
                s.bet += added;
                s.committed += added;
                if s.stack == 0 {
                    s.all_in = true;
                }
                let all_in = s.all_in;
                let raise_size = to - bet_before;
                self.history.push(Event::Raise { seat, to, added, bet: bet_before == 0, all_in });
                if raise_size >= self.last_raise {
                    // a full raise: everyone gets to act again, and may raise again
                    self.last_raise = raise_size;
                    for (i, o) in self.seats.iter_mut().enumerate() {
                        if i != seat {
                            o.acted = false;
                            o.raise_locked = false;
                        }
                    }
                } else {
                    // a short all-in: the others must answer it, but players who had already
                    // acted cannot raise over it
                    for (i, o) in self.seats.iter_mut().enumerate() {
                        if i != seat && o.acted {
                            o.raise_locked = true;
                            o.acted = false;
                        }
                    }
                }
                self.current_bet = to;
            }
        }
        self.seats[seat].acted = true;
        self.advance(seat);
        Ok(())
    }

    fn advance(&mut self, from: usize) {
        // everyone else folded: the hand is over
        let in_pot: Vec<usize> = (0..self.seats.len()).filter(|&i| self.seats[i].in_pot()).collect();
        if in_pot.len() == 1 {
            self.finish(false);
            return;
        }
        self.find_actor(from, false);
        if self.to_act.is_none() {
            self.close_round();
        }
    }

    /// The betting round is over: next street, or showdown.
    fn close_round(&mut self) {
        self.return_uncalled();
        for s in &mut self.seats {
            s.bet = 0;
            s.acted = false;
            s.raise_locked = false;
        }
        self.current_bet = 0;
        self.last_raise = self.big_blind;
        let able = self.seats.iter().filter(|s| s.live()).count();
        loop {
            if self.street == Street::River {
                self.finish(true);
                return;
            }
            self.deal_street();
            if able >= 2 {
                let first = self.next_where(self.button, |s| s.live());
                self.to_act = first;
                if self.to_act.is_some() {
                    return;
                }
            }
            // (nobody can bet any more: the rest of the board is dealt)
        }
    }

    fn deal_street(&mut self) {
        self.deck_pos += 1; // the burn card
        let (street, count) = match self.street {
            Street::Preflop => (Street::Flop, 3),
            Street::Flop => (Street::Turn, 1),
            _ => (Street::River, 1),
        };
        let dealt: Vec<Card> = self.deck[self.deck_pos..self.deck_pos + count].to_vec();
        self.deck_pos += count;
        self.board.extend_from_slice(&dealt);
        self.street = street;
        self.history.push(Event::Board { street, cards: dealt });
    }

    /// Part of a bet that nobody matched goes back to the bettor.
    fn return_uncalled(&mut self) {
        let mut top = 0;
        let mut second = 0;
        let mut who = None;
        for (i, s) in self.seats.iter().enumerate() {
            if s.bet > top {
                second = top;
                top = s.bet;
                who = Some(i);
            } else if s.bet > second {
                second = s.bet;
            }
        }
        if let Some(i) = who {
            if top > second {
                let back = top - second;
                let s = &mut self.seats[i];
                s.bet -= back;
                s.committed -= back;
                s.stack += back;
                if s.stack > 0 {
                    s.all_in = false;
                }
                self.history.push(Event::Uncalled { seat: i, amount: back });
            }
        }
    }

    /// Hands out the pots.
    fn finish(&mut self, showdown: bool) {
        self.return_uncalled();
        self.to_act = None;
        let n = self.seats.len();
        let in_pot: Vec<usize> = (0..n).filter(|&i| self.seats[i].in_pot()).collect();
        let scores: Vec<Option<u32>> = (0..n)
            .map(|i| {
                if showdown && self.seats[i].in_pot() {
                    let hole = self.seats[i].hole.expect("dealt");
                    let mut all = self.board.clone();
                    all.extend_from_slice(&hole);
                    Some(cards::score(&all))
                } else {
                    None
                }
            })
            .collect();
        if showdown {
            // (the order the cards are turned over: from the first player after the button)
            for seat in self.deal_order() {
                if let Some(score) = scores[seat] {
                    self.history.push(Event::Show { seat, hole: self.seats[seat].hole.expect("dealt"), score });
                }
            }
        }
        // layers of the pot: every all-in amount makes a pot that only those who put that much in can win
        let mut levels: Vec<Chips> = in_pot.iter().map(|&i| self.seats[i].committed).collect();
        levels.sort_unstable();
        levels.dedup();
        let mut pots: Vec<Pot> = Vec::new();
        let mut previous = 0;
        for level in levels {
            let amount: Chips = self.seats.iter().map(|s| s.committed.min(level).saturating_sub(s.committed.min(previous))).sum();
            let eligible: Vec<usize> = in_pot.iter().copied().filter(|&i| self.seats[i].committed >= level).collect();
            previous = level;
            if amount == 0 {
                continue;
            }
            // (the same group as the last pot: one pot)
            if let Some(last) = pots.last_mut() {
                if last.eligible == eligible {
                    last.amount += amount;
                    continue;
                }
            }
            pots.push(Pot { amount, eligible, winners: Vec::new() });
        }
        // chips of players who folded beyond the highest level cannot exist (a folder never puts in more than the highest in-pot level... but may), so give any leftover to the top pot
        let counted: Chips = pots.iter().map(|p| p.amount).sum();
        let total = self.pot();
        if counted < total {
            if let Some(last) = pots.last_mut() {
                last.amount += total - counted;
            }
        }
        let order = self.deal_order();
        let mut won = vec![0; n];
        for (pi, pot) in pots.iter_mut().enumerate() {
            let best = pot.eligible.iter().filter_map(|&i| scores[i]).max();
            let winners: Vec<usize> = match best {
                Some(b) => pot.eligible.iter().copied().filter(|&i| scores[i] == Some(b)).collect(),
                None => pot.eligible.clone(), // (no showdown: the one left)
            };
            let share = pot.amount / winners.len() as Chips;
            let mut extra = pot.amount % winners.len() as Chips;
            // odd chips go to the winners nearest the button's left
            let mut ordered: Vec<usize> = order.iter().copied().filter(|i| winners.contains(i)).collect();
            if ordered.is_empty() {
                ordered = winners.clone();
            }
            for &w in &ordered {
                let mut amount = share;
                if extra > 0 {
                    amount += 1;
                    extra -= 1;
                }
                won[w] += amount;
                self.seats[w].stack += amount;
                self.history.push(Event::Won { seat: w, amount, pot: pi });
            }
            pot.winners = ordered;
        }
        for s in &mut self.seats {
            s.committed = 0;
            s.bet = 0;
        }
        for i in 0..n {
            if self.seats[i].in_hand && self.seats[i].stack == 0 {
                self.history.push(Event::Busted { seat: i });
            }
        }
        self.result = Some(HandResult { pots, won, showdown });
        self.running = false;
    }

    /// What `seat` can see right now. Only valid while the hand is running and the seat holds cards.
    pub fn view(&self, seat: usize) -> Option<View> {
        let hole = self.seats.get(seat)?.hole?;
        // the public record: showdown cards only appear once the hand is over
        let history = self.history.clone();
        Some(View {
            seat,
            hole,
            board: self.board.clone(),
            street: self.street,
            hand_no: self.hand_no,
            button: self.button,
            small_blind: self.small_blind,
            big_blind: self.big_blind,
            stacks: self.seats.iter().map(|s| s.stack).collect(),
            bets: self.seats.iter().map(|s| s.bet).collect(),
            committed: self.seats.iter().map(|s| s.committed).collect(),
            folded: self.seats.iter().map(|s| s.folded).collect(),
            all_in: self.seats.iter().map(|s| s.all_in).collect(),
            in_hand: self.seats.iter().map(|s| s.in_hand).collect(),
            pot: self.pot(),
            current_bet: self.current_bet,
            legal: self.legal(seat),
            history,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poker_cards::parse;

    /// "AsKd7h" -> cards.
    fn parse_all(s: &str) -> Vec<Card> {
        s.as_bytes().chunks(2).map(|c| parse(std::str::from_utf8(c).unwrap()).unwrap()).collect()
    }

    /// A deck that deals `holes` (in dealing order: the seat after the button first), then
    /// `board` (flop, turn, river) with unused cards as the burns.
    fn stacked(holes: &[&str], board: &str) -> Deck {
        let mut used: Vec<Card> = Vec::new();
        let mut deck: Vec<Card> = Vec::new();
        for h in holes {
            let c = parse_all(h);
            assert_eq!(c.len(), 2);
            deck.extend(&c);
            used.extend(&c);
        }
        let b = parse_all(board);
        assert_eq!(b.len(), 5);
        used.extend(&b);
        let mut spare: Vec<Card> = (0..52).filter(|c| !used.contains(c)).collect();
        let mut burn = || spare.remove(0);
        deck.push(burn());
        deck.extend(&b[0..3]);
        deck.push(burn());
        deck.push(b[3]);
        deck.push(burn());
        deck.push(b[4]);
        // the rest of the deck
        let mut rest: Vec<Card> = (0..52).filter(|c| !deck.contains(c)).collect();
        deck.append(&mut rest);
        deck.try_into().expect("52 cards")
    }

    #[test]
    fn a_rigged_hand_gives_the_seat_the_cards_and_the_deck_stays_whole() {
        for game in 0..60u8 {
            let mut t = Table::new(&[1000, 1000, 1000, 1000], 10, 20);
            let deck = crate::poker_fair::deck_for(&[game; 32], "x", 1 + game as u64);
            assert!(t.start_hand(deck));
            assert!(t.rig_hole(0, [37, 23]));
            assert_eq!(t.seats[0].hole, Some([37, 23]));
            let mut all: Vec<Card> = t.seats.iter().filter_map(|s| s.hole).flatten().collect();
            all.sort_unstable();
            all.dedup();
            assert_eq!(all.len(), 8, "eight different hole cards");
            let mut deck = t.deck.to_vec();
            deck.sort_unstable();
            assert_eq!(deck, (0..52).collect::<Vec<Card>>());
            // (and the hand still plays out: everyone checks and calls down)
            let mut steps = 0;
            while t.running && steps < 200 {
                steps += 1;
                let seat = t.to_act.unwrap();
                let call = t.legal(seat).can_check;
                let _ = if call { t.act(seat, Action::Check) } else { t.act(seat, Action::Call) };
            }
            assert!(!t.running);
        }
    }

    fn xorshift(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    #[test]
    fn heads_up_the_button_posts_the_small_blind_and_acts_first() {
        let mut t = Table::new(&[1000, 1000], 10, 20);
        assert!(t.start_hand(stacked(&["AsAh", "KsKh"], "2c7d9hJsQc")));
        // the button moved to seat 0: it is the small blind, seat 1 the big blind
        assert_eq!(t.button, 0);
        assert_eq!(t.seats[0].bet, 10);
        assert_eq!(t.seats[1].bet, 20);
        assert_eq!(t.to_act, Some(0));
        // seat 1 is dealt first (the seat after the button): AA; seat 0 holds KK
        assert_eq!(t.seats[1].hole, Some([parse_all("As")[0], parse_all("Ah")[0]]));
        let l = t.legal(0);
        assert!(!l.can_check && l.call == 10 && l.can_raise && l.min_raise_to == 40);
        t.act(0, Action::Call).unwrap();
        // the big blind has the option
        assert_eq!(t.to_act, Some(1));
        assert!(t.legal(1).can_check);
        t.act(1, Action::Check).unwrap();
        // flop: after the button, the big blind (seat 1) acts first heads-up
        assert_eq!(t.street, Street::Flop);
        assert_eq!(t.board.len(), 3);
        assert_eq!(t.to_act, Some(1));
        assert_eq!(t.pot(), 40);
    }

    #[test]
    fn a_hand_plays_to_a_showdown_and_the_better_hand_takes_the_pot() {
        let mut t = Table::new(&[1000, 1000], 10, 20);
        t.start_hand(stacked(&["AsAh", "KsKh"], "2c7d9hJsQc"));
        t.act(0, Action::Raise(60)).unwrap();
        t.act(1, Action::Call).unwrap();
        for _ in 0..3 {
            t.act(1, Action::Check).unwrap();
            t.act(0, Action::Check).unwrap();
        }
        assert!(!t.running);
        let r = t.result.clone().unwrap();
        assert!(r.showdown);
        // seat 1 (AA) beats seat 0 (KK)
        assert_eq!(r.won, vec![0, 120]);
        assert_eq!(t.seats[1].stack, 1060);
        assert_eq!(t.seats[0].stack, 940);
        assert_eq!(t.total_chips(), 2000);
    }

    #[test]
    fn folding_ends_the_hand_without_a_showdown() {
        let mut t = Table::new(&[1000, 1000, 1000], 10, 20);
        t.start_hand(stacked(&["AsAh", "KsKh", "QsQh"], "2c7d9hJs3c"));
        // button seat 0; dealing starts at seat 1; seat 1 is the small blind, seat 2 the big blind
        assert_eq!(t.to_act, Some(0));
        t.act(0, Action::Fold).unwrap();
        t.act(1, Action::Fold).unwrap();
        assert!(!t.running);
        let r = t.result.clone().unwrap();
        assert!(!r.showdown);
        assert_eq!(r.won[2], 20, "the 10 of the big blind nobody matched goes back");
        assert_eq!(t.seats[2].stack, 1010);
        assert_eq!(t.seats[1].stack, 990);
        assert!(!t.history.iter().any(|e| matches!(e, Event::Show { .. })));
    }

    #[test]
    fn an_unmatched_bet_goes_back() {
        let mut t = Table::new(&[500, 100], 10, 20);
        t.start_hand(stacked(&["AsAh", "7c2d"], "3c8d9hJsQc"));
        // seat 0 (button, small blind) shoves 500, the big blind has 100 and calls all-in
        t.act(0, Action::Raise(500)).unwrap();
        t.act(1, Action::Call).unwrap();
        assert!(!t.running);
        let r = t.result.clone().unwrap();
        // the 400 nobody matched went back to seat 0; AA wins the 200 pot
        assert!(t.history.iter().any(|e| matches!(e, Event::Uncalled { seat: 0, amount: 400 })));
        assert_eq!(r.won, vec![0, 200]);
        assert_eq!(t.seats[0].stack, 400);
        assert_eq!(t.seats[1].stack, 200);
        assert_eq!(t.total_chips(), 600);
    }

    #[test]
    fn side_pots_go_to_the_best_hand_each_player_can_win() {
        // three all-ins of 100, 300 and 500: the shortest stack has the best hand, the middle one second
        let mut t = Table::new(&[100, 300, 500], 10, 20);
        // button 0; dealing order 1, 2, 0
        t.start_hand(stacked(&["KsKh", "2c3d", "AsAh"], "4c8d9hJs5c"));
        // seat 0 acts first
        assert_eq!(t.to_act, Some(0));
        t.act(0, Action::Raise(100)).unwrap();
        t.act(1, Action::Raise(300)).unwrap();
        // (seat 2 cannot raise: nobody left could answer it, so it calls)
        assert!(!t.legal(2).can_raise);
        assert_eq!(t.act(2, Action::Raise(500)), Err(ActionError::CannotRaise));
        t.act(2, Action::Call).unwrap();
        assert!(!t.running, "all in: the hand runs out");
        let r = t.result.clone().unwrap();
        // seat 0 (AA) takes the main pot of 300, seat 1 (KK) the side pot of 400, seat 2 keeps the 200 it did not need
        assert_eq!(r.pots.len(), 2);
        assert_eq!(r.won, vec![300, 400, 0]);
        assert_eq!(t.seats[2].stack, 200);
        assert_eq!(t.total_chips(), 900);
    }

    #[test]
    fn equal_hands_split_the_pot_and_the_odd_chip_goes_left_of_the_button() {
        let mut t = Table::new(&[1000, 1000, 1000], 5, 10);
        // everyone plays the board (a royal flush on it), so all three split
        t.start_hand(stacked(&["2c3d", "2d3c", "2h3h"], "AsKsQsJsTs"));
        t.act(0, Action::Call).unwrap();
        t.act(1, Action::Call).unwrap();
        t.act(2, Action::Check).unwrap();
        for _ in 0..3 {
            // the small blind (seat 1) acts first after the flop
            t.act(1, Action::Check).unwrap();
            t.act(2, Action::Check).unwrap();
            t.act(0, Action::Check).unwrap();
        }
        let r = t.result.clone().unwrap();
        assert_eq!(r.won, vec![10, 10, 10]);
        assert_eq!(t.total_chips(), 3000);
        // a pot of 20 split two ways
        let mut t = Table::new(&[1000, 1000, 1000], 5, 10);
        t.start_hand(stacked(&["2c3d", "2d3c", "2h3h"], "AsKsQsJsTs"));
        t.act(0, Action::Fold).unwrap();
        t.act(1, Action::Call).unwrap();
        t.act(2, Action::Check).unwrap();
        for _ in 0..3 {
            t.act(1, Action::Check).unwrap();
            t.act(2, Action::Check).unwrap();
        }
        let r = t.result.clone().unwrap();
        assert_eq!(r.won, vec![0, 10, 10]);
        // a 25-chip pot (the folded small blind is in it) split two ways: the odd chip goes to the
        // winner first to the left of the button
        let mut t = Table::new(&[1000, 1000, 1000], 5, 10);
        t.start_hand(stacked(&["2c3d", "2d3c", "2h3h"], "AsKsQsJsTs"));
        t.act(0, Action::Call).unwrap();
        t.act(1, Action::Fold).unwrap();
        t.act(2, Action::Check).unwrap();
        for _ in 0..3 {
            t.act(2, Action::Check).unwrap();
            t.act(0, Action::Check).unwrap();
        }
        let r = t.result.clone().unwrap();
        assert_eq!(r.won.iter().sum::<Chips>(), 25);
        assert_eq!(r.won, vec![12, 0, 13]);
    }

    /// A random but legal action for the player to act.
    fn random_action(t: &Table, seat: usize, rng: &mut u64) -> Action {
        let l = t.legal(seat);
        match xorshift(rng) % 10 {
            0 => Action::Fold,
            1..=5 => {
                if l.can_check {
                    Action::Check
                } else {
                    Action::Call
                }
            }
            6..=8 => {
                if l.can_raise {
                    let span = l.max_raise_to - l.min_raise_to;
                    Action::Raise(l.min_raise_to + if span == 0 { 0 } else { xorshift(rng) % (span + 1) })
                } else {
                    Action::Call
                }
            }
            _ => {
                if l.can_raise {
                    Action::Raise(l.max_raise_to)
                } else {
                    Action::Call
                }
            }
        }
    }

    #[test]
    fn chips_are_never_made_or_lost_over_thousands_of_random_hands() {
        let mut rng = 0x1234_5678_9abc_def1_u64;
        for n in 2..=5usize {
            for game in 0..40 {
                let stacks: Vec<Chips> = (0..n).map(|_| 200 + xorshift(&mut rng) % 2000).collect();
                let total: Chips = stacks.iter().sum();
                let mut t = Table::new(&stacks, 10, 20);
                for hand in 0..60 {
                    if t.alive().len() < 2 {
                        break;
                    }
                    let mut deck = crate::poker_fair::deck_for(&[game as u8; 32], "x", hand);
                    // (a different seed per game and per hand: also shuffle by the rng)
                    deck.rotate_left((xorshift(&mut rng) % 52) as usize);
                    assert!(t.start_hand(deck));
                    let mut steps = 0;
                    while t.running {
                        steps += 1;
                        assert!(steps < 200, "a hand must end");
                        let seat = t.to_act.expect("someone is to act while it runs");
                        let a = random_action(&t, seat, &mut rng);
                        t.act(seat, a).unwrap_or_else(|e| panic!("{a:?} was legal: {e:?}"));
                        assert_eq!(t.total_chips(), total);
                    }
                    assert_eq!(t.total_chips(), total);
                    assert!(t.seats.iter().all(|s| s.committed == 0 && s.bet == 0));
                    let r = t.result.as_ref().expect("a result");
                    // everything in the pots was handed out
                    assert_eq!(r.won.iter().sum::<Chips>(), r.pots.iter().map(|p| p.amount).sum::<Chips>());
                    // busted means no chips, and the table says so
                    for (i, s) in t.seats.iter().enumerate() {
                        let said = t.history.iter().any(|e| matches!(e, Event::Busted { seat } if *seat == i));
                        assert_eq!(said, s.in_hand && s.stack == 0, "seat {i}");
                    }
                }
            }
        }
    }

    #[test]
    fn raises_must_be_big_enough_and_a_short_all_in_does_not_reopen_the_betting() {
        let mut t = Table::new(&[1000, 1000, 70], 10, 20);
        // button 0, small blind seat 1, big blind seat 2 (70 behind)
        t.start_hand(stacked(&["AsAh", "KsKh", "QsQh"], "2c7d9hJs3c"));
        assert_eq!(t.to_act, Some(0));
        // the minimum raise is to 40; 30 is too small and 20 is not a raise
        assert_eq!(t.act(0, Action::Raise(30)), Err(ActionError::RaiseTooSmall { min: 40 }));
        assert_eq!(t.act(0, Action::Raise(20)), Err(ActionError::BadRaise));
        assert_eq!(t.act(0, Action::Check), Err(ActionError::CannotCheck));
        assert_eq!(t.act(1, Action::Call), Err(ActionError::NotYourTurn));
        t.act(0, Action::Raise(40)).unwrap();
        // small blind raises to 100 (a raise of 60)
        t.act(1, Action::Raise(100)).unwrap();
        // the big blind, with 70 in total, goes all-in for less than a full raise
        assert_eq!(t.legal(2).min_raise_to, 70, "a short stack may raise by whatever it has left");
        t.act(2, Action::Raise(70)).unwrap_err();
        t.act(2, Action::Call).unwrap();
        assert!(t.seats[2].all_in);
        // seat 0 must call 60 more: the betting was not reopened by anything short
        assert_eq!(t.to_act, Some(0));
        assert_eq!(t.legal(0).call, 60);
        assert!(t.legal(0).can_raise, "a full raise by seat 1 stands: seat 0 may re-raise it");
        t.act(0, Action::Call).unwrap();
        assert_eq!(t.street, Street::Flop);
    }

    #[test]
    fn a_short_all_in_raise_locks_those_who_already_acted() {
        let mut t = Table::new(&[1000, 1000, 1000, 130], 10, 20);
        // button 0; blinds seat 1 and 2; seat 3 (130) acts first... order: after the big blind
        t.start_hand(stacked(&["AsAh", "KsKh", "QsQh", "JsJh"], "2c7d9h5s3c"));
        assert_eq!(t.to_act, Some(3));
        t.act(3, Action::Call).unwrap();
        t.act(0, Action::Raise(100)).unwrap();
        t.act(1, Action::Call).unwrap();
        t.act(2, Action::Call).unwrap();
        // seat 3 shoves: 130 total, a raise of only 30 over 100 (less than the last raise of 80)
        assert!(t.legal(3).can_raise);
        t.act(3, Action::Raise(130)).unwrap();
        // seat 0 already acted and cannot raise over a short all-in; it may call
        assert_eq!(t.to_act, Some(0));
        assert!(!t.legal(0).can_raise);
        assert_eq!(t.act(0, Action::Raise(500)), Err(ActionError::CannotRaise));
        t.act(0, Action::Call).unwrap();
    }

    #[test]
    fn a_player_never_sees_the_other_cards() {
        // two tables that differ only in what the other players hold and in the cards not yet dealt
        let mk = |other: &str, board_tail: &str| {
            let mut t = Table::new(&[1000, 1000, 1000], 10, 20);
            t.start_hand(stacked(&["AsAh", other, "QsQh"], &format!("2c7d9h{board_tail}")));
            t
        };
        let mut a = mk("KsKh", "JsTs");
        let mut b = mk("8s8h", "3d4d");
        for t in [&mut a, &mut b] {
            t.act(0, Action::Call).unwrap();
            t.act(1, Action::Call).unwrap();
            t.act(2, Action::Check).unwrap();
        }
        // seat 1 holds AsAh in both... the seat that is dealt "AsAh" is seat 1 (first after the button)
        for seat in [1usize] {
            let (va, vb) = (a.view(seat).unwrap(), b.view(seat).unwrap());
            assert_eq!(va.hole, vb.hole);
            assert_eq!(va, vb, "the view must not depend on anyone else's cards or the rest of the deck");
        }
        // and the flop (the same in both) is the only board shown so far
        assert_eq!(a.board.len(), 3);
    }

    #[test]
    fn busted_players_are_skipped_and_the_button_keeps_moving() {
        let mut t = Table::new(&[20, 1000, 1000], 10, 20);
        // seat 1 (first after the button) gets AA, seat 2 gets KK, seat 0 gets 72
        t.start_hand(stacked(&["AsAh", "KsKh", "7c2d"], "3c8d9hJsQc"));
        t.act(0, Action::Call).unwrap();
        t.act(1, Action::Call).unwrap();
        t.act(2, Action::Fold).unwrap();
        assert!(!t.running);
        assert_eq!(t.seats[0].stack, 0);
        assert!(t.history.iter().any(|e| matches!(e, Event::Busted { seat: 0 })));
        assert_eq!(t.alive(), vec![1, 2]);
        // the next hand is between the two left, and the button is on one of them
        assert!(t.start_hand(stacked(&["AsAh", "KsKh", "7c2d"], "3c8d9hJsQc")));
        assert!(!t.seats[0].in_hand && t.seats[1].in_hand && t.seats[2].in_hand);
        assert!(t.button == 1 || t.button == 2);
    }

    #[test]
    fn a_table_with_one_player_left_does_not_deal() {
        let mut t = Table::new(&[0, 500, 0], 10, 20);
        assert!(!t.start_hand(stacked(&["AsAh", "KsKh", "QsQh"], "2c7d9hJs3c")));
        assert!(!t.running);
        let mut t = Table::new(&[100, 100], 10, 20);
        assert!(t.start_hand(stacked(&["AsAh", "KsKh"], "2c7d9hJs3c")));
        assert!(!t.start_hand(stacked(&["AsAh", "KsKh"], "2c7d9hJs3c")), "a hand is already running");
    }
}
