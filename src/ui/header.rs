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
) {
    ui.horizontal(|ui| {
        // Optional user logo (logo.png next to the exe or in %APPDATA%\CPUnity-R-Helper)
        if let Some(tex) = logo {
            let size = tex.size_vec2();
            let h = 32.0;
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
        egui::Stroke::new(1.0, crate::theme::BORDER),
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
