# ECR — Snack Shop Order Register

A touch-friendly Rust desktop app for taking waffle and drink orders at a small snack shop.

## Run

```sh
cargo run
cargo test
```

Use **Manage menu** to configure categories, category colors, products, and topping flags. Tap a main product to add an individual item, then tap topping products to nest them under the latest main product in that category. In the daily overview, order count and total sale share one row, followed by one main-item count per category. Toppings are excluded from category item counts but included in order totals.

The menu and completed order history are persisted as JSON under the operating system's app data directory. Unfinished orders are not saved. The daily overview can export full order details as a dated JSON file to a USB drive's mounted folder. The export does not overwrite an existing same-day file.

This is a starter, not a complete payment or accounting system. Add backups and any jurisdiction-specific recordkeeping requirements before relying on it for business records.
