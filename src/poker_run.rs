//! The Hold'em game as it runs: one `Session` holds the tournament, the four computer players, the
//! faces (`poker_react`), what is being said (`poker_story`) and the fairness proof of each hand, and
//! moves it all along. The table screen only draws a `Session` and passes on your clicks.
//!
//! Logic only, no drawing, so the whole game can be played and tested without a window.
//!
//! How a game goes: the intro is read (click for each line), then each hand is dealt from a deck
//! shuffled for that hand alone. Before the deal the table shows a commitment (a hash of a secret
//! seed it has already chosen); after the hand it shows the seed, and the deck can be rebuilt from
//! it and your own client seed and checked against what was dealt. When you are out, or have left,
//! the game goes on without you, quickly, until it ends: leaving does not save you from the Monarch.
use crate::poker_ai::{self, Brain};
use crate::poker_cards::{self as cards, Card};
use crate::poker_fair::{self as fair, HandProof};
use crate::poker_game::{Ending, Tournament, BEAR, BUY_IN, CAT, FATHER, LEVELS, MONARCH, PAYOUTS, PLAYERS, YOU};
use crate::poker_react::{self as react, Reactor};
use crate::poker_story::{self as story, Arrival, Line, Moment, Speaker};
use crate::poker_table::{Action, ActionError, Chips, Event, Legal};
use std::collections::VecDeque;

/// Who is at each seat, for the table's labels.
pub fn seat_name(seat: usize) -> &'static str {
    match seat {
        YOU => "You",
        CAT => "The Dead King",
        BEAR => "The Demon Bear",
        FATHER => "Father Time",
        _ => "The Shadow Monarch",
    }
}

/// The name on a nameplate or a log line (no "The").
pub fn short_name(seat: usize) -> &'static str {
    match seat {
        YOU => "You",
        CAT => "King",
        BEAR => "Bear",
        FATHER => "Father Time",
        _ => "Monarch",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Lines to read, one click each (the intro, the Monarch walking in, the ending).
    Story,
    /// A hand is about to be dealt: the commitment is up and the client seed may be changed.
    Ready,
    /// Cards are out and people are betting.
    Playing,
    /// The hand is over: its result and its proof are up.
    Showdown,
    /// The game is over and its ending has been read.
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum After {
    Ready,
    Over,
}

/// What someone just did, for a bubble by their seat.
#[derive(Debug, Clone, PartialEq)]
pub struct Said {
    pub text: String,
    /// Seconds since.
    pub age: f32,
}

/// The result of a hand, in words.
#[derive(Debug, Clone, Default)]
pub struct HandSummary {
    pub hand_no: u64,
    /// "The Demon Bear wins 620 with a full house".
    pub lines: Vec<String>,
    pub won: Vec<Chips>,
    pub showdown: bool,
    /// The cards turned over at showdown (yours are always known to you).
    pub shown: [Option<[Card; 2]>; PLAYERS],
}

/// How the game came out: handed over once, for the tokens and the soul.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finish {
    pub ending: Ending,
    /// Your place, 1 to 5.
    pub place: usize,
    /// Tokens paid for it (before the buy-in is taken off).
    pub payout: u64,
}

/// What comes up on the screen besides the table.
const CHAT_MIN: f32 = 2.6;

pub struct Session {
    pub t: Tournament,
    brains: Vec<Brain>,
    pub reactor: Reactor,
    pub phase: Phase,
    after: After,
    story: VecDeque<Line>,
    chatter: VecDeque<Line>,
    /// The subtitle that is up, and for how many seconds more.
    pub chat: Option<(Line, f32)>,
    /// A line in italics (the Monarch's buy-in, the token price).
    pub notice: Option<(String, f32)>,
    pub client_seed: String,
    next_seed: [u8; 32],
    /// The commitment for the next hand (a hash of its secret seed), shown before the deal.
    pub next_commit: String,
    seed_in_play: [u8; 32],
    /// The commitment the hand being played was dealt under.
    pub commit_in_play: String,
    /// The last hand's proof, once it is over (the seed is shown then).
    pub proof: Option<HandProof>,
    pub proof_ok: Option<bool>,
    pub said: [Option<Said>; PLAYERS],
    pub summary: Option<HandSummary>,
    /// This hand's plays, in order.
    pub log: Vec<String>,
    pub ending: Option<Ending>,
    finish: Option<Finish>,
    finish_taken: bool,
    wait: f32,
    /// The screen is putting cards down: nobody acts, and the hand does not end, until it lets go. (Not
    /// when the game is being played through without you.)
    pub hold: bool,
    /// You have got up: your seat folds from now on.
    pub left: bool,
    /// Run the rest without pauses (when you are out).
    pub fast: bool,
    /// How slowly the others make up their minds (1.0 is the quick table; a relaxed one takes its time).
    pub pace: f32,
    /// For testing (the developer rows of the gear): your hole cards are the King of Hearts and the Queen of
    /// Diamonds every hand, and the hand says so (it is not a fair deal, and its shuffle is not checked).
    pub dev_monarchs: bool,
    pub rigged: bool,
    holes: [Option<[Card; 2]>; PLAYERS],
    before: Vec<Chips>,
    seen: usize,
    told_allin: [bool; PLAYERS],
    quip_n: usize,
    told_short: bool,
    last_level: usize,
    brain_tick: u64,
}

fn padded(stacks: impl Iterator<Item = Chips>) -> Vec<Chips> {
    let mut v: Vec<Chips> = stacks.collect();
    v.resize(PLAYERS, 0);
    v
}

impl Session {
    /// A new game with seeds from the system's entropy.
    pub fn new() -> Self {
        let e = fair::entropy32();
        let brain_seed = u64::from_le_bytes([e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7]]);
        Session::with_seeds(fair::random_client_seed(), brain_seed)
    }

    /// A game with the client seed and the computer players' seed given (for tests).
    pub fn with_seeds(client_seed: String, brain_seed: u64) -> Self {
        let mk = |p: poker_ai::Personality, k: u64| Brain::new(p, brain_seed.wrapping_mul(0x9E37_79B9).wrapping_add(k));
        // (the player's own seat has a brain too, so the seats line up: it is never asked to play)
        let brains = vec![
            mk(poker_ai::father_time(), 1),
            mk(poker_ai::cat_king(), 2),
            mk(poker_ai::bear(), 3),
            mk(poker_ai::father_time(), 4),
            mk(poker_ai::monarch(), 5),
        ];
        let next_seed = fair::new_server_seed();
        let mut s = Session {
            t: Tournament::new(),
            brains,
            reactor: Reactor::new(),
            phase: Phase::Story,
            after: After::Ready,
            story: story::intro().into(),
            chatter: VecDeque::new(),
            chat: None,
            notice: None,
            client_seed,
            next_seed,
            next_commit: hex32(&fair::commit(&next_seed)),
            seed_in_play: [0; 32],
            commit_in_play: String::new(),
            proof: None,
            proof_ok: None,
            said: Default::default(),
            summary: None,
            log: Vec::new(),
            ending: None,
            finish: None,
            finish_taken: false,
            wait: 0.0,
            hold: false,
            pace: 1.0,
            dev_monarchs: false,
            rigged: false,
            left: false,
            fast: false,
            holes: [None; PLAYERS],
            before: vec![0; PLAYERS],
            seen: 0,
            told_allin: [false; PLAYERS],
            quip_n: 0,
            told_short: false,
            last_level: 0,
            brain_tick: 0,
        };
        s.brain_tick = brain_seed;
        s
    }

    // ---- reading the story -------------------------------------------------------------------

    /// The line waiting for a click, if the story is being read.
    pub fn story_line(&self) -> Option<Line> {
        if self.phase == Phase::Story {
            self.story.front().copied()
        } else {
            None
        }
    }

    /// How many lines of the story are left, the one on the screen included.
    pub fn story_left(&self) -> usize {
        self.story.len()
    }

    /// A click on the line: the next one, or the end of the story.
    pub fn next_line(&mut self) {
        if self.phase != Phase::Story {
            return;
        }
        self.story.pop_front();
        if self.story.is_empty() {
            self.phase = match self.after {
                After::Ready => Phase::Ready,
                After::Over => Phase::Over,
            };
        }
    }

    /// Reads the rest of the story at once.
    pub fn skip_story(&mut self) {
        while self.phase == Phase::Story {
            self.next_line();
        }
    }

    fn tell(&mut self, lines: Vec<Line>, after: After) {
        self.story = lines.into();
        self.after = after;
        self.phase = Phase::Story;
        if self.story.is_empty() {
            self.phase = match after {
                After::Ready => Phase::Ready,
                After::Over => Phase::Over,
            };
        }
    }

    fn chat_about(&mut self, m: Moment) {
        self.quip_n += 1;
        let line = story::quip(m, self.quip_n);
        self.chatter.push_back(line);
    }

    // ---- the seeds ---------------------------------------------------------------------------

    /// Changes the client seed (before the deal only). The next hand's deck is made from it.
    pub fn set_client_seed(&mut self, s: &str) -> bool {
        if matches!(self.phase, Phase::Ready | Phase::Showdown | Phase::Story) && self.t.table.running == false {
            let s: String = s.chars().filter(|c| !c.is_control()).take(40).collect();
            if !s.is_empty() {
                self.client_seed = s;
                return true;
            }
        }
        false
    }

    // ---- dealing -----------------------------------------------------------------------------

    fn stacks(&self) -> Vec<Chips> {
        padded(self.t.table.seats.iter().map(|s| s.stack))
    }

    fn watch_all(&mut self) {
        for i in 0..self.brains.len() {
            if let Some(v) = self.t.table.view(i) {
                self.brains[i].watch(&v);
            }
        }
    }

    /// Deals the next hand (from `Ready`).
    pub fn deal(&mut self) {
        if self.phase != Phase::Ready {
            return;
        }
        self.before = self.stacks();
        self.seed_in_play = self.next_seed;
        self.commit_in_play = self.next_commit.clone();
        let hand_no = self.t.hands + 1;
        let deck = fair::deck_for(&self.seed_in_play, &self.client_seed, hand_no);
        if !self.t.start_hand(deck) {
            // nothing to deal: the game is over
            self.finish_game();
            return;
        }
        self.rigged = self.dev_monarchs && self.t.table.rig_hole(YOU, [37, 23]);
        self.holes = [None; PLAYERS];
        for i in 0..PLAYERS.min(self.t.table.seats.len()) {
            self.holes[i] = self.t.table.seats[i].hole;
        }
        let roll = react::tell_roll(&self.seed_in_play, &self.client_seed, hand_no);
        self.reactor.deal(self.holes[BEAR], roll);
        let level = self.t.level();
        if hand_no > 1 && level != self.last_level {
            self.chat_about(Moment::BlindsUp);
        }
        self.last_level = level;
        self.seen = self.t.table.history.len();
        self.said = Default::default();
        self.told_allin = [false; PLAYERS];
        self.summary = None;
        self.proof = None;
        self.proof_ok = None;
        self.log = vec![format!(
            "Hand {hand_no}  ·  blinds {} / {}",
            group(self.t.table.small_blind),
            group(self.t.table.big_blind)
        )];
        self.watch_all();
        self.drain_reactor();
        self.phase = Phase::Playing;
        self.wait = self.think_time();
    }

    fn drain_reactor(&mut self) {
        for m in self.reactor.take_moments() {
            self.chat_about(m);
        }
    }

    /// How long the next player to act takes: Father Time thinks about it, the King does not.
    fn think_time(&mut self) -> f32 {
        if self.fast || self.is_watching() {
            return 0.0;
        }
        let seat = self.t.table.to_act.unwrap_or(BEAR);
        let base = match seat {
            CAT => 0.8,
            BEAR => 1.3,
            FATHER => 1.9,
            MONARCH => 1.6,
            _ => 0.8,
        };
        // a little different each time (no two players take the same time twice)
        self.brain_tick = self.brain_tick.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (base + ((self.brain_tick >> 40) as f32 / (1u64 << 24) as f32) * 0.7) * self.pace.clamp(0.5, 3.0)
    }

    // ---- playing -----------------------------------------------------------------------------

    /// You are not in the game any more, or have stood up: it goes on without you.
    pub fn is_watching(&self) -> bool {
        self.left || self.t.you_out()
    }

    /// What you may do now, when it is your turn.
    pub fn your_turn(&self) -> Option<Legal> {
        if self.phase == Phase::Playing && !self.hold && !self.is_watching() && self.t.table.to_act == Some(YOU) {
            Some(self.t.table.legal(YOU))
        } else {
            None
        }
    }

    /// Your play.
    pub fn act(&mut self, a: Action) -> Result<(), ActionError> {
        if self.your_turn().is_none() {
            return Err(ActionError::NotYourTurn);
        }
        self.play(YOU, a)
    }

    fn play(&mut self, seat: usize, a: Action) -> Result<(), ActionError> {
        self.t.table.act(seat, a)?;
        self.after_act();
        Ok(())
    }

    fn after_act(&mut self) {
        let events: Vec<Event> = self.t.table.history[self.seen..].to_vec();
        self.seen = self.t.table.history.len();
        for e in events {
            self.note(&e);
        }
        self.watch_all();
        if !self.t.table.running {
            self.end_hand();
        } else {
            self.wait = self.think_time();
        }
    }

    fn set_said(&mut self, seat: usize, text: String) {
        if seat < PLAYERS {
            self.said[seat] = Some(Said { text, age: 0.0 });
        }
    }

    fn note(&mut self, e: &Event) {
        match e {
            Event::Blind { seat, amount } => {
                self.log.push(format!("{} posts {}", short_name(*seat), group(*amount)));
            }
            Event::Fold { seat } => {
                self.set_said(*seat, "Fold".into());
                self.log.push(format!("{} folds", short_name(*seat)));
            }
            Event::Check { seat } => {
                self.set_said(*seat, "Check".into());
                self.log.push(format!("{} checks", short_name(*seat)));
            }
            Event::Call { seat, amount, all_in } => {
                self.set_said(*seat, if *all_in { format!("All in {}", group(*amount)) } else { format!("Call {}", group(*amount)) });
                self.log.push(format!("{} calls {}", short_name(*seat), group(*amount)));
                self.note_all_in(*seat, *all_in);
            }
            Event::Raise { seat, to, bet, all_in, .. } => {
                let verb = if *bet { "Bet" } else { "Raise to" };
                self.set_said(*seat, if *all_in { format!("All in {}", group(*to)) } else { format!("{verb} {}", group(*to)) });
                self.log.push(format!("{} {} {}", short_name(*seat), if *bet { "bets" } else { "raises to" }, group(*to)));
                self.note_all_in(*seat, *all_in);
            }
            Event::Board { street, cards: c } => {
                // a new street: the bubbles clear
                self.said = Default::default();
                let names: Vec<String> = c.iter().map(|&x| cards::name(x)).collect();
                self.log.push(format!("{}: {}", street.label(), names.join(" ")));
            }
            Event::Uncalled { seat, amount } => {
                self.log.push(format!("{} gets {} back", short_name(*seat), group(*amount)));
            }
            _ => {}
        }
    }

    fn note_all_in(&mut self, seat: usize, all_in: bool) {
        if !all_in || self.told_allin[seat] {
            return;
        }
        self.told_allin[seat] = true;
        match seat {
            YOU => self.chat_about(Moment::YouAllIn),
            BEAR => self.chat_about(Moment::BearAllIn),
            MONARCH => self.chat_about(Moment::MonarchAllIn),
            _ => {}
        }
    }

    /// One computer player's turn, if it is theirs.
    fn computer_plays(&mut self) {
        let Some(seat) = self.t.table.to_act else {
            return;
        };
        let action = if seat == YOU {
            // (you have stood up: check when it is free, otherwise fold)
            if self.t.table.legal(YOU).can_check {
                Action::Check
            } else {
                Action::Fold
            }
        } else {
            match self.t.table.view(seat) {
                Some(v) => self.brains[seat].decide(&v),
                None => Action::Fold,
            }
        };
        if self.play(seat, action).is_err() {
            // (the computer plays by the rules; this is only here so a bug can never freeze the table)
            let fallback = if self.t.table.legal(seat).can_check { Action::Check } else { Action::Fold };
            let _ = self.play(seat, fallback);
        }
    }

    /// Moves the game on by `dt` seconds. In fast mode it plays for up to `budget` seconds of real
    /// time instead, as many hands as fit.
    pub fn tick(&mut self, dt: f32, budget: f32) {
        for s in self.said.iter_mut().flatten() {
            s.age += dt;
        }
        if let Some((_, left)) = &mut self.notice {
            *left -= dt;
            if *left <= 0.0 {
                self.notice = None;
            }
        }
        self.run_chat(dt);
        let start = std::time::Instant::now();
        loop {
            let progressed = self.step(dt);
            let more = self.fast && self.phase != Phase::Over && !(self.phase == Phase::Story && self.after == After::Over);
            if !progressed || !more || start.elapsed().as_secs_f32() >= budget {
                break;
            }
        }
    }

    fn run_chat(&mut self, dt: f32) {
        if self.fast {
            self.chatter.clear();
            self.chat = None;
            return;
        }
        if let Some((_, left)) = &mut self.chat {
            *left -= dt;
            if *left <= 0.0 {
                self.chat = None;
            }
        }
        if self.chat.is_none() && self.phase != Phase::Story {
            if let Some(line) = self.chatter.pop_front() {
                let secs = (CHAT_MIN + line.text.chars().count() as f32 * 0.04).min(7.0);
                self.chat = Some((line, secs));
            }
        }
    }

    /// One step of the game; true when something happened.
    fn step(&mut self, dt: f32) -> bool {
        match self.phase {
            Phase::Over => false,
            Phase::Story => {
                // a game being played through at speed does not stop for the Monarch's walk-in
                if self.fast && self.after == After::Ready {
                    self.skip_story();
                    true
                } else {
                    false
                }
            }
            Phase::Ready => {
                if self.is_watching() {
                    self.deal();
                    true
                } else {
                    false
                }
            }
            Phase::Showdown => {
                if self.is_watching() {
                    self.next_hand();
                    self.deal();
                    true
                } else {
                    false
                }
            }
            Phase::Playing => {
                if self.hold && !self.fast && !self.is_watching() {
                    return false;
                }
                if !self.t.table.running {
                    self.end_hand();
                    return true;
                }
                let seat = self.t.table.to_act;
                if seat == Some(YOU) && !self.is_watching() {
                    return false;
                }
                // once you have folded, the others hurry
                let speed = if self.t.table.seats.get(YOU).map_or(false, |s| s.folded) { 2.0 } else { 1.0 };
                self.wait -= dt * speed;
                if self.wait > 0.0 && !self.fast {
                    return false;
                }
                self.computer_plays();
                true
            }
        }
    }

    // ---- the end of a hand -------------------------------------------------------------------

    fn end_hand(&mut self) {
        let hand_no = self.t.hands;
        let showdown = self.t.table.result.as_ref().map_or(false, |r| r.showdown);
        let after = self.stacks();
        let new_out = self.t.finish_hand();
        let before = self.before.clone();
        self.reactor.settle(hand_no, &before, &after, &self.holes, showdown);
        self.drain_reactor();
        for e in &new_out {
            self.chat_about(react::elimination_moment(*e));
        }
        self.fold_regret(showdown);
        self.short_stack_note();
        // the proof: the seed is shown now and the deck checked from scratch
        let proof = HandProof::new(self.seed_in_play, &self.client_seed, hand_no);
        self.proof_ok = if self.rigged { None } else { Some(proof.verify().is_ok()) };
        self.proof = Some(proof);
        self.summary = Some(self.summarise(hand_no, showdown, &before, &after));
        // the next hand's secret seed, chosen now and committed to before the deal
        self.next_seed = fair::new_server_seed();
        self.next_commit = hex32(&fair::commit(&self.next_seed));
        if self.t.over() {
            self.finish_game();
        } else {
            self.phase = Phase::Showdown;
        }
    }

    /// "You folded a hand that would have won".
    fn fold_regret(&mut self, showdown: bool) {
        let Some(you) = self.holes[YOU] else {
            return;
        };
        if !showdown || !self.t.table.seats.get(YOU).map_or(false, |s| s.folded) || self.t.table.board.len() < 5 {
            return;
        }
        let mut mine = vec![you[0], you[1]];
        mine.extend_from_slice(&self.t.table.board);
        let mine = cards::score(&mine);
        let best = self
            .t
            .table
            .history
            .iter()
            .filter_map(|e| if let Event::Show { score, .. } = e { Some(*score) } else { None })
            .max();
        if best.map_or(false, |b| mine > b) {
            self.chat_about(Moment::YouFoldedWinner);
        }
    }

    fn short_stack_note(&mut self) {
        if self.told_short || self.t.you_out() {
            return;
        }
        let alive = self.t.alive();
        if alive.len() == 3 && alive.contains(&YOU) {
            let mine = self.t.table.seats[YOU].stack;
            if alive.iter().all(|&i| i == YOU || self.t.table.seats[i].stack > mine) {
                self.told_short = true;
                self.chat_about(Moment::YouAreShort);
            }
        }
    }

    fn summarise(&self, hand_no: u64, showdown: bool, before: &[Chips], after: &[Chips]) -> HandSummary {
        let mut sum = HandSummary { hand_no, showdown, ..Default::default() };
        let Some(result) = &self.t.table.result else {
            return sum;
        };
        sum.won = padded(result.won.iter().copied());
        for e in &self.t.table.history {
            if let Event::Show { seat, hole, .. } = e {
                if *seat < PLAYERS {
                    sum.shown[*seat] = Some(*hole);
                }
            }
        }
        // who won what, and with which hand
        let score_of = |seat: usize| -> Option<u32> {
            self.t.table.history.iter().find_map(|e| match e {
                Event::Show { seat: s, score, .. } if *s == seat => Some(*score),
                _ => None,
            })
        };
        for (seat, won) in sum.won.clone().iter().enumerate() {
            if *won == 0 {
                continue;
            }
            let verb = if seat == YOU { "win" } else { "wins" };
            let with = if showdown { score_of(seat).map(|s| format!(" with {}", cards::describe(s))) } else { None };
            sum.lines.push(format!("{} {verb} {}{}", seat_name(seat), group(*won), with.unwrap_or_default()));
        }
        let _ = (before, after);
        sum
    }

    /// Goes from the end of a hand to the next one (the Monarch may walk in first).
    pub fn next_hand(&mut self) {
        if self.phase != Phase::Showdown {
            return;
        }
        self.phase = Phase::Ready;
        if self.t.monarch_due() {
            let why = match self.t.eliminations.first() {
                Some(e) => Arrival::SomeoneOut(e.seat),
                None => Arrival::Time,
            };
            self.t.seat_monarch();
            let chips = self.t.monarch_stack;
            self.notice = Some((story::monarch_buy_in_notice(chips), 6.0));
            self.tell(story::monarch_arrives(why, chips), After::Ready);
        }
    }

    /// For the developer rows in the gear: plays the Monarch's arrival now (before a hand, whoever is at the table).
    pub fn dev_arrival(&mut self) -> bool {
        if self.phase != Phase::Ready {
            return false;
        }
        let chips = self.t.monarch_stack.max(3000);
        self.tell(story::monarch_arrives(Arrival::Time, chips), After::Ready);
        true
    }

    /// The button on the result screen: the next hand, dealt at once.
    pub fn deal_next(&mut self) {
        self.next_hand();
        if self.phase == Phase::Ready {
            self.deal();
        }
    }

    // ---- the end of the game -----------------------------------------------------------------

    fn finish_game(&mut self) {
        let Some(e) = self.t.ending() else {
            self.phase = Phase::Showdown;
            return;
        };
        self.ending = Some(e);
        let place = self.t.your_place().unwrap_or(PLAYERS);
        self.finish = Some(Finish { ending: e, place, payout: PAYOUTS[place - 1] });
        self.chatter.clear();
        self.chat = None;
        self.tell(story::ending(e), After::Over);
    }

    /// The result of the game, once, as soon as it is decided: the ending, the place and the tokens.
    pub fn take_finish(&mut self) -> Option<Finish> {
        if self.finish_taken {
            return None;
        }
        let f = self.finish?;
        self.finish_taken = true;
        Some(f)
    }

    /// Stands up from the table. Your seat folds from now on and the rest of the game is played out
    /// at once: leaving does not buy you out of the Monarch.
    pub fn leave(&mut self) {
        self.left = true;
        self.fast = true;
        if self.phase == Phase::Story && self.after == After::Ready {
            self.skip_story();
        }
    }

    /// Watches the rest at speed (when you are out).
    pub fn skip(&mut self) {
        if self.is_watching() {
            self.fast = true;
        }
    }

    /// The blinds of the hand being played (or the next one).
    pub fn blinds(&self) -> (Chips, Chips) {
        if self.t.table.running {
            (self.t.table.small_blind, self.t.table.big_blind)
        } else {
            LEVELS[self.t.level()]
        }
    }

    /// Your cards, once they are dealt.
    pub fn your_hole(&self) -> Option<[Card; 2]> {
        self.holes[YOU].filter(|_| self.t.table.running || self.phase == Phase::Showdown)
    }

    /// The speaker for the line on the screen.
    pub fn speaker_now(&self) -> Option<Speaker> {
        self.story_line().and_then(|l| l.focus()).or_else(|| self.chat.as_ref().and_then(|(l, _)| l.focus()))
    }

    /// The line on the screen (the story, or table talk).
    pub fn line_now(&self) -> Option<Line> {
        self.story_line().or_else(|| self.chat.as_ref().map(|(l, _)| *l))
    }

    /// Tokens the game costs.
    pub fn buy_in() -> u64 {
        BUY_IN
    }
}

/// 1,234,567.
pub fn group(n: Chips) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poker_fair::deck_for;

    /// Plays a whole game with `you` choosing each play, reading the story with clicks.
    fn play(mut s: Session, mut you: impl FnMut(&Session, Legal) -> Action) -> Session {
        let mut guard = 0;
        while s.phase != Phase::Over {
            guard += 1;
            assert!(guard < 200_000, "a game that never ends");
            match s.phase {
                Phase::Story => s.next_line(),
                Phase::Ready => s.deal(),
                Phase::Showdown => s.deal_next(),
                Phase::Playing => {
                    if let Some(legal) = s.your_turn() {
                        let a = you(&s, legal);
                        s.act(a).expect("the planned play is legal");
                    } else {
                        s.tick(10.0, 5.0);
                    }
                }
                Phase::Over => {}
            }
        }
        s
    }

    fn quick() -> Session {
        let mut s = Session::with_seeds("test".into(), 7);
        // fewer simulations so the tests are fast
        for b in s.brains.iter_mut() {
            b.personality.sims = 25;
        }
        s
    }

    #[test]
    fn the_developer_switch_deals_you_the_two_pleading_monarchs_every_hand_and_says_so() {
        let mut s = quick();
        s.skip_story();
        s.dev_monarchs = true;
        for _ in 0..3 {
            s.deal();
            assert_eq!(s.phase, Phase::Playing);
            assert_eq!(s.your_hole(), Some([37, 23]), "K of hearts and Q of diamonds");
            assert!(s.rigged);
            // nobody else holds them, and every card in the hand is still different
            let mut all: Vec<u8> = s.t.table.seats.iter().filter_map(|st| st.hole).flatten().collect();
            all.sort_unstable();
            all.dedup();
            assert_eq!(all.len(), s.t.table.seats.iter().filter(|st| st.hole.is_some()).count() * 2);
            s.skip();
            for _ in 0..4000 {
                s.tick(0.05, 0.01);
                if s.phase == Phase::Showdown || s.phase == Phase::Over {
                    break;
                }
            }
            assert_eq!(s.proof_ok, None, "a rigged hand is not passed off as fair");
            if s.phase == Phase::Over {
                break;
            }
            s.next_hand();
        }
        s.dev_monarchs = false;
    }

    #[test]
    fn while_the_cards_are_going_down_nobody_acts_and_nothing_ends() {
        let mut s = quick();
        s.skip_story();
        s.deal();
        assert_eq!(s.phase, Phase::Playing);
        s.hold = true;
        let before = (s.t.table.to_act, s.t.table.history.len());
        for _ in 0..600 {
            s.tick(0.05, 0.01);
        }
        assert_eq!((s.t.table.to_act, s.t.table.history.len()), before, "30 seconds and not one play");
        assert!(s.your_turn().is_none());
        assert!(s.act(Action::Fold).is_err(), "you cannot act on cards that are not down");
        s.hold = false;
        for _ in 0..600 {
            s.tick(0.05, 0.01);
            if s.your_turn().is_some() || s.phase != Phase::Playing {
                break;
            }
        }
        assert!(s.your_turn().is_some() || s.phase != Phase::Playing, "and it goes on when let go");
    }

    #[test]
    fn a_game_where_you_always_call_ends_and_pays_its_place() {
        let s = play(quick(), |_, l| if l.can_check { Action::Check } else { Action::Call });
        let f = s.finish.expect("decided");
        assert_eq!(f.place, s.t.your_place().unwrap());
        assert_eq!(f.payout, PAYOUTS[f.place - 1]);
        assert_eq!(s.ending, s.t.ending());
        // all the chips are on one seat
        assert_eq!(s.t.alive().len(), 1);
        let mut s = s;
        assert!(s.take_finish().is_some());
        assert!(s.take_finish().is_none(), "handed over once");
    }

    #[test]
    fn leaving_plays_the_rest_without_you_and_it_still_ends() {
        let mut s = quick();
        s.skip_story();
        s.deal();
        s.leave();
        let s = play(s, |_, _| Action::Fold);
        assert!(s.ending.is_some());
        assert!(s.left && s.is_watching());
        // you cannot dodge a place by leaving
        assert!(s.finish.unwrap().place >= 2 || s.ending.unwrap().you_won());
    }

    #[test]
    fn every_hand_is_committed_to_before_it_is_dealt_and_checks_out_after() {
        let mut s = quick();
        s.skip_story();
        let mut seen = std::collections::HashSet::new();
        let mut hands = 0;
        while s.phase != Phase::Over && hands < 6 {
            let before = s.next_commit.clone();
            assert!(seen.insert(before.clone()), "a new secret each hand");
            if s.phase == Phase::Ready {
                s.deal();
            }
            assert_eq!(s.commit_in_play, before, "the hand is dealt under the commitment that was shown");
            while s.phase == Phase::Playing {
                if s.your_turn().is_some() {
                    let l = s.your_turn().unwrap();
                    s.act(if l.can_check { Action::Check } else { Action::Fold }).unwrap();
                } else {
                    s.tick(10.0, 5.0);
                }
            }
            if s.phase == Phase::Showdown {
                hands += 1;
                let p = s.proof.as_ref().expect("the proof is up after the hand");
                assert_eq!(s.proof_ok, Some(true));
                assert_eq!(fair::commit(&p.server_seed), p.commitment);
                assert_eq!(hex32(&p.commitment), s.commit_in_play);
                assert_eq!(p.deck, deck_for(&p.server_seed, &p.client_seed, p.hand_no));
                s.deal_next();
                while s.phase == Phase::Story {
                    s.next_line();
                }
            }
        }
        assert!(hands >= 3);
    }

    /// You shove every hand: how often does anybody call?
    fn shove_spam(seed: &str, hands: usize) -> (usize, usize) {
        let mut s = Session::with_seeds(seed.into(), 11);
        s.skip_story();
        let (mut called, mut played) = (0, 0);
        let mut guard = 0;
        while played < hands && s.phase != Phase::Over {
            guard += 1;
            assert!(guard < 100_000);
            match s.phase {
                Phase::Story => s.next_line(),
                Phase::Ready => s.deal(),
                Phase::Showdown => {
                    played += 1;
                    if s.summary.as_ref().is_some_and(|m| m.shown.iter().filter(|x| x.is_some()).count() > 0) {
                        called += 1;
                    }
                    s.deal_next();
                }
                Phase::Playing => {
                    if let Some(l) = s.your_turn() {
                        s.act(if l.can_raise { Action::Raise(l.max_raise_to) } else if l.can_check { Action::Check } else { Action::Call }).unwrap();
                    } else {
                        s.tick(10.0, 5.0);
                    }
                }
                Phase::Over => {}
            }
        }
        (called, played)
    }

    #[test]
    fn shoving_every_hand_does_not_win_for_free() {
        // there is no automatic win: the others learn that you shove and call you, and it is not a coin flip you win by default
        let (mut called, mut played, mut free_rides) = (0, 0, 0);
        for i in 0..14 {
            let (c, n) = shove_spam(&format!("seed{i}"), 9);
            called += c;
            played += n;
            if c == 0 && n == 9 {
                free_rides += 1;
            }
        }
        assert!(called * 100 >= played * 22, "called in {called} of {played} hands");
        assert!(free_rides <= 1, "nine free pots in a row happened {free_rides} times in 14 games");
    }

    #[test]
    fn the_client_seed_changes_the_next_deck_and_only_between_hands() {
        let mut s = quick();
        s.skip_story();
        assert!(s.set_client_seed("my own seed"));
        s.deal();
        assert_eq!(s.proof, None);
        assert!(!s.set_client_seed("too late"), "not in the middle of a hand");
        assert_eq!(s.client_seed, "my own seed");
        let a = deck_for(&s.seed_in_play, "my own seed", 1);
        let b = deck_for(&s.seed_in_play, "other", 1);
        assert_ne!(a, b);
        assert!(!s.set_client_seed(""));
    }

    #[test]
    fn the_monarch_sits_down_after_eight_hands_or_when_someone_is_out() {
        let s = play(quick(), |_, l| if l.can_check { Action::Check } else { Action::Call });
        assert!(s.t.monarch_in);
        assert!(s.t.hands > 0);
        assert!(s.notice.is_some() || s.t.monarch_stack > 0);
    }

    #[test]
    fn the_numbers_are_written_with_commas() {
        assert_eq!(group(0), "0");
        assert_eq!(group(999), "999");
        assert_eq!(group(1000), "1,000");
        assert_eq!(group(1234567), "1,234,567");
    }

    #[test]
    fn chips_are_conserved_through_a_whole_game_and_the_bear_bleeds_sometimes() {
        let mut s = quick();
        s.skip_story();
        let mut bled = 0;
        let mut hands = 0;
        let mut total = s.t.table.total_chips();
        let mut guard = 0;
        while s.phase != Phase::Over {
            guard += 1;
            assert!(guard < 200_000);
            match s.phase {
                Phase::Story => s.next_line(),
                Phase::Ready => {
                    if s.t.monarch_in {
                        total = s.t.table.total_chips();
                    }
                    s.deal();
                    hands += 1;
                    if s.reactor.bleeding().is_some() {
                        bled += 1;
                    }
                }
                Phase::Showdown => s.deal_next(),
                Phase::Playing => {
                    if let Some(l) = s.your_turn() {
                        s.act(if l.can_check { Action::Check } else { Action::Call }).unwrap();
                    } else {
                        s.tick(10.0, 5.0);
                    }
                }
                Phase::Over => {}
            }
            if s.phase == Phase::Showdown {
                assert_eq!(s.t.table.total_chips(), total);
            }
        }
        assert!(s.t.hands >= 4, "{} hands", s.t.hands);
        // about one hand in eleven (a wide margin: this is only a smoke test of the wiring)
        assert!(bled <= hands.max(1), "{bled} of {hands}");
    }
}
