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
    /// A branch peeling off a trunk: the other tasks you could switch to.
    Fork,
}

impl Icon {
    /// The name screen readers and tests see, since the icons are painted, not written.
    fn label(self) -> &'static str {
        match self {
            Self::Plus => "new task",
            Self::Burger => "menu",
            Self::Fork => "task stack",
        }
    }
}

/// What watching a press across frames produced.
#[derive(Clone, Copy, Default)]
pub struct Press {
    /// Released over the widget without the hold ever completing.
    pub clicked: bool,
    /// The hold completed on this frame. Reported once per press.
    pub long_pressed: bool,
    /// How far the hold has come, `0.0` to `1.0`, for the sweep.
    pub progress: f32,
}

/// Watches a press so that a click and a hold can share one widget.
///
/// The hold consumes the click: once it completes, letting go does nothing more. Sliding
/// off the widget before letting go cancels the hold and produces no click either. There
/// is deliberately no minimum press duration — a slow click is still a click, and a dead
/// zone between "too slow to click" and "not yet a hold" would look like a broken app.
///
/// `hold` is in seconds. `armed` says whether the hold leads anywhere at all; when it does
/// not, no sweep is drawn and the widget behaves as a plain button.
///
/// Time comes from egui rather than the clock so that tests can step it forward instead of
/// sleeping. This relies on `max_click_duration` being lifted (see `install_theme`):
/// egui's own `clicked()` otherwise refuses to fire after 0.8 seconds, which is well
/// inside the longest hold here.
pub fn track_press(ui: &Ui, response: &Response, hold: f32, armed: bool) -> Press {
    let key = response.id.with("press");
    let now = ui.input(|input| input.time);
    // Started, whether the hold has already fired, and when this was last written. The
    // last of those catches a widget that vanished mid-press — the bar swaps the name for
    // the rename editor, for one — and left its state behind: without it, the next press
    // on that id would start out already elapsed.
    let previous: Option<(f64, bool, f64)> = ui
        .data(|data| data.get_temp(key))
        .filter(|(_, _, seen): &(f64, bool, f64)| now - seen < STALE_PRESS);
    let mut press = Press::default();

    // Deliberately the raw pointer position rather than `contains_pointer`: the widget's
    // own tooltip opens after a fraction of a second, and being covered by a higher layer
    // is enough for egui to stop calling the pointer "contained" — which cancelled every
    // hold longer than the tooltip delay.
    let over = ui
        .input(|input| input.pointer.interact_pos())
        .is_some_and(|pos| response.rect.contains(pos));

    if response.is_pointer_button_down_on() && over {
        let (started, mut fired, _) = previous.unwrap_or((now, false, now));
        if armed {
            press.progress = (((now - started) as f32) / hold).clamp(0.0, 1.0);
            if press.progress >= 1.0 && !fired {
                fired = true;
                press.long_pressed = true;
            }
        }
        ui.data_mut(|data| data.insert_temp(key, (started, fired, now)));
        // The app asks for a repaint once a second, which is far too slow for a sweep.
        ui.ctx().request_repaint();
    } else {
        if previous.is_some() {
            ui.data_mut(|data| data.remove::<(f64, bool, f64)>(key));
        }
        press.clicked = response.clicked() && !previous.is_some_and(|(_, fired, _)| fired);
    }
    press
}

/// How long a press may go unwatched before it is treated as abandoned rather than live.
/// Comfortably longer than a frame, including the coarse frames the tests step through.
const STALE_PRESS: f64 = 1.0;

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

/// A square icon button that reacts to hover, press and hold.
///
/// `hold` and `armed` are handed straight to [`track_press`]; the sweep it reports is
/// painted under the icon.
pub fn icon_button(ui: &mut Ui, icon: Icon, hold: f32, armed: bool) -> (Response, Press) {
    let (rect, response) = ui.allocate_exact_size(theme::BUTTON_SIZE, Sense::click());
    let enabled = ui.is_enabled();
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, icon.label()));
    let press = track_press(ui, &response, hold, armed);
    if !ui.is_rect_visible(rect) {
        return (response, press);
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
    sweep(ui, rect, press.progress, theme::HOLD_FILL);

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
        Icon::Fork => {
            // A trunk with one branch peeling off to the upper right, a dot at each end:
            // deliberately unlike the burger's three straight lines, which sits two
            // buttons away.
            let trunk = center.x - 3.0;
            let (top, bottom) = (center.y - 6.0, center.y + 6.0);
            painter.vline(trunk, top..=bottom, stroke);
            painter.line_segment(
                [
                    egui::pos2(trunk, center.y + 1.0),
                    egui::pos2(center.x + 4.0, top + 1.0),
                ],
                stroke,
            );
            for point in [
                egui::pos2(trunk, top),
                egui::pos2(trunk, bottom),
                egui::pos2(center.x + 4.0, top + 1.0),
            ] {
                painter.circle_filled(point, 1.6, theme::TEXT);
            }
        }
    }

    (response, press)
}

/// Fills `rect` from the left to show how far a hold has come.
pub fn sweep(ui: &Ui, rect: egui::Rect, progress: f32, color: Color32) {
    if progress <= 0.0 {
        return;
    }
    let filled = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width() * progress.min(1.0), rect.height()),
    );
    ui.painter().rect_filled(filled, 4.0, color);
}
