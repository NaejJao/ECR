use crate::domain::default_theme;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Product {
    pub(crate) name: String,
    #[serde(alias = "price_minor")]
    pub(crate) price: u64,
    pub(crate) category: usize,
    #[serde(default, alias = "is_topping")]
    pub(crate) topping: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Item {
    pub(crate) name: String,
    #[serde(alias = "price_minor")]
    pub(crate) price: u64,
    pub(crate) quantity: u32,
    #[serde(default)]
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) category_index: usize,
    #[serde(default = "default_theme")]
    pub(crate) theme: [u8; 3],
    #[serde(default)]
    pub(crate) toppings: Vec<Item>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Receipt {
    pub(crate) completed_at: i64,
    #[serde(alias = "lines")]
    pub(crate) items: Vec<Item>,
    #[serde(alias = "total_minor")]
    pub(crate) total: u64,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct MenuData {
    pub(crate) categories: Vec<String>,
    #[serde(default, alias = "category_colors")]
    pub(crate) colors: Vec<[u8; 3]>,
    pub(crate) products: Vec<Product>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct History {
    pub(crate) orders: Vec<Receipt>,
}
#[derive(Serialize)]
pub(crate) struct Export<'a> {
    pub(crate) business_date: String,
    pub(crate) orders: &'a [Receipt],
}

impl Item {
    pub(crate) fn total(&self) -> u64 {
        self.toppings.iter().fold(
            self.price.saturating_mul(u64::from(self.quantity)),
            |s, x| s.saturating_add(x.total()),
        )
    }
    pub(crate) fn remap_category(&mut self, i: usize) {
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
