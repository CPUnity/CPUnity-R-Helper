#![allow(dead_code)]
//! The rooms of the Ally of Darkness: an escape-the-room game in a warped alley (logic only).
//!
//! Time has folded the alley in on itself. There are four rooms (the mouth, the yard, the row and
//! the friend's warehouse), each can be seen in three times (past, now, future), and what you can
//! click depends on the time you are looking at. You start in the present with no way to shift: the
//! first lamp gives up the hour hand, and with it you can look into the past; the second gives up
//! the minute hand, and with it the future. The third lamp gives up the second hand. Three hands
//! open the hatch in the floor of the mouth: the way out, into the drain that comes up at the foot
//! of the clock tower.
//!
//! The warehouse is optional and harder: its key is hidden in the yard's past, and inside is the
//! friend and the wreck of his ship, with two jobs (two bigger puzzles) to free him.
//!
//! Everything here is plain rules on top of `ally::State`; the painter is in `ui/ally.rs`.

use crate::ally::{Puzzle, State, HANDS, SIZES, TASKS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    Mouth,
    Yard,
    Row,
    Warehouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Era {
    Past,
    Now,
    Future,
}

impl Era {
    pub const ALL: [Era; 3] = [Era::Past, Era::Now, Era::Future];

    /// The hand of the clock that lets you look at this time (hour: past, minute: future).
    pub fn hand(self) -> Option<usize> {
        match self {
            Era::Past => Some(0),
            Era::Now => None,
            Era::Future => Some(1),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Era::Past => "Past",
            Era::Now => "Now",
            Era::Future => "Future",
        }
    }

    pub fn available(self, state: &State) -> bool {
        self.hand().map_or(true, |h| state.has_hand(h))
    }
}

/// A puzzle you can be asked to solve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PuzzleId {
    /// A lamp's circuit (0 to 2).
    Lamp(usize),
    /// A job on the friend's ship (0 or 1).
    Job(usize),
}

impl PuzzleId {
    pub fn puzzle(self) -> Puzzle {
        match self {
            PuzzleId::Lamp(i) => Puzzle::make(SIZES[i.min(HANDS - 1)], 100 + i as u64),
            PuzzleId::Job(i) => Puzzle::make(7 + i.min(1), 200 + i as u64),
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            PuzzleId::Lamp(0) => "The dead lamp",
            PuzzleId::Lamp(1) => "The yard's fuse box",
            PuzzleId::Lamp(_) => "The row's switchboard",
            PuzzleId::Job(0) => "The ship's power line",
            PuzzleId::Job(_) => "The ship's drive coil",
        }
    }
}

/// What a click on a hotspot does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Go(Room),
    /// The black archway: the cat.
    Talk,
    Lamp(usize),
    /// The hatch in the floor.
    Hatch,
    /// The crate with the warehouse key in it.
    Crate,
    /// The warehouse's rolling shutter.
    Shutter,
    Friend,
    Job(usize),
}

/// A place you can click: a rectangle (x0, y0, x1, y1, as fractions of the room) that is there in `era`
/// (`None`: in every time).
pub struct Hotspot {
    pub room: Room,
    pub era: Option<Era>,
    pub rect: [f32; 4],
    pub act: Act,
}

const fn spot(room: Room, era: Option<Era>, rect: [f32; 4], act: Act) -> Hotspot {
    Hotspot { room, era, rect, act }
}

pub const HOTSPOTS: &[Hotspot] = &[
    // the mouth: the archway with the cat, the dead lamp, the hatch in the floor, the way on
    spot(Room::Mouth, None, [0.44, 0.30, 0.56, 0.74], Act::Talk),
    spot(Room::Mouth, Some(Era::Now), [0.18, 0.36, 0.32, 0.62], Act::Lamp(0)),
    spot(Room::Mouth, None, [0.30, 0.84, 0.70, 0.98], Act::Hatch),
    spot(Room::Mouth, None, [0.92, 0.30, 1.00, 0.82], Act::Go(Room::Yard)),
    // the yard: a fuse box and a crate that are only there in the past
    spot(Room::Yard, None, [0.00, 0.30, 0.08, 0.82], Act::Go(Room::Mouth)),
    spot(Room::Yard, None, [0.92, 0.30, 1.00, 0.82], Act::Go(Room::Row)),
    spot(Room::Yard, Some(Era::Past), [0.42, 0.26, 0.58, 0.52], Act::Lamp(1)),
    spot(Room::Yard, Some(Era::Past), [0.14, 0.62, 0.30, 0.82], Act::Crate),
    // the row: a switchboard that only works in the future, and the warehouse shutter
    spot(Room::Row, None, [0.00, 0.30, 0.08, 0.82], Act::Go(Room::Yard)),
    spot(Room::Row, Some(Era::Future), [0.14, 0.26, 0.30, 0.54], Act::Lamp(2)),
    spot(Room::Row, Some(Era::Now), [0.52, 0.18, 0.90, 0.82], Act::Shutter),
    // the warehouse: the friend, and the two jobs on his ship
    spot(Room::Warehouse, None, [0.00, 0.40, 0.08, 0.90], Act::Go(Room::Row)),
    spot(Room::Warehouse, Some(Era::Now), [0.40, 0.22, 0.58, 0.84], Act::Friend),
    spot(Room::Warehouse, Some(Era::Now), [0.12, 0.34, 0.28, 0.62], Act::Job(0)),
    spot(Room::Warehouse, Some(Era::Now), [0.70, 0.34, 0.88, 0.62], Act::Job(1)),
];

/// Where you are in the game (not saved: you start at the mouth, in the present, each time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub room: Room,
    pub era: Era,
}

impl Default for Run {
    fn default() -> Run {
        Run { room: Room::Mouth, era: Era::Now }
    }
}

/// What a click did, for the painter to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Nothing there.
    Nothing,
    /// You went to another room.
    Moved,
    /// A line to show.
    Say(&'static str),
    /// The cat's talk opens.
    Talk,
    /// The friend's talk opens.
    Friend,
    /// A puzzle opens.
    Puzzle(PuzzleId),
    /// The hatch is open: you are out.
    Escaped,
}

impl Run {
    /// The act at a point (fractions of the room), if there is one in the time being looked at.
    pub fn act_at(&self, x: f32, y: f32) -> Option<Act> {
        HOTSPOTS
            .iter()
            .rev()
            .find(|h| {
                h.room == self.room
                    && h.era.map_or(true, |e| e == self.era)
                    && x >= h.rect[0]
                    && x <= h.rect[2]
                    && y >= h.rect[1]
                    && y <= h.rect[3]
            })
            .map(|h| h.act)
    }

    /// Looks at another time. False when you have not got the hand for it.
    pub fn shift(&mut self, era: Era, state: &State) -> bool {
        if !era.available(state) {
            return false;
        }
        self.era = era;
        true
    }

    /// A click at a point in the room.
    pub fn click(&mut self, state: &mut State, x: f32, y: f32) -> Event {
        let Some(act) = self.act_at(x, y) else {
            return Event::Nothing;
        };
        match act {
            Act::Go(room) => {
                self.room = room;
                Event::Moved
            }
            Act::Talk => Event::Talk,
            Act::Lamp(i) => {
                if state.is_lit(i) {
                    Event::Say(
                        "The lamp burns, steady and warm. It is the only thing here that does.",
                    )
                } else {
                    Event::Puzzle(PuzzleId::Lamp(i))
                }
            }
            Act::Hatch => {
                if state.open_door() || state.door_open() {
                    Event::Escaped
                } else {
                    Event::Say("A hatch in the floor. The lock has three notches, shaped like the hands of a clock.")
                }
            }
            Act::Crate => {
                if state.has_warehouse_key() {
                    Event::Say("The crate is empty now.")
                } else {
                    state.find_warehouse_key();
                    Event::Say("Under the rotten boards: a heavy iron key, tagged WAREHOUSE.")
                }
            }
            Act::Shutter => {
                if !state.has_warehouse_key() {
                    Event::Say("A rolling shutter with a padlock the size of a fist. Something is humming behind it.")
                } else {
                    state.meet_friend();
                    self.room = Room::Warehouse;
                    Event::Moved
                }
            }
            Act::Friend => Event::Friend,
            Act::Job(i) => {
                if state.task_done(i) {
                    Event::Say("It hums. That part of the ship is alive again.")
                } else {
                    Event::Puzzle(PuzzleId::Job(i))
                }
            }
        }
    }
}

/// A puzzle was solved: what changes, and what to say.
pub fn solved(state: &mut State, id: PuzzleId) -> &'static str {
    match id {
        PuzzleId::Lamp(i) => {
            state.light(i);
            state.take_hand(i);
            match i {
                0 => "The lamp wakes. Something drops out of the dark: the hour hand. Hold it up, and you can look into the Past.",
                1 => "The fuse box clacks and the lamp catches. The minute hand falls into your palm. Now you can look into the Future.",
                _ => "The last lamp lights and the second hand ticks into your hand. Three hands. Somewhere, a hatch is listening.",
            }
        }
        PuzzleId::Job(i) => {
            state.finish_task(i);
            if state.help_friend() {
                "The ship's lights come up all at once. He is free."
            } else {
                "One part of the ship comes alive. There is more to do."
            }
        }
    }
}

// ----------------------------------------------------------------------------- the cat in the archway
// (All of this is a placeholder in his voice: the lines are Trey's to write.)

/// What you can ask him.
pub const TOPICS: [&str; 5] = [
    "Where are we?",
    "What is happening?",
    "Why does it keep looping?",
    "How do I get out?",
    "What is that humming?",
];

/// Whether a question is there yet (`asked`: one bit for each question already asked).
pub fn topic_open(topic: usize, asked: u8) -> bool {
    match topic {
        0..=1 | 3 => true,
        2 => asked & 0b11 == 0b11,
        4 => asked & 0b100 != 0,
        _ => false,
    }
}

/// What he says to a question.
pub fn cat_reply(topic: usize, state: &State) -> &'static str {
    match topic {
        0 => "The alley. Or what Time has made of it. Don't touch anything that ticks. ...Touch the lamps. The lamps are fine.",
        1 => "It folded. The alley, I mean. Time folded it in on itself, and you with it. I'm calm. See how calm I am?",
        2 => "Time hates a loose end, and you and I are loose ends. It keeps what it cannot finish. Every way forward is also a way back. Except one.",
        3 => match state.step() {
            crate::ally::Step::Key | crate::ally::Step::Open => "You have all three hands. The hatch in the floor, at my feet. Go on. I will be right behind you. Eventually.",
            _ if state.has_hand(1) => "One more. The minute hand shows what this place will be. Look at the row as it will be.",
            _ if state.has_hand(0) => "The hour hand shows what this place was. Look at the yard as it was.",
            _ => "Three lamps, and each one holds a hand of the clock. Start with the dead one beside me.",
        },
        4 => "Something is humming behind a shutter in the row. I did not ask. You did not hear it from me.",
        _ => "...",
    }
}

/// What the friend says (a placeholder, until Trey writes him).
pub fn friend_line(state: &State) -> &'static str {
    use crate::ally::Friend;
    match state.friend {
        Friend::Helped => "...Free. Go. I will find you where the dark is thickest.",
        _ if state.task_done(0) || state.task_done(1) => "Half of her is awake. Keep going. Please.",
        _ => "...Is that a way out? The ship is dead in the water. Wake her up, and she may carry us both.",
    }
}

/// How many jobs on the ship (for the painter).
pub const SHIP_JOBS: usize = TASKS;

#[cfg(test)]
mod tests {
    use super::*;

    fn centre(room: Room, era: Option<Era>, act: Act) -> (f32, f32) {
        let h = HOTSPOTS
            .iter()
            .find(|h| h.room == room && h.era == era && h.act == act)
            .expect("hotspot");
        ((h.rect[0] + h.rect[2]) / 2.0, (h.rect[1] + h.rect[3]) / 2.0)
    }

    fn click(run: &mut Run, state: &mut State, room: Room, era: Option<Era>, act: Act) -> Event {
        assert_eq!(run.room, room, "you are in the wrong room for {act:?}");
        let (x, y) = centre(room, era, act);
        run.click(state, x, y)
    }

    #[test]
    fn every_hotspot_is_inside_its_room_and_findable() {
        for h in HOTSPOTS {
            assert!(
                h.rect[0] >= 0.0 && h.rect[1] >= 0.0 && h.rect[2] <= 1.0 && h.rect[3] <= 1.0,
                "{:?}",
                h.act
            );
            assert!(h.rect[0] < h.rect[2] && h.rect[1] < h.rect[3]);
            // the middle of it is clicked as itself (nothing in the same room and time sits on top of it)
            let mut run = Run { room: h.room, era: h.era.unwrap_or(Era::Now) };
            let (x, y) = ((h.rect[0] + h.rect[2]) / 2.0, (h.rect[1] + h.rect[3]) / 2.0);
            run.era = h.era.unwrap_or(Era::Now);
            assert_eq!(run.act_at(x, y), Some(h.act), "{:?} in {:?}", h.act, h.room);
        }
    }

    #[test]
    fn you_cannot_look_at_a_time_without_its_hand() {
        let mut state = State::default();
        let mut run = Run::default();
        assert!(!run.shift(Era::Past, &state) && !run.shift(Era::Future, &state));
        assert_eq!(run.era, Era::Now);
        assert!(run.shift(Era::Now, &state));
        state.light(0);
        state.take_hand(0);
        assert!(run.shift(Era::Past, &state) && run.era == Era::Past);
        assert!(!run.shift(Era::Future, &state));
        state.light(1);
        state.take_hand(1);
        assert!(run.shift(Era::Future, &state));
    }

    #[test]
    fn what_you_can_click_depends_on_the_time() {
        let mut run = Run { room: Room::Yard, era: Era::Now };
        let (x, y) = centre(Room::Yard, Some(Era::Past), Act::Lamp(1));
        assert_eq!(run.act_at(x, y), None, "the fuse box is only there in the past");
        run.era = Era::Past;
        assert_eq!(run.act_at(x, y), Some(Act::Lamp(1)));
    }

    #[test]
    fn the_way_out_takes_the_three_hands_in_order() {
        let mut state = State::default();
        let mut run = Run::default();
        // the hatch is locked
        assert!(matches!(
            click(&mut run, &mut state, Room::Mouth, None, Act::Hatch),
            Event::Say(_)
        ));
        // the lamp at the mouth gives the hour hand
        assert_eq!(
            click(&mut run, &mut state, Room::Mouth, Some(Era::Now), Act::Lamp(0)),
            Event::Puzzle(PuzzleId::Lamp(0))
        );
        solved(&mut state, PuzzleId::Lamp(0));
        assert!(state.has_hand(0) && !state.has_key());
        assert!(
            matches!(
                click(&mut run, &mut state, Room::Mouth, Some(Era::Now), Act::Lamp(0)),
                Event::Say(_)
            ),
            "a lit lamp is not a puzzle again"
        );
        // the yard, in the past: the fuse box gives the minute hand
        assert_eq!(
            click(&mut run, &mut state, Room::Mouth, None, Act::Go(Room::Yard)),
            Event::Moved
        );
        assert!(run.shift(Era::Past, &state));
        assert_eq!(
            click(&mut run, &mut state, Room::Yard, Some(Era::Past), Act::Lamp(1)),
            Event::Puzzle(PuzzleId::Lamp(1))
        );
        solved(&mut state, PuzzleId::Lamp(1));
        // the row, in the future: the switchboard gives the second hand
        assert_eq!(click(&mut run, &mut state, Room::Yard, None, Act::Go(Room::Row)), Event::Moved);
        assert!(run.shift(Era::Future, &state));
        assert_eq!(
            click(&mut run, &mut state, Room::Row, Some(Era::Future), Act::Lamp(2)),
            Event::Puzzle(PuzzleId::Lamp(2))
        );
        solved(&mut state, PuzzleId::Lamp(2));
        assert!(state.has_key());
        // back to the mouth, and the hatch opens
        assert_eq!(click(&mut run, &mut state, Room::Row, None, Act::Go(Room::Yard)), Event::Moved);
        assert_eq!(
            click(&mut run, &mut state, Room::Yard, None, Act::Go(Room::Mouth)),
            Event::Moved
        );
        assert_eq!(click(&mut run, &mut state, Room::Mouth, None, Act::Hatch), Event::Escaped);
        assert!(state.door_open());
        // the friend was never needed
        assert!(state.left_friend());
    }

    #[test]
    fn the_friend_is_harder_and_optional() {
        let mut state = State::default();
        state.light(0);
        state.take_hand(0);
        let mut run = Run { room: Room::Yard, era: Era::Past };
        // the shutter is shut without the key
        let mut row = Run { room: Room::Row, era: Era::Now };
        assert!(matches!(
            click(&mut row, &mut state, Room::Row, Some(Era::Now), Act::Shutter),
            Event::Say(_)
        ));
        assert_eq!(row.room, Room::Row);
        // the key is in the yard's past, in a crate
        assert!(matches!(
            click(&mut run, &mut state, Room::Yard, Some(Era::Past), Act::Crate),
            Event::Say(_)
        ));
        assert!(state.has_warehouse_key());
        assert!(matches!(
            click(&mut run, &mut state, Room::Yard, Some(Era::Past), Act::Crate),
            Event::Say(_)
        ));
        // now the shutter opens onto the warehouse and the friend
        assert_eq!(
            click(&mut row, &mut state, Room::Row, Some(Era::Now), Act::Shutter),
            Event::Moved
        );
        assert_eq!(row.room, Room::Warehouse);
        assert_eq!(state.friend, crate::ally::Friend::Met);
        let mut inside = Run { room: Room::Warehouse, era: Era::Now };
        assert_eq!(
            click(&mut inside, &mut state, Room::Warehouse, Some(Era::Now), Act::Friend),
            Event::Friend
        );
        // two jobs, each a bigger puzzle than any lamp
        assert_eq!(
            click(&mut inside, &mut state, Room::Warehouse, Some(Era::Now), Act::Job(0)),
            Event::Puzzle(PuzzleId::Job(0))
        );
        assert!(PuzzleId::Job(0).puzzle().size > *SIZES.last().unwrap());
        assert!(!solved(&mut state, PuzzleId::Job(0)).contains("free"));
        assert_ne!(state.friend, crate::ally::Friend::Helped);
        assert!(matches!(
            click(&mut inside, &mut state, Room::Warehouse, Some(Era::Now), Act::Job(0)),
            Event::Say(_)
        ));
        assert!(solved(&mut state, PuzzleId::Job(1)).contains("free"));
        assert_eq!(state.friend, crate::ally::Friend::Helped);
        // and none of it gave a hand: the key was never touched
        assert_eq!(state.hands_held(), 1);
    }

    #[test]
    fn every_puzzle_in_the_rooms_is_made_and_never_starts_lit() {
        for i in 0..HANDS {
            assert!(!PuzzleId::Lamp(i).puzzle().powered());
        }
        for i in 0..TASKS {
            let puzzle = PuzzleId::Job(i).puzzle();
            assert!(!puzzle.powered());
            assert!(puzzle.size >= 7);
        }
    }

    #[test]
    fn the_cat_opens_up_as_you_ask() {
        assert!(topic_open(0, 0) && topic_open(1, 0) && topic_open(3, 0));
        assert!(!topic_open(2, 0b01) && topic_open(2, 0b11));
        assert!(!topic_open(4, 0b011) && topic_open(4, 0b111));
        assert!(!topic_open(9, 0xff));
        let mut state = State::default();
        for t in 0..TOPICS.len() {
            assert!(!cat_reply(t, &state).is_empty());
        }
        // his advice follows how far you are
        let start = cat_reply(3, &state);
        state.light(0);
        state.take_hand(0);
        assert_ne!(cat_reply(3, &state), start);
        state.light(1);
        state.take_hand(1);
        let two = cat_reply(3, &state);
        state.light(2);
        state.take_hand(2);
        assert_ne!(cat_reply(3, &state), two);
        assert!(!friend_line(&state).is_empty());
    }

    #[test]
    fn a_click_on_nothing_does_nothing() {
        let mut state = State::default();
        let mut run = Run::default();
        assert_eq!(run.click(&mut state, 0.02, 0.02), Event::Nothing);
        assert_eq!(state, State::default());
    }
}
