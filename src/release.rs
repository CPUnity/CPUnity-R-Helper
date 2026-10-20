//! What this build holds back. The Underworld (its look, the Devil's Table, the Soul Whispers backdrop,
//! its presets and its trophies) is being built but is not part of the release yet. While it is held:
//! Game 3's ending still plays and still opens the Underworld in the save file (so the custom songs for
//! Game 3 are unlocked, and the Underworld is there the moment an update that has it arrives), but
//! nothing of the Underworld is shown, and its trophies are neither listed nor needed for Platinum.
//!
//! To work on the Underworld, switch on Options > Game testing > "Show the Underworld", or set
//! `UNDERWORLD` to true.

/// Whether this build shows the Underworld.
pub const UNDERWORLD: bool = false;

/// Whether this build lets players into the Ally of Darkness (the alley escape room). It is built and saved,
/// but only its first area is done, so it is held back: the alley on the market is just an alley, and the
/// Act I trophy is neither listed nor needed for Platinum.
pub const ALLY: bool = false;

/// Whether this build shows the test tools (Options > Shop testing, Trophy testing, Game testing). A normal
/// release hides them. Set the environment variable `CRHELPER_DEV` (to anything) before starting it, or build
/// in debug, to see them and the held-back parts.
pub const TEST_TOOLS: bool = false;

/// Whether this build has the story side of the program: the games, the cat and the backdrops, the
/// trophies, CR-Tokens and the Shop, and the looks (skins, worlds, pairs). This release is the working
/// program only, so all of that is held: nothing of it is shown, played or announced. The save file keeps
/// whatever was earned. Set `STORY` to true, or start with `CRHELPER_DEV`, to bring it all back.
pub const STORY: bool = false;

/// Whether the story side is on (see `STORY`). Always on in the tests.
pub fn story() -> bool {
    STORY || dev() || cfg!(test)
}

fn dev() -> bool {
    cfg!(debug_assertions) || std::env::var_os("CRHELPER_DEV").is_some()
}

/// Whether the test tools are shown.
pub fn test_tools() -> bool {
    TEST_TOOLS || dev()
}

/// Whether the Ally can be entered (and its trophy counts).
pub fn ally_shown() -> bool {
    ALLY || dev() || cfg!(test)
}

/// The trophies that belong to the Ally.
pub const ALLY_TROPHIES: [&str; 1] = ["act1"];

/// The trophies that belong to the Underworld.
pub const UNDERWORLD_TROPHIES: [&str; 6] = ["underworld", "listener", "poker_played", "poker_won", "poker_soul", "poker_together"];

/// Whether `id` is an Underworld trophy that is held back (`held` says the Underworld is held).
pub fn trophy_held(id: &str, held: bool) -> bool {
    (held && UNDERWORLD_TROPHIES.contains(&id)) || (!ally_shown() && ALLY_TROPHIES.contains(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_underworlds_trophies_are_held_and_only_while_it_is() {
        for id in UNDERWORLD_TROPHIES {
            assert!(trophy_held(id, true) && !trophy_held(id, false), "{id}");
            assert!(crate::trophies::trophy(id).is_some(), "{id} is a real trophy");
        }
        assert!(!trophy_held("gold", true) && !trophy_held("platinum", true));
    }
}
