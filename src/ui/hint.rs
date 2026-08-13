//! The window that explains what the bar's controls do, and how far a hold has come.
//!
//! It cannot be a tooltip. egui constrains a tooltip to the window it belongs to, and the
//! bar's window is 32 pixels tall, so a three-line explanation is squeezed into a strip
//! nothing can be read in. The hint is therefore a window of its own, sitting next to the
//! bar: no focus, no taskbar entry, and transparent to the mouse, so it never gets in the
//! way of the control it is describing.

use egui::{Context, ViewportBuilder, ViewportId};

use crate::theme;
use crate::ui::place::Placement;

/// What the bar wants explained this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Hint {
    pub text: String,
    /// How far a hold on this control has come, when one is running.
    pub progress: Option<f32>,
}

impl Hint {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            progress: None,
        }
    }
}

/// Narrower than the bar in either of its widths, so a hint on a bar flush against the
/// right screen edge sticks out no further than the bar already does — the same reasoning
/// that lets the placement keep the bar's own left edge without clamping.
const WIDTH: f32 = 280.0;
const TEXT_SIZE: f32 = 15.0;
const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 10.0;
/// The side of the cat, which is only drawn while a hold is running.
const CAT: f32 = 56.0;
/// Roughly how many characters fit on a line at [`TEXT_SIZE`] in [`WIDTH`].
const CHARS_PER_LINE: usize = 38;

/// How tall the window has to be. Estimated rather than measured: the size has to be known
/// before the window exists, and a line too many costs nothing but a little empty space.
fn wanted_height(hint: &Hint) -> f32 {
    let lines: usize = hint
        .text
        .lines()
        .map(|line| 1 + line.chars().count() / CHARS_PER_LINE)
        .sum();
    lines as f32 * LINE_HEIGHT
        + 2.0 * PADDING
        + if hint.progress.is_some() {
            CAT + PADDING
        } else {
            0.0
        }
}

/// The reading cat, loaded once and kept in the context.
///
/// The same raw buffer the taskbar icon is built from, so no image decoder is needed.
fn cat(ctx: &Context) -> egui::TextureHandle {
    let id = egui::Id::new("hint_cat");
    if let Some(handle) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return handle;
    }
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [64, 64],
        include_bytes!("../../assets/icon.rgba"),
    );
    let handle = ctx.load_texture("hint_cat", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, handle.clone()));
    handle
}

/// Draws the cat greyed out, with the part that is already held painted back in.
///
/// Left to right, so it reads the same way as the sweep on the control being held.
fn draw_cat(ui: &mut egui::Ui, progress: f32) {
    let texture = cat(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(CAT, CAT), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let whole = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let painter = ui.painter();
    painter.image(texture.id(), rect, whole, theme::HOLD_DIM);

    let filled = progress.clamp(0.0, 1.0);
    if filled <= 0.0 {
        return;
    }
    painter.image(
        texture.id(),
        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * filled, rect.height())),
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(filled, 1.0)),
        egui::Color32::WHITE,
    );
}

/// Shows the hint next to the bar. On Wayland the placement is unknowable and the
/// compositor decides.
pub fn show(ctx: &Context, hint: &Hint, placement: &Placement) {
    // Where viewports cannot be separate windows — the test harness, the web backend —
    // this one would be drawn on top of the bar and swallow the very click it is
    // describing. A hint that covers the control it explains is worse than no hint.
    if ctx.embed_viewports() {
        return;
    }
    let (position, height) = placement.near_bar(wanted_height(hint));

    let mut builder = ViewportBuilder::default()
        .with_title("WipTracker hint")
        .with_inner_size([WIDTH, height])
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        // Never takes the focus and never appears in the taskbar: it is a label that
        // happens to need its own window, not something to switch to. Passing the mouse
        // through means it cannot swallow the click that ends a hold either.
        .with_active(false)
        .with_taskbar(false)
        .with_mouse_passthrough(true);
    if let Some(position) = position {
        builder = builder.with_position(position);
    }

    ctx.show_viewport_immediate(ViewportId::from_hash_of("hint"), builder, |ctx, _class| {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .inner_margin(PADDING as i8),
            )
            .show(ctx, |ui| {
                if let Some(progress) = hint.progress {
                    draw_cat(ui, progress);
                    ui.add_space(PADDING);
                }
                ui.label(
                    egui::RichText::new(&hint.text)
                        .size(TEXT_SIZE)
                        .color(theme::TEXT),
                );
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_one_line_hint_is_shorter_than_a_three_line_one() {
        let one = wanted_height(&Hint::text("Click: rename"));
        let three = wanted_height(&Hint::text("Click: rename\nHold: finish\nOr something"));
        assert!(three > one);
    }

    #[test]
    fn a_hold_makes_room_for_the_cat() {
        let plain = Hint::text("Click: rename");
        let held = Hint {
            progress: Some(0.5),
            ..plain.clone()
        };
        assert!(wanted_height(&held) > wanted_height(&plain) + CAT - 1.0);
    }

    /// A line long enough to wrap counts for more than one.
    #[test]
    fn a_long_line_is_counted_as_wrapped() {
        let short = wanted_height(&Hint::text("short"));
        let long = wanted_height(&Hint::text("x".repeat(CHARS_PER_LINE * 3)));
        assert!(long >= short + 3.0 * LINE_HEIGHT - 1.0);
    }
}
