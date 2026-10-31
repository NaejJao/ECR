use crate::{app::App, domain::*, ui::widgets::*};
use chrono::{Local, TimeZone};
use eframe::egui;

impl App {
    pub(crate) fn order_ui(&mut self, ctx: &egui::Context) {
        let screen_width = ctx.screen_rect().width();
        let order_width = (screen_width * 0.34).clamp(400.0, 560.0);
        let order_min_width = (screen_width * 0.28).clamp(360.0, 480.0);
        egui::SidePanel::right("order")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(20)),
            )
            .resizable(true)
            .default_width(order_width)
            .min_width(order_min_width)
            .show(ctx, |ui| self.order_panel(ui));
        egui::CentralPanel::default()
            .frame(page_frame())
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    for i in 0..self.categories.len() {
                        let color = egui::Color32::from_rgb(
                            self.colors[i][0],
                            self.colors[i][1],
                            self.colors[i][2],
                        );
                        let fill = if self.selected == i {
                            color
                        } else {
                            egui::Color32::WHITE
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(&self.categories[i]).strong(),
                                )
                                .fill(fill)
                                .min_size([160.0, 58.0].into()),
                            )
                            .clicked()
                        {
                            self.selected = i;
                        }
                    }
                });
                ui.add_space(18.0);
                if let Some(cat) = self.categories.get(self.selected) {
                    ui.heading(egui::RichText::new(cat).size(26.0).strong());
                }
                let items: Vec<_> = self
                    .products
                    .iter()
                    .filter(|p| p.category == self.selected)
                    .map(|p| (p.name.clone(), p.price, p.topping, p.category))
                    .collect();
                let available_width = ui.available_width().max(240.0);
                let cols = (available_width / 220.0).round() as usize;
                let cols = cols.clamp(1, 5);
                let button_width = ((available_width - 12.0 * cols.saturating_sub(1) as f32)
                    / cols as f32)
                    .clamp(190.0, 240.0);
                if items.is_empty() {
                    ui.label(
                        "Aucun produit dans cette catégorie. Ajoutez-en dans « Gérer le menu ».",
                    );
                } else {
                    egui::Grid::new("product_buttons")
                        .num_columns(cols)
                        .spacing([12.0, 12.0])
                        .show(ui, |ui| {
                            for (n, (name, price, topping, category)) in items.iter().enumerate() {
                                let theme = self.colors[*category];
                                let fill = if *topping {
                                    lighten(theme)
                                } else {
                                    egui::Color32::from_rgb(theme[0], theme[1], theme[2])
                                };
                                let mut label = egui::text::LayoutJob::default();
                                let mut name_style = egui::TextFormat::default();
                                name_style.font_id = egui::FontId::proportional(24.0);
                                name_style.color = egui::Color32::from_rgb(35, 73, 53);
                                label.append(name, 0.0, name_style);
                                label.append("\n", 0.0, egui::TextFormat::default());
                                let mut price_style = egui::TextFormat::default();
                                price_style.font_id = egui::FontId::proportional(15.0);
                                price_style.color = egui::Color32::from_rgb(70, 82, 72);
                                label.append(&money(*price), 0.0, price_style);
                                let button = egui::Button::new(label)
                                    .fill(fill)
                                    .min_size([button_width, 124.0].into());
                                if ui.add(button).clicked() {
                                    self.add_product(name, *price, *topping, *category);
                                }
                                if (n + 1) % cols == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                }
            });
    }

    pub(crate) fn order_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Commande");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.receipts.is_empty(), header_button("Précédente"))
                    .clicked()
                {
                    self.show_previous_order = true;
                }
            });
        });
        ui.separator();
        if let Some(message) = &self.message {
            ui.label(message);
        }
        let total_height =
            ui.fonts(|fonts| fonts.row_height(&egui::FontId::proportional(36.0))) + 20.0;
        let action_height = 58.0_f32.max(ui.spacing().interact_size.y);
        let footer_height =
            total_height + action_height + 8.0 + ui.spacing().item_spacing.y * 3.0 + 1.0;
        let list_height = (ui.available_height() - footer_height).max(80.0);
        let mut remove = None;
        egui::ScrollArea::vertical()
            .max_height(list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let numbered: Vec<_> = self
                    .order
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let topping_category = self
                            .products
                            .iter()
                            .any(|p| p.topping && p.category == item.category_index);
                        let ordinal = self.order[..=i]
                            .iter()
                            .filter(|x| x.category_index == item.category_index)
                            .count();
                        (topping_category, ordinal)
                    })
                    .collect();
                let mut display_indices: Vec<usize> = (0..self.order.len()).collect();
                display_indices.sort_by_key(|&index| {
                    let category = self.order[index].category_index;
                    if self
                        .products
                        .iter()
                        .any(|product| product.category == category && product.topping)
                    {
                        0
                    } else {
                        1
                    }
                });
                for i in display_indices {
                    let item = &mut self.order[i];
                    let tint = egui::Color32::from_rgb(item.theme[0], item.theme[1], item.theme[2]);
                    egui::Frame::new()
                        .fill(tint)
                        .corner_radius(egui::CornerRadius::same(9))
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let item_name = if item.quantity > 1 {
                                    format!("{} × {}", item.quantity, item.name)
                                } else {
                                    item.name.clone()
                                };
                                let title = if numbered[i].0 {
                                    format!("{} {}", item_name, numbered[i].1)
                                } else {
                                    item_name
                                };
                                ui.label(egui::RichText::new(title).size(15.0).strong());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_sized([30.0, 28.0], egui::Button::new("×"))
                                            .clicked()
                                        {
                                            remove = Some((i, None));
                                        }
                                        ui.label(
                                            egui::RichText::new(money(
                                                item.price.saturating_mul(u64::from(item.quantity)),
                                            ))
                                            .size(14.0)
                                            .strong(),
                                        );
                                    },
                                );
                            });
                            ui.indent(("toppings", i), |ui| {
                                for (j, topping) in item.toppings.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        let name = if topping.quantity > 1 {
                                            format!("↳ {} × {}", topping.quantity, topping.name)
                                        } else {
                                            format!("↳ {}", topping.name)
                                        };
                                        ui.label(egui::RichText::new(name).size(14.0));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .add_sized([32.0, 30.0], egui::Button::new("×"))
                                                    .clicked()
                                                {
                                                    remove = Some((i, Some(j)));
                                                }
                                                ui.label(
                                                    egui::RichText::new(money(
                                                        topping.price.saturating_mul(u64::from(
                                                            topping.quantity,
                                                        )),
                                                    ))
                                                    .size(13.0),
                                                );
                                            },
                                        );
                                    });
                                }
                            });
                        });
                    ui.add_space(3.0);
                }
            });
        if let Some((i, child)) = remove {
            if let Some(j) = child {
                self.order[i].toppings.remove(j);
            } else {
                self.order.remove(i);
            }
        }
        ui.separator();
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(235, 243, 235))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("À PAYER").size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(money(self.total())).size(36.0).strong());
                    });
                });
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let button_width =
                ((ui.available_width() - ui.spacing().item_spacing.x) * 0.62).max(160.0);
            if ui
                .add_enabled(
                    !self.order.is_empty(),
                    primary_button("Valider la commande", [button_width, 58.0]),
                )
                .clicked()
            {
                self.complete();
            }
            if ui
                .add_sized([ui.available_width(), 58.0], egui::Button::new("Effacer"))
                .clicked()
            {
                self.order.clear();
            }
        });
    }

    pub(crate) fn previous_order_window(&mut self, ctx: &egui::Context) {
        let Some(receipt) = self.receipts.last().cloned() else {
            self.show_previous_order = false;
            return;
        };
        egui::Window::new("Dernière commande validée")
            .open(&mut self.show_previous_order)
            .resizable(true)
            .collapsible(false)
            .default_size([620.0, 560.0])
            .show(ctx, |ui| {
                if let Some(completed_at) = Local.timestamp_opt(receipt.completed_at, 0).single() {
                    ui.label(completed_at.format("%d/%m/%Y à %H:%M").to_string());
                }
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 80.0).max(160.0))
                    .show(ui, |ui| {
                        for item in &receipt.items {
                            let color = egui::Color32::from_rgb(
                                item.theme[0],
                                item.theme[1],
                                item.theme[2],
                            );
                            egui::Frame::new()
                                .fill(color)
                                .corner_radius(egui::CornerRadius::same(10))
                                .inner_margin(egui::Margin::symmetric(12, 8))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let name = if item.quantity > 1 {
                                            format!("{} × {}", item.quantity, item.name)
                                        } else {
                                            item.name.clone()
                                        };
                                        ui.label(egui::RichText::new(name).strong());
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(money(
                                                    item.price
                                                        .saturating_mul(u64::from(item.quantity)),
                                                ));
                                            },
                                        );
                                    });
                                    for topping in &item.toppings {
                                        let name = if topping.quantity > 1 {
                                            format!("↳ {} × {}", topping.quantity, topping.name)
                                        } else {
                                            format!("↳ {}", topping.name)
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(name);
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(money(topping.price.saturating_mul(
                                                        u64::from(topping.quantity),
                                                    )));
                                                },
                                            );
                                        });
                                    }
                                });
                            ui.add_space(6.0);
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Total payé").size(20.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(money(receipt.total))
                                .size(28.0)
                                .strong(),
                        );
                    });
                });
            });
    }
}
