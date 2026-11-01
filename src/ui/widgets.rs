//! Shared visual primitives used by the order, menu, and daily-report screens.

use eframe::egui;

/// Blends a category color with white to distinguish topping buttons.
pub(crate) fn lighten(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(
        ((u16::from(c[0]) + 255) / 2) as u8,
        ((u16::from(c[1]) + 255) / 2) as u8,
        ((u16::from(c[2]) + 255) / 2) as u8,
    )
}

/// Creates the shared pale background and touch-friendly page margins.
pub(crate) fn page_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(247, 247, 243))
        .inner_margin(egui::Margin::symmetric(24, 18))
}

/// Creates the shared white card surface used for grouped controls and data.
pub(crate) fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(232, 234, 228),
        ))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(12))
}

/// Draws the consistent eyebrow, title, and subtitle heading used by report pages.
pub(crate) fn page_heading(ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(eyebrow)
            .size(12.0)
            .strong()
            .color(egui::Color32::from_rgb(102, 117, 103)),
    );
    ui.heading(egui::RichText::new(title).size(28.0).strong());
    ui.label(subtitle);
}

/// Builds a high-contrast navigation button sized for touch input.
pub(crate) fn header_button(s: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(s).strong().color(egui::Color32::WHITE))
        .fill(egui::Color32::from_rgb(57, 99, 72))
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(10))
        .min_size([150.0, 44.0].into())
}

/// Builds a fixed-size reorder control with a vector-drawn arrow and disabled state.
pub(crate) fn reorder_arrow_button(
    ui: &mut egui::Ui,
    points_up: bool,
    enabled: bool,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size([40.0, 40.0].into(), egui::Sense::click());
        let visuals = ui.style().interact(&response);
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(8), visuals.bg_fill);
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(8),
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let center = rect.center();
        let stroke = egui::Stroke::new(2.5_f32, visuals.fg_stroke.color);
        let (tip_y, tail_y) = if points_up {
            (center.y - 7.0, center.y + 7.0)
        } else {
            (center.y + 7.0, center.y - 7.0)
        };
        ui.painter().line_segment(
            [egui::pos2(center.x, tail_y), egui::pos2(center.x, tip_y)],
            stroke,
        );
        ui.painter().line_segment(
            [
                egui::pos2(center.x - 6.0, tip_y + if points_up { 6.0 } else { -6.0 }),
                egui::pos2(center.x, tip_y),
            ],
            stroke,
        );
        ui.painter().line_segment(
            [
                egui::pos2(center.x + 6.0, tip_y + if points_up { 6.0 } else { -6.0 }),
                egui::pos2(center.x, tip_y),
            ],
            stroke,
        );
        response.on_hover_text(if points_up {
            "Monter le produit"
        } else {
            "Descendre le produit"
        })
    })
    .inner
}

/// Builds the green primary action button with a caller-selected minimum size.
pub(crate) fn primary_button(s: &str, size: [f32; 2]) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(s).strong().color(egui::Color32::WHITE))
        .fill(egui::Color32::from_rgb(49, 104, 70))
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(12))
        .min_size(size.into())
}

/// Displays a labeled, prominent metric in the daily overview.
pub(crate) fn overview_card(ui: &mut egui::Ui, label: &str, value: String) {
    card().inner_margin(egui::Margin::same(20)).show(ui, |ui| {
        ui.set_min_width(240.0);
        ui.label(label);
        ui.label(
            egui::RichText::new(value)
                .size(28.0)
                .strong()
                .color(egui::Color32::from_rgb(35, 73, 53)),
        );
    });
}
