//! Colors and metrics shared by every window.
//!
//! The metrics are constants; the colors live in a [`Palette`] that can be swapped at
//! runtime — between the dark and the light set, and with a random hue rotation on top —
//! so every window reads the current palette instead of hard-wired constants.

use std::sync::RwLock;

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

/// Every color the app paints with. One set for the dark look, one for the light look,
/// and any of them can be hue-rotated for a change of scenery.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Palette {
    /// Whether this is the light set, which picks egui's light base visuals.
    pub light: bool,
    pub background: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    /// The clock turns this colour once a task is past its daily timer.
    pub over_limit: Color32,
    /// The clock turns this colour once the whole day is past its timer. Outranks the amber.
    pub day_over: Color32,
    /// The background of a text field, such as the rename editor.
    pub field: Color32,
    /// A resting button in the menu and the report windows.
    pub button_idle: Color32,
    pub button_hover: Color32,
    pub button_active: Color32,
    /// Sweeps across a widget while it is held, showing how far the hold has come.
    pub hold_fill: Color32,
    /// The same sweep where it has to be painted over text rather than under it.
    pub hold_fill_over: Color32,
}

pub const DARK: Palette = Palette {
    light: false,
    background: Color32::from_rgb(0x1C, 0x20, 0x28),
    border: Color32::from_rgb(0x55, 0x5F, 0x70),
    text: Color32::from_rgb(0xE6, 0xE6, 0xE6),
    text_dim: Color32::from_rgb(0x9A, 0xA3, 0xB2),
    over_limit: Color32::from_rgb(0xE0, 0xB0, 0x5A),
    day_over: Color32::from_rgb(0xE8, 0x6A, 0x6A),
    field: Color32::from_rgb(0x12, 0x16, 0x1C),
    button_idle: Color32::from_rgb(0x25, 0x2C, 0x3A),
    button_hover: Color32::from_rgb(0x2C, 0x37, 0x4E),
    button_active: Color32::from_rgb(0x3D, 0x4B, 0x66),
    hold_fill: Color32::from_rgb(0x46, 0x59, 0x7A),
    hold_fill_over: Color32::from_rgba_premultiplied(0x23, 0x2C, 0x3D, 0x80),
};

/// The light set: the same roles with the lightness turned around, and the alarm colours
/// darkened so they stay readable on a bright background.
pub const LIGHT: Palette = Palette {
    light: true,
    background: Color32::from_rgb(0xEF, 0xF1, 0xF4),
    border: Color32::from_rgb(0x8A, 0x93, 0xA4),
    text: Color32::from_rgb(0x1E, 0x24, 0x2E),
    text_dim: Color32::from_rgb(0x5C, 0x66, 0x76),
    over_limit: Color32::from_rgb(0xA0, 0x6E, 0x10),
    day_over: Color32::from_rgb(0xC2, 0x38, 0x38),
    field: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    button_idle: Color32::from_rgb(0xDD, 0xE2, 0xEA),
    button_hover: Color32::from_rgb(0xC9, 0xD3, 0xE2),
    button_active: Color32::from_rgb(0xAF, 0xBE, 0xD6),
    hold_fill: Color32::from_rgb(0x9F, 0xB4, 0xD4),
    hold_fill_over: Color32::from_rgba_premultiplied(0x58, 0x60, 0x6C, 0x80),
};

static CURRENT: RwLock<Palette> = RwLock::new(DARK);

/// The palette every window paints with right now.
pub fn current() -> Palette {
    match CURRENT.read() {
        Ok(palette) => *palette,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

pub fn set_current(palette: Palette) {
    match CURRENT.write() {
        Ok(mut current) => *current = palette,
        Err(poisoned) => *poisoned.into_inner() = palette,
    }
}

/// The palette the stored preferences describe: dark or light, with an optional hue
/// rotation on top. `None` degrees means the stock colours.
pub fn palette_for(light: bool, hue_shift: Option<f32>) -> Palette {
    let base = if light { LIGHT } else { DARK };
    match hue_shift {
        Some(degrees) => base.rotated(degrees),
        None => base,
    }
}

impl Palette {
    /// The same palette with every colour's hue rotated by `degrees`. Saturation and
    /// lightness stay put, so contrast and readability survive any rotation.
    pub fn rotated(self, degrees: f32) -> Self {
        Self {
            light: self.light,
            background: rotate_hue(self.background, degrees),
            border: rotate_hue(self.border, degrees),
            text: rotate_hue(self.text, degrees),
            text_dim: rotate_hue(self.text_dim, degrees),
            over_limit: rotate_hue(self.over_limit, degrees),
            day_over: rotate_hue(self.day_over, degrees),
            field: rotate_hue(self.field, degrees),
            button_idle: rotate_hue(self.button_idle, degrees),
            button_hover: rotate_hue(self.button_hover, degrees),
            button_active: rotate_hue(self.button_active, degrees),
            hold_fill: rotate_hue(self.hold_fill, degrees),
            hold_fill_over: rotate_hue(self.hold_fill_over, degrees),
        }
    }
}

/// Text colour for a stack row: full strength at the top, fading towards the dim tone at
/// the bottom, so the top of the stack carries the visual weight.
pub fn stack_text(palette: &Palette, depth: usize, rows: usize) -> Color32 {
    if depth == 0 || rows <= 1 {
        return palette.text;
    }
    let t = depth as f32 / (rows - 1) as f32;
    palette.text.lerp_to_gamma(palette.text_dim, t)
}

/// Rotates a colour's hue by `degrees`, keeping saturation, lightness and alpha.
///
/// Works on the premultiplied channels directly: a hue rotation preserves the largest and
/// smallest channel values, so premultiplied colours stay valid.
fn rotate_hue(color: Color32, degrees: f32) -> Color32 {
    let [r, g, b, a] = color.to_array();
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let (r, g, b) = hsl_to_rgb((h + degrees).rem_euclid(360.0), s, l);
    Color32::from_rgba_premultiplied(r, g, b, a)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = f32::from(r) / 255.0;
    let g = f32::from(g) / 255.0;
    let b = f32::from(b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let delta = max - min;
    if delta <= f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = delta / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let channel = |value: f32| ((value + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (channel(r), channel(g), channel(b))
}

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

    /// A gray has no hue, so rotating it changes nothing — which is what keeps the near
    /// gray text readable under any shuffle.
    #[test]
    fn rotating_a_gray_changes_nothing() {
        let gray = Color32::from_rgb(0x80, 0x80, 0x80);
        assert_eq!(rotate_hue(gray, 123.0), gray);
    }

    /// A full turn comes back to where it started, give or take rounding.
    #[test]
    fn a_full_turn_is_the_identity() {
        for color in [DARK.button_hover, DARK.over_limit, LIGHT.button_active] {
            let turned = rotate_hue(color, 360.0);
            let [r0, g0, b0, _] = color.to_array();
            let [r1, g1, b1, _] = turned.to_array();
            assert!(r0.abs_diff(r1) <= 2, "{color:?} vs {turned:?}");
            assert!(g0.abs_diff(g1) <= 2, "{color:?} vs {turned:?}");
            assert!(b0.abs_diff(b1) <= 2, "{color:?} vs {turned:?}");
        }
    }

    /// Rotation moves the hue but keeps the lightness, which is what preserves contrast.
    #[test]
    fn rotation_preserves_lightness() {
        let [r, g, b, _] = DARK.button_active.to_array();
        let (_, _, before) = rgb_to_hsl(r, g, b);
        let [r, g, b, _] = rotate_hue(DARK.button_active, 90.0).to_array();
        let (_, _, after) = rgb_to_hsl(r, g, b);
        assert!((before - after).abs() < 0.02, "{before} vs {after}");
    }

    /// No shift means the stock palette, byte for byte.
    #[test]
    fn no_shift_is_the_stock_palette() {
        assert_eq!(palette_for(false, None), DARK);
        assert_eq!(palette_for(true, None), LIGHT);
        assert_ne!(palette_for(false, Some(120.0)), DARK);
    }

    /// The top row gets the full text colour, the bottom row the dim one, and the rows
    /// between them something in between.
    #[test]
    fn the_stack_fades_from_text_to_dim() {
        assert_eq!(stack_text(&DARK, 0, 5), DARK.text);
        assert_eq!(stack_text(&DARK, 4, 5), DARK.text_dim);
        let middle = stack_text(&DARK, 2, 5);
        assert_ne!(middle, DARK.text);
        assert_ne!(middle, DARK.text_dim);
        // A single row is simply the text colour.
        assert_eq!(stack_text(&DARK, 0, 1), DARK.text);
    }
}
