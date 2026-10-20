// The shop: the CR-Token balance (the CR logo is the coin) and what the tokens buy.
use crate::config::Config;
use crate::shop::{self, BuyError, Item, Kind};
use eframe::egui::{self, Color32, RichText};

/// The coin: the CR logo, baked small.
pub const COIN_PNG: &[u8] = include_bytes!("../../assets/token.png");

/// What happened in the shop this frame.
#[derive(Debug, Default)]
pub struct ShopResult {
    /// Something was bought, put on, or taken off (the caller saves).
    pub changed: bool,
    /// The item that was just bought.
    pub bought: Option<&'static str>,
}

/// Loads the coin texture.
pub fn load_coin(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(COIN_PNG).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ctx.load_texture("cpunity_coin", egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR))
}

/// The coin, `size` points across.
pub fn coin(ui: &mut egui::Ui, tex: Option<&egui::TextureHandle>, size: f32) {
    match tex {
        Some(t) => {
            ui.add(egui::Image::new(egui::load::SizedTexture::new(t.id(), egui::vec2(size, size))));
        }
        None => {
            ui.label("CR");
        }
    }
}

/// The balance, as the coin and the number. Clicking it is how the footer opens the shop.
pub fn balance_chip(ui: &mut egui::Ui, tex: Option<&egui::TextureHandle>, balance: u32) -> egui::Response {
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        coin(ui, tex, 18.0);
        ui.add(egui::Label::new(RichText::new(balance.to_string()).strong().color(crate::theme::text())).selectable(false));
    });
    ui.interact(inner.response.rect, ui.id().with("coin_chip"), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("CR-Tokens. Click to open the shop (Options).")
}

/// A price: the coin and the number.
fn price(ui: &mut egui::Ui, tex: Option<&egui::TextureHandle>, n: u32) {
    ui.spacing_mut().item_spacing.x = 3.0;
    coin(ui, tex, 14.0);
    ui.add(egui::Label::new(RichText::new(n.to_string()).color(crate::theme::text_dim())).selectable(false));
}

fn small(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).small().color(Color32::from_gray(140))).wrap().selectable(false));
}

/// Options > Shop, at the top: not there until the first token has been earned.
pub fn render_shop_section(ui: &mut egui::Ui, cfg: &mut Config, tex: Option<&egui::TextureHandle>) -> ShopResult {
    let mut res = ShopResult::default();
    if !cfg.shop.visible() {
        return res;
    }
    crate::theme::collapsing(ui, "options_shop", "Shop", true, |ui| {
        ui.horizontal(|ui| {
            coin(ui, tex, 22.0);
            ui.add(egui::Label::new(RichText::new(format!("{} CR-Tokens", cfg.shop.balance())).strong().size(17.0)).selectable(false));
        });
        small(ui, "Earned in the hidden games: the better you do, and the harder the setting, the more they pay. Cheats pay nothing.");

        if let Some(id) = cfg.shop_preview.clone() {
            let name = shop::item(&id).map(|i| i.name).unwrap_or("it");
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Trying on {name}")).color(crate::theme::accent_stroke()));
                if ui.button("Stop").clicked() {
                    cfg.shop_preview = None;
                    res.changed = true;
                }
            });
        }

        // What is bought leaves the shop (it is put on from UI Options). It comes back by itself if it
        // is ever not owned any more, since the list is worked out from what is owned.
        let mut for_sale = 0;
        for (kind, heading) in [(Kind::Skin, "Skins"), (Kind::Backdrop, "Backdrops")] {
            let items = cfg.shop.for_sale(kind);
            if items.is_empty() {
                continue;
            }
            for_sale += items.len();
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new(heading).strong()).selectable(false));
            for item in items {
                item_row(ui, cfg, item, tex, &mut res);
            }
        }
        if for_sale == 0 {
            ui.add_space(6.0);
            small(ui, "You own everything the shop sells. Put your skins and backdrops on from Options > UI Options.");
        } else if cfg.shop.owned_count() > 0 {
            ui.add_space(6.0);
            small(ui, "What you have bought is not listed here any more: put it on from Options > UI Options.");
        }
    });
    res
}

/// One thing for sale: its name and price, what it is, and the buy and try-on buttons.
fn item_row(ui: &mut egui::Ui, cfg: &mut Config, item: &'static Item, tex: Option<&egui::TextureHandle>, res: &mut ShopResult) {
    let trying = cfg.shop_preview.as_deref() == Some(item.id);
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(item.name).strong());
        price(ui, tex, item.price);
    });
    small(ui, item.blurb);
    ui.horizontal_wrapped(|ui| {
        let balance = cfg.shop.balance();
        let can = balance >= item.price;
        let buy = ui.add_enabled(can, egui::Button::new("Buy"));
        let buy = if can { buy } else { buy.on_disabled_hover_text(format!("{} more CR-Tokens needed.", item.price - balance)) };
        if buy.clicked() {
            match cfg.shop.buy(item.id) {
                Ok(()) => {
                    res.changed = true;
                    res.bought = Some(item.id);
                    if cfg.shop_preview.as_deref() == Some(item.id) {
                        cfg.shop_preview = None;
                    }
                }
                Err(BuyError::TooExpensive { .. } | BuyError::AlreadyOwned | BuyError::Unknown) => {}
            }
        }
        if !trying && ui.button("Try on").on_hover_text("Free. Nothing is kept: it goes when you press Stop or restart.").clicked() {
            cfg.shop_preview = Some(item.id.to_string());
            res.changed = true;
        }
    });
}

/// The popup when the first CR-Token has been earned: it says what they are and where the shop is.
pub fn render_welcome(ctx: &egui::Context, open: &mut bool, tex: Option<&egui::TextureHandle>, open_shop: &mut bool) {
    if !*open {
        return;
    }
    let mut window_open = true;
    let mut ok = false;
    egui::Window::new("CR-Tokens")
        .open(&mut window_open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_max_width(360.0);
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                coin(ui, tex, 56.0);
                ui.add_space(4.0);
                ui.label(RichText::new("You earned CR-Tokens").strong().size(18.0));
            });
            ui.add_space(6.0);
            ui.add(egui::Label::new("The hidden games pay them out, and the shop at the top of Options sells new looks and backdrops for them. Trying one on is always free.").wrap());
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Open the shop").clicked() {
                    *open_shop = true;
                    ok = true;
                }
                if ui.button("  OK  ").clicked() {
                    ok = true;
                }
            });
        });
    if ok || !window_open {
        *open = false;
    }
}
