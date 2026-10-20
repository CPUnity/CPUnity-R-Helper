// The Gold Edition glitch: the whole window breaks into pixels for a moment. egui cannot read its
// own picture, so the program asks the window for a screenshot, and replays it (see gold.rs for
// the timeline): as big blocks, and as strips torn sideways, drawn over everything.
use crate::gold;
use eframe::egui::{self, pos2, Color32};

/// The window as it looked when the glitch began: whole, and in the coarse block sizes.
pub struct Shots {
    full: egui::TextureHandle,
    blocks: Vec<(u32, egui::TextureHandle)>,
}

/// Where the glitch is.
#[derive(Default)]
pub enum Glitch {
    #[default]
    Off,
    /// A screenshot was asked for at this egui time and has not come yet.
    Waiting(f64),
    /// Running since this egui time.
    Running(f64, Shots),
}

/// Asks the window for a screenshot.
pub fn request(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
}

/// The screenshot that arrived this frame, if any.
pub fn arrived(ctx: &egui::Context) -> Option<std::sync::Arc<egui::ColorImage>> {
    ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    })
}

/// Makes the textures from a screenshot.
pub fn shots(ctx: &egui::Context, image: &egui::ColorImage) -> Shots {
    let [w, h] = image.size;
    let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    let full = ctx.load_texture("cpunity_glitch", egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), egui::TextureOptions::LINEAR);
    let blocks = gold::BLOCKS
        .iter()
        .map(|&b| {
            let (bw, bh, bytes) = gold::pixelate(w, h, &rgba, b as usize);
            let tex = ctx.load_texture(
                format!("cpunity_glitch_{}", b),
                egui::ColorImage::from_rgba_unmultiplied([bw, bh], &bytes),
                egui::TextureOptions::NEAREST,
            );
            (b, tex)
        })
        .collect();
    Shots { full, blocks }
}

/// The colours of a glitch: gold for the Gold Edition, violet and cyan for The Void.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Gold,
    Void,
}

impl Tint {
    fn ghost(self) -> Color32 {
        match self {
            Tint::Gold => Color32::from_rgba_unmultiplied(255, 200, 90, 110),
            Tint::Void => Color32::from_rgba_unmultiplied(150, 110, 255, 120),
        }
    }

    fn echo(self) -> Color32 {
        // a second ghost on the other side (the Void's tears are split cyan and magenta)
        match self {
            Tint::Gold => Color32::TRANSPARENT,
            Tint::Void => Color32::from_rgba_unmultiplied(80, 230, 255, 70),
        }
    }

    fn flash(self, a: u8) -> Color32 {
        match self {
            Tint::Gold => Color32::from_rgba_unmultiplied(234, 185, 74, a),
            Tint::Void => Color32::from_rgba_unmultiplied(120, 70, 230, a),
        }
    }
}

/// Draws the glitch `t` seconds in. Returns false once it is over.
pub fn paint(ctx: &egui::Context, shots: &Shots, t: f32) -> bool {
    let Some(frame) = gold::glitch_frame(t) else { return false };
    draw(ctx, shots, &frame, Tint::Gold);
    true
}

/// Draws one picture of a glitch whose strength (0 to 1) is whatever the caller says right now:
/// the moon's glitch, which follows how far the moon has been dragged.
pub fn paint_level(ctx: &egui::Context, shots: &Shots, t: f32, level: f32, tint: Tint) {
    if level < 0.03 {
        return;
    }
    draw(ctx, shots, &gold::glitch_frame_with(t, level), tint);
}

fn draw(ctx: &egui::Context, shots: &Shots, frame: &gold::GlitchFrame, tint: Tint) {
    let screen = ctx.screen_rect();
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("gold_glitch"));
    let painter = ctx.layer_painter(layer).with_clip_rect(screen);
    let all = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    if let Some(block) = frame.block {
        if let Some((_, tex)) = shots.blocks.iter().find(|(b, _)| *b == block) {
            painter.image(tex.id(), screen, all, Color32::WHITE);
        }
    }
    let (w, h) = (screen.width(), screen.height());
    for band in &frame.bands {
        let top = screen.top() + band.top * h;
        let bottom = screen.top() + band.bottom * h;
        let dx = band.shift * w;
        let uv = egui::Rect::from_min_max(pos2(0.0, band.top), pos2(1.0, band.bottom));
        // a ghost of the strip in the opposite direction, tinted, under the strip itself
        let ghost = egui::Rect::from_min_max(pos2(screen.left() - dx * 0.6, top), pos2(screen.right() - dx * 0.6, bottom));
        painter.image(shots.full.id(), ghost, uv, tint.ghost());
        if tint.echo() != Color32::TRANSPARENT {
            let echo = egui::Rect::from_min_max(pos2(screen.left() + dx * 1.5, top), pos2(screen.right() + dx * 1.5, bottom));
            painter.image(shots.full.id(), echo, uv, tint.echo());
        }
        let dst = egui::Rect::from_min_max(pos2(screen.left() + dx, top), pos2(screen.right() + dx, bottom));
        painter.image(shots.full.id(), dst, uv, Color32::WHITE);
    }
    if frame.flash > 0.0 {
        painter.rect_filled(screen, 0.0, tint.flash((frame.flash * 255.0) as u8));
    }
    ctx.request_repaint();
}
