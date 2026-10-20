// "Synapse standby" group: park / resume Razer Synapse without leaving CPUnity:R-Helper.
use crate::synapse::Status;
use eframe::egui::{self, Color32, RichText};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynapseAction {
    None,
    Standby,
    Resume,
    GuardOn,
    GuardOff,
}

pub fn render_synapse_section(
    ui: &mut egui::Ui,
    status: &Status,
    keep_lighting: &mut bool,
) -> SynapseAction {
    let mut action = SynapseAction::None;

    crate::theme::card(ui, |ui| {
        crate::theme::collapsing(
            ui,
            "synapse_card",
            crate::theme::section_title("🛡 Razer Synapse Standby"),
            true,
            |ui| {
                let (text, color) = status_text(status);
                ui.add(egui::Label::new(RichText::new(text).color(color)).selectable(false));

                ui.horizontal(|ui| {
            ui.add_enabled_ui(!status.busy, |ui| {
                if ui
                    .button("Standby")
                    .on_hover_text("Stop Synapse services, tasks, autostarts and processes (UAC prompt). Hardware keeps the last applied settings.")
                    .clicked()
                {
                    action = SynapseAction::Standby;
                }
                if ui
                    .button("Resume Synapse")
                    .on_hover_text("Restore everything Standby changed and remove the boot guard (UAC prompt).")
                    .clicked()
                {
                    action = SynapseAction::Resume;
                }
            });
            if status.busy {
                ui.spinner();
            }
        });

                ui.add_enabled_ui(!status.busy, |ui| {
            crate::theme::toggle(ui, keep_lighting, "Keep Chroma lighting running").on_hover_text(
                "Leaves only the Chroma half of the Razer engine alive so animated effects keep \
                 working. Off = everything Razer is stopped (static / onboard lighting only).",
            );

            let mut guard = status.guard;
            if crate::theme::toggle(ui, &mut guard, "Auto-park at every boot / logon")
                .on_hover_text(
                    "Installs a startup task that re-applies Standby, so Synapse updates can't \
                     quietly re-enable themselves. Uses the lighting option as set when you tick it.",
                )
                .clicked()
            {
                action = if guard { SynapseAction::GuardOn } else { SynapseAction::GuardOff };
            }
        });

                if let Some(msg) = &status.message {
                    ui.add(
                        egui::Label::new(RichText::new(msg).color(Color32::from_rgb(220, 120, 80)))
                            .selectable(false),
                    );
                }
            },
        );
    });

    action
}

fn status_text(s: &Status) -> (String, Color32) {
    let grey = Color32::from_gray(170);
    let green = crate::theme::accent_stroke();
    let amber = Color32::from_rgb(220, 170, 70);
    if s.busy {
        return ("Working… approve the Windows UAC prompt".to_string(), amber);
    }
    match (s.parked, s.procs) {
        (true, 0) => ("Parked: all Razer software stopped".to_string(), green),
        (true, n) => {
            (format!("Parked: {} Razer processes left (Chroma engine), {} MB", n, s.mem_mb), green)
        }
        (false, 0) => ("Synapse not running".to_string(), grey),
        (false, n) => (format!("Synapse active: {} processes, {} MB", n, s.mem_mb), amber),
    }
}
