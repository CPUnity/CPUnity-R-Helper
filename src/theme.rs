// Global accent color (the "highlight green"), changeable at runtime from the Options window.
use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

// Packed 0x00RRGGBB. Default matches the original R-Helper green (0, 120, 60).
static ACCENT: AtomicU32 = AtomicU32::new(0x0000_783C);

pub const DEFAULT_ACCENT: Color32 = Color32::from_rgb(0, 120, 60);

pub fn set_accent(c: Color32) {
    let packed = ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | (c.b() as u32);
    ACCENT.store(packed, Ordering::Relaxed);
}

pub fn accent() -> Color32 {
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

fn mix(a: Color32, b: Color32, t_toward_b: f32) -> Color32 {
    let m = |x: u8, y: u8| -> u8 {
        ((x as f32) * (1.0 - t_toward_b) + (y as f32) * t_toward_b).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

// ---- palette (dark, slightly cool greys; the accent color is user-selectable) ----
pub const BG: Color32 = Color32::from_rgb(13, 14, 17);
pub const SURFACE: Color32 = Color32::from_rgb(22, 24, 29);
pub const SURFACE_2: Color32 = Color32::from_rgb(33, 36, 43);
pub const BORDER: Color32 = Color32::from_rgb(44, 48, 56);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(62, 67, 77);
pub const TEXT: Color32 = Color32::from_rgb(226, 229, 235);
pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 156, 168);

/// Applies the full app theme (colors, rounding, spacing, text sizes) to egui.
/// Called at startup and whenever the accent color changes.
pub fn apply(ctx: &egui::Context) {
    let accent = accent();
    ctx.style_mut(|style| {
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = BG;
        v.window_fill = SURFACE;
        v.window_stroke = Stroke::new(1.0, BORDER_LIGHT);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(8);
        v.extreme_bg_color = Color32::from_rgb(10, 11, 13);
        v.faint_bg_color = SURFACE_2;
        v.slider_trailing_fill = true;
        v.selection.bg_fill = accent;
        v.selection.stroke = Stroke::new(1.0, Color32::WHITE);

        let r = CornerRadius::same(8);
        v.widgets.noninteractive.corner_radius = r;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);

        v.widgets.inactive.corner_radius = r;
        v.widgets.inactive.bg_fill = SURFACE_2;
        v.widgets.inactive.weak_bg_fill = SURFACE_2;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_LIGHT);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);

        let hover_fill = mix(SURFACE_2, accent, 0.28);
        v.widgets.hovered.corner_radius = r;
        v.widgets.hovered.bg_fill = hover_fill;
        v.widgets.hovered.weak_bg_fill = hover_fill;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent_stroke());
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);

        v.widgets.active.corner_radius = r;
        v.widgets.active.bg_fill = accent;
        v.widgets.active.weak_bg_fill = accent;
        v.widgets.active.bg_stroke = Stroke::new(1.0, accent_stroke());
        v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);

        v.widgets.open.corner_radius = r;
        v.widgets.open.bg_fill = SURFACE_2;
        v.widgets.open.weak_bg_fill = SURFACE_2;
        v.widgets.open.bg_stroke = Stroke::new(1.0, accent_stroke());

        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        style.spacing.slider_rail_height = 7.0;

        style.text_styles.insert(TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Body, FontId::new(14.5, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Button, FontId::new(14.5, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
    });
}

/// A rounded "card" container used for every section of the main window.
pub fn card<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        })
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
