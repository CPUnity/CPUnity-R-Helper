// The painted pictures the shadow monarch is made of: the tear in space, his body and cloak, his
// two faces, and his talons. They are PNGs in assets/ (made by a small renderer, drawn dark with
// violet light round the edges) that the scene lays over each other and moves; monarch.rs says
// where and when. If one cannot be read, `art` gives None and the scene falls back to the plain
// shapes it was made of before.
//
// The pictures are stored premultiplied: each colour is already multiplied by its opacity, and a
// colour brighter than its opacity is light that adds to what is behind it (the glow, the eyes).
use crate::monarch::Pt;
use eframe::egui::{epaint::Vertex, pos2, Color32, ColorImage, Context, Id, Mesh, Painter, Pos2, Shape, TextureHandle, TextureOptions};
use std::sync::Arc;

const BODY_PNG: &[u8] = include_bytes!("../../assets/monarch-body.png");
const HEAD_A_PNG: &[u8] = include_bytes!("../../assets/monarch-head-a.png");
const HEAD_B_PNG: &[u8] = include_bytes!("../../assets/monarch-head-b.png");
const HAND_CURLED_PNG: &[u8] = include_bytes!("../../assets/monarch-hand-curled.png");
const HAND_OPEN_PNG: &[u8] = include_bytes!("../../assets/monarch-hand-open.png");
const RIFT_PNG: &[u8] = include_bytes!("../../assets/monarch-rift.png");

/// One painted layer: the texture, which pixel of it is the origin of the figure (or of the hole,
/// or of the hand's palm), and how many pixels one unit of the figure is.
#[derive(Clone)]
pub struct Sprite {
    tex: TextureHandle,
    size: [usize; 2],
    origin: (f32, f32),
    px_per_unit: f32,
}

impl Sprite {
    /// The picture's edges in the units of the thing it belongs to: left, top, right, bottom.
    pub fn units(&self) -> (f32, f32, f32, f32) {
        let k = self.px_per_unit;
        (-self.origin.0 / k, -self.origin.1 / k, (self.size[0] as f32 - self.origin.0) / k, (self.size[1] as f32 - self.origin.1) / k)
    }

    /// Draws a quadrilateral with `corners` (top left, top right, bottom right, bottom left) showing
    /// the part of the picture from `v0` to `v1` of the way down it.
    fn quad(&self, painter: &Painter, corners: [Pos2; 4], v0: f32, v1: f32, tint: Color32) {
        let uv = [pos2(0.0, v0), pos2(1.0, v0), pos2(1.0, v1), pos2(0.0, v1)];
        let mut mesh = Mesh::with_texture(self.tex.id());
        for (pos, uv) in corners.into_iter().zip(uv) {
            mesh.vertices.push(Vertex { pos, uv, color: tint });
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        painter.add(Shape::mesh(mesh));
    }

    /// All of it, on a quadrilateral with `corners` (top left, top right, bottom right, bottom left).
    pub fn draw_quad(&self, painter: &Painter, corners: [Pos2; 4], tint: Color32) {
        self.quad(painter, corners, 0.0, 1.0, tint);
    }

    /// All of it, with its corners put on the screen by `map` (which takes a point in units).
    pub fn draw(&self, painter: &Painter, map: &dyn Fn(Pt) -> Pos2, tint: Color32) {
        let (x0, y0, x1, y1) = self.units();
        self.quad(painter, [map((x0, y0)), map((x1, y0)), map((x1, y1)), map((x0, y1))], 0.0, 1.0, tint);
    }

    /// A horizontal strip of it, from `v0` to `v1` of the way down, moved `dx` units sideways.
    pub fn draw_strip(&self, painter: &Painter, map: &dyn Fn(Pt) -> Pos2, v0: f32, v1: f32, dx: f32, tint: Color32) {
        let (x0, y0, x1, y1) = self.units();
        let (ya, yb) = (y0 + (y1 - y0) * v0, y0 + (y1 - y0) * v1);
        self.quad(painter, [map((x0 + dx, ya)), map((x1 + dx, ya)), map((x1 + dx, yb)), map((x0 + dx, yb))], v0, v1, tint);
    }
}

pub struct Art {
    pub body: Sprite,
    pub head_a: Sprite,
    pub head_b: Sprite,
    pub hand_curled: Sprite,
    pub hand_open: Sprite,
    pub rift: Sprite,
}

fn sprite(ctx: &Context, name: &str, png: &[u8], origin: (f32, f32), px_per_unit: f32) -> Option<Sprite> {
    let img = image::load_from_memory(png).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let colours = ColorImage::from_rgba_premultiplied(size, img.as_raw());
    let tex = ctx.load_texture(name, colours, TextureOptions::LINEAR);
    Some(Sprite { tex, size, origin, px_per_unit })
}

fn load(ctx: &Context) -> Option<Art> {
    Some(Art {
        body: sprite(ctx, "cpunity_monarch_body", BODY_PNG, (375.0, 435.0), 1.5)?,
        head_a: sprite(ctx, "cpunity_monarch_head_a", HEAD_A_PNG, (195.0, 180.0), 1.5)?,
        head_b: sprite(ctx, "cpunity_monarch_head_b", HEAD_B_PNG, (195.0, 180.0), 1.5)?,
        hand_curled: sprite(ctx, "cpunity_monarch_hand_curled", HAND_CURLED_PNG, (120.0, 260.0), 2.0)?,
        hand_open: sprite(ctx, "cpunity_monarch_hand_open", HAND_OPEN_PNG, (120.0, 260.0), 2.0)?,
        rift: sprite(ctx, "cpunity_monarch_rift", RIFT_PNG, (500.0, 600.0), 2.0)?,
    })
}

/// The pictures, made the first time they are asked for (None if any could not be read).
pub fn art(ctx: &Context) -> Option<Arc<Art>> {
    let id = Id::new("cpunity_monarch_art");
    if let Some(done) = ctx.data(|d| d.get_temp::<Option<Arc<Art>>>(id)) {
        return done;
    }
    let made = load(ctx).map(Arc::new);
    ctx.data_mut(|d| d.insert_temp(id, made.clone()));
    made
}

/// Opacity as the tint of a picture: it fades the whole of it, light and all.
pub fn fade(a: f32) -> Color32 {
    Color32::from_white_alpha((a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// A picture seen through a coloured glass at opacity `a` (for the cyan and magenta ghosts).
pub fn ghost(rgb: [u8; 3], a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    let f = |c: u8| (c as f32 * a) as u8;
    Color32::from_rgba_premultiplied(f(rgb[0]), f(rgb[1]), f(rgb[2]), (a * 255.0) as u8)
}
