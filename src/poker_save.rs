//! What the Hold'em table remembers between runs: how many games were played and won, and the soul
//! lock the Shadow Monarch puts on you when he wins.
//!
//! The lock is a real clock time (seconds since 1970), so it keeps counting while the program is
//! closed. While it is on, the Underworld cannot be left or changed (the screen's look is held
//! where it is); nothing else is touched, so fans, lighting and the rest still work. When the time
//! is up the "you broke free" scene is owed once. A later quest could end the lock early: that goes
//! through `free`, the same as the testing button.
use crate::poker_game::Ending;

/// The longest the lock can ever be: a time further off than this in the file is a clock gone wrong
/// and is cut back to it.
pub const MAX_LOCK_SECS: u64 = 4 * 3600;

/// How long the Monarch holds you for each way he can win: the closer you came, the shorter.
pub fn lock_secs(e: Ending) -> u64 {
    match e {
        Ending::MonarchLate => 3600,
        Ending::MonarchAfterBear { .. } => 2 * 3600,
        Ending::MonarchAfterYou { .. } => 3 * 3600,
        Ending::MonarchSweep => MAX_LOCK_SECS,
        // (nobody else holds a soul)
        Ending::Victory { .. } | Ending::BearWins { .. } | Ending::CatWins { .. } | Ending::FatherWins { .. } => 0,
    }
}

/// Seconds since 1970 by the computer's clock (0 if it cannot be read).
pub fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A time left written for the screen: "3h 05m", "12m 40s" or "9s".
pub fn left_text(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PokerSave {
    /// When the lock ends (unix seconds); 0 for none.
    until: u64,
    /// How many times the soul has been taken.
    takes: u32,
    /// The "you broke free" scene has not been shown yet.
    free_notice: bool,
    /// Games played to the end (or until you were out).
    played: u32,
    /// Games won.
    won: u32,
    /// The intro has been seen (it is short the second time).
    seen_intro: bool,
    /// The gear at the table: deal the next hand by itself.
    auto_deal: bool,
    /// The gear at the table: cards go straight down, with no hand to put them there.
    skip_dealing: bool,
    /// The gear at the table: for players who know the game, the camera stays out of the way while the others think.
    quick_table: bool,
    /// The King of Hearts and the Queen of Diamonds have been dealt to you together (a real deal).
    together: bool,
}

/// The table's options (the hell gear in the corner of the screen).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableOptions {
    /// Deal the next hand by itself, a moment after the last one is over.
    pub auto_deal: bool,
    /// Skip the dealing: the cards are simply there, with one card sound for the last one placed.
    pub skip_dealing: bool,
    /// A quick table: the camera does not turn to whoever is thinking, and the pace is not slowed for anyone.
    pub quick_table: bool,
}

impl PokerSave {
    pub fn played(&self) -> u32 {
        self.played
    }

    pub fn won(&self) -> u32 {
        self.won
    }

    /// Whether you have ever held the two lovers together.
    pub fn together(&self) -> bool {
        self.together
    }

    /// Notes that you were dealt both. True just the first time.
    pub fn mark_together(&mut self) -> bool {
        !std::mem::replace(&mut self.together, true)
    }

    /// Times the soul was taken.
    pub fn takes(&self) -> u32 {
        self.takes
    }

    pub fn seen_intro(&self) -> bool {
        self.seen_intro
    }

    pub fn mark_intro_seen(&mut self) {
        self.seen_intro = true;
    }

    /// What the gear at the table was left set to.
    pub fn options(&self) -> TableOptions {
        TableOptions { auto_deal: self.auto_deal, skip_dealing: self.skip_dealing, quick_table: self.quick_table }
    }

    pub fn set_options(&mut self, o: TableOptions) {
        self.auto_deal = o.auto_deal;
        self.skip_dealing = o.skip_dealing;
        self.quick_table = o.quick_table;
    }

    /// Notes that a game is over and how it ended: counts it, and if the Monarch won, starts the lock
    /// at `now`. Returns the seconds of the lock (0 if none).
    pub fn finish(&mut self, e: Ending, now: u64) -> u64 {
        self.played = self.played.saturating_add(1);
        if e.you_won() {
            self.won = self.won.saturating_add(1);
        }
        let secs = lock_secs(e);
        if e.soul_taken() && secs > 0 {
            self.takes = self.takes.saturating_add(1);
            self.until = now.saturating_add(secs);
            self.free_notice = false;
        }
        secs
    }

    /// Seconds of the lock left at `now`; 0 when it is not on.
    pub fn remaining(&self, now: u64) -> u64 {
        self.until.saturating_sub(now).min(MAX_LOCK_SECS)
    }

    pub fn locked(&self, now: u64) -> bool {
        self.remaining(now) > 0
    }

    /// Call now and then: when the lock has run out this notes that the break-free scene is owed and
    /// clears the lock. True just that once.
    pub fn tick(&mut self, now: u64) -> bool {
        if self.until == 0 {
            return false;
        }
        if now >= self.until {
            self.until = 0;
            self.free_notice = true;
            return true;
        }
        // a time too far away is a clock gone wrong: cut it back
        if self.until - now > MAX_LOCK_SECS {
            self.until = now + MAX_LOCK_SECS;
        }
        false
    }

    /// Whether the break-free scene is owed.
    pub fn free_notice(&self) -> bool {
        self.free_notice
    }

    /// The break-free scene has been shown.
    pub fn take_free_notice(&mut self) -> bool {
        std::mem::take(&mut self.free_notice)
    }

    /// Lets the soul go now: the testing button, and where a quest that helps you escape comes in.
    /// The break-free scene is owed.
    pub fn free(&mut self) {
        if self.until != 0 {
            self.until = 0;
            self.free_notice = true;
        }
    }

    /// Testing: the lock for the longest time, from `now`.
    pub fn lock_for_test(&mut self, now: u64) {
        self.until = now + 120;
        self.takes = self.takes.saturating_add(1);
        self.free_notice = false;
    }

    /// The config file's lines.
    pub fn to_text(&self) -> String {
        format!(
            "poker_lock_until={}\npoker_soul_takes={}\npoker_free_notice={}\npoker_played={}\npoker_won={}\npoker_intro_seen={}\npoker_auto_deal={}\npoker_skip_dealing={}\npoker_quick_table={}\npoker_together={}\n",
            self.until, self.takes, self.free_notice as u8, self.played, self.won, self.seen_intro as u8, self.auto_deal as u8, self.skip_dealing as u8, self.quick_table as u8, self.together as u8
        )
    }

    /// Reads one config line; false when it is not one of ours.
    pub fn read_line(&mut self, name: &str, value: &str) -> bool {
        let v = value.trim();
        let flag = || v == "1" || v.eq_ignore_ascii_case("true");
        match name {
            "poker_lock_until" => self.until = v.parse().unwrap_or(0),
            "poker_soul_takes" => self.takes = v.parse().unwrap_or(0),
            "poker_free_notice" => self.free_notice = flag(),
            "poker_played" => self.played = v.parse().unwrap_or(0),
            "poker_won" => self.won = v.parse().unwrap_or(0),
            "poker_intro_seen" => self.seen_intro = flag(),
            "poker_auto_deal" => self.auto_deal = flag(),
            "poker_skip_dealing" => self.skip_dealing = flag(),
            "poker_quick_table" => self.quick_table = flag(),
            "poker_together" => self.together = flag(),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poker_game::{BEAR, MONARCH, YOU};

    const NOW: u64 = 1_800_000_000;

    #[test]
    fn only_the_monarchs_wins_lock_you_and_the_closer_you_came_the_shorter() {
        assert_eq!(lock_secs(Ending::Victory { first_out: BEAR, last: MONARCH }), 0);
        assert_eq!(lock_secs(Ending::BearWins { you_out_by: BEAR, monarch_out_first: false }), 0);
        let late = lock_secs(Ending::MonarchLate);
        let bear = lock_secs(Ending::MonarchAfterBear { bear_out_by: YOU });
        let you = lock_secs(Ending::MonarchAfterYou { you_out_by: BEAR });
        let sweep = lock_secs(Ending::MonarchSweep);
        assert!(0 < late && late < bear && bear < you && you < sweep);
        assert_eq!(sweep, MAX_LOCK_SECS);
        for e in [Ending::MonarchLate, Ending::MonarchSweep, Ending::MonarchAfterYou { you_out_by: MONARCH }] {
            assert!(e.soul_taken() && lock_secs(e) > 0);
        }
    }

    #[test]
    fn a_loss_to_the_monarch_locks_for_real_time_and_it_runs_out() {
        let mut s = PokerSave::default();
        assert!(!s.locked(NOW));
        let secs = s.finish(Ending::MonarchSweep, NOW);
        assert_eq!(secs, MAX_LOCK_SECS);
        assert!(s.locked(NOW) && s.locked(NOW + 3 * 3600));
        assert_eq!(s.remaining(NOW + 3600), MAX_LOCK_SECS - 3600);
        assert_eq!((s.played(), s.won(), s.takes()), (1, 0, 1));
        // the lock is only a time: still on after a restart, over when the time is up
        let mut again = PokerSave::default();
        for l in s.to_text().lines() {
            let (k, v) = l.split_once('=').unwrap();
            assert!(again.read_line(k, v), "{k}");
        }
        assert_eq!(again, s);
        assert!(again.locked(NOW + 100));
        assert!(!again.tick(NOW + 100));
        assert!(again.tick(NOW + MAX_LOCK_SECS));
        assert!(!again.locked(NOW + MAX_LOCK_SECS));
        assert!(again.free_notice());
        assert!(!again.tick(NOW + MAX_LOCK_SECS + 1), "only once");
        assert!(again.take_free_notice());
        assert!(!again.take_free_notice());
    }

    #[test]
    fn winning_and_a_bear_win_cost_nothing_but_are_counted() {
        let mut s = PokerSave::default();
        assert_eq!(s.finish(Ending::Victory { first_out: BEAR, last: MONARCH }, NOW), 0);
        assert_eq!(s.finish(Ending::BearWins { you_out_by: BEAR, monarch_out_first: true }, NOW), 0);
        assert!(!s.locked(NOW));
        assert_eq!((s.played(), s.won(), s.takes()), (2, 1, 0));
    }

    #[test]
    fn a_bad_clock_in_the_file_cannot_hold_you_longer_than_the_most() {
        let mut s = PokerSave::default();
        assert!(s.read_line("poker_lock_until", &(NOW + 90 * 24 * 3600).to_string()));
        assert_eq!(s.remaining(NOW), MAX_LOCK_SECS);
        assert!(!s.tick(NOW));
        assert_eq!(s.remaining(NOW), MAX_LOCK_SECS);
        assert!(s.tick(NOW + MAX_LOCK_SECS), "and it still ends");
    }

    #[test]
    fn freeing_the_soul_ends_the_lock_and_owes_the_scene_and_does_nothing_when_free() {
        let mut s = PokerSave::default();
        s.free();
        assert!(!s.free_notice(), "nothing to be freed from");
        s.finish(Ending::MonarchLate, NOW);
        assert!(s.locked(NOW));
        s.free();
        assert!(!s.locked(NOW) && s.free_notice());
        s.lock_for_test(NOW);
        assert!(s.locked(NOW) && !s.free_notice());
        assert_eq!(s.takes(), 2);
    }

    #[test]
    fn the_gear_options_are_off_at_first_and_are_remembered() {
        let mut s = PokerSave::default();
        assert_eq!(s.options(), TableOptions::default());
        s.set_options(TableOptions { auto_deal: true, skip_dealing: false, quick_table: true });
        let mut again = PokerSave::default();
        for l in s.to_text().lines() {
            let (k, v) = l.split_once('=').unwrap();
            assert!(again.read_line(k, v), "{k}");
        }
        assert_eq!(again.options(), TableOptions { auto_deal: true, skip_dealing: false, quick_table: true });
        assert_eq!(again, s);
    }

    #[test]
    fn holding_the_two_lovers_together_is_noted_once_and_remembered() {
        let mut s = PokerSave::default();
        assert!(!s.together());
        assert!(s.mark_together(), "new the first time");
        assert!(!s.mark_together(), "and not again");
        let mut again = PokerSave::default();
        for l in s.to_text().lines() {
            let (k, v) = l.split_once('=').unwrap();
            assert!(again.read_line(k, v), "{k}");
        }
        assert!(again.together());
        assert_eq!(again, s);
    }

    #[test]
    fn the_lines_are_read_back_and_bad_values_become_nothing() {
        let mut s = PokerSave::default();
        assert!(s.read_line("poker_played", "7"));
        assert!(s.read_line("poker_won", "x"));
        assert!(s.read_line("poker_intro_seen", "true"));
        assert!(!s.read_line("something_else", "1"));
        assert_eq!((s.played(), s.won(), s.seen_intro()), (7, 0, true));
    }

    #[test]
    fn times_are_written_for_the_screen() {
        assert_eq!(left_text(4 * 3600), "4h 00m");
        assert_eq!(left_text(3 * 3600 + 5 * 60 + 59), "3h 05m");
        assert_eq!(left_text(12 * 60 + 40), "12m 40s");
        assert_eq!(left_text(9), "9s");
        assert_eq!(left_text(0), "0s");
    }
}
