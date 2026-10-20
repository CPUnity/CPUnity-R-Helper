// Global accent color (the "highlight green"), changeable at runtime from the Options window.
use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Margin, Shape, Stroke, TextStyle};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

// Packed 0x00RRGGBB. Default matches the original R-Helper green (0, 120, 60).
static ACCENT: AtomicU32 = AtomicU32::new(0x0000_783C);

/// Size of normal text, in points. Tabs that show bigger text scale everything by their own
/// body size divided by this.
pub const BODY_SIZE: f32 = 14.5;

pub const DEFAULT_ACCENT: Color32 = Color32::from_rgb(0, 120, 60);

pub fn set_accent(c: Color32) {
    let packed = ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | (c.b() as u32);
    ACCENT.store(packed, Ordering::Relaxed);
}

pub fn accent() -> Color32 {
    match skin() {
        Skin::None => {}
        Skin::Fraying => {
            // the chosen accent, washed out toward ash
            let v = ACCENT.load(Ordering::Relaxed);
            let chosen = Color32::from_rgb(((v >> 16) & 0xff) as u8, ((v >> 8) & 0xff) as u8, (v & 0xff) as u8);
            return mix(chosen, Color32::from_rgb(150, 148, 142), 0.45);
        }
        Skin::Prism => {
            let [r, g, b] = crate::skins::hsv(f32::from_bits(PRISM_HUE.load(Ordering::Relaxed)), 0.62, 1.0);
            return Color32::from_rgb(r, g, b);
        }
        other => return skin_colors(other).accent,
    }
    if is_underworld() {
        return UNDERWORLD_ACCENT;
    }
    if is_void() {
        return VOID_ACCENT;
    }
    if is_gold() {
        return GOLD_ACCENT;
    }
    let v = ACCENT.load(Ordering::Relaxed);
    Color32::from_rgb(((v >> 16) & 0xff) as u8, ((v >> 8) & 0xff) as u8, (v & 0xff) as u8)
}

/// Dimmed accent for unselected-but-visible buttons (blend toward dark grey).
pub fn accent_dim() -> Color32 {
    mix(accent(), Color32::from_rgb(40, 40, 40), 0.65)
}

/// Muted accent used for the "Custom is active" fill.
pub fn accent_muted_fill() -> Color32 {
    mix(accent(), Color32::from_rgb(30, 30, 30), 0.35)
}

/// Lighter accent used for outlines.
pub fn accent_stroke() -> Color32 {
    mix(accent(), Color32::WHITE, 0.30)
}

pub fn mix(a: Color32, b: Color32, t_toward_b: f32) -> Color32 {
    let m = |x: u8, y: u8| -> u8 {
        ((x as f32) * (1.0 - t_toward_b) + (y as f32) * t_toward_b).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

// ---- Gold Edition (the reward for beating Father Time) ----

static GOLD: AtomicBool = AtomicBool::new(false);

/// The gold accent: warm, bright, readable as text on the dark background.
pub const GOLD_ACCENT: Color32 = Color32::from_rgb(232, 184, 74);

/// Switches the Gold Edition look on or off (palette, accent, cards, toggles).
pub fn set_gold(on: bool) {
    GOLD.store(on, Ordering::Relaxed);
}

/// Whether the Gold Edition look is on.
pub fn is_gold() -> bool {
    GOLD.load(Ordering::Relaxed)
}

// ---- The Void (the reward for beating the cat king) ----

static VOID: AtomicBool = AtomicBool::new(false);

/// The Void's accent: a bright violet.
pub const VOID_ACCENT: Color32 = Color32::from_rgb(138, 92, 245);
/// The two colours the Void's glitches split into.
pub const VOID_CYAN: Color32 = Color32::from_rgb(63, 240, 255);
pub const VOID_MAGENTA: Color32 = Color32::from_rgb(255, 63, 200);

/// Switches The Void's look on or off. It wins over the Gold Edition if both are asked for.
pub fn set_void(on: bool) {
    VOID.store(on, Ordering::Relaxed);
}

pub fn is_void() -> bool {
    VOID.load(Ordering::Relaxed)
}

// ---- The Underworld (opened by the end of Game 3) ----

static UNDERWORLD: AtomicBool = AtomicBool::new(false);

/// The Underworld's accent: lava orange.
pub const UNDERWORLD_ACCENT: Color32 = Color32::from_rgb(226, 88, 26);

/// Switches the Underworld look on or off. It wins over The Void and the Gold Edition.
pub fn set_underworld(on: bool) {
    UNDERWORLD.store(on, Ordering::Relaxed);
}

pub fn is_underworld() -> bool {
    UNDERWORLD.load(Ordering::Relaxed)
}

static PLATINUM_FX: AtomicBool = AtomicBool::new(true);

/// Whether the Platinum look's moving light is on (the sweep over the cards, the gems' flashes).
pub fn set_platinum_effects(on: bool) {
    PLATINUM_FX.store(on, Ordering::Relaxed);
}

pub fn platinum_effects() -> bool {
    PLATINUM_FX.load(Ordering::Relaxed)
}

// ---- Skins (bought in the shop) ----

/// A skin from the shop: a whole look for the program, bought once. Only one is on at a time, and
/// only while none of the earned looks is (the caller decides).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Skin {
    #[default]
    None,
    /// Ash-grey, the accent washed out, cards with ragged edges.
    Fraying,
    /// Ink-dark plum and indigo, cream paper text, blossom pink and vermilion, shoji-frame cards.
    Sakura,
    /// Deep indigo, hot magenta and cyan, neon-edged cards.
    Synthwave,
    /// Near black, with an accent and a card edge that go round the colour wheel.
    Prism,
    /// Dark teal, northern-lights green and violet, a shimmering edge.
    Aurora,
    /// Navy and teal, a string of bubbles on every card.
    DeepSea,
    /// Ice blue: snow on the cards, icicles, cracks, a pine tree, and a pine forest behind.
    Frost,
    /// Green text on black, square cards with brackets at the corners, monospaced type.
    Terminal,
    /// Near black with a red glow from below, sharp square cards (a premium skin).
    Descent,
    /// Deep blue-black with a periwinkle accent and a starlit line along the top of each card. Comes
    /// with the program.
    Midnight,
    /// Warm charcoal with a copper accent and a copper corner on each card. Comes with the program.
    Copper,
    /// A cut-gem look: brushed platinum on blue-black, glimmering crystal facets with a band of light
    /// that sweeps across, bevelled cards with gems, and prism glints. Earned with the Platinum
    /// trophy, never sold.
    Platinum,
}

impl Skin {
    /// The skin with this shop id (None for anything else).
    pub fn from_id(id: Option<&str>) -> Skin {
        match id {
            Some("fraying") => Skin::Fraying,
            Some("sakura") => Skin::Sakura,
            Some("synthwave") => Skin::Synthwave,
            Some("prism") => Skin::Prism,
            Some("aurora") => Skin::Aurora,
            Some("deepsea") => Skin::DeepSea,
            Some("frost") => Skin::Frost,
            Some("terminal") => Skin::Terminal,
            Some("descent") => Skin::Descent,
            Some("midnight") => Skin::Midnight,
            Some("copper") => Skin::Copper,
            Some("platinum") => Skin::Platinum,
            _ => Skin::None,
        }
    }

    fn to_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(v: u8) -> Skin {
        [Skin::None, Skin::Fraying, Skin::Sakura, Skin::Synthwave, Skin::Prism, Skin::Aurora, Skin::DeepSea, Skin::Frost, Skin::Terminal, Skin::Descent, Skin::Midnight, Skin::Copper, Skin::Platinum]
            .get(v as usize).copied().unwrap_or_default()
    }

    /// Whether its card edge moves (so the program has to keep redrawing).
    pub fn is_animated(self) -> bool {
        matches!(self, Skin::Prism | Skin::Aurora | Skin::Platinum)
    }
}

static SKIN: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// Prism's hue (0 to 1), as the bits of an f32; `set_prism_time` keeps it moving.
static PRISM_HUE: AtomicU32 = AtomicU32::new(0);

/// Puts a skin on (`Skin::None` takes it off).
pub fn set_skin(skin: Skin) {
    SKIN.store(skin.to_u8(), Ordering::Relaxed);
}

pub fn skin() -> Skin {
    Skin::from_u8(SKIN.load(Ordering::Relaxed))
}

/// Tells Prism what time it is, in seconds, so that its accent goes round the colour wheel.
pub fn set_prism_time(t: f32) {
    PRISM_HUE.store(crate::skins::prism_hue(t).to_bits(), Ordering::Relaxed);
}

pub fn is_fraying() -> bool {
    skin() == Skin::Fraying
}

pub fn is_sakura() -> bool {
    skin() == Skin::Sakura
}

/// Sakura's second colour, the vermilion of a shrine gate and a name seal.
pub const SAKURA_VERMILION: Color32 = Color32::from_rgb(200, 58, 48);
pub const SAKURA_ACCENT: Color32 = Color32::from_rgb(238, 146, 176);
/// Synthwave's two neon colours.
pub const SYNTH_MAGENTA: Color32 = Color32::from_rgb(255, 46, 196);
pub const SYNTH_CYAN: Color32 = Color32::from_rgb(0, 229, 255);

/// What a skin paints the program in.
#[derive(Clone, Copy)]
struct SkinColors {
    bg: Color32,
    surface: Color32,
    surface_2: Color32,
    border: Color32,
    border_light: Color32,
    text: Color32,
    text_dim: Color32,
    accent: Color32,
    /// Text on a button filled with the accent.
    on_accent: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn skin_colors(skin: Skin) -> SkinColors {
    match skin {
        Skin::Sakura => SkinColors {
            bg: rgb(15, 11, 18),
            surface: rgb(27, 20, 29),
            surface_2: rgb(41, 30, 43),
            border: rgb(86, 52, 66),
            border_light: rgb(142, 88, 108),
            text: rgb(248, 236, 232),
            text_dim: rgb(206, 166, 176),
            accent: SAKURA_ACCENT,
            on_accent: rgb(50, 16, 32),
        },
        Skin::Synthwave => SkinColors {
            bg: rgb(14, 6, 28),
            surface: rgb(26, 12, 48),
            surface_2: rgb(42, 20, 72),
            border: rgb(98, 28, 120),
            border_light: rgb(190, 50, 190),
            text: rgb(255, 236, 250),
            text_dim: rgb(198, 160, 232),
            accent: SYNTH_MAGENTA,
            on_accent: rgb(24, 4, 38),
        },
        Skin::Prism => SkinColors {
            bg: rgb(11, 11, 14),
            surface: rgb(21, 21, 26),
            surface_2: rgb(33, 33, 41),
            border: rgb(52, 52, 64),
            border_light: rgb(92, 92, 112),
            text: rgb(240, 240, 248),
            text_dim: rgb(162, 162, 184),
            accent: rgb(160, 160, 255),
            on_accent: rgb(12, 12, 20),
        },
        Skin::Aurora => SkinColors {
            bg: rgb(6, 14, 16),
            surface: rgb(11, 26, 28),
            surface_2: rgb(18, 42, 46),
            border: rgb(24, 80, 72),
            border_light: rgb(40, 150, 120),
            text: rgb(230, 252, 244),
            text_dim: rgb(150, 205, 190),
            accent: rgb(60, 240, 150),
            on_accent: rgb(4, 28, 20),
        },
        Skin::DeepSea => SkinColors {
            bg: rgb(5, 13, 24),
            surface: rgb(9, 26, 42),
            surface_2: rgb(14, 40, 62),
            border: rgb(18, 72, 90),
            border_light: rgb(32, 138, 150),
            text: rgb(220, 244, 248),
            text_dim: rgb(136, 186, 198),
            accent: rgb(38, 196, 198),
            on_accent: rgb(2, 24, 30),
        },
        Skin::Frost => SkinColors {
            bg: rgb(8, 14, 22),
            surface: rgb(14, 24, 36),
            surface_2: rgb(22, 37, 54),
            border: rgb(48, 82, 108),
            border_light: rgb(98, 152, 184),
            text: rgb(232, 244, 252),
            text_dim: rgb(150, 184, 206),
            accent: rgb(120, 204, 240),
            on_accent: rgb(5, 24, 38),
        },
        Skin::Terminal => SkinColors {
            bg: rgb(2, 9, 4),
            surface: rgb(5, 19, 9),
            surface_2: rgb(9, 31, 15),
            border: rgb(18, 84, 38),
            border_light: rgb(36, 168, 68),
            text: rgb(150, 255, 172),
            text_dim: rgb(76, 170, 100),
            accent: rgb(52, 255, 104),
            on_accent: rgb(2, 22, 8),
        },
        Skin::Descent => SkinColors {
            bg: rgb(4, 2, 3),
            surface: rgb(20, 10, 12),
            surface_2: rgb(31, 16, 19),
            border: rgb(58, 26, 31),
            border_light: rgb(112, 50, 58),
            text: rgb(217, 200, 200),
            text_dim: rgb(140, 119, 120),
            accent: rgb(255, 42, 60),
            on_accent: rgb(255, 242, 242),
        },
        Skin::Midnight => SkinColors {
            bg: rgb(7, 9, 20),
            surface: rgb(13, 17, 34),
            surface_2: rgb(21, 27, 52),
            border: rgb(36, 46, 92),
            border_light: rgb(86, 104, 190),
            text: rgb(228, 234, 252),
            text_dim: rgb(146, 158, 204),
            accent: rgb(138, 160, 255),
            on_accent: rgb(6, 10, 30),
        },
        Skin::Copper => SkinColors {
            bg: rgb(17, 14, 12),
            surface: rgb(29, 24, 20),
            surface_2: rgb(42, 35, 29),
            border: rgb(74, 56, 42),
            border_light: rgb(150, 104, 70),
            text: rgb(240, 228, 214),
            text_dim: rgb(186, 160, 138),
            accent: rgb(214, 128, 72),
            on_accent: rgb(30, 14, 6),
        },
        Skin::Platinum => SkinColors {
            bg: rgb(9, 11, 16),
            surface: rgb(54, 59, 70),
            surface_2: rgb(33, 38, 49),
            border: rgb(86, 95, 114),
            border_light: rgb(176, 188, 208),
            text: rgb(242, 246, 252),
            text_dim: rgb(170, 180, 198),
            accent: rgb(216, 230, 246),
            on_accent: rgb(12, 16, 24),
        },
        // Fraying (and no skin): the ash palette
        _ => SkinColors {
            bg: rgb(19, 19, 18),
            surface: rgb(31, 31, 30),
            surface_2: rgb(43, 43, 41),
            border: rgb(64, 63, 60),
            border_light: rgb(92, 91, 86),
            text: rgb(214, 212, 205),
            text_dim: rgb(150, 148, 141),
            accent: rgb(150, 148, 142),
            on_accent: Color32::WHITE,
        },
    }
}

/// The colours of the skin that is on, if one is.
fn on_skin() -> Option<SkinColors> {
    match skin() {
        Skin::None => None,
        s => Some(skin_colors(s)),
    }
}

// ---- previews (Options > UI Options) ----

/// A look the program can have, for drawing a small preview of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LookKind {
    /// The program's own look.
    Standard,
    Skin(Skin),
    Gold,
    Void,
    Underworld,
}

/// The colours (and how round the cards are) of a look, for a preview. It does not matter which look
/// is on now.
#[derive(Clone, Copy)]
pub struct Swatch {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_2: Color32,
    pub border: Color32,
    pub border_light: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub radius: u8,
}

pub fn swatch(look: LookKind) -> Swatch {
    let of = |c: SkinColors, radius: u8| Swatch {
        bg: c.bg,
        surface: c.surface,
        surface_2: c.surface_2,
        border: c.border,
        border_light: c.border_light,
        text: c.text,
        text_dim: c.text_dim,
        accent: c.accent,
        radius,
    };
    let plain = |bg, surface, surface_2, border, border_light, text, text_dim, accent| Swatch { bg, surface, surface_2, border, border_light, text, text_dim, accent, radius: 12 };
    match look {
        LookKind::Skin(Skin::None) | LookKind::Standard => plain(
            rgb(13, 14, 17),
            rgb(22, 24, 29),
            rgb(33, 36, 43),
            rgb(44, 48, 56),
            rgb(62, 67, 77),
            rgb(226, 229, 235),
            rgb(150, 156, 168),
            DEFAULT_ACCENT,
        ),
        LookKind::Skin(skin) => {
            let (cards, _) = {
                let saved = SKIN.load(Ordering::Relaxed);
                SKIN.store(skin.to_u8(), Ordering::Relaxed);
                let r = skin_radii();
                SKIN.store(saved, Ordering::Relaxed);
                r
            };
            of(skin_colors(skin), cards)
        }
        LookKind::Gold => plain(rgb(12, 10, 6), rgb(24, 20, 12), rgb(38, 31, 17), rgb(107, 77, 18), rgb(150, 108, 28), rgb(238, 229, 206), rgb(214, 196, 150), GOLD_ACCENT),
        LookKind::Void => plain(rgb(7, 4, 15), rgb(17, 10, 34), rgb(28, 17, 56), rgb(58, 36, 112), rgb(92, 62, 172), rgb(221, 213, 245), rgb(160, 148, 204), VOID_ACCENT),
        LookKind::Underworld => plain(rgb(12, 5, 4), rgb(34, 20, 18), rgb(48, 30, 27), rgb(98, 54, 34), rgb(146, 80, 46), rgb(242, 228, 214), rgb(204, 170, 150), UNDERWORLD_ACCENT),
    }
}

/// How round the corners of cards and widgets are in the skin that is on: (cards, widgets).
fn skin_radii() -> (u8, u8) {
    match skin() {
        Skin::None => (12, 8),
        Skin::Fraying => (2, 3),
        Skin::Sakura => (7, 5),
        Skin::Synthwave => (3, 3),
        Skin::Prism | Skin::Aurora => (10, 7),
        Skin::DeepSea => (12, 9),
        Skin::Frost => (6, 5),
        Skin::Terminal | Skin::Descent => (0, 0),
        Skin::Platinum => (6, 5),
        Skin::Midnight => (8, 6),
        Skin::Copper => (4, 4),
    }
}

/// The space between two cards: wider in the Underworld, where the lava shows between the slabs.
pub fn card_gap() -> f32 {
    if is_underworld() {
        crate::underworld::CARD_GAP
    } else {
        8.0
    }
}

/// Text on top of an accent-coloured button: white normally (and in the Void), dark bronze on gold.
pub fn on_accent() -> Color32 {
    if let Some(c) = on_skin() {
        c.on_accent
    } else if is_underworld() {
        Color32::WHITE
    } else if is_gold() && !is_void() {
        Color32::from_rgb(42, 26, 2)
    } else {
        Color32::WHITE
    }
}

/// Text colour for a button filled with `fill`: dark on the gold accent, otherwise white (light
/// warm white in the Gold Edition). In a skin: its dark ink on the accent, its text colour elsewhere.
pub fn label_on(fill: Color32) -> Color32 {
    if on_skin().is_some() {
        if fill == accent() { on_accent() } else { text() }
    } else if is_void() || is_underworld() {
        Color32::WHITE
    } else if is_gold() && fill == GOLD_ACCENT {
        on_accent()
    } else if is_gold() {
        text()
    } else {
        Color32::WHITE
    }
}

// ---- palette (dark, slightly cool greys; warm black and bronze in the Gold Edition; black and
// violet in the Void) ----
fn pick(standard: Color32, gold: Color32, void: Color32) -> Color32 {
    if is_void() {
        void
    } else if is_gold() {
        gold
    } else {
        standard
    }
}
/// Like `pick`, with the Underworld's colour first (it wins over the other looks).
fn pick4(standard: Color32, gold: Color32, void: Color32, underworld: Color32) -> Color32 {
    if is_underworld() {
        underworld
    } else {
        pick(standard, gold, void)
    }
}
/// A palette colour: the skin's when one is on, otherwise `pick4`'s.
macro_rules! palette {
    ($field:ident, $std:expr, $gold:expr, $void:expr, $under:expr) => {
        match on_skin() {
            Some(c) => c.$field,
            None => pick4($std, $gold, $void, $under),
        }
    };
}
pub fn bg() -> Color32 {
    palette!(bg, Color32::from_rgb(13, 14, 17), Color32::from_rgb(12, 10, 6), Color32::from_rgb(7, 4, 15), Color32::from_rgb(12, 5, 4))
}
pub fn surface() -> Color32 {
    palette!(surface, Color32::from_rgb(22, 24, 29), Color32::from_rgb(24, 20, 12), Color32::from_rgb(17, 10, 34), Color32::from_rgb(34, 20, 18))
}
pub fn surface_2() -> Color32 {
    palette!(surface_2, Color32::from_rgb(33, 36, 43), Color32::from_rgb(38, 31, 17), Color32::from_rgb(28, 17, 56), Color32::from_rgb(48, 30, 27))
}
pub fn border() -> Color32 {
    palette!(border, Color32::from_rgb(44, 48, 56), Color32::from_rgb(107, 77, 18), Color32::from_rgb(58, 36, 112), Color32::from_rgb(98, 54, 34))
}
pub fn border_light() -> Color32 {
    palette!(border_light, Color32::from_rgb(62, 67, 77), Color32::from_rgb(150, 108, 28), Color32::from_rgb(92, 62, 172), Color32::from_rgb(146, 80, 46))
}
pub fn text() -> Color32 {
    palette!(text, Color32::from_rgb(226, 229, 235), Color32::from_rgb(238, 229, 206), Color32::from_rgb(221, 213, 245), Color32::from_rgb(242, 228, 214))
}
pub fn text_dim() -> Color32 {
    palette!(text_dim, Color32::from_rgb(150, 156, 168), Color32::from_rgb(214, 196, 150), Color32::from_rgb(160, 148, 204), Color32::from_rgb(204, 170, 150))
}

/// Applies the full app theme (colors, rounding, spacing, text sizes) to egui.
/// Called at startup and whenever the accent color changes.
pub fn apply(ctx: &egui::Context) {
    let accent = accent();
    ctx.style_mut(|style| {
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = bg();
        v.window_fill = surface();
        v.window_stroke = Stroke::new(1.0, border_light());
        v.window_corner_radius = CornerRadius::same(skin_radii().0);
        v.menu_corner_radius = CornerRadius::same(8);
        v.extreme_bg_color = Color32::from_rgb(10, 11, 13);
        v.faint_bg_color = surface_2();
        v.slider_trailing_fill = true;
        v.selection.bg_fill = accent;
        v.selection.stroke = Stroke::new(1.0, on_accent());

        let r = CornerRadius::same(skin_radii().1);
        v.widgets.noninteractive.corner_radius = r;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border());
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_dim());

        v.widgets.inactive.corner_radius = r;
        v.widgets.inactive.bg_fill = surface_2();
        v.widgets.inactive.weak_bg_fill = surface_2();
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, border_light());
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, text());

        let hover_fill = mix(surface_2(), accent, 0.28);
        v.widgets.hovered.corner_radius = r;
        v.widgets.hovered.bg_fill = hover_fill;
        v.widgets.hovered.weak_bg_fill = hover_fill;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent_stroke());
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);

        v.widgets.active.corner_radius = r;
        v.widgets.active.bg_fill = accent;
        v.widgets.active.weak_bg_fill = accent;
        v.widgets.active.bg_stroke = Stroke::new(1.0, accent_stroke());
        // (bold text is drawn in this colour too, so it must stay light even on gold)
        v.widgets.active.fg_stroke = Stroke::new(1.5, if is_gold() || is_void() || is_underworld() || on_skin().is_some() { text() } else { Color32::WHITE });
        v.weak_text_alpha = if is_gold() || is_void() || is_underworld() || on_skin().is_some() { 1.0 } else { 0.6 };

        v.widgets.open.corner_radius = r;
        v.widgets.open.bg_fill = surface_2();
        v.widgets.open.weak_bg_fill = surface_2();
        v.widgets.open.bg_stroke = Stroke::new(1.0, accent_stroke());

        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        style.spacing.slider_rail_height = 7.0;

        // Terminal types in a monospaced font, which runs about a quarter wider than the others, so it
        // is smaller and its buttons are packed closer. The window has one fixed width, and rows such
        // as the performance modes do not wrap, so they have to come out no wider than they do in the
        // other looks.
        let terminal = skin() == Skin::Terminal;
        let (family, scale, small_scale) = if terminal { (FontFamily::Monospace, 0.82, 0.86) } else { (FontFamily::Proportional, 1.0, 1.0) };
        if terminal {
            style.spacing.button_padding = egui::vec2(5.0, 4.0);
            style.spacing.item_spacing = egui::vec2(6.0, 6.0);
            style.spacing.interact_size.y = 24.0;
        }
        style.text_styles.insert(TextStyle::Heading, FontId::new(22.0 * scale, family.clone()));
        style.text_styles.insert(TextStyle::Body, FontId::new(BODY_SIZE * scale, family.clone()));
        style.text_styles.insert(TextStyle::Button, FontId::new(14.5 * scale, family.clone()));
        style.text_styles.insert(TextStyle::Small, FontId::new(12.0 * small_scale, family));
    });
}

// ---- drop-downs that remember whether they were open ----

static OPEN_STATES: Mutex<BTreeMap<String, bool>> = Mutex::new(BTreeMap::new());
static OPEN_DIRTY: AtomicBool = AtomicBool::new(false);

fn open_states_lock() -> MutexGuard<'static, BTreeMap<String, bool>> {
    OPEN_STATES.lock().unwrap_or_else(|e| e.into_inner())
}

/// Restores the open / closed state of the drop-downs saved in the config file.
pub fn load_open_states(states: &[(String, bool)]) {
    let mut map = open_states_lock();
    for (key, open) in states {
        map.insert(key.clone(), *open);
    }
}

/// The state of every drop-down that has been opened or closed, for saving.
pub fn open_states() -> Vec<(String, bool)> {
    open_states_lock().iter().map(|(k, v)| (k.clone(), *v)).collect()
}

/// The remembered state under `key`, if there is one.
pub fn get_open(key: &str) -> Option<bool> {
    open_states_lock().get(key).copied()
}

/// Opens or closes a drop-down from code.
pub fn set_open(key: &str, open: bool) {
    let previous = open_states_lock().insert(key.to_string(), open);
    if previous != Some(open) {
        OPEN_DIRTY.store(true, Ordering::Relaxed);
    }
}

/// True once after any drop-down changed (the caller then saves the states).
pub fn take_open_dirty() -> bool {
    OPEN_DIRTY.swap(false, Ordering::Relaxed)
}

/// A drop-down whose open / closed state is remembered between launches. `key` names it in the
/// config file; `default_open` is used until the user has opened or closed it once.
pub fn collapsing<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: impl Into<egui::WidgetText>,
    default_open: bool,
    add_body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    let open = open_states_lock().get(key).copied().unwrap_or(default_open);
    let response =
        egui::CollapsingHeader::new(title).id_salt(key).open(Some(open)).show(ui, add_body);
    if response.header_response.clicked() {
        set_open(key, !open);
    }
    response
}

/// Track color of a switched-off toggle (dark grey).
pub fn toggle_off() -> Color32 {
    if is_void() {
        Color32::from_rgb(44, 30, 86)
    } else if is_gold() {
        Color32::from_rgb(64, 52, 30)
    } else {
        Color32::from_rgb(58, 62, 70)
    }
}

/// A side-flip switch: dark grey when off, the accent color (Options) when on. Clicking the
/// switch or its label flips `on`; the returned response reports `changed()` like a checkbox.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool, label: &str) -> egui::Response {
    let inner = ui.horizontal(|ui| {
        let size = egui::vec2(36.0, 20.0);
        let (rect, switch) = ui.allocate_exact_size(size, egui::Sense::click());
        let text = ui.add(egui::Label::new(label).selectable(false).sense(egui::Sense::click()));
        (rect, switch, text)
    });
    let (rect, switch, text) = inner.inner;
    let mut response = switch.union(text);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, label)
    });

    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let mut track = mix(toggle_off(), accent(), t);
        let mut knob = mix(text_dim(), if is_gold() && !is_void() { on_accent() } else { Color32::WHITE }, t);
        if !ui.is_enabled() {
            track = track.gamma_multiply(0.5);
            knob = knob.gamma_multiply(0.5);
        }
        let radius = rect.height() / 2.0;
        let border = if response.hovered() { accent_stroke() } else { border_light() };
        ui.painter().rect(
            rect,
            CornerRadius::same(radius as u8),
            track,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(egui::pos2(knob_x, rect.center().y), radius - 3.0, knob);
    }
    response
}

/// Puts `add_contents` on a slab of rock when the Underworld look is on, and just shows it
/// otherwise (for the pages that are not made of cards).
pub fn slab_on_underworld<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    if is_underworld() {
        crate::ui::underworld::rock_card(ui, add_contents).inner
    } else {
        add_contents(ui)
    }
}

/// The brushed stainless steel of the Platinum look as one shape filling `rect`: fine streaks, each
/// with its own tone, lighter at the top. It is opaque, so nothing behind it shows through.
pub fn steel_shape(rect: egui::Rect, radius: f32) -> Shape {
    let mut mesh = egui::Mesh::default();
    let (w, h) = (rect.width(), rect.height());
    for row in crate::platinum::steel_rows(h) {
        let (a, b) = (crate::platinum::corner_inset(row.top, h, radius), crate::platinum::corner_inset(row.bottom, h, radius));
        let (top, bottom) = (rect.top() + row.top, rect.top() + row.bottom);
        let base = mesh.vertices.len() as u32;
        for (y, inset) in [(top, a), (bottom, b)] {
            let xs = [rect.left() + inset, rect.left() + w * 0.5, rect.right() - inset];
            for (x, c) in xs.into_iter().zip(row.colours) {
                mesh.colored_vertex(egui::pos2(x, y), Color32::from_rgb(c[0], c[1], c[2]));
            }
        }
        for (p, q, r) in [(0, 1, 4), (0, 4, 3), (1, 2, 5), (1, 5, 4)] {
            mesh.add_triangle(base + p, base + q, base + r);
        }
    }
    Shape::mesh(mesh)
}

/// A card of brushed steel (the Platinum look): the steel is painted behind whatever the card holds,
/// by reserving its place first and filling it in once the card's size is known.
fn steel_card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let radius = skin_radii().0;
    let slot = ui.painter().add(Shape::Noop);
    let shown = egui::Frame::new()
        .stroke(Stroke::new(1.0, border()))
        .corner_radius(CornerRadius::same(radius))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        });
    ui.painter().set(slot, steel_shape(shown.response.rect, radius as f32));
    platinum_trim(ui, shown.response.rect);
    shown
}

/// A rounded "card" container used for every section of the main window.
pub fn card<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    if is_underworld() {
        // a slab of rock on the lava (drawn in `ui/underworld.rs`)
        return crate::ui::underworld::rock_card(ui, add_contents);
    }
    if skin() == Skin::Platinum {
        return steel_card(ui, add_contents);
    }
    let shown = egui::Frame::new()
        .fill(surface())
        .stroke(Stroke::new(1.0, border()))
        .corner_radius(CornerRadius::same(skin_radii().0))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        });
    if skin() != Skin::None {
        skin_trim(ui, shown.response.rect);
    } else if is_void() {
        void_card_trim(ui, shown.response.rect);
    } else if is_gold() {
        gold_card_trim(ui, shown.response.rect);
    }
    shown
}

/// The card trim of the skin that is on.
fn skin_trim(ui: &egui::Ui, rect: egui::Rect) {
    match skin() {
        Skin::Fraying => fraying_trim(ui, rect),
        Skin::Sakura => sakura_trim(ui, rect),
        Skin::Synthwave => synthwave_trim(ui, rect),
        Skin::Prism => animated_edge_trim(ui, rect, crate::skins::prism_edge, 2.0),
        Skin::Aurora => animated_edge_trim(ui, rect, crate::skins::aurora_edge, 1.5),
        Skin::DeepSea => deepsea_trim(ui, rect),
        Skin::Frost => frost_trim(ui, rect),
        Skin::Terminal => terminal_trim(ui, rect),
        Skin::Descent => descent_trim(ui, rect),
        Skin::Platinum => platinum_trim(ui, rect),
        Skin::Midnight => midnight_trim(ui, rect),
        Skin::Copper => copper_trim(ui, rect),
        Skin::None => {}
    }
}

/// Midnight's card trim: a thin periwinkle line along the top with a faint glow, and one small star in the
/// top right corner.
fn midnight_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let tint = accent();
    let (a, b) = (egui::pos2(rect.left() + 10.0, rect.top() + 0.5), egui::pos2(rect.right() - 10.0, rect.top() + 0.5));
    painter.line_segment([a, b], Stroke::new(5.0, tint.gamma_multiply(0.14)));
    painter.line_segment([a, b], Stroke::new(1.2, tint.gamma_multiply(0.85)));
    let star = rect.right_top() + egui::vec2(-14.0, 9.0);
    painter.circle_filled(star, 3.5, tint.gamma_multiply(0.18));
    painter.circle_filled(star, 1.4, Color32::from_rgb(230, 236, 255));
}

/// Copper's card trim: a copper bracket at the top left corner and another at the bottom right.
fn copper_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let bright = Stroke::new(2.0, accent());
    let arm = 16.0;
    for (corner, dx, dy) in [(rect.left_top(), 1.0, 1.0), (rect.right_bottom(), -1.0, -1.0)] {
        let c = corner + egui::vec2(dx, dy);
        painter.line_segment([c, c + egui::vec2(dx * arm, 0.0)], bright);
        painter.line_segment([c, c + egui::vec2(0.0, dy * arm)], bright);
    }
}

/// Synthwave's card trim: a neon magenta line along the top and a cyan one along the bottom, each
/// with a faint wider glow, and short cyan brackets at the top left and bottom right.
fn synthwave_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let line = |y: f32, colour: Color32| {
        painter.line_segment([egui::pos2(rect.left() + 8.0, y), egui::pos2(rect.right() - 8.0, y)], Stroke::new(5.0, colour.gamma_multiply(0.18)));
        painter.line_segment([egui::pos2(rect.left() + 8.0, y), egui::pos2(rect.right() - 8.0, y)], Stroke::new(1.5, colour));
    };
    line(rect.top() + 0.5, SYNTH_MAGENTA);
    line(rect.bottom() - 0.5, SYNTH_CYAN);
    let arm = 14.0;
    let bright = Stroke::new(2.0, SYNTH_CYAN);
    let (tl, br) = (rect.left_top() + egui::vec2(1.0, 1.0), rect.right_bottom() - egui::vec2(1.0, 1.0));
    painter.line_segment([tl, tl + egui::vec2(arm, 0.0)], bright);
    painter.line_segment([tl, tl + egui::vec2(0.0, arm)], bright);
    painter.line_segment([br, br - egui::vec2(arm, 0.0)], bright);
    painter.line_segment([br, br - egui::vec2(0.0, arm)], bright);
}

/// A card edge whose colour changes along it and over time (`colour` takes how far round the card
/// it is, 0 to 1, and the time in seconds). It is cut into short pieces, each one colour.
fn animated_edge_trim(ui: &egui::Ui, rect: egui::Rect, colour: fn(f32, f32) -> [u8; 3], width: f32) {
    let time = ui.input(|i| i.time) as f32;
    let painter = ui.painter();
    let origin = rect.left_top();
    let pieces = ((rect.width() + rect.height()) / 5.0).clamp(24.0, 160.0) as usize;
    for (a, b, t) in crate::skins::outline_pieces(rect.width(), rect.height(), pieces) {
        let [r, g, bl] = colour(t, time);
        painter.line_segment(
            [origin + egui::vec2(a.0, a.1), origin + egui::vec2(b.0, b.1)],
            Stroke::new(width, Color32::from_rgb(r, g, bl)),
        );
    }
}

/// Sakura's card trim: a thin second frame just inside, like a shoji screen's, a short vermilion
/// bar along the top left (the seal) and a blossoming twig in the top right corner.
fn sakura_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    painter.rect_stroke(
        rect.shrink(3.5),
        CornerRadius::same(4),
        Stroke::new(1.0, border().gamma_multiply(0.6)),
        egui::StrokeKind::Middle,
    );
    let tl = rect.left_top() + egui::vec2(14.0, 0.5);
    painter.line_segment([tl, tl + egui::vec2(30.0, 0.0)], Stroke::new(2.5, SAKURA_VERMILION));
    // a short blossoming twig from the top right corner, reaching in
    let twig = crate::sakura::branch(7, 2.9, 2);
    crate::ui::backdrop::paint_blossom_art(painter, &twig, rect.right_top() + egui::vec2(-3.0, 5.0), 26.0, 0.0, 0.0, 1.0);
}

/// Deep Sea's card trim: bubbles, a string of small ones along the bottom edge and a column rising up
/// the right side (see `skins.rs`), each a pale ring with a speck of light on it.
fn deepsea_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let origin = rect.left_top();
    let tint = accent();
    for (x, y, r) in crate::skins::card_bubbles(rect.width(), rect.height()) {
        let c = origin + egui::vec2(x, y);
        painter.circle_filled(c, r, tint.gamma_multiply(0.10));
        painter.circle_stroke(c, r, Stroke::new(1.0, tint.gamma_multiply(0.6)));
        painter.circle_filled(c + egui::vec2(-r * 0.35, -r * 0.35), (r * 0.22).max(0.6), Color32::from_white_alpha(150));
    }
}

/// Frost's card trim: snow lying along the top edge, icicles hanging from the bottom, cracks running
/// into the bottom left corner and a small snowy pine in the top right (see `pine.rs`).
fn frost_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let origin = rect.left_top();
    let snow = Color32::from_rgb(236, 246, 255);
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.left() + 6.0, rect.top() - 1.0), egui::pos2(rect.right() - 6.0, rect.top() + 2.5)),
        1.0,
        snow,
    );
    for (x, r) in crate::pine::snow_lumps(rect.width()) {
        painter.circle_filled(origin + egui::vec2(x, 1.0 - r * 0.4), r, snow);
    }
    let ice = Color32::from_rgb(170, 214, 240).gamma_multiply(0.85);
    for (x, len) in crate::pine::icicles(rect.width(), rect.height()) {
        let top = rect.bottom() - 0.5;
        painter.add(Shape::convex_polygon(
            vec![egui::pos2(origin.x + x - 2.5, top), egui::pos2(origin.x + x + 2.5, top), egui::pos2(origin.x + x + 0.4, rect.bottom() + len)],
            ice,
            Stroke::NONE,
        ));
    }
    let crack = Stroke::new(1.0, Color32::from_rgb(190, 226, 248).gamma_multiply(0.55));
    for line in crate::pine::cracks(rect.height()) {
        painter.add(Shape::line(line.iter().map(|p| origin + egui::vec2(p.0, p.1)).collect(), crack));
    }
    let tree = crate::pine::tree(5, 4);
    crate::ui::backdrop::paint_pine(painter, &tree, egui::pos2(rect.right() - 18.0, rect.top() + 38.0), 34.0, Color32::from_rgb(26, 70, 72), snow, 1.0);
}

/// Terminal's card trim: bright brackets at the four corners and faint scanlines across the card.
fn terminal_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let tint = accent();
    let mut y = rect.top() + 4.0;
    while y < rect.bottom() - 3.0 {
        painter.line_segment([egui::pos2(rect.left() + 1.0, y), egui::pos2(rect.right() - 1.0, y)], Stroke::new(1.0, tint.gamma_multiply(0.045)));
        y += 4.0;
    }
    let arm = 10.0;
    let bright = Stroke::new(1.5, tint);
    for (corner, dx, dy) in [(rect.left_top(), 1.0, 1.0), (rect.right_top(), -1.0, 1.0), (rect.left_bottom(), 1.0, -1.0), (rect.right_bottom(), -1.0, -1.0)] {
        let c = corner + egui::vec2(dx, dy);
        painter.line_segment([c, c + egui::vec2(dx * arm, 0.0)], bright);
        painter.line_segment([c, c + egui::vec2(0.0, dy * arm)], bright);
    }
}

/// A cut gem seen from above, `r` points across, with four facets (`platinum::gem_facets`) that
/// brighten with `flash` (0..1), a thin edge, and a cross-shaped flare while it flashes.
fn paint_gem(painter: &egui::Painter, c: egui::Pos2, r: f32, flash: f32) {
    let dark = Color32::from_rgb(64, 74, 92);
    let light = Color32::from_rgb(246, 250, 255);
    for (tri, shade) in crate::platinum::gem_facets(r) {
        let colour = mix(dark, light, (shade * (0.55 + 0.45 * flash)).clamp(0.0, 1.0));
        painter.add(Shape::convex_polygon(tri.iter().map(|(x, y)| c + egui::vec2(*x, *y)).collect(), colour, Stroke::NONE));
    }
    let outline = [(0.0, -r), (r * 0.78, 0.0), (0.0, r), (-r * 0.78, 0.0)];
    painter.add(Shape::closed_line(outline.iter().map(|(x, y)| c + egui::vec2(*x, *y)).collect(), Stroke::new(0.8, Color32::WHITE.gamma_multiply(0.55))));
    if flash > 0.55 {
        let f = (flash - 0.55) / 0.45;
        let arm = r * (1.6 + 2.2 * f);
        let stroke = Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.75 * f));
        painter.line_segment([c - egui::vec2(arm, 0.0), c + egui::vec2(arm, 0.0)], stroke);
        painter.line_segment([c - egui::vec2(0.0, arm), c + egui::vec2(0.0, arm)], stroke);
    }
}

/// Platinum's card trim (see `platinum.rs`): a bevelled frame (lit along the top and left, in shadow
/// along the bottom and right), a second fine edge inside it, a sheen along the top, a gleam that
/// sweeps over the card when the window's sweep of light passes, a medallion gem on the top edge with
/// a fading filigree either side, a gem at each corner and a small one at the bottom.
fn platinum_trim(ui: &egui::Ui, rect: egui::Rect) {
    use crate::platinum;
    let painter = ui.painter();
    let area = ui.clip_rect();
    // (with the effects off the card is lit the way it is at the start, before any light has come)
    let t = if platinum_effects() { (ui.input(|i| i.time) % 3600.0) as f32 } else { 0.0 };
    let frac = |p: egui::Pos2| ((p.x - area.left()) / area.width().max(1.0), (p.y - area.top()) / area.height().max(1.0));
    let tint = accent();
    let sweep = platinum::sweep_position(t);

    // the bevel
    let inner = rect.shrink(1.5);
    let lit = Stroke::new(1.4, Color32::WHITE.gamma_multiply(0.6));
    let shade = Stroke::new(1.4, Color32::from_black_alpha(160));
    painter.line_segment([inner.left_bottom(), inner.left_top()], lit);
    painter.line_segment([inner.left_top(), inner.right_top()], Stroke::new(1.4, Color32::WHITE.gamma_multiply(0.75)));
    painter.line_segment([inner.right_top(), inner.right_bottom()], shade);
    painter.line_segment([inner.right_bottom(), inner.left_bottom()], shade);
    painter.rect_stroke(rect.shrink(5.0), CornerRadius::same(3), Stroke::new(1.0, tint.gamma_multiply(0.22)), egui::StrokeKind::Inside);

    // the sheen along the top
    let top = egui::Rect::from_min_size(rect.left_top() + egui::vec2(4.0, 2.0), egui::vec2((rect.width() - 8.0).max(0.0), 14.0));
    let mut sheen = egui::Mesh::default();
    let (bright, none) = (Color32::WHITE.gamma_multiply(0.09), Color32::TRANSPARENT);
    sheen.colored_vertex(top.left_top(), bright);
    sheen.colored_vertex(top.right_top(), bright);
    sheen.colored_vertex(top.right_bottom(), none);
    sheen.colored_vertex(top.left_bottom(), none);
    sheen.add_triangle(0, 1, 2);
    sheen.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(sheen));

    // the gleam: the card in slices, each as lit as the sweep makes it where it is
    if sweep.is_some() {
        let slices = 14;
        let step = rect.width() / slices as f32;
        for i in 0..slices {
            let x0 = rect.left() + step * i as f32;
            let mid = frac(egui::pos2(x0 + step / 2.0, rect.center().y));
            let light = platinum::light_at(mid.0, mid.1, t);
            if light > 0.02 {
                let slice = egui::Rect::from_min_max(egui::pos2(x0, rect.top() + 2.0), egui::pos2(x0 + step + 0.5, rect.bottom() - 2.0));
                painter.rect_filled(slice, 0.0, Color32::WHITE.gamma_multiply(0.07 * light));
            }
        }
    }

    // the medallion on the top edge, with a filigree either side that fades away
    let top_mid = egui::pos2(rect.center().x, rect.top());
    for side in [-1.0f32, 1.0] {
        let mut from = top_mid + egui::vec2(side * 10.0, 0.0);
        for i in 0..4 {
            let len = 14.0 - 2.5 * i as f32;
            let to = from + egui::vec2(side * len, 0.0);
            painter.line_segment([from, to], Stroke::new(1.2, tint.gamma_multiply(0.7 - 0.15 * i as f32)));
            painter.circle_filled(to + egui::vec2(side * 2.5, 0.0), 1.2 - 0.15 * i as f32, tint.gamma_multiply(0.7 - 0.15 * i as f32));
            from = to + egui::vec2(side * 5.5, 0.0);
        }
    }
    let flash_of = |n: usize, p: egui::Pos2| {
        let (x, y) = frac(p);
        platinum::gem_flash(n, x, y, t)
    };
    paint_gem(painter, top_mid, 6.5, flash_of(0, top_mid));
    // the corners, and a small one at the bottom
    for (n, (corner, dx, dy)) in [(rect.left_top(), 1.0, 1.0), (rect.right_top(), -1.0, 1.0), (rect.left_bottom(), 1.0, -1.0), (rect.right_bottom(), -1.0, -1.0)].into_iter().enumerate() {
        let c = corner + egui::vec2(dx * 9.0, dy * 9.0);
        paint_gem(painter, c, 4.2, flash_of(n + 1, c));
    }
    let bottom_mid = egui::pos2(rect.center().x, rect.bottom());
    paint_gem(painter, bottom_mid, 3.2, flash_of(5, bottom_mid));
}

/// Descent's card trim: a short red bar at the top left and a dark edge along the bottom, as if the
/// card were a slab hanging over the dark.
fn descent_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let tl = rect.left_top() + egui::vec2(0.0, 0.5);
    painter.line_segment([tl, tl + egui::vec2(26.0, 0.0)], Stroke::new(2.0, accent()));
    painter.line_segment(
        [rect.left_bottom() + egui::vec2(0.0, 1.5), rect.right_bottom() + egui::vec2(0.0, 1.5)],
        Stroke::new(2.0, Color32::from_black_alpha(160)),
    );
}

/// Fraying's card edge: bites taken out of the border (painted in the background colour) and a few
/// loose threads (see `fray.rs`).
fn fraying_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let origin = rect.left_top();
    let at = |p: (f32, f32)| origin + egui::vec2(p.0, p.1);
    for n in crate::fray::notches(rect.width(), rect.height()) {
        painter.add(egui::Shape::convex_polygon(n.iter().map(|p| at(*p)).collect(), bg(), Stroke::new(1.0, border())));
    }
    let thread = Stroke::new(1.0, border_light());
    for t in crate::fray::threads(rect.width(), rect.height()) {
        painter.add(egui::Shape::line(t.iter().map(|p| at(*p)).collect(), thread));
    }
}

/// The Gold Edition's card trim: a thin second border just outside, and bright corner brackets at
/// the top left and the bottom right.
fn gold_card_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    painter.rect_stroke(
        rect.expand(2.5),
        CornerRadius::same(14),
        Stroke::new(1.0, Color32::from_rgb(58, 42, 9)),
        egui::StrokeKind::Middle,
    );
    let bright = Stroke::new(2.0, Color32::from_rgb(240, 194, 79));
    let arm = 16.0;
    let (tl, br) = (rect.left_top() + egui::vec2(1.0, 1.0), rect.right_bottom() - egui::vec2(1.0, 1.0));
    painter.line_segment([tl + egui::vec2(6.0, 0.0), tl + egui::vec2(arm, 0.0)], bright);
    painter.line_segment([tl + egui::vec2(0.0, 6.0), tl + egui::vec2(0.0, arm)], bright);
    painter.line_segment([br - egui::vec2(6.0, 0.0), br - egui::vec2(arm, 0.0)], bright);
    painter.line_segment([br - egui::vec2(0.0, 6.0), br - egui::vec2(0.0, arm)], bright);
}

/// The Void's card trim: a cyan tick along the top left and a magenta one along the bottom right,
/// each doubled by a faint copy a pixel off, like a picture that does not quite line up.
fn void_card_trim(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    let (tl, br) = (rect.left_top() + egui::vec2(10.0, 1.0), rect.right_bottom() - egui::vec2(10.0, 1.0));
    for (off, alpha) in [(1.5, 70u8), (0.0, 255u8)] {
        let cyan = Stroke::new(2.0, Color32::from_rgba_unmultiplied(63, 240, 255, alpha));
        let magenta = Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 63, 200, alpha));
        painter.line_segment([tl + egui::vec2(off, off), tl + egui::vec2(46.0 + off, off)], cyan);
        painter.line_segment([br - egui::vec2(off, off), br - egui::vec2(46.0 + off, off)], magenta);
    }
}

/// Card heading text, tinted with the accent color.
pub fn section_title(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().size(15.5).color(accent_stroke())
}

// ---- fonts ----

/// (label shown in Options, file name inside C:\Windows\Fonts). Empty file = special entry.
pub const FONT_CHOICES: &[(&str, &str)] = &[
    ("Bahnschrift", "bahnschrift.ttf"),
    ("Segoe UI Semibold", "seguisb.ttf"),
    ("Segoe UI", "segoeui.ttf"),
    ("Consolas", "consola.ttf"),
    ("Trebuchet MS Bold", "trebucbd.ttf"),
    ("Custom (font.ttf)", ""),
    ("Default", ""),
];

fn load_font_bytes(choice: &str) -> Option<Vec<u8>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if choice.starts_with("Custom") {
        // User-supplied font: font.ttf / font.otf next to the exe or in the settings folder.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                paths.push(dir.join("font.ttf"));
                paths.push(dir.join("font.otf"));
            }
        }
        let cfg = crate::config::config_dir();
        paths.push(cfg.join("font.ttf"));
        paths.push(cfg.join("font.otf"));
    } else if let Some((_, file)) = FONT_CHOICES.iter().find(|(n, f)| *n == choice && !f.is_empty())
    {
        let win = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        paths.push(win.join("Fonts").join(file));
    }
    for p in paths {
        if let Ok(bytes) = std::fs::read(&p) {
            // Validate first: egui panics on a font it cannot parse.
            if ab_glyph::FontRef::try_from_slice(&bytes).is_ok() {
                return Some(bytes);
            }
        }
    }
    None
}

/// Installs the chosen UI font in front of egui's built-ins (which stay as fallbacks for emoji/symbols).
pub fn install_fonts(ctx: &egui::Context, choice: &str) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(bytes) = load_font_bytes(choice) {
        fonts.font_data.insert(
            "cpunity_ui".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
            list.insert(0, "cpunity_ui".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}
