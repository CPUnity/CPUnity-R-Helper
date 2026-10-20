// The dealer's hand at the Devil's Table: Mammon's robotic hand comes off his arm on a rocket, puts the
// cards down one at a time (the hole cards round the table, then for each street a burn card and the
// board cards) and flies home. This is the schedule of it, with no drawing in it: where the hand is at
// any moment, which card has landed and when, and the beats the screen plays a sound for.
//
// The game is held while the hand is busy (`busy`), so nobody bets on a card that is not down yet.
use std::collections::VecDeque;

pub const SEATS: usize = 5;

/// A place the hand goes to, in the board's own pixels (1100 x 700; see `spot_xy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    /// Mammon's wrist, where the hand is when it is on his arm.
    Dock,
    /// In front of a person.
    Seat(usize),
    /// The pile of burnt cards, left of the board.
    Burn,
    /// One of the five places for the board cards.
    Slot(usize),
}

/// A card put down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    /// A seat's first (0) or second (1) card.
    Hole(usize, u8),
    /// The first, second or third card burnt this hand.
    Burn(u8),
    /// A board card, 0 to 4.
    Board(usize),
}

/// What the screen plays a sound for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beat {
    Shuffle,
    Land(Put),
}

/// Where the hand is: `f` of the way (eased) from `from` to `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub from: Spot,
    pub to: Spot,
    pub f: f32,
    /// A card in its fingers.
    pub carrying: bool,
    /// How hard the rocket burns: 0 at rest, 1 at full speed.
    pub thrust: f32,
}

impl Pose {
    pub const HOME: Pose = Pose { from: Spot::Dock, to: Spot::Dock, f: 0.0, carrying: false, thrust: 0.0 };
}

/// Where a spot is, in board pixels. (The dock is where the hand sits on Mammon's wrist: the screen
/// works that out from his drawing; this value only tells the schedule how far things are.)
pub fn spot_xy(s: Spot) -> (f32, f32) {
    match s {
        Spot::Dock => (289.0, 322.0),
        Spot::Seat(i) => [(550.0, 520.0), (115.0, 384.0), (470.0, 380.0), (975.0, 384.0), (715.0, 380.0)][i % SEATS],
        Spot::Burn => (312.0, 447.0),
        Spot::Slot(i) => (386.0 + 82.0 * i as f32, 447.0),
    }
}

fn dist(a: Spot, b: Spot) -> f32 {
    let (a, b) = (spot_xy(a), spot_xy(b));
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

// ---- timing, in seconds --------------------------------------------------------------------------

/// The deck is shuffled this long before the hand sets off.
const SHUFFLE_LEAD: f32 = 0.30;
/// From the wrist to the first card's place.
const LAUNCH: f32 = 0.45;
/// Back to the wrist.
const HOME: f32 = 0.55;
/// The hole cards go round fast; the board cards are put down with some care.
const HOLE_SPEED: f32 = 2100.0;
const BOARD_SPEED: f32 = 1200.0;
const HOLE_PAUSE: f32 = 0.03;
const BOARD_PAUSE: f32 = 0.16;
/// The game goes on this long after the last card is down (the hand is on its way home).
const RELEASE: f32 = 0.20;

fn ease(f: f32) -> f32 {
    let f = f.clamp(0.0, 1.0);
    f * f * (3.0 - 2.0 * f)
}

#[derive(Clone, Debug)]
struct Leg {
    from: Spot,
    to: Spot,
    t0: f32,
    t1: f32,
    carrying: bool,
}

#[derive(Clone, Debug)]
struct Trip {
    legs: Vec<Leg>,
    lands: Vec<(f32, Put)>,
    shuffle: bool,
    /// When the game may go on.
    release: f32,
    end: f32,
}

#[derive(Clone, Debug)]
struct Plan {
    puts: Vec<Put>,
    shuffle: bool,
}

fn spot_of(d: Put) -> Spot {
    match d {
        Put::Hole(seat, _) => Spot::Seat(seat),
        Put::Burn(_) => Spot::Burn,
        Put::Board(i) => Spot::Slot(i),
    }
}

fn build(plan: &Plan) -> Trip {
    let mut legs = Vec::new();
    let mut lands = Vec::new();
    let mut t = if plan.shuffle { SHUFFLE_LEAD } else { 0.0 };
    let mut at = Spot::Dock;
    for (i, &d) in plan.puts.iter().enumerate() {
        let to = spot_of(d);
        let hole = matches!(d, Put::Hole(..));
        let secs = if i == 0 {
            LAUNCH
        } else if hole {
            (dist(at, to) / HOLE_SPEED).clamp(0.12, 0.30)
        } else {
            (dist(at, to) / BOARD_SPEED).clamp(0.22, 0.40)
        };
        legs.push(Leg { from: at, to, t0: t, t1: t + secs, carrying: true });
        lands.push((t + secs, d));
        t += secs + if hole { HOLE_PAUSE } else { BOARD_PAUSE };
        at = to;
    }
    let release = lands.last().map_or(t, |l| l.0) + RELEASE;
    legs.push(Leg { from: at, to: Spot::Dock, t0: t, t1: t + HOME, carrying: false });
    Trip { legs, lands, shuffle: plan.shuffle, release, end: t + HOME }
}

impl Trip {
    fn pose(&self, clock: f32) -> Pose {
        // before the first leg the hand waits on the wrist
        let Some(first) = self.legs.first() else { return Pose::HOME };
        if clock < first.t0 {
            return Pose::HOME;
        }
        let mut cur = first;
        for leg in &self.legs {
            if leg.t0 <= clock {
                cur = leg;
            }
        }
        let f = ((clock - cur.t0) / (cur.t1 - cur.t0).max(0.001)).clamp(0.0, 1.0);
        // a hand that has put its card down has no card
        let carrying = cur.carrying && f < 1.0;
        Pose { from: cur.from, to: cur.to, f: ease(f), carrying, thrust: if f > 0.0 && f < 1.0 { (4.0 * f * (1.0 - f)).min(1.0) } else { 0.0 } }
    }
}

/// The dealing of one hand: what has been put down and what the hand is doing.
#[derive(Clone, Debug)]
pub struct Dealing {
    hand_no: u64,
    now: f32,
    clock: f32,
    trip: Option<Trip>,
    queue: VecDeque<Plan>,
    board_seen: usize,
    burns_seen: u8,
    /// Every card that has landed this hand, and when (`now`).
    landed: Vec<(Put, f32)>,
}

impl Default for Dealing {
    fn default() -> Self {
        Dealing::new()
    }
}

impl Dealing {
    pub fn new() -> Dealing {
        Dealing { hand_no: 0, now: 0.0, clock: 0.0, trip: None, queue: VecDeque::new(), board_seen: 0, burns_seen: 0, landed: Vec::new() }
    }

    pub fn hand_no(&self) -> u64 {
        self.hand_no
    }

    /// A hand begins: the deck is shuffled and a card goes to each of `dealt` (round and round, two
    /// each).
    pub fn begin_hand(&mut self, hand_no: u64, dealt: [bool; SEATS]) {
        *self = Dealing { hand_no, now: self.now, ..Dealing::new() };
        let mut puts = Vec::new();
        for nth in 0..2u8 {
            for (seat, &d) in dealt.iter().enumerate() {
                if d {
                    puts.push(Put::Hole(seat, nth));
                }
            }
        }
        self.queue.push_back(Plan { puts, shuffle: true });
    }

    /// The board has `board_len` cards now: whatever is new is put down, each street after a burn card.
    pub fn street(&mut self, board_len: usize) {
        if board_len <= self.board_seen {
            return;
        }
        let mut puts = Vec::new();
        let mut cur = self.board_seen;
        while cur < board_len.min(5) {
            puts.push(Put::Burn(self.burns_seen));
            self.burns_seen += 1;
            let upto = (if cur == 0 { 3 } else { cur + 1 }).min(board_len.min(5));
            for i in cur..upto {
                puts.push(Put::Board(i));
            }
            cur = upto;
        }
        self.board_seen = board_len.min(5);
        self.queue.push_back(Plan { puts, shuffle: false });
    }

    /// Everything is down at once (the game is being played through, or the hand is over).
    pub fn settle(&mut self, hand_no: u64, dealt: [bool; SEATS], board_len: usize) {
        let first = self.hand_no != hand_no;
        if first {
            *self = Dealing { hand_no, now: self.now, ..Dealing::new() };
        }
        self.trip = None;
        self.queue.clear();
        let board_len = board_len.min(5);
        self.board_seen = board_len;
        let long_ago = self.now - 100.0;
        let mut landed = Vec::new();
        for (seat, &d) in dealt.iter().enumerate() {
            if d {
                landed.push((Put::Hole(seat, 0), long_ago));
                landed.push((Put::Hole(seat, 1), long_ago));
            }
        }
        let burns = match board_len {
            0 => 0,
            1..=3 => 1,
            4 => 2,
            _ => 3,
        };
        for b in 0..burns {
            landed.push((Put::Burn(b), long_ago));
        }
        self.burns_seen = burns;
        for i in 0..board_len {
            landed.push((Put::Board(i), long_ago));
        }
        self.landed = landed;
    }

    /// Moves the hand on by `dt` seconds; the beats that happened are returned.
    pub fn tick(&mut self, dt: f32) -> Vec<Beat> {
        let dt = dt.clamp(0.0, 0.25);
        self.now += dt;
        let mut beats = Vec::new();
        if self.trip.is_none() {
            if let Some(plan) = self.queue.pop_front() {
                self.trip = Some(build(&plan));
                self.clock = -dt.max(0.001);
            }
        }
        let Some(trip) = &self.trip else { return beats };
        let (prev, next) = (self.clock, self.clock + dt);
        if trip.shuffle && prev < 0.0 && next >= 0.0 {
            beats.push(Beat::Shuffle);
        }
        for &(at, d) in &trip.lands {
            if at > prev && at <= next {
                beats.push(Beat::Land(d));
                self.landed.push((d, self.now - (next - at)));
            }
        }
        self.clock = next;
        if next >= trip.end {
            self.trip = None;
        }
        beats
    }

    /// The game waits: a card is still to be put down.
    pub fn busy(&self) -> bool {
        !self.queue.is_empty() || self.trip.as_ref().is_some_and(|t| self.clock < t.release)
    }

    /// Where the hand is now.
    pub fn hand(&self) -> Pose {
        self.hand_back(0.0)
    }

    /// Where the hand was `secs` ago (for the trail of the rocket).
    pub fn hand_back(&self, secs: f32) -> Pose {
        match &self.trip {
            Some(t) => t.pose(self.clock - secs),
            None => Pose::HOME,
        }
    }

    /// How long ago a card landed (None if it has not).
    pub fn age(&self, d: Put) -> Option<f32> {
        self.landed.iter().find(|(x, _)| *x == d).map(|(_, at)| self.now - at)
    }

    pub fn hole_count(&self, seat: usize) -> usize {
        (0..2u8).filter(|&n| self.age(Put::Hole(seat, n)).is_some()).count()
    }

    pub fn board_count(&self) -> usize {
        (0..5).filter(|&i| self.age(Put::Board(i)).is_some()).count()
    }

    pub fn burnt(&self) -> usize {
        (0..3u8).filter(|&n| self.age(Put::Burn(n)).is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_until_quiet(d: &mut Dealing) -> (Vec<Beat>, f32) {
        let mut beats = Vec::new();
        let mut t = 0.0;
        while d.busy() && t < 30.0 {
            beats.extend(d.tick(1.0 / 60.0));
            t += 1.0 / 60.0;
        }
        (beats, t)
    }

    const ALL: [bool; SEATS] = [true; SEATS];

    #[test]
    fn a_hand_starts_with_the_shuffle_and_deals_two_cards_round_the_table() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        assert!(d.busy(), "the game waits for the cards from the first frame");
        let (beats, secs) = run_until_quiet(&mut d);
        assert_eq!(beats[0], Beat::Shuffle);
        let lands: Vec<Put> = beats.iter().filter_map(|b| if let Beat::Land(d) = b { Some(*d) } else { None }).collect();
        assert_eq!(lands.len(), 10);
        assert_eq!(lands[0], Put::Hole(0, 0));
        assert_eq!(lands[4], Put::Hole(4, 0), "the first card goes right round before the second");
        assert_eq!(lands[5], Put::Hole(0, 1));
        assert!(secs < 3.5, "dealing a hand takes {secs}s");
    }

    #[test]
    fn only_those_in_the_hand_are_dealt_to() {
        let mut d = Dealing::new();
        d.begin_hand(3, [true, true, true, true, false]);
        let (beats, _) = run_until_quiet(&mut d);
        assert_eq!(beats.iter().filter(|b| matches!(b, Beat::Land(Put::Hole(4, _)))).count(), 0);
        assert_eq!(d.hole_count(4), 0);
        assert_eq!((0..4).map(|s| d.hole_count(s)).sum::<usize>(), 8);
    }

    #[test]
    fn the_flop_is_a_burn_card_and_three_cards_one_after_another() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        run_until_quiet(&mut d);
        d.street(3);
        assert!(d.busy());
        let mut seen: Vec<(Put, f32)> = Vec::new();
        let mut t = 0.0;
        while d.busy() {
            for b in d.tick(1.0 / 60.0) {
                if let Beat::Land(x) = b {
                    seen.push((x, t));
                }
            }
            t += 1.0 / 60.0;
        }
        let order: Vec<Put> = seen.iter().map(|s| s.0).collect();
        assert_eq!(order, vec![Put::Burn(0), Put::Board(0), Put::Board(1), Put::Board(2)]);
        for w in seen.windows(2) {
            assert!(w[1].1 - w[0].1 > 0.2, "{:?} and {:?} land together", w[0].0, w[1].0);
        }
        assert_eq!(d.board_count(), 3);
        assert_eq!(d.burnt(), 1);
    }

    #[test]
    fn the_cards_on_the_table_come_one_at_a_time() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        run_until_quiet(&mut d);
        d.street(3);
        let mut counts = Vec::new();
        while d.busy() {
            d.tick(1.0 / 60.0);
            if counts.last() != Some(&d.board_count()) {
                counts.push(d.board_count());
            }
        }
        assert_eq!(counts, vec![0, 1, 2, 3], "never three at once");
    }

    #[test]
    fn the_turn_and_the_river_each_have_their_own_burn() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        run_until_quiet(&mut d);
        d.street(3);
        run_until_quiet(&mut d);
        d.street(4);
        let (beats, _) = run_until_quiet(&mut d);
        let lands: Vec<Put> = beats.iter().filter_map(|b| if let Beat::Land(d) = b { Some(*d) } else { None }).collect();
        assert_eq!(lands, vec![Put::Burn(1), Put::Board(3)]);
        d.street(5);
        let (beats, _) = run_until_quiet(&mut d);
        let lands: Vec<Put> = beats.iter().filter_map(|b| if let Beat::Land(d) = b { Some(*d) } else { None }).collect();
        assert_eq!(lands, vec![Put::Burn(2), Put::Board(4)]);
        assert_eq!((d.board_count(), d.burnt()), (5, 3));
    }

    #[test]
    fn an_all_in_runout_is_dealt_street_by_street() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        run_until_quiet(&mut d);
        d.street(5);
        let (beats, secs) = run_until_quiet(&mut d);
        let lands: Vec<Put> = beats.iter().filter_map(|b| if let Beat::Land(d) = b { Some(*d) } else { None }).collect();
        use Put::*;
        assert_eq!(lands, vec![Burn(0), Board(0), Board(1), Board(2), Burn(1), Board(3), Burn(2), Board(4)]);
        assert!(secs < 6.0, "the runout takes {secs}s");
    }

    #[test]
    fn the_hand_leaves_the_wrist_and_comes_back() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        assert_eq!(d.hand(), Pose::HOME);
        let mut away = false;
        let mut carried = false;
        let mut burned = false;
        for _ in 0..1200 {
            d.tick(1.0 / 60.0);
            let p = d.hand();
            if p.to != Spot::Dock || p.from != Spot::Dock {
                away = true;
            }
            carried |= p.carrying;
            burned |= p.thrust > 0.9;
        }
        assert!(away && carried && burned, "it flew, with a card, on its rocket");
        assert_eq!(d.hand(), Pose::HOME, "and is back on the wrist");
        assert!(!d.busy());
    }

    #[test]
    fn the_game_is_let_go_before_the_hand_is_quite_home() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        let mut released_at = None;
        let mut home_at = None;
        let mut t = 0.0;
        for _ in 0..1200 {
            d.tick(1.0 / 60.0);
            t += 1.0 / 60.0;
            if released_at.is_none() && !d.busy() {
                released_at = Some(t);
            }
            if released_at.is_some() && home_at.is_none() && d.hand() == Pose::HOME {
                home_at = Some(t);
            }
        }
        assert!(home_at.unwrap() > released_at.unwrap() + 0.3);
    }

    #[test]
    fn a_street_that_comes_while_the_hand_is_out_waits_its_turn() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        for _ in 0..100 {
            d.tick(1.0 / 60.0);
        }
        assert!(d.hand() != Pose::HOME, "still out with the hole cards");
        d.street(3);
        d.street(4);
        assert!(d.busy());
        run_until_quiet(&mut d);
        assert_eq!((d.board_count(), d.burnt()), (4, 2));
        for _ in 0..120 {
            d.tick(1.0 / 60.0);
        }
        assert_eq!(d.hand(), Pose::HOME);
    }

    #[test]
    fn settling_puts_everything_down_at_once() {
        let mut d = Dealing::new();
        d.begin_hand(7, ALL);
        d.settle(7, ALL, 4);
        assert!(!d.busy());
        assert_eq!((d.board_count(), d.burnt(), d.hole_count(2)), (4, 2, 2));
        assert!(d.age(Put::Board(3)).unwrap() > 50.0, "long since landed, so no flip is played");
        assert_eq!(d.hand(), Pose::HOME);
    }

    #[test]
    fn a_new_hand_clears_the_last_one() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        d.settle(1, ALL, 5);
        d.begin_hand(2, ALL);
        assert_eq!((d.board_count(), d.burnt(), d.hole_count(0)), (0, 0, 0));
        assert_eq!(d.hand_no(), 2);
        d.street(0);
        assert!(d.busy());
    }

    #[test]
    fn a_landed_card_ages() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        run_until_quiet(&mut d);
        let a = d.age(Put::Hole(0, 0)).unwrap();
        d.tick(0.2);
        assert!((d.age(Put::Hole(0, 0)).unwrap() - a - 0.2).abs() < 1e-4);
        assert_eq!(d.age(Put::Board(0)), None);
    }

    #[test]
    fn the_pose_is_always_sane() {
        let mut d = Dealing::new();
        d.begin_hand(1, ALL);
        d.street(5);
        for _ in 0..1500 {
            d.tick(1.0 / 60.0);
            let p = d.hand();
            assert!((0.0..=1.0).contains(&p.f) && (0.0..=1.0).contains(&p.thrust), "{p:?}");
        }
    }
}
