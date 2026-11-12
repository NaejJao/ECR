//! Shared visual primitives used by the order, menu, and daily-report screens.

use chrono::{Datelike, Local, NaiveDate};
use eframe::egui;

/// French month names used by the compact date picker.
const MONTH_NAMES: [&str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];

/// Returns the first day of the month containing `date`.
fn first_of_month(date: NaiveDate) -> NaiveDate {
    date.with_day(1).unwrap_or(date)
}

/// Extends egui responses with activation on pointer or keyboard press.
pub(crate) trait TouchResponseExt {
    /// Returns true on the frame a primary pointer or activation key is pressed.
    fn pressed(&self, ui: &egui::Ui) -> bool;
}

impl TouchResponseExt for egui::Response {
    /// Checks the event position directly so activation occurs on the first press frame.
    fn pressed(&self, ui: &egui::Ui) -> bool {
        let pointer_pressed = ui.input(|input| {
            input.pointer.button_pressed(egui::PointerButton::Primary)
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|position| self.rect.contains(position))
        });
        let keyboard_pressed = self.has_focus()
            && ui.input(|input| {
                input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space)
            });
        pointer_pressed || keyboard_pressed
    }
}

/// Blends a category color with white to distinguish topping buttons.
pub(crate) fn lighten(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(
        ((u16::from(c[0]) + 255) / 2) as u8,
        ((u16::from(c[1]) + 255) / 2) as u8,
        ((u16::from(c[2]) + 255) / 2) as u8,
    )
}

/// Builds a category selector filled with its category color.
///
/// The selected category keeps a dark outline so the active filter remains visible even though
/// every button now carries its own color.
pub(crate) fn category_button(
    text: egui::RichText,
    color: [u8; 3],
    selected: bool,
) -> egui::Button<'static> {
    let stroke = if selected {
        egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(35, 73, 53))
    } else {
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(225, 228, 221))
    };
    egui::Button::new(text.strong().color(egui::Color32::from_rgb(35, 73, 53)))
        .fill(egui::Color32::from_rgb(color[0], color[1], color[2]))
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(10))
}

/// Shows a date text field that opens a compact calendar popup when clicked.
///
/// The value stays in `AAAA-MM-JJ` form so it remains compatible with the export range parsing
/// and PDF validation.
pub(crate) fn date_input(
    ui: &mut egui::Ui,
    id_salt: &str,
    value: &mut String,
    hint: &str,
    width: f32,
) {
    let popup_id = ui.make_persistent_id(("date_input_popup", id_salt));
    let month_id = ui.make_persistent_id(("date_input_month", id_salt));
    let response = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(width)
            .margin(egui::Margin::symmetric(8, 10))
            .min_size(egui::vec2(0.0, 40.0))
            .hint_text(hint),
    );
    let selected = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok();
    let today = Local::now().date_naive();

    if response.clicked() {
        let month = selected
            .map(first_of_month)
            .unwrap_or_else(|| first_of_month(today));
        ui.ctx().data_mut(|data| data.insert_temp(month_id, month));
        ui.memory_mut(|memory| memory.open_popup(popup_id));
    }

    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(246.0);
            let mut month = ui.ctx().data_mut(|data| {
                *data.get_temp_mut_or_insert_with(month_id, || {
                    first_of_month(selected.unwrap_or(today))
                })
            });
            let mut chosen = None;

            ui.horizontal(|ui| {
                if ui.button("‹").clicked() {
                    month = month
                        .checked_sub_months(chrono::Months::new(1))
                        .unwrap_or(month);
                }
                ui.add_sized(
                    [150.0, 28.0],
                    egui::Label::new(
                        egui::RichText::new(format!(
                            "{} {}",
                            MONTH_NAMES[month.month0() as usize],
                            month.year()
                        ))
                        .strong(),
                    ),
                );
                if ui.button("›").clicked() {
                    month = month
                        .checked_add_months(chrono::Months::new(1))
                        .unwrap_or(month);
                }
            });
            ui.ctx().data_mut(|data| data.insert_temp(month_id, month));

            let first = first_of_month(month);
            let start =
                first - chrono::Duration::days(i64::from(first.weekday().num_days_from_monday()));
            egui::Grid::new(("date_input_grid", id_salt))
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    for label in ["L", "M", "M", "J", "V", "S", "D"] {
                        ui.add_sized(
                            [30.0, 20.0],
                            egui::Label::new(
                                egui::RichText::new(label)
                                    .small()
                                    .strong()
                                    .color(egui::Color32::from_rgb(102, 117, 103)),
                            ),
                        );
                    }
                    ui.end_row();

                    for week in 0..6 {
                        for weekday in 0..7 {
                            let date = start + chrono::Duration::days(week * 7 + weekday);
                            let in_month = date.month() == month.month();
                            let is_selected = selected == Some(date);
                            let is_today = date == today;
                            let text = if is_selected {
                                egui::RichText::new(date.day().to_string())
                                    .strong()
                                    .color(egui::Color32::WHITE)
                            } else if in_month {
                                egui::RichText::new(date.day().to_string())
                            } else {
                                egui::RichText::new(date.day().to_string())
                                    .color(egui::Color32::from_rgb(170, 178, 170))
                            };
                            let mut button = egui::Button::new(text)
                                .min_size([30.0, 30.0].into())
                                .corner_radius(egui::CornerRadius::same(6));
                            if is_selected {
                                button = button
                                    .fill(egui::Color32::from_rgb(49, 104, 70))
                                    .stroke(egui::Stroke::NONE);
                            } else if is_today {
                                button = button.stroke(egui::Stroke::new(
                                    1.5_f32,
                                    egui::Color32::from_rgb(49, 104, 70),
                                ));
                            }
                            if ui.add(button).clicked() {
                                chosen = Some(date);
                            }
                        }
                        ui.end_row();
                    }
                });

            if let Some(date) = chosen {
                *value = date.format("%Y-%m-%d").to_string();
                ui.memory_mut(|memory| memory.close_popup());
            }
        },
    );
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

/// Direction and shape of a compact, vector-drawn action icon.
#[derive(Clone, Copy)]
pub(crate) enum ActionIcon {
    /// Arrow directed to the left.
    Left,
    /// Arrow directed to the right.
    Right,
    /// Cross used for remove actions.
    Cross,
}

/// Builds a fixed-size icon button without relying on font glyph coverage.
pub(crate) fn action_icon_button(
    ui: &mut egui::Ui,
    icon: ActionIcon,
    enabled: bool,
    size: [f32; 2],
    tooltip: &'static str,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(size.into(), egui::Sense::click());
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
        match icon {
            ActionIcon::Left | ActionIcon::Right => {
                let direction = if matches!(icon, ActionIcon::Left) {
                    -1.0
                } else {
                    1.0
                };
                let tip = center + egui::vec2(direction * 7.0, 0.0);
                ui.painter()
                    .line_segment([center - egui::vec2(direction * 7.0, 0.0), tip], stroke);
                ui.painter()
                    .line_segment([tip + egui::vec2(-direction * 6.0, -6.0), tip], stroke);
                ui.painter()
                    .line_segment([tip + egui::vec2(-direction * 6.0, 6.0), tip], stroke);
            }
            ActionIcon::Cross => {
                ui.painter().line_segment(
                    [
                        center + egui::vec2(-5.0, -5.0),
                        center + egui::vec2(5.0, 5.0),
                    ],
                    stroke,
                );
                ui.painter().line_segment(
                    [
                        center + egui::vec2(5.0, -5.0),
                        center + egui::vec2(-5.0, 5.0),
                    ],
                    stroke,
                );
            }
        }
        response.on_hover_text(tooltip)
    })
    .inner
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

/// Displays a large, touch-readable labeled metric in the daily overview.
pub(crate) fn overview_card(ui: &mut egui::Ui, label: &str, value: String) {
    card().inner_margin(egui::Margin::same(20)).show(ui, |ui| {
        ui.set_min_width(260.0);
        ui.label(egui::RichText::new(label).size(18.0));
        ui.label(
            egui::RichText::new(value)
                .size(36.0)
                .strong()
                .color(egui::Color32::from_rgb(35, 73, 53)),
        );
    });
}
