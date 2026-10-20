//! The Ally of Darkness: the alley quest (logic only; the scene is painted elsewhere).
//!
//! You click the alley, the view walks down a dark circuit-board alley, and three dead lamps along
//! it each need a little circuit solved (rotate the tiles until the current runs from the battery on
//! the left to the lamp on the right). Each lamp that lights gives up one hand of the clock: the
//! hour, the minute and the second. Three hands are the key to the clock tower's door. At the dark
//! end of the alley, the Shadow Monarch's friend (with the ship) can be met, and helped or not, none
//! of which stops the key.
//!
//! Everything here is plain data and rules, tested without a window: the saved state (`State`)
//! and the puzzles (`Puzzle`, made from a seed so that they can always be solved).

/// The hands of the clock, which together are the key: hour, minute, second.
pub const HANDS: usize = 3;

/// The width and height of the puzzle at each lamp (they grow as the alley goes on).
pub const SIZES: [usize; HANDS] = [4, 5, 6];

// ------------------------------------------------------------------------------------ the state

/// The friend at the dark end of the alley.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Friend {
    /// You have not found him.
    #[default]
    Unmet,
    /// You found him, and have not done what he asked.
    Met,
    /// You did what he asked (kept for good; he is in the Void later).
    Helped,
}

impl Friend {
    fn code(self) -> u8 {
        match self {
            Friend::Unmet => 0,
            Friend::Met => 1,
            Friend::Helped => 2,
        }
    }

    fn from_code(code: u8) -> Friend {
        match code {
            1 => Friend::Met,
            2 => Friend::Helped,
            _ => Friend::Unmet,
        }
    }
}

/// Where you are in the quest (for what the cat says, and what the alley shows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// You have not been down the alley.
    Untouched,
    /// You have been in, and not every lamp is lit.
    Lamps,
    /// All three lamps are lit; the hands are not all in your pocket.
    Hands,
    /// You have the key; the door is still shut.
    Key,
    /// The tower's door is open.
    Open,
}

/// What is kept between runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    /// The lamps that are lit (one bit each, in the order of the alley).
    lit: u8,
    /// The hands that you have taken (one bit each: hour, minute, second).
    hands: u8,
    /// The friend.
    pub friend: Friend,
    /// The tower's door has been opened.
    door: bool,
    /// You have been down the alley at least once.
    entered: bool,
    /// The friend's side of it: bit 0 the warehouse key is yours, bits 1 and 2 the two jobs on his ship are done.
    extra: u8,
    /// What has been done in the alley's chain of events (one bit each; `alley::F_*`).
    flags: u32,
    /// The loose things in your pockets (one bit each; `alley::Item`). The hands and the warehouse key are
    /// kept in their own fields.
    items: u32,
    /// What this run's codes come from (0 until the first time in).
    seed: u32,
    /// Seconds spent in the alley, with the time hints cost.
    spent: u32,
}

/// The two jobs on the friend's ship (each is a bigger puzzle than the lamps).
pub const TASKS: usize = 2;
const KEY_BIT: u8 = 1;
const TASK_BITS: u8 = 0b110;

const ALL: u8 = (1 << HANDS) - 1;

impl State {
    /// You went down the alley.
    pub fn enter(&mut self) {
        self.entered = true;
    }

    /// Gives this run its codes the first time in (`n` is anything that varies, such as the clock).
    pub fn seed_with(&mut self, n: u32) {
        if self.seed == 0 {
            let mixed = n.wrapping_mul(2_654_435_761) ^ (n >> 7) ^ 0x5EED;
            self.seed = mixed % 0xFFFF_FF + 1;
        }
    }

    pub fn seed(&self) -> u32 {
        if self.seed == 0 {
            0x5EED
        } else {
            self.seed
        }
    }

    pub fn flag(&self, bit: u32) -> bool {
        self.flags & bit != 0
    }

    pub fn set_flag(&mut self, bit: u32) {
        self.flags |= bit;
        self.entered = true;
    }

    pub fn has_item(&self, bit: u32) -> bool {
        self.items & bit != 0
    }

    pub fn give_item(&mut self, bit: u32) {
        self.items |= bit;
    }

    pub fn drop_item(&mut self, bit: u32) {
        self.items &= !bit;
    }

    /// Seconds spent in the alley (hints add to it).
    pub fn spent(&self) -> u32 {
        self.spent
    }

    pub fn add_spent(&mut self, secs: u32) {
        self.spent = self.spent.saturating_add(secs).min(359_999);
    }

    pub fn is_lit(&self, lamp: usize) -> bool {
        lamp < HANDS && self.lit & (1 << lamp) != 0
    }

    pub fn has_hand(&self, hand: usize) -> bool {
        hand < HANDS && self.hands & (1 << hand) != 0
    }

    pub fn lamps_lit(&self) -> usize {
        self.lit.count_ones() as usize
    }

    pub fn hands_held(&self) -> usize {
        self.hands.count_ones() as usize
    }

    /// A lamp was lit (its puzzle was solved). Lighting one twice changes nothing.
    pub fn light(&mut self, lamp: usize) {
        if lamp < HANDS {
            self.lit |= 1 << lamp;
            self.entered = true;
        }
    }

    /// You take the hand a lit lamp gives up. False if the lamp is dark or the hand is already yours.
    pub fn take_hand(&mut self, hand: usize) -> bool {
        if !self.is_lit(hand) || self.has_hand(hand) {
            return false;
        }
        self.hands |= 1 << hand;
        true
    }

    /// All three hands are yours.
    pub fn has_key(&self) -> bool {
        self.hands == ALL
    }

    /// Opens the tower's door with the key. False without it (or if it is open already).
    pub fn open_door(&mut self) -> bool {
        if !self.has_key() || self.door {
            return false;
        }
        self.door = true;
        true
    }

    pub fn door_open(&self) -> bool {
        self.door
    }

    /// You found the key to the warehouse.
    pub fn find_warehouse_key(&mut self) {
        self.extra |= KEY_BIT;
    }

    pub fn has_warehouse_key(&self) -> bool {
        self.extra & KEY_BIT != 0
    }

    /// You opened the warehouse and found the friend (needs its key).
    pub fn meet_friend(&mut self) -> bool {
        if self.friend != Friend::Unmet || !self.has_warehouse_key() {
            return false;
        }
        self.friend = Friend::Met;
        true
    }

    /// One of the two jobs on his ship is done. False if he is not met, or it was done already.
    pub fn finish_task(&mut self, task: usize) -> bool {
        if self.friend == Friend::Unmet || task >= TASKS || self.task_done(task) {
            return false;
        }
        self.extra |= 1 << (task + 1);
        true
    }

    pub fn task_done(&self, task: usize) -> bool {
        task < TASKS && self.extra & (1 << (task + 1)) != 0
    }

    /// Both jobs are done: he is free (kept for good). Optional, and it never touches the key.
    pub fn help_friend(&mut self) -> bool {
        if self.friend != Friend::Met || self.extra & TASK_BITS != TASK_BITS {
            return false;
        }
        self.friend = Friend::Helped;
        true
    }

    /// You are out with the key and he is still down there (the Void remembers this).
    pub fn left_friend(&self) -> bool {
        self.door && self.friend != Friend::Helped
    }

    pub fn step(&self) -> Step {
        if self.door {
            Step::Open
        } else if self.has_key() {
            Step::Key
        } else if self.lit == ALL {
            Step::Hands
        } else if self.entered || self.lit != 0 {
            Step::Lamps
        } else {
            Step::Untouched
        }
    }

    /// The state as one config value: `lit:hands:friend:door:entered:extra`.
    pub fn encode(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.lit,
            self.hands,
            self.friend.code(),
            self.door as u8,
            self.entered as u8,
            self.extra,
            self.flags,
            self.items,
            self.seed,
            self.spent
        )
    }

    /// Reads `encode`'s text. Anything wrong gives a state that is possible (hands only for lit lamps,
    /// a door only with the key), so a hand-edited file cannot make an impossible quest.
    pub fn decode(text: &str) -> State {
        let mut parts = text.trim().split(':').map(|p| p.trim().parse::<u8>().unwrap_or(0));
        let mut next = || parts.next().unwrap_or(0);
        let lit = next() & ALL;
        let hands = next() & ALL & lit;
        let friend = Friend::from_code(next());
        let door = next() == 1;
        let entered = next() == 1;
        let extra = next() & (KEY_BIT | TASK_BITS);
        let mut big = text.trim().split(':').skip(6).map(|p| p.trim().parse::<u32>().unwrap_or(0));
        let flags = big.next().unwrap_or(0) & 0xFFFF;
        let items = big.next().unwrap_or(0) & 0xFFFF;
        let seed = big.next().unwrap_or(0) & 0xFF_FFFF;
        let spent = big.next().unwrap_or(0).min(359_999);
        let mut state = State { lit, hands, friend, door, entered, extra, flags, items, seed, spent };
        if !state.has_key() {
            state.door = false;
        }
        // the friend needs the warehouse key, and being freed needs both jobs
        if !state.has_warehouse_key() {
            state.friend = Friend::Unmet;
        }
        if state.friend == Friend::Unmet {
            state.extra &= KEY_BIT;
        }
        if state.friend == Friend::Helped && state.extra & TASK_BITS != TASK_BITS {
            state.friend = Friend::Met;
        }
        if state.lit != 0 || state.door {
            state.entered = true;
        }
        state
    }

    /// Forget the quest (a test button).
    pub fn reset(&mut self) {
        *self = State::default();
    }
}

// ------------------------------------------------------------------------------------ the puzzle

/// The side of a tile: 0 up, 1 right, 2 down, 3 left.
pub type Side = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Empty,
    /// Through: up and down at rotation 0.
    Straight,
    /// A bend: up and right at rotation 0.
    Corner,
    /// A branch: up, right and left at rotation 0.
    Tee,
    Cross,
}

impl Kind {
    fn base(self) -> [bool; 4] {
        match self {
            Kind::Empty => [false; 4],
            Kind::Straight => [true, false, true, false],
            Kind::Corner => [true, true, false, false],
            Kind::Tee => [true, true, false, true],
            Kind::Cross => [true; 4],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub kind: Kind,
    /// Quarter turns clockwise (0 to 3).
    pub turns: u8,
}

impl Tile {
    /// Whether the tile is open on `side`.
    pub fn open(&self, side: Side) -> bool {
        self.kind.base()[(side + 4 - (self.turns as usize % 4)) % 4]
    }
}

/// A square of tiles with a battery at the left of the cell `source` (on the left edge) and the lamp at
/// the right of the cell `lamp` (on the right edge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub size: usize,
    pub tiles: Vec<Tile>,
    /// The row of the battery (left edge).
    pub source: usize,
    /// The row of the lamp (right edge).
    pub lamp: usize,
}

/// A small deterministic random source (the same seed always gives the same alley).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03 | 1)
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

impl Puzzle {
    /// A puzzle of `size` by `size` that can be solved, scrambled so that it is not solved yet.
    pub fn make(size: usize, seed: u64) -> Puzzle {
        Puzzle::make_with_answer(size, seed).0
    }

    /// The same puzzle as `make`, and the puzzle as it is when solved (the same pieces, turned right).
    pub fn make_with_answer(size: usize, seed: u64) -> (Puzzle, Puzzle) {
        let size = size.clamp(3, 8);
        let mut rng = Rng::new(seed ^ (size as u64) << 32);
        let mut tiles = vec![Tile { kind: Kind::Empty, turns: 0 }; size * size];
        let at = |row: usize, col: usize| row * size + col;
        // a walk from the left edge to the right edge: right, or up and down without going back on itself
        let source = rng.below(size);
        let mut path: Vec<(usize, usize)> = vec![(source, 0)];
        let mut seen = vec![false; size * size];
        seen[at(source, 0)] = true;
        let mut guard = 0;
        while path.last().map(|p| p.1) != Some(size - 1) && guard < 500 {
            guard += 1;
            let (row, col) = *path.last().unwrap_or(&(source, 0));
            let mut options: Vec<(usize, usize)> = vec![(row, col + 1), (row, col + 1)];
            if row > 0 && !seen[at(row - 1, col)] {
                options.push((row - 1, col));
            }
            if row + 1 < size && !seen[at(row + 1, col)] {
                options.push((row + 1, col));
            }
            let (r, c) = options[rng.below(options.len())];
            if !seen[at(r, c)] {
                seen[at(r, c)] = true;
                path.push((r, c));
            }
        }
        let lamp = path.last().map(|p| p.0).unwrap_or(source);
        // each cell of the path gets the piece that joins the side it is entered from and the side it leaves by
        for (i, &(row, col)) in path.iter().enumerate() {
            let came = match i.checked_sub(1).map(|j| path[j]) {
                None => 3, // the battery, on the left
                Some((pr, pc)) => direction((row, col), (pr, pc)),
            };
            let goes = match path.get(i + 1) {
                None => 1, // the lamp, on the right
                Some(&(nr, nc)) => direction((row, col), (nr, nc)),
            };
            tiles[at(row, col)] = joining(came, goes);
        }
        // the rest are decoys
        for (i, tile) in tiles.iter_mut().enumerate() {
            if !seen[i] {
                let kind = [Kind::Empty, Kind::Straight, Kind::Corner, Kind::Corner, Kind::Tee]
                    [rng.below(5)];
                *tile = Tile { kind, turns: rng.below(4) as u8 };
            }
        }
        let answer = Puzzle { size, tiles: tiles.clone(), source, lamp };
        // scramble every piece that can be turned
        for tile in tiles.iter_mut() {
            if tile.kind != Kind::Empty && tile.kind != Kind::Cross {
                tile.turns = rng.below(4) as u8;
            }
        }
        let mut puzzle = Puzzle { size, tiles, source, lamp };
        // never start solved: turn pieces of the path until it is not
        let mut i = 0;
        while puzzle.powered() && i < path.len() {
            let (row, col) = path[i];
            puzzle.turn(row, col);
            i += 1;
        }
        (puzzle, answer)
    }

    pub fn tile(&self, row: usize, col: usize) -> Option<&Tile> {
        (row < self.size && col < self.size).then(|| &self.tiles[row * self.size + col])
    }

    /// A quarter turn clockwise for the tile at `row`, `col`.
    pub fn turn(&mut self, row: usize, col: usize) {
        if row < self.size && col < self.size {
            let tile = &mut self.tiles[row * self.size + col];
            tile.turns = (tile.turns + 1) % 4;
        }
    }

    /// Which cells the current reaches from the battery.
    pub fn live(&self) -> Vec<bool> {
        let n = self.size;
        let mut live = vec![false; n * n];
        let start = self.source * n;
        if !self.tiles[start].open(3) {
            return live;
        }
        live[start] = true;
        let mut stack = vec![(self.source, 0usize)];
        while let Some((row, col)) = stack.pop() {
            let tile = self.tiles[row * n + col];
            for side in 0..4 {
                if !tile.open(side) {
                    continue;
                }
                let next = match side {
                    0 if row > 0 => (row - 1, col),
                    1 if col + 1 < n => (row, col + 1),
                    2 if row + 1 < n => (row + 1, col),
                    3 if col > 0 => (row, col - 1),
                    _ => continue,
                };
                let there = self.tiles[next.0 * n + next.1];
                // the current only passes where the neighbour is open towards us
                if there.open((side + 2) % 4) && !live[next.0 * n + next.1] {
                    live[next.0 * n + next.1] = true;
                    stack.push(next);
                }
            }
        }
        live
    }

    /// The lamp is lit: the current reaches its cell and leaves by its right side.
    pub fn powered(&self) -> bool {
        let n = self.size;
        let cell = self.lamp * n + n - 1;
        self.live()[cell] && self.tiles[cell].open(1)
    }
}

/// The side of `from` on which `to` lies (the two are neighbours).
fn direction(from: (usize, usize), to: (usize, usize)) -> usize {
    if to.0 < from.0 {
        0
    } else if to.1 > from.1 {
        1
    } else if to.0 > from.0 {
        2
    } else {
        3
    }
}

/// The tile (kind and turns) that is open on exactly sides `a` and `b`.
fn joining(a: usize, b: usize) -> Tile {
    for kind in [Kind::Straight, Kind::Corner] {
        for turns in 0..4u8 {
            let tile = Tile { kind, turns };
            let open: Vec<usize> = (0..4).filter(|s| tile.open(*s)).collect();
            if open.len() == 2 && open.contains(&a) && open.contains(&b) {
                return tile;
            }
        }
    }
    Tile { kind: Kind::Cross, turns: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_is_open_where_its_kind_and_turns_say() {
        let corner = Tile { kind: Kind::Corner, turns: 0 };
        assert!(corner.open(0) && corner.open(1) && !corner.open(2) && !corner.open(3));
        let turned = Tile { kind: Kind::Corner, turns: 1 };
        assert!(turned.open(1) && turned.open(2) && !turned.open(0) && !turned.open(3));
        let straight = Tile { kind: Kind::Straight, turns: 1 };
        assert!(straight.open(1) && straight.open(3) && !straight.open(0));
        assert!(Tile { kind: Kind::Cross, turns: 2 }.open(0));
        assert!(!Tile { kind: Kind::Empty, turns: 3 }.open(0));
    }

    #[test]
    fn a_made_puzzle_is_never_solved_at_first() {
        for size in SIZES {
            for seed in 0..60 {
                let puzzle = Puzzle::make(size, seed);
                assert_eq!(puzzle.tiles.len(), size * size);
                assert!(!puzzle.powered(), "size {size} seed {seed} started solved");
            }
        }
    }

    #[test]
    fn every_made_puzzle_can_be_solved_by_turning_its_pieces() {
        for size in SIZES {
            for seed in 0..80 {
                let (mut puzzle, answer) = Puzzle::make_with_answer(size, seed);
                assert!(answer.powered(), "size {size} seed {seed}: the answer is not lit");
                for (tile, right) in puzzle.tiles.iter_mut().zip(&answer.tiles) {
                    assert_eq!(tile.kind, right.kind, "turning never changes a piece");
                    // turn it quarter by quarter, as a player would
                    while tile.turns != right.turns {
                        tile.turns = (tile.turns + 1) % 4;
                    }
                }
                assert!(puzzle.powered(), "size {size} seed {seed} could not be solved");
            }
        }
    }

    #[test]
    fn a_player_can_light_the_lamp_by_turning_the_path_one_tile_at_a_time() {
        let (mut puzzle, answer) = Puzzle::make_with_answer(5, 3);
        let mut turns = 0;
        for row in 0..5 {
            for col in 0..5 {
                while puzzle.tile(row, col) != answer.tile(row, col) {
                    puzzle.turn(row, col);
                    turns += 1;
                    assert!(turns < 200);
                }
            }
        }
        assert!(puzzle.powered());
    }

    #[test]
    fn the_same_seed_gives_the_same_puzzle_and_another_gives_another() {
        assert_eq!(Puzzle::make(5, 7), Puzzle::make(5, 7));
        assert_ne!(Puzzle::make(5, 7), Puzzle::make(5, 8));
    }

    #[test]
    fn current_only_passes_between_tiles_that_are_open_to_each_other() {
        // a straight wire along a row of straights, with the lamp at the end
        let n = 3;
        let wire = Tile { kind: Kind::Straight, turns: 1 };
        let mut tiles = vec![Tile { kind: Kind::Empty, turns: 0 }; n * n];
        for col in 0..n {
            tiles[n + col] = wire;
        }
        let mut puzzle = Puzzle { size: n, tiles, source: 1, lamp: 1 };
        assert!(puzzle.powered());
        // turn the middle one upright: the wire is cut
        puzzle.turn(1, 1);
        assert!(!puzzle.powered());
        puzzle.turn(1, 1);
        puzzle.turn(1, 1);
        puzzle.turn(1, 1);
        assert!(puzzle.powered());
        // the battery is on the left of the first cell: a first tile closed on the left gets nothing
        puzzle.turn(1, 0);
        assert!(!puzzle.powered());
        assert!(puzzle.live().iter().all(|c| !c));
    }

    #[test]
    fn the_lamp_needs_its_own_cell_open_to_the_right() {
        let n = 3;
        let mut tiles = vec![Tile { kind: Kind::Empty, turns: 0 }; n * n];
        for col in 0..n - 1 {
            tiles[col] = Tile { kind: Kind::Straight, turns: 1 };
        }
        // the last cell is a bend that is reached but leaves upwards: not the lamp
        tiles[n - 1] = Tile { kind: Kind::Corner, turns: 3 };
        let puzzle = Puzzle { size: n, tiles, source: 0, lamp: 0 };
        assert!(puzzle.live()[n - 1]);
        assert!(!puzzle.powered());
    }

    #[test]
    fn the_quest_runs_in_order() {
        let mut s = State::default();
        assert_eq!(s.step(), Step::Untouched);
        s.enter();
        assert_eq!(s.step(), Step::Lamps);
        // a dark lamp gives nothing
        assert!(!s.take_hand(0));
        for lamp in 0..HANDS {
            s.light(lamp);
            assert!(s.is_lit(lamp));
            assert!(s.take_hand(lamp));
            assert!(!s.take_hand(lamp), "a hand is taken once");
        }
        assert!(s.has_key());
        assert_eq!(s.lamps_lit(), 3);
        assert_eq!(s.hands_held(), 3);
        assert_eq!(s.step(), Step::Key);
        assert!(s.open_door());
        assert!(!s.open_door());
        assert!(s.door_open());
        assert_eq!(s.step(), Step::Open);
    }

    #[test]
    fn the_door_needs_all_three_hands() {
        let mut s = State::default();
        s.light(0);
        s.light(1);
        s.take_hand(0);
        s.take_hand(1);
        assert!(!s.has_key());
        assert!(!s.open_door());
        assert_eq!(s.step(), Step::Lamps);
        s.light(2);
        assert_eq!(s.step(), Step::Hands);
        s.take_hand(2);
        assert!(s.open_door());
    }

    #[test]
    fn the_friend_is_optional_and_never_blocks_the_key() {
        let mut s = State::default();
        assert!(!s.meet_friend(), "the warehouse key first");
        assert!(!s.finish_task(0), "no jobs on a ship you have not found");
        s.find_warehouse_key();
        assert!(!s.help_friend(), "you cannot help someone you have not met");
        assert!(s.meet_friend());
        assert!(!s.meet_friend());
        assert_eq!(s.friend, Friend::Met);
        // he is not free until both jobs are done
        assert!(s.finish_task(0));
        assert!(!s.finish_task(0));
        assert!(!s.help_friend());
        assert!(s.finish_task(1) && !s.finish_task(2));
        // the key and the door do not need him
        for lamp in 0..HANDS {
            s.light(lamp);
            s.take_hand(lamp);
        }
        assert!(s.open_door());
        assert!(s.left_friend());
        assert!(s.help_friend());
        assert_eq!(s.friend, Friend::Helped);
        assert!(!s.help_friend());
        assert!(!s.left_friend());
        // and a player who never meets him still gets through, and leaves him behind
        let mut other = State::default();
        for lamp in 0..HANDS {
            other.light(lamp);
            other.take_hand(lamp);
        }
        assert!(other.open_door());
        assert_eq!(other.friend, Friend::Unmet);
        assert!(other.left_friend());
    }

    #[test]
    fn the_state_is_kept_and_read_back_and_bad_text_gives_a_possible_state() {
        let mut s = State::default();
        for lamp in 0..HANDS {
            s.light(lamp);
            s.take_hand(lamp);
        }
        s.find_warehouse_key();
        s.meet_friend();
        s.finish_task(0);
        s.finish_task(1);
        s.help_friend();
        s.open_door();
        let loaded = State::decode(&s.encode());
        assert_eq!(loaded, s);
        assert_eq!(State::decode(""), State::default());
        assert_eq!(State::decode("what is this"), State::default());
        // a door with no hands, a friend with dark lamps, hands without lamps: all undone
        let odd = State::decode("0:7:2:1:0:6");
        assert_eq!(odd.hands_held(), 0);
        assert!(!odd.door_open());
        assert_eq!(odd.friend, Friend::Unmet);
        // freed without the jobs: only met
        assert_eq!(State::decode("7:7:2:1:1:1").friend, Friend::Met);
        let part = State::decode("1:1:2:1:0");
        assert!(part.is_lit(0) && part.has_hand(0) && !part.door_open());
        assert_eq!(part.step(), Step::Lamps);
        let mut reset = loaded;
        reset.reset();
        assert_eq!(reset, State::default());
    }
}
