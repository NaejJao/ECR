//! Types persisted by the menu and order-history JSON files.
//!
//! Prices are stored as integer minor currency units (centimes). The serde aliases
//! keep older menu and receipt files readable when field names have changed.

use crate::domain::default_theme;
use serde::{Deserialize, Serialize};

/// A product configured in the menu.
#[derive(Serialize, Deserialize)]
pub(crate) struct Product {
    /// Name shown on the product-selection button.
    pub(crate) name: String,
    /// Unit price in centimes.
    #[serde(alias = "price_minor")]
    pub(crate) price: u64,
    /// Index into [`MenuData::categories`].
    pub(crate) category: usize,
    /// Whether this product is an add-on attached to a base product.
    #[serde(default, alias = "is_topping")]
    pub(crate) topping: bool,
    /// Optional filesystem path to the product button image.
    #[serde(default)]
    pub(crate) image_path: Option<String>,
}

/// A product line recorded in an open or completed order.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Item {
    /// Product name captured when the item was added.
    pub(crate) name: String,
    /// Unit price in centimes captured at sale time.
    #[serde(alias = "price_minor")]
    pub(crate) price: u64,
    /// Number of identical units represented by this line.
    pub(crate) quantity: u32,
    /// Category name snapshot, retained for historical reports after menu edits.
    #[serde(default)]
    pub(crate) category: String,
    /// Current menu category index used to associate toppings during an open sale.
    #[serde(default)]
    pub(crate) category_index: usize,
    /// Category color snapshot used to render historical order lines consistently.
    #[serde(default = "default_theme")]
    pub(crate) theme: [u8; 3],
    /// Add-ons attached to this base item; each configured topping may appear once.
    #[serde(default)]
    pub(crate) toppings: Vec<Item>,
}

/// A completed sale with its completion time, item snapshots, and charged total.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Receipt {
    /// Unix timestamp in seconds when the order was completed.
    pub(crate) completed_at: i64,
    /// Base products and their nested toppings.
    #[serde(alias = "lines")]
    pub(crate) items: Vec<Item>,
    /// Total charged, in centimes, as recorded when the order was completed.
    #[serde(alias = "total_minor")]
    pub(crate) total: u64,
}

/// The complete configurable menu written to `menu.json`.
#[derive(Serialize, Deserialize)]
pub(crate) struct MenuData {
    /// Category names; product category fields refer to this list by index.
    pub(crate) categories: Vec<String>,
    /// One RGB theme per category. Missing legacy colors are filled at load time.
    #[serde(default, alias = "category_colors")]
    pub(crate) colors: Vec<[u8; 3]>,
    /// Products belonging to the configured categories.
    pub(crate) products: Vec<Product>,
    /// Default destination folder for PDF exports.
    #[serde(default)]
    pub(crate) export_directory: String,
}

/// Wrapper for the receipt collection stored in `orders.json`.
#[derive(Serialize, Deserialize)]
pub(crate) struct History {
    /// Completed orders in completion order; the last entry is the latest sale.
    pub(crate) orders: Vec<Receipt>,
}

/// Shape of the end-of-day JSON export written to the selected USB directory.
#[derive(Serialize)]
#[allow(dead_code)]
pub(crate) struct Export<'a> {
    /// Business date represented as an ISO `YYYY-MM-DD` string.
    pub(crate) business_date: String,
    /// Full order snapshots for that business date.
    pub(crate) orders: &'a [Receipt],
}

impl Item {
    /// Returns this line's price plus the prices of all nested toppings, in centimes.
    pub(crate) fn total(&self) -> u64 {
        self.toppings.iter().fold(
            self.price.saturating_mul(u64::from(self.quantity)),
            |sum, topping| sum.saturating_add(topping.total()),
        )
    }

    /// Updates category indices after a menu category is removed.
    ///
    /// The removed category becomes `usize::MAX` so existing order snapshots are
    /// not accidentally reassigned to a different category. Later indices shift down.
    pub(crate) fn remap_category(&mut self, removed_index: usize) {
        if self.category_index == removed_index {
            self.category_index = usize::MAX;
        } else if self.category_index > removed_index {
            self.category_index -= 1;
        }
        for topping in &mut self.toppings {
            topping.remap_category(removed_index);
        }
    }
}
