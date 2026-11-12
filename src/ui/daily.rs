//! Daily sales overview, expandable category breakdowns, and PDF export controls.
//!
//! The report includes only receipts completed during the current local calendar day.
//! Exports can cover a configurable date range (default: current month).
//! Topping prices remain part of sales totals, while category quantities count base items.

use crate::{
    app::{App, ExportPreview},
    domain::*,
    ui::widgets::*,
};
use chrono::{Datelike, Local, NaiveDate, TimeZone};
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
        ui.label(date.format("%d/%m/%Y").to_string());
        ui.add_space(12.0);
        ui.columns(2, |cols| {
            // Measure the export card's natural height, then match the overview to it.
            let export_card_height = card()
                .show(&mut cols[1], |ui| {
                    ui.label(egui::RichText::new("EXPORT PDF").size(18.0).strong());
                    ui.add_space(8.0);
                    ui.label("Période");
                    ui.horizontal_wrapped(|ui| {
                        date_input(ui, "export_from", &mut self.export_from, "Début", 130.0);
                        ui.label(
                            egui::RichText::new("au").color(egui::Color32::from_rgb(102, 117, 103)),
                        );
                        date_input(ui, "export_to", &mut self.export_to, "Fin", 130.0);
                        if ui
                            .add(egui::Button::new("Aujourd’hui").min_size(egui::vec2(0.0, 40.0)))
                            .pressed(ui)
                        {
                            let d = Local::now().date_naive().format("%Y-%m-%d").to_string();
                            self.export_from = d.clone();
                            self.export_to = d;
                        }
                        if ui
                            .add(egui::Button::new("Mois en cours").min_size(egui::vec2(0.0, 40.0)))
                            .pressed(ui)
                        {
                            let today = Local::now().date_naive();
                            self.export_from = today
                                .with_day(1)
                                .unwrap_or(today)
                                .format("%Y-%m-%d")
                                .to_string();
                            self.export_to = today.format("%Y-%m-%d").to_string();
                        }
                    });
                    let export_orders =
                        NaiveDate::parse_from_str(self.export_from.trim(), "%Y-%m-%d")
                            .ok()
                            .zip(NaiveDate::parse_from_str(self.export_to.trim(), "%Y-%m-%d").ok())
                            .filter(|(from, to)| from <= to)
                            .map(|(from, to)| self.orders_in_range(from, to))
                            .unwrap_or_default();
                    ui.add_space(8.0);
                    if ui
                        .add_enabled(
                            !export_orders.is_empty(),
                            primary_button("Générer bilan", [ui.available_width(), 58.0]),
                        )
                        .pressed(ui)
                    {
                        self.generate_export_preview();
                    }
                    if self.export_directory.trim().is_empty() {
                        ui.small("Dossier d’export non configuré (Configuration).");
                    } else if export_orders.is_empty() {
                        ui.small("Aucune commande dans la période sélectionnée.");
                    } else {
                        ui.small(format!(
                            "{} commande(s) dans la période sélectionnée.",
                            export_orders.len()
                        ));
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
                    let category_index = self
                        .categories
                        .iter()
                        .position(|name| name.eq_ignore_ascii_case(category));
                    let color = category_index
                        .and_then(|i| self.colors.get(i).copied())
                        .unwrap_or_else(default_theme);
                    let has_toppings = category_index.is_some_and(|index| {
                        self.products
                            .iter()
                            .any(|product| product.topping && product.category == index)
                    });
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
                    if has_toppings {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(
                                    "Les garnitures sont incluses dans les ventes, pas dans les quantités par catégorie.",
                                )
                                .size(15.0)
                                .color(egui::Color32::from_rgb(102, 117, 103)),
                            );
                        });
                    }
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

    /// Shows the generated PDF report with an editable file name and a save action.
    ///
    /// The preview is drawn from the same summary used to build the PDF, so the document is only
    /// written to disk once the user presses **Sauvegarder**.
    pub(crate) fn export_preview_window(&mut self, ctx: &egui::Context) {
        let Some(mut preview) = self.export_preview.take() else {
            return;
        };
        let mut open = true;
        let mut close_requested = false;
        let mut save = false;
        egui::Window::new("Aperçu du bilan")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(880.0)
            .default_height(740.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Nom du fichier").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut preview.file_name)
                            .desired_width(360.0)
                            .hint_text("bilan.pdf"),
                    );
                    if ui
                        .add(primary_button("Sauvegarder", [160.0, 44.0]))
                        .pressed(ui)
                    {
                        save = true;
                    }
                    if ui.button("Fermer").pressed(ui) {
                        close_requested = true;
                    }
                });
                if self.export_directory.trim().is_empty() {
                    ui.small("Dossier d’export non configuré (Configuration).");
                }
                if let Some(message) = &self.message {
                    ui.label(message);
                }
                ui.separator();
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        preview_page(ui, &preview, &self.categories, &self.colors);
                    });
            });
        if close_requested {
            open = false;
        }
        if save && self.save_export_preview(&preview) {
            open = false;
        }
        if open {
            self.export_preview = Some(preview);
        }
    }
}

/// Draws the report content that the PDF contains.
fn preview_page(
    ui: &mut egui::Ui,
    preview: &ExportPreview,
    categories: &[String],
    colors: &[[u8; 3]],
) {
    egui::Frame::new()
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(225, 228, 221),
        ))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(28))
        .show(ui, |ui| {
            ui.set_min_width(720.0);
            ui.label(
                egui::RichText::new("ECR — Résumé des ventes")
                    .size(24.0)
                    .strong(),
            );
            let period = if preview.from == preview.to {
                format!("Période : {}", preview.from.format("%d/%m/%Y"))
            } else {
                format!(
                    "Période : {} au {}",
                    preview.from.format("%d/%m/%Y"),
                    preview.to.format("%d/%m/%Y")
                )
            };
            ui.label(period);
            ui.label(
                egui::RichText::new(format!(
                    "Commandes : {}    Total des ventes : {}",
                    preview.order_count,
                    money(preview.total)
                ))
                .strong(),
            );
            ui.add_space(14.0);

            for category in &preview.ordered_categories {
                let Some(products) = preview.summary.get(category) else {
                    continue;
                };
                let color = categories
                    .iter()
                    .position(|known| known.eq_ignore_ascii_case(category.trim()))
                    .and_then(|index| colors.get(index).copied())
                    .unwrap_or([80, 80, 80]);
                let category_total: u64 = products.values().map(|(_, sum)| *sum).sum();
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(color[0], color[1], color[2]))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}  (Total: {})",
                                category,
                                money(category_total)
                            ))
                            .size(18.0)
                            .strong()
                            .color(egui::Color32::from_rgb(35, 73, 53)),
                        );
                    });
                egui::Grid::new(("apercu_produits", category))
                    .num_columns(3)
                    .spacing([24.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Produit").strong());
                        ui.label(egui::RichText::new("Qté").strong());
                        ui.label(egui::RichText::new("Somme").strong());
                        ui.end_row();
                        for (name, (quantity, subtotal)) in products {
                            ui.label(name);
                            ui.label(quantity.to_string());
                            ui.label(money(*subtotal));
                            ui.end_row();
                        }
                    });
                ui.add_space(14.0);
            }
        });
}
