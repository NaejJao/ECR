//! Menu configuration for categories, products, toppings, product images, and display order.
//!
//! Product rows are filtered by category, reordered with adjacent moves, and kept in a scroll area.
//! Product images can be chosen through the operating system's native file picker.

use crate::{app::App, domain::*, model::Product, ui::widgets::*};
use eframe::egui;

/// Converts an optional image path from an editor field into its persisted value.
fn non_empty_path(path: &str) -> Option<String> {
    let path = path.trim();
    (!path.is_empty()).then(|| path.to_owned())
}

impl App {
    /// Renders category editing and category-filtered product management.
    pub(crate) fn menu_ui(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "CONFIGURATION",
            "Gestion du menu",
            "Gérez les catégories, leurs couleurs, les produits, les garnitures et l’export.",
        );
        ui.add_space(16.0);
        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("EXPORT PDF").strong());
            card().show(&mut cols[0], |ui| {
                ui.label("Dossier d’export");
                ui.add(
                    egui::TextEdit::singleline(&mut self.export_directory)
                        .hint_text("Dossier, ex. /media/vous/USB")
                        .desired_width(ui.available_width()),
                );
                ui.horizontal(|ui| {
                    if ui.button("Choisir…").pressed(ui) {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.export_directory = path.display().to_string();
                            self.save_menu();
                        }
                    }
                    if ui.button("Enregistrer").pressed(ui) {
                        self.save_menu();
                        self.message = Some("Dossier d’export enregistré.".into());
                    }
                });
            });
            cols[0].add_space(12.0);
            cols[0].label(egui::RichText::new("CATÉGORIES").strong());
            card().show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_category)
                        .on_hover_text("Nom de la catégorie");
                    if ui
                        .add(primary_button("Ajouter la catégorie", [135.0, 42.0]))
                        .pressed(ui)
                    {
                        let n = self.new_category.trim();
                        if n.is_empty() {
                            self.message = Some("Saisissez un nom de catégorie.".into());
                        } else if self.categories.iter().any(|x| x.eq_ignore_ascii_case(n)) {
                            self.message = Some("Cette catégorie existe déjà.".into());
                        } else {
                            self.categories.push(n.into());
                            self.colors.push(theme(self.colors.len()));
                            self.new_category_index = self.categories.len() - 1;
                            self.new_category.clear();
                            self.save_menu();
                        }
                    }
                });
            });
            let mut delete_cat = None;
            for i in 0..self.categories.len() {
                card().show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        let previous_name = self.categories[i].clone();
                        let changed = ui.text_edit_singleline(&mut self.categories[i]).changed();
                        let color_changed =
                            ui.color_edit_button_srgb(&mut self.colors[i]).changed();
                        if changed {
                            let name = self.categories[i].trim();
                            if name.is_empty() {
                                self.categories[i] = previous_name;
                                self.message =
                                    Some("Le nom d’une catégorie ne peut pas être vide.".into());
                            } else if self
                                .categories
                                .iter()
                                .enumerate()
                                .any(|(index, category)| {
                                    index != i && category.eq_ignore_ascii_case(name)
                                })
                            {
                                self.categories[i] = previous_name;
                                self.message = Some("Cette catégorie existe déjà.".into());
                            } else {
                                self.categories[i] = name.to_owned();
                                self.save_menu();
                            }
                        }
                        if color_changed {
                            self.save_menu();
                        }
                        let occupied = self.products.iter().any(|p| p.category == i);
                        if ui
                            .add_enabled(!occupied, egui::Button::new("Supprimer"))
                            .pressed(ui)
                        {
                            delete_cat = Some(i);
                        }
                        if occupied {
                            ui.small("Supprimez d’abord ses produits");
                        }
                    });
                });
            }
            if let Some(i) = delete_cat {
                self.categories.remove(i);
                self.colors.remove(i);
                for p in &mut self.products {
                    if p.category > i {
                        p.category -= 1;
                    }
                }
                for item in &mut self.order {
                    item.remap_category(i);
                }
                self.selected = self.selected.min(self.categories.len().saturating_sub(1));
                self.new_category_index = self
                    .new_category_index
                    .min(self.categories.len().saturating_sub(1));
                self.save_menu();
            }
            cols[1].label(egui::RichText::new("PRODUITS").strong());
            if self.categories.is_empty() {
                cols[1].label("Ajoutez d’abord une catégorie.");
            } else {
                cols[1].horizontal_wrapped(|ui| {
                    for (index, category) in self.categories.iter().enumerate() {
                        let count = self.products.iter().filter(|p| p.category == index).count();
                        if ui
                            .add(
                                category_button(
                                    egui::RichText::new(format!("{category}  ·  {count}")),
                                    self.colors[index],
                                    self.new_category_index == index,
                                )
                                .min_size([150.0, 52.0].into()),
                            )
                            .pressed(ui)
                        {
                            self.new_category_index = index;
                            self.edit_product = None;
                        }
                    }
                });
                cols[1].label(
                    egui::RichText::new(format!(
                        "Produits · {}",
                        self.categories[self.new_category_index]
                    ))
                    .size(20.0)
                    .strong(),
                );
                card().show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_name)
                                .hint_text("Nom du produit"),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_price)
                                .hint_text("Prix, ex. 4,50")
                                .desired_width(100.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_image_path)
                                .hint_text("Chemin de l’image (PNG/JPEG, facultatif)"),
                        );
                        if ui.button("Choisir…").pressed(ui) {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.new_image_path = path.display().to_string();
                            }
                        }
                    });

                    ui.checkbox(&mut self.new_topping, "C’est une garniture");
                    if ui
                        .add(primary_button("Ajouter le produit", [140.0, 42.0]))
                        .pressed(ui)
                    {
                        let n = self.new_name.trim();
                        if n.is_empty() {
                            self.message = Some("Saisissez un nom de produit.".into());
                        } else if self.new_topping
                            && self.products.iter().any(|product| {
                                product.topping && product.name.eq_ignore_ascii_case(n)
                            })
                        {
                            self.message = Some("Une garniture portant ce nom existe déjà.".into());
                        } else if let Some(price) = parse_price(&self.new_price) {
                            self.products.push(Product {
                                name: n.into(),
                                price,
                                category: self.new_category_index,
                                topping: self.new_topping,
                                image_path: non_empty_path(&self.new_image_path),
                            });
                            self.new_name.clear();
                            self.new_price.clear();
                            self.new_image_path.clear();
                            self.new_topping = false;
                            self.save_menu();
                        } else {
                            self.message =
                                Some("Saisissez un prix valide (0 accepté pour un produit offert).".into());
                        }
                    }
                });
                let mut delete_product = None;
                let mut reorder_product = None;
                let products_list_height = cols[1].available_height().max(180.0);
                egui::ScrollArea::vertical()
                    .id_salt("menu_products_list")
                    .max_height(products_list_height)
                    .auto_shrink([false, false])
                    .show(&mut cols[1], |ui| {
                        for i in 0..self.products.len() {
                            if self.products[i].category != self.new_category_index {
                                continue;
                            }
                            if self.edit_product == Some(i) {
                                card().show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut self.edit_name);
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.edit_price)
                                                .desired_width(90.0),
                                        );
                                        ui.checkbox(&mut self.edit_topping, "Garniture");
                                    });
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.edit_image_path)
                                                .hint_text("Chemin de l’image (PNG/JPEG, facultatif)"),
                                        );
                                        if ui.button("Choisir…").pressed(ui) {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("Images", &["png", "jpg", "jpeg"])
                                                .pick_file()
                                            {
                                                self.edit_image_path = path.display().to_string();
                                            }
                                        }
                                    });
                                    egui::ComboBox::from_id_salt(("editcat", i))
                                        .selected_text(&self.categories[self.edit_category])
                                        .show_ui(ui, |ui| {
                                            for (j, c) in self.categories.iter().enumerate() {
                                                ui.selectable_value(&mut self.edit_category, j, c);
                                            }
                                        });
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(primary_button("Enregistrer", [145.0, 42.0]))
                                            .pressed(ui)
                                        {
                                            let n = self.edit_name.trim();
                                            if n.is_empty() {
                                                self.message =
                                                    Some("Saisissez un nom de produit.".into());
                                            } else if self.edit_topping
                                                && self.products.iter().enumerate().any(
                                                    |(index, product)| {
                                                        index != i
                                                            && product.topping
                                                            && product.name.eq_ignore_ascii_case(n)
                                                    },
                                                )
                                            {
                                                self.message = Some(
                                                    "Une garniture portant ce nom existe déjà."
                                                        .into(),
                                                );
                                            } else if let Some(price) =
                                                parse_price(&self.edit_price)
                                            {
                                                self.products[i] = Product {
                                                    name: n.into(),
                                                    price,
                                                    category: self.edit_category,
                                                    topping: self.edit_topping,
                                                    image_path: non_empty_path(&self.edit_image_path),
                                                };
                                                self.edit_product = None;
                                                self.save_menu();
                                            } else {
                                                self.message =
                                                    Some("Saisissez un prix valide (0 accepté pour un produit offert).".into());
                                            }
                                        }
                                        if ui.button("Annuler").pressed(ui) {
                                            self.edit_product = None;
                                        }
                                    });
                                });
                            } else {
                                card()
                                    .fill(if self.products[i].topping {
                                        egui::Color32::from_rgb(247, 242, 232)
                                    } else {
                                        egui::Color32::WHITE
                                    })
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let p = &self.products[i];
                                            let category = p.category;
                                            let can_move_up = self.products[..i]
                                                .iter()
                                                .any(|product| product.category == category);
                                            let can_move_down = self.products[i + 1..]
                                                .iter()
                                                .any(|product| product.category == category);
                                            let cat = self
                                                .categories
                                                .get(p.category)
                                                .map(String::as_str)
                                                .unwrap_or("?");
                                            ui.label(format!(
                                                "{} · {} · {}{}",
                                                p.name,
                                                money(p.price),
                                                cat,
                                                if p.topping { " · Garniture" } else { "" }
                                            ));
                                            if reorder_arrow_button(ui, true, can_move_up).pressed(ui)
                                            {
                                                reorder_product = Some((i, true));
                                            }
                                            if reorder_arrow_button(ui, false, can_move_down).pressed(ui)
                                            {
                                                reorder_product = Some((i, false));
                                            }
                                            if ui.button("Modifier").pressed(ui) {
                                                self.edit_name = p.name.clone();
                                                self.edit_price = format!(
                                                    "{},{:02}",
                                                    p.price / 100,
                                                    p.price % 100
                                                );
                                                self.edit_category = p.category;
                                                self.edit_topping = p.topping;
                                                self.edit_image_path =
                                                    p.image_path.clone().unwrap_or_default();
                                                self.edit_product = Some(i);
                                            }
                                            if ui.button("Supprimer").pressed(ui) {
                                                delete_product = Some(i);
                                            }
                                        });
                                    });
                            }
                        }
                    });
                if let Some(i) = delete_product {
                    self.products.remove(i);
                    self.edit_product = None;
                    self.save_menu();
                } else if let Some((i, move_up)) = reorder_product {
                    self.move_product(i, move_up);
                }
            }
        });
        if let Some(m) = &self.message {
            ui.label(m);
        }
    }
}
