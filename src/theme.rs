//! Colors and metrics shared by every window.

use egui::{Color32, Vec2};

/// Width reserved for the task name (roughly 4 cm at 96 dpi).
pub const LABEL_WIDTH: f32 = 150.0;
/// Width reserved for the running clock.
pub const CLOCK_WIDTH: f32 = 54.0;
/// Width of the grip at the left edge, which exists only to drag the window.
pub const GRIP_WIDTH: f32 = 10.0;
pub const BUTTON_SIZE: Vec2 = Vec2::new(26.0, 24.0);
pub const BAR_MARGIN: f32 = 6.0;

/// The bar carries three buttons: the fork, the plus and the burger.
pub const BUTTON_COUNT: f32 = 3.0;
pub const BAR_HEIGHT: f32 = 32.0;

/// How big the bar is, which depends on whether it wears a window frame.
///
/// The grip exists only to drag an undecorated window. A decorated one is dragged by its
/// title bar, so the grip is dropped there and the bar is that much narrower — ten pixels
/// matter on a bar this small.
pub fn bar_size(decorated: bool) -> Vec2 {
    Vec2::new(
        grip_width(decorated)
            + LABEL_WIDTH
            + CLOCK_WIDTH
            + BUTTON_COUNT * BUTTON_SIZE.x
            + 2.0 * BAR_MARGIN,
        BAR_HEIGHT,
    )
}

/// The width the grip takes up, which is none at all on a decorated window.
pub fn grip_width(decorated: bool) -> f32 {
    if decorated { 0.0 } else { GRIP_WIDTH }
}

pub const BACKGROUND: Color32 = Color32::from_rgb(0x1C, 0x20, 0x28);
pub const BORDER: Color32 = Color32::from_rgb(0x55, 0x5F, 0x70);
pub const TEXT: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9A, 0xA3, 0xB2);
/// The clock turns this colour once a task is past its daily timer.
pub const OVER_LIMIT: Color32 = Color32::from_rgb(0xE0, 0xB0, 0x5A);
/// The background of a text field, such as the rename editor.
pub const FIELD: Color32 = Color32::from_rgb(0x12, 0x16, 0x1C);
/// A resting button in the menu and the report windows.
pub const BUTTON_IDLE: Color32 = Color32::from_rgb(0x25, 0x2C, 0x3A);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(0x2C, 0x37, 0x4E);
pub const BUTTON_ACTIVE: Color32 = Color32::from_rgb(0x3D, 0x4B, 0x66);
/// Sweeps across a widget while it is held, showing how far the hold has come.
pub const HOLD_FILL: Color32 = Color32::from_rgb(0x46, 0x59, 0x7A);
/// The same sweep where it has to be painted over text rather than under it.
pub const HOLD_FILL_OVER: Color32 = Color32::from_rgba_premultiplied(0x23, 0x2C, 0x3D, 0x80);
/// Tints the part of the hold indicator that has not been reached yet.
pub const HOLD_DIM: Color32 = Color32::from_rgb(0x50, 0x58, 0x66);

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar is exactly its parts, with and without the grip. The name column is sized
    /// from the same sum, so a mismatch would let the name overlap the buttons.
    #[test]
    fn the_bar_is_the_sum_of_its_parts() {
        for decorated in [false, true] {
            let expected = grip_width(decorated)
                + LABEL_WIDTH
                + CLOCK_WIDTH
                + BUTTON_COUNT * BUTTON_SIZE.x
                + 2.0 * BAR_MARGIN;
            assert_eq!(bar_size(decorated).x, expected);
        }
        assert_eq!(bar_size(true).x, bar_size(false).x - GRIP_WIDTH);
    }
}
