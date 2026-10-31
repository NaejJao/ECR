use chrono::{Local, TimeZone};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_maximized(true)
            .with_fullscreen(true),
        ..Default::default()
    };
    eframe::run_native(
        "ECR — WaffleTime",
        options,
        Box::new(|_| Ok(Box::new(App::default()))),
    )
}

#[derive(Serialize, Deserialize)]
struct Product {
    name: String,
    #[serde(alias = "price_minor")]
    price: u64,
    category: usize,
    #[serde(default, alias = "is_topping")]
    topping: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Item {
    name: String,
    #[serde(alias = "price_minor")]
    price: u64,
    quantity: u32,
    #[serde(default)]
    category: String,
    #[serde(default)]
    category_index: usize,
    #[serde(default = "default_theme")]
    theme: [u8; 3],
    #[serde(default)]
    toppings: Vec<Item>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Receipt {
    completed_at: i64,
    #[serde(alias = "lines")]
    items: Vec<Item>,
    #[serde(alias = "total_minor")]
    total: u64,
}
#[derive(Serialize, Deserialize)]
struct MenuData {
    categories: Vec<String>,
    #[serde(default, alias = "category_colors")]
    colors: Vec<[u8; 3]>,
    products: Vec<Product>,
}
#[derive(Serialize, Deserialize)]
struct History {
    orders: Vec<Receipt>,
}
#[derive(Serialize)]
struct Export<'a> {
    business_date: String,
    orders: &'a [Receipt],
}

struct App {
    menu_path: PathBuf,
    orders_path: PathBuf,
    categories: Vec<String>,
    colors: Vec<[u8; 3]>,
    products: Vec<Product>,
    menu_bad: bool,
    receipts: Vec<Receipt>,
    history_bad: bool,
    order: Vec<Item>,
    selected: usize,
    screen: Screen,
    new_category: String,
    new_name: String,
    new_price: String,
    new_category_index: usize,
    new_topping: bool,
    edit_product: Option<usize>,
    edit_name: String,
    edit_price: String,
    edit_category: usize,
    edit_topping: bool,
    export_directory: String,
    expanded_categories: BTreeSet<String>,
    message: Option<String>,
}
#[derive(PartialEq, Eq)]
enum Screen {
    Order,
    Menu,
    Daily,
}

impl Default for App {
    fn default() -> Self {
        let menu_path = data_path("menu.json").unwrap_or_else(|_| PathBuf::from("ecr-menu.json"));
        let orders_path = menu_path.with_file_name("orders.json");
        let menu_exists = menu_path.exists();
        let menu = fs::read(&menu_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<MenuData>(&b).ok());
        let (categories, mut colors, products, menu_bad) = match menu {
            Some(m) if m.products.iter().all(|p| p.category < m.categories.len()) => {
                (m.categories, m.colors, m.products, false)
            }
            Some(_) => (
                vec!["Gaufres".into(), "Boissons".into()],
                Vec::new(),
                Vec::new(),
                true,
            ),
            None if menu_exists => (
                vec!["Gaufres".into(), "Boissons".into()],
                Vec::new(),
                Vec::new(),
                true,
            ),
            None => (
                vec!["Gaufres".into(), "Boissons".into()],
                Vec::new(),
                Vec::new(),
                false,
            ),
        };
        while colors.len() < categories.len() {
            colors.push(theme(colors.len()));
        }
        colors.truncate(categories.len());
        let history_exists = orders_path.exists();
        let history = fs::read(&orders_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<History>(&b).ok());
        let (receipts, history_bad) = match history {
            Some(h) => (h.orders, false),
            None => (Vec::new(), history_exists),
        };
        let mut app = Self {
            menu_path,
            orders_path,
            categories,
            colors,
            products,
            menu_bad,
            receipts,
            history_bad,
            order: Vec::new(),
            selected: 0,
            screen: Screen::Order,
            new_category: String::new(),
            new_name: String::new(),
            new_price: String::new(),
            new_category_index: 0,
            new_topping: false,
            edit_product: None,
            edit_name: String::new(),
            edit_price: String::new(),
            edit_category: 0,
            edit_topping: false,
            export_directory: String::new(),
            expanded_categories: BTreeSet::new(),
            message: None,
        };
        if !app.menu_path.exists() {
            app.save_menu();
        }
        if app.menu_bad {
            app.message = Some(
                "Impossible de charger le menu enregistré. Corrigez ou supprimez menu.json avant toute modification."
                    .into(),
            );
        }
        if app.history_bad {
            app.message = Some("Impossible de charger l’historique des commandes. La validation est désactivée pour protéger le fichier.".into());
        }
        app
    }
}

impl eframe::App for App {
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
    }
}

impl App {
    fn order_ui(&mut self, ctx: &egui::Context) {
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

    fn order_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Commande");
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

    fn add_product(&mut self, name: &str, price: u64, topping: bool, category: usize) {
        let item = Item {
            name: name.into(),
            price,
            quantity: 1,
            category: self.categories[category].clone(),
            category_index: category,
            theme: self.colors[category],
            toppings: Vec::new(),
        };
        if topping {
            if let Some(parent) = self
                .order
                .iter_mut()
                .rev()
                .find(|x| x.category_index == category)
            {
                if let Some(existing) = parent
                    .toppings
                    .iter_mut()
                    .find(|x| x.name == item.name && x.price == item.price)
                {
                    existing.quantity = existing.quantity.saturating_add(1);
                } else {
                    parent.toppings.push(item);
                }
            } else {
                self.message = Some(format!(
                    "Ajoutez d’abord un produit principal de la catégorie {} avant sa garniture.",
                    self.categories[category]
                ));
            }
        } else {
            let category_has_toppings = self
                .products
                .iter()
                .any(|product| product.category == category && product.topping);
            if !category_has_toppings {
                if let Some(existing) = self.order.iter_mut().find(|line| {
                    line.category_index == category
                        && line.name == item.name
                        && line.price == item.price
                        && line.toppings.is_empty()
                }) {
                    existing.quantity = existing.quantity.saturating_add(1);
                    self.message = None;
                    return;
                }
            }
            self.order.push(item);
            self.message = None;
        }
    }
    fn total(&self) -> u64 {
        self.order
            .iter()
            .fold(0, |sum, x| sum.saturating_add(x.total()))
    }
    fn complete(&mut self) {
        if self.history_bad {
            self.message=Some("Impossible de charger l’historique. La validation est désactivée pour protéger le fichier.".into());
            return;
        }
        if self.order.is_empty() {
            return;
        }
        self.receipts.push(Receipt {
            completed_at: Local::now().timestamp(),
            items: self.order.clone(),
            total: self.total(),
        });
        match self.save_orders() {
            Ok(()) => {
                self.order.clear();
                self.message = Some("Commande validée et enregistrée.".into());
            }
            Err(e) => {
                self.receipts.pop();
                self.message = Some(format!("Impossible d’enregistrer la commande : {e}"));
            }
        }
    }
    fn save_orders(&self) -> std::io::Result<()> {
        let parent = self
            .orders_path
            .parent()
            .ok_or_else(|| std::io::Error::other("invalid order path"))?;
        fs::create_dir_all(parent)?;
        fs::write(
            &self.orders_path,
            serde_json::to_vec_pretty(&History {
                orders: self.receipts.clone(),
            })
            .map_err(std::io::Error::other)?,
        )
    }
    fn save_menu(&mut self) {
        if self.menu_bad {
            self.message =
                Some("Impossible de charger le menu. Corrigez ou supprimez menu.json avant d’enregistrer.".into());
            return;
        }
        let result = (|| {
            if let Some(parent) = self.menu_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let data = MenuData {
                categories: self.categories.clone(),
                colors: self.colors.clone(),
                products: self
                    .products
                    .iter()
                    .map(|p| Product {
                        name: p.name.clone(),
                        price: p.price,
                        category: p.category,
                        topping: p.topping,
                    })
                    .collect(),
            };
            fs::write(
                &self.menu_path,
                serde_json::to_vec_pretty(&data).map_err(std::io::Error::other)?,
            )
        })();
        if let Err(e) = result {
            self.message = Some(format!("Impossible d’enregistrer le menu : {e}"));
        }
    }
    fn menu_ui(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "CONFIGURATION",
            "Gestion du menu",
            "Gérez les catégories, leurs couleurs, les produits et les garnitures.",
        );
        ui.add_space(16.0);
        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("CATÉGORIES").strong());
            card().show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_category)
                        .on_hover_text("Nom de la catégorie");
                    if ui
                        .add(primary_button("Ajouter la catégorie", [135.0, 42.0]))
                        .clicked()
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
                        let changed = ui.text_edit_singleline(&mut self.categories[i]).changed();
                        let color_changed =
                            ui.color_edit_button_srgb(&mut self.colors[i]).changed();
                        if changed || color_changed {
                            self.save_menu();
                        }
                        let occupied = self.products.iter().any(|p| p.category == i);
                        if ui
                            .add_enabled(!occupied, egui::Button::new("Supprimer"))
                            .clicked()
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
                        let color = self.colors[index];
                        let fill = if self.new_category_index == index {
                            egui::Color32::from_rgb(color[0], color[1], color[2])
                        } else {
                            egui::Color32::WHITE
                        };
                        let count = self.products.iter().filter(|p| p.category == index).count();
                        if ui
                            .add(
                                egui::Button::new(format!("{category}  ·  {count}"))
                                    .fill(fill)
                                    .min_size([150.0, 52.0].into()),
                            )
                            .clicked()
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

                    ui.checkbox(&mut self.new_topping, "C’est une garniture");
                    if ui
                        .add(primary_button("Ajouter le produit", [140.0, 42.0]))
                        .clicked()
                    {
                        let n = self.new_name.trim();
                        if n.is_empty() {
                            self.message = Some("Saisissez un nom de produit.".into());
                        } else if let Some(price) = parse_price(&self.new_price) {
                            self.products.push(Product {
                                name: n.into(),
                                price,
                                category: self.new_category_index,
                                topping: self.new_topping,
                            });
                            self.new_name.clear();
                            self.new_price.clear();
                            self.new_topping = false;
                            self.save_menu();
                        } else {
                            self.message =
                                Some("Saisissez un prix valide supérieur à zéro.".into());
                        }
                    }
                });
                let mut delete_product = None;
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
                                            .clicked()
                                        {
                                            let n = self.edit_name.trim();
                                            if n.is_empty() {
                                                self.message =
                                                    Some("Saisissez un nom de produit.".into());
                                            } else if let Some(price) =
                                                parse_price(&self.edit_price)
                                            {
                                                self.products[i] = Product {
                                                    name: n.into(),
                                                    price,
                                                    category: self.edit_category,
                                                    topping: self.edit_topping,
                                                };
                                                self.edit_product = None;
                                                self.save_menu();
                                            } else {
                                                self.message =
                                                    Some("Saisissez un prix valide.".into());
                                            }
                                        }
                                        if ui.button("Annuler").clicked() {
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
                                            if ui.button("Modifier").clicked() {
                                                self.edit_name = p.name.clone();
                                                self.edit_price = format!(
                                                    "{},{:02}",
                                                    p.price / 100,
                                                    p.price % 100
                                                );
                                                self.edit_category = p.category;
                                                self.edit_topping = p.topping;
                                                self.edit_product = Some(i);
                                            }
                                            if ui.button("Supprimer").clicked() {
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
                }
            }
        });
        if let Some(m) = &self.message {
            ui.label(m);
        }
    }
    fn daily_ui(&mut self, ui: &mut egui::Ui) {
        let date = Local::now().date_naive();
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
        page_heading(
            ui,
            "VENTES",
            "Bilan du jour",
            "Résumé des commandes terminées aujourd’hui.",
        );
        ui.horizontal(|ui| {
            ui.label(date.format("%d/%m/%Y").to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.small("Les garnitures sont incluses dans les ventes, pas dans les quantités par catégorie.");
            });
        });
        ui.add_space(12.0);
        ui.columns(2, |cols| {
            card().show(&mut cols[0], |ui| {
                ui.label(egui::RichText::new("APERÇU DU JOUR").strong());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    overview_card(ui, "Commandes", orders.len().to_string());
                    overview_card(ui, "Total des ventes", money(total));
                });
            });
            card().show(&mut cols[1], |ui| {
                ui.label(egui::RichText::new("EXPORT DE FIN DE JOURNÉE").strong());
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
                        primary_button("Exporter les commandes", [ui.available_width(), 50.0]),
                    )
                    .clicked()
                {
                    self.export(date, &orders);
                }
                if orders.is_empty() {
                    ui.small("Aucune commande terminée aujourd’hui.");
                }
            });
        });
        ui.add_space(14.0);
        ui.label(egui::RichText::new("ARTICLES PRINCIPAUX PAR CATÉGORIE").strong());
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
                            let label = if expanded {
                                format!("⌄  {category}")
                            } else {
                                format!("›  {category}")
                            };
                            let width = ui.available_width();
                            let response = ui.add(
                                egui::Button::new(egui::RichText::new(label).size(18.0).strong())
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE)
                                    .min_size([width, 48.0].into()),
                            );
                            let count_rect = egui::Rect::from_min_size(
                                egui::pos2(response.rect.right() - 70.0, response.rect.top()),
                                egui::vec2(58.0, response.rect.height()),
                            );
                            ui.painter().text(
                                count_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                count.to_string(),
                                egui::FontId::proportional(20.0),
                                ui.visuals().text_color(),
                            );
                            clicked = response.clicked();
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
    fn export(&mut self, date: chrono::NaiveDate, orders: &[Receipt]) {
        let dir = self.export_directory.trim();
        if dir.is_empty() {
            self.message = Some("Saisissez le chemin du dossier USB.".into());
            return;
        }
        let path = PathBuf::from(dir).join(format!("ecr-orders-{}.json", date.format("%Y-%m-%d")));
        let data = Export {
            business_date: date.to_string(),
            orders,
        };
        let res = serde_json::to_vec_pretty(&data)
            .map_err(std::io::Error::other)
            .and_then(|bytes| {
                let mut f = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                f.write_all(&bytes)?;
                f.sync_all()
            });
        self.message = Some(match res {
            Ok(()) => format!(
                "{} commandes exportées vers {}",
                orders.len(),
                path.display()
            ),
            Err(e) => format!("Échec de l’export : {e}"),
        });
    }
}

impl Item {
    fn total(&self) -> u64 {
        self.toppings.iter().fold(
            self.price.saturating_mul(u64::from(self.quantity)),
            |s, x| s.saturating_add(x.total()),
        )
    }
    fn remap_category(&mut self, i: usize) {
        if self.category_index == i {
            self.category_index = usize::MAX;
        } else if self.category_index > i {
            self.category_index -= 1;
        }
        for x in &mut self.toppings {
            x.remap_category(i);
        }
    }
}
fn parse_price(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() || s.starts_with('-') || (s.contains(',') && s.contains('.')) {
        return None;
    }
    let normalized = s.replace(',', ".");
    let mut p = normalized.split('.');
    let whole = p.next()?.parse::<u64>().ok()?;
    let frac = p.next().unwrap_or("0");
    if p.next().is_some() || frac.len() > 2 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let c = match frac.len() {
        0 => 0,
        1 => frac.parse::<u64>().ok()? * 10,
        2 => frac.parse::<u64>().ok()?,
        _ => return None,
    };
    whole.checked_mul(100)?.checked_add(c).filter(|x| *x > 0)
}
fn money(c: u64) -> String {
    format!("{},{:02}", c / 100, c % 100)
}
fn data_path(file: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|x| PathBuf::from(x).join("Library/Application Support"));
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|x| PathBuf::from(x).join(".local/share")));
    base.map(|p| p.join("ecr").join(file))
        .ok_or_else(|| "Impossible de trouver le dossier de données de l’application".into())
}
fn main_item_counts(orders: &[Receipt], categories: &[String]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for category in categories {
        if !counts
            .keys()
            .any(|known: &String| known.eq_ignore_ascii_case(category.trim()))
        {
            counts.insert(category.clone(), 0);
        }
    }
    for item in orders.iter().flat_map(|order| &order.items) {
        let recorded = if item.category.trim().is_empty() {
            "Non catégorisé"
        } else {
            item.category.trim()
        };
        let category = canonical_category(recorded, categories);
        let count = counts.entry(category).or_insert(0_u64);
        *count = count.saturating_add(u64::from(item.quantity));
    }
    counts
}

fn ordered_category_counts(
    counts: &BTreeMap<String, u64>,
    categories: &[String],
) -> Vec<(String, u64)> {
    let mut ordered = Vec::new();
    for category in categories {
        if let Some((name, count)) = counts
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(category.trim()))
        {
            if !ordered
                .iter()
                .any(|(known, _): &(String, u64)| known.eq_ignore_ascii_case(name))
            {
                ordered.push((name.clone(), *count));
            }
        }
    }
    for (name, count) in counts {
        if !ordered
            .iter()
            .any(|(known, _)| known.eq_ignore_ascii_case(name))
        {
            ordered.push((name.clone(), *count));
        }
    }
    ordered
}

fn main_products_in_category(
    orders: &[Receipt],
    selected_category: &str,
    categories: &[String],
) -> BTreeMap<String, (u64, u64)> {
    let mut details = BTreeMap::new();
    for item in orders.iter().flat_map(|order| &order.items) {
        let recorded = if item.category.trim().is_empty() {
            "Non catégorisé"
        } else {
            item.category.trim()
        };
        if canonical_category(recorded, categories) != selected_category {
            continue;
        }
        let row = details.entry(item.name.clone()).or_insert((0_u64, 0_u64));
        row.0 = row.0.saturating_add(u64::from(item.quantity));
        row.1 = row
            .1
            .saturating_add(item.price.saturating_mul(u64::from(item.quantity)));
    }
    details
}

fn canonical_category(recorded: &str, categories: &[String]) -> String {
    if let Some(exact) = categories
        .iter()
        .find(|category| category.trim().eq_ignore_ascii_case(recorded))
    {
        return exact.trim().to_owned();
    }
    let close_matches: Vec<&String> = categories
        .iter()
        .filter(|category| {
            category.trim().chars().count() >= 6 && one_edit_apart(category.trim(), recorded)
        })
        .collect();
    if close_matches.len() == 1 {
        close_matches[0].trim().to_owned()
    } else {
        recorded.to_owned()
    }
}

fn one_edit_apart(left: &str, right: &str) -> bool {
    let left: Vec<char> = left.to_lowercase().chars().collect();
    let right: Vec<char> = right.to_lowercase().chars().collect();
    if left.len().abs_diff(right.len()) > 1 {
        return false;
    }
    let (mut i, mut j, mut edits) = (0, 0, 0);
    while i < left.len() && j < right.len() {
        if left[i] == right[j] {
            i += 1;
            j += 1;
            continue;
        }
        edits += 1;
        if edits > 1 {
            return false;
        }
        if left.len() > right.len() {
            i += 1;
        } else if right.len() > left.len() {
            j += 1;
        } else {
            i += 1;
            j += 1;
        }
    }
    if i < left.len() || j < right.len() {
        edits += 1;
    }
    edits <= 1
}

fn theme(i: usize) -> [u8; 3] {
    [
        [210, 232, 211],
        [211, 225, 245],
        [249, 225, 190],
        [237, 215, 231],
        [211, 234, 231],
        [242, 220, 205],
        [228, 224, 246],
        [231, 232, 204],
    ][i % 8]
}
fn default_theme() -> [u8; 3] {
    theme(0)
}
fn lighten(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(
        ((u16::from(c[0]) + 255) / 2) as u8,
        ((u16::from(c[1]) + 255) / 2) as u8,
        ((u16::from(c[2]) + 255) / 2) as u8,
    )
}
fn page_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(247, 247, 243))
        .inner_margin(egui::Margin::symmetric(24, 18))
}
fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(232, 234, 228),
        ))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(12))
}
fn page_heading(ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(eyebrow)
            .size(12.0)
            .strong()
            .color(egui::Color32::from_rgb(102, 117, 103)),
    );
    ui.heading(egui::RichText::new(title).size(28.0).strong());
    ui.label(subtitle);
}
fn header_button(s: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(s).strong().color(egui::Color32::WHITE))
        .fill(egui::Color32::from_rgb(57, 99, 72))
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(10))
        .min_size([150.0, 44.0].into())
}
fn danger_button(s: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(s).strong().color(egui::Color32::WHITE))
        .fill(egui::Color32::from_rgb(176, 54, 48))
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(10))
        .min_size([150.0, 44.0].into())
}
fn primary_button(s: &str, size: [f32; 2]) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(s).strong().color(egui::Color32::WHITE))
        .fill(egui::Color32::from_rgb(49, 104, 70))
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(12))
        .min_size(size.into())
}
fn overview_card(ui: &mut egui::Ui, label: &str, value: String) {
    card().inner_margin(egui::Margin::same(20)).show(ui, |ui| {
        ui.set_min_width(240.0);
        ui.label(label);
        ui.label(
            egui::RichText::new(value)
                .size(28.0)
                .strong()
                .color(egui::Color32::from_rgb(35, 73, 53)),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previous_menu_json_schema_still_loads_products_and_categories() {
        let json = r#"{
            "categories": ["Waffles"],
            "category_colors": [[210, 232, 211]],
            "products": [
                {"name": "Waffle", "price_minor": 350, "category": 0, "is_topping": false},
                {"name": "Berry", "price_minor": 50, "category": 0, "is_topping": true}
            ]
        }"#;
        let menu: MenuData = serde_json::from_str(json).unwrap();
        assert_eq!(menu.categories, ["Waffles"]);
        assert_eq!(menu.products.len(), 2);
        assert_eq!(menu.products[0].price, 350);
        assert!(menu.products[1].topping);
        assert_eq!(menu.colors, [[210, 232, 211]]);
    }

    #[test]
    fn historical_category_typo_merges_only_when_there_is_one_close_match() {
        assert_eq!(
            canonical_category("Boisons froides", &["Boissons froides".into()]),
            "Boissons froides"
        );
        assert_eq!(
            canonical_category("Drinks", &["Drinks".into(), "Drinks!".into()]),
            "Drinks"
        );
        assert_eq!(
            canonical_category("Dessert", &["Desert".into(), "Desserts".into()]),
            "Dessert"
        );
    }

    #[test]
    fn topping_total_is_included_but_daily_category_count_uses_main_items_only() {
        let receipt = Receipt {
            completed_at: 0,
            items: vec![Item {
                name: "Waffle".into(),
                price: 500,
                quantity: 1,
                category: "Waffles".into(),
                category_index: 0,
                theme: theme(0),
                toppings: vec![Item {
                    name: "Berry".into(),
                    price: 100,
                    quantity: 1,
                    category: "Waffles".into(),
                    category_index: 0,
                    theme: theme(0),
                    toppings: vec![],
                }],
            }],
            total: 600,
        };
        assert_eq!(receipt.items[0].total(), 600);
        let counts = main_item_counts(&[receipt], &["Waffles".into(), "Boissons froides".into()]);
        assert_eq!(counts.get("Waffles"), Some(&1));
    }
}
