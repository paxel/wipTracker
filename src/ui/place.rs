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

use egui::{Pos2, Rect, Vec2};

/// Everything known about where the bar is, gathered once per frame.
///
/// `bar` is the bar's whole window in desktop coordinates — frame included, which is the
/// point: a decorated window is taller than the bar it contains, and placing a menu at
/// "top plus the bar's inner height" put it over the bar's own content, with just the
/// title bar peeking out. `None` on Wayland, where a position is unknowable.
#[derive(Clone, Copy, Default)]
pub struct Placement {
    pub bar: Option<Rect>,
    pub monitor: Option<Vec2>,
}

/// Whether the monitor's own bounds can be read as desktop coordinates.
///
/// They can only be trusted when the bar lies inside a rectangle of that size at the
/// origin. A bar further right or further down than the monitor is proof of a second
/// monitor next to or above it, and then nothing here knows where anything begins.
fn fits_one_monitor(bar: Rect, monitor: Vec2) -> bool {
    bar.min.x >= 0.0 && bar.min.y >= 0.0 && bar.max.x <= monitor.x && bar.max.y <= monitor.y
}

impl Placement {
    /// Returns where a window of height `wanted` goes, and how tall it may actually be.
    ///
    /// The left edge is always the bar's own, never clamped. A bar half way across a
    /// second monitor still has coordinates smaller than that monitor's width, so no test
    /// can tell it apart from a bar on one wide screen — and clamping threw it back onto
    /// the first screen from that point on.
    ///
    /// The vertical flip stays, because falling off the bottom is the common case, and it
    /// is only applied where the monitor's bounds can be read at all. Flipping puts the
    /// window's bottom against the bar's frame top, so it clears the title bar too.
    pub fn near_bar(&self, wanted: f32) -> (Option<Pos2>, f32) {
        let Some(bar) = self.bar else {
            return (None, wanted);
        };
        let below = egui::pos2(bar.min.x, bar.max.y);
        let Some(monitor) = self
            .monitor
            .filter(|monitor| fits_one_monitor(bar, *monitor))
        else {
            return (Some(below), wanted);
        };
        let height = wanted.min(monitor.y);
        let top = if below.y + height <= monitor.y {
            below.y
        } else {
            (bar.min.y - height).max(0.0)
        };
        (Some(egui::pos2(bar.min.x, top)), height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

    /// A frameless bar: the window is exactly the bar.
    fn bare(x: f32, y: f32) -> Placement {
        Placement {
            bar: Some(Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(304.0, 32.0),
            )),
            monitor: Some(ONE_SCREEN),
        }
    }

    /// A decorated bar: the frame adds a title bar above the 32 pixel content.
    fn decorated(x: f32, y: f32) -> Placement {
        Placement {
            bar: Some(Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(294.0, 60.0),
            )),
            monitor: Some(ONE_SCREEN),
        }
    }

    #[test]
    fn a_window_that_fits_below_goes_below() {
        let (position, height) = bare(100.0, 40.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 72.0)));
        assert_eq!(height, 350.0);
    }

    /// The regression the mac report was about: with a window frame, "the bar's top plus
    /// 32" is inside the window, and the menu covered everything but the title bar. The
    /// window's real bottom is what counts.
    #[test]
    fn a_decorated_bar_gets_the_window_below_its_frame() {
        let (position, _) = decorated(100.0, 40.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 100.0)));
    }

    #[test]
    fn a_bar_at_the_bottom_gets_the_window_above_it() {
        let (position, _) = bare(100.0, 1000.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 650.0)));
    }

    /// Flipping above a decorated bar clears the whole frame, not just the content.
    #[test]
    fn flipping_above_a_decorated_bar_clears_the_title_bar() {
        let (position, _) = decorated(100.0, 1000.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 650.0)));
    }

    /// A tall window on a short screen is pulled to the top edge and cut to fit, rather
    /// than being pushed off it.
    #[test]
    fn a_window_taller_than_the_screen_is_clamped() {
        let mut placement = bare(0.0, 500.0);
        placement.monitor = Some(Vec2::new(1024.0, 600.0));
        let (position, height) = placement.near_bar(900.0);
        assert_eq!(height, 600.0);
        assert_eq!(position.unwrap().y, 0.0);
    }

    /// Never pulled sideways. Half way across a second monitor the coordinates still look
    /// like a single wide screen, and clamping there is what moved the menu to the wrong
    /// monitor.
    #[test]
    fn the_left_edge_is_always_the_bar_own() {
        for x in [0.0, 1800.0, 3600.0, 5000.0] {
            let (position, _) = bare(x, 40.0).near_bar(350.0);
            assert_eq!(
                position.unwrap().x,
                x,
                "the window follows the bar at x = {x}"
            );
        }
    }

    /// A bar past the right edge of the reported monitor is on a second one, whose origin
    /// is unknowable; the bar's own position is simply kept.
    #[test]
    fn a_bar_on_a_monitor_to_the_right_keeps_its_own_coordinates() {
        let (position, height) = bare(4000.0, 200.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(4000.0, 232.0)));
        assert_eq!(height, 350.0);
    }

    /// The same below the reported monitor: a laptop panel mounted under an external
    /// screen has coordinates larger than its own height.
    #[test]
    fn a_bar_on_a_monitor_below_keeps_its_own_coordinates() {
        let (position, _) = bare(300.0, 1500.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(300.0, 1532.0)));
    }

    /// A negative coordinate means a monitor to the left of the primary one.
    #[test]
    fn a_bar_on_a_monitor_to_the_left_keeps_its_own_coordinates() {
        let (position, _) = bare(-1200.0, 40.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(-1200.0, 72.0)));
    }

    #[test]
    fn no_known_position_means_no_placement() {
        let placement = Placement {
            bar: None,
            monitor: Some(ONE_SCREEN),
        };
        let (position, height) = placement.near_bar(350.0);
        assert_eq!(position, None);
        assert_eq!(height, 350.0);
    }
}
