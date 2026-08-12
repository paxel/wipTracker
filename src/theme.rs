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

pub const BAR_SIZE: Vec2 = Vec2::new(
    GRIP_WIDTH + LABEL_WIDTH + CLOCK_WIDTH + 2.0 * BUTTON_SIZE.x + 2.0 * BAR_MARGIN,
    32.0,
);

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
