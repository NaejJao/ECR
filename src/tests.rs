//! Regression tests for menu compatibility, category reporting, and topping rules.

use crate::{domain::*, model::*};
#[test]
fn zero_price_is_valid_but_negative_price_is_rejected() {
    assert_eq!(parse_price("0"), Some(0));
    assert_eq!(parse_price("0,00"), Some(0));
    assert_eq!(parse_price("-0,50"), None);
}

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
