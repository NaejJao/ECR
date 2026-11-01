//! Touch-first product selection, image-backed product buttons, and order history.
//!
//! Completed receipts are read-only; product images are fitted into a fixed one-third-width
//! button panel without changing the label and price area.

use crate::{app::App, domain::*, ui::widgets::*};
use chrono::{Local, TimeZone};
use eframe::egui;

impl App {
    /// Loads and caches a product image texture, remembering missing or invalid files too.
    fn product_texture(&mut self, ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
        if !self.product_images.contains_key(path) {
            let texture = image::open(path).ok().map(|decoded| {
                let rgba = decoded.into_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let corner = rgba.get_pixel(0, 0).0;
                self.product_image_corners.insert(
                    path.to_owned(),
                    egui::Color32::from_rgba_unmultiplied(
                        corner[0], corner[1], corner[2], corner[3],
                    ),
                );
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                ctx.load_texture(path, color_image, egui::TextureOptions::LINEAR)
            });
            self.product_images.insert(path.to_owned(), texture);
        }
        self.product_images.get(path).cloned().flatten()
    }

    /// Builds the responsive product grid and resizable current-order side panel.
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
                    .map(|p| {
                        (
                            p.name.clone(),
                            p.price,
                            p.topping,
                            p.category,
                            p.image_path.clone(),
                        )
                    })
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
                    ui.horizontal_wrapped(|ui| {
                        for (name, price, topping, category, image_path) in &items {
                            let theme = self.colors[*category];
                            let fill = if *topping {
                                lighten(theme)
                            } else {
                                egui::Color32::from_rgb(theme[0], theme[1], theme[2])
                            };
                            let image = image_path.as_deref().and_then(|path| {
                                self.product_texture(ctx, path)
                                    .map(|texture| (texture, path))
                            });
                            let (button_rect, response) = ui.allocate_exact_size(
                                egui::vec2(button_width, 124.0),
                                egui::Sense::click(),
                            );
                            let button_fill = if response.is_pointer_button_down_on() {
                                ui.visuals().widgets.active.bg_fill
                            } else if response.hovered() {
                                ui.visuals().widgets.hovered.bg_fill
                            } else {
                                fill
                            };
                            ui.painter().rect_filled(
                                button_rect,
                                egui::CornerRadius::same(12),
                                button_fill,
                            );
                            ui.painter().rect_stroke(
                                button_rect,
                                egui::CornerRadius::same(12),
                                ui.visuals().widgets.inactive.bg_stroke,
                                egui::StrokeKind::Inside,
                            );
                            let image_width = button_rect.width() * 0.33;
                            if let Some((image, path)) = image {
                                let image_panel = egui::Rect::from_min_max(
                                    button_rect.min,
                                    egui::pos2(
                                        button_rect.left() + image_width,
                                        button_rect.bottom(),
                                    ),
                                );
                                let corner_color = self
                                    .product_image_corners
                                    .get(path)
                                    .copied()
                                    .unwrap_or(button_fill);
                                ui.painter().rect_filled(
                                    image_panel,
                                    egui::CornerRadius::same(12),
                                    button_fill,
                                );
                                let image_inset = image_panel.shrink(3.0);
                                ui.painter().rect_filled(
                                    image_inset,
                                    egui::CornerRadius::same(9),
                                    corner_color,
                                );
                                let source = image.size_vec2();
                                let available = image_inset.shrink(4.0);
                                let scale = (available.width() / source.x)
                                    .min(available.height() / source.y);
                                let image_size = source * scale;
                                let image_rect =
                                    egui::Rect::from_center_size(image_panel.center(), image_size);
                                ui.painter().image(
                                    image.id(),
                                    image_rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    egui::Color32::WHITE,
                                );
                            }
                            let label_rect = egui::Rect::from_min_max(
                                egui::pos2(
                                    button_rect.left() + image_width + 4.0,
                                    button_rect.top() + 4.0,
                                ),
                                button_rect.max - egui::vec2(4.0, 4.0),
                            );
                            let name_color = egui::Color32::from_rgb(35, 73, 53);
                            let price_color = egui::Color32::from_rgb(70, 82, 72);
                            let name_galley = ui.painter().layout(
                                name.clone(),
                                egui::FontId::proportional(22.0),
                                name_color,
                                label_rect.width(),
                            );
                            let price_galley = ui.painter().layout_no_wrap(
                                money(*price),
                                egui::FontId::proportional(15.0),
                                price_color,
                            );
                            let content_height = name_galley.size().y + 4.0 + price_galley.size().y;
                            let content_top = label_rect.center().y - content_height * 0.5;
                            let painter = ui.painter().with_clip_rect(label_rect);
                            painter.galley(
                                egui::pos2(
                                    label_rect.center().x - name_galley.size().x * 0.5,
                                    content_top,
                                ),
                                name_galley.clone(),
                                name_color,
                            );
                            painter.galley(
                                egui::pos2(
                                    label_rect.center().x - price_galley.size().x * 0.5,
                                    content_top + name_galley.size().y + 4.0,
                                ),
                                price_galley,
                                price_color,
                            );
                            if response.clicked() {
                                self.add_product(name, *price, *topping, *category);
                            }
                        }
                    });
                }
            });
    }

    /// Renders the editable in-progress order, its total, and checkout actions.
    pub(crate) fn order_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Commande");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.receipts.is_empty(), header_button("Historique"))
                    .clicked()
                {
                    self.history_receipt_index = self.receipts.len().saturating_sub(1);
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

    /// Displays the selected completed receipt read-only, with footer navigation through history.
    pub(crate) fn previous_order_window(&mut self, ctx: &egui::Context) {
        if self.receipts.is_empty() {
            self.show_previous_order = false;
            return;
        }
        self.history_receipt_index = self.history_receipt_index.min(self.receipts.len() - 1);
        let receipt = self.receipts[self.history_receipt_index].clone();
        let screen_height = ctx.screen_rect().height();
        let content_height = receipt
            .items
            .iter()
            .map(|item| 58.0 + item.toppings.len() as f32 * 24.0)
            .sum::<f32>();
        let desired_height = (content_height + 190.0).min((screen_height - 40.0).max(240.0));
        let mut close_history = false;
        egui::Window::new("Historique des commandes")
            .open(&mut self.show_previous_order)
            .resizable(true)
            .collapsible(false)
            .default_size([620.0, desired_height])
            .max_size([
                ctx.screen_rect().width() - 24.0,
                (screen_height - 24.0).max(240.0),
            ])
            .show(ctx, |ui| {
                // Reserve a stable header slot; navigation stays anchored above variable order content.
                let header_width = ui.available_width();
                let (header_rect, _) =
                    ui.allocate_exact_size(egui::vec2(header_width, 48.0), egui::Sense::hover());
                let button_y = header_rect.center().y - 20.0;
                let previous_rect = egui::Rect::from_min_size(
                    egui::pos2(header_rect.left(), button_y),
                    egui::vec2(48.0, 40.0),
                );
                let close_rect = egui::Rect::from_min_size(
                    egui::pos2(header_rect.center().x - 44.0, button_y),
                    egui::vec2(88.0, 40.0),
                );
                let next_rect = egui::Rect::from_min_size(
                    egui::pos2(header_rect.right() - 48.0, button_y),
                    egui::vec2(48.0, 40.0),
                );
                let previous_clicked = ui
                    .allocate_new_ui(egui::UiBuilder::new().max_rect(previous_rect), |ui| {
                        ui.add_enabled(self.history_receipt_index > 0, egui::Button::new("◀"))
                            .on_hover_text("Commande précédente")
                    })
                    .inner
                    .clicked();
                let close_clicked = ui
                    .allocate_new_ui(egui::UiBuilder::new().max_rect(close_rect), |ui| {
                        ui.button("Fermer")
                    })
                    .inner
                    .clicked();
                let next_clicked = ui
                    .allocate_new_ui(egui::UiBuilder::new().max_rect(next_rect), |ui| {
                        ui.add_enabled(
                            self.history_receipt_index + 1 < self.receipts.len(),
                            egui::Button::new("▶"),
                        )
                        .on_hover_text("Commande suivante")
                    })
                    .inner
                    .clicked();
                if previous_clicked {
                    self.history_receipt_index -= 1;
                }
                if close_clicked {
                    close_history = true;
                }
                if next_clicked {
                    self.history_receipt_index += 1;
                }
                if let Some(completed_at) = Local.timestamp_opt(receipt.completed_at, 0).single() {
                    ui.label(completed_at.format("%d/%m/%Y à %H:%M").to_string());
                }
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height((desired_height - 180.0).max(100.0))
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
        if close_history {
            self.show_previous_order = false;
        }
    }
}
