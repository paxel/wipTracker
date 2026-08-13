//! Where a small window next to the bar goes.
//!
//! The only rule is that it has to be fully visible. Below the bar is where things end up
//! most of the time, simply because that is usually where they fit; a bar near the bottom
//! of the screen gets them above instead. Nothing here reaches the screen on Wayland,
//! where `with_position` is a no-op and the compositor decides.
//!
//! What makes this awkward is that egui reports the size of the monitor the bar is on but
//! never where that monitor starts, while positions are in desktop coordinates spanning
//! every monitor. Treating the monitor as if it began at (0, 0) is what used to drag a
//! menu off a second screen and onto the first, so the monitor is only trusted when the
//! bar itself sits inside it — see [`fits_one_monitor`].

use egui::{Pos2, Vec2};

use crate::theme;

/// Whether the monitor's own bounds can be read as desktop coordinates.
///
/// They can only be trusted when the bar lies inside a rectangle of that size at the
/// origin. A bar further right or further down than the monitor is proof of a second
/// monitor next to or above it, and then nothing here knows where anything begins.
fn fits_one_monitor(bar: (f32, f32), monitor: Vec2) -> bool {
    let (x, y) = bar;
    x >= 0.0 && y >= 0.0 && x <= monitor.x && y + theme::BAR_HEIGHT <= monitor.y
}

/// Returns where the window goes and how tall it may actually be.
///
/// The left edge is always the bar's own, never clamped. A bar half way across a second
/// monitor still has coordinates smaller than that monitor's width, so no test can tell it
/// apart from a bar on one wide screen — and clamping to `monitor.x - width` threw it back
/// onto the first screen from that point on. Nothing is lost by leaving it: the menu and
/// the hint are no wider than the bar itself, so they stick out no further than it already
/// does.
///
/// The vertical flip stays, because falling off the bottom is the common case and it is
/// only applied where the monitor's bounds can be read at all.
pub fn near_bar(bar: (f32, f32), monitor: Option<Vec2>, _width: f32, wanted: f32) -> (Pos2, f32) {
    let (x, y) = bar;
    let below = egui::pos2(x, y + theme::BAR_HEIGHT);
    let Some(monitor) = monitor.filter(|monitor| fits_one_monitor(bar, *monitor)) else {
        return (below, wanted);
    };
    let height = wanted.min(monitor.y);
    let top = if below.y + height <= monitor.y {
        below.y
    } else {
        (y - height).max(0.0)
    };
    (egui::pos2(x, top), height)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: f32 = 270.0;
    const ONE_SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

    #[test]
    fn a_window_that_fits_below_goes_below() {
        let (position, height) = near_bar((100.0, 40.0), Some(ONE_SCREEN), WIDTH, 350.0);
        assert_eq!(position, egui::pos2(100.0, 72.0));
        assert_eq!(height, 350.0);
    }

    #[test]
    fn a_bar_at_the_bottom_gets_the_window_above_it() {
        let (position, _) = near_bar((100.0, 1000.0), Some(ONE_SCREEN), WIDTH, 350.0);
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

    /// Never pulled sideways. Half way across a second monitor the coordinates still look
    /// like a single wide screen, and clamping there is what moved the menu to the wrong
    /// monitor; the menu is no wider than the bar, so it costs nothing to leave it.
    #[test]
    fn the_left_edge_is_always_the_bar_own() {
        for x in [0.0, 1800.0, 3600.0, 5000.0] {
            let (position, _) = near_bar((x, 40.0), Some(ONE_SCREEN), WIDTH, 350.0);
            assert_eq!(position.x, x, "the window follows the bar at x = {x}");
        }
    }

    /// A bar past the right edge of the reported monitor is on a second one, whose origin
    /// is unknowable. Clamping to the first monitor's width used to drag the menu across
    /// to the other screen; now the bar's own position is simply kept.
    #[test]
    fn a_bar_on_a_monitor_to_the_right_keeps_its_own_coordinates() {
        let (position, height) = near_bar((4000.0, 200.0), Some(ONE_SCREEN), WIDTH, 350.0);
        assert_eq!(position, egui::pos2(4000.0, 232.0));
        assert_eq!(height, 350.0);
    }

    /// The same below the reported monitor: a laptop panel mounted under an external
    /// screen has coordinates larger than its own height.
    #[test]
    fn a_bar_on_a_monitor_below_keeps_its_own_coordinates() {
        let (position, _) = near_bar((300.0, 1500.0), Some(ONE_SCREEN), WIDTH, 350.0);
        assert_eq!(position, egui::pos2(300.0, 1532.0));
    }

    /// A negative coordinate means a monitor to the left of the primary one.
    #[test]
    fn a_bar_on_a_monitor_to_the_left_keeps_its_own_coordinates() {
        let (position, _) = near_bar((-1200.0, 40.0), Some(ONE_SCREEN), WIDTH, 350.0);
        assert_eq!(position, egui::pos2(-1200.0, 72.0));
    }
}
