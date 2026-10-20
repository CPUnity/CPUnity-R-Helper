// Info tab: the author's note, built into the program and read-only, under the full-size logo.
use eframe::egui;

/// `logo` is the logo at full resolution (None when there is none).
pub fn render_info(ui: &mut egui::Ui, logo: Option<&egui::TextureHandle>, script: bool, seed: u64) {
    // The text of this page is bigger than elsewhere in the app.
    ui.scope(|ui| {
        for font in ui.style_mut().text_styles.values_mut() {
            font.size *= TEXT_SCALE;
        }
        content(ui, logo, script, seed);
    });
}

/// How much bigger than normal the text of the Info tab is.
pub const TEXT_SCALE: f32 = 1.3;

fn content(ui: &mut egui::Ui, logo: Option<&egui::TextureHandle>, script: bool, seed: u64) {
    if let Some(tex) = logo {
        let px = tex.size_vec2();
        let (w, h) =
            crate::info::fit_logo(ui.available_width(), (px.x, px.y), ui.ctx().pixels_per_point());
        if w > 0.0 {
            ui.vertical_centered(|ui| {
                ui.add(egui::Image::from_texture(egui::load::SizedTexture::new(
                    tex.id(),
                    egui::vec2(w, h),
                )));
            });
            ui.add_space(8.0);
        }
    }
    super::doc::render_with_dropdowns_as(ui, crate::info::blocks_for(script, seed), script);
}
