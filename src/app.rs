use crate::{
    domain::{attach_topping, data_path, theme},
    model::{Export, History, Item, MenuData, Product, Receipt},
};
use chrono::Local;
use std::{collections::BTreeSet, fs, fs::OpenOptions, io::Write, path::PathBuf};

pub(crate) struct App {
    pub(crate) menu_path: PathBuf,
    pub(crate) orders_path: PathBuf,
    pub(crate) categories: Vec<String>,
    pub(crate) colors: Vec<[u8; 3]>,
    pub(crate) products: Vec<Product>,
    pub(crate) menu_bad: bool,
    pub(crate) receipts: Vec<Receipt>,
    pub(crate) history_bad: bool,
    pub(crate) order: Vec<Item>,
    pub(crate) selected: usize,
    pub(crate) screen: Screen,
    pub(crate) new_category: String,
    pub(crate) new_name: String,
    pub(crate) new_price: String,
    pub(crate) new_category_index: usize,
    pub(crate) new_topping: bool,
    pub(crate) edit_product: Option<usize>,
    pub(crate) edit_name: String,
    pub(crate) edit_price: String,
    pub(crate) edit_category: usize,
    pub(crate) edit_topping: bool,
    pub(crate) export_directory: String,
    pub(crate) expanded_categories: BTreeSet<String>,
    pub(crate) show_previous_order: bool,
    pub(crate) message: Option<String>,
}

#[derive(PartialEq, Eq)]
pub(crate) enum Screen {
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

    pub(crate) fn total(&self) -> u64 {
        self.order
            .iter()
            .fold(0, |sum, x| sum.saturating_add(x.total()))
    }

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

    pub(crate) fn save_orders(&self) -> std::io::Result<()> {
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
            fs::write(
                &self.menu_path,
                serde_json::to_vec_pretty(&data).map_err(std::io::Error::other)?,
            )
        })();
        if let Err(e) = result {
            self.message = Some(format!("Impossible d’enregistrer le menu : {e}"));
        }
    }

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
