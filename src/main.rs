mod app;
mod domain;
mod model;
mod ui;

#[cfg(test)]
mod tests;

fn main() -> eframe::Result {
    app::run()
}
