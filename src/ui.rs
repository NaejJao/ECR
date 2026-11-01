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

impl App {
    /// Prompts for the menu password and opens menu management only after verification.
    fn menu_password_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_menu_password;
        let mut cancel_requested = false;
        egui::Window::new("Accès à la gestion du menu")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Saisissez le mot de passe pour gérer le menu.");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.menu_password)
                        .password(true)
                        .hint_text("Mot de passe"),
                );
                if self.menu_password_needs_focus {
                    response.request_focus();
                    self.menu_password_needs_focus = false;
                }
                let submit = ui.button("Valider").pressed(ui)
                    || (response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)));
                if submit {
                    use sha2::{Digest, Sha256};
                    let digest =
                        Sha256::digest(format!("ecr-menu-access-v1:{}", self.menu_password));
                    let expected = [
                        0xda, 0xa8, 0x60, 0x18, 0x99, 0xbc, 0x95, 0x21, 0x96, 0x79, 0x7a, 0x6e,
                        0xba, 0x89, 0xc4, 0xc9, 0xb2, 0x11, 0x4c, 0x46, 0x8a, 0x59, 0x74, 0xb6,
                        0xb0, 0x70, 0x47, 0xed, 0x07, 0x5d, 0x0d, 0x76,
                    ];
                    if digest.as_slice() == expected {
                        self.screen = Screen::Menu;
                        self.show_menu_password = false;
                        self.menu_password.clear();
                    } else {
                        self.menu_password.clear();
                        self.message = Some("Mot de passe incorrect.".into());
                    }
                }
                if ui.button("Annuler").pressed(ui) {
                    cancel_requested = true;
                }
            });
        if cancel_requested {
            self.show_menu_password = false;
            self.menu_password.clear();
        }
        self.show_menu_password = open && self.show_menu_password;
    }
}

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
                        let is_order_screen = self.screen == Screen::Order;
                        let header_width = if is_order_screen { 190.0 } else { 150.0 };
                        let response =
                            ui.add(header_button("").min_size([header_width, 44.0].into()));
                        let rect = response.rect;
                        let icon_center = egui::pos2(rect.left() + 24.0, rect.center().y);
                        let stroke = egui::Stroke::new(2.5_f32, egui::Color32::WHITE);
                        if is_order_screen {
                            let lock_body = egui::Rect::from_center_size(
                                icon_center + egui::vec2(0.0, 3.0),
                                egui::vec2(12.0, 10.0),
                            );
                            ui.painter().line_segment(
                                [
                                    icon_center + egui::vec2(-4.0, -2.0),
                                    icon_center + egui::vec2(-4.0, -6.0),
                                ],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [
                                    icon_center + egui::vec2(-4.0, -6.0),
                                    icon_center + egui::vec2(4.0, -6.0),
                                ],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [
                                    icon_center + egui::vec2(4.0, -6.0),
                                    icon_center + egui::vec2(4.0, -2.0),
                                ],
                                stroke,
                            );
                            ui.painter().rect_stroke(
                                lock_body,
                                egui::CornerRadius::same(2),
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().text(
                                egui::pos2(
                                    (rect.left() + 44.0 + rect.right() - 8.0) * 0.5,
                                    rect.center().y,
                                ),
                                egui::Align2::CENTER_CENTER,
                                "Configuration",
                                egui::FontId::proportional(18.0),
                                egui::Color32::WHITE,
                            );
                        } else {
                            ui.painter().circle_stroke(
                                icon_center,
                                10.0,
                                egui::Stroke::new(1.5_f32, egui::Color32::WHITE),
                            );
                            let tip = icon_center + egui::vec2(-5.0, 0.0);
                            ui.painter()
                                .line_segment([icon_center + egui::vec2(5.0, 0.0), tip], stroke);
                            ui.painter()
                                .line_segment([tip + egui::vec2(5.0, -5.0), tip], stroke);
                            ui.painter()
                                .line_segment([tip + egui::vec2(5.0, 5.0), tip], stroke);
                            ui.painter().text(
                                egui::pos2(
                                    (rect.left() + 44.0 + rect.right() - 8.0) * 0.5,
                                    rect.center().y,
                                ),
                                egui::Align2::CENTER_CENTER,
                                "Retour",
                                egui::FontId::proportional(18.0),
                                egui::Color32::WHITE,
                            );
                        }
                        if response.pressed(ui) {
                            if is_order_screen {
                                self.show_menu_password = true;
                                self.menu_password_needs_focus = true;
                                self.menu_password.clear();
                                self.message = None;
                            } else {
                                self.screen = Screen::Order;
                                self.message = None;
                            }
                        }

                        if self.screen == Screen::Order
                            && ui.add(header_button("Bilan")).pressed(ui)
                        {
                            self.screen = Screen::Daily;
                            self.message = None;
                        }
                        if self.screen == Screen::Order {
                            let response = ui
                                .add(
                                    egui::Button::new("")
                                        .fill(egui::Color32::from_rgb(176, 54, 48))
                                        .stroke(egui::Stroke::NONE)
                                        .corner_radius(egui::CornerRadius::same(10))
                                        .min_size([48.0, 44.0].into()),
                                )
                                .on_hover_text("Quitter");
                            let rect = response.rect;
                            let painter = ui.painter();
                            let stroke = egui::Stroke::new(2.0_f32, egui::Color32::WHITE);
                            let left = rect.center().x - 8.0;
                            let top = rect.center().y - 10.0;
                            let bottom = rect.center().y + 10.0;
                            painter.line_segment(
                                [egui::pos2(left, top), egui::pos2(left + 12.0, top)],
                                stroke,
                            );
                            painter.line_segment(
                                [egui::pos2(left, top), egui::pos2(left, bottom)],
                                stroke,
                            );
                            painter.line_segment(
                                [egui::pos2(left, bottom), egui::pos2(left + 12.0, bottom)],
                                stroke,
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(left + 4.0, rect.center().y),
                                    egui::pos2(left + 19.0, rect.center().y),
                                ],
                                stroke,
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(left + 14.0, rect.center().y - 5.0),
                                    egui::pos2(left + 19.0, rect.center().y),
                                ],
                                stroke,
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(left + 14.0, rect.center().y + 5.0),
                                    egui::pos2(left + 19.0, rect.center().y),
                                ],
                                stroke,
                            );
                            if response.pressed(ui) {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
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
        if self.show_menu_password {
            self.menu_password_window(ctx);
        }
    }
}
