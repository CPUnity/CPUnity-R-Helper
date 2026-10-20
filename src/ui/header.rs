use crate::messaging::{MessageManager, MessageType};
use crate::system::SystemSpecs;
use eframe::egui::{self, Align, Color32, Layout, RichText};
use librazer::device::Device;
use std::sync::atomic::{AtomicU32, Ordering};

/// Height of the optional header banner, in egui points.
pub const BANNER_HEIGHT: f32 = 52.0;
/// Below this width the banner is not drawn (narrow windows fall back to the plain header).
const BANNER_MIN_WIDTH: f32 = 120.0;

/// Last measured banner slot width in points (f32 bits), 0 until the header was drawn.
static SLOT_WIDTH_BITS: AtomicU32 = AtomicU32::new(0);

/// Size of the free header space right of the title, in points: `(width, height)`.
pub fn banner_slot() -> Option<(f32, f32)> {
    let w = f32::from_bits(SLOT_WIDTH_BITS.load(Ordering::Relaxed));
    if w >= BANNER_MIN_WIDTH {
        Some((w, BANNER_HEIGHT))
    } else {
        None
    }
}

const FADE_START_TIME: f32 = 3.0;
const FADE_DURATION: f32 = 2.0;
const FULL_ALPHA: u8 = 255;

/// Renders the application header with device name and status messages
pub fn render_header(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    loading: bool,
    _system_specs: &SystemSpecs,
    device: &Option<Device>,
    message_manager: &MessageManager,
    detecting_device: bool,
    logo: Option<&egui::TextureHandle>,
    banner: Option<&egui::TextureHandle>,
    logo_height: f32,
) {
    ui.horizontal(|ui| {
        // Optional user logo (logo.png next to the exe or in %APPDATA%\CPUnity-R-Helper)
        if let Some(tex) = logo {
            let size = tex.size_vec2();
            let h = logo_height;
            let w = size.x * h / size.y.max(1.0);
            ui.add(egui::Image::from_texture(egui::load::SizedTexture::new(
                tex.id(),
                egui::vec2(w, h),
            )));
        }

        // Branding + device name
        render_title(ui);

        // Everything to the right of the title is the banner slot.
        // (egui already advanced the cursor by item_spacing after the title, so no extra gap here.)
        let room = ui.available_width().max(0.0);
        SLOT_WIDTH_BITS.store(room.to_bits(), Ordering::Relaxed);

        if let (Some(tex), true) = (banner, room >= BANNER_MIN_WIDTH) {
            render_banner(ui, ctx, tex, room, loading, message_manager, device, detecting_device);
            return;
        }

        // Status messages and connection status
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if loading {
                ui.spinner();
            }

            // Status/warning messages
            render_status_messages(ui, ctx, message_manager, device, detecting_device);
        });
    });
}

/// Draws the banner image across the whole slot (centre-cropped so any aspect ratio
/// fills it without stretching) and floats status messages on top of it.
fn render_banner(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    tex: &egui::TextureHandle,
    width: f32,
    loading: bool,
    message_manager: &MessageManager,
    device: &Option<Device>,
    detecting_device: bool,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, BANNER_HEIGHT), egui::Sense::hover());

    // "Cover" fit: crop the overflowing axis equally on both sides.
    let size = tex.size_vec2();
    let img_aspect = size.x / size.y.max(1.0);
    let box_aspect = rect.width() / rect.height().max(1.0);
    let uv = if img_aspect > box_aspect {
        let off = (1.0 - box_aspect / img_aspect) / 2.0;
        egui::Rect::from_min_max(egui::pos2(off, 0.0), egui::pos2(1.0 - off, 1.0))
    } else {
        let off = (1.0 - img_aspect / box_aspect) / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, off), egui::pos2(1.0, 1.0 - off))
    };

    let radius = egui::CornerRadius::same(10);
    ui.put(
        rect,
        egui::Image::from_texture(egui::load::SizedTexture::new(tex.id(), rect.size()))
            .fit_to_exact_size(rect.size())
            .uv(uv)
            .corner_radius(radius),
    );
    ui.painter().rect_stroke(
        rect,
        radius,
        egui::Stroke::new(1.0, crate::theme::border()),
        egui::StrokeKind::Inside,
    );

    // Status text floats over the banner in a dark pill so it stays readable.
    let has_status = loading || message_manager.get_current_message().is_some() || device.is_none();
    if has_status {
        let mut overlay = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(6.0))
                .layout(Layout::right_to_left(Align::Center)),
        );
        egui::Frame::new()
            .fill(Color32::from_black_alpha(175))
            .corner_radius(egui::CornerRadius::same(7))
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(&mut overlay, |ui| {
                if loading {
                    ui.spinner();
                }
                render_status_messages(ui, ctx, message_manager, device, detecting_device);
            });
    }
}

/// Renders the branding title ("CPUnity" in the accent color, the rest in white).
fn render_title(ui: &mut egui::Ui) {
    let (first, rest) = crate::APP_NAME.split_once(':').unwrap_or((crate::APP_NAME, ""));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add(
            egui::Label::new(
                RichText::new(first).heading().strong().color(crate::theme::accent_stroke()),
            )
            .selectable(false),
        );
        if !rest.is_empty() {
            ui.add(
                egui::Label::new(RichText::new(format!(":{}", rest)).heading().strong())
                    .selectable(false),
            );
        }
    });
}

/// Renders status messages with fade animation
fn render_status_messages(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    message_manager: &MessageManager,
    device: &Option<Device>,
    detecting_device: bool,
) {
    if let Some(current_message) = message_manager.get_current_message() {
        let elapsed = current_message.age_seconds();

        // Calculate fade and apply to message
        let (base_color, icon) = get_message_style_from_type(&current_message.message_type);
        let alpha = calculate_fade_alpha(elapsed);
        let faded_color = apply_alpha_to_color(base_color, alpha);

        ui.add(
            egui::Label::new(
                RichText::new(format!("{} {}", icon, current_message.content)).color(faded_color),
            )
            .selectable(false),
        );

        // Request repaint for smooth animation
        if current_message.should_fade() {
            ctx.request_repaint();
        }
    } else {
        // Show connection status when no device detected
        if device.is_none() {
            if detecting_device {
                ui.add(
                    egui::Label::new(
                        RichText::new("🔎 Detecting device…").color(Color32::LIGHT_BLUE),
                    )
                    .selectable(false),
                );
                ctx.request_repaint();
            } else {
                ui.add(
                    egui::Label::new(RichText::new("❌ No device detected").color(Color32::RED))
                        .selectable(false),
                );
            }
        }
    }
}

/// Message style based on type
fn get_message_style_from_type(message_type: &MessageType) -> (Color32, &'static str) {
    match message_type {
        MessageType::Info => (Color32::LIGHT_BLUE, "ℹ"),
        MessageType::Error => (Color32::RED, "⚠"),
    }
}

/// Calculates alpha value for fade animation
fn calculate_fade_alpha(elapsed: f32) -> f32 {
    if elapsed < FADE_START_TIME {
        1.0
    } else {
        let fade_progress = (elapsed - FADE_START_TIME) / FADE_DURATION;
        (1.0 - fade_progress).max(0.0)
    }
}

/// Applies alpha transparency to color
fn apply_alpha_to_color(base_color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        base_color.r(),
        base_color.g(),
        base_color.b(),
        (FULL_ALPHA as f32 * alpha) as u8,
    )
}

/// The Gold Edition banner: the full width under the title, with a gold frame and a glint of
/// light that sweeps across now and then (`glint` is 0..1 along the banner, `None` while at rest).
pub fn render_gold_banner(ui: &mut egui::Ui, tex: &egui::TextureHandle, glint: Option<f32>) {
    render_framed_banner(ui, tex, glint, BannerTone::GOLD);
}

/// The Platinum banner: the same shape in steel, with a cool glint.
pub fn render_platinum_banner(ui: &mut egui::Ui, tex: &egui::TextureHandle, glint: Option<f32>) {
    render_framed_banner(ui, tex, glint, BannerTone::PLATINUM);
}

/// The colours a framed banner is drawn with: the glow round it, the glint across it and its edge.
#[derive(Clone, Copy)]
struct BannerTone {
    glow: [u8; 4],
    glint: [u8; 3],
    edge: [u8; 3],
}

impl BannerTone {
    const GOLD: BannerTone = BannerTone { glow: [234, 185, 74, 36], glint: [255, 244, 200], edge: [190, 138, 36] };
    const PLATINUM: BannerTone = BannerTone { glow: [200, 218, 244, 40], glint: [236, 244, 255], edge: [150, 164, 186] };
}

fn render_framed_banner(ui: &mut egui::Ui, tex: &egui::TextureHandle, glint: Option<f32>, tone: BannerTone) {
    let width = ui.available_width();
    if width < BANNER_MIN_WIDTH {
        return;
    }
    let size = tex.size_vec2();
    let height = (width * size.y / size.x.max(1.0)).clamp(64.0, 112.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let [u0, v0, u1, v1] = crate::info::cover_uv((size.x, size.y), (rect.width(), rect.height()));
    let radius = egui::CornerRadius::same(12);
    let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
    let painter = ui.painter();
    // a soft gold glow around it
    painter.rect_stroke(
        rect.expand(2.0),
        egui::CornerRadius::same(14),
        egui::Stroke::new(3.0, Color32::from_rgba_unmultiplied(tone.glow[0], tone.glow[1], tone.glow[2], tone.glow[3])),
        egui::StrokeKind::Middle,
    );
    ui.put(
        rect,
        egui::Image::from_texture(egui::load::SizedTexture::new(tex.id(), rect.size()))
            .fit_to_exact_size(rect.size())
            .uv(uv)
            .corner_radius(radius),
    );
    let painter = ui.painter().with_clip_rect(rect);
    if let Some(x) = glint {
        // a slanted band of light
        let centre = rect.left() - 60.0 + (rect.width() + 120.0) * x;
        let slant = rect.height() * 0.5;
        let half = 26.0;
        let clear = Color32::from_rgba_unmultiplied(tone.glint[0], tone.glint[1], tone.glint[2], 0);
        let lit = Color32::from_rgba_unmultiplied(tone.glint[0], tone.glint[1], tone.glint[2], 90);
        let mut mesh = egui::Mesh::default();
        let (top, bottom) = (rect.top(), rect.bottom());
        mesh.colored_vertex(egui::pos2(centre - half + slant, top), clear);
        mesh.colored_vertex(egui::pos2(centre + slant, top), lit);
        mesh.colored_vertex(egui::pos2(centre + half + slant, top), clear);
        mesh.colored_vertex(egui::pos2(centre - half - slant, bottom), clear);
        mesh.colored_vertex(egui::pos2(centre - slant, bottom), lit);
        mesh.colored_vertex(egui::pos2(centre + half - slant, bottom), clear);
        for (a, b, c) in [(0, 1, 4), (0, 4, 3), (1, 2, 5), (1, 5, 4)] {
            mesh.add_triangle(a, b, c);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    ui.painter().rect_stroke(
        rect,
        radius,
        egui::Stroke::new(1.5, Color32::from_rgb(tone.edge[0], tone.edge[1], tone.edge[2])),
        egui::StrokeKind::Inside,
    );
}

/// The Void's banner under the title: "THE VOID" torn across the middle, the halves drifting apart
/// with slivers of cyan and magenta between them, and columns of fake data down the sides (see
/// `void.rs`; `t` is the time in seconds, 0 holds it still).
pub fn render_void_banner(ui: &mut egui::Ui, t: f32) {
    use crate::theme::{VOID_CYAN, VOID_MAGENTA};
    let width = ui.available_width();
    if width < BANNER_MIN_WIDTH {
        return;
    }
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 92.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect.expand(6.0));
    painter.rect_filled(rect, egui::CornerRadius::same(6), Color32::from_rgb(8, 4, 18));
    crate::ui::backdrop::soft_ellipse(&painter, rect.center(), egui::vec2(rect.width() * 0.3, rect.height() * 0.75), [110, 70, 230], 0.45);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, crate::theme::border_light()),
        egui::StrokeKind::Inside,
    );
    // the edges do not quite line up: cyan on the left, magenta on the right
    painter.line_segment(
        [rect.left_top() + egui::vec2(-2.0, 6.0), rect.left_bottom() + egui::vec2(-2.0, -6.0)],
        egui::Stroke::new(2.0, VOID_CYAN.gamma_multiply(0.8)),
    );
    painter.line_segment(
        [rect.right_top() + egui::vec2(2.0, 6.0), rect.right_bottom() + egui::vec2(2.0, -6.0)],
        egui::Stroke::new(2.0, VOID_MAGENTA.gamma_multiply(0.8)),
    );
    let inside = painter.with_clip_rect(rect);
    // data down both sides
    if width > 460.0 {
        let font = egui::FontId::monospace(11.0);
        let colour = Color32::from_rgb(155, 107, 255).gamma_multiply(0.55);
        for row in 0..4u32 {
            let y = rect.top() + 14.0 + row as f32 * 16.0;
            inside.text(egui::pos2(rect.left() + 12.0, y), egui::Align2::LEFT_TOP, crate::void::data_line(row, 0, t), font.clone(), colour);
            inside.text(egui::pos2(rect.right() - 12.0, y), egui::Align2::RIGHT_TOP, crate::void::data_line(row, 1, t), font.clone(), colour);
        }
    }
    // the title, one letter at a time (the font has no letter spacing), in four layers: the upper
    // half and the lower half of the letters, and a sliver of cyan and of magenta between them
    let tear = crate::void::title_tear(t);
    let size = 36.0;
    let step = size * 1.02;
    let title = "THE VOID";
    let centre = rect.center();
    let x0 = centre.x - step * (title.chars().count() as f32 - 1.0) / 2.0;
    let font = egui::FontId::monospace(size);
    let band = |top: f32, bottom: f32| {
        egui::Rect::from_min_max(egui::pos2(rect.left(), top), egui::pos2(rect.right(), bottom)).intersect(rect)
    };
    let ink = Color32::from_rgb(244, 239, 255).gamma_multiply(1.0 - 0.35 * tear.jolt);
    let layers = [
        (band(rect.top(), centre.y), tear.top, ink),
        (band(centre.y, rect.bottom()), tear.bottom, ink),
        (band(centre.y - 8.0, centre.y - 1.0), tear.cyan, VOID_CYAN.gamma_multiply(0.95)),
        (band(centre.y + 1.0, centre.y + 8.0), tear.magenta, VOID_MAGENTA.gamma_multiply(0.95)),
    ];
    for (clip, dx, colour) in layers {
        let layer = painter.with_clip_rect(clip);
        for (i, ch) in title.chars().enumerate() {
            if ch != ' ' {
                layer.text(
                    egui::pos2(x0 + step * i as f32 + dx, centre.y),
                    egui::Align2::CENTER_CENTER,
                    ch.to_string(),
                    font.clone(),
                    colour,
                );
            }
        }
    }
}
