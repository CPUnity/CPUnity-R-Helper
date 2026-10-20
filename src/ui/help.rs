// Help tab: what each function does, one drop-down per function, at the same text size as Info
// and Changelog. The cards link here with `link`.
use eframe::egui::{self, RichText};

use crate::help::{topic, TOPICS};

/// The topic a card's link asked for (taken by the main window).
const REQUEST: &str = "cpunity_help_request";

/// A small "What does this do?" link for the bottom of a card. Clicking it opens the Help tab at
/// that function.
pub fn link(ui: &mut egui::Ui, key: &str) {
    debug_assert!(topic(key).is_some(), "no help topic called {key}");
    let response = ui
        .add(
            egui::Label::new(RichText::new("\u{2753} What does this do?").small().underline())
                .sense(egui::Sense::click()),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Opens the Help tab at this function.");
    if response.clicked() {
        let key = key.to_string();
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(REQUEST), key));
        ui.ctx().request_repaint();
    }
}

/// The topic a link asked for since the last call, if any.
pub fn take_request(ctx: &egui::Context) -> Option<String> {
    ctx.data_mut(|d| {
        let key = d.get_temp::<String>(egui::Id::new(REQUEST));
        if key.is_some() {
            d.remove::<String>(egui::Id::new(REQUEST));
        }
        key
    })
}

/// The key `theme::collapsing` remembers a topic's open state under.
pub fn open_key(topic_key: &str) -> String {
    format!("help_{topic_key}")
}

pub fn render_help(ui: &mut egui::Ui) {
    // the same text size as the Info tab
    ui.scope(|ui| {
        for font in ui.style_mut().text_styles.values_mut() {
            font.size *= super::info::TEXT_SCALE;
        }
        ui.add(
            egui::Label::new(
                RichText::new("What each function does. Every card with more to say than one line has its explanation here.")
                    .weak(),
            )
            .selectable(false),
        );
        ui.add_space(6.0);
        for t in TOPICS {
            crate::theme::collapsing(ui, &open_key(t.key), crate::theme::section_title(t.title), false, |ui| {
                for (i, paragraph) in t.text.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(4.0);
                    }
                    ui.add(egui::Label::new(*paragraph).wrap().selectable(false));
                }
            });
            ui.add_space(4.0);
        }
    });
}
