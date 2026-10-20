//! Ready-made lighting for the keyboard and the cooling pad together, in three tabs. WorldSets: the
//! earned worlds (Gold Edition, The Void, The Underworld). Shop Purchases: the shop's skins (Sakura,
//! Synthwave, Frost, Terminal), once both the skin and its backdrop are bought. Both kinds only exist
//! once the look has been earned or bought, and are put on by themselves when the look is unlocked or
//! switched on (if the lighting toggles are on). A few more (no look needed) are always there, in the
//! Presets tab.
use crate::config::{Config, KbdPart, LightPreset, PadPart};
use librazer::keymap::{KeyFx, KeyLayout, KEYS};

/// A look of the program that has a preset of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Gold,
    Void,
    Underworld,
    /// The shop's skin; its preset needs the skin and its backdrop both bought.
    Sakura,
    /// The shop's Synthwave skin; its preset needs the skin and the grid backdrop both bought.
    Synthwave,
    /// The shop's Frost skin; its preset needs the skin and the Snowfall backdrop both bought.
    Frost,
    /// The shop's Terminal skin; its preset needs the skin and the Scanlines backdrop both bought.
    Terminal,
    /// The Midnight skin that comes with the program (and its Constellations backdrop).
    Midnight,
    /// The Copper skin that comes with the program (and its Sunbeams backdrop).
    Copper,
}

pub struct GamePreset {
    pub id: &'static str,
    pub name: &'static str,
    /// One line about what it does.
    pub blurb: &'static str,
    /// The look it belongs to (`None` = always there).
    pub look: Option<Look>,
}

pub const PRESETS: &[GamePreset] = &[
    GamePreset {
        id: "gold",
        name: "Gold Edition",
        blurb: "Gold stars twinkling on the keyboard and the pad",
        look: Some(Look::Gold),
    },
    GamePreset {
        id: "void",
        name: "The Void",
        blurb: "VOID keys steady and dark purple, the rest breathing light purple and white; \
                the pad burns dark purple and white",
        look: Some(Look::Void),
    },
    GamePreset {
        id: "underworld",
        name: "The Underworld",
        blurb: "The keyboard burns like lava, pale souls rise along the pad",
        look: Some(Look::Underworld),
    },
    GamePreset {
        id: "sakura",
        name: "Sakura",
        blurb: "Pink waves blow across the keyboard, pink stars twinkle along the pad",
        look: Some(Look::Sakura),
    },
    GamePreset {
        id: "synthwave",
        name: "Synthwave",
        blurb: "Magenta and cyan neon rolling across the keyboard, the pad sliding between them",
        look: Some(Look::Synthwave),
    },
    GamePreset {
        id: "frost",
        name: "Frost",
        blurb: "Ice-blue waves rolling over the keyboard, blue stars glittering along the pad",
        look: Some(Look::Frost),
    },
    GamePreset {
        id: "terminal",
        name: "Terminal",
        blurb: "Green waves scrolling across the keyboard, green stars blinking along the pad",
        look: Some(Look::Terminal),
    },
    GamePreset {
        id: "midnight",
        name: "Midnight",
        blurb: "Indigo and violet waves rolling over the keyboard, violet stars twinkling along the pad",
        look: Some(Look::Midnight),
    },
    GamePreset {
        id: "copper",
        name: "Copper",
        blurb: "Copper and amber waves across the keyboard, warm sparks glinting along the pad",
        look: Some(Look::Copper),
    },
    GamePreset {
        id: "deep_sea",
        name: "Deep Sea",
        blurb: "Blue and green waves on both",
        look: None,
    },
    GamePreset {
        id: "aurora",
        name: "Aurora",
        blurb: "Both slide from green through blue to violet and back",
        look: None,
    },
    GamePreset {
        id: "rainbow",
        name: "Rainbow",
        blurb: "Every color of the rainbow at once, scrolling across the keyboard and along the pad",
        look: None,
    },
];

impl Look {
    /// Whether the look is one of the earned worlds (and not a shop purchase).
    pub fn is_world(self) -> bool {
        matches!(self, Look::Gold | Look::Void | Look::Underworld)
    }

    /// The id of the shop skin this look is, for the looks that are bought.
    pub fn skin_id(self) -> Option<&'static str> {
        match self {
            Look::Sakura => Some("sakura"),
            Look::Synthwave => Some("synthwave"),
            Look::Frost => Some("frost"),
            Look::Terminal => Some("terminal"),
            Look::Midnight => Some("midnight"),
            Look::Copper => Some("copper"),
            _ => None,
        }
    }
}

/// The look that is on right now, if any (the one the program is drawn in).
pub fn current_look(cfg: &Config) -> Option<Look> {
    // (trying a skin on, without both of its items bought, does not count: its preset is not there)
    if let Some(look) = [Look::Sakura, Look::Synthwave, Look::Frost, Look::Terminal, Look::Midnight, Look::Copper].into_iter().find(|l| cfg.skin_active() == l.skin_id() && look_unlocked(cfg, *l)) {
        Some(look)
    } else if cfg.underworld_active() {
        Some(Look::Underworld)
    } else if cfg.void_active() {
        Some(Look::Void)
    } else if cfg.gold_active() {
        Some(Look::Gold)
    } else {
        None
    }
}

pub fn look_unlocked(cfg: &Config, look: Look) -> bool {
    match look {
        Look::Sakura => cfg.shop.owns("sakura") && cfg.shop.owns("petals"),
        Look::Synthwave => cfg.shop.owns("synthwave") && cfg.shop.owns("synthgrid"),
        Look::Frost => cfg.shop.owns("frost") && cfg.shop.owns("snowfall"),
        Look::Terminal => cfg.shop.owns("terminal") && cfg.shop.owns("scanlines"),
        Look::Midnight => cfg.shop.owns("midnight"),
        Look::Copper => cfg.shop.owns("copper"),
        Look::Gold => cfg.gold_unlocked,
        Look::Void => cfg.void_unlocked,
        Look::Underworld => cfg.underworld_visible(),
    }
}

/// The presets that can be used now: the always-there ones and those of the looks that were earned.
pub fn available(cfg: &Config) -> Vec<&'static GamePreset> {
    PRESETS.iter().filter(|p| p.look.map_or(true, |look| look_unlocked(cfg, look))).collect()
}

/// The presets of the worlds that were earned: Gold Edition, The Void and The Underworld (the
/// WorldSets tab).
pub fn available_worlds(cfg: &Config) -> Vec<&'static GamePreset> {
    available(cfg).into_iter().filter(|p| p.look.is_some_and(Look::is_world)).collect()
}

/// The presets of the skins bought in the shop, once the skin and its backdrop are both bought (the
/// Shop Purchases tab).
pub fn available_shop(cfg: &Config) -> Vec<&'static GamePreset> {
    available(cfg).into_iter().filter(|p| p.look.is_some_and(|l| !l.is_world())).collect()
}

/// The presets that need no look (the Presets tab).
pub fn available_ready(cfg: &Config) -> Vec<&'static GamePreset> {
    available(cfg).into_iter().filter(|p| p.look.is_none()).collect()
}

/// The preset that goes with `look`.
pub fn for_look(look: Look) -> &'static GamePreset {
    PRESETS.iter().find(|p| p.look == Some(look)).unwrap_or(&PRESETS[0])
}

pub fn by_id(id: &str) -> Option<&'static GamePreset> {
    PRESETS.iter().find(|p| p.id == id)
}

const WHITE: [u8; 3] = [255, 255, 255];
const VOID_KEYS: [u8; 3] = [50, 0, 100];
const VOID_LIGHT: [u8; 3] = [196, 160, 255];

/// The Void's keyboard: V, O, I and D steady in dark purple, every other key breathing, light purple
/// and white by turns along each row.
fn void_layout() -> KeyLayout {
    let mut layout = KeyLayout::default();
    for key in KEYS {
        if matches!(key.id, "V" | "O" | "I" | "D") {
            layout.set_with(key.id, VOID_KEYS, KeyFx::Static);
        } else {
            let color = if key.col % 2 == 0 { VOID_LIGHT } else { WHITE };
            layout.set_with(key.id, color, KeyFx::Breathing);
        }
    }
    layout
}

fn kbd(effect: &str, color: [u8; 3], color2: Option<[u8; 3]>, level: u8) -> KbdPart {
    KbdPart { effect: effect.to_string(), color, color2, split: 0, brightness: level, layout: None }
}

fn pad(mode: &str, color: [u8; 3], color2: Option<[u8; 3]>, level: u8) -> PadPart {
    PadPart { mode: mode.to_string(), color, color2, split: 0, brightness: level }
}

/// The preset to apply. The brightness of the keyboard (`kbd_level`, raw) and of the pad
/// (`pad_percent`) are left as they are now: a preset sets the look, not how bright it is.
pub fn build(preset: &GamePreset, kbd_level: u8, pad_percent: u8) -> LightPreset {
    let (k, p) = match preset.id {
        "gold" => {
            let gold = [255, 196, 40];
            (
                kbd("starlight", gold, Some(WHITE), kbd_level),
                pad("Starlight", gold, Some(WHITE), pad_percent),
            )
        }
        "void" => (
            KbdPart {
                effect: "custom".to_string(),
                color: VOID_KEYS,
                color2: None,
                split: 0,
                brightness: kbd_level,
                layout: Some(void_layout().encode()),
            },
            pad("Fire", WHITE, Some([28, 0, 60]), pad_percent),
        ),
        "underworld" => (
            kbd("fire", [255, 100, 0], Some([110, 0, 0]), kbd_level),
            pad("Starlight", [120, 255, 170], None, pad_percent),
        ),
        "sakura" => (
            // LEDs read as white when the green channel is up, so the pinks have none at all: a
            // rose and a hot pink, red and blue only
            kbd("tidal", [255, 0, 90], Some([255, 0, 160]), kbd_level),
            pad("Starlight", [255, 0, 100], Some([255, 0, 170]), pad_percent),
        ),
        "synthwave" => (
            kbd("tidal", [255, 0, 170], Some([0, 220, 255]), kbd_level),
            pad("Color Dipp", [255, 0, 170], Some([0, 220, 255]), pad_percent),
        ),
        "frost" => (
            // blues only, with no red: red and green together would read as white
            kbd("tidal", [0, 70, 255], Some([0, 190, 255]), kbd_level),
            pad("Starlight", [0, 120, 255], Some([0, 200, 255]), pad_percent),
        ),
        "terminal" => (
            kbd("tidal", [0, 255, 40], Some([0, 110, 20]), kbd_level),
            pad("Starlight", [0, 255, 60], Some([0, 150, 30]), pad_percent),
        ),
        "midnight" => (
            // red and blue only, so that it stays violet and does not read as white
            kbd("tidal", [40, 0, 255], Some([150, 0, 255]), kbd_level),
            pad("Starlight", [70, 0, 255], Some([170, 0, 255]), pad_percent),
        ),
        "copper" => (
            kbd("tidal", [255, 30, 0], Some([255, 110, 0]), kbd_level),
            pad("Starlight", [255, 60, 0], Some([255, 130, 0]), pad_percent),
        ),
        "deep_sea" => (
            kbd("tidal", [0, 110, 255], Some([0, 255, 200]), kbd_level),
            pad("Tidal", [0, 110, 255], Some([0, 255, 200]), pad_percent),
        ),
        "aurora" => (
            kbd("dipp", [0, 255, 90], Some([150, 0, 255]), kbd_level),
            pad("Color Dipp", [0, 255, 90], Some([150, 0, 255]), pad_percent),
        ),
        _ => (kbd("rainbow", WHITE, None, kbd_level), pad("Rainbow", WHITE, None, pad_percent)),
    };
    LightPreset { name: preset.name.to_string(), kbd: Some(k), pad: Some(p) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librazer::command::KeyboardEffect;
    use librazer::cooling_pad::PadLightingMode;

    #[test]
    fn only_the_always_there_presets_exist_until_a_look_is_earned() {
        let mut cfg = Config::default();
        let names: Vec<_> = available(&cfg).iter().map(|p| p.id).collect();
        // (the two skins that come with the program have theirs from the start)
        assert_eq!(names, ["midnight", "copper", "deep_sea", "aurora", "rainbow"]);
        cfg.void_unlocked = true;
        assert!(available(&cfg).iter().any(|p| p.id == "void"));
        assert!(!available(&cfg).iter().any(|p| p.id == "gold" || p.id == "underworld"));
        cfg.gold_unlocked = true;
        cfg.games.open_underworld();
        assert_eq!(available(&cfg).len(), PRESETS.len() - 4, "the shop's are still missing");
        // Sakura needs both of its shop items, the skin and the backdrop
        cfg.shop.earn(200);
        cfg.shop.buy("sakura").unwrap();
        assert!(!available(&cfg).iter().any(|p| p.id == "sakura"), "the skin alone is not enough");
        cfg.shop.buy("petals").unwrap();
        cfg.shop.earn(200);
        cfg.shop.buy("synthwave").unwrap();
        cfg.shop.buy("synthgrid").unwrap();
        cfg.shop.earn(500);
        cfg.shop.buy("frost").unwrap();
        assert!(!available(&cfg).iter().any(|p| p.id == "frost"), "the skin alone is not enough");
        cfg.shop.buy("snowfall").unwrap();
        cfg.shop.buy("terminal").unwrap();
        cfg.shop.buy("scanlines").unwrap();
        assert_eq!(available(&cfg).len(), PRESETS.len());
        // the look-free ones and the earned ones are told apart
        let ready: Vec<_> = available_ready(&cfg).iter().map(|p| p.id).collect();
        assert_eq!(ready, ["deep_sea", "aurora", "rainbow"]);
        let worlds: Vec<_> = available_worlds(&cfg).iter().map(|p| p.id).collect();
        assert_eq!(worlds, ["gold", "void", "underworld"]);
        let shop: Vec<_> = available_shop(&cfg).iter().map(|p| p.id).collect();
        assert_eq!(shop, ["sakura", "synthwave", "frost", "terminal", "midnight", "copper"]);
        assert!(available_worlds(&Config::default()).is_empty());
        let fresh: Vec<_> = available_shop(&Config::default()).iter().map(|p| p.id).collect();
        assert_eq!(fresh, ["midnight", "copper"], "the two that come with the program");
        // every preset is in exactly one tab
        assert_eq!(ready.len() + worlds.len() + shop.len(), PRESETS.len());
    }

    #[test]
    fn sakura_is_the_look_while_its_skin_is_on_and_both_items_are_bought() {
        let mut cfg = Config::default();
        cfg.shop.earn(200);
        cfg.shop.buy("sakura").unwrap();
        cfg.use_skin("sakura");
        assert!(cfg.sakura_active());
        assert_eq!(current_look(&cfg), None, "the backdrop is not bought yet");
        cfg.shop.buy("petals").unwrap();
        assert_eq!(current_look(&cfg), Some(Look::Sakura));
        assert_eq!(for_look(Look::Sakura).id, "sakura");
        // trying the skin on without owning it gives no preset either
        let mut other = Config::default();
        other.shop_preview = Some("sakura".to_string());
        assert!(other.sakura_active() && current_look(&other).is_none());
        let built = build(by_id("sakura").unwrap(), 10, 20);
        assert_eq!(built.kbd.unwrap().effect, "tidal");
        assert_eq!(built.pad.unwrap().mode, "Starlight");
    }

    #[test]
    fn synthwave_needs_its_skin_and_its_grid_and_the_sakura_pinks_have_no_green() {
        let mut cfg = Config::default();
        cfg.shop.earn(500);
        cfg.shop.buy("synthwave").unwrap();
        cfg.use_skin("synthwave");
        assert!(cfg.synthwave_active() && current_look(&cfg).is_none());
        cfg.shop.buy("synthgrid").unwrap();
        assert_eq!(current_look(&cfg), Some(Look::Synthwave));
        cfg.shop.buy("sakura").unwrap();
        cfg.use_skin("sakura");
        assert_eq!(current_look(&cfg), None, "Sakura is on but its backdrop is not bought");
        let built = build(by_id("sakura").unwrap(), 10, 20);
        let (k, p) = (built.kbd.unwrap(), built.pad.unwrap());
        for c in [Some(k.color), k.color2, Some(p.color), p.color2].into_iter().flatten() {
            assert_eq!(c[1], 0, "no green, or the LEDs read as white");
            assert_eq!(c[0], 255);
        }
    }

    #[test]
    fn frost_and_terminal_each_need_their_skin_and_their_backdrop() {
        let mut cfg = Config::default();
        cfg.shop.earn(500);
        for (skin, backdrop, look) in [("frost", "snowfall", Look::Frost), ("terminal", "scanlines", Look::Terminal)] {
            cfg.shop.buy(skin).unwrap();
            cfg.use_skin(skin);
            assert_eq!(current_look(&cfg), None, "{skin} is on but its backdrop is not bought");
            cfg.shop.buy(backdrop).unwrap();
            assert_eq!(current_look(&cfg), Some(look));
            assert_eq!(for_look(look).id, skin);
        }
        // the lights follow the look: blue for Frost with no red, green for Terminal with no blue
        let (k, p) = { let b = build(by_id("frost").unwrap(), 10, 20); (b.kbd.unwrap(), b.pad.unwrap()) };
        for c in [Some(k.color), k.color2, Some(p.color), p.color2].into_iter().flatten() {
            assert_eq!(c[0], 0, "no red");
            assert_eq!(c[2], 255);
        }
        let (k, p) = { let b = build(by_id("terminal").unwrap(), 10, 20); (b.kbd.unwrap(), b.pad.unwrap()) };
        for c in [Some(k.color), k.color2, Some(p.color), p.color2].into_iter().flatten() {
            assert_eq!((c[0], c[2] <= 60), (0, true), "green only: {c:?}");
        }
    }

    #[test]
    fn the_rainbow_preset_uses_the_rainbow_effect_on_both() {
        let built = build(by_id("rainbow").unwrap(), 10, 20);
        assert_eq!(built.kbd.unwrap().effect, "rainbow");
        assert_eq!(built.pad.unwrap().mode, "Rainbow");
    }

    #[test]
    fn the_look_that_is_on_is_the_one_that_wins() {
        let mut cfg = Config::default();
        assert_eq!(current_look(&cfg), None);
        cfg.gold_unlocked = true;
        cfg.gold_on = true;
        assert_eq!(current_look(&cfg), Some(Look::Gold));
        cfg.void_unlocked = true;
        cfg.void_on = true;
        assert_eq!(current_look(&cfg), Some(Look::Void));
        cfg.games.open_underworld();
        cfg.underworld_on = true;
        assert_eq!(current_look(&cfg), Some(Look::Underworld));
        assert_eq!(for_look(Look::Void).id, "void");
    }

    #[test]
    fn every_preset_uses_effects_the_app_knows_and_keeps_the_brightness() {
        for preset in PRESETS {
            let built = build(preset, 77, 33);
            let k = built.kbd.expect("a keyboard part");
            let p = built.pad.expect("a pad part");
            assert!(KeyboardEffect::from_key(&k.effect).is_some(), "{}: {}", preset.id, k.effect);
            assert!(PadLightingMode::from_name(&p.mode).is_some(), "{}: {}", preset.id, p.mode);
            assert_eq!((k.brightness, p.brightness), (77, 33));
            assert_eq!(built.name, preset.name);
            assert!(crate::config::clean_preset_name(&built.name) == built.name);
        }
    }

    #[test]
    fn the_void_keyboard_keeps_void_steady_and_breathes_the_rest() {
        let built = build(by_id("void").unwrap(), 255, 100);
        let layout = KeyLayout::decode(built.kbd.unwrap().layout.as_deref().unwrap()).unwrap();
        for id in ["V", "O", "I", "D"] {
            assert_eq!(layout.fx_of(id), KeyFx::Static);
            assert_eq!(layout.color_of(id), Some(VOID_KEYS));
        }
        for id in ["A", "SPACE", "ESC", "ENTER"] {
            assert_eq!(layout.fx_of(id), KeyFx::Breathing, "{}", id);
        }
        assert!(layout.is_animated());
        assert_eq!(layout.keys.len(), KEYS.len());
        // light purple and white by turns
        assert_ne!(layout.color_of("Q"), layout.color_of("W"));
        let pad = build(by_id("void").unwrap(), 255, 100).pad.unwrap();
        assert_eq!((pad.mode.as_str(), pad.color, pad.color2), ("Fire", WHITE, Some([28, 0, 60])));
    }
}
