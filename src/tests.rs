//! Regression tests for menu compatibility, category reporting, and topping rules.

use crate::{domain::*, model::*};
/// Confirms complimentary products remain valid while malformed prices are rejected.
#[test]
fn zero_price_is_valid_but_malformed_prices_are_rejected() {
    assert_eq!(parse_price("0"), Some(0));
    assert_eq!(parse_price("0,00"), Some(0));
    assert_eq!(parse_price("4.5"), Some(450));
    assert_eq!(parse_price("4,50"), Some(450));
    assert_eq!(parse_price("-0,50"), None);
    assert_eq!(parse_price("4."), None);
    assert_eq!(parse_price("4,"), None);
    assert_eq!(parse_price(".5"), None);
    assert_eq!(parse_price("4..5"), None);
}

/// Ensures legacy menu fields load and absent optional image paths default safely.
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
    assert_eq!(menu.products[0].image_path, None);
    assert!(menu.products[1].topping);
    assert_eq!(menu.colors, [[210, 232, 211]]);
}

/// Verifies historical category spelling is reconciled only when the match is unambiguous.
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

/// Ensures repeated taps cannot attach the same topping twice to one base item.
#[test]
fn repeated_identical_topping_is_ignored_on_the_same_base_product() {
    let mut order = vec![Item {
        name: "Gaufre".into(),
        price: 500,
        quantity: 1,
        category: "Gaufres".into(),
        category_index: 0,
        theme: theme(0),
        toppings: Vec::new(),
    }];
    for _ in 0..2 {
        attach_topping(
            &mut order,
            Item {
                name: "Nutella".into(),
                price: 100,
                quantity: 1,
                category: "Gaufres".into(),
                category_index: 0,
                theme: theme(0),
                toppings: Vec::new(),
            },
            0,
        );
    }
    assert_eq!(order[0].toppings.len(), 1);
    assert_eq!(order[0].total(), 600);
}

/// Ensures a topping can be attached to an earlier base item selected by the cashier.
#[test]
fn topping_attaches_to_explicitly_selected_earlier_base_item() {
    let mut order = vec![
        base_item("Gaufre 1", "Gaufres", 0),
        base_item("Gaufre 2", "Gaufres", 0),
    ];
    let result = attach_topping_to_selected(
        &mut order,
        topping_item("Nutella", "Gaufres", 0),
        0,
        Some(0),
    );
    assert_eq!(result, Some(true));
    assert_eq!(order[0].toppings.len(), 1);
    assert!(order[1].toppings.is_empty());
}

/// Falls back to the latest base item when the selection is stale or in another category.
#[test]
fn invalid_selection_falls_back_to_latest_base_item() {
    let mut order = vec![
        base_item("Gaufre", "Gaufres", 0),
        base_item("Boisson", "Boissons", 1),
    ];
    let result = attach_topping_to_selected(
        &mut order,
        topping_item("Nutella", "Gaufres", 0),
        0,
        Some(1),
    );
    assert_eq!(result, Some(true));
    assert_eq!(order[0].toppings.len(), 1);
    assert!(order[1].toppings.is_empty());

    let mut order = vec![base_item("Gaufre 1", "Gaufres", 0)];
    assert_eq!(
        attach_topping_to_selected(
            &mut order,
            topping_item("Nutella", "Gaufres", 0),
            0,
            Some(9),
        ),
        Some(true)
    );
    assert_eq!(order[0].toppings.len(), 1);
}

/// Ensures selecting the same base item twice still cannot duplicate a topping.
#[test]
fn selected_base_item_keeps_toppings_unique() {
    let mut order = vec![
        base_item("Gaufre 1", "Gaufres", 0),
        base_item("Gaufre 2", "Gaufres", 0),
    ];
    for _ in 0..2 {
        attach_topping_to_selected(
            &mut order,
            topping_item("Nutella", "Gaufres", 0),
            0,
            Some(0),
        );
    }
    assert_eq!(order[0].toppings.len(), 1);
    assert!(order[1].toppings.is_empty());
}

/// Builds a base order line for the topping-selection tests.
fn base_item(name: &str, category: &str, category_index: usize) -> Item {
    Item {
        name: name.into(),
        price: 500,
        quantity: 1,
        category: category.into(),
        category_index,
        theme: theme(category_index),
        toppings: Vec::new(),
    }
}

/// Builds a topping order line for the topping-selection tests.
fn topping_item(name: &str, category: &str, category_index: usize) -> Item {
    Item {
        name: name.into(),
        price: 100,
        quantity: 1,
        category: category.into(),
        category_index,
        theme: theme(category_index),
        toppings: Vec::new(),
    }
}

/// Checks toppings affect revenue totals but not main-product category quantities.
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
