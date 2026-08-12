//! Where a small window next to the bar goes.
//!
//! The only rule is that it has to be fully visible. Below the bar is where things end up
//! most of the time, simply because that is usually where they fit; a bar near the bottom
//! of the screen gets them above instead. Nothing here reaches the screen on Wayland,
//! where `with_position` is a no-op and the compositor decides.

use egui::{Pos2, Vec2};

use crate::theme;

/// Returns where the window goes and how tall it may actually be.
///
/// `monitor` is unknown on some platforms, in which case the window simply goes below the
/// bar and keeps the height it asked for.
pub fn near_bar(bar: (f32, f32), monitor: Option<Vec2>, width: f32, wanted: f32) -> (Pos2, f32) {
    let (x, y) = bar;
    let Some(monitor) = monitor else {
        return (egui::pos2(x, y + theme::BAR_HEIGHT), wanted);
    };
    let height = wanted.min(monitor.y);
    let below = y + theme::BAR_HEIGHT;
    let top = if below + height <= monitor.y {
        below
    } else {
        (y - height).max(0.0)
    };
    let left = x.min((monitor.x - width).max(0.0)).max(0.0);
    (egui::pos2(left, top), height)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: f32 = 270.0;

    #[test]
    fn a_window_that_fits_below_goes_below() {
        let (position, height) =
            near_bar((100.0, 40.0), Some(Vec2::new(1920.0, 1080.0)), WIDTH, 350.0);
        assert_eq!(position, egui::pos2(100.0, 72.0));
        assert_eq!(height, 350.0);
    }

    #[test]
    fn a_bar_at_the_bottom_gets_the_window_above_it() {
        let (position, _) = near_bar(
            (100.0, 1000.0),
            Some(Vec2::new(1920.0, 1080.0)),
            WIDTH,
            350.0,
        );
        assert_eq!(position, egui::pos2(100.0, 650.0));
    }

    /// A tall window on a short screen is pulled to the top edge and cut to fit, rather
    /// than being pushed off it.
    #[test]
    fn a_window_taller_than_the_screen_is_clamped() {
        let (position, height) =
            near_bar((0.0, 500.0), Some(Vec2::new(1024.0, 600.0)), WIDTH, 900.0);
        assert_eq!(height, 600.0);
        assert_eq!(position.y, 0.0);
    }

    /// The bar can sit further right than the window is wide.
    #[test]
    fn a_bar_near_the_right_edge_pulls_the_window_back() {
        let (position, _) = near_bar(
            (1800.0, 40.0),
            Some(Vec2::new(1920.0, 1080.0)),
            WIDTH,
            350.0,
        );
        assert_eq!(position.x, 1650.0);
    }
}
