//! Application state, startup, and persistence operations.
//!
//! The UI is implemented in [`crate::ui`]; this module owns the mutable data behind
//! those screens and performs durable writes for menus, completed orders, and exports.

use crate::{
    domain::{attach_topping_to_selected, canonical_category, data_path, money, theme},
    model::{History, Item, MenuData, Product, Receipt},
};
use chrono::{Datelike, Local, NaiveDate, TimeZone};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::PathBuf,
    time::SystemTime,
};

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
    /// Decoded product images cached for the current application session.
    pub(crate) product_images: HashMap<String, Option<eframe::egui::TextureHandle>>,
    /// Upper-left source pixel used as the background of each product-image panel.
    pub(crate) product_image_corners: HashMap<String, eframe::egui::Color32>,
    /// Last observed modification time of each product image, used to reload edited files.
    pub(crate) product_image_stamps: HashMap<String, Option<SystemTime>>,
    /// Image-loading progress while the splash screen is visible; `None` once startup is done.
    pub(crate) startup: Option<Startup>,
    /// Indicates that the menu file was present but could not be safely loaded.
    pub(crate) menu_bad: bool,
    /// Completed orders, in chronological insertion order.
    pub(crate) receipts: Vec<Receipt>,
    /// Prevents new sales from overwriting unreadable history.
    pub(crate) history_bad: bool,
    /// Items currently being entered by the cashier.
    pub(crate) order: Vec<Item>,
    /// Base item in the open order that will receive the next topping.
    pub(crate) selected_line: Option<usize>,
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
    /// Optional image path currently being entered for the new product.
    pub(crate) new_image_path: String,
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
    /// Optional image path for the product being edited.
    pub(crate) edit_image_path: String,
    /// Edited product category index.
    pub(crate) edit_category: usize,
    /// Whether the edited product is a topping.
    pub(crate) edit_topping: bool,
    /// Destination folder for PDF exports, configured in menu settings.
    pub(crate) export_directory: String,
    /// Start date for export range (YYYY-MM-DD).
    pub(crate) export_from: String,
    /// End date for export range (YYYY-MM-DD).
    pub(crate) export_to: String,
    /// Generated PDF waiting for the user to confirm its file name and save it.
    pub(crate) export_preview: Option<ExportPreview>,
    /// Category names whose detail tables are expanded in the daily report.
    pub(crate) expanded_categories: BTreeSet<String>,
    /// Whether the read-only order display is open on the second screen.
    pub(crate) show_order_viewport: bool,
    /// Whether the read-only order-history dialog is open.
    pub(crate) show_previous_order: bool,
    /// Index of the receipt currently shown in the order-history dialog.
    pub(crate) history_receipt_index: usize,
    /// Whether the protected menu-access dialog is open.
    pub(crate) show_menu_password: bool,
    /// Requests keyboard focus when the menu-access dialog first appears.
    pub(crate) menu_password_needs_focus: bool,
    /// Password currently entered in the menu-access dialog.
    pub(crate) menu_password: String,
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

/// Generated PDF and the report data needed to show it before saving.
pub(crate) struct ExportPreview {
    /// Complete PDF document held in memory until the user confirms the file name.
    pub(crate) pdf: Vec<u8>,
    /// Editable destination file name.
    pub(crate) file_name: String,
    /// First local date included in the report.
    pub(crate) from: NaiveDate,
    /// Last local date included in the report.
    pub(crate) to: NaiveDate,
    /// Number of completed orders included in the report.
    pub(crate) order_count: usize,
    /// Sum of the included order totals.
    pub(crate) total: u64,
    /// Per-category, per-product quantities and subtotals.
    pub(crate) summary: BTreeMap<String, BTreeMap<String, (u64, u64)>>,
    /// Categories in the same order as the generated PDF.
    pub(crate) ordered_categories: Vec<String>,
}

/// Progress of the product-image preload shown on the startup splash screen.
pub(crate) struct Startup {
    /// Images that have not been decoded yet.
    pub(crate) pending: Vec<StartupImage>,
    /// Total number of unique configured images.
    pub(crate) total: usize,
    /// Number of images processed so far.
    pub(crate) loaded: usize,
    /// Product name currently being decoded, shown on the splash screen.
    pub(crate) current: Option<String>,
}

/// One product image queued for decoding during startup.
pub(crate) struct StartupImage {
    /// Filesystem path to the image.
    pub(crate) path: String,
    /// Product name displayed while the image is loaded.
    pub(crate) name: String,
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
        let (categories, mut colors, products, export_directory, menu_bad) = match menu {
            Some(m) if m.products.iter().all(|p| p.category < m.categories.len()) => (
                m.categories,
                m.colors,
                m.products,
                m.export_directory,
                false,
            ),
            Some(_) => (
                vec!["Gaufres".into(), "Boissons".into()],
                Vec::new(),
                Vec::new(),
                String::new(),
                true,
            ),
            None if menu_exists => (
                vec!["Gaufres".into(), "Boissons".into()],
                Vec::new(),
                Vec::new(),
                String::new(),
                true,
            ),
            None => (
                vec!["Gaufres".into(), "Boissons".into()],
                Vec::new(),
                Vec::new(),
                String::new(),
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
        let today = Local::now().date_naive();
        let export_from = today
            .with_day(1)
            .unwrap_or(today)
            .format("%Y-%m-%d")
            .to_string();
        let export_to = today.format("%Y-%m-%d").to_string();
        let mut app = Self {
            menu_path,
            orders_path,
            categories,
            colors,
            products,
            product_images: HashMap::new(),
            product_image_corners: HashMap::new(),
            product_image_stamps: HashMap::new(),
            startup: None,
            menu_bad,
            receipts,
            history_bad,
            order: Vec::new(),
            selected_line: None,
            selected: 0,
            screen: Screen::Order,
            new_category: String::new(),
            new_name: String::new(),
            new_price: String::new(),
            new_image_path: String::new(),
            new_category_index: 0,
            new_topping: false,
            edit_product: None,
            edit_name: String::new(),
            edit_price: String::new(),
            edit_image_path: String::new(),
            edit_category: 0,
            edit_topping: false,
            export_directory,
            export_from,
            export_to,
            export_preview: None,
            expanded_categories: BTreeSet::new(),
            show_order_viewport: false,
            show_previous_order: false,
            history_receipt_index: 0,
            show_menu_password: false,
            menu_password_needs_focus: false,
            menu_password: String::new(),
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
    /// Moves a product one position within its category and persists the new menu order.
    pub(crate) fn move_product(&mut self, index: usize, move_up: bool) {
        let Some(product) = self.products.get(index) else {
            return;
        };
        let category = product.category;
        let target = if move_up {
            (0..index)
                .rev()
                .find(|&candidate| self.products[candidate].category == category)
        } else {
            (index + 1..self.products.len())
                .find(|&candidate| self.products[candidate].category == category)
        };
        if let Some(target) = target {
            self.products.swap(index, target);
            if self.edit_product == Some(index) {
                self.edit_product = Some(target);
            } else if self.edit_product == Some(target) {
                self.edit_product = Some(index);
            }
            self.save_menu();
        }
    }

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
            match attach_topping_to_selected(&mut self.order, item, category, self.selected_line) {
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
                if let Some(index) = self.order.iter().position(|line| {
                    line.category_index == category
                        && line.name == item.name
                        && line.price == item.price
                        && line.toppings.is_empty()
                }) {
                    self.order[index].quantity = self.order[index].quantity.saturating_add(1);
                    self.selected_line = Some(index);
                    self.message = None;
                    return;
                }
            }
            self.order.push(item);
            self.selected_line = Some(self.order.len() - 1);
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
                self.selected_line = None;
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
                        image_path: p.image_path.clone(),
                    })
                    .collect(),
                export_directory: self.export_directory.clone(),
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

    /// Returns orders completed in the inclusive local-date range.
    pub(crate) fn orders_in_range(&self, from: NaiveDate, to: NaiveDate) -> Vec<Receipt> {
        self.receipts
            .iter()
            .filter(|receipt| {
                Local
                    .timestamp_opt(receipt.completed_at, 0)
                    .single()
                    .is_some_and(|dt| {
                        let date = dt.date_naive();
                        date >= from && date <= to
                    })
            })
            .cloned()
            .collect()
    }

    /// Generates the PDF for the configured date range and opens its preview.
    ///
    /// Nothing is written to disk until [`Self::save_export_preview`] is called from the preview.
    pub(crate) fn generate_export_preview(&mut self) {
        let from = match NaiveDate::parse_from_str(self.export_from.trim(), "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                self.message = Some("Date de début invalide (format attendu : AAAA-MM-JJ).".into());
                return;
            }
        };
        let to = match NaiveDate::parse_from_str(self.export_to.trim(), "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                self.message = Some("Date de fin invalide (format attendu : AAAA-MM-JJ).".into());
                return;
            }
        };
        if from > to {
            self.message = Some("La date de début doit précéder la date de fin.".into());
            return;
        }
        let orders = self.orders_in_range(from, to);
        if orders.is_empty() {
            self.message = Some("Aucune commande dans la période sélectionnée.".into());
            return;
        }
        let summary = self.summarize_orders(&orders);
        let ordered_categories = ordered_summary_categories(&summary, &self.categories);
        let total = orders.iter().map(|order| order.total).sum();
        match generate_export_pdf(from, to, &orders, &summary, &self.categories, &self.colors) {
            Ok(pdf) => {
                self.export_preview = Some(ExportPreview {
                    pdf,
                    file_name: default_export_file_name(from, to),
                    from,
                    to,
                    order_count: orders.len(),
                    total,
                    summary,
                    ordered_categories,
                });
                self.message = None;
            }
            Err(error) => {
                self.message = Some(format!("Échec de la génération du PDF : {error}"));
            }
        }
    }

    /// Saves the generated PDF to the configured export folder.
    ///
    /// Returns `true` when the file was written and the preview can be closed.
    pub(crate) fn save_export_preview(&mut self, preview: &ExportPreview) -> bool {
        let directory = self.export_directory.trim();
        if directory.is_empty() {
            self.message =
                Some("Configurez le dossier d’export dans la fenêtre de configuration.".into());
            return false;
        }
        let Some(file_name) = normalize_export_file_name(&preview.file_name) else {
            self.message = Some("Le nom du fichier PDF est invalide.".into());
            return false;
        };
        let path = unique_export_path(std::path::Path::new(directory), &file_name);
        match fs::write(&path, &preview.pdf) {
            Ok(()) => {
                self.message = Some(format!(
                    "{} commandes exportées vers {}",
                    preview.order_count,
                    path.display()
                ));
                true
            }
            Err(error) => {
                self.message = Some(format!("Échec de l’enregistrement du PDF : {error}"));
                false
            }
        }
    }

    /// Builds a per-category, per-product sales summary for the given orders.
    fn summarize_orders(
        &self,
        orders: &[Receipt],
    ) -> BTreeMap<String, BTreeMap<String, (u64, u64)>> {
        let mut summary: BTreeMap<String, BTreeMap<String, (u64, u64)>> = BTreeMap::new();
        fn add_item(
            item: &Item,
            categories: &[String],
            summary: &mut BTreeMap<String, BTreeMap<String, (u64, u64)>>,
        ) {
            let recorded = if item.category.trim().is_empty() {
                "Non catégorisé"
            } else {
                item.category.trim()
            };
            let category = canonical_category(recorded, categories);
            let entry = summary
                .entry(category)
                .or_default()
                .entry(item.name.clone())
                .or_insert((0, 0));
            entry.0 = entry.0.saturating_add(u64::from(item.quantity));
            entry.1 = entry
                .1
                .saturating_add(item.price.saturating_mul(u64::from(item.quantity)));
            for topping in &item.toppings {
                add_item(topping, categories, summary);
            }
        }
        for order in orders {
            for item in &order.items {
                add_item(item, &self.categories, &mut summary);
            }
        }
        summary
    }
}

/// Writes bytes to a neighboring temporary file and renames it into place.
///
/// Keeping the temporary file beside the destination ensures the rename stays on
/// the same filesystem, making replacement atomic on platforms that support it.
fn write_json_atomically(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let temp_path = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = fs::OpenOptions::new()
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

fn generate_export_pdf(
    from: NaiveDate,
    to: NaiveDate,
    orders: &[Receipt],
    summary: &BTreeMap<String, BTreeMap<String, (u64, u64)>>,
    categories: &[String],
    colors: &[[u8; 3]],
) -> std::io::Result<Vec<u8>> {
    use printpdf::*;

    let (doc, page1, layer1) = PdfDocument::new("ECR Export", Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(std::io::Error::other)?;
    let bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(std::io::Error::other)?;

    let mut page = page1;
    let mut layer = layer1;
    let mut y = 282.0_f32;
    let left = 12.0_f32;
    let bottom = 15.0_f32;

    let mut current_layer = doc.get_page(page).get_layer(layer);
    current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    current_layer.use_text("ECR — Résumé des ventes", 18.0, Mm(left), Mm(y), &bold);
    y -= 8.0;
    let period = if from == to {
        format!("Période : {}", from.format("%d/%m/%Y"))
    } else {
        format!(
            "Période : {} au {}",
            from.format("%d/%m/%Y"),
            to.format("%d/%m/%Y")
        )
    };
    current_layer.use_text(period, 11.0, Mm(left), Mm(y), &font);
    y -= 6.0;
    let total_sales: u64 = orders.iter().map(|o| o.total).sum();
    current_layer.use_text(
        format!(
            "Commandes : {}    Total des ventes : {}",
            orders.len(),
            money(total_sales)
        ),
        11.0,
        Mm(left),
        Mm(y),
        &bold,
    );
    y -= 10.0;

    let ordered = ordered_summary_categories(summary, categories);
    let ensure_space = |y: &mut f32,
                        needed: f32,
                        doc: &PdfDocumentReference,
                        page: &mut PdfPageIndex,
                        layer: &mut PdfLayerIndex|
     -> PdfLayerReference {
        if *y < bottom + needed {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
            *page = p;
            *layer = l;
            *y = 282.0;
        }
        doc.get_page(*page).get_layer(*layer)
    };

    for category in ordered {
        let Some(products) = summary.get(&category) else {
            continue;
        };
        current_layer = ensure_space(&mut y, 12.0, &doc, &mut page, &mut layer);
        let color = categories
            .iter()
            .position(|c| c.eq_ignore_ascii_case(category.trim()))
            .and_then(|i| colors.get(i).copied())
            .unwrap_or([80, 80, 80]);
        // Use the category's defined color for its header.
        current_layer.set_fill_color(Color::Rgb(Rgb::new(
            f32::from(color[0]) / 255.0,
            f32::from(color[1]) / 255.0,
            f32::from(color[2]) / 255.0,
            None,
        )));
        let cat_total: u64 = products.values().map(|(_, sum)| *sum).sum();
        current_layer.use_text(
            format!("{}  (Total: {})", category, money(cat_total)),
            12.0,
            Mm(left),
            Mm(y),
            &bold,
        );
        y -= 7.5;

        current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
        current_layer.use_text("Produit", 9.5, Mm(left + 1.0), Mm(y), &bold);
        current_layer.use_text("Qté", 9.5, Mm(145.0), Mm(y), &bold);
        current_layer.use_text("Somme", 9.5, Mm(165.0), Mm(y), &bold);
        y -= 5.5;

        for (name, (qty, sum)) in products {
            current_layer = ensure_space(&mut y, 7.0, &doc, &mut page, &mut layer);
            current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
            let display_name: String = if name.chars().count() > 52 {
                let truncated: String = name.chars().take(49).collect();
                format!("{truncated}...")
            } else {
                name.clone()
            };
            current_layer.use_text(display_name, 9.5, Mm(left + 1.0), Mm(y), &font);
            current_layer.use_text(qty.to_string(), 9.5, Mm(145.0), Mm(y), &font);
            current_layer.use_text(money(*sum), 9.5, Mm(165.0), Mm(y), &font);
            y -= 5.0;
        }
        y -= 4.0;
    }

    doc.save_to_bytes().map_err(std::io::Error::other)
}

/// Builds the default PDF file name for an export range.
fn default_export_file_name(from: NaiveDate, to: NaiveDate) -> String {
    if from == to {
        format!("ecr-export-{}.pdf", from.format("%Y-%m-%d"))
    } else {
        format!(
            "ecr-export-{}_{}.pdf",
            from.format("%Y-%m-%d"),
            to.format("%Y-%m-%d")
        )
    }
}

/// Returns a safe PDF file name derived from the editable preview field.
fn normalize_export_file_name(input: &str) -> Option<String> {
    let name = std::path::Path::new(input.trim())
        .file_name()?
        .to_str()?
        .trim();
    if name.is_empty() || name.eq_ignore_ascii_case(".pdf") {
        return None;
    }
    let mut name: String = name
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '-'
            } else {
                character
            }
        })
        .collect();
    if name.to_ascii_lowercase().ends_with(".pdf") {
        name.truncate(name.len() - 4);
    }
    name.push_str(".pdf");
    Some(name)
}

/// Adds a numeric suffix when the requested export file already exists.
fn unique_export_path(directory: &std::path::Path, file_name: &str) -> PathBuf {
    let path = directory.join(file_name);
    if !path.exists() {
        return path;
    }
    let stem = file_name.strip_suffix(".pdf").unwrap_or(file_name);
    let mut counter = 2;
    loop {
        let candidate = directory.join(format!("{stem}-{counter}.pdf"));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Orders summary categories by the configured menu order, then appends legacy names.
fn ordered_summary_categories(
    summary: &BTreeMap<String, BTreeMap<String, (u64, u64)>>,
    categories: &[String],
) -> Vec<String> {
    let mut ordered = Vec::new();
    for category in categories {
        if let Some((name, _)) = summary
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(category.trim()))
        {
            if !ordered
                .iter()
                .any(|known: &String| known.eq_ignore_ascii_case(name))
            {
                ordered.push(name.clone());
            }
        }
    }
    for name in summary.keys() {
        if !ordered.iter().any(|known| known.eq_ignore_ascii_case(name)) {
            ordered.push(name.clone());
        }
    }
    ordered
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
        Box::new(|_| {
            let mut app = App::default();
            app.begin_startup();
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms the export path now returns PDF bytes without touching the filesystem.
    #[test]
    fn export_pdf_is_generated_in_memory() {
        let from = NaiveDate::from_ymd_opt(2026, 11, 1).expect("valid start date");
        let to = NaiveDate::from_ymd_opt(2026, 11, 30).expect("valid end date");
        let summary = BTreeMap::new();
        let pdf = generate_export_pdf(
            from,
            to,
            &[],
            &summary,
            &["Gaufres".into()],
            &[[210, 232, 211]],
        )
        .expect("PDF generation should succeed");
        assert!(pdf.starts_with(b"%PDF-"));
    }

    /// Ensures preview file names cannot escape the export folder or lose their PDF extension.
    #[test]
    fn export_file_names_are_sanitized_and_keep_pdf_extension() {
        assert_eq!(
            normalize_export_file_name(" bilan.pdf "),
            Some("bilan.pdf".into())
        );
        assert_eq!(
            normalize_export_file_name("bilan"),
            Some("bilan.pdf".into())
        );
        assert_eq!(
            normalize_export_file_name("../secret.pdf"),
            Some("secret.pdf".into())
        );
        assert_eq!(
            normalize_export_file_name("a:b.pdf"),
            Some("a-b.pdf".into())
        );
        assert_eq!(normalize_export_file_name(""), None);
    }
}
