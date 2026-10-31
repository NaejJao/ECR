//! Application state, startup, and persistence operations.
//!
//! The UI is implemented in [`crate::ui`]; this module owns the mutable data behind
//! those screens and performs durable writes for menus, completed orders, and exports.

use crate::{
    domain::{attach_topping, data_path, theme},
    model::{Export, History, Item, MenuData, Product, Receipt},
};
use chrono::Local;
use std::{collections::BTreeSet, fs, fs::OpenOptions, io::Write, path::PathBuf};

/// Mutable runtime state shared by the cashier screens.
///
/// Persistent menu/history data and transient order/editor state are kept together
/// so UI actions can update state first and save completed changes through one path.
pub(crate) struct App {
    /// Location of the user-configured menu JSON file.
    pub(crate) menu_path: PathBuf,
    /// Location of the completed-order history JSON file.
    pub(crate) orders_path: PathBuf,
    /// Category names in the order used by the sale screen and reports.
    pub(crate) categories: Vec<String>,
    /// RGB theme color associated with each category.
    pub(crate) colors: Vec<[u8; 3]>,
    /// Products configured for sale.
    pub(crate) products: Vec<Product>,
    /// Indicates that the menu file was present but could not be safely loaded.
    pub(crate) menu_bad: bool,
    /// Completed orders, in chronological insertion order.
    pub(crate) receipts: Vec<Receipt>,
    /// Prevents new sales from overwriting unreadable history.
    pub(crate) history_bad: bool,
    /// Items currently being entered by the cashier.
    pub(crate) order: Vec<Item>,
    /// Selected category on the sale screen.
    pub(crate) selected: usize,
    /// Currently visible application screen.
    pub(crate) screen: Screen,
    /// Category currently being created in menu management.
    pub(crate) new_category: String,
    /// Product name currently being entered.
    pub(crate) new_name: String,
    /// Product price text currently being entered.
    pub(crate) new_price: String,
    /// Category selected for the new product and product list filter.
    pub(crate) new_category_index: usize,
    /// Whether the new product is a topping.
    pub(crate) new_topping: bool,
    /// Index of the product being edited, if any.
    pub(crate) edit_product: Option<usize>,
    /// Edited product name.
    pub(crate) edit_name: String,
    /// Edited product price text.
    pub(crate) edit_price: String,
    /// Edited product category index.
    pub(crate) edit_category: usize,
    /// Whether the edited product is a topping.
    pub(crate) edit_topping: bool,
    /// USB destination entered on the daily report screen.
    pub(crate) export_directory: String,
    /// Category names whose detail tables are expanded in the daily report.
    pub(crate) expanded_categories: BTreeSet<String>,
    /// Whether the read-only previous-order dialog is open.
    pub(crate) show_previous_order: bool,
    /// Latest user-facing success or error message.
    pub(crate) message: Option<String>,
}

/// Top-level screens selectable from the application header.
#[derive(PartialEq, Eq)]
pub(crate) enum Screen {
    /// Product selection and current sale.
    Order,
    /// Category and product configuration.
    Menu,
    /// Daily totals and order export.
    Daily,
}

impl Default for App {
    /// Loads menu and receipt data, falling back safely when either file is invalid.
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
            show_previous_order: false,
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

impl App {
    /// Adds a product to the open order or attaches a topping to its matching base item.
    ///
    /// Products from categories without toppings are combined into quantity lines.
    /// Topping-capable base products remain separate so each can own its own toppings.
    pub(crate) fn add_product(&mut self, name: &str, price: u64, topping: bool, category: usize) {
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
            match attach_topping(&mut self.order, item, category) {
                Some(true) => self.message = None,
                Some(false) => {}
                None => {
                    self.message = Some(format!(
                        "Ajoutez d’abord un produit principal de la catégorie {} avant sa garniture.",
                        self.categories[category]
                    ));
                }
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

    /// Calculates the open order total in centimes, including nested toppings.
    pub(crate) fn total(&self) -> u64 {
        self.order
            .iter()
            .fold(0, |sum, x| sum.saturating_add(x.total()))
    }

    /// Persists the current order as a receipt and clears it only after a successful save.
    ///
    /// On write failure, the receipt is rolled back in memory and an error is shown.
    pub(crate) fn complete(&mut self) {
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

    /// Serializes the complete receipt history to the configured history file.
    pub(crate) fn save_orders(&self) -> std::io::Result<()> {
        let parent = self
            .orders_path
            .parent()
            .ok_or_else(|| std::io::Error::other("invalid order path"))?;
        fs::create_dir_all(parent)?;
        write_json_atomically(
            &self.orders_path,
            &serde_json::to_vec_pretty(&History {
                orders: self.receipts.clone(),
            })
            .map_err(std::io::Error::other)?,
        )
    }

    /// Persists categories, colors, and products without overwriting an unreadable menu.
    pub(crate) fn save_menu(&mut self) {
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
            write_json_atomically(
                &self.menu_path,
                &serde_json::to_vec_pretty(&data).map_err(std::io::Error::other)?,
            )
        })();
        if let Err(e) = result {
            self.message = Some(format!("Impossible d’enregistrer le menu : {e}"));
        }
    }

    /// Writes the day's detailed receipts as a new dated JSON file in the USB directory.
    ///
    /// The file is opened with `create_new` so a previous export is never silently replaced.
    pub(crate) fn export(&mut self, date: chrono::NaiveDate, orders: &[Receipt]) {
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

/// Writes bytes to a neighboring temporary file and renames it into place.
///
/// Keeping the temporary file beside the destination ensures the rename stays on
/// the same filesystem, making replacement atomic on platforms that support it.
fn write_json_atomically(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp_path = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp_path);
    }
    result
}

/// Configures the maximized full-screen viewport and launches the native GUI.
pub(crate) fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
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
