// The cards of the Devil's Table: engraved sterling-silver plates, drawn once as pictures (see tools/ for
// the program that draws them) and shown as textures, so that every line of the engraving stays sharp.
// A card is a face (0..51: suit * 13 + rank, as in poker_cards.rs) or the back (BACK).
use eframe::egui::{self, pos2, Color32, ColorImage, Mesh, Painter, Rect, Shape, TextureHandle, TextureId, TextureOptions};
use image::imageops::FilterType;
use std::cell::RefCell;
use std::collections::HashMap;

/// The index of the back.
pub const BACK: usize = 52;

static FACES: [&[u8]; 52] = [
    include_bytes!("../../assets/cards/c00.png"),
    include_bytes!("../../assets/cards/c01.png"),
    include_bytes!("../../assets/cards/c02.png"),
    include_bytes!("../../assets/cards/c03.png"),
    include_bytes!("../../assets/cards/c04.png"),
    include_bytes!("../../assets/cards/c05.png"),
    include_bytes!("../../assets/cards/c06.png"),
    include_bytes!("../../assets/cards/c07.png"),
    include_bytes!("../../assets/cards/c08.png"),
    include_bytes!("../../assets/cards/c09.png"),
    include_bytes!("../../assets/cards/c10.png"),
    include_bytes!("../../assets/cards/c11.png"),
    include_bytes!("../../assets/cards/c12.png"),
    include_bytes!("../../assets/cards/c13.png"),
    include_bytes!("../../assets/cards/c14.png"),
    include_bytes!("../../assets/cards/c15.png"),
    include_bytes!("../../assets/cards/c16.png"),
    include_bytes!("../../assets/cards/c17.png"),
    include_bytes!("../../assets/cards/c18.png"),
    include_bytes!("../../assets/cards/c19.png"),
    include_bytes!("../../assets/cards/c20.png"),
    include_bytes!("../../assets/cards/c21.png"),
    include_bytes!("../../assets/cards/c22.png"),
    include_bytes!("../../assets/cards/c23.png"),
    include_bytes!("../../assets/cards/c24.png"),
    include_bytes!("../../assets/cards/c25.png"),
    include_bytes!("../../assets/cards/c26.png"),
    include_bytes!("../../assets/cards/c27.png"),
    include_bytes!("../../assets/cards/c28.png"),
    include_bytes!("../../assets/cards/c29.png"),
    include_bytes!("../../assets/cards/c30.png"),
    include_bytes!("../../assets/cards/c31.png"),
    include_bytes!("../../assets/cards/c32.png"),
    include_bytes!("../../assets/cards/c33.png"),
    include_bytes!("../../assets/cards/c34.png"),
    include_bytes!("../../assets/cards/c35.png"),
    include_bytes!("../../assets/cards/c36.png"),
    include_bytes!("../../assets/cards/c37.png"),
    include_bytes!("../../assets/cards/c38.png"),
    include_bytes!("../../assets/cards/c39.png"),
    include_bytes!("../../assets/cards/c40.png"),
    include_bytes!("../../assets/cards/c41.png"),
    include_bytes!("../../assets/cards/c42.png"),
    include_bytes!("../../assets/cards/c43.png"),
    include_bytes!("../../assets/cards/c44.png"),
    include_bytes!("../../assets/cards/c45.png"),
    include_bytes!("../../assets/cards/c46.png"),
    include_bytes!("../../assets/cards/c47.png"),
    include_bytes!("../../assets/cards/c48.png"),
    include_bytes!("../../assets/cards/c49.png"),
    include_bytes!("../../assets/cards/c50.png"),
    include_bytes!("../../assets/cards/c51.png"),
];
static BACK_PNG: &[u8] = include_bytes!("../../assets/cards/back.png");
/// The shape of the big silver symbol of each suit (clubs, diamonds, hearts, spades), a white picture with
/// the shape as its alpha: the gleam is shown through it.
static GLINTS: [&[u8]; 4] = [
    include_bytes!("../../assets/cards/glint0.png"),
    include_bytes!("../../assets/cards/glint1.png"),
    include_bytes!("../../assets/cards/glint2.png"),
    include_bytes!("../../assets/cards/glint3.png"),
];

/// A card seen smaller than this many physical pixels across is shown from a half-size copy (so that the
/// fine lines are averaged by a proper filter, not skipped by the graphics card).
const SMALL_BELOW_PX: f32 = 210.0;

#[derive(Default)]
struct Store {
    ctx: Option<egui::Context>,
    big: HashMap<usize, TextureHandle>,
    small: HashMap<usize, TextureHandle>,
    glints: HashMap<usize, TextureHandle>,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::default());
}

/// Tells the cards which egui context to put their textures in (call it every frame; it is cheap).
pub fn use_context(ctx: &egui::Context) {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        if s.ctx.is_none() {
            s.ctx = Some(ctx.clone());
        }
    });
}

fn bytes(index: usize) -> &'static [u8] {
    if index >= BACK {
        BACK_PNG
    } else {
        FACES[index]
    }
}

/// The texture of a card to show `points_wide` across (None if the picture cannot be read).
pub fn texture(index: usize, points_wide: f32, pixels_per_point: f32) -> Option<TextureId> {
    let small = points_wide * pixels_per_point < SMALL_BELOW_PX;
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        let ctx = s.ctx.clone()?;
        let map = if small { &s.small } else { &s.big };
        if let Some(t) = map.get(&index) {
            return Some(t.id());
        }
        let mut img = image::load_from_memory(bytes(index)).ok()?.to_rgba8();
        if small {
            img = image::imageops::resize(&img, img.width() / 2, img.height() / 2, FilterType::Lanczos3);
        }
        let size = [img.width() as usize, img.height() as usize];
        let name = format!("poker_card_{}_{}", index, if small { "s" } else { "b" });
        let tex = ctx.load_texture(name, ColorImage::from_rgba_unmultiplied(size, img.as_raw()), TextureOptions::LINEAR);
        let id = tex.id();
        if small {
            s.small.insert(index, tex);
        } else {
            s.big.insert(index, tex);
        }
        Some(id)
    })
}

/// Paints a card into `r`; false if its picture is not there (the caller draws the plain one).
pub fn paint(p: &Painter, r: Rect, index: usize, shadow: bool) -> bool {
    let Some(id) = texture(index, r.width(), p.ctx().pixels_per_point()) else {
        return false;
    };
    let round = (r.width() * 0.07).max(2.0);
    if shadow {
        p.rect_filled(r.translate(eframe::egui::vec2(0.0, r.height() * 0.05)).expand(r.width() * 0.012), round, Color32::from_black_alpha(150));
    }
    let mut m = Mesh::with_texture(id);
    m.add_rect_with_uv(r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    p.add(Shape::mesh(m));
    true
}

/// Whether the card (0..51) has the big silver symbol in the middle (the ace and the twos to the tens).
pub fn has_centre(index: usize) -> bool {
    index < BACK && matches!(index % 13, 0..=8 | 12)
}

/// Whether the card has the two wells that souls rise from (the same cards).
pub fn has_wells(index: usize) -> bool {
    has_centre(index)
}

/// Where the middle of the big symbol is on a card, and how wide its square is, as shares of the card's width.
const CENTRE_X: f32 = 0.4948;
const CENTRE_Y: f32 = 0.4963;
const CENTRE_SIDE: f32 = 0.4948;
/// The wells, as shares of the card's height.
pub const WELL_TOP: f32 = 0.222;
pub const WELL_BOTTOM: f32 = 0.778;

fn glint_texture(suit: usize) -> Option<TextureId> {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        let ctx = s.ctx.clone()?;
        if let Some(t) = s.glints.get(&suit) {
            return Some(t.id());
        }
        let img = image::load_from_memory(GLINTS[suit.min(3)]).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let tex = ctx.load_texture(format!("poker_glint_{suit}"), ColorImage::from_rgba_unmultiplied(size, img.as_raw()), TextureOptions::LINEAR);
        let id = tex.id();
        s.glints.insert(suit, tex);
        Some(id)
    })
}

/// How bright the gleam is at `u` (0 across the symbol's width and height along the sweep, 1 the far side) at time `t`:
/// a narrow bright band with a soft wide one behind it, rolling over from the upper left to the lower right,
/// then a pause before the next.
fn gleam_at(u: f32, phase: f32) -> f32 {
    // the sweep goes from -0.35 to 1.35 and back to waiting, and a second softer sweep trails it
    let band = |c: f32, w: f32| (-((u - c) / w).powi(2)).exp();
    let (a, b) = (phase, phase - 0.07);
    band(-0.35 + a * 1.7, 0.05) * 1.0 + band(-0.35 + b * 1.7, 0.16) * 0.40
}

/// Rolls a gleam over the big silver symbol of a face-up card at `r` (does nothing for the cards that have none).
/// `t` is time in seconds; each card has its own phase so that they do not flash together.
pub fn gleam(p: &Painter, r: Rect, index: usize, t: f32) {
    if !has_centre(index) || r.width() < 40.0 {
        return;
    }
    const EVERY: f32 = 4.6;
    const SWEEP: f32 = 1.5;
    let off = ((index as f32 * 2.399).fract()) * EVERY;
    let k = (t + off) % EVERY;
    if k > SWEEP {
        return;
    }
    let phase = k / SWEEP;
    let Some(id) = glint_texture(index / 13) else {
        return;
    };
    let side = r.width() * CENTRE_SIDE;
    let centre = pos2(r.left() + r.width() * CENTRE_X, r.top() + r.height() * CENTRE_Y);
    let area = Rect::from_center_size(centre, eframe::egui::vec2(side, side));
    let mut m = Mesh::with_texture(id);
    const N: usize = 14;
    // an additive layer: the colour is added to what is there, the picture's alpha keeps it inside the symbol
    for j in 0..=N {
        for i in 0..=N {
            let (fx, fy) = (i as f32 / N as f32, j as f32 / N as f32);
            let u = (fx * 0.55 + fy * 0.45).clamp(0.0, 1.0);
            let v = (gleam_at(u, phase) * 255.0).clamp(0.0, 255.0) as u8;
            m.vertices.push(eframe::egui::epaint::Vertex { pos: pos2(area.left() + area.width() * fx, area.top() + area.height() * fy), uv: pos2(fx, fy), color: Color32::from_rgba_premultiplied(v, v, (v as f32 * 0.98) as u8, 0) });
        }
    }
    for j in 0..N {
        for i in 0..N {
            let a = (j * (N + 1) + i) as u32;
            let (b, c, d) = (a + 1, a + (N + 1) as u32, a + (N + 1) as u32 + 1);
            m.indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    p.add(Shape::mesh(m));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_symbol_shapes_are_pictures_with_a_shape() {
        for (i, g) in GLINTS.iter().enumerate() {
            let img = image::load_from_memory(g).unwrap_or_else(|_| panic!("glint {i}")).to_rgba8();
            assert_eq!((img.width(), img.height()), (190, 190));
            let solid = img.pixels().filter(|p| p[3] > 200).count();
            assert!(solid > 3000 && solid < 20000, "glint {i} has {solid} solid pixels");
        }
    }

    #[test]
    fn the_gleam_comes_and_goes() {
        assert!(gleam_at(0.5, 0.0) < 0.05);
        let peak = (0..100).map(|k| gleam_at(0.5, k as f32 / 100.0)).fold(0.0_f32, f32::max);
        assert!(peak > 0.9, "{peak}");
    }

    #[test]
    fn only_the_ace_and_the_twos_to_tens_have_the_big_symbol() {
        let with: Vec<usize> = (0..13).filter(|&i| has_centre(i)).collect();
        assert_eq!(with, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 12]);
        assert!(!has_centre(BACK));
    }

    #[test]
    fn every_card_picture_is_a_card() {
        for i in 0..=BACK {
            let img = image::load_from_memory(bytes(i)).unwrap_or_else(|_| panic!("card {i} is not a picture")).to_rgba8();
            assert_eq!((img.width(), img.height()), (384, 537), "card {i}");
            // the corners are cut round (clear) and the middle is solid
            assert_eq!(img.get_pixel(0, 0)[3], 0, "card {i} corner");
            assert!(img.get_pixel(192, 268)[3] >= 250, "card {i} middle");
        }
    }
}
