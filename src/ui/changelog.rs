// Changelog tab: every change per version, from the CHANGELOG.md built into the program.
use crate::doc::{parse, Block, CHANGELOG};
use eframe::egui::{self, RichText};
use std::sync::OnceLock;

pub fn render_changelog(ui: &mut egui::Ui) {
    static BLOCKS: OnceLock<Vec<Block>> = OnceLock::new();
    let blocks = BLOCKS.get_or_init(|| parse(CHANGELOG));
    // the same text size as the Info tab
    ui.scope(|ui| {
        for font in ui.style_mut().text_styles.values_mut() {
            font.size *= super::info::TEXT_SCALE;
        }
        ui.add(
            egui::Label::new(
                RichText::new(format!("You are running version {}.", env!("CARGO_PKG_VERSION"))).weak(),
            )
            .selectable(false),
        );
        ui.add_space(6.0);
        super::doc::render_versions(ui, blocks);
    });
}
