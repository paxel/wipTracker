//! Small self-painted widgets.
//!
//! The icons are painted rather than rendered as text: the default egui font has no
//! reliable glyph for a burger menu, and painting sidesteps the trap where a theme-wide
//! text color overrides per-state colors and makes hovered labels unreadable.

use egui::{Color32, Response, Sense, Stroke, Ui};

use crate::theme;

/// Text size for tooltips: the default body size is hard to read on a bar this small.
const TOOLTIP_TEXT_SIZE: f32 = 15.0;
/// Distance between a widget and its tooltip, enough to clear the pointer.
const TOOLTIP_GAP: f32 = 14.0;
/// Width the tooltip may grow to before it wraps.
const TOOLTIP_WIDTH: f32 = 320.0;

/// Shows `text` when `response` is hovered.
///
/// Anchored to the widget rather than following the pointer, and kept a gap away from it,
/// so the tooltip never opens under the mouse where the cursor covers the first words.
pub fn tooltip(response: Response, text: impl Into<String>) -> Response {
    let text = text.into();
    egui::Tooltip::for_enabled(&response)
        .gap(TOOLTIP_GAP)
        .show(|ui| {
            ui.set_max_width(TOOLTIP_WIDTH);
            ui.label(
                egui::RichText::new(text)
                    .size(TOOLTIP_TEXT_SIZE)
                    .color(theme::TEXT),
            );
        });
    response
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Plus,
    Burger,
}

impl Icon {
    /// The name screen readers and tests see, since the icons are painted, not written.
    fn label(self) -> &'static str {
        match self {
            Self::Plus => "new task",
            Self::Burger => "menu",
        }
    }
}

/// The drag handle at the left edge of the bar: a column of dots, dragged to move the
/// window. It exists so there is always somewhere unambiguous to grab, whatever else the
/// bar is showing.
pub fn grip(ui: &mut Ui) -> Response {
    let size = egui::vec2(theme::GRIP_WIDTH, theme::BUTTON_SIZE.y);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let enabled = ui.is_enabled();
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Other, enabled, "move the bar")
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let color = if response.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    let painter = ui.painter();
    let center = rect.center();
    for row in [-6.0, -2.0, 2.0, 6.0] {
        for column in [-2.0, 2.0] {
            painter.circle_filled(egui::pos2(center.x + column, center.y + row), 0.9, color);
        }
    }
    response
}

/// A square icon button that reacts to hover and press.
pub fn icon_button(ui: &mut Ui, icon: Icon) -> Response {
    let (rect, response) = ui.allocate_exact_size(theme::BUTTON_SIZE, Sense::click());
    let enabled = ui.is_enabled();
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, icon.label()));
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let fill = if response.is_pointer_button_down_on() {
        theme::BUTTON_ACTIVE
    } else if response.hovered() {
        theme::BUTTON_HOVER
    } else {
        Color32::TRANSPARENT
    };
    if fill.a() > 0 {
        ui.painter().rect_filled(rect, 4.0, fill);
    }

    let painter = ui.painter();
    let stroke = Stroke::new(1.6, theme::TEXT);
    let center = rect.center();
    match icon {
        Icon::Plus => {
            let arm = 5.0;
            painter.hline(center.x - arm..=center.x + arm, center.y, stroke);
            painter.vline(center.x, center.y - arm..=center.y + arm, stroke);
        }
        Icon::Burger => {
            let half = 5.5;
            for offset in [-5.0, 0.0, 5.0] {
                painter.hline(center.x - half..=center.x + half, center.y + offset, stroke);
            }
        }
    }

    response
}
