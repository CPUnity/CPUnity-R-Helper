//! What is said at the Hold'em table in the Underworld: the dealer, the Demon Bear, the dead
//! king, Father Time and the Shadow Monarch. Words only (the table screen shows them one by one).
//!
//! The intro, the Monarch's arrival, the table talk during play and one ending for each way the
//! tournament can finish (see `poker_game::Ending`).
use crate::poker_game::{Ending, BEAR, CAT, FATHER, MONARCH, YOU};

/// Who is talking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Speaker {
    /// The demon who deals (Mammon).
    Dealer,
    /// The Demon Bear, across from you.
    Bear,
    /// The dead Monarch King Cat, in his chair at the side.
    Cat,
    /// Father Time, smoking.
    Father,
    /// The Shadow Monarch, once he has sat down.
    Monarch,
    /// Not a person: what happens, in italics.
    Stage,
}

impl Speaker {
    pub fn name(self) -> &'static str {
        match self {
            Speaker::Dealer => "Mammon",
            Speaker::Bear => "The Demon Bear",
            Speaker::Cat => "The King",
            Speaker::Father => "Father Time",
            Speaker::Monarch => "The Shadow Monarch",
            Speaker::Stage => "",
        }
    }
}

/// Something the camera does when a line comes up (see `poker_cam`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    None,
    /// You wince and look away from whoever the line is about.
    Flinch,
    /// Father Time blows smoke in your face once the line has been read (when you click past it), and
    /// then the camera pulls back.
    Blow,
}

impl Cue {
    /// The camera does something of its own (rather than just following whoever speaks).
    pub fn moves_camera(self) -> bool {
        matches!(self, Cue::Flinch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    pub who: Speaker,
    pub text: &'static str,
    /// For what happens (a `Stage` line): who the camera turns to while it is up.
    pub look: Option<Speaker>,
    pub cue: Cue,
}

impl Line {
    /// Who the camera is on while this line is up.
    pub fn focus(&self) -> Option<Speaker> {
        match self.who {
            Speaker::Stage => self.look,
            who => Some(who),
        }
    }
}

macro_rules! say {
    ($who:expr, $text:expr) => {
        Line { who: $who, text: $text, look: None, cue: Cue::None }
    };
}

/// Something a person says, with something for the camera to do as they say it.
macro_rules! said {
    ($cue:expr, $who:expr, $text:expr) => {
        Line { who: $who, text: $text, look: None, cue: $cue }
    };
}

/// What happens, with the camera on `$look`.
macro_rules! see {
    ($look:expr, $text:expr) => {
        Line { who: Speaker::Stage, text: $text, look: Some($look), cue: Cue::None }
    };
}

/// What happens, with the camera on `$look` and something for the camera to do.
macro_rules! cue {
    ($cue:expr, $look:expr, $text:expr) => {
        Line { who: Speaker::Stage, text: $text, look: Some($look), cue: $cue }
    };
}

use Speaker::{Bear as B, Cat as C, Dealer as D, Father as F, Monarch as M, Stage as S};

/// The name of the dealer.
pub const DEALER_NAME: &str = "Mammon";

/// The words before the first hand: the dealer lays out the rules, the camera turns to the dead king, you wince
/// past the Bear, and Father Time has the last word with the smoke.
pub fn intro() -> Vec<Line> {
    vec![
        see!(D, "A table of black stone, a red cloth that is not quite felt. A ram-horned demon shuffles without looking."),
        say!(D, "Sit. Twenty tokens, a thousand chips each, and a pot nobody here can spend. The prize is Heaven's power."),
        say!(D, "Fifty-two cards, shuffled fresh every hand. I seal the deck before a card is dealt and open it after. Check my work. Everyone does. Nobody has ever found a thing."),
        say!(C, "Well, well. The one who killed me and opened the door to damnation. Do sit. I would stand, but I am dead."),
        say!(C, "Don't mind the eyes. They were like this when you left me."),
        see!(B, "The Demon Bear is looking at you. He does not blink. He has been looking at you since before you sat down, and he is far too serious about it."),
        cue!(Cue::Flinch, B, "You wince, and look away."),
        see!(F, "He smokes again."),
        said!(Cue::Blow, F, "You made me stop smoking when you started damnation. Now I'm smoking because time has lost its patience.... I want my sand back!"),
        say!(D, "Blinds are ten and twenty. They go up. Cards in the air."),
    ]
}

/// Why the Shadow Monarch is sitting down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// Enough hands have gone by.
    Time,
    /// Someone is out already (the seat number).
    SomeoneOut(usize),
}

/// What is said when the Shadow Monarch walks in. `stack` is what he buys in for.
pub fn monarch_arrives(why: Arrival, _stack: u64) -> Vec<Line> {
    let mut v = vec![
        say!(S, "The lamps go out one by one. Something that is not quite a shadow stops at the empty chair."),
        say!(C, "Oh no. Oh, no, no. That is no friend of mine, and he has never once laughed."),
        say!(M, "I am told there is a game. I would like to buy in."),
        say!(D, "The stakes are tokens, my lord. Chips are only what we call them."),
        say!(M, "Then I will pay what I pay. I have no use for the chips."),
    ];
    match why {
        Arrival::Time => {
            v.push(say!(M, "Killing you was not enough. I wondered where you went. Now I know, and you are playing cards."));
        }
        Arrival::SomeoneOut(s) if s == YOU => {
            v.push(say!(M, "I came for you, and you are already out of the game. Stay. Watch. Your soul does not leave this table."));
        }
        Arrival::SomeoneOut(s) if s == BEAR => {
            v.push(say!(M, "The Bear is gone and I have only just sat down. That is a pity. I wanted to see it done to him."));
            v.push(say!(B, "You were not invited."));
        }
        Arrival::SomeoneOut(s) if s == CAT => {
            v.push(say!(M, "The dead one is out already. I wanted a word with him."));
            v.push(say!(C, "I'm right here. I'm dead, not invisible."));
        }
        Arrival::SomeoneOut(s) if s == FATHER => {
            v.push(say!(M, "Time has left the table already. It never lasts."));
            v.push(say!(F, "I'm only out of chips, buddy. Don't be dramatic."));
        }
        Arrival::SomeoneOut(_) => {
            v.push(say!(M, "Someone is out already. Good. The table is smaller, and I am hungry."));
        }
    }
    v.push(say!(S, "He puts down a chip stack that was not there a moment before. It looks like a handful of ash."));
    v.push(say!(D, "Cards in the air."));
    v
}

/// The things worth a comment while the cards are going round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Moment {
    /// The blinds just went up.
    BlindsUp,
    /// You pushed everything in.
    YouAllIn,
    /// The Bear pushed everything in.
    BearAllIn,
    /// The Monarch pushed everything in.
    MonarchAllIn,
    /// You won a big pot.
    YouWinBig,
    /// The Bear won a big pot.
    BearWinsBig,
    /// The Monarch won a big pot.
    MonarchWinsBig,
    /// You folded a hand that would have won.
    YouFoldedWinner,
    /// Someone bluffed and got away with it.
    BluffWorked,
    /// You are the short stack, three hands from the end.
    YouAreShort,
    /// The King bluffed and got away with it.
    CatBluffWorked,
    /// The King bluffed and was called.
    CatBluffCaught,
    /// Father Time lost most of his chips in one hand and his beard fell off.
    FatherBeardFalls,
    /// The King went out (to the seat).
    CatOut(usize),
    /// Father Time went out (to the seat).
    FatherOut(usize),
    /// The Bear went out (to the seat).
    BearOut(usize),
    /// The Monarch went out (to the seat).
    MonarchOut(usize),
    /// You went out (to the seat).
    YouOut(usize),
}

fn pool(m: Moment) -> &'static [Line] {
    match m {
        Moment::BlindsUp => &[
            say!(D, "The blinds go up. The house enjoys this part."),
            say!(F, "Blinds are up. Time does that. Don't look at me."),
            say!(C, "Higher blinds. Higher stakes. Lower life expectancy. I speak from experience."),
            say!(D, "New level. The little numbers get bigger, and the big ones get hungry."),
        ],
        Moment::YouAllIn => &[
            say!(C, "All in! I love this part. I lost my entire kingdom on a hand like that."),
            say!(B, "..."),
            say!(F, "You put everything in. I respect that. I also know how it ends, usually."),
            say!(D, "All in, from the one with the pulse. Lovely."),
        ],
        Moment::BearAllIn => &[
            say!(B, "All of it."),
            say!(C, "The Bear is all in. He's staring. He's always staring. I can't tell if it's a bluff."),
            say!(F, "The Bear pushes. Well. Take your time. I have all of it."),
            say!(D, "Every chip, from the big fellow. Dramatic."),
        ],
        Moment::MonarchAllIn => &[
            say!(M, "Everything. It was never mine."),
            say!(C, "He's all in. Do not call that. Do not call that. ...Call it, I want to see."),
            say!(F, "The shadow has pushed. Even the cigarette is nervous."),
            say!(D, "The Monarch is all in. The room just got colder, and I run a warm room."),
        ],
        Moment::YouWinBig => &[
            say!(C, "Ha! Look at you, taking pots off the Devil. I'd clap, but, well."),
            say!(F, "Nice hand. Don't let it go to your head. Heads here come off."),
            say!(B, "Hm."),
            say!(D, "A fine pot to the mortal. Do keep it. They never do."),
        ],
        Moment::BearWinsBig => &[
            say!(B, "Mine."),
            say!(C, "He's won again. You can tell because the staring got worse."),
            say!(F, "The Bear takes it. He doesn't even smile. Terrifying."),
            say!(D, "To the Bear. He has the patience of something that has waited for a very long time."),
        ],
        Moment::MonarchWinsBig => &[
            say!(M, "I take what is owed."),
            say!(C, "He's winning. That is bad. He wins everything, and then he wins it again."),
            say!(F, "The shadow collects. There is a chill. Might be the smoke."),
            say!(D, "To the Monarch. The chips go cold in his hands."),
        ],
        Moment::YouFoldedWinner => &[
            say!(C, "You folded the best hand. I saw it. Everybody saw it. The Devil saw it."),
            say!(F, "Folded the winner. Happens. Usually only the once."),
            say!(B, "Thank you."),
        ],
        Moment::BluffWorked => &[
            say!(C, "It was a bluff. A bluff! He had nothing! Honesty is not on the menu down here."),
            say!(F, "Nothing but air in that hand. And I thought I was the one who blew smoke."),
            say!(D, "Nothing, and it won. This is why I love this table."),
        ],
        Moment::YouAreShort => &[
            say!(C, "You've got hardly any chips left. Don't take that personally. I mean, take it personally."),
            say!(F, "You are running short. So am I, but that is a different problem."),
            say!(D, "A short stack. Every chip is a prayer now."),
        ],
        Moment::CatBluffWorked => &[
            say!(C, "Pay up. Never trust a dead man, and never trust a dead man with a stack."),
            say!(C, "I had nothing! I had absolutely nothing, and you all believed me. Say it out loud, it's the best part."),
            say!(C, "Bluff! You fold like a deck chair. Ha."),
            say!(F, "He's laughing. I'd like to be mad, but I've seen worse from the living."),
        ],
        Moment::CatBluffCaught => &[
            say!(C, "Fine. I bluffed. I'm a monster. Next hand."),
            say!(F, "Caught him. The King has the poker face of a man who was stabbed in the back."),
            say!(B, "Pathetic."),
            say!(D, "Called, and he has nothing. The King's pride is a lovely thing to watch."),
        ],
        Moment::FatherBeardFalls => &[
            say!(S, "Father Time's face quivers. His beard drops into his lap."),
            say!(F, "That was a lot of chips. That was... a lot. I need a minute. Or several."),
            say!(C, "His beard fell off! His beard! Somebody get him a hand."),
            say!(D, "The beard stays on the floor. Management is not responsible."),
        ],
        Moment::CatOut(by) => {
            if by == YOU {
                &[
                    say!(C, "You knocked out a dead man. I'd say you should be ashamed, but, well, I'm dead."),
                    say!(F, "The King is out. He doesn't seem sorry about any of it."),
                    say!(D, "The King busts. The house will miss his noise, if not him."),
                ]
            } else {
                &[
                    say!(C, "Out. Beaten by a cheat. Which, considering I was cheating too, is fair."),
                    say!(F, "The King is out. He'll be in the corner, sulking in a crown."),
                    say!(D, "The King busts. He will insist it was bad luck."),
                ]
            }
        }
        Moment::FatherOut(by) => {
            if by == YOU {
                &[
                    say!(F, "I've been out of time before. This is a new one. Well played, kid."),
                    say!(C, "You knocked out Time itself! That's quite the achievement."),
                    say!(D, "Father Time is out. The house has been waiting a long time to say that."),
                ]
            } else {
                &[
                    say!(F, "Out. Time's up, as they say. Pass me a cigarette."),
                    say!(C, "Father Time is out of time. I'll see myself out of that joke."),
                    say!(D, "Father Time busts. The clock stops, briefly."),
                ]
            }
        }
        Moment::BearOut(by) => {
            if by == YOU {
                &[
                    say!(B, "..."),
                    say!(C, "You knocked out the Devil! I'd faint, but I'm dead, so I've done it already."),
                    say!(F, "The Bear is out. He just sits there. Still staring. Still scary."),
                ]
            } else {
                &[
                    say!(B, "It is not over."),
                    say!(C, "Oh dear. The Bear's gone. And that was the sensible one."),
                    say!(F, "The Bear is out. It was a good run. For a bear."),
                ]
            }
        }
        Moment::MonarchOut(by) => {
            if by == YOU {
                &[
                    say!(M, "This is not the end. It was not the end the last time, either."),
                    say!(C, "You beat him! At cards! You'll never get away with it."),
                    say!(F, "The shadow's out. Good. It was getting cold."),
                ]
            } else {
                &[
                    say!(M, "A bear. Beaten by a bear."),
                    say!(C, "Even the Monarch can go out. I'd laugh, but I'm too afraid of him."),
                    say!(F, "The Monarch's out. The bear does not seem pleased about it."),
                ]
            }
        }
        Moment::YouOut(by) => {
            if by == MONARCH {
                &[
                    say!(M, "Yours."),
                    say!(C, "Oh. Oh, that's not good. He gets that look when he's hungry."),
                    say!(F, "Well. That's you out. To him, of all people."),
                ]
            } else {
                &[
                    say!(B, "Out."),
                    say!(C, "You're out. The Devil did it. How typical."),
                    say!(F, "That's you out. Tough break, kid. Have a smoke."),
                ]
            }
        }
    }
}

/// One thing said about a moment; `n` picks which (it can be a counter), so it changes from time to time.
pub fn quip(m: Moment, n: usize) -> Line {
    let p = pool(m);
    p[n % p.len()]
}

fn victory(first_out: usize, last: usize) -> Vec<Line> {
    let mut v = Vec::new();
    match first_out {
        MONARCH => {
            v.push(say!(S, "The Shadow Monarch's last chip crosses the cloth. He was the first to go, and he does not take it well."));
            v.push(say!(M, "A mortal. At my own table."));
        }
        CAT => v.push(say!(S, "The dead King went first, still talking as the last chip left him.")),
        FATHER => v.push(say!(S, "Father Time went first, with a shrug and a fresh cigarette.")),
        _ => v.push(say!(S, "The Bear went first, which nobody at the table believed was possible.")),
    }
    if last == MONARCH {
        v.push(say!(S, "At the end it was you and the Shadow Monarch. His last chips go in a hiss of ash."));
    } else {
        v.push(say!(S, "At the end it was you and the Demon Bear, and his last chip crosses the cloth."));
    }
    v.extend([
        say!(S, "The Demon Bear is silent for a long time."),
        say!(B, "You won. You were not supposed to."),
        say!(B, "Take it. Heaven's power never belonged to me. I only kept it warm."),
        say!(C, "You beat the Devil! And the thing that killed me! Do you know how long I've waited to say this? Good job. Truly. I'm dead."),
        say!(F, "Well. I've seen a lot of years. That is the first time I've seen the Bear hand something over."),
        say!(S, "Mammon sweeps the cards into a black bag and, for once, does not smile."),
        say!(D, "The house pays. Not often. Don't spend it all in one place. Heaven is above the table."),
        say!(S, "Something above you opens. It is the first light you've seen in a long, long time."),
    ]);
    v
}

fn bear_wins(you_out_by: usize, monarch_out_first: bool) -> Vec<Line> {
    let mut v = Vec::new();
    if monarch_out_first {
        v.push(say!(S, "The Shadow Monarch was gone before you. The Bear took his chips and yours."));
        v.push(say!(M, "..."));
    } else {
        v.push(say!(S, "The Bear takes the last of the chips, yours and the Monarch's, and neatly stacks them."));
    }
    if you_out_by == MONARCH {
        v.push(say!(B, "He took you out, and I took him. Everything comes to me in the end."));
    } else {
        v.push(say!(B, "The mortal falls. I am not surprised."));
    }
    v.extend([
        say!(C, "That's it? The Devil wins? He always wins. That is why they call him that."),
        say!(F, "Don't take it hard. It's a game. A game for Heaven's power, in hell, against the Devil. You did fine."),
        say!(D, "The house pays the Bear. Come back with twenty more tokens, and we'll shuffle again."),
        say!(S, "The Bear looks at you. He has not blinked. You get to leave, which is the best outcome you are going to get today."),
    ]);
    v
}

fn cat_wins(you_out_by: usize) -> Vec<Line> {
    vec![
        say!(S, if you_out_by == CAT {
            "The dead King took your last chip, and then the others'. He sits there, stunned, with the whole table."
        } else {
            "You went out early, and then the dead King, of all people, took everything that was left."
        }),
        say!(C, "I WON? I won! A dead man, with a crown and nothing, beat the Devil! I bluffed the whole way!"),
        say!(B, "I will remember this."),
        say!(F, "Don't look at me. I have been telling people for years that he is lucky."),
        say!(D, "The King takes the pot. The house pays what it owes, and Heaven's power goes to a cat in a crown. Tell no one."),
        say!(C, "Thank you, thank you. You may kiss the paw."),
        say!(S, "Nothing is taken from you. The table goes quiet. It is back in the dark."),
    ]
}

fn father_wins(you_out_by: usize) -> Vec<Line> {
    vec![
        say!(S, if you_out_by == FATHER {
            "Father Time took your last chip, slowly, and then the others'. He didn't hurry once."
        } else {
            "You went out, and in the end Father Time took everything that was left. He didn't hurry once."
        }),
        say!(F, "Well. I had nothing better to do. I've got all the time in the world."),
        say!(C, "Time won! Time always wins! That is the whole point of him!"),
        say!(B, "He waited. Of course he waited."),
        say!(D, "The pot goes to Father Time. He has been playing this game since before there were cards."),
        say!(F, "Come back any time, kid. Takes me a while. Wait. That's the joke. Never mind."),
        say!(S, "Nothing is taken from you. He blows smoke at the ceiling and the table goes quiet."),
    ]
}

fn soul_ending(lines: Vec<Line>) -> Vec<Line> {
    let mut v = lines;
    v.extend([
        say!(S, "The lamps are out. The chair across from you is empty and the cards are gone."),
        say!(M, "Your soul is mine now. I did not kill you. I kept you. It is much worse."),
        say!(S, "The floor lets go of you. The Void closes around the table."),
        say!(D, "I deal fairly, mortal. I do not choose who wins. That has always been the problem with fairness."),
        say!(C, "Sorry about that. I mean it. If I could grab your hand I would. But, well."),
    ]);
    v
}

/// One ending for each way the tournament can finish.
pub fn ending(e: Ending) -> Vec<Line> {
    match e {
        Ending::Victory { first_out, last } => victory(first_out, last),
        Ending::CatWins { you_out_by } => cat_wins(you_out_by),
        Ending::FatherWins { you_out_by } => father_wins(you_out_by),
        Ending::BearWins { you_out_by, monarch_out_first } => bear_wins(you_out_by, monarch_out_first),
        Ending::MonarchSweep => soul_ending(vec![
            say!(S, "One chip stack is left on the cloth. Yours and the Bear's are both in the Monarch's hands, and he took both of them himself."),
            say!(B, "He took mine as well. The Devil, beaten at his own table."),
            say!(M, "You were both in my way. Now there is only the table."),
            say!(C, "He took them both. Of all the ways this could have gone, this is the worst one, and I've seen the others."),
            say!(F, "Don't look at me. I stopped smoking for this crowd. Twice."),
        ]),
        Ending::MonarchLate => soul_ending(vec![
            say!(S, "You put the Bear out. You played the best poker of your life. And then the Monarch looked at you across an empty table."),
            say!(B, "The mortal took me down. He should have kept the bear and run."),
            say!(M, "You were good. That is the cruelest part. You were so close."),
            say!(C, "You beat the Devil! And then the shadow beat you! I am so sorry, I would have laughed at this a week ago."),
            say!(F, "That was a good fight. I mean it. A good fight, and a rotten end."),
        ]),
        Ending::MonarchAfterYou { you_out_by } => soul_ending(vec![
            say!(S, if you_out_by == MONARCH {
                "You were out first, to the Monarch. You watched the rest from your chair, and he took the Bear apart a hand at a time."
            } else {
                "You were out first. You watched the rest from your chair, and the Monarch took the Bear apart a hand at a time."
            }),
            say!(B, "Not like this."),
            say!(M, "The Devil is on his knees. And the mortal watched, because the mortal had nowhere to go."),
            say!(C, "You couldn't even stay in to watch the end. Well. You watched anyway. Poor you."),
            say!(F, "The Bear's out. You're out. The shadow's got everything. I'm going to need another cigarette."),
        ]),
        Ending::MonarchAfterBear { bear_out_by } => soul_ending(vec![
            say!(S, if bear_out_by == YOU {
                "You put the Bear out, and the Monarch took what was left of you."
            } else {
                "The Bear was out early, and the Monarch took what was left of you."
            }),
            say!(B, "He took the table from me. I thought I was the worst thing here."),
            say!(M, "One of you gone, one left. It was a short evening."),
            say!(C, "He didn't even look at the cards. I swear he doesn't need them."),
            say!(F, "It happens. A shadow sits down, and everybody else goes home. Sorry, kid."),
        ]),
    }
}

/// Said when the soul lock runs out: you are loose again.
pub fn broke_free() -> Vec<Line> {
    vec![
        say!(S, "Something that has been holding you lets go."),
        say!(S, "The Void is only a place again. You can see the rest of it."),
        say!(C, "Hey. Hey, you. You got out. I didn't think you would. Good for you."),
        say!(S, "Somewhere far away, the Shadow Monarch is still counting his chips."),
    ]
}

/// The line on the locked screen, with how long is left (already written out, like "2h 10m").
pub fn locked_banner(left: &str) -> String {
    format!("Your soul is held. {left} to go. You can look, but not touch.")
}

/// What the dealer says as you sit down, for the token price.
pub fn buy_in_notice(cost: u64) -> String {
    format!("{cost} CR-Tokens go in the pot. Mammon counts them twice.")
}

/// What the dealer says when the Monarch buys in.
pub fn monarch_buy_in_notice(chips: u64) -> String {
    format!("The Shadow Monarch buys in for {chips} chips.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poker_game::Ending;

    fn all_endings() -> Vec<Ending> {
        vec![
            Ending::Victory { first_out: BEAR, last: MONARCH },
            Ending::Victory { first_out: MONARCH, last: BEAR },
            Ending::Victory { first_out: CAT, last: BEAR },
            Ending::Victory { first_out: FATHER, last: MONARCH },
            Ending::CatWins { you_out_by: CAT },
            Ending::CatWins { you_out_by: BEAR },
            Ending::FatherWins { you_out_by: FATHER },
            Ending::FatherWins { you_out_by: BEAR },
            Ending::BearWins { you_out_by: BEAR, monarch_out_first: false },
            Ending::BearWins { you_out_by: MONARCH, monarch_out_first: true },
            Ending::BearWins { you_out_by: BEAR, monarch_out_first: true },
            Ending::MonarchSweep,
            Ending::MonarchLate,
            Ending::MonarchAfterYou { you_out_by: BEAR },
            Ending::MonarchAfterYou { you_out_by: MONARCH },
            Ending::MonarchAfterBear { bear_out_by: YOU },
            Ending::MonarchAfterBear { bear_out_by: MONARCH },
        ]
    }

    fn all_moments() -> Vec<Moment> {
        let mut v = vec![
            Moment::BlindsUp,
            Moment::YouAllIn,
            Moment::BearAllIn,
            Moment::MonarchAllIn,
            Moment::YouWinBig,
            Moment::BearWinsBig,
            Moment::MonarchWinsBig,
            Moment::YouFoldedWinner,
            Moment::BluffWorked,
            Moment::YouAreShort,
            Moment::CatBluffWorked,
            Moment::CatBluffCaught,
            Moment::FatherBeardFalls,
        ];
        for s in [YOU, BEAR, MONARCH, CAT, FATHER] {
            v.push(Moment::CatOut(s));
            v.push(Moment::FatherOut(s));
            v.push(Moment::BearOut(s));
            v.push(Moment::MonarchOut(s));
            v.push(Moment::YouOut(s));
        }
        v
    }

    fn every_line() -> Vec<Line> {
        let mut v = intro();
        for why in [Arrival::Time, Arrival::SomeoneOut(YOU), Arrival::SomeoneOut(BEAR), Arrival::SomeoneOut(CAT), Arrival::SomeoneOut(FATHER)] {
            v.extend(monarch_arrives(why, 3000));
        }
        for e in all_endings() {
            v.extend(ending(e));
        }
        v.extend(broke_free());
        for m in all_moments() {
            for n in 0..8 {
                v.push(quip(m, n));
            }
        }
        v
    }

    #[test]
    fn every_line_has_words_and_fits_the_screen() {
        for l in every_line() {
            assert!(!l.text.trim().is_empty());
            assert!(l.text.chars().count() <= 200, "too long: {}", l.text);
            assert!(!l.text.contains("  "), "double space: {}", l.text);
        }
    }

    #[test]
    fn every_ending_has_its_own_words_and_the_lost_ones_say_the_soul_is_gone() {
        let mut seen = std::collections::HashSet::new();
        for e in all_endings() {
            let text: Vec<&str> = ending(e).iter().map(|l| l.text).collect();
            assert!(text.len() >= 6, "{e:?}");
            assert!(seen.insert(text.join("|")), "two endings read the same: {e:?}");
            let soul = text.iter().any(|t| t.contains("Your soul is mine"));
            assert_eq!(soul, e.soul_taken(), "{e:?}");
            let heaven = text.iter().any(|t| t.contains("Heaven is above the table"));
            assert_eq!(heaven, e.you_won(), "{e:?}");
        }
    }

    #[test]
    fn the_intro_has_the_set_pieces_in_order() {
        let v = intro();
        let at = |needle: &str| v.iter().position(|l| l.text.contains(needle)).unwrap_or_else(|| panic!("missing {needle}"));
        let cat = at("opened the door to damnation");
        let bear = at("far too serious");
        let wince = at("You wince");
        let father = at("I want my sand back");
        assert!(cat < bear && bear < wince && wince < father);
        assert_eq!(v[father].who, Speaker::Father);
        assert_eq!(v[father].cue, Cue::Blow, "he breathes the smoke at you once it has been read");
        assert_eq!(v[wince].cue, Cue::Flinch);
        assert!(!v.iter().any(|l| l.text.contains("blows the smoke")), "the smoke is shown, not told");

        assert_eq!(v[cat].who, Speaker::Cat);
        assert_eq!(v.last().unwrap().who, Speaker::Dealer);
        // the dealer promises the shuffle can be checked
        assert!(v.iter().any(|l| l.who == Speaker::Dealer && l.text.contains("Check my work")));
    }

    #[test]
    fn the_monarch_comes_in_wanting_the_soul_and_says_why_he_came() {
        for why in [Arrival::Time, Arrival::SomeoneOut(YOU), Arrival::SomeoneOut(BEAR)] {
            let v = monarch_arrives(why, 3000);
            assert!(v.iter().any(|l| l.who == Speaker::Monarch && l.text.contains("buy in")));
            assert!(v.iter().any(|l| l.who == Speaker::Monarch && l.text.contains("no use for the chips")));
        }
        let time = monarch_arrives(Arrival::Time, 1);
        assert!(time.iter().any(|l| l.text.contains("Killing you was not enough")));
        let bear = monarch_arrives(Arrival::SomeoneOut(BEAR), 1);
        let you = monarch_arrives(Arrival::SomeoneOut(YOU), 1);
        assert_ne!(bear, you);
    }

    #[test]
    fn the_table_talk_changes_and_the_right_people_talk() {
        for m in all_moments() {
            let a: std::collections::HashSet<_> = (0..8).map(|n| quip(m, n).text).collect();
            assert!(a.len() >= 3, "{m:?} says too few things");
        }
        // the right people react to the right things
        assert!((0..4).any(|n| quip(Moment::MonarchAllIn, n).who == Speaker::Monarch));
        assert!((0..4).any(|n| quip(Moment::BearAllIn, n).who == Speaker::Bear));
        assert!((0..4).any(|n| quip(Moment::YouAllIn, n).who == Speaker::Cat));
    }

    #[test]
    fn nobody_speaks_as_two_people_and_the_stage_has_no_name() {
        assert_eq!(Speaker::Stage.name(), "");
        for s in [Speaker::Dealer, Speaker::Bear, Speaker::Cat, Speaker::Father, Speaker::Monarch] {
            assert!(!s.name().is_empty());
        }
        assert_eq!(Speaker::Dealer.name(), DEALER_NAME);
    }
}
