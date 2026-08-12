//! Small self-painted widgets.
//!
//! The icons are painted rather than rendered as text: the default egui font has no
//! reliable glyph for a burger menu, and painting sidesteps the trap where a theme-wide
//! text color overrides per-state colors and makes hovered labels unreadable.

use egui::{Color32, Response, Sense, Stroke, Ui};

use crate::theme;

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
