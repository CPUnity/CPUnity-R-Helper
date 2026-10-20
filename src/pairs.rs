//! The looks that can be put on from Options > UI Options (none, the skins that were bought, and the
//! earned worlds) and the pairs that were made for each other: a look and the backdrop built to go
//! with it, put on together with one click. Pure apart from the `Config` it changes, so it is tested.
use crate::config::Config;
use crate::info::BackdropStyle;

/// A look the program can wear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wear {
    /// The program's own look: no skin, no earned world.
    Plain,
    /// A skin from the shop (by its id).
    Skin(&'static str),
    Gold,
    Void,
    Underworld,
}

impl Wear {
    pub fn name(self) -> &'static str {
        match self {
            Wear::Plain => "None",
            Wear::Skin(id) => crate::shop::item(id).map(|i| i.name).unwrap_or("Skin"),
            Wear::Gold => "Gold Edition",
            Wear::Void => "The Void",
            Wear::Underworld => "The Underworld",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Wear::Plain => "The program's own look.",
            Wear::Skin(id) => crate::shop::item(id).map(|i| i.blurb).unwrap_or(""),
            Wear::Gold => "The golden look, earned by beating the first game.",
            Wear::Void => "Black and violet, earned by beating the second game.",
            Wear::Underworld => "Rock slabs on moving lava, earned at the end of the third game.",
        }
    }

    /// Whether the angel cats are the Soul Catsters on this look (they are on Platinum).
    pub fn soul_cats(self) -> bool {
        self == Wear::Skin("platinum")
    }

    /// Whether it can be worn: skins once bought, worlds once earned.
    pub fn unlocked(self, cfg: &Config) -> bool {
        match self {
            Wear::Plain => true,
            Wear::Skin(id) => cfg.shop.owns(id),
            // (a build that holds the story back has no worlds to wear)
            Wear::Gold => crate::release::story() && cfg.gold_unlocked,
            Wear::Void => crate::release::story() && cfg.void_unlocked,
            Wear::Underworld => crate::release::story() && cfg.underworld_visible(),
        }
    }

    /// Whether it is what the program looks like now.
    pub fn is_on(self, cfg: &Config) -> bool {
        match self {
            Wear::Plain => cfg.skin_active().is_none() && !cfg.gold_active() && !cfg.void_active() && !cfg.underworld_active(),
            Wear::Skin(id) => cfg.skin_active() == Some(id),
            Wear::Gold => cfg.gold_active(),
            Wear::Void => cfg.void_active(),
            Wear::Underworld => cfg.underworld_active(),
        }
    }

    /// Puts it on, in place of whatever was on: one look at a time. One that is not unlocked is ignored.
    pub fn equip(self, cfg: &mut Config) {
        if !self.unlocked(cfg) {
            return;
        }
        match self {
            Wear::Plain => {
                cfg.shop.set_skin(None);
                cfg.gold_on = false;
                cfg.void_on = false;
                cfg.underworld_on = false;
                cfg.shop_preview = None;
            }
            Wear::Skin(id) => cfg.use_skin(id),
            Wear::Gold => {
                cfg.gold_on = true;
                cfg.void_on = false;
                cfg.underworld_on = false;
                cfg.shop_preview = None;
            }
            Wear::Void => {
                cfg.void_on = true;
                cfg.gold_on = false;
                cfg.underworld_on = false;
                cfg.shop_preview = None;
            }
            Wear::Underworld => {
                cfg.underworld_on = true;
                cfg.gold_on = false;
                cfg.void_on = false;
                cfg.shop_preview = None;
            }
        }
    }
}

/// Every look that can be put on now: none, then the skins that come with the program, then the skins
/// that were bought (in the shop's order), then the earned skins (Platinum), then the worlds that were earned.
pub fn owned_looks(cfg: &Config) -> Vec<Wear> {
    let mut out = vec![Wear::Plain];
    out.extend(crate::shop::PACKAGED.iter().chain(crate::shop::CATALOG).chain(crate::shop::EARNED).filter(|i| i.kind == crate::shop::Kind::Skin && cfg.shop.owns(i.id)).map(|i| Wear::Skin(i.id)));
    out.extend([Wear::Gold, Wear::Void, Wear::Underworld].into_iter().filter(|w| w.unlocked(cfg)));
    out
}

/// The look the program has now.
pub fn current(cfg: &Config) -> Wear {
    owned_looks(cfg).into_iter().rev().find(|w| *w != Wear::Plain && w.is_on(cfg)).unwrap_or(Wear::Plain)
}

/// A look and the backdrop that was made to go with it.
pub struct Pair {
    pub name: &'static str,
    pub look: Wear,
    pub backdrop: BackdropStyle,
    pub blurb: &'static str,
}

pub const PAIRS: &[Pair] = &[
    Pair { name: "Sakura", look: Wear::Skin("sakura"), backdrop: BackdropStyle::Petals, blurb: "Blossom pink, with petals drifting down." },
    Pair { name: "Synthwave", look: Wear::Skin("synthwave"), backdrop: BackdropStyle::SynthGrid, blurb: "Neon magenta and cyan over a rolling grid." },
    Pair { name: "Aurora", look: Wear::Skin("aurora"), backdrop: BackdropStyle::Northern, blurb: "Green and violet, under slow curtains of light." },
    Pair { name: "Deep Sea", look: Wear::Skin("deepsea"), backdrop: BackdropStyle::Bubbles, blurb: "Navy and teal, with bubbles rising through the light." },
    Pair { name: "Frost", look: Wear::Skin("frost"), backdrop: BackdropStyle::Snowfall, blurb: "A winter pine forest with snow falling." },
    Pair { name: "Terminal", look: Wear::Skin("terminal"), backdrop: BackdropStyle::Scanlines, blurb: "Green text on black, with scanlines and a blinking cursor." },
    Pair { name: "Midnight", look: Wear::Skin("midnight"), backdrop: BackdropStyle::Constellations, blurb: "Deep blue-black, under stars that twinkle and join up." },
    Pair { name: "Copper", look: Wear::Skin("copper"), backdrop: BackdropStyle::Sunbeams, blurb: "Warm charcoal and copper, with beams of light and drifting dust." },
    Pair { name: "Platinum", look: Wear::Skin("platinum"), backdrop: BackdropStyle::AngelCats, blurb: "The cut-gem look, with the Soul Catsters." },
    Pair { name: "Gold Edition", look: Wear::Gold, backdrop: BackdropStyle::AngelCats, blurb: "The golden look, with the angel cats." },
    Pair { name: "The Void", look: Wear::Void, backdrop: BackdropStyle::VoidStars, blurb: "Black and violet, with the glitching stars." },
    Pair { name: "The Underworld", look: Wear::Underworld, backdrop: BackdropStyle::SoulWhispers, blurb: "Rock on moving lava, with souls that whisper." },
];

/// Whether Options lists the pair: all of them, except that the Platinum one waits until Platinum
/// is earned (so that it does not give the surprise away).
pub fn shown(cfg: &Config, pair: &Pair) -> bool {
    // a build that holds the story and the shop back lists only the pairs that come with the program
    (crate::release::story() || packaged(pair)) && (!pair.look.soul_cats() || pair.look.unlocked(cfg))
}

/// Whether both halves of the pair come with the program (a packaged skin and a backdrop that is free).
pub fn packaged(pair: &Pair) -> bool {
    matches!(pair.look, Wear::Skin(id) if crate::shop::is_packaged(id))
}

/// Whether the pair is one of the earned worlds (Gold Edition, The Void), which sit together in
/// their own drop-down.
pub fn is_world(pair: &Pair) -> bool {
    matches!(pair.look, Wear::Gold | Wear::Void | Wear::Underworld)
}

/// Whether the backdrop can be used: bought, or earned with the edition it comes with.
pub fn backdrop_unlocked(cfg: &Config, style: BackdropStyle) -> bool {
    style.shop_id().map_or(true, |id| cfg.shop.owns(id)) && (cfg.gold_unlocked || !style.needs_gold()) && (cfg.void_unlocked || !style.needs_void()) && (cfg.games.underworld() || !style.inside_underworld_only())
}

/// What is still missing before the pair can be put on (by name): the look, the backdrop, or both.
pub fn missing(cfg: &Config, pair: &Pair) -> Vec<&'static str> {
    let mut out = Vec::new();
    if !pair.look.unlocked(cfg) {
        out.push(pair.look.name());
    }
    if !backdrop_unlocked(cfg, pair.backdrop) {
        out.push(pair.backdrop.label_in(pair.look.soul_cats()));
    }
    out
}

pub fn ready(cfg: &Config, pair: &Pair) -> bool {
    missing(cfg, pair).is_empty()
}

/// Whether both halves of the pair are on now.
pub fn is_on(cfg: &Config, pair: &Pair) -> bool {
    pair.look.is_on(cfg) && cfg.backdrop && cfg.backdrop_style == pair.backdrop
}

/// Where the pair stands, in a sentence.
pub fn status(cfg: &Config, pair: &Pair) -> String {
    if is_on(cfg, pair) {
        "On now.".to_string()
    } else {
        let missing = missing(cfg, pair);
        if missing.is_empty() {
            "Ready: both are yours.".to_string()
        } else {
            format!("Needs {}.", missing.join(" and "))
        }
    }
}

/// Puts on both the look and the backdrop (the backdrop is switched on too). Does nothing while
/// either is still missing.
pub fn equip(cfg: &mut Config, pair: &Pair) {
    if !ready(cfg, pair) {
        return;
    }
    pair.look.equip(cfg);
    cfg.backdrop = true;
    cfg.backdrop_style = pair.backdrop;
    cfg.shop_preview = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rich() -> Config {
        let mut cfg = Config::default();
        cfg.shop.earn(2000);
        cfg
    }

    #[test]
    fn the_table_names_real_things() {
        let mut names = std::collections::HashSet::new();
        for pair in PAIRS {
            assert!(names.insert(pair.name), "names are unique");
            assert!(!pair.blurb.is_empty());
            if let Wear::Skin(id) = pair.look {
                assert!(crate::shop::item(id).is_some_and(|i| i.kind == crate::shop::Kind::Skin), "{id} is a skin in the shop");
                assert_eq!(pair.look.name(), pair.name, "a skin's pair has the skin's name");
            }
            if let Some(id) = pair.backdrop.shop_id() {
                assert!(crate::shop::item(id).is_some_and(|i| i.kind == crate::shop::Kind::Backdrop), "{id} is a backdrop in the shop");
            }
        }
        assert_eq!(PAIRS.len(), 12);
    }

    #[test]
    fn a_fresh_program_wears_nothing_and_has_one_look_to_pick() {
        let cfg = Config::default();
        // (the two skins that come with the program are there from the start)
        assert_eq!(owned_looks(&cfg), [Wear::Plain, Wear::Skin("midnight"), Wear::Skin("copper")]);
        assert_eq!(current(&cfg), Wear::Plain);
        assert!(Wear::Plain.is_on(&cfg));
        assert!(PAIRS.iter().all(|p| ready(&cfg, p) == packaged(p)), "only the packaged pairs are ready");
        assert!(!missing(&cfg, &PAIRS[0]).is_empty());
        assert_eq!(PAIRS.iter().filter(|p| packaged(p)).count(), 2);
    }

    #[test]
    fn owned_skins_and_earned_worlds_are_all_there_in_order() {
        let mut cfg = rich();
        cfg.shop.buy("frost").unwrap();
        cfg.shop.buy("sakura").unwrap();
        cfg.shop.buy("snowfall").unwrap();
        cfg.gold_unlocked = true;
        cfg.void_unlocked = true;
        assert_eq!(owned_looks(&cfg), [Wear::Plain, Wear::Skin("midnight"), Wear::Skin("copper"), Wear::Skin("sakura"), Wear::Skin("frost"), Wear::Gold, Wear::Void], "the packaged ones, then the shop's order, backdrops left out");
        cfg.games.open_underworld();
        assert_eq!(*owned_looks(&cfg).last().unwrap(), Wear::Underworld);
    }

    #[test]
    fn a_pair_needs_both_halves_and_says_which_is_missing() {
        let mut cfg = rich();
        let sakura = &PAIRS[0];
        assert_eq!(missing(&cfg, sakura), ["Sakura", "Sakura petals"]);
        cfg.shop.buy("sakura").unwrap();
        assert_eq!(missing(&cfg, sakura), ["Sakura petals"]);
        equip(&mut cfg, sakura);
        assert!(!is_on(&cfg, sakura) && !cfg.sakura_active(), "nothing happens until both are there");
        cfg.shop.buy("petals").unwrap();
        assert!(ready(&cfg, sakura));
        equip(&mut cfg, sakura);
        assert!(is_on(&cfg, sakura) && cfg.sakura_active() && cfg.backdrop && cfg.backdrop_style == BackdropStyle::Petals);
        assert_eq!(current(&cfg), Wear::Skin("sakura"));
    }

    #[test]
    fn putting_on_a_pair_switches_the_backdrop_on_and_takes_the_earned_looks_off() {
        let mut cfg = rich();
        cfg.gold_unlocked = true;
        cfg.gold_on = true;
        cfg.backdrop = false;
        cfg.shop.buy("frost").unwrap();
        cfg.shop.buy("snowfall").unwrap();
        let frost = PAIRS.iter().find(|p| p.name == "Frost").unwrap();
        equip(&mut cfg, frost);
        assert!(cfg.backdrop && !cfg.gold_on && cfg.shop.skin() == Some("frost"));
        assert!(is_on(&cfg, frost));
        cfg.backdrop = false;
        assert!(!is_on(&cfg, frost), "the backdrop is half of it");
    }

    #[test]
    fn the_platinum_pair_has_the_soul_catsters_and_waits_until_platinum_is_earned() {
        let mut cfg = rich();
        let platinum = PAIRS.iter().find(|p| p.name == "Platinum").unwrap();
        assert!(Wear::Skin("platinum").soul_cats() && !Wear::Gold.soul_cats() && !Wear::Skin("frost").soul_cats());
        assert!(!shown(&cfg, platinum));
        assert_eq!(missing(&cfg, platinum), ["Platinum", "Soul Catsters"]);
        cfg.gold_unlocked = true;
        cfg.shop.grant("platinum");
        assert!(shown(&cfg, platinum) && ready(&cfg, platinum));
        equip(&mut cfg, platinum);
        assert!(is_on(&cfg, platinum) && cfg.backdrop_style == BackdropStyle::AngelCats);
        assert_eq!(current(&cfg), Wear::Skin("platinum"));
        assert!(PAIRS.iter().filter(|p| p.name != "Platinum").all(|p| shown(&cfg, p)));
    }

    #[test]
    fn the_earned_worlds_pair_with_their_backdrops() {
        let mut cfg = rich();
        let gold = PAIRS.iter().find(|p| p.name == "Gold Edition").unwrap();
        let void = PAIRS.iter().find(|p| p.name == "The Void").unwrap();
        assert_eq!(missing(&cfg, gold), ["Gold Edition", "Angel cats"]);
        assert_eq!(missing(&cfg, void), ["The Void", "Glitch stars"]);
        cfg.gold_unlocked = true;
        equip(&mut cfg, gold);
        assert!(is_on(&cfg, gold) && cfg.gold_active() && cfg.backdrop_style == BackdropStyle::AngelCats);
        cfg.void_unlocked = true;
        equip(&mut cfg, void);
        assert!(is_on(&cfg, void) && cfg.void_active() && !cfg.gold_active() && !cfg.gold_on);
        assert_eq!(current(&cfg), Wear::Void);
    }

    #[test]
    fn the_underworld_pairs_with_soul_whispers_which_show_only_inside_it() {
        let mut cfg = rich();
        let pair = PAIRS.iter().find(|p| p.name == "The Underworld").unwrap();
        assert!(is_world(pair));
        assert_eq!(missing(&cfg, pair), ["The Underworld", "Soul Whispers"]);
        cfg.games.open_underworld();
        assert!(ready(&cfg, pair));
        equip(&mut cfg, pair);
        assert!(is_on(&cfg, pair) && cfg.underworld_active());
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::SoulWhispers);
        // out of the Underworld the choice is kept but something else is shown
        Wear::Plain.equip(&mut cfg);
        assert_eq!(cfg.backdrop_style, BackdropStyle::SoulWhispers);
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::default());
        Wear::Underworld.equip(&mut cfg);
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::SoulWhispers);
    }

    #[test]
    fn wearing_a_look_puts_one_on_in_place_of_the_last() {
        let mut cfg = rich();
        cfg.shop.buy("aurora").unwrap();
        cfg.shop.buy("prism").unwrap();
        cfg.gold_unlocked = true;
        Wear::Gold.equip(&mut cfg);
        assert_eq!(current(&cfg), Wear::Gold);
        Wear::Skin("aurora").equip(&mut cfg);
        assert_eq!(current(&cfg), Wear::Skin("aurora"));
        assert!(!cfg.gold_on, "a skin takes the earned look off");
        Wear::Gold.equip(&mut cfg);
        assert_eq!(current(&cfg), Wear::Gold);
        assert!(!Wear::Skin("aurora").is_on(&cfg), "the earned look wins over the skin that is still chosen");
        Wear::Skin("prism").equip(&mut cfg);
        assert_eq!(current(&cfg), Wear::Skin("prism"));
        Wear::Plain.equip(&mut cfg);
        assert_eq!(current(&cfg), Wear::Plain);
        assert!(cfg.shop.skin().is_none() && !cfg.gold_on);
    }

    #[test]
    fn a_look_that_is_not_unlocked_cannot_be_put_on() {
        let mut cfg = Config::default();
        let before = (cfg.gold_on, cfg.void_on, cfg.underworld_on);
        Wear::Skin("frost").equip(&mut cfg);
        Wear::Gold.equip(&mut cfg);
        Wear::Void.equip(&mut cfg);
        Wear::Underworld.equip(&mut cfg);
        assert_eq!(current(&cfg), Wear::Plain);
        assert_eq!((cfg.gold_on, cfg.void_on, cfg.underworld_on), before, "nothing was touched");
        assert!(cfg.shop.skin().is_none());
    }

    #[test]
    fn the_underworld_is_a_look_of_its_own() {
        let mut cfg = Config::default();
        cfg.games.open_underworld();
        cfg.gold_unlocked = true;
        cfg.gold_on = true;
        Wear::Underworld.equip(&mut cfg);
        assert_eq!(current(&cfg), Wear::Underworld);
        assert!(!cfg.gold_on);
        assert_eq!(Wear::Underworld.name(), "The Underworld");
    }

    #[test]
    fn the_worlds_are_told_apart_and_the_status_reads_in_a_sentence() {
        let worlds: Vec<&str> = PAIRS.iter().filter(|p| is_world(p)).map(|p| p.name).collect();
        assert_eq!(worlds, ["Gold Edition", "The Void", "The Underworld"]);
        let mut cfg = rich();
        let frost = PAIRS.iter().find(|p| p.name == "Frost").unwrap();
        assert_eq!(status(&cfg, frost), "Needs Frost and Snowfall.");
        cfg.shop.buy("frost").unwrap();
        cfg.shop.buy("snowfall").unwrap();
        assert_eq!(status(&cfg, frost), "Ready: both are yours.");
        equip(&mut cfg, frost);
        assert_eq!(status(&cfg, frost), "On now.");
    }
}
