# ECR — Snack Shop Order Register

A touch-friendly Rust desktop app for taking waffle and drink orders at a small snack shop.

## Run

```sh
cargo run
cargo test
```

Use **Manage menu** to configure categories, category colors, products, and topping flags. Menu access is protected by a convenience password embedded in the application (not a security boundary). Tap a main product to add an individual item, then tap topping products to nest them under the selected main product in that category (the latest one by default). Tap an earlier item in the current order to select it, change its toppings, or remove them individually. The monitor button in the header toggles a read-only order display in a second native window, laid out in two half-width columns for a customer-facing screen; press it again to close that display. It refreshes only when the current order changes and returns focus to the main window. In the daily overview, order count and total sale share one row, followed by one main-item count per category. Toppings are excluded from category item counts but included in order totals.

The menu and completed order history are persisted as JSON under the operating system's app data directory. Unfinished orders are not saved. The export folder is configured in **Configuration** and saved with the menu. The daily overview can generate a sales summary for a configurable date range (default: the current month). The generated PDF is shown in an in-app preview where the file name can be edited before saving it to that folder. The PDF groups products by category (using category colors) and lists, for each product, the quantity sold and the total amount. If a file with the same name already exists, saving adds a numeric suffix instead of overwriting it.

This is a starter, not a complete payment or accounting system. Add backups and any jurisdiction-specific recordkeeping requirements before relying on it for business records.
