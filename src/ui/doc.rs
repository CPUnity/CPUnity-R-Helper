// Draws the text blocks of the Changelog and Info tabs.
use crate::doc::{parts, sections, spans, Block, Part};
use crate::theme;
use eframe::egui::{self, text::LayoutJob, RichText, TextFormat};

/// One line of text with `**bold**` parts.
fn job(ui: &egui::Ui, text: &str) -> LayoutJob {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let normal = ui.visuals().text_color();
    let strong = ui.visuals().strong_text_color();
    let mut job = LayoutJob::default();
    for (piece, bold) in spans(text) {
        job.append(
            &piece,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: if bold { strong } else { normal },
                ..Default::default()
            },
        );
    }
    job
}

/// How much bigger than normal the text of this tab is (1.0 = normal size).
fn scale(ui: &egui::Ui) -> f32 {
    egui::TextStyle::Body.resolve(ui.style()).size / theme::BODY_SIZE
}

/// A heading in the accent colour at the tab's text size.
fn section_title(ui: &egui::Ui, text: &str) -> RichText {
    theme::section_title(text).size(15.5 * scale(ui))
}

fn block(ui: &mut egui::Ui, block: &Block, script: bool) {
    match block {
        Block::Title(text) if script => {
            super::script_text::show(ui, text, 22.0 * scale(ui), true);
            ui.add_space(6.0);
        }
        Block::Heading(text) if script => {
            ui.add_space(8.0);
            super::script_text::show(ui, text, 15.5 * scale(ui), true);
            ui.add_space(2.0);
        }
        Block::Subheading(text) if script => {
            ui.add_space(4.0);
            super::script_text::show(ui, text, 14.5 * scale(ui), true);
        }
        Block::Item { level, text } if script => {
            ui.horizontal_top(|ui| {
                ui.add_space(6.0 + 16.0 * f32::from(*level));
                ui.label(if *level == 0 { "\u{2022}" } else { "\u{2013}" });
                ui.vertical(|ui| super::script_text::show(ui, text, 14.5 * scale(ui), false));
            });
        }
        Block::Paragraph(text) if script => {
            super::script_text::show(ui, text, 14.5 * scale(ui), false);
            ui.add_space(6.0);
        }
        Block::Title(text) => {
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .strong()
                        .size(22.0 * scale(ui))
                        .color(theme::accent_stroke()),
                )
                .wrap()
                .selectable(false),
            );
            ui.add_space(6.0);
        }
        Block::Heading(text) => {
            ui.add_space(8.0);
            ui.add(egui::Label::new(section_title(ui, text)).wrap().selectable(false));
            ui.add_space(2.0);
        }
        Block::Subheading(text) => {
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(RichText::new(text).strong().color(theme::accent_stroke()))
                    .selectable(false),
            );
        }
        Block::Item { level, text } => {
            ui.horizontal_top(|ui| {
                ui.add_space(6.0 + 16.0 * f32::from(*level));
                ui.label(if *level == 0 { "\u{2022}" } else { "\u{2013}" });
                ui.add(egui::Label::new(job(ui, text)).wrap().selectable(false));
            });
        }
        Block::Paragraph(text) => {
            ui.add(egui::Label::new(job(ui, text)).wrap().selectable(false));
            ui.add_space(6.0);
        }
        Block::Copy { label, text } => copy_row(ui, label, text),
        Block::Link { label, url } => link_row(ui, label, url),
    }
}

const GREEN: egui::Color32 = egui::Color32::from_rgb(110, 200, 120);

/// How long "Copied" stays visible, in seconds.
const COPIED_SECONDS: f64 = 2.0;

/// `label: text` where clicking the text copies it to the clipboard.
fn copy_row(ui: &mut egui::Ui, label: &str, text: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new(format!("{label}:")).strong()).selectable(false));
        let id = ui.make_persistent_id(("copy", text));
        let clicked = ui
            .add(
                egui::Button::new(
                    RichText::new(text).monospace().underline().color(theme::accent_stroke()),
                )
                .frame(false),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Click to copy")
            .clicked();
        let now = ui.input(|i| i.time);
        if clicked {
            ui.ctx().copy_text(text.to_string());
            ui.data_mut(|d| d.insert_temp(id, now));
        }
        if let Some(at) = ui.data(|d| d.get_temp::<f64>(id)) {
            if now - at < COPIED_SECONDS {
                ui.label(RichText::new("Copied to clipboard").small().color(GREEN));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
    });
    ui.add_space(4.0);
}

/// A world button that opens `url` in the browser, with its name and address next to it.
fn link_row(ui: &mut egui::Ui, label: &str, url: &str) {
    ui.horizontal_top(|ui| {
        if ui
            .button("\u{1f310}")
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(format!("Open {url} in your browser"))
            .clicked()
        {
            crate::places::open_url(url);
        }
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(RichText::new(label).strong().color(theme::accent_stroke()))
                    .selectable(false),
            );
            ui.add(
                egui::Label::new(RichText::new(url).small().monospace())
                    .truncate()
                    .selectable(false),
            );
        });
    });
    ui.add_space(4.0);
}

/// Plain reading text: headings, bullets and paragraphs one after the other.
pub fn render_plain(ui: &mut egui::Ui, blocks: &[Block]) {
    render_plain_as(ui, blocks, false);
}

fn render_plain_as(ui: &mut egui::Ui, blocks: &[Block], script: bool) {
    for b in blocks {
        block(ui, b, script);
    }
}

/// Reading text where a `## [+] Title` heading is a closed drop-down (the Info tab).
pub fn render_with_dropdowns(ui: &mut egui::Ui, blocks: &[Block]) {
    render_with_dropdowns_as(ui, blocks, false);
}

/// The same, with the text in The Void's script when `script` is on (the drop-down titles stay
/// readable: they are how you find your way about, and the words inside decode when hovered).
pub fn render_with_dropdowns_as(ui: &mut egui::Ui, blocks: &[Block], script: bool) {
    for part in parts(blocks) {
        match part {
            Part::Block(b) => block(ui, b, script),
            Part::Dropdown { title, body } => {
                ui.add_space(6.0);
                let key = format!("info/{}", title);
                let heading = section_title(ui, title);
                theme::collapsing(ui, &key, heading, false, |ui| {
                    render_plain_as(ui, body, script);
                });
            }
        }
    }
}

/// A change log: one drop-down per `##` version, the newest open the first time.
pub fn render_versions(ui: &mut egui::Ui, blocks: &[Block]) {
    for (n, (title, body)) in sections(blocks).into_iter().enumerate() {
        let key = format!("changelog/{}", title);
        let heading = section_title(ui, title);
        theme::collapsing(ui, &key, heading, n == 0, |ui| {
            render_plain(ui, body);
        });
        ui.add_space(4.0);
    }
}
