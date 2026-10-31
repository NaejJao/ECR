//! Binary entry point for the WaffleTime electronic cash registry.
//!
//! Runtime state, business rules, data models, and each screen live in dedicated modules.

mod app;
mod domain;
mod model;
mod ui;

#[cfg(test)]
mod tests;

fn main() -> eframe::Result {
    app::run()
}
