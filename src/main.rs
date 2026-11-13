//! Binary entry point for the WaffleTime electronic cash registry.
//!
//! Runtime state, business rules, data models, and each screen live in dedicated modules.
//! The same binary also runs the standalone second-screen order display.

mod app;
mod display;
mod domain;
mod model;
mod ui;

#[cfg(test)]
mod tests;

/// Starts the cashier window, or the standalone display when requested by the cashier.
fn main() -> eframe::Result {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some("--order-display") {
        if let (Some(snapshot), Some(heartbeat)) = (arguments.next(), arguments.next()) {
            return display::run(snapshot.into(), heartbeat.into());
        }
    }
    app::run()
}
