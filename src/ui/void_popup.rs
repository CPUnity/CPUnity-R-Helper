// The popup that says the Void is unlocked, shown once after the cat king's death.
use eframe::egui::{self, Color32, RichText};

const CYAN: Color32 = Color32::from_rgb(70, 240, 255);
const MAGENTA: Color32 = Color32::from_rgb(255, 60, 200);
const PURPLE: Color32 = Color32::from_rgb(170, 110, 255);

/// Draws the popup while `open`; it closes with OK or the cross.
pub fn render(ctx: &egui::Context, open: &mut bool) {
    if !*open {
        return;
    }
    let mut window_open = true;
    let mut ok = false;
    let frame = egui::Frame::window(&ctx.style())
        .fill(Color32::from_rgb(10, 5, 20))
        .stroke(egui::Stroke::new(1.5, PURPLE));
    egui::Window::new("The Void")
        .open(&mut window_open)
        .frame(frame)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                // the title is torn in two colours, a pixel apart
                let title = RichText::new("VOID UNLOCKED").monospace().size(26.0);
                let rect = ui.label(title.clone().color(CYAN)).rect;
                ui.painter().text(
                    rect.center() + egui::vec2(2.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    "VOID UNLOCKED",
                    egui::FontId::monospace(26.0),
                    MAGENTA.gamma_multiply(0.75),
                );
                ui.add_space(8.0);
            });
            ui.add(egui::Label::new("The cat king is dead, and what he left behind took everyone with it. Heaven, the angels, you.").wrap());
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new("The Void is on. You can switch it off, or lock it again, in Options > The Void.").color(PURPLE)).wrap());
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if ui.button("  OK  ").clicked() {
                    ok = true;
                }
            });
        });
    if ok || !window_open {
        *open = false;
    }
}

/// The popup after Game 3's ending: the Underworld is open. When this build holds the Underworld back
/// (`released` is false) it says to be continued instead; the custom songs are unlocked either way.
pub fn render_underworld(ctx: &egui::Context, open: &mut bool, released: bool) {
    if !*open {
        return;
    }
    let mut window_open = true;
    let mut ok = false;
    let frame = egui::Frame::window(&ctx.style())
        .fill(Color32::from_rgb(10, 5, 20))
        .stroke(egui::Stroke::new(1.5, PURPLE));
    egui::Window::new("The Underworld")
        .open(&mut window_open)
        .frame(frame)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                let heading = if released { "THE UNDERWORLD" } else { "TO BE CONTINUED" };
                let title = RichText::new(heading).monospace().size(26.0);
                let rect = ui.label(title.color(CYAN)).rect;
                ui.painter().text(
                    rect.center() + egui::vec2(2.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    heading,
                    egui::FontId::monospace(26.0),
                    MAGENTA.gamma_multiply(0.75),
                );
                ui.add_space(8.0);
            });
            ui.add(egui::Label::new("You were patient. It did not matter. What he opened is on the other side.").wrap());
            ui.add_space(6.0);
            let line = if released {
                "The Underworld is open. Its look is not built yet."
            } else {
                "The Underworld is not ready yet. It is coming in an update, and it will be waiting for you right where you left off."
            };
            ui.add(egui::Label::new(RichText::new(line).color(PURPLE)).wrap());
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    RichText::new("Custom tracks are now enabled for the \"Patience For Death\" mini game inside \"The Void\": add your own mp3s on its first screen under CUSTOM SONGS, to play one at a time or as a playlist.")
                        .color(CYAN),
                )
                .wrap(),
            );
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if ui.button("  OK  ").clicked() {
                    ok = true;
                }
            });
        });
    if ok || !window_open {
        *open = false;
    }
}
