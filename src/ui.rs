//! Root UI module: installs the application-wide style and routes to screen modules.

mod daily;
mod menu;
mod order;
pub(crate) mod widgets;

use crate::{
    app::{App, Screen},
    ui::widgets::*,
};
use eframe::egui;

impl eframe::App for App {
    /// Draws the shared header, applies global styling, and displays the active screen.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut v = egui::Visuals::light();
        v.window_fill = egui::Color32::from_rgb(247, 247, 243);
        v.panel_fill = v.window_fill;
        v.widgets.inactive.bg_fill = egui::Color32::WHITE;
        v.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(225, 228, 221));
        v.widgets.inactive.corner_radius = egui::CornerRadius::same(12);
        v.widgets.hovered.bg_fill = egui::Color32::from_rgb(235, 243, 235);
        v.widgets.hovered.corner_radius = egui::CornerRadius::same(12);
        v.widgets.active.bg_fill = egui::Color32::from_rgb(220, 235, 222);
        v.selection.bg_fill = egui::Color32::from_rgb(49, 104, 70);
        ctx.set_visuals(v);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(12.0, 10.0);
            s.spacing.button_padding = egui::vec2(14.0, 10.0);
            s.text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(18.0));
        });

        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(35, 73, 53))
                    .inner_margin(egui::Margin::symmetric(22, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("ECR")
                            .strong()
                            .color(egui::Color32::from_rgb(192, 220, 195)),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new("WaffleTime")
                            .size(25.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(header_button(if self.screen == Screen::Order {
                                "⚙  Gérer le menu"
                            } else {
                                "←  Retour à la commande"
                            }))
                            .clicked()
                        {
                            self.screen = if self.screen == Screen::Order {
                                Screen::Menu
                            } else {
                                Screen::Order
                            };
                            self.message = None;
                        }

                        if self.screen == Screen::Order
                            && ui.add(header_button("Bilan du jour")).clicked()
                        {
                            self.screen = Screen::Daily;
                            self.message = None;
                        }
                        if ui.add(danger_button("⏻  Quitter")).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });
        match self.screen {
            Screen::Menu => {
                egui::CentralPanel::default()
                    .frame(page_frame())
                    .show(ctx, |ui| self.menu_ui(ui));
            }
            Screen::Daily => {
                egui::CentralPanel::default()
                    .frame(page_frame())
                    .show(ctx, |ui| self.daily_ui(ui));
            }
            Screen::Order => self.order_ui(ctx),
        }
        if self.show_previous_order {
            self.previous_order_window(ctx);
        }
    }
}
