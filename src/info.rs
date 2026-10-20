//! The text of the Info tab. It is built into the program (`assets/info.txt`) and cannot be
//! changed from inside it: it stays as the author wrote it. The `[+] P.S` drop-down is always the
//! last thing in it: new sections are added above it.
use crate::doc::{parse, Block};
use std::sync::{Mutex, OnceLock};

/// The text that is built into the program.
pub const TEXT: &str = include_str!("../assets/info.txt");

/// The text cut into blocks (once): as the author wrote it, with whatever The Void adds left out.
pub fn blocks() -> &'static [Block] {
    blocks_for(false, 0)
}

/// The sentences that are hidden in the CPUnity part of the text while The Void's script is showing,
/// the first telling the reader what to do and the second when (see `place_void_sentences`).
pub const VOID_CALLS: [&str; 2] = [
    "Break the staff... Release the sands of time! It is, the only way..",
    "The moon strikes staff at dusk...",
];

/// The text cut into blocks (once each way). `script` is whether the Info tab is showing The Void's
/// script: then the sentences The Void adds or changes are in (see `apply_void_markup`). Without
/// it the text is exactly as it was written, with none of them. `seed` picks where the hidden
/// sentences sit (it is kept in the settings, so they stay where they are until The Void is locked).
pub fn blocks_for(script: bool, seed: u64) -> &'static [Block] {
    static PLAIN: OnceLock<Vec<Block>> = OnceLock::new();
    static SCRIPT: Mutex<Vec<(u64, &'static [Block])>> = Mutex::new(Vec::new());
    if !script {
        return PLAIN.get_or_init(|| parse(&apply_void_markup(TEXT, false)));
    }
    let mut cache = SCRIPT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, blocks)) = cache.iter().find(|(s, _)| *s == seed) {
        return blocks;
    }
    let placed = place_void_sentences(TEXT, &VOID_CALLS, seed);
    // a handful of these at most (a new one only when The Void is locked and unlocked again)
    let blocks: &'static [Block] = Box::leak(parse(&apply_void_markup(&placed, true)).into_boxed_slice());
    cache.push((seed, blocks));
    blocks
}

/// Hides each of `sentences` somewhere in the sections of the text that are about CPUnity (the
/// headings with "CPUnity" in them), as a `{{+ }}` sentence: it goes after one of the sentences of a
/// paragraph, or at its end, never into a heading, a bullet or a bold phrase, so it reads like the
/// rest. `seed` picks the places (every place can come up) and no two sentences share a paragraph,
/// so they have to be found one by one. Without the script the markup takes them out again, and the
/// text is exactly what it was.
pub fn place_void_sentences(src: &str, sentences: &[&str], seed: u64) -> String {
    let lines: Vec<&str> = src.lines().collect();
    // (line, byte position in it, whether it goes at the end of the line)
    let mut slots: Vec<(usize, usize, bool)> = Vec::new();
    let mut about_cpunity = false;
    for (i, line) in lines.iter().enumerate() {
        if let Some(heading) = line.strip_prefix("# ") {
            about_cpunity = heading.contains("CPUnity");
            continue;
        }
        let t = line.trim();
        let paragraph = !t.is_empty() && !t.starts_with(['#', '-', '@', '{', '|', '>']);
        if !about_cpunity || !paragraph {
            continue;
        }
        for (p, _) in line.match_indices(". ").chain(line.match_indices("! ")).chain(line.match_indices("? ")) {
            let at = p + 2;
            let starts_sentence = line[at..].chars().next().map_or(false, |c| c.is_uppercase());
            if starts_sentence && line[..at].matches("**").count() % 2 == 0 {
                slots.push((i, at, false));
            }
        }
        if line.matches("**").count() % 2 == 0 {
            slots.push((i, line.len(), true));
        }
    }
    slots.sort_unstable();
    // one place per sentence, each in a paragraph of its own
    let mut chosen: Vec<(usize, usize, bool, &str)> = Vec::new();
    for (k, sentence) in sentences.iter().enumerate() {
        let free: Vec<&(usize, usize, bool)> =
            slots.iter().filter(|(line, _, _)| chosen.iter().all(|(used, ..)| used != line)).collect();
        if free.is_empty() {
            break;
        }
        let (line, at, at_end) = *free[(mix(seed.wrapping_add((k as u64).wrapping_mul(0x632B_E59B_D9B4_E019))) % free.len() as u64) as usize];
        chosen.push((line, at, at_end, sentence));
    }
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        match chosen.iter().find(|(l, ..)| *l == i) {
            Some((_, _, true, sentence)) => out.push_str(&format!("{line} {{{{+ {sentence}}}}}")),
            Some((_, at, false, sentence)) => out.push_str(&format!("{}{{{{+ {sentence}}}}} {}", &line[..*at], &line[*at..])),
            None => out.push_str(line),
        }
        out.push('\n');
    }
    if chosen.is_empty() {
        return src.to_string();
    }
    out
}

/// A well-mixed number out of any seed (splitmix64), so neighbouring seeds land far apart.
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Sentences that belong to The Void are marked in the text and are only there while its script is
/// showing; with the script off they are gone and the text reads as it always did.
///
/// - `{{+ a sentence}}` is added (on a line of its own it is a paragraph of its own).
/// - `{{~ the old sentence => the new one}}` is changed: the old one is read without the script.
pub fn apply_void_markup(src: &str, script: bool) -> String {
    let mut out = String::new();
    for raw in src.lines() {
        let line = raw.trim();
        if let Some(inner) = line.strip_prefix("{{+").and_then(|r| r.strip_suffix("}}")) {
            if script && !inner.contains("{{") {
                out.push_str(inner.trim());
                out.push('\n');
            }
            // a whole added line is dropped altogether, not left as an empty one
            continue;
        }
        out.push_str(&void_markup_in_line(raw, script));
        out.push('\n');
    }
    out
}

fn void_markup_in_line(line: &str, script: bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start + 2..].find("}}") else { break };
        let inner = &rest[start + 2..start + 2 + len];
        let after = &rest[start + 2 + len + 2..];
        let chosen: Option<&str> = if let Some(added) = inner.strip_prefix('+') {
            Some(if script { added.trim() } else { "" })
        } else if let Some(change) = inner.strip_prefix('~') {
            change.split_once("=>").map(|(old, new)| if script { new.trim() } else { old.trim() })
        } else {
            None
        };
        match chosen {
            Some(text) => {
                out.push_str(&rest[..start]);
                if text.is_empty() {
                    // nothing left of it: do not leave two spaces behind
                    if out.ends_with(' ') && (after.is_empty() || after.starts_with(' ')) {
                        out.pop();
                    }
                } else {
                    out.push_str(text);
                }
                rest = after;
            }
            None => {
                out.push_str(&rest[..start + 2]);
                rest = &rest[start + 2..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Size in points to draw the logo at the top of the tab: as wide as the tab allows, but never
/// larger than its own pixels (so it is shown sharp, not blown up). `px` is the image size in
/// pixels, `ppp` the screen's pixels per point.
pub fn fit_logo(available: f32, px: (f32, f32), ppp: f32) -> (f32, f32) {
    if px.0 <= 0.0 || px.1 <= 0.0 || ppp <= 0.0 {
        return (0.0, 0.0);
    }
    let width = available.max(0.0).min(px.0 / ppp);
    (width, width * px.1 / px.0)
}

/// The backdrop pictures (`assets/*.png`): white pixels whose transparency carries the pattern, so
/// the program can colour them with the accent colour.
const CIRCUIT_PNG: &[u8] = include_bytes!("../assets/info-backdrop.png");
const PREDATOR_PNG: &[u8] = include_bytes!("../assets/backdrop-predator.png");
const CATS_PNG: &[u8] = include_bytes!("../assets/backdrop-cats.png");
const DRAGONS_PNG: &[u8] = include_bytes!("../assets/backdrop-dragons.png");
const FATHER_BACK_PNG: &[u8] = include_bytes!("../assets/backdrop-father-back.png");
const FATHER_MASK_PNG: &[u8] = include_bytes!("../assets/backdrop-father-mask.png");
const FATHER_FRONT_PNG: &[u8] = include_bytes!("../assets/backdrop-father-front.png");
/// The one picture that is not white: the angel cats are in full colour (see `angels.rs`).
const ANGELS_PNG: &[u8] = include_bytes!("../assets/backdrop-angels.png");
/// The same cats inside The Void: the gold gone out of them, black eyes with a small white X each.
const ANGELS_VOID_PNG: &[u8] = include_bytes!("../assets/backdrop-angels-void.png");
/// The same sky with the Soul Catsters in it: three cats in black suits, hats and sunglasses, in
/// silver (the picture the angel cats are swapped for while Platinum is the look).
const SOULCATS_PNG: &[u8] = include_bytes!("../assets/backdrop-soulcats.png");

/// Which picture sits behind the tabs (Options > Backdrop style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackdropStyle {
    /// Circuit board traces.
    #[default]
    Circuit,
    /// One big predator cat looking out from the middle.
    Predator,
    /// A whole field of small cats, each turned its own way.
    Cats,
    /// Tiny dragons with open jaws, joined by fire that runs like circuit traces.
    Dragons,
    /// The big dragon with a clock behind his head that shows the computer's time.
    FatherTime,
    /// Circuit City, drawn live behind the whole window (the key stays "cat"). On the Info and Changelog
    /// tabs its market lies at the top with the cat before the crown, and a click on him starts the
    /// talk that leads into Game 1.
    Cat,
    /// Winged cats drifting down from above, in gold. Comes with the Gold Edition.
    AngelCats,
    /// Glitching stars in black and dark purple, drawn live. Comes with The Void.
    VoidStars,
    /// The cat king with his gold gone black and his eyes dead, drawn live over glitching stars.
    /// Only inside The Void.
    DeadKing,
    /// Sparks drifting up and cooling, drawn live. Bought in the shop.
    Embers,
    /// Columns of falling characters in the accent colour, drawn live. Bought in the shop.
    DataRain,
    /// Slow curtains of colour, drawn live. Bought in the shop.
    Northern,
    /// Cherry-blossom petals drifting down, drawn live. Bought in the shop.
    Petals,
    /// A striped sun and a neon grid rolling towards you, drawn live. Bought in the shop.
    SynthGrid,
    /// Bubbles rising through beams of light, drawn live. Bought in the shop.
    Bubbles,
    /// Terminal scanlines with a bright band rolling down and a blinking cursor, drawn live. Bought
    /// in the shop.
    Scanlines,
    /// Soft lights drifting about and blinking, drawn live. Bought in the shop.
    Fireflies,
    /// Snow falling, drawn live. Bought in the shop.
    Snowfall,
    /// Slanting rain with lightning flashing far off, drawn live. Bought in the shop.
    Storm,
    /// Stars twinkling slowly, joined by faint lines where they lie close, drawn live. Comes with the program
    /// (the backdrop of the Midnight pair).
    Constellations,
    /// Soft slanting beams of warm light with dust drifting through them, drawn live. Comes with the program
    /// (the backdrop of the Copper pair).
    Sunbeams,
    /// Souls rising out of the lava that come to the cursor and whisper, drawn live. Only inside The
    /// Underworld.
    SoulWhispers,
}

impl BackdropStyle {
    pub const ALL: [BackdropStyle; 22] = [
        BackdropStyle::Circuit,
        BackdropStyle::Predator,
        BackdropStyle::Cats,
        BackdropStyle::Dragons,
        BackdropStyle::FatherTime,
        BackdropStyle::Cat,
        BackdropStyle::AngelCats,
        BackdropStyle::VoidStars,
        BackdropStyle::DeadKing,
        BackdropStyle::Embers,
        BackdropStyle::DataRain,
        BackdropStyle::Northern,
        BackdropStyle::Petals,
        BackdropStyle::SynthGrid,
        BackdropStyle::Bubbles,
        BackdropStyle::Scanlines,
        BackdropStyle::Fireflies,
        BackdropStyle::Snowfall,
        BackdropStyle::Storm,
        BackdropStyle::Constellations,
        BackdropStyle::Sunbeams,
        BackdropStyle::SoulWhispers,
    ];

    /// Name shown in Options.
    pub fn label(self) -> &'static str {
        match self {
            BackdropStyle::Circuit => "Circuit",
            BackdropStyle::Predator => "Predator cat",
            BackdropStyle::Cats => "Cat pack",
            BackdropStyle::Dragons => "Fire dragons",
            BackdropStyle::FatherTime => "Father Time",
            BackdropStyle::Cat => "Circuit City",
            BackdropStyle::AngelCats => "Angel cats",
            BackdropStyle::VoidStars => "Glitch stars",
            BackdropStyle::DeadKing => "Dead Monarch King",
            BackdropStyle::Embers => "Embers",
            BackdropStyle::DataRain => "Data rain",
            BackdropStyle::Northern => "Northern lights",
            BackdropStyle::Petals => "Sakura petals",
            BackdropStyle::SynthGrid => "Synthwave grid",
            BackdropStyle::Bubbles => "Bubbles",
            BackdropStyle::Scanlines => "Scanlines",
            BackdropStyle::Fireflies => "Fireflies",
            BackdropStyle::Snowfall => "Snowfall",
            BackdropStyle::Storm => "Storm",
            BackdropStyle::Constellations => "Constellations",
            BackdropStyle::Sunbeams => "Sunbeams",
            BackdropStyle::SoulWhispers => "Soul Whispers",
        }
    }

    /// The name shown while Platinum is the look (`platinum`): the angel cats go by "Soul Catsters"
    /// there, and everything else keeps its name.
    pub fn label_in(self, platinum: bool) -> &'static str {
        if platinum && self == BackdropStyle::AngelCats {
            "Soul Catsters"
        } else {
            self.label()
        }
    }

    /// The line about it while Platinum is the look.
    pub fn blurb_in(self, platinum: bool) -> &'static str {
        if platinum && self == BackdropStyle::AngelCats {
            "Cats in black suits, hats and sunglasses, drifting down in silver with a trail of light. Comes with the Gold Edition, and wears this look on Platinum."
        } else {
            self.blurb()
        }
    }

    /// One line about what it is, for the preview.
    pub fn blurb(self) -> &'static str {
        match self {
            BackdropStyle::Circuit => "Faint circuit-board traces in the accent colour. The program's own backdrop.",
            BackdropStyle::Predator => "One big predator cat looking out from the middle.",
            BackdropStyle::Cats => "A whole field of small cats, each turned its own way.",
            BackdropStyle::Dragons => "Tiny dragons with open jaws, joined by fire that runs like circuit traces.",
            BackdropStyle::FatherTime => "The big dragon with a clock behind his head that shows the computer's time.",
            BackdropStyle::Cat => "A night city built out of circuit boards (chips, capacitors and traces for buildings) behind the whole window. On the Info and Changelog tabs its market lies at the top, under a clock tower that keeps your time, and a cat in a wine-red coat stands among the crowd.",
            BackdropStyle::AngelCats => "Winged cats drifting down from above, in gold. Comes with the Gold Edition.",
            BackdropStyle::VoidStars => "Glitching stars in black and dark purple. Comes with The Void.",
            BackdropStyle::DeadKing => "The cat king with his gold gone black and his eyes dead, over glitching stars. Only inside The Void.",
            BackdropStyle::Embers => "Sparks drifting up from below, cooling as they rise.",
            BackdropStyle::DataRain => "Columns of falling characters in the accent colour.",
            BackdropStyle::Northern => "Slow curtains of colour that sway and fade.",
            BackdropStyle::Petals => "Cherry-blossom petals drifting down, turning as they fall.",
            BackdropStyle::SynthGrid => "A striped sun on the horizon and a neon grid rolling towards you.",
            BackdropStyle::Bubbles => "Slow bubbles rising through beams of light from above.",
            BackdropStyle::Scanlines => "Fine screen lines with a bright band rolling down and a blinking cursor.",
            BackdropStyle::Fireflies => "Soft lights wandering about and blinking on and off.",
            BackdropStyle::Snowfall => "Snow drifting down, the nearer flakes bigger and faster.",
            BackdropStyle::Storm => "Rain slanting down, with lightning flashing far off.",
            BackdropStyle::Constellations => "Stars that twinkle slowly and drift, joined by faint lines where they lie close. Comes with the program, made for Midnight.",
            BackdropStyle::Sunbeams => "Soft slanting beams of warm light that swell and fade, with dust drifting through them. Comes with the program, made for Copper.",
            BackdropStyle::SoulWhispers => "Souls rising out of the lava. Slow the cursor and they come closer; hold it still and one leans in to whisper. Only inside The Underworld.",
        }
    }

    /// Name written to the config file.
    pub fn key(self) -> &'static str {
        match self {
            BackdropStyle::Circuit => "circuit",
            BackdropStyle::Predator => "predator",
            BackdropStyle::Cats => "cats",
            BackdropStyle::Dragons => "dragons",
            BackdropStyle::FatherTime => "fathertime",
            BackdropStyle::Cat => "cat",
            BackdropStyle::AngelCats => "angelcats",
            BackdropStyle::VoidStars => "voidstars",
            BackdropStyle::DeadKing => "deadking",
            BackdropStyle::Embers => "embers",
            BackdropStyle::DataRain => "datarain",
            BackdropStyle::Northern => "northern",
            BackdropStyle::Petals => "petals",
            BackdropStyle::SynthGrid => "synthgrid",
            BackdropStyle::Bubbles => "bubbles",
            BackdropStyle::Scanlines => "scanlines",
            BackdropStyle::Fireflies => "fireflies",
            BackdropStyle::Snowfall => "snowfall",
            BackdropStyle::Storm => "storm",
            BackdropStyle::Constellations => "constellations",
            BackdropStyle::Sunbeams => "sunbeams",
            BackdropStyle::SoulWhispers => "soulwhispers",
        }
    }

    /// The style a config value names; anything unknown is the default.
    pub fn from_key(key: &str) -> BackdropStyle {
        BackdropStyle::ALL.into_iter().find(|s| s.key() == key.trim()).unwrap_or_default()
    }

    /// The pictures, as PNG bytes. Most styles are one picture; Father Time is three, drawn in
    /// this order with the clock hands between the first and the second: the clock face, the
    /// dragon's silhouette (painted in the background colour so the hands pass behind him) and
    /// the dragon.
    pub fn pngs(self) -> &'static [&'static [u8]] {
        match self {
            BackdropStyle::Circuit => &[CIRCUIT_PNG],
            BackdropStyle::Predator => &[PREDATOR_PNG],
            BackdropStyle::Cats => &[CATS_PNG],
            BackdropStyle::Dragons => &[DRAGONS_PNG],
            BackdropStyle::FatherTime => &[FATHER_BACK_PNG, FATHER_MASK_PNG, FATHER_FRONT_PNG],
            BackdropStyle::AngelCats => &[ANGELS_PNG],
            // drawn live, no picture
            BackdropStyle::Cat | BackdropStyle::VoidStars | BackdropStyle::DeadKing | BackdropStyle::Embers | BackdropStyle::DataRain | BackdropStyle::Northern | BackdropStyle::Petals | BackdropStyle::SynthGrid | BackdropStyle::Bubbles | BackdropStyle::Scanlines | BackdropStyle::Fireflies | BackdropStyle::Snowfall | BackdropStyle::Storm | BackdropStyle::Constellations | BackdropStyle::Sunbeams | BackdropStyle::SoulWhispers => &[],
        }
    }

    /// The pictures to show while The Void is the look the program has (`void`) or not: the same
    /// as `pngs`, except that the angel cats have lost their gold.
    pub fn pngs_in(self, void: bool) -> &'static [&'static [u8]] {
        if void && self == BackdropStyle::AngelCats {
            &[ANGELS_VOID_PNG]
        } else {
            self.pngs()
        }
    }

    /// The pictures to show for a look: The Void's cats (`void`), the Soul Catsters (`platinum`, and
    /// not in The Void), or the plain ones.
    pub fn pngs_with(self, void: bool, platinum: bool) -> &'static [&'static [u8]] {
        if platinum && !void && self == BackdropStyle::AngelCats {
            &[SOULCATS_PNG]
        } else {
            self.pngs_in(void)
        }
    }

    /// Whether the picture moves (so Options offers the Animate backdrop switch).
    pub fn is_animated(self) -> bool {
        matches!(
            self,
            BackdropStyle::Dragons
                | BackdropStyle::FatherTime
                | BackdropStyle::Cat
                | BackdropStyle::AngelCats
                | BackdropStyle::VoidStars
                | BackdropStyle::DeadKing
                | BackdropStyle::Embers
                | BackdropStyle::DataRain
                | BackdropStyle::Northern
                | BackdropStyle::Petals
                | BackdropStyle::SynthGrid
                | BackdropStyle::Bubbles
                | BackdropStyle::Scanlines
                | BackdropStyle::Fireflies
                | BackdropStyle::Snowfall
                | BackdropStyle::Storm
                | BackdropStyle::Constellations
                | BackdropStyle::Sunbeams
                | BackdropStyle::SoulWhispers
        )
    }

    /// Whether the style is one of the six that a release without the story offers (the rest wait for it: the
    /// story's own pictures, and the ones that were sold in the shop).
    pub fn in_release(self) -> bool {
        matches!(
            self,
            BackdropStyle::Circuit
                | BackdropStyle::Predator
                | BackdropStyle::Cats
                | BackdropStyle::Dragons
                | BackdropStyle::Constellations
                | BackdropStyle::Sunbeams
        )
    }

    /// Whether this build lets the style be used and shows it (see `in_release`): a build without the story
    /// has only those six, whatever the config file says.
    pub fn released(self) -> bool {
        self.in_release() || crate::release::story()
    }

    /// Whether the style comes with the Gold Edition: it is only offered in Options once Gold is
    /// unlocked, and is not used while it is locked (but works in any colour theme once it is).
    pub fn needs_gold(self) -> bool {
        self == BackdropStyle::AngelCats
    }

    /// Whether the style comes with The Void (offered once the Void is unlocked, like the above).
    pub fn needs_void(self) -> bool {
        matches!(self, BackdropStyle::VoidStars | BackdropStyle::DeadKing)
    }

    /// Whether the style can only be used while The Void is the look the program has (switched on),
    /// not merely unlocked: it is not offered, and not shown, in any other look.
    pub fn inside_void_only(self) -> bool {
        self == BackdropStyle::DeadKing
    }

    /// Whether the style can only be used while The Underworld is the look the program has: it is not
    /// offered, and not shown, in any other look.
    pub fn inside_underworld_only(self) -> bool {
        self == BackdropStyle::SoulWhispers
    }

    /// Whether the style stands a figure at the top of the page, in the logo's place (Father Time
    /// and the dead king), rather than only lying behind everything.
    pub fn has_stage(self) -> bool {
        self.has_clock() || self == BackdropStyle::DeadKing || self == BackdropStyle::Cat
    }

    /// Whether the style has no picture and is drawn live instead (the Void's and the shop's).
    pub fn is_painted(self) -> bool {
        self.needs_void() || self.shop_id().is_some() || self.inside_underworld_only() || self == BackdropStyle::Cat || matches!(self, BackdropStyle::Constellations | BackdropStyle::Sunbeams)
    }

    /// Whether the style is one of the live-drawn ones of `ambient.rs` (the shop's, and the two that come
    /// with the program), drawn without a picture and without any story.
    pub fn is_ambient(self) -> bool {
        self.shop_id().is_some() || matches!(self, BackdropStyle::Constellations | BackdropStyle::Sunbeams)
    }

    /// The id of the shop item that has to be bought before the style can be used, if it is one
    /// the shop sells (see `shop.rs`).
    pub fn shop_id(self) -> Option<&'static str> {
        match self {
            BackdropStyle::Embers => Some("embers"),
            BackdropStyle::DataRain => Some("datarain"),
            BackdropStyle::Northern => Some("northern"),
            BackdropStyle::Petals => Some("petals"),
            BackdropStyle::SynthGrid => Some("synthgrid"),
            BackdropStyle::Bubbles => Some("bubbles"),
            BackdropStyle::Scanlines => Some("scanlines"),
            BackdropStyle::Fireflies => Some("fireflies"),
            BackdropStyle::Snowfall => Some("snowfall"),
            BackdropStyle::Storm => Some("storm"),
            _ => None,
        }
    }

    /// The styles Options offers that come with the program or an edition: all of them, minus the
    /// ones that wait for an edition and the ones the shop sells.
    pub fn available(gold_unlocked: bool, void_unlocked: bool) -> Vec<BackdropStyle> {
        BackdropStyle::available_with(gold_unlocked, void_unlocked, |_| false)
    }

    /// The same, plus the shop's styles that `owns` says are bought.
    pub fn available_with(gold_unlocked: bool, void_unlocked: bool, owns: impl Fn(&str) -> bool) -> Vec<BackdropStyle> {
        BackdropStyle::ALL
            .into_iter()
            .filter(|s| s.released())
            // (a build without the story has no editions to earn: the angel cats are just there)
            .filter(|s| (gold_unlocked || !crate::release::story() || !s.needs_gold()) && (void_unlocked || !s.needs_void()))
            .filter(|s| s.shop_id().map_or(true, &owns))
            .collect()
    }


    /// Whether the picture has a clock that the program keeps running.
    pub fn has_clock(self) -> bool {
        self == BackdropStyle::FatherTime
    }

    /// How strong it is on the Info tab, where the text sits right on it.
    pub fn info_strength(self) -> f32 {
        match self {
            BackdropStyle::Circuit => 1.0,
            BackdropStyle::Predator => 0.8,
            BackdropStyle::Cats => 0.7,
            BackdropStyle::Dragons => 0.9,
            BackdropStyle::FatherTime => 0.8,
            BackdropStyle::Cat => 0.8,
            BackdropStyle::AngelCats => 0.6,
            BackdropStyle::VoidStars => 0.8,
            BackdropStyle::DeadKing => 0.8,
            BackdropStyle::Embers => 0.8,
            BackdropStyle::DataRain => 0.6,
            BackdropStyle::Northern => 0.7,
            BackdropStyle::Petals => 0.8,
            BackdropStyle::SynthGrid => 0.8,
            BackdropStyle::Bubbles => 0.7,
            BackdropStyle::Scanlines => 0.6,
            BackdropStyle::Fireflies => 0.8,
            BackdropStyle::Snowfall => 0.8,
            BackdropStyle::Storm => 0.6,
            BackdropStyle::Constellations => 0.8,
            BackdropStyle::Sunbeams => 0.7,
            BackdropStyle::SoulWhispers => 0.8,
        }
    }

    /// How strong it is on the tabs with cards: they cover most of it, so it only has to be felt in
    /// the gaps and along the edges.
    pub fn card_strength(self) -> f32 {
        match self {
            BackdropStyle::Circuit => 0.6,
            BackdropStyle::Predator => 0.5,
            BackdropStyle::Cats => 0.45,
            BackdropStyle::Dragons => 0.6,
            BackdropStyle::FatherTime => 0.5,
            BackdropStyle::Cat => 0.5,
            BackdropStyle::AngelCats => 0.42,
            BackdropStyle::VoidStars => 0.7,
            BackdropStyle::DeadKing => 0.7,
            BackdropStyle::Embers => 0.7,
            BackdropStyle::DataRain => 0.5,
            BackdropStyle::Northern => 0.6,
            BackdropStyle::Petals => 0.6,
            BackdropStyle::SynthGrid => 0.6,
            BackdropStyle::Bubbles => 0.6,
            BackdropStyle::Scanlines => 0.5,
            BackdropStyle::Fireflies => 0.7,
            BackdropStyle::Snowfall => 0.7,
            BackdropStyle::Storm => 0.5,
            BackdropStyle::Constellations => 0.7,
            BackdropStyle::Sunbeams => 0.6,
            BackdropStyle::SoulWhispers => 0.6,
        }
    }
}

// ---- the clock in the Father Time picture -------------------------------------------------------
// All numbers are pixels of the 1200 x 1600 picture (tools/father_time.py draws the clock face with
// the same centre). A hand is drawn pointing at 12 and then turned.

/// The middle of the clock face, in picture pixels (it sits behind the dragon's head).
pub const CLOCK_CENTER: (f32, f32) = (600.0, 535.0);
/// Hour hand: outline, then the convex pieces it is filled with (shaft, spear head).
pub const HOUR_OUTLINE: [(f32, f32); 7] =
    [(-7.0, 34.0), (-7.0, -246.0), (-22.0, -288.0), (0.0, -362.0), (22.0, -288.0), (7.0, -246.0), (7.0, 34.0)];
pub const HOUR_SHAFT: [(f32, f32); 4] = [(-7.0, 34.0), (-7.0, -246.0), (7.0, -246.0), (7.0, 34.0)];
pub const HOUR_HEAD: [(f32, f32); 5] =
    [(-7.0, -246.0), (-22.0, -288.0), (0.0, -362.0), (22.0, -288.0), (7.0, -246.0)];
/// Minute hand, the same way.
pub const MINUTE_OUTLINE: [(f32, f32); 7] =
    [(-4.0, 40.0), (-4.0, -318.0), (-11.0, -342.0), (0.0, -392.0), (11.0, -342.0), (4.0, -318.0), (4.0, 40.0)];
pub const MINUTE_SHAFT: [(f32, f32); 4] = [(-4.0, 40.0), (-4.0, -318.0), (4.0, -318.0), (4.0, 40.0)];
pub const MINUTE_HEAD: [(f32, f32); 5] =
    [(-4.0, -318.0), (-11.0, -342.0), (0.0, -392.0), (11.0, -342.0), (4.0, -318.0)];
/// Second hand: a line from its short tail to its tip (pointing at 12).
pub const SECOND_HAND: [(f32, f32); 2] = [(0.0, 64.0), (0.0, -396.0)];
/// Radius of the ring on the hub.
pub const HUB_RADIUS: f32 = 16.0;

/// The angles of the hands in degrees clockwise from 12: (hour, minute, second).
pub fn hand_angles(hour: u32, minute: u32, second: u32) -> (f32, f32, f32) {
    let (h, m, s) = ((hour % 12) as f32, minute.min(59) as f32, second.min(59) as f32);
    ((h + m / 60.0 + s / 3600.0) * 30.0, (m + s / 60.0) * 6.0, s * 6.0)
}

/// Points given pointing at 12 around the clock centre, turned `degrees` clockwise: picture pixels.
pub fn turn_about_clock(points: &[(f32, f32)], degrees: f32) -> Vec<(f32, f32)> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    points
        .iter()
        .map(|&(x, y)| (CLOCK_CENTER.0 + x * cos - y * sin, CLOCK_CENTER.1 + x * sin + y * cos))
        .collect()
}

/// Brightest channel of the backdrop colour: the accent colour is scaled to this, so a dark accent
/// still shows the pattern and a bright one does not drown the text.
const BACKDROP_PEAK: f32 = 230.0;

/// The accent colour (r, g, b) adjusted for the backdrop: same hue, brightest channel 230.
pub fn backdrop_tint(accent: [u8; 3]) -> [u8; 3] {
    let brightest = accent.iter().copied().max().unwrap_or(0);
    if brightest == 0 {
        // a black accent has no hue: use a neutral grey
        return [BACKDROP_PEAK as u8; 3];
    }
    let k = BACKDROP_PEAK / f32::from(brightest);
    accent.map(|c| (f32::from(c) * k).round().clamp(0.0, 255.0) as u8)
}

/// Hue (degrees), saturation and value (0..1) of a colour.
fn hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let d = max - r.min(g).min(b);
    if d <= 0.0 {
        return (0.0, 0.0, max);
    }
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, d / max, max)
}

fn from_hsv(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

/// The logo (RGBA bytes, drawn in red) with its colours turned to the accent colour, the same
/// colour the backdrop has: every hue is rotated by the accent's hue and the saturation follows
/// the accent's. Grey metal stays grey, the darkness and the see-through parts are untouched.
pub fn recolor_logo(rgba: &[u8], accent: [u8; 3]) -> Vec<u8> {
    let (accent_hue, accent_sat, _) =
        hsv(f32::from(accent[0]), f32::from(accent[1]), f32::from(accent[2]));
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        let (r, g, b) =
            (f32::from(px[0]) / 255.0, f32::from(px[1]) / 255.0, f32::from(px[2]) / 255.0);
        let (h, s, v) = hsv(r, g, b);
        if s <= 0.0 {
            continue;
        }
        let (r, g, b) = from_hsv(h + accent_hue, s * accent_sat, v);
        px[0] = (r * 255.0).round().clamp(0.0, 255.0) as u8;
        px[1] = (g * 255.0).round().clamp(0.0, 255.0) as u8;
        px[2] = (b * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Which part of an image (width, height) to show so that it covers a view (width, height) fully
/// without stretching: (u_min, v_min, u_max, v_max), the overflowing side cropped equally.
pub fn cover_uv(image: (f32, f32), view: (f32, f32)) -> [f32; 4] {
    if image.0 <= 0.0 || image.1 <= 0.0 || view.0 <= 0.0 || view.1 <= 0.0 {
        return [0.0, 0.0, 1.0, 1.0];
    }
    let image_aspect = image.0 / image.1;
    let view_aspect = view.0 / view.1;
    if image_aspect > view_aspect {
        let off = (1.0 - view_aspect / image_aspect) / 2.0;
        [off, 0.0, 1.0 - off, 1.0]
    } else {
        let off = (1.0 - image_aspect / view_aspect) / 2.0;
        [0.0, off, 1.0, 1.0 - off]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_the_void_adds_or_changes_is_only_there_with_the_script_on() {
        let src = "# Title\n\nOne sentence. {{+ An extra one.}} Another.\n{{+ A paragraph of its own.}}\n\nThe door is {{~ closed => open}} now.\n- item {{+ plus}}\n";
        let off = apply_void_markup(src, false);
        let on = apply_void_markup(src, true);
        assert_eq!(off, "# Title\n\nOne sentence. Another.\n\nThe door is closed now.\n- item\n");
        assert_eq!(on, "# Title\n\nOne sentence. An extra one. Another.\nA paragraph of its own.\n\nThe door is open now.\n- item plus\n");
        // without the script nothing of it is left to be seen
        assert!(!off.contains("extra") && !off.contains("own") && !off.contains("open") && !off.contains("{{"));
        // broken markup is left alone
        assert_eq!(apply_void_markup("a {{+ b\nc {{~ x }} d\n", true), "a {{+ b\nc {{~ x }} d\n");
        // text without any markup is untouched either way
        assert_eq!(apply_void_markup(TEXT, false), TEXT.lines().map(|l| format!("{l}\n")).collect::<String>());
        assert_eq!(apply_void_markup(TEXT, true), apply_void_markup(TEXT, false));
    }

    #[test]
    fn the_info_text_is_cut_both_ways() {
        assert_eq!(blocks_for(false, 7), blocks());
        // the script text is the same text with one sentence tucked into a paragraph
        assert_eq!(blocks_for(true, 7).len(), blocks().len());
        assert_ne!(blocks_for(true, 7), blocks());
        assert_eq!(blocks_for(true, 7), blocks_for(true, 7));
    }

    /// Where the hidden sentences are, in the script text: for each, the line number and the line.
    fn calls_at(seed: u64) -> Vec<(usize, String)> {
        let text = apply_void_markup(&place_void_sentences(TEXT, &VOID_CALLS, seed), true);
        VOID_CALLS
            .iter()
            .map(|call| {
                let hits: Vec<(usize, &str)> = text.lines().enumerate().filter(|(_, l)| l.contains(call)).collect();
                assert_eq!(hits.len(), 1, "{call:?} is in exactly once (seed {seed})");
                (hits[0].0, hits[0].1.to_string())
            })
            .collect()
    }

    #[test]
    fn the_hidden_sentences_go_in_the_cpunity_part_only_and_come_out_without_the_script() {
        let plain = apply_void_markup(TEXT, false);
        let mut lines_seen = std::collections::BTreeSet::new();
        let mut places = std::collections::BTreeSet::new();
        for seed in 0..400 {
            let placed = place_void_sentences(TEXT, &VOID_CALLS, seed);
            // with the script off the text is what the author wrote, byte for byte
            assert_eq!(apply_void_markup(&placed, false), plain, "seed {seed}");
            let at = calls_at(seed);
            assert_ne!(at[0].0, at[1].0, "seed {seed}: not in the same paragraph");
            for (call, (line, text)) in VOID_CALLS.iter().zip(&at) {
                // it reads as a sentence of the paragraph: never first thing in it, never in bold
                assert!(!text.starts_with(call), "seed {seed}: {text}");
                let before = text.split(call).next().unwrap();
                assert!(before.ends_with(". ") || before.ends_with("! ") || before.ends_with("? ") || before.ends_with("** "), "seed {seed}: {text}");
                assert_eq!(before.matches("**").count() % 2, 0, "seed {seed}: not inside bold");
                lines_seen.insert(*line);
                places.insert((*line, before.len()));
            }
        }
        // only the two CPUnity sections (About CPUnity, CPUnity: Future Endeavors) are ever used
        let on = apply_void_markup(TEXT, true);
        let all: Vec<&str> = on.lines().collect();
        let about = all.iter().position(|l| *l == "# About CPUnity").unwrap();
        let contact = all.iter().position(|l| *l == "# Ways to contact").unwrap();
        assert!(lines_seen.iter().all(|l| about < *l && *l < contact), "{lines_seen:?} not within {about}..{contact}");
        // every paragraph of them gets one some of the time, and there are many places to find them
        assert_eq!(lines_seen.len(), 6, "{lines_seen:?}");
        assert!(places.len() >= 10, "{} places", places.len());
    }

    #[test]
    fn the_hidden_sentences_stay_where_their_seed_puts_them() {
        assert_eq!(calls_at(12345), calls_at(12345));
        let spots: std::collections::BTreeSet<_> = (1..40).map(calls_at).collect();
        assert!(spots.len() > 5, "different seeds, different places");
        // text with no CPUnity part has nowhere to put them, and is left alone
        assert_eq!(place_void_sentences("# One\n\nA. B.\n", &VOID_CALLS, 3), "# One\n\nA. B.\n");
        // a text with room for one paragraph only gets the first sentence
        let one = apply_void_markup(&place_void_sentences("# About CPUnity\n\nA. Bee.\n", &["Hi.", "Ho."], 0), true);
        assert!(one.contains("Hi.") && !one.contains("Ho."), "{one}");
    }

    #[test]
    fn only_the_dragons_father_time_the_angels_the_void_stars_and_the_dead_king_move() {
        for style in BackdropStyle::ALL {
            assert_eq!(
                style.is_animated(),
                matches!(
                    style,
                    BackdropStyle::Dragons
                        | BackdropStyle::FatherTime
                        | BackdropStyle::Cat
                        | BackdropStyle::AngelCats
                        | BackdropStyle::VoidStars
                        | BackdropStyle::DeadKing
                        | BackdropStyle::Embers
                        | BackdropStyle::DataRain
                        | BackdropStyle::Northern
                        | BackdropStyle::Petals
                        | BackdropStyle::SynthGrid
                        | BackdropStyle::Bubbles
                        | BackdropStyle::Scanlines
                        | BackdropStyle::Fireflies
                        | BackdropStyle::Snowfall
                        | BackdropStyle::Storm
                        | BackdropStyle::Constellations
                        | BackdropStyle::Sunbeams
                        | BackdropStyle::SoulWhispers
                )
            );
        }
    }

    #[test]
    fn a_release_without_the_story_has_six_backdrops() {
        let six: Vec<&str> = BackdropStyle::ALL.into_iter().filter(|s| s.in_release()).map(|s| s.key()).collect();
        assert_eq!(six, ["circuit", "predator", "cats", "dragons", "constellations", "sunbeams"]);
        // (the tests run with the story on, so all of them are released here)
        assert!(BackdropStyle::ALL.into_iter().all(|s| s.released()));
        // the two that come with the program are live-drawn, free and not story
        for s in [BackdropStyle::Constellations, BackdropStyle::Sunbeams] {
            assert!(s.is_painted() && s.is_ambient() && s.is_animated() && s.pngs().is_empty() && s.shop_id().is_none() && s.in_release() && !s.has_stage());
            assert!(BackdropStyle::available(false, false).contains(&s));
            assert_eq!(BackdropStyle::from_key(s.key()), s);
        }
        assert_eq!(BackdropStyle::ALL.len(), 22);
    }

    #[test]
    fn the_shop_styles_are_offered_once_bought_and_never_before() {
        let shop: Vec<BackdropStyle> = BackdropStyle::ALL.into_iter().filter(|s| s.shop_id().is_some()).collect();
        assert_eq!(shop.len(), 10);
        for s in &shop {
            assert!(!BackdropStyle::available(true, true).contains(s));
            assert!(BackdropStyle::available_with(false, false, |id| Some(id) == s.shop_id()).contains(s));
            assert!(crate::shop::item(s.shop_id().unwrap()).is_some_and(|i| i.kind == crate::shop::Kind::Backdrop && i.name == s.label()));
        }
        assert_eq!(BackdropStyle::available_with(true, true, |_| true).as_slice(), &BackdropStyle::ALL);
    }

    #[test]
    fn the_angel_cats_wait_for_the_gold_edition() {
        assert!(BackdropStyle::AngelCats.needs_gold());
        assert_eq!(BackdropStyle::ALL.iter().filter(|s| s.needs_gold()).count(), 1);
        assert!(!BackdropStyle::available(false, false).contains(&BackdropStyle::AngelCats));
        assert!(BackdropStyle::available(true, false).contains(&BackdropStyle::AngelCats));
        assert_eq!(BackdropStyle::available_with(true, true, |_| true).as_slice(), &BackdropStyle::ALL);
        assert_eq!(BackdropStyle::available(false, false).len(), BackdropStyle::ALL.len() - 3 - 10);
        // the old styles are all still offered, in their old order, when locked
        assert_eq!(BackdropStyle::available(false, false)[0], BackdropStyle::Circuit);
        assert_eq!(BackdropStyle::from_key("angelcats"), BackdropStyle::AngelCats);
        // it has no clock and is one picture
        assert!(!BackdropStyle::AngelCats.has_clock());
        assert_eq!(BackdropStyle::AngelCats.pngs().len(), 1);
        // inside The Void the angel cats are the dark bronze ones, the same width so that they sit the
        // same; everything else is the same in and out of it
        let size = |png: &[u8]| (u32::from_be_bytes(png[16..20].try_into().unwrap()), u32::from_be_bytes(png[20..24].try_into().unwrap()));
        let (gold, dead) = (BackdropStyle::AngelCats.pngs_in(false), BackdropStyle::AngelCats.pngs_in(true));
        assert_eq!((gold.len(), dead.len()), (1, 1));
        assert_ne!(gold[0], dead[0]);
        // (twice as tall: the cats, and under them only their crosses)
        assert_eq!(size(gold[0]).0, size(dead[0]).0);
        assert_eq!(size(gold[0]).1 * 2, size(dead[0]).1);
        assert_eq!(BackdropStyle::AngelCats.pngs_in(false), BackdropStyle::AngelCats.pngs());
        for style in [BackdropStyle::Circuit, BackdropStyle::Dragons, BackdropStyle::FatherTime, BackdropStyle::VoidStars] {
            assert_eq!(style.pngs_in(true), style.pngs());
        }
    }

    #[test]
    fn the_glitch_stars_wait_for_the_void_and_are_drawn_live() {
        let s = BackdropStyle::VoidStars;
        assert!(s.needs_void() && !s.needs_gold() && !s.has_clock());
        assert_eq!(BackdropStyle::ALL.iter().filter(|s| s.needs_void()).count(), 2);
        assert!(!BackdropStyle::available(true, false).contains(&s));
        assert!(BackdropStyle::available(false, true).contains(&s));
        assert!(s.pngs().is_empty(), "no picture: it is painted");
        assert_eq!(BackdropStyle::from_key("voidstars"), s);
    }

    #[test]
    fn every_backdrop_style_has_a_name_a_key_and_a_picture() {
        let mut keys = std::collections::HashSet::new();
        for style in BackdropStyle::ALL {
            assert!(!style.label().is_empty());
            assert!(keys.insert(style.key()), "keys are unique");
            assert_eq!(BackdropStyle::from_key(style.key()), style);
            assert!(style.is_painted() || !style.pngs().is_empty());
            for png in style.pngs() {
                assert_eq!(&png[1..4], b"PNG");
            }
            assert!(style.info_strength() > 0.0 && style.info_strength() <= 1.0);
            assert!(style.card_strength() > 0.0 && style.card_strength() < style.info_strength());
        }
        assert_eq!(BackdropStyle::default(), BackdropStyle::Circuit);
        // only Father Time has a clock, and it is made of three layers
        assert_eq!(BackdropStyle::ALL.iter().filter(|s| s.has_clock()).count(), 1);
        assert_eq!(BackdropStyle::FatherTime.pngs().len(), 3);
        assert!(BackdropStyle::ALL.iter().filter(|s| !s.has_clock() && !s.is_painted()).all(|s| s.pngs().len() == 1));
        assert_eq!(BackdropStyle::from_key("nonsense"), BackdropStyle::Circuit);
        assert_eq!(BackdropStyle::from_key(" cats \r"), BackdropStyle::Cats);
    }

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
    }

    #[test]
    fn the_hands_follow_the_time() {
        assert_eq!(hand_angles(0, 0, 0), (0.0, 0.0, 0.0));
        assert_eq!(hand_angles(3, 0, 0), (90.0, 0.0, 0.0));
        assert_eq!(hand_angles(12, 30, 0), (15.0, 180.0, 0.0));
        assert_eq!(hand_angles(18, 0, 30), (180.0 + 0.25, 3.0, 180.0));
        // 24 hour times wrap onto the 12 hour dial, and odd values cannot push a hand off it
        assert_eq!(hand_angles(15, 0, 0).0, 90.0);
        assert!(hand_angles(99, 99, 99).1 < 360.0);
    }

    #[test]
    fn a_hand_turns_clockwise_about_the_clock_centre() {
        let tip = [(0.0, -100.0)];
        assert!(close(turn_about_clock(&tip, 0.0)[0], (600.0, 435.0))); // 12 o'clock: up
        assert!(close(turn_about_clock(&tip, 90.0)[0], (700.0, 535.0))); // 3 o'clock: right
        assert!(close(turn_about_clock(&tip, 180.0)[0], (600.0, 635.0))); // 6 o'clock: down
        assert!(close(turn_about_clock(&tip, 270.0)[0], (500.0, 535.0))); // 9 o'clock: left
        // the hand shapes point at 12 and the tail is behind the centre
        for outline in [&HOUR_OUTLINE[..], &MINUTE_OUTLINE[..]] {
            assert!(outline.iter().any(|p| p.1 < -300.0) && outline.iter().any(|p| p.1 > 0.0));
        }
        assert!(MINUTE_OUTLINE.iter().map(|p| p.1).fold(0.0, f32::min) < HOUR_OUTLINE.iter().map(|p| p.1).fold(0.0, f32::min));
    }

    #[test]
    fn the_accent_colour_is_scaled_for_the_backdrop() {
        // keeps the hue, brightest channel 230
        assert_eq!(backdrop_tint([0, 120, 60]), [0, 230, 115]);
        assert_eq!(backdrop_tint([230, 0, 0]), [230, 0, 0]);
        assert_eq!(backdrop_tint([255, 255, 255]), [230, 230, 230]);
        // a black accent falls back to grey instead of dividing by zero
        assert_eq!(backdrop_tint([0, 0, 0]), [230, 230, 230]);
    }

    #[test]
    fn the_logo_takes_the_accent_hue() {
        // red neon, dark red, mid grey, and a see-through pixel
        let logo = [230, 0, 0, 255, 100, 10, 10, 255, 128, 128, 128, 255, 200, 20, 20, 0];
        // a red accent changes nothing
        let same = recolor_logo(&logo, [230, 0, 0]);
        for (a, b) in same.iter().zip(&logo) {
            assert!((i32::from(*a) - i32::from(*b)).abs() <= 1, "{same:?}");
        }
        // the default green accent: red becomes green (hue 150), grey and alpha stay
        let green = recolor_logo(&logo, [0, 120, 60]);
        assert_eq!(&green[0..4], &[0, 230, 115, 255]);
        assert!(green[5] > green[4] && green[5] > green[6], "dark red turns dark green");
        assert_eq!(&green[8..12], &[128, 128, 128, 255]);
        assert_eq!(green[15], 0, "transparency is kept");
        // a grey accent has no colour to give: the neon goes grey
        let grey = recolor_logo(&logo, [120, 120, 120]);
        assert_eq!(&grey[0..3], &[230, 230, 230]);
    }

    #[test]
    fn the_backdrop_covers_any_window_shape() {
        // 4:3 portrait picture in a 1:1 view: top and bottom are cropped equally
        let [u0, v0, u1, v1] = cover_uv((1200.0, 1600.0), (500.0, 500.0));
        assert_eq!((u0, u1), (0.0, 1.0));
        assert!((v0 - 0.125).abs() < 1e-6 && (v1 - 0.875).abs() < 1e-6);
        // very tall view: the sides are cropped instead
        let [u0, v0, u1, v1] = cover_uv((1200.0, 1600.0), (300.0, 1200.0));
        assert_eq!((v0, v1), (0.0, 1.0));
        assert!(u0 > 0.0 && (u0 + (1.0 - u1)).abs() > 0.0 && (u0 - (1.0 - u1)).abs() < 1e-6);
        // same shape: nothing cropped; nonsense sizes fall back to the whole picture
        assert_eq!(cover_uv((1200.0, 1600.0), (600.0, 800.0)), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(cover_uv((0.0, 0.0), (600.0, 800.0)), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn the_logo_is_shown_sharp_and_never_wider_than_the_tab() {
        // a 512 px square on a normal screen: native size when there is room
        assert_eq!(fit_logo(900.0, (512.0, 512.0), 1.0), (512.0, 512.0));
        // narrower tab: scaled down, same shape
        assert_eq!(fit_logo(400.0, (512.0, 512.0), 1.0), (400.0, 400.0));
        // 150% scaling: 512 px cover only 341 points, so it is not stretched past that
        let (w, h) = fit_logo(900.0, (512.0, 512.0), 1.5);
        assert!((w - 341.33).abs() < 0.01 && (h - w).abs() < 1e-3);
        // a wide image keeps its shape
        assert_eq!(fit_logo(300.0, (1000.0, 250.0), 1.0), (300.0, 75.0));
        // nothing sensible to draw
        assert_eq!(fit_logo(300.0, (0.0, 0.0), 1.0), (0.0, 0.0));
        assert_eq!(fit_logo(-5.0, (512.0, 512.0), 1.0), (0.0, 0.0));
    }

    #[test]
    fn built_in_note_has_an_app_part_and_an_about_part() {
        let blocks = blocks();
        match blocks.first() {
            Some(Block::Title(t)) => {
                assert_eq!(t, "The Continuation of Rebellion: CR-Helper")
            }
            other => panic!("starts with the welcome title, got {other:?}"),
        }
        let about = blocks
            .iter()
            .position(|b| matches!(b, Block::Title(t) if t == "About CPUnity"))
            .expect("an About CPUnity section");
        assert!(about > 1, "the app part comes first");
        let rebellion = blocks
            .iter()
            .position(|b| matches!(b, Block::Title(t) if t == "The Rebellion"))
            .expect("a The Rebellion section");
        assert!(0 < rebellion && rebellion < about, "The Rebellion comes before About CPUnity");
        assert!(blocks[about + 1..].iter().any(|b| matches!(b, Block::Paragraph(_))));
        assert!(TEXT.contains("unite people behind the PC"));
        // order: app, About CPUnity, Future Endeavors, then the P.S drop-down at the very end
        let future = blocks
            .iter()
            .position(|b| matches!(b, Block::Title(t) if t == "CPUnity: Future Endeavors"))
            .expect("a Future Endeavors section");
        assert!(about < future);
        // contact details: an address that copies and a link, between Future Endeavors and P.S
        let contact = blocks
            .iter()
            .position(|b| matches!(b, Block::Title(t) if t == "Ways to contact"))
            .expect("a Ways to contact section");
        assert!(future < contact);
        assert!(blocks.iter().any(|b| matches!(b, Block::Copy { text, .. } if text.contains('@'))));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Link { url, .. } if url.starts_with("https://github.com/CPUnity/"))));
        // the P.S always stays last: new sections go above it
        let parts = crate::doc::parts(blocks);
        match parts.last() {
            Some(crate::doc::Part::Dropdown { title, body }) => {
                assert_eq!(*title, "P.S");
                assert!(matches!(body, [Block::Paragraph(t)] if t.contains("diapers")));
            }
            other => panic!("the P.S drop-down comes last, got {other:?}"),
        }
        assert!(!TEXT.contains("Fair warning"));
    }

    #[test]
    fn the_cat_is_a_live_backdrop_that_stands_at_the_top_of_the_page_and_is_free_to_use() {
        let s = BackdropStyle::Cat;
        assert_eq!(s.label(), "Circuit City");
        assert!(s.pngs().is_empty() && s.is_painted() && s.is_animated() && s.has_stage());
        assert!(!s.needs_gold() && !s.needs_void() && !s.inside_void_only() && !s.inside_underworld_only() && !s.has_clock() && s.shop_id().is_none());
        assert!(BackdropStyle::available(false, false).contains(&s), "found from the start");
        assert_eq!(BackdropStyle::from_key("cat"), s);
        assert_eq!(BackdropStyle::from_key(s.key()), s);
        assert!(s.info_strength() > 0.0 && s.card_strength() > 0.0);
    }

    #[test]
    fn the_dead_monarch_king_is_only_for_the_void_and_stands_at_the_top_of_the_page() {
        let s = BackdropStyle::DeadKing;
        assert_eq!(s.label(), "Dead Monarch King");
        assert!(s.needs_void() && s.inside_void_only() && !s.needs_gold() && !s.has_clock());
        assert!(s.has_stage() && BackdropStyle::FatherTime.has_stage() && !BackdropStyle::VoidStars.has_stage());
        assert_eq!(BackdropStyle::ALL.iter().filter(|s| s.inside_void_only()).count(), 1, "the glitch stars stay free to use once unlocked");
        assert!(!BackdropStyle::available(true, false).contains(&s));
        assert!(BackdropStyle::available(true, true).contains(&s));
        assert!(s.pngs().is_empty(), "drawn live");
        assert_eq!(BackdropStyle::from_key("deadking"), s);
        assert!(s.is_animated());
    }

    #[test]
    fn every_backdrop_has_a_description() {
        for style in BackdropStyle::ALL {
            assert!(style.blurb().len() > 20, "{}", style.label());
            if let Some(item) = style.shop_id().and_then(crate::shop::item) {
                assert_eq!(style.blurb(), item.blurb, "the shop and the preview say the same");
            }
        }
    }

    #[test]
    fn on_platinum_the_angel_cats_are_the_soul_catsters() {
        use BackdropStyle::AngelCats;
        assert_eq!(AngelCats.label_in(false), "Angel cats");
        assert_eq!(AngelCats.label_in(true), "Soul Catsters");
        assert!(!AngelCats.blurb_in(true).is_empty() && AngelCats.blurb_in(true) != AngelCats.blurb_in(false));
        // nothing else is renamed
        for style in BackdropStyle::ALL.into_iter().filter(|s| *s != AngelCats) {
            assert_eq!(style.label_in(true), style.label());
            assert_eq!(style.blurb_in(true), style.blurb());
            assert_eq!(style.pngs_with(false, true), style.pngs());
        }
        // a different picture, the same three cells (so that the same placements work)
        let (plain, soul) = (AngelCats.pngs_with(false, false), AngelCats.pngs_with(false, true));
        assert_eq!((plain.len(), soul.len()), (1, 1));
        assert_ne!(plain[0], soul[0]);
        // (a PNG says its size at the start: width, then height, four bytes each)
        let dims = |png: &[u8]| (u32::from_be_bytes(png[16..20].try_into().unwrap()), u32::from_be_bytes(png[20..24].try_into().unwrap()));
        assert_eq!(dims(soul[0]), dims(plain[0]));
        // The Void wins over Platinum
        assert_eq!(AngelCats.pngs_with(true, true), AngelCats.pngs_in(true));
        assert_eq!(AngelCats.pngs_with(false, false), AngelCats.pngs());
    }

    #[test]
    fn soul_whispers_belong_to_the_underworld_alone_and_are_drawn_live() {
        let s = BackdropStyle::SoulWhispers;
        assert_eq!(s.label(), "Soul Whispers");
        assert!(s.inside_underworld_only() && !s.inside_void_only() && !s.needs_gold() && !s.needs_void());
        assert_eq!(BackdropStyle::ALL.iter().filter(|s| s.inside_underworld_only()).count(), 1);
        assert!(s.is_painted() && s.pngs().is_empty() && s.is_animated() && s.shop_id().is_none() && !s.has_clock());
        assert_eq!(BackdropStyle::from_key("soulwhispers"), s);
        assert_eq!(BackdropStyle::from_key(s.key()), s);
        assert!(s.blurb().len() > 20);
    }
}
