//! Startup splash screen shown while configured product images are decoded.

use crate::app::{App, Startup, StartupImage};
use eframe::egui;

impl App {
    /// Queues every configured product image for decoding behind the splash screen.
    pub(crate) fn begin_startup(&mut self) {
        let mut items: Vec<StartupImage> = self
            .products
            .iter()
            .filter_map(|product| {
                product.image_path.clone().map(|path| StartupImage {
                    path,
                    name: product.name.clone(),
                })
            })
            .collect();
        items.sort_unstable_by(|left, right| left.path.cmp(&right.path));
        items.dedup_by(|left, right| left.path == right.path);
        self.startup = (!items.is_empty()).then_some(Startup {
            total: items.len(),
            pending: items,
            loaded: 0,
            current: None,
        });
    }

    /// Decodes one queued image per frame and closes the splash when none remain.
    pub(crate) fn advance_startup(&mut self, ctx: &egui::Context) {
        let item = self.startup.as_mut().and_then(|startup| {
            let item = startup.pending.pop()?;
            startup.loaded += 1;
            startup.current = Some(item.name.clone());
            Some(item)
        });
        let Some(item) = item else {
            self.startup = None;
            return;
        };
        self.product_texture(ctx, &item.path);
        if self.startup.as_ref().is_some_and(|s| s.pending.is_empty()) {
            self.startup = None;
        } else {
            ctx.request_repaint();
        }
    }

    /// Draws the full-window splash with the current image-loading progress.
    pub(crate) fn splash_ui(&mut self, ctx: &egui::Context) {
        let Some(startup) = self.startup.as_ref() else {
            return;
        };
        let progress = startup.loaded as f32 / startup.total as f32;
        let progress_text = format!(
            "Chargement des produits… {}/{}",
            startup.loaded, startup.total
        );
        let current = startup.current.clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(35, 73, 53)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.32);
                    ui.label(
                        egui::RichText::new("ECR")
                            .size(28.0)
                            .strong()
                            .color(egui::Color32::from_rgb(192, 220, 195)),
                    );
                    ui.label(
                        egui::RichText::new("WaffleTime")
                            .size(52.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(30.0);
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .desired_width(380.0)
                            .text(progress_text),
                    );
                    if let Some(name) = current {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(name)
                                .size(20.0)
                                .color(egui::Color32::WHITE),
                        );
                    }
                    ui.add_space(14.0);
                    ui.add(egui::Spinner::new().color(egui::Color32::from_rgb(192, 220, 195)));
                });
            });
    }
}
