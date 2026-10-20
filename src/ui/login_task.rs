// Confirmation window for "start at login without the permission prompt": says exactly what is
// going to be changed and only does it after "Continue".
use crate::login_task::{self, Job, State};
use eframe::egui::{self, RichText};

const ORANGE: egui::Color32 = egui::Color32::from_rgb(230, 160, 60);

/// Draws the dialog while it is open. `elevated`: R-Helper has administrator rights (creating
/// the task needs them).
pub fn render_dialog(ctx: &egui::Context, state: &mut State, elevated: bool) {
    if !state.dialog_open {
        return;
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let busy = matches!(state.busy, Some((Job::Create, _)));
    let mut open = true;
    let mut cancel = false;
    let mut go = false;
    egui::Window::new("Start without the permission prompt")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.label(
                "Windows asks for permission every time R-Helper starts with administrator \
                 rights. A scheduled task can start it at login with those rights and no \
                 prompt. If you continue, this is what happens:",
            );
            ui.add_space(4.0);
            for line in [
                format!(
                    "One task named \"{}\" is created in Windows Task Scheduler.",
                    login_task::TASK_NAME
                ),
                "It runs when you log in, with the highest privileges, and may run on battery \
                 with no time limit."
                    .to_string(),
                format!("It starts this program: {}", exe.display()),
                "\"Launch at Windows startup\" is switched off, so you don't get two copies."
                    .to_string(),
                "Nothing else is changed. You can remove the task here later, or in Task \
                 Scheduler."
                    .to_string(),
                "If you move or rename R-Helper, set this up again.".to_string(),
            ] {
                ui.horizontal_top(|ui| {
                    ui.label("\u{2022}");
                    ui.add(egui::Label::new(line).wrap());
                });
            }
            ui.add_space(6.0);
            ui.label(RichText::new("The PowerShell commands that will run:").strong());
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .inner_margin(6.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(login_task::setup_script(&exe)).monospace().small(),
                            )
                            .wrap(),
                        );
                    });
            });
            ui.add_space(6.0);
            if !elevated {
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "R-Helper is not running as administrator, which creating the task \
                             needs. Close this, click \"Restart as administrator now\" in \
                             Options, then open this again.",
                        )
                        .color(ORANGE),
                    )
                    .wrap(),
                );
            }
            if let Some(error) = &state.error {
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("It did not work: {}", error)).color(ORANGE),
                    )
                    .wrap(),
                );
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if busy {
                    ui.spinner();
                    ui.label("Working...");
                } else if ui.add_enabled(elevated, egui::Button::new("Continue")).clicked() {
                    go = true;
                }
                if ui.add_enabled(!busy, egui::Button::new("Cancel")).clicked() {
                    cancel = true;
                }
            });
        });
    if go {
        state.create(&exe);
    }
    if cancel || !open {
        if !busy {
            state.dialog_open = false;
            state.error = None;
        }
    }
    if busy {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
