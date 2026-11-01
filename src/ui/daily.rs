//! Daily sales overview, expandable category breakdowns, and USB export controls.
//!
//! The report includes only receipts completed during the current local calendar day.
//! Topping prices remain part of sales totals, while category quantities count base items.

use crate::{app::App, domain::*, ui::widgets::*};
use chrono::{Local, TimeZone};
use eframe::egui;

impl App {
    /// Renders today's sales summary, category breakdowns, and receipt export controls.
    ///
    /// Typography is enlarged only within this screen. Summary cards share a row and
    /// the overview card follows the export card's measured natural height.
    pub(crate) fn daily_ui(&mut self, ui: &mut egui::Ui) {
        {
            // Keep the larger report typography local so other screens retain their styling.
            let style = ui.style_mut();
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(18.0));
            style
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(15.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(20.0));
        }
        let date = Local::now().date_naive();
        // Receipt timestamps are converted to local time before comparing calendar dates.
        let orders: Vec<_> = self
            .receipts
            .iter()
            .filter(|r| {
                Local
                    .timestamp_opt(r.completed_at, 0)
                    .single()
                    .is_some_and(|d| d.date_naive() == date)
            })
            .cloned()
            .collect();
        let total = orders.iter().fold(0_u64, |s, o| s.saturating_add(o.total));
        let counts = main_item_counts(&orders, &self.categories);
        let ordered_counts = ordered_category_counts(&counts, &self.categories);
        ui.label(
            egui::RichText::new("VENTES")
                .size(15.0)
                .strong()
                .color(egui::Color32::from_rgb(102, 117, 103)),
        );
        ui.heading(egui::RichText::new("Bilan").size(36.0).strong());
        ui.label(egui::RichText::new("Résumé des commandes terminées aujourd’hui.").size(18.0));
        ui.horizontal(|ui| {
            ui.label(date.format("%d/%m/%Y").to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.small("Les garnitures sont incluses dans les ventes, pas dans les quantités par catégorie.");
            });
        });
        ui.add_space(12.0);
        ui.columns(2, |cols| {
            // Measure the export card's natural height, then match the overview to it.
            let export_card_height = card()
                .show(&mut cols[1], |ui| {
                    ui.label(
                        egui::RichText::new("EXPORT DE FIN DE JOURNÉE")
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label("Enregistrer le détail des commandes sur USB");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.export_directory)
                            .hint_text("Dossier USB, ex. /media/vous/USB")
                            .desired_width(ui.available_width()),
                    );
                    ui.add_space(8.0);
                    if ui
                        .add_enabled(
                            !orders.is_empty(),
                            primary_button("Exporter les commandes", [ui.available_width(), 58.0]),
                        )
                        .pressed(ui)
                    {
                        self.export(date, &orders);
                    }
                    if orders.is_empty() {
                        ui.small("Aucune commande terminée aujourd’hui.");
                    }
                })
                .response
                .rect
                .height();
            card().show(&mut cols[0], |ui| {
                ui.set_min_height((export_card_height - 24.0).max(0.0));
                ui.label(egui::RichText::new("APERÇU DU JOUR").size(18.0).strong());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    overview_card(ui, "Commandes", orders.len().to_string());
                    overview_card(ui, "Total des ventes", money(total));
                });
            });
        });
        ui.add_space(14.0);
        ui.label(
            egui::RichText::new("ARTICLES PRINCIPAUX PAR CATÉGORIE")
                .size(18.0)
                .strong(),
        );
        egui::ScrollArea::vertical()
            .max_height(ui.available_height().max(240.0))
            .show(ui, |ui| {
                for (category, count) in &ordered_counts {
                    let color = self
                        .categories
                        .iter()
                        .position(|name| name.eq_ignore_ascii_case(category))
                        .and_then(|i| self.colors.get(i).copied())
                        .unwrap_or_else(default_theme);
                    let expanded = self.expanded_categories.contains(category);
                    let mut clicked = false;
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(color[0], color[1], color[2]))
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(egui::Margin::symmetric(12, 5))
                        .show(ui, |ui| {
                            let width = ui.available_width();
                            let response = ui.add(
                                egui::Button::new("")
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE)
                                    .min_size([width, 56.0].into()),
                            );
                            ui.painter().text(
                                egui::pos2(response.rect.left() + 40.0, response.rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                category,
                                egui::FontId::proportional(22.0),
                                ui.visuals().text_color(),
                            );
                            let arrow_center =
                                egui::pos2(response.rect.left() + 18.0, response.rect.center().y);
                            let arrow_stroke =
                                egui::Stroke::new(2.5_f32, ui.visuals().text_color());
                            if expanded {
                                ui.painter().line_segment(
                                    [
                                        arrow_center + egui::vec2(-7.0, -3.0),
                                        arrow_center + egui::vec2(0.0, 4.0),
                                    ],
                                    arrow_stroke,
                                );
                                ui.painter().line_segment(
                                    [
                                        arrow_center + egui::vec2(0.0, 4.0),
                                        arrow_center + egui::vec2(7.0, -3.0),
                                    ],
                                    arrow_stroke,
                                );
                            } else {
                                ui.painter().line_segment(
                                    [
                                        arrow_center + egui::vec2(-3.0, -7.0),
                                        arrow_center + egui::vec2(4.0, 0.0),
                                    ],
                                    arrow_stroke,
                                );
                                ui.painter().line_segment(
                                    [
                                        arrow_center + egui::vec2(4.0, 0.0),
                                        arrow_center + egui::vec2(-3.0, 7.0),
                                    ],
                                    arrow_stroke,
                                );
                            }
                            let count_rect = egui::Rect::from_min_size(
                                egui::pos2(response.rect.right() - 70.0, response.rect.top()),
                                egui::vec2(64.0, response.rect.height()),
                            );
                            ui.painter().text(
                                count_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                count.to_string(),
                                egui::FontId::proportional(24.0),
                                ui.visuals().text_color(),
                            );
                            clicked = response.pressed(ui);
                        });
                    if clicked {
                        if expanded {
                            self.expanded_categories.remove(category);
                        } else {
                            self.expanded_categories.insert(category.clone());
                        }
                    }
                    if self.expanded_categories.contains(category) {
                        let details =
                            main_products_in_category(&orders, category, &self.categories);
                        card().show(ui, |ui| {
                            if details.is_empty() {
                                ui.label("Aucun article principal vendu dans cette catégorie.");
                            } else {
                                egui::Grid::new(("bilan_categorie", category))
                                    .num_columns(3)
                                    .spacing([20.0, 8.0])
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Produit").strong());
                                        ui.label(egui::RichText::new("Quantité").strong());
                                        ui.label(egui::RichText::new("Sous-total").strong());
                                        ui.end_row();
                                        for (name, (quantity, subtotal)) in &details {
                                            ui.label(name);
                                            ui.label(quantity.to_string());
                                            ui.label(money(*subtotal));
                                            ui.end_row();
                                        }
                                    });
                            }
                        });
                    }
                    ui.add_space(6.0);
                }
            });
        if let Some(message) = &self.message {
            ui.label(message);
        }
    }
}
